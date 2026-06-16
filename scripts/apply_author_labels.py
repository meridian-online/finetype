#!/usr/bin/env python3
"""Apply author gate decisions on the 24 deferred proposals to gold_corpus_v2.tsv.

Author labels are AUTHORITATIVE. Where author_label == current label → CONFIRM
(upgrade provisional/heuristic provenance to author-verified, no label change).
Where it differs → author OVERRIDE (change label). Append-only: old label kept in
provenance. id->row mapping replayed on v1 (v2 provenance no longer carries the
ac-03 tier) and asserted against the blind build.
"""
import csv, os, json, duckdb, random
random.seed(42)
DATASETS="/Users/hugh/datasets/"; REPO=os.getcwd()
con=duckdb.connect()

auth={r['id']:r['author_label'].strip()
      for r in csv.DictReader(open('output/gold-readjudication/author_labels.tsv'),delimiter='\t')}
canon=json.load(open('models/default/label_map.json')); canon=set(canon.keys() if isinstance(canon,dict) else canon)
assert all(l in canon for l in auth.values()), "non-canonical author label"

v1=list(csv.DictReader(open('eval/gold/gold_corpus_v1.tsv'),delimiter='\t'))
v2=list(csv.DictReader(open('eval/gold/gold_corpus_v2.tsv'),delimiter='\t'))  # same order/membership
def tier(p):
    if p.startswith('ac-04') or 'two-panel' in p or 'adjudication' in p: return 'strong'
    if p.startswith('ac-03'): return 'ac-03'
    if p.startswith('header '): return 'header-heuristic'
    return 'other'
def resolve(fp):
    for c in (os.path.join(DATASETS,fp), os.path.join(REPO,fp)):
        if os.path.exists(c): return c
def rel(p): return f"read_parquet('{p}')" if p.endswith('.parquet') else f"read_csv_auto('{p}',SAMPLE_SIZE=200,ignore_errors=true)"
def nvals(fp,col,n=25):
    p=resolve(fp)
    if not p: return None
    try:
        cols=[c[0] for c in con.execute(f"DESCRIBE SELECT * FROM {rel(p)}").fetchall()]
        cn=col if col in cols else next((c for c in cols if str(c).strip().lower()==col.strip().lower()),None)
        if not cn: return None
        q=cn.replace('"','""')
        r=con.execute(f'SELECT DISTINCT "{q}" FROM {rel(p)} WHERE "{q}" IS NOT NULL LIMIT {n}').fetchall()
        return [str(x[0]) for x in r]
    except Exception: return None

ac03=[i for i,r in enumerate(v1) if tier(r['provenance'])=='ac-03']
strong=[i for i,r in enumerate(v1) if tier(r['provenance'])=='strong']
control=random.sample(strong,40)
id2idx={}; i=0
for kind,idxs in [('ac-03',ac03),('control',control)]:
    for gi in idxs:
        r=v1[gi]; vv=nvals(r['file_path'],r['column_name'])
        if not vv or len(vv)<3: continue
        id2idx[f"g{i:04d}"]=gi; i+=1
blind={json.loads(l)['id']:json.loads(l)['header'] for l in open('output/gold-readjudication/blind_phase1.jsonl')}
assert all(v1[id2idx[c]]['column_name']==h for c,h in blind.items()), "id->row mapping mismatch"
print(f"id->row mapping verified ({len(id2idx)} ids)")

changed=[]; confirmed=[]
for cid,al in auth.items():
    gi=id2idx[cid]; row=v2[gi]; old=row['curated_label']; oldprov=row['provenance'][:40]
    if al!=old:
        row['curated_label']=al
        row['provenance']=f"author adjudication 2026-06-16 (override; was {old} via {oldprov}); taxonomy v0.6.31"
        changed.append((row['column_name'],old,al))
    else:
        row['provenance']=f"author adjudication 2026-06-16 (confirmed; was {oldprov}); taxonomy v0.6.31"
        confirmed.append(row['column_name'])

with open('eval/gold/gold_corpus_v2.tsv','w',newline='') as f:
    w=csv.DictWriter(f,fieldnames=list(v2[0].keys()),delimiter='\t'); w.writeheader(); w.writerows(v2)
print(f"\nauthor decisions: {len(changed)} OVERRIDE, {len(confirmed)} CONFIRM (provenance upgraded to author-verified)")
for col,old,new in changed: print(f"  OVERRIDE  {col}: {old.split('.')[-1]} -> {new.split('.')[-1]}")
