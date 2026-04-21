# Discovery: Locale-Aware Type Inference

## Problem

FineType's locale-specific types (15 across geography and identity) try to be both **general** (capturing all worldwide formats) and **precise** (discriminating them from other data). This creates an impossible tension:

- **postal_code** validation is `minLength: 3, maxLength: 10` — no regex pattern, because no single regex can match US ZIP (10001), UK (W1C 1AX), German (10115), Japanese (100-0001), and Indian (110001) postcodes while still rejecting salary integers or ticket numbers.
- **phone_number** validation is `pattern: ^[+]?[0-9\s()\-\.]+$` — matches nearly any digit string with punctuation. A US phone (555-123-4567), Australian (+61 2 1234 5678), and Japanese (03-1234-5678) all need coverage, so the pattern can't be restrictive.

The result: these types are **format attractors** — their validation is so broad that they match almost anything, making the CharCNN's false positives impossible to catch with Signal 1 (validation failure).

### Driving research cases

**Postal codes — the quintessential locale problem:**

| Locale | Format | Example | Regex |
|---|---|---|---|
| US | 5 digits or ZIP+4 | `10001`, `10001-1234` | `^\d{5}(-\d{4})?$` |
| UK | Alphanumeric | `W1C 1AX`, `EC1A 1BB` | `^[A-Z]{1,2}\d[A-Z\d]?\s?\d[A-Z]{2}$` |
| Canada | Letter-digit alternating | `K1A 0B1` | `^[A-Z]\d[A-Z]\s?\d[A-Z]\d$` |
| Germany | 5 digits | `10115` | `^\d{5}$` |
| Japan | 3-4 digits | `100-0001` | `^\d{3}-?\d{4}$` |
| Australia | 4 digits | `2000` | `^\d{4}$` |
| India | 6 digits | `110001` | `^\d{6}$` |

Each locale regex is **precise and discriminating**. The union is **vague and permissive**. If FineType knew the locale context, it could apply the right regex and reject salary integers (6 digits, but not a valid US/UK/CA postal pattern) and ticket numbers (7+ digits, too long for any postal format).

**Phone numbers — same tension:**

| Locale | Format | Example |
|---|---|---|
| US/CA | +1 (xxx) xxx-xxxx | `+1 (555) 123-4567` |
| UK | +44 xxxx xxxxxx | `+44 20 7946 0958` |
| Australia | +61 x xxxx xxxx | `+61 2 1234 5678` |
| Germany | +49 xxx xxxxxxx | `+49 30 12345678` |
| Japan | +81 x-xxxx-xxxx | `+81 3-1234-5678` |

The current phone_number regex is `^[+]?[0-9\s()\-\.]+$` — this matches port numbers, IP addresses, and version strings. A locale-specific regex would be far more discriminating.

### Scale of the problem

15 types are marked `designation: locale_specific` in the taxonomy:

**Geography (9):** country, region, city, full_address, street_number, street_name, street_suffix, postal_code, calling_code
**Identity (6):** full_name, first_name, last_name, phone_number, username, nationality

These are exactly the types that cause the most false positives in profile eval. Of the 10 attractor types identified in NNFT-115, 7 are locale-specific.

## Proposed Approach

### Locale detection strategy

Two possible approaches:

**A. Column-level locale inference:** Infer the likely locale from the column data itself. For postal codes, check if values match US ZIP pattern vs UK postcode pattern vs German pattern. For names, check character set (Latin, CJK, Cyrillic). For phone numbers, check country code prefix.

**B. Dataset-level locale context:** Use other columns in the same dataset to infer locale. If a `country` column exists with "United States", apply US-specific validation to postal_code, phone, etc. If an `iata_code` column suggests airports, the locale context is mixed/international.

Approach A is self-contained per column; Approach B is more powerful but requires cross-column reasoning.

### Validation schema evolution

Currently, each type has a single `validation` block. Locale-aware inference would need:

```yaml
geography.address.postal_code:
  validation:
    type: string
    minLength: 3
    maxLength: 10
  validation_by_locale:
    EN_US:
      pattern: "^\\d{5}(-\\d{4})?$"
    EN_GB:
      pattern: "^[A-Z]{1,2}\\d[A-Z\\d]?\\s?\\d[A-Z]{2}$"
    EN_AU:
      pattern: "^\\d{4}$"
    DE:
      pattern: "^\\d{5}$"
    JA:
      pattern: "^\\d{3}-?\\d{4}$"
```

The `validation` block remains the universal fallback. `validation_by_locale` provides precise patterns that Signal 1 can use when locale is known or inferred.

### Integration with attractor demotion

If locale is inferred for a column, Signal 1 in `disambiguate_attractor_demotion()` uses the locale-specific pattern instead of the universal one. This makes validation discriminating for postal_code and phone_number without breaking the general case.

## Open Questions

1. **How does locale information flow through the inference pipeline?** Is it a property of the column? The dataset? An external hint? Who sets it?

2. **What's the fallback when locale can't be determined?** Use the universal validation (current behaviour) and accept lower discrimination?

3. **Can locale be inferred from the data itself?** E.g., if all postal codes match `^\d{5}$`, infer US. If all match `^[A-Z]\d[A-Z]\s?\d[A-Z]\d$`, infer Canada. What about mixed-locale columns?

4. **Schema format**: Should `validation_by_locale` live in the YAML definitions (static, version-controlled) or in a separate locale data file (could be generated from CLDR or libphonenumber)?

5. **Interaction with NNFT-116 (JSON Schema migration):** Does JSON Schema have a natural way to express conditional validation by locale? (`if`/`then` keywords, `oneOf` with discriminators?)

6. **Priority**: Which locale-specific types benefit most from locale-aware validation? Postal code and phone number are the clear top two. Is it worth tackling names (first_name, last_name) where the locale signal is character set rather than regex pattern?

7. **Training data implications**: Does locale-aware inference require locale-labelled training data? Or can we infer locale at validation time (post-inference) without retraining?

## Existing Infrastructure

- `designation: locale_specific` field already marks which types need this
- `locales: [EN, EN_AU, ...]` already lists supported locales per type
- `finetype-core::validator::validate_value()` already runs validation schemas
- NNFT-115 attractor demotion already uses validation as Signal 1
- NNFT-116 JSON Schema migration provides the opportunity to add richer validation
- NNFT-058/060 (CLDR locale data) is already in the backlog as future domain work

## Success Criteria

A successful locale-aware inference system would:
- Correctly reject `229080` as a US postal code (too many digits) while accepting `10001`
- Correctly reject `A/5 21171` as any locale's postal code (slashes, too long)
- Correctly validate `+61 2 1234 5678` as an Australian phone number while rejecting `192.168.1.1`
- Not require the user to specify locale upfront (infer it when possible)
- Degrade gracefully to universal validation when locale is unknown
