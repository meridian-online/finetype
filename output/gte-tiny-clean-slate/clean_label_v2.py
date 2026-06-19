"""ac-01 clean labels v2: combined model-independent verifier. v19 proposes a type; it is
confirmed iff the values are members of that type's authoritative signal — taxonomy enum,
KB vocab (geo/names), or validator regex — else demoted to residual. Extends kb_mine.py."""
import csv, re, yaml, glob, pyarrow.parquet as pq
from collections import defaultdict
LR="eval/gold/lens_reference/"; DS="eval/datasets/csv/"; SEP="│"
def norm(s): return s.strip().lower()

# --- taxonomy validation: enum + pattern per type ---
ENUM={}; PAT={}
for f in glob.glob("labels/definitions_*.yaml"):
    for k,v in (yaml.safe_load(open(f)) or {}).items():
        val=(v or {}).get("validation") or {}
        if not isinstance(val,dict): continue
        if val.get("enum"): ENUM[k]=set(norm(str(x)) for x in val["enum"])
        elif val.get("pattern"):
            try: PAT[k]=re.compile(val["pattern"])
            except re.error: pass

# --- KB vocabs (geo) ---
def load_geo():
    cities=set(); regions=set()
    for ln in open(LR+"cities15000.txt",encoding="utf-8"):
        p=ln.split("\t");
        if len(p)>2: cities.add(norm(p[1])); cities.add(norm(p[2]))
    for ln in open(LR+"admin1CodesASCII.txt",encoding="utf-8"):
        p=ln.split("\t")
        if len(p)>2: regions.add(norm(p[1])); regions.add(norm(p[2]))
    cn=set(); cc=set()
    for r in csv.DictReader(open(LR+"iso3166.csv")):
        cn.add(norm(r["name"])); cc.add(norm(r["alpha-2"])); cc.add(norm(r["alpha-3"]))
    for r in csv.DictReader(open(DS+"us_states.csv")): regions.add(norm(r["State"]))
    iata=set(); icao=set()
    for r in csv.DictReader(open(DS+"airports.csv")):
        if r.get("iata") and len(r["iata"])==3: iata.add(norm(r["iata"]))
        if r.get("icao") and len(r["icao"])==4: icao.add(norm(r["icao"]))
    return {"geography.location.city":cities,"geography.location.region":regions,
      "geography.location.country":cn,"geography.transportation.iata_code":iata,
      "geography.transportation.icao_code":icao}
KB=load_geo()
firstnames=set(norm(r["name"]) for r in csv.DictReader(open("eval/datasets/validate_corpus/csv/us_baby_names.csv")))
KB["identity.person.first_name"]=firstnames
NAME_HDR=["name","person","author","contact","owner","applicant","customer","employee"]
ENTITY_HDR=["name","company","org","brand","product","title","entity","vendor","team","club"]
RESIDUAL_LEAVES={"representation.discrete.categorical","representation.text.word","representation.text.plain_text"}
RES="representation.text.word"

def frac_in(vals,voc): return sum(norm(v) in voc for v in vals)/len(vals)
def frac_match(vals,rx): return sum(bool(rx.match(v)) for v in vals)/len(vals)
def verify(T,header,vals):
    if T in RESIDUAL_LEAVES: return RES
    if T in ENUM: return T if frac_in(vals,ENUM[T])>=0.5 else RES
    if T in KB: return T if frac_in(vals,KB[T])>=0.5 else RES
    if T=="identity.person.full_name":
        ok=sum(len(v.split())>=2 and v[:1].isupper() for v in vals)/len(vals); return T if ok>=0.5 else RES
    if T=="representation.text.entity_name":
        return T if any(k in (header or "").lower() for k in ENTITY_HDR) else RES
    if T in PAT: return T if frac_match(vals,PAT[T])>=0.7 else RES
    return T  # no verifier (rare/structural) — keep v19

t=pq.read_table("output/ydf-validation-gate/v19_gated.parquet",
   columns=["column_name","sense_prediction","sample_values_truncated","is_trivial"])
cn=t.column(0).to_pylist(); sp=t.column(1).to_pylist(); sv=t.column(2).to_pylist(); tv=t.column(3).to_pylist()
out=open("output/gte-tiny-clean-slate/clean_v2.tsv","w",newline=""); w=csv.writer(out,delimiter="\t",lineterminator="\n"); w.writerow(["text","label"])
stat=defaultdict(lambda:[0,0]); kept=0
for i in range(len(cn)):
    if tv[i] or sp[i]=="unknown": continue
    vals=[v.strip() for v in (sv[i] or "").split(SEP) if v.strip()][:8]
    if not vals: continue
    lab=verify(sp[i],cn[i],vals); stat[sp[i]][0]+=1
    if lab==sp[i]: stat[sp[i]][1]+=1
    w.writerow([f"header: {cn[i]} | values: "+", ".join(vals), lab]); kept+=1
out.close()
print(f"labelled {kept} cols")
verifiable=[(k,p,c) for k,(p,c) in stat.items() if k in ENUM or k in KB or k in PAT or k in ("identity.person.full_name","representation.text.entity_name")]
print(f"\n{'type':40} {'proposed':>9} {'confirm%':>9}")
for k,p,c in sorted(verifiable,key=lambda x:-x[1])[:24]:
    print(f"  {k.split('.',1)[1][:38]:38} {p:>9} {c/max(p,1):>8.0%}")
