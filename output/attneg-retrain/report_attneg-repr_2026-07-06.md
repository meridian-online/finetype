# Gold eval anchor — attneg-repr

**Date:** 2026-07-06  
**Gold fixture:** `eval/repr/representative_corpus.tsv` (260 columns)  
**Scoring mode:** ENUM REFRAME (categorical/word/plain_text = one text residual)  
**Predictions:** `output/attneg-retrain/predictions_attneg_repr.tsv`  
**Scored:** 260 columns (0 gold columns had no prediction)  

Per-family accuracy (fraction of columns where the model's prediction equals the curated gold label — labels neither lens produced):

| Family | Columns | Accuracy |
|--------|--------:|---------:|
| representative | 260 | 0.746 |

Per-label precision/recall (the curated label is ground truth; YDF is not consulted):

| Curated label | Support | TP | FP | FN | Precision (95% CI) | Recall (95% CI) |
|---------------|--------:|---:|---:|---:|-------------------:|----------------:|
| datetime.date.iso | 1 | 1 | 0 | 0 | 1.000 (0.21-1.00) | 1.000 (0.21-1.00) |
| datetime.date.month_year_full | 1 | 1 | 0 | 0 | 1.000 (0.21-1.00) | 1.000 (0.21-1.00) |
| datetime.epoch.unix_seconds | 5 | 5 | 0 | 0 | 1.000 (0.57-1.00) | 1.000 (0.57-1.00) |
| datetime.offset.iana | 1 | 0 | 0 | 1 | n/a (n/a) | 0.000 (0.00-0.79) |
| datetime.time.hms_24h | 1 | 0 | 0 | 1 | n/a (n/a) | 0.000 (0.00-0.79) |
| datetime.timestamp.iso_8601 | 1 | 1 | 0 | 0 | 1.000 (0.21-1.00) | 1.000 (0.21-1.00) |
| datetime.timestamp.iso_8601_milliseconds | 2 | 0 | 0 | 2 | n/a (n/a) | 0.000 (0.00-0.66) |
| finance.currency.amount | 2 | 1 | 0 | 1 | 1.000 (0.21-1.00) | 0.500 (0.09-0.91) |
| geography.address.postal_code | 1 | 1 | 0 | 0 | 1.000 (0.21-1.00) | 1.000 (0.21-1.00) |
| geography.coordinate.latitude | 2 | 2 | 0 | 0 | 1.000 (0.34-1.00) | 1.000 (0.34-1.00) |
| geography.coordinate.longitude | 1 | 1 | 0 | 0 | 1.000 (0.21-1.00) | 1.000 (0.21-1.00) |
| geography.location.city | 3 | 3 | 0 | 0 | 1.000 (0.44-1.00) | 1.000 (0.44-1.00) |
| geography.location.country | 1 | 1 | 0 | 0 | 1.000 (0.21-1.00) | 1.000 (0.21-1.00) |
| geography.location.country_code | 1 | 1 | 0 | 0 | 1.000 (0.21-1.00) | 1.000 (0.21-1.00) |
| identity.person.email | 2 | 1 | 0 | 1 | 1.000 (0.21-1.00) | 0.500 (0.09-0.91) |
| identity.person.first_name | 1 | 0 | 0 | 1 | n/a (n/a) | 0.000 (0.00-0.79) |
| identity.person.full_name | 4 | 4 | 6 | 0 | 0.400 (0.17-0.69) | 1.000 (0.51-1.00) |
| identity.person.username | 12 | 7 | 0 | 5 | 1.000 (0.65-1.00) | 0.583 (0.32-0.81) |
| representation.boolean.binary | 8 | 4 | 1 | 4 | 0.800 (0.38-0.96) | 0.500 (0.22-0.78) |
| representation.boolean.initials | 4 | 4 | 0 | 0 | 1.000 (0.51-1.00) | 1.000 (0.51-1.00) |
| representation.boolean.terms | 6 | 6 | 0 | 0 | 1.000 (0.61-1.00) | 1.000 (0.61-1.00) |
| representation.discrete.ordinal | 1 | 0 | 0 | 1 | n/a (n/a) | 0.000 (0.00-0.79) |
| representation.format.color_hex | 1 | 0 | 0 | 1 | n/a (n/a) | 0.000 (0.00-0.79) |
| representation.identifier.alphanumeric_id | 6 | 2 | 0 | 4 | 1.000 (0.34-1.00) | 0.333 (0.10-0.70) |
| representation.identifier.increment | 9 | 8 | 1 | 1 | 0.889 (0.56-0.98) | 0.889 (0.56-0.98) |
| representation.identifier.numeric_code | 15 | 0 | 0 | 15 | n/a (n/a) | 0.000 (0.00-0.20) |
| representation.identifier.uuid | 2 | 1 | 0 | 1 | 1.000 (0.21-1.00) | 0.500 (0.09-0.91) |
| representation.numeric.decimal_number | 6 | 4 | 2 | 2 | 0.667 (0.30-0.90) | 0.667 (0.30-0.90) |
| representation.numeric.integer_number | 69 | 67 | 16 | 2 | 0.807 (0.71-0.88) | 0.971 (0.90-0.99) |
| representation.numeric.percentage | 1 | 1 | 0 | 0 | 1.000 (0.21-1.00) | 1.000 (0.21-1.00) |
| representation.scientific.dna_sequence | 1 | 0 | 0 | 1 | n/a (n/a) | 0.000 (0.00-0.79) |
| representation.text.RESIDUAL | 77 | 57 | 6 | 20 | 0.905 (0.81-0.96) | 0.740 (0.63-0.83) |
| representation.text.entity_name | 2 | 2 | 4 | 0 | 0.333 (0.10-0.70) | 1.000 (0.34-1.00) |
| technology.code.doi | 1 | 0 | 0 | 1 | n/a (n/a) | 0.000 (0.00-0.79) |
| technology.code.locale_code | 1 | 0 | 0 | 1 | n/a (n/a) | 0.000 (0.00-0.79) |
| technology.cryptographic.hash | 2 | 2 | 0 | 0 | 1.000 (0.34-1.00) | 1.000 (0.34-1.00) |
| technology.internet.ip_v4 | 1 | 1 | 0 | 0 | 1.000 (0.21-1.00) | 1.000 (0.21-1.00) |
| technology.internet.url | 5 | 5 | 2 | 0 | 0.714 (0.36-0.92) | 1.000 (0.57-1.00) |

**Headline — column accuracy:** 194/260 = 0.746 (95% CI 0.690-0.795)  

**Macro precision** (mean over labels): 0.911  
**Macro recall** (mean over labels): 0.636  
