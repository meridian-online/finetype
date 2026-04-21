# Eval Ground Truth Quality Audit v2

**Date:** 2026-04-12
**Baseline:** 193/227 (85.0%) -- v4-sibling on 227-column eval set
**Model:** sherlock-v4-sibling (250-class label space)

## Summary Table

| # | Dataset | Column | Predicted | Expected | Verdict | Rationale |
|---|---------|--------|-----------|----------|---------|-----------|
| 1 | new_geography | geojson | geography.format.geojson | container.object.json | DEBATABLE | Model predicted the MORE SPECIFIC type -- GeoJSON is a subtype of JSON. Ground truth maps geojson GT to `container.object.json`, but the model has a dedicated `geography.format.geojson` label that is arguably more correct. |
| 2 | codes_and_ids | sha256 | technology.development.git_sha | technology.cryptographic.hash | WRONG | Data is 64-char hex SHA-256 hashes. git_sha is 40-char hex (SHA-1). The lengths don't match. Model should predict `hash`. |
| 3 | datetime_formats | year | datetime.date.compact_ym | datetime.component.year | WRONG | Values are 4-digit years (2020-2023). compact_ym is YYYYMM (6-digit). 4-digit values cannot be compact_ym. |
| 4 | finance_coverage | bitcoin_address | geography.address.full_address | finance.crypto.bitcoin_address | WRONG | Values like `1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa` and `bc1q...` are clearly Bitcoin addresses, not street addresses. Wrong domain entirely. |
| 5 | new_technology | git_sha | technology.development.git_sha | technology.cryptographic.hash | DEBATABLE | Values are 40-char hex strings. GT says `hash`, model says `git_sha`. git_sha IS a hash (SHA-1). The GT label is the generic parent; the prediction is the specific subtype. Both are defensible -- `git_sha` is more specific and format-correct (40 hex chars = SHA-1). |
| 6 | tech_systems | user_agent | technology.cryptographic.jwt | technology.internet.user_agent | WRONG | Values are clearly Mozilla/5.0 browser user-agent strings. JWT prediction is completely wrong domain. |
| 7 | earthquakes_2024 | depthError | geography.coordinate.latitude | representation.numeric.decimal_number | WRONG | Values like 7.431, 8.251, 11.011 are decimal error values. Latitude prediction is wrong -- these are not coordinates. |
| 8 | airports | icao | geography.transportation.unlocode | geography.transportation.icao_code | WRONG | Values like AYGA, AYMD, BGBW are 4-letter ICAO codes. UNLOCODE is 5-char (2-letter country + 3-letter location). Both are in same domain but wrong specific type. |
| 9 | ecommerce_orders | phone | identity.government.ssn | identity.person.phone_number | WRONG | Values like `+1-232-130-2535` are clearly phone numbers with country code. SSN prediction is wrong -- SSNs don't have `+` prefix or country codes. |
| 10 | network_logs | status_code | geography.address.postal_code | representation.numeric.integer_number | WRONG | Values 200-503 are HTTP status codes. Postal code prediction is wrong domain entirely. Expected integer_number is correct. |
| 11 | tech_systems | server_hostname | technology.internet.url | technology.internet.hostname | WRONG | Values like `srv-dev-43.example.com` are hostnames, not full URLs (no scheme like `https://`). Both are technology.internet but wrong specific type. |
| 12 | people_directory | phone | identity.government.ssn | identity.person.phone_number | WRONG | Values like `+61-392-253-9475` are international phone numbers. SSN prediction is completely wrong. |
| 13 | earthquakes_2024 | place | geography.location.region | geography.address.full_address | DEBATABLE | Values like `80 km NW of Kandrian, Papua New Guinea` and `Kermadec Islands region`. These are location descriptions, not structured addresses. Both `region` and `full_address` are reasonable. The `region` prediction correctly identifies the geographic domain. Existing interchangeability rules already cover geo hierarchy. |
| 14 | tech_systems | port | identity.commerce.ean | representation.numeric.integer_number | WRONG | Values 22, 80, 443, 8080 are network port numbers. EAN prediction is wrong domain (identity.commerce vs representation.numeric). |
| 15 | datetime_formats_extended | eu_dot_date | datetime.timestamp.iso_8601 | datetime.date.dmy_dot | WRONG | Values like `24.02.2020` are DMY dot-separated dates. iso_8601 timestamp (YYYY-MM-DDTHH:MM:SSZ) is wrong format entirely. |
| 16 | finance_coverage | isin | representation.identifier.alphanumeric_id | finance.securities.isin | WRONG | Values like `US0378331005`, `GB0002634946` are valid ISINs (2-letter country + 9 alphanumeric + check digit). Model predicted generic alphanumeric_id instead of the specific ISIN type. |
| 17 | network_logs | user_agent | datetime.timestamp.mdy_12h | technology.internet.user_agent | WRONG | Values like `Mozilla/5.0 (Macintosh; Intel Mac OS X) Chrome/88.0` are user agents. mdy_12h timestamp prediction is completely wrong. |
| 18 | technology_coverage | ip_v6 | technology.internet.ip_v4 | technology.internet.ip_v6 | WRONG | Values like `2001:0db8:85a3::8a2e:0370:7334` and `fe80::1ff:fe23:4567:890a` are clearly IPv6 (colon-separated hex). IPv4 prediction is wrong. |
| 19 | technology_coverage | ip_v4_with_port | technology.internet.ip_v4 | technology.internet.ip_v4_with_port | DEBATABLE | Values like `192.168.1.1:8080`. Model predicted ip_v4 which is the base type. ip_v4_with_port is ip_v4 plus a port suffix. The prediction captures the core format correctly but misses the port component. Close but not exact. |
| 20 | earthquakes_2024 | id | geography.coordinate.geohash | representation.identifier.alphanumeric_id | WRONG | Values like `us6000pgkh` are USGS earthquake IDs. Geohash prediction is wrong -- geohashes are base-32 encoded, not prefixed with country codes. |
| 21 | iris | sepal_length | technology.development.version | representation.numeric.decimal_number | WRONG | Values like 5.1, 4.9, 4.7 are decimal measurements. Version prediction is wrong -- these don't follow semver patterns (no multi-part dot notation). |
| 22 | server_logs_json | method | representation.discrete.categorical | technology.internet.http_method | DEBATABLE | Values: GET, POST, DELETE, PUT. Both categorical and http_method are defensible. http_method is more specific, but the values ARE categorical in nature. The model's label space includes http_method (label 241) so it should learn this. Leaning DEBATABLE because categorical is the generic parent of http_method. |
| 23 | earthquakes_2024 | horizontalError | datetime.date.dmy_short_dot | representation.numeric.decimal_number | WRONG | Values like 9.68, 14.54, 5.01 are decimal numbers. dmy_short_dot (e.g., 15.03.24) prediction is wrong. |
| 24 | financial_data | pe_ratio | geography.coordinate.latitude | representation.numeric.decimal_number | WRONG | Values like 54.16, 33.04 are P/E ratios (financial metric). Latitude prediction is wrong domain. |
| 25 | datetime_coverage | mdy_dash | datetime.date.iso | datetime.date.mdy_dash | WRONG | Values like `03-15-2024` are MDY dash format. ISO date (YYYY-MM-DD) has year first. Completely different field ordering. |
| 26 | server_logs_json | status_code | geography.address.postal_code | representation.numeric.integer_number | WRONG | Values 200, 201, 204, 401 are HTTP status codes. Postal code is wrong domain. Same pattern as case 10. |
| 27 | iris | sepal_width | technology.development.version | representation.numeric.decimal_number | WRONG | Values like 3.5, 3.0, 3.2 are decimal measurements. Version prediction is wrong. Same pattern as case 21. |
| 28 | api_users_json | phone | identity.government.abn | identity.person.phone_number | WRONG | Values like `+1 212 6022768` are international phone numbers. ABN (Australian Business Number) is an 11-digit number. Phone numbers with `+` prefix and country codes are clearly phones. |
| 29 | datetime_coverage | iso_week | datetime.date.iso | datetime.date.iso_week | WRONG | Values like `2024-W12`, `2023-W52` contain the `W` prefix indicating ISO week format. ISO date (YYYY-MM-DD) does not contain `W`. |
| 30 | server_logs_json | user_agent | representation.text.plain_text | technology.internet.user_agent | WRONG | Values include `PostmanRuntime/7.36.1`, `curl/8.4.0`, `kube-probe/1.28`, `Mozilla/5.0...`. These are user agent strings. plain_text is too generic -- these have identifiable user-agent structure. |
| 31 | representation_coverage | scientific_notation | representation.text.plain_text | representation.numeric.scientific_notation | WRONG | Values like `1.23e-4`, `6.022e23`, `-3.14E2` are clearly scientific notation. plain_text prediction completely misses the numeric format. |
| 32 | datetime_coverage | dmy_dash | datetime.date.iso | datetime.date.dmy_dash | AMBIGUOUS | Values like `15-03-2024`, `25-12-2023`. Already covered by DMY/MDY interchangeability rule in matching.rs, but `iso` (YYYY-MM-DD) is NOT in that set. However, the values `15-03-2024` could theoretically be ambiguous with some formats. BUT `15` in the first position exceeds 12, proving it's DMY not MDY, and the format `DD-MM-YYYY` is clearly not `YYYY-MM-DD`. Verdict: WRONG -- iso prediction requires YYYY first. |
| 33 | ecommerce_orders_json | order_id | identity.commerce.isbn | representation.identifier.alphanumeric_id | WRONG | Values like `ORD-48291` are order IDs with alphanumeric prefix. ISBN prediction is wrong -- ISBNs are 10 or 13 digit numbers with optional hyphens. |
| 34 | scientific_measurements | measurement_unit | representation.discrete.categorical | representation.scientific.measurement_unit | DEBATABLE | Values: g, bar, L, mm, mL, ohm, cm. These ARE categorical (low cardinality discrete strings) AND they are measurement units. categorical is the generic type; measurement_unit is more specific. The model should learn the more specific type, but categorical is not unreasonable for short strings with few distinct values. |

## Verdict Counts

| Verdict | Count |
|---------|-------|
| WRONG | 26 |
| DEBATABLE | 5 |
| AMBIGUOUS | 0 |

*Note: Case 32 (dmy_dash predicted as iso) was initially considered AMBIGUOUS but on examination the values clearly have day-first ordering (values >12 in first position), making the iso prediction definitively wrong. Reclassified to WRONG.*

## Detailed Analysis

### Case 1: new_geography.geojson
**Values:** `{"type": "Point", "coordinates": [-29.5789, 45.6377]}`, `{"type": "Feature", "geometry": {...}}`
**Predicted:** geography.format.geojson | **Expected:** container.object.json
**Verdict:** DEBATABLE
**Rationale:** The model has a dedicated `geography.format.geojson` label and correctly identifies that these JSON blobs are specifically GeoJSON (they follow the GeoJSON spec with `type`, `coordinates`, `geometry` fields). The ground truth maps this to `container.object.json` -- the generic JSON type. The model's prediction is more informative and technically correct. The schema_mapping should accept `geography.format.geojson` as an alternative for the `geojson` GT label.

### Case 2: codes_and_ids.sha256
**Values:** `1abd775a8e661366c67807273a7bd0fdcd70048964cf985b4d4a1668b391dacb` (64 hex chars)
**Predicted:** technology.development.git_sha | **Expected:** technology.cryptographic.hash
**Verdict:** WRONG
**Rationale:** SHA-256 produces 64-character hex strings. Git SHA (SHA-1) produces 40-character hex strings. The model should distinguish by length -- 64 chars cannot be git_sha. The model needs better training data separating hash lengths.

### Case 3: datetime_formats.year
**Values:** `2022`, `2021`, `2020`, `2023`
**Predicted:** datetime.date.compact_ym | **Expected:** datetime.component.year
**Verdict:** WRONG
**Rationale:** compact_ym format is YYYYMM (6 digits, e.g., `202203`). Year values are 4 digits. The model confused value length -- 4-digit numbers in range 1900-2100 should strongly signal `year`, not `compact_ym`.

### Case 4: finance_coverage.bitcoin_address
**Values:** `1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa`, `bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq`
**Predicted:** geography.address.full_address | **Expected:** finance.crypto.bitcoin_address
**Verdict:** WRONG
**Rationale:** Bitcoin addresses have distinctive format patterns: legacy (1xxx), P2SH (3xxx), bech32 (bc1q...). The model confused these with street addresses despite completely different character patterns. Wrong domain (geography vs finance).

### Case 5: new_technology.git_sha
**Values:** `20ad889500783ba6609f4b95ef3af7bd53b0086f` (40 hex chars)
**Predicted:** technology.development.git_sha | **Expected:** technology.cryptographic.hash
**Verdict:** DEBATABLE
**Rationale:** 40-character hex strings are SHA-1 hashes, commonly used as Git commit SHAs. The GT label `hash` is the generic parent; `git_sha` is a valid, more-specific identification. The schema_mapping already maps `git sha` GT to `technology.cryptographic.hash`, but `technology.development.git_sha` is a reasonable prediction for 40-char hex. However, without repository context, any 40-char hex could be a generic SHA-1 hash used for non-Git purposes.

### Case 6: tech_systems.user_agent
**Values:** `Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15) AppleWebKit/537.36...`
**Predicted:** technology.cryptographic.jwt | **Expected:** technology.internet.user_agent
**Verdict:** WRONG
**Rationale:** User agent strings have `Mozilla/5.0` prefix and nested parenthetical platform info. JWTs are base64url-encoded with two dots (header.payload.signature). Completely different formats. Severe misclassification.

### Case 7: earthquakes_2024.depthError
**Values:** `7.431`, `8.251`, `11.011`, `1.825`
**Predicted:** geography.coordinate.latitude | **Expected:** representation.numeric.decimal_number
**Verdict:** WRONG
**Rationale:** These are error measurements in km. While values overlap with latitude range (-90 to 90), the header "depthError" provides no geographic signal. The model over-indexes on value range without considering that many decimal numbers fall in the latitude range.

### Case 8: airports.icao
**Values:** `AYGA`, `AYMD`, `BGBW`, `BGGH`
**Predicted:** geography.transportation.unlocode | **Expected:** geography.transportation.icao_code
**Verdict:** WRONG
**Rationale:** ICAO codes are 4-letter strings. UNLOCODE is 5-character (2-letter country + 3-letter location, e.g., `USLAX`). The 4-letter format should distinguish these. Both are in the same domain/category but the length difference is deterministic.

### Case 9: ecommerce_orders.phone
**Values:** `+1-232-130-2535`, `+1-206-877-3615`
**Predicted:** identity.government.ssn | **Expected:** identity.person.phone_number
**Verdict:** WRONG
**Rationale:** The `+1-` prefix is a clear phone number indicator (international dialing format). SSNs are XXX-XX-XXXX (3-2-4 digit groups). The segment lengths don't match SSN format at all.

### Case 10: network_logs.status_code
**Values:** `400`, `500`, `301`, `403`, `503`
**Predicted:** geography.address.postal_code | **Expected:** representation.numeric.integer_number
**Verdict:** WRONG
**Rationale:** 3-digit numeric values 200-503. While some postal codes are 3-digit, the header "status_code" and the specific value distribution (clustering around 200, 300, 400, 500) indicate HTTP status codes. Postal code prediction is wrong domain.

### Case 11: tech_systems.server_hostname
**Values:** `srv-dev-43.example.com`, `srv-prod-11.example.com`
**Predicted:** technology.internet.url | **Expected:** technology.internet.hostname
**Verdict:** WRONG
**Rationale:** Hostnames lack the URL scheme (`https://`). A URL requires a scheme prefix. The model is in the right domain but wrong specific type.

### Case 12: people_directory.phone
**Values:** `+61-392-253-9475`, `+49-954-136-4111`, `+33-501-574-2929`
**Predicted:** identity.government.ssn | **Expected:** identity.person.phone_number
**Verdict:** WRONG
**Rationale:** International phone numbers with various country code prefixes (+61, +49, +33). SSN is US-only, 9 digits in XXX-XX-XXXX format. The international prefixes make this unambiguously phone.

### Case 13: earthquakes_2024.place
**Values:** `80 km NW of Kandrian, Papua New Guinea`, `Kermadec Islands region`, `46 km SE of Debre Sina, Ethiopia`
**Predicted:** geography.location.region | **Expected:** geography.address.full_address
**Verdict:** DEBATABLE
**Rationale:** These are relative location descriptions, not structured street addresses. "Region" is a reasonable label since they describe geographic regions. "Full_address" is also reasonable as these are place descriptions. Neither is precisely correct -- these are seismological location descriptions that don't match typical address or region patterns. The geographic domain is correct.

### Case 14: tech_systems.port
**Values:** `22`, `80`, `443`, `8080`, `5432`, `8443`
**Predicted:** identity.commerce.ean | **Expected:** representation.numeric.integer_number
**Verdict:** WRONG
**Rationale:** Network port numbers are small integers. EAN codes are 8 or 13 digit barcodes with check digits. Values like `22` and `80` cannot be EAN codes. Wrong domain and wrong format.

### Case 15: datetime_formats_extended.eu_dot_date
**Values:** `24.02.2020`, `06.03.2020`, `10.05.2020`
**Predicted:** datetime.timestamp.iso_8601 | **Expected:** datetime.date.dmy_dot
**Verdict:** WRONG
**Rationale:** DD.MM.YYYY format vs ISO 8601 timestamp (YYYY-MM-DDTHH:MM:SSZ). Completely different structure. The model confused dot-separated dates with ISO format.

### Case 16: finance_coverage.isin
**Values:** `US0378331005`, `GB0002634946`, `DE0007164600`, `JP3435000009`
**Predicted:** representation.identifier.alphanumeric_id | **Expected:** finance.securities.isin
**Verdict:** WRONG
**Rationale:** ISINs have a distinctive format: 2-letter country prefix + 9 alphanumeric chars + 1 check digit = 12 chars total. The model should recognize this well-defined format. alphanumeric_id is too generic.

### Case 17: network_logs.user_agent
**Values:** `Mozilla/5.0 (Macintosh; Intel Mac OS X) Chrome/88.0`, `Mozilla/5.0 (Windows NT 10.0) Chrome/96.0`
**Predicted:** datetime.timestamp.mdy_12h | **Expected:** technology.internet.user_agent
**Verdict:** WRONG
**Rationale:** User agent strings predicted as timestamps. Completely wrong format and domain. The shorter network_logs user agents (without full nested parentheticals) may confuse the model, but the `Mozilla/5.0` prefix is unmistakable.

### Case 18: technology_coverage.ip_v6
**Values:** `2001:0db8:85a3:0000:0000:8a2e:0370:7334`, `fe80::1ff:fe23:4567:890a`, `::1`
**Predicted:** technology.internet.ip_v4 | **Expected:** technology.internet.ip_v6
**Verdict:** WRONG
**Rationale:** IPv6 addresses use colons as separators and contain hex groups. IPv4 uses dots and decimal octets. The colon-separated hex format is definitionally IPv6. Model has both labels available (242 vs 244).

### Case 19: technology_coverage.ip_v4_with_port
**Values:** `192.168.1.1:8080`, `10.0.0.1:3000`, `172.16.0.100:443`
**Predicted:** technology.internet.ip_v4 | **Expected:** technology.internet.ip_v4_with_port
**Verdict:** DEBATABLE
**Rationale:** The core format is ip_v4, with an appended `:port`. The model recognizes the IP address correctly but misses the port suffix. This is a hierarchy issue -- ip_v4_with_port is a specialization of ip_v4. The prediction is partially correct and in the right domain/category, just missing the port component.

### Case 20: earthquakes_2024.id
**Values:** `us6000pgkh`, `us6000pgkd`, `us6000pj75`
**Predicted:** geography.coordinate.geohash | **Expected:** representation.identifier.alphanumeric_id
**Verdict:** WRONG
**Rationale:** USGS earthquake IDs have a `us` prefix + alphanumeric suffix. Geohashes use base-32 encoding (0-9, b-h, j-k, m-n, p-z). The `us` prefix and mixed alphanumeric pattern should indicate an ID, not a geohash.

### Case 21: iris.sepal_length
**Values:** `5.1`, `4.9`, `4.7`, `4.6`, `5.0`
**Predicted:** technology.development.version | **Expected:** representation.numeric.decimal_number
**Verdict:** WRONG
**Rationale:** Single decimal point numbers like 5.1, 4.9 are not version numbers (versions typically have multiple dots like 1.2.3). These are measurements. Wrong domain (technology vs representation).

### Case 22: server_logs_json.method
**Values:** `GET`, `POST`, `DELETE`, `PUT`
**Predicted:** representation.discrete.categorical | **Expected:** technology.internet.http_method
**Verdict:** DEBATABLE
**Rationale:** HTTP methods (GET, POST, etc.) are a specific categorical vocabulary. The model's label space includes `http_method` (label 241), so ideally it should learn the specific type. However, `categorical` is technically correct as a generic classification. The header "method" provides a hint but the model may not have enough http_method training examples. Both labels are valid characterizations.

### Case 23: earthquakes_2024.horizontalError
**Values:** `9.68`, `14.54`, `15.1`, `9.15`, `10`, `5.01`
**Predicted:** datetime.date.dmy_short_dot | **Expected:** representation.numeric.decimal_number
**Verdict:** WRONG
**Rationale:** Error measurement values in km. dmy_short_dot is a date format like `15.03.24`. While some values like `15.1` superficially resemble a truncated date, the range and context are clearly numeric. Wrong domain.

### Case 24: financial_data.pe_ratio
**Values:** `54.16`, `33.04`, `57.03`, `19.92`
**Predicted:** geography.coordinate.latitude | **Expected:** representation.numeric.decimal_number
**Verdict:** WRONG
**Rationale:** P/E ratio values happen to fall in the latitude range (-90 to 90) in many cases, but values like `54.16` and `70.06` are fine as latitudes too. Without header context, plain decimal numbers in this range are ambiguous with latitude. However, the model should use the header "pe_ratio" to override the latitude guess. This is the same decimal-vs-latitude confusion as case 7. Wrong domain.

### Case 25: datetime_coverage.mdy_dash
**Values:** `03-15-2024`, `12-25-2023`, `01-01-2022`, `07-04-2021`
**Predicted:** datetime.date.iso | **Expected:** datetime.date.mdy_dash
**Verdict:** WRONG
**Rationale:** MDY dash format (MM-DD-YYYY) vs ISO date (YYYY-MM-DD). The first field is 2 digits (month), not 4 digits (year). ISO requires the year-first pattern. Clear structural difference. Note: while dmy_dash and mdy_dash have an interchangeability rule, `iso` is not in that set.

### Case 26: server_logs_json.status_code
**Values:** `200`, `201`, `204`, `401`
**Predicted:** geography.address.postal_code | **Expected:** representation.numeric.integer_number
**Verdict:** WRONG
**Rationale:** Same pattern as case 10. HTTP status codes misidentified as postal codes. Wrong domain.

### Case 27: iris.sepal_width
**Values:** `3.5`, `3.0`, `3.2`, `3.1`, `3.6`
**Predicted:** technology.development.version | **Expected:** representation.numeric.decimal_number
**Verdict:** WRONG
**Rationale:** Same pattern as case 21. Single-decimal measurements misidentified as version numbers.

### Case 28: api_users_json.phone
**Values:** `+1 212 6022768`, `+44 20 5002657`, `+33 1 5258086`
**Predicted:** identity.government.abn | **Expected:** identity.person.phone_number
**Verdict:** WRONG
**Rationale:** International phone numbers with `+` prefix and country codes. ABN is an 11-digit Australian business number. The `+` prefix and international format are clear phone indicators.

### Case 29: datetime_coverage.iso_week
**Values:** `2024-W12`, `2023-W52`, `2022-W01`
**Predicted:** datetime.date.iso | **Expected:** datetime.date.iso_week
**Verdict:** WRONG
**Rationale:** ISO week format contains the distinctive `W` marker (YYYY-Www). ISO date is YYYY-MM-DD. The `W` character is a definitive discriminator.

### Case 30: server_logs_json.user_agent
**Values:** `PostmanRuntime/7.36.1`, `curl/8.4.0`, `kube-probe/1.28`, `Mozilla/5.0...`
**Predicted:** representation.text.plain_text | **Expected:** technology.internet.user_agent
**Verdict:** WRONG
**Rationale:** User agent strings have identifiable patterns (product/version format). plain_text is too generic. However, this is a mixed user-agent column with some very short entries that may challenge the model. Still wrong -- the `Mozilla/` and `/version` patterns are distinctive.

### Case 31: representation_coverage.scientific_notation
**Values:** `1.23e-4`, `6.022e23`, `-3.14E2`, `9.81e0`
**Predicted:** representation.text.plain_text | **Expected:** representation.numeric.scientific_notation
**Verdict:** WRONG
**Rationale:** Scientific notation with `e`/`E` exponent markers is a well-defined numeric format. plain_text completely misses the structure. The model has `scientific_notation` in its label space (label 206).

### Case 32: datetime_coverage.dmy_dash
**Values:** `15-03-2024`, `25-12-2023`, `01-01-2022`, `04-07-2021`, `29-02-2020`
**Predicted:** datetime.date.iso | **Expected:** datetime.date.dmy_dash
**Verdict:** WRONG
**Rationale:** First field contains values like `15`, `25`, `29` which exceed 12, definitively proving DD-first (not month-first or year-first). ISO date is YYYY-MM-DD which requires 4-digit year first. The existing DMY/MDY interchangeability rule does NOT include ISO.

### Case 33: ecommerce_orders_json.order_id
**Values:** `ORD-48291`, `ORD-48292`, `ORD-48293`
**Predicted:** identity.commerce.isbn | **Expected:** representation.identifier.alphanumeric_id
**Verdict:** WRONG
**Rationale:** Order IDs with `ORD-` prefix + sequential numbers. ISBNs are 10 or 13 digits, optionally hyphenated in specific group patterns. The `ORD-` prefix is not an ISBN pattern.

### Case 34: scientific_measurements.measurement_unit
**Values:** `g`, `bar`, `L`, `mm`, `mL`, `ohm` (omega symbol), `cm`
**Predicted:** representation.discrete.categorical | **Expected:** representation.scientific.measurement_unit
**Verdict:** DEBATABLE
**Rationale:** Short strings with low cardinality. The model sees these as categorical (which they structurally are -- short, repeated text tokens). measurement_unit is a more specific and more useful classification, but categorical is not wrong per se. The model has measurement_unit in its label space (label 211) and should learn it, but categorical is a defensible fallback for short-string columns.

## Verdict Counts

| Verdict | Count |
|---------|-------|
| WRONG | 26 |
| DEBATABLE | 5 |
| AMBIGUOUS | 0 |

## Error Pattern Clusters

These clusters identify systematic model weaknesses:

### Cluster A: Decimal number confusion (6 cases)
Cases 7, 21, 23, 24, 27, and partially 3.
Decimal numbers misclassified as latitude, version, dmy_short_dot.
**Root cause:** Model over-indexes on value range for latitude, dot-separated structure for version/dates, without sufficient weight on header context or format discrimination.

### Cluster B: Phone/SSN confusion (3 cases)
Cases 9, 12, 28.
Phone numbers with international prefixes classified as SSN or ABN.
**Root cause:** Model confuses digit-group patterns. The `+` prefix and varying country codes should be a strong phone signal.

### Cluster C: Small integer confusion (3 cases)
Cases 10, 14, 26.
HTTP status codes and port numbers classified as postal_code or EAN.
**Root cause:** Model maps small integers to wrong specific types. The integer_number GT is the correct generic classification.

### Cluster D: User agent confusion (3 cases)
Cases 6, 17, 30.
User agent strings misclassified as JWT, mdy_12h timestamp, plain_text.
**Root cause:** Long strings with mixed alphanumeric content confuse the model. User agent patterns (Mozilla/version, product/version) need stronger training signal.

### Cluster E: Date format confusion (4 cases)
Cases 15, 25, 29, 32.
Various date formats classified as ISO date/timestamp.
**Root cause:** Model defaults to ISO when uncertain about date format. Needs better disambiguation using field ordering and format markers (W for iso_week, dot separators for dmy_dot, etc.).

### Cluster F: Hash/ID confusion (3 cases)
Cases 2, 16, 20.
SHA-256 as git_sha, ISIN as alphanumeric_id, earthquake IDs as geohash.
**Root cause:** Alphanumeric string classification needs better length and structure discrimination.

## Recommendations

### 1. Schema mapping changes (for DEBATABLE verdicts)

These 5 cases should be addressed by expanding accepted alternatives in the eval framework:

**a) new_geography.geojson:** Add `geography.format.geojson` as accepted alternative for GT label `geojson`
```yaml
- gt_label: geojson
  finetype_label: container.object.json
  finetype_labels:
    - container.object.json
    - geography.format.geojson    # More specific, also correct
```

**b) new_technology.git_sha:** Add `technology.development.git_sha` as accepted alternative for GT label `git sha`
```yaml
- gt_label: git sha
  finetype_label: technology.cryptographic.hash
  finetype_labels:
    - technology.cryptographic.hash
    - technology.development.git_sha    # Specific subtype of SHA-1 hash
```

**c) earthquakes_2024.place:** Already partially handled by geo interchangeability rules. The prediction `geography.location.region` should match since `full_address` and `region` are both geographic location types. Check if `full_address` is in the interchangeability set.

**d) technology_coverage.ip_v4_with_port:** Add interchangeability rule: ip_v4 satisfies ip_v4_with_port (the base type captures the core format). OR add to matching.rs:
```rust
// ip_v4 is an acceptable match for ip_v4_with_port (base captures core format)
if expected == "technology.internet.ip_v4_with_port"
    && predicted == "technology.internet.ip_v4" {
    return true;
}
```

**e) server_logs_json.method and scientific_measurements.measurement_unit:** Add `categorical` as acceptable for `http_method` and `measurement_unit` GT labels:
```yaml
- gt_label: http method
  finetype_labels:
    - technology.internet.http_method
    - representation.discrete.categorical    # Generic parent
- gt_label: measurement unit
  finetype_labels:
    - representation.scientific.measurement_unit
    - representation.discrete.categorical    # Generic parent
```

### 2. Interchangeability rules to add in matching.rs

- **hash subtypes:** `technology.cryptographic.hash` and `technology.development.git_sha` should be interchangeable (git_sha IS a hash)
- **json subtypes:** `container.object.json` and `geography.format.geojson` (geojson IS json)
- **ip_v4 hierarchy:** `technology.internet.ip_v4` satisfies `technology.internet.ip_v4_with_port`

### 3. Model retraining priorities (for WRONG verdicts)

By impact (cases affected):

1. **Decimal number vs latitude/version/date** (6 cases) -- highest priority. Need training data with clear decimal numbers that are NOT lat/lon, NOT versions, NOT dates.
2. **Date format discrimination** (4 cases) -- ISO vs dmy_dash vs dmy_dot vs iso_week. Need better training coverage of non-ISO date formats with explicit format markers.
3. **Phone vs SSN/ABN** (3 cases) -- international phone format needs stronger training signal, especially the `+country_code` prefix.
4. **User agent recognition** (3 cases) -- user agent strings with Mozilla/product patterns need more training examples.
5. **Small integer disambiguation** (3 cases) -- HTTP status codes vs postal codes vs port numbers vs EAN.
6. **Alphanumeric ID disambiguation** (3 cases) -- hash length, ISIN format, geohash character set.

### 4. Estimated new baseline after fixes

- **Current:** 193/227 (85.0%)
- **After schema_mapping fixes (5 DEBATABLE):** 198/227 (87.2%)
- **After model improvements (26 WRONG):** theoretical ceiling 227/227, realistic target 210+/227 (92.5%+)

The 5 DEBATABLE cases can be fixed purely through eval framework changes (no model changes needed). The 26 WRONG cases require model improvements through retraining.
