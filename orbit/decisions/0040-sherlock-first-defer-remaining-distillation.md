---
status: accepted
date-created: 2026-03-21
date-modified: 2026-03-21
---
# 0040. Complete Sherlock distillation first, defer remaining sources

## Context and Problem Statement

The distillation pipeline covers 4 sources: Sherlock (1,374 batches), GitTables (448), SOTAB (0), and eval (3). Sherlock is at ~69% and expected to complete today. The retraining spike (PR #18) confirmed that blended distilled+synthetic data improves Tier 2 accuracy from 75.0% to 80.6%. The question is whether to distill all sources before retraining or ship with Sherlock-only.

## Considered Options

- Complete all sources before retraining (~900 batches remaining across GitTables/SOTAB/eval)
- Complete Sherlock only, retrain and ship, defer remaining sources to next sprint

## Decision Outcome

Chosen option: "Complete Sherlock only, retrain and ship", because the spike already proved the value of distilled data using Sherlock-only results. Sherlock provides 137K columns — the largest and most diverse source. Waiting for GitTables/SOTAB delays shipping a meaningful accuracy improvement for diminishing marginal returns. Additional sources can be distilled and blended in a future training cycle.

### Consequences

- Good, because we ship an improved model this sprint instead of waiting for full distillation
- Good, because Sherlock alone provides 122+ distilled types with strong coverage
- Bad, because GitTables (44K columns) and eval (282 columns) data is deferred — those sources may cover types Sherlock doesn't
- Neutral, because the training pipeline is reusable — future distillation rounds feed directly into the next blend
