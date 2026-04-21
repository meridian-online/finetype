# Taxonomy Expansion Candidates — 216 → 250+

**Task:** Identify new FineType types with discrete, well-recognised formats to reach 250+ types.
**Date:** 2026-03-07
**Current state:** 216 types across 7 domains (container 12, datetime 85, finance 29, geography 15, identity 19, representation 32, technology 24)
**Target:** 250+ types (need ≥34 net additions)
**Sources:** DuckDB community extensions, ISO/industry standards, real-world dataset analysis, NNFT-176 revision notes

---

## Guiding Principles (from NNFT-176)

1. **Each type is a transformation contract** — distinct DuckDB cast expression required.
2. **Precision over permissiveness** — types must meaningfully distinguish "is this" from "is not this."
3. **Format strings are sacred** — different `strptime` or `regexp_extract` = different type.
4. **Categoricals are a superpower** — but new types should NOT be things `categorical` already covers.

---

## Summary

| Tier | Count | Description |
|------|-------|-------------|
| **1 — DuckDB Extension Native** | 6 | First-class `TRY_CAST` or parser function in a community extension |
| **2 — International Standard Codes** | 20 | ISO/industry standard identifiers with checksums or rigid format |
| **3 — Common Format Patterns** | 10 | Deterministic parse-to-struct patterns analysts encounter daily |
| **4 — Additional Strong Candidates** | 11 | High-quality formats from second-pass research |
| **5 — Conditional / Lower Priority** | ~10 | Narrower audience, harder disambiguation, or region-specific |
| **Total unique new candidates** | **47 confirmed + ~10 conditional** | Comfortably exceeds 250 target |

**Three candidates from initial research were false positives** (already in taxonomy): `bitcoin_address` → `finance.crypto.bitcoin_address`, `mime_type` → `representation.file.mime_type`, `iana_timezone` → `datetime.offset.iana`. These are excluded below.

---

## Tier 1 — DuckDB Extension Native Types (6)

These have first-class support via DuckDB community extensions, giving them native `TRY_CAST` or parser functions.

### 1. `technology.identifier.ulid`

Universally Unique Lexicographically Sortable Identifier. 26-character Crockford Base32 string with embedded 48-bit millisecond timestamp + 80-bit randomness. Growing adoption as UUID alternative (Discord, Stripe, modern APIs).

- **Examples:** `01ARZ3NDEKTSV4RRFFQ69G5FAV`, `01HQJY3BFQC6SA6BCEAA0GJ3VN`
- **Regex:** `^[0123456789ABCDEFGHJKMNPQRSTVWXYZ]{26}$`
- **DuckDB:** `INSTALL ulid FROM community; TRY_CAST(col AS ULID)`
- **Standard:** ULID spec (ulid/spec on GitHub), Crockford Base32
- **Prevalence:** High — increasingly replacing UUIDv4 as primary keys
- **Priority:** 5

### 2. `geography.index.h3`

Uber's H3 hierarchical hexagonal geospatial index. 15-character hex strings encoding location at configurable resolution (0–15).

- **Examples:** `8f089b1a2bb520a`, `891f1d48177ffff`, `8a2a1072b59ffff`
- **Regex:** `^[0-9a-f]{15}$` (leading hex char is `8` for standard cells)
- **DuckDB:** `INSTALL h3 FROM community; h3_cell_to_lat(h3_string_to_h3(col))`
- **Standard:** Uber H3 specification (Apache 2.0)
- **Prevalence:** High — Uber, Foursquare, CARTO, multiple mapping platforms
- **Priority:** 4

### 3. `geography.format.wkt`

Well-Known Text geometry representation. Strings starting with geometry keywords (`POINT`, `LINESTRING`, `POLYGON`, `MULTIPOLYGON`, `GEOMETRYCOLLECTION`).

- **Examples:** `POINT(-73.9857 40.7484)`, `POLYGON((0 0, 1 0, 1 1, 0 1, 0 0))`
- **Regex:** `^(POINT|LINESTRING|POLYGON|MULTI(POINT|LINESTRING|POLYGON)|GEOMETRYCOLLECTION)\s*(Z|M|ZM)?\s*(\(|EMPTY)`
- **DuckDB:** `INSTALL spatial; ST_GeomFromText(col)`
- **Standard:** OGC Simple Features / ISO 19125 / ISO 19162
- **Prevalence:** High — PostGIS, Shapefile pipelines, Parquet geo columns
- **Priority:** 4

### 4. `geography.format.geojson`

JSON objects with `"type"` and `"coordinates"` keys encoding geographic features.

- **Examples:** `{"type":"Point","coordinates":[-73.9857,40.7484]}`
- **Validation:** Valid JSON where `$.type` ∈ {Point, LineString, Polygon, MultiPoint, MultiLineString, MultiPolygon, GeometryCollection, Feature, FeatureCollection}
- **DuckDB:** `INSTALL spatial; ST_GeomFromGeoJSON(col)`
- **Standard:** RFC 7946 (IETF, 2016)
- **Prevalence:** High — dominant web mapping interchange format
- **Priority:** 4

### 5. `container.object.yaml_ext`

YAML documents with native DuckDB extraction support via community extension. Note: `container.object.yaml` already exists in the taxonomy for YAML detection. This entry is about upgrading the transform to use the extension's `yaml_extract()` function rather than adding a new type. **Action: update existing type's `transform_ext` field, not a new type.**

- **DuckDB:** `INSTALL yaml FROM community; yaml_extract(col, '$.key')`
- **Standard:** YAML 1.2

### 6. `technology.identifier.tsid`

Time-Sorted Unique Identifier. 32-character hex string with embedded timestamp.

- **Examples:** `675716e86985495e9cf575f0b9c4a8db`
- **Regex:** `^[0-9a-f]{32}$` (no hyphens — distinguishes from UUID)
- **DuckDB:** `INSTALL tsid FROM community; tsid_to_timestamp(col)`
- **Standard:** tsid-creator library (Java/PHP)
- **Prevalence:** Medium — Java/PHP applications
- **Priority:** 3
- **Disambiguation note:** 32 hex chars without hyphens. UUID has hyphens (8-4-4-4-12). MD5 hash is also 32 hex chars — needs confidence gating.

**Net new from Tier 1: 5** (YAML is an update to existing type, not a new type)

---

## Tier 2 — International Standard Identifiers (20)

### Healthcare & Medical (5)

#### 7. `identity.medical.icd10`

International Classification of Diseases, 10th Revision. Letter prefix + 2 digits + optional decimal + up to 4 additional characters.

- **Examples:** `E11.9` (Type 2 diabetes), `J18.9` (pneumonia), `M54.5` (low back pain)
- **Regex:** `^[A-TV-Z][0-9][0-9AB](\.[0-9A-TV-Z]{1,4})?$`
- **DuckDB:** `regexp_extract(col, '^([A-TV-Z])(\d[0-9AB])\.?(.*)$', ['chapter','category','detail'])`
- **Standard:** WHO ICD-10 / CMS ICD-10-CM
- **Prevalence:** High — every medical claim globally
- **Priority:** 4

#### 8. `identity.medical.loinc`

Logical Observation Identifiers Names and Codes. 1–5 digit code, hyphen, single check digit.

- **Examples:** `2951-2` (sodium), `718-7` (hemoglobin), `4548-4` (HbA1c)
- **Regex:** `^\d{1,5}-\d$`
- **DuckDB:** `regexp_extract(col, '^(\d{1,5})-(\d)$', ['code','check_digit'])`
- **Standard:** Regenstrief Institute LOINC (HIPAA-mandated)
- **Prevalence:** High — every US electronic lab result
- **Priority:** 3

#### 9. `identity.medical.cpt`

Current Procedural Terminology. 5-digit codes or 4 digits + letter suffix.

- **Examples:** `99213` (office visit), `29580` (Unna boot), `2029F` (Cat II), `0307T` (Cat III)
- **Regex:** `^\d{5}$|^\d{4}[FTU]$`
- **DuckDB:** `CASE WHEN col ~ '^\d{5}$' THEN 'Category I' WHEN col ~ '^\d{4}F$' THEN 'Category II' WHEN col ~ '^\d{4}T$' THEN 'Category III' END`
- **Standard:** AMA CPT (HCPCS Level I)
- **Prevalence:** High — every US medical procedure
- **Priority:** 3
- **Disambiguation note:** Plain 5-digit numbers overlap with postal codes and generic integers. Needs header hints or column-level disambiguation.

#### 10. `identity.medical.hcpcs`

Healthcare Common Procedure Coding System Level II. Single uppercase letter (A–V) + 4 digits.

- **Examples:** `J1234` (drug injection), `E0114` (crutches), `A4253` (glucose strips)
- **Regex:** `^[A-V]\d{4}$`
- **DuckDB:** `regexp_extract(col, '^([A-V])(\d{4})$', ['category','code'])`
- **Standard:** CMS HCPCS Level II
- **Prevalence:** High — Medicare/Medicaid billing
- **Priority:** 3

#### 11. `identity.medical.ndc` *(already exists)*

**Action: no change needed.** Already in taxonomy as `identity.medical.ndc`.

### Logistics & Supply Chain (3)

#### 12. `geography.transportation.iso6346`

Shipping container identification. 3-letter owner code + category letter (U/J/Z) + 6-digit serial + 1 check digit = 11 characters.

- **Examples:** `MSCU1234567`, `CSQU3054383`, `TEXU3070079`
- **Regex:** `^[A-Z]{3}[UJZ]\d{7}$`
- **DuckDB:** `regexp_extract(col, '^([A-Z]{3})([UJZ])(\d{6})(\d)$', ['owner','category','serial','check'])`
- **Standard:** ISO 6346 / Bureau International des Containers (BIC)
- **Prevalence:** High — every intermodal shipping container globally
- **Priority:** 4

#### 13. `geography.transportation.hs_code`

Harmonized System tariff classification. 6–10 digits, often dot-separated.

- **Examples:** `090210`, `1806.31.00`, `8517.12.00.41`
- **Regex:** `^\d{4}\.?\d{2}(\.?\d{2}){0,2}$`
- **DuckDB:** `regexp_extract(regexp_replace(col, '\.', '', 'g'), '^(\d{2})(\d{2})(\d{2})', ['chapter','heading','subheading'])`
- **Standard:** WCO Harmonized System
- **Prevalence:** High — every international trade transaction
- **Priority:** 4

#### 14. `geography.transportation.unlocode`

UN Code for Trade and Transport Locations. 2-letter country code + 3-character location code.

- **Examples:** `USLAX` (Los Angeles), `DEHAM` (Hamburg), `CNSHA` (Shanghai)
- **Regex:** `^[A-Z]{2}[A-Z2-9]{3}$`
- **DuckDB:** `regexp_extract(col, '^([A-Z]{2})([A-Z2-9]{3})$', ['country','location'])`
- **Standard:** UNECE Recommendation 16
- **Prevalence:** High — shipping, customs, trade
- **Priority:** 3
- **Disambiguation note:** 5 uppercase letters overlap with IATA codes (3 chars) and country codes (2 chars). Column-level disambiguation needed.

### Government & Tax Identifiers (4)

#### 15. `identity.government.vin`

Vehicle Identification Number. 17 alphanumeric characters (excluding I, O, Q). Position 9 is check digit.

- **Examples:** `1HGBH41JXMN109186`, `WVWZZZ3CZWE123456`
- **Regex:** `^[A-HJ-NPR-Z0-9]{17}$`
- **DuckDB:** `regexp_extract(col, '^([A-HJ-NPR-Z0-9]{3})([A-HJ-NPR-Z0-9]{5})([0-9X])([A-HJ-NPR-Z0-9])([A-HJ-NPR-Z0-9])([0-9]{6})$', ['wmi','vds','check','year','plant','seq'])`
- **Standard:** ISO 3779:2009 / NHTSA FMVSS 115
- **Prevalence:** High — every vehicle manufactured since 1981
- **Priority:** 4

#### 16. `identity.government.eu_vat`

EU Value Added Tax identification number. 2-letter country prefix + 2–13 alphanumeric characters.

- **Examples:** `DE123456789`, `FR12345678901`, `ATU12345678`, `NL123456789B01`
- **Regex:** `^[A-Z]{2}[0-9A-Za-z+*.]{2,12}$`
- **DuckDB:** `regexp_extract(col, '^([A-Z]{2})(.+)$', ['country','number'])`
- **Standard:** EU Council Directive 2006/112/EC (VIES)
- **Prevalence:** High — every B2B EU transaction
- **Priority:** 3
- **Designation:** `locale_specific`

#### 17. `identity.government.ssn`

US Social Security Number. 9 digits formatted `###-##-####` with exclusion rules.

- **Examples:** `078-05-1120`, `219-09-9999`
- **Regex:** `^(?!000|666|9\d{2})\d{3}-(?!00)\d{2}-(?!0000)\d{4}$`
- **DuckDB:** `regexp_extract(col, '^(\d{3})-(\d{2})-(\d{4})$', ['area','group','serial'])`
- **Standard:** SSA (Social Security Administration)
- **Prevalence:** High — US tax, employment, benefits
- **Priority:** 3
- **Designation:** `locale_specific` (EN_US)
- **Security note:** PII-sensitive. Consider whether detection should trigger a warning.

#### 18. `identity.government.ein`

Employer Identification Number. 9 digits formatted `##-#######`.

- **Examples:** `12-3456789`, `91-1234567`
- **Regex:** `^\d{2}-\d{7}$`
- **DuckDB:** `regexp_extract(col, '^(\d{2})-(\d{7})$', ['campus','number'])`
- **Standard:** IRS (Internal Revenue Service)
- **Prevalence:** High — every US business entity
- **Priority:** 3
- **Designation:** `locale_specific` (EN_US)

### Cloud & DevOps (3)

#### 19. `technology.cloud.aws_arn`

Amazon Resource Name. Colon-delimited: `arn:partition:service:region:account-id:resource`.

- **Examples:** `arn:aws:s3:::my-bucket`, `arn:aws:iam::123456789012:user/johndoe`
- **Regex:** `^arn:(aws|aws-cn|aws-us-gov):[a-zA-Z0-9\-]+:[a-z0-9\-]*:\d{0,12}:.+$`
- **DuckDB:** `regexp_extract(col, '^arn:([^:]+):([^:]+):([^:]*):([^:]*):(.+)$', ['partition','service','region','account','resource'])`
- **Standard:** AWS proprietary
- **Prevalence:** High — every AWS resource, CloudTrail logs, IAM policies
- **Priority:** 4

#### 20. `technology.cloud.s3_uri`

S3 bucket/key URI format.

- **Examples:** `s3://my-bucket/path/to/file.csv`, `s3://data-lake/year=2024/data.parquet`
- **Regex:** `^s3://[a-z0-9][a-z0-9.\-]{1,61}[a-z0-9](/.*)?$`
- **DuckDB:** `regexp_extract(col, '^s3://([^/]+)(/.*)?$', ['bucket','key'])`
- **Standard:** AWS proprietary convention
- **Prevalence:** High — data engineering, ML pipelines, data lakes
- **Priority:** 4

#### 21. `technology.development.docker_ref`

Docker/OCI image reference: `[registry/]repository[:tag][@digest]`.

- **Examples:** `nginx:latest`, `gcr.io/my-project/my-app:v1.2.3`, `ubuntu:22.04`
- **Regex:** `^(?:([a-zA-Z0-9.\-]+(?::\d+)?)/)?([a-z0-9._\-/]+)(?::([a-zA-Z0-9_.\-]+))?(?:@(sha256:[a-fA-F0-9]{64}))?$`
- **DuckDB:** `regexp_extract(col, '^(?:([^/]+)/)?(.+?)(?::([^@]+))?(?:@(.+))?$', ['registry','repo','tag','digest'])`
- **Standard:** OCI Distribution Specification
- **Prevalence:** High — every containerised deployment, Kubernetes manifests
- **Priority:** 3

### Scientific (4)

#### 22. `representation.scientific.cas_number`

Chemical Abstracts Service registry number. 2–7 digits, hyphen, 2 digits, hyphen, 1 check digit.

- **Examples:** `7732-18-5` (water), `64-17-5` (ethanol), `50-78-2` (aspirin)
- **Regex:** `^\d{2,7}-\d{2}-\d$`
- **DuckDB:** `regexp_extract(col, '^(\d{2,7})-(\d{2})-(\d)$', ['body','group','check'])`
- **Standard:** CAS / ASTM E1154
- **Prevalence:** High — chemistry, pharma, materials science, regulatory (REACH, GHS)
- **Priority:** 4

#### 23. `identity.academic.orcid`

Open Researcher and Contributor Identifier. 16 digits in 4 groups of 4, hyphen-separated. Last character is check digit (0–9 or X).

- **Examples:** `0000-0002-1825-0097`, `0000-0001-5109-3700`, `0000-0002-9079-593X`
- **Regex:** `^\d{4}-\d{4}-\d{4}-\d{3}[\dX]$`
- **DuckDB:** `regexp_extract(col, '^(\d{4})-(\d{4})-(\d{4})-(\d{3}[\dX])$', ['b1','b2','b3','b4'])`
- **Standard:** ISO 27729 (ISNI subset)
- **Prevalence:** High — 18M+ researchers; required by major journals/funders
- **Priority:** 4

#### 24. `representation.scientific.inchi`

International Chemical Identifier. Layered molecular structure text, always starting with `InChI=1S/` or `InChI=1/`.

- **Examples:** `InChI=1S/H2O/h1H2` (water), `InChI=1S/C2H6O/c1-2-3/h3H,2H2,1H3` (ethanol)
- **Regex:** `^InChI=1S?/.+$`
- **DuckDB:** `regexp_extract(col, '^InChI=(1S?)/([^/]+)', ['version','formula'])`
- **Standard:** IUPAC/InChI Trust
- **Prevalence:** High — PubChem, ChEBI, every chemical database
- **Priority:** 3

#### 25. `representation.scientific.smiles`

Simplified Molecular-Input Line-Entry System. Compact linear notation for chemical structures.

- **Examples:** `O` (water), `CCO` (ethanol), `CC(=O)Oc1ccccc1C(=O)O` (aspirin)
- **Regex:** `^[A-Za-z0-9@+\-\[\]\(\)\\\/#=%.:]+$` (approximate — context-free grammar)
- **DuckDB:** `regexp_matches(col, '^[BCNOPSFIbcnops][A-Za-z0-9@+\-\[\]\(\)\\\/#=%.:]*$')`
- **Standard:** OpenSMILES specification
- **Prevalence:** High — drug discovery, cheminformatics
- **Priority:** 2
- **Disambiguation note:** Short SMILES strings (e.g., `O`, `N`) overlap with plain text. Column-level disambiguation essential. Designation: `broad_characters`.

### Finance (1 new)

#### 26. `finance.securities.figi`

Financial Instrument Global Identifier. 12 characters: 2 consonants + `G` + 8 alphanumeric (no vowels) + Luhn check digit.

- **Examples:** `BBG000BLNQ16` (Apple), `BBG000B9XRY4`, `BBG000BVPV84` (Amazon)
- **Regex:** `^[BCDFGHJKLMNPQRSTVWXYZ]{2}G[BCDFGHJKLMNPQRSTVWXYZ0-9]{8}\d$`
- **DuckDB:** `regexp_matches(col, '^[BCDFGHJKLMNPQRSTVWXYZ]{2}G[BCDFGHJKLMNPQRSTVWXYZ0-9]{8}\d$')`
- **Standard:** OMG / ANSI X9.145-2021 (Bloomberg as RA; OpenFIGI open data)
- **Prevalence:** High — 300M+ active FIGIs; SEC Form 13F, FINRA TRACE
- **Priority:** 4

**Net new from Tier 2: 19** (NDC already exists, excluded)

---

## Tier 3 — Common Format Patterns (10)

### Geospatial Encodings (4)

#### 27. `geography.coordinate.geohash`

Base-32 encoded rectangular area. 4–12 characters, precision increases with length.

- **Examples:** `u4pruydqqvj`, `9q8yyk8yuv`, `dr5ru6j6v`, `gcpuuz`
- **Regex:** `^[0-9b-hjkmnp-z]{4,12}$` (base-32 alphabet excluding a, i, l, o)
- **DuckDB:** `regexp_matches(col, '^[0-9b-hjkmnp-z]{4,12}$')`
- **Standard:** Public domain (Gustavo Niemeyer, 2008); Elasticsearch, MongoDB, Redis
- **Prevalence:** High — spatial indexing, proximity search
- **Priority:** 3

#### 28. `geography.coordinate.plus_code`

Google's Open Location Code. 8 chars + `+` + 2+ refinement chars.

- **Examples:** `8FVC9G8F+5W`, `6GCRPR6C+24`, `849VCWC8+R9`
- **Regex:** `^[23456789CFGHJMPQRVWX]{8}\+[23456789CFGHJMPQRVWX]{2,}$`
- **DuckDB:** `regexp_matches(col, '^[23456789CFGHJMPQRVWX]{8}\+[23456789CFGHJMPQRVWX]{2,}$')`
- **Standard:** Google open-source (2014), adopted by Google Maps
- **Prevalence:** Medium — growing via Google Maps, addressing in developing countries
- **Priority:** 2

#### 29. `geography.coordinate.dms`

Degrees/Minutes/Seconds notation with cardinal directions.

- **Examples:** `40°26'46"N 79°58'56"W`, `51°30'26"N 0°7'39"W`
- **Regex:** `^\d{1,3}°\d{1,2}'\d{1,2}(\.\d+)?"[NS]\s+\d{1,3}°\d{1,2}'\d{1,2}(\.\d+)?"[EW]$`
- **DuckDB:** Complex `regexp_extract` decomposition → arithmetic conversion to decimal degrees
- **Standard:** ISO 6709 (traditional cartographic convention)
- **Prevalence:** High — maps, GPS devices, geographic references
- **Priority:** 3

#### 30. `geography.coordinate.mgrs`

Military Grid Reference System. Zone + band + grid square + easting/northing.

- **Examples:** `4QFJ12345678`, `18SUJ2338308676`, `33UUP0490`
- **Regex:** `^\d{1,2}[C-X][A-HJ-NP-Z]{2}\d{2,10}$`
- **DuckDB:** `regexp_extract(col, '^(\d{1,2})([C-X])([A-HJ-NP-Z]{2})(\d+)$', ['zone','band','square','coords'])`
- **Standard:** NATO STANAG 2211 / NGA
- **Prevalence:** Medium — military, NATO, emergency services
- **Priority:** 2

### Web & Internet (4)

#### 31. `technology.internet.cidr`

CIDR network notation. IP address + `/` + prefix length.

- **Examples:** `192.168.1.0/24`, `10.0.0.0/8`, `2001:db8::/32`
- **Regex:** `^(\d{1,3}\.){3}\d{1,3}/([0-9]|[12]\d|3[0-2])$` (IPv4)
- **DuckDB:** `regexp_extract(col, '^(.+)/(\d+)$', ['network','prefix_len'])`
- **Standard:** RFC 4632
- **Prevalence:** High — every network ACL, firewall rule, security group
- **Priority:** 4

#### 32. `technology.internet.urn`

Uniform Resource Name. Persistent, location-independent identifier with `urn:` prefix.

- **Examples:** `urn:isbn:0451450523`, `urn:ietf:rfc:2648`, `urn:oid:2.16.840`
- **Regex:** `^urn:[a-z0-9][a-z0-9\-]{0,31}:.+$`
- **DuckDB:** `regexp_extract(col, '^urn:([^:]+):(.+)$', ['nid','nss'])`
- **Standard:** RFC 8141 (IETF)
- **Prevalence:** Medium — XML namespaces, FHIR, W3C standards
- **Priority:** 2

#### 33. `technology.internet.data_uri`

Inline data embedded in URI format with media type and optional base64 encoding.

- **Examples:** `data:text/plain;base64,SGVsbG8=`, `data:image/png;base64,iVBOR...`
- **Regex:** `^data:([a-zA-Z0-9]+/[a-zA-Z0-9\-+.]+)?(;[a-zA-Z0-9\-]+=[^;,]*)*(;base64)?,.+$`
- **DuckDB:** `regexp_extract(col, '^data:([^;,]*)?(?:;(base64))?,(.+)$', ['mediatype','encoding','data'])`
- **Standard:** RFC 2397
- **Prevalence:** High — embedded images in HTML/CSS, email, JSON payloads
- **Priority:** 3

#### 34. `identity.person.email_display`

Email address with display name per RFC 5322 mailbox format.

- **Examples:** `"John Doe" <john@example.com>`, `Jane Smith <jane@corp.com>`
- **Regex:** `^"?[^"<>]+"?\s*<[a-zA-Z0-9._%+\-]+@[a-zA-Z0-9.\-]+\.[a-zA-Z]{2,}>$`
- **DuckDB:** `regexp_extract(col, '<([^>]+)>') AS email, regexp_extract(col, '^"?([^"<]+)"?\s*<') AS display_name`
- **Standard:** RFC 5322 (Internet Message Format)
- **Prevalence:** High — email systems, CRM, contact exports
- **Priority:** 3

### Temporal & Locale (2)

#### 35. `datetime.duration.iso_8601_verbose`

ISO 8601 duration with named components (distinct from existing `datetime.duration.iso_8601` which may only cover the compact form).

- **Examples:** `P1Y2M3DT4H5M6S`, `PT30M`, `P2W`, `P1DT12H`
- **Regex:** `^(-?)P(?=\d|T\d)(?:(\d+)Y)?(?:(\d+)M)?(?:(\d+)([DW]))?(?:T(?:(\d+)H)?(?:(\d+)M)?(?:(\d+(?:\.\d+)?)S)?)?$`
- **DuckDB:** `regexp_extract` decomposition into year/month/day/hour/minute/second components
- **Standard:** ISO 8601-1:2019 §5.5.2
- **Prevalence:** High — YouTube API, Google Calendar, XML Schema, iCalendar
- **Priority:** 4
- **Note:** Verify whether existing `datetime.duration.iso_8601` already covers this exact format. If so, skip.

#### 36. `technology.code.bcp47`

Language tag per BCP 47: `language[-script][-region][-variant]`.

- **Examples:** `en-US`, `zh-Hans-CN`, `sr-Latn-RS`, `pt-BR`
- **Regex:** `^[a-zA-Z]{2,3}(-[a-zA-Z]{4})?(-([a-zA-Z]{2}|\d{3}))?(-([a-zA-Z\d]{5,8}|\d[a-zA-Z\d]{3}))*$`
- **DuckDB:** `regexp_extract(col, '^([a-zA-Z]{2,3})(?:-([a-zA-Z]{4}))?(?:-([a-zA-Z]{2}|\d{3}))?', ['lang','script','region'])`
- **Standard:** BCP 47 (RFC 5646 + RFC 4647)
- **Prevalence:** High — every multilingual app, HTTP headers, CMS
- **Priority:** 3
- **Note:** Check overlap with existing `technology.code.locale_code`. BCP 47 (`en-US`) vs POSIX locale (`en_US.UTF-8`) — if both are covered by `locale_code`, skip. If `locale_code` is POSIX only, BCP 47 is a distinct format.

**Net new from Tier 3: 10** (pending dedup checks on duration and BCP 47)

---

## Tier 4 — Additional Strong Candidates (11)

From second-pass research. All have discrete formats and clear DuckDB transforms.

#### 37. `technology.cryptographic.jwt`

JSON Web Token. Three base64url segments separated by dots.

- **Examples:** `eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U`
- **Regex:** `^[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+$`
- **DuckDB:** `from_json(decode(split_part(col, '.', 2)))` — extracts payload as JSON
- **Standard:** RFC 7519 (IETF)
- **Prevalence:** High — web authentication, API logs, security datasets
- **Priority:** 4
- **Note:** Distinct from existing `token_hex` (hex chars only) and `token_urlsafe` (single segment).

#### 38. `technology.development.git_sha`

Git commit hash. Exactly 40 lowercase hex characters (full) or 7+ abbreviated.

- **Examples:** `a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0`
- **Regex (full):** `^[0-9a-f]{40}$`
- **DuckDB:** `substring(col, 1, 7) AS short_sha`
- **Prevalence:** High — CI/CD logs, version control exports, development datasets
- **Priority:** 3
- **Disambiguation:** Exactly 40 hex chars. Distinguished from SHA-1 hash (same length) by column context (header hints: "commit", "sha", "revision").

#### 39. `technology.identifier.snowflake_id`

Twitter/Discord-style Snowflake ID. 17–20 digit integer with embedded millisecond timestamp.

- **Examples:** `1766611625139814401`, `175928847299117063`
- **Regex:** `^\d{17,20}$`
- **DuckDB:** `epoch_ms((col::BIGINT >> 22) + 1288834974657)` (Twitter epoch)
- **Prevalence:** High — Twitter/X, Discord, Mastodon, Instagram
- **Priority:** 3
- **Disambiguation:** Very long integers. Overlaps with large increment IDs. Column-level disambiguation needed.

#### 40. `identity.person.phone_e164`

Strict international phone format: `+` prefix + 7–15 digits.

- **Examples:** `+61412345678`, `+14155552671`, `+442071234567`
- **Regex:** `^\+[1-9]\d{6,14}$`
- **DuckDB:** `regexp_extract(col, '^\+(\d{1,3})(\d+)$', ['country_code','subscriber'])` (approximate)
- **Standard:** ITU-T E.164
- **Prevalence:** High — universal phone number format
- **Priority:** 4
- **Note:** Distinct from locale-specific `phone_number` which allows formatting variations. E.164 is the strict canonical form.

#### 41. `representation.text.color_hsl`

HSL/HSLA CSS colour notation.

- **Examples:** `hsl(120, 100%, 50%)`, `hsla(240, 100%, 50%, 0.5)`
- **Regex:** `^hsla?\(\s*\d{1,3}\s*,\s*\d{1,3}%\s*,\s*\d{1,3}%\s*(,\s*[\d.]+\s*)?\)$`
- **DuckDB:** `regexp_extract(col, 'hsla?\(\s*(\d+)\s*,\s*(\d+)%\s*,\s*(\d+)%', ['hue','saturation','lightness'])`
- **Prevalence:** Medium — CSS, design tools, frontend datasets
- **Priority:** 2
- **Note:** Completes the colour triple: `color_hex`, `color_rgb`, `color_hsl`.

#### 42. `identity.government.pan_india`

Indian Permanent Account Number. 5 letters + 4 digits + 1 letter, 4th char indicates entity type.

- **Examples:** `ABCPD1234E`, `AAACB0000C`
- **Regex:** `^[A-Z]{5}\d{4}[A-Z]$`
- **DuckDB:** `regexp_extract(col, '^([A-Z]{3})([A-Z])([A-Z])(\d{4})([A-Z])$', ['area','type','initial','number','check'])`
- **Standard:** Indian Income Tax Department
- **Prevalence:** High — every Indian financial transaction
- **Priority:** 3
- **Designation:** `locale_specific` (EN_IN)

#### 43. `identity.government.abn`

Australian Business Number. 11 digits with weighted checksum (modulus 89).

- **Examples:** `51 824 753 556`, `53004085616`
- **Regex:** `^\d{2}\s?\d{3}\s?\d{3}\s?\d{3}$`
- **DuckDB:** `regexp_replace(col, '\s', '', 'g')::BIGINT`
- **Standard:** Australian Business Register (ABR)
- **Prevalence:** High — every Australian invoice and business document
- **Priority:** 3
- **Designation:** `locale_specific` (EN_AU)

#### 44. `finance.banking.bsb`

Australian Bank-State-Branch number. 6 digits formatted `###-###`.

- **Examples:** `062-000` (CBA), `033-001` (Westpac), `012-003` (ANZ)
- **Regex:** `^\d{3}-\d{3}$`
- **DuckDB:** `regexp_extract(col, '^(\d{3})-(\d{3})$', ['bank_state','branch'])`
- **Standard:** Australian Payments Network
- **Prevalence:** High — every Australian bank transfer
- **Priority:** 3
- **Designation:** `locale_specific` (EN_AU)

#### 45. `finance.banking.aba_routing`

US ABA routing transit number. 9 digits with weighted checksum.

- **Examples:** `021000021` (JPMorgan), `111000025` (BoA)
- **Regex:** `^(0[0-9]|1[0-2]|2[1-9]|3[0-2]|6[1-9]|7[0-2]|80)\d{7}$`
- **DuckDB:** Checksum validation via substring extraction and arithmetic
- **Standard:** American Bankers Association (1910)
- **Prevalence:** High — every US check, ACH, wire transfer
- **Priority:** 3
- **Designation:** `locale_specific` (EN_US)

#### 46. `identity.commerce.upc`

Universal Product Code (UPC-A). 12 digits with MOD-10 check digit.

- **Examples:** `042100005264`, `883028594054`
- **Regex:** `^\d{12}$`
- **DuckDB:** `'0' || lpad(col, 12, '0') AS ean13_equivalent`
- **Standard:** GS1 GTIN-12
- **Prevalence:** High — billions of retail products
- **Priority:** 3
- **Note:** UPC = EAN-13 with leading zero. Distinct from existing `technology.code.ean` (13 digits).

#### 47. `identity.commerce.isrc`

International Standard Recording Code. 12 characters: 2-letter country + 3-char registrant + 2-digit year + 5-digit designation.

- **Examples:** `USUAN1400011`, `GBAYE7700223`, `QMDA71500001`
- **Regex:** `^[A-Z]{2}[A-Z0-9]{3}\d{7}$`
- **DuckDB:** `regexp_extract(col, '^([A-Z]{2})([A-Z0-9]{3})(\d{2})(\d{5})$', ['country','registrant','year','designation'])`
- **Standard:** ISO 3901:2019 (IFPI)
- **Prevalence:** High — 150M+ ISRCs; required by Spotify, Apple Music, all streaming
- **Priority:** 3

**Net new from Tier 4: 11**

---

## Tier 5 — Conditional / Lower Priority (~10)

These are viable but have narrower scope, harder disambiguation, or weaker transformation value. Include only if headroom permits after Tiers 1–4.

| # | Proposed type | Format | Concern |
|---|---|---|---|
| 48 | `container.object.toml` | TOML config files | Growing fast (Cargo.toml, pyproject.toml) but no DuckDB extension yet |
| 49 | `geography.coordinate.wkb` | Well-Known Binary (hex) | Paired with WKT. `ST_GeomFromWKB()`. Appears in PostGIS exports |
| 50 | `geography.location.country_code_alpha3` | 3-letter ISO 3166-1 | `AUS`, `GBR`, `USA`. Check if existing `country_code` is alpha-2 only |
| 51 | `technology.development.git_sha` | 40-char hex | See #38 above — overlap with SHA-1 hash |
| 52 | `technology.code.xpath` | XPath expressions | `/html/body/div[1]`. Niche but discrete pattern |
| 53 | `representation.text.roman_numeral` | Roman numerals | `XIV`, `MCMXCIX`. DuckDB transform to integer. Fills the gap left by removed `century` |
| 54 | `representation.numeric.hex_integer` | `0xFF`, `0x1A2B` | Common in programming. Distinct from hash (numeric value, not fixed-length digest) |
| 55 | `identity.government.uk_nino` | UK National Insurance | `AB123456C`. Highly structured but UK-specific |
| 56 | `technology.identifier.ssh_pubkey` | SSH public key | `ssh-rsa AAAA...`, `ssh-ed25519 AAAA...`. Distinctive prefix |
| 57 | `geography.index.a5` | A5 pentagonal index | New DuckDB extension. Equal-area, millimetre-accurate. Very new |

---

## Recommended Domain Placement

The new types slot into the existing domain structure as follows. Types marked with `*` require a new category.

### geography (15 → 23, +8)
- `geography.coordinate.geohash` (new)
- `geography.coordinate.plus_code` (new)
- `geography.coordinate.dms` (new)
- `geography.coordinate.mgrs` (new)
- `geography.format.wkt` (new category: `format`)*
- `geography.format.geojson` (new category: `format`)*
- `geography.index.h3` (new category: `index`)*
- `geography.transportation.iso6346` (new)
- `geography.transportation.hs_code` (new)
- `geography.transportation.unlocode` (new)

### technology (24 → 34, +10)
- `technology.identifier.ulid` (new category: `identifier`)*
- `technology.identifier.tsid` (new)
- `technology.identifier.snowflake_id` (new)
- `technology.cryptographic.jwt` (new)
- `technology.development.docker_ref` (new)
- `technology.development.git_sha` (new)
- `technology.internet.cidr` (new)
- `technology.internet.urn` (new)
- `technology.internet.data_uri` (new)
- `technology.cloud.aws_arn` (new category: `cloud`)*
- `technology.cloud.s3_uri` (new)
- `technology.code.bcp47` (new — verify vs `locale_code`)

### identity (19 → 31, +12)
- `identity.medical.icd10` (new)
- `identity.medical.loinc` (new)
- `identity.medical.cpt` (new)
- `identity.medical.hcpcs` (new)
- `identity.academic.orcid` (new category: `academic`)*
- `identity.government.vin` (new category: `government`)*
- `identity.government.eu_vat` (new)
- `identity.government.ssn` (new)
- `identity.government.ein` (new)
- `identity.government.pan_india` (new)
- `identity.government.abn` (new)
- `identity.person.email_display` (new)
- `identity.person.phone_e164` (new)
- `identity.commerce.upc` (new)
- `identity.commerce.isrc` (new)

### finance (29 → 32, +3)
- `finance.securities.figi` (new)
- `finance.banking.aba_routing` (new)
- `finance.banking.bsb` (new)

### representation (32 → 35, +3)
- `representation.scientific.cas_number` (new)
- `representation.scientific.inchi` (new)
- `representation.scientific.smiles` (new)
- `representation.text.color_hsl` (new)

### datetime (85 → 86, +1)
- `datetime.duration.iso_8601_verbose` (new — verify vs existing duration type)

---

## Implementation Notes

### Priority order for implementation

1. **DuckDB-native types first** (ULID, H3, WKT, GeoJSON) — strongest transformation contracts, native `TRY_CAST`
2. **High-distinctiveness identifiers** (VIN, ORCID, CAS number, FIGI, ISO 6346, AWS ARN) — highly unique regex patterns, minimal false positives
3. **Network/web patterns** (CIDR, JWT, S3 URI, data URI) — very common in tech datasets, unambiguous prefixes
4. **Healthcare codes** (ICD-10, LOINC, CPT, HCPCS) — enormous prevalence but some need column-level disambiguation
5. **Government/tax IDs** (SSN, EIN, EU VAT, PAN, ABN) — high value but locale-specific and some are PII-sensitive
6. **Geospatial encodings** (geohash, plus code, DMS, MGRS) — solid formats, moderate prevalence
7. **Lower priority** (SMILES, TOML, roman numerals, hex integers) — niche audience or harder disambiguation

### Disambiguation concerns

Several candidates share character space with existing types or generic patterns:

| Candidate | Overlaps with | Resolution |
|---|---|---|
| TSID (32 hex) | MD5 hash (32 hex) | Length match but different distribution. Timestamp extraction as tiebreaker |
| CPT (5 digits) | postal codes, integers | Header hints: "procedure", "cpt", "code" |
| Snowflake ID (17–20 digits) | large integers | Timestamp extraction validates. Header hints help |
| SMILES | plain text | `broad_characters` designation. Column-level only |
| LOINC (N-N) | generic codes | Header hints: "loinc", "test_code", "lab_code" |
| UNLOCODE (5 uppercase) | short text, IATA | Length differs from IATA (3). Header hints help |

### New categories required

| Domain | New category | Types |
|---|---|---|
| geography | `format` | wkt, geojson |
| geography | `index` | h3 |
| technology | `identifier` | ulid, tsid, snowflake_id |
| technology | `cloud` | aws_arn, s3_uri |
| identity | `government` | vin, eu_vat, ssn, ein, pan_india, abn |
| identity | `academic` | orcid |

### Running totals

| Tier | New types | Running total |
|------|-----------|---------------|
| Current taxonomy | — | 216 |
| Tier 1 (DuckDB native) | +5 | 221 |
| Tier 2 (Standard identifiers) | +19 | 240 |
| Tier 3 (Format patterns) | +10 | 250 |
| Tier 4 (Additional strong) | +11 | 261 |
| Tier 5 (Conditional) | +~10 | ~271 |

**Tiers 1–3 alone reach exactly 250.** Tier 4 provides comfortable headroom for any that fail the precision test during generator implementation.
