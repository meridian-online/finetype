# build ac-04 — corpus-honest gate on the encoder escalation: NO-GO (relocation)

**Spec:** 2026-06-18-minilm-encoder-build · **Date:** 2026-06-19
**Run:** `scripts/corpus_honest_gate.py --candidate escalation_candidate.parquet --label-remap
representation.discrete.categorical=representation.text.word`
**Gate output:** `output/fine-tuned-encoder-discovery/gate/gate_gte-tiny-escalation.json`

## Verdict: NO-GO — the encoder escalation RELOCATES errors at corpus scale.

The single test that decides the bet. It failed — the same failure mode that killed the six
retrains, and it caught what gold/representative (0.811, residual P 0.88) physically could not.

**Three bands fired (33k stratified sample, 852k columns):**

| label | band | v19 → cand | what happened |
|---|---|---|---|
| `representation.text.word` (residual) | **oracle_fp** | 207k → **478k** (2.3×) | **52k oracle-CONTRADICTED moves into residual** — columns the oracle says are *not* text, demoted to "just text" |
| `geography.location.city` | **collapse** | 38k → 13k (−66%) | real cities demoted away |
| `geography.location.country` | **collapse** | 11k → 5k (−56%) | real countries demoted away |

Driver: ~61k `entity_name` and ~71k `full_name` columns demoted to residual. **The encoder
over-emits residual at corpus scale** — invisible on the balanced 244-col gold slice (53%
residual), exposed by the corpus (where the model's entity/person predictions are mostly correct).

## Why this matters (and why we ran it before the full build)

This is the **7th confirmation** that *gold-looks-good ≠ corpus-honest-clean*. The encoder build
looked fully de-risked — latency, attractor, data, recipe, 0.811 — and the gate still caught a
fundamental relocation problem. Running it now, before the 250-class scale-up + Rust integration,
**saved days** and is exactly the fail-cheap discipline the spec was built around.

## The load-bearing caveat: this over-states the real design

This is the **strict "escalate every contested column" regime** — NOT the build's **low-band-only**
design (I couldn't run that: `v19_gated.parquet` has null `sense_confidence`). Most of the damage
is *high-confidence-correct* `entity_name`/`full_name` columns demoted to residual — precisely the
columns the low-band design would **never escalate**. So the real design relocates **far less**
than this. The NO-GO is on a regime stricter than what we'd ship.

## Honest read + next steps

- **The encoder over-emits residual** — a real signal, not only an artifact. Two causes: (a) the
  every-column regime escalates confident-correct columns; (b) the training data is 44% residual,
  teaching over-prediction vs the corpus distribution.
- **The bet is not de-risked through the gate** — but it's not dead either. To judge the actual
  design fairly:
  1. **Capture confidence** (re-run a corpus pass on the sample recording `sense_confidence`, or
     re-profile the contested columns) → escalate **low-band-only** → re-gate. This is the fair test.
  2. **Rebalance the training data** toward the corpus residual rate (it's 44% residual; the corpus
     contested distribution is far less), and add a residual-precision penalty — the encoder must
     learn to demote-to-residual *sparingly*.
- **Until a low-band-only run clears the corpus-honest gate, this is a yellow/red flag**, not a GO.
  The corpus-honest relocation gate remains the arbiter — and it just earned its keep again.

## One line

The encoder that scored 0.811 on curated gold RELOCATES ~52k columns into residual at corpus
scale — caught cheaply by the gate before the full build; the low-band-only design (untested here,
no confidence) would relocate far less, so the next move is a fair confidence-gated re-run plus a
residual-rebalanced retrain, not a promotion.
</content>
