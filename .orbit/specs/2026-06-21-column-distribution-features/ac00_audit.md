# ac-00 — B07 audit of the feature contract. Premise corrected; clean architecture confirmed.

## The surface: extract_column_stats (column_stats.rs), COLUMN_STATS_DIM = 27
The multi-branch "stats" branch is `extract_column_stats(values) -> [f32; 27]`. Its 27 dims:
- **Entropy & Cardinality (4):** `col_entropy`, `frac_unique`, `n_values`, `frac_empty`
- **Length stats (8):** mean/var/min/max/median/skew/kurtosis/sum of value LENGTH
- **Character composition (10):** frac numeric/alpha/special cells; avg/std digit/alpha/special; avg word
- **Structural (5):** std word; frac starts-upper/all-upper/all-lower/mixed-case

## PREMISE CORRECTION (why the audit mattered)
The spec said the model is blind to "cardinality, sequentiality, range." **Cardinality is
ALREADY present** — `frac_unique` + `col_entropy`. (The `column/mod.rs:2266` comment "the value
head cannot see column cardinality" is about the separate VALUE/fusion head, NOT the multi-branch
stats branch the Sense model uses.) So:
- **Cardinality: PRESENT** → the categorical/id residual-attractor is NOT feature-poverty. 0096 holds
  for those; do NOT re-litigate them here.
- **Numeric value / range: ABSENT** — extract_column_stats never parses the value; `length_min/max`
  are STRING length, not magnitude. This is the genuine gap (coordinates/year/unix/port/amount).
- **Precision: ABSENT from the branch** — `decimal_places` exists per-value (features.rs) but is NOT
  aggregated into the 27; the multi-branch never sees it.
- **Sequentiality / monotonicity: ABSENT** — no contiguous-run / sortedness feature (increment/serial).

→ Revised ac-01 target: **numeric value/range + column-level precision + sequentiality. NOT cardinality.**

## Consumer graph + clean architecture
`extract_column_stats` / `COLUMN_STATS_DIM` consumers (rg, lib.rs export):
- `column_stats.rs` — definition (+ `COLUMN_STATS_NAMES`).
- `finetype-cli/src/main.rs:1473` — the `extract-features --json` subcommand (TRAINING-feature extraction).
- `finetype-model/src/multi_branch/mod.rs` — ×4 call sites (the INFERENCE / profile forward).
- `finetype-model/src/lib.rs:38` — re-export.

**KEY: extract_column_stats is the UNIFIED Rust path — same function at training-feature extraction AND
inference.** So adding features there automatically covers both → NO train/inference skew, and NO candle
build (unlike the gte embed). This is the decisive advantage of the stats branch over the embed.

## Edit set (per surface)
| surface | change |
|---|---|
| `column_stats.rs` | extend `extract_column_stats` with the new features; `COLUMN_STATS_DIM` 27 -> N; `COLUMN_STATS_NAMES` |
| (auto) `main.rs` extract-features + `multi_branch/mod.rs` ×4 | no edit — they read `COLUMN_STATS_DIM`; recompile |
| `scripts/prepare_multibranch_data.py` | `STATS_DIM` 27 -> N (must match) |
| model config | `stats_dim` 27 -> N for the new model (old configs keep 27; old models still load) |
| FTMB format | **none** — the v5 reader reads `stats_dim` from the header (dim-agnostic, like embed) |
| per-value `FEATURE_DIM` (37) / `value_feature_row` / YDF pipeline | **untouched** — separate function (`aggregate_features`), separate contract |

No FTMB version bump, no candle, no YDF/value-pipeline impact, no per-value contract change. The blast
radius is one Rust function + two dims (COLUMN_STATS_DIM, STATS_DIM, config stats_dim) + a retrain.

## Decision
Extend `extract_column_stats` (the unified Rust column-stats function). ac-01 adds: parsed numeric
value/range (min/max/mean/std/percentiles, signed-log magnitude, sign fraction, bounded-ness), column-
level precision (aggregate decimal_places: mean/max), sequentiality (monotonic fraction, contiguous-
integer-run ratio, sortedness), and length-consistency refinements (fixed-width fraction, leading-zero
fraction). Drop cardinality (present). Narrows the 0096 re-test (ac-04) to the increment/range classes,
not categorical/id.
