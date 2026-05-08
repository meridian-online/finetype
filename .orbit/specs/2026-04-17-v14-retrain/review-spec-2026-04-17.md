# Spec Review: v14 Retrain

**Spec:** `.orbit/specs/2026-04-17-v14-retrain/spec.yaml`
**Reviewer:** Nightingale (context-separated)
**Date:** 2026-04-17
**Verdict:** CONDITIONAL PASS -- one critical ordering bug must be fixed before implementation

---

## 1. Assumption Audit

### A1: The country_code value_sharpen rule will survive the header hint pipeline -- INVALID

**This is the critical finding.**

The spec (ac-05) proposes adding a value_sharpen rule that overrides `country` to `country_code` when >=95% of values match `^[A-Z]{2,3}$`. The pipeline order is:

```
multi-branch -> feature_sharpen -> value_sharpen (Step 4) -> header hints (Step 5)
```

The audit's items #4 and #5 have the header `location.country`, which triggers the hardcoded hint `"country" => geography.location.country` (line 3929 of column.rs). If value_sharpen changes the label to `country_code`, the same-category hardcoded hint override (line 2252-2268) will immediately change it back:

- Hint category: `geography.location` (from `rsplitn(2, '.')` on `geography.location.country`)
- Result category: `geography.location` (from `rsplitn(2, '.')` on `geography.location.country_code`)
- They match. `hint_is_hardcoded` is true. Override fires unconditionally.

**The value_sharpen rule as specified will have zero effect on the two columns it's designed to fix.** The spec must either:

1. Move the country_code check into `apply_header_sharpen` (after the hint is resolved but before it overrides), or
2. Add `country_code` to the `LOCATION_TYPES` array so the same-domain geo override fires first (which has the `hint_is_hardcoded` escape), blocking the same-category override, or
3. Add an exception to the same-category override: skip when the result was set by a value_sharpen rule with higher confidence (would require plumbing the rule source through), or
4. Change the hardcoded hint from `"country" => country` to `"country" => country_code` (wrong -- breaks actual country name columns).

**Recommendation:** Option 1. Add a post-hint guard: after `apply_header_sharpen`, if the label is `country` and >=95% of values match `^[A-Z]{2,3}$`, override to `country_code`. This respects the pipeline ordering and fires last.

### A2: Subtype decontamination is achievable via the current data pipeline -- PARTIALLY VALID

The spec says to update `prepare_multibranch_data.py` for subtype decontamination (ac-01), but the current `filter_distilled_columns` function only handles *dropping* bad rows (ssn, user_agent) and *pattern-filtering* (phone, postal). Subtype decontamination is different: it requires removing values from *parent type* distilled columns that match a *child type* pattern. For example, removing `data:` URIs from `url` columns, removing `Name <email>` from `email` columns.

This is a new capability. The existing filter framework can be extended, but the spec doesn't acknowledge the implementation gap. The `url`, `email`, and `phone_number` distilled data needs per-value filtering (not per-column dropping), which is a different code path.

### A3: `^[A-Z]{2,3}$` correctly targets country codes without false positives -- MOSTLY VALID, WITH CAVEATS

The taxonomy defines `country_code` as strictly `^[A-Z]{2}$` (ISO 3166-1 alpha-2). The spec's rule uses `^[A-Z]{2,3}$` which also matches 3-letter strings. This is fine for the audit's two columns (all 2-letter codes), but introduces risk:

- `state_code` is also `^[A-Z]{2}$` -- the rule would match state codes too. If the model ever predicts `country` for a state column, the rule would "fix" it to `country_code` when it should be `state_code`. This is an edge case today (the model doesn't confuse country with state_code), but it's worth noting.
- 3-letter matches could collide with `currency_code` (USD, EUR, GBP are 3-letter uppercase). The rule only fires when predicted=`country`, so this is safe for now, but the 3-letter extension is unnecessary given the taxonomy only defines alpha-2.

**Recommendation:** Tighten to `^[A-Z]{2}$` to match the taxonomy definition, or validate against the actual ISO 3166-1 enum list.

### A4: 50 training examples per subtype is sufficient -- PLAUSIBLE BUT UNVERIFIED

AC-03 verification says ">=50 examples each in synthetic data" for json and user_agent. The v13 spec used 1200 cols/type. 50 synthetic examples for a 240-class model is an order of magnitude below the standard training volume. This may produce correct predictions at low confidence rather than fixing the problem.

### A5: The "no architecture changes" constraint is safe -- VALID

Reusing v13's architecture (5-branch, valid_hidden [192, 128], 240 classes) eliminates architecture risk. The only code change is the country_code rule.

### A6: The same config parameters (50 epochs, 1200 cols/type, 70/30 blend) are optimal for v14 -- REASONABLE

These worked for v13. The data changes are incremental. No reason to change hyperparameters.

---

## 2. Failure Mode Analysis

### F1: Country_code rule undone by header hints (CRITICAL, P0)

As described in A1. If not addressed, items #4 and #5 will remain broken despite the rule being implemented and passing unit tests (which don't run the full pipeline including header hints).

**Likelihood:** Certain
**Impact:** 2 of 15 misclassifications guaranteed to remain unfixed
**Mitigation:** Fix the ordering as described in A1

### F2: Subtype decontamination introduces new regressions (MEDIUM, P1)

Removing `data:` URIs from URL training data, `Name <email>` from email training data, and E.164 from phone training data reduces parent type diversity. If the filters are too aggressive, the model may lose accuracy on edge cases:

- URLs with `data` in the path (not data URIs but containing "data")
- Emails with angle brackets in display names (non-standard but common)
- Phone numbers that happen to match E.164 format but with context

**Likelihood:** Low-medium
**Impact:** Potential regression on currently-correct columns
**Mitigation:** The spec's "no regressions" exit condition (line 131) catches this, but only after the full training run. A pre-training data audit step would catch it earlier.

### F3: JSON training data quality (MEDIUM, P1)

AC-03 says to add "JSON-as-string training examples" but doesn't specify how. If the synthetic generator produces unrealistic JSON (e.g., deeply nested, huge objects), the model may learn wrong patterns. The current generator for `json` in the taxonomy needs checking -- if it exists, the spec should reference it. If it doesn't, creating one is not just a data pipeline change but a generator change (listed as a deliverable but not an AC).

**Likelihood:** Medium
**Impact:** Item #12 (geojson -> json) may not be fixed
**Mitigation:** Verify the json generator exists and produces CSV-typical JSON strings

### F4: Hash/TSID separation by length is incomplete (LOW, P2)

AC-02d says to separate tsid (32-char hex) from hash (40/64-char hex). But MD5 hashes are also 32-char hex. The training data would need to either: (a) accept that MD5 hashes may classify as tsid (acceptable if tsid and md5 aren't in the taxonomy separately), or (b) add MD5 as a confounding type.

**Likelihood:** Low
**Impact:** hash vs tsid disambiguation may not fully resolve
**Mitigation:** Document the MD5 overlap as a known limitation

### F5: Silent data pipeline corruption (MEDIUM, P1)

The spec relies on modifying `prepare_multibranch_data.py` for subtype decontamination. The current filtering operates at column level (drop entire columns). Subtype decontamination requires value-level filtering within columns. If the filter removes too many values from a column, the remaining column may have too few examples (<5 values) and be silently dropped by the `--min-values` guard.

**Likelihood:** Medium (depends on contamination prevalence in distilled data)
**Impact:** Parent types could lose significant training volume
**Mitigation:** Add a logging step that reports before/after column counts per type. The spec's verification for ac-01 ("grep training data output for contamination patterns") partially covers this but doesn't mandate volume preservation checks.

---

## 3. Test Adequacy

### Strong points

- AC-05 verification explicitly requires a unit test for the country_code rule, including positive (ISO codes) and negative (country names) cases.
- AC-07 verification requires item-by-item comparison against the v13 audit.
- The "no regressions" exit condition is well-specified.

### Gaps

**T1: No integration test for the country_code rule through the full pipeline.**
AC-05 verification says "cargo test -p finetype-model includes test for the new rule." But unit tests for `value_sharpen` don't exercise `apply_header_sharpen`. The rule will pass unit tests but fail in production due to F1. The spec needs a test that runs the full `classify_multi_branch` pipeline with header="country" and values=["AU", "US", "GB"] and asserts the output is `country_code`.

**T2: No pre-training data validation step.**
The spec has verification for each data change (ac-01 through ac-04) but relies on post-hoc grepping. There's no automated gate that runs before training starts to confirm data integrity. If the overnight script has a bug in decontamination, you'll discover it only after a full training run.

**T3: No regression baseline snapshot.**
AC-07 says "no regressions on columns that v13 got correct" but doesn't specify how to detect regressions. The eval pipeline outputs a CSV of predictions. The spec should mandate diffing v14 predictions against v13 predictions column-by-column, not just checking the aggregate number.

**T4: Subtype decontamination coverage is untested.**
AC-01 says "parent type training data excludes these patterns" but doesn't specify a threshold. If 1 out of 5000 URL columns still contains a data URI after filtering, is that a pass or fail? The spec should specify: zero contamination in synthetic data, and <1% contamination in distilled data (or whatever the acceptable threshold is).

---

## 4. Gap Analysis

### G1: The spec doesn't address the `h.contains("uri")` header hint problem (item #1)

The audit (item #1) explicitly calls out that `header_hint` maps "data_uri" to `url` via the `h.contains("uri")` substring match. The spec only addresses this through retraining (ac-01). But even if retraining fixes the model's prediction, the hardcoded header hint `h.contains("uri")` will still fire on "data_uri" headers and override the model's correct prediction.

Looking at the header_hint function: I need to verify whether `h.contains("uri")` exists. Let me note that the audit says it does but decision 0042 deprecated new regex hints. The fix here is either removing the "uri" substring match (violating the "no new rules" constraint by removing one) or accepting that headers containing "uri" will always map to url.

**Severity:** Medium -- affects item #1 directly

### G2: No mention of `VALID_DIM` staying at 240

The spec says "n_classes 240" in constraints. The data pipeline's `VALID_DIM = 240` constant in `prepare_multibranch_data.py` must match. Since no new types are added (unlike v13 which added state_code), this is implicitly safe, but worth calling out as a constraint check.

### G3: Status code header hint missing from the spec

Items #10 and #15 (postal_code vs integer_number for status codes) are addressed only through retrain (ac-02c). But the audit notes "the header 'status_code' contains 'code' but there's no generic hint for that." The retrain may not fix this because the header signal (containing "code") actively pushes toward postal_code. A header hint for "status" or "status_code" mapping to integer_number would be the clean fix, but conflicts with decision 0042.

The audit recommends a value_sharpen rule as supplementary. The spec omits it, relying purely on retrain. This is a conscious choice (decision 0038) but the risk should be documented.

### G4: Hard-negative mining scope is narrow

AC-04 adds hard-negative decimal_number examples specifically for latitude confusion. But item #8 (amount_accounting vs decimal_number for "gap") is a different confusion vector. The spec addresses #8 through "ensure amount_accounting has formatting" (ac-02b) but doesn't add hard-negative decimal_number examples with accounting-adjacent headers ("amount", "total", "sum").

### G5: User agent dual-failure not fully addressed

Items #7 and #14 are both user_agent misclassifications but with different wrong answers (jwt vs plain_text). The spec addresses this through ac-03b (diverse UA training data). But #7 is at 1.00 confidence predicting JWT -- this suggests the model has a strong false pattern. Simply adding more UA examples may not overcome a 1.00-confidence wrong prediction without also improving JWT negative examples (things that look like JWTs but aren't).

### G6: The target of >=222/227 is aggressive but not analyzed

The spec targets fixing 10+ of 15 misclassifications. But some fixes (subtype decontamination for #1, #2, #3) depend on the model learning a distinction it has never made before, from synthetic data alone (distilled data is contaminated by definition). The audit's confidence for #1, #2, #3 is 1.00 -- the model is maximally confident in the wrong answer. Flipping a 1.00-confidence prediction requires strong new training signal.

---

## 5. Constraint Check

### C1: "No architecture changes" -- SATISFIED
Only a new value_sharpen rule (ac-05) modifies Rust code. Model config is unchanged.

### C2: "Training data curation only" -- MOSTLY SATISFIED
AC-05 (country_code rule) is explicitly called out as the exception. Consistent with decision 0038's "last resort" clause.

### C3: "Same parameters as v13" -- SATISFIED
50 epochs, 1200 cols/type, 70/30 blend, Metal, distilled cap 600/type all match v13.

### C4: "models/default symlink stays on sherlock-v13" -- SATISFIED
Explicitly stated in constraints.

### C5: "All 15 audit items addressed" -- SATISFIED IN INTENT
Every audit item maps to at least one AC. The "even if not all are fixable by retrain, document why" clause provides appropriate escape valve.

### C6: Decision 0038 compliance -- SATISFIED
Rules are a last resort. Only one rule added (country_code), with clear justification (deterministic pattern).

### C7: Decision 0042 compliance -- SATISFIED
No new regex header hints added. The existing "uri" hint problem (G1) is not addressed but also not worsened.

---

## Summary

```
| Category           | Items | Critical | Notes                                                    |
|--------------------|-------|----------|----------------------------------------------------------|
| Assumptions        | 6     | 1        | A1: country_code rule ordering bug                       |
| Failure modes      | 5     | 1        | F1: certain failure, F2/F3/F5: medium risk               |
| Test gaps          | 4     | 1        | T1: no integration test through full pipeline             |
| Spec gaps          | 6     | 1        | G1: uri header hint actively hurts item #1                |
| Constraint checks  | 7     | 0        | All satisfied                                             |
```

### Verdict: CONDITIONAL PASS

The spec is well-structured, grounded in a thorough audit, and follows established patterns from v13. The data curation approach is sound.

**Must fix before implementation:**

1. **A1/F1: Country_code rule ordering.** The value_sharpen rule will be undone by `apply_header_sharpen`'s same-category override. Either move the check to a post-hint position or add a same-category exception. This must have an integration test (T1) that exercises the full pipeline with header="country" and 2-letter code values.

**Should fix:**

2. **G1: Document or mitigate the `uri` header hint.** Even with perfect retraining, `header_hint("data_uri")` may still fire and override to `url`. Verify whether this substring match exists and, if so, decide whether to remove it (cleanup, not new rule) or accept item #1 as partially blocked.

3. **T2: Add a pre-training data audit script.** Run contamination checks and volume checks before kicking off the overnight training run. Discovering a pipeline bug after 8 hours of training wastes a night.

4. **T3: Mandate column-level regression diff.** Don't just check the aggregate number -- diff every column prediction between v13 and v14 to catch label swaps (where one fix causes a different regression).

**Nice to have:**

5. **A3: Tighten the regex to `^[A-Z]{2}$`** to match the taxonomy's alpha-2 definition.
6. **G4: Add accounting-adjacent hard negatives** for the decimal_number confusion.
7. **G5: Add JWT negative examples** alongside positive UA examples to break the 1.00-confidence false pattern.
