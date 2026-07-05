# Gold eval anchor — item9-reframe

**Date:** 2026-07-05  
**Gold fixture:** `eval/gold/gold_corpus.tsv` (986 columns)  
**Scoring mode:** ENUM REFRAME (categorical/word/plain_text = one text residual)  
**Predictions:** `output/company-reference-audit/eval_item9/predictions_item9.tsv`  
**Scored:** 986 columns (0 gold columns had no prediction)  

Per-family accuracy (fraction of columns where the model's prediction equals the curated gold label — labels neither lens produced):

| Family | Columns | Accuracy |
|--------|--------:|---------:|
| A_tight_code_vs_alnum | 30 | 1.000 |
| B_country_vs_categorical | 60 | 1.000 |
| C_lat_lon_temperature | 90 | 1.000 |
| D_year_vs_integer | 60 | 0.950 |
| author-open:datetime.component.year | 1 | 0.000 |
| author-open:finance.currency.amount | 5 | 0.000 |
| author-open:geography.location.region | 2 | 0.500 |
| author-open:representation.discrete.categorical | 4 | 0.750 |
| author-open:representation.identifier.alphanumeric_id | 1 | 0.000 |
| author-open:representation.text.plain_text | 2 | 0.500 |
| author-open:technology.internet.url | 1 | 0.000 |
| backbone:datetime.date.iso | 51 | 1.000 |
| backbone:representation.numeric.decimal_number | 12 | 0.750 |
| backbone:representation.numeric.integer_number | 21 | 1.000 |
| backbone:representation.text.plain_text | 7 | 0.571 |
| compref:compound_codes | 3 | 0.000 |
| compref:entity_name_check | 1 | 0.000 |
| compref:gleif | 12 | 0.833 |
| compref:iata_boundary | 4 | 0.250 |
| compref:icao_boundary | 7 | 0.286 |
| compref:icd10 | 2 | 1.000 |
| compref:naics | 2 | 0.500 |
| compref:org_person_boundary | 8 | 0.875 |
| compref:sec_edgar | 3 | 0.667 |
| compref:user_agent_boundary | 6 | 0.833 |
| compref:wkt_boundary | 3 | 0.000 |
| compref:wkt_kept | 4 | 1.000 |
| external:datetime.component.year | 2 | 1.000 |
| external:datetime.date.iso | 9 | 0.889 |
| external:datetime.offset.utc | 1 | 1.000 |
| external:geography.address.postal_code | 1 | 1.000 |
| external:geography.coordinate.latitude | 6 | 1.000 |
| external:geography.coordinate.longitude | 4 | 1.000 |
| external:geography.location.city | 1 | 1.000 |
| external:representation.discrete.categorical | 9 | 0.778 |
| external:representation.identifier.alphanumeric_id | 4 | 0.500 |
| external:representation.numeric.decimal_number | 3 | 1.000 |
| external:representation.numeric.integer_number | 7 | 0.857 |
| external:representation.text.plain_text | 5 | 0.000 |
| external:technology.internet.top_level_domain | 2 | 0.000 |
| external:technology.internet.url | 2 | 1.000 |
| llm:datetime.component.year | 9 | 0.778 |
| llm:datetime.date.iso | 4 | 1.000 |
| llm:datetime.epoch.unix_seconds | 4 | 1.000 |
| llm:datetime.offset.utc | 28 | 0.964 |
| llm:geography.address.postal_code | 2 | 1.000 |
| llm:geography.coordinate.latitude | 30 | 0.867 |
| llm:geography.coordinate.longitude | 32 | 0.781 |
| llm:geography.location.city | 33 | 0.909 |
| llm:geography.location.country_code | 31 | 0.871 |
| llm:geography.location.region | 35 | 0.743 |
| llm:identity.commerce.isbn | 1 | 1.000 |
| llm:representation.discrete.categorical | 43 | 0.698 |
| llm:representation.identifier.alphanumeric_id | 37 | 0.676 |
| llm:representation.numeric.decimal_number | 1 | 0.000 |
| llm:representation.numeric.integer_number | 3 | 1.000 |
| llm:representation.text.plain_text | 3 | 0.333 |
| llm:technology.internet.url | 30 | 0.700 |
| tier1:datetime.offset.utc | 35 | 0.829 |
| tier1:geography.coordinate.latitude | 8 | 1.000 |
| tier1:geography.coordinate.longitude | 12 | 0.917 |
| tier1:geography.location.city | 3 | 0.333 |
| tier1:geography.location.country_code | 13 | 0.769 |
| tier1:geography.location.region | 2 | 0.500 |
| tier1:representation.discrete.categorical | 2 | 0.500 |
| tier1:representation.identifier.alphanumeric_id | 4 | 1.000 |
| tier1:technology.internet.url | 24 | 1.000 |
| tier2:datetime.component.year | 12 | 1.000 |
| tier2:datetime.epoch.unix_seconds | 20 | 0.850 |
| tier2:finance.currency.amount | 18 | 1.000 |
| tier2:geography.address.postal_code | 10 | 0.900 |
| tier2:identity.commerce.isbn | 29 | 0.931 |
| tier2:technology.internet.data_uri | 6 | 1.000 |
| tier2:technology.internet.top_level_domain | 4 | 0.750 |

Per-label precision/recall (the curated label is ground truth; YDF is not consulted):

| Curated label | Support | TP | FP | FN | Precision (95% CI) | Recall (95% CI) |
|---------------|--------:|---:|---:|---:|-------------------:|----------------:|
| container.array.semicolon_separated | 3 | 0 | 1 | 3 | 0.000 (0.00-0.79) | 0.000 (0.00-0.56) |
| container.object.csv | 1 | 0 | 0 | 1 | n/a (n/a) | 0.000 (0.00-0.79) |
| datetime.component.year | 41 | 39 | 3 | 2 | 0.929 (0.81-0.98) | 0.951 (0.84-0.99) |
| datetime.date.dmy_slash | 1 | 1 | 0 | 0 | 1.000 (0.21-1.00) | 1.000 (0.21-1.00) |
| datetime.date.iso | 55 | 55 | 0 | 0 | 1.000 (0.93-1.00) | 1.000 (0.93-1.00) |
| datetime.date.mdy_slash | 4 | 4 | 0 | 0 | 1.000 (0.51-1.00) | 1.000 (0.51-1.00) |
| datetime.epoch.unix_milliseconds | 4 | 2 | 0 | 2 | 1.000 (0.34-1.00) | 0.500 (0.15-0.85) |
| datetime.epoch.unix_seconds | 15 | 11 | 0 | 4 | 1.000 (0.74-1.00) | 0.733 (0.48-0.89) |
| datetime.offset.iana | 3 | 3 | 0 | 0 | 1.000 (0.44-1.00) | 1.000 (0.44-1.00) |
| datetime.offset.timezone_abbreviation | 6 | 6 | 0 | 0 | 1.000 (0.61-1.00) | 1.000 (0.61-1.00) |
| datetime.timestamp.dmy_hm | 1 | 1 | 0 | 0 | 1.000 (0.21-1.00) | 1.000 (0.21-1.00) |
| datetime.timestamp.iso_8601_milliseconds | 3 | 3 | 0 | 0 | 1.000 (0.44-1.00) | 1.000 (0.44-1.00) |
| datetime.timestamp.iso_milliseconds | 5 | 5 | 0 | 0 | 1.000 (0.57-1.00) | 1.000 (0.57-1.00) |
| datetime.timestamp.sql_standard | 10 | 9 | 0 | 1 | 1.000 (0.70-1.00) | 0.900 (0.60-0.98) |
| finance.currency.amount | 5 | 0 | 0 | 5 | n/a (n/a) | 0.000 (0.00-0.43) |
| finance.securities.lei | 1 | 1 | 0 | 0 | 1.000 (0.21-1.00) | 1.000 (0.21-1.00) |
| geography.address.full_address | 4 | 4 | 2 | 0 | 0.667 (0.30-0.90) | 1.000 (0.51-1.00) |
| geography.address.postal_code | 5 | 5 | 2 | 0 | 0.714 (0.36-0.92) | 1.000 (0.57-1.00) |
| geography.coordinate.latitude | 39 | 39 | 1 | 0 | 0.975 (0.87-1.00) | 1.000 (0.91-1.00) |
| geography.coordinate.longitude | 45 | 44 | 0 | 1 | 1.000 (0.92-1.00) | 0.978 (0.88-1.00) |
| geography.format.wkt | 4 | 4 | 0 | 0 | 1.000 (0.51-1.00) | 1.000 (0.51-1.00) |
| geography.location.city | 25 | 25 | 4 | 0 | 0.862 (0.69-0.95) | 1.000 (0.87-1.00) |
| geography.location.continent | 1 | 1 | 0 | 0 | 1.000 (0.21-1.00) | 1.000 (0.21-1.00) |
| geography.location.country | 11 | 9 | 0 | 2 | 1.000 (0.70-1.00) | 0.818 (0.52-0.95) |
| geography.location.country_code | 55 | 53 | 2 | 2 | 0.964 (0.88-0.99) | 0.964 (0.88-0.99) |
| geography.location.region | 15 | 10 | 7 | 5 | 0.588 (0.36-0.78) | 0.667 (0.42-0.85) |
| geography.location.state_code | 8 | 7 | 3 | 1 | 0.700 (0.40-0.89) | 0.875 (0.53-0.98) |
| geography.transportation.iata_code | 2 | 2 | 2 | 0 | 0.500 (0.15-0.85) | 1.000 (0.34-1.00) |
| geography.transportation.icao_code | 1 | 1 | 0 | 0 | 1.000 (0.21-1.00) | 1.000 (0.21-1.00) |
| identity.commerce.isbn | 18 | 18 | 0 | 0 | 1.000 (0.82-1.00) | 1.000 (0.82-1.00) |
| identity.industry.naics | 1 | 1 | 0 | 0 | 1.000 (0.21-1.00) | 1.000 (0.21-1.00) |
| identity.medical.icd10 | 2 | 1 | 0 | 1 | 1.000 (0.21-1.00) | 0.500 (0.09-0.91) |
| identity.person.full_name | 7 | 7 | 2 | 0 | 0.778 (0.45-0.94) | 1.000 (0.65-1.00) |
| identity.person.gender | 1 | 1 | 0 | 0 | 1.000 (0.21-1.00) | 1.000 (0.21-1.00) |
| identity.person.username | 1 | 0 | 0 | 1 | n/a (n/a) | 0.000 (0.00-0.79) |
| representation.boolean.binary | 1 | 1 | 5 | 0 | 0.167 (0.03-0.56) | 1.000 (0.21-1.00) |
| representation.boolean.terms | 10 | 10 | 2 | 0 | 0.833 (0.55-0.95) | 1.000 (0.72-1.00) |
| representation.file.extension | 1 | 0 | 0 | 1 | n/a (n/a) | 0.000 (0.00-0.79) |
| representation.identifier.alphanumeric_id | 63 | 47 | 2 | 16 | 0.959 (0.86-0.99) | 0.746 (0.63-0.84) |
| representation.identifier.increment | 1 | 1 | 6 | 0 | 0.143 (0.03-0.51) | 1.000 (0.21-1.00) |
| representation.identifier.numeric_code | 4 | 4 | 0 | 0 | 1.000 (0.51-1.00) | 1.000 (0.51-1.00) |
| representation.identifier.uuid | 2 | 2 | 0 | 0 | 1.000 (0.34-1.00) | 1.000 (0.34-1.00) |
| representation.numeric.decimal_number | 95 | 89 | 7 | 6 | 0.927 (0.86-0.96) | 0.937 (0.87-0.97) |
| representation.numeric.integer_number | 192 | 169 | 3 | 23 | 0.983 (0.95-0.99) | 0.880 (0.83-0.92) |
| representation.text.RESIDUAL | 146 | 94 | 13 | 52 | 0.879 (0.80-0.93) | 0.644 (0.56-0.72) |
| representation.text.entity_name | 16 | 13 | 26 | 3 | 0.333 (0.21-0.49) | 0.812 (0.57-0.93) |
| technology.filesystem.windows_path | 1 | 1 | 0 | 0 | 1.000 (0.21-1.00) | 1.000 (0.21-1.00) |
| technology.internet.hostname | 2 | 2 | 0 | 0 | 1.000 (0.34-1.00) | 1.000 (0.34-1.00) |
| technology.internet.top_level_domain | 5 | 2 | 0 | 3 | 1.000 (0.34-1.00) | 0.400 (0.12-0.77) |
| technology.internet.url | 44 | 37 | 1 | 7 | 0.974 (0.87-1.00) | 0.841 (0.71-0.92) |

**Headline — column accuracy:** 844/986 = 0.856 (95% CI 0.833-0.877)  

**Production accuracy — production (representative):** 185/260 = 0.712 (95% CI 0.654-0.763)  

> The headline above is the **curated-hard gold** set — hand-picked contested columns and the engine's optimisation target, so it overstates everyday accuracy. This companion scores a **uniform-random** draw of production columns: the honest number an analyst sees on a column picked at random. The gap is by design, not a regression.

**Macro precision** (mean over labels): 0.867  
**Macro recall** (mean over labels): 0.823  
