#!/usr/bin/env python3
"""Extract column values from SOTAB CTA tables for FineType evaluation.

Reads SOTAB table files (gzipped JSON, one row per line) and ground truth CSV,
samples up to N non-null values per annotated column, and writes a parquet file
for DuckDB classification.

Usage:
    python3 eval/sotab/prepare_values.py --split validation
    python3 eval/sotab/prepare_values.py --split test --gt-file CTA_test_gt.csv
"""
import argparse
import csv
import gzip
import json
import os
import random
import sys
from collections import defaultdict
from pathlib import Path

random.seed(42)
SAMPLE_VALUES_PER_COL = 20
MAX_VALUE_LEN = 500
DEFAULT_SOTAB_DIR = Path(os.environ.get("SOTAB_DIR", os.path.expanduser("~/datasets/sotab/cta")))


def load_ground_truth(gt_path: Path) -> dict[str, list[tuple[int, str]]]:
    """Load ground truth CSV → {table_name: [(col_index, label), ...]}"""
    gt = defaultdict(list)
    with open(gt_path) as f:
        for row in csv.DictReader(f):
            gt[row["table_name"]].append((int(row["column_index"]), row["label"]))
    return gt


def extract_table_values(
    table_path: Path, annotated_cols: list[tuple[int, str]]
) -> list[dict]:
    """Extract sampled values from a single SOTAB JSON table."""
    # Collect all values per column index
    col_values: dict[int, list[str]] = defaultdict(list)
    annotated_indices = {idx for idx, _ in annotated_cols}

    try:
        with gzip.open(table_path, "rt", encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if not line:
                    continue
                try:
                    row = json.loads(line)
                except json.JSONDecodeError:
                    continue
                for key, value in row.items():
                    try:
                        col_idx = int(key)
                    except ValueError:
                        continue
                    if col_idx not in annotated_indices:
                        continue
                    if value is not None:
                        s = str(value).strip()
                        if 0 < len(s) < MAX_VALUE_LEN:
                            col_values[col_idx].append(s)
    except Exception:
        return []

    # Sample and build output rows
    rows = []
    table_name = table_path.name
    for col_idx, label in annotated_cols:
        values = col_values.get(col_idx, [])
        if len(values) > SAMPLE_VALUES_PER_COL:
            values = random.sample(values, SAMPLE_VALUES_PER_COL)
        for v in values:
            rows.append(
                {
                    "table_name": table_name,
                    "col_index": col_idx,
                    "gt_label": label,
                    "col_value": v,
                }
            )
    return rows


def main():
    parser = argparse.ArgumentParser(description="Extract SOTAB column values for evaluation.")
    parser.add_argument(
        "--sotab-dir",
        type=str,
        default=str(DEFAULT_SOTAB_DIR),
        help=f"SOTAB CTA data directory (default: {DEFAULT_SOTAB_DIR})",
    )
    parser.add_argument(
        "--split",
        type=str,
        default="validation",
        choices=["validation", "test"],
        help="Dataset split to process (default: validation)",
    )
    parser.add_argument(
        "--gt-file",
        type=str,
        default=None,
        help="Ground truth CSV filename (default: auto-detect from split)",
    )
    parser.add_argument(
        "--output",
        type=str,
        default=None,
        help="Output parquet path (default: {sotab-dir}/{split}/column_values.parquet)",
    )
    args = parser.parse_args()

    try:
        import pyarrow as pa
        import pyarrow.parquet as pq
    except ImportError:
        print("Need pyarrow: pip install pyarrow")
        sys.exit(1)

    sotab_dir = Path(args.sotab_dir)
    split = args.split

    # Locate tables directory
    if split == "validation":
        tables_dir = sotab_dir / "validation" / "Validation"
        default_gt = "CTA_validation_gt.csv"
        gt_dir = sotab_dir / "validation"
    else:
        tables_dir = sotab_dir / "test" / "Test"
        default_gt = "CTA_test_gt.csv"
        gt_dir = sotab_dir / "test"

    gt_file = args.gt_file or default_gt
    gt_path = gt_dir / gt_file

    if not tables_dir.exists():
        print(f"Tables directory not found: {tables_dir}")
        sys.exit(1)
    if not gt_path.exists():
        print(f"Ground truth not found: {gt_path}")
        sys.exit(1)

    output_path = Path(args.output) if args.output else gt_dir / "column_values.parquet"

    # Load ground truth
    gt = load_ground_truth(gt_path)
    print(f"Ground truth: {gt_path.name}")
    print(f"  Tables with annotations: {len(gt)}")
    print(f"  Total annotated columns: {sum(len(v) for v in gt.values())}")
    print(f"  Unique labels: {len(set(label for cols in gt.values() for _, label in cols))}")

    # Process tables
    all_rows = []
    found = 0
    missing = 0
    for i, (table_name, cols) in enumerate(sorted(gt.items())):
        if i % 500 == 0:
            print(f"  {i}/{len(gt)} tables processed, {len(all_rows)} values collected")

        table_path = tables_dir / table_name
        if not table_path.exists():
            missing += 1
            continue
        found += 1

        rows = extract_table_values(table_path, cols)
        all_rows.extend(rows)

    print(f"  {len(gt)}/{len(gt)} tables processed, {len(all_rows)} values collected")
    print(f"  Found: {found}, Missing: {missing}")

    if not all_rows:
        print("No values extracted!")
        sys.exit(1)

    # Write parquet
    out_table = pa.table(
        {
            "table_name": [r["table_name"] for r in all_rows],
            "col_index": [r["col_index"] for r in all_rows],
            "gt_label": [r["gt_label"] for r in all_rows],
            "col_value": [r["col_value"] for r in all_rows],
        }
    )

    pq.write_table(out_table, output_path)
    print(f"\nOutput: {output_path}")
    print(f"  Rows: {len(all_rows)}")
    print(
        f"  Columns: {len(set((r['table_name'], r['col_index']) for r in all_rows))}"
    )
    print(f"  Labels: {len(set(r['gt_label'] for r in all_rows))}")


if __name__ == "__main__":
    main()
