#!/usr/bin/env python3
"""Audit the current YDF training corpus for gold-anchor leakage — by IDENTITY.

Spec 2026-06-05-gold-eval-anchor ac-06.

The sibling audit `audit_ydf_training_leakage.py` keys on value-hash (catches a
labelled_eval value tuple that slipped into training). This one keys on the
durable (file_content_sha256, column_name) identity the gold fixture carries,
so a gold column is caught regardless of how it was later sampled.

"Current training corpus" = the rows train_ydf.py actually trains on: the
Sherlock-distilled CSV plus the synthetic generator output. Neither carries a
GitTables (file, column) identity today, so the expected overlap is 0 by
construction — this audit is the standing proof of that, and it will start
catching real collisions the moment a GitTables-derived source enters training
(the judge->miner transition this whole spec exists to keep honest).

Output: output/gold-eval-anchor/leakage_audit_identity.json
        {n_gold, n_training_with_identity, n_overlap, overlap: [...]}
Exits non-zero if n_overlap > 0.

Usage:
  python3 scripts/audit_gold_anchor_leakage.py
"""
from __future__ import annotations

import argparse
import csv
import gzip
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from gold_anchor_guard import (  # noqa: E402
    DEFAULT_GOLD,
    Identity,
    load_gold_identities,
)

REPO = Path(__file__).resolve().parent.parent
DEFAULT_DISTILLED = REPO / "output" / "distillation-v3" / "sherlock_distilled.csv.gz"
DEFAULT_OUTPUT = REPO / "output" / "gold-eval-anchor" / "leakage_audit_identity.json"


def _distilled_identities(distilled: Path) -> set[Identity]:
    """The (file, column) identities the distilled training rows carry. The
    Sherlock corpus is keyed on `sherlock_index` and has no such columns, so
    this is empty today — extracting it explicitly is what makes the audit
    fire automatically if the corpus ever gains GitTables provenance."""
    ids: set[Identity] = set()
    if not distilled.exists():
        return ids
    with gzip.open(distilled, "rt") as fh:
        for row in csv.DictReader(fh):
            sha = (row.get("file_content_sha256") or "").strip()
            col = (row.get("column_name") or "").strip()
            if sha and col:
                ids.add((sha, col))
    return ids


def main() -> int:
    ap = argparse.ArgumentParser(
        description="Audit YDF training corpus for gold-anchor identity leakage."
    )
    ap.add_argument("--gold", type=Path, default=DEFAULT_GOLD)
    ap.add_argument("--distilled", type=Path, default=DEFAULT_DISTILLED)
    ap.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    args = ap.parse_args()

    gold = load_gold_identities(args.gold)
    if not gold:
        print(f"error: no gold identities loaded from {args.gold}", file=sys.stderr)
        return 2

    training = _distilled_identities(args.distilled)
    overlap = gold & training

    summary = {
        "n_gold": len(gold),
        "n_training_with_identity": len(training),
        "n_overlap": len(overlap),
        "overlap": sorted(f"{s}:{c}" for s, c in overlap),
        "gold_path": str(args.gold),
        "distilled_path": str(args.distilled),
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(summary, indent=2) + "\n")
    print(json.dumps(summary, indent=2))

    if summary["n_overlap"] > 0:
        print(
            f"FAIL: {summary['n_overlap']} gold columns leaked into the "
            f"training corpus by (file, column) identity", file=sys.stderr,
        )
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
