# Gold eval anchor — offline-sense-reframe

**Date:** 2026-06-23  
**Gold fixture:** `eval/gold/gold_corpus.tsv` (931 columns)  
**Scoring mode:** ENUM REFRAME (categorical/word/plain_text = one text residual)  
**Predictions:** `output/embed-frontier/preds/m2v8m-s44_sense.tsv`  
**Scored:** 931 columns (0 gold columns had no prediction)  

Per-family accuracy (fraction of columns where the model's prediction equals the curated gold label — labels neither lens produced):

| Family | Columns | Accuracy |
|--------|--------:|---------:|
| A_tight_code_vs_alnum | 30 | 0.500 |
| B_country_vs_categorical | 60 | 0.450 |
| C_lat_lon_temperature | 90 | 0.300 |
| D_year_vs_integer | 60 | 0.517 |
| author-open:datetime.component.year | 1 | 0.000 |
| author-open:finance.currency.amount | 5 | 0.000 |
| author-open:geography.location.region | 2 | 1.000 |
| author-open:representation.discrete.categorical | 4 | 0.750 |
| author-open:representation.identifier.alphanumeric_id | 1 | 0.000 |
| author-open:representation.text.plain_text | 2 | 0.000 |
| author-open:technology.internet.url | 1 | 1.000 |
| backbone:datetime.date.iso | 51 | 1.000 |
| backbone:representation.numeric.decimal_number | 12 | 0.750 |
| backbone:representation.numeric.integer_number | 21 | 0.476 |
| backbone:representation.text.plain_text | 7 | 0.714 |
| external:datetime.component.year | 2 | 0.000 |
| external:datetime.date.iso | 9 | 0.222 |
| external:datetime.offset.utc | 1 | 0.000 |
| external:geography.address.postal_code | 1 | 0.000 |
| external:geography.coordinate.latitude | 6 | 0.500 |
| external:geography.coordinate.longitude | 4 | 0.750 |
| external:geography.location.city | 1 | 1.000 |
| external:representation.discrete.categorical | 9 | 0.111 |
| external:representation.identifier.alphanumeric_id | 4 | 0.250 |
| external:representation.numeric.decimal_number | 3 | 0.667 |
| external:representation.numeric.integer_number | 7 | 0.143 |
| external:representation.text.plain_text | 5 | 0.200 |
| external:technology.internet.top_level_domain | 2 | 0.000 |
| external:technology.internet.url | 2 | 0.000 |
| llm:datetime.component.year | 9 | 0.889 |
| llm:datetime.date.iso | 4 | 0.250 |
| llm:datetime.epoch.unix_seconds | 4 | 0.500 |
| llm:datetime.offset.utc | 28 | 0.393 |
| llm:geography.address.postal_code | 2 | 0.000 |
| llm:geography.coordinate.latitude | 30 | 0.667 |
| llm:geography.coordinate.longitude | 32 | 0.375 |
| llm:geography.location.city | 33 | 0.788 |
| llm:geography.location.country_code | 31 | 0.710 |
| llm:geography.location.region | 35 | 0.486 |
| llm:identity.commerce.isbn | 1 | 1.000 |
| llm:representation.discrete.categorical | 43 | 0.349 |
| llm:representation.identifier.alphanumeric_id | 37 | 0.378 |
| llm:representation.numeric.decimal_number | 1 | 1.000 |
| llm:representation.numeric.integer_number | 3 | 1.000 |
| llm:representation.text.plain_text | 3 | 0.333 |
| llm:technology.internet.url | 30 | 0.633 |
| tier1:datetime.offset.utc | 35 | 0.514 |
| tier1:geography.coordinate.latitude | 8 | 0.125 |
| tier1:geography.coordinate.longitude | 12 | 0.500 |
| tier1:geography.location.city | 3 | 0.000 |
| tier1:geography.location.country_code | 13 | 0.538 |
| tier1:geography.location.region | 2 | 0.500 |
| tier1:representation.discrete.categorical | 2 | 0.500 |
| tier1:representation.identifier.alphanumeric_id | 4 | 0.750 |
| tier1:technology.internet.url | 24 | 0.583 |
| tier2:datetime.component.year | 12 | 1.000 |
| tier2:datetime.epoch.unix_seconds | 20 | 0.400 |
| tier2:finance.currency.amount | 18 | 0.278 |
| tier2:geography.address.postal_code | 10 | 0.300 |
| tier2:identity.commerce.isbn | 29 | 0.552 |
| tier2:technology.internet.data_uri | 6 | 0.167 |
| tier2:technology.internet.top_level_domain | 4 | 0.750 |

Per-label precision/recall (the curated label is ground truth; YDF is not consulted):

| Curated label | Support | TP | FP | FN | Precision (95% CI) | Recall (95% CI) |
|---------------|--------:|---:|---:|---:|-------------------:|----------------:|
| container.object.csv | 1 | 0 | 0 | 1 | n/a (n/a) | 0.000 (0.00-0.79) |
| datetime.component.year | 41 | 38 | 13 | 3 | 0.745 (0.61-0.84) | 0.927 (0.81-0.97) |
| datetime.date.dmy_slash | 1 | 0 | 3 | 1 | 0.000 (0.00-0.56) | 0.000 (0.00-0.79) |
| datetime.date.iso | 52 | 52 | 0 | 0 | 1.000 (0.93-1.00) | 1.000 (0.93-1.00) |
| datetime.date.mdy_slash | 4 | 1 | 1 | 3 | 0.500 (0.09-0.91) | 0.250 (0.05-0.70) |
| datetime.epoch.unix_milliseconds | 4 | 1 | 0 | 3 | 1.000 (0.21-1.00) | 0.250 (0.05-0.70) |
| datetime.epoch.unix_seconds | 15 | 4 | 4 | 11 | 0.500 (0.22-0.78) | 0.267 (0.11-0.52) |
| datetime.offset.iana | 4 | 3 | 0 | 1 | 1.000 (0.44-1.00) | 0.750 (0.30-0.95) |
| datetime.offset.utc | 1 | 0 | 1 | 1 | 0.000 (0.00-0.79) | 0.000 (0.00-0.79) |
| datetime.timestamp.dmy_hm | 1 | 0 | 0 | 1 | n/a (n/a) | 0.000 (0.00-0.79) |
| datetime.timestamp.iso_8601_milliseconds | 4 | 3 | 0 | 1 | 1.000 (0.44-1.00) | 0.750 (0.30-0.95) |
| datetime.timestamp.sql_standard | 14 | 8 | 0 | 6 | 1.000 (0.68-1.00) | 0.571 (0.33-0.79) |
| finance.currency.amount | 5 | 0 | 0 | 5 | n/a (n/a) | 0.000 (0.00-0.43) |
| geography.address.full_address | 4 | 4 | 8 | 0 | 0.333 (0.14-0.61) | 1.000 (0.51-1.00) |
| geography.address.postal_code | 4 | 0 | 2 | 4 | 0.000 (0.00-0.66) | 0.000 (0.00-0.49) |
| geography.coordinate.latitude | 39 | 5 | 9 | 34 | 0.357 (0.16-0.61) | 0.128 (0.06-0.27) |
| geography.coordinate.longitude | 45 | 6 | 14 | 39 | 0.300 (0.15-0.52) | 0.133 (0.06-0.26) |
| geography.location.city | 24 | 23 | 9 | 1 | 0.719 (0.55-0.84) | 0.958 (0.80-0.99) |
| geography.location.continent | 1 | 1 | 1 | 0 | 0.500 (0.09-0.91) | 1.000 (0.21-1.00) |
| geography.location.country | 11 | 10 | 1 | 1 | 0.909 (0.62-0.98) | 0.909 (0.62-0.98) |
| geography.location.country_code | 54 | 44 | 31 | 10 | 0.587 (0.47-0.69) | 0.815 (0.69-0.90) |
| geography.location.region | 15 | 9 | 18 | 6 | 0.333 (0.19-0.52) | 0.600 (0.36-0.80) |
| geography.location.state_code | 7 | 0 | 3 | 7 | 0.000 (0.00-0.56) | 0.000 (0.00-0.35) |
| geography.transportation.iata_code | 2 | 2 | 12 | 0 | 0.143 (0.04-0.40) | 1.000 (0.34-1.00) |
| geography.transportation.icao_code | 1 | 1 | 0 | 0 | 1.000 (0.21-1.00) | 1.000 (0.21-1.00) |
| identity.commerce.isbn | 18 | 15 | 19 | 3 | 0.441 (0.29-0.61) | 0.833 (0.61-0.94) |
| identity.person.full_name | 1 | 1 | 2 | 0 | 0.333 (0.06-0.79) | 1.000 (0.21-1.00) |
| identity.person.gender | 1 | 1 | 0 | 0 | 1.000 (0.21-1.00) | 1.000 (0.21-1.00) |
| identity.person.username | 1 | 1 | 3 | 0 | 0.250 (0.05-0.70) | 1.000 (0.21-1.00) |
| representation.boolean.terms | 10 | 6 | 2 | 4 | 0.750 (0.41-0.93) | 0.600 (0.31-0.83) |
| representation.file.extension | 1 | 0 | 0 | 1 | n/a (n/a) | 0.000 (0.00-0.79) |
| representation.identifier.alphanumeric_id | 62 | 28 | 5 | 34 | 0.848 (0.69-0.93) | 0.452 (0.33-0.57) |
| representation.identifier.increment | 1 | 0 | 3 | 1 | 0.000 (0.00-0.56) | 0.000 (0.00-0.79) |
| representation.identifier.numeric_code | 3 | 2 | 61 | 1 | 0.032 (0.01-0.11) | 0.667 (0.21-0.94) |
| representation.identifier.uuid | 2 | 2 | 0 | 0 | 1.000 (0.34-1.00) | 1.000 (0.34-1.00) |
| representation.numeric.decimal_number | 94 | 75 | 80 | 19 | 0.484 (0.41-0.56) | 0.798 (0.71-0.87) |
| representation.numeric.integer_number | 194 | 52 | 1 | 142 | 0.981 (0.90-1.00) | 0.268 (0.21-0.33) |
| representation.text.RESIDUAL | 129 | 35 | 21 | 94 | 0.625 (0.49-0.74) | 0.271 (0.20-0.35) |
| representation.text.entity_name | 8 | 5 | 20 | 3 | 0.200 (0.09-0.39) | 0.625 (0.31-0.86) |
| technology.internet.hostname | 2 | 2 | 0 | 0 | 1.000 (0.34-1.00) | 1.000 (0.34-1.00) |
| technology.internet.top_level_domain | 6 | 2 | 1 | 4 | 0.667 (0.21-0.94) | 0.333 (0.10-0.70) |
| technology.internet.url | 44 | 26 | 0 | 18 | 1.000 (0.87-1.00) | 0.591 (0.44-0.72) |

**Headline — column accuracy:** 468/931 = 0.503 (95% CI 0.471-0.535)  
**Macro precision** (mean over labels): 0.567  
**Macro recall** (mean over labels): 0.542  
