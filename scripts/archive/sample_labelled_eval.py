#!/usr/bin/env python3
"""
sample_labelled_eval.py — sample 200 rows from failure_log.measure.tsv
for hand-labelling per spec ac-13 / labelling_protocol.md.

Output:
  spec 2026-05-04-autonomous-type-inference (labelled_eval.unlabelled.tsv)

Rules:
- Source: eval/gittables/failure_log.measure.tsv (bucket 1, never bucket 0)
- Stratification: ≥30 distinct predicted_types
- Per-stratum allocation: ≥5 rows per predicted_type until 30-type quota met
- Deterministic seed: 20260504
- Output columns: original 9 from failure_log + 4 empty for hand-labelling
  (truth_inferred_type, truth_mechanism, labeller, note)
"""

from __future__ import annotations

import argparse
import collections
import csv
import random
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
DEFAULT_SOURCE = REPO_ROOT / "eval" / "gittables" / "failure_log.measure.tsv"
DEFAULT_OUTPUT = (
    REPO_ROOT
    / "orbit"
    / "specs"
    / "2026-05-04-autonomous-type-inference"
    / "labelled_eval.unlabelled.tsv"
)

ORIGINAL_COLS = [
    "cycle_id",
    "timestamp",
    "file_path",
    "file_content_sha256",
    "column_name",
    "predicted_type",
    "observed_values_sample",
    "inferred_correct_type",
    "mechanism",
]
LABEL_COLS = ["truth_inferred_type", "truth_mechanism", "labeller", "note"]
TARGET_TOTAL = 200
TARGET_DISTINCT_TYPES = 30
PER_TYPE_FLOOR = 5
SAMPLING_SEED = 20260504


def main(source: Path, output: Path, seed: int = SAMPLING_SEED) -> int:
    if not source.exists():
        print(f"error: source not found: {source}", file=sys.stderr)
        return 1

    with source.open("r", encoding="utf-8", newline="") as f:
        reader = csv.DictReader(f, delimiter="\t")
        rows = list(reader)

    print(f"source rows: {len(rows)}")

    by_type: dict[str, list[dict]] = collections.defaultdict(list)
    for r in rows:
        by_type[r.get("predicted_type", "")].append(r)

    print(f"distinct predicted_types in source: {len(by_type)}")
    if len(by_type) < TARGET_DISTINCT_TYPES:
        print(
            f"error: only {len(by_type)} distinct predicted_types in source; "
            f"need ≥{TARGET_DISTINCT_TYPES}",
            file=sys.stderr,
        )
        return 2

    rng = random.Random(seed)

    # Sort types by frequency (descending) for proportional fill after the
    # 30-type quota is met. Tie-break lex for determinism.
    type_freq = sorted(
        ((t, len(rs)) for t, rs in by_type.items()),
        key=lambda x: (-x[1], x[0]),
    )

    sampled: list[dict] = []
    sampled_keys: set[tuple] = set()

    # Phase 1: meet the 30-type quota with PER_TYPE_FLOOR rows each.
    quota_types = [t for t, _ in type_freq[:TARGET_DISTINCT_TYPES]]
    for t in quota_types:
        candidates = list(by_type[t])
        rng.shuffle(candidates)
        take = candidates[: PER_TYPE_FLOOR]
        for r in take:
            key = (r["cycle_id"], r["file_path"], r["file_content_sha256"], r["column_name"])
            if key in sampled_keys:
                continue
            sampled.append(r)
            sampled_keys.add(key)

    # Phase 2: fill the rest proportionally to frequency.
    remaining_budget = TARGET_TOTAL - len(sampled)
    if remaining_budget > 0:
        # Pool of all rows EXCEPT those already sampled, weighted by type
        # frequency.
        pool: list[dict] = []
        for t, _ in type_freq:
            for r in by_type[t]:
                key = (r["cycle_id"], r["file_path"], r["file_content_sha256"], r["column_name"])
                if key not in sampled_keys:
                    pool.append(r)
        rng.shuffle(pool)
        for r in pool[:remaining_budget]:
            sampled.append(r)
            key = (r["cycle_id"], r["file_path"], r["file_content_sha256"], r["column_name"])
            sampled_keys.add(key)

    # Verify
    distinct_in_sample = len({r["predicted_type"] for r in sampled})
    print(
        f"sampled: {len(sampled)} rows; distinct predicted_types: {distinct_in_sample}"
    )
    if distinct_in_sample < TARGET_DISTINCT_TYPES:
        print(
            f"error: distinct types in sample ({distinct_in_sample}) < target "
            f"({TARGET_DISTINCT_TYPES})",
            file=sys.stderr,
        )
        return 3
    if len(sampled) < TARGET_TOTAL:
        print(
            f"warning: sampled {len(sampled)} < target {TARGET_TOTAL} "
            "(likely low source diversity)",
            file=sys.stderr,
        )

    output.parent.mkdir(parents=True, exist_ok=True)
    with output.open("w", encoding="utf-8", newline="") as f:
        writer = csv.DictWriter(
            f,
            fieldnames=ORIGINAL_COLS + LABEL_COLS,
            delimiter="\t",
            lineterminator="\n",
        )
        writer.writeheader()
        for r in sampled:
            writer.writerow({**{c: r.get(c, "") for c in ORIGINAL_COLS}, **{c: "" for c in LABEL_COLS}})

    print(f"wrote {output}")
    return 0


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, default=DEFAULT_SOURCE)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--seed", type=int, default=SAMPLING_SEED)
    args = parser.parse_args()
    sys.exit(main(args.source, args.output, args.seed))
