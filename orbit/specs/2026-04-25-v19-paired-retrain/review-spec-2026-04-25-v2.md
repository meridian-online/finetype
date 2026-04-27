# Spec Review

**Date:** 2026-04-25
**Reviewer:** Context-separated agent (fresh session)
**Spec:** orbit/specs/2026-04-25-v19-paired-retrain/spec.yaml
**Verdict:** APPROVE

---

## Review Depth

| Pass | Triggered by | Findings |
|------|-------------|----------|
| 1 --- Structural scan | always | 1 |
| 2 --- Assumption & failure | Content signals: training data, model architecture | not triggered (no MEDIUM+ findings) |
| 3 --- Adversarial | not triggered | --- |

## Findings

### [LOW] AC-03 verification uses "structurally distinguishable" --- still subjective but acceptable
**Category:** test-gap
**Pass:** 1
**Description:** AC-03's verification ("produces values that are structurally distinguishable from their confused-neighbour type") is a human-judgement check. However, the spec now names all 6 confused-neighbour pairs explicitly (iso_8601_compact vs isbn/alphanumeric_id, ordinal vs month_year_full/abbreviated_month, etc.), making the comparison concrete. The real gate for datetime generator quality is downstream eval (AC-09), so this functions correctly as a pre-sweep sanity check rather than a hard criterion.
**Evidence:** AC-03 description lists the 6 pairs. Downstream AC-09 provides the objective measurement.
**Recommendation:** No change needed. Acceptable as-is.

---

## Prior Review Resolution

The v1.0 review (review-spec-2026-04-25.md) raised 3 MEDIUM and 3 LOW findings. The v1.1 spec addresses all of them:

| Prior finding | Resolution in v1.1 |
|---|---|
| AC-04 audit gate unscoped | Now specifies inline gate in overnight script, reusing v16 pattern (overnight_v16_retraining.sh lines 280-384), with explicit checks listed |
| Cherry-pick conflict surface | AC-01 now includes pre-flight `git diff --stat` and pause-and-reassess if conflicts touch inference logic |
| Partial-failure semantics | AC-09 now states: "a partial-seed architecture (fewer than 3 completed seeds) automatically fails this condition, no makeup runs" |
| AC-11/AC-12 sequencing | Both now say "Conditional on AC-09 PASS" with explicit fallback language for the failure case |
| Sharpen threshold interaction | Constraint 9 now notes: "if GELU+LN wins, a follow-up check of Sharpen threshold sensitivity against the new confidence distribution is warranted" |
| AC-03 subjective verification | Confused-neighbour pairs now named explicitly in description |

All prior findings resolved. No new MEDIUM or HIGH issues found.

## Gate-AC Verification Check

| AC | ac_type | Verification present | Non-placeholder | Length >= 20 | Result |
|---|---|---|---|---|---|
| ac-04 | gate | yes | yes | yes (131 chars) | PASS |
| ac-09 | gate | yes | yes | yes (119 chars) | PASS |
| ac-10 | gate | yes | yes | yes (111 chars) | PASS |
| ac-11 | gate | yes | yes | yes (107 chars) | PASS |

All gate ACs pass the deterministic verification check.

## Content Signal Scan

Training data changes (v4 corpus, TABLE_TEMPLATES, generator improvements) and architecture comparison (GELU+LN vs ReLU+BN) are present. However, the spec's handling of these signals is thorough:
- MADR 0066 gate provides a deterministic pass/fail for model promotion
- Three-way diff design isolates data vs architecture effects
- Overnight script failure recovery is explicitly scoped
- Sharpen interaction acknowledged as a follow-up concern

No content signals trigger Pass 2 escalation given the absence of MEDIUM+ structural findings.

---

## Honest Assessment

This spec is ready for implementation. The v1.1 revision addressed every finding from the prior review --- the audit gate is now scoped, partial-failure semantics are explicit, cherry-pick risk has a pre-flight check, and the conditional sequencing of post-sweep ACs is clear. The MADR 0066 hard gate provides rigorous, deterministic acceptance criteria that leave no room for narrative-based promotion. The three-way comparison design is clean: identical training data isolates architecture effect, and the winner-vs-v16 diff captures the combined effect. The biggest operational risk is the ~15-hour overnight run, but each run is independent and the script continues past failures, so a partial result still yields useful information. The only remaining subjective element (AC-03 generator quality) is correctly scoped as a sanity check with the real measurement deferred to AC-09.
