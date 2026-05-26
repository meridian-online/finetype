# Broad enum audit across `labels/definitions_*.yaml`

Per spec `2026-05-26-taxonomy-country-code-enum-cleanup` ac-02.

## Method

For every `validation.enum:` block in `labels/definitions_*.yaml`:
read the block, count members, check for: duplicates, non-canonical
entries, cross-domain contamination (e.g. state codes in country
enums), deprecated members. The locale-keyed
`validation_by_locale.<locale>.enum` blocks are also scanned.

Audit covers the 240-type taxonomy as of `main @ 1c9ee9c`
(2026-05-26).

## Verdict

**Audit clean.** No contamination found in any universal or
locale-keyed enum. The country_code contamination memory was
incorrect (see `consumers.md` headline); the actual yaml is fine.

## Universal `validation.enum` blocks

| Label | Size | Canonical source | Verdict |
|---|---:|---|---|
| `geography.location.country_code` | 249 | ISO 3166-1 alpha-2 (officially assigned) | clean — AD..ZW sorted, no duplicates, no state codes |
| `geography.location.continent` | 7 | 7-continent model (Africa, Asia, Europe, NA, SA, Oceania, Antarctica) | clean |
| `geography.address.street_suffix` | 19 | Common English/US suffixes (Street/St, Avenue/Ave, Boulevard/Blvd, ...) | clean — illustrative, designation `broad_words` |
| `identity.person.gender` | 6 | Common inclusive set (Male, Female, Non-binary, Other, Prefer not to say, Unknown) | clean — illustrative, designation `broad_words` per notes |
| `representation.scientific.measurement_unit` | 30 | SI base units + common derived (m, kg, s, A, K, mol, cd, Hz, N, J, W, Pa, °C, L, g and full-name variants) | clean — designation `broad_words` |
| `representation.boolean.binary` | 2 | "0", "1" | clean |
| `representation.boolean.initials` | 8 | T, F, t, f, Y, N, y, n | clean |
| `representation.boolean.terms` | 30 | true/false/yes/no/on/off/enabled/disabled/active/inactive × 3 cases | clean — case-explicit, exhaustive |
| `technology.internet.http_method` | 27 | 9 HTTP methods × 3 cases (UPPER/Title/lower) | clean |
| `datetime.offset.iana` | 12 | IANA tz database (sampled) | **illustrative** — 12 of ~400 names. Pattern carries the strictness; enum is a hint. Not a contamination issue but flag for clarity. |
| `datetime.component.day_of_week` | 7 | Monday–Sunday | clean |
| `datetime.component.month_name` | 12 | January–December | clean |
| `datetime.component.periodicity` | 8 | Once, Daily, Weekly, Biweekly, Monthly, Quarterly, Yearly, Never | clean |

## Locale-keyed `validation_by_locale.<locale>.enum` blocks

| Label | Locales | Size per locale | Verdict |
|---|---|---:|---|
| `geography.location.state_code` | EN_US, EN_CA, EN_AU | 54 / 13 / 8 | clean — US states + DC + territories (AS, GU, MP, PR, VI); Canadian provinces (AB..YT); Australian states (ACT, NSW, NT, QLD, SA, TAS, VIC, WA) |
| `datetime.component.day_of_week` | ~30 (EN, FR, DE, ES, IT, PT, AR, BG, CS, DA, EL, ET, FI, HR, HU, LT, LV, NL, NO, PL, RO, RU, SK, SL, SV, TR, UK, ...) | 7 each | clean — CLDR-sourced, attribution in yaml |
| `datetime.component.month_name` | ~30 (same locale set) | 12 each | clean — CLDR-sourced, attribution in yaml |

## Notable observations

- **The cross-collision space the spec was worried about exists in
  reality, just not in the yaml.** US state codes that are ALSO ISO
  3166-1 alpha-2 country codes: AL=Albania, AR=Argentina, AS=American
  Samoa (territory in both), AZ=Azerbaijan, CA=Canada, CO=Colombia,
  DE=Germany, GA=Georgia (country and state), ID=Indonesia,
  IL=Israel, IN=India, KY=Cayman Islands, LA=Laos, MA=Morocco,
  MD=Moldova, ME=Montenegro, MO=Macau, MP=Northern Mariana Islands
  (territory in both), MS=Montserrat, MT=Malta, NC=New Caledonia,
  NE=Niger, PA=Panama, PR=Puerto Rico (territory in both),
  SC=Seychelles, SD=Sudan, VA=Vatican. The country_code enum
  legitimately contains these — they are valid country codes. The
  state_code enum legitimately contains them too — they are valid
  state codes. The collision is real but the data is correct.

- **`datetime.offset.iana`'s 12-entry illustrative enum is the only
  edge case.** It's not contaminated — every entry IS a real IANA
  name — but the enum is wildly incomplete relative to the ~400-name
  IANA database. Downstream consumers that treat this enum as
  authoritative (rather than a hint) will reject valid IANA names
  outside the 12. Flag for follow-up only if downstream behaviour
  warrants — the pattern handles strict validation.

- **No duplicates anywhere.** Every enum I scanned has unique
  members.

- **No deprecated members.** No legacy codes (e.g. removed ISO
  alpha-2 codes like AN, CS, DD, TP) appear in country_code or
  state_code.

## ac-07 triage

Per the spec's ac-07, follow-up specs only get filed if ac-02
surfaces contamination beyond country_code. **It doesn't.** Append
"audit clean — 2026-05-26" — no follow-up specs filed.

If the `datetime.offset.iana` illustrative-enum question warrants
attention in the future, it is a *completeness* issue, not a
*contamination* issue, and should be addressed in its own card under
data-quality work, not under this enum-cleanup spec's scope.

## Audit clean — 2026-05-26
