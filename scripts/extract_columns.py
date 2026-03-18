#!/usr/bin/env python3
"""Extract columns from CSV files into compact JSONL for LLM classification.

Usage:
    python3 scripts/extract_columns.py <csv_dir> <output_jsonl> [--max-values N] [--max-files N]

Each line is a JSON object: {"file", "column", "values": [...], "row_count"}
Only columns with ≥3 non-empty values are included.
"""

import csv
import json
import os
import sys


def extract_columns(csv_path, max_values=20):
    """Extract column headers and sample values from a CSV file."""
    columns = []
    try:
        with open(csv_path, "r", encoding="utf-8", errors="replace") as f:
            reader = csv.DictReader(f)
            if not reader.fieldnames:
                return columns, 0

            col_values = {h: [] for h in reader.fieldnames}
            row_count = 0
            for row in reader:
                row_count += 1
                if row_count > 500:
                    break
                for h in reader.fieldnames:
                    val = row.get(h, "").strip()
                    if val and len(col_values[h]) < max_values:
                        col_values[h].append(val)

            for h in reader.fieldnames:
                if len(col_values[h]) >= 3:
                    columns.append({
                        "column": h,
                        "values": col_values[h][:max_values],
                    })
    except Exception as e:
        print(f"  Error reading {csv_path}: {e}", file=sys.stderr)

    return columns, row_count


def main():
    args = sys.argv[1:]
    config = {"max_values": 20, "max_files": 0}

    positional = []
    i = 0
    while i < len(args):
        if args[i] == "--max-values":
            config["max_values"] = int(args[i + 1])
            i += 2
        elif args[i] == "--max-files":
            config["max_files"] = int(args[i + 1])
            i += 2
        else:
            positional.append(args[i])
            i += 1

    if len(positional) < 2:
        print(__doc__)
        sys.exit(1)

    csv_dir = positional[0]
    output_jsonl = positional[1]

    csv_files = sorted(
        os.path.join(root, f)
        for root, _, files in os.walk(csv_dir)
        for f in files
        if f.endswith(".csv")
    )

    if config["max_files"] > 0:
        csv_files = csv_files[:config["max_files"]]

    print(f"Extracting columns from {len(csv_files)} CSV files...")

    total_columns = 0
    total_files = 0

    os.makedirs(os.path.dirname(output_jsonl) or ".", exist_ok=True)
    with open(output_jsonl, "w") as out:
        for csv_path in csv_files:
            file_name = os.path.basename(csv_path)
            columns, row_count = extract_columns(csv_path, config["max_values"])
            if columns:
                total_files += 1
                for col in columns:
                    record = {
                        "file": file_name,
                        "column": col["column"],
                        "values": col["values"],
                        "row_count": row_count,
                    }
                    out.write(json.dumps(record) + "\n")
                    total_columns += 1

    print(f"Done: {total_columns} columns from {total_files} files → {output_jsonl}")


if __name__ == "__main__":
    main()
