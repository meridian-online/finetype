#!/usr/bin/env python3
"""Stratified per-class cap for the ac-06 training blends.

Both the cleaned (ac-04) and dirty (confound-control) NDJSONs are ~18-20M
rows — 75x v14-feat's 238k synthetic set, infeasible to train 12 epochs on a
CPU box. We cap each class to a fixed ceiling so the recipe stays comparable to
v14 AND the clean/dirty head-to-head is matched: identical ceiling, identical
seed, so the ONLY clean-vs-dirty difference remains the triad cleaning.

Per-class reservoir sampling (Algorithm R) keeps memory bounded — at most
ceiling rows per class are ever resident, never the full frame. Deterministic
under a fixed seed.

  python cap_ndjson.py <in.ndjson> <out.ndjson> <ceiling> <seed>
"""
from __future__ import annotations

import collections
import json
import random
import sys


def main() -> int:
    inp, outp = sys.argv[1], sys.argv[2]
    ceiling = int(sys.argv[3])
    seed = int(sys.argv[4])
    rng = random.Random(seed)

    reservoir: dict[str, list[str]] = collections.defaultdict(list)
    seen: dict[str, int] = collections.defaultdict(int)
    total = 0
    with open(inp, encoding="utf-8") as fh:
        for line in fh:
            line = line.rstrip("\n")
            if not line:
                continue
            cls = json.loads(line)["classification"]
            n = seen[cls]
            seen[cls] = n + 1
            total += 1
            r = reservoir[cls]
            if len(r) < ceiling:
                r.append(line)
            else:
                j = rng.randint(0, n)
                if j < ceiling:
                    r[j] = line

    kept = 0
    with open(outp, "w", encoding="utf-8") as fh:
        for cls in sorted(reservoir):
            for line in reservoir[cls]:
                fh.write(line + "\n")
                kept += 1

    capped = sum(1 for c in seen if seen[c] > ceiling)
    print(json.dumps({
        "input": inp, "output": outp, "ceiling": ceiling, "seed": seed,
        "rows_in": total, "rows_out": kept,
        "classes": len(seen), "classes_hit_ceiling": capped,
    }, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
