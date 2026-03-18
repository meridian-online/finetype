#!/usr/bin/env python3
"""Merge distillation v2 batch CSVs and compute summary statistics.

Usage:
    python3 scripts/merge_distillation.py output/distillation-v2/

Reads all batch_*.csv files, merges into merged_labels.csv, and prints stats.
"""

import csv
import json
import os
import sys


def load_taxonomy():
    """Get valid FineType labels."""
    import subprocess
    result = subprocess.run(
        ["finetype", "taxonomy", "--output", "json"],
        capture_output=True, text=True, timeout=30,
    )
    if result.returncode == 0:
        data = json.loads(result.stdout)
        return set(d["key"] for d in data)
    return set()


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        sys.exit(1)

    output_dir = sys.argv[1]
    batch_files = sorted(
        os.path.join(output_dir, f)
        for f in os.listdir(output_dir)
        if f.startswith("batch_") and f.endswith(".csv")
    )

    if not batch_files:
        print(f"No batch_*.csv files found in {output_dir}")
        sys.exit(1)

    # Load taxonomy for validation
    taxonomy = load_taxonomy()
    print(f"Taxonomy: {len(taxonomy)} valid labels")

    # Merge all batches
    all_rows = []
    expected_fields = [
        "source_file", "column_name", "sample_values",
        "blind_label", "blind_confidence", "finetype_label",
        "agreement", "final_label", "reasoning",
    ]

    for batch_file in batch_files:
        try:
            with open(batch_file, "r", encoding="utf-8") as f:
                reader = csv.DictReader(f)
                batch_rows = list(reader)
                all_rows.extend(batch_rows)
                print(f"  {os.path.basename(batch_file)}: {len(batch_rows)} rows")
        except Exception as e:
            print(f"  ERROR reading {batch_file}: {e}")

    print(f"\nTotal rows: {len(all_rows)}")

    if not all_rows:
        return

    # Write merged file
    merged_path = os.path.join(output_dir, "merged_labels.csv")
    with open(merged_path, "w", newline="") as f:
        writer = csv.DictWriter(f, fieldnames=expected_fields, extrasaction="ignore")
        writer.writeheader()
        for row in all_rows:
            writer.writerow(row)
    print(f"Merged: {merged_path}")

    # --- Statistics ---
    print("\n" + "=" * 60)
    print("  Distillation v2 Summary")
    print("=" * 60)

    # Validity
    blind_valid = sum(
        1 for r in all_rows
        if r.get("blind_label", "") in taxonomy
    )
    final_valid = sum(
        1 for r in all_rows
        if r.get("final_label", "") in taxonomy
    )
    total = len(all_rows)
    print(f"\n  Blind label validity:  {blind_valid}/{total} ({blind_valid*100//total}%)")
    print(f"  Final label validity:  {final_valid}/{total} ({final_valid*100//total}%)")

    # Agreement
    agree = sum(1 for r in all_rows if r.get("agreement", "").lower() == "yes")
    disagree = sum(1 for r in all_rows if r.get("agreement", "").lower() == "no")
    print(f"\n  Agreement:    {agree}/{total} ({agree*100//total}%)")
    print(f"  Disagreement: {disagree}/{total} ({disagree*100//total}%)")

    # Confidence distribution
    conf = {}
    for r in all_rows:
        c = r.get("blind_confidence", "unknown").lower()
        conf[c] = conf.get(c, 0) + 1
    print(f"\n  Confidence distribution:")
    for c in ["high", "medium", "low"]:
        n = conf.get(c, 0)
        print(f"    {c:8s}: {n:4d} ({n*100//total}%)")

    # Agreement by domain (using blind_label domain)
    domain_agree = {}
    domain_total = {}
    for r in all_rows:
        label = r.get("blind_label", "")
        domain = label.split(".")[0] if "." in label else "unknown"
        domain_total[domain] = domain_total.get(domain, 0) + 1
        if r.get("agreement", "").lower() == "yes":
            domain_agree[domain] = domain_agree.get(domain, 0) + 1

    print(f"\n  Agreement by domain (blind_label):")
    for domain in sorted(domain_total.keys()):
        a = domain_agree.get(domain, 0)
        t = domain_total[domain]
        print(f"    {domain:20s}: {a:3d}/{t:3d} ({a*100//t}%)")

    # Disagreement details
    if disagree > 0:
        print(f"\n  Top disagreements (blind_label → finetype_label):")
        disagreements = {}
        for r in all_rows:
            if r.get("agreement", "").lower() == "no":
                key = f"{r.get('blind_label', '?')} → {r.get('finetype_label', '?')}"
                disagreements[key] = disagreements.get(key, 0) + 1

        for key, count in sorted(disagreements.items(), key=lambda x: -x[1])[:15]:
            print(f"    {count:3d}× {key}")

    # Files processed
    files = set(r.get("source_file", "") for r in all_rows)
    print(f"\n  Files processed: {len(files)}")

    print("\n" + "=" * 60)


if __name__ == "__main__":
    main()
