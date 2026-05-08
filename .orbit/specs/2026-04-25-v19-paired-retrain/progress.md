# Implementation Progress

Spec path: .orbit/specs/2026-04-25-v19-paired-retrain/spec.yaml
Spec hash: pending
Started: 2026-04-25
Current AC: ac-09 (post-sweep)

## Hard Constraints
- [x] v4 corpus base with pre-flight conflict check
- [x] 3-seed × 2 architectures = 6 runs, partial-seed = auto-fail gate condition 1
- [x] Single overnight script (~15h), failure recovery per run
- [x] MADR 0066 hard gate applies independently per architecture
- [x] Three-way diff: v16 baseline vs ReLU-v19 vs GELU-v19
- [x] Winner takes all promotion
- [x] Container types via TABLE_TEMPLATES
- [x] No Sharpen changes
- [x] GELU+LN via config only
- [x] Existing pipeline-layer fixes remain

## Acceptance Criteria
- [x] ac-01: Cherry-pick v4 corpus additions (UA loader, LOINC, generators) — v4 changes already on main, loaders copied from branch
- [x] ac-02: Container TABLE_TEMPLATES in prepare_multibranch_data.py — 5 new templates covering all 11 container types
- [x] ac-03: Datetime generator improvements (6 subtypes) — iso_8601_compact, iso_8601_milliseconds, iso_microseconds, pg_short_offset, ordinal, jp_era_short. cargo check 240/240
- [x] ac-04: FTMB v5 audit gate passes — embedded in overnight script with 6 gates including container type check
- [x] ac-05: MADR 0068 superseding 0046 (status: proposed) — .orbit/choices/0068-revisit-gelu-layernorm.md
- [x] ac-06: overnight_v19_paired.sh sweep script — 6 runs, pre-flight checks, audit gate, failure recovery, summary table
- [x] ac-07: GELU+LN config file — models/sherlock-v19-gelu-config.json (activation: GELU, use_layer_norm: true)
- [x] ac-08: v19_compare post-sweep eval script — three-way diff, per-domain delta, MADR 0066 gate evaluation, winner recommendation
- [x] ac-09: (post-sweep) At least one architecture passes MADR 0066 gate — **NO. Both FAIL.**
  - ReLU+BN: Gates 1,2,4,5,6 PASS. Gate 3 FAIL: net_label_delta = −6 (365 vs 371)
  - GELU+LN: Gates 1,3,4 FAIL. val_acc < 91.2%, label delta −44, domain delta −19
- [x] ac-10: (post-sweep) Three-way diff published — diagnostics/v19_per_column_diff.tsv, diagnostics/v19_per_domain_delta.tsv
- [x] ac-11: (post-sweep, conditional on ac-09) Winner promoted — **SKIPPED: no winner. v16 remains shipped.**
- [x] ac-12: (post-sweep, conditional on ac-09) CLAUDE.md updated — **SKIPPED: no promotion.**

## Notes

AC-01: v4 loaders (user_agent.py, loinc.py) copied from origin/distilled-data-relabel-7-types-v17.
The core v4 changes (prepare_multibranch_data.py, generator improvements) were already merged to main
via prior PRs.

AC-03: Generator improvements stay within existing validation patterns in taxonomy YAML. Key signals:
iso_8601_compact (T separator), iso_8601_milliseconds (non-zero millis .NNN), iso_microseconds
(non-zero micros .NNNNNN), pg_short_offset (space separator + offset), ordinal (YYYY-DDD),
jp_era_short (era prefix + slash separators).

AC-09..AC-12 are post-sweep — addressed after Hugh kicks off the overnight run and results are in.

## Sweep Results (2026-04-27)

### Training accuracy (val_acc)
- ReLU+BN: s42=91.26% s43=91.53% s44=91.73% — all 3 pass ≥91.2% gate
- GELU+LN: s42=85.99% s43=85.76% s44=85.99% — ~6 pts below ReLU, all fail

### Profile eval (448-row expanded manifest)
- v16 baseline: 371/448 (82.8% label, 88.3% domain)
- Best ReLU (s42): 365/448 (81.4% label, 88.3% domain) — Δlabel = −6
- Best GELU (s44): 327/448 (72.9% label, 84.1% domain) — Δlabel = −44

### Outcome
Neither architecture passes MADR 0066. v16 remains the shipped model.
GELU+LN is definitively worse (~6 pts val_acc, ~10 pts profile eval).
ReLU v19 improves val_acc slightly over v16 but regresses on profile eval —
the Sharpen layer (header hints, disambiguation rules) is doing heavy lifting
that raw model accuracy doesn't capture. Training data improvements alone are
insufficient; the model needs to learn patterns currently handled by rules.
