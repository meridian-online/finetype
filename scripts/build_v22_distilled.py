#!/usr/bin/env python3
"""ac-04 — Build the v22 distilled blend.

Concatenates, in this order:
  1. v19 sherlock_distilled.csv.gz                       (~102,461 rows)
  2. v21 geonames_geography.csv.gz                       (~ 30,000 rows)
  3. ac-03 corroborated_gaps_distilled.csv.gz            (≥  6,000 rows)
  4. ac-02 hard_negatives_mined.csv.gz                   (≥  5,000 rows)
  5. ac-02 wikidata_persons.csv.gz                       (≥ 25,000 rows)

Output schema is the sherlock_distilled (final_label, sample_values
JSON, column_name) — drop-in for prepare_multibranch_data.py's
`--distilled` flag. prepare_multibranch_data applies its own per-type
caps (DOMAIN_CAP_OVERRIDES) downstream; this script does not cap.

Audit gate (per spec 2026-05-25-v22-boundary-training ac-04):
  - Total rows ≥ 170,000 before cap
  - identity.person.full_name has ≥ 5,000 hard-negative rows present
  - ≥ 30 distinct corroborated-gap source types contribute rows

Failing any of these returns exit code 3 — the launcher
(scripts/overnight_v22_boundary.sh) treats that as a hard stop.

Output:
  output/distillation-v22/sherlock_distilled_v22.csv.gz
  output/distillation-v22/v22_blend_manifest.json
"""
from __future__ import annotations

import argparse
import csv
import gzip
import json
import sys
from collections import defaultdict
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent

DEFAULT_SOURCES = [
    ("v19_sherlock",
     REPO / "output/distillation-v3/sherlock_distilled.csv.gz"),
    ("v21_geonames",
     REPO / "output/distillation-v21-geonames/geonames_geography.csv.gz"),
    ("ac03_corroborated",
     REPO / "output/distillation-v22/corroborated_gaps_distilled.csv.gz"),
    ("ac02_hard_negatives",
     REPO / "output/distillation-v22/hard_negatives_mined.csv.gz"),
    ("ac02_wikidata_persons",
     REPO / "output/distillation-v22/wikidata_persons.csv.gz"),
]
DEFAULT_OUT = REPO / "output/distillation-v22/sherlock_distilled_v22.csv.gz"
DEFAULT_MANIFEST = REPO / "output/distillation-v22/v22_blend_manifest.json"
DEFAULT_CORROBORATED_MANIFEST = REPO / "output/distillation-v22/corroborated_manifest.json"

AUDIT_TOTAL_ROWS = 170_000
AUDIT_FULL_NAME_ROWS = 5_000
AUDIT_CORROBORATED_TYPES = 30


def main() -> int:
    p = argparse.ArgumentParser(description=(__doc__ or "").splitlines()[0])
    p.add_argument("--out", type=Path, default=DEFAULT_OUT)
    p.add_argument("--manifest", type=Path, default=DEFAULT_MANIFEST)
    p.add_argument("--corroborated-manifest", type=Path,
                   default=DEFAULT_CORROBORATED_MANIFEST,
                   help="Used to extract corroborated-source type-count.")
    p.add_argument("--skip-audit", action="store_true",
                   help="Skip the audit gate (diagnostic; do not ship a model trained with this).")
    args = p.parse_args()

    # ── (1) Verify every source exists ────────────────────────────────
    missing = [(name, path) for name, path in DEFAULT_SOURCES
               if not path.exists()]
    if missing:
        for name, path in missing:
            print(f"error: source missing — {name} = {path}", file=sys.stderr)
        return 2

    args.out.parent.mkdir(parents=True, exist_ok=True)
    per_source_rows: dict[str, int] = {}
    per_label_rows: dict[str, int] = defaultdict(int)
    per_source_per_label: dict[str, dict[str, int]] = defaultdict(
        lambda: defaultdict(int))
    total = 0

    print(f"writing {args.out}...", file=sys.stderr)
    with gzip.open(args.out, "wt", newline="", encoding="utf-8") as fout:
        writer = csv.writer(fout)
        writer.writerow(["final_label", "sample_values", "column_name"])
        for name, path in DEFAULT_SOURCES:
            n_this = 0
            with gzip.open(path, "rt", newline="", encoding="utf-8") as fin:
                reader = csv.DictReader(fin)
                for row in reader:
                    label = (row.get("final_label") or "").strip()
                    sample = row.get("sample_values") or ""
                    cname = row.get("column_name") or ""
                    if not label:
                        continue
                    writer.writerow([label, sample, cname])
                    per_label_rows[label] += 1
                    per_source_per_label[name][label] += 1
                    n_this += 1
                    total += 1
            per_source_rows[name] = n_this
            print(f"  {name:>26s}: {n_this:>8,} rows", file=sys.stderr)
    print(f"  total before cap:           {total:>8,} rows", file=sys.stderr)

    # ── (2) Audit gate ────────────────────────────────────────────────
    audit_failures: list[str] = []

    if total < AUDIT_TOTAL_ROWS:
        audit_failures.append(
            f"total rows {total:,} < gate {AUDIT_TOTAL_ROWS:,}")

    full_name_total = per_label_rows.get("identity.person.full_name", 0)
    # "hard-negative rows" — interpret as ac-02 wikidata + mined hard-negative
    # full_name rows. (v19/v21 may also contribute full_name rows; those are
    # positive examples, not boundary disambiguation. The audit gate's spirit
    # is "boundary signal must be at least 5k", so we measure ac-02 sources.)
    full_name_from_ac02 = (
        per_source_per_label["ac02_wikidata_persons"]
        .get("identity.person.full_name", 0)
        + per_source_per_label["ac02_hard_negatives"]
        .get("identity.person.full_name", 0)
    )
    if full_name_from_ac02 < AUDIT_FULL_NAME_ROWS:
        audit_failures.append(
            f"full_name hard-negative rows from ac-02 sources "
            f"({full_name_from_ac02:,}) < gate {AUDIT_FULL_NAME_ROWS:,}")

    corroborated_types = 0
    if args.corroborated_manifest.exists():
        corro = json.loads(args.corroborated_manifest.read_text())
        corroborated_types = corro.get("n_types_with_rows", 0)
    if corroborated_types < AUDIT_CORROBORATED_TYPES:
        audit_failures.append(
            f"distinct corroborated source types {corroborated_types} "
            f"< gate {AUDIT_CORROBORATED_TYPES}")

    manifest = {
        "out": str(args.out),
        "total_rows": total,
        "per_source_rows": per_source_rows,
        "per_label_rows": dict(sorted(per_label_rows.items())),
        "per_source_per_label": {
            k: dict(sorted(v.items()))
            for k, v in per_source_per_label.items()
        },
        "audit": {
            "total_rows_gate": AUDIT_TOTAL_ROWS,
            "full_name_rows_gate": AUDIT_FULL_NAME_ROWS,
            "corroborated_types_gate": AUDIT_CORROBORATED_TYPES,
            "full_name_total_all_sources": full_name_total,
            "full_name_from_ac02_sources": full_name_from_ac02,
            "corroborated_types": corroborated_types,
            "failures": audit_failures,
            "passed": not audit_failures,
        },
    }
    args.manifest.write_text(json.dumps(manifest, indent=2) + "\n")
    print(f"manifest: {args.manifest}", file=sys.stderr)

    if audit_failures:
        print("AUDIT GATE FAILED:", file=sys.stderr)
        for f in audit_failures:
            print(f"  - {f}", file=sys.stderr)
        if args.skip_audit:
            print("(--skip-audit set; continuing)", file=sys.stderr)
            return 0
        return 3
    print("AUDIT GATE PASSED.", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
