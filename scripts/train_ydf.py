#!/usr/bin/env python3
"""Train the YDF lens for the gittables multi-lens corpus diagnostic.

Spec: `2026-05-20-gittables-multi-lens-diagnostic`
ac-03 (YDF training pipeline + leakage audit).

Training corpus: synthetic-generator output from
`crates/finetype-core/src/generator.rs`, materialised by
`finetype generate`. Values are grouped into 8-value synthetic columns
(matching `OBSERVED_SAMPLE_LIMIT`).

Lens independence (per spec ac-03):
  (a) Excludes ≥1 of Sense's five branches {char, embed, stats, header,
      validation} entirely. This script excludes char, embed, header,
      validation — keeping only stats from Sense's set.
  (b) Includes ≥1 feature category NOT in any Sense branch. This
      script uses character-bigram TF-IDF (a non-Sense category).

Excluded from training: every row of labelled_eval.tsv. The synthetic
generator produces de-novo values, so no overlap is expected by
construction; the leakage audit (separate script) formalises this.

Output:
  - eval/gittables/models/ydf.bin                — trained YDF model
  - eval/gittables/models/training_rows_manifest.tsv — one row per
    training example: (source, generator, type_id, sample_idx, value_hash)
  - eval/gittables/models/ydf_tfidf_vocab.json   — bigram vocabulary
    (needed by eval script to compute features deterministically)
  - eval/gittables/models/ydf_meta.json          — training metadata
    (samples per type, n_train, val_acc, feature categories)

Usage:
  python3 scripts/train_ydf.py [--samples 200] [--seed 42]
"""

from __future__ import annotations

import argparse
import csv
import gzip
import hashlib
import json
import subprocess
import sys
import tempfile
from collections import Counter
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from gold_anchor_guard import (  # noqa: E402
    DEFAULT_GOLD,
    is_gold_column,
    load_gold_identities,
)

REPO = Path(__file__).resolve().parent.parent
DEFAULT_OUT = REPO / "eval" / "gittables" / "models"
DEFAULT_DISTILLED = REPO / "output" / "distillation-v3" / "sherlock_distilled.csv.gz"
DEFAULT_LABELLED_EVAL = (
    REPO / ".orbit" / "specs" / "2026-05-04-autonomous-type-inference"
    / "labelled_eval.tsv"
)
SAMPLES_PER_COLUMN = 8  # matches OBSERVED_SAMPLE_LIMIT
LABELLED_EVAL_SAMPLE_SEP = "│"  # U+2502
SENSE_BRANCHES = ["char", "embed", "stats", "header", "validation"]
USED_SENSE_BRANCHES = ["stats"]
EXCLUDED_SENSE_BRANCHES = ["char", "embed", "header", "validation"]
NON_SENSE_CATEGORIES = ["tfidf_char_bigram"]


def _generate_synthetic(samples_per_type: int, seed: int, out: Path) -> int:
    cmd = [
        "finetype", "generate",
        "--samples", str(samples_per_type),
        "--seed", str(seed),
        "--output", str(out),
    ]
    res = subprocess.run(cmd, capture_output=True, text=True, timeout=600)
    if res.returncode != 0:
        raise RuntimeError(
            f"finetype generate failed: {res.stderr.strip()[-400:]}"
        )
    n = 0
    with out.open() as fh:
        for _ in fh:
            n += 1
    print(f"generated {n} synthetic samples → {out}", file=sys.stderr)
    return n


def _labelled_eval_hashes(tsv: Path) -> set[str]:
    """Pre-compute the per-row value-hashes of every labelled_eval row so
    distilled rows whose sample values match can be excluded from
    training (constraint 6 leakage firewall)."""
    out: set[str] = set()
    if not tsv.exists():
        return out
    with tsv.open() as fh:
        for row in csv.DictReader(fh, delimiter="\t"):
            raw = (row.get("observed_values_sample") or "").strip()
            if not raw:
                continue
            values = [v for v in raw.split(LABELLED_EVAL_SAMPLE_SEP) if v]
            if values:
                out.add(_value_hash(values))
    return out


def _load_distilled_columns(
    distilled: Path,
    excluded_hashes: set[str] | None = None,
    gold_ids: set[tuple[str, str]] | None = None,
) -> tuple[list[tuple[str, list[str], str]], int, int]:
    """Returns [(type_id, [val1..valK], source_tag)] from the distilled
    CSV. Sample values are JSON-parsed, truncated to SAMPLES_PER_COLUMN,
    and filtered for empty strings.

    source_tag is "distilled" (vs the "synthetic" tag emitted by the
    NDJSON grouper). Used downstream in the training-rows manifest.

    Two leakage filters apply, in order of independence strength:
      - value-hash (excluded_hashes): drops rows whose sampled value tuple
        matches a labelled_eval row. Window-sensitive (see gold_anchor_guard).
      - gold identity (gold_ids): drops any row carrying a gold-anchor
        (file_content_sha256, column_name). Keyed on durable identity, so it
        holds regardless of sampling window (spec ac-06). Distilled Sherlock
        rows carry no such identity today, so this fires 0 times now — the
        wiring is the mechanical guarantee, not a current-corpus correction.
    """
    out: list[tuple[str, list[str], str]] = []
    n_excluded = 0
    n_gold_excluded = 0
    excluded = excluded_hashes or set()
    gold = gold_ids or set()
    with gzip.open(distilled, "rt") as fh:
        for row in csv.DictReader(fh):
            label = (row.get("final_label") or "").strip()
            samples_raw = (row.get("sample_values") or "").strip()
            if not label or not samples_raw:
                continue
            if is_gold_column(
                row.get("file_content_sha256"), row.get("column_name"), gold
            ):
                n_gold_excluded += 1
                continue
            try:
                samples = json.loads(samples_raw)
            except json.JSONDecodeError:
                continue
            vals = [
                str(v) for v in samples
                if v is not None and str(v).strip()
            ]
            if not vals:
                continue
            vals = vals[:SAMPLES_PER_COLUMN]
            if excluded and _value_hash(vals) in excluded:
                n_excluded += 1
                continue
            out.append((label, vals, "distilled"))
    return out, n_excluded, n_gold_excluded


def _group_into_columns(ndjson: Path) -> list[tuple[str, list[str]]]:
    """Reads NDJSON, groups consecutive samples-of-same-type into columns
    of SAMPLES_PER_COLUMN values. Returns [(type_id, [val1, ..., valK])].
    Order is preserved from the NDJSON; finetype generate emits samples
    grouped by label, so consecutive same-label rows form columns
    naturally."""
    columns: list[tuple[str, list[str]]] = []
    current_type: str | None = None
    current_vals: list[str] = []
    with ndjson.open() as fh:
        for line in fh:
            row = json.loads(line)
            t = row["classification"]
            v = row["text"]
            if t != current_type:
                if current_type and current_vals:
                    for i in range(0, len(current_vals), SAMPLES_PER_COLUMN):
                        chunk = current_vals[i:i + SAMPLES_PER_COLUMN]
                        if len(chunk) >= 1:
                            columns.append((current_type, chunk))
                current_type = t
                current_vals = [v]
            else:
                current_vals.append(v)
    if current_type and current_vals:
        for i in range(0, len(current_vals), SAMPLES_PER_COLUMN):
            chunk = current_vals[i:i + SAMPLES_PER_COLUMN]
            if len(chunk) >= 1:
                columns.append((current_type, chunk))
    return columns


def _stats_features(values: list[str]) -> dict[str, float]:
    """Sense's `stats` branch — basic numerical aggregates over the
    column's values. Reuse-friendly subset."""
    if not values:
        return {}
    lens = [len(v) for v in values]
    n_digit_chars = sum(sum(1 for c in v if c.isdigit()) for v in values)
    n_alpha_chars = sum(sum(1 for c in v if c.isalpha()) for v in values)
    n_punct_chars = sum(
        sum(1 for c in v if not c.isalnum() and not c.isspace())
        for v in values
    )
    n_total_chars = sum(lens) or 1
    distinct = len(set(values))
    return {
        "stats_mean_len": sum(lens) / len(lens),
        "stats_min_len": min(lens),
        "stats_max_len": max(lens),
        "stats_range_len": max(lens) - min(lens),
        "stats_digit_frac": n_digit_chars / n_total_chars,
        "stats_alpha_frac": n_alpha_chars / n_total_chars,
        "stats_punct_frac": n_punct_chars / n_total_chars,
        "stats_distinct_ratio": distinct / len(values),
        "stats_n_values": len(values),
    }


def _char_bigrams(value: str) -> list[str]:
    s = "^" + value + "$"
    return [s[i:i + 2] for i in range(len(s) - 1)]


def _build_tfidf_vocab(
    columns: list[tuple[str, list[str]]], top_k: int = 200
) -> list[str]:
    counts: Counter[str] = Counter()
    for _, vals in columns:
        for v in vals:
            for bg in set(_char_bigrams(v)):
                counts[bg] += 1
    return [bg for bg, _ in counts.most_common(top_k)]


def _tfidf_features(values: list[str], vocab: list[str]) -> dict[str, float]:
    bg_counts: Counter[str] = Counter()
    total_bg = 0
    for v in values:
        for bg in _char_bigrams(v):
            bg_counts[bg] += 1
            total_bg += 1
    total_bg = total_bg or 1
    out: dict[str, float] = {}
    for bg in vocab:
        out[f"bg_{bg}"] = bg_counts.get(bg, 0) / total_bg
    return out


def _features(values: list[str], vocab: list[str]) -> dict[str, float]:
    f = _stats_features(values)
    f.update(_tfidf_features(values, vocab))
    return f


def _value_hash(values: list[str]) -> str:
    h = hashlib.sha256()
    for v in values:
        h.update(v.encode("utf-8"))
        h.update(b"\x00")
    return h.hexdigest()


def main() -> int:
    parser = argparse.ArgumentParser(description="Train YDF lens for ac-03.")
    parser.add_argument("--samples", type=int, default=200,
                        help="Synthetic samples per type (default 200).")
    parser.add_argument("--seed", type=int, default=42)
    parser.add_argument("--out-dir", type=Path, default=DEFAULT_OUT)
    parser.add_argument("--ndjson", type=Path, default=None,
                        help="Reuse an existing synthetic NDJSON instead of "
                             "regenerating.")
    parser.add_argument("--distilled", type=Path, default=DEFAULT_DISTILLED,
                        help="Distilled training CSV (gzipped).")
    parser.add_argument("--no-distilled", action="store_true",
                        help="Train on synthetic only (debug/comparison).")
    parser.add_argument("--tfidf-top-k", type=int, default=500)
    args = parser.parse_args()

    try:
        import ydf  # type: ignore
        import pandas as pd  # type: ignore
    except ImportError as exc:
        print(f"error: missing dependency ({exc}). Activate the venv: "
              "source eval/gittables/.venv/bin/activate", file=sys.stderr)
        return 2

    args.out_dir.mkdir(parents=True, exist_ok=True)

    if args.ndjson and args.ndjson.exists():
        ndjson_path = args.ndjson
        print(f"reusing {ndjson_path}", file=sys.stderr)
    else:
        tmp = Path(tempfile.mkdtemp(prefix="ydf-train-"))
        ndjson_path = tmp / "synthetic.ndjson"
        _generate_synthetic(args.samples, args.seed, ndjson_path)

    synthetic_cols = _group_into_columns(ndjson_path)
    print(f"grouped {len(synthetic_cols)} synthetic training columns "
          f"(target ≥1 per type, samples/col ≤ {SAMPLES_PER_COLUMN})",
          file=sys.stderr)

    distilled_cols: list[tuple[str, list[str], str]] = []
    n_distilled_excluded = 0
    if not args.no_distilled:
        if not args.distilled.exists():
            print(f"error: distilled file not found: {args.distilled}",
                  file=sys.stderr)
            return 2
        labelled_hashes = _labelled_eval_hashes(DEFAULT_LABELLED_EVAL)
        gold_ids = load_gold_identities(DEFAULT_GOLD)
        print(f"  labelled_eval hash count: {len(labelled_hashes)}",
              file=sys.stderr)
        print(f"  gold-anchor identity count: {len(gold_ids)} "
              f"(excluded by (file, column), spec ac-06)", file=sys.stderr)
        distilled_cols, n_distilled_excluded, n_gold_excluded = (
            _load_distilled_columns(
                args.distilled, excluded_hashes=labelled_hashes,
                gold_ids=gold_ids,
            )
        )
        print(f"loaded {len(distilled_cols)} distilled training columns "
              f"from {args.distilled.name} "
              f"({n_distilled_excluded} excluded for labelled_eval overlap, "
              f"{n_gold_excluded} excluded for gold-anchor identity)",
              file=sys.stderr)

    # Merge: tag synthetic rows with "synthetic" source; distilled rows
    # already carry "distilled". Order: distilled first, then synthetic
    # — deterministic given the same input files.
    merged: list[tuple[str, list[str], str]] = list(distilled_cols)
    for t, v in synthetic_cols:
        merged.append((t, v, "synthetic"))

    type_counts = Counter(t for t, _, _ in merged)
    source_counts = Counter(s for _, _, s in merged)
    print(
        f"  merged training: {len(merged)} columns "
        f"({dict(source_counts)})",
        file=sys.stderr,
    )
    print(f"  distinct types in training: {len(type_counts)}",
          file=sys.stderr)

    vocab = _build_tfidf_vocab(
        [(t, v) for t, v, _ in merged], top_k=args.tfidf_top_k
    )
    (args.out_dir / "ydf_tfidf_vocab.json").write_text(
        json.dumps(vocab, indent=2)
    )

    manifest_rows = []
    feature_rows = []
    for sample_idx, (type_id, vals, source) in enumerate(merged):
        feats: dict[str, float | str] = dict(_features(vals, vocab))
        feats["label"] = type_id
        feature_rows.append(feats)
        manifest_rows.append({
            "source": source,
            "generator": (
                "crates/finetype-core/src/generator.rs"
                if source == "synthetic"
                else "output/distillation-v3/sherlock_distilled.csv.gz"
            ),
            "type_id": type_id,
            "sample_idx": sample_idx,
            "value_hash": _value_hash(vals),
        })

    manifest_path = args.out_dir / "training_rows_manifest.tsv"
    with manifest_path.open("w") as fh:
        fh.write("source\tgenerator\ttype_id\tsample_idx\tvalue_hash\n")
        for r in manifest_rows:
            fh.write(
                f"{r['source']}\t{r['generator']}\t{r['type_id']}\t"
                f"{r['sample_idx']}\t{r['value_hash']}\n"
            )
    print(f"wrote {manifest_path}", file=sys.stderr)

    df = pd.DataFrame(feature_rows)
    print(f"features: {df.shape[1] - 1} columns, "
          f"{df.shape[0]} training rows", file=sys.stderr)

    learner = ydf.RandomForestLearner(
        label="label",
        task=ydf.Task.CLASSIFICATION,
        num_trees=200,
        max_depth=24,
    )
    model = learner.train(df)
    bin_path = args.out_dir / "ydf.bin"
    if bin_path.exists():
        import shutil
        shutil.rmtree(bin_path)
    model.save(str(bin_path))
    print(f"wrote {bin_path}", file=sys.stderr)

    eval_view = model.evaluate(df)
    val_acc = float(eval_view.accuracy)

    meta = {
        "samples_per_type": args.samples,
        "seed": args.seed,
        "n_training_columns": len(merged),
        "n_distilled_columns": len(distilled_cols),
        "n_synthetic_columns": len(synthetic_cols),
        "n_distilled_excluded_for_leakage": n_distilled_excluded,
        "n_distinct_types": len(type_counts),
        "tfidf_top_k": args.tfidf_top_k,
        "tfidf_vocab_size": len(vocab),
        "samples_per_column": SAMPLES_PER_COLUMN,
        "train_acc": round(val_acc, 6),
        "sense_branches_used": USED_SENSE_BRANCHES,
        "sense_branches_excluded": EXCLUDED_SENSE_BRANCHES,
        "non_sense_categories_used": NON_SENSE_CATEGORIES,
    }
    (args.out_dir / "ydf_meta.json").write_text(
        json.dumps(meta, indent=2) + "\n"
    )
    print(json.dumps(meta, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
