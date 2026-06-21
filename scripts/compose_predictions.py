#!/usr/bin/env python3
"""Compose any model's standalone predictions through the REAL Sharpen stack
(spec gte honest gate, 2026-06-21).

A gte model isn't in the binary, so it can't run `profile`. But the Sharpen rules are
value-based (decision 0048) and decoupled from the embedding. This injects the model's
predicted label as the Sense result (FINETYPE_INJECT_LABEL) and runs the real profile
Sharpen on it — giving composed predictions for any model, no candle needed.

Usage:
  compose_predictions.py --preds <standalone.tsv> --out <composed.tsv>
  # standalone.tsv: file_content_sha256<TAB>column_name<TAB>predicted_label[...]
"""
import argparse
import csv
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

_S = os.path.dirname(os.path.abspath(__file__))
if _S not in sys.path:
    sys.path.insert(0, _S)
from score_gold_anchor import SEP, _vendored_values, load_gold  # noqa: E402

BIN = str(Path("target/release/finetype").resolve())
GOLD = "eval/gold/gold_corpus.tsv"
COLS = "eval/gittables/corpus_pass/columns.parquet"


def compose_one(column_name, values, inject_label):
    with tempfile.TemporaryDirectory() as td:
        p = Path(td) / "c.csv"
        with p.open("w", newline="") as fh:
            w = csv.writer(fh, lineterminator="\n")
            w.writerow([column_name])
            for v in values:
                w.writerow([v])
        env = dict(os.environ, FINETYPE_INJECT_LABEL=inject_label)
        r = subprocess.run([BIN, "profile", "-f", str(p), "-o", "json-schema"],
                           capture_output=True, text=True, timeout=120, env=env)
        if r.returncode != 0:
            return ""
        defn = json.loads(r.stdout).get("properties", {}).get(column_name, {})
        return defn.get("x-finetype-label", "") if isinstance(defn, dict) else ""


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--preds", required=True, help="standalone predictions TSV")
    ap.add_argument("--out", required=True)
    a = ap.parse_args()

    import pyarrow.parquet as pq
    preds = {(r["file_content_sha256"], r["column_name"]): r["predicted_label"]
             for r in csv.DictReader(open(a.preds), delimiter="\t")}
    gold = {(r["file_content_sha256"], r["column_name"]): r for r in load_gold(Path(GOLD))}
    tbl = pq.read_table(COLS, columns=["file_content_sha256", "column_name", "sample_values_truncated"])
    samples = {}
    for r in tbl.to_pylist():
        k = (r.get("file_content_sha256") or "", r.get("column_name") or "")
        if k in preds and k not in samples:
            samples[k] = [v for v in (r.get("sample_values_truncated") or "").split(SEP) if v]

    n = 0
    with open(a.out, "w") as fh:
        w = csv.writer(fh, delimiter="\t", lineterminator="\n")
        w.writerow(["file_content_sha256", "column_name", "predicted_label"])
        for k, inj in preds.items():
            vals = samples.get(k) or (_vendored_values(gold[k].get("file_path", ""), k[1]) if k in gold else [])
            if not vals:
                continue
            w.writerow([k[0], k[1], compose_one(k[1], vals, inj) or inj])
            n += 1
            if n % 100 == 0:
                print(f"  {n}/{len(preds)}", file=sys.stderr)
    print(f"composed {n} -> {a.out}", file=sys.stderr)


if __name__ == "__main__":
    main()
