#!/usr/bin/env python3
"""Offline corpus-honest gate for NON-NATIVE potion models (potion-8M / code-16M).

These models train on Python-computed potion embeds, so native `finetype profile` would
truncate them. This produces their corpus predictions OFFLINE on the SAME stratified-sample
columns the native m2v-244 gate used (read from m2v-244's candidate parquet → exact apples-
to-apples), via: extract base features (char/stats/header/validation, parallel) + potion
4-stat embed → table-grouped FTMB (sibling context preserved) → predict_multibranch.

Base features are extracted ONCE per column and reused across all potions (the bottleneck;
~67 ms/col × 8 workers). Memory-safe: processed in table-batches, predictions collected,
records freed per batch. Output: one predictions parquet per potion (file_path, column_name,
sense_prediction) ready for corpus_honest_gate.py --candidate.

Usage:
  eval/gittables/.venv/bin/python scripts/offline_corpus_gate.py \
      --source output/corpus-honest-gate/m2v-244/candidate/corpus_pass/columns.parquet \
      --potion minishlab/potion-base-8M:m2v8m --potion minishlab/potion-code-16M:m2v-code16m \
      --out-dir output/corpus-honest-gate/offline --jobs 8 [--limit-tables 5]
"""
import argparse
import subprocess
import sys
from collections import defaultdict
from concurrent.futures import ProcessPoolExecutor

import numpy as np
import pyarrow as pa
import pyarrow.parquet as pq

sys.path.insert(0, "scripts")
import prepare_multibranch_data as P  # noqa: E402

SEP = "│"          # value separator in sample_values_truncated
JOIN = "\x1f"      # record-label separator: <file_path>\x1f<column_name>
VALUE_CAP = 32
BIN = "./target/release/finetype"


def probe_valid_dim():
    out = subprocess.run([BIN, "extract-features", "--json", "--header", "p", "--validation"],
                         input='["x"]', capture_output=True, text=True, check=True)
    import json
    return len(json.loads(out.stdout.strip())["validation"])


def _extract_one(item):
    """Module-level (picklable for ProcessPoolExecutor): base features for one column.
    Each worker process json-decodes independently — its own GIL — so this is the real
    8x parallelism the threaded version couldn't get (json.loads is GIL-bound)."""
    fp, cn, vals = item
    f = P.extract_features(BIN, vals, header=cn, include_validation=True)
    return (fp, cn, vals, f)


def potion_4stat(model, texts):
    """mean ++ var ++ min ++ max over model2vec L2-normed value vectors (matches Rust)."""
    pv = np.asarray(model.encode(texts, normalize=True), dtype=np.float64)
    return np.concatenate([pv.mean(0), pv.var(0), pv.min(0), pv.max(0)]).tolist()


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--source", required=True, help="parquet with file_path, column_name, sample_values_truncated")
    ap.add_argument("--potion", action="append", required=True, help="model_id:tag (repeatable)")
    ap.add_argument("--out-dir", required=True)
    ap.add_argument("--jobs", type=int, default=8)
    ap.add_argument("--limit-tables", type=int, default=0, help="smoke test: only N tables")
    ap.add_argument("--batch-tables", type=int, default=3000)
    a = ap.parse_args()

    import os
    os.makedirs(a.out_dir, exist_ok=True)
    potions = [p.split(":", 1) for p in a.potion]  # [(model_id, tag), ...]

    from model2vec import StaticModel
    encoders = [(tag, StaticModel.from_pretrained(mid)) for mid, tag in potions]
    dim = encoders[0][1].embedding.shape[1]
    P.EMBED_DIM = 4 * dim
    P.VALID_DIM = probe_valid_dim()
    P.VERSION_V4 = 5
    print(f"[offline-gate] potions={[t for t,_ in encoders]} EMBED_DIM={P.EMBED_DIM} VALID_DIM={P.VALID_DIM}")

    # Column source grouped by table (file_path) to preserve sibling context.
    tbl = pq.read_table(a.source, columns=["file_path", "column_name", "sample_values_truncated"]).to_pylist()
    tables = defaultdict(list)
    for r in tbl:
        vals = [v for v in (r["sample_values_truncated"] or "").split(SEP) if v][:VALUE_CAP]
        if vals:
            tables[r["file_path"]].append((r["column_name"], vals))
    table_items = list(tables.items())
    if a.limit_tables:
        table_items = table_items[:a.limit_tables]
    n_cols = sum(len(c) for _, c in table_items)
    print(f"[offline-gate] {len(table_items)} tables, {n_cols} columns with values")

    preds = {tag: [] for tag, _ in encoders}   # (file_path, column_name, label)
    done = 0
    # One pool for the whole run — created INSIDE the loop respawns 8 workers per batch
    # and deadlocked the feed on the first large batch. Hoisted out: workers stay warm.
    ex = ProcessPoolExecutor(max_workers=a.jobs)
    for start in range(0, len(table_items), a.batch_tables):
        batch = table_items[start:start + a.batch_tables]
        # Parallel base-feature extraction (process pool → real parallelism on json.loads).
        flat = [(fp, cn, vals) for fp, cols in batch for cn, vals in cols]
        results = list(ex.map(_extract_one, flat, chunksize=64))

        # Per potion, assemble a table-grouped sub-FTMB and predict.
        by_table = defaultdict(list)
        for fp, cn, vals, f in results:
            if f is None:
                continue
            ok = (f.get("char") and f.get("stats") and f.get("header_features") and f.get("validation")
                  and len(f["char"]) == P.CHAR_DIM and len(f["stats"]) == P.STATS_DIM
                  and len(f["header_features"]) == P.HEADER_DIM and len(f["validation"]) == P.VALID_DIM)
            if ok:
                by_table[fp].append((cn, vals, f))

        for tag, model in encoders:
            groups = []
            for fp, cols in by_table.items():
                recs, headers = [], []
                for cn, vals, f in cols:
                    headers.append(cn)
                    recs.append({"label": f"{fp}{JOIN}{cn}", "column_index": 0,
                                 "char": f["char"], "embed": potion_4stat(model, vals),
                                 "stats": f["stats"], "header": f["header_features"],
                                 "validation": f["validation"]})
                groups.append({"sibling_headers": headers, "records": recs})
            ftmb = f"{a.out_dir}/_batch_{tag}.ftmb"
            P.write_ftmb_v4(ftmb, groups)
            raw = f"{a.out_dir}/_batch_{tag}_pred.tsv"
            subprocess.run(["./target/release/predict_multibranch", "--model", f"models/{tag}-s44",
                            "--data", ftmb, "--out", raw], check=True, capture_output=True, text=True)
            with open(raw) as fh:
                next(fh, None)
                for line in fh:
                    p = line.rstrip("\n").split("\t")
                    if len(p) >= 2 and JOIN in p[0]:
                        fp, cn = p[0].split(JOIN, 1)
                        preds[tag].append((fp, cn, p[1]))
        done += len(batch)
        print(f"[offline-gate] {done}/{len(table_items)} tables done")
    ex.shutdown()

    for tag, _ in encoders:
        rows = preds[tag]
        out = f"{a.out_dir}/{tag}_candidate.parquet"
        pq.write_table(pa.table({
            "file_path": [r[0] for r in rows],
            "column_name": [r[1] for r in rows],
            "sense_prediction": [r[2] for r in rows],
        }), out)
        print(f"[offline-gate] {tag}: {len(rows)} predictions -> {out}")


if __name__ == "__main__":
    main()
