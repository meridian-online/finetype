# Gold eval anchor — v19-gold-corpus-v1

**Date:** 2026-06-10  
**Gold fixture:** `eval/gold/gold_corpus_v1.tsv` (589 columns)  
**Predictions:** `output/gold-corpus/predictions_v19.tsv`  
**Scored:** 589 columns (0 gold columns had no prediction)  

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
| datetime.component.year | 36 | 36 | 2 | 0 | 0.947 (0.83-0.99) | 1.000 (0.90-1.00) |
| datetime.date.iso | 67 | 52 | 0 | 15 | 1.000 (0.93-1.00) | 0.776 (0.66-0.86) |
| datetime.epoch.unix_seconds | 11 | 10 | 0 | 1 | 1.000 (0.72-1.00) | 0.909 (0.62-0.98) |
| datetime.offset.utc | 1 | 1 | 0 | 0 | 1.000 (0.21-1.00) | 1.000 (0.21-1.00) |
| geography.address.postal_code | 2 | 2 | 25 | 0 | 0.074 (0.02-0.23) | 1.000 (0.34-1.00) |
| geography.coordinate.latitude | 39 | 39 | 0 | 0 | 1.000 (0.91-1.00) | 1.000 (0.91-1.00) |
| geography.coordinate.longitude | 35 | 35 | 0 | 0 | 1.000 (0.90-1.00) | 1.000 (0.90-1.00) |
| geography.location.city | 1 | 1 | 4 | 0 | 0.200 (0.04-0.62) | 1.000 (0.21-1.00) |
| geography.location.country_code | 42 | 34 | 1 | 8 | 0.971 (0.85-0.99) | 0.810 (0.67-0.90) |
| identity.commerce.isbn | 17 | 14 | 6 | 3 | 0.700 (0.48-0.85) | 0.824 (0.59-0.94) |
| representation.discrete.categorical | 39 | 31 | 6 | 8 | 0.838 (0.69-0.92) | 0.795 (0.64-0.89) |
| representation.identifier.alphanumeric_id | 39 | 5 | 0 | 34 | 1.000 (0.57-1.00) | 0.128 (0.06-0.27) |
| representation.numeric.decimal_number | 56 | 53 | 0 | 3 | 1.000 (0.93-1.00) | 0.946 (0.85-0.98) |
| representation.numeric.integer_number | 146 | 57 | 0 | 89 | 1.000 (0.94-1.00) | 0.390 (0.32-0.47) |
| representation.text.plain_text | 25 | 5 | 0 | 20 | 1.000 (0.57-1.00) | 0.200 (0.09-0.39) |
| technology.internet.top_level_domain | 6 | 2 | 0 | 4 | 1.000 (0.34-1.00) | 0.333 (0.10-0.70) |
| technology.internet.url | 27 | 27 | 5 | 0 | 0.844 (0.68-0.93) | 1.000 (0.88-1.00) |

**Headline — column accuracy:** 404/589 = 0.686 (95% CI 0.647-0.722)  
**Macro precision** (mean over labels): 0.857  
**Macro recall** (mean over labels): 0.771  
