#!/usr/bin/env python3
"""ac-01 efficacy probe for spec 2026-06-16-finish-sibling-context-attention.

Question: does cross-column (sibling/table) context SEPARATE the boundaries v19
confuses, beyond what the column's own values reveal? If yes, sibling-context
attention has a signal to learn; if no, it is not our lever.

Method: for each gold column whose source table is on disk, read its REAL
siblings (every other column in the table) and compute, per boundary:
  - own value-shape features (what the column alone shows)
  - table-context features (what the siblings reveal)
Then measure separability of the confused class pair with each feature set.

No model training, no network. Reads gittables parquets + local gold CSVs.
"""
import csv, os, re, statistics

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
GOLD = os.path.join(REPO, "eval/gold/gold_corpus_v1.tsv")
DATASETS = "/Users/hugh/datasets/"           # gittables live here
SAMPLE_N = 40

import duckdb  # venv python

# ---- column value-shape detectors -------------------------------------------
LAT_RE = re.compile(r"\b(lat|latitude)\b", re.I)
LON_RE = re.compile(r"\b(lon|lng|long|longitude)\b", re.I)
MONEY_RE = re.compile(r"(salary|wage|pay|gross|price|amount|cost|fee|income|"
                      r"revenue|balance|budget|paid|earnings|usd|gbp|eur|"
                      r"compensation|net|total)", re.I)

def floats(vals):
    out = []
    for v in vals:
        if v is None:
            continue
        s = str(v).strip().replace(",", "")
        try:
            f = float(s)
        except ValueError:
            continue
        if f == f and abs(f) != float("inf"):   # drop NaN/inf
            out.append(f)
    return out

def shape(name, vals):
    """Value-shape summary of one column."""
    name = str(name)
    fs = floats(vals)
    n = max(len(vals), 1)
    frac_float = len(fs) / n
    in90 = sum(1 for x in fs if abs(x) <= 90) / max(len(fs), 1) if fs else 0.0
    in180 = sum(1 for x in fs if abs(x) <= 180) / max(len(fs), 1) if fs else 0.0
    has_gt90 = any(90 < abs(x) <= 180 for x in fs)
    has_dec = any(abs(x - round(x)) > 1e-9 for x in fs)
    return dict(name=name, frac_float=frac_float, in90=in90, in180=in180,
                has_gt90=has_gt90, has_dec=has_dec, n_float=len(fs))

def is_coord_like(sh):
    """Sibling looks like a coordinate (by name or by value-range)."""
    if LAT_RE.search(sh["name"]) or LON_RE.search(sh["name"]):
        return True
    # mostly-float, all within [-180,180], decimal-valued
    return sh["frac_float"] >= 0.8 and sh["in180"] >= 0.95 and sh["has_dec"]

def is_money_name(name):
    return bool(MONEY_RE.search(name))

# ---- table reader ------------------------------------------------------------
con = duckdb.connect()

def read_table(path):
    """Return {col_name: [sample values]} for a parquet/csv table, or None."""
    try:
        if path.endswith(".parquet"):
            rel = f"read_parquet('{path}')"
        else:
            rel = f"read_csv_auto('{path}', SAMPLE_SIZE=200, ignore_errors=true)"
        cols = [c[0] for c in con.execute(f"DESCRIBE SELECT * FROM {rel}").fetchall()]
        if not cols:
            return None
        df = con.execute(f"SELECT * FROM {rel} LIMIT {SAMPLE_N}").fetchdf()
        return {c: df[c].tolist() for c in cols}
    except Exception:
        return None

def resolve(file_path):
    cand = os.path.join(DATASETS, file_path)
    if os.path.exists(cand):
        return cand
    cand2 = os.path.join(REPO, file_path)
    if os.path.exists(cand2):
        return cand2
    return None

# ---- load gold + build per-column context -----------------------------------
gold = list(csv.DictReader(open(GOLD), delimiter="\t"))

# cache tables by resolved path
table_cache = {}
def get_table(fp):
    rp = resolve(fp)
    if rp is None:
        return None
    if rp not in table_cache:
        table_cache[rp] = read_table(rp)
    return table_cache[rp]

records = []   # one per gold col we could resolve: label + own shape + sibling features
for r in gold:
    lab = r["curated_label"]
    tbl = get_table(r["file_path"])
    if not tbl:
        continue
    target = r["column_name"]
    if target not in tbl:
        # try case/space-insensitive match
        m = {k.strip().lower(): k for k in tbl}
        if target.strip().lower() in m:
            target = m[target.strip().lower()]
        else:
            continue
    own = shape(target, tbl[target])
    sibs = [shape(c, v) for c, v in tbl.items() if c != target]
    n_coord_sib = sum(1 for s in sibs if is_coord_like(s))
    n_coord_name_sib = sum(1 for s in sibs if LAT_RE.search(s["name"]) or LON_RE.search(s["name"]))
    n_money_name_sib = sum(1 for s in sibs if is_money_name(s["name"]))
    target_money_name = is_money_name(target)
    records.append(dict(
        label=lab, ncols=len(tbl),
        own_in90=own["in90"], own_in180=own["in180"], own_has_gt90=own["has_gt90"],
        own_frac_float=own["frac_float"], own_has_dec=own["has_dec"],
        n_coord_sib=n_coord_sib, n_coord_name_sib=n_coord_name_sib,
        n_money_name_sib=n_money_name_sib, target_money_name=target_money_name,
    ))

print(f"resolved {len(records)} / {len(gold)} gold columns with source tables\n")

# ---- separability helpers ----------------------------------------------------
def auc(pos, neg):
    """Mann-Whitney AUC of a single numeric feature (pos should score higher)."""
    if not pos or not neg:
        return float("nan")
    wins = ties = 0
    for a in pos:
        for b in neg:
            if a > b: wins += 1
            elif a == b: ties += 1
    return (wins + 0.5 * ties) / (len(pos) * len(neg))

def rate(recs, pred):
    sel = [r for r in recs if pred(r)]
    return len(sel)

def summarise(recs, feat):
    vals = [r[feat] for r in recs]
    return f"mean={statistics.mean(vals):.2f} ≥1={sum(1 for v in vals if v>=1)/len(vals):.0%}" if vals else "n=0"

# ---- BOUNDARY 1: coordinates vs decimal -------------------------------------
print("="*72)
print("BOUNDARY 1 — latitude/longitude  vs  decimal_number")
print("="*72)
lat = [r for r in records if r["label"]=="geography.coordinate.latitude"]
lon = [r for r in records if r["label"]=="geography.coordinate.longitude"]
dec = [r for r in records if r["label"]=="representation.numeric.decimal_number"]
coords = lat + lon
print(f"  resolved: latitude={len(lat)} longitude={len(lon)} decimal={len(dec)}")
for nm, recs in [("latitude", lat), ("longitude", lon), ("DECIMAL(neg)", dec)]:
    print(f"  {nm:14s}  coord-named sibling: {summarise(recs,'n_coord_name_sib')}"
          f"   |  coord-shaped sibling: {summarise(recs,'n_coord_sib')}")
# separability: does 'has coord-named sibling' separate coords from decimals?
pos = [r["n_coord_name_sib"] for r in coords]
neg = [r["n_coord_name_sib"] for r in dec]
print(f"\n  AUC(coord-named-sibling | coords vs decimal)   = {auc(pos,neg):.3f}")
pos2 = [r["n_coord_sib"] for r in coords]; neg2=[r["n_coord_sib"] for r in dec]
print(f"  AUC(coord-shaped-sibling | coords vs decimal)  = {auc(pos2,neg2):.3f}")
own_pos=[r["own_in90"] for r in lat]; own_neg=[r["own_in90"] for r in dec]
print(f"  (ref) AUC(own in-[-90,90] | latitude vs decimal) = {auc(own_pos,own_neg):.3f}"
      "  <- value-only baseline")

# ---- BOUNDARY 2: currency vs decimal/integer --------------------------------
print("\n"+"="*72)
print("BOUNDARY 2 — finance.currency.amount  vs  decimal/integer")
print("="*72)
cur = [r for r in records if r["label"]=="finance.currency.amount"]
num = [r for r in records if r["label"] in
       ("representation.numeric.decimal_number","representation.numeric.integer_number")]
print(f"  resolved: currency={len(cur)} (of 5)  numeric(neg)={len(num)}")
for nm, recs in [("currency", cur), ("NUMERIC(neg)", num)]:
    if not recs: continue
    mn = sum(r["target_money_name"] for r in recs)/len(recs)
    print(f"  {nm:14s}  target money-name: {mn:.0%}   money-named sibling: {summarise(recs,'n_money_name_sib')}")
if cur:
    pos=[r["n_money_name_sib"] for r in cur]; neg=[r["n_money_name_sib"] for r in num]
    print(f"\n  AUC(money-named-sibling | currency vs numeric) = {auc(pos,neg):.3f}")
    pos=[1.0 if r["target_money_name"] else 0.0 for r in cur]
    neg=[1.0 if r["target_money_name"] else 0.0 for r in num]
    print(f"  AUC(target-money-NAME  | currency vs numeric)  = {auc(pos,neg):.3f}  <- header-only")

print("\n[done]")
