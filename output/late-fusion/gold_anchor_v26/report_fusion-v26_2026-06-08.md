# Gold eval anchor — fusion-v26

**Date:** 2026-06-08  
**Gold fixture:** `eval/gold/gold_eval_anchor.tsv` (240 columns)  
**Predictions:** `output/late-fusion/gold_anchor_v26/predictions.tsv`  
**Scored:** 240 columns (0 gold columns had no prediction)  

Per-family accuracy (fraction of columns where the model's prediction equals the curated gold label — labels neither lens produced):

| Family | Columns | Accuracy |
|--------|--------:|---------:|
| A_tight_code_vs_alnum | 30 | 0.967 |
| B_country_vs_categorical | 60 | 0.500 |
| C_lat_lon_temperature | 90 | 0.344 |
| D_year_vs_integer | 60 | 0.650 |

Per-label precision/recall (the curated label is ground truth; YDF is not consulted):

| Family | Curated label | Support | TP | FP | FN | Precision | Recall |
|--------|---------------|--------:|---:|---:|---:|----------:|-------:|
| A_tight_code_vs_alnum | representation.identifier.alphanumeric_id | 30 | 29 | 0 | 1 | 1.000 | 0.967 |
| B_country_vs_categorical | geography.location.country_code | 30 | 30 | 30 | 0 | 0.500 | 1.000 |
| B_country_vs_categorical | representation.discrete.categorical | 30 | 0 | 0 | 30 | n/a | 0.000 |
| C_lat_lon_temperature | geography.coordinate.latitude | 30 | 0 | 0 | 30 | n/a | 0.000 |
| C_lat_lon_temperature | geography.coordinate.longitude | 30 | 1 | 0 | 29 | 1.000 | 0.033 |
| C_lat_lon_temperature | representation.numeric.decimal_number | 30 | 30 | 59 | 0 | 0.337 | 1.000 |
| D_year_vs_integer | datetime.component.year | 30 | 30 | 8 | 0 | 0.789 | 1.000 |
| D_year_vs_integer | representation.numeric.integer_number | 30 | 9 | 0 | 21 | 1.000 | 0.300 |

**Macro precision** (mean over labels): 0.771  
**Macro recall** (mean over labels): 0.537  
