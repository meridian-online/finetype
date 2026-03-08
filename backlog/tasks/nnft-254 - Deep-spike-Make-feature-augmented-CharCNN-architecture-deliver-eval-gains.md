---
id: NNFT-254
title: 'Deep spike: Make feature-augmented CharCNN architecture deliver eval gains'
status: To Do
assignee: []
created_date: '2026-03-08 04:20'
labels:
  - discovery
  - model
  - m-12
dependencies:
  - NNFT-253
references:
  - discovery/feature-retrain/FINDING.md
  - crates/finetype-model/src/features.rs
  - crates/finetype-model/src/charcnn.rs
  - crates/finetype-model/src/column.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Question:** What combination of data generation, layer shapes, feature design, and post-vote rules can make the feature_dim>0 architecture outperform the current feature_dim=0 + F1-F3 rules approach?

**Context:** NNFT-253 showed that naively enabling feature_dim=32 causes a city attractor regression (-1.6pp profile). But the architecture is sound — the problem is in what we feed it and how. This spike explores multiple levers systematically:

1. **Data generation** — Is 1500 samples/type enough? Do we need harder negatives? Should confusable types get more samples?
2. **Layer shapes** — Should features go through a separate MLP before fusion? Should we use a gating mechanism (learned weight on features vs CNN)?
3. **Feature design** — Are the 14 char-stat features causing city attraction? Would a subset (only parse-test + structural) work better? Are there missing features that would help?
4. **More rules** — Can we extend F1-F3 with additional post-vote disambiguation rules for the remaining 5 model-level confusions?

**Time budget:** 1-2 days (multiple experiments, not a single train-eval cycle)

**Success:** A concrete plan (with data) for either adopting feature-augmented model OR expanding the rule set, backed by eval numbers.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 At least 3 experiments run with different configurations (feature subsets, layer shapes, data scaling)
- [ ] #2 Each experiment has full eval numbers (profile + actionability)
- [ ] #3 Analysis of which features contribute positively vs cause regressions
- [ ] #4 Concrete recommendation: specific config that beats v14 baseline OR expanded rule set that closes remaining gaps
- [ ] #5 Written finding with experiment log, comparison tables, and next steps
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [ ] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [ ] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [ ] #4 Decision record created if plan involved choosing between approaches
- [ ] #5 Daily memory log updated with session outcomes
- [ ] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
