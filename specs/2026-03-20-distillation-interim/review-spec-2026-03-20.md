# Spec Review

**Date:** 2026-03-20
**Reviewer:** Context-separated agent (fresh session, Haiku)
**Spec:** specs/2026-03-20-distillation-interim/spec.yaml
**Verdict:** REQUEST_CHANGES

---

## Findings

### [CRITICAL] AC-3: Pipeline resume not actually tested
**Category:** test-gap
**Description:** Spec says "pipeline can resume" but only checks .done files exist. Doesn't execute distill_run.py post-merge or verify it correctly skips completed batches.
**Evidence:** AC-3 verification is passive (file existence) not active (execution test).
**Recommendation:** Add acceptance step: run `python3 scripts/distill_run.py status` post-merge and verify correct counts. Run `next --count 1` and confirm it returns the correct next pending batch.

### [CRITICAL] No test of concat script before running on 833 batches
**Category:** missing-requirement
**Description:** scripts/distill_concat.py doesn't exist yet. No spec for testing it on sample data first.
**Evidence:** This is the most complex step and critical path.
**Recommendation:** Add S-1.5: test distill_concat.py on a 5-batch sample before running on full data. Verify row count, schema, label validity on sample output.

### [CRITICAL] Batch 0055 exclusion verified by file absence, not content
**Category:** assumption
**Description:** AC-4 checks .done doesn't exist — but doesn't verify 0055's rows aren't in the concatenated output by content inspection.
**Evidence:** Batch CSVs include a source column but no batch_id. If 0055 rows leaked, they'd look identical to valid rows.
**Recommendation:** The concat script should skip any batch CSV where the .done marker doesn't exist (not a hardcoded exclude list). This naturally excludes 0055 since its .done was deleted.

### [CRITICAL] Pipeline safety during background execution unclear
**Category:** constraint-conflict
**Description:** Spec says pipeline "continues in background" but deletes batch CSVs. Does distill_run.py reference batch CSVs or only .done markers?
**Evidence:** Constraint conflict between "delete batch CSVs" and "pipeline continues."
**Recommendation:** Clarify: distill_run.py only reads .done markers (safe). The background pipeline writes NEW batch CSVs which won't conflict with the concatenated file. Add explicit note to spec.

### [WARN] No duplicate detection in concatenation
**Category:** test-gap
**Description:** If a batch was re-processed, duplicates could exist. Row count check won't catch this.
**Evidence:** Pipeline had duplicate agent completions earlier in the session.
**Recommendation:** Add duplicate detection to distill_concat.py (by source_file + column_name key). Log and exclude duplicates.

### [WARN] Validation failure handling unspecified
**Category:** test-gap
**Description:** Spec says "rows failing validation are logged and excluded" but doesn't specify what happens if >N% of rows fail.
**Evidence:** If validation logic has a bug, it could silently exclude large amounts of valid data.
**Recommendation:** Add threshold: if >1% of rows excluded, fail and require manual review. Log all exclusions to distill_concat.log.

### [WARN] Transient file cleanup not precise
**Category:** missing-requirement
**Description:** S-3 mentions removing .claimed markers, gen_batch_*.py, etc. but doesn't specify exact cleanup.
**Evidence:** These files exist in output/distillation-v3/ but aren't part of the pipeline state.
**Recommendation:** List exact glob patterns for cleanup in S-3.

### [INFO] Rebase process not specified
**Category:** test-gap
**Description:** "Interactive rebase to single commit" doesn't specify exact command.
**Recommendation:** Specify: `git reset --soft main && git commit` (simpler than interactive rebase for pure squash).

---

## Honest Assessment

The spec is solid in intent but under-specified on the critical path: the concatenation script that doesn't exist yet. The biggest real risk is AC-3 (pipeline resume) — verifying it by file existence rather than actual execution could mask a silent failure post-merge. The batch 0055 exclusion is better handled by design (skip batches without .done) than by hardcoded exclude list. The background pipeline conflict is a non-issue if clarified — distill_run.py only reads .done markers. Overall: address the four critical findings (mostly clarifications and one test step) and this is ready to implement.
