# B3 deferral selector — the value-expert can't be banked by deferral either

**Date:** 2026-06-08
**What we tested:** the reframe from additive fusion to **learning-to-defer** — keep v19
as a guaranteed floor, train a tiny selector on the cached 968-dim features to *override*
v19 with the value-expert's label only on columns it's confident about. Collapse-proof by
construction (worst case: never fires → exactly v19).
**Verdict:** the architecture does exactly what it promises (improve-or-hold holds; one
class loses 0.001; zero collapse) — but it **banks no win**, because the value-expert
alone holds neither headline boundary. Dead end for the value-expert fusion thesis.

## The two numbers that settle it (gold anchor, 240 cols)

**Family A — `alphanumeric_id` (tight-code vs alphanumeric), n=30:**
- v19 floor: **12/30**. Value-expert alone: **12/30**. *No standalone advantage.*
- The "17%→93%" family-A win in `ac07_ship_gate_decision.md` was the **additive head's**
  (head_logits + α·mb_logits combined), **not** the expert's. Deferral overrides with the
  expert's *own* label — which is no better than v19. There is nothing for deferral to bank.

**Family C — `latitude`, n=30:**
- v19 floor: **0/30**. Value-expert alone: **11/30**. *A real standalone win.*
- But on those 11 override-worthy columns the selector assigns **p_override = 0.04–0.16**
  (low). Why: the same expert over-emits latitude/decimal across the general corpus
  (this is why v27 over-emitted latitude ×3.13). Calibrated honestly on that corpus, the
  selector learns "the expert's latitude votes are usually wrong" and won't fire — even on
  the columns where it's right. The lat/lon signal is real but **entangled** with the
  over-emission at a level the column-level cached features cannot separate.

## General-corpus probe (held-out, 100,046 cols)

- Base/expert disagree on **64.2%** of columns; the expert is actually right on only
  **3.6%** of those disagreements. The expert is a poor witness whenever it differs from v19.
- At the precision-floored threshold (τ=0.95): override coverage **0.33%**, precision
  86.4%, **Δ +0.0026** accuracy. Improve-or-hold holds — but the gains land on
  `plain_text`/`city`, **not** the starved confusion families.
- Applied to the gold anchor, that same selector fires on **0 of 240** columns. Inert
  exactly where B3 was supposed to help.

## Why both fusion architectures fail, in one line each

- **Additive (v26/v27):** sums the expert's generic-attractor prior into v19 → collapses
  the corpus label space (unknown +52%/+68%, categorical 10.7×/6.05×, 47–90 types zeroed).
  NO-GO ×2.
- **Deferral (this):** refuses to fire wherever the expert is unreliable → safe but inert;
  banks neither family-A (expert doesn't have it) nor family-C (can't isolate it from the
  over-emission). Δ +0.0026 on the general corpus, 0 on the gold anchor.

The value-expert is not a bankable contributor to a general column classifier through
*any* score-combination mechanism we've tried. Its one genuine standalone signal (lat/lon)
is inseparable from its over-emission without a per-value distinguishing feature the
column-level views don't carry.

## Recommendation

**Abandon the value-expert fusion thesis for 0.7.0.** Keep v19 as `models/default`. The
two findings worth banking for a *future* design:
1. v19 has a real **latitude blind spot** (0/30 on the gold anchor family-C) — a genuine
   weakness, but the fix is not this value-expert. A future targeted approach needs a
   feature that separates "is a coordinate" from "is any decimal", which lives in sibling
   distribution + header, not per-value char-CNN.
2. The family-A win is an **additive-head artifact**, not an expert property — so any
   attempt to bank it must apply the *fused score* on a narrow trigger, accepting the
   head's corpus-collapse risk scoped to that trigger. Small, fragile, probably not worth
   the release.

For a compelling 0.7.0, pivot the release off the value-expert. `models/default` stays
v19; Cargo stays 0.6.23 until a different candidate clears the corpus-honest gate.

## State
- Nothing promoted. `models/default` = v19, Cargo 0.6.23, no HF model swap, CI pin unchanged.
- v25 expert preserved (local backup + published to HF `value-charcnn-v25/`) so the
  findings are reproducible.
- Evidence: `output/late-fusion/deferral_v1_report.md`, `scripts/fusion/deferral_selector.py`,
  this memo.
