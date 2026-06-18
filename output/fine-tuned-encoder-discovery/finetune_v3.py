"""ac-01 end-to-end fine-tune v3 — anti-collapse: freeze lower encoder layers, tiny
encoder LR (discriminative), NO class weights (natural CE), to avoid the catastrophic
forgetting v2 hit. Goal: beat the frozen-head 0.783/0.836 by gently adapting gte-tiny."""
import csv, time
import numpy as np
import torch, torch.nn as nn
from transformers import AutoTokenizer, AutoModel, get_linear_schedule_with_warmup
MODEL="TaylorAI/gte-tiny"
def load(p,t,l): r=list(csv.DictReader(open(p),delimiter="\t")); return [x[t] for x in r],[x[l] for x in r]
Xtr_t,ytr_l=load("output/fine-tuned-encoder-discovery/encoder_train_region.tsv","text","label")
Xte_t,yte_l=load("output/fine-tuned-encoder-discovery/probe_data.tsv","text","true_family")
labels=sorted(set(ytr_l)|set(yte_l)); l2i={l:i for i,l in enumerate(labels)}
ytr=torch.tensor([l2i[l] for l in ytr_l]); yte_i=np.array([l2i[l] for l in yte_l])
tok=AutoTokenizer.from_pretrained(MODEL)
dev="mps" if torch.backends.mps.is_available() else "cpu"
enc=AutoModel.from_pretrained(MODEL).to(dev); head=nn.Linear(enc.config.hidden_size,len(labels)).to(dev)
# freeze embeddings + all but the top 2 transformer layers (gte-tiny has 6)
for n,p in enc.named_parameters():
    p.requires_grad = ("encoder.layer.4." in n or "encoder.layer.5." in n)
trainable=[p for p in enc.parameters() if p.requires_grad]
print(f"dev {dev}; trainable encoder params {sum(p.numel() for p in trainable)/1e6:.1f}M (top-2 layers)")
opt=torch.optim.AdamW([{"params":trainable,"lr":2e-6},{"params":head.parameters(),"lr":1e-3}], eps=1e-6, weight_decay=0.01)
lossf=nn.CrossEntropyLoss()  # natural, no class weights
bs=32; epochs=3; steps=epochs*(len(Xtr_t)//bs); sched=get_linear_schedule_with_warmup(opt,int(0.1*steps),steps)
def emb(texts):
    e=tok(texts,padding=True,truncation=True,max_length=64,return_tensors="pt").to(dev)
    o=enc(**e).last_hidden_state; m=e["attention_mask"].unsqueeze(-1).float()
    return (o*m).sum(1)/m.sum(1).clamp(min=1)
def evaluate():
    enc.eval();head.eval();preds=[]
    with torch.no_grad():
        for i in range(0,len(Xte_t),64): preds.append(head(emb(Xte_t[i:i+64])).argmax(1).cpu().numpy())
    p=np.concatenate(preds);acc=(p==yte_i).mean();ri=l2i["RESIDUAL"];rp,rt=(p==ri),(yte_i==ri)
    per={fm:round((p[yte_i==l2i[fm]]==l2i[fm]).mean(),2) for fm in ["country_code","city","region","country","full_name"] if (yte_i==l2i[fm]).sum()}
    return acc,(rp&rt).sum()/max(rp.sum(),1),(rp&rt).sum()/max(rt.sum(),1),per
idx=np.arange(len(Xtr_t))
for ep in range(epochs):
    enc.train();head.train();np.random.shuffle(idx);t0=time.time();tot=nb=0
    for b in range(0,len(idx),bs):
        bi=idx[b:b+bs]; loss=lossf(head(emb([Xtr_t[i] for i in bi])),ytr[bi].to(dev))
        opt.zero_grad();loss.backward();torch.nn.utils.clip_grad_norm_(trainable+list(head.parameters()),1.0);opt.step();sched.step()
        tot+=loss.item();nb+=1
    acc,rprec,rrec,per=evaluate()
    print(f"  epoch {ep+1}: loss {tot/nb:.3f} ({time.time()-t0:.0f}s) | contested {acc:.3f} | RESIDUAL P {rprec:.2f} R {rrec:.2f} | {per}")
print("reference: frozen-head region-fix 0.783 / best 0.836 | shipped 0.684 | ceiling 0.893")
