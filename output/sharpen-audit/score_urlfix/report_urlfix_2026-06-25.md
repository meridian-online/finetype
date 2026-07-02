# Gold eval anchor — urlfix

**Date:** 2026-06-25  
**Gold fixture:** `eval/gold/gold_corpus.tsv` (931 columns)  
**Scoring mode:** legacy (categorical is a competing label)  
**Predictions:** `output/sharpen-audit/attn_urlfix.tsv`  
**Scored:** 931 columns (0 gold columns had no prediction)  

Per-family accuracy (fraction of columns where the model's prediction equals the curated gold label — labels neither lens produced):

| Family | Columns | Accuracy |
|--------|--------:|---------:|
| A_tight_code_vs_alnum | 30 | 1.000 |
| B_country_vs_categorical | 60 | 1.000 |
| C_lat_lon_temperature | 90 | 1.000 |
| D_year_vs_integer | 60 | 0.933 |
| author-open:datetime.component.year | 1 | 0.000 |
| author-open:finance.currency.amount | 5 | 0.000 |
| author-open:geography.location.region | 2 | 1.000 |
| author-open:representation.discrete.categorical | 4 | 0.750 |
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
| external:geography.address.postal_code | 1 | 1.000 |
| external:geography.coordinate.latitude | 6 | 0.833 |
| external:geography.coordinate.longitude | 4 | 0.750 |
| external:geography.location.city | 1 | 1.000 |
| external:representation.discrete.categorical | 9 | 0.111 |
| external:representation.identifier.alphanumeric_id | 4 | 0.750 |
| external:representation.numeric.decimal_number | 3 | 1.000 |
| external:representation.numeric.integer_number | 7 | 0.857 |
| external:representation.text.plain_text | 5 | 0.200 |
| external:technology.internet.top_level_domain | 2 | 0.000 |
| external:technology.internet.url | 2 | 0.500 |
| llm:datetime.component.year | 9 | 0.889 |
| llm:datetime.date.iso | 4 | 1.000 |
| llm:datetime.epoch.unix_seconds | 4 | 1.000 |
| llm:datetime.offset.utc | 28 | 0.536 |
| llm:geography.address.postal_code | 2 | 1.000 |
| llm:geography.coordinate.latitude | 30 | 0.867 |
| llm:geography.coordinate.longitude | 32 | 0.906 |
| llm:geography.location.city | 33 | 0.879 |
| llm:geography.location.country_code | 31 | 0.871 |
| llm:geography.location.region | 35 | 0.771 |
| llm:identity.commerce.isbn | 1 | 1.000 |
| llm:representation.discrete.categorical | 43 | 0.465 |
| llm:representation.identifier.alphanumeric_id | 37 | 0.622 |
| llm:representation.numeric.decimal_number | 1 | 0.000 |
| llm:representation.numeric.integer_number | 3 | 1.000 |
| llm:representation.text.plain_text | 3 | 0.333 |
| llm:technology.internet.url | 30 | 0.600 |
| tier1:datetime.offset.utc | 35 | 0.829 |
| tier1:geography.coordinate.latitude | 8 | 0.875 |
| tier1:geography.coordinate.longitude | 12 | 0.750 |
| tier1:geography.location.city | 3 | 0.000 |
| tier1:geography.location.country_code | 13 | 0.769 |
| tier1:geography.location.region | 2 | 0.000 |
| tier1:representation.discrete.categorical | 2 | 0.000 |
| tier1:representation.identifier.alphanumeric_id | 4 | 0.750 |
| tier1:technology.internet.url | 24 | 0.833 |
| tier2:datetime.component.year | 12 | 0.917 |
| tier2:datetime.epoch.unix_seconds | 20 | 0.750 |
| tier2:finance.currency.amount | 18 | 0.944 |
| tier2:geography.address.postal_code | 10 | 1.000 |
| tier2:identity.commerce.isbn | 29 | 0.655 |
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
| datetime.epoch.unix_milliseconds | 4 | 2 | 0 | 2 | 1.000 (0.34-1.00) | 0.500 (0.15-0.85) |
| datetime.epoch.unix_seconds | 15 | 10 | 5 | 5 | 0.667 (0.42-0.85) | 0.667 (0.42-0.85) |
| datetime.offset.iana | 4 | 4 | 0 | 0 | 1.000 (0.51-1.00) | 1.000 (0.51-1.00) |
| datetime.offset.utc | 1 | 1 | 4 | 0 | 0.200 (0.04-0.62) | 1.000 (0.21-1.00) |
| datetime.timestamp.dmy_hm | 1 | 0 | 0 | 1 | n/a (n/a) | 0.000 (0.00-0.79) |
| datetime.timestamp.iso_8601_milliseconds | 4 | 3 | 0 | 1 | 1.000 (0.44-1.00) | 0.750 (0.30-0.95) |
| datetime.timestamp.sql_standard | 14 | 9 | 0 | 5 | 1.000 (0.70-1.00) | 0.643 (0.39-0.84) |
| finance.currency.amount | 5 | 0 | 0 | 5 | n/a (n/a) | 0.000 (0.00-0.43) |
| geography.address.full_address | 4 | 4 | 1 | 0 | 0.800 (0.38-0.96) | 1.000 (0.51-1.00) |
| geography.address.postal_code | 4 | 4 | 1 | 0 | 0.800 (0.38-0.96) | 1.000 (0.51-1.00) |
| geography.coordinate.latitude | 39 | 39 | 1 | 0 | 0.975 (0.87-1.00) | 1.000 (0.91-1.00) |
| geography.coordinate.longitude | 45 | 44 | 0 | 1 | 1.000 (0.92-1.00) | 0.978 (0.88-1.00) |
| geography.location.city | 24 | 23 | 5 | 1 | 0.821 (0.64-0.92) | 0.958 (0.80-0.99) |
| geography.location.continent | 1 | 1 | 0 | 0 | 1.000 (0.21-1.00) | 1.000 (0.21-1.00) |
| geography.location.country | 11 | 10 | 0 | 1 | 1.000 (0.72-1.00) | 0.909 (0.62-0.98) |
| geography.location.country_code | 54 | 53 | 8 | 1 | 0.869 (0.76-0.93) | 0.981 (0.90-1.00) |
| geography.location.region | 15 | 14 | 3 | 1 | 0.824 (0.59-0.94) | 0.933 (0.70-0.99) |
| geography.location.state_code | 7 | 6 | 2 | 1 | 0.750 (0.41-0.93) | 0.857 (0.49-0.97) |
| geography.transportation.iata_code | 2 | 2 | 4 | 0 | 0.333 (0.10-0.70) | 1.000 (0.34-1.00) |
| geography.transportation.icao_code | 1 | 1 | 0 | 0 | 1.000 (0.21-1.00) | 1.000 (0.21-1.00) |
| identity.commerce.isbn | 18 | 15 | 1 | 3 | 0.938 (0.72-0.99) | 0.833 (0.61-0.94) |
| identity.person.full_name | 1 | 1 | 0 | 0 | 1.000 (0.21-1.00) | 1.000 (0.21-1.00) |
| identity.person.gender | 1 | 1 | 0 | 0 | 1.000 (0.21-1.00) | 1.000 (0.21-1.00) |
| identity.person.username | 1 | 0 | 0 | 1 | n/a (n/a) | 0.000 (0.00-0.79) |
| representation.boolean.terms | 10 | 10 | 1 | 0 | 0.909 (0.62-0.98) | 1.000 (0.72-1.00) |
| representation.file.extension | 1 | 0 | 0 | 1 | n/a (n/a) | 0.000 (0.00-0.79) |
| representation.identifier.alphanumeric_id | 62 | 42 | 1 | 20 | 0.977 (0.88-1.00) | 0.677 (0.55-0.78) |
| representation.identifier.increment | 1 | 1 | 8 | 0 | 0.111 (0.02-0.44) | 1.000 (0.21-1.00) |
| representation.identifier.numeric_code | 3 | 3 | 1 | 0 | 0.750 (0.30-0.95) | 1.000 (0.44-1.00) |
| representation.identifier.uuid | 2 | 2 | 0 | 0 | 1.000 (0.34-1.00) | 1.000 (0.34-1.00) |
| representation.numeric.decimal_number | 94 | 88 | 7 | 6 | 0.926 (0.86-0.96) | 0.936 (0.87-0.97) |
| representation.numeric.integer_number | 194 | 158 | 4 | 36 | 0.975 (0.94-0.99) | 0.814 (0.75-0.86) |
| representation.text.entity_name | 8 | 6 | 23 | 2 | 0.207 (0.10-0.38) | 0.750 (0.41-0.93) |
| representation.text.plain_text | 42 | 8 | 12 | 34 | 0.400 (0.22-0.61) | 0.190 (0.10-0.33) |
| representation.text.word | 87 | 52 | 16 | 35 | 0.765 (0.65-0.85) | 0.598 (0.49-0.69) |
| technology.internet.hostname | 2 | 2 | 5 | 0 | 0.286 (0.08-0.64) | 1.000 (0.34-1.00) |
| technology.internet.top_level_domain | 6 | 0 | 0 | 6 | n/a (n/a) | 0.000 (0.00-0.39) |
| technology.internet.url | 44 | 35 | 1 | 9 | 0.972 (0.86-1.00) | 0.795 (0.65-0.89) |

**Headline — column accuracy:** 751/931 = 0.807 (95% CI 0.780-0.831)  
**Macro precision** (mean over labels): 0.816  
**Macro recall** (mean over labels): 0.762  
