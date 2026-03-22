# Spec Review: Overnight v2

**Date:** 2026-03-22
**Reviewer:** Context-separated agent (fresh session)
**Spec:** specs/2026-03-22-multi-branch-production/overnight-v2-spec.yaml
**Verdict:** REQUEST_CHANGES

---

## Findings

### [CRITICAL] 1. 300k math is wrong
**Category:** failure-mode
**Description:** `--samples-per-type 1200` does NOT produce 1200 columns per type. `finetype generate --samples 1200` generates 1200 *values* per type, chunked into ~12 columns of 100 values. The blend cap of 1200 won't help because only ~12 synthetic + distilled columns exist per type. V1 at `--samples-per-type 1500` produced 33k records total.
**Recommendation:** Specify the actual mechanism for 300k. Options: (A) pass `--samples N*100` to `finetype generate` to get N synthetic columns per type, or (B) add a `--synthetic-columns-per-type` parameter separate from the blend cap.

### [CRITICAL] 2. Hierarchical timing estimate is ~2× too optimistic
**Category:** failure-mode
**Description:** Ablation data: hier = 52.5s/epoch at 28.5k. At 300k (10.5×) and 20 epochs: 52.5 × 10.5 × 20 = 11,025s = 184 min, not the spec's "~90 min". Data prep (45) + flat (59) + hier (184) + eval (10) = 298 min vs 180-min budget.
**Recommendation:** Either remove hier from this run (align with parent spec M-7 conditionality), or set hier to 15 epochs with patience 7 and add epoch-1 abort if >10 min.

### [CRITICAL] 3. TUI default-on corrupts overnight log
**Category:** failure-mode
**Description:** Overnight script uses `exec > >(tee -a "$LOG_FILE") 2>&1`. TUI alternate screen writes ANSI escape codes into the log. OV2-4 doesn't specify `--no-tui` for overnight training invocations.
**Recommendation:** OV2-4 must add `--no-tui` to both training invocations in the overnight script.

### [WARN] 4. Eval results overwritten between models
**Category:** failure-mode
**Description:** `eval.sh` hardcodes output to `eval/eval_output/`. Second model's eval overwrites first's. "Eval scores recorded for both models" gate impossible.
**Recommendation:** Copy eval results to `models/<name>/eval/` after each eval run, or capture scores from log output.

### [WARN] 5. Memory peak ~3.4 GB at 300k records
**Category:** assumption
**Description:** `read_training_data` + `from_records` hold two copies simultaneously (~1.7 GB each). Plus model params + Adam state + Metal buffers on 16 GB M1 Pro.
**Recommendation:** Add memory estimate to known risks. Consider taking Vec by value in `from_records` to halve peak.

### [WARN] 6. Dry-run doesn't validate feature extraction
**Category:** test-gap
**Description:** `--dry-run` returns before calling `finetype extract-features`. The 300k gate can't be verified by dry-run alone.
**Recommendation:** Add a preflight sample extraction (10 types × 5 cols) before full run.

### [WARN] 7. No authoritative list of 29 non-canonical labels
**Category:** missing-requirement
**Description:** The 29 labels are mentioned in ablation-results.md with 2 examples but no full list. The remap table can't be verified against a reference.
**Recommendation:** Extract the full list from distilled data before implementing OV2-1.

### [WARN] 8. Parent spec conditionality conflict
**Category:** constraint-conflict
**Description:** Parent spec M-7 says hier is conditional on flat < 95%. This spec trains both unconditionally.
**Recommendation:** Explicitly note this diverges from parent spec as an exploratory decision.

### [INFO] 9. Tracing subscriber conflict with TUI
**Category:** assumption
**Description:** Global tracing subscriber is initialized before TUI. TUI mode needs a file-backed subscriber, but can't replace it mid-process without `reload::Handle`.
**Recommendation:** Spec should note TUI mode requires tracing redirect setup before `train_multi_branch` call.

---

## Honest Assessment

Two blockers are arithmetic errors: the 300k calculation is wrong (current code would produce ~33k again), and the hierarchical timing estimate is ~2× too optimistic. Both are fixable in the spec. The TUI-corrupts-log issue is a simple omission (add `--no-tui` to overnight script). Everything else is implementation detail that would surface as unpleasant surprises during the overnight run.
