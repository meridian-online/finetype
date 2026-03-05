---
id: NNFT-222
title: 'NNFT-223 — Format Coverage Taxonomy Expansion (Phase 1: YAML + Generators)'
status: To Do
assignee: []
created_date: '2026-03-05 01:55'
updated_date: '2026-03-05 02:25'
labels:
  - format-coverage
  - taxonomy
  - phase-1
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Add 54 new date, timestamp, and currency format types to FineType taxonomy based on comprehensive research synthesis from Claude and Gemini agents.

This is Phase 1 of a 2-phase implementation:
- **Phase 1 (this task)**: Add 54 type definitions to YAML, implement 54 generators, update LabelCategoryMap, validate taxonomy alignment
- **Phase 2 (NNFT-227)**: Retrain CharCNN model on 213 classes (163 existing + 54 new), profile eval, release v0.6.0

**Scope: 54 actionable formats**
- 25 date formats (22 Claude research + 1 Gemini unique: fiscal_year)
  - Includes CJK: Chinese 中文, Korean 한글, Japanese imperial era 令和
  - Includes Oracle date (dmy_dash_abbrev), year-month, partial dates, regional separators
- 16 timestamp formats (all Claude research)
  - Includes CLF (Apache logs), syslog, SQL microseconds/milliseconds, ISO 8601 milliseconds/microseconds with timezone
- 13 currency formats (12 Claude research + 1 Gemini unique: yield)
  - Includes accounting parentheses, EU suffix notation, Indian lakh/crore, Swiss apostrophe, zero-decimal, code prefix, basis points, financial yield

**Research validation**: Zero naming collisions with existing 163 types; both agents independently validated formats against disambiguation surfaces

**Taxonomy impact**: 163 types → 213 types (33% increase); temporal category grows 45→83, currency grows 4→16
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 All 54 types added to YAML definitions with zero validation collisions
- [ ] #2 All 54 generators implemented with samples matching format_string or custom parsing
- [ ] #3 CJK generators produce correct Unicode output (Chinese 年月日, Korean 년월일, Japanese era offset)
- [ ] #4 LabelCategoryMap updated with 38 new datetime types (45→83) and 12 new currency types (4→16)
- [ ] #5 `cargo run -- check` passes (taxonomy ↔ generator alignment validated)
- [ ] #6 `cargo test --all` passes (type count assertions updated: 163→213)
- [ ] #7 CLI inference works for all 54 new formats and 163 existing types
- [ ] #8 No breaking changes to existing types or locale validation (NNFT-195-201 untouched)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Type count correction: sql_offset removed (redundant with rfc_3339).
Final: 53 new types, 216 total (not 54/217 from initial plan).
Timestamps: 15 new (not 16). All other counts unchanged.
<!-- SECTION:NOTES:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [ ] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [ ] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [ ] #4 Decision record created if plan involved choosing between approaches
- [ ] #5 Daily memory log updated with session outcomes
- [ ] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
