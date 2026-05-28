# v23 fixture — reachability scores vs ground truth

Per spec `2026-05-29-cluster-reachability-scoring` ac-03.

Source: `output/cluster-reachability/v23_fixture_scores.parquet`
(produced by `scripts/compute_cluster_reachability.py --gap-ids
721b890ea74d,1b858e0d073b,3f2aa8465552,20803deffbad,81b63a52e3ef,cdde5d05b73a`).

## Headline

**Mismatch.** Three crossovers in the v23 six-cluster fixture. The
metric ranks `datetime.offset.utc → integer_number` as the LOWEST
reachability of the six (rank 6 / 6), but the v23 outcome for that
cluster was a clean Met (−92.2% FP drop) — it belongs in HIGH. Every
LOW-expected cluster scores above it.

The combined ac-03 verdict is **Mismatch (2+ crossovers)**. ac-04
routes to Path C — redesign memo, no threshold shipped.

## Per-cluster table

| Rank | gap_id | fp_label → correct_label | n_cluster | tightness | specificity | reachability | v23 outcome | expected | crossover? |
|---:|---|---|---:|---:|---:|---:|---|---|---|
| 1 | 721b890ea74d | gender_code → categorical | 24,028 | 0.950 | 0.105 | **0.100** | Met (−95.7%) | HIGH | ✓ |
| 2 | 3f2aa8465552 | alphanumeric_id → categorical | 12,748 | 0.867 | 0.058 | 0.050 | Met (−95.8%) | HIGH | ✓ |
| 3 | 81b63a52e3ef | boolean.binary → integer_number | 37,268 | 0.858 | 0.021 | 0.018 | Flat (−0.8%) | LOW | **above utc** |
| 4 | 20803deffbad | url → integer_number | 3,686 | 0.853 | 0.011 | 0.010 | Flat (0%) | LOW | **above utc** |
| 5 | cdde5d05b73a | periodicity → categorical | 13,488 | 0.982 | 0.005 | 0.005 | Regressed (+139%) | LOW | **above utc** |
| 6 | 1b858e0d073b | utc → integer_number | 23,158 | 0.990 | 0.002 | **0.002** | Met (−92.2%) | **HIGH** | crossed by all three LOW |

Total crossovers: **3** (utc is below boolean.binary, url, and
periodicity).

Threshold band: `≥ 2 crossovers → Mismatch`. The metric is not
shipping; the v22 corpus pass keeps its existing `corroborated_gaps.
parquet` schema unchanged.

## Why the metric mis-scores utc

The utc cluster's sampled values are dominated by columns whose
actual content is `0|0|0|0|0|0|0|0` — Sense mislabelled these as
`datetime.offset.utc` but they are integer columns with all-zero
content. Cluster tightness is 0.99 because every column looks
identical. Specificity is 0.002 because the cluster IS already
inside the `representation.numeric.integer_number` value-shape
population.

The metric reads this as "cluster indistinguishable from correct_label
neighbours → training will leak outward → unsafe." The v23 result
reads the same data as "cluster IS correct_label → training is
absorbed harmlessly → safe."

Both readings are internally consistent. The metric is measuring
the wrong direction of risk for this cluster type.

## Why gender_code and alphanumeric_id rank correctly

Both clusters have moderate tightness (0.95, 0.87) — they share a
shape but not perfectly — and specificity higher than the rest
(0.105, 0.058). The cluster columns (basketball F/C/G/G letters,
TEAM_ABBREVIATION codes) genuinely differ from the broad
`representation.discrete.categorical` baseline. Training Sense on
them as categorical adds a new tight pattern to the categorical
decision region; the model learns the pattern, and v23's FP-rate
drops 95% on these clusters.

The metric correctly identifies these as the two cleanest training
targets in the fixture.

## Why periodicity, boolean.binary, url all rank below utc

Each of these is "between" cluster and correct_label in a way the
specificity term captures only partially:

- **periodicity → categorical** (Regressed +139%). Cluster values
  look like categorical labels (small-string low-cardinality), so
  specificity is low (0.005). v23's actual failure mode: training
  the model to treat these as categorical pulled the boundary
  outward into nearby categorical look-alikes (the same mechanism
  that hit geography). The metric scores it appropriately LOW.

- **boolean.binary → integer_number** (Flat). Cluster values are
  0/1 — value-shape identical to small integers. Specificity 0.021.
  v23 failed because 0/1 boundary between binary and integer is
  genuinely tangled; teaching the model "0/1 columns are integer"
  reinforces what the model already does for half of them and
  conflicts with what it does for the other half. Net Flat. The
  metric scores it appropriately LOW.

- **url → integer_number** (Flat). Cluster size is only 3,686 (the
  smallest in the fixture). Tightness 0.85, specificity 0.011. The
  cluster's URL strings look nothing like integers, but the
  pre-existing training-signal scarcity (the relitigation memo
  named this) meant v23 had no leverage to learn the boundary. The
  metric doesn't capture cluster-size scarcity as a separate
  dimension; it scores LOW on specificity (URLs share some n-grams
  with integer-formatted phone numbers and timestamps), which
  matches the v23 outcome by coincidence rather than mechanism.

## Conclusion

The metric correctly separates the two clearest HIGH cases
(gender_code, alphanumeric_id) from the three clearest LOW cases.
It fails on utc because the cluster's value-shape is
indistinguishable from its correct_label population — a case where
"cluster IS correct_label" should score HIGH but the specificity
term scores it LOW.

The redesign memo (ac-04 Path C) proposes adding a "neighbour-label
risk" dimension that measures distance from cluster to NON-correct
labels that the trained model would most likely pull toward. That
addresses both the utc case (no at-risk neighbours nearby → HIGH
reachability) and gives a sharper signal on the categorical-target
clusters that drove the v23 failure.

`reachability_score` does not ship to the corroborated-gaps
artefact. ac-05 closes as deferred. ac-06 cites the redesign memo
rather than a threshold.
