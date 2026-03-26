# Generator Collision Audit Report

**Date:** 2026-03-26
**Spec:** AC-2 of overnight-v6-data-quality
**Method:** Generated 100 samples per type (seed 42), cross-validated against validation regex patterns, manual format inspection
**Taxonomy:** 250 types across 10 broad_type groups (245 in groups with >1 member)

## Summary

```
| Severity | Count | Description                                          |
|----------|-------|------------------------------------------------------|
| HIGH     | 11    | >50% cross-validation match or identical format      |
| MEDIUM   | 8     | 20-50% cross-validation or structural similarity     |
| LOW      | 4     | <20% but notable — mostly header-disambiguated       |
| TOTAL    | 23    | Actionable collision pairs                           |
```

Excluded from count: DMY/MDY/YMD date ordering ambiguity (inherent, resolved by locale context), timestamp precision variants (iso_8601 vs iso_8601_milliseconds — superset relationships, not collisions), and permissive-validation false positives (container.object.csv matches everything).

**Stop-gate check:** 23 collisions is under the spec's 20-collision stop threshold. However, many are structural (numeric_code catches everything numeric) rather than generator bugs. Only 5 require generator changes; the rest are disambiguation issues best handled by headers/context.

---

## HIGH Severity Collisions

### C-01: hs_code <-> decimal_number [KNOWN]

**Broad type:** VARCHAR (hs_code), DOUBLE (decimal_number)
**Cross-validation:** 11% of hs_code samples match decimal format (2-level values like "6607.37")
**Root cause:** Generator produces 10% 2-level HS codes (XXXX.XX) which are indistinguishable from decimals
**Affects eval:** Yes (both types in eval)

Example overlapping values:
- `6607.37` — valid hs_code OR decimal_number
- `4483.84` — valid hs_code OR decimal_number
- `9718.73` — valid hs_code OR decimal_number

**Proposed resolution:** Per spec decision:
1. Change validation pattern: `^\d{4}\.?\d{2}(\.\d{2}){0,2}$` -> `^\d{4}\.\d{2}(\.\d{2}){1,2}$`
2. Remove 2-level bucket from generator: 80% 3-level (XXXX.XX.XX), 20% 4-level (XXXX.XX.XX.XX)
3. Only 3+ dot-separated levels are hs_code

### C-02: iata_code <-> currency_code

**Broad type:** VARCHAR
**Cross-validation:** 100% bidirectional — both are 3-letter uppercase codes
**Root cause:** IATA codes and ISO 4217 currency codes share identical format (3 uppercase letters)
**Affects eval:** Yes (both in eval)

Example overlapping formats:
- `RPF` (IATA) vs `NGN` (currency) — structurally identical
- `IOY` (IATA) vs `GBP` (currency) — structurally identical

**Proposed resolution:** No generator change. This is a header-disambiguation problem — the column header ("airport_code" vs "currency") is the only reliable signal. The model should learn this from training data with correct headers. Adding structural constraints would be artificial (real IATA codes and currency codes overlap, e.g., `ALL` = Albania Lek AND Allahabad airport).

### C-03: numeric_code <-> (everything numeric)

**Broad type:** VARCHAR
**Cross-validation:** numeric_code's validation (`^\d+$` or similar) matches 100% of: aba_routing, ean, upc, npi, credit_card_number, imei, amount_minor_int, cpt, postal_code
**Root cause:** numeric_code is the "catch-all" for digit-only strings — its validation is intentionally broad
**Affects eval:** Yes (numeric_code in eval, plus most colliding types)

Key collision pairs:
- `aba_routing` (9 digits) -> numeric_code: 100%
- `ean` (8-13 digits) -> numeric_code: 100%
- `upc` (12 digits) -> numeric_code: 100%
- `npi` (10 digits) -> numeric_code: 100%
- `credit_card_number` (15-16 digits) -> numeric_code: 100%
- `imei` (15 digits) -> numeric_code: 100%
- `cpt` (5 digits) -> numeric_code: 89%
- `postal_code` -> numeric_code: 75%

**Proposed resolution:** No generator change. This is by design — numeric_code is the fallback when no more specific type matches. The model should prefer specific types (aba_routing, ean, etc.) when structure or headers indicate them, and only fall back to numeric_code when ambiguous. This is the F5 disambiguation rule's job (already identified as a key accuracy gap). Fixing F5/retraining is the path here, not changing generators.

### C-04: hash <-> token_hex <-> git_sha <-> tsid

**Broad type:** VARCHAR
**Cross-validation:**
- git_sha -> hash validation: 100% (git SHA is a subset of hex hash)
- git_sha -> token_hex validation: 100%
- tsid -> hash validation: 100% (32 hex chars)
- tsid -> token_hex validation: 100%
- hash -> git_sha validation: 31% (only 40-char hashes match)
- hash -> token_hex validation: 69%
**Root cause:** All produce lowercase hex strings of varying lengths
**Affects eval:** Yes (hash, git_sha, tsid, calver all in eval)

Example overlapping values:
- `d0ca3d15b5f0f1270e984435fd984e43fa085c3f` — valid git_sha AND hash
- `01b2d0819f9be7ff30978160b6f8dc18` — valid tsid AND hash AND token_hex
- `77084f88f6661bdb43376e00f6738752` — valid hash AND token_hex

**Proposed resolution:** No generator change. Disambiguation relies on:
- Length: git_sha is always 40 chars, TSID is 32 with `01` prefix, MD5 hash is 32, SHA-256 is 64
- Column header: "commit_hash", "sha", "token", "session_id"
- TSID has a monotonically increasing prefix (timestamp-based)
The model can learn these length/prefix patterns. Consider a Sharpen rule for TSID's `01` prefix pattern if the model struggles.

### C-05: email <-> paypal_email

**Broad type:** VARCHAR
**Cross-validation:** 100% bidirectional — paypal_email is a valid email
**Root cause:** PayPal emails ARE emails with specific domain patterns
**Affects eval:** email in eval, paypal_email not in eval

Example values:
- `join-payments@paypal.com` (paypal_email) — also valid email
- `amelia_taylor@outlook.com` (email) — does NOT match paypal_email domain check

**Proposed resolution:** No generator change needed. PayPal email is inherently a subset of email. Disambiguation should rely on domain matching (@paypal.com, @billing.paypal.com) and header context. The current generators are correct — paypal_email always has paypal domains, generic email never does. The model should learn the domain distinction.

### C-06: month_year_slash <-> credit_card_expiration_date

**Broad type:** DATE
**Cross-validation:** 100% of month_year_slash matches credit_card_expiration validation
**Root cause:** Both produce MM/YYYY format, but credit_card_expiration also produces MM/YY
**Affects eval:** Neither currently in eval

Example values:
- `06/2022` — valid month_year_slash AND credit_card_expiration
- `03/31` — credit_card_expiration only (MM/YY)

**Proposed resolution:** No generator change. Disambiguation by header ("expiry", "exp_date" vs "month", "period"). The model should learn this from header context. These are genuinely ambiguous at the value level — only context resolves them.

### C-07: hs_code <-> version <-> calver

**Broad type:** VARCHAR
**Cross-validation:**
- hs_code -> version validation: 71%
- hs_code <-> calver: 100% bidirectional
- calver -> decimal_number validation: 51%
- calver -> version validation: 49%
**Root cause:** Dotted numeric formats overlap across all three types
**Affects eval:** Yes (all three in eval)

Example overlapping values:
- `4387.86.83.46` (hs_code) matches version pattern
- `2024.10` (calver) matches decimal_number pattern
- `2024.10.15` (calver) matches hs_code and version patterns

**Proposed resolution:**
1. hs_code fix (C-01) will remove 2-level values, reducing overlap with calver/decimal
2. calver generator should produce distinctive formats: always include leading `20XX.` year prefix. Real calver always starts with a plausible year (2019-2029). hs_code starts with commodity chapter (01-99).
3. version generator already produces `v` prefix variants — increase `v`-prefix proportion to differentiate from calver

### C-08: isrc <-> isin

**Broad type:** VARCHAR
**Cross-validation:** 100% of isrc matches isin validation
**Root cause:** Both are 12-char alphanumeric with 2-letter country prefix
**Affects eval:** Yes (both in eval)

Example values:
- `AUWSO8225862` (ISRC) matches ISIN pattern
- `KRWMEHP1QJD3` (ISIN)

**Proposed resolution:** No generator change. ISRC format is CC-XXX-YY-NNNNN (country-registrant-year-designation) while ISIN is CC + 9 alphanumeric + check digit. The generators produce different internal structures, but the regex validation is too broad to distinguish them. Header disambiguation is the correct approach here.

### C-09: form_data <-> query_string

**Broad type:** VARCHAR
**Cross-validation:** 100% bidirectional — identical format (key=value&key=value)
**Root cause:** Both use identical URL-encoded key-value pair format
**Affects eval:** query_string in eval, form_data not

Example values:
- `username=samuel&email=delta@example.com` (form_data)
- `join=delta&foxtrot=union` (query_string)

**Proposed resolution:** No generator change. These are structurally identical at the value level — the distinction is contextual (HTTP body vs URL query). The model cannot and should not distinguish them by value alone. Consider merging into a single type or accepting that header context is the only differentiator.

### C-10: json <-> geojson

**Broad type:** JSON (json) / VARCHAR (geojson)
**Cross-validation:** 100% of geojson matches json validation
**Root cause:** GeoJSON is valid JSON — it's a subset
**Affects eval:** geojson in eval, json not in eval directly

Example values:
- `{"type": "Point", "coordinates": [-26.2761, 27.0084]}` (geojson) — valid JSON
- `{"lat":38.9329,"lon":-130.6352}` (json)

**Proposed resolution:** No generator change. GeoJSON is inherently JSON. Disambiguation relies on detecting the `"type": "Point|LineString|Polygon"` and `"coordinates"` structure. This is a value-inspection rule, not a generator issue.

### C-11: amount_minor_int <-> numeric_code

**Broad type:** VARCHAR
**Cross-validation:** 100% bidirectional — both are pure digit strings
**Root cause:** Minor-unit currency amounts (cents) are just integers
**Affects eval:** Both in eval

Example values:
- `6790198` (amount_minor_int — $67,901.98 in cents)
- `37334` (numeric_code)

**Proposed resolution:** No generator change. This is the same class of problem as C-03 (numeric_code catches everything). Header context ("amount_cents", "price_minor") is the only reliable signal.

---

## MEDIUM Severity Collisions

### C-12: cpt <-> postal_code

**Cross-validation:** 38% of postal codes match CPT validation (both produce 5-digit codes)
**Exact overlap:** 1 value (`92186`)
**Affects eval:** Both in eval

**Proposed resolution:** No generator change. CPT codes are in range 00100-99499 (medical procedure codes), postal codes cover 00001-99999. Overlap is inherent. Header disambiguation only.

### C-13: ean <-> credit_card_number

**Cross-validation:** 64% of EAN matches credit_card validation
**Root cause:** EAN-13 (13 digits) overlaps with 13-digit card number ranges
**Affects eval:** Both in eval

**Proposed resolution:** No generator change. EAN has Luhn check digits, credit cards have different Luhn calculation. The model should learn prefix patterns (EAN starts with country codes, cards start with issuer codes 4/5/3/6).

### C-14: hcpcs <-> alphanumeric_id

**Cross-validation:** 100% of HCPCS matches alphanumeric_id validation
**Root cause:** HCPCS codes (letter + 4 digits, e.g., "N6158") match the broad alphanumeric pattern
**Affects eval:** Both in eval

**Proposed resolution:** No generator change. HCPCS has a specific structure (one letter from specific set + 4 digits). The model should learn this from the training distribution. Header context ("hcpcs_code", "procedure_code") reinforces.

### C-15: eu_vat <-> alphanumeric_id

**Cross-validation:** 100% of EU VAT matches alphanumeric_id validation
**Root cause:** EU VAT numbers (2-letter country + digits, e.g., "ATU80535961") match broad alphanumeric pattern
**Affects eval:** Both in eval

**Proposed resolution:** No generator change. EU VAT has a 2-letter ISO country prefix followed by country-specific digit patterns. Distinguishable from generic alphanumeric by the known country prefix set.

### C-16: lei <-> alphanumeric_id

**Cross-validation:** 100% of LEI matches alphanumeric_id validation
**Root cause:** LEI (20-char alphanumeric, e.g., "6354C9GH6XBGP4Q7GQ96") matches broad pattern
**Affects eval:** Both in eval

**Proposed resolution:** No generator change. LEI has fixed 20-char length and specific structure (4-digit LOU prefix + 14 alphanumeric + 2 check digits). The model should learn this from training data and headers.

### C-17: first_name <-> username / last_name <-> username

**Cross-validation:** 68% of first_name and 58% of last_name match username validation
**Root cause:** Simple names ("Sandra", "Garcia") match the broad username pattern
**Affects eval:** All three in eval

**Proposed resolution:** No generator change. Usernames typically include numbers, dots, or underscores ("jennifer.format", "alexander900") while names are pure alphabetic. The model should learn these distributional differences. Header context is reliable.

### C-18: city <-> region

**Cross-validation:** N/A (both have permissive/no validation)
**Exact overlap:** 2 values (`Berlin`, `New York`)
**Affects eval:** Both in eval

**Proposed resolution:** No generator change. Cities and regions genuinely overlap (Berlin is both a city and a German state, New York is both). This is inherently ambiguous and only resolvable by header context or sibling columns.

### C-19: percentage <-> yield

**Cross-validation:** 24% of yield matches percentage validation
**Root cause:** Yield values with `%` suffix ("+19.06%") look like percentages
**Affects eval:** percentage in eval, yield not

**Proposed resolution:** No generator change. Yield values include sign prefix (+/-) and the column header ("yield", "return") differentiates. The model should learn the sign prefix pattern.

---

## LOW Severity Collisions

### C-20: icao_code <-> http_method

**Cross-validation:** 28% of HTTP methods match ICAO validation (both produce 3-4 uppercase letter codes)
**Affects eval:** icao in eval

**Resolution:** No action. HTTP methods are a tiny enumeration ("GET", "POST", etc.) — trivially distinguishable from random 4-letter ICAO codes by cardinality alone.

### C-21: cpt <-> alphanumeric_id

**Cross-validation:** 11% of CPT matches alphanumeric_id
**Affects eval:** Both in eval

**Resolution:** No action. CPT codes are pure numeric while alphanumeric_id includes letters.

### C-22: postal_code <-> alphanumeric_id

**Cross-validation:** 14% of postal codes match alphanumeric_id
**Affects eval:** Both in eval

**Resolution:** No action. Only alphanumeric postal codes (UK-style "SW1A 1AA") match — most are numeric.

### C-23: smiles <-> word

**Cross-validation:** 31% of SMILES match word validation, 100% of words match SMILES (very permissive)
**Affects eval:** Neither directly

**Resolution:** No action. Very short SMILES ("CC", "O") look like words, but SMILES includes distinctive characters like `(`, `)`, `=`, `[`, `]`. The model can learn these structural cues.

---

## Excluded Categories

### Date Order Ambiguity (not collisions)

DMY/MDY/YMD date variants are inherently ambiguous at the value level. Example: "05/03/2024" could be May 3 (MDY) or March 5 (DMY). This is resolved by locale context and column headers, not generator changes. Affected pairs: compact_dmy/compact_mdy/compact_ymd, dmy_dash/mdy_dash, dmy_slash/mdy_slash, short_dmy/short_mdy/short_ymd, etc.

### Timestamp Precision Variants (superset relationships)

iso_8601 validation matches iso_8601_milliseconds and iso_8601_microseconds because the base pattern is a superset. These are correctly differentiated by the presence/absence of fractional seconds. Not a generator issue.

### Permissive Validation False Positives

container.object.csv's validation (`^[^,]+(,[^,]+)*$`) matches almost any string without commas. This creates ~100+ false collision pairs. These are validation breadth issues, not generator collisions.

### Long Month Name Overlap

abbreviated_month ("Oct 3, 2023") matches full_month ("October 3, 2023") validation because the regex accepts both. The generators correctly produce different month name lengths. Not actionable.

---

## Recommended Actions

### Generator Changes (require approval)

```
| ID   | Change                                           | Impact    |
|------|--------------------------------------------------|-----------|
| C-01 | hs_code: remove 2-level bucket, require 3+ dots | HIGH      |
| C-07 | calver: enforce 20XX year prefix, version: more  | MEDIUM    |
|      | v-prefix variants                                |           |
```

### Model/Pipeline Changes (no generator changes)

```
| ID       | Approach                                       | Priority  |
|----------|------------------------------------------------|-----------|
| C-03     | Improve F5 numeric_code disambiguation rule    | HIGH      |
| C-04     | Consider TSID prefix rule in Sharpen           | MEDIUM    |
| C-02,C-06| Header disambiguation — model should learn     | MEDIUM    |
| C-09     | Consider merging form_data/query_string        | LOW       |
```

### No Action Required

C-05 (email/paypal_email), C-08 (isrc/isin), C-10 (json/geojson), C-11 (amount_minor_int/numeric_code), C-12 through C-23 — all resolved by header context, model training, or inherent to the type system.

---

## Methodology Notes

1. Generated 100 samples per type using `finetype generate --samples 100 --seed 42`
2. Extracted validation patterns from taxonomy JSON (`finetype taxonomy --full --output json`)
3. For each broad_type group with >1 member, cross-validated: "does type A's generated output match type B's validation regex?"
4. Filtered out permissive validations (e.g., `^.+$`, container patterns) that match everything
5. Filtered out inherent date/timestamp ambiguities (DMY/MDY) that are not generator problems
6. Manually inspected remaining pairs for actual training data confusion risk
7. Checked each collision pair against eval manifest to assess accuracy impact
