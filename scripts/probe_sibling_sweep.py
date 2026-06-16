#!/usr/bin/env python3
"""ac-01 sweep: which gold boundaries are SIBLING-RECOVERABLE?

Extends scripts/probe_sibling_context.py from 3 hand-picked boundaries to every
confusable boundary in the gold corpus. For each label L, finds where v19's
misses go (its top confusion partner M), then DATA-DRIVEN searches for the single
sibling-header token that best separates L's tables from M's tables. A high AUC =
the disambiguator lives in a neighbouring column → sibling-context attention can
learn it.

No model, no network. Reads real source tables (gittables + local fixtures).
"""
import csv, os, re
from collections import defaultdict, Counter

STOP = {"the","and","for","with","from","this","that","not","any","all","per",
        "min","max","avg","num","val","col","row","key","id"}

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
GOLD = os.path.join(REPO, "eval/gold/gold_corpus_v1.tsv")
PRED = os.path.join(REPO, "output/country-code-corrob/predictions_country-code-corrob.tsv")
DATASETS = "/Users/hugh/datasets/"
SAMPLE_N = 30
MIN_SUPPORT = 6          # min resolved gold cols per side of a boundary
MIN_TOKEN_TABLES = 3     # a candidate token must appear in >=3 tables of a side
AUC_BAR = 0.70           # report boundaries whose best sibling token clears this

import duckdb
con = duckdb.connect()

TOKEN_RE = re.compile(r"[a-z]{3,}")
def tokens(name):
    return {t for t in TOKEN_RE.findall(str(name).lower()) if t not in STOP}

def read_table(path):
    try:
        rel = (f"read_parquet('{path}')" if path.endswith(".parquet")
               else f"read_csv_auto('{path}', SAMPLE_SIZE=200, ignore_errors=true)")
        cols = [c[0] for c in con.execute(f"DESCRIBE SELECT * FROM {rel}").fetchall()]
        return cols or None
    except Exception:
        return None

def resolve(fp):
    for cand in (os.path.join(DATASETS, fp), os.path.join(REPO, fp)):
        if os.path.exists(cand):
            return cand
    return None

def auc(pos, neg):
    if not pos or not neg:
        return float("nan")
    w = t = 0
    for a in pos:
        for b in neg:
            if a > b: w += 1
            elif a == b: t += 1
    return (w + 0.5 * t) / (len(pos) * len(neg))

# ---- load gold + v19 predictions --------------------------------------------
gold = list(csv.DictReader(open(GOLD), delimiter="\t"))
pred = {}
for r in csv.DictReader(open(PRED), delimiter="\t"):
    pred[(r["file_content_sha256"], r["column_name"])] = r["predicted_label"]

# ---- per gold column: sibling header-token set ------------------------------
tbl_cache = {}
def sibling_tokens(fp, target):
    rp = resolve(fp)
    if rp is None:
        return None
    if rp not in tbl_cache:
        tbl_cache[rp] = read_table(rp)
    cols = tbl_cache[rp]
    if not cols:
        return None
    tmatch = target if target in cols else next(
        (c for c in cols if str(c).strip().lower() == target.strip().lower()), None)
    sib = set()
    for c in cols:
        if c == tmatch:
            continue
        sib |= tokens(c)
    return sib

recs = []   # (label, fn_dest, sibling_token_set)
for r in gold:
    st = sibling_tokens(r["file_path"], r["column_name"])
    if st is None:
        continue
    p = pred.get((r["file_content_sha256"], r["column_name"]))
    recs.append((r["curated_label"], p, st))

print(f"resolved {len(recs)} / {len(gold)} gold columns\n")

# ---- confusion partners: where each label's MISSES go -----------------------
by_label = defaultdict(list)
for lab, p, st in recs:
    by_label[lab].append((p, st))

fn_dest = {}
for lab, items in by_label.items():
    misses = Counter(p for p, st in items if p != lab and p)
    if misses:
        fn_dest[lab] = misses.most_common(1)[0][0]

# ---- token-bag per (label) --------------------------------------------------
def token_lists(lab):
    return [st for l, p, st in recs if l == lab]

def best_distinctive(labA, labB):
    """Sibling token most OVER-represented in A vs B (A-distinctive).
    Returns (token, rate_A, rate_B, gap). Direction-correct: rate_A > rate_B."""
    A = token_lists(labA); B = token_lists(labB)
    if len(A) < MIN_SUPPORT or len(B) < MIN_SUPPORT:
        return None
    cand = set()
    for st in A:
        cand |= st
    best = None
    for tok in cand:
        inA = sum(1 for st in A if tok in st)
        if inA < MIN_TOKEN_TABLES:
            continue
        rA = inA / len(A)
        rB = sum(1 for st in B if tok in st) / len(B)
        gap = rA - rB
        if best is None or gap > best[3]:
            best = (tok, rA, rB, gap)
    return best

# real-signal bar, calibrated to the coordinate exemplar (95% vs 2%):
def is_go(rA, rB, gap):
    return rA >= 0.50 and gap >= 0.40

short = lambda s: s.split(".")[-1]

# Boundaries = every label's top-miss destination, PLUS explicit positive
# controls (coordinates have ~0 gold misses so they don't appear as fn_dest).
boundaries = []
seen = set()
for lab, dest in fn_dest.items():
    boundaries.append((lab, dest)); seen.add((lab, dest))
for ctrl in [("geography.coordinate.latitude", "representation.numeric.decimal_number"),
             ("geography.coordinate.longitude", "representation.numeric.decimal_number"),
             ("finance.currency.amount", "representation.numeric.decimal_number")]:
    if ctrl not in seen:
        boundaries.append(ctrl)

print("="*92)
print(f"{'boundary  (label  →  confused-with)':50s} {'nA/nB':>7s}  best A-distinctive sibling  rateA/rateB")
print("="*92)
rows = []
for lab, dest in boundaries:
    nA = len(token_lists(lab)); nB = len(token_lists(dest))
    if nA < MIN_SUPPORT or nB < MIN_SUPPORT:
        continue
    bd = best_distinctive(lab, dest)
    if not bd:
        continue
    tok, rA, rB, gap = bd
    rows.append((gap, lab, dest, nA, nB, tok, rA, rB))

rows.sort(reverse=True)
for gap, lab, dest, nA, nB, tok, rA, rB in rows:
    flag = "  <= GO" if is_go(rA, rB, gap) else ""
    print(f"  {short(lab):22s} → {short(dest):20s} {nA:3d}/{nB:<3d}  "
          f"{('“'+tok+'”'):20s} {rA:.0%} / {rB:.0%}{flag}")

go = [r for r in rows if is_go(r[6], r[7], r[0])]
print(f"\nSIBLING-RECOVERABLE boundaries (rateA≥50%, gap≥40pp) — {len(go)} of {len(rows)}:")
for gap, lab, dest, nA, nB, tok, rA, rB in go:
    print(f"  • {short(lab)} vs {short(dest)}: a “{tok}” sibling appears in {rA:.0%} of "
          f"{short(lab)} tables but only {rB:.0%} of {short(dest)} tables")
if not go:
    print("  (none — the remaining gold recall gaps are NOT sibling-recoverable)")
print("\n[done]")
