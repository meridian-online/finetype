# Phase 0 Finding: Taxonomy Audit & Simplification

**Task:** NNFT-162
**Date:** 2026-02-28
**Author:** @nightingale

## Summary

Audited all 171 FineType types against collapse criteria. Collapsed 8 types (171 → 163) with zero regressions on all evaluation benchmarks. The taxonomy is now cleaner and ready for the Sense & Sharpen pivot.

## Audit Criteria

Each type was evaluated against three questions:
1. **Structural detectability** — Does the type have a character-level format signal that CharCNN can learn? Types with `designation: broad_words` or `broad_characters` rely on vocabulary, not structure.
2. **Analyst value** — Would an analyst distinguish this type from its parent category? If they'd say "that's fine, just call it X", collapse it.
3. **Model confusion risk** — Does the type overlap with another type in training data or inference? Evidence from NNFT-161 regression analysis.

## Types Collapsed

| # | Type | Target | Rationale |
|---|------|--------|-----------|
| 1 | `technology.hardware.cpu` | `representation.text.entity_name` | Product names ("Intel i7") — not structurally detectable, broad_words designation |
| 2 | `technology.hardware.generation` | `representation.text.entity_name` | Product descriptors ("Gen 4", "DDR5") — no format signal, broad_words designation |
| 3 | `identity.academic.degree` | `representation.discrete.categorical` | Low cardinality enumerated list ("BSc", "PhD") — broad_words designation |
| 4 | `identity.academic.university` | `representation.text.entity_name` | Named entities — universities are organisations, broad_words designation |
| 5 | `identity.person.nationality` | `representation.discrete.categorical` | Short enumerated list ("Australian", "French") — broad_words, locale_specific but only 1 locale |
| 6 | `identity.person.occupation` | `representation.discrete.categorical` | Free text with no format signal ("Engineer", "Doctor") — broad_words designation |
| 7 | `technology.internet.slug` | `representation.code.alphanumeric_id` | Rarely analytically important; confused with hostname in CLDR regression (NNFT-161) |
| 8 | `technology.internet.uri` | MERGED into `technology.internet.url` | 37% training data overlap — http/https URIs are indistinguishable from URLs (NNFT-161 regression analysis) |

## Types Explicitly Retained

27 types with `broad_*` designation and low priority were reviewed but retained:

- **Datetime components** (century, year, day_of_month, periodicity) — useful temporal context for analysts
- **Person attributes** (gender, gender_code) — extremely common, analysts expect this classification
- **Security types** (password, pin) — security-relevant, analysts need to identify these
- **Payment types** (credit_card_network) — useful in payment data analysis
- **Technology types** (os, programming_language, software_license, stage, http_method) — essential for tech analysts
- **Text types** (paragraph, sentence, plain_text) — needed for text length demotion rules (Rule 16)
- **File types** (extension, mime_type) — useful file metadata classification

## Structural Changes

### Taxonomy
- 171 → 163 type definitions
- 6 domains unchanged
- Removed types have tombstone comments in YAML with collapse target and rationale
- URI merged into URL with alias (`aliases: [web_url, uri]`) and merge note

### Tier Graph
- 2 T1 categories eliminated: `VARCHAR_academic` (0 types remaining), `VARCHAR_hardware` (0 types remaining)
- 2 T2 models simplified: `VARCHAR_person` (13 → 11 types), `VARCHAR_internet` (7 → 5 types)
- T0 routing unchanged
- Model weight files (.safetensors, labels.json) NOT modified — v0.3.0 models (169 types) still predict old labels

### Backward Compatibility
Added `remap_collapsed_label()` function in `column.rs` that intercepts predictions of collapsed types and redirects them to their targets. Applied at two points:
1. **Vote aggregation** (Step 3 of `classify_column`) — before HashMap counting
2. **Semantic header hints** — Model2Vec `label_index.json` still has 169 types including collapsed ones; outputs are remapped before application

This ensures the v0.3.0 models continue working without retraining. Both hardcoded `header_hint()` and Model2Vec semantic hint outputs go through `remap_collapsed_label()`.

### Eval Schema Mapping
- Updated `occupation` GT label mapping from `identity.person.occupation` to `representation.discrete.categorical`
- SOTAB/GitTables eval SQL: no references to any collapsed types — no changes needed

## Evaluation Results

### Profile Eval (120 columns, 21 datasets)
| Metric | Before | After | Delta |
|--------|--------|-------|-------|
| Label accuracy | 116/120 (96.7%) | 116/120 (96.7%) | 0 |
| Domain accuracy | 118/120 (98.3%) | 118/120 (98.3%) | 0 |
| Actionability | 2990/3030 (98.7%) | 2990/3030 (98.7%) | 0 |

**Zero regressions.** The 4 remaining misclassifications are unchanged:
- `codes_and_ids.swift_code`: sedol overcall (SEDOL validation should fail on 8-char SWIFT)
- `countries.name`: entity_name instead of country (entity classifier fires correctly but GT expects geography)
- `people_directory.company`: categorical instead of entity_name (entity demotion doesn't fire — top vote isn't full_name)
- `books_catalog.publisher`: city instead of entity_name (low-confidence model confusion)

### SOTAB CTA CLI (16,765 columns)
| Metric | Before | After | Delta |
|--------|--------|-------|-------|
| Label accuracy (format-detectable) | 43.3% | 43.6% | +0.3pp |
| Domain accuracy (format-detectable) | 68.3% | 68.6% | +0.3pp |
| Entity demotion columns | 3,027 (18.1%) | 3,037 (18.1%) | +10 |

**Marginal improvement.** The +0.3pp on both metrics comes from:
- Columns previously predicted as collapsed types now mapping to more appropriate targets
- 10 additional entity demotion firings (likely from occupation/nationality values now routing through different T2 paths)

### Taxonomy Check
- 163/163 definitions: all generators pass
- All CI checks pass (fmt, clippy, test)

## Conclusion

Phase 0 achieved its goal: a cleaner taxonomy with zero accuracy cost. The 8 collapsed types were analytically redundant — their removal simplifies the label space without losing information an analyst cares about. The tier graph is smaller (2 fewer T1 categories, 4 fewer T2 types across 2 models), which marginally reduces inference path complexity.

This clears the way for Phase 1 (Sense model spike), which can now train against 163 well-defined types rather than 171 including niche ones that blur category boundaries.
