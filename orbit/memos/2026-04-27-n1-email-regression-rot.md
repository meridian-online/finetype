# Memo: N=1 email regression spec is interview-stage rot

**Date:** 2026-04-27
**Author:** Nightingale (with Hugh)
**Status:** Observation — proposing a verify-and-close
**Tags:** specs, regression, hygiene

## What the spec is

```
orbit/specs/2026-04-20-v16-n1-email-regression/
  interview.md   (Apr 21 11:01, ~2.5 KB, 7 days old)
```

Single artefact. No `spec.yaml`, no `progress.md`, no follow-up.
Created on 2026-04-20 against the v0.6.17 release; touched once on
2026-04-21 as part of the orbit-layout migration (PR #41) that moved
all in-flight specs to `orbit/`. No content change since.

## What the regression is

A single-value email column under `--mode column` returns
`representation.text.plain_text` instead of `identity.person.email`.
Larger samples (N≥5) classify correctly. v14 returned `email` even at
N=1.

Documented severity: low. The pathological case is "I have one row of
data and want column-mode inference," which is essentially "use single
mode instead." The CLI already has `--mode single` for this.

## Why it's still here

```
| State           | Reason                                              |
|-----------------|-----------------------------------------------------|
| Interview only  | spec was never written                              |
| Severity low    | nobody felt the pain                                |
| v16-specific    | the model has changed twice since (v18 held, v19)   |
| No card         | not on any sprint backlog                           |
```

Seven days isn't long, but the spec hasn't moved through any of the
three architecture shifts since it was filed: v18 retrain (held), v19
paired retrain (shipped as default), Sharpen rule audit (3 rules
removed, models/default flipped). Each one could have changed the
behaviour. Nobody re-checked.

## Three options

**A. Verify against v19, ship a closing note.** Run the original
repro on `models/default` (now sherlock-v19-relu-s42). If fixed, write
a 5-line `progress.md` documenting the verification and close the
spec. If still broken, write `spec.yaml` and proceed normally.

**B. Add a regression test, close regardless.** The behaviour is rare
enough that a unit test in `crates/finetype-cli/tests/cli_golden.rs`
asserting "single-value email column → email or single-value email →
email" is sufficient guard. If the test passes today, close the spec
on the strength of the test.

**C. Write the spec proper.** Treat as an active card, design fix
(value-based rule R32 per decision 0048 / value-rules-only policy),
ship, evaluate.

## Recommendation: A first, then B

Verify against v19. The most likely outcome: it's already fixed,
because v19 has substantially different value-branch behaviour and the
N=1 case was a v16 confidence-margin artefact, not a structural bug.
Close with a regression test (option B) to lock in the verification.

Cost: ~15 minutes. Less than writing this memo.

The rule of thumb that surfaces here: **specs at interview state for
more than one model promotion are stale by definition.** v19 shipped
on 2026-04-27. Anything filed pre-v19 against v16 needs verification
before it gets implementation budget. Don't write specs against models
two architectures back.

## Process implication

The orbit workflow already has `/orb:discovery` for ambiguous
problems. Interview-state artefacts should either advance to spec
within one sprint or close as resolved/won't-fix. Long-tail
interview-state files are a backlog smell — they accumulate context
and never ship.

A linting pass: any interview.md older than 30 days without a sibling
spec.yaml gets a comment in the next review:

```
- close (no longer reproducible)
- close (resolved by intermediate work, add regression test)
- promote (write spec.yaml, attach to a card)
```

That's the only three options. None of them is "leave the file alone."

## Composition

Standalone. Doesn't depend on the v0.7.0 CLI polish; doesn't depend on
the doc drift gate. Pure backlog hygiene.

Same shape as the sweep-script-graveyard memo and the repo-cleanliness
memo — three observations of stale state surfacing in the same
session, suggesting a broader theme: **the repo accumulates mid-flight
artefacts and never cleans them up at completion.** Bundle if useful.

## Not action yet

Observation memo. The verify-and-close itself is ~15 minutes; the
process change ("close interview-state files within one promotion
cycle") wants a one-line addition to the orbit guidance, not a memo.
