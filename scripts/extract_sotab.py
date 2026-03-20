#!/usr/bin/env python3
"""Extract SOTAB V2 CTA columns into JSONL for FineType distillation v3.

Reads SOTAB V2 nested ZIPs (tables + ground truth) and produces a JSONL file
with one record per annotated column containing sample values and Schema.org
ground truth labels.

Usage:
    python3 scripts/extract_sotab.py [--round 2] [--dest output/distillation-v3/] \
                                     [--validate-only] [--max-tables N]

SOTAB V2 table format: Each JSON.gz file contains JSONL rows. Each row is a
dict with string column indices ("0", "1", ...) as keys and cell values as
values. There are no column headers in the data.

Ground truth CSVs map: table_name, column_index, label (Schema.org types like
"Date", "Person/name", "Country", etc.)

Data location: ~/datasets/sotab/cta/SOTAB V2 for SemTab 2023/
"""

import argparse
import csv
import gzip
import io
import json
import os
import sys
import zipfile
from collections import Counter


# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------

SOTAB_BASE = os.path.expanduser(
    "~/datasets/sotab/cta/SOTAB V2 for SemTab 2023"
)

ROUND_CONFIG = {
    1: {
        "tables_zip": "Round1-SOTAB-CTA-SCH-Tables.zip",
        "gt_zip": "Round1-SOTAB-CTA-DatasetsAndGroundTruth.zip",
        "gt_files": {
            "train": "sotab_cta_train.csv",
            "validation": "sotab_cta_validation.csv",
            "test": "gt/sotab_cta_test.csv",
        },
        "output_name": "sotab_round1.jsonl",
    },
    2: {
        "tables_zip": "Round2-SOTAB-CTA-Tables.zip",
        "gt_zip": "Round2-SOTAB-CTA-SCH-DatasetsAndGroundTruth.zip",
        "gt_files": {
            "train": "sotab_cta_train_round2.csv",
            "validation": "sotab_cta_validation_round2.csv",
            "test": "gt/sotab_cta_test_round2.csv",
        },
        "output_name": "sotab_round2.jsonl",
    },
}


# ---------------------------------------------------------------------------
# Ground truth loading
# ---------------------------------------------------------------------------


def load_ground_truth(gt_zip_path, gt_files):
    """Load all ground truth CSVs from the nested ZIP.

    Returns dict: (table_name, column_index) -> label
    """
    gt = {}
    label_counts = Counter()

    with zipfile.ZipFile(gt_zip_path, "r") as zf:
        for split_name, csv_name in gt_files.items():
            try:
                data = zf.read(csv_name).decode("utf-8")
            except KeyError:
                print(f"  Warning: {csv_name} not found in {gt_zip_path}", file=sys.stderr)
                continue

            reader = csv.DictReader(io.StringIO(data))
            count = 0
            for row in reader:
                table_name = row["table_name"].strip()
                col_idx = int(row["column_index"].strip())
                label = row["label"].strip()
                gt[(table_name, col_idx)] = label
                label_counts[label] += 1
                count += 1
            print(f"  Loaded {count} annotations from {split_name} ({csv_name})")

    return gt, label_counts


# ---------------------------------------------------------------------------
# Table extraction
# ---------------------------------------------------------------------------


def extract_table_columns(content_bytes, max_values=20):
    """Extract columns from a SOTAB JSON.gz table.

    Each JSON.gz contains JSONL: one JSON object per row, with string column
    indices ("0", "1", ...) as keys.

    Returns: dict of {col_index: {"values": [...], "num_rows": int}}
    """
    try:
        text = gzip.decompress(content_bytes).decode("utf-8", errors="replace")
    except Exception as e:
        return None, f"gzip error: {e}"

    lines = text.strip().split("\n")
    if not lines:
        return None, "empty file"

    # Parse first line to discover columns
    try:
        first = json.loads(lines[0])
    except json.JSONDecodeError as e:
        return None, f"JSON parse error: {e}"

    if not isinstance(first, dict):
        return None, f"unexpected row type: {type(first).__name__}"

    # Determine column indices (sorted numerically)
    col_keys = sorted(first.keys(), key=lambda k: int(k) if k.isdigit() else k)

    columns = {int(k) if k.isdigit() else k: [] for k in col_keys}
    num_rows = 0

    for line in lines:
        if not line.strip():
            continue
        try:
            row = json.loads(line)
        except json.JSONDecodeError:
            continue

        num_rows += 1
        for k in col_keys:
            col_idx = int(k) if k.isdigit() else k
            val = row.get(k)
            if val is not None and val != "None" and val != "N/A" and str(val).strip():
                vals = columns[col_idx]
                if len(vals) < max_values:
                    vals.append(str(val).strip())

    return {
        idx: {"values": vals, "num_rows": num_rows}
        for idx, vals in columns.items()
    }, None


# ---------------------------------------------------------------------------
# Main extraction
# ---------------------------------------------------------------------------


def run_extraction(args):
    round_num = args.round
    if round_num not in ROUND_CONFIG:
        print(f"Error: Round {round_num} not configured. Available: {list(ROUND_CONFIG.keys())}")
        sys.exit(1)

    config = ROUND_CONFIG[round_num]
    tables_zip_path = os.path.join(SOTAB_BASE, config["tables_zip"])
    gt_zip_path = os.path.join(SOTAB_BASE, config["gt_zip"])

    for path in [tables_zip_path, gt_zip_path]:
        if not os.path.exists(path):
            print(f"Error: {path} not found")
            sys.exit(1)

    # Step 1: Load ground truth
    print(f"Loading Round {round_num} ground truth from {os.path.basename(gt_zip_path)}...")
    gt, label_counts = load_ground_truth(gt_zip_path, config["gt_files"])
    print(f"  Total annotations: {len(gt)}")
    print(f"  Unique labels: {len(label_counts)}")

    # Step 2: Open tables ZIP and enumerate JSON.gz files
    print(f"\nOpening tables archive: {os.path.basename(tables_zip_path)}...")
    tables_zf = zipfile.ZipFile(tables_zip_path, "r")
    table_names = [
        n for n in tables_zf.namelist()
        if n.endswith(".json.gz")
    ]
    print(f"  Found {len(table_names)} table files")

    if args.max_tables:
        table_names = table_names[: args.max_tables]
        print(f"  Limited to {len(table_names)} tables (--max-tables)")

    # Step 3: Validate on first 100 tables
    validate_count = min(100, len(table_names))
    print(f"\n--- Validation pass ({validate_count} tables) ---")

    valid_records = 0
    missing_gt = 0
    empty_values = 0
    parse_errors = 0

    for name in table_names[:validate_count]:
        basename = os.path.basename(name)
        content = tables_zf.read(name)
        columns, err = extract_table_columns(content)

        if err or columns is None:
            parse_errors += 1
            continue

        for col_idx, col_data in columns.items():
            if not isinstance(col_idx, int):
                continue
            key = (basename, col_idx)
            if key in gt:
                if col_data["values"]:
                    valid_records += 1
                else:
                    empty_values += 1
            else:
                missing_gt += 1

    total_checked = valid_records + missing_gt + empty_values
    print(f"  Valid records (values + GT): {valid_records}")
    print(f"  Columns without GT label:    {missing_gt}")
    print(f"  Columns with empty values:   {empty_values}")
    print(f"  Tables with parse errors:    {parse_errors}")
    print(f"  Total columns checked:       {total_checked}")

    if total_checked > 0 and valid_records / max(total_checked, 1) < 0.1:
        print(
            f"\nError: Only {valid_records}/{total_checked} columns have both "
            "values and ground truth. Aborting — check data format.",
            file=sys.stderr,
        )
        sys.exit(1)

    if args.validate_only:
        print("\n--validate-only: stopping after validation pass.")
        tables_zf.close()
        return

    # Step 4: Full extraction
    dest = args.dest
    os.makedirs(dest, exist_ok=True)
    output_path = os.path.join(dest, config["output_name"])

    print(f"\n--- Full extraction → {output_path} ---")

    total_tables = 0
    total_columns = 0
    columns_with_gt = 0
    columns_without_gt = 0
    error_tables = 0
    gt_labels_seen = Counter()

    with open(output_path, "w", encoding="utf-8") as out:
        for i, name in enumerate(table_names):
            basename = os.path.basename(name)

            if (i + 1) % 1000 == 0:
                print(f"  Progress: {i + 1}/{len(table_names)} tables...")

            content = tables_zf.read(name)
            columns, err = extract_table_columns(content)

            if err or columns is None:
                error_tables += 1
                continue

            total_tables += 1

            for col_idx, col_data in sorted(columns.items()):
                if not isinstance(col_idx, int):
                    continue

                values = col_data["values"]
                if not values:
                    continue

                total_columns += 1
                key = (basename, col_idx)
                label = gt.get(key)

                if label:
                    columns_with_gt += 1
                    gt_labels_seen[label] += 1
                else:
                    columns_without_gt += 1

                record = {
                    "source": "sotab",
                    "source_file": basename,
                    "column_index": col_idx,
                    "column_name": None,  # SOTAB has no column headers
                    "values": values,
                    "ground_truth_label": label,
                    "ground_truth_source": "sotab" if label else None,
                }
                out.write(json.dumps(record, ensure_ascii=False) + "\n")

    tables_zf.close()

    # Step 5: Summary
    print(f"\n--- Summary ---")
    print(f"  Tables processed:       {total_tables}")
    print(f"  Tables with errors:     {error_tables}")
    print(f"  Total columns (values): {total_columns}")
    print(f"  Columns with GT:        {columns_with_gt}")
    print(f"  Columns without GT:     {columns_without_gt}")
    print(f"  Unique GT labels seen:  {len(gt_labels_seen)}")
    print(f"  Output: {output_path}")

    # Top labels
    print(f"\n  Top 20 ground truth labels:")
    for label, count in gt_labels_seen.most_common(20):
        print(f"    {label:40s} {count:6d}")


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------


def main():
    parser = argparse.ArgumentParser(
        description="Extract SOTAB V2 CTA columns into JSONL for distillation."
    )
    parser.add_argument(
        "--round",
        type=int,
        default=2,
        help="SOTAB round (default: 2)",
    )
    parser.add_argument(
        "--dest",
        default="output/distillation-v3/",
        help="Output directory (default: output/distillation-v3/)",
    )
    parser.add_argument(
        "--validate-only",
        action="store_true",
        help="Run validation on first 100 tables only, then stop",
    )
    parser.add_argument(
        "--max-tables",
        type=int,
        default=0,
        help="Max tables to process (0 = all)",
    )
    args = parser.parse_args()

    if args.max_tables == 0:
        args.max_tables = None

    run_extraction(args)


if __name__ == "__main__":
    main()
