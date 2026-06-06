# Gold eval anchor — latdec-s43

**Date:** 2026-06-06  
**Gold fixture:** `eval/gold/gold_eval_anchor.tsv` (240 columns)  
**Predictions:** `output/gold-eval-anchor/predictions_latdec-s43.tsv`  
**Scored:** 240 columns (0 gold columns had no prediction)  

Per-family accuracy (fraction of columns where the model's prediction equals the curated gold label — labels neither lens produced):

| Family | Columns | Accuracy |
|--------|--------:|---------:|
| A_tight_code_vs_alnum | 30 | 0.200 |
| B_country_vs_categorical | 60 | 0.933 |
| C_lat_lon_temperature | 90 | 1.000 |
| D_year_vs_integer | 60 | 0.667 |

Per-label precision/recall (the curated label is ground truth; YDF is not consulted):

| Family | Curated label | Support | TP | FP | FN | Precision | Recall |
|--------|---------------|--------:|---:|---:|---:|----------:|-------:|
| A_tight_code_vs_alnum | representation.identifier.alphanumeric_id | 30 | 6 | 0 | 24 | 1.000 | 0.200 |
| B_country_vs_categorical | geography.location.country_code | 30 | 27 | 0 | 3 | 1.000 | 0.900 |
| B_country_vs_categorical | representation.discrete.categorical | 30 | 29 | 0 | 1 | 1.000 | 0.967 |
| C_lat_lon_temperature | geography.coordinate.latitude | 30 | 30 | 0 | 0 | 1.000 | 1.000 |
| C_lat_lon_temperature | geography.coordinate.longitude | 30 | 30 | 0 | 0 | 1.000 | 1.000 |
| C_lat_lon_temperature | representation.numeric.decimal_number | 30 | 30 | 0 | 0 | 1.000 | 1.000 |
| D_year_vs_integer | datetime.component.year | 30 | 30 | 2 | 0 | 0.938 | 1.000 |
| D_year_vs_integer | representation.numeric.integer_number | 30 | 10 | 0 | 20 | 1.000 | 0.333 |

**Macro precision** (mean over labels): 0.992  
**Macro recall** (mean over labels): 0.800  
