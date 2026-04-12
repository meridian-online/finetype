# Design: Accuracy Gap Retraining

**Date:** 2026-04-12
**Interviewer:** Nightingale
**Card:** cards/0002-semantic-type-detection.yaml

---

## Context

Card: *Semantic type detection* — 4 scenarios, goal: close accuracy gap via data quality + retraining
Prior specs: 5 — shipped multi-branch pipeline (155/190), autoresearch (96.6% val on PyTorch), GELU+LN port (closed: -5 labels). Eval ground truth fixed, set expanded 214 → 227.
Gap: 193/227 (85.0%) → target 205/227 (~90%). 34 misclassifications remaining in 6 clusters.

## Evidence Summary

### Current baseline (v4-sibling on corrected eval)
- Label: 193/227 (85.0%)
- Domain: 206/227 (90.7%)
- Actionability: 96.9%

### Misclassification clusters (34 total)
| Pattern                          | Count | Key examples                                   |
|----------------------------------|-------|-------------------------------------------------|
| Numeric confusion (decimal/int)  | 8     | decimal→latitude/version, int→postal_code       |
| Hash/ID confusion                | 5     | hash→git_sha, alphanumeric_id→isbn/geohash      |
| Phone → wrong ID format          | 3     | phone→ssn, phone→abn                            |
| Date format confusion            | 3     | iso→dmy_dash/mdy_dash, eu_dot→iso_8601          |
| Free-text misrouting             | 3     | user_agent→jwt/mdy_12h/plain_text               |
| Type-specific single errors      | 12    | geojson→json, isin→alphanumeric_id, etc.         |

### Key prior findings
- Decision 0046: GELU+LN regresses -5 labels vs ReLU+BN. Architecture is settled.
- Decision 0038: Retraining > new rules. Data quality is the lever.
- Autoresearch: weight_decay=0.01, lr=1e-3 validated on base architecture.
- v10 FTMB prep pipeline has header validation (hard-fails on zero features).
- Sherlock distillation: 102k annotations, 74.6% coverage. 20× current training set.

## Q&A

### Q1: Goal recalibration
**Q:** The card goal says '170+/190' but the eval now has 227 columns and scores 193/227 (85%). Should we recalibrate the goal to the new eval set?
**A:** Recalibrate to 227. The old 190-column eval had debatable ground truth that's now fixed. Work against the current eval set.

### Q2: Approach selection
**Q:** Which path to prioritise — retrain on better data, misclassification surgery, or staged?
**A:** Retrain on better data. Consistent with decision 0038 (retraining > new rules).

### Q3: Data mix
**Q:** What mix of synthetic vs distilled training data?
**A:** 70/30 distillation-heavy (Nightingale recommendation). Real-world data captures confusable patterns naturally. Synthetic fills the 25% coverage gap for types missing from real datasets.

### Q4: Sibling-context enrichment
**Q:** Should we add sibling-context enrichment during FTMB preparation?
**A:** Yes — enrich during prep with frozen sibling model weights. Maintains parity with v4-sibling training.

### Q5: Target accuracy
**Q:** What's the target on the 227-column eval?
**A:** 205/227 (~90%). Requires closing ~12 of 34 failures. Focus on numeric confusion (8) and phone→ssn (3) clusters.

### Q6: Publish policy
**Q:** If the new model meets 205/227, should it replace v4-sibling as the HuggingFace default?
**A:** Yes — publish to HuggingFace if ≥205/227 with no actionability regression.

### Q7: Eval ground truth quality
**Q:** Should we audit the 34 remaining failures before retraining?
**A:** Quick audit (30 min) — classify each as WRONG/DEBATABLE/AMBIGUOUS. Fix debatable labels before retraining to avoid optimising against bad ground truth.

---

## Summary

### Goal
Close accuracy gap from 193/227 (85.0%) → 205/227 (~90%) on the corrected 227-column eval set via retraining on distillation-heavy data.

### Constraints
- Mac Metal training (no cloud GPU — RunPod zombie protection not yet fixed)
- ReLU+BN architecture (decision 0046 — GELU+LN rejected)
- Sibling-context enrichment during FTMB prep (frozen weights)
- v10 pipeline with header validation (hard-fail on zero features)
- 70/30 distillation:synthetic data mix
- No actionability regression (currently 96.9%)

### Success Criteria
- Profile eval ≥205/227 (~90%) label accuracy
- No actionability regression
- Publish to HuggingFace as new default model

### Decisions Surfaced
- **Recalibrate goal to 227-column eval**: chose new eval set over stale 190, because ground truth improved and eval set expanded
- **Retrain over rules**: chose data quality path over Sharpen surgery, because decision 0038 established this principle
- **70/30 distillation-heavy mix**: chose real-world-dominant mix over 50/50 or fresh distillation, because confusable patterns exist naturally in real data
- **Pre-retraining eval audit**: chose quick audit before retraining over trusting current GT, because optimising against bad ground truth wastes training cycles

### Open Questions
- Exact number of training epochs (v4-sibling used 30, autoresearch sweet spot was ~27-30)
- Whether the 102k Sherlock annotations need re-filtering (74.6% coverage — some annotations may be low quality)
- How to handle the 12 single-instance type-specific errors (too diverse for a single fix)
