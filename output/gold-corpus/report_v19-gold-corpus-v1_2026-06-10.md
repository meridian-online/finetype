# Gold eval anchor — v19-gold-corpus-v1

**Date:** 2026-06-10  
**Gold fixture:** `eval/gold/gold_corpus_v1.tsv` (915 columns)  
**Predictions:** `output/gold-corpus/predictions_v19.tsv`  
**Scored:** 915 columns (0 gold columns had no prediction)  

Per-family accuracy (fraction of columns where the model's prediction equals the curated gold label — labels neither lens produced):

| Family | Columns | Accuracy |
|--------|--------:|---------:|
| A_tight_code_vs_alnum | 30 | 0.167 |
| B_country_vs_categorical | 60 | 0.967 |
| C_lat_lon_temperature | 90 | 1.000 |
| D_year_vs_integer | 60 | 0.667 |
| backbone:datetime.date.iso | 51 | 1.000 |
| backbone:representation.numeric.decimal_number | 12 | 1.000 |
| backbone:representation.numeric.integer_number | 21 | 0.810 |
| backbone:representation.text.plain_text | 7 | 0.714 |
| external:datetime.component.year | 2 | 1.000 |
| external:datetime.date.iso | 9 | 0.000 |
| external:datetime.offset.utc | 1 | 1.000 |
| external:geography.address.postal_code | 1 | 1.000 |
| external:geography.coordinate.latitude | 6 | 0.833 |
| external:geography.coordinate.longitude | 4 | 0.750 |
| external:geography.location.city | 1 | 1.000 |
| external:representation.discrete.categorical | 9 | 0.222 |
| external:representation.identifier.alphanumeric_id | 4 | 0.000 |
| external:representation.numeric.decimal_number | 3 | 1.000 |
| external:representation.numeric.integer_number | 7 | 0.571 |
| external:representation.text.plain_text | 5 | 0.000 |
| external:technology.internet.top_level_domain | 2 | 0.000 |
| external:technology.internet.url | 2 | 1.000 |
| llm:datetime.component.year | 9 | 0.444 |
| llm:datetime.date.iso | 4 | 1.000 |
| llm:datetime.epoch.unix_seconds | 4 | 0.750 |
| llm:datetime.offset.utc | 28 | 0.607 |
| llm:geography.address.postal_code | 2 | 1.000 |
| llm:geography.coordinate.latitude | 30 | 0.833 |
| llm:geography.coordinate.longitude | 32 | 0.844 |
| llm:geography.location.city | 33 | 0.788 |
| llm:geography.location.country_code | 31 | 0.806 |
| llm:geography.location.region | 35 | 0.457 |
| llm:identity.commerce.isbn | 1 | 1.000 |
| llm:representation.discrete.categorical | 43 | 0.256 |
| llm:representation.identifier.alphanumeric_id | 37 | 0.514 |
| llm:representation.numeric.decimal_number | 1 | 0.000 |
| llm:representation.numeric.integer_number | 3 | 0.667 |
| llm:representation.text.plain_text | 3 | 0.333 |
| llm:technology.internet.url | 30 | 0.633 |
| tier1:datetime.offset.utc | 35 | 0.457 |
| tier1:geography.coordinate.latitude | 8 | 0.750 |
| tier1:geography.coordinate.longitude | 12 | 0.500 |
| tier1:geography.location.city | 3 | 0.000 |
| tier1:geography.location.country_code | 13 | 0.385 |
| tier1:geography.location.region | 2 | 0.000 |
| tier1:representation.discrete.categorical | 2 | 0.000 |
| tier1:representation.identifier.alphanumeric_id | 4 | 0.000 |
| tier1:technology.internet.url | 24 | 0.792 |
| tier2:datetime.component.year | 12 | 0.833 |
| tier2:datetime.epoch.unix_seconds | 20 | 0.750 |
| tier2:finance.currency.amount | 18 | 0.111 |
| tier2:geography.address.postal_code | 10 | 0.100 |
| tier2:identity.commerce.isbn | 29 | 0.483 |
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
| datetime.epoch.unix_milliseconds | 2 | 2 | 2 | 0 | 0.500 (0.15-0.85) | 1.000 (0.34-1.00) |
| datetime.epoch.unix_seconds | 12 | 10 | 3 | 2 | 0.769 (0.50-0.92) | 0.833 (0.55-0.95) |
| datetime.offset.iana | 3 | 3 | 1 | 0 | 0.750 (0.30-0.95) | 1.000 (0.44-1.00) |
| datetime.offset.utc | 1 | 1 | 4 | 0 | 0.200 (0.04-0.62) | 1.000 (0.21-1.00) |
| datetime.timestamp.dmy_hm | 1 | 1 | 0 | 0 | 1.000 (0.21-1.00) | 1.000 (0.21-1.00) |
| datetime.timestamp.iso_8601_milliseconds | 1 | 1 | 2 | 0 | 0.333 (0.06-0.79) | 1.000 (0.21-1.00) |
| datetime.timestamp.sql_standard | 2 | 2 | 6 | 0 | 0.250 (0.07-0.59) | 1.000 (0.34-1.00) |
| geography.address.full_address | 4 | 4 | 4 | 0 | 0.500 (0.22-0.78) | 1.000 (0.51-1.00) |
| geography.address.postal_code | 4 | 4 | 26 | 0 | 0.133 (0.05-0.30) | 1.000 (0.51-1.00) |
| geography.coordinate.latitude | 39 | 39 | 1 | 0 | 0.975 (0.87-1.00) | 1.000 (0.91-1.00) |
| geography.coordinate.longitude | 45 | 44 | 0 | 1 | 1.000 (0.92-1.00) | 0.978 (0.88-1.00) |
| geography.location.city | 24 | 24 | 12 | 0 | 0.667 (0.50-0.80) | 1.000 (0.86-1.00) |
| geography.location.continent | 1 | 1 | 0 | 0 | 1.000 (0.21-1.00) | 1.000 (0.21-1.00) |
| geography.location.country | 10 | 9 | 2 | 1 | 0.818 (0.52-0.95) | 0.900 (0.60-0.98) |
| geography.location.country_code | 57 | 47 | 2 | 10 | 0.959 (0.86-0.99) | 0.825 (0.71-0.90) |
| geography.location.region | 13 | 7 | 13 | 6 | 0.350 (0.18-0.57) | 0.538 (0.29-0.77) |
| geography.location.state_code | 7 | 0 | 1 | 7 | 0.000 (0.00-0.79) | 0.000 (0.00-0.35) |
| geography.transportation.iata_code | 2 | 2 | 6 | 0 | 0.250 (0.07-0.59) | 1.000 (0.34-1.00) |
| geography.transportation.icao_code | 1 | 1 | 0 | 0 | 1.000 (0.21-1.00) | 1.000 (0.21-1.00) |
| identity.commerce.isbn | 18 | 15 | 7 | 3 | 0.682 (0.47-0.84) | 0.833 (0.61-0.94) |
| identity.person.full_name | 1 | 1 | 5 | 0 | 0.167 (0.03-0.56) | 1.000 (0.21-1.00) |
| representation.boolean.terms | 10 | 8 | 0 | 2 | 1.000 (0.68-1.00) | 0.800 (0.49-0.94) |
| representation.discrete.categorical | 100 | 39 | 6 | 61 | 0.867 (0.74-0.94) | 0.390 (0.30-0.49) |
| representation.identifier.alphanumeric_id | 53 | 6 | 1 | 47 | 0.857 (0.49-0.97) | 0.113 (0.05-0.23) |
| representation.identifier.increment | 1 | 1 | 16 | 0 | 0.059 (0.01-0.27) | 1.000 (0.21-1.00) |
| representation.identifier.numeric_code | 3 | 2 | 1 | 1 | 0.667 (0.21-0.94) | 0.667 (0.21-0.94) |
| representation.identifier.uuid | 1 | 1 | 1 | 0 | 0.500 (0.09-0.91) | 1.000 (0.21-1.00) |
| representation.numeric.decimal_number | 96 | 83 | 0 | 13 | 1.000 (0.96-1.00) | 0.865 (0.78-0.92) |
| representation.numeric.integer_number | 195 | 91 | 0 | 104 | 1.000 (0.96-1.00) | 0.467 (0.40-0.54) |
| representation.text.entity_name | 5 | 4 | 7 | 1 | 0.364 (0.15-0.65) | 0.800 (0.38-0.96) |
| representation.text.plain_text | 36 | 8 | 3 | 28 | 0.727 (0.43-0.90) | 0.222 (0.12-0.38) |
| representation.text.word | 2 | 0 | 13 | 2 | 0.000 (0.00-0.23) | 0.000 (0.00-0.66) |
| technology.internet.hostname | 2 | 2 | 0 | 0 | 1.000 (0.34-1.00) | 1.000 (0.34-1.00) |
| technology.internet.top_level_domain | 6 | 2 | 0 | 4 | 1.000 (0.34-1.00) | 0.333 (0.10-0.70) |
| technology.internet.url | 44 | 44 | 16 | 0 | 0.733 (0.61-0.83) | 1.000 (0.92-1.00) |

**Headline — column accuracy:** 606/915 = 0.662 (95% CI 0.631-0.692)  
**Macro precision** (mean over labels): 0.667  
**Macro recall** (mean over labels): 0.784  
