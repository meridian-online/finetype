#!/usr/bin/env python3
"""ac-07 — roadmap synthesis.

Joins ac-01 (inventory) + ac-05 (classification) into the final roadmap TSV,
one row per family_id.

Proposed-stage rules (spec ac-07):
  * Default: the family's ``domain`` field is the proposed stage.
  * Cross-domain families get ``cross-domain-stage`` unless a more-specific
    driver is recorded.

blocked_on rules (spec ac-07):
  * Non-model-gap classifications: empty string.
  * model-gap with domain=="finance" AND type_targets contains "amount_"
    → ``v19-retrain`` (MADR 0065: 11 amount-variant arms shipped).
  * All other model-gap → ``training-data-fortification-<primary-target>``
    where primary-target == first entry in type_targets (';' split).

Secondary targets are recorded in the notes field.

Deliverable: diagnostics/rhh_roadmap.tsv
"""

from __future__ import annotations

import argparse
import csv
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_INVENTORY = REPO_ROOT / "diagnostics" / "rhh_family_inventory.tsv"
DEFAULT_CLASSIFICATION = REPO_ROOT / "diagnostics" / "rhh_classification.tsv"
DEFAULT_OUTPUT = REPO_ROOT / "diagnostics" / "rhh_roadmap.tsv"

VALID_STAGES = {
    "container",
    "datetime",
    "finance",
    "geography",
    "identity",
    "representation",
    "technology",
    "cross-domain-stage",
}


def _strip_comments(path: Path) -> list[str]:
    with path.open(newline="", encoding="utf-8") as fh:
        return [ln for ln in fh if not ln.startswith("#")]


def proposed_stage(domain: str) -> str:
    if domain == "cross-domain":
        return "cross-domain-stage"
    return domain


def blocked_on(
    classification: str, domain: str, type_targets: str
) -> tuple[str, str]:
    """Return (blocked_on, notes_extra). ``domain`` is accepted for spec
    parity but not currently consulted — v19-retrain gates on type_targets."""
    _ = domain  # unused; retained in signature for schema stability
    if classification != "model-gap":
        return ("", "")
    targets = [t.strip() for t in type_targets.split(";") if t.strip()]
    primary = targets[0] if targets else ""
    secondaries = targets[1:]
    # v19-retrain gate: any family that targets finance.currency.amount_*
    # variants (MADR 0065 shipped the 11-arm match-table addition; the
    # underlying training-data gap is what blocks removal). Catches both
    # substring_matcher_finance (domain=finance) and header_hint_table
    # (domain=cross-domain) when they reach for amount_* variants.
    if any("finance.currency.amount_" in t for t in targets):
        return ("v19-retrain", "")
    ticket = (
        f"training-data-fortification-{primary}"
        if primary
        else "training-data-fortification-unassigned"
    )
    notes_extra = (
        f"secondary targets: {'; '.join(secondaries)}" if secondaries else ""
    )
    return (ticket, notes_extra)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--inventory", type=Path, default=DEFAULT_INVENTORY)
    parser.add_argument(
        "--classification", type=Path, default=DEFAULT_CLASSIFICATION
    )
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args()

    inv = list(csv.DictReader(_strip_comments(args.inventory), delimiter="\t"))
    classes = {
        r["family_id"]: r
        for r in csv.DictReader(
            _strip_comments(args.classification), delimiter="\t"
        )
    }

    args.output.parent.mkdir(parents=True, exist_ok=True)
    header = [
        "# rhh_roadmap.tsv — ac-07 output (final artefact)",
        f"# Source: {args.inventory.relative_to(REPO_ROOT)}"
        f" + {args.classification.relative_to(REPO_ROOT)}",
        "# proposed_stage: domain of family, or 'cross-domain-stage'.",
        "# blocked_on:",
        "#   empty         — not model-gap; removable (or already kept).",
        "#   v19-retrain   — model-gap finance.currency.amount_* variant.",
        "#   training-data-fortification-<type> — model-gap otherwise;",
        "#     primary target from type_targets[0].",
        "# Schema: family_id\\tfile\\tline_start\\tline_end\\tdomain"
        "\\ttype_targets\\tclassification\\tproposed_stage"
        "\\tblocked_on\\tnotes",
    ]
    cols = [
        "family_id",
        "file",
        "line_start",
        "line_end",
        "domain",
        "type_targets",
        "classification",
        "proposed_stage",
        "blocked_on",
        "notes",
    ]
    with args.output.open("w", encoding="utf-8") as fh:
        for line in header:
            fh.write(line + "\n")
        fh.write("\t".join(cols) + "\n")
        for row in inv:
            fam = row["family_id"]
            cls_row = classes.get(fam)
            classification = cls_row["classification"] if cls_row else "no-hit"
            stage = proposed_stage(row["domain"])
            assert stage in VALID_STAGES, f"invalid stage {stage!r} for {fam}"
            block, notes_extra = blocked_on(
                classification, row["domain"], row["type_targets"]
            )
            notes_parts: list[str] = []
            if row.get("keep_required", "").strip().lower() == "true":
                notes_parts.append(
                    "keep_required (spec constraint #2 — not removable)"
                )
            if notes_extra:
                notes_parts.append(notes_extra)
            if cls_row and cls_row.get("notes"):
                # Avoid duplicating the keep_required note from ac-05.
                existing = cls_row["notes"].strip()
                if existing and existing not in notes_parts:
                    if existing != "keep_required (spec constraint #2)":
                        notes_parts.append(existing)
            notes = " | ".join(notes_parts)
            fh.write(
                f"{fam}\t{row['file']}\t{row['line_start']}\t{row['line_end']}"
                f"\t{row['domain']}\t{row['type_targets']}\t{classification}"
                f"\t{stage}\t{block}\t{notes}\n"
            )

    # Console summary.
    print(
        f"rhh ac-07: roadmap written to {args.output.relative_to(REPO_ROOT)}"
        f" ({len(inv)} families)"
    )
    tally_by_block: dict[str, int] = {}
    tally_by_stage: dict[str, int] = {}
    for row in inv:
        fam = row["family_id"]
        classification = (
            classes.get(fam, {}).get("classification", "no-hit")
        )
        block, _ = blocked_on(
            classification, row["domain"], row["type_targets"]
        )
        stage = proposed_stage(row["domain"])
        tally_by_block[block or "(removable)"] = (
            tally_by_block.get(block or "(removable)", 0) + 1
        )
        tally_by_stage[stage] = tally_by_stage.get(stage, 0) + 1

    print("  blocked_on:")
    for key, n in sorted(tally_by_block.items()):
        print(f"    {key}\t{n}")
    print("  proposed_stage:")
    for key, n in sorted(tally_by_stage.items()):
        print(f"    {key}\t{n}")

    return 0


if __name__ == "__main__":
    import sys
    sys.exit(main())
