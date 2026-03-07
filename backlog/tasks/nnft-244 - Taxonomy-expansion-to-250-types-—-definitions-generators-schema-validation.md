---
id: NNFT-244
title: 'Taxonomy expansion to 250+ types — definitions, generators, schema validation'
status: Done
assignee:
  - '@nightingale'
created_date: '2026-03-07 05:10'
updated_date: '2026-03-07 05:54'
labels:
  - taxonomy
  - expansion
dependencies: []
references:
  - discovery/taxonomy-revision/EXPANSION.md
  - labels/definitions_geography.yaml
  - labels/definitions_technology.yaml
  - labels/definitions_identity.yaml
  - labels/definitions_finance.yaml
  - labels/definitions_representation.yaml
  - labels/definitions_datetime.yaml
  - crates/finetype-model/src/label_category_map.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Expand the FineType taxonomy from 207 types toward 250+ by adding new type definitions across all domains. This is a definitions-first effort — YAML definitions, generators, validation patterns, and LabelCategoryMap updates ship now; model retrain is deferred to a follow-up task.

Sources: discovery/taxonomy-revision/EXPANSION.md (47 confirmed + ~10 conditional candidates across 5 tiers).

Key structural changes:
- New `pii: true` boolean field on taxonomy definitions (retroactively tag existing PII types too)
- New `transform_ext` field for extension-dependent DuckDB transforms
- 6 new categories: geography.format, geography.index, technology.cloud, identity.government, identity.academic, identity.commerce
- LabelCategoryMap updated for all new categories

Disambiguation rule: only include both overlapping types if a cheap deterministic function can separate them (e.g., timestamp range check for TSID vs MD5). Otherwise exclude one or alias.

Target is soft — quality over count. Don't force marginal types to hit a number.

Execution: agent team split by domain (geography, technology, identity, finance+representation), with shared lead work for structural changes.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 All new type definitions have YAML entries with validation, format_string, transform, broad_type, and tier fields
- [x] #2 All new types have working generators that pass `finetype check`
- [x] #3 `finetype schema <new_type>` returns valid JSON Schema for every new type
- [x] #4 LabelCategoryMap updated for all new categories (geography.format, geography.index, technology.cloud, identity.government, identity.academic, identity.commerce)
- [x] #5 New `pii` boolean field added to taxonomy Definition struct and YAML spec
- [x] #6 Existing PII types retroactively tagged (email, phone_number, full_name, etc.)
- [x] #7 New `transform_ext` field added to taxonomy Definition struct and YAML spec
- [x] #8 Dedup checklist verified: iso_8601_verbose vs duration, bcp47 vs locale_code, phone_e164 vs phone_number
- [x] #9 Disambiguation-ambiguous types either have a deterministic tiebreaker function or are excluded/aliased
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Expanded FineType taxonomy from 207 to 250 types (+43 new definitions) across all domains, with structural improvements.

## Summary by domain

| Domain | Before | After | New types |
|--------|--------|-------|-----------|
| container | 12 | 12 | — |
| datetime | 84 | 84 | — (duration regex expanded, iso_8601_verbose alias added) |
| finance | 28 | 31 | +3 (figi, aba_routing, bsb) |
| geography | 15 | 25 | +10 (wkt, geojson, h3, geohash, plus_code, dms, mgrs, iso6346, hs_code, unlocode) |
| identity | 19 | 34 | +15 (icd10, loinc, cpt, hcpcs, vin, eu_vat, ssn, ein, pan_india, abn, orcid, email_display, phone_e164, upc, isrc) |
| representation | 32 | 36 | +4 (cas_number, inchi, smiles, color_hsl) |
| technology | 17 | 28 | +11 (ulid, tsid, snowflake_id, aws_arn, s3_uri, jwt, docker_ref, git_sha, cidr, urn, data_uri) |

## Structural changes
- **`pii: Option<bool>`** — New field on Definition struct. 11 types tagged (7 existing + 4 new).
- **`x-finetype-pii`** and **`x-finetype-transform-ext`** — Emitted in JSON Schema output.
- **LabelCategoryMap** — Updated with all new labels routed to correct BroadCategory.
- **Duration dedup** — Existing iso_8601 regex expanded to full spec (weeks, negatives, fractional). `iso_8601_verbose` added as alias.
- **bcp47 dedup** — Added as alias to existing `locale_code`.

## Dedup decisions
- `iso_8601_verbose` → alias of `datetime.duration.iso_8601`
- `bcp47` → alias of `technology.code.locale_code`
- `phone_e164` → kept distinct from `phone_number` (strict +CC format vs flexible locale patterns)
- `tsid` → timestamp-range check (2015-2035) disambiguates from random hex

## Execution
Agent team of 4 working in parallel worktrees, each owning separate domain YAML + generator files. Lead (nightingale) handled structural changes first (NNFT-244.05), then 4 domain agents ran concurrently.

## Verification
- `cargo test`: 254 passed, 0 failed
- `cargo run -- check`: 250/250 generators passing, 12500/12500 samples (100%)
- All 7 domains green

## Follow-up
- NNFT-245: Model retrain on 250-type taxonomy (deferred by design — definitions-first strategy)
<!-- SECTION:FINAL_SUMMARY:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [ ] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [ ] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [ ] #4 Decision record created if plan involved choosing between approaches
- [ ] #5 Daily memory log updated with session outcomes
- [ ] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
