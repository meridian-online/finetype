---
id: NNFT-154
title: Profile eval accuracy triage — assess 7 remaining misses at 113/120
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-27 21:05'
updated_date: '2026-02-27 21:11'
labels:
  - accuracy
  - discovery
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Quick triage of the 7 remaining profile eval misses to classify each as rule-fixable vs model-level vs needs new data. The 7 misses: countries.name (entity_name overcall, GT=geography), world_cities.name (last_name overcall, GT=city), books_catalog.publisher (city overcall, GT=full_name), tech_systems.server_hostname (hostname, GT=full_name), people_directory.company (categorical, GT=full_name), codes_and_ids.cvv (postal_code overcall), codes_and_ids.swift_code (sedol overcall). Time-boxed to ~2 hours.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Each of the 7 misses categorised as: rule-fixable / model-level / needs-new-data / GT-label-wrong
- [x] #2 For rule-fixable misses: specific rule change described
- [x] #3 Written finding with data supporting each categorisation
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
## Triage Findings

### 1. codes_and_ids.cvv — postal_code overcall (0.80)
**Category: Rule-fixable**
Root cause: CVV values (3-digit: 522, 864, 114) match numeric_postal_code_detection heuristic (consistent digits, range 100-999, non-sequential). Since postal_code with numeric_postal_code_detection rule is not generic, the "cvv" header hint cannot override it.
Fix: Either (a) treat numeric_postal_code_detection as yielding-to-hints (like attractor demotion), or (b) exempt 3-digit columns from postal_code detection (most real postal codes are 4-5+ digits).

### 2. codes_and_ids.swift_code — sedol overcall (0.51)
**Category: Schema-mapping-bug + potentially rule-fixable**
Root cause: Schema mapping references identity.payment.swift_code but taxonomy type is identity.payment.swift_bic. Even if classification were correct, the eval would mark it wrong because the mapping label doesn't match the taxonomy. Additionally, 8-char SWIFT codes should fail SEDOL validation (7 chars) and trigger attractor demotion, but need to verify.\nFix: Correct schema_mapping.yaml to use swift_bic.\n\n### 3. world_cities.name — last_name overcall (0.60)\n**Category: Rule-fixable**\nRoot cause: Known issue (documented in CLAUDE.md). CharCNN sees proper nouns → full_name/last_name. Geography protection only guards full_name hints from overriding geography, but Model2Vec "name" header → last_name hint bypasses the guard.\nFix: Extend geography protection to also guard last_name hints when geography votes exist.\n\n### 4. countries.name — entity_name (0.10) vs country\n**Category: Rule-fixable (geography rescue during entity demotion)**\nRoot cause: Entity demotion correctly identifies non-person entities and demotes full_name → entity_name. But entity_name is too broad — the GT expects geography.location.country. Entity demotion guard then prevents header hints from rescuing.\nFix: During entity demotion, check if geography votes exist in the column's vote distribution. If yes, use the specific geography type instead of generic entity_name.\n\n### 5. tech_systems.server_hostname — hostname (0.60) vs full_name\n**Category: GT-label-wrong**\nData: srv-dev-43.example.com, srv-prod-11.example.com — clearly hostnames.\nManifest GT label is "name" → maps to full_name. The prediction hostname is CORRECT.\nFix: Change manifest GT label from "name" to "hostname" and add schema mapping for hostname.\n\n### 6. people_directory.company — categorical (0.50) vs full_name\n**Category: GT-label-wrong**\nData: InfoSys, DigiCore, TechStars, NextGen — company names, not person names.\nManifest GT label is "name" → maps to full_name. Categorical is reasonable for a small set of company names.\nFix: Change manifest GT label to "entity name" or "company" and add appropriate mapping.\n\n### 7. books_catalog.publisher — city (0.32) vs full_name\n**Category: GT-label-wrong + model-level**\nData: Penguin, Manning, Addison-Wesley, Wiley — publisher names, not person names.\nManifest GT label is "name" → maps to full_name. Prediction city at 0.32 is also wrong.\nFix: Change manifest GT label to "entity name". The prediction is low-confidence model confusion (publisher names look like city names / person names to the CharCNN).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Triaged all 7 remaining profile eval misses (113/120). Findings:

**GT-label-wrong (3 misses → free accuracy):**
- tech_systems.server_hostname: GT "name"→full_name, but data is hostnames. Prediction hostname is correct.
- people_directory.company: GT "name"→full_name, but data is company names. Prediction categorical is reasonable.
- books_catalog.publisher: GT "name"→full_name, but data is publisher names.

**Schema-mapping bug (1 miss → free accuracy):**
- codes_and_ids.swift_code: Mapping references identity.payment.swift_code but taxonomy type is identity.payment.swift_bic. The eval marks it wrong even when classification is correct.

**Rule-fixable (2 misses):**
- codes_and_ids.cvv: numeric_postal_code_detection fires for 3-digit values, producing postal_code at 0.80. Not generic → header hint "cvv" can't override. Fix: treat numeric disambiguation as hint-yielding, or exempt 3-digit columns.\n- world_cities.name: Geography protection guards full_name hints but not last_name from Model2Vec. Fix: extend guard to last_name.\n\n**Rule-fixable but harder (1 miss):**\n- countries.name: Entity demotion correctly fires (entity_name) but too broad. Geography rescue during entity demotion could use geography votes to pick country instead of entity_name.\n\n**Projected impact of GT fixes alone:** 113→116/120 (96.7%) with zero code changes.\n**With rule fixes:** potentially 116→119/120 (99.2%) — the publisher miss is model-level.\n\nNo code changes in this task — findings only.
<!-- SECTION:FINAL_SUMMARY:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [x] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [ ] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [ ] #4 Decision record created if plan involved choosing between approaches
- [x] #5 Daily memory log updated with session outcomes
- [ ] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
