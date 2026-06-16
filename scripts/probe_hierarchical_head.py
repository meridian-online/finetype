#!/usr/bin/env python3
"""ac (layer-2) probe: would a hierarchical Domain→Family→Type head recover recall?

A hierarchical head can only fix an error whose COARSE node (domain/family) it can
get right while the flat leaf softmax went wrong — i.e. it prevents CROSS-domain /
CROSS-family leakage and rescues `unknown`. It cannot fix a WITHIN-family leaf
confusion (same coarse node, leaf still ambiguous).

So the cheap, decisive measurement: of v19's gold leaf ERRORS, how many cross a
domain / family boundary (recoverable in principle) vs stay within family
(irreducible by hierarchy)? And is v19 meaningfully better at DOMAIN than at LEAF
(the headroom a domain head could exploit)?

No model, no tables — just gold labels + v19 predictions.
"""
import csv, os
from collections import Counter

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
GOLD = os.path.join(REPO, "eval/gold/gold_corpus_v1.tsv")
PRED = os.path.join(REPO, "output/country-code-corrob/predictions_country-code-corrob.tsv")

def domain(lbl):
    return lbl.split(".")[0] if lbl and "." in lbl else (lbl or "unknown")
def family(lbl):
    p = (lbl or "").split(".")
    return ".".join(p[:2]) if len(p) >= 2 else (lbl or "unknown")

NONLEAF = {"unknown", "", None}    # predictions with no taxonomy domain

gold = list(csv.DictReader(open(GOLD), delimiter="\t"))
pred = {}
for r in csv.DictReader(open(PRED), delimiter="\t"):
    pred[(r["file_content_sha256"], r["column_name"])] = r["predicted_label"]

rows = []
for r in gold:
    g = r["curated_label"]
    p = pred.get((r["file_content_sha256"], r["column_name"]))
    rows.append((g, p))

N = len(rows)
leaf_ok   = sum(1 for g, p in rows if p == g)
family_ok = sum(1 for g, p in rows if family(p) == family(g))
domain_ok = sum(1 for g, p in rows if domain(p) == domain(g) and domain(g) != "unknown")

print(f"gold columns: {N}\n")
print(f"  LEAF   accuracy : {leaf_ok}/{N} = {leaf_ok/N:.3f}   (the headline)")
print(f"  FAMILY accuracy : {family_ok}/{N} = {family_ok/N:.3f}")
print(f"  DOMAIN accuracy : {domain_ok}/{N} = {domain_ok/N:.3f}")
print(f"  → domain-over-leaf headroom: +{(domain_ok-leaf_ok)/N:.3f} "
      f"({domain_ok-leaf_ok} cols where v19 gets the DOMAIN right but the LEAF wrong)\n")

# classify every LEAF ERROR
errs = [(g, p) for g, p in rows if p != g]
cls = Counter()
for g, p in errs:
    if p in NONLEAF:
        cls["pred=unknown (rescuable if domain known)"] += 1
    elif domain(p) != domain(g):
        cls["CROSS-domain (hierarchy can prevent)"] += 1
    elif family(p) != family(g):
        cls["cross-family / same-domain (maybe)"] += 1
    else:
        cls["WITHIN-family (hierarchy cannot fix)"] += 1

print(f"v19 leaf errors: {len(errs)} (of {N})")
for k in ["CROSS-domain (hierarchy can prevent)",
          "pred=unknown (rescuable if domain known)",
          "cross-family / same-domain (maybe)",
          "WITHIN-family (hierarchy cannot fix)"]:
    c = cls.get(k, 0)
    print(f"   {c:3d}  ({c/len(errs):.0%} of errors, {c/N:+.1%} of gold)  {k}")

ceiling = cls.get("CROSS-domain (hierarchy can prevent)", 0) + \
          cls.get("pred=unknown (rescuable if domain known)", 0)
print(f"\n  HIERARCHY RECALL CEILING (cross-domain + unknown-rescue): "
      f"{ceiling} cols = +{ceiling/N:.1%} gold  (only if a domain head is near-perfect)")

# per recall-gap label: where do its misses sit?
print("\n" + "="*78)
print("per recall-gap label — are its misses cross-domain (fixable) or within-family?")
print("="*78)
GAP_LABELS = ["representation.discrete.categorical",
              "representation.numeric.integer_number",
              "representation.text.plain_text",
              "representation.identifier.alphanumeric_id",
              "datetime.date.iso",
              "representation.numeric.decimal_number"]
for lab in GAP_LABELS:
    fns = [(g, p) for g, p in errs if g == lab]
    if not fns:
        continue
    xd = sum(1 for g, p in fns if p not in NONLEAF and domain(p) != domain(g))
    unk = sum(1 for g, p in fns if p in NONLEAF)
    wf = sum(1 for g, p in fns if p not in NONLEAF and family(p) == family(g))
    xf = len(fns) - xd - unk - wf
    dests = Counter(p for g, p in fns).most_common(4)
    print(f"\n  {lab.split('.')[-1]:16s} {len(fns):2d} misses | "
          f"cross-dom {xd}  unknown {unk}  cross-fam {xf}  within-fam {wf}")
    print(f"       → {dict((d.split('.')[-1] if d else 'unknown', n) for d,n in dests)}")
print("\n[done]")
