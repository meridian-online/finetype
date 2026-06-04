#!/usr/bin/env python3
"""ac-07: apples-to-apples head-to-head — the question this spec settles.

Three cells, one corpus (150 sampled files), v19 as the reference call:
  (1) cleaned-data CharCNN   models/char-cnn-v15-gittables
  (2) DIRTY-data CharCNN     models/char-cnn-v15-dirty   (confound control)
  (3) v19 multi-branch       models/sherlock-v19-relu-s42 (reference)

For each candidate cell we report the latitude/utc collateral footprint — the
number of columns the candidate calls latitude/utc that v19 does NOT — plus the
absolute corpus footprint of those labels. The win condition (set in the spec
BEFORE the run) is judged on whether v14's latitude=0 survives on cleaned REAL
data and whether the cleaned cell's footprint beats the dirty control.

Run from the eval venv:
  PYTHONPATH=scripts/value_ydf eval/gittables/.venv/bin/python \
    scripts/value_ydf/ac07_headtohead.py
"""
from __future__ import annotations

import collections
import csv
import io
import json
import os
import subprocess
import tempfile

import pandas as pd

import common as C

BIN = str(C.FINETYPE_BIN)
COLS = str(C.CORPUS_PARQUET)
V19 = "models/sherlock-v19-relu-s42"
CLEAN = "models/char-cnn-v15-gittables"
DIRTY = "models/char-cnn-v15-dirty"
UTC, LAT = "datetime.offset.utc", "geography.coordinate.latitude"
CAT = "representation.discrete.categorical"


def profile_file(model: str, csvpath: str, char_cnn: bool) -> dict:
    cmd = [BIN, "profile", "-f", csvpath, "-o", "csv"]
    if char_cnn:
        cmd += ["--model-type", "char-cnn"]
    out = subprocess.run(
        cmd, env={"FINETYPE_MODEL": model, "PATH": "/usr/bin:/bin"}, capture_output=True, text=True
    ).stdout
    res = {}
    for row in list(csv.reader(io.StringIO(out)))[1:]:
        if len(row) >= 2:
            res[row[0]] = row[1]
    return res


def main() -> int:
    import argparse

    ap = argparse.ArgumentParser()
    ap.add_argument("--n-files", type=int, default=150)
    args = ap.parse_args()

    allfp = pd.read_parquet(COLS, columns=["file_path"])["file_path"].drop_duplicates()
    files = allfp.sample(args.n_files, random_state=42).tolist()

    cells = {"v19": (V19, False)}
    if os.path.isdir(f"{C.REPO}/{CLEAN}"):
        cells["clean_v15"] = (CLEAN, True)
    if os.path.isdir(f"{C.REPO}/{DIRTY}"):
        cells["dirty_v15"] = (DIRTY, True)

    ncols = 0
    foot = {name: collections.Counter() for name in cells}
    coll = {name: [] for name in cells if name != "v19"}
    # Net-regression proxy for the win condition's part (b): per-column
    # agreement with v19 on the same lens. The v15 CharCNNs are value-level
    # models, not column-level Sense models wired into the gated cell-2
    # pipeline, so full-corpus gated cell-2 accuracy is out of scope here;
    # agreement with v19 across the 150 profiled files is the faithful,
    # runnable approximation of "does swapping toward this method lose broad
    # accuracy elsewhere". Disagreement is split into disease (utc/lat) vs
    # other, so a win that is purely disease-suppression is distinguishable
    # from a broad regression.
    agree = {name: 0 for name in cells if name != "v19"}
    cmp_total = {name: 0 for name in cells if name != "v19"}
    disagree_other = {name: 0 for name in cells if name != "v19"}
    for f in files:
        try:
            d = pd.read_parquet(f)
        except Exception:
            continue
        if d.shape[1] == 0 or len(d) == 0:
            continue
        with tempfile.NamedTemporaryFile("w", suffix=".csv", delete=False) as tf:
            d.to_csv(tf.name, index=False)
            csvp = tf.name
        preds = {name: profile_file(model, csvp, cc) for name, (model, cc) in cells.items()}
        os.unlink(csvp)
        ref = preds["v19"]
        for c in preds["v19"]:
            ncols += 1
        for name, p in preds.items():
            for c, lbl in p.items():
                foot[name][lbl] += 1
                if name != "v19":
                    if c in ref:
                        cmp_total[name] += 1
                        if ref[c] == lbl:
                            agree[name] += 1
                        elif lbl not in (UTC, LAT):
                            disagree_other[name] += 1
                    if lbl in (UTC, LAT) and ref.get(c) != lbl:
                        coll[name].append((lbl, ref.get(c, "?"), c))

    summary = {"n_cols_profiled": ncols, "cells": {}}
    for name in cells:
        s = {
            "utc_footprint": foot[name][UTC],
            "lat_footprint": foot[name][LAT],
            "cat_footprint": foot[name][CAT],
        }
        if name != "v19":
            s["utc_collateral"] = sum(1 for b, *_ in coll[name] if b == UTC)
            s["lat_collateral"] = sum(1 for b, *_ in coll[name] if b == LAT)
            s["cols_compared_with_v19"] = cmp_total[name]
            s["agree_with_v19"] = agree[name]
            s["agreement_rate"] = round(agree[name] / cmp_total[name], 4) if cmp_total[name] else None
            s["disagree_non_disease"] = disagree_other[name]
        summary["cells"][name] = s
    (C.OUT_DIR / "ac07_headtohead.json").write_text(json.dumps(summary, indent=2))

    lines = [
        "# ac-07 — head-to-head collateral footprint (150 files, v19 reference)",
        "",
        f"- columns profiled: **{ncols}**",
        "",
        "| cell | utc collateral | latitude collateral | utc footprint | lat footprint |",
        "|---|---|---|---|---|",
    ]
    for name in ("clean_v15", "dirty_v15"):
        if name in summary["cells"]:
            s = summary["cells"][name]
            lines.append(
                f"| {name} | {s['utc_collateral']} | {s['lat_collateral']} | {s['utc_footprint']} | {s['lat_footprint']} |"
            )
    lines += [
        f"| v19 (ref) | — | — | {summary['cells']['v19']['utc_footprint']} | {summary['cells']['v19']['lat_footprint']} |",
        "",
        "## Net-regression check (win condition part b)",
        "",
        "Per-column agreement with v19 across the same 150 files — the runnable "
        "proxy for 'net cell-2 does not regress vs v19'. The v15 CharCNNs are "
        "value-level models, not column-level Sense models in the gated cell-2 "
        "pipeline, so full-corpus gated cell-2 accuracy is out of scope; this "
        "agreement rate is the faithful approximation. `disagree (non-disease)` "
        "isolates broad regression from the intended utc/latitude suppression.",
        "",
        "| cell | cols compared | agreement with v19 | disagree (non-disease) |",
        "|---|---|---|---|",
    ]
    for name in ("clean_v15", "dirty_v15"):
        if name in summary["cells"]:
            s = summary["cells"][name]
            rate = s.get("agreement_rate")
            rate_s = f"{rate:.1%}" if rate is not None else "—"
            lines.append(
                f"| {name} | {s.get('cols_compared_with_v19', 0)} | {rate_s} "
                f"({s.get('agree_with_v19', 0)}) | {s.get('disagree_non_disease', 0)} |"
            )
    lines += [
        "",
        "Reference: v24 multi-branch on this lens was 26 utc + 3 latitude = 29 collateral.",
        "v14 synthetic-data CharCNN drove latitude collateral to 0 — this table tests "
        "whether that survives once trained on cleaned REAL data (clean_v15) and whether "
        "it beats the un-cleaned control (dirty_v15).",
    ]
    (C.OUT_DIR / "ac07_headtohead.md").write_text("\n".join(lines) + "\n")
    print("\n".join(lines))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
