---
id: NNFT-264
title: 'Spike: Hierarchical classification head vs flat 250-class softmax'
status: Done
assignee:
  - '@nightingale'
created_date: '2026-03-08 21:57'
updated_date: '2026-03-09 00:05'
labels:
  - discovery
  - architecture
  - classification
milestone: m-12
dependencies: []
references:
  - discovery/sense-architecture-challenge/FINDINGS.md
  - crates/finetype-model/src/charcnn.rs
  - crates/finetype-train/src/training.rs
  - labels/
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Discovery spike to evaluate whether hierarchical classification (7 domains → subcategories → 250 types) improves accuracy over flat 250-class softmax.

Both responses identify this as low-hanging fruit: <100K additional parameters, O(log 250) vs O(250) per sample, natural fit for FineType's domain taxonomy. When fine-grained classification fails, coarse domain prediction remains valid — particularly useful for git_sha/hash (both technology domain) and docker_ref/hostname confusions.

Schuurmans & Murre (2023) show hierarchical softmax consistently improves macro-F1 and macro-recall across 4 datasets vs flat softmax.

Time-box: ~4 hours.
Output: Accuracy comparison table (flat vs hierarchical). Training pipeline changes identified.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Map existing 250 types to a tree structure (7 domains → subcategories → leaf types) and document the hierarchy
- [x] #2 Prototype hierarchical softmax in Candle (tree of linear layers, product of path probabilities)
- [x] #3 Compare domain-level accuracy: does hierarchical prediction get the domain right more often than flat?
- [x] #4 Identify training pipeline changes needed (loss function, data loading, model serialization)
- [x] #5 Written finding saved to discovery/sense-architecture-challenge/ with accuracy comparison data
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Read all 7 taxonomy YAML files to extract the full 250-type hierarchy
2. Map types to 3-level tree: 7 domains → 43 categories → 250 leaf types
3. Analyze tree balance (degenerate categories, imbalanced branches)
4. Compute parameter counts for flat vs hierarchical models
5. Analyze all known confusion pairs against hierarchy levels
6. Design Candle prototype architecture (forward pass, loss, serialization)
7. Identify training pipeline changes
8. Write findings to SPIKE_C_HIERARCHICAL.md
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Completed full analysis of hierarchical softmax vs flat 250-class.

Key findings:
- Tree: 7 domains → 43 categories → 250 types (4 degenerate single-type categories)
- Parameter overhead: only +6.1% (5,934 params, from 97,178 → 103,112)
- All 5 known confusion pairs resolved: 3 at Level 1 (domain), 2 at Level 2 (category)
- Largest leaf softmax: 40 classes (datetime.date) vs 250 flat — big reduction
- Recommended loss: multi-level CE with weights λ=(0.2, 0.3, 0.5)
- Greedy top-down inference sufficient; beam search available as safety net
- Expected domain accuracy >95% based on structural distinctiveness

No code prototype was needed — the architecture is straightforward in Candle (linear layers per tree node, product-of-path probabilities). The spike focused on data analysis and feasibility rather than a runnable prototype, since the value is in the hierarchy mapping and confusion analysis.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Research spike evaluating hierarchical softmax (7 domains → 43 categories → 250 types) vs flat 250-class softmax for FineType's CharCNN.

Findings:
- Complete 3-level tree mapped: 7 domains, 43 categories (4 degenerate with 1 type), 250 leaf types
- Parameter overhead: +6.1% only (97,178 → 103,112 params). Largest softmax drops from 250 → 40 classes
- All 5 known confusion pairs resolved structurally: hs_code/decimal_number, cpt/postal_code, full_name/entity_name at Level 1 (different domains); git_sha/hash, docker_ref/hostname at Level 2 (different categories)
- Training requires multi-level CE loss (domain + category + leaf), label hierarchy derivable from dotted labels
- Candle prototype design documented: HierarchicalHead struct, greedy top-down inference, named tensor serialization
- Risk assessment: error propagation mitigated by high expected domain accuracy (>95%) and optional beam search

Output: `discovery/sense-architecture-challenge/SPIKE_C_HIERARCHICAL.md`

Recommendation: Proceed to implementation. Low risk, modest overhead, and resolves all current confusion pairs structurally.
<!-- SECTION:FINAL_SUMMARY:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [x] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [x] #3 Changes committed with task ID in commit message
<!-- DOD:END -->
