#!/usr/bin/env python3
"""ac-03 — per-family hit counts on the full 448-row eval manifest.

Deliverable: diagnostics/rhh_hit_counts.tsv

Method:

  1. Load the family inventory (ac-01 output) to get the canonical family list
     and the keep_required flag.
  2. Profile each unique (dataset, file_path) pair in the eval manifest once,
     using sherlock-v16 via models/default (CLI default).
  3. For every profiled column, extract the ``disambiguation_rule`` field. The
     rule string has the shape ``<prefix>:<header>`` (or a bare identifier).
     Strip the prefix before the first colon and match it against the
     inventory's family_id set.
  4. Aggregate per-family hit counts.

Measurement mode (output column ``measurement_mode``) — critical methodology
note carried into ac-08:

  * ``direct``    — family emits a ``<family_id>:<header>`` rule string from
                    ``column.rs``; hits are observable from profile output.
  * ``internal``  — family fires inside ``header_hint()`` (the match_table or
                    the per-domain substring matchers) and does NOT emit its
                    own rule prefix. Hits surface via caller families
                    (header_hint, header_hint_hardcoded, header_hint_generic,
                    header_hint_fallback, header_hint_cross_domain,
                    header_hint_same_category). ac-04's counterfactual is the
                    measurement mechanism for these — disabling the internal
                    family and diffing predictions reveals real hit volume.

Per spec constraint #3, granularity is rule-family, not per-arm.
"""

from __future__ import annotations

import argparse
import csv
import json
import subprocess
import sys
from collections import defaultdict
from dataclasses import dataclass
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
INVENTORY_TSV = REPO_ROOT / "diagnostics" / "rhh_family_inventory.tsv"
DEFAULT_MANIFEST = REPO_ROOT / "eval" / "datasets" / "manifest.csv"
DEFAULT_MODEL = REPO_ROOT / "models" / "default"
DEFAULT_BINARY = REPO_ROOT / "target" / "release" / "finetype"
DEFAULT_OUTPUT = REPO_ROOT / "diagnostics" / "rhh_hit_counts.tsv"

# Families that fire inside header_hint() itself — they do not emit a dedicated
# disambiguation_rule prefix, so hit_count is reported as 0 with
# measurement_mode=internal. Counterfactual measurement (ac-04) captures their
# true hit volume.
INTERNAL_FAMILIES = frozenset(
    [
        "header_hint_table",
        "substring_matcher_identity",
        "substring_matcher_technology",
        "substring_matcher_geography",
        "substring_matcher_datetime",
        "substring_matcher_finance",
        "substring_matcher_representation",
    ]
)


@dataclass(frozen=True)
class Family:
    family_id: str
    rule_family_class: str
    keep_required: bool


def load_inventory(path: Path) -> list[Family]:
    """Parse the ac-01 inventory TSV, ignoring comment-prefixed schema lines."""
    families: list[Family] = []
    with path.open(newline="", encoding="utf-8") as fh:
        lines = [ln for ln in fh if not ln.startswith("#")]
    reader = csv.DictReader(lines, delimiter="\t")
    for row in reader:
        families.append(
            Family(
                family_id=row["family_id"],
                rule_family_class=row["rule_family_class"],
                keep_required=(row.get("keep_required", "").strip().lower() == "true"),
            )
        )
    return families


def unique_files(manifest: Path) -> list[tuple[str, Path]]:
    """Return unique (dataset, absolute_file_path) pairs from the manifest."""
    seen: set[tuple[str, str]] = set()
    pairs: list[tuple[str, Path]] = []
    with manifest.open(newline="", encoding="utf-8") as fh:
        reader = csv.DictReader(fh)
        for row in reader:
            ds = row["dataset"]
            fp = row["file_path"]
            key = (ds, fp)
            if key in seen:
                continue
            seen.add(key)
            abs_path = Path(fp)
            if not abs_path.is_absolute():
                abs_path = REPO_ROOT / fp
            pairs.append((ds, abs_path))
    return pairs


def profile_file(binary: Path, model: Path, file_path: Path) -> dict | None:
    """Invoke finetype profile and return the parsed JSON, or None on error."""
    cmd = [
        str(binary),
        "profile",
        "-f",
        str(file_path),
        "-o",
        "json",
        "--model",
        str(model),
    ]
    try:
        proc = subprocess.run(
            cmd, capture_output=True, text=True, check=True, timeout=300
        )
    except subprocess.CalledProcessError as exc:
        print(
            f"  ! profile failed for {file_path.name}: {exc.stderr.strip()[:200]}",
            file=sys.stderr,
        )
        return None
    except subprocess.TimeoutExpired:
        print(f"  ! profile timed out for {file_path.name}", file=sys.stderr)
        return None
    try:
        return json.loads(proc.stdout)
    except json.JSONDecodeError as exc:
        print(f"  ! malformed JSON for {file_path.name}: {exc}", file=sys.stderr)
        return None


def extract_prefix(rule: str) -> str:
    """Return the part of the rule before the first ':', or the whole rule."""
    idx = rule.find(":")
    return rule if idx < 0 else rule[:idx]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", type=Path, default=DEFAULT_MANIFEST)
    parser.add_argument("--model", type=Path, default=DEFAULT_MODEL)
    parser.add_argument("--binary", type=Path, default=DEFAULT_BINARY)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--inventory", type=Path, default=INVENTORY_TSV)
    args = parser.parse_args()

    if not args.binary.exists():
        print(f"finetype binary not found: {args.binary}", file=sys.stderr)
        print("build with: cargo build --release -p finetype-cli", file=sys.stderr)
        return 2

    families = load_inventory(args.inventory)
    family_ids = {f.family_id for f in families}

    # Prefix -> family_id. For direct-mode families, prefix == family_id.
    prefix_to_family = {f.family_id: f.family_id for f in families if f.family_id not in INTERNAL_FAMILIES}

    files = unique_files(args.manifest)
    print(f"rhh ac-03: profiling {len(files)} unique files with {args.model.name}")

    hit_counts: dict[str, int] = defaultdict(int)
    other_prefixes: dict[str, int] = defaultdict(int)
    columns_total = 0
    columns_with_rule = 0
    files_profiled = 0
    files_failed = 0

    for ds, fp in files:
        if not fp.exists():
            print(f"  ! missing: {fp}", file=sys.stderr)
            files_failed += 1
            continue
        print(f"  → {ds} ({fp.name})")
        result = profile_file(args.binary, args.model, fp)
        if result is None:
            files_failed += 1
            continue
        files_profiled += 1
        for col in result.get("columns", []):
            columns_total += 1
            rule = col.get("disambiguation_rule")
            if not rule:
                continue
            columns_with_rule += 1
            prefix = extract_prefix(rule)
            if prefix in prefix_to_family:
                hit_counts[prefix_to_family[prefix]] += 1
            elif prefix in family_ids:
                # Safety net — shouldn't happen because INTERNAL_FAMILIES don't
                # emit rule prefixes, but if one ever does, count it.
                hit_counts[prefix] += 1
            else:
                other_prefixes[prefix] += 1

    print(
        f"\n  profiled {files_profiled}/{len(files)} files"
        f" ({files_failed} failed);"
        f" {columns_total} columns total,"
        f" {columns_with_rule} with disambiguation_rule"
    )

    # Write TSV
    args.output.parent.mkdir(parents=True, exist_ok=True)
    schema_header = (
        "# rhh_hit_counts.tsv — ac-03 output\n"
        f"# Source: {args.binary.relative_to(REPO_ROOT)} profile"
        f" over {args.manifest.relative_to(REPO_ROOT)}\n"
        f"# Model: {args.model.relative_to(REPO_ROOT)}\n"
        f"# Files profiled: {files_profiled}/{len(files)};"
        f" columns total: {columns_total};"
        f" columns with disambiguation_rule: {columns_with_rule}\n"
        "# Schema: family_id\\thit_count\\tmeasurement_mode\\trule_family_class\\tkeep_required\\tnote\n"
        "# measurement_mode:\n"
        "#   direct   — family emits <family_id>:<header> rule string;"
        " hit_count is the observed prefix-match count.\n"
        "#   internal — family fires inside header_hint() without a dedicated"
        " rule prefix; hit_count=0 here.\n"
        "#                true hit volume is measured via ac-04 counterfactual.\n"
    )
    with args.output.open("w", encoding="utf-8") as fh:
        fh.write(schema_header)
        fh.write(
            "family_id\thit_count\tmeasurement_mode\trule_family_class"
            "\tkeep_required\tnote\n"
        )
        for fam in families:
            mode = "internal" if fam.family_id in INTERNAL_FAMILIES else "direct"
            count = 0 if mode == "internal" else hit_counts.get(fam.family_id, 0)
            note = (
                "hits surface via caller families; measured by counterfactual (ac-04)"
                if mode == "internal"
                else ""
            )
            fh.write(
                f"{fam.family_id}\t{count}\t{mode}\t{fam.rule_family_class}"
                f"\t{'true' if fam.keep_required else 'false'}\t{note}\n"
            )

    print(f"\n  wrote {args.output.relative_to(REPO_ROOT)}")

    if other_prefixes:
        print(
            f"\n  {sum(other_prefixes.values())} disambiguation_rule hits with"
            " prefixes outside RHH inventory scope (expected — non-header-hint rules):"
        )
        for prefix, count in sorted(
            other_prefixes.items(), key=lambda kv: -kv[1]
        )[:15]:
            print(f"    {prefix}\t{count}")

    return 0 if files_failed == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
