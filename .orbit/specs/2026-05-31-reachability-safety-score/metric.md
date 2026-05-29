# Safety-score metric — design

Per spec `2026-05-31-reachability-safety-score` ac-01.

The redesign memo at `output/cluster-reachability/redesign_memo_v3
.md` named the v3 metric. This document ratifies it and commits
the threshold bands the diagnostic surface uses.

## Algorithm

```
safety_score = clip(1 - risk, 0.0, 1.0)
```

where `risk` is computed identically to v2's risk term:

  For each cluster column c, find its k=100 nearest neighbours in
  a 50,000-row stratified neighbour pool (excluding the column
  itself by `(file_path, column_name)`):

    risk_c = fraction of c's 100 NN where
             `ydf_prediction != correct_label` AND
             `sense_prediction != correct_label`

  Cluster aggregation:
    risk = mean over cluster columns of risk_c
    safety_score = 1 - risk

The neighbour pool, the embedding, and the cluster exclusion are
all unchanged from
`.orbit/specs/2026-05-30-reachability-metric-v2/metric.md`. v3
drops only the `absorption` term from v2's combined formula.

## Why v3 drops absorption

v2 closed Path C because absorption × (1 - risk) collapsed two
distinct properties of a cluster into one number, and the
absorption term turned out to be dominated by structural
correct_label density rather than cluster-specific reachability —
integer_number-target clusters all scored ~0.91 absorption
regardless of cluster specifics.

The risk term, by contrast, is genuinely cluster-specific:
periodicity → categorical scored risk 0.31 (v2 cluster-leakage
signal) while utc → integer_number scored risk 0.05. Risk
captures the v23 categorical-bleed mechanism directly.

## Interpretation bands

These bands are advisory — they guide spec author judgement, not
a blocking gate.

| band | safety_score | interpretation |
|---|---|---|
| HIGH | ≥ 0.80 | Cluster's neighbourhood is dominated by correct_label-shaped columns. Training is unlikely to pull non-correct_label populations in. Safe candidate for `training_data_addition` if cluster size supports leverage. |
| MODERATE | 0.50–0.80 | Training will shift some non-correct_label columns. Magnitude depends on cluster size and other clusters being trained simultaneously. Author must include the Sense-distribution pre/post check on correct_label and its neighbours (CLAUDE.md interim guidance). |
| LOW | < 0.50 | High proportion of neighbours are mis-predicted by both YDF and Sense. Training will drag a measurable share into correct_label — the v23 categorical-bleed mechanism. Prefer Sharpen rule or taxonomy intervention. |

## Why advisory rather than gating

We have ONE labelled training-bet outcome (v23). A blocking gate
derived from one data point either:
- Over-fits to v23's specific clusters (the v2 failure mode).
- Or relaxes thresholds enough to be no signal at all.

v3 ships safety_score as advisory. The next two or three retrain
bets close the validation gap. After three labelled outcomes, a
follow-up spec can codify the thresholds as a gate IF the
illustration holds.

## v23 fixture safety scores (illustration)

Derived from `output/cluster-reachability/cluster_scores_v2
.parquet`'s `risk` column:

| cluster | risk | safety | v23 outcome | cluster size | reading |
|---|---:|---:|---|---:|---|
| utc → integer_number | 0.050 | 0.95 | Met (−92.2%) | 23,158 | HIGH safety, large cluster → trained cleanly |
| boolean.binary → integer_number | 0.063 | 0.94 | Flat (−0.8%) | 37,268 | HIGH safety, leverage absent (Sense indifferent on 0/1) |
| url → integer_number | 0.086 | 0.91 | Flat (0%) | 3,686 | HIGH safety, leverage absent (cluster too small) |
| periodicity → categorical | 0.310 | 0.69 | Regressed (+139%) | 13,488 | MODERATE safety should have triggered caution |
| gender_code → categorical | 0.394 | 0.61 | Met (−95.7%) per-cluster | 24,028 | MODERATE safety — per-cluster Met but contributed to net Failed |
| alphanumeric_id → categorical | 0.573 | 0.43 | Met (−95.8%) per-cluster | 12,748 | LOW safety — correctly flags geography-bleed risk despite per-cluster Met |

Reading the matrix: HIGH safety clusters split on leverage
(cluster size + Sense behaviour). MODERATE and LOW safety clusters
are where v3 adds the most signal — the categorical-target
clusters that v23 trained per-cluster Met but that aggregated to
net Failed.

The author judgement: for the next retrain bet, prefer clusters
where safety ≥ 0.80 AND cluster size ≥ ~5,000. Below 0.50, route
to Sharpen rule design or taxonomy work, not retrain.

## What ships

- One numeric column on `corroborated_gaps.parquet`:
  `safety_score` (double, NULL when cluster size < 5 or pool floor
  unmet).
- The `safety: 0.NN` segment in each cluster's `report.md` header.
- The CLAUDE.md guidance extension keeping the Sense-distribution
  pre/post check and naming safety_score as an input to it.

## What does NOT ship

- A leverage_score column. Leverage is a per-spec author judgement
  reading cluster size + Sense-distribution behaviour — moving it
  into the substrate prematurely repeats the v1/v2 mistake of
  conflating signals.
- A blocking gate. v3's thresholds are advisory bands.
- A fixture pass/fail test. The v23 illustration is reference, not
  acceptance criterion. ac-03 of this spec writes the illustration
  report and closes without a verdict tag.
