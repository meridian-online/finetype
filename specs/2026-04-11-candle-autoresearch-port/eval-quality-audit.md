# Eval Ground Truth Quality Audit

**Date:** 2026-04-11
**Triggered by:** v8 and v9 GELU+LN experiments both regressed ~13 labels vs baseline,
prompting the question: is the eval ground truth itself stale?

## Findings

Audited all 48 misclassifications from v6-gelu-conservative (166/214, 77.6% label accuracy).

| Verdict   | Count | Meaning |
|-----------|-------|---------|
| WRONG     |    30 | Model clearly at fault |
| DEBATABLE |    11 | Ground truth questionable |
| AMBIGUOUS |     7 | Can't distinguish without external context |

**~37% of "failures" are eval harness problems, not model problems.**

If the 18 debatable+ambiguous cases are resolved in the model's favour,
effective accuracy becomes **184/214 (86.0%)** — above the 179/214 baseline.

## Changes Made

### 1. Manifest fix: `airports.name` (1 case)

**Problem:** Airport names like "Goroka Airport" were labelled `name` which maps to
`identity.person.full_name`. These are place/entity names, not person names.

**Fix:** Changed GT label from `name` to `airport name` with a new schema_mapping
entry that accepts `entity_name`, `full_name`, or `full_address`.

### 2. Accepted alternatives added to `schema_mapping.yaml` (9 cases)

| GT Label | Primary | Alternative Added | Rationale |
|----------|---------|-------------------|-----------|
| `latitude` | `geography.coordinate.latitude` | `coordinates` | Isolated decimals need header signal |
| `longitude` | `geography.coordinate.longitude` | `coordinates` | Same — header-dependent |
| `abbreviated month date` | `datetime.date.abbreviated_month` | `abbrev_month_no_comma` | Comma/no-comma is a fine-grained distinction |
| `long full month date` | `datetime.date.long_full_month` | `full_month_no_comma` | Same |
| `iso date` | `datetime.date.iso` | `datetime.timestamp.iso_8601` | Date is a valid prefix of timestamp |
| `upc` | `identity.commerce.upc` | `identity.commerce.ean` | UPC-A is a 12-digit subset of EAN |
| `dmy dash` | `datetime.date.dmy_dash` | `mdy_dash` | Inherently ambiguous DD-MM vs MM-DD |
| `mdy dash` | `datetime.date.mdy_dash` | `dmy_dash` | Mirror of above |
| `currency symbol` | `finance.currency.currency_symbol` | `iso_4217` | Mixed symbols ("$") and codes ("EUR") |

### 3. Interchangeability rules added to `matching.rs` + `eval_profile.sql` (4 cases)

| Rule | Types | Rationale |
|------|-------|-----------|
| DMY/MDY dash | `dmy_dash` ~ `mdy_dash` | Classic DD-MM vs MM-DD ambiguity |
| Coordinate subtypes | `latitude` ~ `longitude` ~ `coordinates` | Isolated decimals need header |

## Impact Assessment

These fixes resolve **up to 11 cases** from the 48 misclassifications:
- airports.name: 1 case (now accepts entity_name/full_address)
- lat/lon → coordinates: 2 cases
- abbreviated/long month comma variants: 2 cases
- iso date → iso_8601: 1 case
- upc → ean: 1 case
- dmy_dash ↔ mdy_dash: 2 cases
- currency_symbol → iso_4217: 0-1 cases (model predicted `locale_code`, not `iso_4217`)

**Expected new scores** (to be verified by re-running eval on Mac):

| Model | Before fixes | After fixes (estimated) |
|-------|-------------|------------------------|
| v4-sibling (baseline) | 179/214 | ~183/214 (some fixes help baseline too) |
| v6-gelu-conservative | 166/214 | ~177/214 |

Note: Both models benefit from the fixes. The relative comparison is what matters.

## Remaining genuine model bugs (30 cases)

These are real model errors that should be fixed via retraining or Sharpen rules:

1. **Decimal → date/IP** (6 cases): "7.431" → `dmy_short_dot`. Dots in decimals confuse the model.
2. **Person name → `email_display`** (3 cases): "Sarah Brown" → `email_display`. No @ or angle brackets.
3. **Phone → SSN/ABN** (3 cases): "+1-232-130-2535" → `ssn`. Ignores `+` prefix.
4. **Hash → `ethereum_address`** (2 cases): No `0x` prefix present.
5. **User-agent → `docker_ref`** (2 cases): "/" separators cause confusion.
6. **Various single-instance errors** (14 cases): bitcoin_address→full_address, year→compact_ym, etc.

## Monitoring

Re-run this audit after each eval set expansion. Key metrics to track:
- Ratio of DEBATABLE+AMBIGUOUS to total misclassifications (should stay < 25%)
- Any GT label appearing in misclassifications across 3+ models → suspect GT label
- New datasets should be validated against at least 2 model variants before being trusted

## Corrected Eval Results (with ground truth fixes applied)

Eval set expanded from 214 → 227 columns (JSON fixtures added). Ground truth fixes
(accepted alternatives, interchangeability rules) resolved many debatable failures.

| Model | Label (old) | Label (corrected) | Domain (corrected) | Actionability |
|-------|-------------|--------------------|---------------------|---------------|
| v4-sibling (baseline) | 179/214 (83.6%) | 193/227 (85.0%) | 206/227 (90.7%) | 96.9% |
| v6-gelu-conservative | ~167/214 (78.0%) | 185/227 (81.5%) | 201/227 (88.5%) | 97.0% |
| **Delta** | **-12** | **-8** | **-5** | **+0.1%** |

Gap narrowed from 12 → 8 labels. However, **this comparison is still invalid** — see
progress.md root cause analysis. v6-gelu-conservative trained on data with all-zero headers
(effectively 3-branch), while v4-sibling trained with real Model2Vec headers + sibling-context
(true 4-branch). The v10 retraining will produce the first fair comparison.
