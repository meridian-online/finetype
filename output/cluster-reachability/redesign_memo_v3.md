# Reachability metric v3 — redesign memo

Per spec `2026-05-30-reachability-metric-v2` ac-04 Path C (Mismatch).
Six crossovers in the v23 six-cluster fixture (`v23_fixture_v2.md`);
v2 metric does not ship.

## Headline

The v23 fixture itself is the load-bearing problem, not just the
metric. v2's `absorption × (1 - risk)` collapses two distinct
properties of a cluster into one number; the v23 fixture's HIGH/LOW
labels mix three orthogonal mechanisms. No single score will
separate them.

**v3 splits reachability into two orthogonal scores: `safety_score`
and `leverage_score`.** Both must clear independent thresholds for a
cluster to be a `training_data_addition` candidate. v3 ships
`safety_score` to `corroborated_gaps.parquet` as an advisory column
(not a hard gate); `leverage_score` remains a per-retrain-spec
decision (driven by author judgement on cluster size plus
Sense-distribution context, the interim CLAUDE.md guidance).

## What v1 and v2 each got right and wrong

| dimension | v1 captures? | v2 captures? | v3 needs to capture? |
|---|---|---|---|
| cluster IS correct_label (safe absorption) | ✗ inverted | ✓ via absorption | ✓ via safety |
| cluster has tight specific shape | ✓ tightness | partial | not load-bearing |
| cluster is far from confusable wrong-labels | partial | partial | ✓ via safety (risk term) |
| training-signal-size sufficient | ✗ | ✗ | ✗ (out of scope, see below) |
| Sense's current behaviour on neighbourhood | ✗ | ✗ | ✗ (out of scope, see below) |

v2's `risk` term — "fraction of NN where YDF disagrees with
correct_label AND Sense also disagrees" — is the genuinely useful
output of the v2 work. It directly measures the v23 categorical-
bleed risk. v3 uses `safety = 1 - risk` as its first score.

v2's `absorption` term — "fraction of NN where YDF agrees with
correct_label" — turned out to be dominated by structural correct_
label density rather than cluster-specific reachability. v3 drops
it from the score.

## Proposed v3 algorithm

For each (fp_label, correct_label) cluster:

1. Build value-shape embeddings (re-use v1/v2 — char 3-gram hashed
   + length percentiles + char-class summary).
2. Build a 50,000-row stratified neighbour pool (re-use v2's
   stratification — even split with population cap, then
   proportional fill of residual to dominant labels).
3. For each cluster column, find k=100 nearest neighbours in the
   pool, excluding cluster columns by `(file_path, column_name)`.
4. Per-column risk:
   `risk_c = fraction of c's 100 NN with`
   `ydf_prediction != correct_label AND sense_prediction != correct_label`
5. Cluster aggregation:
   `risk = mean(risk_c)`
   `safety_score = clip(1 - risk, 0.0, 1.0)`

Only one number per cluster. No absorption term.

### Interpretation

- `safety_score ≥ 0.8` — training on this cluster is unlikely to
  pull a meaningful population of non-correct_label columns into
  correct_label. Safe to add to a `training_data_addition` retrain.
- `safety_score 0.5–0.8` — moderate risk. Training will shift some
  non-correct_label columns toward correct_label; the magnitude
  depends on cluster size and other clusters being trained at the
  same time. Spec author must include a Sense-distribution pre/post
  check on the correct_label and its neighbours.
- `safety_score < 0.5` — high risk. Sharpen rule or taxonomy
  intervention preferred over retrain. Training this cluster will
  drag a measurable share of non-correct_label columns into
  correct_label (the v23 categorical-bleed mechanism).

### v23 fixture (illustration, not validation)

| cluster | v2 reach | v3 safety | v23 outcome | interpretation |
|---|---:|---:|---|---|
| utc → integer_number | 0.867 | 0.95 | Met | safety high, leverage met → trained cleanly |
| boolean.binary → integer_number | 0.856 | 0.94 | Flat | safety high, leverage absent (cluster size 37k but Sense indifferent) |
| url → integer_number | 0.824 | 0.91 | Flat | safety high, leverage absent (cluster size 3.7k) |
| periodicity → categorical | 0.468 | 0.69 | Regressed | safety MODERATE — should have flagged caution |
| gender_code → categorical | 0.362 | 0.61 | Met | safety MODERATE — per-cluster Met but contributed to net Failed |
| alphanumeric_id → categorical | 0.173 | 0.43 | Met | safety LOW — per-cluster Met but the LOW score correctly flags the geography-bleed risk |

The v3 safety_score is monotonic with risk avoidance: high-safety
clusters are the ones that DON'T pull non-correct_label populations
in. The three integer_number-target clusters score 0.91-0.95 (safe);
the three categorical-target clusters score 0.43-0.69 (moderate to
low) because categorical's neighbourhood is full of YDF-disagree-
Sense-disagree columns that would shift toward categorical with
training.

This matches v23's actual mechanism: aggregate v23 failed because
the *categorical* retrains pulled geography in. v3 flags every
categorical-target cluster as moderate-to-low safety — which is
the actionable signal.

## What v3 deliberately does NOT do

- **No leverage score, no combined reachability number.** Leverage
  (cluster size × Sense-behaviour-on-neighbours) is a per-retrain
  decision that depends on the full slate of clusters being trained
  AND on cluster size. The author has both at spec-writing time;
  the metric doesn't need to combine them prematurely.
- **No threshold on a combined score.** A single threshold over a
  combined score would replicate the v1/v2 failure of conflating
  signals.
- **No replacement of the CLAUDE.md interim guidance.** The
  "Sense-distribution pre/post check" remains the load-bearing
  gate. v3's `safety_score` is an advisory input to that check —
  the spec author reads safety_score AND the cluster size AND
  decides.

## Validation honesty

We have ONE labeled training-bet outcome (v23). One data point can
illustrate a metric's design but cannot validate it. v3 ships as a
first-principles design with the v23 fixture as illustration; the
true validation comes from the next two or three retrain bets,
where author decisions guided by v3's safety_score get checked
against actual outcomes.

This is a meaningful change in the closure rule for reachability
work: rather than "fail the fixture → redesign", v3 ships as
advisory and gathers ground truth from real retrain bets. The fix
loop runs at retrain-spec cadence, not at fixture-test cadence.

## Implementation cost

The v2 script is already the v3 script — just drop the absorption
output and rename `reachability_score` to `safety_score`. ~30
minutes of code change plus a re-run on the full corpus. Add the
column to `corroborated_gaps.parquet` via a post-processing pass
(re-using the v1 / v2 infrastructure pattern).

## What ships from the v2 spec

The v2 metric and its fixture report. The v3 redesign memo (this
document). v2's `cluster_scores_v2.parquet` ships as a reference
artefact.

The v3 metric itself is **NOT** part of the v2 spec — implementing
it is the next spec's work (`2026-05-31-reachability-safety-score`
or similar). v2's ac-05 stays deferred per Path C.

## What the next spec must do

- Implement v3 (`safety_score` only, drop absorption).
- Run on full corpus and on the v23 fixture for illustration.
- Add `safety_score` column to `corroborated_gaps.parquet`.
- Update `report.md` to include safety_score in cluster headers.
- Update CLAUDE.md: extend the existing
  `training_data_addition` paragraph to require spec authors
  to consult `safety_score` as input to the Sense-distribution
  pre/post check, rather than as a replacement for it.

## One-line for a stakeholder

v2 picked up the safety signal v1 missed but lost the cluster-
ranking v1 got right because absorption tracks correct_label
density rather than cluster specificity, so v3 drops absorption,
keeps risk, and ships safety_score as an advisory column with the
honesty that one v23 fixture is illustration not validation.
