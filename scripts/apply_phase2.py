#!/usr/bin/env python3
"""Apply phase-2 re-adjudication to eval/gold/gold_corpus.tsv.

9 PROPOSE corrections (all datetime.date.iso -> datetime.timestamp.sql_standard) +
186 CONFIRM provenance upgrades to author-verified. Robust (sha,column)-key apply:
replay the phase-1 + phase-2 blind builds on the PRISTINE pre-phase-1 gold
(/tmp/gold_orig.tsv from git) to recover each id's (sha,column), then apply to the
current gold by key. Append-only: old label kept in provenance.
"""
import csv, os, json, duckdb, random
DATASETS="/Users/hugh/datasets/"; REPO=os.getcwd()
con=duckdb.connect()
canon=json.load(open('models/default/label_map.json')); canon=list(canon.keys() if isinstance(canon,dict) else canon)
leaf2key={}
for k in canon: leaf2key.setdefault(k.split('.')[-1],k)

orig=list(csv.DictReader(open('/tmp/gold_orig.tsv'),delimiter='\t'))
def tier(p):
    if p.startswith('ac-04') or 'two-panel' in p or 'adjudication' in p: return 'strong'
    if p.startswith('ac-03'): return 'ac-03'
    if p.startswith('header '): return 'header-heuristic'
    return 'other'
def resolve(fp):
    for c in (os.path.join(DATASETS,fp), os.path.join(REPO,fp)):
        if os.path.exists(c): return c
def rel(p): return f"read_parquet('{p}')" if p.endswith('.parquet') else f"read_csv_auto('{p}',SAMPLE_SIZE=200,ignore_errors=true)"
def has_vals(fp,col,n=25):
    p=resolve(fp)
    if not p: return None
    try:
        cols=[c[0] for c in con.execute(f"DESCRIBE SELECT * FROM {rel(p)}").fetchall()]
        cn=col if col in cols else next((c for c in cols if str(c).strip().lower()==col.strip().lower()),None)
        if not cn: return None
        q=cn.replace('"','""')
        r=con.execute(f'SELECT DISTINCT "{q}" FROM {rel(p)} WHERE "{q}" IS NOT NULL LIMIT {n}').fetchall()
        return len(r)
    except Exception: return None

def key(r): return (r['file_content_sha256'], r['column_name'])

# --- replay phase-1: ac-03 + control(seed42) -> g#### -> key ---
random.seed(42)
ac03=[r for r in orig if tier(r['provenance'])=='ac-03']
strong=[r for r in orig if tier(r['provenance'])=='strong']
ctl1=random.sample(strong,40)
p1={}; i=0
for kind,rows in [('ac-03',ac03),('control',ctl1)]:
    for r in rows:
        n=has_vals(r['file_path'],r['column_name'])
        if not n or n<3: continue
        p1[f"g{i:04d}"]=key(r); i+=1
# --- replay phase-2: header-heuristic + control(seed43) -> p#### -> key ---
random.seed(43)
hdr=[r for r in orig if tier(r['provenance'])=='header-heuristic']
ctl2=random.sample(strong,40)
p2={}; j=0
for kind,rows in [('header-heuristic',hdr),('control',ctl2)]:
    for r in rows:
        n=has_vals(r['file_path'],r['column_name'])
        if not n or n<3: continue
        p2[f"p{j:04d}"]=key(r); j+=1
start=j
# datetime-rerun: p(start+k) -> dt_ids[k] (phase-1 ids) -> p1 key
v1verd=[json.loads(l) for l in open('output/gold-readjudication/verdicts.jsonl')]
DT={'iso','sql_standard','calendar_date','date','datetime','timestamp'}
dt_ids=[r['id'] for r in v1verd if r['kind']=='ac-03' and r['verdict']=='CONTESTED' and r['current_gold'] in DT]
for k,gid in enumerate(dt_ids):
    p2[f"p{start+k:04d}"]=p1[gid]

# verify against blind_phase2 headers
blind={json.loads(l)['id']:json.loads(l)['header'] for l in open('output/gold-readjudication/blind_phase2.jsonl')}
mism=[cid for cid,h in blind.items() if p2.get(cid,('',''))[1]!=h]
assert not mism, f"id->key mapping mismatch on {len(mism)}: {mism[:5]}"
print(f"phase-2 id->key verified ({len(p2)} ids match blind headers)")

# --- apply to current gold_corpus.tsv by key ---
cur=list(csv.DictReader(open('eval/gold/gold_corpus.tsv'),delimiter='\t'))
bykey={key(r):r for r in cur}
verd={json.loads(l)['id']:json.loads(l) for l in open('output/gold-readjudication/verdicts_phase2.jsonl')}
changed=[]; upgraded=0
for cid,v in verd.items():
    if v['kind'] not in ('header-heuristic','datetime-rerun'): continue
    if cid not in p2: continue
    row=bykey.get(p2[cid])
    if not row: continue
    if v['verdict']=='PROPOSE':
        newkey=leaf2key[v['panel']]; old=row['curated_label']; op=row['provenance'][:40]
        row['curated_label']=newkey
        row['provenance']=f"mixed-panel re-adjudication 2026-06-16 phase2 (author-accepted; was {old} via {op}); taxonomy v0.6.31"
        changed.append((row['column_name'],old,newkey,v['mean_conf']))
    elif v['verdict']=='CONFIRM':
        row['provenance']=f"author-verified 2026-06-16 phase2 (confirmed; was {row['provenance'][:40]}); taxonomy v0.6.31"
        upgraded+=1

with open('eval/gold/gold_corpus.tsv','w',newline='') as f:
    w=csv.DictWriter(f,fieldnames=list(cur[0].keys()),delimiter='\t'); w.writeheader(); w.writerows(cur)
print(f"\napplied {len(changed)} corrections, {upgraded} provenance upgrades")
for col,old,new,c in changed:
    print(f"  {col[:24]:24s} {old.split('.')[-1]} -> {new.split('.')[-1]} (conf {c})")
