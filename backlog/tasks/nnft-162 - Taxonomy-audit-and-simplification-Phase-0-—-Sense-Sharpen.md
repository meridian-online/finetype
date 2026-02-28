---
id: NNFT-162
title: Taxonomy audit and simplification (Phase 0 — Sense & Sharpen)
status: To Do
assignee:
  - '@nightingale'
created_date: '2026-02-28 03:41'
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
- [ ] #1 Audit all 171 types — document collapse/expand recommendation for each candidate with rationale
- [ ] #2 Create revised taxonomy YAML definitions (target: ~160 types after collapsing ~7-10 niche types)
- [ ] #3 Update profile eval schema_mapping.yaml for collapsed types
- [ ] #4 Update SOTAB eval schema mapping for collapsed types
- [ ] #5 Regenerate training data with revised taxonomy (finetype generate)
- [ ] #6 Run make ci — all tests, clippy, fmt pass with revised taxonomy
- [ ] #7 Run make eval-report — re-baseline profile eval scores and document delta vs 116/120
- [ ] #8 Run SOTAB CLI eval — re-baseline and document delta vs 43.3% label / 68.3% domain
- [ ] #9 Document audit findings and decisions in discovery/architectural-pivot/PHASE0_FINDING.md
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [ ] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [ ] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [ ] #4 Decision record created if plan involved choosing between approaches
- [ ] #5 Daily memory log updated with session outcomes
- [ ] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
