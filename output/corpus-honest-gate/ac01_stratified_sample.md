# ac-01 — stratified corpus sample

Spec `2026-06-07-corpus-honest-quality-gate`, ac-01. Builds the fixed file list the
honest gate scores against, oversampling rare labels so a 0.13% label is legible to
a 3× move. Tool: `scripts/build_stratified_sample.py` (per-label file quota,
`--tmin 400 --rare-floor 40000`). Source: v19 corpus pass
(`eval/gittables/corpus_pass/columns.parquet`, 6,590,432 cols / 505,708 files).

## Headline — small *and* legible

| | files | columns |
|---|---:|---:|
| full corpus | 505,708 | 6,590,432 |
| **stratified sample** | **33,250 (6.57%)** | **855,917 (12.99%)** |

Under the ≤15% budget. The acceptance bar is the resolution table, not the size —
small is only worth it if latitude is still legible. It is.

## Resolution — every label resolves a 3× shift, or is captured whole

243 labels. **0 unresolved.** Every label is either ≥ 400 columns in the sample
(the quota) or 100% captured (81 labels whose entire corpus population is < 400
columns — the honest ceiling: you cannot measure a 3× move on a label with 7
columns anywhere, and we took all 7).

The labels the four known failures turn on — all richly present:

| label | full | sample | sample rate | the failure it gates |
|---|---:|---:|---:|---|
| geography.coordinate.latitude | 7,974 | **2,213** | 0.278 | latdec relocation |
| geography.coordinate.longitude | 10,200 | 1,493 | 0.146 | latdec relocation |
| representation.numeric.decimal_number | 1,900,526 | 217,907 | 0.115 | latdec source label |
| representation.discrete.categorical | 67,864 | 13,358 | 0.197 | v23 +529% explosion |
| geography.location.city | 38,312 | 6,616 | 0.173 | v22 −10.2% |
| geography.location.region | 20,310 | 4,233 | 0.208 | v22 −12.8% |
| geography.location.country_code | 6,196 | 2,399 | 0.387 | v22 country collapse |
| technology.internet.top_level_domain | 87,542 | 1,315 | 0.015 | latdec incidental win |

**latitude lands 2,213 columns in the sample. The destination-drift proxy that
missed the relocation landed ~18.** Same corpus, ~120× the resolution on the exact
label that fooled every curated instrument — because the rare labels are quota'd up
rather than left to fall where a uniform sample drops them.

## Deliverables

- `output/corpus-honest-gate/stratified_sample.files.txt.gz` — the fixed file list
  (feed to `snapshot_sense_distribution.py --file-list` and the gate scorer).
- `output/corpus-honest-gate/stratified_sample.resolution.json` — the full
  243-label resolution table + summary.
- `scripts/build_stratified_sample.py` — regenerates both from any baseline parquet.

## Next

ac-02 builds the oracle-honest FP scorer (counts the ydf=None relocation the latdec
metric drove to zero); ac-03 runs sample + scorer against all four labelled parquets
to prove the gate reproduces every known verdict.
