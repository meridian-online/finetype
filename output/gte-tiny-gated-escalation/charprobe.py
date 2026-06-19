"""Does gte-tiny + CHEAP char/stats features close the structural-ID gap (making the char-CNN
unnecessary)? Compare linear probe: gte-tiny alone vs gte-tiny + hand-crafted features."""
import numpy as np, pyarrow.parquet as pq, re
from sentence_transformers import SentenceTransformer
from sklearn.linear_model import LogisticRegression
from sklearn.model_selection import train_test_split
from collections import defaultdict
SEP="│"
t=pq.read_table("/tmp/probe_sample.parquet")
cn=t.column("column_name").to_pylist(); y=np.array(t.column("sense_prediction").to_pylist()); sv=t.column("sample_values_truncated").to_pylist()

def feats(vals):
    vals=[v for v in vals if v.strip()][:8]
    if not vals: return [0]*12
    joined="".join(vals); L=max(len(joined),1)
    nd=len(set(vals))/len(vals)
    lens=[len(v) for v in vals]
    digit=sum(c.isdigit() for c in joined)/L
    up=sum(c.isupper() for c in joined)/L
    lo=sum(c.islower() for c in joined)/L
    punct=sum(not c.isalnum() and not c.isspace() for c in joined)/L
    space=sum(c.isspace() for c in joined)/L
    has_delim=float(any(d in joined for d in "-_/.:"))
    pure_num=sum(bool(re.fullmatch(r'-?\d+(\.\d+)?',v)) for v in vals)/len(vals)
    alnum_mix=sum(bool(re.search(r'[A-Za-z]',v) and re.search(r'\d',v)) for v in vals)/len(vals)
    return [nd, np.mean(lens), np.std(lens), digit, up, lo, punct, space, has_delim, pure_num, alnum_mix, len(vals)]

texts=[f"header: {cn[i]} | values: "+", ".join([v for v in (sv[i] or '').split(SEP) if v.strip()][:8]) for i in range(len(cn))]
F=np.array([feats((sv[i] or '').split(SEP)) for i in range(len(cn))])
F=(F-F.mean(0))/(F.std(0)+1e-6)
print(f"embedding {len(texts)} cols...")
m=SentenceTransformer("TaylorAI/gte-tiny", device="mps")
E=m.encode(texts, batch_size=256, show_progress_bar=False)

idx=np.arange(len(y)); itr,ite=train_test_split(idx,test_size=0.3,random_state=42,stratify=y)
def probe(X,tag):
    clf=LogisticRegression(max_iter=2000,C=10).fit(X[itr],y[itr]); pred=clf.predict(X[ite])
    overall=(pred==y[ite]).mean()
    bt=defaultdict(lambda:[0,0])
    for p,t_ in zip(pred,y[ite]): bt[t_][0]+=int(p==t_); bt[t_][1]+=1
    return overall, {k:(v[0]/v[1],v[1]) for k,v in bt.items()}, pred

oE,btE,_=probe(E,"gte-only")
oEF,btEF,_=probe(np.hstack([E,F]),"gte+feats")
print(f"\nOVERALL: gte-only {oE:.3f}  ->  gte+feats {oEF:.3f}")
print(f"\n{'type':42} {'gte-only':>9} {'gte+feats':>10} {'delta':>7}")
for t_ in ["representation.identifier.alphanumeric_id","representation.identifier.increment",
           "representation.identifier.numeric_code","representation.identifier.uuid",
           "representation.numeric.integer_number","representation.discrete.categorical",
           "technology.cryptographic.hash","representation.file.extension"]:
    if t_ in btE:
        a,_=btE[t_]; b,n=btEF[t_]; print(f"  {t_.split('.',1)[1]:40} {a:>9.2f} {b:>10.2f} {b-a:>+7.2f}")
print("DONE")
