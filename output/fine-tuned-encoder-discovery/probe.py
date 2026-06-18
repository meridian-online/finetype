import csv, time, sys
import numpy as np
from sklearn.linear_model import LogisticRegression
from sklearn.model_selection import cross_val_score
from sklearn.preprocessing import StandardScaler
from sklearn.pipeline import make_pipeline

rows=list(csv.DictReader(open("output/fine-tuned-encoder-discovery/probe_data.tsv"),delimiter="\t"))
texts=[r["text"] for r in rows]
y=np.array([r["true_family"] for r in rows])
n=len(rows)
print(f"probe set: {n} columns, {len(set(y))} families")
import collections
dummy=max(collections.Counter(y).values())/n
print(f"majority-class baseline: {dummy:.3f}; model's own accuracy on this set: {sum(int(r['model_correct']) for r in rows)/n:.3f}\n")

def probe(name, vecs):
    vecs=np.asarray(vecs)
    clf=make_pipeline(StandardScaler(), LogisticRegression(max_iter=2000, C=1.0))
    acc=cross_val_score(clf, vecs, y, cv=5, scoring="accuracy")
    print(f"{name:32} dim={vecs.shape[1]:4} linear-probe 5fold acc = {acc.mean():.3f} ± {acc.std():.3f}")
    return acc.mean()

results={}
# --- static baseline: Model2Vec-class (potion) ---
from model2vec import StaticModel
sm=StaticModel.from_pretrained("minishlab/potion-base-8M")
t0=time.perf_counter(); V=sm.encode(texts); dt=(time.perf_counter()-t0)/n*1000
# per-column latency single-item
t1=time.perf_counter(); [sm.encode([texts[i]]) for i in range(50)]; lat_static=(time.perf_counter()-t1)/50*1000
print(f"[STATIC] potion-base-8M: batch {dt:.3f} ms/col | single {lat_static:.2f} ms/col")
results["static_potion"]=probe("STATIC potion-base-8M", V)

# --- contextual: all-MiniLM-L6-v2 ---
from sentence_transformers import SentenceTransformer
st=SentenceTransformer("sentence-transformers/all-MiniLM-L6-v2", device="cpu")
t0=time.perf_counter(); Vc=st.encode(texts, show_progress_bar=False); dt=(time.perf_counter()-t0)/n*1000
t1=time.perf_counter(); [st.encode([texts[i]], show_progress_bar=False) for i in range(50)]; lat_ctx=(time.perf_counter()-t1)/50*1000
print(f"[CONTEXTUAL] all-MiniLM-L6-v2: batch {dt:.3f} ms/col | single {lat_ctx:.2f} ms/col")
results["ctx_minilm"]=probe("CONTEXTUAL all-MiniLM-L6-v2", Vc)

print(f"\n=== LATENCY (single-col, CPU) ===")
print(f"  current engine (Model2Vec+multibranch+duckdb): ~0.9 ms/col")
print(f"  static potion-base-8M:   {lat_static:.2f} ms/col")
print(f"  contextual all-MiniLM:   {lat_ctx:.2f} ms/col   (ceiling for low-band-only: <=10ms)")
print(f"\n=== SEPARABILITY (ac-02 crux) ===")
print(f"  static  probe acc: {results['static_potion']:.3f}")
print(f"  context probe acc: {results['ctx_minilm']:.3f}")
print(f"  delta (context - static): {results['ctx_minilm']-results['static_potion']:+.3f}")
