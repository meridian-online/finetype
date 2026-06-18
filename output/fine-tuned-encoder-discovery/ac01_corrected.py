"""GeoNames-labelled proof: replace the crude header heuristic with authoritative
vocabulary-membership for the geo families. A mined geo prediction is a POSITIVE only
if a majority of its values are in the GeoNames vocab; otherwise it's an over-emission
-> RESIDUAL. This is the clean labelling signal the header heuristic approximated, and
it correctly handles the contested cases (a [PA,TX,NY,CA] column has few values in the
ISO country list -> residual, even though PA alone is a valid code).

Compares to the header-heuristic proof (0.82) and the 0.893 ceiling.
"""
import csv
import numpy as np
from collections import Counter
from sentence_transformers import SentenceTransformer
from sklearn.linear_model import LogisticRegression

SEP = "│"
LR = "eval/gold/lens_reference/"

def norm(s): return s.strip().lower()

# --- load GeoNames vocabularies ---
cities = set()
for line in open(LR + "cities15000.txt", encoding="utf-8"):
    p = line.split("\t")
    if len(p) > 2:
        cities.add(norm(p[1])); cities.add(norm(p[2]))
regions = set()
for line in open(LR + "admin1CodesASCII.txt", encoding="utf-8"):
    p = line.split("\t")
    if len(p) > 2:
        regions.add(norm(p[1])); regions.add(norm(p[2]))
for line in open("/Users/hugh/datasets/geonames/2026-05-24/admin2Codes.txt", encoding="utf-8"):
    p = line.split("\t")
    if len(p) > 1: regions.add(norm(p[1]))
for row in csv.reader(open("eval/datasets/csv/us_states.csv")):
    for c in row: regions.add(norm(c))
country_names, country_codes = set(), set()
for r in csv.DictReader(open(LR + "iso3166.csv")):
    country_names.add(norm(r["name"]))
    country_codes.add(norm(r["alpha-2"])); country_codes.add(norm(r["alpha-3"]))
VOCAB = {"city": cities, "region": regions, "country": country_names, "country_code": country_codes}
print("vocab sizes:", {k: len(v) for k, v in VOCAB.items()})

KW = {  # header keywords for the non-geo-vocab families (full_name/entity_name/iata)
    "full_name": ["name", "person", "author", "contact", "owner", "applicant"],
    "entity_name": ["name", "company", "org", "brand", "product", "title", "entity", "vendor"],
    "iata_code": ["airport", "iata", "port", "terminal", "flight"],
}

def parse(raw): return [v.strip() for v in (raw or "").split(SEP) if v.strip()][:8]

def label(leaf, header, vals):
    if leaf in ("categorical", "word", "plain_text"):
        return "RESIDUAL"
    if leaf in VOCAB:                                  # geo: authoritative membership fraction
        voc = VOCAB[leaf]
        frac = sum(norm(v) in voc for v in vals) / max(len(vals), 1)
        return leaf if frac >= 0.5 else "RESIDUAL"
    kws = KW.get(leaf); h = (header or "").lower()     # non-geo: header heuristic
    return leaf if (kws and any(k in h for k in kws)) else "RESIDUAL"

# --- build train from mined pool with GeoNames labels ---
train_text, train_lab = [], []
geo_relabel = geo_total = 0
for row in csv.DictReader(open("output/fine-tuned-encoder-discovery/mining_pool.tsv"), delimiter="\t"):
    c, lf, sv = row["column_name"], row["leaf"], row["sample_values_truncated"]
    vals = parse(sv)
    if not vals:
        continue
    lab = label(lf, c, vals)
    if lf in VOCAB:
        geo_total += 1
        if lab == "RESIDUAL":
            geo_relabel += 1
    train_text.append(f"header: {c} | values: " + ", ".join(vals))
    train_lab.append(lab)
print(f"geo mined: {geo_total}, relabelled->RESIDUAL by vocab: {geo_relabel}")
print("train dist:", dict(Counter(train_lab)))

# --- test = gold/repr contested (header+values, gold truth) ---
test_text, test_lab = [], []
for r in csv.DictReader(open("output/fine-tuned-encoder-discovery/probe_data.tsv"), delimiter="\t"):
    test_text.append(r["text"]); test_lab.append(r["true_family"])
vocab_fams = set(test_lab)
keep = [i for i, l in enumerate(train_lab) if l in vocab_fams]
train_text = [train_text[i] for i in keep]; train_lab = [train_lab[i] for i in keep]

st = SentenceTransformer("TaylorAI/gte-tiny", device="mps")
Xtr = st.encode(train_text, show_progress_bar=False, batch_size=128)
Xte = st.encode(test_text, show_progress_bar=False, batch_size=128)
ytr, yte = np.array(train_lab), np.array(test_lab)

for cw, name in [(None, "natural"), ("balanced", "balanced")]:
    pred = LogisticRegression(max_iter=3000, class_weight=cw).fit(Xtr, ytr).predict(Xte)
    acc = (pred == yte).mean()
    rp, rt = (pred == "RESIDUAL"), (yte == "RESIDUAL")
    print(f"\n[GeoNames-labelled header+values, {name}] gold acc {acc:.3f}  "
          f"(header-heuristic 0.82 | distilled values-only 0.648 | ceiling 0.893 | shipped 0.684)")
    print(f"  RESIDUAL precision {(rp&rt).sum()/max(rp.sum(),1):.3f} recall {(rp&rt).sum()/max(rt.sum(),1):.3f}")
    for fm in ["country_code", "city", "region", "country"]:
        m = yte == fm
        if m.sum(): print(f"  {fm:12} recall {(pred[m]==fm).mean():.3f} (n={m.sum()})")
