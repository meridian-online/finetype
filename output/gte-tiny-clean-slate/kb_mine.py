"""ac-01 (clean labels): extended KB-membership miner. v19 proposes a KB-backed family;
the authoritative vocab confirms (>=50% members -> clean positive) or demotes to residual.
Produces clean positives AND clean residuals, model-independent. Extends geonames_proof.py."""
import csv, pyarrow.parquet as pq
from collections import defaultdict
LR="eval/gold/lens_reference/"; DS="eval/datasets/csv/"; SEP="│"
def norm(s): return s.strip().lower()

cities=set()
for ln in open(LR+"cities15000.txt",encoding="utf-8"):
    p=ln.split("\t");
    if len(p)>2: cities.add(norm(p[1])); cities.add(norm(p[2]))
regions=set()
for ln in open(LR+"admin1CodesASCII.txt",encoding="utf-8"):
    p=ln.split("\t")
    if len(p)>2: regions.add(norm(p[1])); regions.add(norm(p[2]))
country_names=set(); country_codes=set(); continents=set()
for r in csv.DictReader(open(LR+"iso3166.csv")):
    country_names.add(norm(r["name"])); country_codes.add(norm(r["alpha-2"])); country_codes.add(norm(r["alpha-3"]))
    if r.get("region"): continents.add(norm(r["region"]))
states=set()
for r in csv.DictReader(open(DS+"us_states.csv")):
    states.add(norm(r["Abbreviation"])); regions.add(norm(r["State"]))
iata=set(); icao=set()
for r in csv.DictReader(open(DS+"airports.csv")):
    if r.get("iata") and len(r["iata"])==3: iata.add(norm(r["iata"]))
    if r.get("icao") and len(r["icao"])==4: icao.add(norm(r["icao"]))
tlds=set(norm(l) for l in open(LR+"iana_tlds_alpha.txt") if l.strip() and not l.startswith("#"))

VOCAB={"geography.location.city":cities,"geography.location.region":regions,
  "geography.location.country":country_names,"geography.location.country_code":country_codes,
  "geography.location.continent":continents,"geography.location.state_code":states,
  "geography.transportation.iata_code":iata,"geography.transportation.icao_code":icao,
  "technology.internet.top_level_domain":tlds}
print("vocab sizes:", {k.split('.')[-1]:len(v) for k,v in VOCAB.items()})

t=pq.read_table("output/ydf-validation-gate/v19_gated.parquet",
   columns=["column_name","sense_prediction","sample_values_truncated","is_trivial"])
cn=t.column(0).to_pylist(); sp=t.column(1).to_pylist(); sv=t.column(2).to_pylist(); tv=t.column(3).to_pylist()
out=open("output/gte-tiny-clean-slate/kb_clean.tsv","w",newline=""); w=csv.writer(out,delimiter="\t",lineterminator="\n"); w.writerow(["text","label"])
stat=defaultdict(lambda:[0,0])  # leaf -> [proposed, confirmed]
nres=0
for i in range(len(cn)):
    if tv[i] or sp[i] not in VOCAB: continue
    vals=[v.strip() for v in (sv[i] or "").split(SEP) if v.strip()][:8]
    if not vals: continue
    voc=VOCAB[sp[i]]; frac=sum(norm(v) in voc for v in vals)/len(vals)
    stat[sp[i]][0]+=1
    lab = sp[i] if frac>=0.5 else "representation.text.word"
    if lab==sp[i]: stat[sp[i]][1]+=1
    else: nres+=1
    w.writerow([f"header: {cn[i]} | values: "+", ".join(vals), lab])
out.close()
print(f"\n{'family':22} {'proposed':>9} {'confirmed':>10} {'confirm%':>9}")
for k,(p,c) in sorted(stat.items(),key=lambda x:-x[1][0]):
    print(f"  {k.split('.')[-1]:22} {p:>9} {c:>10} {c/max(p,1):>8.0%}")
print(f"\nv19->residual demotions (over-emission corrected): {nres}")
