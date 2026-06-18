"""ac-01 first cut (stable): train a head on FROZEN gte-tiny embeddings of the assembled
clean training set; eval on the held-out gold/repr contested set. Sidesteps the MPS
end-to-end NaN; tests whether the assembled data (clean vocab positives incl region admin2
+ mined residuals) lifts accuracy with gte-tiny. Balanced (precedence-aware residual).
"""
import csv
import numpy as np
from collections import Counter
from sentence_transformers import SentenceTransformer
from sklearn.linear_model import LogisticRegression

def load(p, t, l): r = list(csv.DictReader(open(p), delimiter="\t")); return [x[t] for x in r], [x[l] for x in r]
Xtr_t, ytr = load("output/fine-tuned-encoder-discovery/encoder_train.tsv", "text", "label")
Xte_t, yte = load("output/fine-tuned-encoder-discovery/probe_data.tsv", "text", "true_family")
print("train", len(Xtr_t), Counter(ytr), "\ntest", len(Xte_t), Counter(yte))

m = SentenceTransformer("TaylorAI/gte-tiny", device="mps")
Xtr = m.encode(Xtr_t, show_progress_bar=False, batch_size=256)
Xte = m.encode(Xte_t, show_progress_bar=False, batch_size=256)
ytr, yte = np.array(ytr), np.array(yte)

for cw, name in [(None, "natural"), ("balanced", "balanced")]:
    clf = LogisticRegression(max_iter=3000, class_weight=cw).fit(Xtr, ytr)
    pred = clf.predict(Xte)
    acc = (pred == yte).mean()
    rp, rt = (pred == "RESIDUAL"), (yte == "RESIDUAL")
    rprec = (rp & rt).sum() / max(rp.sum(), 1); rrec = (rp & rt).sum() / max(rt.sum(), 1)
    per = {fm: round((pred[yte == fm] == fm).mean(), 2) for fm in ["country_code", "city", "region", "country", "full_name"] if (yte == fm).sum()}
    print(f"\n[gte-tiny frozen + head, {name}] gold-contested acc {acc:.3f}  "
          f"(shipped 0.684 | mined-MiniLM proof 0.82 | ceiling 0.893)")
    print(f"  RESIDUAL P {rprec:.3f} R {rrec:.3f} (pred {rp.sum()}/{len(yte)}, truth {rt.sum()})")
    print(f"  per-family recall: {per}")
