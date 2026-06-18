"""ac-01 training rebuild v2 — fixes the corpus-gate root cause:
1. CLEAN entity_name from distilled LLM-panel labels (not v19's noisy calls).
2. entity_name and word kept DISTINCT, well-populated classes.
3. RESIDUAL rebalanced down (was 44%) + sourced only from clean distilled residual +
   GEO-family over-emissions (no entity contamination).
Real headers throughout: distilled (header-less) gets headers SAMPLED from the actual
corpus headers seen for that family (keeps the header distribution realistic + consistent).
"""
import csv, gzip, json, re, random
random.seed(7)
GN="/Users/hugh/datasets/geonames/2026-05-24/"; LR="eval/gold/lens_reference/"; SEP="│"
def norm(s): return s.strip().lower()
SUF=re.compile(r"\s+(county|parish|borough|district|municipality|co\.?|shire|prefecture|province|region|oblast|raion|department|governorate)$", re.I)

def col(path,i,enc="utf-8"): return [p[i].strip() for p in (l.split("\t") for l in open(path,encoding=enc)) if len(p)>i and p[i].strip()]
cities=set(norm(x) for x in col(LR+"cities15000.txt",1)+col(LR+"cities15000.txt",2))
countries=set(norm(r["name"]) for r in csv.DictReader(open(LR+"iso3166.csv")))
ccodes=set();  _=[ccodes.update({norm(r["alpha-2"]),norm(r["alpha-3"])}) for r in csv.DictReader(open(LR+"iso3166.csv"))]
iata=set()
for r in csv.DictReader(open("eval/datasets/csv/airports.csv")):
    for k,v in r.items():
        if "iata" in k.lower() and v and len(v.strip())==3: iata.add(norm(v))
reg=set()
for x in col(LR+"admin1CodesASCII.txt",1)+col(LR+"admin1CodesASCII.txt",2)+col(GN+"admin2Codes.txt",1):
    reg.add(norm(x)); reg.add(SUF.sub("",norm(x)))
for row in csv.reader(open("eval/datasets/csv/us_states.csv")):
    for c in row: reg.add(norm(c))
reg|={"brooklyn","queens","bronx","manhattan","staten island","kings"}
def reg_match(v): n=norm(v); return n in reg or SUF.sub("",n) in reg or (n[:3] in ("us-","gb-","ca-") and n[3:] in reg)
VOCSET={"city":cities,"country":countries,"country_code":ccodes,"iata_code":iata}
GEO=set(VOCSET)|{"region"}

def parse(raw): return [v.strip() for v in (raw or "").split(SEP) if v.strip()][:8]
def frac(vals,fn): return sum(fn(v) for v in vals)/max(len(vals),1)

# --- collect REAL corpus headers per family (for realistic headers on distilled data) ---
corp_headers={"entity_name":[],"full_name":[],"RESIDUAL":[]}
mined=list(csv.DictReader(open("output/fine-tuned-encoder-discovery/mining_pool.tsv"),delimiter="\t"))
for r in mined:
    lf=r["leaf"]; h=r["column_name"]
    if lf=="entity_name": corp_headers["entity_name"].append(h)
    elif lf=="full_name": corp_headers["full_name"].append(h)
    elif lf in ("categorical","word","plain_text"): corp_headers["RESIDUAL"].append(h)
for k in corp_headers:
    if not corp_headers[k]: corp_headers[k]=["value","name","field"]
def hdr(fam): return random.choice(corp_headers[fam])

rows=[]
# --- GEO positives + geo-family over-emission residuals (real headers, vocab-clean) ---
geo_resid=0
for r in mined:
    leaf,h,vals=r["leaf"],r["column_name"],parse(r["sample_values_truncated"])
    if not vals: continue
    txt=f"header: {h} | values: "+", ".join(vals)
    if leaf in GEO:
        fn=reg_match if leaf=="region" else (lambda v,s=VOCSET[leaf]: norm(v) in s)
        if frac(vals,fn)>=0.4: rows.append((txt,leaf))
        else: rows.append((txt,"RESIDUAL")); geo_resid+=1     # geo-shaped, not in vocab -> residual (NOT entity)
for r in csv.DictReader(open("output/fine-tuned-encoder-discovery/region_pool.tsv"),delimiter="\t"):
    vals=parse(r["sample_values_truncated"])
    if vals and frac(vals,reg_match)>=0.4: rows.append((f"header: {r['column_name']} | values: "+", ".join(vals),"region"))

# --- CLEAN entity_name / full_name / residual from DISTILLED (LLM labels) ---
RS={"representation.discrete.categorical","representation.text.word","representation.text.plain_text"}
CAP={"entity_name":6000,"full_name":4000,"RESIDUAL":6000}
got={"entity_name":0,"full_name":0,"RESIDUAL":0}
with gzip.open("output/distillation-v3/sherlock_distilled.csv.gz","rt") as f:
    for r in csv.DictReader(f):
        lab=r.get("final_label") or ""; sv=r.get("sample_values") or ""
        try: vals=[str(v) for v in json.loads(sv) if str(v).strip() and str(v)!="None"][:8]
        except: continue
        if not vals: continue
        fam = "entity_name" if lab=="representation.text.entity_name" else ("full_name" if lab=="identity.person.full_name" else ("RESIDUAL" if lab in RS else None))
        if fam and got[fam]<CAP[fam]:
            rows.append((f"header: {hdr(fam)} | values: "+", ".join(vals),fam)); got[fam]+=1
# people_directory clean full_name
for r in csv.DictReader(open("eval/datasets/csv/people_directory.csv")):
    if r.get("full_name"): rows.append((f"header: {random.choice(['name','full_name','author','contact'])} | values: {r['full_name']}","full_name"))

random.shuffle(rows)
from collections import Counter
# downsample RESIDUAL to ~25% (it was defaulting to word at corpus scale)
nonres=[r for r in rows if r[1]!="RESIDUAL"]; res=[r for r in rows if r[1]=="RESIDUAL"]
target_res=int(len(nonres)/3)            # residual ~= 25% of total
random.shuffle(res); rows=nonres+res[:target_res]; random.shuffle(rows)
d=Counter(l for _,l in rows)
print(f"geo-family residuals: {geo_resid}; distilled clean: {got}; total {len(rows)}")
print("label dist:", dict(d), "| residual frac:", round(d['RESIDUAL']/len(rows),3))
with open("output/fine-tuned-encoder-discovery/encoder_train_v2.tsv","w",newline="") as f:
    w=csv.writer(f,delimiter="\t",lineterminator="\n"); w.writerow(["text","label"])
    for t,l in rows: w.writerow([t.replace("\t"," ").replace("\n"," "),l])
print("wrote encoder_train_v2.tsv")
