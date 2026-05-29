# v23 fixture — safety_score illustration

Per spec `2026-05-31-reachability-safety-score` ac-03.
**Non-gating — illustration only.**

Source: `output/cluster-reachability/v23_fixture_safety_scores
.parquet` (produced by `scripts/compute_cluster_safety_score.py
--gap-ids 721b890ea74d,1b858e0d073b,3f2aa8465552,20803deffbad,
81b63a52e3ef,cdde5d05b73a`).

## What this report is

A reference table showing the six v23 fixture clusters with their
v3 `safety_score` alongside cluster size (the leverage proxy) and
the v23 outcome. There is no Clean / Near-miss / Mismatch verdict.
v3 ships as advisory regardless of fixture outcome — the
illustration is reference, not acceptance criterion.

## Per-cluster table

| gap_id | fp_label → correct_label | cluster size | risk | safety | v23 outcome | reading |
|---|---|---:|---:|---:|---|---|
| 1b858e0d073b | utc → integer_number | 23,158 | 0.049 | **0.95** | Met (−92.2%) | HIGH safety, HIGH leverage → trained cleanly |
| 81b63a52e3ef | boolean.binary → integer_number | 37,268 | 0.063 | **0.94** | Flat (−0.8%) | HIGH safety, low leverage (Sense indifferent on 0/1 cols) |
| 20803deffbad | url → integer_number | 3,686 | 0.087 | **0.91** | Flat (0%) | HIGH safety, low leverage (cluster too small to move) |
| cdde5d05b73a | periodicity → categorical | 13,488 | 0.305 | 0.70 | Regressed (+139%) | **MODERATE safety should have triggered caution** |
| 721b890ea74d | gender_code → categorical | 24,028 | 0.396 | 0.60 | Met (−95.7%) per-cluster | MODERATE safety — per-cluster Met but contributed to net Failed |
| 3f2aa8465552 | alphanumeric_id → categorical | 12,748 | 0.574 | **0.43** | Met (−95.8%) per-cluster | **LOW safety — flags geography-bleed risk despite per-cluster Met** |

## How to read this

The three integer_number-target clusters cluster at HIGH safety
(0.91–0.95). Their v23 outcomes differ — Met, Flat, Flat — driven
entirely by leverage (cluster size and Sense's pre-training
behaviour on the cluster's neighbourhood). Safety alone does not
predict outcome; safety + leverage does.

The three categorical-target clusters span MODERATE to LOW safety
(0.43–0.70). This is the signal v3 adds for retrain decisions:

- **alphanumeric_id (safety 0.43)** — LOW band. v23 trained this
  cluster to per-cluster Met (−95.8% FP drop), but the categorical-
  bleed risk it carried contributed to net Failed (geography
  column counts collapsed). The v3 reading: "do not retrain;
  prefer Sharpen rule or taxonomy work."

- **gender_code (safety 0.60)** — MODERATE band. Same
  per-cluster Met as alphanumeric_id, less collateral but still
  contributed. The v3 reading: "if retrain, include Sense-
  distribution pre/post check across categorical AND geography."

- **periodicity (safety 0.70)** — MODERATE band. Aggregate v23
  Regressed (+139%) on this cluster — the worst per-cluster
  outcome. v3 reads MODERATE and would have prompted the Sense-
  distribution check; the regression was foreseeable.

## What this fixture tells us

v3's safety_score successfully:
- Separates the integer_number-target clusters (all HIGH) from
  the categorical-target clusters (MODERATE to LOW).
- Ranks the categorical-target clusters in the order their
  collateral risk argues for (alphanumeric_id worst,
  periodicity / gender_code MODERATE).
- Does NOT separate Met from Flat within the integer_number-target
  group — because that distinction is driven by leverage, which v3
  deliberately does not score.

What this fixture does NOT tell us:
- Whether 0.80 is the right HIGH/MODERATE threshold.
- Whether the safety_score generalises beyond the six v23 clusters.

Both gaps close as agents apply v3 to the next two or three
retrain bets and check author judgement against actual outcomes.

## What ships next

This is the last fixture-only step. v3's full-corpus run feeds
ac-04 (augment `corroborated_gaps.parquet`) and ac-05 (`report.md`
header line). The next retrain spec reads `safety_score` from the
augmented diagnostic surface and applies the advisory bands.
