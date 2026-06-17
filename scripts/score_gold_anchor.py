#!/usr/bin/env python3
"""Score a model's predictions against the gold eval anchor.

Spec 2026-06-05-gold-eval-anchor ac-04 (harness) + ac-05 (v19 re-baseline).

Two modes:

  predict — for each gold column, pull its sample values from the corpus parquet,
            run the REAL Sense path (`finetype profile -f csv -o json-schema`,
            reading x-finetype-label — uses models/default), and write a
            predictions TSV keyed on (file_content_sha256, column_name). This is
            how the shipped default (v19) baseline is produced. NB: not
            `infer --mode column`, which returns a degenerate container.array
            tie for plain numeric columns — see _profile_column below.

  score   — join the gold fixture with ANY predictions TSV and report per-family
            precision/recall. The gold label is ground truth; YDF is NOT consulted
            (the harness is independent of the mining lens by construction).

The harness is model-agnostic: any future bet (B3 late-fusion, v24, a retrain)
emits a predictions TSV with columns (file_content_sha256, column_name,
predicted_label) and is scored by the same instrument.

Output is sanitised — metrics, labels and counts only, never raw values.

Examples:
  ../eval/gittables/.venv/bin/python score_gold_anchor.py predict \\
      --gold ../eval/gold/gold_eval_anchor.tsv \\
      --columns ../eval/gittables/corpus_pass/columns.parquet \\
      --binary ../target/release/finetype \\
      --out ../output/gold-eval-anchor/predictions_v19.tsv

  python score_gold_anchor.py score \\
      --gold ../eval/gold/gold_eval_anchor.tsv \\
      --predictions ../output/gold-eval-anchor/predictions_v19.tsv \\
      --model-name v19-models-default \\
      --out-dir ../output/gold-eval-anchor

LOAD-BEARING PROPERTIES (read before building analysis on top of this harness):

  1. SAMPLE CONSISTENCY. `predict` feeds the model the SMALL per-column sample
     stored in `sample_values_truncated` (a handful of representative values,
     ~8), NOT the full column. Crucially, the gold labels were ADJUDICATED from
     those same samples (the fixture's sense_context / ydf_context). So the eval
     is internally consistent: the model is scored on the same evidence the
     labeller saw. Do NOT "improve" predict by reading full columns from disk —
     that feeds the model more than ground truth was based on, and accuracy drops
     against gold (measured 2026-06-17: 0.785 -> 0.707). It is not understatement;
     it is consistency. A full-column-statistic study CANNOT be run on this
     harness for the same reason (predictions and any disk-read stats come from
     different value sets). See memory `full-column-stat-sharpen-is-redundant`.

  2. SEP is "│" (U+2502 BOX DRAWINGS LIGHT VERTICAL), not a control char. When
     parsing `sample_values_truncated` anywhere, import this SEP — do not retype a
     guess (splitting on the wrong byte silently collapses every sample to one
     value, which reads as a fake "n=1" degeneracy).
"""
from __future__ import annotations

import argparse
import csv
import datetime as dt
import json
import subprocess
import sys
from collections import defaultdict
from pathlib import Path

SEP = "│"


def load_gold(path: Path) -> list[dict]:
    with path.open() as fh:
        return list(csv.DictReader(fh, delimiter="\t"))


def _profile_column(binary: Path, column_name: str, values: list[str]) -> str:
    """Run the REAL Sense pipeline: reconstruct a single-column CSV and profile
    it (`finetype profile -f csv -o json-schema`), returning x-finetype-label.

    This is the same path the corpus diagnostic used to produce sense_prediction
    (scripts/gittables_corpus_pass.py via gittables_gate._profile) — NOT
    `infer --mode column`, which is a refinement path that returns a degenerate
    container.array tie for plain numeric columns.
    """
    import tempfile

    with tempfile.TemporaryDirectory() as td:
        csv_path = Path(td) / "col.csv"
        with csv_path.open("w", newline="") as fh:
            w = csv.writer(fh, lineterminator="\n")
            w.writerow([column_name])
            for v in values:
                w.writerow([v])
        proc = subprocess.run(
            [str(binary), "profile", "-f", str(csv_path), "-o", "json-schema"],
            capture_output=True,
            text=True,
            timeout=120,
        )
        if proc.returncode != 0:
            raise RuntimeError(f"profile failed: {proc.stderr.strip()[-200:]}")
        schema = json.loads(proc.stdout)
    defn = schema.get("properties", {}).get(column_name, {})
    return defn.get("x-finetype-label", "") if isinstance(defn, dict) else ""


def _open_row_identity(r: dict) -> dict:
    """The open-rows CSV carries row_index into the llm TSV; resolve identity."""
    llm = Path("eval/gold/gold_review_queue_v1_llm.tsv")
    if not hasattr(_open_row_identity, "_cache"):
        _open_row_identity._cache = {
            row["row_index"]: row for row in load_gold(llm)} if llm.exists() else {}
    row = _open_row_identity._cache.get(r["row_index"], {})
    paths: dict[str, str] = {}
    for cand in (Path("eval/gold/gold_corpus_candidates.tsv"),
                 Path("eval/gold/gold_corpus_candidates_external.tsv")):
        if cand.exists():
            for c in load_gold(cand):
                paths.setdefault(c["file_content_sha256"], c["file_path"])
    sha = row.get("file_content_sha256", "")
    return {"family": "author-open:" + row.get("stratum", ""),
            "file_path": paths.get(sha, ""), "file_content_sha256": sha,
            "column_name": row.get("column_name", "")}


def cmd_build_gold(args: argparse.Namespace) -> int:
    """Merge the verified gold layers into one fixture (gold_corpus_v1.tsv).

    Layers (spec 2026-06-10-human-verified-gold-corpus ac-06):
      anchor      eval/gold/gold_eval_anchor.tsv (240, human-curated)
      consensus   gold_lens_labels.tsv rows with verdict=consensus (ac-03)
      adjudicated gold_review_queue_v1_review.csv rows where the author has
                  filled human_verdict (correct -> proposed_label;
                  wrong -> correct_label_if_wrong; unsure -> skipped)
      llm tier    gold_review_queue_v1_llm.tsv rows double-confirmed by two
                  independent panels (status=llm-adjudicated), ACCEPTED by the
                  author 2026-06-10 via a 40/40 spot-check (Wilson 95% lower
                  bound on agreement >= 0.91). Spot-checked rows carry the
                  author tier; the rest carry labeller=llm-adjudicated-2panel.
    Later layers never override earlier ones; duplicate keys keep first layer.
    Re-run as author verdicts land — the fixture grows monotonically."""
    fields = ["family", "file_path", "file_content_sha256", "column_name",
              "curated_label", "ydf_context", "sense_context", "labeller", "provenance"]
    out_rows: list[dict] = []
    seen: set[tuple[str, str]] = set()

    def add(row: dict) -> None:
        key = (row["file_content_sha256"], row["column_name"])
        if key in seen or not row.get("curated_label"):
            return
        seen.add(key)
        out_rows.append({f: row.get(f, "") for f in fields})

    for r in load_gold(args.anchor):
        add(r)
    n_anchor = len(out_rows)
    if args.lens_labels.exists():
        for r in load_gold(args.lens_labels):
            if r.get("verdict") != "consensus":
                continue
            add({**r, "family": r.get("tier", "") + ":" + r.get("stratum_label", ""),
                 "curated_label": r.get("consensus_label", ""),
                 "ydf_context": r.get("ydf_context", ""),
                 "sense_context": r.get("sense_context", ""),
                 "labeller": "lens-consensus",
                 "provenance": "ac-03 " + r.get("verdict_basis", "")})
    n_consensus = len(out_rows) - n_anchor
    # llm tier (two-panel double-confirmed; author-accepted 2026-06-10)
    n_llm = 0
    if args.llm.exists():
        spot_ok: set[int] = set()
        if args.spotcheck.exists():
            with args.spotcheck.open() as fh:
                for r in csv.DictReader(fh):
                    vkey = next((k for k in r if k.startswith("author_verdict")), None)
                    if vkey and (r[vkey] or "").strip().lower() in ("ok", "yes", "y"):
                        spot_ok.add(int(r["row_index"]))
        # sha -> file_path so external rows keep their vendored-CSV path
        paths: dict[str, str] = {}
        for cand in (Path("eval/gold/gold_corpus_candidates.tsv"),
                     Path("eval/gold/gold_corpus_candidates_external.tsv")):
            if cand.exists():
                for r in load_gold(cand):
                    paths.setdefault(r["file_content_sha256"], r["file_path"])
        for r in load_gold(args.llm):
            if r.get("status") != "llm-adjudicated" or not r.get("final_label"):
                continue
            human = int(r["row_index"]) in spot_ok
            add({"family": "llm:" + r.get("stratum", ""),
                 "file_path": paths.get(r["file_content_sha256"], ""),
                 "file_content_sha256": r["file_content_sha256"],
                 "column_name": r["column_name"],
                 "curated_label": r["final_label"],
                 "ydf_context": "", "sense_context": "",
                 "labeller": ("author-adjudicated" if human
                              else "llm-adjudicated-2panel"),
                 "provenance": ("ac-04 two-panel adjudication 2026-06-10; "
                                + ("author spot-check ok" if human
                                   else "author-accepted tier (40/40 spot-check)"))})
            n_llm += 1
    # open-row resolutions (panel splits/unsures the author settled directly)
    n_open = 0
    if args.open_rows.exists():
        with args.open_rows.open() as fh:
            for r in csv.DictReader(fh):
                lkey = next((k for k in r if k.startswith("your_label")), None)
                label = (r.get(lkey) or "").strip() if lkey else ""
                if not label or label.lower() == "skip":
                    continue
                add({"family": "author-open:" ,
                     "file_path": "", "file_content_sha256": "", "column_name": "",
                     **_open_row_identity(r), "curated_label": label,
                     "ydf_context": "", "sense_context": "",
                     "labeller": "author-adjudicated",
                     "provenance": "ac-04 open-row resolution 2026-06-10"})
                n_open += 1
    n_adj = 0
    if args.review.exists():
        with args.review.open() as fh:
            for r in csv.DictReader(fh):
                verdict = (r.get("human_verdict") or "").strip().lower()
                if verdict not in ("correct", "wrong"):
                    continue
                label = (r.get("proposed_label") if verdict == "correct"
                         else r.get("correct_label_if_wrong") or "").strip()
                if not label:
                    continue
                add({"family": "adjudicated:" + r.get("stratum", ""),
                     "file_path": "", "file_content_sha256": r["file_content_sha256"],
                     "column_name": r["column_name"], "curated_label": label,
                     "ydf_context": "", "sense_context": "",
                     "labeller": "author-adjudicated",
                     "provenance": "ac-04 sitting; " + (r.get("note") or "")})
                n_adj += 1
    args.out.parent.mkdir(parents=True, exist_ok=True)
    with args.out.open("w", newline="") as fh:
        w = csv.DictWriter(fh, fieldnames=fields, delimiter="\t", lineterminator="\n")
        w.writeheader()
        w.writerows(out_rows)
    print(f"gold v1: {len(out_rows)} columns (anchor {n_anchor}, consensus "
          f"{n_consensus}, llm-tier {n_llm}, open-rows {n_open}, "
          f"adjudicated {n_adj}) -> {args.out}", file=sys.stderr)
    return 0


def _vendored_values(file_path: str, column_name: str, cap: int = 100) -> list[str]:
    """External gold columns live in vendored CSVs, not the corpus parquet."""
    p = Path(file_path)
    if not file_path or not p.exists() or p.suffix.lower() not in (".csv", ".dat"):
        return []
    vals: list[str] = []
    with p.open(newline="", encoding="utf-8", errors="replace") as fh:
        for row in csv.DictReader(fh):
            v = (row.get(column_name) or "").strip()
            if v:
                vals.append(v)
            if len(vals) >= cap:
                break
    return vals


def cmd_predict(args: argparse.Namespace) -> int:
    import pyarrow.parquet as pq

    gold = load_gold(args.gold)
    wanted = {(r["file_content_sha256"], r["column_name"]) for r in gold}
    print(f"{len(wanted)} gold columns to predict", file=sys.stderr)

    # Pull sample values for exactly the gold columns from the corpus parquet.
    tbl = pq.read_table(
        args.columns,
        columns=["file_content_sha256", "column_name", "sample_values_truncated"],
    )
    samples: dict[tuple[str, str], list[str]] = {}
    for r in tbl.to_pylist():
        key = (r.get("file_content_sha256") or "", r.get("column_name") or "")
        if key in wanted and key not in samples:
            raw = r.get("sample_values_truncated") or ""
            samples[key] = [v for v in raw.split(SEP) if v != ""]

    args.out.parent.mkdir(parents=True, exist_ok=True)
    n_ok = n_err = 0
    with args.out.open("w") as fh:
        w = csv.writer(fh, delimiter="\t", lineterminator="\n")
        w.writerow(["file_content_sha256", "column_name", "predicted_label", "confidence"])
        for i, r in enumerate(gold, 1):
            key = (r["file_content_sha256"], r["column_name"])
            vals = samples.get(key)
            if not vals:
                vals = _vendored_values(r.get("file_path", ""), r["column_name"])
            if not vals:
                n_err += 1
                continue
            try:
                label = _profile_column(args.binary, r["column_name"], vals)
            except Exception as e:  # noqa: BLE001
                print(f"  row {i}: {e}", file=sys.stderr)
                n_err += 1
                continue
            w.writerow([r["file_content_sha256"], r["column_name"], label, ""])
            n_ok += 1
            if i % 50 == 0:
                print(f"  ... {i}/{len(gold)}", file=sys.stderr)
    print(f"predicted {n_ok} columns ({n_err} missing/err) -> {args.out}", file=sys.stderr)
    return 0


# The text residual under the enum reframe (spec 2026-06-17-enum-accuracy-reframe,
# choice 0102): `categorical` is retired as a competing leaf — it, `word`, and
# `plain_text` are one untyped-bounded-text residual, scored as a single class.
# `entity_name` is a real semantic type and stays DISTINCT (it is not the residual).
# enum-ness itself moves to the orthogonal x-finetype-enum property (shipped 0.6.33),
# which carries bounded-domain info and is NOT part of this semantic-type headline
# (no enum ground truth exists — see ac00-finding.md).
REFRAME_RESIDUAL = {
    "representation.discrete.categorical",
    "representation.text.word",
    "representation.text.plain_text",
}
REFRAME_RESIDUAL_LABEL = "representation.text.RESIDUAL"


def _reframe(label: str) -> str:
    return REFRAME_RESIDUAL_LABEL if label in REFRAME_RESIDUAL else label


def cmd_score(args: argparse.Namespace) -> int:
    gold = load_gold(args.gold)
    reframe = getattr(args, "reframe", False)
    norm = _reframe if reframe else (lambda x: x)
    preds: dict[tuple[str, str], str] = {}
    with args.predictions.open() as fh:
        for r in csv.DictReader(fh, delimiter="\t"):
            preds[(r["file_content_sha256"], r["column_name"])] = r["predicted_label"]

    # family -> list of (curated, predicted)
    pairs: dict[str, list[tuple[str, str]]] = defaultdict(list)
    n_unpredicted = 0
    for r in gold:
        key = (r["file_content_sha256"], r["column_name"])
        pred = preds.get(key)
        if pred is None:
            n_unpredicted += 1
            continue
        pairs[r["family"]].append((norm(r["curated_label"]), norm(pred)))

    def wilson(k: int, n: int) -> tuple[float, float]:
        """95% Wilson interval for k/n; returns (lo, hi). (nan, nan) if n=0."""
        if n == 0:
            return float("nan"), float("nan")
        z = 1.96
        ph = k / n
        denom = 1 + z * z / n
        centre = (ph + z * z / (2 * n)) / denom
        half = z * ((ph * (1 - ph) / n + z * z / (4 * n * n)) ** 0.5) / denom
        return max(0.0, centre - half), min(1.0, centre + half)

    def prf(curated_pred: list[tuple[str, str]], label: str) -> tuple[int, int, int, float, float]:
        tp = sum(1 for c, p in curated_pred if c == label and p == label)
        fp = sum(1 for c, p in curated_pred if c != label and p == label)
        fn = sum(1 for c, p in curated_pred if c == label and p != label)
        precision = tp / (tp + fp) if (tp + fp) else float("nan")
        recall = tp / (tp + fn) if (tp + fn) else float("nan")
        return tp, fp, fn, precision, recall

    args.out_dir.mkdir(parents=True, exist_ok=True)
    stamp = dt.date.today().isoformat()
    tsv_path = args.out_dir / f"metrics_{args.model_name}_{stamp}.tsv"
    md_path = args.out_dir / f"report_{args.model_name}_{stamp}.md"

    all_pairs: list[tuple[str, str]] = [t for cp in pairs.values() for t in cp]
    rows_out: list[dict] = []
    fam_acc: dict[str, float] = {}
    for family in sorted(pairs):
        cp = pairs[family]
        fam_acc[family] = sum(1 for c, p in cp if c == p) / len(cp)
    # Per-label metrics over the WHOLE scored set: a false positive counts
    # against a label no matter which family the column was drawn for.
    for label in sorted({c for c, _ in all_pairs}):
        tp, fp, fn, pr, rc = prf(all_pairs, label)
        plo, phi = wilson(tp, tp + fp)
        rlo, rhi = wilson(tp, tp + fn)
        rows_out.append(
            {
                "family": "(global)",
                "label": label,
                "support": tp + fn,
                "tp": tp,
                "fp": fp,
                "fn": fn,
                "precision": pr,
                "recall": rc,
                "precision_ci": f"{plo:.2f}-{phi:.2f}" if plo == plo else "n/a",
                "recall_ci": f"{rlo:.2f}-{rhi:.2f}" if rlo == rlo else "n/a",
            }
        )

    with tsv_path.open("w") as fh:
        w = csv.DictWriter(
            fh,
            fieldnames=["family", "label", "support", "tp", "fp", "fn",
                        "precision", "recall", "precision_ci", "recall_ci"],
            delimiter="\t",
            lineterminator="\n",
        )
        w.writeheader()
        for row in rows_out:
            row = dict(row)
            row["precision"] = f'{row["precision"]:.3f}'
            row["recall"] = f'{row["recall"]:.3f}'
            w.writerow(row)

    def fmt(x: float) -> str:
        return "n/a" if x != x else f"{x:.3f}"  # x!=x is NaN

    lines = [
        f"# Gold eval anchor — {args.model_name}",
        "",
        f"**Date:** {stamp}  ",
        f"**Gold fixture:** `{args.gold}` ({len(gold)} columns)  ",
        f"**Scoring mode:** {'ENUM REFRAME (categorical/word/plain_text = one text residual)' if reframe else 'legacy (categorical is a competing label)'}  ",
        f"**Predictions:** `{args.predictions}`  ",
        f"**Scored:** {sum(len(v) for v in pairs.values())} columns "
        f"({n_unpredicted} gold columns had no prediction)  ",
        "",
        "Per-family accuracy (fraction of columns where the model's prediction "
        "equals the curated gold label — labels neither lens produced):",
        "",
        "| Family | Columns | Accuracy |",
        "|--------|--------:|---------:|",
    ]
    for family in sorted(pairs):
        lines.append(f"| {family} | {len(pairs[family])} | {fam_acc[family]:.3f} |")
    lines += [
        "",
        "Per-label precision/recall (the curated label is ground truth; YDF is "
        "not consulted):",
        "",
        "| Curated label | Support | TP | FP | FN | Precision (95% CI) | Recall (95% CI) |",
        "|---------------|--------:|---:|---:|---:|-------------------:|----------------:|",
    ]
    for row in rows_out:
        lines.append(
            f"| {row['label']} | {row['support']} | {row['tp']} | "
            f"{row['fp']} | {row['fn']} | {fmt(row['precision'])} ({row.get('precision_ci', 'n/a')}) "
            f"| {fmt(row['recall'])} ({row.get('recall_ci', 'n/a')}) |"
        )
    macro_p = [r["precision"] for r in rows_out if r["precision"] == r["precision"]]
    macro_r = [r["recall"] for r in rows_out if r["recall"] == r["recall"]]
    n_correct = sum(1 for c, pr2 in all_pairs if c == pr2)
    alo, ahi = wilson(n_correct, len(all_pairs))
    lines += [
        "",
        f"**Headline — column accuracy:** {n_correct}/{len(all_pairs)} = "
        f"{n_correct/len(all_pairs):.3f} (95% CI {alo:.3f}-{ahi:.3f})  ",
        f"**Macro precision** (mean over labels): {sum(macro_p)/len(macro_p):.3f}  ",
        f"**Macro recall** (mean over labels): {sum(macro_r)/len(macro_r):.3f}  ",
    ]
    md_path.write_text("\n".join(lines) + "\n")
    print(f"wrote {tsv_path}\nwrote {md_path}", file=sys.stderr)
    print("\n".join(lines))
    return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    sub = ap.add_subparsers(dest="mode", required=True)

    b = sub.add_parser("build-gold")
    b.add_argument("--anchor", type=Path,
                   default=Path("eval/gold/gold_eval_anchor.tsv"))
    b.add_argument("--lens-labels", type=Path,
                   default=Path("eval/gold/gold_lens_labels.tsv"))
    b.add_argument("--review", type=Path,
                   default=Path("eval/gold/gold_review_queue_v1_review.csv"))
    b.add_argument("--llm", type=Path,
                   default=Path("eval/gold/gold_review_queue_v1_llm.tsv"))
    b.add_argument("--spotcheck", type=Path,
                   default=Path("eval/gold/gold_spotcheck_author.csv"))
    b.add_argument("--open-rows", dest="open_rows", type=Path,
                   default=Path("eval/gold/gold_open_rows_author.csv"))
    b.add_argument("--out", type=Path,
                   default=Path("eval/gold/gold_corpus.tsv"))
    b.set_defaults(func=cmd_build_gold)

    p = sub.add_parser("predict")
    p.add_argument("--gold", required=True, type=Path)
    p.add_argument("--columns", required=True, type=Path)
    p.add_argument("--binary", required=True, type=Path)
    p.add_argument("--out", required=True, type=Path)
    p.set_defaults(func=cmd_predict)

    s = sub.add_parser("score")
    s.add_argument("--gold", required=True, type=Path)
    s.add_argument("--predictions", required=True, type=Path)
    s.add_argument("--model-name", required=True)
    s.add_argument("--out-dir", required=True, type=Path)
    s.add_argument("--reframe", action="store_true",
                   help="Enum reframe (spec 2026-06-17-enum-accuracy-reframe / choice 0102): "
                        "score the semantic type with {categorical, word, plain_text} collapsed "
                        "to one text residual (entity_name stays distinct); enum-ness is the "
                        "orthogonal x-finetype-enum property, not a competing label. Headline "
                        "becomes invariant to the categorical-vs-residual boundary on both sides.")
    s.set_defaults(func=cmd_score)

    args = ap.parse_args()
    return args.func(args)


if __name__ == "__main__":
    raise SystemExit(main())
