# Next session — start here

You're picking up the cluster reachability scoring spec
(`.orbit/specs/2026-05-29-cluster-reachability-scoring/spec.yaml`,
six ACs, attached to card 0002).

## Context in one paragraph

v23-precision-retrain closed Failed because the categorical hard
negatives over-corrected: FP-rate Met (−70.8%) but v22's geography
collapsed (country +70.3%, region +29.0%, city +14.1% vs v22).
Post-mortem at `output/v23-precision-retrain/relitigation_memo.md`.
The root cause: the multi-lens diagnostic ranks `affected_column_count`
but doesn't measure whether training on a cluster will generalise
correctly. This spec adds that measurement so every future retrain
bet is pre-screened. v22 remains the default.

## What to do first

**ac-01 (doc).** Draft the reachability-metric design at
`.orbit/specs/2026-05-29-cluster-reachability-scoring/metric.md`.
The spec spells out default candidates — char n-gram embedding,
cosine distance, `tightness × specificity` aggregation — and names
two known threats (sample-size bias, correct_label broadness). The
doc commits to one shape, or explains the alternative chosen.

## Validation fixture (pre-committed)

Six v23 clusters with ground-truth labels — the metric must
distinguish HIGH from LOW:

| gap_id (prefix) | v23 outcome | expected |
|---|---|---|
| 721b890ea74d (gender_code → categorical) | Met −95.7% | HIGH |
| 1b858e0d073b (utc → integer_number) | Met −92.2% | HIGH |
| 3f2aa8465552 (alphanumeric_id → categorical) | Met −95.8% | HIGH |
| 20803deffbad (url → integer_number) | Flat 0% | LOW |
| 81b63a52e3ef (boolean.binary → integer) | Flat −0.8% | LOW |
| cdde5d05b73a (periodicity → categorical) | Regressed +139% | LOW |

ac-03 is a gate: Clean separation = ship threshold; ≤1 crossover =
ship with caveat; 2+ crossovers = redesign metric (no shipping).

## Load-bearing memories

- `v23-precision-retrain-failed-band` — what broke and why
- `v22-true-band` — v22's −10.4% Partial-band reading on the gated
  baseline
- `taxonomy-country-code-enum-contamination` (corrected) — the
  enum is canonical 249 ISO codes; previous claim of contamination
  was wrong
- Spec `2026-05-20-gittables-multi-lens-diagnostic` — the
  diagnostic that produces `corroborated_gaps.parquet`

## Inputs already in place

- `eval/gittables/corpus_pass/corroborated_gaps.parquet` — cluster
  definitions
- `eval/gittables/corpus_pass/columns.parquet` — per-column sample
  values + Sense + YDF predictions (v22 corpus pass)
- `output/corpus-pass-v23/corpus_pass/columns.parquet` — same for
  v23 if needed for diagnosis
- `output/v23-precision-retrain/per_cluster_fp_rate.md` — ground-
  truth fixture verdicts for the six clusters

## House style reminder

CLAUDE.md + `.orbit/STYLE.md` govern. Plain words. Lead with the
answer. One imperative action, not a menu. End-of-turn = one or two
sentences max. Don't add features beyond what each AC asks for.
Commit at natural boundaries; push at session close — work isn't
done until `git push` succeeds.

## Prime the substrate

```bash
orbit session prime
orbit spec show 2026-05-29-cluster-reachability-scoring
orbit memory search v23-precision-retrain
```
