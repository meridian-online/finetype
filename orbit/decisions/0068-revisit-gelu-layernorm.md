---
status: proposed
date-created: 2026-04-25
date-modified: 2026-04-25
supersedes: 0046
---

# 0068. Revisit GELU+LayerNorm architecture

## Context and Problem Statement

Decision 0046 (2026-04-12) closed the GELU+LayerNorm experiment with a
measured -5 label regression vs ReLU+BatchNorm on profile eval (188/227
vs 193/227). However, the Sharpen post-processing layer has changed
substantially since that measurement:

- **PR #44** (2026-04-21): Sharpen demotion guard — validator-confirmed
  rescue for types demoted by categorical rules
- **PR #47** (2026-04-24): Amount-variant collapse fix — 11-arm hint
  table edit, +11 target lift, +10 non-target lift
- **PR #48** (2026-04-25): Header-hint regex removal — removed harmful
  regex-based header hints per decision 0042

The v10 experiment (decision 0046) noted that "val_accuracy and profile
eval measure different things — profile eval includes Sharpen
post-processing, which the GELU+LN output distribution doesn't interact
well with." Given that the Sharpen layer's behaviour has changed
materially, the GELU+LN interaction may be different.

Additionally, the eval corpus has expanded from 227 to 448 rows,
providing a more representative measurement surface.

## Considered Options

- **Option A:** Keep decision 0046 — GELU+LN is closed, don't revisit
- **Option B:** Re-run paired comparison on improved training data
  through today's pipeline

## Decision Outcome

Chosen option: **Option B — re-run paired comparison**, because:

1. The Sharpen layer has changed enough that the original measurement
   is no longer representative of the GELU+LN interaction
2. The training data is also improving (v4 corpus + container types +
   datetime generator improvements) — the combined effect of better
   data + different architecture may be different from either alone
3. The GELU+LN infrastructure already exists in both train and
   inference crates (backward compatible via `#[serde(default)]`)
4. A 15-hour overnight sweep is acceptable compute cost to get a
   definitive answer on today's pipeline

### Measurement plan

- 3-seed sweep (42, 43, 44) × 100 epochs for each architecture
- Both architectures train on identical FTMB v5 data
- Three-way diff: v16 baseline (297/352) vs ReLU-v19 vs GELU-v19
- MADR 0066 hard gate applies independently to each architecture
- Winner takes all — no margin requirement

### Outcome

_To be filled after sweep results._

### Consequences

- Good, because a definitive answer on today's pipeline removes the
  "what if" from future model discussions
- Good, because the GELU+LN code is already maintained — the experiment
  costs compute, not engineering time
- Bad, because the paired sweep doubles overnight compute (~15h vs ~7.5h)

## Cross-references

- **Decision 0046** — original GELU+LN not-adopted decision (superseded)
- **Decision 0066** — v19 retrain hard gate (applies to both architectures)
- **Decision 0038** — strength through simplification
- Spec: orbit/specs/2026-04-25-v19-paired-retrain/
