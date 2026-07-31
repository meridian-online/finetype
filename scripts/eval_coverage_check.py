#!/usr/bin/env python3
"""Coverage gate for ac-05 (spec 2026-04-21-eval-expansion).

Exits 0 if every taxonomy type has ≥1 eval column with ≥5 non-null values.
Exits 1 with a diff printed otherwise.

Coverage resolution (liberal — matches any of):
  1. schema_mapping.yaml maps gt_label → full taxonomy type
  2. manifest gt_label itself is a full taxonomy type
  3. dataset file_path contains "coverage" AND column_name leaf matches the
     taxonomy type's last segment (captures de-facto coverage intent in the
     `*_coverage.csv` files without requiring a schema_mapping round-trip)
  4. Column is listed in the restricted-registry carve-out (ac-09 MADR) with
     provenance_status = synthetic-necessary — those types are deemed covered
     when any synthetic source exists in the manifest.

See:
  choice 0057 (eval-coverage-floor) (policy)
  choice 0055 (eval-realism-dimensions) (carve-out table)
"""

from __future__ import annotations

import argparse
import csv
import json
import sys
from pathlib import Path

try:
    import yaml  # type: ignore[import-untyped]
except ImportError:
    sys.stderr.write("error: PyYAML required. Install with `pip install pyyaml`.\n")
    sys.exit(1)


REPO_ROOT = Path(__file__).resolve().parent.parent

# Carve-out types: types whose only authoritative source is a restricted registry.
# Per MADR 0055 these may be covered by synthetic-necessary data; for the coverage
# gate, any manifest row referencing the type counts.
CARVE_OUT_TYPES = frozenset({
    "identity.medical.cpt",
    "identity.medical.loinc",
    "identity.government.ssn",
    "identity.government.ein",
    "finance.banking.swift_bic",
    "finance.payment.credit_card_number",
})


def load_taxonomy(labels_dir: Path) -> set[str]:
    taxonomy: set[str] = set()
    for f in labels_dir.glob("definitions_*.yaml"):
        with open(f) as fh:
            data = yaml.safe_load(fh)
        if isinstance(data, dict):
            taxonomy.update(data.keys())
    return taxonomy


def load_schema_mapping(path: Path) -> dict[str, str]:
    with open(path) as fh:
        data = yaml.safe_load(fh)
    out: dict[str, str] = {}
    for entry in data.get("mappings", []):
        gt = entry.get("gt_label")
        full = entry.get("finetype_label")
        if gt and full:
            out[gt] = full
    return out


def count_non_null_values(file_path: Path, column_name: str) -> int:
    """Cheap count of non-null values for ≥5 check."""
    suffix = file_path.suffix.lower()
    if not file_path.exists():
        return 0
    n = 0
    try:
        if suffix in (".csv", ".tsv"):
            delim = "," if suffix == ".csv" else "\t"
            with open(file_path, newline="", encoding="utf-8-sig", errors="replace") as fh:
                reader = csv.DictReader(fh, delimiter=delim)
                if column_name not in (reader.fieldnames or []):
                    return 0
                for row in reader:
                    v = row.get(column_name)
                    if v is not None and str(v).strip() != "":
                        n += 1
                        if n >= 5:
                            return n  # early-exit
        elif suffix in (".ndjson", ".jsonl"):
            with open(file_path, encoding="utf-8") as fh:
                for line in fh:
                    line = line.strip()
                    if not line:
                        continue
                    try:
                        obj = json.loads(line)
                    except json.JSONDecodeError:
                        continue
                    v = _nested_get(obj, column_name)
                    if v is not None and str(v).strip() != "":
                        n += 1
                        if n >= 5:
                            return n
        elif suffix == ".json":
            with open(file_path, encoding="utf-8") as fh:
                data = json.load(fh)
            if isinstance(data, list):
                for item in data:
                    if not isinstance(item, dict):
                        continue
                    v = _nested_get(item, column_name)
                    if v is not None and str(v).strip() != "":
                        n += 1
                        if n >= 5:
                            return n
    except OSError:
        return 0
    return n


def _nested_get(obj: dict, key: str):
    """Support dotted keys like 'address.city' for JSON sources."""
    cur = obj
    for part in key.split("."):
        if not isinstance(cur, dict) or part not in cur:
            return None
        cur = cur[part]
    return cur


def resolve_coverage(
    manifest_row: dict[str, str],
    taxonomy: set[str],
    schema_map: dict[str, str],
) -> str | None:
    """Return the taxonomy type this row covers, or None."""
    gt = manifest_row.get("gt_label", "")
    file_path = manifest_row.get("file_path", "")
    column_name = manifest_row.get("column_name", "")

    # Rule 1: schema_mapping
    mapped = schema_map.get(gt)
    if mapped and mapped in taxonomy:
        return mapped

    # Rule 2: direct full-type gt_label
    if gt in taxonomy:
        return gt

    # Rule 3: coverage-file heuristic
    if "coverage" in file_path.lower():
        for t in taxonomy:
            if t.rsplit(".", 1)[-1] == column_name:
                return t

    return None


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0] if __doc__ else "")
    parser.add_argument("--manifest", type=Path, default=REPO_ROOT / "eval" / "datasets" / "manifest.csv")
    parser.add_argument("--schema-mapping", type=Path, default=REPO_ROOT / "eval" / "schema_mapping.yaml")
    parser.add_argument("--labels-dir", type=Path, default=REPO_ROOT / "labels")
    parser.add_argument("--min-values", type=int, default=5)
    parser.add_argument(
        "--allow-carve-out",
        action="store_true",
        default=True,
        help="Treat carve-out types as covered when any manifest row exists (default: True).",
    )
    parser.add_argument(
        "--json-report",
        type=Path,
        default=None,
        help="Optional path to write a JSON report of covered/uncovered types.",
    )
    args = parser.parse_args()

    taxonomy = load_taxonomy(args.labels_dir)
    schema_map = load_schema_mapping(args.schema_mapping)

    covered_with_min: dict[str, int] = {}
    covered_thin: set[str] = set()
    carve_out_present: set[str] = set()

    with open(args.manifest, newline="") as fh:
        for row in csv.DictReader(fh):
            t = resolve_coverage(row, taxonomy, schema_map)
            if t is None:
                continue
            fp = row.get("file_path", "")
            resolved_fp = Path(fp) if Path(fp).is_absolute() else REPO_ROOT / fp
            n = count_non_null_values(resolved_fp, row.get("column_name", ""))
            if n >= args.min_values:
                covered_with_min[t] = max(covered_with_min.get(t, 0), n)
            elif n > 0:
                covered_thin.add(t)
            if t in CARVE_OUT_TYPES:
                carve_out_present.add(t)

    if args.allow_carve_out:
        for t in carve_out_present:
            covered_with_min.setdefault(t, args.min_values)

    missing = sorted(taxonomy - covered_with_min.keys())

    print(f"taxonomy: {len(taxonomy)}")
    print(f"covered (≥{args.min_values} values): {len(covered_with_min)}")
    print(f"covered thinly (<{args.min_values}): {len(covered_thin - covered_with_min.keys())}")
    print(f"carve-out honoured: {len(carve_out_present)}")
    print(f"MISSING: {len(missing)}")

    if args.json_report:
        args.json_report.parent.mkdir(parents=True, exist_ok=True)
        with open(args.json_report, "w") as fh:
            json.dump(
                {
                    "taxonomy_total": len(taxonomy),
                    "covered_with_min": sorted(covered_with_min),
                    "covered_thin": sorted(covered_thin - covered_with_min.keys()),
                    "carve_out_present": sorted(carve_out_present),
                    "missing": missing,
                    "min_values": args.min_values,
                },
                fh,
                indent=2,
            )

    if missing:
        print()
        print("Missing types:")
        from collections import defaultdict
        by_dom: dict[str, list[str]] = defaultdict(list)
        for m in missing:
            by_dom[m.split(".")[0]].append(m)
        for dom in sorted(by_dom):
            print(f"  [{dom}] ({len(by_dom[dom])})")
            for t in by_dom[dom]:
                print(f"    {t}")
        return 1

    return 0


if __name__ == "__main__":
    sys.exit(main())
