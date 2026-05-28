# v23 precision retrain — re-litigation memo

Per spec `2026-05-27-v23-precision-retrain` ac-05 Path C. v23 does
not ship as default. Sense retrains pause until a different
intervention class is identified.

## Headline

**v23 wins the precision battle and loses the geography war.** The
six false-positive clusters drop −70.8% in aggregate (FP-rate
component Met) — three of them by 92–96%. But v22's −10.4% gated
cell-2 lift vs v19 collapses to **+5.1% (a regression)** under v23.
Per-subtype, the three monotone-movers v22 earned all regress
sharply: country +70.3%, region +29.0%, city +14.1%.

The combined ac-04 band is **Failed** (either-component-fails rule).

## What broke

The load-bearing assumption — explicit in the spec's "negative
transfer concern still applies" caveat — was that additive
hard-negative training on `representation.discrete.categorical`
would tighten the categorical boundary without disrupting Sense's
geography predictions. The assumption was wrong.

Sense's prediction distribution shows the mechanism. v23 fires
discrete.categorical on **548,409 columns** (vs v22's 87,105 — a
+529.6% explosion). Roughly half a million columns moved INTO
categorical, including ~48k that were geography in v22:

| Sense label | v22 cols | v23 cols | Δ |
|---|---:|---:|---:|
| `representation.discrete.categorical` | 87,105 | **548,409** | +529.6% |
| `geography.location.city` | 86,192 | **38,084** | **−55.8%** |
| `representation.text.word` | 189,866 | 26,779 | −85.9% |
| `representation.identifier.alphanumeric_id` | 154,209 | 57,210 | −62.9% |
| `representation.text.entity_name` | 513,503 | 342,844 | −33.2% |

The model didn't learn "fire categorical only when the column is a
basketball START_POSITION-shaped F/C/G column." It learned "fire
categorical aggressively on low-cardinality string columns" — which
is most geography columns at small sample sizes. The boundary
training was too coarse for the feature space the model uses.

## Why "the precision battle" was a false dichotomy

The spec framed three risks (boundary blend / data composition /
m-19), implicitly assuming the corroborated-gaps report's
`training_data_addition` action class was sufficient direction.
What the report actually said: "these columns are mislabeled by
Sense" — true. What it did not say: "training Sense on these columns
under their YDF-correct labels won't collateral-damage other
predictions" — left unverified.

The diagnostic surfaced 6 clusters but didn't measure their
*reachability via training* — whether the relationship between
"these columns are wrong" and "the model can learn to be right
about them without giving up something else" is one-to-one. It
isn't for the three categorical-target clusters; they trade for
geography.

## Per-cluster post-mortem

| cluster_id | result | post-mortem |
|---|---|---|
| `721b890ea74d` (gender_code → categorical) | Met, −95.7% | Clean win. Basketball START_POSITION columns now read as categorical. But this *was* the boundary that over-fired into geography. |
| `1b858e0d073b` (utc → integer) | Met, −92.2% | Clean win. Integer Comments/Points columns now read as integer. No geography collateral. |
| `20803deffbad` (url → integer) | Flat (0%) | Hard negs were only 3,686 — smallest pool. Probably training-signal-starved. |
| `81b63a52e3ef` (boolean.binary → integer) | Flat (−0.8%) | 37k hard negs available; barely moved. The boundary between boolean.binary and integer is genuinely hard — small integers (0/1) ARE booleans-as-integers in the wild. Cluster may not be teachable from value evidence alone. |
| `cdde5d05b73a` (periodicity → categorical) | **Regressed +139%** | The categorical signal that beat the gender_code FPs flooded into periodicity territory. Worst per-cluster outcome. |
| `3f2aa8465552` (alphanumeric_id → categorical) | Met, −95.8% | Clean win. TEAM_ABBREVIATION columns now categorical. |

Three clear wins (clusters 1/2/6), two no-ops (3/4), one regression
(5). The aggregate −70.8% is real but the per-cluster picture is
uneven — adding 50k categorical hard negatives doesn't deliver a
uniform 70% drop across categorical-labeled clusters.

## What we learned (worth keeping)

1. **The corroborated-gaps report ranks "what to fix" but not
   "what's safe to fix this way."** Each `training_data_addition`
   action needs a per-cluster reachability check before retraining.
2. **The `representation.discrete.categorical` label is too broad
   to be a useful training target.** Its definition is "discrete
   string value from a small unordered set" — a description that
   covers most low-cardinality columns, including geography subtypes
   at sample-size 5–10. Training Sense on this label as the *correct*
   answer for 50k specific columns generalises to "many other
   low-cardinality columns" because the model has no finer feature
   for distinguishing them.
3. **COLUMN_LEVEL_TYPES was filtering categorical out of training
   for a real reason.** "Negative transfer" was concrete: when
   categorical is a training target, the model rebalances its
   low-cardinality decisions toward categorical. The fix was opt-in
   per `--include-column-level-types`; the gate caught the cost.
4. **The v22 geography lift is fragile.** v22's gains on
   city/region/country were absorbed by ~1200 training samples per
   type via boundary blending; v23's blending undid them with the
   same per-type budget on a different target. The geography lift
   should be re-baselined as "robust to additive training" before
   the next retrain. ac-06's observation soak (now n/a here) was
   supposed to catch drift; this exposes how easily v22 can be
   un-done.

## What we should NOT try next

- **A v24 patch retrain with a different hard-negative mix.** Same
  recipe, same risk. Without a mechanism for "train categorical
  rows without disturbing geography boundaries," any additive
  retrain has the same failure mode.
- **Lifting `--samples-per-type` to drown out the disruption.**
  More samples of the SAME problematic signal scales the disruption.
- **Tightening the FP-rate band to >70% to "force quality."** The
  band is already Met; the metric being chased isn't the
  load-bearing one.

## What might actually work (research bets, not committed scope)

1. **Sharpen-stage rule, not retrain.** Decisions 0038/0048 govern
   ("rules are a last resort", "value-based rules only"). The three
   monotone-mover clusters (gender_code FP, utc FP, alphanumeric_id
   FP) have specific value shapes. A targeted Sharpen rule on each —
   e.g. "if Sense fires gender_code AND values are single-letter AND
   not in {M, F, Male, Female, ...}, demote to N/A" — could capture
   the precision wins without touching the model. The v23-sharpen-
   code-discriminator precedent (closed without shipping) is the
   warning but also the template: small rules can ship if scoped
   right.
2. **Cluster-specific architectural feature, not training data.**
   The model has 5 branches (char, embed, stats, header, validation).
   The categorical/integer/geography distinction is plausibly a
   header+stats signal (small cardinality + column-name match) that
   no single branch captures. A 6th branch for "low-cardinality
   semantic classifier" would be a bigger investment but addresses
   the right level.
3. **Better diagnostic: cluster reachability scoring.** Extend the
   multi-lens diagnostic to estimate, per cluster, whether the
   columns share a *value-shape signature* tight enough to train on
   without generalising to neighbouring types. Cluster 4
   (boolean.binary on integer) failed this implicitly — the values
   (0/1) really are the same shape as nearby clusters.

## Status changes

- v23 candidate models stay at `models/sherlock-v23-precision-relu-s{42,43,44}`
  for diagnostic access. `models/default` continues to resolve to
  v22 (`sherlock-v22-boundary-relu-s44`).
- Card 0002 goal stays as v22.
- No new spec opened automatically — the next bet's shape (Sharpen
  rule vs architectural branch vs different diagnostic) is a design
  question for the author to drive.

## One-line for a stakeholder

We trained v23 to drop the corroborated false positives by 71%, and
it did — but it also reversed v22's country/region/city geography
gains, so it doesn't ship and we keep v22 as the default while we
work out a different way to attack the long-tail FPs.
