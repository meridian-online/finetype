#!/usr/bin/env python3
"""Export a human spot-check sample of the rare-type gold set (choice 0093's
canonical-isation step). Confirming ~120 rows upgrades the scoreboard from a
proven relative comparator to an absolute accuracy meter.

For each type it samples (deterministically, by header hash):
  - positives  — header says T; confirm the column really IS T
  - FP columns — header says not-T but the SHIPPED model (v19) calls it T;
                 confirm these are real false positives (the negatives hold)

Output: output/eval-ceiling-diagnosis/review_sample.csv with a blank
`human_verdict` column (correct / wrong / unsure). Re-run a verified copy
through build_rare_type_gold.py's gold logic to get the human-checked accuracy.

Usage:  python3 scripts/rare_type_gold_review.py
"""
import subprocess, re, csv

ANSI = re.compile(r"\x1b\[[0-9;]*m")
PARQUET = "output/ydf-validation-gate/v19_gated.parquet"
PER_BUCKET = 40  # positives and FP-columns sampled per type

# (label, pos_predicate|None, neg_predicate) — mirrors build_rare_type_gold.py
from importlib import util as _u
_spec = _u.spec_from_file_location("g", "scripts/build_rare_type_gold.py")
_g = _u.module_from_spec(_spec); _spec.loader.exec_module(_g)
TYPES, SEP = _g.TYPES, _g.SEP

def sql():
    branches = []
    for typ, (label, pos, neg) in TYPES.items():
        pos_when = f"WHEN {pos} THEN 'pos'" if pos else ""
        # sample positives, and negatives the model wrongly calls T (true FPs)
        branches.append(f"""
SELECT * FROM (
  SELECT '{typ}' typ, grole, h, sp,
         array_to_string(list_slice(vals,1,6),' | ') sample6, n
  FROM tagged_{typ}
  WHERE (grole='pos') OR (grole='neg' AND sp='{label}')
  ORDER BY grole, hash(h) LIMIT {2*PER_BUCKET}
)""")
        # NB: per-type role CTE below
    ctes = []
    for typ, (label, pos, neg) in TYPES.items():
        pos_when = f"WHEN {pos} THEN 'pos'" if pos else ""
        ctes.append(f"""tagged_{typ} AS (
  SELECT h, sp, n, vals,
    CASE {pos_when} WHEN {neg} THEN 'neg' ELSE NULL END grole
  FROM vs )""")
    return f"""
WITH base AS (
  SELECT lower(trim(column_name)) h, sense_prediction sp,
         string_split(sample_values_truncated, {SEP}) vals
  FROM '{PARQUET}' WHERE sample_values_truncated IS NOT NULL AND sample_values_truncated <> ''
),
vs AS (
  SELECT h, sp, vals, len(vals) n,
    len(list_filter(vals, v -> TRY_CAST(v AS DOUBLE) IS NOT NULL)) n_num,
    len(list_filter(vals, v -> TRY_CAST(v AS BIGINT) IS NOT NULL)) n_int,
    len(list_filter(vals, v -> TRY_CAST(v AS DOUBLE) BETWEEN -90 AND 90 AND contains(v,'.'))) n_latlike,
    len(list_filter(vals, v -> regexp_matches(lower(v), '^\\s*(https?://|www\\.)'))) n_url,
    len(list_filter(vals, v -> TRY_CAST(v AS DOUBLE) IS NULL AND length(trim(v))>0)) n_str
  FROM base ),
{','.join(ctes)}
{' UNION ALL '.join(branches)}
ORDER BY typ, grole;
"""

def main():
    out = subprocess.run(["duckdb", "-init", "/dev/null", "-noheader", "-csv",
                          "-c", sql()], capture_output=True, text=True)
    rows = []
    for line in out.stdout.splitlines():
        line = ANSI.sub("", line)
        parts = next(csv.reader([line]))
        if len(parts) == 6 and parts[0] in TYPES:
            typ, grole, h, sp, sample6, n = parts
            label = TYPES[typ][0]
            trusted = label if grole == "pos" else f"not {label}"
            rows.append([typ, grole, h, n, sample6, trusted, sp, "", ""])
    dst = "output/eval-ceiling-diagnosis/review_sample.csv"
    with open(dst, "w", newline="") as f:
        w = csv.writer(f)
        w.writerow(["type", "role", "header", "n_values", "sample_values",
                    "trusted_label", "v19_prediction", "human_verdict", "note"])
        w.writerows(rows)
    pos = sum(r[1] == "pos" for r in rows)
    fp = sum(r[1] == "neg" for r in rows)
    print(f"wrote {len(rows)} rows ({pos} positives, {fp} model-FP columns) -> {dst}")
    print("Fill `human_verdict` (correct / wrong / unsure) to canonicalise the gold set.")

main()
