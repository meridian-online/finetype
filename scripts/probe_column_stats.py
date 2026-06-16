#!/usr/bin/env python3
"""Probe: do cheap FULL-COLUMN statistics (the kind DuckDB hands you for free)
separate the recall-gap boundaries the per-value model misses?

Tests the column-statistics lever — cardinality ratio (distinct/rows), numeric
range (min/max), int-ness, and an increment signature — against the boundaries
that survived every model-side fix: the residual categories and coordinates.

Stats are computed with DuckDB aggregates over the FULL column (not a 100-row
sample), which is exactly what the DuckDB extension would read near-free.
No model, no neural inference.
"""
import csv, os
from collections import defaultdict

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
GOLD = os.path.join(REPO, "eval/gold/gold_corpus_v1.tsv")
DATASETS = "/Users/hugh/datasets/"

import duckdb
con = duckdb.connect()
con.execute("SET threads=4")

def resolve(fp):
    for c in (os.path.join(DATASETS, fp), os.path.join(REPO, fp)):
        if os.path.exists(c):
            return c
    return None

def rel(path):
    return (f"read_parquet('{path}')" if path.endswith(".parquet")
            else f"read_csv_auto('{path}', SAMPLE_SIZE=-1, ignore_errors=true)")

def colstats(path, col):
    """Full-column stats via DuckDB aggregates. Returns dict or None."""
    q = col.replace('"', '""')
    src = rel(path)
    try:
        # cast to varchar for safe distinct/length; try_cast to double for numeric
        row = con.execute(f'''
            SELECT
              count("{q}")                                  AS n,
              count(DISTINCT "{q}")                         AS nd,
              count(TRY_CAST("{q}" AS DOUBLE))              AS n_num,
              min(TRY_CAST("{q}" AS DOUBLE))                AS mn,
              max(TRY_CAST("{q}" AS DOUBLE))                AS mx,
              count(DISTINCT TRY_CAST("{q}" AS DOUBLE))     AS nd_num,
              sum(CASE WHEN TRY_CAST("{q}" AS DOUBLE) = floor(TRY_CAST("{q}" AS DOUBLE))
                       THEN 1 ELSE 0 END)                   AS n_int,
              avg(length(CAST("{q}" AS VARCHAR)))           AS avglen
            FROM {src}
        ''').fetchone()
    except Exception:
        return None
    n, nd, n_num, mn, mx, nd_num, n_int, avglen = row
    if not n or n < 5:
        return None
    card = nd / n
    num_frac = (n_num or 0) / n
    int_frac = (n_int or 0) / max(n_num or 0, 1)
    # increment signature: integer, near-unique, near-contiguous range
    incr = 0.0
    if num_frac >= 0.95 and int_frac >= 0.99 and mn is not None and mx is not None and nd_num:
        span = (mx - mn + 1)
        contig = nd_num / span if span > 0 else 0.0          # 1.0 = no gaps
        uniq = nd_num / n
        incr = min(contig, 1.0) * uniq                        # both → increment
    return dict(n=n, card=card, num_frac=num_frac, int_frac=int_frac,
                mn=mn if mn is not None else float("nan"),
                mx=mx if mx is not None else float("nan"),
                avglen=avglen or 0.0, incr=incr)

# ---- load gold, compute stats ------------------------------------------------
gold = list(csv.DictReader(open(GOLD), delimiter="\t"))
recs = defaultdict(list)   # label -> [stats]
done = 0
for r in gold:
    p = resolve(r["file_path"])
    if not p:
        continue
    # match column name case-insensitively
    s = colstats(p, r["column_name"])
    if s is None:
        # try resolving exact column name from schema
        try:
            cols = [c[0] for c in con.execute(f"DESCRIBE SELECT * FROM {rel(p)}").fetchall()]
            m = {str(c).strip().lower(): c for c in cols}
            cn = m.get(r["column_name"].strip().lower())
            if cn:
                s = colstats(p, cn)
        except Exception:
            s = None
    if s is None:
        continue
    recs[r["curated_label"]].append(s)
    done += 1

print(f"computed full-column stats for {done}/{len(gold)} gold columns\n")

def auc(pos, neg):
    pos = [x for x in pos if x == x]; neg = [x for x in neg if x == x]
    if not pos or not neg:
        return float("nan")
    w = t = 0
    for a in pos:
        for b in neg:
            if a > b: w += 1
            elif a == b: t += 1
    return (w + 0.5 * t) / (len(pos) * len(neg))

def feat(label, key):
    return [s[key] for s in recs.get(label, [])]

def line(name, a):
    bar = "  <= separates" if a == a and abs(a - 0.5) >= 0.20 else ""
    print(f"    {name:52s} AUC {a:.3f}{bar}")

CAT = "representation.discrete.categorical"
ALN = "representation.identifier.alphanumeric_id"
ORD = "representation.discrete.ordinal"
INT = "representation.numeric.integer_number"
INC = "representation.identifier.increment"
DEC = "representation.numeric.decimal_number"
LAT = "geography.coordinate.latitude"
LON = "geography.coordinate.longitude"

print("support:", {k.split('.')[-1]: len(recs.get(k, [])) for k in
                   [CAT, ALN, ORD, INT, INC, DEC, LAT, LON]})

print("\n=== B1  cardinality: categorical (low) vs alphanumeric_id (high) ===")
line("card_ratio  alnum>cat", auc(feat(ALN, "card"), feat(CAT, "card")))
print(f"    median card  categorical={sorted(feat(CAT,'card'))[len(feat(CAT,'card'))//2]:.3f}  "
      f"alnum={sorted(feat(ALN,'card'))[len(feat(ALN,'card'))//2]:.3f}")

print("\n=== B2  categorical vs its real FN destinations (cardinality) ===")
for dst in [ALN, ORD, "representation.text.entity_name", "geography.location.city"]:
    if recs.get(dst):
        line(f"card  {dst.split('.')[-1]}>cat", auc(feat(dst, "card"), feat(CAT, "card")))

print("\n=== B3  integer_number vs increment (increment signature) ===")
line("incr_signature  increment>integer", auc(feat(INC, "incr"), feat(INT, "incr")))
line("card_ratio      increment>integer", auc(feat(INC, "card"), feat(INT, "card")))

print("\n=== B4  coordinates vs decimal (numeric range — positive control) ===")
# latitude: |max|<=90 ; decimals: often larger. Use max-abs proxy via mx and mn.
def maxabs(label):
    return [max(abs(s["mn"]), abs(s["mx"])) for s in recs.get(label, [])
            if s["mn"] == s["mn"] and s["mx"] == s["mx"]]
line("max|val| decimal>latitude (lat in +-90)", auc(maxabs(DEC), maxabs(LAT)))
line("max|val| decimal>longitude(lon in +-180)", auc(maxabs(DEC), maxabs(LON)))

print("\n[done]")
