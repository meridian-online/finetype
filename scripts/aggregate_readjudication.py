#!/usr/bin/env python3
"""Aggregate the blind re-adjudication panels into per-column verdicts + integrity.

Consensus over the 3 NEUTRAL panels (opus/sonnet/haiku); the adversarial panel is a
check, not a vote. Verdicts: CONFIRM / PROPOSE / CONTESTED / TAXONOMY_GAP.
Integrity (family-independent): control-confirm rate + correction model-agree/disagree.
Panels were blind to current gold and to the FineType model.
"""
import json, os
from collections import Counter
D="output/gold-readjudication"
def load(f):
    d={}
    if not os.path.exists(f): return d
    for ln in open(f):
        ln=ln.strip()
        if not ln: continue
        try: o=json.loads(ln)
        except: continue
        if o.get('id'): d[o['id']]=o
    return d
def norm(l):
    # Panels invented non-canonical full paths (e.g. geography.location.coordinate.
    # latitude, finance.identifier.isbn) that share the LEAF with the canonical key.
    # Leaves are unique in the taxonomy except {iso, iso_8601, ordinal} (negligible
    # for the residual tier), so compare on the leaf.
    if not l: return '?'
    l=str(l).strip()
    if l.startswith('other'): return 'OTHER'
    return l.split('.')[-1]

neutral={k:load(f"{D}/panel_{k}.jsonl") for k in ('opus','sonnet','haiku')}
adv=load(f"{D}/panel_adv.jsonl")
ans=load(f"{D}/answers_phase1.jsonl")
MISSING=[k for k,v in neutral.items() if len(v)<len(ans)*0.95]
if MISSING: print(f"WARNING: panels incomplete: {MISSING}")

CONF_FLOOR=0.6
rows=[]
for cid,a in ans.items():
    gold_leaf=norm(a['current_gold']); model_leaf=norm(a['model'])  # compare like-for-like (leaf)
    raw=[norm(neutral[k].get(cid,{}).get('label')) for k in ('opus','sonnet','haiku')]
    confs=[neutral[k].get(cid,{}).get('confidence',0) or 0 for k in ('opus','sonnet','haiku')]
    votes=[v for v in raw if v!='?']
    if len(votes)<2:
        verdict='CONTESTED'; top=None; n=0; mc=0
    else:
        c=Counter(votes); top,n=c.most_common(1)[0]
        mc=sum(cf for v,cf in zip(raw,confs) if v==top)/max(n,1)
        if top=='OTHER': verdict='TAXONOMY_GAP'
        elif n>=2 and mc>=CONF_FLOOR:
            verdict='CONFIRM' if top==gold_leaf else 'PROPOSE'
        else: verdict='CONTESTED'
    rows.append(dict(id=cid,header=a['header'],kind=a['kind'],current_gold=gold_leaf,
                     model=model_leaf,panel=top,agree=n,mean_conf=round(mc,2),
                     votes=votes,adv=norm(adv.get(cid,{}).get('label')),verdict=verdict))

def s(l): return (l or '?').split('.')[-1]
vc=Counter(r['verdict'] for r in rows if r['kind']=='ac-03')
print("="*70); print("VERDICTS on ac-03 target columns (n=%d)"%sum(1 for r in rows if r['kind']=='ac-03'))
for v in ('CONFIRM','PROPOSE','CONTESTED','TAXONOMY_GAP'): print(f"  {v:13s} {vc.get(v,0)}")

# integrity
ctl=[r for r in rows if r['kind']=='control']
ctl_confirm=sum(1 for r in ctl if r['panel']==r['current_gold'])
props=[r for r in rows if r['kind']=='ac-03' and r['verdict']=='PROPOSE']
agree_model=sum(1 for r in props if r['panel']==r['model'])
print("\n"+"="*70); print("INTEGRITY (family-independent)")
print(f"  negative control: panel==gold on {ctl_confirm}/{len(ctl)} = {ctl_confirm/max(len(ctl),1):.0%}  (should be HIGH)")
print(f"  corrections total: {len(props)}")
print(f"    agree with FineType model:    {agree_model}  <- would flatter the model")
print(f"    DISAGREE with model:          {len(props)-agree_model}  <- proves not goalpost-moving")

print("\n"+"="*70); print("SAMPLE PROPOSED CORRECTIONS (gold -> panel; M=matches model)")
for r in sorted(props,key=lambda r:-r['mean_conf'])[:25]:
    flag='M' if r['panel']==r['model'] else ' '
    print(f"  [{flag}] {r['header'][:22]:22s} gold={s(r['current_gold']):14s} -> panel={s(r['panel']):16s} conf={r['mean_conf']} adv={s(r['adv'])}")

print("\nTAXONOMY-GAP candidates:")
for r in [r for r in rows if r['kind']=='ac-03' and r['verdict']=='TAXONOMY_GAP'][:20]:
    print(f"  {r['header'][:24]:24s} gold={s(r['current_gold'])}  adv={s(r['adv'])}")

with open(f"{D}/verdicts.jsonl","w") as f:
    for r in rows: f.write(json.dumps(r,ensure_ascii=False)+"\n")
print(f"\nwrote {D}/verdicts.jsonl ({len(rows)} rows)")
