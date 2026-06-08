#!/usr/bin/env python3
"""Prototype: a trusted, oracle-free precision metric for the contested rare
types the corpus metric is blind to (finding: output/eval-ceiling-diagnosis/).

The corpus precision metric (~0.49 vs gated-YDF) cannot see rare-type fixes:
raw YDF proposes `latitude` 8x across 6.6M columns, so it never enters the
denominator. This builds an INDEPENDENT gold set for the latitude boundary
straight from the corpus, using the only trustworthy signal for these
header-identifiable types — the HEADER — and validates it against the values:

  positives  = header is an unambiguous latitude name  AND values are decimals
               in [-90, 90]  (genuinely latitude)
  hard-negs  = header is a clearly non-coordinate quantity (population, score,
               error, rms, magnitude, ...) AND values are decimals in [-90, 90]
               (value-confusable with latitude — the columns models wrongly flip)

Then it scores each model's `sense_prediction` on the gold set:

  recall    = positives the model calls latitude
  FP-rate   = hard-negatives the model wrongly calls latitude   <- the battle
  precision = TP / (TP + FP)

This number MOVES when a round relocates latitude FPs — exactly what the corpus
metric cannot register. No oracle involved.

Usage:  python3 scripts/build_rare_type_gold.py
"""
import subprocess, re

ANSI = re.compile(r"\x1b\[[0-9;]*m")

# (model label, parquet path).  All carry: column_name, sense_prediction,
# sample_values_truncated. v19/22/23 = full corpus; latdec/v0624/fusion_v27 =
# 33k stratified sample (rate is comparable across samples).
MODELS = [
    ("v19 (shipped)", "output/ydf-validation-gate/v19_gated.parquet"),
    ("v22",           "output/ydf-validation-gate/v22_gated.parquet"),
    ("v23",           "output/ydf-validation-gate/v23_gated.parquet"),
    ("latdec",        "eval/gittables/corpus_pass_latdec/corpus_pass/columns.parquet"),
    ("v0624",         "output/corpus-honest-gate/v0624_pass/corpus_pass/columns.parquet"),
    ("fusion_v27",    "output/corpus-honest-gate/fusion_v27_pass/corpus_pass/columns.parquet"),
]

SEP = "chr(9474)"  # U+2502, the sample-value delimiter
LAT = "geography.coordinate.latitude"

# Full-match (anchored) latitude header names — excludes false friends like
# "translate"/"platitude"/"latrine" because the WHOLE header must match.
POS_RE = r'(x|y|geo|dec|decimal|wgs84)?[ _-]?lat(itude)?[ _-]?(dd|deg|degrees|decimal|n|s|wgs84|coord|gms|new|x|y|[0-9])?'
# Substring match — header names a non-coordinate quantity.
NEG_RE = (r'(population|score|rating|\brate\b|ratio|error|\berr\b|rms|residual|'
          r'magnitude|\bmag\b|depth|temperature|\btemp\b|price|cost|amount|'
          r'weight|height|\bage\b|percent|\bpct\b|probability|\bprob\b|'
          r'correlation|coefficient|\bcoef\b|\bmean\b|median|stdev|\bstd\b|'
          r'variance|\bvar\b|salary|income|revenue|\bgdp\b|density|frequency|'
          r'voltage|velocity|pressure|balance|elevation|distance|speed)')

def sql(parquet):
    return f"""
WITH base AS (
  SELECT lower(trim(column_name)) h, sense_prediction sp,
         string_split(sample_values_truncated, {SEP}) vals
  FROM '{parquet}'
  WHERE sample_values_truncated IS NOT NULL AND sample_values_truncated <> ''
),
vs AS (
  SELECT h, sp,
    len(vals) n,
    len(list_filter(vals, v -> TRY_CAST(v AS DOUBLE) IS NOT NULL)) n_num,
    len(list_filter(vals, v -> TRY_CAST(v AS DOUBLE) BETWEEN -90 AND 90 AND contains(v,'.'))) n_latlike
  FROM base
),
roled AS (
  SELECT sp, n, n_num, n_latlike,
    CASE WHEN regexp_full_match(h, '{POS_RE}') THEN 'pos'
         WHEN regexp_matches(h, '{NEG_RE}')    THEN 'neg'
         ELSE NULL END grole
  FROM vs
),
gold AS (
  -- positives: a real latitude column (numeric, decimals in [-90,90]).
  -- hard-negs: a numeric column whose header names a non-coordinate quantity
  -- (header is the trusted 'not latitude' label; the model flipping it = FP).
  SELECT sp, grole FROM roled
  WHERE (grole='pos' AND n>=5 AND n_num>=n*0.9 AND n_latlike>=n_num*0.8)
     OR (grole='neg' AND n>=5 AND n_num>=n*0.9)
)
SELECT grole, count(*) cols, sum((sp='{LAT}')::INT) pred_lat
FROM gold GROUP BY grole ORDER BY grole;
"""

def run(parquet):
    out = subprocess.run(["duckdb", "-init", "/dev/null", "-noheader", "-list",
                          "-c", sql(parquet)], capture_output=True, text=True)
    res = {"pos": (0, 0), "neg": (0, 0)}
    for line in out.stdout.splitlines():
        parts = [p.strip() for p in ANSI.sub("", line).strip().split("|")]
        if len(parts) == 3 and parts[0] in res and parts[1].isdigit():
            res[parts[0]] = (int(parts[1]), int(parts[2]))
    return res

def main():
    rows = []
    for label, pq in MODELS:
        r = run(pq)
        npos, tp = r["pos"]; nneg, fp = r["neg"]
        recall = tp / npos if npos else float("nan")
        fprate = fp / nneg if nneg else float("nan")
        prec = tp / (tp + fp) if (tp + fp) else float("nan")
        rows.append((label, npos, recall, nneg, fprate, prec))

    print(f"\n{'model':<16}{'lat+ cols':>10}{'recall':>9}{'lat-neg':>9}{'FP-rate':>9}{'precision':>11}")
    print("-" * 64)
    for label, npos, rec, nneg, fpr, prec in rows:
        print(f"{label:<16}{npos:>10}{rec:>9.3f}{nneg:>9}{fpr:>9.3f}{prec:>11.3f}")
    print("\nFP-rate = hard-negative decimal columns (population/score/error/rms/…)")
    print("the model wrongly calls latitude. This is the precision battle the")
    print("corpus metric cannot see; here it moves per model.")

main()
