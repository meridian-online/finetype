"""Build the gte-tiny escalation candidate for the corpus-honest gate: override v19's
prediction on every CONTESTED-leaf column in the stratified sample with the encoder's call.
(sense_confidence is null in v19_gated, so we can't low-band-gate here — escalating ALL
contested columns is a STRICTER relocation test; the low-band design relocates <= this.)
Encoder = gte-tiny frozen + natural LR head on the region-fixed training set (holds residual).
"""
import csv
import numpy as np
import pyarrow as pa, pyarrow.parquet as pq
from sentence_transformers import SentenceTransformer
from sklearn.linear_model import LogisticRegression

CONTESTED = {"country","country_code","city","region","full_name","entity_name","iata_code","categorical","word","plain_text"}
FAM2LABEL = {"RESIDUAL":"representation.text.word","country":"geography.location.country",
             "country_code":"geography.location.country_code","city":"geography.location.city",
             "region":"geography.location.region","full_name":"identity.person.full_name",
             "entity_name":"representation.text.entity_name","iata_code":"geography.transportation.iata_code"}
SEP="│"

# --- train encoder head ---
def load(p,t,l): r=list(csv.DictReader(open(p),delimiter="\t")); return [x[t] for x in r],[x[l] for x in r]
Xtr_t,ytr=load("output/fine-tuned-encoder-discovery/encoder_train_v2.tsv","text","label")
m=SentenceTransformer("TaylorAI/gte-tiny",device="mps")
clf=LogisticRegression(max_iter=3000).fit(m.encode(Xtr_t,show_progress_bar=False,batch_size=256), np.array(ytr))

# --- load v19_gated sample rows ---
sample=set(l.strip() for l in open("output/corpus-honest-gate/stratified_sample.files.txt") if l.strip())
t=pq.read_table("output/ydf-validation-gate/v19_gated.parquet",
   columns=["file_path","column_name","sense_prediction","sample_values_truncated","is_trivial"])
fp=t.column("file_path").to_pylist(); cn=t.column("column_name").to_pylist()
sp=t.column("sense_prediction").to_pylist(); sv=t.column("sample_values_truncated").to_pylist()
tv=t.column("is_trivial").to_pylist()
idx=[i for i in range(len(fp)) if fp[i] in sample]
print(f"sample rows: {len(idx)}")

# contested rows to re-classify
esc=[i for i in idx if not tv[i] and (sp[i] or "").split(".")[-1] in CONTESTED and (sv[i] or "").strip()]
print(f"contested columns to escalate: {len(esc)}")
texts=[f"header: {cn[i]} | values: " + ", ".join([v for v in (sv[i] or '').split(SEP) if v.strip()][:8]) for i in esc]
emb=m.encode(texts, show_progress_bar=False, batch_size=512)
proba=clf.predict_proba(emb); fams=clf.classes_[proba.argmax(1)]; conf=proba.max(1)
override={esc[k]: FAM2LABEL[fams[k]] for k in range(len(esc)) if conf[k]>0.90}
changed=sum(1 for i in esc if override[i]!=sp[i])
print(f"escalation changed {changed}/{len(esc)} contested predictions")

# write candidate (all sample rows; contested overridden, else v19)
out_fp=[fp[i] for i in idx]; out_cn=[cn[i] for i in idx]
out_sp=[override.get(i, sp[i]) for i in idx]
pq.write_table(pa.table({"file_path":out_fp,"column_name":out_cn,"sense_prediction":out_sp}),
               "output/fine-tuned-encoder-discovery/escalation_candidate.parquet")
print("wrote escalation_candidate.parquet")
