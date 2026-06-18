"""ac-01/ac-02 PREVIEW (bounded): does a MiniLM head trained on the REAL distilled
data generalise to the gold/repr contested columns, and does it reproduce or escape
the residual-attractor (the 0-for-6 killer)?

NOT the production build: values-only (distilled data is header-less), family-level
(not 250-class), a linear head on frozen MiniLM (not an encoder fine-tune), no corpus
gate. It isolates ONE question — training dynamics on the contested families:
  - natural-frequency training (residual-heavy) -> does it collapse to RESIDUAL? (attractor)
  - class-balanced training (the ac-03 precedence-aware proxy) -> does precision hold?
"""
import csv, gzip, json
import numpy as np
from collections import Counter
from sentence_transformers import SentenceTransformer
from sklearn.linear_model import LogisticRegression
from sklearn.model_selection import cross_val_score

RESID = {"representation.discrete.categorical", "representation.text.word", "representation.text.plain_text"}
def fam(x): return "RESIDUAL" if x in RESID else x.split(".")[-1]

# --- test set: the gold/repr contested cols, VALUES-ONLY (strip the header prefix) ---
test_text, test_fam = [], []
for r in csv.DictReader(open("output/fine-tuned-encoder-discovery/probe_data.tsv"), delimiter="\t"):
    t = r["text"]
    vals = t.split(" | values: ", 1)[1] if " | values: " in t else t
    test_text.append("values: " + vals)
    test_fam.append(r["true_family"])
TESTVOCAB = set(test_fam)
print("test:", len(test_text), "cols; families:", dict(Counter(test_fam)))

# --- train set: distilled data, values-only, families in TESTVOCAB, capped per family ---
CAP = 2500
by_fam = {}
with gzip.open("output/distillation-v3/sherlock_distilled.csv.gz", "rt") as f:
    for row in csv.DictReader(f):
        lab = row.get("final_label") or ""; sv = row.get("sample_values") or ""
        if not lab or not sv: continue
        fm = fam(lab)
        if fm not in TESTVOCAB: continue
        if len(by_fam.get(fm, [])) >= CAP: continue
        try:
            vals = json.loads(sv)
        except Exception:
            continue
        vals = [str(v) for v in vals if str(v).strip() and str(v) != "None"][:8]
        if not vals: continue
        by_fam.setdefault(fm, []).append("values: " + ", ".join(vals))
train_text, train_fam = [], []
for fm, texts in by_fam.items():
    train_text += texts; train_fam += [fm] * len(texts)
print("train:", len(train_text), "rows; families:", dict(Counter(train_fam)))

# --- embed (MiniLM on M1 GPU) ---
st = SentenceTransformer("sentence-transformers/all-MiniLM-L6-v2", device="mps")
Xtr = st.encode(train_text, show_progress_bar=False, batch_size=128)
Xte = st.encode(test_text, show_progress_bar=False, batch_size=128)
ytr = np.array(train_fam); yte = np.array(test_fam)

def report(name, pred):
    acc = (pred == yte).mean()
    # RESIDUAL precision = of cols predicted RESIDUAL, how many truly are (attractor = low precision + over-prediction)
    pr_pred = (pred == "RESIDUAL").sum()
    pr_true = (yte == "RESIDUAL").sum()
    resid_prec = ((pred == "RESIDUAL") & (yte == "RESIDUAL")).sum() / max(pr_pred, 1)
    resid_rec = ((pred == "RESIDUAL") & (yte == "RESIDUAL")).sum() / max(pr_true, 1)
    print(f"\n[{name}] overall acc {acc:.3f}")
    print(f"  RESIDUAL: predicted {pr_pred}/{len(yte)} (truth {pr_true}); precision {resid_prec:.3f} recall {resid_rec:.3f}"
          f"  {'<-- ATTRACTOR (over-predicts residual)' if pr_pred > pr_true*1.3 else ''}")
    # semantic prize: non-residual families recovered
    for fm in ["country_code", "city", "region", "country"]:
        m = yte == fm
        if m.sum() == 0: continue
        rec = (pred[m] == fm).mean()
        print(f"  {fm:12} recall {rec:.3f} (n={m.sum()})")

# baseline: values-only zero-shot probe (5fold CV on test) — the separability ceiling, values-only
probe = LogisticRegression(max_iter=2000)
cv = cross_val_score(probe, Xte, yte, cv=5).mean()
print(f"\n[values-only zero-shot probe, 5fold CV on test] acc {cv:.3f}  (header+values was 0.893; shipped model 0.684)")

# the two training regimes
for cw, name in [(None, "head trained on distilled — NATURAL frequency"),
                 ("balanced", "head trained on distilled — CLASS-BALANCED (ac-03 proxy)")]:
    clf = LogisticRegression(max_iter=3000, class_weight=cw)
    clf.fit(Xtr, ytr)
    report(name, clf.predict(Xte))
