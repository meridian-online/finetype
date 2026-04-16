# Confidence Analysis: Persistent Errors with Increased Confidence (ac-04)

**Date:** 2026-04-16
**Model:** sherlock-v12 (5-branch: char+embed+stats+header+validation)

## Summary

6 persistent errors became more confident in v12 compared to v11. The validation branch (239-dim type pass rate vector) provides correct signal in 4 of 6 cases — the model ignores it. In 1 case (IATA/http_method), the validation partially reinforces the wrong prediction. In 1 case (geohash/alphanumeric_id), the validation is ambiguous.

## Methodology

For each of the 6 errors, I examined:
1. The taxonomy validation regex for both the predicted and expected types
2. Which values from the eval dataset would pass each type's validation
3. Whether the validation branch is providing correct, misleading, or neutral signal

Validation patterns were extracted from the taxonomy via `finetype taxonomy --full --output json`.

---

## Case 1: tech_systems / user_agent — jwt conf 0.37→1.00 (+0.63)

**This is the worst confidence regression: the model went from uncertain to maximally confident in a completely wrong prediction.**

**Values:** `Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15) AppleWebKit/537.36 ...`

**Validation analysis:**

```
| Type        | Validation Pattern                                        | Pass Rate |
|-------------|-----------------------------------------------------------|-----------|
| jwt         | ^[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]  | 0%        |
| user_agent  | (none)                                                    | N/A       |
```

- **JWT validation fails:** User agents contain spaces, parentheses, semicolons — none of which are allowed in the JWT regex. The validation branch should show pass_rate=0% for jwt.
- **user_agent has no validation:** The validation branch provides no positive signal for the correct type.

**Conclusion:** The validation branch correctly shows jwt=fail, user_agent=no_signal. The confidence increase is NOT from validation — it's from the char/embed/stats branches shifting weights during v12 retraining. The validation branch is neutral (no signal for either type) rather than corrective.

**Recommendation:** Add a user_agent validation pattern (e.g., must contain `/` and one of `Mozilla`, `curl`, `python-requests`, etc.) so the validation branch can provide positive signal.

---

## Case 2: earthquakes_2024 / depthError — latitude conf 0.61→1.00 (+0.39)

**Values:** `7.431, 8.251, 11.011, 1.825, 9.663`

**Validation analysis:**

```
| Type           | Validation Pattern    | Pass Rate |
|----------------|-----------------------|-----------|
| latitude       | (none)                | N/A       |
| decimal_number | ^-?[0-9]+(\.[0-9]+)? | 100%      |
```

- **decimal_number validation passes:** All values match the decimal regex perfectly.
- **latitude has no validation:** No pass/fail signal for latitude.

**Conclusion:** The validation branch should be HELPING here — decimal_number gets strong positive signal, latitude gets none. Yet the model became maximally confident in latitude. The char/embed/stats branches dominate with enough weight to override the validation signal. This means the validation branch's hidden layer learned to discount the decimal_number pass signal in contexts where other branches strongly favour latitude.

**Recommendation:** Add latitude validation pattern (e.g., `^-?([0-8]?\d(\.\d+)?|90(\.0+)?)$`) so the validation branch can at least provide a competing signal. Currently latitude validation is a gap.

---

## Case 3: server_logs_json / method — iata_code conf 0.29→0.61 (+0.32)

**Values:** `GET, POST, DELETE, PUT, PATCH, OPTIONS`

**Validation analysis:**

```
| Type        | Validation Pattern | Pass Rate | Matching Values |
|-------------|--------------------|-----------|-----------------|
| iata_code   | ^[A-Z]{3}$         | 33%       | GET, PUT        |
| http_method | (none)             | N/A       |                 |
```

- **IATA validation partially passes:** `GET` and `PUT` are 3 uppercase letters, matching the IATA airport code pattern. 2/6 values = 33% pass rate.
- **http_method has no validation:** No positive signal for the correct type.

**Conclusion:** The validation branch is REINFORCING the wrong prediction. IATA's 33% pass rate provides a positive signal, while http_method gets nothing. The model correctly identifies 3-letter uppercase codes as matching the IATA pattern — the problem is that GET/PUT happen to be valid 3-letter uppercase strings.

**Recommendation:** Add http_method validation pattern (e.g., `^(GET|POST|PUT|DELETE|PATCH|HEAD|OPTIONS|CONNECT|TRACE)$`) to give the validation branch a competing signal. With this pattern, 100% of values would pass http_method validation vs 33% for iata_code.

---

## Case 4: network_logs / status_code — postal_code conf 0.69→0.99 (+0.29)

**Note:** Prediction changed from bsb (v11) to postal_code (v12).

**Values:** `400, 500, 301, 403, 503, 401, 204, 404`

**Validation analysis:**

```
| Type           | Validation Pattern    | Pass Rate |
|----------------|-----------------------|-----------|
| bsb            | ^\d{3}-\d{3}$        | 0%        |
| postal_code    | (none)                | N/A       |
| integer_number | ^-?[0-9]+$           | 100%      |
```

- **BSB validation fails:** No dashes in the values. This is why the prediction changed from bsb to postal_code.
- **postal_code has no validation:** The universal validation is locale-specific only.
- **integer_number passes 100%:** Strong positive signal for the correct type.

**Conclusion:** The validation branch successfully eliminated the v11 wrong answer (bsb fails validation → eliminated). But the model then selected the next-wrong answer (postal_code) instead of the correct one (integer_number). Partial success — the validation branch is working as designed for bsb elimination, but the char/embed branches' preference for postal_code overrides the integer_number validation signal.

**Recommendation:** Consider adding postal_code locale-specific validation patterns to make the validation branch more discriminative for numeric codes.

---

## Case 5: codes_and_ids / sha256 — ethereum_address conf 0.96→0.97 (+0.01)

**Values:** `1abd775a8e661366c67807273a7bd0fdcd70048964cf985b4d4a1668b391dacb` (64 hex chars)

**Validation analysis:**

```
| Type             | Validation Pattern                          | Pass Rate |
|------------------|---------------------------------------------|-----------|
| ethereum_address | ^0x[a-fA-F0-9]{40}$                         | 0%        |
| hash             | ^[0-9a-f]{32}$\|^[0-9a-f]{40}$\|^[0-9a-f]{64}$ | 100%  |
```

- **Ethereum validation fails:** No `0x` prefix, wrong length (64 not 40).
- **Hash validation passes:** 64-char lowercase hex matches the SHA-256 alternative.

**Conclusion:** Validation provides CORRECT, STRONG signal — hash=100%, ethereum_address=0%. Yet the model prediction barely changed (0.96→0.97). The char/embed branches are completely dominating and the validation branch signal is ignored for hex strings. This is the clearest case of the validation branch being overridden.

**Recommendation:** This is a model-level issue. The validation branch is providing perfect information but the merge layer's learned weights suppress it for hex-string contexts.

---

## Case 6: earthquakes_2024 / id — geohash conf 0.99→1.00 (+0.01)

**Values:** `us6000pgkh, us6000pgkd, us6000pj75`

**Validation analysis:**

```
| Type           | Validation Pattern                                   | Pass Rate |
|----------------|------------------------------------------------------|-----------|
| geohash        | ^[0-9b-hjkmnp-z]{4,12}$                              | 100%      |
| alphanumeric_id| ^[A-Za-z].*[0-9].*$\|^[0-9].*[A-Za-z].*$            | 100%      |
```

- **Geohash validation passes:** All characters in `us6000pgkh` are in the base32 set (0-9, b-z excluding a/i/l/o). Length 10 is in [4,12].
- **Alphanumeric_id validation also passes:** Mixed letter+digit pattern matches.

**Conclusion:** Both validations pass — the validation branch is ambiguous. It cannot discriminate between geohash and alphanumeric_id for these values. The geohash validation pattern is too permissive: it matches any lowercase alphanumeric string of 4-12 characters using the base32 character set, which includes many non-geohash strings.

**Recommendation:** Tighten geohash validation — real geohashes have more uniform character distribution across the base32 alphabet. Or add a negative signal: USGS earthquake IDs always start with a 2-letter country code followed by digits, which is not a typical geohash pattern.

---

## Cross-Case Findings

```
| Case | Validation Signal | Model Response | Assessment                    |
|------|-------------------|----------------|-------------------------------|
| 1    | Correct (neutral) | Ignored        | Validation can't help — no pattern for correct type |
| 2    | Correct (strong)  | Ignored        | Validation overridden by char/embed branches         |
| 3    | Misleading        | Reinforced     | IATA validation false-matches GET/PUT                |
| 4    | Partially correct | Partially used | Eliminated bsb, but not postal_code                  |
| 5    | Correct (strong)  | Ignored        | Strongest signal, completely overridden               |
| 6    | Ambiguous          | N/A            | Both types pass validation                            |
```

### Key Takeaways

1. **The validation branch is providing correct signal in most cases but lacks the weight to override char/embed branches.** Cases 2 and 5 show perfect validation signal being ignored. The merge layer may need rebalancing — or the validation branch's hidden dimensions (128→64) may be too small relative to char (960→450→450) and embed (512→300→300).

2. **Missing validation patterns for 3 correct types (user_agent, http_method, latitude) prevent the validation branch from helping.** Adding these patterns would give the model more information to work with.

3. **One overly permissive validation (geohash) creates false matches.** The geohash pattern matches far too many lowercase alphanumeric strings.

4. **The validation branch's best demonstrated value is elimination.** Case 4 (bsb eliminated by validation failure) shows the branch working as intended. Negative signal (fail) is more reliably used than positive signal (pass).
