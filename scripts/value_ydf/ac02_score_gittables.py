#!/usr/bin/env python3
"""ac-02: score every non-trivial gittables value with the value-level prior.

The ac-01 prior is synthetic-only, so scoring gittables values is out-of-fold
(OOF) by construction — no gittables value was in the prior's training set. The
cross-fit folds only become load-bearing at ac-05, when KEPT gittables values
are folded back into the YDF pool; this single pass needs no k-fold.

Two memory-bounded passes (a single-pass global cache OOMs at corpus scale):
  Pass 1 — collect the de-duplicated value set, score it in batches (extract the
           277-dim contract via the ac-00 Rust bin, RF-predict, derive the
           JSON-Schema pass set), stream to uniq_scores.parquet.
  Pass 2 — stream the corpus row-groups, explode values, merge the unique
           scores back, write the per-occurrence scored_values.parquet.

Per value we emit: value, weak column label (column-level ydf_prediction — the
unverified distillation we audit, independent of the multi-branch model),
value-level ydf_label + ydf_confidence (RF vote fraction), and the schema pass
set (space-joined type keys the value satisfies).

Run from the eval venv (unbuffered so progress is visible):
  PYTHONPATH=scripts/value_ydf eval/gittables/.venv/bin/python -u \
    scripts/value_ydf/ac02_score_gittables.py
"""
from __future__ import annotations

import argparse
import json
import time

import numpy as np
import pandas as pd
import pyarrow as pa
import pyarrow.parquet as pq
import ydf

import common as C

CORPUS_COLS = ["file_path", "column_name", C.WEAK_LABEL_COL, "sample_values_truncated", "is_trivial"]


def explode_values(df: pd.DataFrame) -> pd.DataFrame:
    df = df[~df["is_trivial"]].dropna(subset=["sample_values_truncated"])
    if df.empty:
        return df.assign(value=pd.Series(dtype=str))
    parts = df["sample_values_truncated"].str.split("│")
    lens = parts.str.len().to_numpy()
    occ = pd.DataFrame(
        {
            "file_path": np.repeat(df["file_path"].to_numpy(), lens),
            "column_name": np.repeat(df["column_name"].to_numpy(), lens),
            "weak_label": np.repeat(df[C.WEAK_LABEL_COL].to_numpy(), lens),
            "value": np.concatenate(parts.to_numpy()),
        }
    )
    occ["value"] = occ["value"].str.strip()
    return occ[occ["value"].str.len() > 0].reset_index(drop=True)


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--batch", type=int, default=150_000, help="unique-value scoring batch")
    args = ap.parse_args()

    C.OUT_DIR.mkdir(parents=True, exist_ok=True)
    model = ydf.load_model(str(C.YDF_VALUE_MODEL))
    classes = list(model.label_classes())
    pf = pq.ParquetFile(str(C.CORPUS_PARQUET))
    t0 = time.time()

    # ---- Pass 1: collect unique values --------------------------------------
    print("[pass 1] collecting unique values", flush=True)
    uniq: set[str] = set()
    for rg in range(pf.num_row_groups):
        occ = explode_values(pf.read_row_group(rg, columns=CORPUS_COLS).to_pandas())
        uniq.update(occ["value"].unique().tolist())
        print(f"  rg{rg}: cum_unique={len(uniq):,} ({time.time()-t0:.0f}s)", flush=True)
    uniq_list = list(uniq)
    del uniq
    print(f"[pass 1] {len(uniq_list):,} unique values; scoring in batches of {args.batch:,}", flush=True)

    uniq_path = C.OUT_DIR / "uniq_scores.parquet"
    writer = None
    for start in range(0, len(uniq_list), args.batch):
        batch = uniq_list[start : start + args.batch]
        feats = C.extract_value_features_df(batch, C.OUT_DIR)
        fcols = C.feature_columns(feats.columns)
        scols = C.schema_columns(feats.columns)
        keys = np.array([c[len(C.SCHEMA_PREFIX):] for c in scols])
        proba = np.asarray(model.predict(feats[fcols]))
        idx = proba.argmax(axis=1)
        conf = proba.max(axis=1)
        schema_mat = feats[scols].to_numpy() >= 0.5
        schema_pass = [" ".join(keys[schema_mat[i]]) for i in range(len(batch))]
        tbl = pa.table(
            {
                "value": pa.array(batch, type=pa.string()),
                "ydf_label": pa.array([classes[i] for i in idx], type=pa.string()),
                "ydf_confidence": pa.array(conf.astype("float32")),
                "schema_pass": pa.array(schema_pass, type=pa.string()),
            }
        )
        if writer is None:
            writer = pq.ParquetWriter(str(uniq_path), tbl.schema, compression="zstd")
        writer.write_table(tbl)
        print(f"  scored {min(start+args.batch, len(uniq_list)):,}/{len(uniq_list):,} ({time.time()-t0:.0f}s)", flush=True)
    if writer is not None:
        writer.close()
    for f in ("_score_values.ndjson", "_score_values.csv"):
        p = C.OUT_DIR / f
        if p.exists():
            p.unlink()

    # ---- Pass 2: emit per-occurrence scored parquet -------------------------
    print("[pass 2] merging scores onto corpus occurrences", flush=True)
    uniq_scores = pd.read_parquet(uniq_path)
    out_path = C.OUT_DIR / "scored_values.parquet"
    writer = None
    total = 0
    for rg in range(pf.num_row_groups):
        occ = explode_values(pf.read_row_group(rg, columns=CORPUS_COLS).to_pandas())
        if occ.empty:
            continue
        merged = occ.merge(uniq_scores, on="value", how="left")
        tbl = pa.Table.from_pandas(
            merged[["file_path", "column_name", "value", "weak_label", "ydf_label", "ydf_confidence", "schema_pass"]],
            preserve_index=False,
        )
        if writer is None:
            writer = pq.ParquetWriter(str(out_path), tbl.schema, compression="zstd")
        writer.write_table(tbl)
        total += len(merged)
        print(f"  rg{rg}: rows={len(merged):,} cum={total:,} ({time.time()-t0:.0f}s)", flush=True)
    if writer is not None:
        writer.close()

    summary = {
        "scored_rows": total,
        "unique_values": len(uniq_list),
        "rowgroups": pf.num_row_groups,
        "model": str(C.YDF_VALUE_MODEL.relative_to(C.REPO)),
        "oof": "synthetic-only prior; gittables scoring is OOF by construction (cross-fit folds deferred to ac-05)",
        "weak_label_source": C.WEAK_LABEL_COL,
        "output": str(out_path.relative_to(C.REPO)),
    }
    (C.OUT_DIR / "ac02_scored_summary.json").write_text(json.dumps(summary, indent=2))
    print(f"done. {total:,} scored value-rows -> {out_path.relative_to(C.REPO)} ({time.time()-t0:.0f}s)", flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
