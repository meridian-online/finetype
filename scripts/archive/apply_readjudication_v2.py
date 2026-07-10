#!/usr/bin/env python3
"""Apply the accepted re-adjudication corrections -> eval/gold/gold_corpus_v2.tsv.

Recommended set (author-approved): PROPOSE verdicts that are NOT debatable
transitions (numeric->alphanumeric_id, TLD->categorical) and clear conf>=0.75.
Append-only per choice 0095: the old label is preserved in the provenance string;
gold_corpus_v1.tsv stays as the historical record. Datetime vocab-noise (CONTESTED)
is untouched. Verifies id->row mapping against the original blind build before writing.
"""
import csv, os, json, duckdb, random
random.seed(42)
DATASETS="/Users/hugh/datasets/"; REPO=os.getcwd()
con=duckdb.connect()
CONF=0.75
DEBATABLE={('top_level_domain','categorical'),('decimal_number','alphanumeric_id'),
           ('integer_number','alphanumeric_id')}

canon=json.load(open('models/default/label_map.json'))
canon=list(canon.keys()) if isinstance(canon,dict) else canon
leaf2key={}
for k in canon: leaf2key.setdefault(k.split('.')[-1],k)

gold=list(csv.DictReader(open('eval/gold/gold_corpus_v1.tsv'),delimiter='\t'))
def tier(p):
    if p.startswith('ac-04') or 'two-panel' in p or 'adjudication' in p: return 'strong'
    if p.startswith('ac-03'): return 'ac-03'
    if p.startswith('header '): return 'header-heuristic'
    return 'other'
def resolve(fp):
    for c in (os.path.join(DATASETS,fp), os.path.join(REPO,fp)):
        if os.path.exists(c): return c
    return None
def rel(p): return f"read_parquet('{p}')" if p.endswith('.parquet') else f"read_csv_auto('{p}',SAMPLE_SIZE=200,ignore_errors=true)"
def nvals(fp,col,n=25):
    p=resolve(fp)
    if not p: return None
    try:
        cols=[c[0] for c in con.execute(f"DESCRIBE SELECT * FROM {rel(p)}").fetchall()]
        cn=col if col in cols else next((c for c in cols if str(c).strip().lower()==col.strip().lower()),None)
        if not cn: return None
        q=cn.replace('"','""')
        rows=con.execute(f'SELECT DISTINCT "{q}" FROM {rel(p)} WHERE "{q}" IS NOT NULL LIMIT {n}').fetchall()
        return [str(r[0]) for r in rows]
    except Exception: return None

# replay the EXACT blind-build id assignment to map id -> gold row index
ac03=[i for i,r in enumerate(gold) if tier(r['provenance'])=='ac-03']
strong=[i for i,r in enumerate(gold) if tier(r['provenance'])=='strong']
control=random.sample(strong,40)
id2idx={}; i=0
for kind,idxs in [('ac-03',ac03),('control',control)]:
    for gi in idxs:
        r=gold[gi]; v=nvals(r['file_path'],r['column_name'])
        if not v or len(v)<3: continue
        id2idx[f"g{i:04d}"]=gi; i+=1

# integrity: reproduced id->header must match the blind file exactly
blind={json.loads(l)['id']:json.loads(l)['header'] for l in open('output/gold-readjudication/blind_phase1.jsonl')}
mism=[cid for cid,h in blind.items() if gold[id2idx.get(cid,-1)]['column_name']!=h]
assert not mism, f"id->row mapping mismatch on {len(mism)} ids — ABORT: {mism[:5]}"
print(f"id->row mapping verified ({len(id2idx)} ids match blind headers)")

verds={json.loads(l)['id']:json.loads(l) for l in open('output/gold-readjudication/verdicts.jsonl')}
accept=[(cid,v) for cid,v in verds.items()
        if v['kind']=='ac-03' and v['verdict']=='PROPOSE' and v['mean_conf']>=CONF
        and (v['current_gold'],v['panel']) not in DEBATABLE]

applied=[]
for cid,v in accept:
    gi=id2idx[cid]; row=gold[gi]
    newkey=leaf2key[v['panel']]
    old=row['curated_label']; oldprov=row['provenance'][:40]
    row['curated_label']=newkey
    row['provenance']=f"mixed-teacher re-adjudication 2026-06-16 (panels opus/sonnet/haiku, conf {v['mean_conf']}; was {old} via {oldprov}); taxonomy v0.6.31"
    applied.append((row['column_name'],old,newkey,v['mean_conf']))

out='eval/gold/gold_corpus_v2.tsv'
with open(out,'w',newline='') as f:
    w=csv.DictWriter(f,fieldnames=list(gold[0].keys()),delimiter='\t')
    w.writeheader(); w.writerows(gold)
print(f"\nwrote {out} ({len(gold)} rows, {len(applied)} corrections applied)\n")
print("=== APPLIED corrections ===")
for col,old,new,c in applied:
    print(f"  {col[:26]:26s} {old.split('.')[-1]:14s} -> {new.split('.')[-1]:18s} (conf {c})")
