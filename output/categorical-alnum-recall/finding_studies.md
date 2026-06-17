# ac-00/01/02 — re-baseline + recall-rule studies

**Spec:** 2026-06-17-categorical-alnum-recall-rule
**Date:** 2026-06-17
**Substrate:** `scripts/study_recall_rule.py`, `output/categorical-alnum-recall/{recall_study.md, report_baseline_v19_2026-06-17_*.md}`

## ac-00 — today's baseline (post-re-adjudication, shipped v19)

Headline **0.789** (731/927; CI 0.761–0.814). The two targets are far healthier
than the stale 2026-06-12 snapshot suggested — high precision, real recall gap:

| target | support | P | R | FN |
|---|---|---|---|---|
| categorical | 106 | 0.870 | 0.443 | 59 |
| alphanumeric_id | 59 | 0.952 | 0.678 | 19 |

## Headline finding — my pre-study prediction inverted

I predicted alnum was the promising side and categorical narrow/no-go. **The
data says the opposite.** Measuring before coding paid for itself.

## ac-02 alphanumeric_id — NO-GO (stat-gated override)

The alnum FN concentrate on `unknown` (7) and `url` (4). But a full-column
shape override does not recover them cleanly:

- `unknown` → alnum fires on **zero** columns — the unknowns do not carry the
  alnum shape (mixed letters+digits, near-unique).
- adding `url` to the triggers fires on 20 and **breaks 18 correct URL
  predictions** to recover 2 alnum — precision collapses **0.952 → 0.68**.

Alnum's defining shape (high-card alpha+digit) is shared by real URLs, the same
contaminant the ac-04b sweep surfaced. There is no clean full-column-stat recall
rule for alnum. **NO-GO** — matches `decisive-stat-skip-is-no-go`.

## ac-01 categorical — gold says GO, but the headline number is a trap

The categorical FN (59) scatter across ~19 labels. A low-card override recovers
recall at apparently-held precision, scaling with how many trigger labels it
absorbs:

| trigger set (card≤0.6) | recovered | broke correct | R | P |
|---|---|---|---|---|
| word | 4 | 0 | 0.443→0.481 | 0.870→0.864 |
| word + ordinal | 9 | 0 | 0.443→0.528 | 0.870→0.862 |
| word + ordinal + **entity_name + plain_text** | 16 | 0 | 0.443→**0.594** | 0.870→0.863 |

The bottom row looks like the best fix of the session — +16 recall, precision
held, zero correct predictions broken. **It is a gold mirage.** `entity_name` +
`plain_text` broadening is *exactly* the move the corpus-honest gate killed at
corpus scale (3,752 + 2,115 oracle-refuted moves, spec 2026-06-12-text-vocab-override
round 1): a column repeating eight manufacturer names IS entity_name; repeated
boilerplate IS plain_text. Gold holds only 12 entity_name + 21 plain_text
columns — too few to show the relocation the corpus's thousands reveal. **This is
the canonical demonstration of why the corpus-honest gate is blocking and gold
is not sufficient.**

## What is actually shippable from ac-01

The variants that avoid the refuted labels:

- **`word`, full-column stats: +4 recall, P held (0.864), 0 broke.** This is the
  shipped `text_vocab_override` (sample-based) upgraded to the exact full-column
  cardinality — it catches 4 `word` columns the 100-row sample misses. Lowest
  risk (word is already a validated categorical trigger).
- **`word + ordinal`: +9 recall, P held (0.862), 0 broke.** Adds ordinal, which
  has zero gold TP to break — but ordinal→categorical relocation at corpus scale
  is unmeasured. Candidate, pending the gate.

Both must still clear the corpus-honest gate (ac-04) before shipping — the gate
is the only instrument that can see the relocation gold cannot.

## Recommendation

1. Ship-candidate: **upgrade `text_vocab_override` to full-column stats (word)**
   — +4 recall, lowest risk. Then test **+ordinal** through the gate.
2. **Do NOT ship the entity/plain broadening** despite its gold appeal — it is
   the refuted move; the gate will (correctly) reject it.
3. alphanumeric_id: **no rule** — the stat shape is url-contaminated.

## Scope / what we don't know

- Full-column stats resolved for 748/927 predicted columns; 179 lack stats
  (column-name mismatches / read errors) — the alnum `unknown` FN may be partly
  among them, but even with stats the alnum shape is url-shared, so the NO-GO
  holds.
- The categorical word/ordinal candidates' real verdict is the corpus-honest
  gate, not these gold deltas. Gold is necessary, not sufficient — this study is
  itself the proof.

---

## ac-03/04 ADDENDUM — the categorical fix is a NO-GO (full-file verification)

Implemented the `word` full-column upgrade, then verified it on the **actual full
files** (not the truncated-sample harness). It recovers **nothing real**:

- `Type` (one winner): on the full file the model ALREADY returns `categorical`
  with no rule — correct without any fix. The baseline called it `word` only
  because the harness fed it a **1-value** truncated sample.
- `Unnamed: 4` (other winner): on the full file it is `top_level_domain` — not
  `word`, so the override never applies.

**Why the +2 was spurious — a measurement confound.** The study joined baseline
predictions from the *truncated-sample* harness (some columns have `n=1` stored
samples) with full-column stats from disk — a pairing that occurs in neither
production (full files → different predictions) nor the harness (truncated → fix
inert).

**Why the upgrade is worthless even in principle.** `text_vocab_override` fires
when distinct ∈ [2,12] and distinct/n ≤ 0.6. Sample distinct ≤ full distinct
always (subset), and a proper 100-value sample of a low-cardinality column gives
ratio ≤ 12/100 = 0.12 — so the **sample-based rule already fires whenever the
full-column version would**. The only exception is a degenerate `n=1` sample,
which is the harness artifact, not production. Code reverted.

## The finding that actually matters

The gold eval harness feeds the binary `sample_values_truncated`, and some columns
have **degenerate `n=1` samples**. This (a) pessimistically misclassifies columns
the model gets right on full data (`Type`→categorical), so part of the categorical
"recall gap" is a harness artifact not a model deficiency; and (b) makes any
full-column-statistic study invalid, because the baseline predictions and the
stats come from different value sets. **Full-column-stat Sharpen fixes cannot be
studied on the truncated-sample harness.**

## Net verdict for the spec

- alphanumeric_id: NO-GO (url-contaminated shape).
- categorical: NO-GO (full-column upgrade redundant with proper sampling; apparent
  gap partly a harness artifact).
- **The column-statistics lever ships nothing** — neither as a skip (ac-04b) nor
  as a recall rule. ColumnScanStats (ac-04a) remains as tested plumbing with no
  current consumer.
- Real follow-up: the truncated-sample harness (n=1 columns) — fix the eval to
  feed proper samples before trusting low-sample gold verdicts.

---

## CORRECTION (2026-06-17) — the "n=1 harness" finding was wrong

The addendum above claimed the gold harness feeds degenerate `n=1` truncated
samples. **That was a separator bug in my analysis** — I split
`sample_values_truncated` on `\x1f` instead of the real `SEP = "│"`
(score_gold_anchor.py:50), so every multi-value sample collapsed to one element.

Corrected facts:
- Stored samples are **~8 representative values** (median 8, max 8), not 1.
- Gold was adjudicated from those samples (the `sense_context`/`ydf_context`
  fields), so the eval is **internally consistent** — the harness is the right
  instrument.
- Re-baselining by reading larger source samples (`scripts/predict_from_source.py`,
  cap 5000) made accuracy **WORSE: 0.785 → 0.707** (84 columns right→wrong,
  e.g. `msg_id` alphanumeric_id→h3). Feeding the model more data than the
  labeller saw breaks consistency with gold.

**The "fix the harness / understates accuracy" claim is RETRACTED.** The harness
is not broken. Headline of record stands at 0.800 (745/931); a fresh re-run
measures ~0.785 (4 columns fail to load + variance), same instrument.

What survives unchanged: the column-statistics lever ships nothing (alnum
url-contaminated; categorical upgrade redundant — full-file profiling confirmed
the 2 'winners' classify correctly without any fix).
