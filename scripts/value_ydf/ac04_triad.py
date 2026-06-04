#!/usr/bin/env python3
"""ac-04: corroboration triad — clean gittables to value-level (value, label).

Three signals per scored value, all relative to the column's weak label L:
  (a) YDF predicts L at conf >= floor   (value-level prior agrees)
  (b) value passes L's JSON Schema      (Precision Principle as a feature)
  (c) L == weak column label            (trivially true for the column's label)

Quarantine-first policy (author-approved design decision): we do NOT
auto-relabel a value to YDF's call. So the KEEP label is the column's weak
label, cleaned at value granularity:

  KEEP (value, L=weak_label)  when >= 2 of {(a),(b),(c)} hold
                              i.e. passes-weak-schema OR ydf-confidently-agrees
  DROP value                  off-distribution: fails (b) AND not (a)
  FLAG column -> quarantine    when YDF confidently predicts a single OTHER
                              label X for the column majority AND those values
                              validate against X better than against L. The
                              whole column is pulled from training (NOT
                              relabelled) and written to the quarantine list.

Emits the cleaned training NDJSON ({classification, text}) and a label-error
report. representation.discrete.categorical is never a positive target
(CLAUDE.md) — kept rows with that weak label are dropped from training.

Run from the eval venv:
  PYTHONPATH=scripts/value_ydf eval/gittables/.venv/bin/python \
    scripts/value_ydf/ac04_triad.py
"""
from __future__ import annotations

import argparse
import json
import sys

import numpy as np
import pandas as pd
import pyarrow.parquet as pq

import common as C

sys.path.insert(0, str(C.REPO / "scripts"))
from eval_leakage import row_hash  # noqa: E402

CATEGORICAL = "representation.discrete.categorical"
SCORED = "scored_values.parquet"
ROW_HASHES = C.REPO / "eval" / "row_hashes.tsv"


def load_eval_hashes() -> set[str]:
    """The row_hash(header,value) firewall set, mirroring the ac-08 loader.
    At value granularity common atomic values (Year=2016, ID=...) coincide
    with eval-holdout rows across the partition; those rows are removed from
    the cleaned set by construction so the model never trains on a held-out
    value. The primary file-partition firewall (SHA%2) is enforced upstream."""
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


BATCH = 1_000_000
READ_COLS = [
    "file_path", "column_name", "value", "weak_label", "ydf_label",
    "ydf_confidence", "schema_pass",
]


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--floor", type=float, default=0.85)
    ap.add_argument("--majority", type=float, default=0.5, help="column majority fraction for FLAG")
    args = ap.parse_args()

    path = C.OUT_DIR / SCORED
    pf = pq.ParquetFile(str(path))

    # ---- Pass A: streamed compact frame ------------------------------------
    # The value and schema_pass strings (25.6M rows) never all reside at once.
    # We keep only small integer arrays: a factorised column id, int16 label
    # codes for the weak/ydf labels, and three booleans. Membership of the weak
    # label in the value's JSON-Schema pass-set is computed inline per batch.
    file_codes: dict[str, int] = {}
    file_list: list[str] = []
    col_codes: dict[tuple, int] = {}
    col_keys: list[tuple] = []          # col_id -> (file_code, column_name)
    lbl_list: list[str] = [""]
    lbl_codes: dict[str, int] = {"": 0}

    def fcode(s: str) -> int:
        c = file_codes.get(s)
        if c is None:
            c = len(file_list)
            file_codes[s] = c
            file_list.append(s)
        return c

    def lcode(s: str) -> int:
        c = lbl_codes.get(s)
        if c is None:
            c = len(lbl_list)
            lbl_codes[s] = c
            lbl_list.append(s)
        return c

    cid_parts, wl_parts, yl_parts = [], [], []
    pw_parts, agree_parts, dis_parts = [], [], []
    for batch in pf.iter_batches(columns=READ_COLS, batch_size=BATCH):
        fp = batch.column("file_path").to_pylist()
        cn = batch.column("column_name").to_pylist()
        sp = batch.column("schema_pass").to_pylist()
        wlb = batch.column("weak_label").to_pylist()
        ylb = batch.column("ydf_label").to_pylist()
        conf = batch.column("ydf_confidence").to_numpy(zero_copy_only=False)
        m = len(fp)
        cid = np.empty(m, dtype=np.int32)
        wlc = np.empty(m, dtype=np.int16)
        ylc = np.empty(m, dtype=np.int16)
        pw = np.empty(m, dtype=bool)
        for i in range(m):
            key = (fcode(fp[i]), cn[i])
            c = col_codes.get(key)
            if c is None:
                c = len(col_keys)
                col_codes[key] = c
                col_keys.append(key)
            cid[i] = c
            w = wlb[i] or ""
            wlc[i] = lcode(w)
            ylc[i] = lcode(ylb[i] or "")
            s = sp[i]
            pw[i] = bool(w) and s is not None and (w in s.split())
        conf_ok = conf >= args.floor
        cid_parts.append(cid)
        wl_parts.append(wlc)
        yl_parts.append(ylc)
        pw_parts.append(pw)
        agree_parts.append(conf_ok & (ylc == wlc))
        dis_parts.append(conf_ok & (ylc != wlc))

    col_id = np.concatenate(cid_parts)
    wl = np.concatenate(wl_parts)
    yl = np.concatenate(yl_parts)
    passes_weak = np.concatenate(pw_parts)
    ydf_agrees = np.concatenate(agree_parts)
    disagree = np.concatenate(dis_parts)
    n_total = col_id.size
    del col_codes, file_codes
    del cid_parts, wl_parts, yl_parts, pw_parts, agree_parts, dis_parts

    def col_lookup(cid_i: int) -> tuple:
        fc, cn = col_keys[cid_i]
        return file_list[fc], cn

    d = pd.DataFrame({
        "col_id": col_id, "wl": wl, "yl": yl,
        "passes_weak": passes_weak, "disagree": disagree,
    })
    col_n = d.groupby("col_id").size().rename("col_n")
    passes_w_col = d.groupby("col_id")["passes_weak"].sum().rename("pw")
    col_weak = d.groupby("col_id")["wl"].first().rename("wl_code")

    # ---- Column-level FLAG (quarantine) detection --------------------------
    flagged_cols: set[int] = set()
    quarantine_rows = []
    dis = d[d["disagree"]]
    if not dis.empty:
        share = (
            dis.groupby(["col_id", "yl"]).size().rename("k").reset_index()
            .merge(col_n.reset_index(), on="col_id")
        )
        share["frac"] = share["k"] / share["col_n"]
        top = share.sort_values("k").drop_duplicates("col_id", keep="last")
        cand = top[top["frac"] >= args.majority].copy()
        if not cand.empty:
            # passes_x: streamed second pass restricted to candidate columns,
            # zipped against the in-memory col_id array (iter_batches preserves
            # row order). X is the candidate's majority ydf label (code in yl).
            x_code = dict(zip(cand["col_id"].astype(int), cand["yl"].astype(int)))
            cand_set = set(x_code)
            px_counter = {c: 0 for c in cand_set}
            off = 0
            for batch in pf.iter_batches(columns=["schema_pass"], batch_size=BATCH):
                sp = batch.column("schema_pass").to_pylist()
                cids = col_id[off:off + len(sp)]
                for j, s in enumerate(sp):
                    c = int(cids[j])
                    if c in cand_set and s is not None:
                        if lbl_list[x_code[c]] in s.split():
                            px_counter[c] += 1
                off += len(sp)
            px = pd.Series(px_counter, name="px")
            px.index.name = "col_id"
            chk = (
                cand.merge(px.reset_index(), on="col_id")
                .merge(passes_w_col.reset_index(), on="col_id")
                .merge(col_weak.reset_index(), on="col_id")
            )
            flagged = chk[chk["px"] > chk["pw"]]
            for _, r in flagged.iterrows():
                cid_i = int(r["col_id"])
                fp, cn = col_lookup(cid_i)
                flagged_cols.add(cid_i)
                quarantine_rows.append(
                    {
                        "file_path": fp,
                        "column_name": cn,
                        "weak_label": lbl_list[int(r["wl_code"])],
                        "suggested_label": lbl_list[int(r["yl"])],
                        "majority_frac": round(float(r["frac"]), 3),
                        "col_n": int(r["col_n"]),
                    }
                )

    flagged_mask = (
        np.isin(col_id, np.fromiter(flagged_cols, dtype=np.int32))
        if flagged_cols
        else np.zeros(n_total, dtype=bool)
    )

    # ---- Per-value KEEP / DROP on the non-flagged columns ------------------
    cat_code = lbl_codes.get(CATEGORICAL, -1)
    votes = passes_weak.astype(int) + ydf_agrees.astype(int) + 1
    keep_mask = (votes >= 2) & (~flagged_mask) & (wl != 0) & (wl != cat_code)
    drop_mask = (~keep_mask) & (~flagged_mask)

    # ---- Pass C: streamed emit of the cleaned NDJSON + provenance ----------
    # The row_hash(header, value) firewall is applied here BY CONSTRUCTION:
    # a kept value whose (column header, value) hashes into the eval holdout is
    # dropped so the model never trains on a held-out value. ac-08 then audits
    # this leak-free-by-construction set as an independent check.
    eval_hashes = load_eval_hashes()
    leaked = 0
    out_ndjson = C.OUT_DIR / "cleaned_value_training.ndjson"
    prov_fp, prov_cn, prov_val, prov_lbl = [], [], [], []
    off = 0
    with open(out_ndjson, "w", encoding="utf-8") as fh:
        for batch in pf.iter_batches(
            columns=["file_path", "column_name", "value", "weak_label"], batch_size=BATCH
        ):
            vals = batch.column("value").to_pylist()
            wlb = batch.column("weak_label").to_pylist()
            fpb = batch.column("file_path").to_pylist()
            cnb = batch.column("column_name").to_pylist()
            km = keep_mask[off:off + len(vals)]
            for j, ok in enumerate(km):
                if ok:
                    if row_hash(cnb[j] or "", str(vals[j])) in eval_hashes:
                        keep_mask[off + j] = False  # leak: drop by construction
                        leaked += 1
                        continue
                    lbl = wlb[j] or ""
                    fh.write(json.dumps({"classification": lbl, "text": vals[j]}, ensure_ascii=False) + "\n")
                    prov_fp.append(fpb[j])
                    prov_cn.append(cnb[j])
                    prov_val.append(vals[j])
                    prov_lbl.append(lbl)
            off += len(vals)
    # Provenance for the ac-08 leakage audit (the headerless NDJSON above cannot
    # carry the column header the row_hash firewall needs).
    pd.DataFrame(
        {"file_path": prov_fp, "column_name": prov_cn, "value": prov_val, "label": prov_lbl}
    ).to_parquet(C.OUT_DIR / "kept_rows.parquet", index=False)

    # Leaked rows flipped to drop above; recompute drop_mask to absorb them.
    drop_mask = (~keep_mask) & (~flagged_mask)

    # within-column heterogeneity removed = of all values in KEPT columns, the
    # fraction that did not survive (the noise the value-gate stripped out).
    kept_col_ids = np.unique(col_id[keep_mask])
    in_kept_cols = np.isin(col_id, kept_col_ids)
    het_removed = float((in_kept_cols & ~keep_mask).sum() / in_kept_cols.sum()) if in_kept_cols.sum() else 0.0

    kdf = pd.DataFrame({"label": prov_lbl, "value": prov_val})
    per_type = kdf.groupby("label")["value"].agg(["size", "nunique"]).sort_values("nunique", ascending=False)

    pd.DataFrame(quarantine_rows).to_csv(C.OUT_DIR / "quarantine_label_errors.csv", index=False)

    report = {
        "floor": args.floor,
        "scored_rows": n_total,
        "kept_rows": int(keep_mask.sum()),
        "dropped_rows": int(drop_mask.sum()),
        "flagged_rows": int(flagged_mask.sum()),
        "flagged_columns": len(flagged_cols),
        "kept_types": int(per_type.shape[0]),
        "kept_distinct_values": int(kdf["value"].nunique()),
        "within_column_heterogeneity_removed": round(het_removed, 4),
        "categorical_positive_targets": 0,
        "leakage_collisions_removed": leaked,
        "lat_kept_distinct": int(per_type.loc["geography.coordinate.latitude", "nunique"]) if "geography.coordinate.latitude" in per_type.index else 0,
    }
    (C.OUT_DIR / "ac04_triad_report.json").write_text(json.dumps(report, indent=2))

    lines = [
        "# ac-04 — corroboration triad (quarantine-first)",
        "",
        f"- scored value-rows: **{n_total:,}**",
        f"- KEEP: **{report['kept_rows']:,}** ({report['kept_distinct_values']:,} distinct values, {report['kept_types']} types)",
        f"- DROP (off-distribution): **{report['dropped_rows']:,}**",
        f"- FLAG/quarantine: **{report['flagged_rows']:,}** rows across **{report['flagged_columns']:,}** columns",
        f"- within-column heterogeneity removed (kept columns): **{report['within_column_heterogeneity_removed']*100:.1f}%**",
        f"- categorical positive targets: **{report['categorical_positive_targets']}** (excluded by policy)",
        f"- leakage collisions removed by row_hash firewall: **{report['leakage_collisions_removed']}**",
        f"- latitude kept (distinct values): **{report['lat_kept_distinct']}**",
        "",
        "## Top kept types by distinct values",
        "",
        "| type | kept rows | distinct values |",
        "|---|---|---|",
    ]
    for lbl, row in per_type.head(20).iterrows():
        lines.append(f"| {lbl} | {int(row['size'])} | {int(row['nunique'])} |")
    lines += [
        "",
        f"Quarantine list: `output/value-level-labelling/quarantine_label_errors.csv` "
        f"({len(quarantine_rows)} columns flagged as distillation label-errors — NOT auto-relabelled).",
        f"Cleaned training set: `output/value-level-labelling/cleaned_value_training.ndjson`.",
    ]
    (C.OUT_DIR / "ac04_triad_report.md").write_text("\n".join(lines) + "\n")
    print("\n".join(lines))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
