# Spec Review

**Date:** 2026-03-20
**Reviewer:** Context-separated agent (fresh session, Opus)
**Spec:** specs/2026-03-20-retraining-spike/spec.yaml
**Verdict:** REQUEST_CHANGES

---

## Findings

### [CRITICAL] F1: `score_tier2.py` has no `--model` flag
**Category:** assumption
**Description:** The spec (S-2, step 2) calls `python3 scripts/score_tier2.py --model models/spike-<mix>`. The actual `score_tier2.py` has no `--model` parameter — it accepts `--benchmark`, `--finetype`, `--output`, and `--format` only. The finetype binary resolves the model via the `models/default` symlink.
**Evidence:** `score_tier2.py` lines 231-254 — argument parser has no `--model` handling.
**Recommendation:** The experiment runner must either (a) temporarily swap the `models/default` symlink before each scoring invocation, or (b) add `--model` support to `score_tier2.py`. Spec S-2 should explicitly describe the symlink management mechanism and restore-on-failure trap.

### [CRITICAL] F2: Nested symlink swapping between `run_spike.sh` and `eval.sh`
**Category:** failure-mode
**Description:** `eval.sh` saves the current `models/default` symlink target, swaps it, runs eval, then restores. If `run_spike.sh` already swapped the symlink before calling `eval.sh`, the "original" that `eval.sh` saves is the spike model, not the production model. Nesting breaks the restore logic.
**Evidence:** `eval.sh` lines 53-95 — save/swap/restore pattern assumes it owns the symlink.
**Recommendation:** Define a single symlink management strategy in `run_spike.sh`. Either bypass `eval.sh`'s symlink logic (call `make eval-profile` directly), or use an explicit env var for model resolution. Add pre-flight and post-flight assertions that `models/default` → `char-cnn-v14-250`.

### [WARN] F3: 499 JSON parse errors in `sample_values` unaddressed
**Category:** missing-requirement
**Description:** 499 rows in `sherlock_distilled.csv.gz` fail to parse `sample_values` as JSON. The prep script will silently skip or crash on these.
**Evidence:** Direct analysis of the distilled data.
**Recommendation:** Add JSON parse error handling to S-0 (characterise) and S-1 (skip with warning, log count).

### [WARN] F4: "Distilled-only" experiment will have zero samples for 77+ types
**Category:** assumption
**Description:** 77+ types have no distilled data. A model trained on only ~173 classes will catastrophically fail on missing types, making the distilled-only Tier 1/Tier 2 scores misleading as a comparison point.
**Evidence:** 250 taxonomy types minus ~173 distilled types = ~77 missing. The spec lists this as one of the 5 required experiments without caveats.
**Recommendation:** Either (a) acknowledge distilled-only will be a "partial model" and document the caveat prominently, or (b) replace with "distilled + synthetic-fill" where missing types get synthetic data.

### [WARN] F5: Blending strategy under-specified for imbalanced types
**Category:** assumption
**Description:** The spec says "per-type balanced" blending but doesn't define what happens when a type has 2 distilled rows and the target is 1000 samples/type at 50/50. Does it oversample to 500? Use only 2? Oversampling 2 values to 500 teaches the model to memorise those strings.
**Evidence:** Top type has 15,363 rows; bottom types have 0-5.
**Recommendation:** Explicitly define: (a) what `--samples-per-type N` means when one source has fewer than N samples, (b) whether under-represented types are oversampled or capped, (c) the default N.

### [WARN] F6: Categorical training data will predictably cause negative transfer
**Category:** assumption
**Description:** `categorical` is 15,363 rows (18% of all data). Individual categorical values like "Male", "Yes", "No" directly conflict with gender, boolean, and other type labels in synthetic data. This isn't just "monitoring" — it's predictable negative transfer at massive scale.
**Evidence:** 15,363 of 85,194 distilled rows are categorical. The CharCNN sees individual strings with no column context.
**Recommendation:** Add at least one experiment explicitly excluding categorical, ordinal, and increment. A "blended-70-30-no-column-types" experiment would be the most impactful addition. The current 5 experiments risk concluding "distilled data hurts" when the real signal is "column-level type data hurts."

### [WARN] F7: Tier 2 benchmark has no categorical/ordinal/increment columns
**Category:** test-gap
**Description:** The benchmark contains zero `representation.discrete.categorical`, `representation.discrete.ordinal`, or `representation.identifier.increment` columns. AC-5's column-level type analysis can't measure direct impact.
**Evidence:** Grep of `tier2_benchmark.csv` — zero rows for these types.
**Recommendation:** Measure negative transfer indirectly: do types sharing value patterns with categorical (boolean, gender, enum-like types) degrade when categorical training data is included? Name the specific proxy types.

### [INFO] F8: 8 rows with empty `final_label`
**Category:** missing-requirement
**Description:** 8 rows have empty `final_label`, producing invalid training samples with `classification: ""`.
**Recommendation:** Filter these in S-1. Minor but should be explicit.

### [INFO] F9: Training time estimate may be wrong
**Category:** assumption
**Description:** "~45 min per model" appears based on Metal. The current machine is Linux CPU. CPU training is significantly slower — potentially 2-3 hours each, totalling 10-15 hours.
**Recommendation:** Clarify expected hardware. Update time estimate if running on CPU.

### [INFO] F10: 85K/58K split is actually a three-way split
**Category:** assumption
**Description:** 85,194 total = 58,429 (≥5 values) + 26,266 (<5 values) + 499 (parse errors). The parse error category is missing from the spec.
**Recommendation:** S-0 will catch this, but acknowledge the three-way split.

---

## Honest Assessment

The spec is well-structured for a spike — clear goal, held variables, defined experiments, automated runner. The two critical issues (F1, F2) are about the same thing: the model resolution mechanism. Neither `score_tier2.py` nor `eval.sh` were designed for multi-model comparison, and the spec assumes interfaces that don't exist. This will cause implementation debugging that could be avoided by designing the symlink strategy upfront.

The biggest experimental risk is F6: training on 15K categorical values that actively conflict with other type labels. The "include but monitor" stance is insufficient when categorical is 18% of training data. At minimum, one experiment should exclude column-level types — otherwise the spike may conclude "distilled data hurts" when the real conclusion is "mislabelled column-level data hurts."
