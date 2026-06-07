# ac-03 — the gate, trust by reproduction

Spec `2026-06-07-corpus-honest-quality-gate`, ac-03 (the blocking gate). Runs the
ac-01 stratified sample + ac-02 oracle-honest scorer against the four labelled
corpus-scale outcomes on disk and confirms it re-derives every verdict **from the
sample alone** — no 9-hour full pass. Oracle: v19 gated YDF
(`output/ydf-validation-gate/v19_gated.parquet`).

## All four reproduce

| candidate | known verdict | gate | fired on | load-bearing mover |
|---|---|---|---|---|
| **v19 vs itself** | GO (no false alarm) | **GO** | — | zero moves → zero net contra by construction |
| **latdec** | NO-GO (latitude relocation) | **NO-GO** | latitude `oracle_fp` | latitude net **+2,481** new oracle-refuted FPs |
| **v22** | NO-GO (geography) | **NO-GO** | city/region/country | city **3.06×** over_emit, region/country `oracle_fp` |
| **v23** | NO-GO (categorical +529%) | **NO-GO** | categorical/latitude | categorical **8.68×**, latitude **8.75×** over_emit |

The verdict the curated stack missed — latdec's latitude relocation — is the one the
gate lands on: `oracle_fp`, **2,481** scaled net new false positives on columns the
gated oracle calls decimal. The gold anchor (240 cols), m-19 (448 rows) and the drift
proxy (~18 latitude cols) all cleared it. The gate sees it because the stratified
sample lands **446 observed** `→latitude` contradictions and scales them honestly.

Each known failure trips its **own** signature band:
- v23 categorical explosion → `over_emit` (8.68×).
- v22 geography over-emission → `over_emit` + `oracle_fp` (v22 predicts *more*
  geography, but wrong — the "collapse" in the original review is a correctness-cell
  collapse, which surfaces here as contradicted over-emission).
- latdec rare-label relocation → `oracle_fp` at stable marginal (ratio ≈ 1.0): the
  members swapped (real latitude out, feature-floats in) while the count held — exactly
  the move ratio-only instruments cannot see.

## What the gate is honest about — two scope limits

**1. A rare-label relocation is legible, not dominant.** latitude's 2,481 net new FPs
are real and flagged, but they are *not* the largest mover for latdec — broad retrain
churn (a retrain moves 12–25% of all columns: latdec 12.0%, v22 21.3%, v23 24.7%)
produces larger absolute moves on common labels. The gate's win over the proxy is
**legibility** (2,481 vs ~18 invisible columns), not isolation. The report ranks movers
by net contra so the load-bearing rows surface; latitude is among them, correctly
banded.

**2. GO-precision is unvalidated.** Because every retrain on disk is a known NO-GO and
v19-self is a degenerate GO (no moves), the gate's trigger count is high (52–76 labels
per bad candidate) and we have **no known-good non-trivial candidate** to set the
false-alarm threshold against. The bands are tuned to reproduce the four labelled
verdicts; whether they clear a genuinely-good retrain without a false NO-GO is the open
question. That validation arrives with the first real GO candidate — the
sibling-context model is the first to run *through* this gate (out of scope here, a
separate spec). Until then the gate is trustworthy as a **NO-GO detector** (it has
never missed a known failure) and advisory on GO, matching the `safety_score`
precedent: trust accrues from labelled outcomes, not assertion.

## Reproduce

```
for c in v19self:output/ydf-validation-gate/v19_gated.parquet \
         v22:output/ydf-validation-gate/v22_gated.parquet \
         v23:output/ydf-validation-gate/v23_gated.parquet \
         latdec:eval/gittables/corpus_pass_latdec/corpus_pass/columns.parquet; do
  python3 scripts/corpus_honest_gate.py --candidate "${c#*:}" --label "${c%%:*}"
done
```

Artefacts: `output/corpus-honest-gate/gate_{v19self,v22,v23,latdec}.json`.
