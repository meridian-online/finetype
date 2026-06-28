# Clean-label retrain — FINAL VERDICT: NO-GO (training labels are NOT the ceiling)

Spec 2026-06-28-clean-label-retrain. Decided 2026-06-28. Single seed (42), shipped
architecture + Sharpen held FIXED; only the geo/person training labels changed.

## The number (composed gold, reframe-scored; go/no-go bar = s43 0.853)

| system | Sense (reframe) | composed (reframe) | vs s43 |
|---|---|---|---|
| **baseline s43** (noisy distilled-Sherlock labels) | 0.536 | **0.853** | — |
| REPLACE — drop v3 geo/person, use generator-only clean positives | 0.450 | 0.774 | **−7.9pp** |
| **AUGMENT — keep all real v3 + ADD clean positives** (the clean test) | 0.516 | **0.845** | **−0.8pp (FLAT, within CI)** |

Gold CI at n=931 is ≈ ±2.4pp, so AUGMENT 0.845 is **statistically indistinguishable** from
the 0.853 baseline. Raw Sense even ticked *down* (0.536→0.516). **Clean vocab-membership
labels do not move composed gold.**

## The diagnosis (three-way per-family recall, composed)

| family | s43 (noisy) | REPLACE (synthetic) | AUGMENT (real+clean) |
|---|---|---|---|
| geography.location.city | 0.958 | 0.458 | **0.958** |
| geography.location.country | 0.818 | 0.455 | **0.909** |
| geography.location.country_code | 0.926 | 0.500 | **0.944** |
| geography.location.region | 0.800 | 0.533 | **0.800** |
| geography.location.continent | 1.000 | 0.000 | **1.000** |

Two things are now proven:

1. **The shipped model already saturates semantic gold.** s43 scores 0.80–1.00 recall on
   every geo family. The labels there are NOT the bottleneck — there is no headroom for clean
   labels to recover. AUGMENT confirms this: it *preserved* every family (and marginally lifted
   country/country_code via the clean positives), yet the headline stayed flat because the
   families were already maxed and the gains were per-family, not net.

2. **REPLACE's −7.9pp was a distribution shift, not a label-quality signal.** Swapping real
   columns for synthetic GeoNames/Wikidata positives cratered exactly the synthesised families
   (continent 1.000→0.000, city 0.958→0.458) — the model overfit clean synthetic values
   (val_acc 0.934) and lost real-column generalisation. The ac-0 audit predicted this by name.

## Conclusion — the thesis is refuted on composed gold

The "training-label quality is the unrun ceiling lever" thesis (memory
`data-label-quality-is-the-unrun-ceiling-lever`) does **not** hold for composed gold:

- The data-lever proof (contested set 0.684→0.82, `encoder-data-lever-proven`) was
  **instrument-specific** — measured on the CONTESTED subset, RAW Sense, a frozen-MiniLM
  family-level (8-class) linear probe. It does NOT translate to the shipped 245-class
  multibranch on composed gold.
- The empty cell "static model + clean vocab labels + composed gold" is now **filled: flat**.
- `composed-is-rule-bound` earns its **4th confirmation** — and this time on the open-vocab
  semantic mass that Sharpen supposedly *cannot* fix, which was the strongest case for the
  data lever. It still didn't move.

**Decisive either way (handoff's pre-registered framing): composed did not move → the ceiling
is the rule layer + already-saturated semantic gold + the irreducible residual mass, NOT the
training labels. Stop chasing data/labels for composed accuracy.**

## Caveats

- **Taxonomy drift 244→245.** `datetime.offset.timezone_abbreviation` was added after s43
  (Sharpen-recovered, ZERO training rows). The candidate was trained at 245 (the live taxonomy =
  the env where 0.853 was measured); the extra leaf is inert (no training data, Sharpen-recovered
  in both baseline and candidate). A pre-flight `valid_dim` guard now prevents the dimension
  mismatch that crashed the first 8M run.
- **NaN at REPLACE epoch 44** (gradient blowup) — the best-checkpoint mechanism (`model_best`,
  epoch 40, val 0.934) preserved the healthy model; the result is valid. AUGMENT trained clean
  to epoch 66, no NaN.
- **Single seed** — the −0.8pp is within seed + CI noise → flat, not a real regression.
- **Untested:** the FULL proven recipe also *mined* mislabelled residual columns and relabelled
  hard negatives; this run added clean POSITIVES only. The saturated-semantic-gold finding makes
  a mining variant a weak prior, but it is not strictly closed.
- **4M speed track: NOT run** — gated on AUGMENT being GO; it was flat, so the smaller encoder
  was not pursued (no accuracy headroom to make it viable).

## Substrate

`scripts/{audit_clean_labels.py, build_clean_label_blend.py, run_clean_label_retrain.sh,
run_clean_label_augment.sh, score_clean_label.sh}`, `output/clean-label-retrain/{ac0_*.md,
*manifest.json, retrain.log, augment.log, scores/}`. Models `clean8m-s42` (REPLACE),
`clean8m-aug-s42` (AUGMENT). Builds on `composed-is-rule-bound`, `ceiling-discovery-both-levers-dead`.
