# Implementation Progress

**Spec:** specs/2026-04-12-accuracy-gap-retraining/spec.yaml
**Started:** 2026-04-12

## Hard Constraints
- [ ] Mac Metal training only
- [ ] ReLU+BN architecture (decision 0046)
- [ ] Production-scale config: sherlock-v5-scaled-config.json (5 branch groups)
- [ ] 70/30 distillation:synthetic data mix
- [ ] Sibling-context enrichment during FTMB prep
- [ ] v10-style header validation (hard-fail on zero features)
- [ ] No actionability regression (>=96.5%)
- [ ] Backward compatible via serde defaults
- [ ] Hyperparameters: lr=1e-4, weight_decay=1e-4 (v4-sibling-proven)
- [ ] n_classes=239 (current taxonomy)
- [ ] Epoch cap: 40 with patience=10
- [ ] Sharpen rules, Model2Vec, sibling-context model frozen

## Acceptance Criteria
- [x] ac-01: Audit 34 misclassifications — 26 WRONG, 6 DEBATABLE, 0 AMBIGUOUS (2 extra DEBATABLE vs agent summary)
- [x] ac-02: Fix 6 DEBATABLE labels — geojson, git_sha, region/address, ip_v4/port, categorical/http_method, categorical/measurement_unit. Expected baseline: 199/227 (87.7%)
- [ ] ac-03: Prepare 70/30 FTMB, >=150/239 types with distilled data
- [ ] ac-04: FTMB real Model2Vec headers (non-zero validation)
- [ ] ac-05: Sibling-context enriched headers in FTMB
- [ ] ac-06: Train ReLU+BN on Mac Metal
- [ ] ac-07: val_accuracy >=88%
- [ ] ac-08: Profile eval >=205/227
- [ ] ac-09: Actionability >=96.5%
- [ ] ac-10: Misclassification delta analysis
- [ ] ac-11: Publish to HuggingFace (if ac-08 + ac-09 pass)
- [ ] ac-12: Update CLAUDE.md
