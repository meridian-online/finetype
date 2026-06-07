#!/usr/bin/env python3
"""Assemble manufactured value-level reference data into synthetic *columns* for the
B3 fusion head's training corpus.

Spec 2026-06-08-late-fusion-sense-classifier, ac-04 SALVAGE. The first head (fusion-v25)
regressed latitude/longitude on the gold anchor because the distilled sherlock corpus the
head trained on contains essentially no coordinate columns (latitude 5, longitude 2), so
the head never learned to connect View1's lat/lon signal to the lat/lon output. The
manufactured reference corpus (output/mining-factory/firewalled_values.ndjson) has tens of
thousands of lat/lon VALUES; this script groups them into synthetic COLUMNS in the exact
shape dump-fusion-features consumes (final_label, sample_values JSON array, column_name),
so the starved families gain real support in head training.

We only manufacture the genuinely data-starved families (default: latitude, longitude).
Categorical is deliberately NOT manufactured: the head corpus already has ~17k categorical
columns, and its gold regression is the intrinsic country-code-lookalike boundary a
value-level view cannot resolve — more data does not help it.
"""
from __future__ import annotations

import argparse
import csv
import json
import random
from collections import defaultdict
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]


def main() -> int:
    ap = argparse.ArgumentParser(description=(__doc__ or "").splitlines()[0])
    ap.add_argument("--values", type=Path,
                    default=REPO / "output/mining-factory/firewalled_values.ndjson")
    ap.add_argument("--out", type=Path, required=True,
                    help="output CSV: final_label,sample_values,column_name")
    ap.add_argument("--types", nargs="+", default=[
        "geography.coordinate.latitude",
        "geography.coordinate.longitude",
    ])
    ap.add_argument("--values-per-col", type=int, default=20)
    ap.add_argument("--max-cols-per-type", type=int, default=1500)
    ap.add_argument("--seed", type=int, default=42)
    args = ap.parse_args()

    wanted = set(args.types)
    by_type: dict[str, list[str]] = defaultdict(list)
    with args.values.open(encoding="utf-8") as f:
        for line in f:
            try:
                rec = json.loads(line)
            except json.JSONDecodeError:
                continue
            t = rec.get("type")
            v = rec.get("value")
            if t in wanted and isinstance(v, str) and v.strip():
                by_type[t].append(v)

    rng = random.Random(args.seed)
    args.out.parent.mkdir(parents=True, exist_ok=True)
    n_cols = 0
    with args.out.open("w", encoding="utf-8", newline="") as fh:
        w = csv.writer(fh)
        w.writerow(["final_label", "sample_values", "column_name"])
        for t in args.types:
            vals = by_type.get(t, [])
            rng.shuffle(vals)
            k = args.values_per_col
            # chunk shuffled values into columns; drop a short trailing remainder
            cols = [vals[i:i + k] for i in range(0, len(vals) - k + 1, k)]
            if len(cols) > args.max_cols_per_type:
                cols = cols[:args.max_cols_per_type]
            for chunk in cols:
                w.writerow([t, json.dumps(chunk, ensure_ascii=False), ""])
                n_cols += 1
            print(f"  {t}: {len(vals)} values -> {len(cols)} columns "
                  f"(capped at {args.max_cols_per_type})")
    print(f"wrote {n_cols} synthetic columns -> {args.out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
