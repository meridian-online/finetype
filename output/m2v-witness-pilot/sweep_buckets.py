"""Sweep the m2v->GBT one-vs-rest abstaining witness across every gold bucket with
>=8 gold cols. For each: AUC + the abstaining-veto regime (max recall at precision
>=0.90/0.95). Tag value-determined (Sharpen owns it; witness redundant) vs semantic
residual (witness is the unique lever). Spec 2026-06-27-m2v-witness-specialiser-pilot.

Learner: sklearn HistGradientBoosting (histogram GBT) — ~15x faster than ydf on 1024
dense features, same separability signal. Cross-checked on integer_number vs the ydf
anchor (ydf AUC 0.886): a separability probe is about the embedding's carve-ability,
not the GBT impl."""
import numpy as np, time, collections, sys
from sklearn.ensemble import HistGradientBoostingClassifier
from sklearn.metrics import roc_auc_score, precision_recall_curve

tr = np.load("output/m2v-witness-pilot/train_emb.npz", allow_pickle=True)
Xtr, ytr = tr['X'], tr['y']
go = np.load("output/m2v-witness-pilot/gold_labeled.npz", allow_pickle=True)
Xg, yg = go['X'], go['y']

# Sharpen already owns these at high precision (closed sets / validators / value shape) ->
# a witness there is redundant. The rest are the SEMANTIC residual where value-rules can't carve.
VALUE_DETERMINED = {
    "representation.numeric.integer_number", "representation.numeric.decimal_number",
    "datetime.date.iso", "datetime.component.year", "datetime.epoch.unix_seconds",
    "datetime.timestamp.sql_standard", "geography.coordinate.longitude",
    "geography.coordinate.latitude", "geography.location.country_code",
    "identity.commerce.isbn", "technology.internet.url", "representation.boolean.terms",
}

gc = collections.Counter(yg)
buckets = [lab for lab, n in gc.most_common() if n >= 8]

rows = []
for lab in buckets:
    ytr_b = (ytr == lab).astype(int)
    yg_b = (yg == lab).astype(int)
    t0 = time.time()
    m = HistGradientBoostingClassifier(max_iter=300, learning_rate=0.1,
                                       max_leaf_nodes=31, validation_fraction=0.1,
                                       early_stopping=True, random_state=42)
    m.fit(Xtr, ytr_b)
    p = m.predict_proba(Xg)[:, 1]
    auc = roc_auc_score(yg_b, p)
    prec, rec, _ = precision_recall_curve(yg_b, p)
    def rec_at(bar):
        ok = [re for pr, re in zip(prec, rec) if pr >= bar]
        return max(ok, default=0.0)
    r90, r95 = rec_at(0.90), rec_at(0.95)
    kind = "value" if lab in VALUE_DETERMINED else "SEMANTIC"
    rows.append((lab, kind, gc[lab], int(ytr_b.sum()), auc, r90, r95))
    print(f"  {lab:43s} {kind:8s} AUC {auc:.3f}  R@.90 {r90:.3f}  R@.95 {r95:.3f}  ({time.time()-t0:.0f}s)",
          file=sys.stderr, flush=True)

rows.sort(key=lambda r: r[5], reverse=True)  # by abstaining-veto recall@.90
print("# m2v->GBT witness sweep across gold buckets (sklearn HistGBT)\n")
print("Sorted by abstaining-veto recall (max recall at precision>=0.90). "
      "value = Sharpen already owns it (witness redundant); "
      "SEMANTIC = the residual where the witness would be the unique lever.\n")
print(f"| {'bucket':43s} | kind | gold | train | AUC | R@P.90 | R@P.95 |")
print(f"|{'-'*45}|------|-----:|------:|----:|------:|------:|")
for lab, kind, g, t, auc, r90, r95 in rows:
    print(f"| {lab:43s} | {kind} | {g} | {t} | {auc:.3f} | {r90:.3f} | {r95:.3f} |")
