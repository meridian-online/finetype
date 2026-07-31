# Reference-data inventory — the B2 mining-factory catalogue

**Status:** tracked, durable. Promoted from memo `2026-06-05-reference-data-inventory.md`
via spec `2026-06-05-reference-data-inventory` (ac-01). The memo is deleted on
distillation; this file is the canonical record.

**Purpose:** catalogue every authoritative reference input and the taxonomy type(s)
it backs, so the mining factory (B2) knows what raw material exists, where the gaps
are, and what licensing permits. Breadth of this catalogue caps breadth of the
harvest.

**Scope reminder:** this file records catalogue *counts*, *licences*, and *embed
line-ranges* only. It vendors no raw third-party data values. Any actual list
ingestion happens under the B2 ingestion pattern (MADR `0092`), never by pasting
values here.

---

## Headline finding

What is embedded today is **demonstration-grade, not authoritative**: small
hand-curated subsets sufficient to *generate* a few synthetic examples, nowhere
near the full standard lists a mining sieve needs. The generators emit a stub
list; the *validation* enums are larger but still incomplete except for
`country_code`.

| Type | Embedded now (generator) | Full authoritative list | Source file:line |
|------|-------------------------:|------------------------:|------------------|
| `finance.currency.currency_code` | 39 codes | ISO 4217 ≈ 180 | `generator.rs:3808–3813` |
| `finance.currency.currency_symbol` | 30 symbols | Unicode Sc ≈ full | `generator.rs:3817–3820` |
| `technology.internet.top_level_domain` | 10 | IANA root zone ≈ 1,500 | `generator.rs:908–910` |
| `datetime.offset.iana` (tz enum) | 12 | IANA tz db ≈ 600 | `generator.rs:707–720` |
| `geography.location.city` | ≈ 12 per locale | Geonames ≈ 4M populated places | `locale_data.rs` `cities()` (~1080–1320) |
| `geography.location.country_code` (generator) | 20 | ISO 3166-1 ≈ 249 | `generator.rs:2285–2288` |
| `geography.location.country_code` (**validation enum**) | **249 ✓** | ISO 3166-1 alpha-2 = 249 | `definitions_geography.yaml:74` |
| `identity.person.first/last_name` | ≈ 40 per locale | (no single canonical source) | `locale_data.rs` `first_names()` / `last_names()` |
| `representation.scientific.measurement_unit` | 28 | SI/UCUM full | `generator.rs:3144–3148` |

**The one type already done right:** `geography.location.country_code` carries the
full **249-member** ISO 3166-1 alpha-2 enum in its validation block
(`definitions_geography.yaml:74`, note at `:336`). That is the **completeness
template** — every reference-backed type should reach this completeness. Note the
generator stub (`generator.rs:2285`, 20 codes) is *not* the template; it only feeds
synthetic generation. The factory's recall target is the validation enum, not the
generator stub.

**No external data crates.** All reference data is hand-embedded in
`generator.rs` (6,848 lines) and `locale_data.rs` (3,333 lines). No `rust_iso3166`,
`chrono-tz`, `celes`, etc. Integrating Geonames/CLDR/IANA means a new
data-ingestion step — a pattern the repo does not have. Decided in MADR `0092`
(ac-05).

---

## Mining-readiness tiers — the actionable cut

A sieve can only mine a type well if an authoritative **vocabulary** exists to
recognise its real-world values. By that test the 240 types split three ways:

- **Tier 1 — Reference-backed, mine directly.** A large authoritative list gives
  high-recall positive examples. The factory's core targets — the 10–100× wins.
- **Tier 2 — Large enumerable code space, mine with the pattern as precision gate.**
  Authoritative lists exist externally but are large and sometimes licensed.
  Sieve = pattern recall + list-membership precision.
- **Tier 3 — Pure pattern, no vocabulary.** No enumerable reference helps; the label
  comes from the pattern we already have. Mining real columns still yields realistic
  *distributions* for training, but a sieve is no better than the validator here.

### Tier assignment — all 240 types, by domain

Counts: **Tier 1 = 22, Tier 2 = 25, Tier 3 = 193. Total = 240.** No type unassigned.
Per-domain sub-totals are stated under each block; the 7 domain totals sum to 240
(container 11, datetime 84, finance 28, geography 25, identity 33, representation 33,
technology 26).

#### container (11) — Tier 3 = 11

Pure format/structure detection; no vocabulary exists. All Tier 3.

- T3: `array.comma_separated`, `array.pipe_separated`, `array.semicolon_separated`,
  `array.whitespace_separated`, `key_value.query_string`, `object.csv`,
  `object.html`, `object.json`, `object.json_array`, `object.xml`, `object.yaml`

#### datetime (84) — Tier 1 = 3, Tier 3 = 81

Date/time *formats* are pattern types (Tier 3); only the human-readable *name*
vocabularies are reference-backed (CLDR).

- **T1 (3):** `component.day_of_week`, `component.month_name`,
  `date.abbreviated_month` → **CLDR** name vocabularies (per-locale). `day_of_week`
  and `month_name` are already CLDR-sourced via `locale_data.rs month_names()`;
  the abbreviated forms close the same source.
- **T3 (81):** every other datetime type — all `date.*` format patterns
  (`iso`, `dmy_*`, `mdy_*`, `ymd_*`, `compact_*`, `short_*`, `chinese_ymd`,
  `korean_ymd`, `jp_era_*`, `julian`, `ordinal`, `iso_week`, `weekday_*`,
  `month_year_*`, `year_month`, `long_full_month`, `full_month_no_comma`,
  `abbrev_month_no_comma`), all `time.*`, all `timestamp.*`, all `epoch.*`,
  `duration.iso_8601`, `offset.iana`†, `offset.utc`, `period.fiscal_year`,
  `period.quarter`, `component.year`, `component.periodicity`.
  † `offset.iana` *values* come from the IANA tz database (≈600), so its vocabulary
  is reference-backed — but the column is recognised by the `Region/City` slash
  pattern, and mining adds little discriminative signal over the pattern. Kept T3;
  flagged for re-evaluation if tz-name false positives surface.

#### finance (28) — Tier 1 = 2, Tier 2 = 9, Tier 3 = 17

- **T1 (2):** `currency.currency_code` (ISO 4217), `currency.currency_symbol`
  (Unicode Sc block).
- **T2 (9):** `banking.iban` (≈80 country IBAN specs), `banking.swift_bic`
  (SWIFT/BIC registry, licensed — pattern+checksum), `banking.aba_routing`
  (Fed list), `banking.bsb` (AU BSB directory), `securities.isin`†,
  `securities.cusip`†, `securities.sedol` (FTSE-licensed),
  `securities.figi` (OpenFIGI), `securities.lei` (GLEIF CC0, open).
  †`isin`/`cusip` are **DO-NOT-VENDOR** (see ac-03).
- **T3 (17):** all 12 `currency.amount*` format types, `crypto.bitcoin_address`,
  `crypto.ethereum_address`, `payment.credit_card_number` (Luhn pattern),
  `rate.basis_points`, `rate.yield`.

> Finance per-type ledger (authoritative, sums to 28):
> T1: currency_code, currency_symbol = **2**.
> T2: iban, swift_bic, aba_routing, bsb, isin, cusip, sedol, figi, lei = **9**.
> T3: 12 × amount*, bitcoin_address, ethereum_address, credit_card_number,
> basis_points, yield = **17**. 2 + 9 + 17 = 28.

#### geography (25) — Tier 1 = 9, Tier 2 = 5, Tier 3 = 11

- **T1 (9):** `location.city`, `location.region`, `location.country`,
  `location.country_code` (✓ complete template), `location.continent`,
  `location.state_code`, `address.postal_code`, `address.street_name`,
  `address.street_suffix` → **Geonames** (places, admin hierarchy, postal,
  street vocab) + ISO 3166-2 for subdivisions.
- **T2 (5):** `transportation.iata_code` (≈10k airports, OpenFlights),
  `transportation.icao_code`, `transportation.unlocode` (UN/LOCODE),
  `transportation.iso6346` (container ISO, checksum), `transportation.hs_code`
  (WCO Harmonised System).
- **T3 (11):** `address.full_address`, `contact.calling_code`†,
  `coordinate.coordinates`, `coordinate.dms`, `coordinate.geohash`,
  `coordinate.latitude`, `coordinate.longitude`, `coordinate.mgrs`,
  `coordinate.plus_code`, `format.wkt`, `index.h3`.
  † `calling_code` *values* come from ITU-T E.164 (≈240), but the column is a short
  numeric code recognised by pattern + range; shape-ambiguous against other small
  integers, so mining narrows rather than closes. Kept T3.

#### identity (33) — Tier 1 = 3, Tier 2 = 11, Tier 3 = 19

- **T1 (3):** `person.gender`, `person.gender_code` (HL7 FHIR / ICAO 9303 /
  ISO 5218 — small but complete), `person.blood_type` (8-member closed vocab).
- **T2 (11):** `commerce.ean`, `commerce.upc`, `commerce.isbn`, `commerce.issn`,
  `commerce.isrc`, `medical.npi`, `medical.ndc`, `medical.icd10`, `medical.loinc`,
  plus the two DO-NOT-VENDOR landmines `medical.cpt`, `medical.hcpcs`
  (**CPT is AMA-copyrighted**; mine via pattern + structure only). CMS/FDA/
  Regenstrief lists — open except as flagged.
- **T3 (19):** `academic.orcid` (checksum), `government.abn` / `ein` / `eu_vat` /
  `pan_india` / `ssn` / `vin` (checksum/pattern), `medical.dea_number` (checksum),
  `person.email`, `person.email_display`, `person.first_name`†, `person.last_name`†,
  `person.full_name`†, `person.username`, `person.password`, `person.phone_e164`,
  `person.phone_number`, `person.height`, `person.weight`.
  † person-name types have rich per-locale vocabularies (Geonames has none for
  names; no single canonical source) — kept T3 because no authoritative *complete*
  list exists; mining real name columns gives distribution, not a membership gate.

> Identity per-type ledger (sums to 33):
> T1: gender, gender_code, blood_type = **3**.
> T2: ean, upc, isbn, issn, isrc, npi, ndc, icd10, loinc, cpt, hcpcs = **11**.
> T3: orcid, abn, ein, eu_vat, pan_india, ssn, vin, dea_number, email,
> email_display, first_name, last_name, full_name, username, password,
> phone_e164, phone_number, height, weight = **19**. 3 + 11 + 19 = 33.

#### representation (33) — Tier 1 = 2, Tier 3 = 31

- **T1 (2):** `file.mime_type` (IANA media types), `scientific.measurement_unit`
  (SI/UCUM).
- **T3 (31):** all 3 `boolean.*`, both `discrete.*` (`categorical`, `ordinal` —
  open vocabularies, no membership gate), `file.excel_format`, `file.extension`,
  `file.file_size`, all 3 `format.color_*`, all 4 `identifier.*`
  (`alphanumeric_id`, `increment`, `numeric_code`, `uuid`), all 6 `numeric.*`,
  all 7 `scientific.*` except measurement_unit (`cas_number`, `dna_sequence`,
  `inchi`, `protein_sequence`, `rna_sequence`, `smiles` — structural patterns,
  no enumerable list), all 4 `text.*` (`emoji`†, `entity_name`, `plain_text`,
  `word`).
  † `emoji` has a closed Unicode vocabulary but is recognised by codepoint class,
  not membership; kept T3.

#### technology (26) — Tier 1 = 3, Tier 3 = 23

- **T1 (3):** `internet.top_level_domain` (IANA root zone),
  `internet.http_method` (RFC 7231 — small, complete ✓), `code.locale_code`
  (ISO 639 + BCP 47 / CLDR).
- **T3 (23):** `cloud.aws_arn`, `cloud.s3_uri`, `code.doi`, `code.imei` (checksum),
  `cryptographic.hash`, `cryptographic.jwt`, `cryptographic.token_urlsafe`,
  `development.calver`, `development.docker_ref`, `development.version`,
  `identifier.snowflake_id`, `identifier.tsid`, `identifier.ulid`,
  `internet.cidr`, `internet.data_uri`, `internet.hostname`, `internet.ip_v4`,
  `internet.ip_v4_with_port`, `internet.ip_v6`, `internet.mac_address`,
  `internet.url`, `internet.urn`, `internet.user_agent`.

### Tier roll-up

| Domain | Total | Tier 1 | Tier 2 | Tier 3 |
|--------|------:|-------:|-------:|-------:|
| container | 11 | 0 | 0 | 11 |
| datetime | 84 | 3 | 0 | 81 |
| finance | 28 | 2 | 9 | 17 |
| geography | 25 | 9 | 5 | 11 |
| identity | 33 | 3 | 11 | 19 |
| representation | 33 | 2 | 0 | 31 |
| technology | 26 | 3 | 0 | 23 |
| **Total** | **240** | **22** | **25** | **193** |

Tier totals: **Tier 1 = 22, Tier 2 = 25, Tier 3 = 193. Sum = 240.** Every taxonomy
type is assigned; no type is unassigned.

---

## Source catalogue (ac-02)

Each row: the taxonomy type(s) it backs, current embedded coverage (count), the full
authoritative list size, the licence, and the **embed location it would replace** so
the gap is concrete. The full-249 `country_code` validation enum is the completeness
template every reference-backed type should reach.

| Source | Backs (types) | Current | Full | Embed location (replace) | Licence (public-repo verdict) |
|--------|---------------|--------:|-----:|--------------------------|-------------------------------|
| **Geonames** | geography: city, region, country, continent, state_code, postal_code, street_name/suffix | ≈12/locale (city), 15–40/locale | ≈4M places + admin/postal | `locale_data.rs` `cities()` (~1080–1320); region/postal scattered | **CC-BY (attribution)** — site states "CC-BY"; version not pinned (commonly cited 4.0). Verify version before vendoring. SAFE to mine + attribute. |
| **CLDR** | datetime: day_of_week, month_name, abbreviated_month; technology.locale_code; (currency/number formats) | 32 locales partial; `month_names()` per-locale | full, all locales | `locale_data.rs` `month_names()` (~1708–2099), `day_names` | **Unicode Licence V3** — permissive, redistribution allowed with attribution. SAFE. |
| **ISO 3166-1** | geography: country_code ✓, country (name), state_code (3166-2) | 249 (validation enum ✓); generator stub 20 | 249 alpha-2 | `definitions_geography.yaml:74` (✓ template); `generator.rs:2285` (stub) | Codes free to use; ISO charges only for the printed standard. SAFE. |
| **ISO 4217** | finance: currency_code, currency_symbol | 39 codes / 30 symbols | ≈180 codes | `generator.rs:3808–3813` (codes); `3817–3820` (symbols) | Codes free to use (ISO 4217 list published openly). SAFE. |
| **ISO 639 / BCP 47** | technology.locale_code; (language display via CLDR) | partial | full (639-1/2/3 + BCP 47) | embedded in locale handling | Codes free to use. SAFE. |
| **IANA tz database** | datetime.offset.iana | 12 | ≈600 zones | `generator.rs:707–720` | **Public domain.** SAFE. |
| **IANA root zone (TLD)** | technology.top_level_domain | 10 | ≈1,500 | `generator.rs:908–910` | Public domain / open registry. SAFE. |
| **IANA media types** | representation.file.mime_type | small subset | full registry | `generator.rs:2775` block | Public domain / open registry. SAFE. |
| **ITU-T E.164** | geography.contact.calling_code; identity.person.phone_e164 (prefix) | ≈40 | ≈240 country prefixes | `locale_data.rs:2865` `calling_codes()`; `generator.rs:2551` contact block | ITU-T recommendations free to view/use. SAFE. |
| **HL7 FHIR / ICAO 9303 / ISO/IEC 5218** | identity.person.gender, gender_code | complete ✓ | small closed set | `generator.rs:1694` (gender), `1702` (gender_code) | HL7 FHIR open (CC0-style); ICAO/ISO codes free. SAFE. |
| **SI / UCUM** | representation.scientific.measurement_unit | 28 | full SI + UCUM | `generator.rs:3144–3148` | SI public; UCUM open licence. SAFE. |
| **RFC 7231 (+ RFC 5789)** | technology.internet.http_method | complete ✓ | small closed set | `generator.rs` http_method block (~907 region) | IETF RFCs free (BCP 78). SAFE. |
| **OpenFlights / ICAO** | geography: iata_code, icao_code | pattern-only | ≈10k airports | n/a (no embed; pattern validation) | OpenFlights ODbL — **verify before use** (share-alike obligations). |
| **UN/LOCODE, WCO HS** | geography: unlocode, hs_code | pattern-only | large | n/a | UN/LOCODE open; WCO HS nomenclature — **verify before use**. |
| **GS1 / Bowker / ISSN / IFPI** | identity.commerce: ean, upc, isbn, issn, isrc | pattern-only | large | n/a | Allocation registries; full lists licensed — **verify; mine via pattern + checksum**. |
| **CMS / FDA / Regenstrief** | identity.medical: npi, ndc, icd10, loinc | pattern-only | large | n/a | NPI/NDC/ICD-10 open (US gov); LOINC free with registration — **verify LOINC terms before vendoring**. |
| **AMA (CPT) / CMS (HCPCS)** | identity.medical: cpt, hcpcs | pattern-only | large | n/a | **CPT is AMA-copyrighted — DO-NOT-VENDOR.** HCPCS Level II open; Level I = CPT, so treat HCPCS as pattern+checksum only. |
| **ANNA / CUSIP Global Services / FTSE (SEDOL)** | finance.securities: isin, cusip, sedol | pattern-only | large | n/a | **CUSIP & ISIN licensed — DO-NOT-VENDOR.** SEDOL licensed too. Mine via pattern + check-digit only. |
| **GLEIF (LEI), Bloomberg OpenFIGI** | finance.securities: lei, figi | pattern-only | large | n/a | GLEIF LEI **CC0 (public domain)** — SAFE. OpenFIGI free with attribution — verify terms. |
| **SWIFT / national bank registries** | finance.banking: swift_bic, aba_routing, bsb, iban specs | ≈16 IBAN specs | ≈80 IBAN countries; SWIFT licensed | `generator.rs` iban/swift blocks | IBAN registry (SWIFT) — **verify; SWIFT BIC directory licensed.** ABA/BSB national lists open. |

---

## Licensing verdict per source (ac-03) — public-repo safety is explicit

**Tier-1 permissive sources, specific licence named, SAFE to mine + attribute:**

- **Geonames** — CC-BY (attribution). The export page states "cc-by licence" without
  pinning a version; widely cited as CC-BY 4.0. **Verdict: SAFE to mine with
  attribution; verify the exact CC-BY version before vendoring a snapshot.**
- **CLDR** — Unicode Licence V3 (permissive, redistribution allowed with the
  copyright/permission notice). **Verdict: SAFE.**
- **IANA** (tz database, root-zone TLDs, media types) — public domain / open
  registry. **Verdict: SAFE.**
- **ISO codes** (3166, 4217, 639) — the *code lists* are free to use; ISO charges only
  for the printed standard document. **Verdict: SAFE.**
- **ITU-T E.164** — ITU-T recommendations free to view and use. **Verdict: SAFE.**
- **RFC 7231 / IETF**, **HL7 FHIR**, **ISO/IEC 5218**, **ICAO 9303**, **SI/UCUM** —
  small closed/standard vocabularies, all free. **Verdict: SAFE.**
- **GLEIF LEI** — CC0 (public domain). **Verdict: SAFE.**

**The two landmines — DO-NOT-VENDOR (mine only via pattern + checksum, never vendor
the list):**

- **CPT** (`identity.medical.cpt`, and HCPCS Level I) — **AMA-copyrighted.** Vendoring
  the code descriptions/list infringes. Recognise by structure only.
- **CUSIP / ISIN** (`finance.securities.cusip`, `finance.securities.isin`; SEDOL too) —
  **commercially licensed.** Vendoring the full enumerated list breaches the licence.
  Recognise by pattern + check-digit only.

**Marked "verify before use" (NOT assumed safe) — licence could not be confirmed
permissive for vendoring:**

- **OpenFlights** (iata/icao) — ODbL share-alike; verify obligations before vendoring.
- **WCO HS nomenclature**, **UN/LOCODE** — verify per-source terms.
- **GS1 / Bowker / ISSN / IFPI** allocation registries (ean/upc/isbn/issn/isrc) —
  full lists licensed; mine via pattern + checksum, verify before any vendoring.
- **LOINC** — free but registration-gated; verify the terms-of-use before vendoring.
- **SWIFT BIC directory / IBAN registry** — SWIFT directory licensed; the IBAN
  *spec* (structure per country) is fine, the BIC list is not. Verify.
- **OpenFIGI** — free with attribution; verify the redistribution terms.

Default rule applied throughout: any source whose licence is not confirmed permissive
for *vendoring* is marked "verify before use" and may only be mined via
pattern + checksum until cleared. No source is assumed safe.

---

## B2 build order — locked and justified (ac-04)

Ordering principle: **coverage-gap × licence-permissiveness** — front-load the
largest coverage gains against the most permissive licences.

1. **Geonames** — geography names (city, region, district, country, continent,
   postal, street). **The single biggest unrealised win:** the thinnest types
   (city/region/state_code at ≈12–40 hand-curated per locale) against the richest
   available source (≈4M places), under a permissive CC-BY licence. Build first.
2. **Full CLDR** — datetime name vocabularies (month/weekday/abbreviated) and locale
   formats. Already partially wired (`month_names()`), permissive Unicode licence;
   close the coverage from 32 partial locales to full.
3. **IANA** — timezones (12→≈600), TLDs (10→≈1,500), media types. Small,
   public-domain, trivial to complete; high ratio of coverage gain to effort.
4. **ISO 4217 / 639 (full)** — currency codes (39→≈180), language/locale codes.
   Finishes what `country_code` already models, free codes, small.

Tier-2 / medical / securities sources come later, and only via pattern + checksum
where licensing forbids vendoring (CPT, CUSIP/ISIN, and every "verify before use"
source above).

---

## Reconciliation with card 0017 (ac-06)

**Card 0017** (`ydf-specialist-geography`) proposes pulling
`(Sense-missed, YDF-confident)` columns from the multi-lens diagnostic as training
rows — a self-improving loop that mines GitTables using the **YDF model** as the
confidence signal.

**This spec's method** (the reference-data-trained sieve) trains a sieve **from
Geonames/CLDR/ISO** — an authoritative external vocabulary, not a model — and then
mines GitTables for columns whose values match that vocabulary.

**Relationship: two stages of one funnel, not a fork.** B2 builds *on* card 0017:

- **Stage 1 — reference sieve (this spec):** high-precision membership filter. A
  column whose values are ≥X% members of the Geonames city vocabulary is a city, with
  near-zero false-positive risk. This stage harvests the *easy, certain* positives the
  authoritative list can vouch for.
- **Stage 2 — YDF-confidence extractor (card 0017):** the *residual* recall layer.
  Columns the reference sieve can't vouch for (misspellings, abbreviations, non-Latin
  names absent from the snapshot, types with no vocabulary) but where YDF is confident
  — these are card 0017's `(Sense-missed, YDF-confident)` rows. Stage 2 catches what
  Stage 1's membership gate misses.

So the reference sieve **complements and precedes** the YDF extractor: it does not
supersede card 0017, and card 0017 does not need a separate mining mechanism. One
funnel — authoritative-membership first, model-confidence residual second — feeding
the same v20+ Sense retrain.

**Eval-independence implication (load-bearing).** Once the YDF lens *mines* training
rows, it can no longer also be the *judge* — scoring Sense against YDF after training
Sense on YDF-mined labels is circular (Sense passes by construction). The independent
ground truth is the gold-label eval anchor built in spec
`2026-06-05-gold-eval-anchor` (closed), whose ac-01 independence contract explicitly
forbids gold labels deriving from the YDF lens, the Sense cascade, **or any
authoritative source the mining factory (B2) ingests** — i.e. not from Geonames/CLDR/
ISO either. The reference sieve and the gold anchor must share no inputs, or the eval
is circular through the back door. B2 inherits this constraint.

---

## Cross-references

- Spec: `2026-06-05-reference-data-inventory`
- Roadmap memo: `the 2026-06-05-precision-release-roadmap memo` (Line B / B0)
- Ingestion pattern: MADR `0092` (ac-05)
- Card 0017: `the ydf-specialist-geography card`
- Gold eval anchor: `spec 2026-06-05-gold-eval-anchor`
- Dataset provenance registry: `choice 0090 (dataset-provenance-pattern)`,
  `eval/datasets/sources.yaml`
- Completeness template: `labels/definitions_geography.yaml:74` (full-249
  country_code enum)
