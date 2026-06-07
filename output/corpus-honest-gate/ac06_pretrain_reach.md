# ac-06 — pre-train reach: the gate moves EARLIER

Spec `2026-06-07-corpus-honest-quality-gate`, ac-06 (`ac_type: observation`).
Investigation, recorded either way: can the honest gate predict the 50-epoch corpus
NO-GO from the **10-epoch proxy** model — the one the destination-drift pre-check
already builds — instead of waiting for the overnight spend? Measured 2026-06-07, M1
(Metal), `--jobs 8`.

## Answer — YES, for the load-bearing relocation

Profiled the ac-01 stratified sample (33,250 files) through
`models/sherlock-latdec-proxy-s42` (10 epochs) and scored it against the same stable
gated-v19 oracle. **Verdict: NO-GO. Latitude `oracle_fp` fires — harder at 10 epochs
than at 50.**

| latitude row | 10-epoch proxy | 50-epoch full |
|---|---:|---:|
| v19 marginal | 7,974 | 7,974 |
| est. candidate marginal | **10,156** | 7,888 |
| ratio | **1.274** (rising) | 0.989 (settled) |
| net contra-in | **4,716** | 2,481 |
| observed sample cols | **707** | 446 |
| band | **`oracle_fp`** | `oracle_fp` |

The relocation the gold anchor, m-19 and the drift proxy all cleared is **legible
before the overnight train**, not after. So the gate stacks onto the destination-drift
pre-check (which already trains the 10-epoch proxy on the candidate's FTMB) at the cost
of one extra ~22-min profile pass — and catches the relocation before the 50-epoch
spend, not after.

## Why it fires *harder* at 10 epochs — the head-unsettled effect

This is the ac-05 latdec finding (head mid-trajectory) pointing the helpful way. At 10
epochs the latitude head is still **over-emitting** (ratio 1.274, +2,182 marginal over
v19); by epoch 50 it has partially self-corrected the marginal back to baseline (0.989)
while the *contradicted* core persists (net 2,481). The honest read keys on net
contradicted inflow, which is large at both — but at 10 epochs the over-emission is not
yet sanded off, so latitude is *more* conspicuous, not less. The relocation is loudest
exactly when it is cheapest to catch.

## The honest scope limit — verdict reproduces, trigger *list* is provisional

| | 10-epoch proxy | 50-epoch full |
|---|---:|---:|
| verdict | NO-GO | NO-GO |
| trigger count | 56 | 52 |
| shared triggers | — | 29 |

The **verdict** and the **load-bearing band** (latitude `oracle_fp`) reproduce exactly.
The **per-label trigger list** does not: only 29 of ~52 overlap. The 10-epoch-only
triggers are dominated by soft, rare labels mid-trajectory (`entity_name`, `jwt`,
`token_urlsafe`, `first_name`, `country_code`) whose heads have not settled — the same
unsettled effect, here producing transient over-emission that may sand off by epoch 50.
Conversely a few 50-epoch triggers (`word`, `top_level_domain`, `version`) are quiet at
10 epochs because their drift arrives late in training.

So the pre-train gate is a **reliable NO-GO predictor** and **reliably lands the
load-bearing latitude relocation**, but its full per-label trigger detail at 10 epochs
**over-predicts** — read it as "this candidate is a NO-GO and here is the relocation",
not as the final per-label ledger. The settled ledger still needs the post-train run.

## Durable finding — where in the loop honesty is affordable

- **The relocation signal is affordable pre-train.** The gate moves earlier and upgrades
  the proxy: the destination-drift pre-check already builds the 10-epoch model; adding
  the honest gate's profile pass turns "over-emit on common boundaries" (proxy's existing
  remit) into "+ rare-label relocation incl. the ydf-abstain bucket". A relocating
  candidate dies before the overnight train, not after.
- **The cost is the profile pass, not the epochs.** 10-epoch and 50-epoch profile passes
  both cost ~22 min (1,334 s vs 1,349 s) — inference is per-column, training-epoch
  independent. The proxy doesn't make the *gate* cheaper; it makes the *candidate*
  cheaper to reject. Cheapness comes from killing the bet before 50-epoch spend, not from
  a faster gate.
- **The settled ledger stays post-train.** Per-label trigger detail at 10 epochs is
  provisional. The gate stays a post-train instrument for the final verdict ledger; what
  moves pre-train is the **go/no-go decision and the load-bearing relocation flag**.

## Reproduce

```
source eval/gittables/.venv/bin/activate
FINETYPE_MODEL=models/sherlock-latdec-proxy-s42 \
python3 scripts/gittables_corpus_pass.py \
  --corpus-index output/corpus-honest-gate/stratified_sample.files.txt \
  --execute --jobs 8 --out-dir output/corpus-honest-gate/proxy10_pass
python3 scripts/corpus_honest_gate.py \
  --candidate output/corpus-honest-gate/proxy10_pass/corpus_pass/columns.parquet \
  --label latdec-proxy10
```

The CLI reads `FINETYPE_MODEL` (model dir); the multi-branch `model2vec/` resources
resolve relative to CWD, so the pass must run from the repo root. Artefact:
`output/corpus-honest-gate/gate_latdec-proxy10.json`. Large parquets are local-only.
