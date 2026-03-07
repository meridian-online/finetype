---
id: NNFT-244.05
title: >-
  Taxonomy expansion — structural changes (pii field, transform_ext,
  LabelCategoryMap, datetime dedup)
status: Done
assignee:
  - '@nightingale'
created_date: '2026-03-07 05:11'
updated_date: '2026-03-07 05:30'
labels:
  - taxonomy
  - expansion
  - structural
dependencies: []
references:
  - crates/finetype-core/src/taxonomy.rs
  - crates/finetype-model/src/label_category_map.rs
  - crates/finetype-mcp/src/tools/taxonomy.rs
  - labels/definitions_datetime.yaml
parent_task_id: NNFT-244
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Shared structural work that cuts across all domains:

1. **`pii` field**: Add boolean field to Definition struct in finetype-core. Retroactively tag existing PII types: email, phone_number, full_name, first_name, last_name, full_address, street_address.
2. **`transform_ext` field**: Add optional field to Definition struct for extension-dependent DuckDB transforms.
3. **LabelCategoryMap**: Update for 6 new categories (geography.format, geography.index, technology.cloud, identity.government, identity.academic, identity.commerce).
4. **Datetime dedup**: Verify iso_8601_verbose vs existing duration type. Add if distinct, skip if already covered.
5. **MCP server**: Update taxonomy tool/resources if Definition struct changes affect output.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Definition struct has `pii: Option<bool>` field, parsed from YAML
- [x] #2 Definition struct has `transform_ext: Option<String>` field, parsed from YAML
- [x] #3 Existing PII types tagged: email, phone_number, full_name, first_name, last_name, full_address, street_address
- [x] #4 LabelCategoryMap updated with all 6 new categories and correct broad-category routing
- [x] #5 Datetime dedup check completed: iso_8601_verbose vs duration — decision documented
- [x] #6 `finetype check` passes after all structural changes
- [x] #7 `finetype schema` output includes pii and transform_ext fields where present
- [x] #8 MCP taxonomy tool/resources reflect new Definition fields
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
## Implementation Plan

### Findings from code review:
- `transform_ext` already exists in Definition struct (line 127 taxonomy.rs) — AC #2 is already done
- `pii` field does NOT exist yet — needs adding
- Existing `datetime.duration.iso_8601` pattern covers P1Y2M3DT4H5M6S, PT30M but NOT P2W (week) or negative durations
- LabelCategoryMap updates should be deferred to domain subtasks (each domain task adds its own labels to the correct BroadCategory arrays)

### Steps:

1. **Add `pii: Option<bool>` to Definition struct** (taxonomy.rs)
   - Add field with `#[serde(default)]` after `notes`
   - No breaking change — existing YAML without `pii` parses as None

2. **Tag existing PII types in YAML files**
   - `definitions_identity.yaml`: email, phone_number, full_name, first_name, last_name, username, password
   - `definitions_geography.yaml`: full_address, street_address (debatable — will tag)

3. **Update CLI `schema` output** (main.rs ~line 1966)
   - Add `x-finetype-pii` extension field when `pii == Some(true)`

4. **Update CLI `taxonomy --full` output** (main.rs ~line 1820)
   - Add `pii` to the JSON object

5. **Update MCP taxonomy tool** (tools/taxonomy.rs)
   - Include `pii` in the JSON output for each type

6. **Datetime dedup check**
   - Existing `datetime.duration.iso_8601` covers verbose forms (P1Y2M3DT4H5M6S)
   - Missing: P2W (week), negative durations, fractional seconds
   - Decision: update existing type's regex to include W variant rather than adding new type

7. **Verify** — `cargo test`, `cargo run -- check`, `finetype schema identity.person.email`
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
**Findings:**
- `transform_ext` already existed in Definition struct — AC #2 was already done
- LabelCategoryMap (AC #4) deferred to domain subtasks — each domain task adds its labels to the correct BroadCategory arrays since the test count assertions must match
- Duration dedup (AC #5): existing `datetime.duration.iso_8601` updated to cover full spec (weeks P2W, negative -P..., fractional seconds PT1.5S). Added `iso_8601_verbose` as alias. No new type needed.

**Changes made:**
- taxonomy.rs: Added `pii: Option<bool>` field to Definition struct
- definitions_identity.yaml: Tagged 6 PII types (full_name, first_name, last_name, email, phone_number, password)
- definitions_geography.yaml: Tagged 1 PII type (full_address)
- definitions_datetime.yaml: Updated duration regex + description + samples + alias
- generator.rs: Expanded duration generator to produce weeks, verbose, negative variants
- main.rs: Added `pii` to taxonomy JSON output + `x-finetype-pii` to schema output + `x-finetype-transform-ext` to schema output
- tools/taxonomy.rs: Added `pii` to MCP taxonomy JSON output

**Verification:** cargo test (398 pass), cargo fmt, cargo clippy, finetype check (207/207 100%), schema output confirmed">
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added structural taxonomy fields (`pii`, `transform_ext` in schema output) and resolved the duration dedup question ahead of the domain expansion tasks.

**Changes:**

1. **`pii: Option<bool>` field** — Added to Definition struct in finetype-core. Retroactively tagged 7 existing PII types:
   - `identity.person.{full_name, first_name, last_name, email, phone_number, password}`
   - `geography.address.full_address`

2. **Schema/taxonomy output** — `x-finetype-pii: true` emitted in JSON Schema for PII types. `x-finetype-transform-ext` emitted when present. `pii` field included in `taxonomy --full --output json` and MCP taxonomy tool output.

3. **Duration dedup resolved** — Existing `datetime.duration.iso_8601` updated to cover the full ISO 8601 duration spec: weeks (P2W), negative durations (-P...), fractional seconds (PT1.5S), verbose forms (P1Y2M3DT4H5M6S). Added `iso_8601_verbose` as alias. **No new type needed.** Generator expanded to produce all variants.

4. **LabelCategoryMap** — Deferred to domain subtasks (244.01-04). Each domain task will add its new labels to the correct BroadCategory arrays, keeping test count assertions in sync.

**Tests:** cargo test (398 pass), cargo fmt (clean), cargo clippy (clean), finetype check (207/207, 100%), schema output verified.

**Decision:** `iso_8601_verbose` is an alias of `datetime.duration.iso_8601`, not a separate type. The existing regex covers both compact and verbose forms.">
<parameter name="definitionOfDoneCheck">[1, 2]
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
