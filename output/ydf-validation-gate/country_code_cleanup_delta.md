# Country-code gate cleanup — pre/post delta

Per spec `2026-05-26-taxonomy-country-code-enum-cleanup` ac-05.

## What changed

`scripts/apply_ydf_validation_gate.py`:

- Removed `ENUM_SKIP_LABELS = frozenset(["geography.location.country_code"])`.
  The skip was dead code — `_compile_spec` had a pattern-wins-first
  priority that already excluded the enum for country_code,
  regardless of the skip list. The skip's comment was also wrong:
  the yaml enum is canonical 249 ISO 3166-1 alpha-2 codes, not
  contaminated.
- Replaced the single-kind priority logic with joint pattern + enum
  application when both are present. `passes()` now requires the
  value to satisfy BOTH the regex AND be in the enum set, mirroring
  the joint semantics already in
  `crates/finetype-core/src/validator.rs::CompiledValidator`.

13 labels now use the joint mechanism (up from 12 enum-only labels
before — country_code joined the group).

## Cell-2 delta (canonical scoring lens)

Re-ran the gate against `output/corpus-pass-v22/corpus_pass/columns.parquet`:

| metric | pre-cleanup | post-cleanup | Δ |
|---|---:|---:|---:|
| files | 503,643 | 503,643 | 0 |
| YDF country_code (raw) | 4,044 | 4,044 | 0 |
| YDF country_code (gated, kept) | 2,329 | 2,323 | **−6 (−0.3%)** |
| Cell-2 total (gated) | 69,458 | 69,458 | 0 |
| Cell-2 country_code (gated) | 11 | 11 | 0 |

The cell-2 metric **did not move**. The 6 newly-refused columns
all had `sense_prediction` already in geography (country_code: 3,
region: 2, country: 1) — they don't contribute to cell-2 (which
counts `sense NOT LIKE 'geography.%' AND ydf LIKE 'geography.%'`).

v22's headline `−10.4% gated cell-2 vs v19` is unchanged.

## Why so little movement

Most YDF country_code mislabels fall into two large buckets:

1. **Wrong shape** (e.g. 3-letter team codes like `UTA`, `FLA`, or
   `TEAM_ABBREVIATION` values, exchange codes like `GER`). These
   fail the alpha-2 pattern → already refused pre-cleanup.
2. **Right shape, right meaning** (e.g. `US`, `GB`, `JP`). Pass
   both pattern and enum → kept in both regimes.

The space where the cleanup actually matters — values that match
`^[A-Z]{2}$` but are NOT in ISO 3166-1 (`UT`, `OK`, `OR`, `OH`,
`IA`, `NY`, ...) — is a small slice of the corpus. The 6-column
delta in this batch sets the magnitude.

## Pass-rate distribution

Across 4,044 YDF-labelled country_code columns:

- Average pass-rate dropped by 0.009 (i.e. ~1pp tighter).
- 202 columns saw a strictly lower pass-rate post-cleanup.
- Min shift: −0.75 (a single column where enum membership was 25%
  while pattern match was 100%).
- Max shift: 0.00 (no column got more permissive).

Only 6 of the 202 columns crossed the 0.5 threshold.

## Verdict

Principled cleanup, alignment win, near-zero metric movement. The
gate is now consistent with the rest of the codebase
(`CompiledValidator`, MCP schema export, DuckDB `finetype_validate`).
Future Sense retrains scored against this gate get a slightly more
honest baseline.
