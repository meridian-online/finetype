# Gold eval anchor — off-code16m-s44

**Date:** 2026-06-23  
**Gold fixture:** `eval/gold/gold_corpus.tsv` (931 columns)  
**Scoring mode:** ENUM REFRAME (categorical/word/plain_text = one text residual)  
**Predictions:** `output/embed-frontier/preds/m2v-code16m-s44_composed.tsv`  
**Scored:** 931 columns (0 gold columns had no prediction)  

Per-family accuracy (fraction of columns where the model's prediction equals the curated gold label — labels neither lens produced):

| Family | Columns | Accuracy |
|--------|--------:|---------:|
| A_tight_code_vs_alnum | 30 | 0.567 |
| B_country_vs_categorical | 60 | 0.983 |
| C_lat_lon_temperature | 90 | 1.000 |
| D_year_vs_integer | 60 | 0.933 |
| author-open:datetime.component.year | 1 | 0.000 |
| author-open:finance.currency.amount | 5 | 0.000 |
| author-open:geography.location.region | 2 | 0.000 |
| author-open:representation.discrete.categorical | 4 | 0.500 |
| author-open:representation.identifier.alphanumeric_id | 1 | 0.000 |
| author-open:representation.text.plain_text | 2 | 0.000 |
| author-open:technology.internet.url | 1 | 0.000 |
| backbone:datetime.date.iso | 51 | 1.000 |
| backbone:representation.numeric.decimal_number | 12 | 0.750 |
| backbone:representation.numeric.integer_number | 21 | 1.000 |
| backbone:representation.text.plain_text | 7 | 0.571 |
| external:datetime.component.year | 2 | 1.000 |
| external:datetime.date.iso | 9 | 0.333 |
| external:datetime.offset.utc | 1 | 1.000 |
| external:geography.address.postal_code | 1 | 0.000 |
| external:geography.coordinate.latitude | 6 | 1.000 |
| external:geography.coordinate.longitude | 4 | 0.750 |
| external:geography.location.city | 1 | 1.000 |
| external:representation.discrete.categorical | 9 | 0.333 |
| external:representation.identifier.alphanumeric_id | 4 | 0.500 |
| external:representation.numeric.decimal_number | 3 | 1.000 |
| external:representation.numeric.integer_number | 7 | 0.714 |
| external:representation.text.plain_text | 5 | 0.400 |
| external:technology.internet.top_level_domain | 2 | 1.000 |
| external:technology.internet.url | 2 | 0.500 |
| llm:datetime.component.year | 9 | 0.444 |
| llm:datetime.date.iso | 4 | 1.000 |
| llm:datetime.epoch.unix_seconds | 4 | 0.250 |
| llm:datetime.offset.utc | 28 | 0.607 |
| llm:geography.address.postal_code | 2 | 1.000 |
| llm:geography.coordinate.latitude | 30 | 0.833 |
| llm:geography.coordinate.longitude | 32 | 0.906 |
| llm:geography.location.city | 33 | 0.788 |
| llm:geography.location.country_code | 31 | 0.871 |
| llm:geography.location.region | 35 | 0.686 |
| llm:identity.commerce.isbn | 1 | 1.000 |
| llm:representation.discrete.categorical | 43 | 0.419 |
| llm:representation.identifier.alphanumeric_id | 37 | 0.514 |
| llm:representation.numeric.decimal_number | 1 | 0.000 |
| llm:representation.numeric.integer_number | 3 | 1.000 |
| llm:representation.text.plain_text | 3 | 0.000 |
| llm:technology.internet.url | 30 | 0.700 |
| tier1:datetime.offset.utc | 35 | 0.829 |
| tier1:geography.coordinate.latitude | 8 | 0.750 |
| tier1:geography.coordinate.longitude | 12 | 0.833 |
| tier1:geography.location.city | 3 | 0.333 |
| tier1:geography.location.country_code | 13 | 0.769 |
| tier1:geography.location.region | 2 | 0.000 |
| tier1:representation.discrete.categorical | 2 | 0.500 |
| tier1:representation.identifier.alphanumeric_id | 4 | 0.500 |
| tier1:technology.internet.url | 24 | 0.833 |
| tier2:datetime.component.year | 12 | 1.000 |
| tier2:datetime.epoch.unix_seconds | 20 | 0.750 |
| tier2:finance.currency.amount | 18 | 1.000 |
| tier2:geography.address.postal_code | 10 | 1.000 |
| tier2:identity.commerce.isbn | 29 | 0.690 |
| tier2:technology.internet.data_uri | 6 | 1.000 |
| tier2:technology.internet.top_level_domain | 4 | 0.750 |

Per-label precision/recall (the curated label is ground truth; YDF is not consulted):

| Curated label | Support | TP | FP | FN | Precision (95% CI) | Recall (95% CI) |
|---------------|--------:|---:|---:|---:|-------------------:|----------------:|
| container.object.csv | 1 | 0 | 0 | 1 | n/a (n/a) | 0.000 (0.00-0.79) |
| datetime.component.year | 41 | 40 | 3 | 1 | 0.930 (0.81-0.98) | 0.976 (0.87-1.00) |
| datetime.date.dmy_slash | 1 | 1 | 0 | 0 | 1.000 (0.21-1.00) | 1.000 (0.21-1.00) |
| datetime.date.iso | 52 | 52 | 0 | 0 | 1.000 (0.93-1.00) | 1.000 (0.93-1.00) |
| datetime.date.mdy_slash | 4 | 4 | 0 | 0 | 1.000 (0.51-1.00) | 1.000 (0.51-1.00) |
| datetime.epoch.unix_milliseconds | 4 | 1 | 0 | 3 | 1.000 (0.21-1.00) | 0.250 (0.05-0.70) |
| datetime.epoch.unix_seconds | 15 | 10 | 1 | 5 | 0.909 (0.62-0.98) | 0.667 (0.42-0.85) |
| datetime.offset.iana | 4 | 4 | 0 | 0 | 1.000 (0.51-1.00) | 1.000 (0.51-1.00) |
| datetime.offset.utc | 1 | 1 | 4 | 0 | 0.200 (0.04-0.62) | 1.000 (0.21-1.00) |
| datetime.timestamp.dmy_hm | 1 | 1 | 0 | 0 | 1.000 (0.21-1.00) | 1.000 (0.21-1.00) |
| datetime.timestamp.iso_8601_milliseconds | 4 | 3 | 0 | 1 | 1.000 (0.44-1.00) | 0.750 (0.30-0.95) |
| datetime.timestamp.sql_standard | 14 | 9 | 0 | 5 | 1.000 (0.70-1.00) | 0.643 (0.39-0.84) |
| finance.currency.amount | 5 | 0 | 0 | 5 | n/a (n/a) | 0.000 (0.00-0.43) |
| geography.address.full_address | 4 | 4 | 7 | 0 | 0.364 (0.15-0.65) | 1.000 (0.51-1.00) |
| geography.address.postal_code | 4 | 3 | 1 | 1 | 0.750 (0.30-0.95) | 0.750 (0.30-0.95) |
| geography.coordinate.latitude | 39 | 39 | 1 | 0 | 0.975 (0.87-1.00) | 1.000 (0.91-1.00) |
| geography.coordinate.longitude | 45 | 44 | 0 | 1 | 1.000 (0.92-1.00) | 0.978 (0.88-1.00) |
| geography.location.city | 24 | 22 | 6 | 2 | 0.786 (0.60-0.90) | 0.917 (0.74-0.98) |
| geography.location.continent | 1 | 1 | 1 | 0 | 0.500 (0.09-0.91) | 1.000 (0.21-1.00) |
| geography.location.country | 11 | 8 | 1 | 3 | 0.889 (0.56-0.98) | 0.727 (0.43-0.90) |
| geography.location.country_code | 54 | 53 | 3 | 1 | 0.946 (0.85-0.98) | 0.981 (0.90-1.00) |
| geography.location.region | 15 | 10 | 4 | 5 | 0.714 (0.45-0.88) | 0.667 (0.42-0.85) |
| geography.location.state_code | 7 | 6 | 3 | 1 | 0.667 (0.35-0.88) | 0.857 (0.49-0.97) |
| geography.transportation.iata_code | 2 | 2 | 4 | 0 | 0.333 (0.10-0.70) | 1.000 (0.34-1.00) |
| geography.transportation.icao_code | 1 | 1 | 0 | 0 | 1.000 (0.21-1.00) | 1.000 (0.21-1.00) |
| identity.commerce.isbn | 18 | 15 | 3 | 3 | 0.833 (0.61-0.94) | 0.833 (0.61-0.94) |
| identity.person.full_name | 1 | 0 | 5 | 1 | 0.000 (0.00-0.43) | 0.000 (0.00-0.79) |
| identity.person.gender | 1 | 1 | 0 | 0 | 1.000 (0.21-1.00) | 1.000 (0.21-1.00) |
| identity.person.username | 1 | 0 | 0 | 1 | n/a (n/a) | 0.000 (0.00-0.79) |
| representation.boolean.terms | 10 | 8 | 1 | 2 | 0.889 (0.56-0.98) | 0.800 (0.49-0.94) |
| representation.file.extension | 1 | 0 | 0 | 1 | n/a (n/a) | 0.000 (0.00-0.79) |
| representation.identifier.alphanumeric_id | 62 | 26 | 4 | 36 | 0.867 (0.70-0.95) | 0.419 (0.30-0.54) |
| representation.identifier.increment | 1 | 0 | 7 | 1 | 0.000 (0.00-0.35) | 0.000 (0.00-0.79) |
| representation.identifier.numeric_code | 3 | 2 | 1 | 1 | 0.667 (0.21-0.94) | 0.667 (0.21-0.94) |
| representation.identifier.uuid | 2 | 1 | 0 | 1 | 1.000 (0.21-1.00) | 0.500 (0.09-0.91) |
| representation.numeric.decimal_number | 94 | 83 | 9 | 11 | 0.902 (0.82-0.95) | 0.883 (0.80-0.93) |
| representation.numeric.integer_number | 194 | 162 | 5 | 32 | 0.970 (0.93-0.99) | 0.835 (0.78-0.88) |
| representation.text.RESIDUAL | 129 | 60 | 11 | 69 | 0.845 (0.74-0.91) | 0.465 (0.38-0.55) |
| representation.text.entity_name | 8 | 5 | 22 | 3 | 0.185 (0.08-0.37) | 0.625 (0.31-0.86) |
| technology.internet.hostname | 2 | 2 | 0 | 0 | 1.000 (0.34-1.00) | 1.000 (0.34-1.00) |
| technology.internet.top_level_domain | 6 | 4 | 2 | 2 | 0.667 (0.30-0.90) | 0.667 (0.30-0.90) |
| technology.internet.url | 44 | 39 | 9 | 5 | 0.812 (0.68-0.90) | 0.886 (0.76-0.95) |

**Headline — column accuracy:** 727/931 = 0.781 (95% CI 0.753-0.806)  
**Macro precision** (mean over labels): 0.779  
**Macro recall** (mean over labels): 0.708  
