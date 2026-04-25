#!/usr/bin/env python3
"""ac-01: Family inventory for remove-header-hints roadmap.

Scans crates/finetype-model/src/column.rs for every rule-based header-hint
rule family and emits diagnostics/rhh_family_inventory.tsv.

Granularity is rule-family, not per-arm — matching the runtime
disambiguation_rule emitter. Sub-arm enumeration is deferred to a future
spec (see spec.yaml constraint #3).

Run from repo root:
    python3 scripts/rhh/ac01_inventory.py
"""
from __future__ import annotations

import pathlib
import re
import sys

REPO_ROOT = pathlib.Path(__file__).resolve().parents[2]
COLUMN_RS = REPO_ROOT / "crates" / "finetype-model" / "src" / "column.rs"
OUT_TSV = REPO_ROOT / "diagnostics" / "rhh_family_inventory.tsv"

# Curated family inventory. Each entry describes one rule family at the
# granularity the runtime disambiguation_rule emitter uses.
#
# Fields:
#   family_id:        stable identifier (matches runtime tag where one exists)
#   rule_family_class: match_table | branch | substring | legacy_sense
#   domain:            7-domain enum, or "cross-domain"
#   type_targets:      semicolon-separated taxonomy types
#   locator:           regex pattern to grep column.rs for line_start; if tuple
#                      of (start_pattern, end_pattern), both are grepped
#   description_notes: optional prose for methodology doc
FAMILIES = [
    # 1. Top-level match table inside fn header_hint — ~185 lines of exact-arm
    # matches. Emits via header_hint_hardcoded when a match fires (the branch
    # at line 1158/2350 tags the result after calling header_hint()).
    {
        "family_id": "header_hint_table",
        "rule_family_class": "match_table",
        "domain": "cross-domain",
        "type_targets": "identity.person.email;technology.internet.url;technology.internet.ip_v4;technology.identifier.uuid;identity.person.gender;representation.numeric.integer_number;datetime.epoch.unix_seconds;datetime.epoch.unix_milliseconds;finance.currency.amount;finance.currency.amount_accounting;finance.currency.amount_apostrophe;finance.currency.amount_code_prefix;finance.currency.amount_comma;finance.currency.amount_comma_suffix;finance.currency.amount_crypto;finance.currency.amount_lakh;finance.currency.amount_multisym;finance.currency.amount_neg_trailing;finance.currency.amount_nodecimal;finance.currency.amount_space",
        "locator_start": r"^fn header_hint\(header: &str\)",
        "locator_end": r"^    // Keyword/substring matching \(less specific\)",
    },
    # 2. Substring matchers grouped by target domain. All live inside
    # fn header_hint after the match table. The matchers are *interleaved*
    # across domains (e.g. identity → technology → geography → identity
    # again), so contiguous-block bracketing does not apply.
    #
    # Canonical locator: the `!disable_<domain>` guard inserted by ac-02
    # instrumentation. Every call-site of a domain's substring matcher
    # starts with this guard, so min/max/count of the guard lines are the
    # authoritative line_start / line_end / arm_count for the family.
    # This is deliberate — ac-02 is the ground truth for which call-sites
    # belong to which family, so reusing its markers keeps ac-01 and ac-02
    # in lock-step across future edits.
    {
        "family_id": "substring_matcher_identity",
        "rule_family_class": "substring",
        "domain": "identity",
        "type_targets": "identity.person.email;identity.person.phone_number;identity.person.first_name;identity.person.last_name;identity.person.full_name;identity.person.weight;identity.person.height;identity.credential.password",
        "locator_start": r"!disable_identity",
        "multi_site": True,
    },
    {
        "family_id": "substring_matcher_technology",
        "rule_family_class": "substring",
        "domain": "technology",
        "type_targets": "technology.internet.ip_v6;technology.internet.ip_v4;technology.internet.url",
        "locator_start": r"!disable_technology",
        "multi_site": True,
    },
    {
        "family_id": "substring_matcher_geography",
        "rule_family_class": "substring",
        "domain": "geography",
        "type_targets": "geography.address.postal_code;geography.address.street_address;geography.address.full_address",
        "locator_start": r"!disable_geography",
        "multi_site": True,
    },
    {
        "family_id": "substring_matcher_datetime",
        "rule_family_class": "substring",
        "domain": "datetime",
        "type_targets": "datetime.date.iso;datetime.timestamp.rfc_2822;datetime.timestamp.rfc_3339;datetime.timestamp.sql_standard;datetime.epoch.unix_seconds;datetime.epoch.unix_milliseconds;datetime.timestamp.iso_8601;datetime.component.year",
        "locator_start": r"!disable_datetime",
        "multi_site": True,
    },
    {
        "family_id": "substring_matcher_finance",
        "rule_family_class": "substring",
        "domain": "finance",
        "type_targets": "finance.currency.amount",
        "locator_start": r"!disable_finance",
        "multi_site": True,
    },
    {
        "family_id": "substring_matcher_representation",
        "rule_family_class": "substring",
        "domain": "representation",
        "type_targets": "representation.numeric.integer_number;representation.discrete.ordinal;representation.alphanumeric.alphanumeric_id;representation.numeric.decimal_number",
        "locator_start": r"!disable_representation",
        "multi_site": True,
    },
    # 3. Branch families — each has a dedicated runtime disambiguation_rule tag.
    {
        "family_id": "header_hint_measurement",
        "rule_family_class": "branch",
        "domain": "representation",
        "type_targets": "representation.numeric.decimal_number;representation.numeric.integer_number",
        "locator_start": r'Some\(format!\("header_hint_measurement',
        "multi_site": True,
    },
    {
        "family_id": "header_hint_location",
        "rule_family_class": "branch",
        "domain": "geography",
        "type_targets": "geography.coordinate.latitude;geography.coordinate.longitude;geography.address.postal_code",
        "locator_start": r'Some\(format!\("header_hint_location:',
        "multi_site": True,
    },
    {
        "family_id": "header_hint_hardcoded",
        "rule_family_class": "branch",
        "domain": "cross-domain",
        "type_targets": "cross-domain (routes match_table results)",
        "locator_start": r'Some\(format!\("header_hint_hardcoded',
        "multi_site": True,
    },
    {
        "family_id": "header_hint_generic",
        "rule_family_class": "branch",
        "domain": "cross-domain",
        "type_targets": "cross-domain (generic substring hits)",
        "locator_start": r'Some\(format!\("header_hint_generic',
        "multi_site": True,
    },
    {
        "family_id": "header_hint_fallback",
        "rule_family_class": "branch",
        "domain": "cross-domain",
        "type_targets": "cross-domain (fallback tier)",
        "locator_start": r'Some\(format!\("header_hint_fallback',
        "multi_site": True,
    },
    {
        "family_id": "header_hint",
        "rule_family_class": "branch",
        "domain": "cross-domain",
        "type_targets": "cross-domain (catchall tier)",
        "locator_start": r'Some\(format!\("header_hint:',
        "multi_site": True,
    },
    # 3b. Multi-branch threshold / specialty branches.
    # Discovered via grep audit of `disambiguation_rule = Some(format!("..."))`
    # emit sites that escape the broader `header_hint:` / `header_hint_<tier>:`
    # locators. Two of these (cross_domain, same_category) are explicitly kept
    # by spec constraint #2 — flagged via `keep_required: True`.
    {
        "family_id": "header_hint_cross_domain",
        "rule_family_class": "branch",
        "domain": "cross-domain",
        "type_targets": "cross-domain (cross-domain hardcoded hint promotion at 0.85)",
        "locator_start": r'^\s*"header_hint_cross_domain:',
        "multi_site": True,
        "keep_required": True,
    },
    {
        "family_id": "header_hint_same_category",
        "rule_family_class": "branch",
        "domain": "cross-domain",
        "type_targets": "cross-domain (same-category hardcoded hint promotion at 0.95)",
        "locator_start": r'^\s*"header_hint_same_category:',
        "multi_site": True,
        "keep_required": True,
    },
    {
        "family_id": "header_hint_geo_override",
        "rule_family_class": "branch",
        "domain": "geography",
        "type_targets": "geography.location.* (same-domain location override)",
        "locator_start": r'^\s*"header_hint_geo_override:',
        "multi_site": True,
    },
    {
        "family_id": "header_hint_person_override",
        "rule_family_class": "branch",
        "domain": "identity",
        "type_targets": "identity.person.* (hardcoded person-name hint over location)",
        "locator_start": r'^\s*"header_hint_person_override:',
        "multi_site": True,
    },
    {
        "family_id": "header_hint_sci_measurement",
        "rule_family_class": "branch",
        "domain": "representation",
        "type_targets": "representation.numeric.decimal_number (sci-measurement override of latitude/longitude)",
        "locator_start": r'^\s*"header_hint_sci_measurement:',
        "multi_site": True,
    },
    {
        "family_id": "header_hint_location_keep",
        "rule_family_class": "branch",
        "domain": "geography",
        "type_targets": "geography.location.* (preserves model's location prediction over person-name hint)",
        "locator_start": r'^\s*"header_hint_location_keep:',
        "multi_site": True,
    },
    # 4. Legacy sense branches — may or may not still fire in the default pipeline
    # but kept for completeness; ac-03 hit counts will reveal.
    # Note: the `sense_header_hint` family entry's locator is a prefix match
    # that buckets all 12 sense_header_hint* variants (sense_header_hint,
    # _measurement, _location, _hardcoded, _generic, _fallback,
    # _geo_override, _person_override, _sci_measurement, _location_keep,
    # _same_category, _cross_domain) under one family — this is intentional
    # because the Sense path is legacy and finer instrumentation buys nothing.
    {
        "family_id": "sense_geo_hint_override",
        "rule_family_class": "legacy_sense",
        "domain": "geography",
        "type_targets": "geography.coordinate.latitude;geography.coordinate.longitude",
        "locator_start": r'Some\(format!\("sense_geo_hint_override',
        "multi_site": True,
    },
    {
        "family_id": "sense_geo_rescue",
        "rule_family_class": "legacy_sense",
        "domain": "geography",
        "type_targets": "geography.coordinate.latitude;geography.coordinate.longitude",
        "locator_start": r'Some\(format!\("sense_geo_rescue',
        "multi_site": True,
    },
    {
        "family_id": "sense_header_hint",
        "rule_family_class": "legacy_sense",
        "domain": "cross-domain",
        "type_targets": "cross-domain (legacy Sense path)",
        "locator_start": r'Some\(format!\("sense_header_hint',
        "multi_site": True,
    },
]


def grep_line(pattern: str, path: pathlib.Path) -> list[int]:
    """Return 1-indexed line numbers of all matches of pattern in path."""
    text = path.read_text(encoding="utf-8")
    rx = re.compile(pattern)
    return [i + 1 for i, line in enumerate(text.splitlines()) if rx.search(line)]


def count_arms_in_range(path: pathlib.Path, start: int, end: int) -> int:
    """Count match arms or `if h.contains/h==` lines in [start, end]."""
    text = path.read_text(encoding="utf-8")
    lines = text.splitlines()[start - 1 : end]
    arm_count = 0
    arm_rx = re.compile(r'^\s*"[^"]+"( \| "[^"]+")*\s*(=>|if)|^\s*if h\.contains\(|^\s*if h\.ends_with\(|^\s*if h\.starts_with\(|^\s*if \(h\.contains\(|^\s*if h ==')
    for line in lines:
        if arm_rx.match(line):
            arm_count += 1
    return arm_count


def resolve_family(entry: dict) -> dict:
    """Look up line_start/line_end/arm_count for an entry."""
    resolved = dict(entry)
    if entry.get("multi_site"):
        lines = grep_line(entry["locator_start"], COLUMN_RS)
        if not lines:
            print(
                f"WARN: family {entry['family_id']} has no matches for pattern: {entry['locator_start']}",
                file=sys.stderr,
            )
            resolved["line_start"] = 0
            resolved["line_end"] = 0
            resolved["arm_count"] = 0
        else:
            resolved["line_start"] = min(lines)
            resolved["line_end"] = max(lines)
            resolved["arm_count"] = len(lines)
    else:
        starts = grep_line(entry["locator_start"], COLUMN_RS)
        ends = grep_line(entry["locator_end"], COLUMN_RS)
        if not starts or not ends:
            print(
                f"WARN: family {entry['family_id']} failed to resolve start={starts} end={ends}",
                file=sys.stderr,
            )
            resolved["line_start"] = 0
            resolved["line_end"] = 0
            resolved["arm_count"] = 0
        else:
            resolved["line_start"] = min(starts)
            resolved["line_end"] = max(ends)
            resolved["arm_count"] = count_arms_in_range(
                COLUMN_RS, resolved["line_start"], resolved["line_end"]
            )
    return resolved


def main() -> int:
    if not COLUMN_RS.exists():
        print(f"ERROR: {COLUMN_RS} not found", file=sys.stderr)
        return 1

    OUT_TSV.parent.mkdir(parents=True, exist_ok=True)

    resolved_families = [resolve_family(f) for f in FAMILIES]

    with OUT_TSV.open("w", encoding="utf-8") as fh:
        fh.write(
            "# rhh_family_inventory.tsv — ac-01 output\n"
            "# Source: crates/finetype-model/src/column.rs\n"
            "# Schema: family_id\\trule_family_class\\tfile\\tline_start\\tline_end\\tdomain\\ttype_targets\\tarm_count\\tkeep_required\n"
            "# keep_required=true means spec constraint #2 forbids removal "
            "(threshold-gate families); other rows are eligible for the "
            "removal roadmap subject to ac-05 classification.\n"
        )
        fh.write(
            "family_id\trule_family_class\tfile\tline_start\tline_end\t"
            "domain\ttype_targets\tarm_count\tkeep_required\n"
        )
        for fam in resolved_families:
            keep = "true" if fam.get("keep_required") else "false"
            fh.write(
                f"{fam['family_id']}\t{fam['rule_family_class']}\t"
                f"crates/finetype-model/src/column.rs\t"
                f"{fam['line_start']}\t{fam['line_end']}\t"
                f"{fam['domain']}\t{fam['type_targets']}\t{fam['arm_count']}\t{keep}\n"
            )

    print(f"Wrote {len(resolved_families)} families to {OUT_TSV}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
