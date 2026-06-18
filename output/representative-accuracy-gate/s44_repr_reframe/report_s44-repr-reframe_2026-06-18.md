# Gold eval anchor — s44-repr-reframe

**Date:** 2026-06-18  
**Gold fixture:** `eval/repr/representative_corpus.tsv` (260 columns)  
**Scoring mode:** ENUM REFRAME (categorical/word/plain_text = one text residual)  
**Predictions:** `output/representative-accuracy-gate/predictions_s44_repr.tsv`  
**Scored:** 259 columns (1 gold columns had no prediction)  

Per-family accuracy (fraction of columns where the model's prediction equals the curated gold label — labels neither lens produced):

| Family | Columns | Accuracy |
|--------|--------:|---------:|
| representative | 259 | 0.683 |

Per-label precision/recall (the curated label is ground truth; YDF is not consulted):

| Curated label | Support | TP | FP | FN | Precision (95% CI) | Recall (95% CI) |
|---------------|--------:|---:|---:|---:|-------------------:|----------------:|
| datetime.date.iso | 1 | 1 | 0 | 0 | 1.000 (0.21-1.00) | 1.000 (0.21-1.00) |
| datetime.date.month_year_full | 1 | 1 | 0 | 0 | 1.000 (0.21-1.00) | 1.000 (0.21-1.00) |
| datetime.epoch.unix_seconds | 5 | 4 | 3 | 1 | 0.571 (0.25-0.84) | 0.800 (0.38-0.96) |
| datetime.offset.iana | 1 | 0 | 0 | 1 | n/a (n/a) | 0.000 (0.00-0.79) |
| datetime.time.hms_24h | 1 | 0 | 0 | 1 | n/a (n/a) | 0.000 (0.00-0.79) |
| datetime.timestamp.iso_8601 | 1 | 1 | 0 | 0 | 1.000 (0.21-1.00) | 1.000 (0.21-1.00) |
| datetime.timestamp.iso_8601_milliseconds | 2 | 0 | 0 | 2 | n/a (n/a) | 0.000 (0.00-0.66) |
| finance.currency.amount | 2 | 0 | 0 | 2 | n/a (n/a) | 0.000 (0.00-0.66) |
| geography.address.postal_code | 1 | 1 | 0 | 0 | 1.000 (0.21-1.00) | 1.000 (0.21-1.00) |
| geography.coordinate.latitude | 2 | 2 | 0 | 0 | 1.000 (0.34-1.00) | 1.000 (0.34-1.00) |
| geography.coordinate.longitude | 1 | 1 | 0 | 0 | 1.000 (0.21-1.00) | 1.000 (0.21-1.00) |
| geography.location.city | 3 | 3 | 0 | 0 | 1.000 (0.44-1.00) | 1.000 (0.44-1.00) |
| geography.location.country | 1 | 1 | 2 | 0 | 0.333 (0.06-0.79) | 1.000 (0.21-1.00) |
| geography.location.country_code | 1 | 1 | 1 | 0 | 0.500 (0.09-0.91) | 1.000 (0.21-1.00) |
| identity.person.email | 2 | 1 | 0 | 1 | 1.000 (0.21-1.00) | 0.500 (0.09-0.91) |
| identity.person.first_name | 1 | 0 | 0 | 1 | n/a (n/a) | 0.000 (0.00-0.79) |
| identity.person.full_name | 4 | 4 | 9 | 0 | 0.308 (0.13-0.58) | 1.000 (0.51-1.00) |
| identity.person.username | 12 | 7 | 1 | 5 | 0.875 (0.53-0.98) | 0.583 (0.32-0.81) |
| representation.boolean.binary | 8 | 4 | 4 | 4 | 0.500 (0.22-0.78) | 0.500 (0.22-0.78) |
| representation.boolean.initials | 4 | 4 | 0 | 0 | 1.000 (0.51-1.00) | 1.000 (0.51-1.00) |
| representation.boolean.terms | 6 | 6 | 0 | 0 | 1.000 (0.61-1.00) | 1.000 (0.61-1.00) |
| representation.discrete.ordinal | 1 | 0 | 3 | 1 | 0.000 (0.00-0.56) | 0.000 (0.00-0.79) |
| representation.format.color_hex | 1 | 0 | 0 | 1 | n/a (n/a) | 0.000 (0.00-0.79) |
| representation.identifier.alphanumeric_id | 6 | 1 | 0 | 5 | 1.000 (0.21-1.00) | 0.167 (0.03-0.56) |
| representation.identifier.increment | 9 | 8 | 2 | 1 | 0.800 (0.49-0.94) | 0.889 (0.56-0.98) |
| representation.identifier.numeric_code | 15 | 0 | 0 | 15 | n/a (n/a) | 0.000 (0.00-0.20) |
| representation.identifier.uuid | 2 | 1 | 0 | 1 | 1.000 (0.21-1.00) | 0.500 (0.09-0.91) |
| representation.numeric.decimal_number | 6 | 3 | 3 | 3 | 0.500 (0.19-0.81) | 0.500 (0.19-0.81) |
| representation.numeric.integer_number | 69 | 64 | 13 | 5 | 0.831 (0.73-0.90) | 0.928 (0.84-0.97) |
| representation.numeric.percentage | 1 | 1 | 0 | 0 | 1.000 (0.21-1.00) | 1.000 (0.21-1.00) |
| representation.scientific.dna_sequence | 1 | 0 | 0 | 1 | n/a (n/a) | 0.000 (0.00-0.79) |
| representation.text.RESIDUAL | 76 | 47 | 4 | 29 | 0.922 (0.81-0.97) | 0.618 (0.51-0.72) |
| representation.text.entity_name | 2 | 2 | 4 | 0 | 0.333 (0.10-0.70) | 1.000 (0.34-1.00) |
| technology.code.doi | 1 | 0 | 0 | 1 | n/a (n/a) | 0.000 (0.00-0.79) |
| technology.code.locale_code | 1 | 0 | 0 | 1 | n/a (n/a) | 0.000 (0.00-0.79) |
| technology.cryptographic.hash | 2 | 2 | 0 | 0 | 1.000 (0.34-1.00) | 1.000 (0.34-1.00) |
| technology.internet.ip_v4 | 1 | 1 | 0 | 0 | 1.000 (0.21-1.00) | 1.000 (0.21-1.00) |
| technology.internet.url | 5 | 5 | 2 | 0 | 0.714 (0.36-0.92) | 1.000 (0.57-1.00) |

**Headline — column accuracy:** 177/259 = 0.683 (95% CI 0.624-0.737)  
**Macro precision** (mean over labels): 0.792  
**Macro recall** (mean over labels): 0.605  
