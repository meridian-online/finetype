"""Clean-slate decider: can a LINEAR probe on raw gte-tiny embeddings reproduce v19's
decisions across the FULL label space — especially structural types? Linear probe = weakest
head, so this is a conservative lower bound on gte-tiny-as-backbone."""
import numpy as np, pyarrow.parquet as pq
from sentence_transformers import SentenceTransformer
from sklearn.linear_model import LogisticRegression
from sklearn.model_selection import train_test_split
from collections import defaultdict
SEP="│"
t=pq.read_table("/tmp/probe_sample.parquet")
cn=t.column("column_name").to_pylist(); y=t.column("sense_prediction").to_pylist(); sv=t.column("sample_values_truncated").to_pylist()
texts=[f"header: {cn[i]} | values: "+", ".join([v for v in (sv[i] or '').split(SEP) if v.strip()][:8]) for i in range(len(cn))]
print(f"embedding {len(texts)} cols with raw gte-tiny...")
m=SentenceTransformer("TaylorAI/gte-tiny", device="mps")
X=m.encode(texts, batch_size=256, show_progress_bar=False)
y=np.array(y)
Xtr,Xte,ytr,yte=train_test_split(X,y,test_size=0.3,random_state=42,stratify=y)
print("fitting linear probe over full label space...")
clf=LogisticRegression(max_iter=2000,C=10).fit(Xtr,ytr)
pred=clf.predict(Xte)
print(f"\n=== OVERALL linear-probe accuracy (reproduce v19): {(pred==yte).mean():.3f} on {len(yte)} held-out cols, {len(set(y))} labels ===")

def domain(l): return l.split(".")[0]
def is_structural(l):
    d=l.split("."); dom=d[0]; cat=d[1] if len(d)>1 else ""
    if dom in ("datetime","finance","container"): return True
    if dom=="geography" and cat in ("coordinate",): return True
    if dom=="geography" and cat in ("location","address"): return False
    if dom=="identity" and cat=="person": return False
    if dom=="identity": return True  # gov/medical/commerce codes
    if dom=="technology": return True
    if dom=="representation" and cat in ("text","discrete","boolean"): return False
    if dom=="representation": return True  # numeric, identifier, format, file, scientific
    return True

# per-domain + structural/semantic macro accuracy
per=defaultdict(lambda:[0,0])
struct=[0,0]; sem=[0,0]; bytype=defaultdict(lambda:[0,0])
for p,t_ in zip(pred,yte):
    correct=int(p==t_); per[domain(t_)][0]+=correct; per[domain(t_)][1]+=1
    bytype[t_][0]+=correct; bytype[t_][1]+=1
    (struct if is_structural(t_) else sem)[0]+=correct; (struct if is_structural(t_) else sem)[1]+=1
print("\n=== by domain ===")
for d,(c,n) in sorted(per.items(),key=lambda x:-x[1][1]): print(f"  {d:16} {c/n:.3f}  (n={n})")
print(f"\n=== STRUCTURAL types: {struct[0]/struct[1]:.3f} (n={struct[1]})   SEMANTIC types: {sem[0]/sem[1]:.3f} (n={sem[1]}) ===")
print("\n=== worst-probed STRUCTURAL types (where gte-tiny can't reproduce v19) ===")
ws=sorted([(c/n,t_,n) for t_,(c,n) in bytype.items() if is_structural(t_) and n>=10],key=lambda x:x[0])[:12]
for acc,t_,n in ws: print(f"  {acc:.2f}  {t_}  (n={n})")
print("\n=== worst-probed SEMANTIC types ===")
ws=sorted([(c/n,t_,n) for t_,(c,n) in bytype.items() if not is_structural(t_) and n>=10],key=lambda x:x[0])[:8]
for acc,t_,n in ws: print(f"  {acc:.2f}  {t_}  (n={n})")
print("DONE")
