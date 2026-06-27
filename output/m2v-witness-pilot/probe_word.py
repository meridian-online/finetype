import numpy as np, pandas as pd, ydf, time
from sklearn.metrics import roc_auc_score, precision_recall_curve
TARGET="representation.text.word"
tr=np.load("output/m2v-witness-pilot/train_emb.npz",allow_pickle=True); Xtr,ytr=tr['X'],tr['y']
go=np.load("output/m2v-witness-pilot/gold_labeled.npz",allow_pickle=True); Xg,yg=go['X'],go['y']
cols=[f"f{i}" for i in range(Xtr.shape[1])]
def mk(X,y): d=pd.DataFrame(X,columns=cols); d["y"]=(y==TARGET).astype(int); return d
print(f"train pos {(ytr==TARGET).sum()}/{len(ytr)} | gold pos {(yg==TARGET).sum()}/{len(yg)}")
t0=time.time()
m=ydf.GradientBoostedTreesLearner(label="y",task=ydf.Task.CLASSIFICATION,num_trees=200).train(mk(Xtr,ytr))
print(f"ydf GBT trained in {time.time()-t0:.0f}s")
p=m.predict(mk(Xg,yg))  # P(positive)
yt=(yg==TARGET).astype(int)
auc=roc_auc_score(yt,p)
prec,rec,thr=precision_recall_curve(yt,p)
print(f"\n=== m2v->ydf word-witness on gold (AUC {auc:.3f}) ===")
# precision-at-recall, and the abstaining regime: max recall at precision >= bar
for bar in (0.95,0.90,0.80):
    ok=[(pr,re) for pr,re in zip(prec,rec) if pr>=bar]
    best=max((re for pr,re in ok), default=0.0)
    print(f"  precision>={bar:.2f} achievable at recall up to {best:.3f}  (= the abstaining-veto regime)")
# where does the model itself currently sit? (raw Sense recall on word ~0.60 per CLAUDE)
print(f"\n  NOTE: shipped pipeline word recall ~0.60; the witness must beat THAT in its assert region to add value.")
