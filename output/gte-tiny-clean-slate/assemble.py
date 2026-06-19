"""Clean-slate full-label training set (ac-01), residual-balanced to ~25%."""
import csv, random, pyarrow.parquet as pq
random.seed(42)
SEP="│"
FAM2LABEL={"country":"geography.location.country","country_code":"geography.location.country_code",
  "city":"geography.location.city","region":"geography.location.region",
  "full_name":"identity.person.full_name","entity_name":"representation.text.entity_name",
  "iata_code":"geography.transportation.iata_code"}  # word/RESIDUAL kept from corpus (representative)
JUNE_LABELS=set(FAM2LABEL.values())
RESIDUAL={"representation.text.word","representation.text.plain_text"}
SPEC_CAP=800

t=pq.read_table("/tmp/cs_train_raw.parquet")
cn=t.column("column_name").to_pylist(); lbl=t.column("lbl").to_pylist(); sv=t.column("sample_values_truncated").to_pylist()
corp=dict()
for i in range(len(cn)):
    corp.setdefault(lbl[i],[])
    vals=", ".join([v for v in (sv[i] or '').split(SEP) if v.strip()][:8])
    corp[lbl[i]].append(f"header: {cn[i]} | values: {vals}")

by=dict()
# specific corpus labels (not residual, not June-replaced) capped
for k,v in corp.items():
    if k in RESIDUAL or k in JUNE_LABELS: continue
    by[k]=v[:SPEC_CAP]
# June clean data for the 7 semantic families
jc=dict()
for r in csv.DictReader(open("output/fine-tuned-encoder-discovery/encoder_train_v2.tsv"),delimiter="\t"):
    full=FAM2LABEL.get(r["label"])
    if full: jc.setdefault(full,[]).append(r["text"])
for k,v in jc.items():
    random.shuffle(v); by[k]=v[:1200]
# residual to ~25% of total
S=sum(len(v) for v in by.values())
resid_target=round(S/3)
res=[]
for k in RESIDUAL:
    res+= [(txt,k) for txt in corp.get(k,[])]
random.shuffle(res); res=res[:resid_target]
print(f"specific+semantic: {S}, residual target {resid_target}, residual available {sum(len(corp.get(k,[])) for k in RESIDUAL)}")

rows=[(txt,k) for k,txts in by.items() for txt in txts]+res
random.shuffle(rows)
with open("/tmp/cs_train.tsv","w",newline="") as f:
    w=csv.writer(f,delimiter="\t",lineterminator="\n"); w.writerow(["text","label"])
    for txt,k in rows: w.writerow([txt,k])
nres=sum(1 for _,k in rows if k in RESIDUAL)
print(f"wrote {len(rows)} rows, {len(set(k for _,k in rows))} labels, residual {nres}/{len(rows)} = {nres/len(rows):.0%}")
