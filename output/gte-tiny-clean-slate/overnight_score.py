"""Overnight: load a checkpoint, write gold+repr prediction TSVs (standalone + composed).
Arg: checkpoint path. Composed = gte-tiny on semantic families, v19 kept on structural."""
import csv, sys
import numpy as np, torch, torch.nn as nn, pyarrow.parquet as pq
sys.path.insert(0,"scripts"); from score_gold_anchor import _vendored_values, SEP
from transformers import AutoTokenizer, AutoModel
CK=sys.argv[1]; TAG=CK.split("/")[-1].replace(".pt","")
ck=torch.load(CK,map_location="cpu"); labels=ck["labels"]; H=768 if False else None
dev="mps" if torch.backends.mps.is_available() else "cpu"
tok=AutoTokenizer.from_pretrained("TaylorAI/gte-tiny")
enc=AutoModel.from_pretrained("TaylorAI/gte-tiny").to(dev); enc.load_state_dict(ck["enc"]); enc.eval()
hid=enc.config.hidden_size
if ck.get("head_type")=="mlp":
    head=nn.Sequential(nn.Linear(hid,hid),nn.ReLU(),nn.Dropout(0.1),nn.Linear(hid,len(labels))).to(dev)
else:
    head=nn.Linear(hid,len(labels)).to(dev)
head.load_state_dict(ck["head"]); head.eval()
def emb(t):
    e=tok(t,padding=True,truncation=True,max_length=64,return_tensors="pt").to(dev)
    o=enc(**e).last_hidden_state; m=e["attention_mask"].unsqueeze(-1).float(); return (o*m).sum(1)/m.sum(1).clamp(min=1)
def predict(texts):
    out=[]
    with torch.no_grad():
        for i in range(0,len(texts),256): out+=[labels[j] for j in head(emb(texts[i:i+256])).argmax(1).cpu().numpy()]
    return out
samp={}
t=pq.read_table("eval/gittables/corpus_pass/columns.parquet",columns=["file_content_sha256","column_name","sample_values_truncated"])
for s,c,v in zip(t.column(0).to_pylist(),t.column(1).to_pylist(),t.column(2).to_pylist()): samp[(s,c)]=v or ""
SEM=("geography.location","geography.address","identity.person","representation.text","representation.boolean")
def owns(l): return any(l.startswith(p) for p in SEM)
def run(gold_tsv,v19,tag):
    rows=list(csv.DictReader(open(gold_tsv),delimiter="\t"))
    v=dict(((r["file_content_sha256"],r["column_name"]),r["predicted_label"]) for r in csv.DictReader(open(v19),delimiter="\t"))
    keys,txts,base=[],[],[]
    for r in rows:
        k=(r["file_content_sha256"],r["column_name"]); b=v.get(k)
        if b is None: continue
        vals=[x for x in samp.get(k,"").split(SEP) if x.strip()]
        if not vals: vals=[x for x in _vendored_values(r.get("file_path",""),r["column_name"]) if x.strip()]
        keys.append(k);base.append(b);txts.append(f"header: {r['column_name']} | values: "+", ".join(vals[:8]))
    gp=predict(txts)
    for mode in ("standalone","composed"):
        o=f"/tmp/on_{TAG}_{tag}_{mode}.tsv"
        with open(o,"w",newline="") as f:
            w=csv.writer(f,delimiter="\t",lineterminator="\n"); w.writerow(["file_content_sha256","column_name","predicted_label","confidence"])
            for k,b,g in zip(keys,base,gp): w.writerow([k[0],k[1], g if mode=="standalone" else (g if owns(b) else b),""])
run("eval/gold/gold_corpus.tsv","output/ceiling-and-rules-discovery/predictions_v19.tsv","gold")
run("eval/repr/representative_corpus.tsv","output/representative-accuracy-gate/predictions_v19_repr.tsv","repr")
print(f"scored {TAG}",flush=True)
