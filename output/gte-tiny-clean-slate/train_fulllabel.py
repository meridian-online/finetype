"""ac-02: train the full-label gte-tiny backbone (clean-slate Sense, choice 0103).
v3 recipe: partial-unfreeze top-2 layers, discriminative LR (enc 2e-6 / head 1e-3),
natural CE, grad-clip, warmup. Saves checkpoint after each epoch."""
import csv, time
import numpy as np
import torch, torch.nn as nn
from transformers import AutoTokenizer, AutoModel, get_linear_schedule_with_warmup

MODEL="TaylorAI/gte-tiny"
OUT="output/gte-tiny-clean-slate/gte_tiny_fulllabel.pt"
EPOCHS=4; BS=64

def load(p):
    r=list(csv.DictReader(open(p),delimiter="\t")); return [x["text"] for x in r],[x["label"] for x in r]
Xtr,ytr_l=load("output/gte-tiny-clean-slate/cs_train.tsv")
labels=sorted(set(ytr_l)); l2i={l:i for i,l in enumerate(labels)}
ytr=torch.tensor([l2i[l] for l in ytr_l])
print(f"train rows={len(Xtr)} labels={len(labels)}", flush=True)

dev="mps" if torch.backends.mps.is_available() else "cpu"
tok=AutoTokenizer.from_pretrained(MODEL); enc=AutoModel.from_pretrained(MODEL).to(dev)
head=nn.Linear(enc.config.hidden_size,len(labels)).to(dev)
for n,p in enc.named_parameters(): p.requires_grad=("encoder.layer.4." in n or "encoder.layer.5." in n)
tr=[p for p in enc.parameters() if p.requires_grad]
opt=torch.optim.AdamW([{"params":tr,"lr":2e-6},{"params":head.parameters(),"lr":1e-3}],eps=1e-6,weight_decay=0.01)
lossf=nn.CrossEntropyLoss(); steps=EPOCHS*(len(Xtr)//BS); sched=get_linear_schedule_with_warmup(opt,int(0.1*steps),steps)
def emb(texts):
    e=tok(texts,padding=True,truncation=True,max_length=64,return_tensors="pt").to(dev)
    o=enc(**e).last_hidden_state; m=e["attention_mask"].unsqueeze(-1).float(); return (o*m).sum(1)/m.sum(1).clamp(min=1)
idx=np.arange(len(Xtr))
for e in range(EPOCHS):
    enc.train();head.train();np.random.shuffle(idx);t0=time.time();tot=0;nb=0
    for b in range(0,len(idx),BS):
        bi=idx[b:b+BS]; loss=lossf(head(emb([Xtr[i] for i in bi])),ytr[bi].to(dev))
        opt.zero_grad();loss.backward();torch.nn.utils.clip_grad_norm_(tr+list(head.parameters()),1.0);opt.step();sched.step()
        tot+=loss.item();nb+=1
        if b % 25600==0: print(f"  e{e+1} {b}/{len(idx)} loss={loss.item():.3f}", flush=True)
    torch.save({"enc":enc.state_dict(),"head":head.state_dict(),"labels":labels}, OUT)
    print(f"epoch {e+1}/{EPOCHS} done ({time.time()-t0:.0f}s) avg_loss={tot/nb:.3f} -> saved {OUT}", flush=True)
print("DONE", flush=True)
