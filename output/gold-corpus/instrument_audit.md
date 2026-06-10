# Instrument audit against gold v1 (ac-07)

**Date:** 2026-06-10 · **Gold:** `eval/gold/gold_corpus_v1.tsv` (931 verified columns;
809 of them in the GitTables corpus where the proxy instruments operate) ·
**Status:** recommendations are ADVISORY until the author accepts them (spec ac-07).

## The headline

**On the columns that decided every promotion fight, the model has been better than
its referee.** Scored against verified labels on the same 809 columns: shipped v19 =
68.2% correct; the gated-YDF oracle = 57.9% correct when it asserts (it abstains on
16.3%; counting abstentions as non-answers it covers 48.5%). When the oracle asserts
and disagrees with truth — 42.1% of its assertions here — every metric built on it
charged the model for the oracle's mistake or credited the oracle's error as model
correctness.

Selection caveat, stated plainly: gold over-samples contested columns by design, so
these are NOT corpus-wide rates — the oracle is far better on the boring 95%+ of
columns. But promotion decisions were fought precisely on the contested ground, which
is where the referee is weakest. Its top error families mirror the campaign's pain:
`year` asserted on plain integers (31×), `plain_text` on real URLs (29×),
`numeric_code` on plain integers, `categorical` on real cities, `decimal` on real
latitudes.

## Instrument-by-instrument

| Instrument | Measured against gold | Recommendation (advisory) |
|---|---|---|
| **Gated-YDF oracle** (`ydf_prediction_gated`) | 57.9% precision-when-asserting on contested ground (392/677); 16.3% abstain; error families above | **Demote from accuracy referee to candidate-generation lens.** Already not the headline (choice 0093); this audit closes the question with a number. Keep for mining/corroboration where its cheapness at corpus scale is unmatched. Do NOT use it to adjudicate per-column correctness in any future gate band without a gold cross-check. |
| **Rare-type scoreboard** (`build_rare_type_gold.py`) | Its header-anchored gold logic verified at **99.0%** (193/195 two-panel-verified rows; the 2 failures are protocol-relative `//` URL columns wrongly in the negative pool) — `scoreboard_canonicalisation.md` | **Keep; now a validated absolute instrument** on the types it covers. One refinement: count `//`-prefixed values as url-positive. |
| **Corpus-honest gate** (`corpus_honest_gate.py`) | Not directly scorable per-column (it measures candidate-vs-baseline transitions). Its oracle-aware bands inherit the oracle's 42% contested-ground error per column, BUT its aggregate band design + the reproduced four-verdict record (v19 GO; v22/v23/latdec NO-GO) remain the only proven detector of error RELOCATION. | **Keep its blocking NO-GO role (H05) unchanged** — the latdec constraint stands: nothing else catches relocation. Treat its per-label `oracle_fp` counts as directional, not exact; a future refinement could re-base the bands on gold-verified columns where they overlap. |
| **Destination-drift proxy** (`proxy_pretrain.sh` + `drift_report.py`) | Distribution-level, not per-column — not scorable against gold directly. Record: caught v23/v24-style over-emit explosions; missed latdec's rare-label relocation (known, documented blind spot). | **Keep as the cheap PRE-train gate** for common-boundary over-emit; unchanged. |
| **Gold corpus** (this spec) | 931 verified columns, per-row provenance, leakage-firewalled; v19 = 65.5% (CI 62.4–68.5) overall | **Becomes the canonical accuracy eval** (ac-08 MADR formalises). |

## What this resolves

- The "precision ceiling" story closes end-to-end: the metric was blind (eval-ceiling
  diagnosis), and the referee underneath it was wrong 4-in-10 on the contested columns
  (this audit). Four retrains were judged by that referee.
- The promotion order simplifies (pending ac-08): gold-anchor (efficacy) → drift proxy
  (pre-train) → **gold corpus (headline accuracy)** + rare-type scoreboard (validated,
  contested types) → corpus-honest gate (blocking, relocation) → swap.

## What we still don't know

- Oracle accuracy on UNCONTESTED columns (gold's backbone is small); corpus-wide
  oracle quality is certainly higher than 57.9% — this audit measures the battleground.
- Whether gold detects relocation as well as the corpus-honest gate (the recorded
  precondition for ever retiring the gate's blocking role) — untested until the next
  real candidate runs both.
