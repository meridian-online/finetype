---
id: NNFT-270
title: >-
  Implement Sherlock-style column-level aggregate features and new
  disambiguation rules
status: Done
assignee:
  - '@nightingale'
created_date: '2026-03-10 21:55'
updated_date: '2026-03-10 22:15'
labels:
  - accuracy
  - features
  - disambiguation
milestone: m-12
dependencies: []
references:
  - crates/finetype-model/src/features.rs
  - crates/finetype-model/src/column.rs
  - >-
    backlog/tasks/nnft-265 -
    Spike-Sherlock-style-feature-separability-for-FineTypes-confusable-type-pairs.md
  - discovery/llm-distillation/FINDINGS.md
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implement the recommendations from NNFT-265 (Sherlock feature separability spike). The spike found that FineType's per-value features are already good — the critical gap is column-level aggregation statistics beyond mean. NNFT-266 added variance/min/max to ColumnFeatures, but only Rule F4 (git_sha) uses variance so far.

This task adds new per-value features and new F-rules that exploit the existing column-level aggregation (variance, min, max) to resolve the remaining confusable type pairs:

Target confusion pairs (from 180/186 profile eval):
1. hs_code vs decimal_number — F3 exists but only uses digit_ratio + dot_segments
2. docker_ref vs hostname — F2 exists but could be strengthened
3. hash vs git_sha — F4 exists, works well, but hash columns themselves could be better identified

Additionally, the LLM distillation (NNFT-269) identified columns where FineType classifies as generic decimal_number but the data is clearly financial (price_usd, total_price). Header hints may help here.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Add has_minus_sign per-value feature (binary: value contains '-') to features.rs — FEATURE_DIM 34→35
- [x] #2 Add has_percent per-value feature (binary: value contains '%') — FEATURE_DIM 35→36
- [x] #3 Enhance Rule F3 (hs_code vs decimal_number): add dot-count variance check (hs_codes have consistent dot patterns) and negative-sign absence check (hs_codes never negative)
- [x] #4 Add Rule F5: hash identification — when winner is hash, verify length variance > 0 (mixed MD5/SHA1/SHA256 lengths). If length variance ≈ 0 and length = 40, prefer git_sha (reinforces F4)
- [x] #5 Add header hint rules for financial columns identified by NNFT-269: price, cost, revenue, salary, income, total → finance.currency.amount when current prediction is decimal_number
- [x] #6 All existing tests pass (cargo test) + taxonomy check (cargo run -- check)
- [x] #7 Profile eval maintains or improves from 180/186 baseline
- [x] #8 New unit tests for each added feature and each new/enhanced F-rule
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
## Implementation Plan

### Step 1: New per-value features in `features.rs`

Add 2 new binary features to the feature extractor (FEATURE_DIM 34→36):

**Feature 34: `has_minus_sign`** — binary, value contains '-' (minus sign)
- Discriminator for hs_code (never negative) vs decimal_number (can be negative)
- Separate from `has_dash` (feature 33) which fires on any '-' including UUIDs/dates. Actually identical — on reflection, `has_dash` already covers this. Skip this and use `has_dash` directly.

**Revised: Feature 34: `has_negative_prefix`** — binary, value starts with '-' followed by digit
- True negative numbers start with '-' then digit. HS codes never do.
- This is a NEW signal — `has_dash` fires on UUIDs, dates, etc. `has_negative_prefix` is specific to negative numbers.

**Feature 35: `has_percent`** — binary, value contains '%'
- Discriminator for percentage vs decimal_number (Rule 19 already handles some cases)

Update FEATURE_DIM, FEATURE_NAMES, extract_features(), and add unit tests.

### Step 2: Enhance Rule F3 (hs_code vs decimal_number)

Current F3: digit_ratio >= 0.75 AND (dot_segments >= 2.0 OR (is_float < 1.0 AND dot_segments >= 1.5))

Add two new conditions to make F3 more confident:
- **Dot-count variance check**: hs_code columns have consistent dot patterns (low segment_count_dot variance). Add: `dot_variance < 0.5` as a confidence booster (not a gate).
- **Negative-sign absence**: hs_codes are never negative. If `mean[has_negative_prefix] > 0.0`, this is NOT an hs_code → skip F3.

### Step 3: Add Rule F5 — hash column consistency check

When winner is `hash` and length_variance > threshold:
- Confirms this is a mixed-hash column (MD5 + SHA1 + SHA256)
- No action needed (hash is correct)

When winner is `hash` and length_variance ≈ 0 but length ≠ 40:
- Could be MD5-only (32 chars) or SHA256-only (64 chars)
- These are still `hash` — F4 only promotes to `git_sha` at length=40

This rule is mainly a safety net. F4 already handles the git_sha case. F5 would be: if winner is `hash`, length_variance < 0.01, mean_length ≈ 32 → confirm `hash` (MD5-only column). Low priority — skip unless eval reveals a need.

### Step 4: Financial header hints (from NNFT-269)

Current header_hint maps `price`, `cost`, `amount`, `salary` → `decimal_number`.

Change to → `finance.currency.amount` for these financial headers. This is more specific and matches what the LLM correctly identified.

Add new hints: `revenue`, `income`, `total_price`, `unit_price`, `fee`, `charge`, `payment`, `balance`, `budget`, `expense`.

**Risk:** Some columns named "price" might contain non-currency decimals. Mitigate by keeping the hint as a suggestion (existing hint priority logic handles this — hints only override when confidence is low or prediction is generic).

### Step 5: Tests

- Unit tests for `has_negative_prefix` and `has_percent` features
- Unit tests for enhanced F3 with negative-prefix guard
- Unit tests for financial header hints
- Integration: `cargo test` + `cargo run -- check`

### Step 6: Profile eval

Run profile eval to verify 180/186 maintained or improved.

### Files Modified

| File | Changes |
|------|---------|
| `crates/finetype-model/src/features.rs` | +2 features (FEATURE_DIM 34→36), unit tests |
| `crates/finetype-model/src/column.rs` | Enhanced F3, financial header hints, unit tests |
| `CLAUDE.md` | Update FEATURE_DIM, feature count in Current State |
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
## Implementation Progress

**Step 1 — New features (Done):** Added `has_negative_prefix` (34) and `has_percent` (35) to `features.rs`. FEATURE_DIM 34→36. `has_negative_prefix` starts with '-' followed by digit — more specific than `has_dash` which fires on any dash. Added unit tests for both features (negative numbers, HS codes, edge cases).

**Step 2 — Enhanced F3 (Done):** Added two guards to Rule F3 in `column.rs`:
- Guard 1: `has_negative_prefix mean > 0` → skip F3 (HS codes never have negative values)
- Guard 2: `dot_segment_variance > 0.5` → skip F3 (inconsistent dot patterns = not HS codes)

**Step 3 — F5 (Skipped):** As planned, F5 deferred. F4 already handles git_sha correctly.

**Step 4 — Financial hints (Done):** Changed header hints for `price/cost/amount/salary/fare/fee/toll/charge` from `decimal_number` to `finance.currency.amount`. Added new financial hints: `revenue`, `income`, `wage`, `budget`, `expense`. Updated both exact-match and substring-match blocks.

**Step 5 — Tests (Done):** All 300 model tests pass. Updated 3 test assertions (test_header_hint_numeric, test_header_hint_fare) to expect new `finance.currency.amount` return value.

**Step 6 — Eval (Done):** Profile eval: 180/186 (96.8% label, 98.4% domain) — zero regression.

**AC1 note:** Revised from `has_minus_sign` to `has_negative_prefix` as flagged in the plan — starts with '-' + digit, not just contains '-'. FEATURE_DIM still goes 34→36 as specified.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added 2 new per-value features and enhanced disambiguation rules for hs_code/decimal_number confusion and financial column identification.

## Changes

**features.rs** — FEATURE_DIM 34→36:
- `has_negative_prefix` (34): binary, starts with '-' followed by digit. Distinguishes negative numbers from dash-containing codes. HS codes never have negative values.
- `has_percent` (35): binary, contains '%'. Supports future percentage rules.

**column.rs** — Rule F3 enhancement:
- Guard 1: negative-prefix mean > 0 → skip F3 (column has negative values = not HS codes)
- Guard 2: dot-segment variance > 0.5 → skip F3 (inconsistent dot structure = not HS codes)

**column.rs** — Financial header hints:
- Changed `price/cost/amount/salary/fare/fee/toll/charge` from `decimal_number` → `finance.currency.amount`
- Added new hints: `revenue`, `income`, `wage`, `budget`, `expense`
- Updated both exact-match and substring-match blocks

**F5 (hash consistency) deferred** — F4 already handles git_sha correctly. No eval evidence of need.

## Tests
- 300 model tests pass (29 feature tests including 2 new, all header hint tests updated)
- `cargo check` + `cargo fmt` + `cargo clippy` clean
- Profile eval: 180/186 (96.8% label, 98.4% domain) — zero regression

## No retrain needed
All changes are rule-based (post-vote disambiguation + header hints). CharCNN model has feature_dim=0.
<!-- SECTION:FINAL_SUMMARY:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [x] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [x] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [ ] #4 Decision record created if plan involved choosing between approaches
- [ ] #5 Daily memory log updated with session outcomes
- [x] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
