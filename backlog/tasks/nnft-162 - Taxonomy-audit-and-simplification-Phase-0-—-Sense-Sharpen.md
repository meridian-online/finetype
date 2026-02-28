---
id: NNFT-162
title: Taxonomy audit and simplification (Phase 0 — Sense & Sharpen)
status: In Progress
assignee:
  - '@nightingale'
created_date: '2026-02-28 03:41'
updated_date: '2026-02-28 05:10'
labels:
  - architecture
  - taxonomy
  - sense-and-sharpen
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Audit all 171 FineType types against collapse/expand criteria from the Sense & Sharpen discovery brief. This phase is independent of the architecture pivot and delivers immediate value: a cleaner label space for the existing CharCNN, which may improve accuracy before the Sense model exists.

The brief identifies 7 types to collapse:
- technology.hardware.cpu → plain_text (not structurally detectable; niche)
- technology.hardware.generation → plain_text (no format signal)
- identity.academic.university → organisation (universities are organisations)
- identity.academic.degree → categorical or plain_text (low cardinality, no format signal)
- representation.text.slug → plain_text or merge with identifiers (rarely analytically important)
- identity.person.nationality → categorical (usually a short enumerated list)
- identity.person.occupation → categorical or plain_text (free text, no format signal)

Full audit may identify additional candidates. The principle: if an analyst would say "that's fine, just call it X", then collapse it.\n\nThis is Phase 0 of the Sense & Sharpen pivot (decision-004). It runs independently of Phase 1 (Sense model spike) and can proceed in parallel with Phase 1 data curation.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Audit all 171 types — document collapse/expand recommendation for each candidate with rationale
- [x] #2 Create revised taxonomy YAML definitions (target: ~160 types after collapsing ~7-10 niche types)
- [x] #3 Update profile eval schema_mapping.yaml for collapsed types
- [x] #4 Update SOTAB eval schema mapping for collapsed types
- [x] #5 Regenerate training data with revised taxonomy (finetype generate)
- [x] #6 Run make ci — all tests, clippy, fmt pass with revised taxonomy
- [x] #7 Run make eval-report — re-baseline profile eval scores and document delta vs 116/120
- [x] #8 Run SOTAB CLI eval — re-baseline and document delta vs 43.3% label / 68.3% domain
- [x] #9 Document audit findings and decisions in discovery/architectural-pivot/PHASE0_FINDING.md
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
## Implementation Plan

### Audit Findings

Scanned all 171 types against collapse criteria:
- designation: broad_words / broad_characters / broad_numbers (semantic, not structural)
- release_priority: ≤ 2 (low)
- Analyst test: would an analyst say "that's fine, just call it X"?\n\n27 types have broad designation + low priority, but most provide clear analyst value\n(gender, year, http_method, programming_language, etc.) and should NOT be collapsed.\n\n### Types to Collapse (8 total, 171 → 163)\n\nThe brief's 7 candidates all pass the audit criteria. One additional candidate emerges\nfrom the NNFT-161 regression analysis:\n\n| # | Type | Target | Rationale |\n|---|------|--------|----------|\n| 1 | technology.hardware.cpu | representation.discrete.categorical | Not structurally detectable; niche. "Intel i7" is just a category |\n| 2 | technology.hardware.generation | representation.discrete.categorical | No format signal. "Gen 4", "DDR5" are just categories |\n| 3 | identity.academic.degree | representation.discrete.categorical | Low cardinality enumerated list. "BSc", "PhD" |\n| 4 | identity.academic.university | representation.text.entity_name | Universities are named entities |\n| 5 | identity.person.nationality | representation.discrete.categorical | Short enumerated list. "Australian", "French" |\n| 6 | identity.person.occupation | representation.discrete.categorical | Broad words with no format signal. "Engineer", "Doctor" |\n| 7 | technology.internet.slug | representation.code.alphanumeric_id | Has format signal but rarely analytically important; confused with hostname in CLDR regression |\n| 8 | technology.internet.uri | MERGE into technology.internet.url | 37% training data overlap (NNFT-161). http/https URIs are indistinguishable from URLs. Model cannot and should not distinguish. |\n\nTypes explicitly NOT collapsed despite broad designation:\n- datetime components (century, year, day_of_month, periodicity) — useful temporal context\n- identity.person.gender/gender_code — extremely common, analysts expect it\n- identity.person.password — security-relevant\n- identity.payment.credit_card_network — useful in payment data\n- technology.development.{os, programming_language, software_license, stage} — useful for tech analysts\n- technology.internet.http_method — essential for API analytics\n- representation.{text.paragraph, text.sentence, text.plain_text} — needed for text demotion rules\n- representation.file.{extension, mime_type} — useful file metadata\n- technology.code.pin — security-relevant\n\n### Structural Impact\n\nCollapsing 8 types changes the tier graph:\n- VARCHAR_hardware: loses cpu (1 of 2 types). If only screen_size + ram_size remain,\n  hardware goes from 2→0 VARCHAR types. cpu was the only VARCHAR hardware type in T2.\n  generation was also VARCHAR hardware. With both removed, VARCHAR_hardware category\n  is EMPTY → remove from T1 routing entirely.\n- VARCHAR_academic: loses both degree and university (2 of 2 types) → EMPTY category\n  → remove from T1 routing.\n- VARCHAR_person: loses nationality and occupation (2 of 13 types) → 11 types. Stays.\n- VARCHAR_internet: loses slug and uri (2 of 7 types) → 5 types. Stays.\n\nNet: 2 T1 categories eliminated, 2 T2 models simplified.\n\n### Implementation Steps\n\n1. Remove collapsed type definitions from labels/definitions_*.yaml\n2. Update aliases/synonyms if any of the removed types are referenced\n3. Remove generators for collapsed types from crates/finetype-core/\n4. Update tier_graph.json — remove empty categories, update type lists\n5. Update crates/finetype-model/src/column.rs — remove any disambiguation rules\n   referencing collapsed types (slug is in internet attractors?)\n6. Update crates/finetype-model/src/semantic.rs — if type_embeddings reference\n   collapsed types, need to regenerate\n7. Update eval/schema_mapping.yaml — remap affected GT labels\n8. Update eval SOTAB mapping (eval/sotab/ SQL)\n9. Regenerate training data: cargo run -- generate\n10. Run make ci (fmt + clippy + test + check)\n11. Retrain is NOT needed for Phase 0 — we keep v0.3.0 models (169 types).\n    The collapsed types are simply no longer in the taxonomy. The model may\n    still predict them, but column.rs will treat them as unknown → plain_text.\n12. Run make eval-report — re-baseline profile eval\n13. Run SOTAB CLI eval — re-baseline\n14. Write PHASE0_FINDING.md with audit results and score deltas
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Phase 0 implementation complete:
- 8 types collapsed (171 → 163): cpu, generation → entity_name; degree, nationality, occupation → categorical; university → entity_name; slug → alphanumeric_id; uri → merged into url
- Added remap_collapsed_label() in column.rs to handle v0.3.0 model predictions of removed types
- Also remap semantic hint outputs (Model2Vec label_index still has old types)
- Tier graph updated: removed VARCHAR_academic and VARCHAR_hardware categories; updated VARCHAR_person (13→11) and VARCHAR_internet (7→5)
- Profile eval: 116/120 (96.7%) — UNCHANGED from baseline
- Actionability: 98.7% — UNCHANGED
- Running SOTAB eval next
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Phase 0 Taxonomy Audit & Simplification — Collapsed 8 niche types (171 → 163) with zero regressions.

## Changes

**Taxonomy (labels/):**
- Removed 8 type definitions: cpu, generation (→ entity_name), degree, nationality, occupation (→ categorical), university (→ entity_name), slug (→ alphanumeric_id), uri (→ merged into url)
- URI merged into URL with alias and notes documenting 37% training overlap (NNFT-161)
- Tombstone comments left in YAML for each removed type

**Tier graph (models/tiered-v2/tier_graph.json):**
- Removed 2 T1 categories: VARCHAR_academic (empty), VARCHAR_hardware (empty)
- Updated VARCHAR_person (13→11) and VARCHAR_internet (7→5)
- Model weight files untouched — v0.3.0 models remain active

**Column inference (crates/finetype-model/src/column.rs):**
- Added remap_collapsed_label() for v0.3.0 backward compatibility
- Applied at vote aggregation (Step 3) and semantic header hint output
- Updated hardcoded header_hint() for occupation → categorical

**Eval:**
- Updated schema_mapping.yaml: occupation → categorical
- SOTAB/GitTables SQL: no collapsed type references — no changes needed

## Evaluation Results

- Profile eval: 116/120 (96.7%) — UNCHANGED
- SOTAB CTA: 43.6% label / 68.6% domain (+0.3pp each)
- Actionability: 98.7% — UNCHANGED
- Taxonomy check: 163/163 passing
- CI: all checks pass

## Discovery Output

- discovery/architectural-pivot/PHASE0_FINDING.md — full audit findings
- decision-004 — Sense & Sharpen pivot rationale
<!-- SECTION:FINAL_SUMMARY:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [x] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [x] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [x] #4 Decision record created if plan involved choosing between approaches
- [x] #5 Daily memory log updated with session outcomes
- [ ] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
