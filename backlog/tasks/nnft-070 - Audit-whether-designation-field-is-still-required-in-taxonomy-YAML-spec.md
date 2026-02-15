---
id: NNFT-070
title: Audit whether designation field is still required in taxonomy YAML spec
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-15 07:49'
updated_date: '2026-02-15 08:19'
labels:
  - taxonomy
  - cleanup
dependencies: []
references:
  - labels/definitions_identity.yaml
  - crates/finetype-core/src/taxonomy.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Each taxonomy type definition includes a `designation` field (values: "universal", "locale-specific", etc.). It's unclear if this field is actively used anywhere in the codebase — it may be vestigial from the v1 taxonomy migration.

Investigate:
1. Where `designation` is referenced in Rust code (parsing, inference, generation, evaluation)
2. Whether any logic branches on its value
3. Whether it duplicates information already expressed by the `locales` array
4. If unused, whether it should be removed from the spec or kept as documentation metadata

If it's purely informational, it could be simplified or removed to reduce YAML noise. If it's load-bearing, document where and why.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Grep/search confirms where designation is parsed and used in Rust code
- [x] #2 Document whether any runtime logic depends on designation value
- [x] #3 Determine if designation is redundant with locales array
- [x] #4 Recommendation: keep, remove, or simplify — with rationale
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Audited the `designation` field across the codebase.

## Findings

**Where it's used:**
1. **generator.rs:96-118** — LOAD-BEARING: `Designation::LocaleSpecific` triggers per-locale 4-level label generation. All other designations get `.UNIVERSAL` suffix. This is the branching logic for localized vs universal training data.
2. **taxonomy.rs:77** — Parsed into `Designation` enum (Universal, LocaleSpecific, BroadNumbers, BroadCharacters, BroadWords, BroadObject)
3. **main.rs:876,893,910** — Displayed in taxonomy CLI output (plain, JSON, CSV formats)

**Is it redundant with `locales`?** Partially. `LocaleSpecific` could be inferred from `locales` containing non-UNIVERSAL entries. But `BroadNumbers`, `BroadCharacters`, `BroadWords`, `BroadObject` carry additional semantic grouping info that `locales` doesn't capture.

## Recommendation: KEEP

The field is actively used in training data generation to determine labeling strategy. The Broad* variants provide useful grouping metadata. Removing it would require reworking the localized training data pipeline. Not worth the disruption for cosmetic cleanup."
<!-- SECTION:FINAL_SUMMARY:END -->
