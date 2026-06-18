# ac-02 — representative baseline + study reconciliation

**Spec:** 2026-06-18-representative-accuracy-gate
**Date:** 2026-06-18 · binary 0.6.34 · `models/default` (sherlock-v19-relu-s42)

## v19 representative baseline (record this)

| lens | headline | n | 95% CI |
|---|---|---|---|
| **reframe (primary, gate against this)** | **0.691** | 259 | 0.632–0.744 |
| raw / legacy | 0.610 | 259 | 0.549–0.667 |

The existing `score_gold_anchor.py predict | score --reframe` reads the fixture
unchanged (it mirrors gold's schema; all 260 columns resolve in `columns.parquet`).
No new scoring code. 1 of 260 did not score (no prediction returned), so n=259.

## Reconciliation — why this is not the study's 0.648 (discrepancy run down, not banked)

The study reported raw-panel **0.648**; a fresh run lands at raw **0.610**. Both
parts are explained and intended:

1. **Denominator.** The study scored **250** (it dropped 10 harness-artifact
   columns); its 162 correct / 250 = **0.648**. Those same study predictions over
   the full set = 162/260 = 0.623. The fixture keeps all 260, so the denominators
   differ. Reconciled — not a labelling drift.

2. **Prediction drift: 23 of 259 changed between the study's 0.6.33 binary and
   0.6.34 — all intended Sharpen evolution, not regression:**
   - **~16 `categorical → word`.** The enum reframe (spec 2026-06-17, shipped)
     retired the categorical-emitting Sharpen rules, so columns that were
     `categorical` now surface as `word`. **Invisible under `--reframe`** (both
     collapse to the text RESIDUAL) — which is why the raw number dropped and the
     reframe number did not.
   - **~6 `Author: full_name → username`.** The shipped username veto (spec
     2026-06-17-full-name-username-veto) correctly fires on login-handle columns —
     the single highest-value fix the representative study itself flagged.
   - 1 `Layer name: alphanumeric_id → username`.

**Lesson, baked into the fixture's design:** the raw headline is unstable to
Sharpen-rule changes the reframe lens is invariant to. The representative fixture
is therefore scored under `--reframe`, same as gold — and the gate reads the
reframe number. The raw 0.610 / study 0.648 are recorded only as the
reconciliation trail.

## Consistency check

The reframe lift mirrors gold's: gold raw 0.793 → reframe 0.797 (+0.4pp); repr raw
0.610 → reframe 0.691 (+8.1pp). The larger repr lift is expected — representative
data carries far more of the categorical/word residual mass the reframe collapses
(repr: 36 categorical + 39 plain_text + word, vs gold's curated-hard mix). The two
instruments move the same direction under the same lens; the magnitudes differ
because the populations differ. Coincidentally, reframe 0.691 sits near the study's
adjudicated 0.68 band — but for a different reason (reframe collapse, not the
unpersisted adjudication), so it is not treated as reproducing the adjudicated number.
</content>
