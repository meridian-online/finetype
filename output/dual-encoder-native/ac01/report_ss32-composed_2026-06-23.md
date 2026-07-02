# Gold eval anchor — ss32-composed

**Date:** 2026-06-23  
**Gold fixture:** `eval/gold/gold_corpus.tsv` (931 columns)  
**Scoring mode:** ENUM REFRAME (categorical/word/plain_text = one text residual)  
**Predictions:** `output/dual-encoder-native/ac01/pred_s44_composed_ss32.tsv`  
**Scored:** 927 columns (4 gold columns had no prediction)  

Per-family accuracy (fraction of columns where the model's prediction equals the curated gold label — labels neither lens produced):

| Family | Columns | Accuracy |
|--------|--------:|---------:|
| A_tight_code_vs_alnum | 30 | 0.967 |
| B_country_vs_categorical | 60 | 0.983 |
| C_lat_lon_temperature | 90 | 0.944 |
| D_year_vs_integer | 60 | 0.900 |
| author-open:datetime.component.year | 1 | 0.000 |
| author-open:finance.currency.amount | 5 | 0.000 |
| author-open:geography.location.region | 2 | 1.000 |
| author-open:representation.discrete.categorical | 4 | 0.500 |
| author-open:representation.identifier.alphanumeric_id | 1 | 0.000 |
| author-open:representation.text.plain_text | 2 | 0.500 |
| author-open:technology.internet.url | 1 | 0.000 |
| backbone:datetime.date.iso | 51 | 1.000 |
| backbone:representation.numeric.decimal_number | 12 | 0.667 |
| backbone:representation.numeric.integer_number | 21 | 0.524 |
| backbone:representation.text.plain_text | 7 | 0.571 |
| external:datetime.component.year | 2 | 1.000 |
| external:datetime.date.iso | 9 | 0.333 |
| external:datetime.offset.utc | 1 | 1.000 |
| external:geography.address.postal_code | 1 | 1.000 |
| external:geography.coordinate.latitude | 6 | 0.500 |
| external:geography.coordinate.longitude | 4 | 0.750 |
| external:geography.location.city | 1 | 1.000 |
| external:representation.discrete.categorical | 9 | 0.556 |
| external:representation.identifier.alphanumeric_id | 4 | 0.250 |
| external:representation.numeric.decimal_number | 3 | 0.667 |
| external:representation.numeric.integer_number | 7 | 0.571 |
| external:representation.text.plain_text | 5 | 0.000 |
| external:technology.internet.top_level_domain | 2 | 0.000 |
| external:technology.internet.url | 2 | 1.000 |
| llm:datetime.component.year | 9 | 0.333 |
| llm:datetime.date.iso | 4 | 1.000 |
| llm:datetime.epoch.unix_seconds | 4 | 0.250 |
| llm:datetime.offset.utc | 28 | 0.607 |
| llm:geography.address.postal_code | 2 | 1.000 |
| llm:geography.coordinate.latitude | 29 | 0.759 |
| llm:geography.coordinate.longitude | 32 | 0.781 |
| llm:geography.location.city | 30 | 0.800 |
| llm:geography.location.country_code | 31 | 0.613 |
| llm:geography.location.region | 35 | 0.686 |
| llm:identity.commerce.isbn | 1 | 1.000 |
| llm:representation.discrete.categorical | 43 | 0.233 |
| llm:representation.identifier.alphanumeric_id | 37 | 0.459 |
| llm:representation.numeric.decimal_number | 1 | 0.000 |
| llm:representation.numeric.integer_number | 3 | 1.000 |
| llm:representation.text.plain_text | 3 | 0.000 |
| llm:technology.internet.url | 30 | 0.500 |
| tier1:datetime.offset.utc | 35 | 0.429 |
| tier1:geography.coordinate.latitude | 8 | 0.500 |
| tier1:geography.coordinate.longitude | 12 | 0.750 |
| tier1:geography.location.city | 3 | 0.000 |
| tier1:geography.location.country_code | 13 | 0.385 |
| tier1:geography.location.region | 2 | 0.500 |
| tier1:representation.discrete.categorical | 2 | 0.000 |
| tier1:representation.identifier.alphanumeric_id | 4 | 0.500 |
| tier1:technology.internet.url | 24 | 0.958 |
| tier2:datetime.component.year | 12 | 0.333 |
| tier2:datetime.epoch.unix_seconds | 20 | 0.700 |
| tier2:finance.currency.amount | 18 | 0.944 |
| tier2:geography.address.postal_code | 10 | 0.900 |
| tier2:identity.commerce.isbn | 29 | 0.690 |
| tier2:technology.internet.data_uri | 6 | 1.000 |
| tier2:technology.internet.top_level_domain | 4 | 0.000 |

Per-label precision/recall (the curated label is ground truth; YDF is not consulted):

| Curated label | Support | TP | FP | FN | Precision (95% CI) | Recall (95% CI) |
|---------------|--------:|---:|---:|---:|-------------------:|----------------:|
| container.object.csv | 1 | 0 | 0 | 1 | n/a (n/a) | 0.000 (0.00-0.79) |
| datetime.component.year | 41 | 39 | 2 | 2 | 0.951 (0.84-0.99) | 0.951 (0.84-0.99) |
| datetime.date.dmy_slash | 1 | 1 | 0 | 0 | 1.000 (0.21-1.00) | 1.000 (0.21-1.00) |
| datetime.date.iso | 52 | 52 | 0 | 0 | 1.000 (0.93-1.00) | 1.000 (0.93-1.00) |
| datetime.date.mdy_slash | 4 | 4 | 0 | 0 | 1.000 (0.51-1.00) | 1.000 (0.51-1.00) |
| datetime.epoch.unix_milliseconds | 4 | 1 | 0 | 3 | 1.000 (0.21-1.00) | 0.250 (0.05-0.70) |
| datetime.epoch.unix_seconds | 15 | 8 | 1 | 7 | 0.889 (0.56-0.98) | 0.533 (0.30-0.75) |
| datetime.offset.iana | 4 | 3 | 0 | 1 | 1.000 (0.44-1.00) | 0.750 (0.30-0.95) |
| datetime.offset.utc | 1 | 1 | 4 | 0 | 0.200 (0.04-0.62) | 1.000 (0.21-1.00) |
| datetime.timestamp.dmy_hm | 1 | 0 | 0 | 1 | n/a (n/a) | 0.000 (0.00-0.79) |
| datetime.timestamp.iso_8601_milliseconds | 4 | 3 | 0 | 1 | 1.000 (0.44-1.00) | 0.750 (0.30-0.95) |
| datetime.timestamp.sql_standard | 14 | 9 | 0 | 5 | 1.000 (0.70-1.00) | 0.643 (0.39-0.84) |
| finance.currency.amount | 5 | 0 | 0 | 5 | n/a (n/a) | 0.000 (0.00-0.43) |
| geography.address.postal_code | 4 | 4 | 2 | 0 | 0.667 (0.30-0.90) | 1.000 (0.51-1.00) |
| geography.coordinate.latitude | 39 | 37 | 0 | 2 | 1.000 (0.91-1.00) | 0.949 (0.83-0.99) |
| geography.coordinate.longitude | 45 | 44 | 0 | 1 | 1.000 (0.92-1.00) | 0.978 (0.88-1.00) |
| geography.location.city | 24 | 24 | 13 | 0 | 0.649 (0.49-0.78) | 1.000 (0.86-1.00) |
| geography.location.continent | 1 | 1 | 0 | 0 | 1.000 (0.21-1.00) | 1.000 (0.21-1.00) |
| geography.location.country | 11 | 9 | 7 | 2 | 0.562 (0.33-0.77) | 0.818 (0.52-0.95) |
| geography.location.country_code | 54 | 44 | 1 | 10 | 0.978 (0.88-1.00) | 0.815 (0.69-0.90) |
| geography.location.region | 15 | 12 | 7 | 3 | 0.632 (0.41-0.81) | 0.800 (0.55-0.93) |
| geography.location.state_code | 7 | 7 | 1 | 0 | 0.875 (0.53-0.98) | 1.000 (0.65-1.00) |
| geography.transportation.iata_code | 2 | 1 | 1 | 1 | 0.500 (0.09-0.91) | 0.500 (0.09-0.91) |
| geography.transportation.icao_code | 1 | 1 | 0 | 0 | 1.000 (0.21-1.00) | 1.000 (0.21-1.00) |
| identity.commerce.isbn | 18 | 15 | 0 | 3 | 1.000 (0.80-1.00) | 0.833 (0.61-0.94) |
| identity.person.full_name | 1 | 0 | 4 | 1 | 0.000 (0.00-0.49) | 0.000 (0.00-0.79) |
| identity.person.gender | 1 | 1 | 0 | 0 | 1.000 (0.21-1.00) | 1.000 (0.21-1.00) |
| identity.person.username | 1 | 0 | 0 | 1 | n/a (n/a) | 0.000 (0.00-0.79) |
| representation.boolean.terms | 10 | 8 | 1 | 2 | 0.889 (0.56-0.98) | 0.800 (0.49-0.94) |
| representation.file.extension | 1 | 0 | 0 | 1 | n/a (n/a) | 0.000 (0.00-0.79) |
| representation.identifier.alphanumeric_id | 62 | 36 | 1 | 26 | 0.973 (0.86-1.00) | 0.581 (0.46-0.70) |
| representation.identifier.increment | 1 | 0 | 7 | 1 | 0.000 (0.00-0.35) | 0.000 (0.00-0.79) |
| representation.identifier.numeric_code | 3 | 0 | 0 | 3 | n/a (n/a) | 0.000 (0.00-0.56) |
| representation.identifier.uuid | 2 | 1 | 0 | 1 | 1.000 (0.21-1.00) | 0.500 (0.09-0.91) |
| representation.numeric.decimal_number | 94 | 63 | 41 | 31 | 0.606 (0.51-0.69) | 0.670 (0.57-0.76) |
| representation.numeric.integer_number | 194 | 124 | 6 | 70 | 0.954 (0.90-0.98) | 0.639 (0.57-0.70) |
| representation.text.RESIDUAL | 129 | 53 | 21 | 76 | 0.716 (0.60-0.81) | 0.411 (0.33-0.50) |
| representation.text.entity_name | 8 | 5 | 40 | 3 | 0.111 (0.05-0.23) | 0.625 (0.31-0.86) |
| technology.internet.hostname | 2 | 2 | 0 | 0 | 1.000 (0.34-1.00) | 1.000 (0.34-1.00) |
| technology.internet.top_level_domain | 6 | 0 | 0 | 6 | n/a (n/a) | 0.000 (0.00-0.39) |
| technology.internet.url | 44 | 37 | 11 | 7 | 0.771 (0.63-0.87) | 0.841 (0.71-0.92) |

**Headline — column accuracy:** 650/927 = 0.701 (95% CI 0.671-0.730)  
**Macro precision** (mean over labels): 0.792  
**Macro recall** (mean over labels): 0.625  
