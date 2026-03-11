---
status: accepted
date-created: 2026-03-06
date-modified: 2026-03-11
---
# 0025. Type naming convention — describe format, not locale

## Context and Problem Statement

FineType's datetime and numeric type names originally used geographic references: `eu_slash` (European slash-separated date), `us_slash` (American slash-separated date), `decimal_number_eu` (European decimal notation with comma). This was misleading — the format `DD/MM/YYYY` is used across dozens of countries, not just Europe. And `MM/DD/YYYY` is used in the Philippines and Palau, not just the US.

## Considered Options

- **Geographic naming (status quo)** — `eu_slash`, `us_slash`, `american_12h`, `european_hm`. Familiar but inaccurate and culturally presumptive.
- **Format-structural naming** — `dmy_slash`, `mdy_slash`, `mdy_12h`, `dmy_hm`, `decimal_number_comma`. Describes what the format *looks like*, not where it's used.
- **ISO standard references** — `iso_8601`, `rfc_2822`. Already used where applicable but doesn't cover all formats.

## Decision Outcome

Chosen option: **Format-structural naming** — "describe the format, not the locale." Applied across 10 type renames in NNFT-234:

| Old name | New name |
|----------|----------|
| eu_slash | dmy_slash |
| us_slash | mdy_slash |
| american | mdy_12h |
| european | dmy_hm |
| decimal_number_eu | decimal_number_comma |
| amount_eu | amount_comma |
| amount_eu_suffix | amount_comma_suffix |
| amount_us | amount |
| amount_accounting_us | amount_accounting |
| amount_ch | amount_apostrophe |

Old names preserved in `aliases` for backward compatibility.

### Consequences

- Good, because type names are now culturally neutral and technically accurate
- Good, because aliases preserve backward compatibility — old names still resolve
- Good, because new names are self-documenting: `dmy_slash` immediately tells you the field order
- Bad, because users familiar with the old names need to learn new ones (aliases mitigate this)
- Neutral, because the principle only applies to format types — semantic types (city, country, email) don't have this problem
