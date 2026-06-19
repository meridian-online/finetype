"""One-sided corpus over-emission check (STRICTER bound than the proven two-sided gate).
On the 33k stratified sample's 100k contested columns: override v19 with gte-tiny where
conf>=tau, measure per-label emission shift vs v19. Flags destination drift (over_emit)."""
import csv
from collections import defaultdict
import numpy as np
import torch, torch.nn as nn
import pyarrow.parquet as pq
from transformers import AutoTokenizer, AutoModel

MODEL="TaylorAI/gte-tiny"
FAM2LABEL={"RESIDUAL":"representation.text.word","country":"geography.location.country",
  "country_code":"geography.location.country_code","city":"geography.location.city",
  "region":"geography.location.region","full_name":"identity.person.full_name",
  "entity_name":"representation.text.entity_name","iata_code":"geography.transportation.iata_code"}
SEP="│"
ckpt=torch.load("output/fine-tuned-encoder-discovery/gte_tiny_v2.pt",map_location="cpu"); labels=ckpt["labels"]
dev="mps" if torch.backends.mps.is_available() else "cpu"
tok=AutoTokenizer.from_pretrained(MODEL)
enc=AutoModel.from_pretrained(MODEL).to(dev); enc.load_state_dict(ckpt["enc"]); enc.eval()
head=nn.Linear(enc.config.hidden_size,len(labels)).to(dev); head.load_state_dict(ckpt["head"]); head.eval()
def emb(texts):
    e=tok(texts,padding=True,truncation=True,max_length=64,return_tensors="pt").to(dev)
    o=enc(**e).last_hidden_state; m=e["attention_mask"].unsqueeze(-1).float(); return (o*m).sum(1)/m.sum(1).clamp(min=1)

t=pq.read_table("/tmp/contested_sample.parquet")
cn=t.column("column_name").to_pylist(); sp=t.column("sense_prediction").to_pylist(); sv=t.column("sample_values_truncated").to_pylist()
texts=[f"header: {cn[i]} | values: "+", ".join([v for v in (sv[i] or '').split(SEP) if v.strip()][:8]) for i in range(len(cn))]
print(f"embedding {len(texts)} contested cols...")
fams=[]; confs=[]
with torch.no_grad():
    for i in range(0,len(texts),512):
        p=torch.softmax(head(emb(texts[i:i+512])),1).cpu().numpy(); fams+=[labels[j] for j in p.argmax(1)]; confs+=list(p.max(1))
        if i % 20480==0: print(f"  {i}/{len(texts)}")
confs=np.array(confs)

base={}
for r in csv.DictReader(open("/tmp/v19_baseline_counts.tsv"),delimiter="\t"): base[r["lbl"]]=int(r["n"])

for tau in [0.90,0.95,0.98]:
    delta=defaultdict(int); nov=0
    for i in range(len(fams)):
        if confs[i]>=tau:
            new=FAM2LABEL[fams[i]]
            if new!=sp[i]: delta[sp[i]]-=1; delta[new]+=1; nov+=1
    print(f"\n=== tau={tau}: {nov}/{len(fams)} contested overridden ({100*nov/len(fams):.0f}%) ===")
    rows=[]
    for lbl in set(list(base)+list(delta)):
        b=base.get(lbl,0); g=b+delta.get(lbl,0)
        if abs(delta.get(lbl,0))>=50: rows.append((lbl.split('.')[-1], b, g, (g/b if b else float('inf')), delta.get(lbl,0)))
    rows.sort(key=lambda r:-r[4])
    print(f"  {'label':22} {'v19':>7} {'gated':>7} {'ratio':>6} {'delta':>7}")
    for lbl,b,g,ratio,d in rows[:8]:
        print(f"  {lbl:22} {b:>7} {g:>7} {ratio:>6.2f} {d:>+7}")
    print("  ...largest drains:")
    for lbl,b,g,ratio,d in sorted(rows,key=lambda r:r[4])[:4]:
        print(f"  {lbl:22} {b:>7} {g:>7} {ratio:>6.2f} {d:>+7}")
print("\nDONE")
