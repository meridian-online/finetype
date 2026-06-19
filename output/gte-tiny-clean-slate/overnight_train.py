"""Overnight: train full-label gte-tiny on CLEAN labels. Args: --data --head(linear|mlp)
--epochs --snapshots(csv) --prefix. Saves a checkpoint at each snapshot epoch."""
import argparse, csv, time
import numpy as np, torch, torch.nn as nn
from transformers import AutoTokenizer, AutoModel, get_linear_schedule_with_warmup
ap=argparse.ArgumentParser()
ap.add_argument("--data",required=True); ap.add_argument("--head",default="linear")
ap.add_argument("--epochs",type=int,default=16); ap.add_argument("--snapshots",default="8,12,16")
ap.add_argument("--prefix",required=True)
a=ap.parse_args(); SNAP={int(x) for x in a.snapshots.split(",")}; BS=64; MODEL="TaylorAI/gte-tiny"

def load(p): r=list(csv.DictReader(open(p),delimiter="\t")); return [x["text"] for x in r],[x["label"] for x in r]
Xtr,ytr_l=load(a.data); labels=sorted(set(ytr_l)); l2i={l:i for i,l in enumerate(labels)}
ytr=torch.tensor([l2i[l] for l in ytr_l])
print(f"[{a.prefix}] rows={len(Xtr)} labels={len(labels)} head={a.head} epochs={a.epochs}",flush=True)
dev="mps" if torch.backends.mps.is_available() else "cpu"
tok=AutoTokenizer.from_pretrained(MODEL); enc=AutoModel.from_pretrained(MODEL).to(dev)
H=enc.config.hidden_size
if a.head=="mlp":
    head=nn.Sequential(nn.Linear(H,H),nn.ReLU(),nn.Dropout(0.1),nn.Linear(H,len(labels))).to(dev)
else:
    head=nn.Linear(H,len(labels)).to(dev)
for n,p in enc.named_parameters(): p.requires_grad=("encoder.layer.4." in n or "encoder.layer.5." in n)
tr=[p for p in enc.parameters() if p.requires_grad]
opt=torch.optim.AdamW([{"params":tr,"lr":2e-6},{"params":head.parameters(),"lr":1e-3}],eps=1e-6,weight_decay=0.01)
lossf=nn.CrossEntropyLoss(); steps=a.epochs*(len(Xtr)//BS); sched=get_linear_schedule_with_warmup(opt,int(0.1*steps),steps)
def emb(t):
    e=tok(t,padding=True,truncation=True,max_length=64,return_tensors="pt").to(dev)
    o=enc(**e).last_hidden_state; m=e["attention_mask"].unsqueeze(-1).float(); return (o*m).sum(1)/m.sum(1).clamp(min=1)
idx=np.arange(len(Xtr))
for e in range(a.epochs):
    enc.train();head.train();np.random.shuffle(idx);t0=time.time();tot=0;nb=0
    for b in range(0,len(idx),BS):
        bi=idx[b:b+BS]; loss=lossf(head(emb([Xtr[i] for i in bi])),ytr[bi].to(dev))
        opt.zero_grad();loss.backward();torch.nn.utils.clip_grad_norm_(tr+list(head.parameters()),1.0);opt.step();sched.step()
        tot+=loss.item();nb+=1
    print(f"[{a.prefix}] epoch {e+1}/{a.epochs} ({time.time()-t0:.0f}s) loss={tot/nb:.3f}",flush=True)
    if e+1 in SNAP:
        out=f"output/gte-tiny-clean-slate/{a.prefix}_e{e+1}.pt"
        torch.save({"enc":enc.state_dict(),"head":head.state_dict(),"labels":labels,"head_type":a.head},out)
        print(f"[{a.prefix}] saved {out}",flush=True)
print(f"[{a.prefix}] DONE",flush=True)
