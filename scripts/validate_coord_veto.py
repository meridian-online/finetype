#!/usr/bin/env python3
"""Dry-run validation of the 0094 coordinate header-veto rule against the
rare-type scoreboard, over existing predictions (the rule is deterministic in
header + values, both present in the parquets — no corpus pass needed).

Replicates the Rust `header_corroborates_coordinate` + value-numeric gate in
SQL, then for each round computes the latitude FP-rate and recall BEFORE and
AFTER the rule, plus the recall-risk set (genuine-latitude gold positives the
rule would wrongly demote because their header carries no coordinate token).

Usage:  python3 scripts/validate_coord_veto.py
"""
import subprocess, re

ANSI = re.compile(r"\x1b\[[0-9;]*m")
SEP = "chr(9474)"
LAT = "geography.coordinate.latitude"

MODELS = [
    ("v19 (shipped)", "output/ydf-validation-gate/v19_gated.parquet"),
    ("v22",           "output/ydf-validation-gate/v22_gated.parquet"),
    ("v23",           "output/ydf-validation-gate/v23_gated.parquet"),
]

# SQL mirror of header_corroborates_coordinate(h): substring tokens OR a
# coordinate token split on non-alphanumerics. (header_hint's lat/lon arms are
# subsumed by the 'lat'/'lon'/... token set, so this matches the Rust.)
CORROB = """(
  h LIKE '%latitude%' OR h LIKE '%longitude%' OR h LIKE '%coord%'
  OR h LIKE '%wgs84%' OR h LIKE '%northing%' OR h LIKE '%easting%'
  OR len(list_filter(regexp_split_to_array(h, '[^a-z0-9]+'),
      t -> t IN ('lat','lon','lng','long','lats','lons','latdd','londd','gps'))) > 0
)"""
# value gate: >=80% of sample values numeric
NUMERIC = "n_num*100 >= n*80"

# Latitude gold set, identical header rules to build_rare_type_gold.py.
POS = r"regexp_full_match(h, '(x|y|geo|dec|decimal|wgs84)?[ _-]?lat(itude)?[ _-]?(dd|deg|degrees|decimal|n|s|wgs84|coord|gms|new|x|y|[0-9])?')"
NEG = (r"regexp_matches(h, '(population|score|rating|\brate\b|ratio|error|\berr\b|rms|residual|"
       r"magnitude|\bmag\b|depth|temperature|\btemp\b|price|cost|amount|weight|height|\bage\b|"
       r"percent|\bpct\b|probability|\bprob\b|correlation|coefficient|\bcoef\b|\bmean\b|median|"
       r"stdev|\bstd\b|variance|\bvar\b|salary|income|revenue|\bgdp\b|density|frequency|voltage|"
       r"velocity|pressure|balance|elevation|distance|speed)')")

def sql(parquet):
    return f"""
WITH base AS (
  SELECT lower(trim(column_name)) h, sense_prediction sp,
         string_split(sample_values_truncated, {SEP}) vals
  FROM '{parquet}' WHERE sample_values_truncated IS NOT NULL AND sample_values_truncated <> ''
),
vs AS (
  SELECT h, sp, len(vals) n,
    len(list_filter(vals, v -> TRY_CAST(v AS DOUBLE) IS NOT NULL)) n_num,
    len(list_filter(vals, v -> TRY_CAST(v AS DOUBLE) BETWEEN -90 AND 90 AND contains(v,'.'))) n_latlike
  FROM base
),
g AS (
  SELECT *,
    CASE WHEN {POS} THEN 'pos' WHEN {NEG} THEN 'neg' END grole,
    {CORROB} corrob,
    (sp = '{LAT}') is_lat,
    -- rule: demote latitude when not corroborated AND numeric
    (sp = '{LAT}' AND NOT {CORROB} AND {NUMERIC}) demoted
  FROM vs
)
SELECT
  -- positives (genuine latitude): recall before/after, + recall-risk count
  sum((grole='pos' AND n>=5 AND n_num>=n*0.9 AND n_latlike>=n_num*0.8)::INT) pos_cols,
  sum((grole='pos' AND n>=5 AND n_num>=n*0.9 AND n_latlike>=n_num*0.8 AND is_lat)::INT) pos_lat_before,
  sum((grole='pos' AND n>=5 AND n_num>=n*0.9 AND n_latlike>=n_num*0.8 AND is_lat AND NOT demoted)::INT) pos_lat_after,
  -- hard negatives: FP before/after
  sum((grole='neg' AND n>=5 AND n_num>=n*0.9)::INT) neg_cols,
  sum((grole='neg' AND n>=5 AND n_num>=n*0.9 AND is_lat)::INT) neg_fp_before,
  sum((grole='neg' AND n>=5 AND n_num>=n*0.9 AND is_lat AND NOT demoted)::INT) neg_fp_after
FROM g;
"""

def run(parquet):
    out = subprocess.run(["duckdb", "-init", "/dev/null", "-noheader", "-list", "-c", sql(parquet)],
                         capture_output=True, text=True)
    for line in out.stdout.splitlines():
        p = [x.strip() for x in ANSI.sub("", line).strip().split("|")]
        if len(p) == 6 and all(x.isdigit() for x in p):
            return list(map(int, p))
    return None

def main():
    print(f"\n{'round':<16}{'recall before':>14}{'recall after':>14}{'FP before':>11}{'FP after':>10}{'FP drop':>9}")
    print("-" * 74)
    for label, pq in MODELS:
        r = run(pq)
        if not r:
            print(f"{label:<16}  (no data)"); continue
        pcol, pb, pa, ncol, fb, fa = r
        rb = pb / pcol if pcol else 0
        ra = pa / pcol if pcol else 0
        fpb = fb / ncol if ncol else 0
        fpa = fa / ncol if ncol else 0
        drop = (fpb - fpa) / fpb if fpb else 0
        print(f"{label:<16}{rb:>14.3f}{ra:>14.3f}{fpb:>11.4f}{fpa:>10.4f}{drop:>8.0%}")
    print("\nrecall after = genuine latitude still called latitude post-rule (recall must hold)")
    print("FP after     = non-coordinate columns still called latitude post-rule (lower = better)")

main()
