# Spec Review

**Date:** 2026-04-25
**Reviewer:** Context-separated agent (fresh session)
**Spec:** .orbit/specs/2026-04-25-v19-paired-retrain/spec.yaml
**Verdict:** REQUEST_CHANGES

---

## Review Depth

| Pass | Triggered by | Findings |
|------|-------------|----------|
| 1 — Structural scan | always | 4 |
| 2 — Assumption & failure | Content signals: training data, model architecture, overnight infra | 3 |
| 3 — Adversarial | not triggered | — |

## Findings

### [MEDIUM] AC-04 gate verification assumes external tooling without specifying it
**Category:** test-gap
**Pass:** 1
**Description:** AC-04 verification says "Pre-training audit gate output shows ALL GATES PASSED" but the spec never defines what script produces this output. There is no deliverable for a pre-training audit script. The constraint list mentions "valid_dim=240, min type count >=50, total types >=239" but these checks don't exist today — they would need to be written or added to `prepare_multibranch_data.py`.
**Evidence:** AC-04 verification references "Pre-training audit gate output" but no deliverable or AC covers creating this gate. The deliverables list includes `prepare_multibranch_data.py` but only for "v4 corpus additions + container TABLE_TEMPLATES."
**Recommendation:** Either add an AC for the pre-training audit script (even if it's just a `--audit` flag on `prepare_multibranch_data.py`) or fold the audit checks explicitly into AC-04's verification as manual checks with specific commands to run.

### [LOW] AC-03 verification is subjective — "visually distinct"
**Category:** test-gap
**Pass:** 1
**Description:** AC-03's verification for datetime generator improvements relies on "values visually distinct from the confused-neighbour type." This is human-judgement-dependent and not repeatable. However, given that generator quality is inherently a judgement call and the real test is the downstream eval score (AC-09), this is acceptable as a pre-sweep sanity check rather than a hard gate.
**Evidence:** AC-03 verification: "cargo run -- generate <type> produces values visually distinct from the confused-neighbour type for each of the 6 subtypes."
**Recommendation:** No change required, but consider naming the 6 specific confused-neighbour pairs explicitly in the AC description so the reviewer knows exactly which comparisons to make (e.g., "iso_8601_compact vs iso_8601", "ordinal vs date").

### [MEDIUM] Cherry-pick strategy not specified — merge conflicts acknowledged but not scoped
**Category:** missing-requirement
**Pass:** 1
**Description:** AC-01 says "merge conflicts in .gitignore and generator.rs resolved" as if this is a known, bounded set. The interview notes this as an open question ("v4 branch rebase strategy — cherry-pick vs merge"). If the v4 branch has diverged significantly from main over the 5 days of active development since (PRs #44, #46, #47, #48), the conflict set may be larger than anticipated. The spec treats this as trivially resolvable but doesn't scope the risk.
**Evidence:** Constraint: "Cherry-pick from origin/distilled-data-relabel-7-types-v17." Interview open question: "v4 branch rebase strategy — cherry-pick vs merge (implementation detail)."
**Recommendation:** Add a pre-flight check: before starting AC-01, verify the cherry-pick target commits and estimate conflict surface. If conflicts touch inference logic (column.rs, multi_branch.rs), that should trigger a pause-and-reassess rather than silent resolution.

### [LOW] Content signal: training data changes + architecture experiment
**Category:** content-signal
**Pass:** 1
**Description:** This spec touches training data (new TABLE_TEMPLATES, generator improvements, v4 corpus adoption) and model architecture (GELU+LN vs ReLU+BN). Both are high-impact domains. Triggering Pass 2.
**Evidence:** AC-01 through AC-04 modify training data; AC-06/AC-07 introduce architecture comparison.
**Recommendation:** Proceed to Pass 2.

### [MEDIUM] Assumption: "GELU+LN activated via config — no architecture code changes needed"
**Category:** assumption
**Pass:** 2
**Description:** The spec and interview both assert that GELU+LN requires only config changes, not code changes. This is verified — `crates/finetype-model/src/multi_branch.rs` and `crates/finetype-train/src/multi_branch.rs` both contain `activation` and `use_layer_norm` support. However, the assumption that the training loop handles both architectures identically (same learning rate schedule, same batch norm momentum, same gradient behaviour) deserves scrutiny. LayerNorm and BatchNorm have different training-mode vs eval-mode semantics. If the training crate doesn't correctly handle LN's lack of running-mean/variance tracking, the GELU+LN runs could produce models that behave differently in training vs inference.
**Evidence:** MADR 0046 notes GELU+LN showed "higher val_accuracy (85.9% vs ~84%) — the benefit doesn't transfer through Sharpen." This suggests the model's confidence distribution changes under GELU+LN. The spec doesn't address whether Sharpen's fixed thresholds (which were tuned against ReLU+BN confidence distributions) need adjustment for GELU+LN output.
**Recommendation:** Add a note (not a full AC) acknowledging that if GELU+LN wins the gate, a follow-up check of Sharpen threshold sensitivity against the new confidence distribution is warranted. This doesn't block v19 but should be on the radar.

### [MEDIUM] Failure mode: overnight script partial failure leaves ambiguous state
**Category:** failure-mode
**Pass:** 2
**Description:** The spec says "script continues on failure and records which runs completed." AC-06 requires a summary table. But the post-sweep eval (AC-08/AC-09) assumes at least one complete 3-seed set per architecture. If, say, seed 43 of GELU fails, the MADR 0066 gate requires "3-seed sweep completed" (gate condition 1). A partial failure means that architecture automatically fails the gate — but the spec doesn't explicitly say this. The overnight script could complete 5/6 runs, the morning eval could show the surviving architecture passes, and someone might be tempted to run a single makeup seed rather than re-running the full architecture.
**Evidence:** MADR 0066 gate condition 1: "3-seed sweep completed: seeds 42, 43, 44 x 100 epochs each, all three with val_acc >= 0.912." Constraint: "Failure recovery: each run independent, script continues on failure."
**Recommendation:** Add an explicit statement to ac-09 or constraints: "A partial-seed architecture (fewer than 3 completed seeds) automatically fails gate condition 1. No makeup runs — re-run the full 3-seed set for that architecture if needed."

### [LOW] AC-11 and AC-12 are post-sweep — sequencing dependency implicit
**Category:** assumption
**Pass:** 2
**Description:** AC-11 (promotion) and AC-12 (CLAUDE.md update) only apply if AC-09 passes. The spec's exit conditions cover the "neither passes" case but the ACs themselves don't mark this conditional dependency. An implementer reading AC-11 cold might try to satisfy it even if AC-09 failed.
**Evidence:** Exit conditions: "Neither architecture passes gate -> HOLD both." AC-11 says "Winner promoted to models/default" without qualifying "if AC-09 passes."
**Recommendation:** Add "Conditional on AC-09 PASS" to AC-11 and AC-12 descriptions, or note this in a sequencing section. Minor — the exit conditions cover it, but explicit is better than implicit for overnight work.

---

## Honest Assessment

This is a well-structured spec for a moderately complex retrain sweep. The goal is clear, the MADR 0066 gate provides rigorous acceptance criteria, and the three-way comparison design cleanly isolates data vs architecture effects. The biggest risk is the pre-training audit gate (AC-04) referencing tooling that doesn't exist yet and isn't scoped as a deliverable — this could block the overnight run if the audit checks need to be written from scratch during implementation. The cherry-pick from a 5-day-old branch is a known unknown but manageable. The Sharpen-threshold interaction with GELU+LN confidence distributions is worth watching but shouldn't block the sweep — the whole point is to measure end-to-end through Sharpen.

The three MEDIUM findings are all addressable with minor spec edits (scope the audit gate, acknowledge partial-failure semantics, pre-flight the cherry-pick). None require rethinking the approach.
