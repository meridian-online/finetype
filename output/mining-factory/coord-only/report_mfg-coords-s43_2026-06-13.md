# Gold eval anchor — mfg-coords-s43

**Date:** 2026-06-13  
**Gold fixture:** `eval/gold/gold_corpus_v1.tsv` (931 columns)  
**Predictions:** `output/mining-factory/coord-only/predictions_mfg-coords.tsv`  
**Scored:** 931 columns (0 gold columns had no prediction)  

Per-family accuracy (fraction of columns where the model's prediction equals the curated gold label — labels neither lens produced):

| Family | Columns | Accuracy |
|--------|--------:|---------:|
| A_tight_code_vs_alnum | 30 | 0.867 |
| B_country_vs_categorical | 60 | 0.950 |
| C_lat_lon_temperature | 90 | 1.000 |
| D_year_vs_integer | 60 | 0.917 |
| author-open:datetime.component.year | 1 | 0.000 |
| author-open:finance.currency.amount | 5 | 0.600 |
| author-open:geography.location.region | 2 | 0.000 |
| author-open:representation.discrete.categorical | 4 | 0.250 |
| author-open:representation.identifier.alphanumeric_id | 1 | 0.000 |
| author-open:representation.text.plain_text | 2 | 0.500 |
| author-open:technology.internet.url | 1 | 0.000 |
| backbone:datetime.date.iso | 51 | 1.000 |
| backbone:representation.numeric.decimal_number | 12 | 1.000 |
| backbone:representation.numeric.integer_number | 21 | 0.810 |
| backbone:representation.text.plain_text | 7 | 0.714 |
| external:datetime.component.year | 2 | 1.000 |
| external:datetime.date.iso | 9 | 0.000 |
| external:datetime.offset.utc | 1 | 1.000 |
| external:geography.address.postal_code | 1 | 0.000 |
| external:geography.coordinate.latitude | 6 | 0.833 |
| external:geography.coordinate.longitude | 4 | 0.750 |
| external:geography.location.city | 1 | 1.000 |
| external:representation.discrete.categorical | 9 | 0.222 |
| external:representation.identifier.alphanumeric_id | 4 | 0.250 |
| external:representation.numeric.decimal_number | 3 | 1.000 |
| external:representation.numeric.integer_number | 7 | 0.429 |
| external:representation.text.plain_text | 5 | 0.000 |
| external:technology.internet.top_level_domain | 2 | 0.000 |
| external:technology.internet.url | 2 | 1.000 |
| llm:datetime.component.year | 9 | 0.444 |
| llm:datetime.date.iso | 4 | 1.000 |
| llm:datetime.epoch.unix_seconds | 4 | 0.750 |
| llm:datetime.offset.utc | 28 | 0.571 |
| llm:geography.address.postal_code | 2 | 1.000 |
| llm:geography.coordinate.latitude | 30 | 0.833 |
| llm:geography.coordinate.longitude | 32 | 0.844 |
| llm:geography.location.city | 33 | 0.758 |
| llm:geography.location.country_code | 31 | 0.806 |
| llm:geography.location.region | 35 | 0.429 |
| llm:identity.commerce.isbn | 1 | 1.000 |
| llm:representation.discrete.categorical | 43 | 0.349 |
| llm:representation.identifier.alphanumeric_id | 37 | 0.622 |
| llm:representation.numeric.decimal_number | 1 | 0.000 |
| llm:representation.numeric.integer_number | 3 | 0.667 |
| llm:representation.text.plain_text | 3 | 0.333 |
| llm:technology.internet.url | 30 | 0.633 |
| tier1:datetime.offset.utc | 35 | 0.457 |
| tier1:geography.coordinate.latitude | 8 | 0.750 |
| tier1:geography.coordinate.longitude | 12 | 0.667 |
| tier1:geography.location.city | 3 | 0.000 |
| tier1:geography.location.country_code | 13 | 0.615 |
| tier1:geography.location.region | 2 | 0.500 |
| tier1:representation.discrete.categorical | 2 | 0.000 |
| tier1:representation.identifier.alphanumeric_id | 4 | 0.500 |
| tier1:technology.internet.url | 24 | 0.792 |
| tier2:datetime.component.year | 12 | 1.000 |
| tier2:datetime.epoch.unix_seconds | 20 | 0.500 |
| tier2:finance.currency.amount | 18 | 0.111 |
| tier2:geography.address.postal_code | 10 | 1.000 |
| tier2:identity.commerce.isbn | 29 | 0.621 |
| tier2:technology.internet.data_uri | 6 | 1.000 |
| tier2:technology.internet.top_level_domain | 4 | 0.500 |

Per-label precision/recall (the curated label is ground truth; YDF is not consulted):

| Curated label | Support | TP | FP | FN | Precision (95% CI) | Recall (95% CI) |
|---------------|--------:|---:|---:|---:|-------------------:|----------------:|
| container.object.csv | 1 | 0 | 0 | 1 | n/a (n/a) | 0.000 (0.00-0.79) |
| datetime.component.year | 40 | 40 | 3 | 0 | 0.930 (0.81-0.98) | 1.000 (0.91-1.00) |
| datetime.date.dmy_slash | 1 | 1 | 0 | 0 | 1.000 (0.21-1.00) | 1.000 (0.21-1.00) |
| datetime.date.iso | 67 | 52 | 0 | 15 | 1.000 (0.93-1.00) | 0.776 (0.66-0.86) |
| datetime.date.mdy_slash | 4 | 4 | 0 | 0 | 1.000 (0.51-1.00) | 1.000 (0.51-1.00) |
| datetime.epoch.unix_milliseconds | 2 | 2 | 1 | 0 | 0.667 (0.21-0.94) | 1.000 (0.34-1.00) |
| datetime.epoch.unix_seconds | 12 | 8 | 1 | 4 | 0.889 (0.56-0.98) | 0.667 (0.39-0.86) |
| datetime.offset.iana | 3 | 3 | 1 | 0 | 0.750 (0.30-0.95) | 1.000 (0.44-1.00) |
| datetime.offset.utc | 1 | 1 | 4 | 0 | 0.200 (0.04-0.62) | 1.000 (0.21-1.00) |
| datetime.timestamp.dmy_hm | 1 | 0 | 0 | 1 | n/a (n/a) | 0.000 (0.00-0.79) |
| datetime.timestamp.iso_8601_milliseconds | 1 | 1 | 2 | 0 | 0.333 (0.06-0.79) | 1.000 (0.21-1.00) |
| datetime.timestamp.sql_standard | 2 | 2 | 6 | 0 | 0.250 (0.07-0.59) | 1.000 (0.34-1.00) |
| finance.currency.amount | 5 | 3 | 17 | 2 | 0.150 (0.05-0.36) | 0.600 (0.23-0.88) |
| geography.address.full_address | 4 | 4 | 7 | 0 | 0.364 (0.15-0.65) | 1.000 (0.51-1.00) |
| geography.address.postal_code | 4 | 3 | 1 | 1 | 0.750 (0.30-0.95) | 0.750 (0.30-0.95) |
| geography.coordinate.latitude | 39 | 39 | 1 | 0 | 0.975 (0.87-1.00) | 1.000 (0.91-1.00) |
| geography.coordinate.longitude | 45 | 44 | 0 | 1 | 1.000 (0.92-1.00) | 0.978 (0.88-1.00) |
| geography.location.city | 24 | 23 | 8 | 1 | 0.742 (0.57-0.86) | 0.958 (0.80-0.99) |
| geography.location.continent | 1 | 1 | 0 | 0 | 1.000 (0.21-1.00) | 1.000 (0.21-1.00) |
| geography.location.country | 10 | 10 | 3 | 0 | 0.769 (0.50-0.92) | 1.000 (0.72-1.00) |
| geography.location.country_code | 57 | 48 | 2 | 9 | 0.960 (0.87-0.99) | 0.842 (0.73-0.91) |
| geography.location.region | 15 | 6 | 13 | 9 | 0.316 (0.15-0.54) | 0.400 (0.20-0.64) |
| geography.location.state_code | 7 | 0 | 1 | 7 | 0.000 (0.00-0.79) | 0.000 (0.00-0.35) |
| geography.transportation.iata_code | 2 | 2 | 7 | 0 | 0.222 (0.06-0.55) | 1.000 (0.34-1.00) |
| geography.transportation.icao_code | 1 | 1 | 0 | 0 | 1.000 (0.21-1.00) | 1.000 (0.21-1.00) |
| identity.commerce.isbn | 18 | 15 | 9 | 3 | 0.625 (0.43-0.79) | 0.833 (0.61-0.94) |
| identity.person.full_name | 1 | 1 | 5 | 0 | 0.167 (0.03-0.56) | 1.000 (0.21-1.00) |
| identity.person.gender | 1 | 1 | 0 | 0 | 1.000 (0.21-1.00) | 1.000 (0.21-1.00) |
| representation.boolean.terms | 10 | 8 | 1 | 2 | 0.889 (0.56-0.98) | 0.800 (0.49-0.94) |
| representation.discrete.categorical | 101 | 43 | 8 | 58 | 0.843 (0.72-0.92) | 0.426 (0.33-0.52) |
| representation.identifier.alphanumeric_id | 54 | 34 | 3 | 20 | 0.919 (0.79-0.97) | 0.630 (0.50-0.75) |
| representation.identifier.increment | 1 | 1 | 18 | 0 | 0.053 (0.01-0.25) | 1.000 (0.21-1.00) |
| representation.identifier.numeric_code | 3 | 3 | 0 | 0 | 1.000 (0.44-1.00) | 1.000 (0.44-1.00) |
| representation.identifier.uuid | 1 | 1 | 1 | 0 | 0.500 (0.09-0.91) | 1.000 (0.21-1.00) |
| representation.numeric.decimal_number | 97 | 85 | 2 | 12 | 0.977 (0.92-0.99) | 0.876 (0.80-0.93) |
| representation.numeric.integer_number | 196 | 116 | 1 | 80 | 0.991 (0.95-1.00) | 0.592 (0.52-0.66) |
| representation.text.entity_name | 6 | 5 | 19 | 1 | 0.208 (0.09-0.40) | 0.833 (0.44-0.97) |
| representation.text.plain_text | 39 | 9 | 2 | 30 | 0.818 (0.52-0.95) | 0.231 (0.13-0.38) |
| representation.text.word | 2 | 0 | 5 | 2 | 0.000 (0.00-0.43) | 0.000 (0.00-0.66) |
| technology.internet.hostname | 2 | 2 | 0 | 0 | 1.000 (0.34-1.00) | 1.000 (0.34-1.00) |
| technology.internet.top_level_domain | 6 | 2 | 1 | 4 | 0.667 (0.21-0.94) | 0.333 (0.10-0.70) |
| technology.internet.url | 44 | 44 | 17 | 0 | 0.721 (0.60-0.82) | 1.000 (0.92-1.00) |

**Headline — column accuracy:** 668/931 = 0.718 (95% CI 0.688-0.745)  
**Macro precision** (mean over labels): 0.666  
**Macro recall** (mean over labels): 0.774  
