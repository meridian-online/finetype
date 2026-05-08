# Spec Review

**Date:** 2026-03-25
**Reviewer:** Context-separated agent (fresh session)
**Spec:** .orbit/specs/2026-03-25-overnight-training-v5/spec.yaml
**Verdict:** REQUEST_CHANGES

---

## Findings

### [CRITICAL] Distilled label quality makes 50/50 blend ratio a silent liability
**Category:** assumption
**Description:** 72% disagreement rate is systemic, not noise. AC-4 validation pass will exclude many distilled rows, causing most types to silently degrade to synthetic-only. No logging of actual achieved blend ratio.
**Evidence:** 71 types have zero distilled data, 103 have <10 rows. Validation pass exclusion + low distilled counts = synthetic-dominated for most types.
**Recommendation:** Add AC logging actual per-type blend ratio. Flag types where distilled falls below 20%. Decision point if >30 types hit threshold.

### [CRITICAL] Eval set contamination unaddressed
**Category:** test-gap
**Description:** No verification that the 29 profile eval datasets are excluded from the distilled training corpus. If any eval data leaked into training, accuracy improvements are inflated.
**Evidence:** Distilled pipeline ingests real-world CSVs. Profile eval uses 29 real-world datasets. Overlap not checked.
**Recommendation:** Add exclusion check in AC-5 verification.

### [CRITICAL] 8-hour budget not validated — arithmetic suggests it won't fit
**Category:** failure-mode
**Description:** Scaled architecture × 5× data × 40 epochs could take 10-14 hours for the scaled variant alone. No empirical timing probe.
**Evidence:** Current: 125s/epoch × 194k records. Scaled: ~2× FLOPs per forward pass × 5× data = 7.5-10× per epoch. 40 epochs × 1000s/epoch = 11+ hours for scaled alone.
**Recommendation:** Add 2-epoch timing probe as gate. Reduce epochs or abort if projection exceeds 7 hours.

### [WARN] AC-2 augmentation verification is incoherent with augmentation goal
**Category:** assumption
**Description:** Verification says augmented samples must "still parse as their labelled type." But augmented samples intentionally violate strict format patterns (leading whitespace on SSN fails SSN validator).
**Recommendation:** Verify pre-augmentation source value, not post-augmentation value.

### [WARN] No per-column regression guard
**Category:** missing-requirement
**Description:** "≥155/190 post-Sharpen" floor allows breaking 20 currently-correct columns if 35 new ones pass. Aggregate scores hide catastrophic per-type regressions.
**Recommendation:** Add AC: no more than 5 currently-passing columns should fail in new model.

### [WARN] Augmentation additive vs replacement unclear
**Category:** constraint-conflict
**Description:** Does augmentation increase total records or replace in-place? Matters for timing and corpus size validation.
**Recommendation:** Clarify in AC-2/AC-5. State expected total record count.

### [WARN] AC-7 resumability not verified
**Category:** test-gap
**Description:** Script claims idempotency but verification only checks wall time. No test for resume after partial failure.
**Recommendation:** Simulate mid-run failure, verify resume skips completed stages.

### [WARN] AC-10 no DuckDB integration spot-check or rollback plan
**Category:** missing-requirement
**Description:** "Loads successfully" doesn't verify correct output. Scaled architecture may produce garbage through DuckDB path if config.json is wrong.
**Recommendation:** Add 5-column spot-check through DuckDB. Add rollback procedure.

### [INFO] AC-6 stats branch dimensions inconsistent
**Category:** assumption
**Description:** CLAUDE.md describes current stats as both "27→64" and "27→192→96". AC-6 "scaled" matches one of the "current" descriptions.
**Recommendation:** Verify actual model dimensions from config.json before defining current vs scaled.

### [INFO] No Metal GPU verification
**Category:** missing-requirement
**Description:** If Metal silently falls back to CPU, epoch time is 10-20× longer. No preflight check.
**Recommendation:** Add 1-epoch throughput probe in overnight script.

---

## Honest Assessment

The spec has sound intuition — generator fixes + augmentation + hard negatives are the right interventions — but is undercooked on two dangerous failure modes. First, the 72% distilled label disagreement rate is structural, and the label validation pass is not calibrated to detect systematic error at that scale. Second, the time budget has no empirical basis and the arithmetic strongly suggests the scaled architecture × 5× data × 40 epochs will not fit in 8 hours. Fix the time probe and blend-ratio logging requirements before starting implementation.
