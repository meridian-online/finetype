#!/usr/bin/env python3
"""Re-score distillation labels against current finetype predictions.

Runs `finetype profile` on all source files from a merged_labels.csv,
then compares new predictions against the banked Claude labels (final_label).

Usage:
    python3 scripts/rescore_distillation.py [OPTIONS]

Options:
    --labels PATH     Path to merged_labels.csv
                      (default: output/distillation-v2/merged_labels.csv)
    --csvs-dir PATH   Directory containing source CSV files
                      (default: data/csvs/)
    --finetype PATH   Path to finetype binary
                      (default: finetype, found on PATH)
    --output PATH     Write detailed comparison CSV to this path
                      (default: prints summary only)
    --cache PATH      Cache profile results to avoid re-running
                      (default: no cache)

Output summary includes:
    - Old agreement rate (banked finetype_label vs final_label)
    - New agreement rate (fresh finetype prediction vs final_label)
    - Changed predictions breakdown (improved, regressed, lateral)
"""

import argparse
import csv
import json
import os
import subprocess
import sys
from collections import Counter, defaultdict


def parse_args():
    p = argparse.ArgumentParser(
        description="Re-score distillation labels against current finetype."
    )
    p.add_argument(
        "--labels",
        default="output/distillation-v2/merged_labels.csv",
        help="Path to merged_labels.csv",
    )
    p.add_argument(
        "--csvs-dir",
        default="data/csvs/",
        help="Directory containing source CSV files",
    )
    p.add_argument(
        "--finetype",
        default="finetype",
        help="Path to finetype binary",
    )
    p.add_argument(
        "--output",
        default=None,
        help="Write detailed comparison CSV to this path",
    )
    p.add_argument(
        "--cache",
        default=None,
        help="Cache profile results as JSONL to avoid re-running",
    )
    return p.parse_args()


def load_labels(path):
    """Load merged_labels.csv and return list of dicts.

    Filters out rows with corrupted agreement values (parsing artefacts
    from sample_values containing commas).
    """
    valid_agreements = {"yes", "no", "agree", "disagree", "partial"}
    rows = []
    skipped = 0
    with open(path, newline="") as f:
        reader = csv.DictReader(f)
        for row in reader:
            if row["agreement"] in valid_agreements:
                rows.append(row)
            else:
                skipped += 1
    if skipped:
        print(f"  Skipped {skipped} rows with corrupted agreement values", file=sys.stderr)
    return rows


def load_cache(path):
    """Load cached profile results from JSONL file."""
    cache = {}
    if path and os.path.exists(path):
        with open(path) as f:
            for line in f:
                entry = json.loads(line)
                cache[entry["source_file"]] = entry["columns"]
        print(f"  Loaded {len(cache)} cached profiles from {path}", file=sys.stderr)
    return cache


def save_cache_entry(path, source_file, columns):
    """Append one profile result to the cache file."""
    if path:
        with open(path, "a") as f:
            json.dump({"source_file": source_file, "columns": columns}, f)
            f.write("\n")


def run_profile(finetype_bin, csv_path):
    """Run finetype profile on a CSV file, return dict of column_name -> type."""
    try:
        result = subprocess.run(
            [finetype_bin, "profile", "--file", csv_path, "-o", "json"],
            capture_output=True,
            text=True,
            timeout=120,
        )
        if result.returncode != 0:
            return None
        data = json.loads(result.stdout)
        return {col["column"]: col["type"] for col in data.get("columns", [])}
    except (subprocess.TimeoutExpired, json.JSONDecodeError, KeyError) as e:
        print(f"  Error profiling {csv_path}: {e}", file=sys.stderr)
        return None


def normalise_agreement(value):
    """Map agreement variants to boolean."""
    return value in ("yes", "agree")


def main():
    args = parse_args()

    # Load banked labels
    print(f"Loading labels from {args.labels}...", file=sys.stderr)
    labels = load_labels(args.labels)
    print(f"  {len(labels)} valid label rows", file=sys.stderr)

    # Group by source file
    by_file = defaultdict(list)
    for row in labels:
        by_file[row["source_file"]].append(row)
    print(f"  {len(by_file)} unique source files", file=sys.stderr)

    # Load cache if available
    cache = load_cache(args.cache)

    # Profile each file
    print(f"\nProfiling with: {args.finetype}", file=sys.stderr)
    print(f"CSVs directory: {args.csvs_dir}", file=sys.stderr)

    new_predictions = {}  # (source_file, column_name) -> new_type
    files_profiled = 0
    files_cached = 0
    files_missing = 0
    files_failed = 0

    source_files = sorted(by_file.keys())
    total_files = len(source_files)

    for i, source_file in enumerate(source_files, 1):
        csv_path = os.path.join(args.csvs_dir, source_file)

        if not os.path.exists(csv_path):
            files_missing += 1
            continue

        # Check cache first
        if source_file in cache:
            col_types = cache[source_file]
            files_cached += 1
        else:
            # Progress indicator every 50 files
            if i % 50 == 0 or i == total_files:
                print(f"  [{i}/{total_files}] Profiling {source_file}...", file=sys.stderr)

            col_types = run_profile(args.finetype, csv_path)
            if col_types is None:
                files_failed += 1
                continue

            files_profiled += 1
            save_cache_entry(args.cache, source_file, col_types)

        # Record predictions for each column
        for row in by_file[source_file]:
            col_name = row["column_name"]
            if col_name in col_types:
                new_predictions[(source_file, col_name)] = col_types[col_name]

    print(f"\n  Profiled: {files_profiled}, Cached: {files_cached}, "
          f"Missing: {files_missing}, Failed: {files_failed}", file=sys.stderr)

    # Compare old vs new
    old_agree = 0
    old_total = 0
    new_agree = 0
    new_total = 0

    improved = []   # was wrong, now right
    regressed = []  # was right, now wrong
    lateral = []    # changed but both wrong (or both right)
    unchanged = []  # same prediction

    detail_rows = []

    for row in labels:
        key = (row["source_file"], row["column_name"])
        final = row["final_label"]
        old_ft = row["finetype_label"]
        old_matched = (old_ft == final)

        old_total += 1
        if old_matched:
            old_agree += 1

        if key not in new_predictions:
            continue

        new_ft = new_predictions[key]
        new_matched = (new_ft == final)

        new_total += 1
        if new_matched:
            new_agree += 1

        detail = {
            "source_file": row["source_file"],
            "column_name": row["column_name"],
            "final_label": final,
            "old_finetype": old_ft,
            "new_finetype": new_ft,
            "old_match": old_matched,
            "new_match": new_matched,
            "changed": old_ft != new_ft,
        }
        detail_rows.append(detail)

        if old_ft == new_ft:
            unchanged.append(detail)
        elif new_matched and not old_matched:
            improved.append(detail)
        elif old_matched and not new_matched:
            regressed.append(detail)
        else:
            lateral.append(detail)

    # Print summary
    print("\n" + "=" * 70)
    print("DISTILLATION RE-SCORE SUMMARY")
    print("=" * 70)

    print(f"\nLabels file:  {args.labels}")
    print(f"Total label rows (valid): {len(labels)}")
    print(f"Rows with new predictions: {new_total}")
    print(f"Rows without predictions:  {len(labels) - new_total} "
          f"(missing files or columns)")

    print(f"\n--- Agreement with final_label (Claude ground truth) ---")
    if old_total:
        print(f"Old finetype: {old_agree}/{old_total} "
              f"({100*old_agree/old_total:.1f}%)")
    if new_total:
        print(f"New finetype: {new_agree}/{new_total} "
              f"({100*new_agree/new_total:.1f}%)")

    changed_count = len(improved) + len(regressed) + len(lateral)
    print(f"\n--- Prediction changes ---")
    print(f"Unchanged:  {len(unchanged)}")
    print(f"Changed:    {changed_count}")
    print(f"  Improved:   {len(improved)} (was wrong, now right)")
    print(f"  Regressed:  {len(regressed)} (was right, now wrong)")
    print(f"  Lateral:    {len(lateral)} (changed, both wrong)")

    if new_total and old_total:
        delta = (new_agree / new_total) - (old_agree / old_total)
        direction = "+" if delta >= 0 else ""
        print(f"\nNet accuracy change: {direction}{100*delta:.1f}pp")

    # Show top improvements
    if improved:
        print(f"\n--- Top improvements (was wrong, now right) ---")
        # Group by type change pattern
        patterns = Counter()
        for d in improved:
            patterns[(d["old_finetype"], d["new_finetype"])] += 1
        for (old, new), count in patterns.most_common(10):
            print(f"  {old} -> {new}: {count}x")

    # Show top regressions
    if regressed:
        print(f"\n--- Top regressions (was right, now wrong) ---")
        patterns = Counter()
        for d in regressed:
            patterns[(d["old_finetype"], d["new_finetype"])] += 1
        for (old, new), count in patterns.most_common(10):
            print(f"  {old} -> {new}: {count}x")

    # Write detailed output CSV if requested
    if args.output and detail_rows:
        with open(args.output, "w", newline="") as f:
            writer = csv.DictWriter(f, fieldnames=[
                "source_file", "column_name", "final_label",
                "old_finetype", "new_finetype",
                "old_match", "new_match", "changed",
            ])
            writer.writeheader()
            writer.writerows(detail_rows)
        print(f"\nDetailed output written to: {args.output}")

    print()


if __name__ == "__main__":
    main()
