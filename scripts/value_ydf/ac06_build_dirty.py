#!/usr/bin/env python3
"""ac-07 cell-2 prep: the DIRTY (un-cleaned) gittables blend.

The confound control trains the same CharCNN recipe on the UN-cleaned blend —
every scored value tagged with its raw column weak label, skipping the triad.
Three constraints still apply, all orthogonal to the triad:
  - drop empty weak labels;
  - never train representation.discrete.categorical as a positive target
    (CLAUDE.md);
  - apply the SAME row_hash(header, value) firewall as the cleaned set, so the
    confound control is held to the identical leakage standard — otherwise a
    cleaned-cell win could be confounded by the dirty cell memorising held-out
    values.

Streamed in batches so the 25.6M-row value/header strings are never all
resident at once (a full-frame load OOMs on a 16GB box).

Run from the eval venv:
  PYTHONPATH=scripts/value_ydf eval/gittables/.venv/bin/python \
    scripts/value_ydf/ac06_build_dirty.py
"""
from __future__ import annotations

import json
import sys

import pyarrow.parquet as pq

import common as C

sys.path.insert(0, str(C.REPO / "scripts"))
from eval_leakage import row_hash  # noqa: E402

CATEGORICAL = "representation.discrete.categorical"
ROW_HASHES = C.REPO / "eval" / "row_hashes.tsv"
BATCH = 1_000_000


def load_eval_hashes() -> set[str]:
    hashes: set[str] = set()
    with open(ROW_HASHES, encoding="utf-8") as fh:
        for line in fh:
            line = line.rstrip("\n")
            if not line or line.startswith("#") or line.startswith("dataset\t"):
                continue
            parts = line.split("\t")
            if len(parts) >= 4:
                hashes.add(parts[3])
    return hashes


def main() -> int:
    eval_hashes = load_eval_hashes()
    pf = pq.ParquetFile(str(C.OUT_DIR / "scored_values.parquet"))
    out = C.OUT_DIR / "dirty_value_training.ndjson"
    kept = 0
    leaked = 0
    with open(out, "w", encoding="utf-8") as fh:
        for batch in pf.iter_batches(
            columns=["column_name", "value", "weak_label"], batch_size=BATCH
        ):
            cn = batch.column("column_name").to_pylist()
            vals = batch.column("value").to_pylist()
            wlb = batch.column("weak_label").to_pylist()
            for j in range(len(vals)):
                lbl = wlb[j] or ""
                if lbl == "" or lbl == CATEGORICAL:
                    continue
                if row_hash(cn[j] or "", str(vals[j])) in eval_hashes:
                    leaked += 1
                    continue
                fh.write(json.dumps({"classification": lbl, "text": vals[j]}, ensure_ascii=False) + "\n")
                kept += 1
    print(json.dumps(
        {"dirty_rows": kept, "leakage_collisions_removed": leaked,
         "output": str(out.relative_to(C.REPO))}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
