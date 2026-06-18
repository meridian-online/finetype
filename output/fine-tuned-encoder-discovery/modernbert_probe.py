import csv, time
import numpy as np
from sklearn.linear_model import LogisticRegression
from sklearn.model_selection import cross_val_score
from sentence_transformers import SentenceTransformer
rows=list(csv.DictReader(open("output/fine-tuned-encoder-discovery/probe_data.tsv"),delimiter="\t"))
texts=[r["text"] for r in rows]; y=np.array([r["true_family"] for r in rows])
def cv(X): return cross_val_score(LogisticRegression(max_iter=2000),np.asarray(X),y,cv=5).mean()
def lat(m,n=30):
    t=time.perf_counter()
    for i in range(n): m.encode([texts[i%len(texts)]],show_progress_bar=False)
    return (time.perf_counter()-t)/n*1000
for mid,label in [("nomic-ai/modernbert-embed-base","ModernBERT-embed-base (149M)"),
                  ("TaylorAI/gte-tiny","gte-tiny (lead, 23M)")]:
    try:
        m=SentenceTransformer(mid, device="cpu", trust_remote_code=True)
        X=m.encode(texts, show_progress_bar=False)
        print(f"{label:32} dim={np.asarray(X).shape[1]:4} separability={cv(X):.3f}  cpu_lat={lat(m):.2f}ms")
    except Exception as e:
        print(f"{label:32} FAILED: {str(e)[:140]}")
print("(244 contested, header+values, no-scaler CV ranking; gte-tiny 0.872, MiniLM 0.807)")
