# Gold eval anchor — m2v8m-s44-sense-reframe

**Date:** 2026-06-23  
**Gold fixture:** `eval/gold/gold_corpus.tsv` (931 columns)  
**Scoring mode:** ENUM REFRAME (categorical/word/plain_text = one text residual)  
**Predictions:** `output/dual-encoder-native/ac01/pred_m2v8m_s44_sense.tsv`  
**Scored:** 927 columns (4 gold columns had no prediction)  

Per-family accuracy (fraction of columns where the model's prediction equals the curated gold label — labels neither lens produced):

| Family | Columns | Accuracy |
|--------|--------:|---------:|
| A_tight_code_vs_alnum | 30 | 0.267 |
| B_country_vs_categorical | 60 | 0.033 |
| C_lat_lon_temperature | 90 | 0.200 |
| D_year_vs_integer | 60 | 0.617 |
| author-open:datetime.component.year | 1 | 0.000 |
| author-open:finance.currency.amount | 5 | 0.000 |
| author-open:geography.location.region | 2 | 0.500 |
| author-open:representation.discrete.categorical | 4 | 0.750 |
| author-open:representation.identifier.alphanumeric_id | 1 | 0.000 |
| author-open:representation.text.plain_text | 2 | 0.000 |
| author-open:technology.internet.url | 1 | 1.000 |
| backbone:datetime.date.iso | 51 | 0.000 |
| backbone:representation.numeric.decimal_number | 12 | 0.667 |
| backbone:representation.numeric.integer_number | 21 | 0.429 |
| backbone:representation.text.plain_text | 7 | 0.571 |
| external:datetime.component.year | 2 | 0.000 |
| external:datetime.date.iso | 9 | 0.000 |
| external:datetime.offset.utc | 1 | 0.000 |
| external:geography.address.postal_code | 1 | 0.000 |
| external:geography.coordinate.latitude | 6 | 0.500 |
| external:geography.coordinate.longitude | 4 | 0.250 |
| external:geography.location.city | 1 | 1.000 |
| external:representation.discrete.categorical | 9 | 0.000 |
| external:representation.identifier.alphanumeric_id | 4 | 0.250 |
| external:representation.numeric.decimal_number | 3 | 0.000 |
| external:representation.numeric.integer_number | 7 | 0.000 |
| external:representation.text.plain_text | 5 | 0.000 |
| external:technology.internet.top_level_domain | 2 | 0.000 |
| external:technology.internet.url | 2 | 0.000 |
| llm:datetime.component.year | 9 | 0.556 |
| llm:datetime.date.iso | 4 | 0.000 |
| llm:datetime.epoch.unix_seconds | 4 | 0.250 |
| llm:datetime.offset.utc | 28 | 0.214 |
| llm:geography.address.postal_code | 2 | 0.000 |
| llm:geography.coordinate.latitude | 29 | 0.414 |
| llm:geography.coordinate.longitude | 32 | 0.344 |
| llm:geography.location.city | 30 | 0.733 |
| llm:geography.location.country_code | 31 | 0.419 |
| llm:geography.location.region | 35 | 0.343 |
| llm:identity.commerce.isbn | 1 | 1.000 |
| llm:representation.discrete.categorical | 43 | 0.209 |
| llm:representation.identifier.alphanumeric_id | 37 | 0.324 |
| llm:representation.numeric.decimal_number | 1 | 1.000 |
| llm:representation.numeric.integer_number | 3 | 0.333 |
| llm:representation.text.plain_text | 3 | 0.000 |
| llm:technology.internet.url | 30 | 0.300 |
| tier1:datetime.offset.utc | 35 | 0.057 |
| tier1:geography.coordinate.latitude | 8 | 0.000 |
| tier1:geography.coordinate.longitude | 12 | 0.250 |
| tier1:geography.location.city | 3 | 0.000 |
| tier1:geography.location.country_code | 13 | 0.000 |
| tier1:geography.location.region | 2 | 0.500 |
| tier1:representation.discrete.categorical | 2 | 0.000 |
| tier1:representation.identifier.alphanumeric_id | 4 | 0.250 |
| tier1:technology.internet.url | 24 | 0.250 |
| tier2:datetime.component.year | 12 | 0.333 |
| tier2:datetime.epoch.unix_seconds | 20 | 0.000 |
| tier2:finance.currency.amount | 18 | 0.278 |
| tier2:geography.address.postal_code | 10 | 0.500 |
| tier2:identity.commerce.isbn | 29 | 0.552 |
| tier2:technology.internet.data_uri | 6 | 0.000 |
| tier2:technology.internet.top_level_domain | 4 | 0.000 |

Per-label precision/recall (the curated label is ground truth; YDF is not consulted):

| Curated label | Support | TP | FP | FN | Precision (95% CI) | Recall (95% CI) |
|---------------|--------:|---:|---:|---:|-------------------:|----------------:|
| container.object.csv | 1 | 0 | 0 | 1 | n/a (n/a) | 0.000 (0.00-0.79) |
| datetime.component.year | 41 | 36 | 8 | 5 | 0.818 (0.68-0.90) | 0.878 (0.74-0.95) |
| datetime.date.dmy_slash | 1 | 0 | 0 | 1 | n/a (n/a) | 0.000 (0.00-0.79) |
| datetime.date.iso | 52 | 0 | 0 | 52 | n/a (n/a) | 0.000 (0.00-0.07) |
| datetime.date.mdy_slash | 4 | 0 | 0 | 4 | n/a (n/a) | 0.000 (0.00-0.49) |
| datetime.epoch.unix_milliseconds | 4 | 0 | 0 | 4 | n/a (n/a) | 0.000 (0.00-0.49) |
| datetime.epoch.unix_seconds | 15 | 0 | 0 | 15 | n/a (n/a) | 0.000 (0.00-0.20) |
| datetime.offset.iana | 4 | 2 | 0 | 2 | 1.000 (0.34-1.00) | 0.500 (0.15-0.85) |
| datetime.offset.utc | 1 | 0 | 1 | 1 | 0.000 (0.00-0.79) | 0.000 (0.00-0.79) |
| datetime.timestamp.dmy_hm | 1 | 0 | 0 | 1 | n/a (n/a) | 0.000 (0.00-0.79) |
| datetime.timestamp.iso_8601_milliseconds | 4 | 0 | 0 | 4 | n/a (n/a) | 0.000 (0.00-0.49) |
| datetime.timestamp.sql_standard | 14 | 0 | 0 | 14 | n/a (n/a) | 0.000 (0.00-0.22) |
| finance.currency.amount | 5 | 0 | 1 | 5 | 0.000 (0.00-0.79) | 0.000 (0.00-0.43) |
| geography.address.postal_code | 4 | 0 | 0 | 4 | n/a (n/a) | 0.000 (0.00-0.49) |
| geography.coordinate.latitude | 39 | 3 | 4 | 36 | 0.429 (0.16-0.75) | 0.077 (0.03-0.20) |
| geography.coordinate.longitude | 45 | 2 | 8 | 43 | 0.200 (0.06-0.51) | 0.044 (0.01-0.15) |
| geography.location.city | 24 | 23 | 20 | 1 | 0.535 (0.39-0.67) | 0.958 (0.80-0.99) |
| geography.location.continent | 1 | 1 | 0 | 0 | 1.000 (0.21-1.00) | 1.000 (0.21-1.00) |
| geography.location.country | 11 | 10 | 41 | 1 | 0.196 (0.11-0.32) | 0.909 (0.62-0.98) |
| geography.location.country_code | 54 | 3 | 0 | 51 | 1.000 (0.44-1.00) | 0.056 (0.02-0.15) |
| geography.location.region | 15 | 7 | 19 | 8 | 0.269 (0.14-0.46) | 0.467 (0.25-0.70) |
| geography.location.state_code | 7 | 1 | 4 | 6 | 0.200 (0.04-0.62) | 0.143 (0.03-0.51) |
| geography.transportation.iata_code | 2 | 1 | 3 | 1 | 0.250 (0.05-0.70) | 0.500 (0.09-0.91) |
| geography.transportation.icao_code | 1 | 0 | 0 | 1 | n/a (n/a) | 0.000 (0.00-0.79) |
| identity.commerce.isbn | 18 | 15 | 41 | 3 | 0.268 (0.17-0.40) | 0.833 (0.61-0.94) |
| identity.person.full_name | 1 | 0 | 0 | 1 | n/a (n/a) | 0.000 (0.00-0.79) |
| identity.person.gender | 1 | 1 | 0 | 0 | 1.000 (0.21-1.00) | 1.000 (0.21-1.00) |
| identity.person.username | 1 | 0 | 2 | 1 | 0.000 (0.00-0.66) | 0.000 (0.00-0.79) |
| representation.boolean.terms | 10 | 3 | 1 | 7 | 0.750 (0.30-0.95) | 0.300 (0.11-0.60) |
| representation.file.extension | 1 | 0 | 0 | 1 | n/a (n/a) | 0.000 (0.00-0.79) |
| representation.identifier.alphanumeric_id | 62 | 15 | 2 | 47 | 0.882 (0.66-0.97) | 0.242 (0.15-0.36) |
| representation.identifier.increment | 1 | 0 | 3 | 1 | 0.000 (0.00-0.56) | 0.000 (0.00-0.79) |
| representation.identifier.numeric_code | 3 | 0 | 63 | 3 | 0.000 (0.00-0.06) | 0.000 (0.00-0.56) |
| representation.identifier.uuid | 2 | 1 | 0 | 1 | 1.000 (0.21-1.00) | 0.500 (0.09-0.91) |
| representation.numeric.decimal_number | 94 | 49 | 100 | 45 | 0.329 (0.26-0.41) | 0.521 (0.42-0.62) |
| representation.numeric.integer_number | 194 | 44 | 2 | 150 | 0.957 (0.85-0.99) | 0.227 (0.17-0.29) |
| representation.text.RESIDUAL | 129 | 23 | 60 | 106 | 0.277 (0.19-0.38) | 0.178 (0.12-0.25) |
| representation.text.entity_name | 8 | 4 | 73 | 4 | 0.052 (0.02-0.13) | 0.500 (0.22-0.78) |
| technology.internet.hostname | 2 | 2 | 0 | 0 | 1.000 (0.34-1.00) | 1.000 (0.34-1.00) |
| technology.internet.top_level_domain | 6 | 0 | 1 | 6 | 0.000 (0.00-0.79) | 0.000 (0.00-0.39) |
| technology.internet.url | 44 | 9 | 0 | 35 | 1.000 (0.70-1.00) | 0.205 (0.11-0.35) |

**Headline — column accuracy:** 255/927 = 0.275 (95% CI 0.247-0.305)  
**Macro precision** (mean over labels): 0.479  
**Macro recall** (mean over labels): 0.269  
