"""ac-01 end-to-end fine-tune (NaN-fixed): gte-tiny encoder + head, trained on the
region-fixed contested set. Fixes the earlier MPS NaN with grad-clipping + LR warmup +
lower LR + larger AdamW eps, and auto-falls back to CPU if MPS still diverges.
"""
import csv, time
import numpy as np
import torch, torch.nn as nn
from collections import Counter
from transformers import AutoTokenizer, AutoModel, get_linear_schedule_with_warmup

MODEL = "TaylorAI/gte-tiny"
def load(p, t, l): r = list(csv.DictReader(open(p), delimiter="\t")); return [x[t] for x in r], [x[l] for x in r]
Xtr_t, ytr_l = load("output/fine-tuned-encoder-discovery/encoder_train_region.tsv", "text", "label")
Xte_t, yte_l = load("output/fine-tuned-encoder-discovery/probe_data.tsv", "text", "true_family")
labels = sorted(set(ytr_l) | set(yte_l)); l2i = {l: i for i, l in enumerate(labels)}
ytr = torch.tensor([l2i[l] for l in ytr_l]); yte_i = np.array([l2i[l] for l in yte_l])
tok = AutoTokenizer.from_pretrained(MODEL)
cnt = Counter(ytr.tolist())
cw = torch.tensor([1.0/cnt[i] for i in range(len(labels))]); cw = cw/cw.sum()*len(labels)

def run(device, epochs=3, bs=32, lr=1e-5):
    enc = AutoModel.from_pretrained(MODEL).to(device); head = nn.Linear(enc.config.hidden_size, len(labels)).to(device)
    lossf = nn.CrossEntropyLoss(weight=cw.to(device))
    opt = torch.optim.AdamW(list(enc.parameters())+list(head.parameters()), lr=lr, eps=1e-6, weight_decay=0.01)
    steps = epochs*(len(Xtr_t)//bs); sched = get_linear_schedule_with_warmup(opt, int(0.1*steps), steps)
    def emb(texts):
        e = tok(texts, padding=True, truncation=True, max_length=64, return_tensors="pt").to(device)
        o = enc(**e).last_hidden_state; m = e["attention_mask"].unsqueeze(-1).float()
        return (o*m).sum(1)/m.sum(1).clamp(min=1)
    def evaluate():
        enc.eval(); head.eval(); preds=[]
        with torch.no_grad():
            for i in range(0,len(Xte_t),64): preds.append(head(emb(Xte_t[i:i+64])).argmax(1).cpu().numpy())
        p=np.concatenate(preds); acc=(p==yte_i).mean(); ri=l2i["RESIDUAL"]
        rp,rt=(p==ri),(yte_i==ri)
        per={fm:round((p[yte_i==l2i[fm]]==l2i[fm]).mean(),2) for fm in ["country_code","city","region","country","full_name"] if (yte_i==l2i[fm]).sum()}
        return acc,(rp&rt).sum()/max(rp.sum(),1),(rp&rt).sum()/max(rt.sum(),1),per
    idx=np.arange(len(Xtr_t)); best=None
    for ep in range(epochs):
        enc.train(); head.train(); np.random.shuffle(idx); t0=time.time(); tot=0; nb=0
        for b in range(0,len(idx),bs):
            bi=idx[b:b+bs]
            loss=lossf(head(emb([Xtr_t[i] for i in bi])), ytr[bi].to(device))
            if torch.isnan(loss): print(f"  NaN on {device} step {nb}"); return None
            opt.zero_grad(); loss.backward()
            torch.nn.utils.clip_grad_norm_(list(enc.parameters())+list(head.parameters()), 1.0)
            opt.step(); sched.step(); tot+=loss.item(); nb+=1
        acc,rprec,rrec,per=evaluate()
        print(f"  epoch {ep+1}: loss {tot/nb:.3f} ({time.time()-t0:.0f}s) | contested acc {acc:.3f} | RESIDUAL P {rprec:.2f} R {rrec:.2f} | {per}")
        best=(acc,rprec,rrec,per)
    return best

dev = "mps" if torch.backends.mps.is_available() else "cpu"
print(f"=== fine-tune on {dev} (grad-clip + warmup + lr1e-5 + eps1e-6) ===")
res = run(dev)
if res is None:
    print("=== MPS diverged; retry on CPU ===")
    res = run("cpu")
print(f"\nfinal: {res}\nreference: frozen-head region-fix 0.783 / best-frozen 0.836 | shipped 0.684 | ceiling 0.893")
