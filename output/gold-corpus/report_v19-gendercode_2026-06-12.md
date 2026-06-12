# Gold eval anchor — v19-gendercode

**Date:** 2026-06-12  
**Gold fixture:** `eval/gold/gold_corpus_v1.tsv` (931 columns)  
**Predictions:** `output/gold-corpus/predictions_v19_gendercode.tsv`  
**Scored:** 300 columns (631 gold columns had no prediction)  

Per-family accuracy (fraction of columns where the model's prediction equals the curated gold label — labels neither lens produced):

| Family | Columns | Accuracy |
|--------|--------:|---------:|
| A_tight_code_vs_alnum | 30 | 0.933 |
| B_country_vs_categorical | 60 | 0.967 |
| C_lat_lon_temperature | 90 | 1.000 |
| D_year_vs_integer | 60 | 0.917 |
| tier1:datetime.offset.utc | 16 | 0.438 |
| tier1:geography.coordinate.latitude | 8 | 0.750 |
| tier1:geography.coordinate.longitude | 12 | 0.500 |
| tier1:technology.internet.url | 24 | 0.792 |

Per-label precision/recall (the curated label is ground truth; YDF is not consulted):

| Curated label | Support | TP | FP | FN | Precision (95% CI) | Recall (95% CI) |
|---------------|--------:|---:|---:|---:|-------------------:|----------------:|
| datetime.component.year | 30 | 30 | 2 | 0 | 0.938 (0.80-0.98) | 1.000 (0.89-1.00) |
| datetime.date.iso | 1 | 0 | 0 | 1 | n/a (n/a) | 0.000 (0.00-0.79) |
| geography.coordinate.latitude | 34 | 34 | 0 | 0 | 1.000 (0.90-1.00) | 1.000 (0.90-1.00) |
| geography.coordinate.longitude | 32 | 32 | 0 | 0 | 1.000 (0.89-1.00) | 1.000 (0.89-1.00) |
| geography.location.country_code | 30 | 29 | 0 | 1 | 1.000 (0.88-1.00) | 0.967 (0.83-0.99) |
| representation.discrete.categorical | 30 | 29 | 1 | 1 | 0.967 (0.83-0.99) | 0.967 (0.83-0.99) |
| representation.identifier.alphanumeric_id | 30 | 28 | 0 | 2 | 1.000 (0.88-1.00) | 0.933 (0.79-0.98) |
| representation.numeric.decimal_number | 32 | 32 | 0 | 0 | 1.000 (0.89-1.00) | 1.000 (0.89-1.00) |
| representation.numeric.integer_number | 61 | 36 | 0 | 25 | 1.000 (0.90-1.00) | 0.590 (0.46-0.70) |
| representation.text.plain_text | 1 | 0 | 0 | 1 | n/a (n/a) | 0.000 (0.00-0.79) |
| technology.internet.url | 19 | 19 | 5 | 0 | 0.792 (0.60-0.91) | 1.000 (0.83-1.00) |

**Headline — column accuracy:** 269/300 = 0.897 (95% CI 0.857-0.926)  
**Macro precision** (mean over labels): 0.966  
**Macro recall** (mean over labels): 0.769  
