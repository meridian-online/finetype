#!/usr/bin/env python3
"""Score the header-hint ablation: hints ON (A) vs OFF (B) vs DEFER (C), on the
33k stratified sample, using the corpus-scale instruments the April RHH ablation
could not (rare-type scoreboard + overall divergence). Answers: do the hardcoded
hints help or hurt at corpus scale, and does the defer-fix capture the best of both?

Runs after scripts/ablation_score expects the three passes at
output/eval-ceiling-diagnosis/abl_{A,B,C}/corpus_pass/columns.parquet.

Usage:  python3 scripts/ablation_score.py
"""
import subprocess, re
from importlib import util as _u

_spec = _u.spec_from_file_location("g", "scripts/build_rare_type_gold.py")
_g = _u.module_from_spec(_spec); _spec.loader.exec_module(_g)

ANSI = re.compile(r"\x1b\[[0-9;]*m")
BASE = "output/eval-ceiling-diagnosis"
CFG = [("A hints-ON", f"{BASE}/abl_A/corpus_pass/columns.parquet"),
       ("B hints-OFF", f"{BASE}/abl_B/corpus_pass/columns.parquet"),
       ("C hints-DEFER", f"{BASE}/abl_C/corpus_pass/columns.parquet")]

def duck(sql):
    out = subprocess.run(["duckdb", "-init", "/dev/null", "-noheader", "-list", "-c", sql],
                         capture_output=True, text=True)
    return [ANSI.sub("", l).strip() for l in out.stdout.splitlines() if l.strip()]

def scoreboard(parquet):
    rows = {}
    for l in duck(_g.sql(parquet)):
        p = [x.strip() for x in l.split("|")]
        if len(p) == 4 and p[2].isdigit():
            rows[(p[0], p[1])] = (int(p[2]), int(p[3]))
    return rows

def main():
    # --- rare-type scoreboard per config ---
    print(f"\n=== rare-type scoreboard ===")
    print(f"{'config':<15}{'lat FP-rate↓':>14}{'url recall↑':>13}{'utc FP-rate↓':>14}")
    print("-" * 56)
    data = {}
    for label, pq in CFG:
        d = scoreboard(pq); data[label] = d
        lneg, lfp = d.get(("latitude", "neg"), (0, 0))
        upos, utp = d.get(("url", "pos"), (0, 0))
        uneg, ufp = d.get(("utc", "neg"), (0, 0))
        print(f"{label:<15}{(lfp/lneg if lneg else 0):>14.4f}"
              f"{(utp/upos if upos else 0):>13.3f}{(ufp/uneg if uneg else 0):>14.4f}")

    # --- overall divergence: how much do hints change predictions vs A ---
    print(f"\n=== overall divergence from A (hints ON) ===")
    a = CFG[0][1]
    for label, pq in CFG[1:]:
        r = duck(f"""
          SELECT count(*) tot, sum((a.sense_prediction != b.sense_prediction)::INT) diff
          FROM '{a}' a JOIN '{pq}' b USING (file_path, column_name)""")
        for line in r:
            p = [x.strip() for x in line.split("|")]
            if len(p) == 2 and p[0].isdigit():
                tot, diff = int(p[0]), int(p[1])
                print(f"  {label}: {diff}/{tot} cols changed ({diff/tot*100:.1f}%)")

    # --- net oracle agreement (needs the v19 oracle, joined by sha+col) ---
    print(f"\n=== net agreement with the v19 gated-YDF oracle (matched cols) ===")
    oracle = "output/ydf-validation-gate/v19_gated.parquet"
    for label, pq in CFG:
        r = duck(f"""
          SELECT count(*) n,
            round(avg((c.sense_prediction = o.ydf_prediction_gated)::INT), 4) agree
          FROM '{pq}' c JOIN '{oracle}' o USING (file_content_sha256, column_name)
          WHERE o.ydf_prediction_gated IS NOT NULL AND o.ydf_prediction_gated <> ''""")
        for line in r:
            p = [x.strip() for x in line.split("|")]
            if len(p) == 2 and p[0].isdigit():
                print(f"  {label}: {p[1]} agreement over {p[0]} oracle-opining cols")
    print("\nNB: oracle agreement is bulk-numeric-weighted (the blind-ruler caveat);")
    print("read the rare-type scoreboard for the contested types, and run the")
    print("corpus-honest gate (B,C vs v19 baseline) for cross-type regressions.")

main()
