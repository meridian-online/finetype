#!/usr/bin/env python3
"""Reliability curve for FineType's per-column confidence (spec 2026-06-18-
calibrated-confidence-abstention ac-00).

For each fixture column: pull the stored ~8-value sample from columns.parquet
(same contract as score_gold_anchor.predict), profile it via `finetype profile
-o json`, capture (confidence, predicted type, disambiguation_rule, vetoed),
and mark correct = reframe(pred) == reframe(truth). Then bin by confidence and
report per-bin accuracy + count — the reliability curve.

NO model change: this reads the shipped confidence, it does not alter it.
"""
import argparse
import csv
import json
import subprocess
import tempfile
from pathlib import Path

import pyarrow.parquet as pq

RESIDUAL = {
    "representation.discrete.categorical",
    "representation.text.word",
    "representation.text.plain_text",
}
SEP = "│"  # score_gold_anchor SEP


def rf(x):
    return "representation.text.RESIDUAL" if x in RESIDUAL else x


def profile_json(binary, col, values):
    with tempfile.TemporaryDirectory() as td:
        p = Path(td) / "c.csv"
        with p.open("w", newline="") as fh:
            w = csv.writer(fh, lineterminator="\n")
            w.writerow([col])
            for v in values:
                w.writerow([v])
        proc = subprocess.run([str(binary), "profile", "-f", str(p), "-o", "json"],
                              capture_output=True, text=True, timeout=120)
        if proc.returncode != 0:
            return None
        d = json.loads(proc.stdout)
    cols = d.get("columns", [])
    return cols[0] if cols else None


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--gold", required=True)
    ap.add_argument("--columns", default="eval/gittables/corpus_pass/columns.parquet")
    ap.add_argument("--binary", default="target/release/finetype")
    ap.add_argument("--out", required=True)
    args = ap.parse_args()

    truth = {}
    for r in csv.DictReader(open(args.gold), delimiter="\t"):
        truth[(r["file_content_sha256"], r["column_name"])] = r["curated_label"]

    tbl = pq.read_table(args.columns,
                        columns=["file_content_sha256", "column_name", "sample_values_truncated"])
    sha = tbl.column("file_content_sha256").to_pylist()
    cn = tbl.column("column_name").to_pylist()
    sv = tbl.column("sample_values_truncated").to_pylist()
    samples = {}
    for s, c, v in zip(sha, cn, sv):
        if (s, c) in truth:
            samples[(s, c)] = (v or "").split(SEP)

    rows = []
    for (s, c), t in truth.items():
        vals = samples.get((s, c))
        if not vals:
            continue
        col = profile_json(args.binary, c, vals)
        if not col:
            continue
        rows.append({
            "sha": s, "column": c, "true": t, "pred": col.get("type", ""),
            "confidence": col.get("confidence", 0.0),
            "quality_band": col.get("quality_band", ""),
            "runner_up": col.get("runner_up", ""),
            "rule": col.get("disambiguation_rule") or "",
            "vetoed": col.get("validation_vetoed", False),
            "correct": int(rf(col.get("type", "")) == rf(t)),
        })

    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    with out.open("w", newline="") as fh:
        w = csv.DictWriter(fh, fieldnames=["sha", "column", "true", "pred",
                                           "confidence", "quality_band", "runner_up",
                                           "rule", "vetoed", "correct"],
                           delimiter="\t")
        w.writeheader()
        w.writerows(rows)

    # reliability curve — fixed bins
    bins = [(0.0, 0.5), (0.5, 0.7), (0.7, 0.85), (0.85, 0.95), (0.95, 1.0001)]
    print(f"\n=== {args.gold} — reliability curve (n={len(rows)}) ===")
    print(f"{'conf bin':>14} {'n':>5} {'acc':>7} {'mean_conf':>10}")
    for lo, hi in bins:
        b = [r for r in rows if lo <= r["confidence"] < hi]
        if not b:
            continue
        acc = sum(r["correct"] for r in b) / len(b)
        mc = sum(r["confidence"] for r in b) / len(b)
        print(f"  [{lo:.2f},{hi:.2f}) {len(b):>5} {acc:>7.3f} {mc:>10.3f}")
    # rule-override vs pure-vote split (hardcoded/rule confidences pollute the curve)
    ruled = [r for r in rows if r["rule"]]
    pure = [r for r in rows if not r["rule"]]
    for name, grp in [("rule/veto-set", ruled), ("pure-vote", pure)]:
        if grp:
            acc = sum(r["correct"] for r in grp) / len(grp)
            mc = sum(r["confidence"] for r in grp) / len(grp)
            print(f"  {name:>14}: n={len(grp)} acc={acc:.3f} mean_conf={mc:.3f}")
    print(f"wrote {len(rows)} -> {out}")


if __name__ == "__main__":
    raise SystemExit(main())
