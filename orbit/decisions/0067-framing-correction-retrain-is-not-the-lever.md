---
status: accepted
date-created: 2026-04-24
date-modified: 2026-04-24
supersedes: 0062-followups
---

# 0067. Framing correction — retrain is not the lever for header-hint failures

## Context and Problem Statement

`orbit/specs/2026-04-21-v18-retrain/handover.md` closed with three
follow-up cards, framed as generator work:

> - amount-variant generators (11 persistent misses collapsing to plain `amount`)
> - container-type generators (8 collapsing to `categorical`)
> - datetime-subtype generators (6 collapsing to nearest-but-wrong timestamp)

That framing presumed the remedy lived in training data — that the
multi-branch model could not distinguish the variants because its
training corpus under-represented the shapes. The
`2026-04-24-amount-variant-generators` spec tested that presumption and
found it **false** for the amount family:

- **ac-01**: corpus counts 294–357 across 11 subtypes + plain amount.
  max/min = 1.214. Corpus is near-balanced. Imbalance ruled out.
- **ac-02**: Jaccard similarity across the 12 subtypes' generated
  value-shape signatures: mean off-diagonal = 0.0102, max = 0.1935.
  Shapes are near-disjoint. Overlap ruled out.
- **ac-03**: v16 profile on the 11 eval columns. All 11 predict plain
  `finance.currency.amount`. Disambiguation trace attributes every
  prediction to `header_hint_*`.
- **ac-04**: raw multi-branch softmax (Sharpen bypassed via
  `MultiBranchClassifier::classify_column_topk`). Top-1 confidences
  range 0.33–0.99. 1/11 correct at top-1, 3/11 in top-5. 8/11 target
  labels absent from top-5 entirely — a model representation gap, but
  **not** the dominant pipeline failure.
- **ac-05** (MADR 0065): primary mechanism named `other` — `header_hint()`
  at `crates/finetype-model/src/column.rs:4303-4314` over-generalises on
  the `amount` substring and forces every variant header back to plain
  `finance.currency.amount` via the Sharpen hint-override branches.
- **ac-06**: fix was an 11-arm addition to the `match h` block. No
  training data changed. No model weights changed.
- **ac-07**: 11/11 target columns flipped from plain amount to their
  expected variant after the fix.
- **ac-09/ac-10**: on the 448-row eval manifest, net target lift = **+11**,
  non-target regression delta = **+10** (strictly positive overall).

## Considered Options

- Option A — keep the v18 handover framing. Continue treating container
  and datetime subtype failures as training-data problems; write new
  generators for each.
- Option B — retract only the amount-variant framing. Leave container
  and datetime cards unchanged on the assumption those families are
  different.
- Option C — retract the whole training-data framing as the default
  hypothesis for header-driven subtype collapses. Require a diagnostic
  arc analogous to `2026-04-24-amount-variant-generators` (corpus counts,
  value-shape disjointness, confusion matrix, raw softmax top-k) before
  committing to any training-layer remediation.

## Decision Outcome

Chosen option: **Option C — diagnostic arc precedes any training-layer
commitment for header-driven subtype collapses.**

The amount-variant episode demonstrates that the cheapest lever for a
multi-class header-driven collapse is often a **hint-layer edit, not a
retrain**. The diagnostic arc takes <1 day; a 3-seed retrain sweep takes
~7.5 hours and — as this spec proved — can move 0/11 where a 12-arm
table edit moves 11/11.

Concrete guidance for future `/orb:discovery` sessions on the two
remaining v18 follow-up clusters:

1. **container-type generators (card TBD)** — before writing generators,
   run the equivalent of ac-01..ac-04 on the 8 persistent columns. The
   `header_hint()` block contains broad substring checks for type names
   that may be the actual failure mechanism; if the raw softmax already
   carries the correct label in top-5, no retrain is needed.
2. **datetime-subtype generators (card TBD)** — same pattern. Datetime
   has 84 taxonomy definitions; substring aliasing is even more likely
   to short-circuit correct predictions.

If and only if the diagnostic arc lands on `imbalance`, `overlap`,
`confident_wrong`, or `flat_confidence` as the primary mechanism does
training-data or model-architecture remediation become the right
response. The closed enum from MADR 0065 carries forward as the
per-family triage vocabulary.

### Consequences

- Good, because future specs in this pattern start by measuring before
  remediating — reducing compute waste and surfacing the cheapest fix.
- Good, because the "hint layer is destructive" signal now has a name
  and a precedent; future spec authors don't have to rediscover it.
- Bad, because the diagnostic arc requires tooling
  (`classify_column_topk`, corpus-count harnesses, Jaccard computation).
  Each new family may need minor extensions.
- Bad, because ruling out `other` requires reading into the hint layer
  by hand. A future enhancement could add a `header_hint()` debug tool
  that annotates the pipeline path for a given header.

## Cross-references

- **Decision 0042** — remove regex header hints (strategic direction).
  This MADR operationalises 0042 for subtype-collapse diagnosis.
- **Decision 0048** — value-based rules only (header disambiguation
  waits for model improvements). The `header_hint()` layer is the legacy
  being narrowed; this MADR documents one such narrowing.
- **Decision 0062** — v18 hold. The "retraining alone is not the lever"
  conclusion generalised here.
- **MADR 0065** — amount-subtype collapse mechanism.
- **MADR 0066** — v19 retrain hard gate.
