# Spec Review: v14 Retrain (v1.2)

**Spec:** `.orbit/specs/2026-04-17-v14-retrain/spec.yaml` (version 1.2)
**Reviewer:** Nightingale (context-separated)
**Date:** 2026-04-17
**Prior review:** `.orbit/specs/2026-04-17-v14-retrain/review-spec-2026-04-17.md` (v1.1, CONDITIONAL PASS)
**Verdict:** PASS

---

## 1. v1.1 Review Findings — Resolution Check

### A1/F1: Country_code rule ordering bug — RESOLVED

The v1.1 review's critical finding was that placing the country_code guard in `value_sharpen` would be overwritten by `apply_header_sharpen`'s same-category override. v1.2 moves it correctly:

- AC-05 now says "post-hint guard in apply_header_sharpen (column.rs)" placed "AFTER the same-category hardcoded hint override block (~line 2252-2268)."
- Constraint line 15 explicitly states: "Country_code guard must be placed AFTER apply_header_sharpen's same-category override."
- The verification now requires an integration test through the "FULL classify_multi_branch pipeline" with header="country" and 2-letter code values.

Confirmed correct. The guard at the end of `apply_header_sharpen` (after the same-category override at lines 2252-2268 and all other hint paths) ensures it fires last and cannot be undone.

**One concern:** The guard lives inside `apply_header_sharpen`, which is called from both `classify_multi_branch` (~line 2023) and `classify_multi_branch_with_enriched` (~line 2123). Both call the same method, so a single implementation covers both paths. Good.

### A2: Subtype decontamination is per-value filtering — RESOLVED

Constraint line 16 explicitly states: "Subtype decontamination is per-value filtering within parent columns, not per-column dropping — different code path from v13's filter_distilled_columns."

AC-01 description is clear: "Per-value filtering within parent type columns" with specific regex patterns for each subtype.

### A3: Regex tightened to `^[A-Z]{2}$` — RESOLVED

AC-05 now uses `^[A-Z]{2}$` (not `^[A-Z]{2,3}$`) and explicitly calls out: "Use ^[A-Z]{2}$ not ^[A-Z]{2,3}$ to avoid collision with state_code." The verification includes a test confirming "^[A-Z]{2}$ does not match state codes like 'NSW' (3 chars)."

### T1: Integration test for full pipeline — RESOLVED

AC-05 verification now requires: "Integration test exercises the FULL classify_multi_branch pipeline with header='country' and values=['AU', 'US', 'GB', 'DE', 'FR'] — asserts output is country_code (not country)."

### T2: Pre-training data audit gate — RESOLVED

AC-06 is dedicated to this. Constraint line 17 explicitly states: "Pre-training data audit gate must run before overnight training to catch pipeline bugs early."

### T3: Column-level regression diff — RESOLVED

AC-09 verification now includes: "Column-level regression diff: compare every column prediction between v13 and v14 to catch label swaps." Also in exit conditions: "No regressions from v13 baseline (column-level diff)."

### G1: `h.contains("uri")` header hint — RESOLVED

Now addressed by AC-07(a). The v1.1 review flagged this as a gap; v1.2 adds it as a safe removal based on the rule audit.

### G4: Accounting-adjacent hard negatives — RESOLVED

AC-02(b) now says: "Add hard-negative decimal_number examples with accounting-adjacent headers ('amount', 'total', 'sum', 'balance')."

### G5: JWT negative examples — RESOLVED

AC-03(b) now says: "Also add JWT negative examples — strings that superficially resemble JWTs (long, dotted) but are UAs."

### G6: Target lowered from >=222 to >=220 — RESOLVED

More realistic target reflecting the difficulty of flipping 1.00-confidence hierarchical subtype predictions (#1, #2, #3).

**Summary:** All 7 "must fix" and "should fix" items from v1.1 are addressed. The 3 "nice to have" items (A3, G4, G5) are also addressed.

---

## 2. AC-07 Analysis: Rule Removals

AC-07 introduces two rule removals based on the rule audit. This is the main new content in v1.2.

### AC-07(a): Remove `h.contains("uri")` — SAFE

**Code context:** Line 4135 has `h.contains("url") || h.contains("uri") || h.contains("link") || h.contains("href")`. AC-07 removes only the `h.contains("uri")` clause.

**Positive impact:** Directly fixes the root cause of audit item #1. The header "data_uri" currently matches `h.contains("uri")` and overrides the model's prediction to `url`. Removing this lets the model's prediction (or the retrained prediction) stand.

**Regression risk analysis:**
- The exact match `"uri"` at line 3859 still maps to `url` for headers that are literally "uri". No loss.
- Headers like `"request_uri"`, `"redirect_uri"`, `"base_uri"` would previously match the keyword rule and map to `url`. After removal, these rely on the model or the `h.contains("url")` match (which doesn't help for "uri"-only headers). The Model2Vec semantic hint should cover "request_uri" and similar.
- **Risk:** A column with header "base_uri" containing URLs would no longer get the keyword hint boost. However, the model should predict `url` on URL values regardless of the header hint. And `h.contains("url")` still catches headers like "redirect_url".
- **Mitigation:** AC-09 verification says "For removed rules (ac-07), verify no regression on columns those rules previously corrected." This is adequate.

**Verdict:** Safe. The exact match covers the clean case; the keyword match was actively harmful.

### AC-07(b): Remove F3 (HS code feature rule) — SAFE

**Code context:** F3 at ~line 2436 uses statistical features (digit_ratio, dot_segments, float_fraction) to override decimal_number predictions to hs_code. R20 at ~line 2672 validates actual HS code format and demotes false positives back to decimal_number.

**Positive impact:** Eliminates the F3->R20 round-trip. The rule audit correctly identifies this as net zero with extra complexity. F3 creates false hs_code intermediate states that R20 then cleans up.

**Regression risk analysis:**
- If F3 ever correctly identifies an hs_code column that the model misses AND R20 validates it, removing F3 would lose that correct prediction. But looking at the eval: hs_code columns are classified correctly by the model+R20 without F3 needing to initiate the override.
- R20 remains as the validation gate. Any model prediction of hs_code is still validated by R20.
- F3 only fires when the model predicts decimal_number. If the model predicts hs_code directly, F3 is irrelevant.

**Verdict:** Safe. R20 is the authoritative check. F3 removal reduces false-positive hs_code intermediate states.

### AC-07 verification adequacy

The verification requires: cargo test passes, h.contains("uri") is gone, exact match still works, url and hs_code eval columns still correct. This is adequate for Phase 1 removals. AC-09's column-level diff provides the backstop.

---

## 3. AC Renumbering — Cross-Reference Consistency

v1.2 added AC-07 (rule removals) and renumbered training from ac-07 to ac-08, eval from ac-08 to ac-09.

**Cross-references checked:**
- AC-06 line 116: "ac-01 checks" -- correct, still refers to subtype decontamination
- AC-08 line 145: "ac-01 through ac-04 data changes" -- correct (training uses data ACs)
- AC-09 line 161: "For removed rules (ac-07)" -- correct, refers to new rule removal AC
- Deliverables line 172: "country_code post-hint guard, h.contains('uri') removal, F3 removal" for column.rs -- correct, covers ac-05 + ac-07
- Exit conditions: no AC-specific references, all generic -- fine

**No stale references found.** The renumbering is clean.

---

## 4. Audit Coverage: 15 Items vs 9 ACs

```
| Audit # | Column         | Root Cause           | Covered By          | Notes                                    |
|---------|----------------|----------------------|---------------------|------------------------------------------|
| 1       | data_uri       | hierarchical_subtype | ac-01(a) + ac-07(a) | Decontaminate url + remove uri hint       |
| 2       | email_display  | hierarchical_subtype | ac-01(b)            | Decontaminate email                       |
| 3       | phone_e164     | hierarchical_subtype | ac-01(c)            | Decontaminate phone_number                |
| 4       | location.country| ground_truth_debate | ac-05               | Post-hint country_code guard              |
| 5       | address.country | ground_truth_debate | ac-05               | Post-hint country_code guard              |
| 6       | year           | model_error          | ac-02(a)            | Strict 6-digit compact_ym training        |
| 7       | user_agent     | model_error          | ac-03(b)            | Diverse UAs + JWT negatives               |
| 8       | gap            | model_error          | ac-02(b) + ac-04    | amount_accounting formatting + hard negs  |
| 9       | depthError     | model_error          | ac-04               | Lat/decimal hard negatives with headers   |
| 10      | status_code    | training_collision   | ac-02(c)            | Postal/status separation                  |
| 11      | id             | training_collision   | ac-02(e)            | alphanumeric_id prefix+code patterns      |
| 12      | geojson        | data_gap             | ac-03(a)            | JSON-as-string training examples          |
| 13      | sha256         | training_collision   | ac-02(d)            | Hash/TSID length separation               |
| 14      | user_agent     | data_gap             | ac-03(b)            | Short tool UAs                            |
| 15      | status_code    | training_collision   | ac-02(c)            | Same as #10                               |
```

**All 15 audit items are covered by at least one AC.** Items #7 and #14 (both user_agent) are jointly addressed by ac-03(b). Items #8 and #9 (decimal confusion) are addressed by both ac-02(b)/ac-04 from different angles.

**Coverage quality:**
- Items #1-5, #10-11, #13, #15: Strong fix paths (decontamination, guard, collision separation)
- Items #6, #12: Reasonable fix paths (training data quality)
- Items #7, #8, #9, #14: Fix depends on model learning from improved data. Known limitation line 178 acknowledges #2/#3 may not flip. Items #7/#14 at 1.00 confidence are similarly at risk but not called out.

**Gap:** known_limitations mentions #2/#3 as hardest to fix but not #7 (user_agent->jwt at 1.00). The spec should acknowledge that #7's 1.00-confidence false pattern may resist retraining just as much as #2/#3.

---

## 5. New Findings

### N1: AC-05 placement detail — minor ambiguity (LOW)

AC-05 says the guard goes "in apply_header_sharpen" but doesn't specify exactly where. The function has multiple early-return paths (same-category override at 2266, cross-domain override at 2288, general hint at 2304, etc.). The guard must go at the very END of `apply_header_sharpen`, AFTER the locale re-detection block (line 2346-2352) but BEFORE the closing `}`. If placed after a specific override block but before later blocks, a later path could overwrite it.

Actually, re-reading: the function uses `return` for the override paths (lines 2245, 2266, 2288). The guard should go after all hint logic but before the locale re-detection. The spec says "AFTER the same-category hardcoded hint override block" which is technically correct but could be misread as "immediately after that block." The constraint is clearer: "placed AFTER apply_header_sharpen's same-category override." The integration test (verifying the full pipeline) is the real safety net here.

**Recommendation:** During implementation, place the guard as the LAST logic before locale re-detection (line 2346), covering all hint paths including the threshold-based hardcoded hint authority at line 2331. The integration test will catch incorrect placement.

### N2: `h.contains("url")` subsumes many "uri" cases (INFO)

After removing `h.contains("uri")`, headers like "download_url" still match via `h.contains("url")`. Only pure "uri" headers without "url" lose the keyword hint. The exact match for `"uri"` at line 3859 covers the literal case. Headers like "base_uri" or "request_uri" lose the keyword hint — but these are URI-containing URLs that the model should classify correctly from value patterns.

### N3: AC-08 scope is narrow (INFO)

AC-08 says "Train v14 with all ac-01 through ac-04 data changes." This excludes ac-05 (country_code guard) and ac-07 (rule removals) from the training scope, which is correct — those are code changes, not data changes. The wording is precise.

### N4: Missing known_limitation for #7 (LOW)

As noted in section 4, the known_limitations block mentions #2/#3 as hardest to fix (1.00-confidence subtypes) but #7 (user_agent->jwt at 1.00) is equally resistant. AC-03(b) does add JWT negative examples which helps, but the 1.00 confidence is a strong false pattern.

**Recommendation:** Add #7 to known_limitations alongside #2/#3.

### N5: Dual-path guard placement (LOW)

Both `classify_multi_branch` and `classify_multi_branch_with_enriched` call the same `apply_header_sharpen` method. The country_code guard inside that method covers both paths automatically. No action needed — just confirming no gap.

---

## 6. Constraint Compliance

All constraints from v1.1 review remain satisfied. The three new constraints (lines 15-17) addressing review findings are well-specified and integrated into the relevant ACs.

---

## Summary

```
| Category                        | Items | Issues | Notes                                              |
|---------------------------------|-------|--------|----------------------------------------------------|
| v1.1 findings resolution        | 10    | 0      | All must-fix, should-fix, and nice-to-have resolved |
| AC-07 rule removals             | 2     | 0      | Both removals are safe; R20 backstops F3            |
| AC renumbering consistency      | 5     | 0      | All cross-references correct                        |
| Audit coverage (15 items)       | 15    | 0      | All items mapped to at least one AC                 |
| New findings                    | 5     | 0 crit | N1/N4 are low-severity; rest informational          |
```

### Verdict: PASS

The v1.2 spec properly addresses all findings from the v1.1 CONDITIONAL PASS review. The new AC-07 (rule removals) is well-grounded in the rule audit, both removals are safe, and the AC renumbering is consistent. All 15 audit items have clear AC coverage.

**Optional improvements (not blocking):**

1. **N4:** Add audit item #7 (user_agent->jwt at 1.00) to `known_limitations` alongside #2/#3. Same risk profile.
2. **N1:** During implementation, place the country_code guard as the last logic in `apply_header_sharpen` before locale re-detection. The integration test will catch incorrect placement regardless.
