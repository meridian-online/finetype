import csv, time, collections
import numpy as np
import torch
from sklearn.linear_model import LogisticRegression
from sklearn.model_selection import cross_val_score
from sklearn.preprocessing import StandardScaler
from sklearn.pipeline import make_pipeline

rows=list(csv.DictReader(open("output/fine-tuned-encoder-discovery/probe_data.tsv"),delimiter="\t"))
texts=[r["text"] for r in rows]; y=np.array([r["true_family"] for r in rows]); n=len(rows)
dummy=max(collections.Counter(y).values())/n
model_acc=sum(int(r["model_correct"]) for r in rows)/n
print(f"probe set {n} cols; majority {dummy:.3f}; shipped-model acc {model_acc:.3f}\n")
res={}
def probe(name, vecs, lat):
    clf=make_pipeline(StandardScaler(), LogisticRegression(max_iter=2000))
    acc=cross_val_score(clf, np.asarray(vecs), y, cv=5).mean()
    res[name]=(acc,lat,np.asarray(vecs).shape[1])
    print(f"{name:30} dim={np.asarray(vecs).shape[1]:4} acc={acc:.3f}  lat={lat:.2f} ms/col")

# static + MiniLM (cheap, re-run for one table)
from model2vec import StaticModel
sm=StaticModel.from_pretrained("minishlab/potion-base-8M")
t=time.perf_counter(); V=sm.encode(texts); 
t0=time.perf_counter();[sm.encode([texts[i]]) for i in range(30)];ls=(time.perf_counter()-t0)/30*1000
probe("static potion-base-8M", V, ls)
from sentence_transformers import SentenceTransformer
for mname,label in [("sentence-transformers/all-MiniLM-L6-v2","enc all-MiniLM-L6-v2"),
                    ("BAAI/bge-small-en-v1.5","enc bge-small-en-v1.5")]:
    st=SentenceTransformer(mname, device="cpu")
    V=st.encode(texts, show_progress_bar=False)
    t0=time.perf_counter();[st.encode([texts[i]],show_progress_bar=False) for i in range(30)];l=(time.perf_counter()-t0)/30*1000
    probe(label, V, l)

# decoder LLM: mean-pooled hidden states
from transformers import AutoTokenizer, AutoModel
def dec_embed(mn, texts, bs=8):
    tok=AutoTokenizer.from_pretrained(mn)
    if tok.pad_token is None: tok.pad_token=tok.eos_token
    mdl=AutoModel.from_pretrained(mn, torch_dtype=torch.float32); mdl.eval()
    out=[]
    with torch.no_grad():
        for i in range(0,len(texts),bs):
            enc=tok(texts[i:i+bs],padding=True,truncation=True,max_length=128,return_tensors="pt")
            h=mdl(**enc).last_hidden_state; m=enc["attention_mask"].unsqueeze(-1).float()
            out.append(((h*m).sum(1)/m.sum(1).clamp(min=1)).numpy())
    return np.vstack(out), (tok,mdl)
for mn,label in [("Qwen/Qwen2.5-0.5B","dec Qwen2.5-0.5B")]:
    V,(tok,mdl)=dec_embed(mn,texts)
    with torch.no_grad():
        t0=time.perf_counter()
        for i in range(20):
            enc=tok([texts[i]],truncation=True,max_length=128,return_tensors="pt"); mdl(**enc)
        l=(time.perf_counter()-t0)/20*1000
    probe(label, V, l)

print("\n=== ac-02 SEPARABILITY + ac-01 LATENCY (sorted by acc) ===")
print(f"{'candidate':30} {'acc':>6} {'lat ms/col':>11}  vs current 0.9ms / ≤10ms low-band ceiling")
for k,(a,l,d) in sorted(res.items(),key=lambda x:-x[1][0]):
    flag = "" if l<=10 else "  ⚠ over 10ms"
    print(f"{k:30} {a:>6.3f} {l:>11.2f}{flag}")
print(f"\nreference: shipped model {model_acc:.3f}, majority {dummy:.3f}")
