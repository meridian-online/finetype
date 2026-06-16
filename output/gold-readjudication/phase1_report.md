# Mixed-panel gold re-adjudication — phase 1 report (ac-05 author gate)

**Date:** 2026-06-16 · 4 blind panels (Opus + Sonnet + Haiku neutral, Opus adversarial)
over 260 `ac-03`-heuristic gold columns + 28 strong-tier controls. Panels saw header +
values only — blind to current gold and to the FineType model. Consensus = majority of the
3 neutral panels, conf floor 0.6; leaf-normalised comparison.

## Verdicts (260 ac-03 columns)

| verdict | n | % |
|---|---:|---:|
| CONFIRM (panel == current gold) | 160 | 62% |
| PROPOSE (panel disagrees, high-confidence) | 47 | 18% |
| CONTESTED (no strong consensus) | 53 | 20% |
| TAXONOMY-GAP | 0 | — |

## Integrity — this is not goalpost-moving

- **Negative control: panel confirms the strong-tier gold on 23/28 = 82%.** The bar isn't
  runaway-aggressive. The 5 control "misses" are mostly defensible-either-way or
  blind-disadvantaged (e.g. `base_salary` → the blind panel says decimal_number because,
  seeing only bare numbers, it can't see the money context the original contextual
  adjudication used — confirming currency is header-decisive, not a gold error).
- **Of the 47 corrections, 37 DISAGREE with the FineType model, only 10 agree.** 79% of
  corrections move gold to a label the model *also* misses → re-adjudication is demonstrably
  not flattering the model.

## Proposed corrections by transition

| n | gold → panel | matches model |
|--:|---|--:|
| 15 | decimal_number → alphanumeric_id | 0 |
| 6 | plain_text → categorical | 0 |
| 6 | plain_text → alphanumeric_id | 4 |
| 6 | plain_text → entity_name | 2 |
| 5 | top_level_domain → categorical | 2 |
| 3 | integer_number → alphanumeric_id | 0 |
| 3 | decimal_number → unix_seconds | 0 |
| 1 | integer_number → unix_milliseconds | 1 |
| 1 | plain_text → username | 0 |
| 1 | alphanumeric_id → uuid | 1 |

The 10 headline-movers (accepting → model becomes correct vs corrected gold): `Submission
Time`→unix_milliseconds, `id`→uuid, ×5 `BenchmarkName`→alphanumeric_id, `Title`/`name`→
entity_name, `TLD`/`IDN_TLD`→categorical.

## Caveats

1. **Datetime "contested" is mostly vocabulary noise.** Columns like `created_at`/`date`
   split as `iso` / `calendar_date` / `date` / `sql_standard` — the panels agree it's a
   date/timestamp but used different sub-format names (the brief didn't constrain datetime
   granularity). These are a vocabulary-cleanup, not genuine disagreement.
2. **Blind value-only panels are disadvantaged on header-decisive types** (currency, some
   codes) — the `base_salary` control miss is the tell. Header-decisive corrections should
   be treated cautiously.
3. Same-family panel (Opus/Sonnet/Haiku) → consensus is an upper bound on determinability
   (cross-vendor unavailable). The integrity proof (above) does not depend on this.

## Recommended acceptance policy (author decides)

- **ACCEPT** the unambiguous improvements: `decimal/integer → unix_seconds/milliseconds`
  (epoch timestamps mislabelled numeric), `id → uuid`, `plain_text → entity_name/
  alphanumeric_id` where panel+adversarial agree at conf ≥ 0.85.
- **DEFER/REVIEW** the debatable ones: `top_level_domain → categorical` (TLD is a real
  specific type; the panel may be over-generalising), `decimal_number → alphanumeric_id`
  ×15 (inspect — could be record-ids or could be panel over-reach).
- **KEEP gold + flag confidence** on CONTESTED (the analyst-answer set).
- **Re-run datetime** with a constrained sub-format vocabulary before relabelling any.
