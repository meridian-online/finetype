#!/usr/bin/env python3
"""Extract Sherlock/VizNet columns into JSONL for FineType distillation v3.

Usage:
    python3 scripts/extract_sherlock.py [--split test|val|train|all] [--dest output/distillation-v3/]

Input:  Sherlock parquet files at ~/datasets/sherlock/data/data/raw/
Output: JSONL files at <dest>/sherlock_{split}.jsonl

Each JSONL record:
    {"source": "sherlock", "split": "<split>", "index": <n>,
     "column_name": "", "values": [...], "ground_truth_label": "<type>",
     "ground_truth_source": "sherlock"}
"""

import ast
import json
import os
import sys

try:
    import pyarrow.parquet as pq
except ImportError:
    print("ERROR: pyarrow is required. Install with: pip install pyarrow", file=sys.stderr)
    sys.exit(1)


SHERLOCK_DIR = os.path.expanduser("~/datasets/sherlock/data/data/raw")
SPLITS = ["test", "val", "train"]
MAX_VALUES = 20


def extract_split(split, dest_dir):
    """Extract one split from Sherlock parquet files into JSONL."""
    values_path = os.path.join(SHERLOCK_DIR, f"{split}_values.parquet")
    labels_path = os.path.join(SHERLOCK_DIR, f"{split}_labels.parquet")

    for path in [values_path, labels_path]:
        if not os.path.exists(path):
            print(f"  SKIP: {path} not found", file=sys.stderr)
            return 0

    print(f"Reading {split} split...")
    values_table = pq.read_table(values_path)
    labels_table = pq.read_table(labels_path)

    n_rows = len(values_table)
    if len(labels_table) != n_rows:
        print(f"  WARNING: row count mismatch: values={n_rows}, labels={len(labels_table)}", file=sys.stderr)
        n_rows = min(n_rows, len(labels_table))

    # Convert to Python lists in bulk for performance (avoids per-element .as_py())
    print(f"  Converting columns to Python lists...")
    all_values = values_table.column("values").to_pylist()
    all_labels = labels_table.column("type").to_pylist()
    all_indices = values_table.column("__index_level_0__").to_pylist()

    output_path = os.path.join(dest_dir, f"sherlock_{split}.jsonl")
    written = 0
    errors = 0

    with open(output_path, "w") as out:
        for i in range(n_rows):
            if i > 0 and i % 10_000 == 0:
                print(f"  {split}: {i:,}/{n_rows:,} rows processed ({errors} errors)")

            try:
                raw_values = all_values[i]
                label = all_labels[i]
                idx = all_indices[i]

                # values column is a string containing a Python-repr list
                # (single quotes for strings, bare values for numbers).
                # Try json.loads first (fast, works for numeric lists),
                # then fall back to ast.literal_eval for single-quoted strings.
                if isinstance(raw_values, list):
                    parsed = raw_values
                elif isinstance(raw_values, str):
                    try:
                        parsed = json.loads(raw_values)
                    except (json.JSONDecodeError, ValueError):
                        try:
                            parsed = ast.literal_eval(raw_values)
                        except (ValueError, SyntaxError):
                            errors += 1
                            if errors <= 10:
                                print(f"  WARNING: unparseable values at row {i}: {raw_values[:80]}", file=sys.stderr)
                            continue
                else:
                    print(f"  WARNING: unexpected type {type(raw_values)} at row {i}", file=sys.stderr)
                    errors += 1
                    continue

                # Sample up to MAX_VALUES, convert all to strings
                sampled = [str(v) for v in parsed[:MAX_VALUES]]

                record = {
                    "source": "sherlock",
                    "split": split,
                    "index": idx,
                    "column_name": "",
                    "values": sampled,
                    "ground_truth_label": label,
                    "ground_truth_source": "sherlock",
                }
                out.write(json.dumps(record) + "\n")
                written += 1

            except Exception as e:
                errors += 1
                if errors <= 10:
                    print(f"  ERROR at row {i}: {e}", file=sys.stderr)

    print(f"  {split}: {written:,} records written, {errors} errors → {output_path}")
    return written


def main():
    args = sys.argv[1:]
    split_filter = "all"
    dest_dir = "output/distillation-v3/"

    i = 0
    while i < len(args):
        if args[i] == "--split":
            split_filter = args[i + 1]
            i += 2
        elif args[i] == "--dest":
            dest_dir = args[i + 1]
            i += 2
        elif args[i] in ("-h", "--help"):
            print(__doc__)
            sys.exit(0)
        else:
            print(f"Unknown argument: {args[i]}", file=sys.stderr)
            sys.exit(1)

    if split_filter != "all" and split_filter not in SPLITS:
        print(f"ERROR: --split must be one of: {', '.join(SPLITS)}, all", file=sys.stderr)
        sys.exit(1)

    os.makedirs(dest_dir, exist_ok=True)

    splits = SPLITS if split_filter == "all" else [split_filter]
    total = 0
    for split in splits:
        total += extract_split(split, dest_dir)

    print(f"\nDone: {total:,} total records across {len(splits)} split(s)")


if __name__ == "__main__":
    main()
