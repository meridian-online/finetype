# Reachability metric — design

Per spec `2026-05-29-cluster-reachability-scoring` ac-01.

## The question

For each corroborated-gap cluster, the metric must answer: **if Sense
is retrained with these columns labelled under the cluster's
`correct_label`, will it learn a generalisation that lands inside the
cluster — or one that bleeds outward into the rest of the
`correct_label` population?**

v23 lost the geography war because three of its six clusters mapped
to `representation.discrete.categorical`, a label so broad that
teaching Sense "these specific low-cardinality string columns are
categorical" generalised to "low-cardinality string columns are
categorical", and ~48k v22 `geography.location.city` predictions
moved into the categorical bucket.

Reachability is the pre-flight check that catches this before training.

## The signal — column-level value-shape embedding

Each column gets a fixed-length real-valued vector built from its
sampled values (the existing `sample_values_truncated` column in
`columns.parquet` carries up to 8 pipe-separated values per column —
no extra sampling needed).

The embedding has three blocks, concatenated:

1. **Char 3-gram histogram** (dim 1024, hashed). All 3-grams across
   the column's sampled values feed a hashing trick at modulo 1024.
   L1-normalise after hashing so columns with more sampled-value mass
   don't dominate. Captures the alphabet, n-gram shape, and rough
   token signature of the values.

2. **Length distribution percentiles** (dim 5). p10, p25, p50, p75,
   p90 of value length, divided by 64 so the entries sit in roughly
   [0, 1]. Captures "all values short" vs "all values long" vs
   "mixed", which is the first thing a reader uses to tell a
   boolean.binary column from an integer column from a URL column.

3. **Character-class summary** (dim 4). Fraction of characters per
   value (averaged across sampled values) in {alpha, numeric, punct,
   other}. Captures the alpha/numeric balance that separates
   alphanumeric_id from integer_number from category labels.

Total embedding: 1033 dims. Built once per column, cached per cluster
run. Implementation is pure Python on the sampled values that already
sit in `columns.parquet`; no model inference.

### Alternatives considered and rejected

- **Sense's own char-branch embeddings.** They'd be the most
  semantically relevant features, but pulling them requires standing
  up an inference path against `models/sherlock-v22-boundary-relu-s44`
  for every column in the corpus. For v1 the question is "are these
  columns tight" not "what does Sense think" — pure value-shape
  features answer it without the model dependency. v2 could swap in
  Sense embeddings if v1's value-shape features misfire on the v23
  fixture.

- **Word-level tokens.** Many gittables column values are codes, IDs,
  or short labels with no useful word boundary. Char n-grams degrade
  more gracefully to short or tokenless values.

- **Higher n-gram order (4 or 5).** Sparser histograms, more
  hash-collision noise on the ~8 sampled values per column. 3-grams
  are the standard char-shape signature in the type-inference
  literature and survive small sample counts.

## Distance metric — cosine

Cosine distance over the concatenated embedding. Reasons:

- Scale-invariant — a column with 8 sampled values and a column with
  3 sampled values still compare on shape, not on raw histogram mass.
- Bounded in [0, 1] after the standard `1 − cos(θ)/2` rewrite for
  non-negative vectors, which the three blocks all are. The bounded
  range matters for the [0, 1] aggregation in (iii).

L2 / Euclidean rejected: sensitive to sampled-value count differences
across columns; pulls clusters apart when they shouldn't be.

## Aggregation — tightness and specificity

Two per-cluster scores, both in [0, 1], computed from the column
embeddings:

### tightness ∈ [0, 1]

```
tightness = 1 − mean_pairwise_cosine_distance(cluster_columns)
```

High when intra-cluster columns look like each other.

- For clusters with ≥ 50 columns, compute the mean over all
  C(n, 2) pairs (or a uniform sample of 2,000 pairs if n > 200, for
  runtime).
- For clusters with < 50 columns, the mean is noisy. Apply epsilon
  smoothing: `tightness_smoothed = tightness × n / (n + 10)`. This
  pulls small clusters toward 0 rather than letting a 3-column
  cluster with two identical columns score 1.0.

### specificity ∈ [0, 1]

```
specificity = mean(
    min_k_distance(c, baseline_for_correct_label)
    for c in cluster_columns
)
```

For each cluster column, take the mean cosine distance to its k=10
nearest neighbours in a sampled-1000 baseline of `correct_label`
columns drawn from OUTSIDE the cluster. Average across cluster
columns. High when cluster columns are far from the rest of the
correct_label population.

Why k-NN min-distance rather than mean to the whole baseline:

- The baseline is heterogeneous (the broad-label threat below).
  Mean-to-baseline would dilute against unrelated outliers.
- k-NN measures the nearest plausible confusion target. If the
  cluster's columns each have ten very-close neighbours under the
  correct_label, retraining will generalise outward to those
  neighbours. If the nearest neighbours are far, the cluster is
  isolated and safe.

Baseline construction: from `columns.parquet`, take 1000 columns
where `ydf_prediction == correct_label` AND `sense_prediction !=
fp_label` (i.e. same correct label, different Sense prediction —
outside the cluster). Reservoir-sample with seed 44 for
reproducibility. If fewer than 200 baseline candidates exist for the
correct_label outside the cluster, the cluster's specificity is
emitted as null with error string `"baseline < 200 columns for
correct_label X outside cluster"`.

Note on partitioning: `columns.parquet` is the multi-lens diagnostic's
MEASURE half (`file_content_sha256 MOD 2 == 1`) — the training-vs-eval
leakage firewall lives upstream of this artefact, so within the
eval set the cluster/baseline split is the only firewall the metric
needs. No column appears in both cluster and baseline by construction.

### Combined reachability

```
reachability_score = clip(tightness_smoothed × specificity, 0.0, 1.0)
```

Product rather than harmonic mean because both terms must be high for
safe retraining; a high-tightness cluster with low specificity (the
boolean.binary / integer case) needs to score near zero, and the
product collapses faster than the harmonic mean on either dimension
going low. The harmonic mean tolerates one term being weak; for this
metric weakness on either dimension is exactly the failure case.

Geometric mean (`sqrt(tightness × specificity)`) was also considered;
it preserves the same monotonicity as the product but moves the
operating range higher, compressing the headroom the v23 fixture
needs for clean separation between HIGH and LOW. The product keeps
scores spread, which makes ac-03's threshold easier to defend.

## Threats

### Sample-size bias

Clusters with few columns produce noisier embeddings and noisier
pairwise means. Two cluster sizes in the v23 fixture sit below 5,000:
`cdde5d05b73a` (periodicity → categorical, 5,764 cols — fine) and
`20803deffbad` (url → integer, 9,779 cols — fine). The smallest
v23 cluster is well above the 50-column floor where smoothing kicks
in, so for the v23 fixture sample size is not load-bearing.

For the broader corroborated-gaps set, clusters with < 50 columns
exist and need handling. v1 uses the epsilon smoothing on tightness
described above. Specificity is unaffected by cluster size in the
same way (it's a per-column mean), but baseline construction can
underweight a rare `correct_label` — handled by the null-with-error
path.

**v2 follow-up:** bootstrap confidence intervals on tightness for
small clusters; report `reachability_score ± width` rather than a
point estimate, so the threshold logic in ac-04 can require the
lower-bound to clear the threshold.

### Correct_label broadness

The v23 failure mode itself: `representation.discrete.categorical`
covers any low-cardinality string column, so the baseline of
"non-cluster categorical columns" is genuinely heterogeneous.
Specificity measured against this heterogeneous baseline can
overestimate isolation — a cluster's columns might be far from
*most* of the baseline but very close to a subpopulation that the
retrained model will then pull toward the cluster.

The k=10 min-distance design is the v1 mitigation: it measures
closeness to the nearest baseline subpopulation, not the average
distance to all of it. A cluster surrounded on one side by a tight
group of near-neighbours under the same correct_label will score
low specificity even if the rest of the baseline is far away.

This is the metric's deliberate bias toward catching the v23 failure
mode. The cost is that legitimately isolated clusters whose nearest
ten baseline neighbours happen to share a coincidental n-gram shape
may also score low. ac-03's verdict bands account for this: a single
crossover in the v23 fixture is the "Near-miss" path, not "Mismatch".

**v2 follow-up:** if Path B (Near-miss) ships and the crossover is
on the broad-label dimension, replace the k-NN baseline with a
clustered baseline — first k-means the baseline into 5 sub-clusters,
then compute specificity as min distance to any sub-cluster centroid.
That treats broad correct_labels as multimodal rather than averaging
through the modes.

## Summary

| Component | Choice | Default in spec? |
|---|---|---|
| Embedding | char 3-gram hashed (1024) + length p10/25/50/75/90 + char-class (alpha/num/punct/other) | yes |
| Distance | cosine | yes |
| Tightness | 1 − mean intra-cluster pairwise cosine, ε-smoothed for n < 50 | yes |
| Specificity | mean of per-column k=10 NN cosine distance to 1000-sample correct_label baseline outside the cluster | yes (with k=10 and baseline construction spelled out) |
| Combined | product (tightness × specificity), clipped [0, 1] | yes |
| Sample-size threat | ε-smoothing on tightness; null-with-error on tiny baselines; CI bounds deferred to v2 | partial (v2 named) |
| Broad-label threat | k-NN min-distance baseline (not mean-to-all); sub-cluster baseline deferred to v2 | partial (v2 named) |

v1 ships as-described. The v23 six-cluster fixture in ac-03 is the
acceptance test: the metric earns its threshold by cleanly separating
the three HIGH from the three LOW. If it can't, ac-04 routes to Path C
and the v2 follow-ups become the redesign starting point.
