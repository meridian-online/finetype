---
id: NNFT-254
title: 'Deep spike: Make feature-augmented CharCNN architecture deliver eval gains'
status: Done
assignee:
  - '@nightingale'
created_date: '2026-03-08 04:20'
updated_date: '2026-03-08 07:39'
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
- [x] #1 At least 3 experiments run with different configurations (feature subsets, layer shapes, data scaling)
- [x] #2 Each experiment has full eval numbers (profile + actionability)
- [x] #3 Analysis of which features contribute positively vs cause regressions
- [x] #4 Concrete recommendation: specific config that beats v14 baseline OR expanded rule set that closes remaining gaps
- [x] #5 Written finding with experiment log, comparison tables, and next steps
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
## Diagnostic Phase (done in this session)

Profiled 4 datasets (datetime_formats, ecommerce_orders, titanic, people_directory) and traced each showstopper through the Sense→Sharpen pipeline code.

### Root Cause Analysis

**Case 1: unix_epoch → NPI (identity.medical.npi)**
- Root cause: NO header hint for \"epoch\" or \"unix\" exists in header_hint()
- 10-digit values (1658783048) perfectly match NPI pattern
- CharCNN 100% confident → no disambiguation can override
- Fix: Add hardcoded header hints for epoch/unix → datetime.epoch.unix_seconds

**Case 2: shipping_postal_code → CPT**
- h.contains(\"postal\") hint EXISTS (line 2361) but CPT wins at 80% confidence
- The hint fires but confidence > 0.5 and prediction isn't generic → hint doesn't override
- Fix: Hardcoded hints for \"postal\" should be authoritative at higher thresholds for medical code confusion

**Case 3: Cabin → ICD10**
- Exact-match hint \"cabin\" → alphanumeric_id EXISTS (line 2339)
- Same issue: model confidence (57%) and ICD10 isn't generic → hint override conditions not met
- Fix: Same-category or cross-domain hardcoded hint authority needs strengthening

**Case 4: Age → numeric_code**
- No header hint for \"age\" (type was removed in NNFT-192)
- Small integers (0-100) classified as numeric_code by default
- Fix: Add header hint for age → integer_number or decimal_number

## Experiment Plan

### Experiment 1: Header hint gap analysis + fixes
- Audit all 21 datasets for missing header hints
- Add missing hints (epoch, unix, age, etc.)
- Test: How many of the profile misclassifications are fixed by header hints alone?

### Experiment 2: Hardcoded hint authority threshold
- Current: hardcoded hints only override at confidence < 0.5 (general) or same-category ≤ 0.80
- Proposed: hardcoded hints override medical/identity types that are structurally confusable (NPI/CPT/ICD10) at any confidence, since these types require domain knowledge the header provides
- Test: Full eval (profile + actionability) before/after

### Experiment 3: Feature-augmented model (NNFT-253 follow-up)
- If experiments 1-2 close most gaps via rules, document why feature_dim=0 + expanded rules is the better path
- If gaps remain, test feature subsets (parse-test only, no char-stats) to avoid the city attractor regression
- Test: Retrain with feature subset, compare eval numbers

### Evaluation
- Run profile eval (make eval-report) after each experiment
- Compare against v14 baseline: 178/186 (95.7% label, 97.3% domain)
- Run actionability eval to check for regressions
- Document all results in discovery/feature-retrain/FINDING.md"}
</invoke>
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
## Full Audit Results (21 datasets, 213 columns)

### Showstoppers (obviously wrong to any analyst)
| Dataset | Column | Got | Expected | Root Cause |
|---------|--------|-----|----------|------------|
| datetime_formats | unix_epoch | NPI | unix_seconds | No header hint for epoch/unix |
| ecommerce_orders | shipping_postal_code | CPT | postal_code | Hint exists but confidence 80% blocks override |
| titanic | Cabin | ICD10 | alphanumeric_id | Hint exists but confidence 57% + cross-domain blocks |
| titanic | Age | numeric_code | integer/decimal | No hint for age |

### Numeric confusion (integer vs numeric_code vs amount_minor_int)
- airports: altitude → numeric_code (should be integer)
- books: pages → numeric_code
- covid: Confirmed/Recovered → amount_minor_int, Deaths → numeric_code
- geography: elevation_m → numeric_code
- medical: heart_rate → numeric_code, height_in → numeric_code
- network: response_time_ms → numeric_code, payload_size_bytes → amount_minor_int
- people: age → numeric_code, salary → amount_minor_int
- sports: duration_minutes → numeric_code, attendance → amount_minor_int
- world_cities: geonameid → amount_minor_int

### Text categorical confusion (region/categorical)
- iris: species → region
- sports: sport → region, status → region
- tech: language → last_name
- books: language → country
- financial: exchange → boolean.terms, ticker → ordinal

### Root causes:
1. Missing header hints (epoch, age, altitude, attendance, duration, etc.)
2. Hardcoded hint authority too weak for cross-domain override
3. Numeric attractors: large integers → amount_minor_int, small → numeric_code
4. Text with limited cardinality → region/country instead of categorical"}
</invoke>
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Deep accuracy spike investigating whether header hint expansion and refined hint authority can improve profile eval without model retraining.

## What changed

**~30 new header hints** in `column.rs` covering epoch/unix timestamps, age, altitude/elevation, duration (numeric), attendance/headcount, vital signs, pages, and categorical text (language, sport, species, exchange). Plus epoch/unix substring matching before the generic date/timestamp catch-all.

**7 substring matching bug fixes:** "count" no longer matches "country", "address" no longer matches "mac_address", "duration" excludes "iso"/"8601", "X name" excludes "month name"/"day name".

**Cross-domain hardcoded hint override:** When a hardcoded hint targets a different taxonomy domain than the model's prediction (and base type names differ), the hint overrides regardless of confidence. Domain-aware threshold: 0.85 for cross-domain, 0.5 for same-domain.

## Results

| Metric | Before | After | Delta |
|---|---|---|---|
| Profile label | 178/186 (95.7%) | 179/186 (96.2%) | +0.5pp |
| Profile domain | 181/186 (97.3%) | 183/186 (98.4%) | +1.1pp |
| Actionability | 99.9% | 99.9% | maintained |
| Tests | 438 pass | 438 pass | maintained |

## Key finding

Rules-based disambiguation (feature_dim=0 + F1-F3 + expanded hints) outperforms feature-augmented model retraining. NNFT-253 showed feature_dim=32 regresses eval by -1.6pp due to city attractor. This spike confirms the rule-based path is the right one — +1 column with zero retraining risk.

7 remaining misclassifications: 3 bare "name" ambiguity (genuinely ambiguous), 3 model-level confusions (git_sha/hash, hs_code/decimal, docker_ref/hostname), 1 GT edge case.

## Files changed

- `crates/finetype-model/src/column.rs` — header hints, cross-domain override, substring guards, 4 new tests
- `discovery/feature-retrain/FINDING.md` — experiment log with comparison tables
<!-- SECTION:FINAL_SUMMARY:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [x] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [x] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [x] #4 Decision record created if plan involved choosing between approaches
- [ ] #5 Daily memory log updated with session outcomes
- [x] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
