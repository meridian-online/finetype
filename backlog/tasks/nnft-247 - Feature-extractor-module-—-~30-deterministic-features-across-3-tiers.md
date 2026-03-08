---
id: NNFT-247
title: Feature extractor module — ~30 deterministic features across 3 tiers
status: Done
assignee:
  - '@nightingale'
created_date: '2026-03-07 23:55'
updated_date: '2026-03-08 00:14'
labels:
  - model
  - architecture
milestone: m-12
dependencies: []
references:
  - crates/finetype-model/src/char_cnn.rs
  - crates/finetype-core/src/lib.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Design and implement a feature extraction module that computes ~30 deterministic features per input string value. Features are organized in 3 tiers:

**Tier 1 — Parse tests (binary):** is_numeric, is_integer, is_float, is_date_parseable, is_uuid_like, has_leading_zero, is_email_like, has_at_sign, has_protocol_prefix, is_json, is_hex, matches_phone_pattern, etc.

**Tier 2 — Character stats (counts/ratios):** alpha_count, digit_count, symbol_count, uppercase_count, lowercase_count, space_count, punctuation_count, digit_ratio, alpha_ratio, uppercase_ratio, max_run_length, unique_char_count, string_length, etc.

**Tier 3 — Structural (pattern-derived):** segment_count (split by common delimiters), delimiter_type (dominant separator), length_bucket, character_class_pattern_hash, prefix_pattern, has_mixed_case, etc.

This module lives in finetype-core or finetype-model and exposes a `extract_features(value: &str) -> Vec<f32>` API. Features must be deterministic, reproducible, and fast (<0.1ms per value).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Feature extractor produces a fixed-size f32 vector (~30 dimensions) per input string
- [x] #2 All 3 tiers implemented: parse tests (binary 0/1), character stats (counts/ratios), structural (pattern-derived)
- [x] #3 Features are fully deterministic — same input always produces same output
- [x] #4 Feature extraction runs in <0.1ms per value (benchmark test)
- [x] #5 Leading-zero detection included as explicit feature (critical for numeric_code vs postal_code)
- [x] #6 Unit tests cover representative values from each domain (datetime, identity, geography, finance, representation, technology)
- [x] #7 Feature names/indices are documented for interpretability
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
## Implementation Plan

### Location
New module `crates/finetype-model/src/features.rs`, exported from `lib.rs`.
Lives in finetype-model (not core) since it's consumed by the model pipeline.

### Feature Design — 32 features across 3 tiers

**Tier 1 — Parse tests (10 binary features, 0.0 or 1.0):**
0. `is_numeric` — parseable as f64
1. `is_integer` — parseable as i64
2. `is_float` — contains '.' and parseable as f64
3. `has_leading_zero` — starts with '0' followed by digit (critical for numeric_code)
4. `has_at_sign` — contains '@' (email signal)
5. `has_protocol_prefix` — starts with http://, https://, ftp://, etc.
6. `is_uuid_like` — matches 8-4-4-4-12 hex pattern
7. `is_hex_string` — all chars are hex digits (0-9, a-f, A-F)
8. `has_iso_date_sep` — contains 'T' between digits (ISO 8601 signal)
9. `matches_phone_pattern` — starts with + followed by digits, or has parenthesized area code

**Tier 2 — Character stats (14 count/ratio features, normalized):**
10. `length` — string length (f32, raw)
11. `digit_count` — count of digit chars
12. `alpha_count` — count of alphabetic chars
13. `uppercase_count` — count of uppercase chars
14. `lowercase_count` — count of lowercase chars
15. `space_count` — count of whitespace chars
16. `symbol_count` — count of non-alphanumeric, non-space chars
17. `digit_ratio` — digit_count / length (0.0 if empty)
18. `alpha_ratio` — alpha_count / length
19. `uppercase_ratio` — uppercase_count / max(alpha_count, 1)
20. `unique_char_ratio` — unique chars / length
21. `max_digit_run` — longest consecutive digit sequence
22. `max_alpha_run` — longest consecutive alpha sequence
23. `punctuation_density` — symbol_count / max(length, 1)

**Tier 3 — Structural (8 pattern-derived features):**
24. `segment_count_dot` — split('.').count()
25. `segment_count_dash` — split('-').count()
26. `segment_count_slash` — split('/').count()
27. `segment_count_space` — split on whitespace count
28. `has_mixed_case` — both upper and lower present (1.0/0.0)
29. `starts_with_digit` — first char is digit
30. `ends_with_digit` — last char is digit
31. `length_bucket` — log2(length+1) for scale-invariant length signal

### API
```rust
pub const FEATURE_DIM: usize = 32;
pub const FEATURE_NAMES: [&str; FEATURE_DIM] = [...];
pub fn extract_features(value: &str) -> [f32; FEATURE_DIM]
```

Fixed-size array for compile-time guarantees. FEATURE_NAMES array for interpretability/debugging.

### Tests
- Unit tests for each tier with representative values (email, date, UUID, phone, number, address, etc.)
- Determinism test: same input → same output
- Benchmark test: 10k values in <1 second (0.1ms/value)
- Leading-zero test: explicit coverage for numeric_code patterns

### Files changed
1. `crates/finetype-model/src/features.rs` (NEW) — feature extractor implementation
2. `crates/finetype-model/src/lib.rs` — add `pub mod features` + re-export
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
- All 25 feature tests pass, 279 workspace tests pass, 1 doc test pass
- Clippy clean (-D warnings), fmt applied
- Fixed test_representation_value: #FF5733 has no lowercase so has_mixed_case=0.0
- Fixed 3 clippy lints: map_or→is_some_and, manual strip_prefix
- Performance: 10k extractions in 0.17s (0.017ms/value — 6x under budget)"
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added deterministic feature extraction module for text values — foundation for feature-augmented CharCNN (m-12).

## Changes
- **New module** `crates/finetype-model/src/features.rs` — 32 features across 3 tiers:
  - Tier 1 (10 parse tests): is_numeric, is_integer, is_float, has_leading_zero, has_at_sign, has_protocol_prefix, is_uuid_like, is_hex_string, has_iso_date_sep, matches_phone_pattern
  - Tier 2 (14 char stats): length, digit/alpha/upper/lower/space/symbol counts, digit/alpha/uppercase/unique_char ratios, max digit/alpha runs, punctuation density
  - Tier 3 (8 structural): segment counts by dot/dash/slash/space, has_mixed_case, starts/ends_with_digit, length_bucket (log2)
- **API**: `extract_features(value: &str) -> [f32; 32]` — fixed-size, zero-allocation (except unique char HashSet), deterministic
- **Constants**: `FEATURE_DIM = 32`, `FEATURE_NAMES` for interpretability
- **Re-exported** from `finetype-model` crate root

## Tests
- 25 unit tests covering all 3 tiers, cross-domain representative values (datetime, identity, geography, finance, technology, representation), determinism, and performance
- Performance: 10k extractions in 0.17s (0.017ms/value — 6x under 0.1ms budget)
- Leading-zero detection explicitly tested for numeric_code patterns (007, 0123, 00501)
- Full workspace: 279 tests + 1 doc test pass, zero regressions"
<!-- SECTION:FINAL_SUMMARY:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [x] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [ ] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [ ] #4 Decision record created if plan involved choosing between approaches
- [ ] #5 Daily memory log updated with session outcomes
- [ ] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
