# Reachability metric — redesign memo

Per spec `2026-05-29-cluster-reachability-scoring` ac-04 Path C
(Mismatch). Three crossovers in the v23 six-cluster fixture
(`v23_fixture.md`); v1 metric does not ship.

## What v1 got right

- Identified `gender_code → categorical` (rank 1, 0.100) and
  `alphanumeric_id → categorical` (rank 2, 0.050) as the two
  highest-reachability clusters. Both were per-cluster Met in v23
  with clean −95% FP drops.
- Identified `periodicity → categorical` (rank 5, 0.005) — the
  Regressed +139% cluster — as LOW, capturing the same "cluster too
  close to broad correct_label" signal that drove the categorical-
  bleed failure mode.
- Identified `boolean.binary → integer_number` (rank 3, 0.018) as
  LOW. The 0/1 boundary between binary and small-integer is tangled
  and v23 was Flat; v1 correctly down-scored.

## What v1 got wrong — and why

`datetime.offset.utc → integer_number` scored 0.002 (rank 6) but
the v23 outcome was Met (−92.2% FP drop). The cluster's actual
values are predominantly `0|0|0|0|0|0|0|0` — Sense mislabelled
all-zero integer columns as `datetime.offset.utc`. The cluster IS
the correct_label population. Specificity collapses to ~0 because
the cluster has no value-shape distance from integer_number.

v1's specificity term reads "distance from cluster to correct_label
neighbours" and interprets short distance as risk of training
leakage. For the utc case, short distance means "training absorbed
into the existing correct_label region" — the opposite of risk.

The metric measures distance to correct_label without separating
the two regimes:

1. **Cluster IS correct_label** (utc). Training adds the cluster's
   pattern to an already-large correct_label region. Boundary
   effects are negligible because the model already fires
   correct_label here. SAFE.
2. **Cluster is between correct_label and a confusable wrong-label**
   (periodicity, boolean.binary). Training shifts the boundary
   into a region populated by another type. RISKY.

Both regimes register as low specificity. v1 can't tell them apart.

The url case is a tertiary issue: v23 was Flat because the cluster
had only 3,686 training rows (the smallest hard-neg pool). v1 scored
it LOW on specificity, by coincidence matching the LOW outcome. A
better metric would carry training-signal-size as an explicit input.

## Proposed v2 — neighbour-label composition

The load-bearing question for safety is: **of the columns whose
value-shape looks like the cluster's, what does YDF say their true
label is?**

- If YDF says correct_label for most neighbours → training is
  absorbed (the utc case). SAFE.
- If YDF says correct_label for SOME neighbours and other labels
  for many neighbours → training risks pulling the other labels in
  (the periodicity → categorical case). RISKY.
- If YDF says correct_label for very few neighbours → cluster's
  shape is far from correct_label's actual population; training a
  thin signal there has little leverage. WEAK.

### Concrete v2 algorithm

For each (fp_label, correct_label) cluster:

1. Build value-shape embeddings (same as v1 — char 3-gram + length
   percentiles + char-class summary).
2. For each cluster column, find the 100 nearest neighbours in
   `columns.parquet` excluding the cluster itself (no fp_label
   restriction).
3. Bucket neighbours by their `ydf_prediction`. Compute the share
   that equals `correct_label`.
4. Aggregate the share across cluster columns:

```
absorption  = mean over cluster columns of
              (fraction of 100 NN with ydf_prediction == correct_label)
risk        = mean over cluster columns of
              (fraction of 100 NN with ydf_prediction != correct_label
               AND sense_prediction != correct_label)
              ⌐ the model currently misclassifies these AND YDF disagrees;
                shifting boundary onto them would mis-label them too
reachability_v2 = absorption × (1 − risk)
```

Both terms are in [0, 1]; their product is too.

Worked predictions:

- utc → integer_number: neighbours are mostly all-zero integer
  columns; most ydf=integer_number → absorption ≈ 1, risk ≈ 0 →
  reachability ≈ 1. HIGH ✓
- gender_code → categorical: neighbours are single-letter columns;
  ydf mix is categorical + text + geography codes → absorption
  moderate (~0.4), risk moderate (~0.3) → reachability ≈ 0.28.
  Lower than v1 placed it, which is closer to the truth — v23 did
  net-damage on this cluster's collateral.
- periodicity → categorical: neighbours are short-string columns,
  ydf is mixed across categorical/ordinal/text.word → absorption
  moderate, risk HIGH (Sense already fires non-categorical labels
  here) → reachability low. LOW ✓
- boolean.binary → integer_number: neighbours are 0/1 columns,
  many of which ydf labels as boolean (correctly!) → risk high →
  reachability low. LOW ✓
- url → integer_number: neighbours are URL-shaped columns, almost
  none ydf-labelled integer → absorption low → reachability low.
  LOW ✓
- alphanumeric_id → categorical: neighbours are TEAM_ABBREVIATION-
  style codes; ydf is mostly categorical → absorption high, risk
  low → reachability HIGH ✓

This formulation answers the load-bearing question directly: "what
would the trained model learn about the cluster's neighbourhood?"

### Cost vs v1

- v1: O(cluster_size × baseline_size) per cluster. Total ≈ 10
  minutes for full corpus at ~1000 baseline samples.
- v2: O(cluster_size × N_total) per cluster for k-NN over 6.6M
  rows. Either approximate with a global ANN index (build once,
  query per cluster) or down-sample to ~50k random columns and
  reservoir-sample neighbours from that. With ANN index: ~30 min
  total for full corpus. With down-sampled neighbour pool: ~10
  min, slight accuracy hit.

## Alternative v2 — Sense-embedding distance

A second candidate worth naming, rejected for v2 only because of
infrastructure cost. Replace the hand-built value-shape embedding
with the multi-branch Sense model's char-branch + embed-branch
intermediate activations. This is the model's own view of the
cluster's similarity to other columns — strictly more relevant than
char n-grams since it captures what the model has learned to
distinguish.

Cost: requires standing up an inference path against `models/
sherlock-v22-boundary-relu-s44` for every column in the corpus.
`bench_infer.py` shows ~50ms per column for the full pipeline; at
6.6M columns that's ~90 hours. Could be sharded but the
infrastructure delta is large.

Worth revisiting if v2's neighbour-label composition fails its own
fixture test. Reads: "v1 was wrong about utc because it didn't
match the model's view; v2's value-shape distances still don't
match the model's view; only the model's own embeddings do."

## Alternative v2 — training-signal-size dimension

Add a `n_training_rows` factor that down-weights small clusters.
v23's url result was Flat because the cluster had only 3,686 rows;
no amount of value-shape distinction would have made retraining
work. A reachability metric that ranks small clusters HIGH (when
their shape is genuinely separable) is technically right but
operationally misleading: nobody should retrain a 3k-row cluster
expecting movement.

Proposed: gate reachability to 0 when cluster size is below a
training-volume threshold (~5,000 rows for a hard-negative-style
retrain). This is a configuration concern, not a metric concern;
add it as a CLI flag, not part of the score itself.

## What ships from this spec

Nothing. v1 stays in `scripts/compute_cluster_reachability.py` for
historical reference and as the starting point for v2 (the
embedding pipeline carries over directly). The corroborated_gaps
artefact does NOT get a `reachability_score` column.

The diagnostic substrate (corroborated_gaps + report.md) and v22 as
default model remain unchanged.

## What the next retrain spec must do

Until a v2 metric ships:

- No `training_data_addition` retrain bet may rely on per-cluster
  FP-rate Met as the safety signal alone. v23 had per-cluster FP
  Met on 3 of 6 clusters and lost net.
- Any cluster-driven retrain must include a Sense-distribution
  pre/post check on the correct_label and its neighbours (the same
  mechanism that surfaced the v23 categorical bleed in the
  post-mortem). The substrate is in place — it just wasn't a gate.

Suggested next spec: `2026-05-30-reachability-metric-v2` (or
similar), scoping the neighbour-label composition algorithm above
plus an updated v23 fixture pass. The redesign-memo's worked
predictions are the pre-committed hypotheses.

## One-line for a stakeholder

The first-pass reachability metric got two of six v23 clusters
wrong because it measured "how different is this cluster from its
correct label" when the right question is "what does YDF think the
columns next to this cluster actually are" — so we don't ship the
score, and the next iteration starts with a neighbour-label
composition design.
