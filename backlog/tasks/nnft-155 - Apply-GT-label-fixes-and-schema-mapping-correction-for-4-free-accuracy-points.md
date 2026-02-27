---
id: NNFT-155
title: Apply GT label fixes and schema mapping correction for 4 free accuracy points
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-27 21:28'
updated_date: '2026-02-27 21:34'
labels:
  - accuracy
  - eval
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Apply the findings from NNFT-154 triage: fix 3 GT-label-wrong entries in manifest.csv and 1 schema-mapping bug in schema_mapping.yaml. Zero code changes — eval infrastructure only. Expected impact: 113/120 → 117/120.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 tech_systems.server_hostname GT label changed from 'name' to 'hostname' in manifest.csv
- [x] #2 people_directory.company GT label changed from 'name' to 'entity name' in manifest.csv
- [x] #3 books_catalog.publisher GT label changed from 'name' to 'entity name' in manifest.csv
- [x] #4 Schema mapping for 'swift code' corrected from identity.payment.swift_code to identity.payment.swift_bic
- [x] #5 Schema mapping entries added for new GT labels (hostname, entity name) if needed
- [x] #6 Profile eval re-run confirms accuracy improvement
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Fixed 3 GT labels in manifest.csv and 1 schema mapping bug in schema_mapping.yaml.

Changes:
- manifest.csv: tech_systems.server_hostname GT "name" → "hostname" (prediction was correct)
- manifest.csv: people_directory.company GT "name" → "entity name" (company names aren't person names)\n- manifest.csv: books_catalog.publisher GT "name" → "entity name" (publisher names aren't person names)\n- schema_mapping.yaml: "swift code" finetype_label corrected from identity.payment.swift_code → identity.payment.swift_bic (matching actual taxonomy type)\n- schema_mapping.yaml: Added "hostname" mapping → technology.internet.hostname (direct)\n- schema_mapping.yaml: Added "entity name" mapping → representation.text.entity_name (direct)\n- schema_mapping.csv regenerated from YAML\n\nImpact:\n- Label accuracy: 113/120 (94.2%) → 114/120 (95.0%) — +1 from hostname GT fix\n- Domain accuracy: 114/120 (95.0%) → 116/120 (96.7%) — +2 from hostname and company domain alignment\n- Remaining 6 misses are all genuine prediction errors (cvv, swift_code, world_cities.name, countries.name, publisher, company)\n\nNote: My triage (NNFT-154) overestimated the "free" gains — company and publisher GT fixes correct what we measure against but the predictions (categorical, city) don't match entity_name either. The hostname fix was the only truly free label improvement.
<!-- SECTION:FINAL_SUMMARY:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [x] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [ ] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [ ] #4 Decision record created if plan involved choosing between approaches
- [x] #5 Daily memory log updated with session outcomes
- [x] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
