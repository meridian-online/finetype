# Gold eval anchor — tierA-batch1-reframe

**Date:** 2026-06-27  
**Gold fixture:** `eval/gold/gold_corpus.tsv` (931 columns)  
**Scoring mode:** ENUM REFRAME (categorical/word/plain_text = one text residual)  
**Predictions:** `output/sharpen-audit/tierA/pred_batch1.tsv`  
**Scored:** 927 columns (4 gold columns had no prediction)  

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
| external:datetime.component.year | 2 | 1.000 |
| external:datetime.date.iso | 9 | 0.333 |
| external:datetime.offset.utc | 1 | 1.000 |
| external:geography.address.postal_code | 1 | 0.000 |
| external:geography.coordinate.latitude | 6 | 1.000 |
| external:geography.coordinate.longitude | 4 | 1.000 |
| external:geography.location.city | 1 | 1.000 |
| external:representation.discrete.categorical | 9 | 0.556 |
| external:representation.identifier.alphanumeric_id | 4 | 0.500 |
| external:representation.numeric.decimal_number | 3 | 1.000 |
| external:representation.numeric.integer_number | 7 | 0.857 |
| external:representation.text.plain_text | 5 | 0.000 |
| external:technology.internet.top_level_domain | 2 | 0.000 |
| external:technology.internet.url | 2 | 1.000 |
| llm:datetime.component.year | 9 | 0.778 |
| llm:datetime.date.iso | 4 | 1.000 |
| llm:datetime.epoch.unix_seconds | 4 | 0.750 |
| llm:datetime.offset.utc | 28 | 0.750 |
| llm:geography.address.postal_code | 2 | 1.000 |
| llm:geography.coordinate.latitude | 29 | 0.862 |
| llm:geography.coordinate.longitude | 32 | 0.781 |
| llm:geography.location.city | 30 | 0.900 |
| llm:geography.location.country_code | 31 | 0.839 |
| llm:geography.location.region | 35 | 0.714 |
| llm:identity.commerce.isbn | 1 | 1.000 |
| llm:representation.discrete.categorical | 43 | 0.581 |
| llm:representation.identifier.alphanumeric_id | 37 | 0.649 |
| llm:representation.numeric.decimal_number | 1 | 0.000 |
| llm:representation.numeric.integer_number | 3 | 1.000 |
| llm:representation.text.plain_text | 3 | 0.333 |
| llm:technology.internet.url | 30 | 0.933 |
| tier1:datetime.offset.utc | 35 | 0.829 |
| tier1:geography.coordinate.latitude | 8 | 0.875 |
| tier1:geography.coordinate.longitude | 12 | 0.833 |
| tier1:geography.location.city | 3 | 0.333 |
| tier1:geography.location.country_code | 13 | 0.769 |
| tier1:geography.location.region | 2 | 0.500 |
| tier1:representation.discrete.categorical | 2 | 0.500 |
| tier1:representation.identifier.alphanumeric_id | 4 | 0.750 |
| tier1:technology.internet.url | 24 | 1.000 |
| tier2:datetime.component.year | 12 | 1.000 |
| tier2:datetime.epoch.unix_seconds | 20 | 0.700 |
| tier2:finance.currency.amount | 18 | 1.000 |
| tier2:geography.address.postal_code | 10 | 0.900 |
| tier2:identity.commerce.isbn | 29 | 0.793 |
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
| datetime.epoch.unix_milliseconds | 4 | 1 | 0 | 3 | 1.000 (0.21-1.00) | 0.250 (0.05-0.70) |
| datetime.epoch.unix_seconds | 15 | 8 | 0 | 7 | 1.000 (0.68-1.00) | 0.533 (0.30-0.75) |
| datetime.offset.iana | 3 | 3 | 0 | 0 | 1.000 (0.44-1.00) | 1.000 (0.44-1.00) |
| datetime.offset.timezone_abbreviation | 6 | 6 | 0 | 0 | 1.000 (0.61-1.00) | 1.000 (0.61-1.00) |
| datetime.offset.utc | 1 | 1 | 4 | 0 | 0.200 (0.04-0.62) | 1.000 (0.21-1.00) |
| datetime.timestamp.dmy_hm | 1 | 1 | 0 | 0 | 1.000 (0.21-1.00) | 1.000 (0.21-1.00) |
| datetime.timestamp.iso_8601_milliseconds | 4 | 3 | 0 | 1 | 1.000 (0.44-1.00) | 0.750 (0.30-0.95) |
| datetime.timestamp.sql_standard | 14 | 9 | 0 | 5 | 1.000 (0.70-1.00) | 0.643 (0.39-0.84) |
| finance.currency.amount | 5 | 0 | 0 | 5 | n/a (n/a) | 0.000 (0.00-0.43) |
| geography.address.postal_code | 5 | 4 | 0 | 1 | 1.000 (0.51-1.00) | 0.800 (0.38-0.96) |
| geography.coordinate.latitude | 39 | 39 | 1 | 0 | 0.975 (0.87-1.00) | 1.000 (0.91-1.00) |
| geography.coordinate.longitude | 45 | 44 | 0 | 1 | 1.000 (0.92-1.00) | 0.978 (0.88-1.00) |
| geography.location.city | 24 | 24 | 4 | 0 | 0.857 (0.69-0.94) | 1.000 (0.86-1.00) |
| geography.location.continent | 1 | 1 | 0 | 0 | 1.000 (0.21-1.00) | 1.000 (0.21-1.00) |
| geography.location.country | 11 | 9 | 0 | 2 | 1.000 (0.70-1.00) | 0.818 (0.52-0.95) |
| geography.location.country_code | 54 | 52 | 2 | 2 | 0.963 (0.87-0.99) | 0.963 (0.87-0.99) |
| geography.location.region | 15 | 10 | 7 | 5 | 0.588 (0.36-0.78) | 0.667 (0.42-0.85) |
| geography.location.state_code | 7 | 6 | 2 | 1 | 0.750 (0.41-0.93) | 0.857 (0.49-0.97) |
| geography.transportation.iata_code | 2 | 2 | 3 | 0 | 0.400 (0.12-0.77) | 1.000 (0.34-1.00) |
| geography.transportation.icao_code | 1 | 1 | 0 | 0 | 1.000 (0.21-1.00) | 1.000 (0.21-1.00) |
| identity.commerce.isbn | 18 | 18 | 0 | 0 | 1.000 (0.82-1.00) | 1.000 (0.82-1.00) |
| identity.person.full_name | 1 | 1 | 1 | 0 | 0.500 (0.09-0.91) | 1.000 (0.21-1.00) |
| identity.person.gender | 1 | 1 | 0 | 0 | 1.000 (0.21-1.00) | 1.000 (0.21-1.00) |
| identity.person.username | 1 | 0 | 0 | 1 | n/a (n/a) | 0.000 (0.00-0.79) |
| representation.boolean.terms | 10 | 10 | 1 | 0 | 0.909 (0.62-0.98) | 1.000 (0.72-1.00) |
| representation.file.extension | 1 | 0 | 0 | 1 | n/a (n/a) | 0.000 (0.00-0.79) |
| representation.identifier.alphanumeric_id | 62 | 41 | 2 | 21 | 0.953 (0.85-0.99) | 0.661 (0.54-0.77) |
| representation.identifier.increment | 1 | 1 | 8 | 0 | 0.111 (0.02-0.44) | 1.000 (0.21-1.00) |
| representation.identifier.numeric_code | 3 | 3 | 0 | 0 | 1.000 (0.44-1.00) | 1.000 (0.44-1.00) |
| representation.identifier.uuid | 2 | 2 | 0 | 0 | 1.000 (0.34-1.00) | 1.000 (0.34-1.00) |
| representation.numeric.decimal_number | 94 | 84 | 7 | 10 | 0.923 (0.85-0.96) | 0.894 (0.82-0.94) |
| representation.numeric.integer_number | 193 | 167 | 2 | 26 | 0.988 (0.96-1.00) | 0.865 (0.81-0.91) |
| representation.text.RESIDUAL | 123 | 77 | 10 | 46 | 0.885 (0.80-0.94) | 0.626 (0.54-0.71) |
| representation.text.entity_name | 8 | 6 | 25 | 2 | 0.194 (0.09-0.36) | 0.750 (0.41-0.93) |
| technology.filesystem.windows_path | 1 | 1 | 0 | 0 | 1.000 (0.21-1.00) | 1.000 (0.21-1.00) |
| technology.internet.hostname | 2 | 2 | 0 | 0 | 1.000 (0.34-1.00) | 1.000 (0.34-1.00) |
| technology.internet.top_level_domain | 6 | 2 | 1 | 4 | 0.667 (0.21-0.94) | 0.333 (0.10-0.70) |
| technology.internet.url | 44 | 44 | 1 | 0 | 0.978 (0.88-1.00) | 1.000 (0.92-1.00) |

**Headline — column accuracy:** 780/927 = 0.841 (95% CI 0.817-0.864)  
**Macro precision** (mean over labels): 0.866  
**Macro recall** (mean over labels): 0.799  
