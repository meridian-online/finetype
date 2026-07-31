#!/usr/bin/env python3
"""Audit YDF training set for overlap with labelled_eval.

Spec: `2026-05-20-gittables-multi-lens-diagnostic`
ac-03 (leakage audit).

For each row in `labelled_eval.tsv`, computes a value-hash over its
`observed_values_sample` (split on the U+2502 separator that the file
uses). For each row in `training_rows_manifest.tsv`, reads the
pre-computed `value_hash`. Reports the set intersection — overlap MUST
be 0 (the synthetic generator produces de-novo values that should
never collide with gittables column samples; the audit is insurance).

Output: `eval/gittables/ydf_leakage_audit.json` with
`{n_training_rows, n_labelled_eval_rows, n_overlap, overlap_hashes:
[...]}`. Exits non-zero if `n_overlap > 0`.

Usage:
  python3 scripts/audit_ydf_training_leakage.py
"""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
DEFAULT_MANIFEST = REPO / "eval" / "gittables" / "models" / "training_rows_manifest.tsv"
DEFAULT_LABELLED_EVAL = (
    REPO / ".orbit" / "specs" / "2026-05-04-autonomous-type-inference"
    / "labelled_eval.tsv"
)
DEFAULT_OUTPUT = REPO / "eval" / "gittables" / "ydf_leakage_audit.json"
SAMPLE_SEPARATOR = "│"  # vertical bar U+2502


def _value_hash(values: list[str]) -> str:
    h = hashlib.sha256()
    for v in values:
        h.update(v.encode("utf-8"))
        h.update(b"\x00")
    return h.hexdigest()


def _training_hashes(manifest: Path) -> set[str]:
    out: set[str] = set()
    with manifest.open() as fh:
        for row in csv.DictReader(fh, delimiter="\t"):
            h = row.get("value_hash")
            if h:
                out.add(h)
    return out


def _labelled_eval_hashes(tsv: Path) -> set[str]:
    out: set[str] = set()
    with tsv.open() as fh:
        for row in csv.DictReader(fh, delimiter="\t"):
            samples_raw = (row.get("observed_values_sample") or "").strip()
            if not samples_raw:
                continue
            values = [
                v for v in samples_raw.split(SAMPLE_SEPARATOR) if v
            ]
            if values:
                out.add(_value_hash(values))
    return out


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Audit YDF training rows for labelled_eval leakage."
    )
    parser.add_argument("--manifest", type=Path, default=DEFAULT_MANIFEST)
    parser.add_argument(
        "--labelled-eval", type=Path, default=DEFAULT_LABELLED_EVAL,
    )
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args()

    if not args.manifest.exists():
        print(f"error: manifest not found: {args.manifest}", file=sys.stderr)
        return 2
    if not args.labelled_eval.exists():
        print(f"error: labelled_eval not found: {args.labelled_eval}",
              file=sys.stderr)
        return 2

    train_hashes = _training_hashes(args.manifest)
    eval_hashes = _labelled_eval_hashes(args.labelled_eval)
    overlap = train_hashes & eval_hashes

    summary = {
        "n_training_rows": len(train_hashes),
        "n_labelled_eval_rows": len(eval_hashes),
        "n_overlap": len(overlap),
        "overlap_hashes": sorted(overlap),
        "manifest_path": str(args.manifest),
        "labelled_eval_path": str(args.labelled_eval),
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(summary, indent=2) + "\n")
    print(json.dumps(summary, indent=2))

    if summary["n_overlap"] > 0:
        print(
            f"FAIL: {summary['n_overlap']} training rows overlap with "
            f"labelled_eval", file=sys.stderr,
        )
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
