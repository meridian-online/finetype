# ac-01 — Dual-encoder native: built, verified, and potion-8M ties v19 natively

**Date:** 2026-06-23 (CORRECTED — supersedes ac01-native-verdict-HALT.md, whose HALT
was a false alarm from a config bug, not a model regression)

## Headline

The dual-encoder works, and potion-8M reproduces its offline number natively:
**native gold composed best-of-3 = 0.794, a tie with v19's 0.797** (within CI). The
earlier "−11pp HALT" was wrong — it was caused by a training-pipeline config bug
(missing `type_index_keys`) that silently zeroed the validation branch at native
inference. Fixed. potion-8M is a live candidate; the real blocking gate (corpus-honest)
is next.

## The corrected numbers (native, faithful, reframe, n=927)

| model | native composed |
|---|---|
| v19 (shipped) | 0.797 |
| potion-8M s42 / s43 / s44 | 0.756 / **0.794** / 0.781 |
| **potion-8M best-of-3** | **0.794 — ties v19 (within CI)** |

This matches the offline 0.794. The offline number was faithful after all.

## The bug (what actually happened)

The multi-branch model has a validation branch: one taxonomy-validator pass-rate per
type, ordered by `config.type_index_keys`. v19's config carries all 240 keys. The
potion training pipeline (`overnight_potion.sh` → `train-multi-branch` with
`m2v8m-244-config.json`) **never wrote `type_index_keys`** — every potion-8M/code-16M
seed config had it missing.

`MultiBranchClassifier` disabled the validation branch when the keys were absent and
fed it **zeros**, while the model was *trained* on real pass-rates and the offline
`predict_multibranch` reads them from the FTMB. So native and offline were running
genuinely different models.

**Proof:** native vs offline *pure argmax* (no veto, no Sharpen) agreed only **46%**;
the v19 control agreed **93%**. After restoring the keys, agreement jumped to **90%**
(matching v19's baseline) and composed went to 0.794. `msg_id` flipped `isbn` →
`alphanumeric_id`.

My earlier diagnosis (veto-exposes-drift) was wrong — `--no-validation-veto` left the
gap unchanged (0.687), which is what exposed that the veto was never the cause.

## The fix (committed)

Two layers:
1. **Runtime robustness (commit fb44b26):** when `type_index_keys` is missing but a
   trained validation branch exists, derive the order from the live taxonomy
   (`ValidationFeatureExtractor::new` — the exact ordering training's `extract-features`
   used), cached on first classify, guarded by a `valid_dim` match. Config-pinned keys
   still take precedence. Prevents any model from silently zeroing the branch again.
2. **For shipping:** the model config must still *persist* `type_index_keys` (the
   version-decoupling path). The staged potion configs were patched with the 244 keys;
   the training pipeline should be updated to write them (follow-up).

## What was built (ac-00 + ac-01, kept)

Dual-encoder native inference in `MultiBranchClassifier`: optional `value_model2vec`
(potion-8M, 256-dim → 1024 agg) for the value branch; potion-4M for header + Sense +
entity + semantic + sibling. `extract_embedding_aggregation_dyn` (any width). Absent
`value_embed_model` → single-encoder, bit-identical to v19. Value features verified
byte-identical to offline `potion_embed` (msg_id, region). Tests green.

## Implications

- **potion-8M is back as a live candidate** — reproducible, 244-label, ties v19 on gold.
- **The m2v-244 corpus NO-GO is now suspect.** It was likely the same zeroed-validation
  artifact (it reported 101k unknowns), not real data drift. Memory
  [[m2v244-corpus-nogo-gold-reproducible-not-corpus-faithful]] must be re-checked with
  the fix before its "fresh-retrain drift" conclusion stands.
- The data-drift follow-up task (t-000133e418) is **paused pending re-check** — its
  premise may have been the bug, not the data.

## Next (ac-02, the real blocking gate)

Run the native corpus-honest gate on the fixed potion-8M config vs the v19 oracle —
the actual GO/NO-GO. This is the gate the offline path could never give faithfully.
