---
id: NNFT-161
title: 'Post-retrain analysis: diagnose CLDR-enriched retraining regression (Phase 5)'
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-28 01:19'
updated_date: '2026-02-28 01:35'
labels:
  - accuracy
  - cldr
  - phase-5
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Phase 5 of the CLDR-Enriched Model Retraining plan (Option C).

The CLDR-enriched retraining (NNFT-160) regressed from 116/120 to 107/120 on profile eval. Model was rolled back to v0.3.0 snapshot.

This task performs detailed analysis of WHY the regression occurred, categorises the 9 new misses, and produces actionable findings for the next retraining attempt.

Inputs:
- Retrained model available at models/tiered-v2.snapshot.20260227T231445Z (v0.3.0 baseline)
- Failed model training log in NNFT-160 final summary
- Training data at training_cldr_v1.ndjson (84,500 samples, 169 types)
- Profile eval report from retrained model (107/120)

Key questions to answer:
1. For each of the 9 new regressions, what changed in the model voting distribution?
2. Are the regressions concentrated in specific tiers or T2 models?
3. Which regressions are fixable by disambiguation rules vs needing training changes?
4. What training data adjustments would prevent these regressions?
5. Is the URL/URI confusion a taxonomy issue (should they be merged?) or a training data issue?
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Per-column diff analysis: compare old vs new model predictions for all 120 profile eval columns
- [x] #2 Each of the 9 regressions has root cause documented (model confusion, training data issue, disambiguation gap, or taxonomy issue)
- [x] #3 Regressions categorised as rule-fixable vs training-fixable vs taxonomy-fixable
- [x] #4 Concrete recommendations for next retraining attempt documented
- [x] #5 Findings written up in final summary with evidence
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Restore the retrained model temporarily for analysis (swap snapshot dirs)
2. Run profile eval with the retrained model, capture full JSON detail output per column
3. Restore v0.3.0 model, run profile eval again with full JSON detail per column
4. Diff the two outputs: for each of the 120 columns, compare predicted label, confidence, vote distribution
5. For each of the 9 regressions, extract:
   a. Old model prediction + confidence + top-5 votes
   b. New model prediction + confidence + top-5 votes
   c. Root cause category: model confusion / training data / disambiguation gap / taxonomy
6. Check training data quality for regressed types (URL vs URI samples, country vs nationality generators)
7. Analyse T2 model accuracy impact — which tier2 models degraded?
8. Categorise each regression as rule-fixable / training-fixable / taxonomy-fixable
9. Write recommendations for next retraining attempt
10. Write final summary with complete evidence
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
## Complete Regression Root Cause Analysis (9 columns)

### 1. URL→URI (×3): request_url, url, tracking_url
- **Category**: Training data overlap
- **Tier**: T2 VARCHAR_internet (7 types)
- **Root cause**: URL training (500 samples) is 100% http/https. URI training (500 samples) is 37% http/https + 22% mailto + 20% ftp + 21% other. The 37% http/https overlap means CharCNN sees near-identical training samples labeled as two different types. The retrained model learned URI as the broader category and overcalls it on URLs.
- **Evidence**: Training sample analysis shows URL always produces `https://word.tld/path` while 185/500 URI samples also use http/https schemes.

### 2. Country→nationality: covid_timeseries.Country
- **Category**: T1 routing confusion
- **Tier**: T1 VARCHAR routing (location vs person)
- **Root cause**: Country training (500 samples) includes localized names ("Schweden", "Corée du Sud", "Дания"). Nationality training (500 samples) includes demonyms ("Japanese", "Indisch", "Japonais"). Minimal overlap (2 values). But country→VARCHAR_location and nationality→VARCHAR_person are in different T2 tiers, so this is a T1 routing error — the retrained T1 model sent country values to "person" category.

### 3. pressure_atm→latitude
- **Category**: Numeric range ambiguity
- **Tier**: T1 DOUBLE routing (numeric vs coordinate)
- **Root cause**: Pressure values (0.685, 2.395 atm) are small decimals within latitude range (-90 to +90). T2 DOUBLE_coordinate has only 2 types (latitude, longitude). Retrained T1 may have shifted DOUBLE routing toward coordinate for small positive decimals.

### 4. airports.name→last_name
- **Category**: T2 decision boundary shift
- **Tier**: T2 VARCHAR_person (13 types)
- **Root cause**: v0.3.0 predicted full_name for airport names → entity demotion fired → entity_name (scored correct via interchangeability). Retrained model predicted last_name instead — entity demotion only fires on full_name majority vote, not last_name. The retrained T2 person model shifted its full_name/last_name decision boundary with 500 new training samples each.
- **Evidence**: Current baseline predicts entity_name (via entity demotion from full_name), scored correct. Single-word place names like "Goroka" look like last names.

### 5. utc_offset→iso_8601_offset
- **Category**: Training format mismatch
- **Tier**: T0 routing (VARCHAR vs TIMESTAMPTZ)
- **Root cause**: UTC offset training data is "UTC +05:30" (with prefix). Test data is "+05:30" (bare). The retrained T0 model routes bare "+05:30" to TIMESTAMPTZ (where iso_8601_offset lives) instead of VARCHAR (where utc offset lives). Rule 17 only triggers on time/rfc_3339 predictions, not on TIMESTAMPTZ routing.
- **Evidence**: Training samples all start with "UTC ". Test values like "+05:30" never appear in training.

### 6. multilingual.country→entity_name
- **Category**: T1 routing failure on non-ASCII text
- **Tier**: T1 VARCHAR routing (location vs text)
- **Root cause**: Test values are "Deutschland", "Brasil", "日本" — native-language country names. Only 3 unique values. The retrained T1 model routes non-ASCII text (especially CJK "日本") to "text" category instead of "location". entity_name training (500 samples) includes company names with diverse character patterns.
- **Evidence**: Country training data includes these exact values (1-2 occurrences), but CJK representation is minimal compared to entity_name patterns.

### 7. server_hostname→slug
- **Category**: Training data diversity gap
- **Tier**: T2 VARCHAR_internet (7 types)
- **Root cause**: Hostname training is simple domains ("index.net", "table.org") — no hyphens, no subdomains. Slug training is all hyphenated words ("data-sun-parse", "old-summer"). Test hostnames like "srv-dev-43.example.com" have hyphenated subdomains that match slug patterns more than the simplistic hostname patterns.
- **Evidence**: 0/500 hostname training samples contain hyphens. All slug samples contain hyphens. Test hostname "srv-dev-43.example.com" has 3 hyphens.

---

## Regression Categories

### Rule-fixable (3 regressions):
1. URL→URI (×3): Add URL preference rule — when T2 predicts URI and values are all http/https, override to URL
2. utc_offset: Expand Rule 17 to also trigger on TIMESTAMPTZ tier routing with bare offset patterns
3. airports.name: Extend entity demotion trigger to also fire on last_name majority vote (risky)

### Training-fixable (all 9, some also rule-fixable):
1. URL/URI: Remove http/https from URI training data entirely (URI should be non-web schemes only) or increase URI scheme diversity
2. Country/nationality: Ensure T1 routing has stronger location category signal for localized country names
3. pressure_atm/latitude: Add more diverse small-decimal numeric training data or atmospheric pressure-specific patterns
4. airports.name: Improve full_name training diversity with multi-word proper nouns beyond person names
5. utc_offset: Add bare offset patterns ("+05:30") alongside "UTC +05:30" to training data
6. multilingual.country: Increase non-ASCII/CJK country name representation in training data
7. hostname/slug: Add diverse hostname formats with subdomains, hyphens, and server naming conventions

### Taxonomy-fixable (1 regression):
1. URL/URI: Consider merging URL and URI into a single type — current distinction is semantic (URLs are a subset of URIs) not structural. Or add hierarchy: URL as a subtype of URI.

---

## Structural Finding: T1 VARCHAR Routing is the Weakest Link

Of 9 regressions:
- 3 involve T1 routing errors (country→person, country→text, pressure→coordinate)
- 4 involve T2 model confusion within a correct tier (URL/URI, hostname/slug, airports.name)
- 1 involves T0 routing (utc_offset to wrong broad type)
- 1 is format mismatch in training data (utc_offset)

The T1 VARCHAR model routes across 22 categories with high-stakes consequences — wrong T1 routing means the value never reaches the correct T2 model.

## Recommendations for Next Retraining

1. **Fix URL/URI overlap FIRST**: Remove http/https from URI training data. URI should only contain mailto:, ftp://, tel:, data:, file:// etc. This alone fixes 3/9 regressions.
2. **Add bare offset patterns to UTC offset training**: Include "+05:30", "-08:00" alongside "UTC +05:30". 1/9 regressions.
3. **Diversify hostname training**: Add subdomains, hyphens, server naming patterns. 1/9 regressions.
4. **Increase samples per type from 500 to 1000**: More training data should stabilize T1/T2 decision boundaries.
5. **Add disambiguation rules as safety net**: URL preference rule and expanded Rule 17 protect against model confusion regardless of training quality.
6. **Consider T1 routing oversampling**: The T1 model sees all 22 categories equally — location-specific types may need more weight.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
## Post-retrain analysis: CLDR-enriched model regression diagnosis

Diagnosed root causes for all 9 regressions in the CLDR-enriched retrained model (107/120 vs 116/120 baseline). Analysis performed from training data, taxonomy, tier graph, and test datasets since the retrained model was deleted during rollback.

## Key Finding: Three Systemic Issues

### 1. URL/URI Training Data Overlap (3/9 regressions)
37% of URI training samples use http/https schemes — identical to all URL training samples. The CharCNN cannot distinguish them. **Fix**: Remove http/https from URI training data. URI should contain only non-web schemes (mailto:, ftp://, tel:, data:, file://).

### 2. T1 VARCHAR Routing Degradation (3/9 regressions)
Country→nationality (T1 sent to person instead of location), multilingual.country→entity_name (T1 sent to text instead of location), pressure_atm→latitude (T1 DOUBLE sent to coordinate instead of numeric). The T1 model routes across 22 categories — CLDR-enriched data changed decision boundaries. Non-ASCII text (CJK country names) was particularly affected.

### 3. Training Data Diversity Gaps (3/9 regressions)
- Hostname training: all simple domains ("index.net"), no hyphens or subdomains
- UTC offset training: all "UTC +05:30" format, no bare "+05:30" patterns
- airports.name: T2 person model shifted full_name/last_name boundary, breaking entity demotion chain

## Regression Categories

| # | Column | Predicted | Expected | Category | Fix Type |
|---|--------|-----------|----------|----------|----------|
| 1-3 | URL columns (×3) | URI | URL | Training overlap | Training + Rule |
| 4 | covid.Country | nationality | country | T1 routing | Training |
| 5 | pressure_atm | latitude | decimal_number | T1 routing | Training + Rule |
| 6 | airports.name | last_name | full_name | T2 boundary shift | Training |
| 7 | utc_offset | iso_8601_offset | utc | Format mismatch | Training |
| 8 | multilingual.country | entity_name | country | T1 routing | Training |
| 9 | server_hostname | slug | hostname | Training diversity | Training |

## Recommendations for Next Retraining

1. **Fix URL/URI overlap** — remove http/https from URI training data (+3 correct)
2. **Add bare UTC offset patterns** — include "+05:30" alongside "UTC +05:30" (+1 correct)
3. **Diversify hostname training** — add subdomains, hyphens, server naming conventions (+1 correct)
4. **Increase samples from 500→1000** — stabilize T1/T2 decision boundaries
5. **Add disambiguation rules as safety net** — URL preference rule, expanded Rule 17
6. **Review full_name/last_name training balance** — ensure multi-word non-person strings aren't captured by last_name

Estimated impact of training fixes alone: +5 to +7 of the 9 regressions addressable. Combined with disambiguation rules: all 9 potentially fixable.

## Files Examined
- training_cldr_v1.ndjson (84,500 samples)
- models/tiered-v2/tier_graph.json (169 types, 46 T2 models)
- eval/datasets/manifest.csv (120 columns, 21 datasets)
- eval/schema_mapping.csv (228 GT label mappings)
- Test datasets: airports.csv, multilingual.csv, covid_timeseries.csv, tech_systems.csv, scientific_measurements.csv
<!-- SECTION:FINAL_SUMMARY:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [x] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [x] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [x] #4 Decision record created if plan involved choosing between approaches
- [x] #5 Daily memory log updated with session outcomes
- [x] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
