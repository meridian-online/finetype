---
id: NNFT-259
title: 'Discovery: Context-aware header classifier via Model2Vec sibling embeddings'
status: To Do
assignee:
  - '@nightingale'
created_date: '2026-03-08 09:20'
labels:
  - discovery
  - model
  - m-12
dependencies:
  - NNFT-254
references:
  - crates/finetype-model/src/sense.rs
  - crates/finetype-model/src/semantic.rs
  - crates/finetype-model/src/model2vec_shared.rs
  - crates/finetype-model/src/column.rs
  - discovery/feature-retrain/FINDING.md
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Question:** Can a context-aware header classifier (Model2Vec embeddings of header + sibling column headers) replace hardcoded header hints and solve the bare-name ambiguity?

**Context:** NNFT-254 pushed rules-based disambiguation to 179/186 but hit diminishing returns. 3 of the 7 remaining misclassifications are bare "name" ambiguity — the header alone is genuinely ambiguous, but sibling columns provide the signal:
- airports.name alongside iata_code, latitude, longitude → entity_name
- world_cities.name alongside country, population → city
- multilingual.name alongside country, language → full_name

The current Sense layer encodes headers via Model2Vec but outputs only 6 broad categories — too coarse. A finer-grained classifier using header + sibling context could:
1. Replace many hardcoded header hints with learned associations
2. Solve the bare "name" problem that no amount of single-header analysis can fix
3. Provide a more robust, generalizable disambiguation signal

**Approach (Option B from NNFT-254 debrief):** Train a lightweight MLP on Model2Vec embeddings of the target header concatenated/averaged with sibling column headers from the same dataset. Output: fine-grained type prediction or type-group prediction that feeds into CharCNN masking.

**Time budget:** 1-2 days

**Success:** A trained prototype classifier + eval numbers showing whether sibling context improves disambiguation over current hardcoded hints. Written finding with architecture decision.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Prototype classifier trained on Model2Vec header + sibling embeddings with at least 2 architecture variants tested
- [ ] #2 Evaluation on the 3 bare-name cases (airports.name, world_cities.name, multilingual.name) plus full profile eval
- [ ] #3 Comparison table: current hardcoded hints vs learned classifier on all 186 GT columns
- [ ] #4 Analysis of training data requirements — how many dataset/column examples needed for robust generalisation
- [ ] #5 Written finding in discovery/ with architecture, training details, eval numbers, and recommendation on whether to adopt
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
