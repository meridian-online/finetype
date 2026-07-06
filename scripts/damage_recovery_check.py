#!/usr/bin/env python3
"""Damage-recovery precheck (t-000133e418 round 3, derisk review 2026-07-06).

Before spending the overnight run, profile samples of the ROUND-1 damage sets and the
attractor stay-dead sets with the cheap proxy model and require the repairs to be
measurably working. Round-1 evidence says proxy-depth (10-20 epoch) readouts preserve
direction (json_array melted, unix persisted), so direction thresholds are set modestly.

Pre-registered thresholds (exit 1 if any fails):
  decimal_unknown  >= 50% of sampled damage columns recover to decimal_number
  decimal_word     >= 50% recover to decimal_number
  ymd_broken       >= 50% recover to compact_ymd
  ws_gainers       >= 50% revert to a text-family label (entity/plain/word)
  npi_demoted      >= 90% stay NON-npi (attractor stays dead)
  upc_demoted      >= 90% stay NON-upc

Usage: eval/gittables/.venv/bin/python scripts/damage_recovery_check.py \
           --model models/sherlock-attneg2-proxy20-s42 [--files-per-set 40]
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
import sys
import tempfile
from collections import Counter
from pathlib import Path

import duckdb

REPO = Path(__file__).resolve().parent.parent
BIN = REPO / "target/release/finetype"
BASE = REPO / "output/attneg-retrain/baseline_with_oracle.parquet"
CAND = REPO / "output/attneg-retrain/cand_pass/corpus_pass/columns.parquet"

DEC = "representation.numeric.decimal_number"
INT = "representation.numeric.integer_number"
YMD = "datetime.date.compact_ymd"
WS = "container.array.whitespace_separated"
TEXT = {"representation.text.entity_name", "representation.text.plain_text",
        "representation.text.word"}

SETS = {
    # name: (baseline label, round-1 candidate label(s), success test)
    "decimal_unknown": (DEC, ("unknown",), lambda lbl: lbl == DEC, 0.50),
    "decimal_word": (DEC, ("representation.text.word",), lambda lbl: lbl == DEC, 0.50),
    "ymd_broken": (YMD, ("unknown", INT), lambda lbl: lbl == YMD, 0.50),
    "ws_gainers": (None, (WS,), lambda lbl: lbl in TEXT, 0.50),
    "npi_demoted": ("identity.medical.npi", (INT,),
                    lambda lbl: lbl != "identity.medical.npi", 0.90),
    "upc_demoted": ("identity.commerce.upc", (INT,),
                    lambda lbl: lbl != "identity.commerce.upc", 0.90),
}


def pick_columns(con, frm, dsts, n_files):
    frm_cond = f"b.sense_prediction = '{frm}'" if frm else f"b.sense_prediction NOT IN ('{WS}')"
    dst_list = ", ".join(f"'{d}'" for d in dsts)
    rows = con.execute(f"""
        SELECT b.file_path, b.column_name
        FROM read_parquet('{BASE}') b
        JOIN read_parquet('{CAND}') c USING (file_path, column_name)
        WHERE {frm_cond} AND c.sense_prediction IN ({dst_list})
          AND b.sense_prediction != c.sense_prediction
          AND len(string_split(c.sample_values_truncated, '│')) >= 5
    """).fetchall()
    rows.sort(key=lambda r: hashlib.md5(f"{r[0]}|{r[1]}".encode()).hexdigest())
    files, cols = [], []
    for fp, col in rows:
        if fp not in files:
            if len(files) >= n_files:
                continue
            files.append(fp)
        cols.append((fp, col))
    return files, cols


def profile(model, files, workdir):
    listfile = Path(workdir) / "files.txt"
    listfile.write_text("\n".join(os.path.realpath(f) for f in files if os.path.isfile(f)) + "\n")
    outdir = Path(workdir) / "schemas"
    outdir.mkdir(exist_ok=True)
    subprocess.run([str(BIN), "profile", "--files", str(listfile),
                    "--out-dir", str(outdir), "-o", "json-schema"],
                   env={**os.environ, "FINETYPE_MODEL": model},
                   capture_output=True, text=True)
    labels = {}
    for p in outdir.glob("*.json"):
        try:
            s = json.loads(p.read_text())
        except ValueError:
            continue
        for name, spec in (s.get("properties") or {}).items():
            labels[(p.stem, name)] = spec.get("x-finetype-label", "unknown")
    return labels


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--model", required=True)
    ap.add_argument("--files-per-set", type=int, default=40)
    ap.add_argument("--out", default="output/attneg-retrain/damage_recovery.json")
    args = ap.parse_args()

    con = duckdb.connect()
    report, failed = {}, []
    with tempfile.TemporaryDirectory(prefix="dmgrec_") as work:
        for name, (frm, dsts, ok_fn, threshold) in SETS.items():
            files, cols = pick_columns(con, frm, dsts, args.files_per_set)
            if not cols:
                report[name] = {"error": "no columns found"}
                failed.append(name)
                continue
            setdir = Path(work) / name
            setdir.mkdir()
            labels = profile(args.model, files, setdir)
            hits = misses = 0
            outcome = Counter()
            for fp, col in cols:
                stem = Path(fp).stem
                lbl = labels.get((stem, col))
                if lbl is None:
                    continue
                outcome[lbl] += 1
                if ok_fn(lbl):
                    hits += 1
                else:
                    misses += 1
            total = hits + misses
            rate = hits / total if total else 0.0
            ok = total > 0 and rate >= threshold
            report[name] = {"columns_scored": total, "recovery_rate": round(rate, 3),
                            "threshold": threshold, "pass": ok,
                            "top_outcomes": outcome.most_common(5)}
            if not ok:
                failed.append(name)
            print(f"  {name:16s} {hits}/{total} = {rate:.1%} (need >={threshold:.0%}) "
                  f"{'PASS' if ok else 'FAIL'}  top={outcome.most_common(3)}")

    Path(args.out).parent.mkdir(parents=True, exist_ok=True)
    Path(args.out).write_text(json.dumps(
        {"model": args.model, "sets": report, "failed": failed}, indent=2) + "\n")
    print(f"DAMAGE-RECOVERY: {'GO' if not failed else 'NO-GO — failed: ' + ', '.join(failed)}")
    return 0 if not failed else 1


if __name__ == "__main__":
    sys.exit(main())
