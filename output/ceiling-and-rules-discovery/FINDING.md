# Ceiling-and-rules discovery — finding

**Spec:** 2026-06-18-ceiling-and-rules-discovery (cards 0020, 0002)
**Date:** 2026-06-18
**Discipline:** discovery only — no model or rule ships from this work.
**Substrate (this run, reproducible):**
`output/ceiling-and-rules-discovery/{predictions_v19.tsv, predictions_v19_nosibling.tsv, reframe/, raw/, reframe_nosibling/}`
all from `models/default` (= sherlock-v19-relu-s42) on the 0.6.34 binary, scored
against `eval/gold/gold_corpus.tsv` (931 cols) by `score_gold_anchor.py … --reframe`.

---

## Headline (the recommendation)

**Accept the ~0.80 gold ceiling as real and stop spending model-side. Neither
lever moves it: the column-statistics layer was already tested to ship nothing,
and the semantic layer is stranded with a noise-level-to-negative prior. The
"fewer rules" prize is also not unlocked by either lever — the one deletion
available now (10 zero-impact header-hint families) is independent of both.**

The next bet that moves a pillar is **product, not model** — card 0020's honest
production typing: surface confidence and `unknown` honestly rather than chase a
boundary that three instruments now agree is at its practical ceiling. The only
model-side experiment with a *credible* (not proven) mechanism is a **scoped
sibling-context-for-geography** bet — ceiling ~31 gold columns, almost certainly
far less realised, against a negative prior. Spend there only if the author wants
to close that question with eyes open.

One line for a stakeholder: *FineType is right ~8 times in 10 on hard columns;
the last 2 are mostly genuine ambiguity a bigger model can't resolve, so the win
now is telling the analyst honestly when we're unsure — not squeezing the model.*

---

## ac-00 — the headroom, partitioned by error mass (GATE)

Clean v19 re-run: **739/927 = 0.797** (reframe, primary lens; raw 735/927 = 0.793).
This matches the quoted baseline exactly. Of 192 raw misses (4 are instrument
load-failures, not model errors), the partition by what lever *could* address them:

| bucket | cols | % | what it is | winnable? |
|---|---|---|---|---|
| **STRUCTURAL** | 46 | 24% | separable by full-column stats the per-value model is blind to: alnum↔residual/cardinality (15), numeric shape/role e.g. integer→increment/binary/ordinal/year (24), categorical↔entity_name by cardinality (7) | **~0** — proven (ac-01) |
| **SEMANTIC** | 49 | 26% | needs meaning: geography categorical↔city/region/country/state (31), person/name (7), iata code (6), currency-by-header (5) | **~0 model-side** (ac-02); LLM/taxonomy/human |
| **IRREDUCIBLE / OTHER** | 97 | 51% | veto→unknown/empty precision-protection (37), gross value-shape collisions & rare codes e.g. isbn↔npi, unix↔npi, smiles, longitude→latitude (48), datetime-subtype granularity (8), load-fail (4) | mostly no; a few clean veto-rule candidates |

**The honest split to ~0.90:** of the ~10pp gap, the structural-and-winnable
share is **near zero** (the lever that targets it ships nothing), the
semantic-and-hard share (~26% of misses ≈ 2.5pp) needs meaning the model doesn't
have, and the majority (51%) is precision-protected demotions + rare collisions +
datetime granularity + label noise. **~0.80 on contested gold is at the practical
ceiling for this architecture** — corroborated independently by the representative
baseline (~0.68 on uniform-random columns, memory `representative-baseline-is-068`)
and the cardinality re-adjudication (memory `cardinality-boundary-error-is-real`).

Partition is reproducible; bucket boundaries are judgment calls at the margin
(iata, ordinal) but the three-way shape is robust.

---

## ac-01 — column-statistics layer: feasibility + rule consolidation (GATE)

**Verdict: the column-statistics layer ships NOTHING as an accuracy lever, and
its rule-consolidation promise is unrealised. This is settled by measurement, not
re-opened here.**

The 2026-06-16 probe's separability was real (cardinality AUC 0.985 categorical↔alnum,
0.972 integer↔increment). But two downstream ship attempts already consumed that
separability and failed:

1. **Neural-skip form — NO-GO** (`output/decisive-stat-sweep/finding.md`,
   memory `decisive-stat-skip-is-no-go`). All four decisive predicates miss the
   ≥0.98 bar badly (low-card→categorical 0.33, alnum→0.63, increment 0.13, binary
   0.00). **AUC is a ceiling for a discriminator *with a second opinion*, not a
   threshold a *blind assertion* can ship at** — the losing side of every boundary
   is a high-frequency impostor with the identical signature (URLs under alnum,
   codes/years under categorical, Year-ranges/ranks under increment). Binary and
   increment skips would re-assert the exact labels two shipped vetoes
   (`binary_vocab_veto`, `increment_substance_veto`) exist to strip.

2. **Recall-rule form — NO-GO** (`output/categorical-alnum-recall/finding_studies.md`,
   memory `full-column-stat-sharpen-is-redundant`). alnum: a stat override breaks
   18 correct URL predictions to recover 2 (P 0.952→0.68). categorical: the
   full-column upgrade is redundant — sample distinct ≤ full distinct always, so
   the shipped sample-based `text_vocab_override` already fires whenever the
   full-column version would; the study's apparent +2 was a join confound.

**Rule consolidation (the "fewer rules" angle) — does NOT net out.** The current
cardinality/shape rules:

| rule | location | scope today |
|---|---|---|
| `text_vocab_override` (R32) | `column/value_sharpen.rs:442` | sample distinct∈[2,12], distinct/n≤0.6 |
| `is_username_handle` guard | `column/mod.rs:271` | sample distinct-fraction |
| `increment_substance_veto` | `column/mod.rs:2951` | **already full-column** (contiguity over values) |
| `binary_vocab_veto` | `column/mod.rs:2980` | sample domain |
| `fusion_cardinality_gate` | `column/mod.rs:2244` | distinct>50 — **fusion path only; inert on v19** (no fusion) |
| `veto_shape_fallback` | `core/validation_veto.rs:121` | full-column in `validate`, sample in `profile` |

A single full-column-stats pass would *change behaviour*, not just consolidate:
full-column cardinality ≥ sample cardinality, so `text_vocab_override` would fire
*more* aggressively — exactly the categorical over-emission the corpus-honest gate
killed (spec 2026-06-12-text-vocab-override round 1). `increment_substance_veto` is
already full-column. `fusion_cardinality_gate` is dead code on v19. **Net: a
full-column layer is not a free consolidation; it is a behaviour change in the
known-bad direction.**

**Already cleaned:** `ColumnScanStats` (the "free-stats plumbing with no consumer"
the memories flagged for deletion) is **gone** — `grep` finds it nowhere in
`crates/`. No action.

*Probe reproduction note:* re-running `probe_column_stats.py` on current gold is
blocked — it reads raw source files (`read_parquet`/`read_csv_auto` on `file_path`)
that are not on disk locally. It is also moot: the two downstream NO-GO findings
already superseded the AUC the probe would re-measure.

---

## ac-02 — semantic-signal headroom: the stranded layer, measured (GATE)

**Verdict: the semantic mass is not reducible by any current model-side lever.
The sibling-context layer is stranded, not dead — but its realised effect is
noise-level. Finishing it is unvalidated 250-class integration engineering with a
negative-leaning prior. The bulk of the semantic mass is LLM/taxonomy/human
territory.**

**(a) Ablation — does sibling-context change v19 predictions?** YES, but barely
and laterally. `models/sibling-context` (the **6-class**, 397k-param, val-0.780
module from 20 Mar — confirmed never trained for the 250-class head) is loaded at
inference whenever the dir is present (`wire_sibling_context`, hardcoded path).
Diffing gold predictions with it present vs moved aside:

- **26 of 927 predictions change.** Net headline **735→739 (+4 cols, +0.4pp)**.
- Of the 26: **6 better, 2 worse, 18 lateral** (both-wrong or both-residual).

Two consequences. (1) It is *not* inert — the memory's "v19 runs with zero
cross-column context" is true of the *published artifact* but not of the local
dev binary. (2) The widely-quoted **0.797 baseline is inflated ~0.4pp by a
module that isn't in the shipped model** — shipped v19 (no sibling dir) scores
**0.793**. Worth correcting in the record.

**(b) Headroom if finished.** The realistic gold gain from finishing it (FTMB v3
table-grouped + frozen-sibling 250-class training, the spec
2026-03-24-sibling-context-multi-branch that closed **0/7**) is unknown, and the
priors are poor:
- The current 6-class module delivers +0.4pp of *lateral noise* on gold.
- Its sibling on the architecture program, the hierarchical head, was **trained
  and falsified: −3.3pp** (memory `hierarchical-head-falsified`) — splitting/
  enriching the head doesn't fix interference that lives in the shared trunk.
- The architecture challenge's +14–18% was on *external* benchmarks (Sato,
  Pythagoras), not our corpus.

The one credible mechanism: a column sitting among other geography columns is more
likely geography — which maps onto the **geography sub-mass (31 cols)**, the
largest semantic bucket. Coordinates (lat always has a lon sibling) are the
textbook case but already solved on gold (1 miss: longitude→latitude).

**(c) Verdict.** Declare the bulk of the semantic mass **irreducible by any
value/shape/sibling model** — route to LLM-in-the-loop / taxonomy / human tier.
The single bounded exception worth a *scoped* bet is **sibling-context for
geography** (ceiling ~31 gold cols, realisation almost certainly far lower),
against the −3.3pp/+0.4pp prior. Not recommended ahead of the product bet.

---

## ac-03 — rule-deletion scoreboard (the "fewer rules" target)

**Neither lever, in its current (proven-dead / stranded) state, unlocks any rule
deletion.** The semantic-advance goal's rule-reduction deliverable is therefore
**0 rules retired by the levers**. What *is* deletable is independent of them:

**Header-hint families** (`diagnostics/rhh_classification.tsv`, decision 0042
deprecation roadmap):

| class | families | deletable? |
|---|---|---|
| `no-hit` (zero eval impact) | 10 (`header_hint_measurement/location/fallback/geo_override/person_override/sci_measurement/location_keep`, `sense_geo_hint_override/rescue/header_hint`) | **YES, now** — gate on a no-regression gold run (card 0019 scenario 5) |
| `model-covered` | 2 (`substring_matcher_datetime`, `header_hint`) | yes — model already covers |
| `model-gap` | 9 families | **blocked on a stronger model** — neither lever delivers it |
| `keep_required` | 2 (`header_hint_cross_domain/same_category`) | no (spec constraint) |

**Deterministic value/shape rules** (username veto, `veto_shape_fallback`,
`text_vocab_override`, `binary_vocab_veto`, `increment_substance_veto`, the 0096
residual-precedence rules): **all stay.** They are the shipped, gold-validated
precision floor. The column-stats layer was supposed to subsume them; ac-01 shows
it would instead *broaden* them in the known-bad direction. `fusion_cardinality_gate`
is dead on v19 (fusion path only) — a deletion candidate, but tied to the
abandoned fusion work, not to either discovery lever.

**Scoreboard:** rules retired by a column-stats layer = **0**. By sibling-context =
**0** (the model-gap header families it would theoretically retire require the
finished 250-class integration, which is the unvalidated bet). Rules safely
retirable *today, lever-independent* = **~12 header-hint families** (10 no-hit + 2
model-covered), as a clean card-0019-scenario-5 follow-up.

---

## ac-04 — synthesis + recommendation (GATE)

**(1) Structural headroom & rules retired by the column-stats layer:** ~46 gold
cols of separable-but-impostor-shared boundaries; **converts to ~0 shippable
accuracy and 0 rules retired**. Proven across two ship forms (skip + recall rule),
two findings, before this discovery. The lever is closed.

**(2) Semantic mass — any model-side lever left?** No clean one. ~49 gold cols.
Sibling-context is stranded (6-class, +0.4pp lateral noise), the hierarchical head
is falsified (−3.3pp), and additive retraining is **0-for-6** — the most recent
(identity-fortification, 2026-06-18) ran *after* the enum reframe removed the
categorical attractor (card 0020's named prerequisite) and **still regressed** by
broadening full_name. That directly weakens card 0020's hypothesis (2): the clean
rebalance space was supplied and the additive retrain failed anyway. The semantic
mass is LLM/taxonomy/human territory; the only scoped model exception is
geography-via-sibling, unproven, negative prior.

**(3) The honest ceiling.** ~0.80 on contested gold is the practical ceiling for
this architecture — three independent instruments agree (gold partition here,
representative baseline 0.68, cardinality re-adjudication). Of the gap to ~0.90:
near-zero is structural-and-winnable, ~2–3pp is semantic-and-hard (no model lever),
and the majority is precision-protected veto demotions + rare value collisions +
datetime granularity + irreducible label ambiguity.

**Recommended next spec:** **accept the ceiling and shift to product — drive card
0020's honest production typing** (confidence surfacing, honest `unknown`,
representative-baseline tracking), NOT a new model layer. Hold the
sibling-context-for-geography bet as a documented, scoped option the author can
elect against its negative prior. Separately, file the lever-independent win: a
card-0019-scenario-5 deletion of the ~12 dead header-hint families on a
no-regression gold run.

No code or model ships from this spec.
</content>
