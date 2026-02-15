---
id: NNFT-069
title: 'Create taxonomy comparison guide: finetype vs schema.org, Wikidata, GitTables'
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-15 05:13'
updated_date: '2026-02-15 05:25'
labels:
  - documentation
  - taxonomy
dependencies: []
documentation:
  - 'https://schema.org/docs/full.html'
  - >-
    https://www.wikidata.org/wiki/Wikidata:Database_reports/Constraint_violations
  - 'https://gittables.github.io/'
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Create a reference document comparing finetype's taxonomy to established type systems used in the data ecosystem. This helps users understand where finetype fits, what it covers that others don't, and vice versa.

Compare against:
1. **schema.org types** — the web's dominant structured data vocabulary. Map finetype types to schema.org equivalents (e.g., identity.person.email ↔ schema:email, geography.coordinate.latitude ↔ schema:latitude). Note gaps in both directions.
2. **Wikidata property types** — property constraints and expected types for real-world data (e.g., P625 coordinate location, P2037 GitHub username). Show how Wikidata's property model relates to finetype's format-based approach.
3. **GitTables column annotations** — the column type annotations used in the GitTables benchmark. This is our evaluation baseline, so documenting the mapping is directly useful.

Format: markdown document with comparison tables showing:
- finetype type → nearest equivalent(s) in each system
- Types unique to finetype (no equivalent elsewhere)
- Types in other systems that finetype doesn't cover (gaps/opportunities)
- Philosophical differences (finetype: format detection vs schema.org: semantic meaning vs Wikidata: entity properties)

Place in docs/ or as a standalone reference page.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Comparison table mapping finetype types to schema.org equivalents
- [x] #2 Comparison table mapping finetype types to relevant Wikidata properties
- [x] #3 Comparison table mapping finetype types to GitTables column annotations
- [x] #4 Section documenting types unique to finetype with no external equivalent
- [x] #5 Section documenting gaps — types in external systems that finetype doesn't cover
- [x] #6 Section explaining philosophical differences between the type systems
- [x] #7 Document is well-formatted markdown, suitable for docs/ directory or project page
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Research actual schema.org, Wikidata, and GitTables type systems
2. Build comparison tables mapping finetype types to equivalents in each system
3. Document types unique to finetype (format-specific types like ip_v4, uuid, hash)
4. Document gaps in finetype vs external systems (semantic types like rank, species, category)
5. Write philosophical differences section
6. Create as docs/TAXONOMY_COMPARISON.md
7. Link from README taxonomy section
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Created docs/TAXONOMY_COMPARISON.md — a comprehensive comparison of FineType's 159-type taxonomy against schema.org (59 GitTables labels), Wikidata properties, and DBpedia (122 GitTables labels).

The document covers:
- **Philosophical differences** table contrasting format detection vs semantic meaning vs entity relationships
- **FineType → schema.org** mapping table (16 direct/close matches, 13 schema.org types with no FineType equivalent)
- **FineType → Wikidata** mapping table (16 properties, highlighting identifier types as strongest overlap)
- **FineType → GitTables** three-tier mapping: format-detectable (12 types, 88-100% accuracy), semantic-only (14+ types, not detectable), boundary types (6 types, partially detectable)
- **Types unique to FineType** — 46 datetime formats, 35 technology formats, 11 container formats that no external system distinguishes
- **Gaps and opportunities** — 6 proposed new types mapped to backlog items (NNFT-063, NNFT-065), plus permanent semantic gaps that are out of scope by design
- **Practical implications** — when to use FineType vs semantic systems vs both combined

Added cross-reference link from README.md taxonomy section to the new document.
<!-- SECTION:FINAL_SUMMARY:END -->
