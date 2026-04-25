#!/usr/bin/env python3
"""ac-05 — family classification at the 80% threshold.

Joins ac-03 (hit counts) and ac-04 (counterfactual) to classify every family
as one of:

  * ``no-hit``         — columns_hit == 0. Safe to remove with zero measured
                         risk but unverifiable; flagged for next eval-corpus
                         expansion.
  * ``model-covered``  — columns_hit ≥ 1 AND no_hint_label_correct / columns_hit
                         ≥ 0.80. Safe to remove — the multi-branch model holds
                         up without the family.
  * ``model-gap``      — columns_hit ≥ 1 AND accuracy < 0.80. Removal blocked;
                         training-data fortification required.

"Hit" definition (uniform across direct and internal measurement modes):
  A column counts as a hit for family F iff disabling F changes the baseline
  label, i.e. the row's ``label_changed == 1`` in rhh_counterfactual.tsv. This
  is the honest, measurable "family was effective" criterion and applies
  symmetrically to direct-mode families (rule prefix emitted) and internal
  families (hits surface via callers).

  Alternative: for direct families one could use ac-03's rule-prefix hit
  count (77 rows across all direct families). But that overcounts ineffective
  hits (rule fires but label unchanged because the model already agreed).
  The counterfactual-based definition is tighter and uniform.

Deliverable: diagnostics/rhh_classification.tsv
"""

from __future__ import annotations

import argparse
import csv
from collections import defaultdict
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_COUNTERFACTUAL = REPO_ROOT / "diagnostics" / "rhh_counterfactual.tsv"
DEFAULT_INVENTORY = REPO_ROOT / "diagnostics" / "rhh_family_inventory.tsv"
DEFAULT_OUTPUT = REPO_ROOT / "diagnostics" / "rhh_classification.tsv"

THRESHOLD = 0.80


def load_counterfactual(path: Path) -> dict[str, list[dict[str, str]]]:
    rows_by_family: dict[str, list[dict[str, str]]] = defaultdict(list)
    with path.open(newline="", encoding="utf-8") as fh:
        lines = [ln for ln in fh if not ln.startswith("#")]
    reader = csv.DictReader(lines, delimiter="\t")
    for r in reader:
        rows_by_family[r["family_id"]].append(r)
    return rows_by_family


def load_inventory_ids(path: Path) -> list[tuple[str, str, bool]]:
    """Return [(family_id, rule_family_class, keep_required)] in file order."""
    out: list[tuple[str, str, bool]] = []
    with path.open(newline="", encoding="utf-8") as fh:
        lines = [ln for ln in fh if not ln.startswith("#")]
    reader = csv.DictReader(lines, delimiter="\t")
    for r in reader:
        keep = r.get("keep_required", "").strip().lower() == "true"
        out.append((r["family_id"], r["rule_family_class"], keep))
    return out


def classify(columns_hit: int, correct: int) -> tuple[str, float]:
    if columns_hit == 0:
        return ("no-hit", 0.0)
    acc = correct / columns_hit
    return ("model-covered" if acc >= THRESHOLD else "model-gap", acc)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--counterfactual", type=Path, default=DEFAULT_COUNTERFACTUAL
    )
    parser.add_argument("--inventory", type=Path, default=DEFAULT_INVENTORY)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args()

    fams = load_inventory_ids(args.inventory)
    by_fam = load_counterfactual(args.counterfactual)

    args.output.parent.mkdir(parents=True, exist_ok=True)
    header = [
        "# rhh_classification.tsv — ac-05 output",
        f"# Source: {args.counterfactual.relative_to(REPO_ROOT)}",
        f"# Threshold: {THRESHOLD:.2f}",
        "# Hit definition: column where disabling the family changed the",
        "# baseline label (label_changed == 1).",
        "# Schema: family_id\\tclassification\\tcolumns_hit"
        "\\tno_hint_label_correct\\tno_hint_label_accuracy\\trule_family_class"
        "\\tkeep_required\\tnotes",
    ]
    cols_header = (
        "family_id\tclassification\tcolumns_hit\tno_hint_label_correct"
        "\tno_hint_label_accuracy\trule_family_class\tkeep_required\tnotes"
    )
    with args.output.open("w", encoding="utf-8") as fh:
        for line in header:
            fh.write(line + "\n")
        fh.write(cols_header + "\n")
        for family_id, klass, keep in fams:
            rows = by_fam.get(family_id, [])
            hits = [r for r in rows if r.get("label_changed") == "1"]
            columns_hit = len(hits)
            correct = sum(1 for r in hits if r.get("disabled_correct") == "1")
            classification, acc = classify(columns_hit, correct)
            notes: list[str] = []
            if keep:
                notes.append("keep_required (spec constraint #2)")
            if columns_hit == 0 and classification == "no-hit":
                notes.append("zero-impact on current eval")
            fh.write(
                f"{family_id}\t{classification}\t{columns_hit}\t{correct}"
                f"\t{acc:.3f}\t{klass}\t{'true' if keep else 'false'}"
                f"\t{'; '.join(notes)}\n"
            )

    # Summary to stdout
    print(f"rhh ac-05: classified {len(fams)} families @ threshold {THRESHOLD}")
    counts: dict[str, int] = defaultdict(int)
    for family_id, _, _ in fams:
        rows = by_fam.get(family_id, [])
        hits = [r for r in rows if r.get("label_changed") == "1"]
        correct = sum(1 for r in hits if r.get("disabled_correct") == "1")
        c, _ = classify(len(hits), correct)
        counts[c] += 1
    for c in ("model-covered", "model-gap", "no-hit"):
        print(f"  {c}\t{counts[c]}")
    print(f"  wrote {args.output.relative_to(REPO_ROOT)}")
    return 0


if __name__ == "__main__":
    import sys

    sys.exit(main())
