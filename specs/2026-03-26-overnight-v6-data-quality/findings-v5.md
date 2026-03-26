# Overnight v5 Training — Findings Report

**Date:** 2026-03-26
**Spec:** specs/2026-03-25-overnight-training-v5/spec.yaml
**Sprint:** m-16
**Outcome:** STOP condition triggered — both variants <155/190 post-Sharpen

---

## Results Summary

```
| Model                        | Raw Label | Post-Sharpen Label | Post-Sharpen Domain |
|------------------------------|-----------|-------------------|---------------------|
| v4-sibling (baseline)        | 121/190   | 155/190 (81.6%)   | 167/190 (87.9%)     |
| v5-current (20 epochs)       | 154/190   | 154/190 (81.1%)   | 166/190 (87.4%)     |
| v5-scaled (20 epochs)        | 154/190   | 154/190 (81.1%)   | 164/190 (86.3%)     |
```

**Key observation:** The raw model improved dramatically (+33 columns, 121→154), but post-Sharpen regressed by 1 (155→154). The model is now doing work that Sharpen used to do, but making different mistakes that Sharpen can't rescue.

---

## What Worked

### 1. Raw model accuracy leap (121 → 154)
The combination of generator fixes, augmentation, and oversampling produced a much stronger base model. This is the largest single-run improvement in the project's history.

### 2. Generator collision fixes (AC-1)
hs_code (3-level XXXX.XX.XX), version (≥3-part M.N.P), ssn (area-constrained), lat/lon (variable precision) — all verified passing via `finetype check`.

### 3. Training infrastructure
Overnight script with TUI dashboard, adaptive epoch reduction (40→20 based on timing probe), resumable stages, comparison reporting — all solid for future runs.

### 4. Augmentation module
91,692/749,261 columns augmented (12.2%) across whitespace, encoding, null-mix, and case variation. Infrastructure is in place.

---

## What Didn't Work

### 1. Augmentation rate mismatch (12.2% actual vs 35% target)
The spec called for 30-40% of training samples to be augmented. The actual rate was 12.2%. The augmentation function applies at the column level but many columns were skipped (likely due to type-specific filtering or implementation gaps). **This means the model got less noise exposure than intended.**

### 2. Label validation was inert (0 exclusions)
AC-4 called for validating distilled labels and excluding rows where <50% of values match. The validation ran on 63,254 columns but excluded zero. Either:
- (a) The validation patterns are too permissive (likely — many types lack strict validators), or
- (b) The distilled labels are genuinely clean (unlikely given 230/249 types have <20% distilled data)

This means potentially noisy distilled labels passed through unchecked.

### 3. Massive distilled data gap
- 230/249 types (92%) have <20% distilled data
- 125/249 types (50%) have **zero** distilled data
- Only ~20 types reach the target 50/50 distilled/synthetic blend

The model is training predominantly on synthetic data. This limits its ability to handle real-world distribution quirks.

### 4. Oversampling didn't increase total samples
The 10 oversampled types show 3,000-5,234 samples each — **not** the 6,000-9,000 the spec called for (3x the base 3,000). The oversample multiplier was set to 3x but appears to have been applied to the distilled portion only, not the total.

### 5. No "format mixing" augmentation
AC-2(c) called for format mixing within columns (e.g., 85% E.164 + 15% local phone format). The augmentation stats show only whitespace, encoding, null_mix, and case_variation — no format mixing was implemented.

---

## Regression Analysis

16 regressions from v4 baseline, 70 fixes. Net: many more columns improved than regressed, but on the 190-column format-detectable subset, the regressions cancelled out the gains.

### Top Regression Patterns

**Pattern 1: Decimal → HS Code confusion (2 columns)**
- `iris.petal_length` (decimal_number → technology.internet.cidr)
- `scientific_measurements.value` (decimal_number → geography.transportation.hs_code)
- **Root cause:** The hs_code generator fix (3-level XXXX.XX.XX) created training data that looks like decimal numbers. The model now over-predicts hs_code for any dotted decimal.

**Pattern 2: Date format confusion (3-4 columns)**
- `medical_records.date_of_birth` (iso → mdy_dash)
- `medical_records.visit_date` (iso → iso_date)
- **Root cause:** Date format variants have overlapping patterns. The model can't reliably distinguish `YYYY-MM-DD` from `YYYY-MM-DDThh:mm:ss` (iso_date includes the T separator). Sharpen R-rules that rescued v4 don't fire on the v5 model's predictions because the confidence distribution shifted.

**Pattern 3: Currency code mapping (2 columns)**
- `ecommerce_orders.currency` (finance.currency.currency_code → identity.financial.currency_code)
- `financial_data.currency` (same)
- **Root cause:** Taxonomy has two currency_code types in different domains. The model learned one, eval expects the other. This may be a taxonomy issue, not a model issue.

**Pattern 4: Miscellaneous integer/numeric (4-5 columns)**
- Various integer columns getting wrong sub-classifications
- **Root cause:** Broad type confusion in the numeric domain where many types share similar character distributions.

---

## Data Quality Signals

### The "synthetic wall"
With 92% of types below 20% distilled data, the model is learning synthetic patterns, not real-world patterns. The generator fixes improved synthetic quality (hence the raw accuracy jump), but the gap between synthetic and real-world data remains the primary ceiling.

### Sharpen rule alignment
The v4 model was weak (121 raw) but Sharpen rescued 34 columns (121→155). The v5 model is stronger (154 raw) but Sharpen rescues 0 net columns (154→154). This suggests the Sharpen rules were tuned to compensate for v4's specific failure modes. They may need re-tuning for v5's different error profile.

### Augmentation under-delivery
At 12.2% instead of 35%, the model got insufficient noise exposure. This may explain why it performs well on clean synthetic data but struggles with messy real-world formats.

---

## Recommendations for v6

1. **Fix augmentation coverage** — Debug why 12.2% instead of 35%. Implement format mixing (AC-2c from v5 spec).
2. **Fix oversampling** — Ensure confused types actually get 3x total samples (9,000), not 3x of the distilled portion.
3. **Expand distilled corpus** — The biggest accuracy ceiling is the synthetic wall. Either:
   - (a) Run another distillation pass on new real-world datasets, or
   - (b) Accept the synthetic-heavy blend and focus on generator quality for the 125 zero-distilled types.
4. **Re-tune Sharpen for v5** — Analyze which Sharpen rules fired for v4 but not v5, and whether new rules or adjusted thresholds could rescue the 16 regressions.
5. **Address hs_code generator regression** — The 3-level fix created a new collision with decimal numbers. Consider adding leading-zero constraints or chapter-prefix patterns that don't overlap with measurement values.
6. **Resolve currency_code taxonomy ambiguity** — Decide which domain owns currency_code (finance or identity) and collapse the duplicate.

---

## Appendix: Training Parameters

```
| Parameter              | Value                  |
|------------------------|------------------------|
| Samples per type       | 3,000                  |
| Oversample multiplier  | 3x (10 types)          |
| Augmentation rate      | 0.35 (actual: 12.2%)   |
| Epochs (adaptive)      | 20 (reduced from 40)   |
| Batch size             | 32                     |
| Learning rate          | 0.0001                 |
| Seed                   | 42                     |
| Total records          | 389,095                |
| Table groups           | 53,215                 |
| Types covered          | 249/250                |
| Wall time              | 8h 0m                  |
```
