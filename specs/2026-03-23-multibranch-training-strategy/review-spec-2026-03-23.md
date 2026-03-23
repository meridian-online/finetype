# Spec Review

**Date:** 2026-03-23
**Reviewer:** Context-separated agent (fresh session)
**Spec:** specs/2026-03-23-multibranch-training-strategy/spec.yaml
**Verdict:** REQUEST_CHANGES

---

## Findings

### [CRITICAL] Baseline number discrepancy
**Category:** assumption
**Description:** The spec cites sherlock-v2-flat as 128/190 (67.4%), but the findings doc shows 101/190 (53%). These numbers are irreconcilable — the 150/190 target is actually a 49-column improvement from the real baseline, not 22.
**Evidence:** `findings.md` line 16: `sherlock-v2-flat (raw) | 101/190 (53%)`. Spec AC-6: "Improvement over sherlock-v2-flat baseline (128/190, 67.4%)".
**Recommendation:** Correct the baseline. Restate AC-6's improvement threshold accordingly.

### [CRITICAL] Phase 1 skipped without justification
**Category:** missing-requirement
**Description:** The findings doc explicitly states Phase 1 (post-processing overlay) is the prerequisite — "Phase 2 and 3 are only worth pursuing if Phase 1 shows the model adds signal." The spec jumps straight to Phase 2 without establishing whether the base model adds signal within the existing pipeline.
**Evidence:** `findings.md` lines 119–160.
**Recommendation:** Explicitly justify skipping Phase 1 and revise success criteria to acknowledge comparison is against the raw multi-branch baseline (not the production pipeline).

### [CRITICAL] Core deliverables don't exist yet
**Category:** failure-mode
**Description:** `extract-features` CLI doesn't produce header embeddings. `TrainingRecord` has no `header_features` field. `prepare_multibranch_data.py` never passes headers. These are the core deliverables — the spec should enumerate them as implementation tasks.
**Evidence:** `main.rs` lines 988–994, `prepare_multibranch_data.py` line 558, `multi_branch.rs` TrainingRecord struct.
**Recommendation:** AC-3 must explicitly list extending the CLI, the TrainingRecord struct, and the Python script.

### [CRITICAL] FTMB backward compat requires code changes not listed
**Category:** constraint-conflict
**Description:** Both Rust and Python readers hard-fail on unknown FTMB versions. The spec requires backward-compatible reading but the code rejects unknown versions by design.
**Evidence:** `multi_branch.rs` lines 523–525: `bail!("Unsupported FTMB version")`. Python reader has same strict assertion.
**Recommendation:** AC-2 must include modifying both readers' version-check logic.

### [WARN] Synthetic data header signal is unrealistically discriminative
**Category:** assumption
**Description:** Synthetic data has no real headers. If type keys are used as headers, the model trains with perfect discriminative signal unavailable at inference. Creates train/inference distribution mismatch.
**Evidence:** `prepare_multibranch_data.py` `generate_synthetic_columns()` produces values without headers.
**Recommendation:** Specify what canonical headers look like for synthetic data. Test accuracy with and without headers at inference.

### [WARN] No integration tests for FTMB round-trip
**Category:** test-gap
**Description:** Format changes carry same risk as the VarBuilder prefix mismatch that caused 0/0 eval results in the prior cycle. Current verification is manual spot-check.
**Evidence:** VarBuilder bug in findings.md lines 100–113.
**Recommendation:** Add a Rust unit test for FTMB v1↔v2 round-trip.

### [WARN] Merge dimension arithmetic needs code changes
**Category:** constraint-conflict
**Description:** `merged_dim()` in both Rust files computes `char_hidden[1] + embed_hidden[1] + stats_hidden[1]` — no header branch. BatchNorm will be sized at 564 instead of 628.
**Evidence:** Both `multi_branch.rs` files, lines 89–91.
**Recommendation:** AC-1 must explicitly update `merged_dim()` and add `header_hidden` to `MultiBranchConfig`.

### [WARN] Overnight script hardcodes 30/70 ratio
**Category:** missing-requirement
**Description:** `overnight_sherlock.sh` hardcodes `--ratio-distilled 0.3` and output path `blend-30-70.ftmb`.
**Evidence:** `overnight_sherlock.sh` lines 110–129.
**Recommendation:** Add exit condition for updating the overnight script.

### [INFO] Lock down Model2Vec dimension
**Category:** assumption
**Description:** Spec mentions both 384-dim and 128-dim options. The codebase uses 128-dim consistently.
**Recommendation:** Commit to 128-dim. Remove 384-dim option.

### [INFO] Eval contamination check is unverifiable
**Category:** test-gap
**Description:** FTMB format has no provenance metadata. AC-4 says "no file paths from eval/datasets/ appear" but this can't be verified from the .ftmb file.
**Recommendation:** Document that distillation pipeline is known clean, or add provenance to manifest JSON.

---

## Honest Assessment

The plan is directionally correct — header branch is the right next step, 50/50 data is reasonable. But three implementation gaps will cause work to stop before training: extract-features doesn't produce header embeddings, TrainingRecord has no header_features field, and prepare_multibranch_data.py never passes headers. The FTMB version bump is also underspecified. Fix the baseline number, enumerate the missing implementation tasks, commit to 128-dim Model2Vec, and justify skipping Phase 1 — then this is ready.
