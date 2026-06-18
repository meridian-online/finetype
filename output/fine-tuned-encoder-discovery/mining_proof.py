"""Cheap proof (heuristic-labeled): does corpus-mined HEADER+VALUES training data —
with a header-domain-mismatch heuristic relabelling over-emitted specific predictions
as residual — lift the gold contested accuracy toward the 0.893 ceiling?

If even noisy heuristic labels + headers convert (beat the 0.648 distilled-values-only
baseline, approach 0.893), the LLM-labelling spend is justified. If not, reconsider.
NOISE CAVEAT: the header keyword heuristic is crude; this estimates the lever, it is not
the production label quality (that's the LLM-panel step).
"""
import csv
import numpy as np
from collections import Counter
from sentence_transformers import SentenceTransformer
from sklearn.linear_model import LogisticRegression

SEP = "│"
RESID = {"representation.discrete.categorical", "representation.text.word", "representation.text.plain_text"}
# header keywords that SUPPORT a specific-type prediction; absence -> treat as over-emission (residual)
KW = {
    "country_code": ["countr", "nation", "iso", "ctry", "cntry"],
    "country": ["countr", "nation"],
    "city": ["city", "town", "munic", "locality", "place"],
    "region": ["region", "state", "provinc", "district", "area", "zone", "county"],
    "full_name": ["name", "person", "author", "contact", "owner", "applicant"],
    "entity_name": ["name", "company", "org", "brand", "product", "title", "entity", "vendor", "supplier"],
    "iata_code": ["airport", "iata", "port", "terminal", "flight"],
}

def parse_vals(raw):
    return [v for v in (raw or "").split(SEP) if v.strip()][:8]

def heuristic_label(leaf, header):
    if leaf in ("categorical", "word", "plain_text"):
        return "RESIDUAL"
    h = (header or "").lower()
    kws = KW.get(leaf)
    if kws and any(k in h for k in kws):
        return leaf            # header supports the type -> keep as positive
    return "RESIDUAL"          # specific prediction with no header support -> over-emission heuristic

# --- build train from mined corpus pool (header+values) ---
rows = list(csv.DictReader(open("output/fine-tuned-encoder-discovery/mining_pool.tsv"), delimiter="\t"))
train_text, train_lab = [], []
relabelled = 0
for row in rows:
    c, lf, sv = row["column_name"], row["leaf"], row["sample_values_truncated"]
    vals = parse_vals(sv)
    if not vals:
        continue
    lab = heuristic_label(lf, c)
    if lf not in ("categorical", "word", "plain_text") and lab == "RESIDUAL":
        relabelled += 1
    train_text.append(f"header: {c} | values: " + ", ".join(vals))
    train_lab.append(lab)
print(f"train rows {len(train_text)}; heuristic relabelled {relabelled} specific->RESIDUAL")
print("train label dist:", dict(Counter(train_lab)))

# --- test = gold/repr contested, header+values, gold truth ---
test_text, test_lab = [], []
for r in csv.DictReader(open("output/fine-tuned-encoder-discovery/probe_data.tsv"), delimiter="\t"):
    test_text.append(r["text"])           # already "header: X | values: ..."
    test_lab.append(r["true_family"])
# restrict train label space to families present in test (+ RESIDUAL)
vocab = set(test_lab)
keep = [i for i, l in enumerate(train_lab) if l in vocab]
train_text = [train_text[i] for i in keep]; train_lab = [train_lab[i] for i in keep]
print("test rows", len(test_text), "| test families", dict(Counter(test_lab)))

st = SentenceTransformer("sentence-transformers/all-MiniLM-L6-v2", device="mps")
Xtr = st.encode(train_text, show_progress_bar=False, batch_size=128)
Xte = st.encode(test_text, show_progress_bar=False, batch_size=128)
ytr, yte = np.array(train_lab), np.array(test_lab)

for cw, name in [(None, "natural"), ("balanced", "balanced")]:
    clf = LogisticRegression(max_iter=3000, class_weight=cw).fit(Xtr, ytr)
    pred = clf.predict(Xte)
    acc = (pred == yte).mean()
    rp = (pred == "RESIDUAL"); rt = (yte == "RESIDUAL")
    rprec = (rp & rt).sum() / max(rp.sum(), 1); rrec = (rp & rt).sum() / max(rt.sum(), 1)
    print(f"\n[corpus-mined header+values, {name}] gold acc {acc:.3f}  "
          f"(distilled values-only 0.648 | zero-shot header+values 0.893 | shipped 0.684)")
    print(f"  RESIDUAL precision {rprec:.3f} recall {rrec:.3f} (pred {rp.sum()}/{len(yte)}, truth {rt.sum()})")
    for fm in ["country_code", "city", "region", "country"]:
        m = yte == fm
        if m.sum(): print(f"  {fm:12} recall {(pred[m]==fm).mean():.3f} (n={m.sum()})")
