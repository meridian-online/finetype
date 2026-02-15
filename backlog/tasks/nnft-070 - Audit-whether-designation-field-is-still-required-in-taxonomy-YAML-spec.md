---
id: NNFT-070
title: Audit whether designation field is still required in taxonomy YAML spec
status: To Do
assignee: []
created_date: '2026-02-15 07:49'
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
- [ ] #1 Grep/search confirms where designation is parsed and used in Rust code
- [ ] #2 Document whether any runtime logic depends on designation value
- [ ] #3 Determine if designation is redundant with locales array
- [ ] #4 Recommendation: keep, remove, or simplify — with rationale
<!-- AC:END -->
