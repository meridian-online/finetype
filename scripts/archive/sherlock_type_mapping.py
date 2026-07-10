#!/usr/bin/env python3
"""Generate Sherlock→FineType type mapping CSV from schema_mapping.yaml.

Usage:
    python3 scripts/sherlock_type_mapping.py [--dest output/distillation-v3/]

Reads Sherlock's 78 ground truth types from test_labels.parquet, looks up each
in eval/schema_mapping.yaml (the canonical mapping infrastructure), and outputs
a focused CSV for the distillation three-way comparison.

Output: <dest>/sherlock_type_mapping.csv
"""

import csv
import os
import re
import sys

try:
    import pyarrow.parquet as pq
except ImportError:
    print("ERROR: pyarrow is required. Install with: pip install pyarrow", file=sys.stderr)
    sys.exit(1)


SHERLOCK_DIR = os.path.expanduser("~/datasets/sherlock/data/data/raw")
REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SCHEMA_MAPPING_PATH = os.path.join(REPO_ROOT, "eval", "schema_mapping.yaml")


def load_schema_mapping():
    """Load gt_label → mapping from eval/schema_mapping.yaml.

    Returns dict keyed by lowercase gt_label.
    """
    mappings = {}

    # Try PyYAML first
    try:
        import yaml
        with open(SCHEMA_MAPPING_PATH) as f:
            data = yaml.safe_load(f)
        for m in data["mappings"]:
            key = m["gt_label"].lower()
            mappings[key] = m
        return mappings
    except ImportError:
        pass

    # Fallback: regex extraction from YAML
    # Each mapping block starts with "- gt_label:" and contains key: value pairs
    with open(SCHEMA_MAPPING_PATH) as f:
        content = f.read()

    # Split on mapping entries
    entry_pattern = re.compile(
        r"-\s+gt_label:\s*(.+?)$"
        r"(.*?)(?=\n\s+-\s+gt_label:|\Z)",
        re.MULTILINE | re.DOTALL,
    )
    field_pattern = re.compile(r"^\s+(\w+):\s*(.+?)$", re.MULTILINE)

    for match in entry_pattern.finditer(content):
        gt_label = match.group(1).strip()
        body = match.group(2)
        entry = {"gt_label": gt_label}
        for field_match in field_pattern.finditer(body):
            key = field_match.group(1)
            val = field_match.group(2).strip()
            if val in ("null", "~"):
                val = None
            entry[key] = val
        mappings[gt_label.lower()] = entry

    return mappings


def load_sherlock_types():
    """Get unique Sherlock type labels from test_labels.parquet."""
    path = os.path.join(SHERLOCK_DIR, "test_labels.parquet")
    if not os.path.exists(path):
        print(f"ERROR: {path} not found", file=sys.stderr)
        sys.exit(1)
    table = pq.read_table(path)
    return sorted(set(table.column("type").to_pylist()))


def main():
    args = sys.argv[1:]
    dest_dir = "output/distillation-v3/"

    i = 0
    while i < len(args):
        if args[i] == "--dest":
            dest_dir = args[i + 1]
            i += 2
        elif args[i] in ("-h", "--help"):
            print(__doc__)
            sys.exit(0)
        else:
            print(f"Unknown argument: {args[i]}", file=sys.stderr)
            sys.exit(1)

    os.makedirs(dest_dir, exist_ok=True)

    # Load canonical schema mapping
    schema_map = load_schema_mapping()
    print(f"Schema mapping: {len(schema_map)} entries from {SCHEMA_MAPPING_PATH}")

    # Load Sherlock types
    sherlock_types = load_sherlock_types()
    print(f"Sherlock types: {len(sherlock_types)}")

    # Build mapping CSV
    rows = []
    by_quality = {"direct": 0, "close": 0, "partial": 0, "semantic_only": 0, "unmapped": 0}

    for stype in sherlock_types:
        key = stype.lower()
        if key in schema_map:
            m = schema_map[key]
            ft_label = m.get("finetype_label") or ""
            if ft_label == "null":
                ft_label = ""
            quality = m.get("match_quality", "semantic_only")
            notes = m.get("notes", "")
            source = m.get("source", "")
            rows.append({
                "sherlock_type": stype,
                "finetype_type": ft_label,
                "match_quality": quality,
                "source_in_mapping": source,
                "notes": notes,
            })
            by_quality[quality] = by_quality.get(quality, 0) + 1
        else:
            rows.append({
                "sherlock_type": stype,
                "finetype_type": "",
                "match_quality": "unmapped",
                "source_in_mapping": "",
                "notes": "Not found in schema_mapping.yaml — needs manual addition",
            })
            by_quality["unmapped"] += 1

    # Write CSV
    output_path = os.path.join(dest_dir, "sherlock_type_mapping.csv")
    fieldnames = ["sherlock_type", "finetype_type", "match_quality", "source_in_mapping", "notes"]
    with open(output_path, "w", newline="") as f:
        writer = csv.DictWriter(f, fieldnames=fieldnames)
        writer.writeheader()
        writer.writerows(rows)

    # Summary
    print(f"\nMapping written to {output_path}")
    print(f"\nBy match quality:")
    for quality in ["direct", "close", "partial", "semantic_only", "unmapped"]:
        count = by_quality.get(quality, 0)
        pct = 100 * count / len(rows) if rows else 0
        print(f"  {quality:15s}: {count:3d} ({pct:.0f}%)")
    print(f"  {'TOTAL':15s}: {len(rows):3d}")

    # Coverage summary
    format_detectable = by_quality["direct"] + by_quality["close"] + by_quality["partial"]
    print(f"\nFormat-detectable (direct+close+partial): {format_detectable}/{len(rows)} ({100*format_detectable/len(rows):.0f}%)")
    print(f"Semantic-only or unmapped: {by_quality['semantic_only'] + by_quality['unmapped']}/{len(rows)}")

    unmapped = [r for r in rows if r["match_quality"] == "unmapped"]
    if unmapped:
        print(f"\nUnmapped types ({len(unmapped)}) — add to eval/schema_mapping.yaml:")
        for r in unmapped:
            print(f"  {r['sherlock_type']}")


if __name__ == "__main__":
    main()
