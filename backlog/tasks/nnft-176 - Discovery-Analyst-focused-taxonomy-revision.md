---
id: NNFT-176
title: 'Discovery: Analyst-focused taxonomy revision'
status: Done
assignee:
  - '@nightingale'
created_date: '2026-03-01 09:38'
updated_date: '2026-03-02 05:49'
labels:
  - discovery
  - taxonomy
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Full taxonomy audit to make FineType's type system as impactful and intuitive as possible for analysts.

Three workstreams:

1. **Remove/fix low-value types** — CVV (false positives, low analyst value), generic postal_code without locale (imprecise, frequent false positives). Identify any other types that hurt more than they help.

2. **Naming and structure review** — Are type names intuitive for analysts? Do domain/category groupings match analyst mental models? Would an analyst guess what 'representation.numeric.si_number' means?

3. **Coverage gap analysis** — Large-scale search across Kaggle, government open data, public interest datasets, and reference taxonomies to find types analysts encounter in the wild that FineType doesn't recognise.

Outcome: A recommendation document with specific additions, removals, and renames — each with data-driven justification. NOT implementation.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Search brief produced for external research agents
- [x] #2 External research results collected and synthesised
- [x] #3 Coverage gaps: prioritised list of missing types with real-world frequency evidence
- [x] #4 Domain/category structure recommendations with rationale
- [x] #5 Final recommendation document in discovery/taxonomy-revision/BRIEF.md
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Search brief produced at discovery/taxonomy-revision/SEARCH_BRIEF.md. Covers 4 research questions: real-world column frequency, existing taxonomy comparisons, analyst frustrations, and high-value DuckDB transforms. Includes full current taxonomy appendix and 5 specific structural questions. Ready for Hugh to hand to online research agents.

External research responses reviewed (RESPONSE_CLAUDE.md and RESPONSE_GEMINI.md). Key finding: the SEARCH_BRIEF appendix was out of date, causing both research agents to flag phantom duplicates (no duplicate email, boolean, or entity_name exist) and missing types that already exist (UUID, country_code, locale_code). After correcting errors and synthesising with Hugh's feedback in META_RESPONSE.md, produced BRIEF.md with validated recommendations.

Updated BRIEF.md with Hugh's decisions: finance.banking (swift_bic + new IBAN), identity.commerce.product (EAN/ISBN/ISSN from technology.code). Skipping ACs #3-5 (local dataset profiling, false-positive frequency, naming intuitiveness scoring) per Hugh — sufficient evidence to proceed to implementation. BRIEF.md is the final recommendation document.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Discovery task: analyst-focused taxonomy revision for FineType v0.5.1.

Process:
- Produced SEARCH_BRIEF.md with 4 research questions covering real-world frequency, tool comparisons, analyst frustrations, and DuckDB transform value
- Two external research agents (Claude, Gemini) independently surveyed Kaggle, government data, enterprise SaaS schemas, and analyst communities
- Validated findings against actual v0.5.0 taxonomy — discovered SEARCH_BRIEF appendix was stale (pre-NNFT-162), correcting ~50% of research findings (phantom duplicates, already-existing types)
- Synthesised with Hugh's feedback and domain expertise\n\nDeliverables:\n- discovery/taxonomy-revision/BRIEF.md — final recommendation document\n- Key decisions: new finance domain, identity.commerce.product category, representation.identifier category, 5-7 new types, 2 removals\n- Implementation tasks created for v0.5.1 release
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
