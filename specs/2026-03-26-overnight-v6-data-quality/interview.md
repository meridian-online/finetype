# Interview: Overnight v6 Training — Generator Audit, Eval Expansion, Data Quality

**Date:** 2026-03-26
**Interviewer:** Nightingale

---

## Context

Follow-up to overnight v5 training (154/190 raw, 154/190 post-Sharpen — STOP condition triggered). Hugh raised three observations that expand the scope beyond the original v6 data quality spec:

1. Eval only covers 131/250 types (119 dark) — can't measure what we can't see
2. Password generator status unclear (found: working, but stale header_hint reference)
3. HS code collision with decimal: 2-level values like `8471.30` shouldn't be classified as hs_code

## Pre-Interview Investigation

Three parallel audits were run before the interview:

- **Eval gap**: 119 types uncovered. Datetime biggest gap (59/84 dark). ~25 high-priority types identified.
- **Password audit**: Generator working, passes `check`. Stale `identity.credential.password` in deprecated `header_hint()` (should be `identity.person.password`).
- **HS code audit**: 10% of generated hs_code values are 2-level (e.g., `8471.30`) — indistinguishable from decimals. Validation pattern allows this.

## Interview Q&A

### Q1: Scope — data quality only vs eval expansion vs both
**Q:** The v6 spec focuses on 3 data quality bugs + Sharpen re-tuning. But 119/250 types have zero eval coverage. Should v6 expand eval first, stay focused, or do both?
**A:** Both in parallel. Also asked: "Are there any other collision areas that we can simplify?"

### Q2: Currency code collision
**Q:** The currency_code collision is a taxonomy issue — two types in different domains for the same values. Collapse to one type or accept both in eval?
**A:** Collapse to one type (`finance.currency.currency_code`).

### Q3: Date format ambiguity
**Q:** iso_date (2024-03-15) vs iso (2024-03-15T10:00:00Z) — collapse or keep separate?
**A:** Keep separate. "This is exactly what the Sharpen layer is for. Checking for `T` is an easy rule to implement. The difference is meaningful for analysts."

### Q4: Generator collision audit scope
**Q:** Should we audit just hs_code, do a quick check of known collisions, or run a full generator collision audit?
**A:** Full generator collision audit — systematically test every generator pair that shares a broad_type.

### Q5: Eval expansion target
**Q:** How many new eval types should we target — top 10, all ~25 high-priority, or just datetime gaps?
**A:** All ~25 high-priority types.

### Q6: Collision resolution framework
**Q:** When the audit finds collisions, use "simpler type wins" universally, context-dependent, or case-by-case?
**A:** Case-by-case. No universal rule — decide each collision individually based on what analysts would expect.

### Q7: Training parameters
**Q:** Same parameters as v5 (3000/type, 20 epochs, seed 42), increase samples for colliders, or bump epochs?
**A:** Asked about v5 scaled architecture results first (Q8).

### Q8: Architecture — scaled vs current
**Q:** v5 scaled model (char 450, embed 300, stats 192→96, trunk 750→750) scored identically on raw (154/190) and worse on domain (164 vs 166). The ceiling is data quality, not model capacity. Confirm current arch only?
**A:** Initially asked to "confirm the scale change in the architecture" — wanted to see the actual parameters. After reviewing, confirmed: **current architecture only**, invest saved time into data quality.

### Q9: Collision approval process
**Q:** The case-by-case approach means someone must approve each resolution. Audit → table → approve → fix, or faster approaches?
**A:** Audit → table → approve → fix. Full checkpoint before changing generators.

### Q10: Success metric with expanded eval
**Q:** If eval denominator grows, how to measure success? Both baselines, expanded only, or percentage-based?
**A:** Expanded set only. New baseline for v7+. Old 190-column baseline becomes historical.

### Q11: Sharpen re-tuning approach
**Q:** Just add the T-separator rule, or systematic analysis of v5 error patterns first?
**A:** Systematic analysis first. Run v5 model through profile eval with detailed Sharpen trace, then propose rules based on evidence.

---

## Post-Interview Discovery: Stale Label Audit

During the interview, investigating the `identity.financial.currency_code` prediction in v5
eval output revealed a systemic issue: the `header_hint()` function and Sharpen rules in
`column.rs` contain **9 stale label references** — labels that don't exist in the current
250-type taxonomy. When these fire, they inject phantom labels into model output, causing
guaranteed eval regressions.

### How it was found

The v5 eval showed the model predicting `identity.financial.currency_code` for currency
columns. But the model's `label_map.json` only contains `finance.currency.currency_code`
(correct). The stale label was being injected by `header_hint()` at line 3843 of `column.rs`,
**overriding the model's correct prediction**.

### Full stale label inventory

```
| Line(s)       | Stale Label                      | Correct Label                      | Source          |
|---------------|----------------------------------|------------------------------------|-----------------|
| 3843          | identity.financial.currency_code | finance.currency.currency_code     | header_hint()   |
| 4006          | identity.credential.password     | identity.person.password           | header_hint()   |
| 2444,2921-30  | geography.trade.hs_code          | geography.transportation.hs_code   | Sharpen rule    |
| 5664          | geography.transport.iata_code    | geography.transportation.iata_code | Sharpen rule    |
| 3767          | technology.identifier.uuid       | representation.identifier.uuid     | header_hint()   |
| 3895          | technology.development.os        | NO MATCH in taxonomy               | header_hint()   |
| 3964-3969     | geography.address.street_address | NO MATCH (nearest: full_address)   | header_hint()   |
| 3972          | datetime.date.iso_date           | NO MATCH (nearest: datetime.date.iso) | header_hint() |
| 186-188       | technology.development.boolean   | representation.boolean.* (3 variants) | legacy compat  |
| 188           | technology.data.boolean          | representation.boolean.* (3 variants) | legacy compat  |
```

### Impact assessment

- **Confirmed v5 regressions caused by stale labels**: 2 currency columns (ecommerce_orders.currency, financial_data.currency) — model predicted correctly, header_hint overwrote with nonexistent label
- **Potential hidden regressions**: hs_code, iata_code, uuid, and boolean Sharpen rules may also be overriding correct model predictions with stale labels
- **Labels with NO taxonomy match** (os, street_address, iso_date): these produce predictions that can never score correctly in eval, regardless of model quality
- **Decision 0042** deprecated `header_hint()` but the function is still active in the pipeline

### Recommendation

Fix all stale references before training v6. This is pre-requisite work — training on clean
data won't help if the Sharpen layer corrupts predictions with phantom labels afterwards.
This should be a new AC in the v6 spec.

---

## Summary

### Goal
Push post-Sharpen accuracy past the v4 baseline by fixing data quality issues (generators, augmentation, oversampling), expanding eval coverage to 25 new high-priority types, auditing all generators for collisions, and re-tuning Sharpen rules based on systematic error analysis. Ship as sherlock-v6.

### Constraints
- Single overnight run on M1 Pro with Metal (~8 hours, same budget as v5)
- Current architecture only (scaled showed no benefit in v5)
- Case-by-case collision resolution with approval gate (audit → table → Hugh approves → fix)
- 29 profile eval datasets excluded from training corpus
- Sharpen rule changes must be justified by systematic error analysis
- Eval moves to expanded set — old 190-column baseline becomes historical

### Success Criteria
- All 9 stale label references in column.rs fixed or removed (zero phantom labels)
- Generator collision audit complete with approved resolutions
- Eval expanded to cover ~25 high-priority types (new denominator)
- Augmentation rate ≥30% (was 12.2% in v5)
- Oversampling reaches target multiplier (3x = 9000/type)
- Post-Sharpen accuracy improves over v5 on expanded eval
- Currency_code collapsed to single canonical type (MADR recorded)
- HS code validation requires 3+ dot levels
- No regression below v4 baseline on the original 190-column set

### Open Questions
- Training parameters (3000/type, 20 epochs, seed 42) — not explicitly confirmed for v6, assumed same as v5
- Sharpen systematic analysis may surface rules beyond the T-separator — scope TBD after analysis
- Full generator collision audit scope unknown until audit runs — could be 5 or 50 collisions
- For stale labels with NO taxonomy match (os, street_address, iso_date): remove the hint entirely or map to nearest valid type?

### Decisions to Record (MADR)
1. Collapse currency_code to `finance.currency.currency_code`
2. HS code: 3+ dot levels required (2-level values are decimal)
3. Eval moves to expanded baseline (old 190-column historical)
4. Fix all stale label references in Sharpen/header_hint before training v6
