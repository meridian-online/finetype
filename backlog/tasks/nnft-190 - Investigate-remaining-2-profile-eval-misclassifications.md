---
id: NNFT-190
title: Investigate remaining 2 profile eval misclassifications
status: Done
assignee:
  - '@accuracy-researcher'
created_date: '2026-03-03 06:31'
updated_date: '2026-03-03 07:14'
labels:
  - accuracy
  - discovery
dependencies: []
references:
  - crates/finetype-model/src/column.rs
  - eval/eval_output/report.md
  - eval/datasets/manifest.csv
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Profile eval is at 117/119 (98.3% label). Two misclassifications remain:

1. **countries.name** — predicted full_name, ground truth country. The header is "name" which is ambiguous. CharCNN doesn't see country as plurality vote because the Sense pipeline masks it out. Model2Vec returns full_name for "name" headers. Geography rescue doesn't fire because the values don't look geographic enough to CharCNN.

2. **datetime_formats_extended.long_full_month_date** — predicted iso_8601, ground truth long_full_month. The column contains dates like "January 15, 2024" but the model sees them as ISO 8601. This is a datetime format confusion issue.

This is an investigation/discovery task. The output should be a written analysis with:
- Root cause analysis for each misclassification
- CharCNN vote distributions and Sense predictions for each
- Potential fix approaches ranked by feasibility and risk
- Recommendation on whether to pursue fixes or accept as baseline
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Root cause analysis documented for countries.name misclassification
- [x] #2 Root cause analysis documented for long_full_month_date misclassification
- [x] #3 CharCNN vote distributions captured for both columns
- [x] #4 Fix approaches ranked by feasibility with trade-offs noted
- [x] #5 Written findings committed to discovery/ or task notes
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
## Investigation Results (2026-03-03)

### Case 1: countries.name → full_name (GT: country)

**CharCNN value-level vote breakdown (all 249 values):**
- city: 45/249 (18.1%) — plurality
- first_name: 35 (14.1%)
- street_name: 27 (10.8%)
- full_name: 25 (10.0%)
- country: 22 (8.8%) — correct type only 5th place
- region: 21 (8.4%)
- entity_name: 12 (4.8%)
- continent: 9 (3.6%)
- month_name: 9 (3.6%)
- + 12 other types at <3%

**By Sense category:**
- Geographic: 133/249 (53.4%) — city, street_name, country, region, continent, street_suffix, full_address
- Entity: 85/249 (34.1%) — first_name, full_name, entity_name, username, last_name, gender
- Temporal: 14/249 (5.6%) — month_name, day_of_week
- Text: 14/249 (5.6%)
- Format: 3/249 (1.2%)

**Sense prediction: TEMPORAL** (wrong — should be geographic)
- Only temporal types survive masking: month_name (4%), day_of_week (2%) = 6%
- Geographic (53%) + Entity (34%) all masked out
- Safety valve does NOT fire because Sense confidence ≥ 0.75
- month_name wins at ~4% confidence (way below 0.3)
- header_hint("name") matches h == "name" → full_name
- Fallback fires (confidence < 0.3, full_name not in temporal votes) → full_name

**Without Sense (--sharp-only):**
- city wins at 17% (plurality) — correct domain, wrong type
- header_hint("name") → full_name (person-name hint)
- Geography protection fires: person-name hint + location type → keep city
- Result: city (geography domain correct, label wrong)

**Root cause:** Deeply ambiguous. Country names look like person names, city names, and region names to CharCNN. Sense misroutes to temporal. Even without Sense, country only gets 8.8% of votes.

### Case 2: long_full_month_date → iso_8601 (GT: long_full_month)

**CharCNN value-level vote breakdown (all 80 values):**
- long_full_month: 67/80 (83.8%) — CORRECT, dominant
- abbreviated_month: 10/80 (12.5%) — temporal, close relative
- full_address: 3/80 (3.8%) — geographic, wrong

**By Sense category:**
- Temporal: 77/80 (96.3%) — long_full_month + abbreviated_month
- Geographic: 3/80 (3.8%) — full_address only

**Sense prediction: GEOGRAPHIC** (wrong — should be temporal)
- Only geographic types survive masking: full_address at 3/80 (3.75%)
- 96% of votes (long_full_month + abbreviated_month) masked out
- Safety valve does NOT fire because Sense confidence ≥ 0.75
- full_address wins at 3.75% confidence
- header_hint("long_full_month_date") normalizes to "long full month date"
  → matches h.contains("date") at column.rs:2110 → iso_8601
- Fallback fires (confidence 0.0375 < 0.3, iso_8601 not in geographic votes) → iso_8601

**Without Sense (--sharp-only):**
- long_full_month: 83.7% confidence — CORRECT
- No disambiguation needed, CharCNN is strong and unambiguous
- header_hint returns iso_8601 but confidence is too high (0.84) for override
- Result: long_full_month (correct!)

**Root cause:** Sense misroutes to geographic with high confidence, masking out correct temporal votes. Then the overly-broad header_hint h.contains("date") match returns iso_8601 as fallback. TWO bugs compound: Sense misrouting AND imprecise header hint.

### Ironic pattern: inverted Sense misrouting

The two columns have inverted Sense misrouting:
- countries.name (geographic data) → Sense predicts TEMPORAL
- long_full_month_date (temporal data) → Sense predicts GEOGRAPHIC

Both result in >90% of CharCNN votes being masked out, safety valve not firing (Sense confidence ≥ 0.75), and header hint fallback producing the wrong type.

### Fix approaches ranked by feasibility

**1. Fix header_hint for "date" keyword — long_full_month_date (EASY, LOW RISK)**
- column.rs:2110: h.contains("date") → iso_8601 is too broad
- "long full month date" contains "date" but is NOT an iso_8601 column
- Fix: add specific match for headers containing "month" before the "date" catch-all
- Or: exclude "month" from the "date" keyword match
- Impact: fixes long_full_month_date immediately
- Risk: minimal — only affects headers containing both "month" and "date"

**2. Adjust safety valve threshold (MEDIUM, MEDIUM RISK)**
- When >90% of votes are masked out, Sense is likely wrong regardless of confidence
- Current: masked_out_frac > 0.4 AND Sense confidence < 0.75
- Proposed: add secondary threshold: masked_out_frac > 0.90 always triggers fallback
- Impact: fixes both cases (would fall back to unmasked votes)
- Risk: might cause false fallbacks in some cases. Needs careful eval.

**3. Retrain Sense model (HARD, MEDIUM RISK)**
- Sense confuses geographic and temporal categories in these cases
- Fix: add more training examples of long_full_month dates and country name columns
- Impact: most principled fix, would improve overall Sense accuracy
- Risk: retraining can regress other predictions

**4. Accept countries.name as known limitation (NO EFFORT)**
- "name" header is genuinely ambiguous — no fix is reliable
- CharCNN only gives country 8.8% of votes — the data itself is ambiguous
- 117/119 (98.3%) is a strong baseline
- Recommended: accept this one, focus effort on long_full_month_date

**5. Improve CharCNN for country names (HARD, LOW FEASIBILITY)**
- Country names are inherently similar to person/city names
- Training is unlikely to help significantly without major architecture changes
- Not recommended

### Recommendation

- **Fix long_full_month_date** via approach #1 (header_hint fix) + optionally #2 (safety valve).
  This is easy, low-risk, and gets us to 118/119 (99.2% label).
- **Accept countries.name** as a known limitation. The ambiguity is real and no reliable fix exists.
  Even without Sense, the best result is "city" (wrong label, right domain).
- **Consider approach #2** (safety valve) as a general robustness improvement in a future task.
  Would help both cases but needs careful evaluation.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Investigated both remaining profile eval misclassifications (117/119 → potential 118/119).

## Findings

**countries.name → full_name (GT: country)**
CharCNN votes are deeply fragmented: city 18.1%, first_name 14.1%, country only 8.8%. Sense misroutes to temporal category (inverted routing), masking out 94% of votes. Header hint h=="name" → full_name via fallback. Root cause: genuine ambiguity — country names look like person/city/region names. No reliable fix.

**long_full_month_date → iso_8601 (GT: long_full_month)**
CharCNN correctly predicts long_full_month at 83.8% — WITHOUT Sense, this column is correct. Sense misroutes to geographic (inverted routing), masking out 96% of votes. Then header_hint h.contains("date") returns iso_8601 via fallback. TWO compounding bugs: Sense misrouting + overly-broad "date" keyword match in header_hint().

## Root cause pattern
Both cases share the same mechanism: Sense misroutes with high confidence (≥0.75), safety valve does not fire, >90% of CharCNN votes are masked out, header hint fallback produces wrong type. Ironically, Sense routes the geographic column to temporal and the temporal column to geographic.

## Recommended fixes
1. **header_hint fix** (easy): Add "month" exclusion to the "date" keyword match — fixes long_full_month_date immediately
2. **Safety valve adjustment** (medium): Add secondary threshold at 90% mask-out rate — fixes both cases as general robustness
3. **Accept countries.name** as known limitation — ambiguity is genuine, no reliable fix
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
