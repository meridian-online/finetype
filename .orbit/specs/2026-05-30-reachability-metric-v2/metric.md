# Reachability metric v2 — design

Per spec `2026-05-30-reachability-metric-v2` ac-01.

The redesign memo at `output/cluster-reachability/redesign_memo.md`
named the v2 algorithm, the candidate predictions, and the failure-
mode the metric is meant to catch. This document ratifies that
design and commits to the implementation edge cases.

## Algorithm — ratified verbatim from the redesign memo

For each cluster (a `(fp_label, correct_label)` pair from a
corroborated gap):

  Embedding (re-use v1):
    - Char 3-gram hashed histogram (1024 dim, L1-normalised)
    - Length distribution percentiles p10/25/50/75/90 (5 dim,
      divided by 64)
    - Character-class summary fractions (4 dim:
      alpha/numeric/punct/other)
    - Total 1033 dim per column, cosine-distance-friendly after
      L2 normalisation.

  Nearest-neighbour search:
    For each cluster column c, find its k=100 nearest neighbours
    in a 50,000-column neighbour pool (see "Neighbour pool"
    below). Cosine distance.

  Per-column scores:
    absorption_c = fraction of c's 100 NN with
                   `ydf_prediction == correct_label`
    risk_c       = fraction of c's 100 NN with
                   `ydf_prediction != correct_label` AND
                   `sense_prediction != correct_label`

  Cluster aggregation:
    absorption = mean over cluster columns of absorption_c
    risk       = mean over cluster columns of risk_c
    reachability_v2 = clip(absorption * (1 - risk), 0.0, 1.0)

  Interpretation:
    - High absorption + low risk → cluster lives where YDF already
      agrees with correct_label; training is absorbed (utc case).
    - Mixed absorption + high risk → cluster lives where YDF
      disagrees with correct_label AND Sense disagrees too;
      training will pull those disagreeing columns into the
      cluster's training direction (the categorical-bleed
      mechanism).
    - Low absorption + low risk → cluster's value-shape
      neighbours are scarce in the corpus; training has little
      leverage (the v23 url case).

No deviation from the redesign memo.

## Neighbour pool

50,000-column reservoir-sample of `columns.parquet`, seed 44,
**stratified by `ydf_prediction`** so rare correct_labels stay
represented. Build once, query per cluster.

Stratification rule: split the 50k budget evenly across the 198
distinct `ydf_prediction` values, capped at the population size of
each label. Labels with fewer than `50000 / 198 ≈ 252` columns
contribute all their columns; the remainder pool is filled from the
larger labels in proportion.

Why this size: with 50,000 columns, 100-NN search per cluster is
exact, fast (one matmul per cluster), and small enough to cache the
embeddings in RAM (50,000 × 1033 × float32 ≈ 200 MB). ANN over the
full 6.6M corpus is deferred — the redesign memo names ~30-minute
hnswlib build vs ~10-minute down-sampled approach; the 50k pool is
the simpler v2 floor that we can graduate from if v2 fails.

Stratification is the load-bearing design choice. An unstratified
50k sample would heavily over-represent the few dominant labels
(`representation.text.entity_name`, `representation.text.sentence`)
and under-represent the long tail. Specifically: under-representing
the cluster's correct_label population (an extreme case is
`representation.text.sentence`, where v1's unstratified baseline
errored out for 22,687 gaps) directly biases the absorption term
toward zero.

## Excluding the cluster from its own neighbour search

A cluster's own columns must not appear in its k-NN search; if they
did, the absorption term would lift artificially (cluster columns
all share `(sense=fp_label, ydf=correct_label)`, so they vote
themselves "correct_label" 100% of the time).

The exclusion key is `(file_path, column_name)` — the only
identifier `columns.parquet` carries that uniquely identifies a
column. The pool itself is sampled from all of `columns.parquet`,
INCLUDING cluster columns; per-cluster exclusion happens at NN-
search time via a boolean mask over the pool's
`(file_path, column_name)` set.

Implementation: load the cluster's `{(file_path, column_name)}` set
once per pair, mask out matching pool rows before the matmul. The
matmul itself runs over the unmasked-pool indices.

## Cluster-size floor

Same as v1: if the cluster has fewer than 5 columns, the score is
emitted as null with error string
`"cluster size < 5 columns (n=<n>), embedding unstable"`. Below
that, the value-shape embedding for the cluster is too noisy to
support a 100-NN aggregation.

## Neighbour-pool-per-correct_label floor

If the stratified pool contains fewer than 1,000 columns with
`ydf_prediction == correct_label`, the absorption term is too
noisy. Score is null with error
`"neighbour pool has <n> columns for correct_label <label> "
"(below 1000 floor)"`.

This is stricter than v1's "baseline < 200" floor because the
absorption term is a fraction over 100 NN per cluster column,
averaged across the cluster. With fewer than 1,000 correct_label
columns in the pool, the 100-NN sets across cluster columns
substantially overlap and absorption decays toward a coarse
0/1 indicator rather than a smooth fraction.

## What gets emitted to the parquet

One row per gap_id:

  gap_id, criterion, mechanism, fp_label, correct_label,
  affected_column_count, n_cluster_columns, n_neighbour_pool,
  absorption (double), risk (double),
  reachability_score (double, = absorption × (1 - risk)),
  error (varchar).

All double fields null when `error` is set. Otherwise all
non-null.

## Cost expectation

- Pool embedding: 50,000 columns × ~500 μs each ≈ 25 s, one-time.
- Per-pair scoring: cluster embedding (varies by cluster size) +
  matmul (cluster_size × 50,000) + k-NN partition. For the largest
  cluster (boolean.binary, 37k cols), the matmul is 37k × 50k =
  1.85e9 float ops ≈ 1.2 s on a single-thread numpy. With
  down-sampling of clusters above LARGE_CLUSTER_CAP=5,000 the cost
  is bounded.
- 878 unique (fp, correct) pairs × ~1 s per pair ≈ 15 minutes
  total for the full corpus. Same order of magnitude as v1.

## Pre-committed predictions

From the redesign memo (`output/cluster-reachability/redesign_memo
.md` worked predictions section):

| cluster | predicted reachability | expected band |
|---|---|---|
| utc → integer_number | absorption≈1, risk≈0 → ≈1.0 | HIGH |
| gender_code → categorical | absorption≈0.4, risk≈0.3 → ≈0.28 | HIGH |
| alphanumeric_id → categorical | absorption high, risk low → HIGH | HIGH |
| periodicity → categorical | risk HIGH → LOW | LOW |
| boolean.binary → integer_number | risk HIGH → LOW | LOW |
| url → integer_number | absorption≈0 → LOW | LOW |

If the three HIGH all score above all three LOW, the metric ships
(ac-03 Clean band). Any v1-style mismatch routes ac-04 to Path C
and the next iteration is the redesign memo's Sense-embedding
alternative.

## Summary table

| Choice | Default | Source |
|---|---|---|
| Embedding | char 3-gram (1024) + length percentiles + char-class | v1 (re-used) |
| Distance | cosine | v1 (re-used) |
| k | 100 | redesign memo |
| Pool size | 50,000 | redesign memo's "down-sampled neighbour pool" |
| Pool stratification | by ydf_prediction, even split with population cap | new (this doc) |
| Cluster-size floor | 5 | v1 (re-used) |
| Pool-per-label floor | 1,000 | new (this doc, justified above) |
| Cluster exclusion | (file_path, column_name) match | new (this doc) |
| Combined score | absorption × (1 - risk), clipped [0, 1] | redesign memo |

v2 ships as-described. ac-02 implements; ac-03 is the acceptance
test against the v23 fixture.
