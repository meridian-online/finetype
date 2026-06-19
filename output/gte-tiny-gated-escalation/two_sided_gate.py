"""Two-sided gate: override v19 with gte-tiny ONLY where gte-tiny conf >= tau AND v19 was
uncertain (quality_band). v19 confidence from the shipped calibrated-confidence artifacts."""
import csv, sys
import numpy as np
import torch, torch.nn as nn
import pyarrow.parquet as pq
sys.path.insert(0, "scripts")
from score_gold_anchor import _vendored_values, SEP
from transformers import AutoTokenizer, AutoModel

MODEL="TaylorAI/gte-tiny"
CONTESTED={"country","country_code","city","region","full_name","entity_name","iata_code","categorical","word","plain_text"}
FAM2LABEL={"RESIDUAL":"representation.text.word","country":"geography.location.country",
  "country_code":"geography.location.country_code","city":"geography.location.city",
  "region":"geography.location.region","full_name":"identity.person.full_name",
  "entity_name":"representation.text.entity_name","iata_code":"geography.transportation.iata_code"}
def leaf(x): return (x or "").split(".")[-1]

ckpt=torch.load("output/fine-tuned-encoder-discovery/gte_tiny_v2.pt",map_location="cpu"); labels=ckpt["labels"]
dev="mps" if torch.backends.mps.is_available() else "cpu"
tok=AutoTokenizer.from_pretrained(MODEL)
enc=AutoModel.from_pretrained(MODEL).to(dev); enc.load_state_dict(ckpt["enc"]); enc.eval()
head=nn.Linear(enc.config.hidden_size,len(labels)).to(dev); head.load_state_dict(ckpt["head"]); head.eval()
def emb(texts):
    e=tok(texts,padding=True,truncation=True,max_length=64,return_tensors="pt").to(dev)
    o=enc(**e).last_hidden_state; m=e["attention_mask"].unsqueeze(-1).float(); return (o*m).sum(1)/m.sum(1).clamp(min=1)
def predict(texts):
    fams,confs=[],[]
    with torch.no_grad():
        for i in range(0,len(texts),256):
            p=torch.softmax(head(emb(texts[i:i+256])),1).cpu().numpy(); fams+=[labels[j] for j in p.argmax(1)]; confs+=list(p.max(1))
    return fams,confs

samp={}
t=pq.read_table("eval/gittables/corpus_pass/columns.parquet",columns=["file_content_sha256","column_name","sample_values_truncated"])
for s,c,v in zip(t.column(0).to_pylist(),t.column(1).to_pylist(),t.column(2).to_pylist()): samp[(s,c)]=v or ""

def load_band(path):
    d={}
    for r in csv.DictReader(open(path),delimiter="\t"): d[(r["sha"],r["column"])]=r["quality_band"]
    return d

def run(gold_tsv,v19_preds,band_tsv,combos,tag):
    gold=list(csv.DictReader(open(gold_tsv),delimiter="\t"))
    v19={(r["file_content_sha256"],r["column_name"]):r["predicted_label"] for r in csv.DictReader(open(v19_preds),delimiter="\t")}
    band=load_band(band_tsv)
    base_rows,to_route,rk=[],[],[]
    for r in gold:
        k=(r["file_content_sha256"],r["column_name"]); base=v19.get(k)
        if base is None: continue
        if leaf(base) in CONTESTED:
            vals=[x for x in samp.get(k,"").split(SEP) if x.strip()]
            if not vals: vals=[x for x in _vendored_values(r.get("file_path",""),r["column_name"]) if x.strip()]
            if vals: to_route.append(f"header: {r['column_name']} | values: "+", ".join(vals[:8])); rk.append((k,base)); continue
        base_rows.append((k,base))
    fams,confs=predict(to_route)
    res={}
    for (uncertain,tau) in combos:
        rows=list(base_rows); ov=0
        for (k,base),fam,conf in zip(rk,fams,confs):
            b=band.get(k,"low")
            v19_uncertain = (b=="low") if uncertain=="low" else (b in ("low","medium"))
            if conf>=tau and v19_uncertain: rows.append((k,FAM2LABEL[fam])); ov+=1
            else: rows.append((k,base))
        out=f"/tmp/g2_{tag}_{uncertain}_{int(tau*100)}.tsv"
        with open(out,"w",newline="") as f:
            w=csv.writer(f,delimiter="\t",lineterminator="\n"); w.writerow(["file_content_sha256","column_name","predicted_label","confidence"])
            for (s,c),lab in rows: w.writerow([s,c,lab,""])
        res[(uncertain,tau)]=(out,ov)
    return res

COMBOS=[("low",0.95),("low",0.85),("lowmed",0.95),("lowmed",0.85)]
print("GOLD");
for kk,(out,ov) in run("eval/gold/gold_corpus.tsv","output/ceiling-and-rules-discovery/predictions_v19.tsv","output/calibrated-confidence/calib_gold.tsv",COMBOS,"gold").items(): print(kk,"overrides=",ov,out)
print("REPR");
for kk,(out,ov) in run("eval/repr/representative_corpus.tsv","output/representative-accuracy-gate/predictions_v19_repr.tsv","output/calibrated-confidence/calib_repr.tsv",COMBOS,"repr").items(): print(kk,"overrides=",ov,out)
print("DONE")
