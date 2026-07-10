#!/usr/bin/env python3
"""Compose a non-native model's raw Sense predictions through the NATIVE veto+Sharpen,
at corpus scale, so they can be corpus-honest-gated apples-to-apples.

predict_multibranch emits RAW pre-veto Sense (0 unknowns); the gate's baseline is the native
`finetype profile` x-finetype-label (post-veto). This re-runs `finetype profile` per column
with FINETYPE_INJECT_LABEL=<the model's Sense> so the native veto+Sharpen run on the real
values — yielding composed predictions comparable to the baseline. No model-native needed.

Parallel (ProcessPoolExecutor) — per-column profile is the bottleneck, like the extract pass.

Usage:
  eval/gittables/.venv/bin/python scripts/compose_corpus.py \
      --candidate output/corpus-honest-gate/offline/m2v8m_candidate.parquet \
      --source output/corpus-honest-gate/m2v-244/candidate/corpus_pass/columns.parquet \
      --out output/corpus-honest-gate/offline/m2v8m_composed.parquet --jobs 8 [--limit N]
"""
import argparse
import csv
import os
import subprocess
import tempfile
from concurrent.futures import ProcessPoolExecutor
from pathlib import Path

import pyarrow as pa
import pyarrow.parquet as pq

SEP = "│"
BIN = "./target/release/finetype"
VALUE_CAP = 64


def _compose_one(item):
    """(file_path, column_name, values, inject_label) -> (file_path, column_name, composed)."""
    fp, cn, values, inject = item
    with tempfile.TemporaryDirectory() as td:
        p = Path(td) / "c.csv"
        with open(p, "w", newline="") as fh:
            w = csv.writer(fh, lineterminator="\n")
            w.writerow([cn])
            for v in values:
                w.writerow([v])
        env = dict(os.environ, FINETYPE_INJECT_LABEL=inject)
        r = subprocess.run([BIN, "profile", "-f", str(p), "-o", "json-schema"],
                           capture_output=True, text=True, env=env)
        if r.returncode != 0:
            return (fp, cn, inject)  # fall back to raw Sense on failure
        import json
        try:
            schema = json.loads(r.stdout)
            props = schema.get("properties", {})
            defn = props.get(cn) or (next(iter(props.values())) if props else {})
            return (fp, cn, defn.get("x-finetype-label", inject))
        except Exception:
            return (fp, cn, inject)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--candidate", required=True, help="parquet: file_path, column_name, sense_prediction (raw)")
    ap.add_argument("--source", required=True, help="parquet with sample_values_truncated for the same columns")
    ap.add_argument("--out", required=True)
    ap.add_argument("--jobs", type=int, default=8)
    ap.add_argument("--limit", type=int, default=0)
    a = ap.parse_args()

    raw = {(r["file_path"], r["column_name"]): r["sense_prediction"]
           for r in pq.read_table(a.candidate).to_pylist()}
    vals = {}
    for r in pq.read_table(a.source, columns=["file_path", "column_name", "sample_values_truncated"]).to_pylist():
        k = (r["file_path"], r["column_name"])
        if k in raw and k not in vals:
            vals[k] = [v for v in (r["sample_values_truncated"] or "").split(SEP) if v][:VALUE_CAP]

    items = [(fp, cn, vals[(fp, cn)], raw[(fp, cn)]) for (fp, cn) in raw if vals.get((fp, cn))]
    if a.limit:
        items = items[:a.limit]
    print(f"[compose] {len(items)} columns to compose (of {len(raw)} raw)", flush=True)

    out = []
    with ProcessPoolExecutor(max_workers=a.jobs) as ex:
        for i, res in enumerate(ex.map(_compose_one, items, chunksize=64)):
            out.append(res)
            if (i + 1) % 50000 == 0:
                print(f"[compose] {i+1}/{len(items)}", flush=True)

    pq.write_table(pa.table({
        "file_path": [r[0] for r in out],
        "column_name": [r[1] for r in out],
        "sense_prediction": [r[2] for r in out],
    }), a.out)
    n_unk = sum(1 for r in out if r[2] == "unknown" or "unknown" in (r[2] or ""))
    print(f"[compose] wrote {len(out)} -> {a.out} (unknown now: {n_unk})", flush=True)


if __name__ == "__main__":
    main()
