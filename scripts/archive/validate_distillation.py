#!/usr/bin/env python3
"""Validate distillation batch CSVs against the FineType 250-label taxonomy.

Usage:
    python3 scripts/validate_distillation.py [--batch-dir output/distillation-v3/] [--taxonomy-cmd "finetype taxonomy --full --output json"]

Checks:
  1. Label validity: blind_label, finetype_label, final_label against taxonomy keys
  2. Missing required fields
  3. Per-source agreement rates
  4. Per-source row counts

Exit code 0 if invalid label rate < 5%, exit code 1 otherwise.
"""

import csv
import glob
import json
import os
import sys


REQUIRED_FIELDS = ["source", "source_file", "column_name", "blind_label", "finetype_label", "final_label", "agreement"]
LABEL_FIELDS = ["blind_label", "finetype_label", "final_label"]


def load_taxonomy_from_yaml():
    """Load taxonomy keys directly from labels/definitions_*.yaml files."""
    labels_dir = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "labels")
    keys = set()

    yaml_files = sorted(glob.glob(os.path.join(labels_dir, "definitions_*.yaml")))
    if not yaml_files:
        print(f"WARNING: No definition files found in {labels_dir}", file=sys.stderr)
        return keys

    # Try PyYAML first
    try:
        import yaml
        for path in yaml_files:
            with open(path) as f:
                data = yaml.safe_load(f)
            if isinstance(data, dict):
                for key in data:
                    if "." in str(key):
                        keys.add(str(key))
        return keys
    except ImportError:
        pass

    # Fallback: regex extraction of top-level keys (lines matching domain.category.type:)
    import re
    pattern = re.compile(r"^([a-z]+\.[a-z_]+\.[a-z_0-9]+):", re.MULTILINE)
    for path in yaml_files:
        with open(path) as f:
            content = f.read()
        for match in pattern.finditer(content):
            keys.add(match.group(1))

    return keys


def load_taxonomy_from_cmd(cmd):
    """Load taxonomy keys by running finetype CLI."""
    import subprocess
    try:
        result = subprocess.run(
            cmd.split(),
            capture_output=True, text=True, timeout=30,
        )
        if result.returncode == 0:
            data = json.loads(result.stdout)
            return set(d["key"] for d in data)
    except Exception as e:
        print(f"WARNING: Could not run taxonomy command: {e}", file=sys.stderr)
    return set()


def main():
    args = sys.argv[1:]
    batch_dir = "output/distillation-v3/"
    taxonomy_cmd = None

    i = 0
    while i < len(args):
        if args[i] == "--batch-dir":
            batch_dir = args[i + 1]
            i += 2
        elif args[i] == "--taxonomy-cmd":
            taxonomy_cmd = args[i + 1]
            i += 2
        elif args[i] in ("-h", "--help"):
            print(__doc__)
            sys.exit(0)
        else:
            print(f"Unknown argument: {args[i]}", file=sys.stderr)
            sys.exit(1)

    # Load taxonomy
    taxonomy = load_taxonomy_from_yaml()
    if taxonomy_cmd and not taxonomy:
        taxonomy = load_taxonomy_from_cmd(taxonomy_cmd)
    if not taxonomy:
        print("ERROR: Could not load taxonomy from YAML files or CLI", file=sys.stderr)
        sys.exit(1)
    print(f"Taxonomy: {len(taxonomy)} valid type keys\n")

    # Find batch files
    batch_files = sorted(glob.glob(os.path.join(batch_dir, "batch_*.csv")))
    if not batch_files:
        print(f"ERROR: No batch_*.csv files found in {batch_dir}", file=sys.stderr)
        sys.exit(1)
    print(f"Found {len(batch_files)} batch file(s)\n")

    # Counters
    total_rows = 0
    missing_fields = {f: 0 for f in REQUIRED_FIELDS}
    invalid_labels = {f: 0 for f in LABEL_FIELDS}
    label_totals = {f: 0 for f in LABEL_FIELDS}
    source_counts = {}
    source_agreement = {}  # source -> {"yes": n, "no": n}

    for batch_path in batch_files:
        try:
            with open(batch_path, "r", encoding="utf-8") as f:
                reader = csv.DictReader(f)
                if not reader.fieldnames:
                    print(f"  WARNING: empty file {batch_path}", file=sys.stderr)
                    continue

                for row in reader:
                    total_rows += 1

                    # Check missing required fields
                    for field in REQUIRED_FIELDS:
                        val = row.get(field, "").strip()
                        if not val:
                            missing_fields[field] += 1

                    # Check label validity
                    for field in LABEL_FIELDS:
                        val = row.get(field, "").strip()
                        if val:
                            label_totals[field] += 1
                            if val not in taxonomy:
                                invalid_labels[field] += 1

                    # Per-source stats
                    source = row.get("source", "").strip() or "<empty>"
                    source_counts[source] = source_counts.get(source, 0) + 1

                    agreement = row.get("agreement", "").strip().lower()
                    if source not in source_agreement:
                        source_agreement[source] = {"yes": 0, "no": 0, "other": 0}
                    if agreement == "yes":
                        source_agreement[source]["yes"] += 1
                    elif agreement == "no":
                        source_agreement[source]["no"] += 1
                    else:
                        source_agreement[source]["other"] += 1

        except Exception as e:
            print(f"  ERROR reading {batch_path}: {e}", file=sys.stderr)

    # Report
    print(f"{'=' * 60}")
    print(f"DISTILLATION VALIDATION REPORT")
    print(f"{'=' * 60}")
    print(f"Total rows: {total_rows:,}\n")

    # Missing fields
    print("Missing required fields:")
    any_missing = False
    for field in REQUIRED_FIELDS:
        count = missing_fields[field]
        if count > 0:
            pct = 100.0 * count / total_rows if total_rows else 0
            print(f"  {field:20s}: {count:,} ({pct:.1f}%)")
            any_missing = True
    if not any_missing:
        print("  (none)")
    print()

    # Invalid labels
    print("Invalid labels (not in taxonomy):")
    total_invalid = 0
    total_label_values = 0
    for field in LABEL_FIELDS:
        count = invalid_labels[field]
        total = label_totals[field]
        total_invalid += count
        total_label_values += total
        pct = 100.0 * count / total if total else 0
        print(f"  {field:20s}: {count:,}/{total:,} ({pct:.1f}%)")
    overall_invalid_rate = 100.0 * total_invalid / total_label_values if total_label_values else 0
    print(f"  {'OVERALL':20s}: {total_invalid:,}/{total_label_values:,} ({overall_invalid_rate:.1f}%)")
    print()

    # Per-source row counts
    print("Per-source row counts:")
    for source in sorted(source_counts):
        print(f"  {source:30s}: {source_counts[source]:,}")
    print()

    # Per-source agreement rates
    print("Per-source agreement rates:")
    for source in sorted(source_agreement):
        stats = source_agreement[source]
        total = stats["yes"] + stats["no"] + stats["other"]
        agree_pct = 100.0 * stats["yes"] / total if total else 0
        print(f"  {source:30s}: {stats['yes']:,} yes / {stats['no']:,} no / {stats['other']:,} other ({agree_pct:.1f}% agreement)")
    print()

    # Exit code
    threshold = 5.0
    if overall_invalid_rate >= threshold:
        print(f"FAIL: Invalid label rate {overall_invalid_rate:.1f}% >= {threshold}% threshold")
        sys.exit(1)
    else:
        print(f"PASS: Invalid label rate {overall_invalid_rate:.1f}% < {threshold}% threshold")
        sys.exit(0)


if __name__ == "__main__":
    main()
