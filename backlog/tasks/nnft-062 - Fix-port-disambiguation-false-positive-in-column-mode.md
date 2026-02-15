---
id: NNFT-062
title: Fix port disambiguation false positive in column mode
status: To Do
assignee: []
created_date: '2026-02-15 05:12'
labels:
  - bugfix
  - disambiguation
dependencies: []
references:
  - crates/finetype-model/src/column.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The `numeric_port_detection` disambiguation rule in `column.rs` fires incorrectly on columns like Titanic's Age (values 0-80). The bug: `has_common_ports` uses `.any()` — a single matching value (e.g., 22, 25, 53 which are both common ages AND common ports) is enough to trigger the rule.

Fix: require that a significant fraction (≥30%) of parsed values match the common port list, not just "any". Real port columns will have many common ports; age/count columns will coincidentally match just a few.

File: `crates/finetype-model/src/column.rs`, function `disambiguate_numeric()`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Port disambiguation requires ≥30% of values to match common port list (not just any)
- [ ] #2 Titanic Age column no longer classified as technology.internet.port
- [ ] #3 Existing port detection unit test still passes with real port data
- [ ] #4 New unit test: column of ages (22, 25, 30, 35, 40, 45, 50, 53, 60, 70) does NOT trigger port detection
- [ ] #5 All existing column.rs tests pass
<!-- AC:END -->
