---
id: NNFT-115
title: Add confidence-gated fallback to demote over-eager specific type predictions
status: To Do
assignee:
  - '@nightingale'
created_date: '2026-02-24 06:11'
labels:
  - accuracy
  - disambiguation
dependencies: []
references:
  - crates/finetype-model/src/column.rs
  - eval/eval_output/profile_results.csv
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The CharCNN model produces confident-looking predictions for specific identity/geography types when the actual data is generic (integers, short words, text). This is because many real-world values share the same character-level distribution as the trained specific types.

**Problem:** 37 false positives across profile eval where the model assigns specific types to generic data:
- Integers (attendance, salary, volume, pages) → postal_code or street_number
- Short words (sport names, status values, language names) → first_name
- 3-digit codes (altitude, region codes) → cvv
- Multi-word text (descriptions, venues, companies) → full_name or username

**Root cause:** These types are not in the `is_generic` list in column.rs, so header hints and disambiguation rules don't override them. The model's confidence (0.3–0.8) is mediocre but treated as authoritative.

**Solution:** Confidence-gated fallback — when a specific type wins the vote but confidence is below a calibrated threshold and no header hint confirms it, demote to the best generic representation.* type from the vote distribution. True positives for these types typically have confidence > 0.85; false positives cluster at 0.3–0.8.

**Demotable types (the "attractor" types):**
- geography.address.postal_code (9 false positives)
- identity.payment.cvv (6 false positives)
- geography.address.street_number (5 false positives)
- identity.person.first_name (5 false positives)
- identity.person.username (3 false positives)
- identity.person.full_name (2 false positives — only when no header hint)
- geography.transportation.icao_code (3 false positives)
- identity.medical.ndc (2 false positives)
- identity.payment.cusip (1 false positive)
- geography.address.street_name (1 false positive)

**Fallback targets:**
- Numeric attractors (postal_code, street_number, cvv) → representation.numeric.integer_number or decimal_number
- Text attractors (first_name, full_name, username) → representation.discrete.categorical (if low cardinality) or representation.text.word
- Code attractors (icao_code, cusip, ndc) → representation.code.alphanumeric_id or representation.discrete.categorical
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Define demotable type list and confidence threshold(s) in column.rs
- [ ] #2 Implement fallback logic: demote to best representation.* type from vote distribution when confidence below threshold and no confirming header hint
- [ ] #3 Fallback selects appropriate generic type based on value characteristics (numeric → integer/decimal, text → categorical/word, code → alphanumeric_id)
- [ ] #4 No regressions on true positives: actual postal codes, actual names, actual CVVs still classified correctly
- [ ] #5 Profile eval accuracy improves (baseline: 68/74 format-detectable correct)
- [ ] #6 sports_events: sport and status no longer classified as first_name
- [ ] #7 titanic: Ticket and Cabin no longer classified as postal_code/cvv
- [ ] #8 airports: type and source no longer classified as first_name
- [ ] #9 Unit tests for demotion logic covering true positives (no demotion) and false positives (demotion fires)
<!-- AC:END -->
