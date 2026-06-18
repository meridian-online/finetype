"""ac-01 region fix: improved region matcher (admin1+admin2 raw+suffix-stripped+us_states+
boroughs+ISO-strip) + a mined pool of real region/county columns. Re-label, train gte-tiny
frozen head, eval the 244 contested — does region recall recover?
"""
import csv, re
import numpy as np
from collections import Counter
from sentence_transformers import SentenceTransformer
from sklearn.linear_model import LogisticRegression

GN="/Users/hugh/datasets/geonames/2026-05-24/"; LR="eval/gold/lens_reference/"; SEP="│"
def norm(s): return s.strip().lower()
SUF=re.compile(r"\s+(county|parish|borough|district|municipality|co\.?|shire|prefecture|province|region|oblast|raion|department|governorate)$", re.I)

# --- vocabs ---
def col(path,i,enc="utf-8"):
    return [p[i].strip() for p in (l.split("\t") for l in open(path,encoding=enc)) if len(p)>i and p[i].strip()]
cities=set(norm(x) for x in col(LR+"cities15000.txt",1)+col(LR+"cities15000.txt",2))
countries=set(norm(r["name"]) for r in csv.DictReader(open(LR+"iso3166.csv")))
ccodes=set()
for r in csv.DictReader(open(LR+"iso3166.csv")): ccodes|={norm(r["alpha-2"]),norm(r["alpha-3"])}
iata=set()
for r in csv.DictReader(open("eval/datasets/csv/airports.csv")):
    for k,v in r.items():
        if "iata" in k.lower() and v and len(v.strip())==3: iata.add(norm(v))
# improved region vocab
reg=set()
for x in col(LR+"admin1CodesASCII.txt",1)+col(LR+"admin1CodesASCII.txt",2)+col(GN+"admin2Codes.txt",1):
    reg.add(norm(x)); reg.add(SUF.sub("",norm(x)))
for row in csv.reader(open("eval/datasets/csv/us_states.csv")):
    for c in row: reg.add(norm(c))
reg|={"brooklyn","queens","bronx","manhattan","staten island","kings"}
def reg_match(v):
    n=norm(v)
    return n in reg or SUF.sub("",n) in reg or (n[:3] in ("us-","gb-","ca-") and n[3:] in reg)
VOCSET={"city":cities,"country":countries,"country_code":ccodes,"iata_code":iata}

def parse(raw): return [v.strip() for v in (raw or "").split(SEP) if v.strip()][:8]
def frac_in(vals,fn): return sum(fn(v) for v in vals)/max(len(vals),1)

rows=[]  # (text,label)
# geo + over-emission residual from the mined pool
for r in csv.DictReader(open("output/fine-tuned-encoder-discovery/mining_pool.tsv"),delimiter="\t"):
    leaf,hdr,vals=r["leaf"],r["column_name"],parse(r["sample_values_truncated"])
    if not vals: continue
    txt=f"header: {hdr} | values: "+", ".join(vals)
    if leaf=="region" or leaf in VOCSET:
        fn=reg_match if leaf=="region" else (lambda v,s=VOCSET.get(leaf,set()): norm(v) in s)
        rows.append((txt, leaf if frac_in(vals,fn)>=0.4 else "RESIDUAL"))
    elif leaf in ("categorical","word","plain_text"): rows.append((txt,"RESIDUAL"))
    elif leaf in ("full_name","entity_name"): rows.append((txt,leaf))  # keep model's call for these
# real region columns (header-confirmed) -> region when values match, else skip (avoid noise)
nreg=0
for r in csv.DictReader(open("output/fine-tuned-encoder-discovery/region_pool.tsv"),delimiter="\t"):
    vals=parse(r["sample_values_truncated"])
    if vals and frac_in(vals,reg_match)>=0.4:
        rows.append((f"header: {r['column_name']} | values: "+", ".join(vals),"region")); nreg+=1
# residual + entity from distilled
import gzip,json
RS={"representation.discrete.categorical","representation.text.word","representation.text.plain_text"}
GH=["value","data","field","name","label","item","type","status","code","description","notes","key"]
import random; random.seed(1)
with gzip.open("output/distillation-v3/sherlock_distilled.csv.gz","rt") as f:
    nr=ne=0
    for r in csv.DictReader(f):
        lab=r.get("final_label") or ""; sv=r.get("sample_values") or ""
        try: vals=[str(v) for v in json.loads(sv) if str(v).strip() and str(v)!="None"][:8]
        except: continue
        if not vals: continue
        if lab in RS and nr<6000: rows.append((f"header: {random.choice(GH)} | values: "+", ".join(vals),"RESIDUAL")); nr+=1
        elif lab=="representation.text.entity_name" and ne<2500: rows.append((f"header: {random.choice(GH+['company','brand','product'])} | values: "+", ".join(vals),"entity_name")); ne+=1

print(f"region from region_pool: {nreg}; total rows {len(rows)}; dist {dict(Counter(l for _,l in rows))}")
with open("output/fine-tuned-encoder-discovery/encoder_train_region.tsv","w",newline="") as f:
    w=csv.writer(f,delimiter="\t",lineterminator="\n"); w.writerow(["text","label"])
    for t,l in rows: w.writerow([t.replace("\t"," ").replace("\n"," "),l])
print("dumped encoder_train_region.tsv")
Xtr_t=[t for t,_ in rows]; ytr=np.array([l for _,l in rows])
te=list(csv.DictReader(open("output/fine-tuned-encoder-discovery/probe_data.tsv"),delimiter="\t"))
Xte_t=[r["text"] for r in te]; yte=np.array([r["true_family"] for r in te])
m=SentenceTransformer("TaylorAI/gte-tiny",device="mps")
Xtr=m.encode(Xtr_t,show_progress_bar=False,batch_size=256); Xte=m.encode(Xte_t,show_progress_bar=False,batch_size=256)
for cw,name in [(None,"natural"),("balanced","balanced")]:
    pred=LogisticRegression(max_iter=3000,class_weight=cw).fit(Xtr,ytr).predict(Xte)
    acc=(pred==yte).mean(); rp,rt=(pred=="RESIDUAL"),(yte=="RESIDUAL")
    per={fm:round((pred[yte==fm]==fm).mean(),2) for fm in ["country_code","city","region","country","full_name"] if (yte==fm).sum()}
    print(f"\n[region-fix gte-tiny, {name}] gold-contested acc {acc:.3f} (prev best 0.836; shipped 0.684; ceiling 0.893)")
    print(f"  RESIDUAL P {(rp&rt).sum()/max(rp.sum(),1):.3f} R {(rp&rt).sum()/max(rt.sum(),1):.3f} | per-family {per}")
