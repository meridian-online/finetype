"""ac-01: end-to-end fine-tune of gte-tiny (encoder + head) on the assembled contested
training set, precedence-aware (class-balanced loss so RESIDUAL doesn't dominate — the
0-for-6 attractor defence). Eval each epoch on the held-out gold/repr contested set.
"""
import csv, time
import numpy as np
import torch
import torch.nn as nn
from collections import Counter
from transformers import AutoTokenizer, AutoModel

dev = "mps" if torch.backends.mps.is_available() else "cpu"
MODEL = "TaylorAI/gte-tiny"

def load(path, tcol, lcol):
    r = list(csv.DictReader(open(path), delimiter="\t"))
    return [x[tcol] for x in r], [x[lcol] for x in r]

Xtr_text, ytr_lab = load("output/fine-tuned-encoder-discovery/encoder_train.tsv", "text", "label")
Xte_text, yte_lab = load("output/fine-tuned-encoder-discovery/probe_data.tsv", "text", "true_family")
labels = sorted(set(ytr_lab) | set(yte_lab))
l2i = {l: i for i, l in enumerate(labels)}
ytr = torch.tensor([l2i[l] for l in ytr_lab])
yte_i = np.array([l2i[l] for l in yte_lab])
print(f"device {dev}; {len(Xtr_text)} train, {len(Xte_text)} test; {len(labels)} classes: {labels}")

tok = AutoTokenizer.from_pretrained(MODEL)
enc = AutoModel.from_pretrained(MODEL).to(dev)
head = nn.Linear(enc.config.hidden_size, len(labels)).to(dev)

def embed(texts, train=False):
    e = tok(texts, padding=True, truncation=True, max_length=64, return_tensors="pt").to(dev)
    out = enc(**e).last_hidden_state
    m = e["attention_mask"].unsqueeze(-1).float()
    return (out * m).sum(1) / m.sum(1).clamp(min=1)

# class-balanced loss (precedence-aware: residual is majority, downweight it)
cnt = Counter(ytr.tolist())
w = torch.tensor([1.0 / cnt[i] for i in range(len(labels))], dtype=torch.float32)
w = (w / w.sum() * len(labels)).to(dev)
lossf = nn.CrossEntropyLoss(weight=w)
opt = torch.optim.AdamW(list(enc.parameters()) + list(head.parameters()), lr=2e-5)

def evaluate():
    enc.eval(); head.eval()
    preds = []
    with torch.no_grad():
        for i in range(0, len(Xte_text), 64):
            logits = head(embed(Xte_text[i:i+64]))
            preds.append(logits.argmax(1).cpu().numpy())
    pred = np.concatenate(preds)
    acc = (pred == yte_i).mean()
    ri = l2i["RESIDUAL"]
    rp, rt = (pred == ri), (yte_i == ri)
    rprec = (rp & rt).sum() / max(rp.sum(), 1); rrec = (rp & rt).sum() / max(rt.sum(), 1)
    per = {}
    for fm in ["country_code", "city", "region", "country", "full_name"]:
        if fm in l2i:
            m = yte_i == l2i[fm]
            if m.sum(): per[fm] = round((pred[m] == l2i[fm]).mean(), 2)
    return acc, rprec, rrec, per

bs = 32
idx = np.arange(len(Xtr_text))
for ep in range(3):
    enc.train(); head.train()
    np.random.shuffle(idx)
    t0 = time.time(); tot = 0.0
    for b in range(0, len(idx), bs):
        bi = idx[b:b+bs]
        logits = head(embed([Xtr_text[i] for i in bi], train=True))
        loss = lossf(logits, ytr[bi].to(dev))
        opt.zero_grad(); loss.backward(); opt.step()
        tot += loss.item()
    acc, rprec, rrec, per = evaluate()
    print(f"epoch {ep+1}: loss {tot/(len(idx)//bs):.3f} ({time.time()-t0:.0f}s) | "
          f"gold-contested acc {acc:.3f} | RESIDUAL P {rprec:.3f} R {rrec:.3f} | {per}")

print("\nreference: shipped model 0.684 | frozen-head mined proof 0.82 | zero-shot ceiling 0.893")
torch.save({"enc": enc.state_dict(), "head": head.state_dict(), "labels": labels},
           "output/fine-tuned-encoder-discovery/gte_tiny_finetuned.pt")
print("saved gte_tiny_finetuned.pt")
