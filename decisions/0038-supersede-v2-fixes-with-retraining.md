---
status: accepted
date-created: 2026-03-20
date-modified: 2026-03-20
---
# 0038. Supersede distillation v2 heuristic fixes with retraining path

## Context and Problem Statement

Distillation v2 identified 9 fixes grouped into 3 PRs (spec: `specs/2026-03-18-distillation-v2-fixes/`). PR-1 (pipeline guards: boolean binary, points guard, epoch detection, financial guard) shipped as `54bb26a`. PR-2 (5 column heuristic fixes) and PR-3 (eval update + re-score) were never implemented — displaced by the `rm -rf` incident and overtaken by distillation v3.

Distillation v3 has now produced 83K+ adjudicated labels from real-world Sherlock data. The project direction has shifted toward "strength through simplification" — reducing disambiguation rules rather than adding more.

## Considered Options

- **Option A: Implement PR-2 and PR-3 as originally specified** — Add 5 new disambiguation rules (categorical/ordinal default, username demotion guard, float-stored ID detection, sequential ID detection, sentence/entity_name length heuristic) plus offline re-score script.
- **Option B: Supersede with retraining path** — Use distillation v3 data to retrain the CharCNN so the model absorbs the patterns that PR-2's rules would have addressed. Build Tier 2 benchmark instead of the ad-hoc re-score script.

## Decision Outcome

Chosen option: "Option B — Supersede with retraining path", because:

1. PR-2's fixes are all heuristic rules in `column.rs`. Adding 5 more rules increases maintenance burden and brittleness. The distillation data provides a path to make the model handle these cases natively.
2. PR-3's offline re-score is superseded by the Tier 2 distillation benchmark — a more rigorous, stratified evaluation framework built from 83K+ labelled columns.
3. The *problems* identified by PR-2 remain valid test cases. They are carried forward as evaluation targets for the retraining spike, not discarded.

### What shipped (PR-1)

- fix-1: Boolean binary heuristic (require >=2 distinct values)
- fix-3: "points" model2vec guard
- fix-6: Epoch seconds detection
- fix-9: Financial model2vec guard (yield, pct_change)

### What is superseded (PR-2)

These problems become retraining spike test cases:
- fix-2: Categorical vs ordinal default (+123 agreements)
- fix-4: Username demotion guard (+92 agreements)
- fix-5: Float-stored integer ID detection (+83 agreements)
- fix-7: Sequential ID vs amount_minor_int (+77 agreements)
- fix-8: Sentence vs entity_name length (+71 agreements)

### What is superseded (PR-3)

- Offline re-score script → replaced by Tier 2 distillation benchmark
- Eval update → deferred to post-retraining evaluation

### Consequences

- Good, because fewer rules means a simpler, more maintainable disambiguation pipeline
- Good, because real-world training data should generalise better than hand-crafted heuristics
- Good, because Tier 2 benchmark is a more rigorous evaluation than the ad-hoc re-score
- Bad, because the retraining spike may not absorb all 5 patterns — some rules may still be needed
- Mitigation: if retraining doesn't fix a pattern, it can be added as a targeted rule at that point with Tier 2 evidence
