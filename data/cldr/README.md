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

## Future Data Sources (Phase 2+)

| Source | Purpose | Status |
|--------|---------|--------|
| [CLDR JSON](https://github.com/unicode-org/cldr-json) | Date/time format patterns, number formatting | Planned |
| [Google libphonenumber](https://github.com/google/libphonenumber) | Phone number validation patterns | Planned |
