# Gold eval anchor — clean8m-composed

**Date:** 2026-06-28  
**Gold fixture:** `eval/gold/gold_corpus.tsv` (931 columns)  
**Scoring mode:** ENUM REFRAME (categorical/word/plain_text = one text residual)  
**Predictions:** `output/clean-label-retrain/scores/clean8m_composed.tsv`  
**Scored:** 931 columns (0 gold columns had no prediction)  

Per-family accuracy (fraction of columns where the model's prediction equals the curated gold label — labels neither lens produced):

| Family | Columns | Accuracy |
|--------|--------:|---------:|
| A_tight_code_vs_alnum | 30 | 0.467 |
| B_country_vs_categorical | 60 | 0.833 |
| C_lat_lon_temperature | 90 | 1.000 |
| D_year_vs_integer | 60 | 0.933 |
| author-open:datetime.component.year | 1 | 0.000 |
| author-open:finance.currency.amount | 5 | 0.000 |
| author-open:geography.location.region | 2 | 0.000 |
| author-open:representation.discrete.categorical | 4 | 0.750 |
| author-open:representation.identifier.alphanumeric_id | 1 | 0.000 |
| author-open:representation.text.plain_text | 2 | 0.500 |
| author-open:technology.internet.url | 1 | 0.000 |
| backbone:datetime.date.iso | 51 | 1.000 |
| backbone:representation.numeric.decimal_number | 12 | 0.750 |
| backbone:representation.numeric.integer_number | 21 | 1.000 |
| backbone:representation.text.plain_text | 7 | 0.571 |
| external:datetime.component.year | 2 | 1.000 |
| external:datetime.date.iso | 9 | 0.889 |
| external:datetime.offset.utc | 1 | 1.000 |
| external:geography.address.postal_code | 1 | 1.000 |
| external:geography.coordinate.latitude | 6 | 1.000 |
| external:geography.coordinate.longitude | 4 | 1.000 |
| external:geography.location.city | 1 | 1.000 |
| external:representation.discrete.categorical | 9 | 0.333 |
| external:representation.identifier.alphanumeric_id | 4 | 0.750 |
| external:representation.numeric.decimal_number | 3 | 1.000 |
| external:representation.numeric.integer_number | 7 | 0.857 |
| external:representation.text.plain_text | 5 | 0.200 |
| external:technology.internet.top_level_domain | 2 | 0.000 |
| external:technology.internet.url | 2 | 1.000 |
| llm:datetime.component.year | 9 | 0.889 |
| llm:datetime.date.iso | 4 | 1.000 |
| llm:datetime.epoch.unix_seconds | 4 | 1.000 |
| llm:datetime.offset.utc | 28 | 0.857 |
| llm:geography.address.postal_code | 2 | 1.000 |
| llm:geography.coordinate.latitude | 30 | 0.933 |
| llm:geography.coordinate.longitude | 32 | 0.906 |
| llm:geography.location.city | 33 | 0.455 |
| llm:geography.location.country_code | 31 | 0.548 |
| llm:geography.location.region | 35 | 0.629 |
| llm:identity.commerce.isbn | 1 | 1.000 |
| llm:representation.discrete.categorical | 43 | 0.465 |
| llm:representation.identifier.alphanumeric_id | 37 | 0.568 |
| llm:representation.numeric.decimal_number | 1 | 0.000 |
| llm:representation.numeric.integer_number | 3 | 1.000 |
| llm:representation.text.plain_text | 3 | 0.000 |
| llm:technology.internet.url | 30 | 0.700 |
| tier1:datetime.offset.utc | 35 | 0.857 |
| tier1:geography.coordinate.latitude | 8 | 1.000 |
| tier1:geography.coordinate.longitude | 12 | 0.750 |
| tier1:geography.location.city | 3 | 0.333 |
| tier1:geography.location.country_code | 13 | 0.154 |
| tier1:geography.location.region | 2 | 0.500 |
| tier1:representation.discrete.categorical | 2 | 0.500 |
| tier1:representation.identifier.alphanumeric_id | 4 | 0.500 |
| tier1:technology.internet.url | 24 | 1.000 |
| tier2:datetime.component.year | 12 | 1.000 |
| tier2:datetime.epoch.unix_seconds | 20 | 0.800 |
| tier2:finance.currency.amount | 18 | 1.000 |
| tier2:geography.address.postal_code | 10 | 0.900 |
| tier2:identity.commerce.isbn | 29 | 0.759 |
| tier2:technology.internet.data_uri | 6 | 1.000 |
| tier2:technology.internet.top_level_domain | 4 | 0.250 |

Per-label precision/recall (the curated label is ground truth; YDF is not consulted):

| Curated label | Support | TP | FP | FN | Precision (95% CI) | Recall (95% CI) |
|---------------|--------:|---:|---:|---:|-------------------:|----------------:|
| container.object.csv | 1 | 0 | 0 | 1 | n/a (n/a) | 0.000 (0.00-0.79) |
| datetime.component.year | 41 | 40 | 3 | 1 | 0.930 (0.81-0.98) | 0.976 (0.87-1.00) |
| datetime.date.dmy_slash | 1 | 1 | 0 | 0 | 1.000 (0.21-1.00) | 1.000 (0.21-1.00) |
| datetime.date.iso | 52 | 52 | 0 | 0 | 1.000 (0.93-1.00) | 1.000 (0.93-1.00) |
| datetime.date.mdy_slash | 4 | 4 | 0 | 0 | 1.000 (0.51-1.00) | 1.000 (0.51-1.00) |
| datetime.epoch.unix_milliseconds | 4 | 3 | 0 | 1 | 1.000 (0.44-1.00) | 0.750 (0.30-0.95) |
| datetime.epoch.unix_seconds | 15 | 11 | 1 | 4 | 0.917 (0.65-0.99) | 0.733 (0.48-0.89) |
| datetime.offset.iana | 3 | 3 | 0 | 0 | 1.000 (0.44-1.00) | 1.000 (0.44-1.00) |
| datetime.offset.timezone_abbreviation | 6 | 6 | 0 | 0 | 1.000 (0.61-1.00) | 1.000 (0.61-1.00) |
| datetime.timestamp.dmy_hm | 1 | 0 | 0 | 1 | n/a (n/a) | 0.000 (0.00-0.79) |
| datetime.timestamp.iso_8601_milliseconds | 3 | 3 | 0 | 0 | 1.000 (0.44-1.00) | 1.000 (0.44-1.00) |
| datetime.timestamp.iso_milliseconds | 5 | 5 | 0 | 0 | 1.000 (0.57-1.00) | 1.000 (0.57-1.00) |
| datetime.timestamp.sql_standard | 10 | 9 | 0 | 1 | 1.000 (0.70-1.00) | 0.900 (0.60-0.98) |
| finance.currency.amount | 5 | 0 | 0 | 5 | n/a (n/a) | 0.000 (0.00-0.43) |
| geography.address.full_address | 4 | 4 | 1 | 0 | 0.800 (0.38-0.96) | 1.000 (0.51-1.00) |
| geography.address.postal_code | 5 | 5 | 0 | 0 | 1.000 (0.57-1.00) | 1.000 (0.57-1.00) |
| geography.coordinate.latitude | 39 | 39 | 1 | 0 | 0.975 (0.87-1.00) | 1.000 (0.91-1.00) |
| geography.coordinate.longitude | 45 | 44 | 0 | 1 | 1.000 (0.92-1.00) | 0.978 (0.88-1.00) |
| geography.location.city | 24 | 11 | 6 | 13 | 0.647 (0.41-0.83) | 0.458 (0.28-0.65) |
| geography.location.continent | 1 | 0 | 5 | 1 | 0.000 (0.00-0.43) | 0.000 (0.00-0.79) |
| geography.location.country | 11 | 5 | 1 | 6 | 0.833 (0.44-0.97) | 0.455 (0.21-0.72) |
| geography.location.country_code | 54 | 27 | 6 | 27 | 0.818 (0.66-0.91) | 0.500 (0.37-0.63) |
| geography.location.region | 15 | 8 | 14 | 7 | 0.364 (0.20-0.57) | 0.533 (0.30-0.75) |
| geography.location.state_code | 7 | 6 | 4 | 1 | 0.600 (0.31-0.83) | 0.857 (0.49-0.97) |
| geography.transportation.iata_code | 2 | 0 | 4 | 2 | 0.000 (0.00-0.49) | 0.000 (0.00-0.66) |
| geography.transportation.icao_code | 1 | 1 | 1 | 0 | 0.500 (0.09-0.91) | 1.000 (0.21-1.00) |
| identity.commerce.isbn | 18 | 18 | 0 | 0 | 1.000 (0.82-1.00) | 1.000 (0.82-1.00) |
| identity.person.full_name | 1 | 0 | 5 | 1 | 0.000 (0.00-0.43) | 0.000 (0.00-0.79) |
| identity.person.gender | 1 | 1 | 0 | 0 | 1.000 (0.21-1.00) | 1.000 (0.21-1.00) |
| identity.person.username | 1 | 0 | 0 | 1 | n/a (n/a) | 0.000 (0.00-0.79) |
| representation.boolean.binary | 1 | 1 | 5 | 0 | 0.167 (0.03-0.56) | 1.000 (0.21-1.00) |
| representation.boolean.terms | 10 | 10 | 1 | 0 | 0.909 (0.62-0.98) | 1.000 (0.72-1.00) |
| representation.file.extension | 1 | 0 | 0 | 1 | n/a (n/a) | 0.000 (0.00-0.79) |
| representation.identifier.alphanumeric_id | 62 | 27 | 3 | 35 | 0.900 (0.74-0.97) | 0.435 (0.32-0.56) |
| representation.identifier.increment | 1 | 1 | 8 | 0 | 0.111 (0.02-0.44) | 1.000 (0.21-1.00) |
| representation.identifier.numeric_code | 3 | 2 | 3 | 1 | 0.400 (0.12-0.77) | 0.667 (0.21-0.94) |
| representation.identifier.uuid | 2 | 2 | 0 | 0 | 1.000 (0.34-1.00) | 1.000 (0.34-1.00) |
| representation.numeric.decimal_number | 95 | 94 | 7 | 1 | 0.931 (0.86-0.97) | 0.989 (0.94-1.00) |
| representation.numeric.integer_number | 192 | 163 | 2 | 29 | 0.988 (0.96-1.00) | 0.849 (0.79-0.89) |
| representation.text.RESIDUAL | 120 | 67 | 35 | 53 | 0.657 (0.56-0.74) | 0.558 (0.47-0.64) |
| representation.text.entity_name | 12 | 8 | 41 | 4 | 0.163 (0.09-0.29) | 0.667 (0.39-0.86) |
| technology.filesystem.windows_path | 1 | 1 | 0 | 0 | 1.000 (0.21-1.00) | 1.000 (0.21-1.00) |
| technology.internet.hostname | 2 | 2 | 2 | 0 | 0.500 (0.15-0.85) | 1.000 (0.34-1.00) |
| technology.internet.top_level_domain | 5 | 0 | 1 | 5 | 0.000 (0.00-0.79) | 0.000 (0.00-0.43) |
| technology.internet.url | 44 | 37 | 1 | 7 | 0.974 (0.87-1.00) | 0.841 (0.71-0.92) |

**Headline — column accuracy:** 721/931 = 0.774 (95% CI 0.746-0.800)  
**Macro precision** (mean over labels): 0.727  
**Macro recall** (mean over labels): 0.692  
