# Discovery: v12 Data Quality Audit

**Date:** 2026-04-16
**Interviewer:** Nightingale
**Card:** cards/0010-synthetic-data-generation.yaml (tangential)
**Mode:** discovery

---

## Context

v12 (5-branch with validation features) scored 204/227 (+3 over v11's 201/227).
The headline improvement masks significant churn: 11 fixes, 8 regressions. Three
regressions are at confidence 1.00 (phone->ssn x2, git_sha->tsid), meaning the
model is confidently wrong and Sharpen rules cannot override them. Additionally,
9 of 11 persistent errors increased in confidence in v12, suggesting the validation
branch may be reinforcing wrong patterns.

Prior art:
- specs/2026-03-26-overnight-v6-data-quality/collisions.md — 23 collision pairs identified
- specs/2026-04-12-accuracy-gap-retraining/eval-audit-v2.md — 34 misclassifications audited
- decision 0037 — eval GT can be updated when evidence contradicts
- decision 0038 — retraining over heuristic rules
- decision 0039 — use all adjudicated distillation rows

## Q&A

### Q1: Scope
**Q:** The v12 regressions fall into two buckets: (A) training data quality and
(B) eval ground truth. Which should this spec cover?
**A:** Both — full audit.

### Q2: Depth
**Q:** Should this be a hands-on investigation with per-item evidence, or a
structured framework for future investigation?
**A:** Hands-on audit. Actually inspect each misclassification and sample training
data for regressed types.

### Q3: Data sources
**Q:** Which data sources should the audit cover — upstream (distilled + generators),
FTMB feature vectors, or both?
**A:** Upstream: distilled CSV + generators. Get at root cause — are the types
genuinely confusable in the training data?

### Q4: Priority
**Q:** Should the audit prioritise high-confidence regressions, or cover all 23
misclassifications equally?
**A:** All 23 equally. Full audit of every v12 misclassification.

### Q5: GT authority
**Q:** If evidence shows a ground truth label is wrong, do we update GT in this
spec or flag for separate review?
**A:** Update GT in this spec. Decision 0037 already established this principle.

### Q6: Confidence analysis
**Q:** Should the audit investigate why persistent errors got more confident in v12?
**A:** Yes, include confidence analysis. Check validation features for persistent
errors to see if the validation branch is reinforcing wrong answers.

### Q7: Exit condition
**Q:** What outcome makes this spec complete?
**A:** Audit + GT fixes + retrain brief. All 23 audited with verdicts, GT corrections
committed, and a brief for a follow-up v13 retrain spec if training data issues found.

---

## Summary

### Goal
Audit all 23 v12 misclassifications to determine root cause (model error, training
data quality, or eval GT error), fix GT where justified, and produce a retrain brief
if training data issues are found.

### Constraints
- Inspect upstream data (distilled CSV values + generator output), not just FTMB features
- All 23 misclassifications audited equally — no triage by priority
- GT corrections committed directly (per decision 0037)
- Confidence analysis included for persistent errors that got more confident

### Success Criteria
- Per-item verdict for all 23 v12 misclassifications
- Root cause classification: model error / GT error / training data collision / debatable
- GT corrections committed to eval/schema_mapping.yaml where justified
- Adjusted v12 score after GT corrections
- Retrain brief with training data recommendations if issues found

### Decisions Surfaced
- GT update authority: update directly in this spec (extends decision 0037)
- Upstream-first investigation: check raw values before feature vectors
- Full coverage: no triage, all 23 get equal treatment

### Open Questions
- None — scope is clear, ready for spec
