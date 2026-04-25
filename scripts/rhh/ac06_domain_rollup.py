#!/usr/bin/env python3
"""ac-06 — domain rollup of family classifications.

Aggregates ac-05 classifications by the family's ``domain`` field from the
ac-01 inventory. Emits one row per domain (7 taxonomy domains + cross-domain,
unless the inventory has zero cross-domain families).

removal_readiness:
  * READY               — model_gap == 0 AND total_families > 0
  * NEEDS_FORTIFICATION — model_gap > 0
  * NO_EVIDENCE         — total_families == no_hit (zero measured impact)

Deliverable: diagnostics/rhh_domain_rollup.tsv
"""

from __future__ import annotations

import argparse
import csv
from collections import defaultdict
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_INVENTORY = REPO_ROOT / "diagnostics" / "rhh_family_inventory.tsv"
DEFAULT_CLASSIFICATION = REPO_ROOT / "diagnostics" / "rhh_classification.tsv"
DEFAULT_OUTPUT = REPO_ROOT / "diagnostics" / "rhh_domain_rollup.tsv"

# The 7 taxonomy domains listed in spec constraint set (all always emitted,
# even if zero families fall under them).
TAXONOMY_DOMAINS = (
    "container",
    "datetime",
    "finance",
    "geography",
    "identity",
    "representation",
    "technology",
)
CROSS_DOMAIN = "cross-domain"


def _strip_comments(path: Path) -> list[str]:
    with path.open(newline="", encoding="utf-8") as fh:
        return [ln for ln in fh if not ln.startswith("#")]


def load_inventory_domains(path: Path) -> dict[str, str]:
    reader = csv.DictReader(_strip_comments(path), delimiter="\t")
    # If a family_id spans multiple domains in the TSV, take the first token
    # before any ';' (inventory uses ';' to join multi-type lists within a
    # single domain). The ``domain`` column is authoritative.
    return {r["family_id"]: r["domain"] for r in reader}


def load_classifications(path: Path) -> dict[str, str]:
    reader = csv.DictReader(_strip_comments(path), delimiter="\t")
    return {r["family_id"]: r["classification"] for r in reader}


def readiness(
    total: int, model_gap: int, no_hit: int
) -> str:
    if total == 0:
        return "NO_EVIDENCE"
    if total == no_hit:
        return "NO_EVIDENCE"
    if model_gap > 0:
        return "NEEDS_FORTIFICATION"
    return "READY"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--inventory", type=Path, default=DEFAULT_INVENTORY)
    parser.add_argument(
        "--classification", type=Path, default=DEFAULT_CLASSIFICATION
    )
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args()

    fam_domain = load_inventory_domains(args.inventory)
    fam_class = load_classifications(args.classification)

    counts: dict[str, dict[str, int]] = defaultdict(
        lambda: {"total": 0, "model-covered": 0, "model-gap": 0, "no-hit": 0}
    )
    for family_id, domain in fam_domain.items():
        klass = fam_class.get(family_id, "no-hit")
        counts[domain]["total"] += 1
        counts[domain][klass] += 1

    # Ensure all 7 taxonomy domains appear, even with zero families.
    domain_rows: list[str] = list(TAXONOMY_DOMAINS)
    has_cross = counts.get(CROSS_DOMAIN, {}).get("total", 0) > 0
    if has_cross:
        domain_rows.append(CROSS_DOMAIN)

    args.output.parent.mkdir(parents=True, exist_ok=True)
    header = [
        "# rhh_domain_rollup.tsv — ac-06 output",
        f"# Source: {args.inventory.relative_to(REPO_ROOT)}"
        f" + {args.classification.relative_to(REPO_ROOT)}",
        "# removal_readiness:",
        "#   READY               — model_gap == 0 AND at least one non-no-hit family.",
        "#   NEEDS_FORTIFICATION — model_gap > 0.",
        "#   NO_EVIDENCE         — all families no-hit OR total_families == 0.",
        "# Schema: domain\\ttotal_families\\tmodel_covered\\tmodel_gap"
        "\\tno_hit\\tremoval_readiness",
    ]
    with args.output.open("w", encoding="utf-8") as fh:
        for line in header:
            fh.write(line + "\n")
        fh.write(
            "domain\ttotal_families\tmodel_covered\tmodel_gap"
            "\tno_hit\tremoval_readiness\n"
        )
        for dom in domain_rows:
            c = counts.get(dom, {"total": 0, "model-covered": 0, "model-gap": 0, "no-hit": 0})
            ready = readiness(c["total"], c["model-gap"], c["no-hit"])
            fh.write(
                f"{dom}\t{c['total']}\t{c['model-covered']}\t{c['model-gap']}"
                f"\t{c['no-hit']}\t{ready}\n"
            )

    total_rows = len(domain_rows)
    print(
        f"rhh ac-06: domain rollup written to"
        f" {args.output.relative_to(REPO_ROOT)} ({total_rows} rows)"
    )
    for dom in domain_rows:
        c = counts.get(dom, {"total": 0, "model-covered": 0, "model-gap": 0, "no-hit": 0})
        ready = readiness(c["total"], c["model-gap"], c["no-hit"])
        print(
            f"  {dom:<16} total={c['total']:<2}"
            f"  covered={c['model-covered']:<2}"
            f"  gap={c['model-gap']:<2}"
            f"  no_hit={c['no-hit']:<2}  {ready}"
        )

    # Sanity: sum(total) equals total families counted.
    total_families = sum(counts[d]["total"] for d in domain_rows)
    print(f"\n  total families across domains: {total_families}")
    return 0


if __name__ == "__main__":
    import sys
    sys.exit(main())
