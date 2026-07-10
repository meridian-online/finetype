#!/usr/bin/env python3
"""ac-02 prep: where does m2v-244 lose composed vs v19, and is the loss embed-addressable?

Joins gold + v19 composed + m2v-244-s44 composed + both Sense preds. For each gold
column classifies the m2v-244 outcome and, where it loses to v19, whether the loss is:
  - SENSE loss  -> the embed got it wrong -> a richer embed COULD fix it (ac-02 target)
  - RULE loss   -> Sense was right, Sharpen mangled it -> embed won't help (ac-04 territory)

Output: a per-type table of m2v-244's composed losses, split embed-addressable vs rule-owned.
"""
import csv
from collections import defaultdict

GOLD = "eval/gold/gold_corpus.tsv"
P = "output/embed-frontier/preds"
V19_C = f"{P}/v19-relu-s42 (reference)_composed.tsv"
V19_S = f"{P}/v19-relu-s42 (reference)_sense.tsv"
M2V_C = f"{P}/repro-s44_composed.tsv"
M2V_S = f"{P}/repro-s44_sense.tsv"


def load_preds(path):
    d = {}
    with open(path) as f:
        for r in csv.DictReader(f, delimiter="\t"):
            d[(r["file_content_sha256"], r["column_name"])] = r["predicted_label"]
    return d


def load_gold():
    d = {}
    with open(GOLD) as f:
        for r in csv.DictReader(f, delimiter="\t"):
            d[(r["file_content_sha256"], r["column_name"])] = r["curated_label"]
    return d


gold = load_gold()
v19c, v19s = load_preds(V19_C), load_preds(V19_S)
m2vc, m2vs = load_preds(M2V_C), load_preds(M2V_S)

keys = [k for k in gold if k in v19c and k in m2vc]
print(f"scored {len(keys)} gold columns present in all pred sets\n")

# Headline correctness
v19_ok = sum(1 for k in keys if v19c[k] == gold[k])
m2v_ok = sum(1 for k in keys if m2vc[k] == gold[k])
print(f"composed correct: v19 {v19_ok}/{len(keys)} ({v19_ok/len(keys):.3f})  "
      f"m2v-244 {m2v_ok}/{len(keys)} ({m2v_ok/len(keys):.3f})\n")

# Residual-attractor gold types: decision 0096 says these CANNOT be trained as a
# flat-softmax winner — when gold IS one of these and the model picked a "tighter"
# type, a richer embed will NOT recover it (it's rule-shaped: needs a value-based
# veto, e.g. the geohash/alnum veto). Only NON-residual Sense losses are genuinely
# embed-addressable.
RESIDUAL = {
    "representation.identifier.alphanumeric_id",
    "representation.discrete.categorical",
    "representation.numeric.decimal_number",
    "representation.numeric.integer_number",
    "representation.identifier.numeric_code",
    "representation.text.plain_text",
    "representation.text.word",
}

# m2v-244 composed LOSSES = v19 right, m2v wrong.
loss_embed = defaultdict(list)   # Sense wrong, gold NOT residual -> embed CAN target
loss_residual = defaultdict(list)  # Sense wrong, gold IS residual -> 0096 rule-shaped
loss_rule = defaultdict(list)    # Sense right, composed wrong -> Sharpen mangled it
win = 0
for k in keys:
    g, vc, mc = gold[k], v19c[k], m2vc[k]
    if mc == g and vc != g:
        win += 1
    if vc == g and mc != g:                       # a composed loss
        ms = m2vs.get(k, "")
        if ms == g:
            loss_rule[g].append((k[1], mc))
        elif g in RESIDUAL:
            loss_residual[g].append((k[1], ms, mc))
        else:
            loss_embed[g].append((k[1], ms, mc))

n_embed = sum(len(v) for v in loss_embed.values())
n_resid = sum(len(v) for v in loss_residual.values())
n_rule = sum(len(v) for v in loss_rule.values())
total = n_embed + n_resid + n_rule
print(f"m2v-244 composed losses vs v19: {total} total  (m2v also WINS {win} where v19 wrong)")
print(f"  EMBED-ADDRESSABLE (Sense wrong, non-residual): {n_embed}  <- the real ac-02 target")
print(f"  residual-attractor (0096, rule-shaped):        {n_resid}  <- needs value rule, not embed")
print(f"  rule-owned (Sense was right, Sharpen broke it): {n_rule}  <- ac-04 territory")
print(f"  => realistic composed ceiling recovery from a better embed: "
      f"~{n_embed/len(keys)*100:.1f}pp of the {(total)/len(keys)*100:.1f}pp gap\n")

print("== EMBED-ADDRESSABLE losses (a richer embed could genuinely recover) ==")
for t, rows in sorted(loss_embed.items(), key=lambda x: -len(x[1])):
    print(f"  {t}: {len(rows)}")
    for col, ms, mc in rows[:4]:
        print(f"      {col!r}: sense->{ms.split('.')[-1]} composed->{mc.split('.')[-1]}")

print("\n== RESIDUAL-ATTRACTOR losses (0096: rule-shaped, embed won't reliably fix) ==")
for t, rows in sorted(loss_residual.items(), key=lambda x: -len(x[1])):
    print(f"  {t}: {len(rows)}")
    for col, ms, mc in rows[:4]:
        print(f"      {col!r}: sense->{ms.split('.')[-1]} composed->{mc.split('.')[-1]}")

print("\n== RULE-OWNED losses (Sense right, Sharpen changed it) ==")
for t, rows in sorted(loss_rule.items(), key=lambda x: -len(x[1])):
    print(f"  {t}: {len(rows)}  {[c for c,_ in rows][:4]}")
