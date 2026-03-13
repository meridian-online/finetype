# Interview: Sense Mask Numeric Misrouting

**Date:** 2026-03-13
**Interviewer:** Nightingale

---

## Context

The earthquake dataset's `horizontalError` column (100% decimal numbers like `9.68`, `14.54`) is classified as `representation.boolean.initials` at 1% confidence, causing a `CAST(horizontalError AS BOOLEAN)` failure in DuckDB.

Root cause chain: Sense classifier predicts "Text" for the column (likely influenced by sibling context), the Text mask blocks all numeric types, CharCNN's `decimal_number` votes are discarded, and `boolean.initials` (text-eligible) wins from a tiny vote fraction. The safety valve (fallback when votes are mostly masked out) apparently didn't fire despite nearly 100% of votes being ineligible.

## Interview Q&A

### Q1: Root cause
**Q:** The immediate symptom is 'numeric column misrouted to Text'. But is this a Sense classifier accuracy problem, a mask rigidity problem, or a safety valve problem?
**A:** "I don't know — we'd have to run some tests." The user wants to investigate before assuming the root cause.

### Q2: Instrumentation
**Q:** Should we instrument the pipeline to see what's actually happening before designing a fix?
**A:** Yes, instrument first. Understand the exact failure path before fixing.

### Q3: Tracing scope
**Q:** Should instrumentation be permanent (behind `--verbose` or `RUST_LOG`) or a one-off diagnostic?
**A:** Permanent verbose tracing. This pipeline has been a recurring debugging target.

### Q4: Trace points
**Q:** Which decision points should we instrument? (Sense prediction, vote aggregation, mask application, header hints, feature rules F1–F6, final result)
**A:** All six points. Full pipeline visibility.

### Q5: PR strategy
**Q:** Should the fix ship in the same PR as the tracing, or separately?
**A:** Two PRs: merge tracing first, then open a fix PR informed by the trace output.

### Q6: Activation mechanism
**Q:** `RUST_LOG=debug`, `--verbose` CLI flag, or both?
**A:** Both. `tracing::debug!` calls activated by RUST_LOG, plus a `--verbose` flag on profile/load commands that sets the filter automatically.

---

## Summary

### Goal
Add permanent tracing instrumentation to the Sense→Sharpen column classification pipeline, then use it to diagnose and fix the `horizontalError` misclassification bug.

### Constraints
- Two separate PRs: tracing instrumentation first, bug fix second
- Use `tracing::debug!` with RUST_LOG activation + `--verbose` CLI flag
- Instrument all 6 pipeline decision points
- Tracing stays in the codebase permanently

### Success Criteria
- `RUST_LOG=finetype_model=debug finetype profile -f earthquakes_2024.csv` shows the full decision trail for every column
- `finetype profile -f earthquakes_2024.csv --verbose` produces the same output without RUST_LOG
- Trace output reveals why the safety valve didn't fire for horizontalError
- Follow-up fix PR resolves the BOOLEAN cast failure

### Open Questions
- Why didn't the safety valve fire? (nearly 100% of votes should have been masked out)
- Is the root cause Sense accuracy, mask rigidity, or safety valve threshold?
- Does sibling context contribute to the misrouting?

### Decisions to Record
- Decision 0035: Add permanent pipeline tracing (tracing::debug! + --verbose flag)
