#!/usr/bin/env python3
"""Gold-corpus FP/FN analysis helper.

Joins a predictions TSV against the human-verified gold fixture and reports,
for one or more target labels, the false positives (pred==target, gold!=target)
grouped by gold label, and the misses/FNs (gold==target, pred!=target) grouped
by predicted label.

THE LOAD-BEARING INVARIANT: file_content_sha256 is PER-FILE, not per-column.
A file's many columns share one sha. Joining predictions to gold on sha ALONE
mispairs a prediction with an arbitrary other column's gold label in the same
file and produces a corrupted FP breakdown. The join key is ALWAYS the
composite (file_content_sha256, column_name). This script exists because that
join was hand-rolled wrong three times.

Run under the project venv (pyarrow lives there, not in system python):

    eval/gittables/.venv/bin/python scripts/gold_fp_analysis.py \
        --predictions output/housekeeping-baseline/predictions_binveto.tsv \
        --label representation.numeric.integer_number --values

Dependencies: stdlib csv + (only when --values) pyarrow.
"""
from __future__ import annotations

import argparse
import csv
import sys
from collections import Counter

DEFAULT_GOLD = "eval/gold/gold_corpus_v1.tsv"
DEFAULT_PARQUET = "eval/gittables/corpus_pass/columns.parquet"


def load_predictions(path: str) -> dict[tuple[str, str], str]:
    """Map (sha, column_name) -> predicted_label.

    Expects header columns file_content_sha256, column_name, predicted_label
    (confidence optional). Resolves columns by header name, not position.
    """
    preds: dict[tuple[str, str], str] = {}
    with open(path, newline="") as f:
        reader = csv.DictReader(f, delimiter="\t")
        required = {"file_content_sha256", "column_name", "predicted_label"}
        missing = required - set(reader.fieldnames or [])
        if missing:
            sys.exit(
                f"predictions file {path} missing columns: {sorted(missing)} "
                f"(has {reader.fieldnames})"
            )
        for row in reader:
            key = (row["file_content_sha256"], row["column_name"])
            preds[key] = row["predicted_label"]
    return preds


def load_gold(path: str) -> dict[tuple[str, str], str]:
    """Map (sha, column_name) -> curated gold label.

    Gold header: file_content_sha256, column_name (the column header),
    curated_label. Resolved by name, not position.
    """
    gold: dict[tuple[str, str], str] = {}
    with open(path, newline="") as f:
        reader = csv.DictReader(f, delimiter="\t")
        fields = set(reader.fieldnames or [])
        if "file_content_sha256" not in fields or "column_name" not in fields:
            sys.exit(
                f"gold file {path} missing key columns; has {reader.fieldnames}"
            )
        label_col = "curated_label" if "curated_label" in fields else None
        if label_col is None:
            sys.exit(
                f"gold file {path} has no curated_label column; "
                f"has {reader.fieldnames}"
            )
        for row in reader:
            key = (row["file_content_sha256"], row["column_name"])
            gold[key] = row[label_col]
    return gold


def load_values(parquet_path: str, keys: set[tuple[str, str]]) -> dict[tuple[str, str], str]:
    """Pull sample_values_truncated for the requested (sha, column_name) keys."""
    import pyarrow.parquet as pq

    wanted = set(keys)
    out: dict[tuple[str, str], str] = {}
    pf = pq.ParquetFile(parquet_path)
    cols = ["file_content_sha256", "column_name", "sample_values_truncated"]
    for batch in pf.iter_batches(columns=cols, batch_size=65536):
        shas = batch.column("file_content_sha256").to_pylist()
        names = batch.column("column_name").to_pylist()
        vals = batch.column("sample_values_truncated").to_pylist()
        for sha, name, val in zip(shas, names, vals):
            key = (sha, name)
            if key in wanted and key not in out:
                out[key] = val
        if len(out) == len(wanted):
            break
    return out


def joined_rows(
    preds: dict[tuple[str, str], str], gold: dict[tuple[str, str], str]
) -> list[tuple[tuple[str, str], str, str]]:
    """Inner join on the composite key. Returns (key, pred, gold) tuples."""
    rows = []
    for key, pred in preds.items():
        if key in gold:
            rows.append((key, pred, gold[key]))
    return rows


def report_label(
    rows: list[tuple[tuple[str, str], str, str]],
    target: str,
    values: dict[tuple[str, str], str] | None,
) -> None:
    fps = [(k, p, g) for (k, p, g) in rows if p == target and g != target]
    fns = [(k, p, g) for (k, p, g) in rows if g == target and p != target]
    tps = sum(1 for (_, p, g) in rows if p == target and g == target)

    print(f"\n=== {target} ===")
    print(f"join: {len(rows)} columns matched on (sha, column_name)")
    print(f"TP={tps}  FP={len(fps)}  FN={len(fns)}")

    print(f"\nFALSE POSITIVES (pred={target}, gold!=target) by gold label:")
    if not fps:
        print("  (none)")
    for label, n in Counter(g for (_, _, g) in fps).most_common():
        print(f"  {n:5d}  gold={label}")

    print(f"\nMISSES / FN (gold={target}, pred!=target) by predicted label:")
    if not fns:
        print("  (none)")
    for label, n in Counter(p for (_, p, _) in fns).most_common():
        print(f"  {n:5d}  pred={label}")

    if values is not None:
        print("\nflagged columns (FP then FN):")
        for (k, p, g) in fps + fns:
            header = k[1]
            v = values.get(k, "<no values>")
            print(f"  {header} -> {p} (gold={g}) vals={v}")


def report_overall(rows: list[tuple[tuple[str, str], str, str]]) -> None:
    fp_by_pred: Counter[str] = Counter()
    fn_by_gold: Counter[str] = Counter()
    for (_, p, g) in rows:
        if p != g:
            fp_by_pred[p] += 1  # label over-emitted (asserted but wrong)
            fn_by_gold[g] += 1  # label missed (true but not predicted)

    print(f"\njoin: {len(rows)} columns matched on (sha, column_name)")
    print("\nWORST TYPES BY FALSE-POSITIVE COUNT (over-emitters):")
    for label, n in fp_by_pred.most_common(20):
        print(f"  {n:5d}  pred={label}")
    print("\nWORST TYPES BY MISS / FN COUNT (under-recalled):")
    for label, n in fn_by_gold.most_common(20):
        print(f"  {n:5d}  gold={label}")


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--predictions", required=True, help="predictions TSV")
    ap.add_argument("--gold", default=DEFAULT_GOLD, help=f"gold fixture (default {DEFAULT_GOLD})")
    ap.add_argument("--label", help="focus a single taxonomy label (e.g. geography.location.region)")
    ap.add_argument("--values", action="store_true", help="pull sample values from parquet for flagged columns")
    ap.add_argument("--parquet", default=DEFAULT_PARQUET, help=f"values parquet (default {DEFAULT_PARQUET})")
    args = ap.parse_args()

    preds = load_predictions(args.predictions)
    gold = load_gold(args.gold)
    rows = joined_rows(preds, gold)

    if not rows:
        sys.exit(
            "ERROR: zero rows after the composite join — predictions and gold "
            "share no (sha, column_name) keys. Check the inputs."
        )

    if args.label:
        keys_needed: set[tuple[str, str]] = set()
        if args.values:
            keys_needed = {
                k for (k, p, g) in rows
                if (p == args.label and g != args.label) or (g == args.label and p != args.label)
            }
        vals = load_values(args.parquet, keys_needed) if args.values else None
        report_label(rows, args.label, vals)
    else:
        report_overall(rows)


if __name__ == "__main__":
    main()
