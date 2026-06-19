"""Step 1 of the slim plan: fine-tune gte-tiny (v3 recipe, saved) on clean v2 data, then
produce TYPE-ROUTED two-stage predictions for gold + representative (v19 everywhere; gte-tiny
re-decides only where v19 predicts a contested-semantic type). Writes routed prediction TSVs
for scoring with score_gold_anchor.py (the human-truth gates).
"""
import csv, sys, time
import numpy as np
import torch, torch.nn as nn
import pyarrow.parquet as pq
from transformers import AutoTokenizer, AutoModel, get_linear_schedule_with_warmup
sys.path.insert(0, "scripts")
from score_gold_anchor import _vendored_values, SEP

MODEL = "TaylorAI/gte-tiny"
CONTESTED = {"country","country_code","city","region","full_name","entity_name","iata_code","categorical","word","plain_text"}
FAM2LABEL = {"RESIDUAL":"representation.text.word","country":"geography.location.country",
             "country_code":"geography.location.country_code","city":"geography.location.city",
             "region":"geography.location.region","full_name":"identity.person.full_name",
             "entity_name":"representation.text.entity_name","iata_code":"geography.transportation.iata_code"}
def leaf(x): return (x or "").split(".")[-1]

# --- train gte-tiny v3 on clean v2 data ---
def load(p,t,l): r=list(csv.DictReader(open(p),delimiter="\t")); return [x[t] for x in r],[x[l] for x in r]
Xtr_t,ytr_l = load("output/fine-tuned-encoder-discovery/encoder_train_v2.tsv","text","label")
labels=sorted(set(ytr_l)); l2i={l:i for i,l in enumerate(labels)}
ytr=torch.tensor([l2i[l] for l in ytr_l])
dev="mps" if torch.backends.mps.is_available() else "cpu"
tok=AutoTokenizer.from_pretrained(MODEL); enc=AutoModel.from_pretrained(MODEL).to(dev); head=nn.Linear(enc.config.hidden_size,len(labels)).to(dev)
for n,p in enc.named_parameters(): p.requires_grad = ("encoder.layer.4." in n or "encoder.layer.5." in n)
tr=[p for p in enc.parameters() if p.requires_grad]
opt=torch.optim.AdamW([{"params":tr,"lr":2e-6},{"params":head.parameters(),"lr":1e-3}],eps=1e-6,weight_decay=0.01)
lossf=nn.CrossEntropyLoss(); bs=32; ep=3; steps=ep*(len(Xtr_t)//bs); sched=get_linear_schedule_with_warmup(opt,int(0.1*steps),steps)
def emb(texts):
    e=tok(texts,padding=True,truncation=True,max_length=64,return_tensors="pt").to(dev)
    o=enc(**e).last_hidden_state; m=e["attention_mask"].unsqueeze(-1).float(); return (o*m).sum(1)/m.sum(1).clamp(min=1)
idx=np.arange(len(Xtr_t))
for e in range(ep):
    enc.train();head.train();np.random.shuffle(idx);t0=time.time()
    for b in range(0,len(idx),bs):
        bi=idx[b:b+bs]; loss=lossf(head(emb([Xtr_t[i] for i in bi])),ytr[bi].to(dev))
        opt.zero_grad();loss.backward();torch.nn.utils.clip_grad_norm_(tr+list(head.parameters()),1.0);opt.step();sched.step()
    print(f"  epoch {e+1} done ({time.time()-t0:.0f}s)")
torch.save({"enc":enc.state_dict(),"head":head.state_dict(),"labels":labels},"output/fine-tuned-encoder-discovery/gte_tiny_v2.pt")
enc.eval();head.eval()
def predict_fam(texts):
    out=[]
    with torch.no_grad():
        for i in range(0,len(texts),256): out.append(head(emb(texts[i:i+256])).argmax(1).cpu().numpy())
    return [labels[j] for j in np.concatenate(out)] if texts else []

# --- route gold + representative ---
def route(gold_tsv, v19_preds, out_tsv):
    gold=list(csv.DictReader(open(gold_tsv),delimiter="\t"))
    v19={(r["file_content_sha256"],r["column_name"]):r["predicted_label"] for r in csv.DictReader(open(v19_preds),delimiter="\t")}
    samp={}
    t=pq.read_table("eval/gittables/corpus_pass/columns.parquet",columns=["file_content_sha256","column_name","sample_values_truncated"])
    for s,c,v in zip(t.column(0).to_pylist(),t.column(1).to_pylist(),t.column(2).to_pylist()):
        samp[(s,c)]=v or ""
    rows=[]; to_route=[]; rk=[]
    for r in gold:
        k=(r["file_content_sha256"],r["column_name"]); base=v19.get(k)
        if base is None: continue
        if leaf(base) in CONTESTED:
            vals=[x for x in samp.get(k,"").split(SEP) if x.strip()]
            if not vals: vals=[x for x in _vendored_values(r.get("file_path",""),r["column_name"]) if x.strip()]
            if vals:
                to_route.append(f"header: {r['column_name']} | values: "+", ".join(vals[:8])); rk.append((k,base)); continue
        rows.append((k,base))
    fams=predict_fam(to_route)
    for (k,base),fam in zip(rk,fams): rows.append((k,FAM2LABEL[fam]))
    with open(out_tsv,"w",newline="") as f:
        w=csv.writer(f,delimiter="\t",lineterminator="\n"); w.writerow(["file_content_sha256","column_name","predicted_label","confidence"])
        for (s,c),lab in rows: w.writerow([s,c,lab,""])
    print(f"{out_tsv}: {len(rows)} preds, {len(to_route)} routed through gte-tiny")

route("eval/gold/gold_corpus.tsv","output/ceiling-and-rules-discovery/predictions_v19.tsv","output/fine-tuned-encoder-discovery/routed_gold.tsv")
route("eval/repr/representative_corpus.tsv","output/representative-accuracy-gate/predictions_v19_repr.tsv","output/fine-tuned-encoder-discovery/routed_repr.tsv")
print("done")
