# Session handoff → next session is the MODEL LABEL-SPACE RESHAPE

_Updated 2026-06-27 (end of the ac-2 + architecture-discovery session)._

## Headline

**Composed gold 0.847 → 0.863 this session**, all on main + pushed. Banked: the
batch-path speed win + CSV-ingestion robustness, the #3-utc Sharpen rule, and the
full ac-2 eval cleanup (17+2 value-verified re-adjudications). Then a substrate
survey produced the next architectural move as a written-up spec.

## What this session shipped (all on main, pushed)

| commit | what | composed gold |
|---|---|---|
| 841cd2b | batch-path taxonomy hoist (~19% faster) + `parallel=false` CSV fallback + batch skip-and-log | 0.847 (held; +4 recovered) |
| ec117d0 | `utc_bare_number_veto` (#3) + 1 re-adjudication (adversarially verified) | 0.847→0.852 |
| 4a6a5df | ac-2(a) 17 clean re-adjudications (of 34 verified; 17 kept = model over-emit) | 0.852→0.861 |
| f4581dd | ac-2(b) re-pull = NO-OP (all 931 scored; the "31 missing" was stale) | 0.861 |
| 4dfe549 | ac-2(c) 2 contested re-adjudications (of 13; 11 kept) | 0.861→0.863 |
| **8f2096f** | **discovery spec: model label-space reshape** | — |

**Live composed gold (reframe PRIMARY) = 803/931 = 0.863.** Raw Sense ~0.52.

## NEXT SESSION = `2026-06-27-model-label-space-reshape` (spec written, 8 ACs)

**The thesis (load-bearing):** Sharpen carries composed accuracy (3 retrains
+4-7pp raw Sense, composed ZERO). The model's fine value-determinable label space
is its main LIABILITY — it manufactures the over-emission that NO-GOs every fresh
retrain (isbn collapse, currency_code 3.2×, user_agent 7.2×). This session's gold
work is the same signal: 28 of 47 re-adjudication candidates were KEPT because the
model over-emits, not because gold was wrong.

**The move:** drop the validator-ownable closed/format/checksum leaves from the
model's TRAINING label space; cede them to Sharpen+validators (correct-by-
construction; recovery already ships). Simplicity + accuracy + the unblock for a
shippable retrain, in one reshape.

**Read first:** `orbit spec show 2026-06-27-model-label-space-reshape` + its
`interview.md` (the ranked levers + the full DO-NOT-REPROPOSE dead-end catalog —
do not re-run a proven failure). Survey provenance: workflow `wf_e68a56d0-90b`.

**Sequence (in interview.md):**
1. **ac-6(a)** delete the 0107-blessed orphan modules (CharCNN/Tiered/TextClassifier/
   legacy Trainer) — free, gold-no-regression gated. Good first move.
2. **ac-0 → ac-3** the leaf-drop reshape. The decisive cheap experiment is **ac-2**:
   single-seed potion retrain on the reduced label space → does the corpus
   over-emit band clear? Fold the encoder/branch ablations (ac-6b: dual→single
   potion −20-40ms; drop the validation branch) into the same retrain.
3. **ac-4** abstaining-head DE-RISK probe (kill-switch: are over-tighten misses
   separable in the shared trunk?) — only after the leaf-drop clears; **ac-5**
   builds the full gate only if ac-4 passes, measured on composed PASS-THROUGH.
4. **ac-7** the division-of-labour MADR (owner decision the ship depends on).

## Guardrails (carry-over)
- Model swap → gold-parity + gold-adjudicated relocation review (0104), NOT
  corpus-honest GO (0% pass; structurally unpassable by any retrain).
- Sharpen rule → `gate_from_cache.sh` (~seconds, reuses
  `output/sharpen-audit/tierA/sense_cache.tsv`) or `corpus_honest_gate_fast.sh`.
- Re-adjudication / gold change → adversarial defend-gold verification + leakage
  guard (label-only = membership identical = ALL PASS). Corrections skew AWAY from
  the model (it cleans gold, doesn't inflate).
- Do NOT cede open-vocab leaves (city/entity_name/username/numeric_code) — closure
  not semantics decides; those are the retrain-negatives target (t-000133e418).
- Dead branches, do not re-propose: see the interview.md catalog (bigger static
  encoder, transformer in-model, value-fusion, m2v witness, hierarchical head,
  44-stat trunk, additive flat-softmax retrains, residual merge).

## Key memories to read at session start
`composed-is-rule-bound`, `specialiser-is-the-abstaining-witness`,
`categorical-is-a-residual-category`, `sense-stage-ceiling-and-free-latency-wins`,
`repull-is-noop-gold-is-fully-scored`, `eval-csv-fragility-is-reader-not-format`.

## Open follow-ups (filed)
- ac-6 SECOND HALF — N files per duckdb spawn (t-00007d2e), latency only.
- #6 NPI Luhn HELD (t-00016c8f), corpus-wide checksum change, gate alone.
- ac-2(c) defensible-tail remainder + taxonomy-gap ledger (numeric UTC offset)
  in `.orbit/memos/2026-06-19-taxonomy-gap-discovery.md`.

## How to continue
1. `orbit session prime`, then `orbit spec show 2026-06-27-model-label-space-reshape`.
2. Read this file + that spec's `interview.md` + the memories above.
3. Start ac-6(a) (free) or scope ac-0 (the cede/keep/exclude partition).
