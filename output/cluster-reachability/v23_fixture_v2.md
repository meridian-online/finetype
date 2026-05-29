# v23 fixture — v2 reachability scores vs ground truth

Per spec `2026-05-30-reachability-metric-v2` ac-03.

Source: `output/cluster-reachability/v23_fixture_scores_v2.parquet`
(produced by `scripts/compute_cluster_reachability_v2.py --gap-ids ...`).

## Headline

**Mismatch.** Six crossovers in the v23 six-cluster fixture. Both
`gender_code → categorical` and `alphanumeric_id → categorical`
(HIGH-expected) scored below all three LOW-expected clusters.

The combined ac-03 verdict is **Mismatch (≥ 2 crossovers)**. ac-04
routes to Path C — v3 redesign memo, no threshold shipped, v2 score
does NOT fold into `corroborated_gaps.parquet`.

## Per-cluster ranking

| Rank | gap_id | fp_label → correct_label | n_cluster | absorption | risk | reachability | v23 outcome | expected | result |
|---:|---|---|---:|---:|---:|---:|---|---|---|
| 1 | 1b858e0d073b | utc → integer_number | 23,158 | 0.913 | 0.050 | **0.867** | Met (−92.2%) | HIGH | ✓ |
| 2 | 81b63a52e3ef | boolean.binary → integer_number | 37,268 | 0.914 | 0.063 | 0.856 | Flat (−0.8%) | LOW | above 2 HIGHs |
| 3 | 20803deffbad | url → integer_number | 3,686 | 0.902 | 0.086 | 0.824 | Flat (0%) | LOW | above 2 HIGHs |
| 4 | cdde5d05b73a | periodicity → categorical | 13,488 | 0.679 | 0.310 | 0.468 | Regressed (+139%) | LOW | above 2 HIGHs |
| 5 | 721b890ea74d | gender_code → categorical | 24,028 | 0.597 | 0.394 | 0.362 | Met (−95.7%) | **HIGH** | below all 3 LOWs |
| 6 | 3f2aa8465552 | alphanumeric_id → categorical | 12,748 | 0.404 | 0.573 | **0.173** | Met (−95.8%) | **HIGH** | below all 3 LOWs |

Total HIGH-vs-LOW crossovers: **6** (each of gender_code and
alphanumeric_id sits below all three LOW-expected clusters).

## Why v2 mis-ranks: absorption tracks correct_label density, not
cluster-specific reachability

The absorption term `mean fraction of 100 NN with ydf_prediction ==
correct_label` is dominated by how dense the cluster's correct_label
is in the value-shape embedding space. For correct_labels that form
a tight, well-defined region of the embedding (integer_number,
decimal_number), absorption is high regardless of which cluster sits
inside that region. For correct_labels that are heterogeneous
(categorical, which the v22 corpus splits across short codes, longer
labels, ordinal-like values), absorption is structurally lower.

Concretely:

- **integer_number cluster columns** (utc, boolean.binary, url):
  All three clusters' actual values are integer-shaped — utc's
  columns are mostly `0|0|0|0|...`, url's columns are mostly small
  integers (Sense mislabelled both as their FP labels), boolean.
  binary's columns are 0/1 codes. The 100 NN for every cluster
  column in this group land in the integer_number-dense region.
  absorption ≈ 0.91 for all three.
- **categorical cluster columns** (gender_code, periodicity,
  alphanumeric_id): Short string labels. The 100 NN spread across
  categorical, ordinal, text.word — categorical's value-shape
  region is heterogeneous. absorption sits between 0.40 and 0.68.

The metric thus ranks "is your correct_label structurally easy to
absorb new training into" rather than "is THIS CLUSTER specifically
safe to train on." That structural property is real and useful, but
it's not the question the v23 fixture's HIGH/LOW labels test.

## The fixture's LOW labels also encode multiple mechanisms

A second observation from the run: v2's HIGH-scoring "LOW" clusters
(boolean.binary, url, periodicity) failed v23 for THREE different
reasons:

- **url (LOW, v23 Flat)**: training-signal-size. Only 3,686 hard-
  negatives — too few to move the boundary. Per the relitigation
  memo. v2 reads this cluster's absorption as high (which is
  correct — training would be absorbed) but the v23 outcome was
  driven by leverage, not safety. The fixture treats "didn't move"
  as LOW; v2 treats absorption as the safety signal. Different
  questions.
- **boolean.binary (LOW, v23 Flat)**: boundary ambiguity. 0/1
  columns are integer-shaped AND boolean-shaped; the v23 outcome
  was Flat because Sense already disagreed with itself on these.
  v2 reads absorption high because most 0/1 columns are integer-
  labelled by YDF. v23 says LOW because training didn't shift
  Sense's behaviour.
- **periodicity (LOW, v23 Regressed +139%)**: the genuine leakage
  case the v23 fixture's LOW label was supposed to flag. v2 scores
  this 0.468 (in the middle) — neither HIGH nor LOW. The metric
  partially catches it via the risk term (0.31) but not strongly
  enough.

The fixture's three LOW labels conflate (1) training-signal-size,
(2) boundary ambiguity, (3) leakage risk. A score that only measures
(3) — leakage risk — won't separate these three.

## Why v1's specificity term wasn't right either

v1 measured "distance from cluster to correct_label baseline" and
ranked utc LAST because utc cluster IS its correct_label
population. v2 reverses that — high absorption now correctly puts
utc at rank 1. But v2 lost the categorical-target ranking that v1
got right (gender_code rank 1 → rank 5).

Neither metric splits the underlying signals cleanly:

| dimension | v1 captures? | v2 captures? |
|---|---|---|
| cluster IS correct_label (safe absorption) | ✗ (calls this risky) | ✓ |
| cluster has a tight specific shape | ✓ (tightness term) | weakly (via absorption) |
| cluster is far from confusable wrong-labels | partial (specificity, against correct_label baseline only) | partial (risk term) |
| training-signal-size sufficient | ✗ | ✗ |

## What the next iteration must do

The v3 redesign memo (ac-04 Path C output) proposes splitting
reachability into two orthogonal scores:

- **safety_score** — would training on this cluster as
  correct_label cause Sense to mis-fire correct_label on other
  populations? Driven by `risk` (NN where ydf != correct_label AND
  sense != correct_label). High when training is contained.
- **leverage_score** — would training on this cluster move the FP
  rate meaningfully? Driven by cluster_size and by Sense's
  pre-training prediction distribution on the cluster's neighbours.
  High when training has something to learn.

Both above threshold → safe and useful to train.
safety alone → safe but won't move anything.
leverage alone → would move things but at unacceptable collateral.

This formulation matches the v23 fixture's three failure modes
directly: url and boolean.binary are safety-positive but
leverage-negative (the v23 Flat cases). periodicity is safety-
negative (the v23 Regressed case). gender_code and alphanumeric_id
are both above threshold on both (the v23 per-cluster Met cases —
even if the aggregate v23 still lost net).

The v3 redesign memo details the algorithm; ac-04 of the v2 spec
routes there.

## Conclusion

v2 correctly identifies the utc case that v1 missed (cluster IS
correct_label → safe to train). It loses the categorical-target
ranking by collapsing absorption into a structural correct_label-
density signal rather than a cluster-specific one. The fixture's
HIGH/LOW dichotomy itself encodes multiple mechanisms that one
score cannot separate; v3 splits reachability into safety and
leverage.

`reachability_score_v2` does not ship to the corroborated-gaps
artefact. ac-05 closes as deferred. ac-06 extends the CLAUDE.md
interim guidance with a v3 pointer rather than replacing it.
