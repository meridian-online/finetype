# B0 — Reference-data inventory (draft)

**Date:** 2026-06-05
**Status:** draft for the precision-release roadmap (memo
`2026-06-05-precision-release-roadmap.md`, Line B / B0). Awaiting author react,
then distil into the B0 spec.
**Purpose:** catalogue every authoritative reference input and the taxonomy type(s)
it backs, so the mining factory (B2) knows what raw material exists and where the
gaps are. Breadth of this catalogue caps breadth of the harvest.

## Headline finding

What is embedded today is **demonstration-grade, not authoritative**: small
hand-curated subsets sufficient to *generate* a few synthetic examples, nowhere near
the full standard lists a mining sieve needs. Evidence:

| Type | Embedded now | Full authoritative list |
|------|-------------:|------------------------:|
| `finance.currency.currency_code` | 39 codes | ISO 4217 ≈ 180 |
| `technology.internet.top_level_domain` | 10 | IANA root zone ≈ 1,500 |
| `datetime.offset.iana` (tz enum) | 12 | IANA tz db ≈ 600 |
| `geography.location.city` | 15–40 per locale | Geonames ≈ 4M populated places |
| `identity.person.first/last_name` | ≈ 40 per locale | (no single canonical source) |
| `finance.banking.iban` country specs | 16 | ≈ 80 IBAN countries |

**The one type already done right:** `geography.location.country_code` carries the
full 249-member ISO 3166-1 alpha-2 enum (`definitions_geography.yaml:74`). That is the
template — every reference-backed type should reach this completeness.

**No external data crates.** All reference data is hand-embedded in
`generator.rs` (≈6,850 lines) and `locale_data.rs` (≈3,330 lines). No
`rust_iso3166`, `chrono-tz`, `celes`, etc. Integrating Geonames/CLDR/IANA means a
new data-ingestion step (build-time generation or pipeline-time fetch) — a pattern
the repo does not have yet. Flag for B2.

## Mining-readiness tiers (the actionable cut)

A sieve can only mine a type well if an authoritative VOCABULARY exists to recognise
its real-world values. By that test the 240 types split three ways:

### Tier 1 — Reference-backed, mine directly (the 10–100× wins)

Enumerable types where a large authoritative list gives high-recall positive
examples. The factory's core targets.

- **Geography names** — `city`, `state`/`region`, `country` (name), `continent`,
  `address.district`, `postal_code` → **Geonames** (4M places, admin hierarchy,
  postal). Today 15–40 hand-curated per locale. Biggest single gap.
- **Datetime names** — `day_of_week`, `month_name`, `abbreviated_month/weekday`,
  date/number patterns → **CLDR** (full, all locales). Today 32 locales, partial;
  `day_of_week` is already CLDR-sourced and authoritative.
- **Codes already standard-sourced** — `country_code` (ISO 3166, full ✓),
  `currency_code` (ISO 4217, 39→180), `currency_symbol` (Unicode Sc),
  `locale_code`/language (ISO 639 + BCP 47), `gender`/`gender_code`
  (HL7 FHIR / ICAO 9303 / ISO 5218 — small but complete), `http_method`
  (RFC 7231 ✓), `measurement_unit` (SI/UCUM, 23→full).
- **IANA registries** — `offset.iana` timezones (12→600), `top_level_domain`
  (10→1,500), `representation.format.mime_type` (→ IANA media types).
- **Telephony** — `calling_code`/`phone` country prefixes → **ITU-T E.164**
  (40→≈240).

### Tier 2 — Large enumerable code space, mine with pattern as precision gate

Authoritative lists EXIST externally but are large and sometimes licensed. Sieve =
pattern recall + list-membership precision.

- **Transport codes** — `iata_code` (≈10k airports), `icao_code`, `unlocode`,
  `iso6346`, `hs_code` → OpenFlights / ICAO / UN.
- **Securities** — `isin`, `cusip`, `sedol`, `figi`, `lei` → check-digit
  algorithms exist; full lists are licensed (CUSIP, ISIN) — **flag licensing**.
- **Commerce/medical codes** — `ean`/`upc`/`isbn`/`issn`, `npi`, `ndc`, `icd10`,
  `loinc`, `cpt`, `hcpcs` → CMS/FDA/Regenstrief lists; **CPT is AMA-copyrighted,
  CUSIP licensed — cannot redistribute**. Mine via pattern + checksum only.

### Tier 3 — Pure pattern, no vocabulary (mining adds little over the regex)

No enumerable reference helps; the label comes from the pattern we already have.
Mining real columns still gives realistic *distributions* for training, but a sieve
is no better than the validator here — and this is where shared-shape ambiguity
bites.

- **Structural/format** — all 11 `container.*` (JSON/XML/CSV/YAML…), `uuid`,
  `ulid`, `hash`, `jwt`, `ip_v4/v6`, `mac_address`, `email`, `url`, `color_*`,
  `aws_arn`, `s3_uri`, version strings.
- **Shared-shape numerics** — `latitude`/`longitude`/`coordinates`,
  `component.year`, `percentage`, `integer_number`/`decimal_number`,
  `basis_points`. **These stay the late-fusion model's job** (header + sibling
  context). Geonames gives real lat/long *values*, but a bare float column is still
  shape-ambiguous — mining narrows, doesn't close.

## Source catalogue

| Source | Backs (types) | Current | Full | Licence (public-repo safe?) |
|--------|---------------|--------:|-----:|------------------------------|
| **Geonames** | city, state/region, country names, continent, district, postal | 15–40/locale | ≈4M | CC-BY 4.0 ✓ |
| **CLDR** | month/weekday names, date/number/currency formats, territory & language display names | 32 locales partial | full | Unicode licence ✓ |
| **ISO 3166-1** | country_code ✓, state_code (subset), country names | 249 (code ✓) | full | codes free to use ✓ |
| **ISO 4217** | currency_code, currency_symbol | 39 | ≈180 | codes free ✓ |
| **ISO 639 / BCP 47** | locale_code, language | partial | full | free ✓ |
| **IANA** | timezones, TLDs, media types | 12 / 10 / — | 600 / 1.5k / full | public domain ✓ |
| **ITU-T E.164** | calling_code, phone prefixes | ≈40 | ≈240 | free ✓ |
| **HL7 FHIR / ICAO 9303 / ISO 5218** | gender, gender_code | complete ✓ | — | free ✓ |
| **SI / UCUM** | measurement_unit | 23 | full | free ✓ |
| **RFC 7231** | http_method | complete ✓ | — | free ✓ |
| **OpenFlights / ICAO / UN** | iata/icao/unlocode/iso6346/hs_code | pattern-only | large | mostly open; verify per-source |
| **CMS / FDA / Regenstrief** | npi, ndc, icd10, loinc, hcpcs | pattern-only | large | open EXCEPT **CPT (AMA ©)** |
| **ANNA / CUSIP Global** | isin, cusip, sedol | pattern-only | large | **licensed — do not redistribute** |

## Gaps to flag

1. **Geonames is the biggest unrealised win** — `city`/`region`/`district` are the
   thinnest (15–40 hand-curated) against the richest available source (≈4M).
   First integration to build.
2. **No ingestion pattern exists.** Everything is hand-embedded; B2 needs a
   repeatable fetch→normalise→column-assemble step. Decide build-time vs
   pipeline-time, and where the normalised data lives (tracked vs downloaded —
   mirrors the model-weights download path in the release-readiness spec).
3. **Licensing is mostly clean but has two landmines for a public repo:** CPT
   (AMA-copyrighted) and CUSIP/ISIN (licensed). Mine those via pattern+checksum
   only; never vendor the lists. Geonames (CC-BY), CLDR, IANA, ISO codes, ITU are
   all safe.
4. **Shared-shape numerics get no help from reference data** (latitude/longitude/
   year/temperature). Explicitly out of the factory's reach; they remain the
   late-fusion model's disambiguation job (B3).
5. **Container domain (11 types) has no reference data possible** — pure format
   detection; mine via pattern only or skip.
6. **Independence reminder (from roadmap):** the curated gold eval anchor (B1) must
   NOT draw from any of these sources if the factory (B2) does — otherwise eval is
   circular. Gold anchor labels come from neither model nor the factory's inputs.

## Recommended first sources (build order for B2)

1. **Geonames** — geography names. Largest gap, permissive licence, immediately
   10–100×'s the weakest types.
2. **CLDR (full)** — datetime names + locale formats. Already partially wired; close
   the coverage.
3. **IANA** — timezones, TLDs, media types. Small, public-domain, trivial to
   complete.
4. **ISO 4217 / 639 full** — currency + language codes. Finish what `country_code`
   already models.

Tier-2/medical/securities come later and only via pattern+checksum where licensing
forbids vendoring.
