---
status: accepted
date-created: 2026-04-28
date-modified: 2026-04-28
---
# 0074. Validate-corpus iter-1 in-scope vs deferred fix mechanisms

## Context and Problem Statement

The validate-precision harness attributes every failing column to one
of four named mechanisms: `enum_overfit`, `format_diversity`,
`misclassification`, `code_vs_canonical` (plus `unknown` as a
quality-signal fallback and `no_gt` for ungrounded columns). Iteration
1 must decide which mechanisms get fixes alongside the harness
landing. Fixing all four would couple the harness ship to a model
retrain; fixing none would ship a harness with no movement story.
The choice has to balance scope against signal.

## Considered Options

- **Fix `enum_overfit` + `format_diversity`; defer `misclassification`
  + `code_vs_canonical`.**
  The two in-scope mechanisms are addressable by code/YAML edits
  alone (no model retrain): enum-overfit by tightening the JSON
  Schema cardinality cap (50→32 default + boolean.* gate parity);
  format-diversity by widening 1–5 taxonomy validators (precision
  principle still holds — each widening rejects clearly-invalid
  input). Misclassification needs the model to learn a different
  mapping (retrain, gated by MADR 0066). Code-vs-canonical needs
  a cross-cutting taxonomy decision (when does
  `geography.country.iso_3166_alpha2` accept 3-letter NOC codes?
  That's a label-design question, not a regex tweak).
- **Fix all four mechanisms in iter-1.**
  Maximum movement. But couples the harness PR to a model retrain
  PR (multi-day, gated) and a taxonomy redesign for the canonical
  seam — easily three sprints of work on the critical path. The
  harness, which is independently valuable, would block on the
  slowest item.
- **Fix only `enum_overfit`; defer `format_diversity` too.**
  Minimum coupling. But format-diversity widenings are
  mechanically the simplest fix in scope (one-line YAML edits
  with regression tests) and skipping them would leave the
  harness with one fewer demonstrated movement story.
- **Ship harness only, no fixes.**
  Cleanest separation: harness lands, baseline measured,
  follow-up cards do the fixes. But the harness without a
  paired fix cycle means iter-1 has no proof that the harness
  *drives* fixes — only that it *measures* them. The two
  selected fixes are precisely the cheap proof.

## Decision Outcome

Chosen option: **"Fix `enum_overfit` + `format_diversity`; defer
`misclassification` + `code_vs_canonical`"**, because these are the
two mechanisms whose fixes don't require a model retrain or a
taxonomy redesign — code/YAML edits alone — so they can ship in the
same PR as the harness and demonstrate the measurement→fix loop
without blocking on the slow critical path.

### Consequences

- Good, because the iter-1 PR is self-contained: harness +
  baseline + two fixes + post-fix delta + three MADRs, no
  retrain dependency, no taxonomy redesign dependency.
- Good, because deferred mechanisms remain visible — the
  per-mechanism breakdown counts and names them, surfacing
  the future work without hiding it.
- Good, because each in-scope fix is reversible: the
  enum-threshold default is a `default_value="32"` literal in
  one clap struct; each YAML widening is a one-line revert.
  No architectural lock-in.
- Bad, because the headline `N of M datasets pass` may not
  improve — the fixes target mechanism reduction, and the
  remaining failure surface (misclassification-driven) can
  still drag whole datasets below P=99%. Iter-1's iter-1
  baseline exemplifies this: format_diversity drops 1→0, but
  the misclassification surface is unchanged so the headline
  stays at 3 of 7.
- Bad, because misclassification fixes wait for the next
  retrain. MADR 0066 imposes a hard gate on retrains (3-seed
  sweep, +3 lift threshold, ≤-1 regression bound), so this
  may take multiple sprints.
- Neutral, because code-vs-canonical fixes wait for a
  cross-cutting taxonomy decision (a separate MADR), which
  is the right pace for a label-design change. The
  `code_vs_canonical` mechanism in the report is its own
  forcing function — if the count grows, the decision
  becomes more urgent.

References: MADR 0066 (retrain hard gate), MADR 0001 (precision
principle for ac-10 widenings).
