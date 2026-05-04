#!/usr/bin/env python3
"""
compute_precision_on_labelled.py — join labelled_eval.tsv with
labelled_eval.module_predictions.tsv on (cycle_id, file_path,
file_content_sha256, column_name) and report precision-on-labelled
per spec ac-13 / progress.md.

Output: text table to stdout suitable for pasting into progress.md.
"""

from __future__ import annotations

import argparse
import csv
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
DEFAULT_LABELLED = (
    REPO_ROOT
    / "orbit"
    / "specs"
    / "2026-05-04-autonomous-type-inference"
    / "labelled_eval.tsv"
)
DEFAULT_MODULE_PREDS = (
    REPO_ROOT
    / "orbit"
    / "specs"
    / "2026-05-04-autonomous-type-inference"
    / "labelled_eval.module_predictions.tsv"
)


def join_key(r: dict) -> tuple:
    return (
        r.get("cycle_id", ""),
        r.get("file_path", ""),
        r.get("file_content_sha256", ""),
        r.get("column_name", ""),
    )


def main(labelled: Path, module_preds: Path) -> int:
    if not labelled.exists():
        print(f"error: not found: {labelled}", file=sys.stderr)
        return 1
    if not module_preds.exists():
        print(f"error: not found: {module_preds}", file=sys.stderr)
        return 1

    with labelled.open("r", encoding="utf-8", newline="") as f:
        labelled_rows = list(csv.DictReader(f, delimiter="\t"))
    with module_preds.open("r", encoding="utf-8", newline="") as f:
        module_rows = list(csv.DictReader(f, delimiter="\t"))

    module_by_key = {join_key(r): r for r in module_rows}

    # Counters
    total = len(labelled_rows)
    matched = 0
    type_match = 0
    mech_match = 0
    type_match_at_07 = 0
    type_match_decisive = 0  # non-unknown module output
    decisive_total = 0
    truth_unknown = 0

    # Per-threshold buckets for precision-on-labelled at varying conf
    thresholds = [0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9]
    by_thresh: dict[float, dict[str, int]] = {
        t: {"decisive": 0, "type_match": 0} for t in thresholds
    }

    # Confusion: rows where module says non-unknown vs truth says unknown
    module_overconfident = 0
    module_underdecisive = 0  # module says unknown when truth is specific

    for lr in labelled_rows:
        key = join_key(lr)
        mr = module_by_key.get(key)
        if mr is None:
            continue
        matched += 1
        truth_t = lr.get("truth_inferred_type", "")
        truth_m = lr.get("truth_mechanism", "")
        mod_t = mr.get("module_inferred_type", "")
        mod_m = mr.get("module_mechanism", "")
        try:
            mod_c = float(mr.get("module_confidence", "0") or 0)
        except ValueError:
            mod_c = 0.0

        if truth_t == "unknown":
            truth_unknown += 1

        if truth_t == mod_t:
            type_match += 1
        if truth_m == mod_m:
            mech_match += 1

        if mod_t != "unknown":
            decisive_total += 1
            if truth_t == mod_t:
                type_match_decisive += 1

            if mod_c >= 0.7:
                if truth_t == mod_t:
                    type_match_at_07 += 1

        if mod_t != "unknown" and truth_t == "unknown":
            module_overconfident += 1
        if mod_t == "unknown" and truth_t != "unknown":
            module_underdecisive += 1

        for t in thresholds:
            if mod_c >= t and mod_t != "unknown":
                by_thresh[t]["decisive"] += 1
                if truth_t == mod_t:
                    by_thresh[t]["type_match"] += 1

    print(f"## Precision-on-labelled (ac-13)\n")
    print(f"- labelled rows: {total}")
    print(f"- joined with module predictions: {matched}")
    if matched == 0:
        return 2
    print(f"- truth_inferred_type == 'unknown': {truth_unknown}")
    print(f"- module decisive (non-unknown): {decisive_total}")
    print()
    print("### Overall agreement\n")
    print(
        f"- type-match (truth_inferred_type == module_inferred_type): "
        f"{type_match}/{matched} = {type_match/matched:.3%}"
    )
    print(
        f"- mechanism-match (truth_mechanism == module_mechanism): "
        f"{mech_match}/{matched} = {mech_match/matched:.3%}"
    )
    if decisive_total > 0:
        print(
            f"- precision on decisive: {type_match_decisive}/{decisive_total} = "
            f"{type_match_decisive/decisive_total:.3%}"
        )
    print()
    print("### Precision at varying confidence thresholds\n")
    print("| threshold | decisive | correct | precision |")
    print("|-----------|----------|---------|-----------|")
    for t in thresholds:
        d = by_thresh[t]["decisive"]
        c = by_thresh[t]["type_match"]
        p = c / d if d > 0 else float("nan")
        print(f"| {t} | {d} | {c} | {p:.3%}" if d > 0 else f"| {t} | 0 | 0 | n/a |")
    print()
    print("### Confusion edges\n")
    print(
        f"- module decisive but truth=unknown (over-confident): {module_overconfident}"
    )
    print(
        f"- module unknown but truth=specific (under-decisive): {module_underdecisive}"
    )
    return 0


if __name__ == "__main__":
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--labelled", type=Path, default=DEFAULT_LABELLED)
    p.add_argument("--module-preds", type=Path, default=DEFAULT_MODULE_PREDS)
    args = p.parse_args()
    sys.exit(main(args.labelled, args.module_preds))
