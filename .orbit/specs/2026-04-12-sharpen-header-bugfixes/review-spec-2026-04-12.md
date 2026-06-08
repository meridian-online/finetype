# Spec Review

**Date:** 2026-04-12
**Reviewer:** Context-separated agent (fresh session)
**Spec:** .orbit/specs/2026-04-12-sharpen-header-bugfixes/spec.yaml
**Verdict:** REQUEST_CHANGES

---

## Findings

### [CRITICAL] ac-03 threshold change does NOT fix the stated scenario (ssn@1.00 -> phone_number)
**Category:** assumption
**Description:** The spec says raising the same-category threshold from 0.80 to 0.95 will fix `ssn@1.00` being blocked when the header says "phone". This is wrong. At confidence 1.00, the check `result.confidence <= 0.95` still fails. The ssn@1.00 override never fires through the same-category path.

Furthermore, the same-category path is not the only path that could handle this. Tracing through all subsequent code paths for a same-domain, same-category case where `hint_is_hardcoded=true`, `confidence=1.00`:

1. **Same-category (line 2245):** `confidence <= 0.80` (or 0.95) -- fails at 1.00
2. **Cross-domain (line 2264):** `hint_domain != pred_domain` -- both `identity`, fails
3. **Hardcoded authority (line 2315-2326):** `h_domain == p_domain` so threshold=0.5, `confidence < 0.5` -- 1.00 >= 0.5, fails

No path overrides ssn@1.00 with the proposed fix. The test in ac-03 verification ("hardcoded phone hint overrides ssn@1.00") will fail.

**Evidence:** Lines 2244-2261 (`apply_header_sharpen`), lines 2263-2283, lines 2315-2326. The `rsplitn(2, '.').last()` extracts `identity.person` for both ssn and phone_number, confirming same-category. But 1.00 > 0.95.
**Recommendation:** Either (a) the spec must explain that same-category hardcoded hints should override unconditionally (no confidence threshold) when `hint_is_hardcoded`, or (b) the test scenario should use a realistic confidence (say 0.90) rather than 1.00, or (c) a new code path is needed for "hardcoded same-category unconditional override" that the spec currently does not describe.

---

### [CRITICAL] ac-03 verification scenarios for year and URL also fail
**Category:** assumption
**Description:** The spec's verification for ac-03 includes two additional scenarios that also do not work with the proposed fix:

**Year overrides compact_ym@0.83:** `hint_category = datetime.component`, `pred_category = datetime.format` -- these are NOT equal, so the same-category path never fires regardless of threshold. The cross-domain path also fails (same domain `datetime`). The hardcoded authority path uses threshold 0.5 for same-domain, and 0.83 >= 0.5, so it also fails.

**URL overrides docker_ref@0.60:** `hint_category = technology.internet`, `pred_category = technology.development` -- NOT equal, same-category does not fire. Same domain `technology`, cross-domain does not fire. Hardcoded authority threshold=0.5 for same-domain, 0.60 >= 0.5, fails.

**Evidence:** Line 2246-2247: `rsplitn(2, '.')` on `datetime.component.year` yields `datetime.component`, on `datetime.format.compact_ym` yields `datetime.format`. These are different categories.
**Recommendation:** The spec must specify which code path(s) actually need to change to handle same-domain-different-category overrides. The same-category threshold change alone only affects cases where hint and prediction share the exact same `domain.category` prefix. A separate mechanism is needed for same-domain overrides -- likely the hardcoded authority path at line 2315-2326, where the same-domain threshold could be raised from 0.5 to something higher (e.g. 0.95).

---

### [MAJOR] ac-04 wrong taxonomy label for ICAO
**Category:** missing-requirement
**Description:** The spec says `"icao" -> technology.transport.icao_code` but the actual taxonomy label is `geography.transportation.icao_code`. The domain is `geography`, not `technology`, and the category is `transportation`, not `transport`.
**Evidence:** `labels/definitions_geography.yaml:1032` defines `geography.transportation.icao_code`. The eval mapping at `eval/schema_mapping.yaml:228-234` confirms: `gt_label: icao, finetype_label: geography.transportation.icao_code`. The existing codebase at `column.rs:2379` references `geography.transportation.icao_code` in `CODE_ATTRACTORS`.
**Recommendation:** Fix the label in the spec to `geography.transportation.icao_code`.

---

### [MAJOR] ac-04 "method" hint risks regression on network_logs.method
**Category:** failure-mode
**Description:** The spec proposes `"method" -> technology.internet.http_method`. The eval manifest contains two "method" columns: `server_logs_json.method` (expected: http_method) and `network_logs.method` (expected: category). A bare "method" hint would force http_method on both.

Currently, `http_method` happens to be listed as an acceptable alternative under the "category" ground truth mapping (schema_mapping.yaml:877), so it would not show as a regression in the eval score today. But this is a semantic error -- forcing a specific type on a column whose ground truth is the generic "category" depends on a coincidental mapping entry. If the schema_mapping is ever tightened, this becomes a visible regression.

More importantly, outside the eval set, "method" is genuinely ambiguous (payment method, scientific method, cooking method). Hardcoding it to http_method contradicts the Precision Principle in CLAUDE.md: "A validation that confirms 90% of random input is not a validation."
**Evidence:** `eval/datasets/manifest.csv:167` (`network_logs,method,category`), `eval/schema_mapping.yaml:870-882` (category accepts http_method).
**Recommendation:** Restrict the hint to `"http method"` only. Remove bare `"method"` from the hint. The server_logs_json case has header "method" but values are `GET, POST, DELETE, PUT` -- the model should learn this from values + sibling context, not a hardcoded header rule. Alternatively, note that the parent spec's eval audit (Case 22) already classified this as DEBATABLE and added categorical as acceptable for http_method -- meaning the model's current categorical prediction is already counted as correct.

---

### [MAJOR] ac-04 "port" hint was previously removed -- needs justification
**Category:** constraint-conflict
**Description:** The `port` hint was deliberately removed, along with the removal of the `port` type from the taxonomy. The CHANGELOG documents this: "Removed 2 low-precision integer-range types: http_status_code and port (false positives on plain integers)." Re-adding a `port` -> `integer_number` hint reverses that decision without explicit justification or a new decision record.

Additionally, "port" as a column header could refer to a seaport (city name), a wine port, or a computing port. The hint to integer_number is not universally correct.
**Evidence:** `CHANGELOG.md:98`, `column.rs:5913` ("port header hint removed"), `.orbit/choices/0023-taxonomy-pruning-principle.md:26`.
**Recommendation:** Either (a) add a decision record explaining why re-adding the port hint is justified now, or (b) remove "port" from ac-04's list. The eval mapping already maps port ground truth to integer_number, so if the model predicts integer_number without the hint, it already passes.

---

### [MODERATE] Adding 5 new hardcoded hints conflicts with decision 0042 direction
**Category:** constraint-conflict
**Description:** Decision 0042 ("Remove regex header hints in favour of learned approaches") explicitly says: "Regex-based header_hint() and hardcoded header rules are deprecated in favour of learned approaches." CLAUDE.md's architectural direction section confirms: "No more regex rabbit holes." This spec adds 5 new hardcoded hints, moving in the opposite direction.

The spec's constraint "only Sharpen changes" frames this as bugfix work, but adding new hints is not a bugfix -- it is new heuristic development. The architectural direction says to add these patterns to training data instead.
**Evidence:** `.orbit/choices/0042-remove-regex-header-hints.md`: "Remove regex header hints, rely on learned approaches." CLAUDE.md: "Regex-based header_hint() and hardcoded header rules are deprecated."
**Recommendation:** Acknowledge the tension with decision 0042 explicitly. These hints should be framed as temporary stopgaps until the next retraining cycle absorbs them. Consider whether only the highest-impact hints (icao, author) are worth adding, given that method/port/status_code are either ambiguous or previously removed.

---

### [MODERATE] ac-05 target of >= 205/227 may be unreachable from bug fixes alone
**Category:** assumption
**Description:** The spec says 18 of 24 misclassifications are "Sharpen-fixable," but the ac-03 threshold change (the "biggest single lever") does not actually work as described. If ac-03's mechanism is wrong, the projected gain of +2 or more to reach 205/227 is not substantiated. The spec needs to identify exactly which of the 24 misclassifications each AC fixes, and verify the code path for each.
**Evidence:** The parent spec's progress.md shows v11 at 203/227. The gap is only 2. But the core mechanism (ac-03) is broken in the spec as written.
**Recommendation:** Add a table mapping each of the 24 v11 misclassifications to the specific AC that fixes it. For ac-03, work through the actual code path end-to-end for each claimed fix.

---

### [MINOR] ac-02 verification says "source_ip" still returns ip_v4, but this is tested via exact match
**Category:** test-gap
**Description:** The spec says `header_hint("source_ip") still returns ip_v4` as regression verification. In the current code, "source ip" (after underscore normalization) is in the exact match arm at line 3830. Adding a v6 check before the substring matching at line 3998-4001 would not affect this exact match. The verification is testing the right thing but not the risky thing -- the real regression risk is headers like "ipv6_address" that contain both "v6" and match the "ip" substring pattern.
**Evidence:** Lines 3830-3832 (exact match for "source ip"), lines 3998-4001 (substring match).
**Recommendation:** Add explicit regression tests for edge cases like "ipv6_enabled", "ipv6_ready" (should these return ip_v6 or None?), and "server_ipv6" to ensure the v6 check has proper boundaries.

---

### [MINOR] ac-04 "status code" / "http status" -> integer_number is semantically wrong
**Category:** assumption
**Description:** HTTP status codes (200, 404, 500) are better described as categorical/ordinal values, not arbitrary integers. `integer_number` implies the value can be any integer, but HTTP status codes are from a fixed enumeration. The parent spec's eval audit removed `http_status_code` as a type because it had "false positives on plain integers" -- but the fix should be `categorical` or `ordinal`, not `integer_number`. Check what the eval mapping says.
**Evidence:** The `http_status_code` type was removed specifically because it was an "integer-range type with no distinguishing character patterns." Mapping it to `integer_number` does not capture the categorical nature of status codes.
**Recommendation:** Consider whether `representation.discrete.ordinal` or `representation.discrete.categorical` would be a more accurate hint target. Or simply drop this hint -- it was removed for a reason.

---

## Honest Assessment

This spec is not ready for implementation. The core mechanism (ac-03, the same-domain threshold change) is broken -- the proposed threshold of 0.95 does not fix any of the three verification scenarios because the code paths the spec claims to leverage either (a) still reject at high confidence, or (b) operate on category equality which does not hold for the claimed examples. The spec needs to re-derive the actual code changes required by tracing through `apply_header_sharpen()` for each target misclassification. The bugfixes in ac-01 and ac-02 are sound and well-motivated. Three of the five new hints in ac-04 have issues (wrong taxonomy label for icao, ambiguous bare "method", previously-removed "port"). The biggest risk is that implementing the spec as written will not reach the 205/227 target, because the mechanism is wrong, leading to rework.
