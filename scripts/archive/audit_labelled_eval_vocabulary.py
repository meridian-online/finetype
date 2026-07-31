#!/usr/bin/env python3
"""ac-11 vocabulary precondition — assert truth_mechanism in closed set.

Enumerates distinct `truth_mechanism` values in
`labelled_eval.tsv` and asserts each is in the MADR 0075 + 0081
closed-10 mechanism token set. If a future relabelling introduces an
unknown value, this audit fails fast; remediation (renormalisation
mapping or amend closed set) is a follow-up spec, not ac-11.

USAGE
    python3 scripts/audit_labelled_eval_vocabulary.py
    python3 scripts/audit_labelled_eval_vocabulary.py --tsv /path/to/labelled_eval.tsv
"""
from __future__ import annotations

import argparse
import csv
import json
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent

CLOSED_MECHANISMS = frozenset({
    "format_diversity_path_a", "format_diversity_path_b",
    "code_vs_canonical_path_a", "code_vs_canonical_path_b",
    "enum_overfit", "misclassification", "prediction_confirmed",
    "validator_widening", "unknown_no_fit", "fallthrough",
})


def main() -> int:
    p = argparse.ArgumentParser(description=(__doc__ or "").splitlines()[0])
    p.add_argument(
        "--tsv", type=Path,
        default=REPO / "spec 2026-05-04-autonomous-type-inference (labelled_eval.tsv)",
    )
    args = p.parse_args()

    if not args.tsv.exists():
        print(f"error: {args.tsv} not found", file=sys.stderr)
        return 2

    distinct: dict[str, int] = {}
    n_rows = 0
    with args.tsv.open(newline="") as f:
        reader = csv.DictReader(f, delimiter="\t")
        if "truth_mechanism" not in (reader.fieldnames or []):
            print(f"error: no truth_mechanism column in {args.tsv}",
                  file=sys.stderr)
            return 2
        for row in reader:
            n_rows += 1
            tm = (row.get("truth_mechanism") or "").strip()
            if tm:
                distinct[tm] = distinct.get(tm, 0) + 1

    unknown = sorted(set(distinct) - CLOSED_MECHANISMS)
    out = {
        "n_rows_total": n_rows,
        "n_distinct_truth_mechanisms": len(distinct),
        "distinct_truth_mechanisms": dict(
            sorted(distinct.items(), key=lambda kv: (-kv[1], kv[0]))
        ),
        "closed_set_size": len(CLOSED_MECHANISMS),
        "unknown_truth_mechanisms": unknown,
        "n_rows_excluded": sum(distinct[u] for u in unknown),
    }
    print(json.dumps(out, indent=2))
    if unknown:
        print(
            f"\nERROR: {len(unknown)} truth_mechanism value(s) outside the "
            f"closed 10-token set: {unknown}\n"
            "Remediation: commit a renormalisation mapping or amend "
            "the closed set via a follow-up spec.",
            file=sys.stderr,
        )
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
