# Gold eval anchor — s42-composed-fixed

**Date:** 2026-06-23  
**Gold fixture:** `eval/gold/gold_corpus.tsv` (931 columns)  
**Scoring mode:** ENUM REFRAME (categorical/word/plain_text = one text residual)  
**Predictions:** `output/dual-encoder-native/ac01/pred_s42_composed_fixed.tsv`  
**Scored:** 927 columns (4 gold columns had no prediction)  

Per-family accuracy (fraction of columns where the model's prediction equals the curated gold label — labels neither lens produced):

| Family | Columns | Accuracy |
|--------|--------:|---------:|
| A_tight_code_vs_alnum | 30 | 0.267 |
| B_country_vs_categorical | 60 | 0.983 |
| C_lat_lon_temperature | 90 | 1.000 |
| D_year_vs_integer | 60 | 0.933 |
| author-open:datetime.component.year | 1 | 0.000 |
| author-open:finance.currency.amount | 5 | 0.000 |
| author-open:geography.location.region | 2 | 1.000 |
| author-open:representation.discrete.categorical | 4 | 0.500 |
| author-open:representation.identifier.alphanumeric_id | 1 | 0.000 |
| author-open:representation.text.plain_text | 2 | 0.500 |
| author-open:technology.internet.url | 1 | 0.000 |
| backbone:datetime.date.iso | 51 | 1.000 |
| backbone:representation.numeric.decimal_number | 12 | 0.750 |
| backbone:representation.numeric.integer_number | 21 | 1.000 |
| backbone:representation.text.plain_text | 7 | 0.429 |
| external:datetime.component.year | 2 | 1.000 |
| external:datetime.date.iso | 9 | 0.333 |
| external:datetime.offset.utc | 1 | 1.000 |
| external:geography.address.postal_code | 1 | 0.000 |
| external:geography.coordinate.latitude | 6 | 0.833 |
| external:geography.coordinate.longitude | 4 | 0.750 |
| external:geography.location.city | 1 | 1.000 |
| external:representation.discrete.categorical | 9 | 0.222 |
| external:representation.identifier.alphanumeric_id | 4 | 0.750 |
| external:representation.numeric.decimal_number | 3 | 0.667 |
| external:representation.numeric.integer_number | 7 | 0.714 |
| external:representation.text.plain_text | 5 | 0.200 |
| external:technology.internet.top_level_domain | 2 | 0.000 |
| external:technology.internet.url | 2 | 0.500 |
| llm:datetime.component.year | 9 | 0.333 |
| llm:datetime.date.iso | 4 | 1.000 |
| llm:datetime.epoch.unix_seconds | 4 | 0.750 |
| llm:datetime.offset.utc | 28 | 0.536 |
| llm:geography.address.postal_code | 2 | 1.000 |
| llm:geography.coordinate.latitude | 29 | 0.759 |
| llm:geography.coordinate.longitude | 32 | 0.781 |
| llm:geography.location.city | 30 | 0.833 |
| llm:geography.location.country_code | 31 | 0.903 |
| llm:geography.location.region | 35 | 0.714 |
| llm:identity.commerce.isbn | 1 | 1.000 |
| llm:representation.discrete.categorical | 43 | 0.419 |
| llm:representation.identifier.alphanumeric_id | 37 | 0.568 |
| llm:representation.numeric.decimal_number | 1 | 0.000 |
| llm:representation.numeric.integer_number | 3 | 1.000 |
| llm:representation.text.plain_text | 3 | 0.000 |
| llm:technology.internet.url | 30 | 0.533 |
| tier1:datetime.offset.utc | 35 | 0.829 |
| tier1:geography.coordinate.latitude | 8 | 0.750 |
| tier1:geography.coordinate.longitude | 12 | 0.750 |
| tier1:geography.location.city | 3 | 0.333 |
| tier1:geography.location.country_code | 13 | 0.769 |
| tier1:geography.location.region | 2 | 0.500 |
| tier1:representation.discrete.categorical | 2 | 0.500 |
| tier1:representation.identifier.alphanumeric_id | 4 | 0.500 |
| tier1:technology.internet.url | 24 | 0.917 |
| tier2:datetime.component.year | 12 | 0.583 |
| tier2:datetime.epoch.unix_seconds | 20 | 0.750 |
| tier2:finance.currency.amount | 18 | 1.000 |
| tier2:geography.address.postal_code | 10 | 0.900 |
| tier2:identity.commerce.isbn | 29 | 0.690 |
| tier2:technology.internet.data_uri | 6 | 1.000 |
| tier2:technology.internet.top_level_domain | 4 | 0.750 |

Per-label precision/recall (the curated label is ground truth; YDF is not consulted):

| Curated label | Support | TP | FP | FN | Precision (95% CI) | Recall (95% CI) |
|---------------|--------:|---:|---:|---:|-------------------:|----------------:|
| container.object.csv | 1 | 0 | 0 | 1 | n/a (n/a) | 0.000 (0.00-0.79) |
| datetime.component.year | 41 | 39 | 3 | 2 | 0.929 (0.81-0.98) | 0.951 (0.84-0.99) |
| datetime.date.dmy_slash | 1 | 1 | 0 | 0 | 1.000 (0.21-1.00) | 1.000 (0.21-1.00) |
| datetime.date.iso | 52 | 52 | 0 | 0 | 1.000 (0.93-1.00) | 1.000 (0.93-1.00) |
| datetime.date.mdy_slash | 4 | 4 | 0 | 0 | 1.000 (0.51-1.00) | 1.000 (0.51-1.00) |
| datetime.epoch.unix_milliseconds | 4 | 3 | 0 | 1 | 1.000 (0.44-1.00) | 0.750 (0.30-0.95) |
| datetime.epoch.unix_seconds | 15 | 11 | 6 | 4 | 0.647 (0.41-0.83) | 0.733 (0.48-0.89) |
| datetime.offset.iana | 4 | 3 | 0 | 1 | 1.000 (0.44-1.00) | 0.750 (0.30-0.95) |
| datetime.offset.utc | 1 | 1 | 4 | 0 | 0.200 (0.04-0.62) | 1.000 (0.21-1.00) |
| datetime.timestamp.dmy_hm | 1 | 0 | 0 | 1 | n/a (n/a) | 0.000 (0.00-0.79) |
| datetime.timestamp.iso_8601_milliseconds | 4 | 3 | 0 | 1 | 1.000 (0.44-1.00) | 0.750 (0.30-0.95) |
| datetime.timestamp.sql_standard | 14 | 9 | 0 | 5 | 1.000 (0.70-1.00) | 0.643 (0.39-0.84) |
| finance.currency.amount | 5 | 0 | 0 | 5 | n/a (n/a) | 0.000 (0.00-0.43) |
| geography.address.postal_code | 4 | 3 | 2 | 1 | 0.600 (0.23-0.88) | 0.750 (0.30-0.95) |
| geography.coordinate.latitude | 39 | 39 | 1 | 0 | 0.975 (0.87-1.00) | 1.000 (0.91-1.00) |
| geography.coordinate.longitude | 45 | 44 | 0 | 1 | 1.000 (0.92-1.00) | 0.978 (0.88-1.00) |
| geography.location.city | 24 | 24 | 8 | 0 | 0.750 (0.58-0.87) | 1.000 (0.86-1.00) |
| geography.location.continent | 1 | 1 | 0 | 0 | 1.000 (0.21-1.00) | 1.000 (0.21-1.00) |
| geography.location.country | 11 | 10 | 2 | 1 | 0.833 (0.55-0.95) | 0.909 (0.62-0.98) |
| geography.location.country_code | 54 | 53 | 1 | 1 | 0.981 (0.90-1.00) | 0.981 (0.90-1.00) |
| geography.location.region | 15 | 12 | 1 | 3 | 0.923 (0.67-0.99) | 0.800 (0.55-0.93) |
| geography.location.state_code | 7 | 6 | 4 | 1 | 0.600 (0.31-0.83) | 0.857 (0.49-0.97) |
| geography.transportation.iata_code | 2 | 1 | 7 | 1 | 0.125 (0.02-0.47) | 0.500 (0.09-0.91) |
| geography.transportation.icao_code | 1 | 1 | 1 | 0 | 0.500 (0.09-0.91) | 1.000 (0.21-1.00) |
| identity.commerce.isbn | 18 | 15 | 0 | 3 | 1.000 (0.80-1.00) | 0.833 (0.61-0.94) |
| identity.person.full_name | 1 | 0 | 3 | 1 | 0.000 (0.00-0.56) | 0.000 (0.00-0.79) |
| identity.person.gender | 1 | 1 | 0 | 0 | 1.000 (0.21-1.00) | 1.000 (0.21-1.00) |
| identity.person.username | 1 | 0 | 0 | 1 | n/a (n/a) | 0.000 (0.00-0.79) |
| representation.boolean.terms | 10 | 8 | 1 | 2 | 0.889 (0.56-0.98) | 0.800 (0.49-0.94) |
| representation.file.extension | 1 | 0 | 0 | 1 | n/a (n/a) | 0.000 (0.00-0.79) |
| representation.identifier.alphanumeric_id | 62 | 17 | 2 | 45 | 0.895 (0.69-0.97) | 0.274 (0.18-0.40) |
| representation.identifier.increment | 1 | 1 | 7 | 0 | 0.125 (0.02-0.47) | 1.000 (0.21-1.00) |
| representation.identifier.numeric_code | 3 | 1 | 0 | 2 | 1.000 (0.21-1.00) | 0.333 (0.06-0.79) |
| representation.identifier.uuid | 2 | 2 | 0 | 0 | 1.000 (0.34-1.00) | 1.000 (0.34-1.00) |
| representation.numeric.decimal_number | 94 | 72 | 8 | 22 | 0.900 (0.81-0.95) | 0.766 (0.67-0.84) |
| representation.numeric.integer_number | 194 | 160 | 1 | 34 | 0.994 (0.97-1.00) | 0.825 (0.77-0.87) |
| representation.text.RESIDUAL | 129 | 57 | 11 | 72 | 0.838 (0.73-0.91) | 0.442 (0.36-0.53) |
| representation.text.entity_name | 8 | 7 | 15 | 1 | 0.318 (0.16-0.53) | 0.875 (0.53-0.98) |
| technology.internet.hostname | 2 | 2 | 2 | 0 | 0.500 (0.15-0.85) | 1.000 (0.34-1.00) |
| technology.internet.top_level_domain | 6 | 2 | 1 | 4 | 0.667 (0.21-0.94) | 0.333 (0.10-0.70) |
| technology.internet.url | 44 | 36 | 11 | 8 | 0.766 (0.63-0.86) | 0.818 (0.68-0.90) |

**Headline — column accuracy:** 701/927 = 0.756 (95% CI 0.728-0.783)  
**Macro precision** (mean over labels): 0.777  
**Macro recall** (mean over labels): 0.699  
