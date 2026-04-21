---
status: accepted
date-created: 2026-03-18
date-modified: 2026-03-18
---
# 0037. Eval Serves the Engine, Not the Other Way Around

## Context and Problem Statement

When implementing fixes identified by the distillation v2 pipeline (5,364 columns, 10 systematic gap categories), some fixes may regress the existing profile eval (170/174 on 30 datasets) while being correct according to the distillation evidence (2,047 reasoned disagreements with adjudication). The question is whether the profile eval is a hard gate that can block correct fixes.

## Considered Options

- Profile eval is the hard gate — fixes must not regress the 170/174 score
- Distillation evidence is primary — profile eval expectations can be updated when a fix is correct
- Feature-flag new behaviour — preserve both baselines, promote after validation

## Decision Outcome

Chosen option: "Distillation evidence is primary", because the eval exists to measure the engine's accuracy, not to constrain it. If a fix is correct by the distillation evidence but regresses the profile eval, we should update the eval expectations rather than preserve a wrong baseline.

### Consequences

- Good, because fixes are not blocked by potentially stale eval expectations
- Good, because the distillation pipeline (5,364 columns with reasoned adjudication) is a stronger ground truth than the profile eval (174 columns with manual labels)
- Bad, because profile eval regressions require investigation to confirm the fix is genuinely correct before updating expectations
- Neutral, because both eval sources must still improve overall — this decision covers individual cases where they conflict
