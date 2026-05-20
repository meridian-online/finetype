#!/usr/bin/env python3
"""Extend `dbpedia_finetype_mapping.tsv` to cover every DBpedia class
that appears ≥10× in the measure-half corpus.

Spec: `.orbit/specs/2026-05-20-gittables-multi-lens-diagnostic/spec.yaml`
ac-05 (DBpedia / Schema.org annotation extraction + mapping extension).

The base mapping was hand-curated for the 98 classes in ac-02's
94-table sample (`eval/gittables/dbpedia_finetype_mapping.tsv`).
This script:

  1. Preserves every existing manually-curated row (priority).
  2. For each newly-surfaced ≥10× class, applies a deterministic
     heuristic to assign one of {direct, partial, no_finetype_equivalent}.
     Heuristic rules live in HEURISTIC_RULES below and aim to err on
     the side of `no_finetype_equivalent` — false positives in the
     mapping would inflate the DBpedia lens's vote count and weaken
     corroboration noise control.
  3. Marks each heuristic-applied row with `auto-classified` in the
     `note` column so the author can spot-check.

Usage:
  python3 scripts/extend_dbpedia_mapping.py

Output (overwrites in place):
  eval/gittables/dbpedia_finetype_mapping.tsv — extended to ≥1707 rows.
"""

from __future__ import annotations

import argparse
import csv
import json
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
DEFAULT_MAPPING = REPO / "eval" / "gittables" / "dbpedia_finetype_mapping.tsv"
DEFAULT_COVERAGE = REPO / "eval" / "gittables" / "dbpedia_coverage.json"


HEURISTIC_RULES: list[tuple[str, str, str]] = [
    # Order matters — first match wins. Patterns are case-insensitive
    # word-boundary matches against the local part of the DBpedia URI
    # (the bit after the last '/').
    # (regex_pattern, finetype_taxonomy_id, mapping_status)
    (r"country code|countrycode|iso31661", "geography.location.country_code", "direct"),
    (r"\bcountry\b", "geography.location.country", "direct"),
    (r"continent", "geography.location.continent", "direct"),
    (r"\bcity\b|capital", "geography.location.city", "direct"),
    (r"\bstate\b", "geography.location.state_code", "partial"),
    (r"\bregion\b|province|district|county", "geography.location.region", "partial"),
    (r"postal|zipcode|zip code", "geography.address.postal_code", "direct"),
    (r"\baddress\b|street", "geography.address.full_address", "partial"),
    (r"\bemail\b|emailaddress", "technology.internet.email", "direct"),
    (r"\burl\b|uri\b|fileurl|website|homepage|link", "technology.internet.url", "direct"),
    (r"\bphone\b|telephone|mobile|fax", "technology.identifier.alphanumeric_id", "partial"),
    (r"\bissn\b", "identity.commerce.issn", "direct"),
    (r"\bisbn\b", "identity.commerce.isbn", "direct"),
    (r"\bdoi\b", "identity.commerce.doi", "direct"),
    (r"uuid|guid", "representation.identifier.uuid", "direct"),
    (r"\bgender\b|\bsex\b", "identity.person.gender", "direct"),
    (r"author|writer|editor|artist|composer|director|producer|performer|illustrator",
     "identity.person.full_name", "direct"),
    (r"\bperson\b|individual|member|teacher|student|patient|player|presenter",
     "identity.person.full_name", "partial"),
    (r"firstname|first name|givenname|given name", "identity.person.first_name", "direct"),
    (r"lastname|last name|surname|familyname|family name", "identity.person.last_name", "direct"),
    (r"\bfullname\b|full name", "identity.person.full_name", "direct"),
    (r"\bname\b", "identity.person.full_name", "partial"),
    (r"organisation|organization|company|publisher|employer|institution|university|department|agency|ministry|publisher|recordlabel|record label",
     "representation.text.entity_name", "partial"),
    (r"\byear\b|fiscalyear|budgetyear", "datetime.component.year", "direct"),
    (r"\bmonth\b", "datetime.component.month_name", "partial"),
    (r"\bweekday\b|day of week|dayofweek", "datetime.component.day_of_week", "direct"),
    (r"\bdate\b|created|modified|published|updated|start|end|inception|dissolved|founded|established|maidenflight|bodydiscovered|registered|dateuse",
     "datetime.date.iso_8601", "partial"),
    (r"\btime\b(?!stamp)|hour|minute", "datetime.time.iso_8601", "partial"),
    (r"timestamp|datetime", "datetime.timestamp.iso_8601", "partial"),
    (r"duration|period|elapsed", "datetime.period.duration", "partial"),
    (r"amount|price|cost|fee|salary|wage|revenue|funding|budget|payment|charge|tax|currency",
     "finance.currency.amount", "partial"),
    (r"currencycode|currency code", "finance.currency.currency_code", "direct"),
    (r"latitude|lat\b", "geography.location.latitude", "direct"),
    (r"longitude|lng\b|long\b", "geography.location.longitude", "direct"),
    (r"coordinate|coords", "geography.location.coordinates", "partial"),
    (r"\bidentifier\b|\bid\b$|\bid_|_id\b|projectreference",
     "representation.identifier.alphanumeric_id", "partial"),
    (r"\bcode\b", "representation.identifier.alphanumeric_id", "partial"),
    # Boolean / categorical / counts / measures — explicit non-mappings
    (r"\btype\b|category|class|kind|tag|status|state|enabled|disabled|active|flag|isactive|ispartof|haspart",
     "", "no_finetype_equivalent"),
    (r"\btitle\b|caption|headline|label", "", "no_finetype_equivalent"),
    (r"\babstract\b|description|comment|note|text|summary|body|content|message|review|definition|meaning|reasoning",
     "", "no_finetype_equivalent"),
    (r"count|number\b|num\b|qty|quantity|total|max\b|min\b|avg|sum|score|rank|rating|points|percent|ratio|percentage|growth|share|stats?|stdev|variance|standardDeviation",
     "", "no_finetype_equivalent"),
    (r"\bheight\b|\bwidth\b|\blength\b|\bdepth\b|\bweight\b|\bmass\b|\bsize\b|\barea\b|\bvolume\b|temperature|pressure|altitude|velocity",
     "", "no_finetype_equivalent"),
    (r"abbreviation|symbol|formula", "", "no_finetype_equivalent"),
    (r"genre|family|species|breed|strain|clade|kingdom|phylum|order|genus",
     "", "no_finetype_equivalent"),
    (r"chromosome|gene|protein|enzyme|virus|disease|cancer|drug|medication|treatment|diagnosis",
     "", "no_finetype_equivalent"),
]


def _classify(annotation_id: str) -> tuple[str, str, str]:
    """Returns (finetype_taxonomy_id, mapping_status, note)."""
    short = annotation_id.rsplit("/", 1)[-1]
    for pattern, taxonomy_id, status in HEURISTIC_RULES:
        if re.search(pattern, short, flags=re.IGNORECASE):
            return (taxonomy_id, status, "auto-classified")
    return ("", "no_finetype_equivalent", "auto-classified; no heuristic match")


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Extend DBpedia → FineType mapping (ac-05)."
    )
    parser.add_argument("--mapping", type=Path, default=DEFAULT_MAPPING)
    parser.add_argument("--coverage", type=Path, default=DEFAULT_COVERAGE)
    args = parser.parse_args()

    if not args.coverage.exists():
        print(f"error: coverage file not found: {args.coverage}",
              file=sys.stderr)
        return 2

    cov = json.loads(args.coverage.read_text())
    target_classes = [
        e["class"] for e in cov["classes_at_or_above_threshold"]
    ]
    print(f"target classes (≥{cov['min_class_freq_threshold']}×): "
          f"{len(target_classes)}", file=sys.stderr)

    # Load existing mapping; preserve every row.
    existing: dict[str, dict[str, str]] = {}
    if args.mapping.exists():
        with args.mapping.open() as fh:
            reader = csv.DictReader(fh, delimiter="\t")
            for row in reader:
                cls = row.get("dbpedia_or_schemaorg_class", "").strip()
                if cls:
                    existing[cls] = {
                        "dbpedia_or_schemaorg_class": cls,
                        "finetype_taxonomy_id": row.get("finetype_taxonomy_id", ""),
                        "mapping_status": row.get("mapping_status", ""),
                        "note": row.get("note", ""),
                    }
    print(f"existing mapping rows: {len(existing)}", file=sys.stderr)

    # For each target class not in existing, apply heuristic.
    n_added = 0
    n_status: dict[str, int] = {}
    for cls in target_classes:
        if cls in existing:
            n_status[existing[cls]["mapping_status"]] = (
                n_status.get(existing[cls]["mapping_status"], 0) + 1
            )
            continue
        tid, status, note = _classify(cls)
        existing[cls] = {
            "dbpedia_or_schemaorg_class": cls,
            "finetype_taxonomy_id": tid,
            "mapping_status": status,
            "note": note,
        }
        n_added += 1
        n_status[status] = n_status.get(status, 0) + 1

    print(f"added {n_added} new rows via heuristic", file=sys.stderr)
    print(f"  status distribution: {n_status}", file=sys.stderr)

    # Write in deterministic order: classes sorted by URI.
    with args.mapping.open("w", newline="") as fh:
        writer = csv.DictWriter(
            fh,
            fieldnames=[
                "dbpedia_or_schemaorg_class",
                "finetype_taxonomy_id",
                "mapping_status",
                "note",
            ],
            delimiter="\t",
        )
        writer.writeheader()
        for cls in sorted(existing.keys()):
            writer.writerow(existing[cls])
    print(f"wrote {args.mapping} ({len(existing)} rows)", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
