# Spec Review: v15 Value Rules (R25-R30)

**Reviewer:** Nightingale (fresh context)
**Date:** 2026-04-18
**Spec:** .orbit/specs/2026-04-18-v15-value-rules/spec.yaml
**References reviewed:** decision 0048, v14 retrain spec, v13 audit, column.rs value_sharpen/feature_sharpen/apply_header_sharpen

---

## Findings

### [BLOCKER] B1: R28 and R29 will be overwritten by header hints

**Category:** Rule interaction
**Severity:** Blocker

**Description:** R28 (email -> email_display) and R29 (phone_number -> phone_e164) are placed in `value_sharpen()` which runs at Step 4. But `apply_header_sharpen()` runs at Step 5 and contains a same-category hardcoded hint override (NNFT-194, lines 2248-2267) with NO confidence threshold. The header_hint function matches substrings:

- `h.contains("email")` matches "email_display" -> hints `identity.person.email`
- `h.contains("phone")` matches "phone_e164" -> hints `identity.person.phone_number`

The same-category override checks `rsplitn(2, '.').last()` to get the category. Both `email` and `email_display` share category `identity.person`, and both `phone_number` and `phone_e164` share category `identity.person`. The override fires unconditionally (no confidence threshold, no return guard), setting the label back to the parent type.

**Evidence:** The eval dataset `new_identity` has CSV column headers `email_display` and `phone_e164` (manifest.csv lines 283-284). The header_hint keyword matches at column.rs lines 4039 and 4042 confirm the substring match. The same-category override at lines 2252-2267 confirms no threshold. This is the exact same problem that forced the country_code guard to be placed AFTER header hints rather than in value_sharpen (v14 spec ac-05, review finding A1/F1).

**Recommendation:** R28 and R29 must be implemented as post-hint guards inside `apply_header_sharpen()`, following the pattern of the country_code post-hint guard at lines 2346-2374. They cannot live in `value_sharpen()`. This contradicts the spec's constraint "Rules go in value_sharpen(), not feature_sharpen() or apply_header_sharpen()" -- that constraint must be relaxed for R28 and R29.

---

### [HIGH] H1: R25 is dead code due to R12 (numeric disambiguation) firing first

**Category:** Rule interaction
**Severity:** High

**Description:** R25 triggers on `result_label == "geography.address.postal_code"` and checks for 3-digit integers in 100-599. However, R12 (`disambiguate_numeric`) fires earlier in value_sharpen (line 2654) and also triggers on postal_code. For 3-digit HTTP status codes like [200, 404, 500]:

- R12 checks `consistent_digits` (all 3-digit -> true), `typical_postal_range` (min=200 >= 100, max=500 <= 99999 -> true), `is_sequential` (HTTP codes aren't evenly spaced -> false)
- R12 returns `("geography.address.postal_code", "numeric_postal_code_detection")` -- confirming the wrong prediction
- value_sharpen returns `Some(...)` with the same label, and R25 never executes

This means R25 cannot fix the status_code misclassifications (audit items #10, #15) in its current position.

**Evidence:** R12 implementation at column.rs lines 4559-4638. The `consistent_digits && typical_postal_range && !is_sequential` path at line 4625 matches HTTP status code columns exactly.

**Recommendation:** Either (a) place R25 BEFORE R12 in value_sharpen, or (b) add a guard to R12's postal_code detection that excludes the 100-599 + 3-digit-only pattern, or (c) integrate R25's logic into R12 as an early check. Option (a) is simplest but requires that R25 only fire when appropriate (the current 90% threshold on 100-599 range should suffice).

---

### [HIGH] H2: Confidence cannot be set from value_sharpen -- return type mismatch

**Category:** Implementation gap
**Severity:** High

**Description:** The spec says each rule should set confidence via `max(current, 0.7)` or similar. But `value_sharpen()` returns `Option<(String, String)>` -- only label and rule name. The calling code at lines 2010-2018 sets `result.label` and `result.disambiguation_rule` but does NOT update `result.confidence`. Existing rules in value_sharpen never set confidence; they all rely on the original model confidence persisting.

**Evidence:** Function signature at line 2544: `-> Option<(String, String)>`. Calling code at lines 2016-2018 only updates `label` and `disambiguation_rule`. No existing value_sharpen rule touches confidence.

**Recommendation:** Either (a) change the return type to `Option<(String, String, f32)>` to include confidence, or (b) change value_sharpen to take `&mut ColumnResult` like feature_sharpen does, or (c) accept that confidence won't be set by these rules and remove the confidence statements from the spec. Option (b) is cleanest and aligns with feature_sharpen's pattern.

---

### [HIGH] H3: R15 (attractor demotion) may preempt R29 for low-confidence phone_number predictions

**Category:** Rule interaction
**Severity:** High

**Description:** `phone_number` is a TEXT_ATTRACTOR (line 2413). R15 (attractor demotion, line 2794) fires on TEXT_ATTRACTORS. For phone_number predictions below 0.85 confidence where locale validation fails, R15 demotes to `categorical` before R29 would fire.

For audit item #3 (phone_e164), the model predicts phone_number at 1.00 confidence, so R15 won't fire (1.00 > 0.85). However, for any future case where the model predicts phone_number at lower confidence for E.164 values, R15 would fire first and prevent R29.

Additionally, R15 does locale validation for phone_number. E.164 values (+18285346333) may or may not pass phone_number locale validators -- if they pass, R15 confirms phone_number, preventing R29. If they fail, R15 demotes to categorical, also preventing R29.

**Evidence:** TEXT_ATTRACTORS at line 2411 includes `"identity.person.phone_number"`. R15 at line 2794 fires before where R25-R30 would be placed. The audit shows the specific eval case has 1.00 confidence (safe), but the rule should be robust to future predictions.

**Recommendation:** Place R29 BEFORE R15, or add `phone_e164` value check inside R15's phone_number handling path. Note: this finding is partially mooted by B1 (R29 needs to be a post-hint guard anyway), but the interaction should still be documented.

---

### [MEDIUM] M1: R30 comma heuristic has edge cases beyond European notation

**Category:** Edge case
**Severity:** Medium

**Description:** The spec says "comma followed by exactly 3 digits before end or next comma" detects thousands separators. This heuristic fails on:

1. **Indian lakhs notation** (1,00,000): comma followed by 2 digits, not 3. Would not be detected as a thousands separator, so R30 would incorrectly override to decimal_number.
2. **Values like "1,23"**: comma followed by 2 digits -- not thousands, not the European decimal "1,234.56" either. R30 would not detect this as currency formatting, so it would override to decimal_number even though the comma presence suggests some formatting.
3. **Mixed formats**: a column with ["$100", "200", "300"] where only one value has currency formatting. R30 checks "0% of values contain currency formatting" -- a single formatted value would block the override. Is this correct behavior?

The known_limitation about European notation is appropriate but incomplete.

**Recommendation:** (a) Accept Indian lakhs as a known limitation and document it. (b) Clarify in the spec whether mixed format columns (some values with $, some without) should stay as amount_accounting. The 0% threshold is aggressive -- consider whether a small threshold (e.g., <=5%) would be more robust while still catching the target case.

---

### [MEDIUM] M2: R25 HTTP status range (100-599) overlaps with legitimate postal codes

**Category:** False positive risk
**Severity:** Medium

**Description:** Some real postal codes are 3-digit integers in 100-599. Examples:
- US: no 3-digit ZIP codes (all 5 or 9 digits) -- safe
- Many countries have 3-digit codes: Iceland (101-902), parts of Bahrain, etc.

R25 requires 90% match rate, which reduces risk. A column of Icelandic postal codes like [101, 105, 200, 300] would match the 100-599 range at 100% and be incorrectly classified as integer_number.

The spec's known_limitation acknowledges the 418 "I'm a teapot" case but does not address the postal code overlap.

**Evidence:** The threshold of 90% is reasonable -- most mixed postal code columns would have codes outside 100-599. But pure Icelandic or similar columns would be misclassified.

**Recommendation:** Add this as a known_limitation. Consider whether the rule should also require that the model's original confidence was below some threshold (e.g., < 0.9), providing a confidence gate that respects high-confidence postal_code predictions.

---

### [MEDIUM] M3: R26 does not validate hex content, only length

**Category:** Completeness
**Severity:** Medium

**Description:** R26 checks "hexadecimal strings of length 40 or 64" but the spec's description doesn't explicitly require validating that the characters are all hex digits (0-9, a-f, A-F). The TSID is "strictly 32-char hex" so the model already knows these are hex. But R26 should validate hex content, not just length, to avoid false positives on non-hex 40/64-char strings.

**Recommendation:** The verification tests use real hex values, implying hex validation is intended. Make the hex check explicit in the AC description: "hexadecimal strings (only characters 0-9, a-f, A-F) of length 40 or 64."

---

### [MEDIUM] M4: Target of 223/227 (ac-07) counts 8 fixes but B1 blocks 2 of them

**Category:** Arithmetic
**Severity:** Medium

**Description:** The spec targets 223/227 from 215/227 baseline, claiming 8 new corrections. The 8 are: status_code x2 (R25), sha256 (R26), year (R27), email_display (R28), phone_e164 (R29), gap (R30), and "one of the country columns already fixed." But:

- R28 and R29 are blocked (B1): -2
- R25 is dead code (H1): -2

That leaves R26 (sha256), R27 (year), R30 (gap) = 3 fixes. 215 + 3 = 218/227, not 223.

**Recommendation:** After fixing B1 and H1, re-derive the target. If all fixes land after architectural corrections, 223/227 is achievable. But the spec should acknowledge that R28, R29, and R25 require structural changes beyond "pure code additions to column.rs value_sharpen."

---

### [LOW] L1: R27 duplicates R12's year detection logic

**Category:** Redundancy
**Severity:** Low

**Description:** R12 (`disambiguate_numeric`) already detects year columns (4-digit integers in 1900-2100 at >=80%). R27 detects compact_ym -> year when values are 4-digit. However, R12 only triggers when `result_label` is in [increment, integer_number, decimal_number, postal_code, year, numeric_code]. It does NOT trigger on `compact_ym`. So R27 fills a genuine gap -- compact_ym is not in R12's trigger set. But the two rules implement similar logic with slightly different thresholds (R12: 80% in 1900-2100 range + 80% 4-digit; R27: 90% exactly 4 digits, no range check).

**Recommendation:** R27 should also include the 1900-2100 range check from R12 for consistency. Without it, a column of 4-digit values like ["1234", "5678", "9012"] would be overridden from compact_ym to year, even though those are not plausible years. Add a range guard: >=80% of 4-digit values must be in 1900-2100.

---

### [LOW] L2: Rule numbering gap (R25-R30 skips no numbers, but R18 is missing)

**Category:** Documentation
**Severity:** Low

**Description:** The existing rules use numbers R1-R24 but R18 is not present in the code (Rule 18 is missing). The spec introduces R25-R30 sequentially. This is fine but the missing R18 gap should be noted to avoid confusion during implementation.

**Recommendation:** No action needed -- just be aware R18 doesn't exist in the current code.

---

### [LOW] L3: No rule interaction matrix in the spec

**Category:** Documentation
**Severity:** Low

**Description:** The spec doesn't document how the 6 new rules interact with each other or with existing rules. Since value_sharpen returns on the first match, rule ordering is critical. Rules that share input types or produce output types that could trigger other rules should be documented.

- R25 output (integer_number) could trigger R10 (small integer ordinal) or R12 (numeric). But since value_sharpen already returned, these won't fire in the same pass. However, the output label matters for downstream steps.
- R30 output (decimal_number) doesn't trigger any existing rules.
- None of the 6 new rules produce output types that would trigger another new rule.

**Recommendation:** Add a brief rule interaction section noting that the 6 new rules have no mutual conflicts and documenting the ordering requirements relative to R12 and R15.

---

## Assumption Audit

```
| # | Assumption | Risk if Wrong | Status |
|---|-----------|---------------|--------|
| A1 | value_sharpen rules execute before header hints | Correct per pipeline Steps 4/5. But this means rules can be undone. | PROBLEM (B1) |
| A2 | R25-R30 will be placed at end of value_sharpen | True by convention. But R12 fires first on postal_code. | PROBLEM (H1) |
| A3 | value_sharpen can set confidence | Wrong -- return type is (String, String), not (String, String, f32). | PROBLEM (H2) |
| A4 | All existing 395 tests pass unchanged | Likely true -- new rules only add, don't modify. | OK |
| A5 | The v14 baseline is 215/227 | Confirmed by decision 0048 (3 fixes from code changes). | OK |
| A6 | Thresholds (80%/90%) are appropriate | 90% for clear-cut disambiguation (R25/R26/R27) is conservative. 80% for format detection (R28/R29) is reasonable. 0% for R30 is aggressive. | ACCEPTABLE |
| A7 | "No model changes" constraint is achievable | True for the rules themselves. But fixing B1 requires touching apply_header_sharpen, which is header hint code not rule code. | NEEDS CLARIFICATION |
```

## Test Adequacy

- **ac-01 through ac-06:** Unit tests are well-designed with positive, negative, and boundary cases. However, none of the tests exercise the full `classify_multi_branch` pipeline, so they won't catch the B1 or H1 interactions. The tests would pass in isolation while failing in production.
- **ac-07:** Profile eval is the correct integration test. It would catch B1 and H1 failures -- the 8 target columns would not flip. This is the safety net, but it's a late discovery point.
- **Missing:** No test verifies that R25 fires before R12 (or that R12 is modified to not reconfirm postal_code for status codes). No test verifies that R28/R29 survive the header hint override.

## Honest Assessment

The spec's target (223/227, 98.2%) is achievable and the 6 rules are well-justified by the audit findings. The rule logic itself is clean -- each rule checks a clear value pattern. Decision 0048's "value-based rules only" principle is sound.

However, the spec has a fundamental structural problem: it assumes value_sharpen is the right place for all 6 rules, but the pipeline architecture makes this incorrect for R25, R28, and R29. This is the same class of problem that was discovered during v14 implementation for the country_code guard (v14 spec review finding A1/F1). The lesson from that finding -- "value_sharpen corrections get overwritten by header hints" -- was not fully applied here.

The fixes are straightforward:
1. R28, R29 -> post-hint guards (like country_code guard)
2. R25 -> place before R12, or modify R12 to not confirm postal_code for 3-digit-only columns
3. value_sharpen return type -> add confidence, or switch to &mut ColumnResult

These are design corrections, not scope changes. The spec's goals and rules are correct; only the placement needs revision.

## Verdict: REVISE

The spec correctly identifies what to fix and how to detect it. But 3 of 6 rules (R25, R28, R29) will not work as specified due to rule ordering and header hint interactions. The confidence return type issue (H2) affects all 6 rules. These are fixable without scope change, but the spec must be updated before implementation to avoid the same discovery-during-implementation cycle that happened with the country_code guard in v14.

**Required changes:**
1. Move R28 and R29 from value_sharpen to post-hint guards in apply_header_sharpen
2. Address R25/R12 ordering conflict
3. Fix value_sharpen return type or remove confidence-setting from the spec
4. Update the constraint "Rules go in value_sharpen()" to allow post-hint guards for subtype disambiguation
5. Add R27 year range check (1900-2100) to avoid false positives on non-year 4-digit values
