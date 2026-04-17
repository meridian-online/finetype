# Header Hint Impact Analysis

**Date:** 2026-04-18
**Model:** sherlock-v14 (215/227, 94.7%)
**Method:** Disabled both `apply_header_sharpen()` call sites in `column.rs`, rebuilt release binary, ran full profile eval on 35 datasets (227 columns).

---

## Headline

```
With hints (v14 baseline):  215/227 (94.7% label, 93.8% domain)
Without hints:              199/227 (87.7% label, 88.5% domain)
Net:                        -16 columns
```

The net loss masks the real story. Header hints help 18 columns but actively harm 12. The model's multi-branch header branch already handles those 12 cases correctly — the post-hoc hint system overrides correct predictions.

---

## Columns hints are HURTING (12)

These columns the model classifies correctly, but `apply_header_sharpen()` overrides to the wrong type:

```
| Dataset                | Column           | Model (correct)     | After hints (wrong)   | Ground Truth         |
|------------------------|------------------|---------------------|-----------------------|----------------------|
| new_identity           | email_display    | email_display       | email                 | email display        |
| new_identity           | phone_e164       | phone_e164          | phone_number          | phone e164           |
| codes_and_ids          | sha256           | hash                | tsid                  | hash                 |
| earthquakes_2024       | gap              | decimal_number      | amount_accounting     | decimal number       |
| earthquakes_2024       | depthError       | decimal_number      | latitude              | decimal number       |
| server_logs_json       | user_agent       | user_agent          | plain_text            | user agent           |
| datetime_coverage      | mdy_dash         | mdy_dash            | dmy_dash              | mdy dash             |
| server_logs_json       | response_time_ms | decimal_number      | integer_number        | decimal number       |
| technology_coverage    | ip_v4_with_port  | ip_v4_with_port     | ip_v4                 | ip_v4 with port      |
| new_identity           | upc              | upc                 | ean                   | upc                  |
| geography_data         | region           | region              | state                 | region               |
| world_cities           | subcountry       | region              | state                 | region               |
```

### Root causes

- **Same-category override (NNFT-194):** `h.contains("email")` matches header "email_display" → hints `identity.person.email`. Same-category override fires unconditionally (no confidence threshold) because email and email_display share category `identity.person`. Same mechanism for phone_e164 via `h.contains("phone")`.
- **Keyword match overshoot:** `h.contains("state")` in header_hint matches "us_states/State" → hints `geography.location.state`, overriding the model's correct `region` prediction. Similarly `h.contains("dash")` or date-related keywords may be interfering with mdy_dash.
- **Cross-domain override:** Some hardcoded hints pull predictions across domains with insufficient evidence, overriding correct model predictions.

### Overlap with v15 spec targets

8 of these 12 columns are targets of the v15 value-rules spec:

```
| Column           | v15 rule        | Would hint removal fix? |
|------------------|-----------------|-------------------------|
| email_display    | R28 (post-hint) | ✓ (model already right) |
| phone_e164       | R29 (post-hint) | ✓ (model already right) |
| sha256           | R26             | ✓ (model already right) |
| gap              | R30             | ✓ (model already right) |
| depthError       | — (deferred)    | ✓ (model already right) |
| user_agent       | — (deferred)    | ✓ (model already right) |
| status_code ×2   | R25             | ✗ (not in gained list)  |
| year             | R27             | ✗ (not in gained list)  |
```

---

## Columns hints are HELPING (18)

These columns the model gets wrong, but header hints correct:

```
| Dataset                | Column           | Model (wrong)        | After hints (correct) | Ground Truth         |
|------------------------|------------------|----------------------|-----------------------|----------------------|
| medical_records        | first_name       | categorical          | first_name            | first name           |
| people_directory       | first_name       | categorical          | first_name            | first name           |
| medical_records        | height_in        | increment            | height                | height               |
| medical_records        | weight_lbs       | decimal_number       | weight                | weight               |
| people_directory       | height_cm        | decimal_number       | height                | height               |
| people_directory       | weight_kg        | decimal_number       | weight                | weight               |
| countries              | alpha-2          | state_code           | country_code          | country code         |
| countries              | alpha-3          | iata_code            | country_code          | country code         |
| books_catalog          | publisher        | categorical          | entity_name           | entity name          |
| books_catalog          | url              | plain_text           | url                   | url                  |
| sports_events          | venue            | city                 | entity_name           | entity name          |
| weather_stations_json  | station_name     | city                 | entity_name           | entity name          |
| api_users_json         | address.postal_code | categorical       | postal_code           | postal code          |
| tech_systems           | server_hostname  | plain_text           | hostname              | hostname             |
| technology_coverage    | ip_v6            | plain_text           | ip_v6                 | ip_v6                |
| network_logs           | user_agent       | whitespace_separated | user_agent            | user agent           |
| us_states              | State            | region               | state                 | state                |
| datetime_coverage      | rfc_3339         | iso_space_zulu       | rfc_3339              | rfc 3339             |
```

### Categories

- **Semantic confusion (8):** first_name→categorical (×2), entity_name→categorical/city (×3), height/weight→decimal_number/increment (×4). The model sees the values but can't infer the semantic type without the header name.
- **Format confusion (4):** country_code→state_code/iata_code (×2), rfc_3339→iso_space_zulu, ip_v6→plain_text. Similar-looking formats that the header disambiguates.
- **Generic types (3):** url→plain_text, postal_code→categorical, hostname→plain_text. The model defaults to a generic type; the header provides the specific type.
- **Inconsistent (1):** user_agent in network_logs gets whitespace_separated without hints, but user_agent in server_logs_json gets user_agent without hints. The difference is likely the value distribution (network_logs has full browser UAs, server_logs_json has mixed).

---

## Implications

### For v15 strategy

The analysis reveals that 6 of the 8 v15 target columns don't need new rules at all — the model already predicts them correctly. The header hint system is the problem, not the model. Two approaches:

**Option A: Selective hint removal.** Remove or narrow the specific keyword matches causing harm (`h.contains("email")`, `h.contains("phone")`, etc.) while keeping the hints that help. This is subtractive — less code, not more.

**Option B: Value-based post-hint guards (current spec).** Keep all hints but add guards that override back to the correct type based on value patterns. This is additive — more code layered on top of the problem.

**Option C: Hybrid.** Remove harmful keyword matches + add value-based rules only for the cases where the model is also wrong (status_code, year).

### For decision 0042 (remove regex header hints)

This data supports 0042's direction. The model's header branch has learned enough to handle 12 cases that the regex system actively harms. The 18 cases where hints help are candidates for improved training data rather than permanent hint rules — especially the semantic confusion cases (first_name, entity_name, height, weight) which the model should learn with better training signal.

### Ceiling analysis

If we could surgically fix both the 18 lost and 12 gained:
- Theoretical maximum with perfect hints: 227/227
- Practical ceiling with current model: 215 + 12 = 227 - 18 + 18 = 227 (if model learns all 18 hint-dependent cases)
- Realistic near-term: 215 + 8 (v15 targets) = 223/227, with 4 more available from hint removal if status_code and year can be fixed by other means
