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


def cmd_score(args: argparse.Namespace) -> int:
    gold = load_gold(args.gold)
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
        pairs[r["family"]].append((r["curated_label"], pred))

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

    rows_out: list[dict] = []
    fam_acc: dict[str, float] = {}
    for family in sorted(pairs):
        cp = pairs[family]
        labels = sorted({c for c, _ in cp})
        acc = sum(1 for c, p in cp if c == p) / len(cp)
        fam_acc[family] = acc
        for label in labels:
            tp, fp, fn, pr, rc = prf(cp, label)
            rows_out.append(
                {
                    "family": family,
                    "label": label,
                    "support": tp + fn,
                    "tp": tp,
                    "fp": fp,
                    "fn": fn,
                    "precision": pr,
                    "recall": rc,
                }
            )

    with tsv_path.open("w") as fh:
        w = csv.DictWriter(
            fh,
            fieldnames=["family", "label", "support", "tp", "fp", "fn", "precision", "recall"],
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
        "| Family | Curated label | Support | TP | FP | FN | Precision | Recall |",
        "|--------|---------------|--------:|---:|---:|---:|----------:|-------:|",
    ]
    for row in rows_out:
        lines.append(
            f"| {row['family']} | {row['label']} | {row['support']} | {row['tp']} | "
            f"{row['fp']} | {row['fn']} | {fmt(row['precision'])} | {fmt(row['recall'])} |"
        )
    macro_p = [r["precision"] for r in rows_out if r["precision"] == r["precision"]]
    macro_r = [r["recall"] for r in rows_out if r["recall"] == r["recall"]]
    lines += [
        "",
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
    s.set_defaults(func=cmd_score)

    args = ap.parse_args()
    return args.func(args)


if __name__ == "__main__":
    raise SystemExit(main())
