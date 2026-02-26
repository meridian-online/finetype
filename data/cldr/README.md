# CLDR and Locale Data Sources

This directory documents the authoritative data sources used for
locale-specific type validation in FineType.

## Postal Code Patterns

**Source:** [Google libaddressinput](https://github.com/google/libaddressinput)
**License:** Apache License 2.0
**Used in:** `labels/definitions_geography.yaml` → `geography.address.postal_code` → `validation_by_locale`

Postal code regex patterns for 14 locales (EN_US, EN_GB, EN_AU, EN_CA, DE,
FR, ES, IT, NL, PL, RU, JA, ZH, KO) were sourced from Google's
libaddressinput project, which maintains authoritative address validation
data for 249 countries.

The patterns are embedded directly in the YAML taxonomy definitions rather
than downloaded at build time. This ensures deterministic builds and avoids
runtime network dependencies.

### Verification API

Patterns can be verified against the live API:

```
https://chromium-i18n.appspot.com/ssl-address/data/{country_code}
```

For example, US postal code data:
```
https://chromium-i18n.appspot.com/ssl-address/data/US
```

### Refreshing Patterns

If libaddressinput updates its postal code patterns:

1. Check the [libaddressinput repository](https://github.com/google/libaddressinput) for changes
2. Verify patterns against the API endpoint above
3. Update `validation_by_locale` entries in `labels/definitions_geography.yaml`
4. Run `cargo test` and `cargo run -- check` to verify alignment

## Phone Number Patterns

**Source:** [Google libphonenumber](https://github.com/google/libphonenumber)
**License:** Apache License 2.0
**Used in:** `labels/definitions_identity.yaml` → `identity.person.phone_number` → `validation_by_locale`

Phone number validation patterns for 14 locales (EN_US, EN_CA, EN_GB, EN_AU,
DE, FR, ES, IT, NL, PL, RU, JA, ZH, KO) were derived from Google's
libphonenumber project, which maintains authoritative phone number metadata
for 300+ regions.

Patterns validate the national number format including optional international
dialling prefix (country code). Each locale pattern enforces the correct digit
count and grouping structure for that country's phone numbers.

The patterns are embedded directly in the YAML taxonomy definitions rather
than downloaded at build time. This ensures deterministic builds and avoids
runtime network dependencies.

### Refreshing Patterns

If libphonenumber updates its phone number metadata:

1. Check the [libphonenumber repository](https://github.com/google/libphonenumber) for changes to `resources/PhoneNumberMetadata.xml`
2. Extract `nationalNumberPattern` for the relevant countries
3. Update `validation_by_locale` entries in `labels/definitions_identity.yaml`
4. Run `cargo test` and `cargo run -- check` to verify alignment

## Calling Code Patterns

**Source:** [ITU-T E.164](https://www.itu.int/rec/T-REC-E.164) country calling codes (public domain)
**Used in:** `labels/definitions_geography.yaml` → `geography.contact.calling_code` → `validation_by_locale`

Per-locale regex patterns for 17 locales (EN, EN_US, EN_CA, EN_GB, EN_AU, DE, FR,
ES, IT, NL, PL, RU, JA, ZH, KO, AR, ZA). Each pattern validates the exact calling
code for that country. Added in NNFT-141.

## Month Name and Day of Week Enumerations

**Source:** [Unicode CLDR JSON](https://github.com/unicode-org/cldr-json) (`cldr-dates-full`, wide format)
**License:** Unicode License (permissive, attribution)
**Used in:**
- `labels/definitions_datetime.yaml` → `datetime.component.month_name` → `validation_by_locale`
- `labels/definitions_datetime.yaml` → `datetime.component.day_of_week` → `validation_by_locale`

Full month name enumerations (12 values per locale) and full weekday name
enumerations (7 values per locale) for 6 locales (EN, FR, DE, ES, IT, PT).
Sourced from `cldr-json/cldr-dates-full/main/{locale}/ca-gregorian.json`,
wide format sections. Added in NNFT-141.

### Refreshing Enumerations

If CLDR updates month/day names:

1. Check the [cldr-json repository](https://github.com/unicode-org/cldr-json) for changes
2. Extract wide-format month/day names from `cldr-dates-full/main/{locale}/ca-gregorian.json`
3. Update `validation_by_locale` enum entries in `labels/definitions_datetime.yaml`
4. Run `cargo test` and `cargo run -- check` to verify alignment

## Future Data Sources

| Source | Purpose | Status |
|--------|---------|--------|
| [CLDR JSON](https://github.com/unicode-org/cldr-json) | Date/time format patterns, number formatting | Partially used (month/day names) |
