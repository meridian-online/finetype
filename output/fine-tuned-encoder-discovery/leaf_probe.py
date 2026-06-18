import csv, time
import numpy as np
from sklearn.linear_model import LogisticRegression
from sklearn.model_selection import cross_val_score
from sentence_transformers import SentenceTransformer

rows = list(csv.DictReader(open("output/fine-tuned-encoder-discovery/probe_data.tsv"), delimiter="\t"))
texts = [r["text"] for r in rows]; y = np.array([r["true_family"] for r in rows])
def cv(X): return cross_val_score(LogisticRegression(max_iter=2000), np.asarray(X), y, cv=5).mean()
def lat(m, n=40):
    t=time.perf_counter()
    for i in range(n): m.encode([texts[i%len(texts)]], show_progress_bar=False)
    return (time.perf_counter()-t)/n*1000

for mid,label in [("MongoDB/mdbr-leaf-ir","mdbr-leaf-ir"),
                  ("TaylorAI/gte-tiny","gte-tiny (current lead)"),
                  ("sentence-transformers/all-MiniLM-L6-v2","MiniLM-L6")]:
    try:
        m=SentenceTransformer(mid, device="cpu", trust_remote_code=True)
        # try a 'document' prompt if the model defines prompts (IR models do); else plain
        try:
            X=m.encode(texts, prompt_name="document", show_progress_bar=False)
            mode="doc-prompt"
        except Exception:
            X=m.encode(texts, show_progress_bar=False); mode="plain"
        dim=np.asarray(X).shape[1]
        print(f"{label:26} dim={dim:4} ({mode:9}) separability={cv(X):.3f}  lat={lat(m):.2f}ms")
    except Exception as e:
        print(f"{label:26} FAILED: {str(e)[:120]}")
print("(244 contested, header+values, 5-fold CV no-scaler ranking; gte-tiny was 0.872, MiniLM 0.807)")
