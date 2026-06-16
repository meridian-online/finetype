# Design note: categorical + alphanumeric_id recall rule

**Date:** 2026-06-17
**Spec:** 2026-06-17-categorical-alnum-recall-rule
**Card:** 0002-semantic-type-detection
**Mode:** evidence-driven design (no Q&A — decisions deferred to studies)

## Why these two types

The current per-type gold breakdown (2026-06-12 snapshot, ranking stable) puts the
two biggest accuracy holes at **categorical (~0.25)** and **alphanumeric_id
(~0.5)**, both with adequate gold support (106, 59). Coordinates / ISO dates /
decimals are already ~1.0. The 0.800 headline is gated by these residual/id
buckets — so this is where recall recovery moves the number most.

## Why a post-pass rule, not a skip, not a retrain

- **Not a skip:** the ac-04b sweep (memory `decisive-stat-skip-is-no-go`) proved no
  bare full-column stat clears 98% — each signature is shared by a high-frequency
  impostor. A skip on these types relocates error. The lever survives only as a
  post-pass rule that **keeps the neural vote** and layers value-shape guards.
- **Not a retrain:** categorical cannot be trained as a flat-softmax class (memory
  `categorical-is-a-residual-category`); additive retrains are 0-for-4. The shipped
  path is value-based Sharpen rules (decisions 0048/0096).

## The evidence that constrains the design (so the studies don't rediscover it)

1. **Categorical broadening is already refuted.** `text_vocab_override`
   (value_sharpen.rs:429) is scoped to `word`-only *on purpose*: the corpus-honest
   gate killed the broad-trigger version — entity_name 3,752 + plain_text 2,115
   oracle-refuted moves (spec 2026-06-12-text-vocab-override round 1). A column
   repeating eight manufacturer names IS entity_name; low cardinality does not make
   it an enum. **So ac-01 does NOT broaden the trigger label set.** Its only lever
   is swapping the 100-sample distinct for the EXACT full-column distinct
   (`ColumnScanStats`). Honest prior: narrow headroom; may be a NO-GO.

2. **Alnum is capped by the veto trigger.** `veto_shape_fallback` (mod.rs:3069)
   fires only on validation-VETOED columns. Alnum columns the model calls
   url/plain_text/uuid pass their own validators, so no veto fires and they are
   never recovered — that is the headroom ac-02 targets, by extending the trigger
   to those neural labels with full-column high-card + alpha + digit stats and
   url/uuid/address EXCLUSION guards (the ac-04b sweep showed URL is the dominant
   contaminant: 17 of 79 misfires).

3. **The ceiling is ~0.82, not 0.98** (ac-04b compounded sweep). This is a recall
   rule whose bar is precision-HELD, not an assertion at 98%.

## The decision discipline (every threshold is a study output)

- **ac-00** re-baselines on today's gold — no claim is measured against the stale
  06-12 reports.
- **ac-01 / ac-02** are gold studies that produce the recall/precision curves and
  name the ship-candidate setting. The author does not pre-pick thresholds; the
  curve does (the ac-04b model).
- **ac-04** is the blocking ship gate: recall-up + precision-held + headline-
  non-regression + **corpus-honest GO** — the same four dials that gated the four
  shipped fixes, and the relocation detector that killed the broad categorical
  trigger. A NO-GO blocks that rule only; the other may still ship.

## Expected shape of the result (a prediction to be tested, not a decision)

Alnum is the more promising side (real uncapped headroom, clear contaminant to
guard against). Categorical may well return a narrow gain or a NO-GO. Either is a
clean outcome — the spec is built so a per-type NO-GO lands as an explicit no-op,
exactly like ac-04b.

**Next step:** implement ac-00 (re-baseline), then the ac-01/02 studies, before any
rule code.
