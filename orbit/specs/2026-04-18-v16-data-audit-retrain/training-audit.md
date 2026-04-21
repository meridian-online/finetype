# Training Data Audit — Model-as-Critic (ac-02)

## Methodology

- **Training data:** `output/distillation-v3/sherlock_distilled.csv.gz` (102,461 rows, 176 unique labels)
- **Label remapping:** Applied `data/label_remap.json` (35 mappings) before comparison
- **After remap:** 146 unique labels (139 in taxonomy, 4 not in taxonomy)
- **Model:** sherlock-v14 (240 taxonomy types)
- **Sampling:** Up to 20 training examples per label
- **Phase 1:** Combined values from all examples per label into one column, classified once per label (tests aggregate signal)
- **Phase 2:** Classified each training example individually (tests per-example agreement)
- **Header hints disabled:** Used empty header to avoid header bias (tests pure value classification)

## Summary Statistics

### Phase 1: Combined column classification (1 prediction per label)

```
Labels in taxonomy:       139
  Agreements:             69 (49.6%)
  Disagreements:          70 (50.4%)
Labels not in taxonomy:   4
```

### Phase 2: Per-example classification

```
Total examples tested:    1592
  Agreements:             936 (58.8%)
  Disagreements:          656 (41.2%)
```

## Interpreting the Disagreement Rate

The 41.2% per-example disagreement rate is **expected and does not indicate widespread mislabeling**. This audit ran with empty headers, so the model classified on value patterns alone. Many types are inherently header-dependent -- a column of names like `["Fernando", "Fernando"]` will classify as `categorical` without the header "first_name" to disambiguate. The high disagreement rate reflects three distinct phenomena:

1. **Header-dependent types** (expected, not mislabeling) -- Types like `first_name`, `last_name`, `username`, `latitude`, `numeric_code`, `ordinal` are indistinguishable from `categorical`/`decimal_number`/`integer_number` without header context. The model's multi-branch architecture uses header features to make these distinctions. These disagreements confirm the header branch is doing its job.

2. **Genuine mislabeling** (action needed) -- Rows where the values clearly don't match the label. Example: `swift_bic` rows containing `["ROSEDALE", "FRANCE", "WINNEBAGO"]` -- these are place names, not SWIFT/BIC codes.

3. **Ambiguous boundary cases** (quality improvement) -- Types with genuinely overlapping value distributions. Example: `entity_name` vs `categorical` vs `plain_text` depends on interpretation and context.

### Classification of disagreements

```
Category                    Labels  Training Rows   Action
--------------------------  ------  -------------   ------
Header-dependent (OK)           10          3,726   No action (model relies on header branch)
Genuine mislabeling (drop)       5             38   Add to _DROP_ALL_TYPES
Partial mislabeling (filter)     4             98   Add value-pattern filters
Broken remap chain               2          4,574   Fix label_remap.json
Unmappable rows                  4             31   Drop from data
Ambiguous boundary               3         45,293   Review entity_name/categorical/plain_text criteria
Low coverage (augment)          24            ~95   Generate synthetic training data
```

## Phase 1: Combined Column Disagreements

Labels where the model predicts a different type when all training examples for that label are combined into one column.

```
Training Label                                     Model Prediction                                    Conf   Rows Domain?
-------------------------------------------------- -------------------------------------------------- ----- ------ -------
representation.discrete.categorical                representation.text.entity_name                     0.54  17063     yes
representation.text.plain_text                     representation.text.entity_name                     0.74   9998     yes
geography.location.city                            representation.text.entity_name                     0.99   5137      NO
representation.identifier.alphanumeric_id          representation.discrete.ordinal                     0.88   3091     yes
geography.address.full_address                     representation.text.plain_text                      0.99   1989      NO
representation.text.word                           representation.text.entity_name                     0.91   1972     yes
geography.address.street_name                      geography.address.full_address                      0.62   1049     yes
geography.location.country_code                    geography.location.region                           0.78    776     yes
representation.identifier.numeric_code             representation.numeric.integer_number               0.84    605     yes
representation.discrete.ordinal                    representation.text.plain_text                      0.68    404     yes
identity.person.last_name                          geography.location.region                           0.51    311      NO
representation.file.file_size                      representation.numeric.decimal_number               0.38    301     yes
technology.code.locale_code                        technology.development.version                      0.61    217     yes
container.array.comma_separated                    representation.text.entity_name                     0.77    215      NO
representation.numeric.decimal_number_comma        representation.text.plain_text                      0.47    198     yes
representation.boolean.initials                    representation.identifier.alphanumeric_id           0.23    191     yes
representation.identifier.increment                representation.numeric.decimal_number               0.60    191     yes
representation.boolean.terms                       representation.discrete.categorical                 1.00    163     yes
representation.scientific.measurement_unit         representation.identifier.alphanumeric_id           0.50    132     yes
geography.transportation.iata_code                 identity.person.last_name                           0.47    118      NO
representation.boolean.binary                      representation.discrete.categorical                 0.92    114     yes
geography.transportation.icao_code                 finance.banking.swift_bic                           0.96     74      NO
finance.currency.amount_comma                      representation.text.plain_text                      0.55     60      NO
geography.address.postal_code                      representation.text.entity_name                     0.36     60      NO
datetime.period.fiscal_year                        representation.text.plain_text                      0.57     48      NO
container.array.whitespace_separated               representation.text.plain_text                      0.48     41      NO
finance.currency.amount                            representation.text.entity_name                     0.44     36      NO
datetime.date.dmy_space_abbrev                     representation.text.plain_text                      0.96     34      NO
technology.development.version                     representation.text.plain_text                      0.58     30      NO
geography.address.street_suffix                    representation.discrete.ordinal                     1.00     26      NO
datetime.date.year_month                           representation.identifier.alphanumeric_id           0.98     22      NO
geography.coordinate.dms                           geography.address.street_name                       0.91     21     yes
representation.text.emoji                          representation.text.plain_text                      0.95     18     yes
container.array.semicolon_separated                container.array.comma_separated                     0.94     15     yes
datetime.component.month_name                      representation.discrete.ordinal                     0.37     15      NO
geography.coordinate.coordinates                   geography.format.wkt                                0.24     14     yes
datetime.date.weekday_full_month                   datetime.date.weekday_dmy_full                      0.76     11     yes
datetime.duration.iso_8601                         representation.identifier.alphanumeric_id           0.69      9      NO
datetime.date.mdy_dash                             representation.text.plain_text                      0.47      7      NO
identity.commerce.ean                              identity.commerce.isbn                              0.99      6     yes
technology.internet.http_method                    geography.location.region                           0.64      6      NO
technology.internet.url                            representation.text.plain_text                      0.40      6      NO
finance.banking.swift_bic                          geography.location.region                           0.48      5      NO
geography.coordinate.latitude                      representation.numeric.decimal_number               1.00      5      NO
datetime.date.ordinal                              representation.discrete.categorical                 0.56      4      NO
identity.medical.cpt                               representation.discrete.categorical                 0.51      4      NO
identity.person.phone_number                       representation.discrete.categorical                 0.45      4      NO
technology.internet.user_agent                     representation.text.plain_text                      0.52      4      NO
container.array.pipe_separated                     representation.text.entity_name                     0.74      3      NO
datetime.date.month_year_slash                     datetime.component.year                             0.33      3     yes
identity.government.ssn                            representation.discrete.categorical                 0.90      3      NO
container.object.yaml                              representation.discrete.categorical                 0.52      2      NO
datetime.date.dmy_space_full                       representation.discrete.categorical                 0.76      2      NO
datetime.date.ymd_slash                            datetime.period.fiscal_year                         0.27      2     yes
geography.coordinate.longitude                     representation.discrete.categorical                 0.70      2      NO
geography.transportation.hs_code                   representation.numeric.decimal_number               1.00      2      NO
technology.cryptographic.token_urlsafe             representation.text.entity_name                     0.56      2      NO
container.object.csv                               geography.address.full_address                      0.99      1      NO
datetime.date.dmy_dash                             datetime.date.year_month                            0.35      1     yes
datetime.date.dmy_short_slash                      representation.discrete.categorical                 0.64      1      NO
datetime.date.short_dmy                            datetime.date.dmy_space_abbrev                      0.62      1     yes
datetime.time.iso                                  datetime.time.hm_24h                                0.90      1     yes
finance.currency.amount_code_prefix                finance.currency.amount_comma                       0.49      1     yes
geography.format.wkt                               representation.text.entity_name                     0.78      1      NO
geography.location.state_code                      geography.location.region                           0.84      1     yes
identity.medical.ndc                               representation.alphanumeric.alphanumeric_id         0.89      1      NO
representation.format.color_hsl                    representation.text.plain_text                      0.97      1     yes
representation.format.color_rgb                    representation.text.plain_text                      0.87      1     yes
technology.development.docker_ref                  representation.file.mime_type                       0.59      1      NO
technology.internet.top_level_domain               technology.internet.hostname                        0.75      1     yes
```

## Phase 2: Labels with >20% Per-Example Disagreement

These labels have significant per-example disagreement, suggesting potential mislabeling or ambiguous training data.

```
Training Label                                     Disagree  Total   Rate Most Common Alternative                 
-------------------------------------------------- -------- ------ ------ ----------------------------------------
finance.banking.swift_bic                                 5      5   100% geography.location.city (2x)            
geography.address.street_name                            19     19   100% representation.discrete.categorical (16x)
geography.coordinate.latitude                             5      5   100% representation.numeric.decimal_number (4x)
identity.person.first_name                               20     20   100% representation.discrete.categorical (15x)
identity.person.username                                 20     20   100% representation.discrete.categorical (16x)
representation.discrete.ordinal                          20     20   100% representation.discrete.categorical (10x)
representation.identifier.numeric_code                   20     20   100% representation.numeric.integer_number (12x)
container.array.whitespace_separated                     19     20    95% representation.text.plain_text (8x)     
representation.text.word                                 19     20    95% representation.identifier.alphanumeric_id (6x)
representation.file.excel_format                         15     16    94% representation.text.plain_text (4x)     
datetime.period.fiscal_year                              17     19    89% datetime.component.year (7x)            
datetime.duration.iso_8601                                8      9    89% representation.discrete.categorical (3x)
identity.medical.loinc                                    6      7    86% representation.discrete.categorical (5x)
identity.person.gender_code                              17     20    85% identity.person.gender (14x)            
datetime.time.hm_12h                                      5      6    83% datetime.time.hm_24h (3x)               
finance.currency.amount_accounting                        5      6    83% finance.currency.amount (3x)            
identity.commerce.ean                                     5      6    83% identity.commerce.isbn (4x)             
technology.internet.url                                   5      6    83% representation.text.plain_text (2x)     
geography.transportation.unlocode                         7      9    78% identity.person.last_name (2x)          
identity.person.last_name                                15     20    75% representation.text.entity_name (6x)    
representation.discrete.categorical                      15     20    75% representation.text.entity_name (5x)    
representation.file.extension                            15     20    75% representation.discrete.categorical (9x)
datetime.date.mdy_dash                                    5      7    71% representation.text.plain_text (2x)     
geography.address.street_suffix                          14     20    70% representation.text.entity_name (5x)    
geography.transportation.icao_code                       14     20    70% representation.identifier.alphanumeric_id (13x)
technology.internet.http_method                           4      6    67% geography.location.region (2x)          
representation.numeric.integer_number                    13     20    65% representation.discrete.ordinal (7x)    
representation.text.plain_text                           13     20    65% representation.text.entity_name (6x)    
geography.address.postal_code                            12     20    60% representation.text.plain_text (3x)     
representation.scientific.measurement_unit               11     20    55% identity.person.weight (4x)             
container.array.semicolon_separated                       8     15    53% representation.text.entity_name (3x)    
datetime.date.year_month                                 10     19    53% datetime.component.year (6x)            
finance.currency.amount_comma                            10     20    50% representation.numeric.decimal_number_comma (3x)
datetime.component.month_name                             7     15    47% representation.text.entity_name (2x)    
geography.transportation.iata_code                        9     20    45% representation.identifier.alphanumeric_id (4x)
geography.coordinate.coordinates                          6     14    43% representation.discrete.categorical (3x)
finance.currency.amount_nodecimal                         8     20    40% representation.discrete.categorical (8x)
geography.location.country_code                           8     20    40% geography.location.region (6x)          
representation.boolean.initials                           8     20    40% representation.discrete.categorical (3x)
representation.identifier.increment                       8     20    40% representation.discrete.ordinal (5x)    
technology.development.version                            8     20    40% representation.numeric.decimal_number (4x)
datetime.component.periodicity                            4     11    36% datetime.component.day_of_week (2x)     
representation.scientific.smiles                          6     17    35% representation.identifier.alphanumeric_id (2x)
finance.currency.amount                                   6     20    30% finance.currency.amount_comma (2x)      
geography.address.full_address                            6     20    30% representation.discrete.categorical (3x)
geography.location.city                                   6     20    30% identity.person.full_name (2x)          
datetime.date.weekday_abbreviated_month                   3     11    27% datetime.date.weekday_dmy_full (2x)     
identity.person.weight                                    5     20    25% representation.scientific.measurement_unit (5x)
representation.file.mime_type                             5     20    25% representation.text.plain_text (2x)     
representation.text.entity_name                           5     20    25% identity.person.full_name (3x)          
```

### Labels with 10-20% Per-Example Disagreement

```
Training Label                                     Disagree  Total   Rate Most Common Alternative                 
-------------------------------------------------- -------- ------ ------ ----------------------------------------
datetime.time.hms_24h                                     4     20    20% datetime.time.hm_24h (4x)               
geography.coordinate.dms                                  4     20    20% representation.discrete.categorical (3x)
representation.boolean.binary                             4     20    20% representation.discrete.ordinal (3x)    
container.array.comma_separated                           3     20    15% representation.text.entity_name (2x)    
datetime.date.long_full_month                             3     20    15% representation.text.entity_name (1x)    
finance.currency.currency_code                            3     20    15% geography.location.country_code (1x)    
representation.numeric.decimal_number_comma               3     20    15% representation.discrete.categorical (2x)
identity.person.email                                     1      8    12% representation.text.entity_name (1x)    
datetime.date.abbreviated_month                           2     20    10% representation.discrete.categorical (1x)
datetime.date.weekday_full_month                          1     10    10% datetime.date.weekday_dmy_full (1x)     
geography.location.region                                 2     20    10% identity.person.full_name (1x)          
identity.person.full_name                                 2     20    10% geography.format.wkt (1x)               
representation.file.file_size                             2     20    10% representation.text.entity_name (1x)    
representation.identifier.alphanumeric_id                 2     20    10% representation.discrete.categorical (2x)
technology.code.locale_code                               2     20    10% representation.text.entity_name (1x)    
technology.internet.hostname                              2     20    10% representation.discrete.categorical (1x)
```

## Unmapped Labels Analysis

Labels that exist in training data but have no corresponding taxonomy type (even after remap).

### Empty label (`''`)
- **Count:** 20
- **Resolution:** Drop from training data (unlabeled rows)

### `yes`
- **Count:** 2
- **Resolution:** Drop from training data (not a type label)

### `finance.currency.amount_minor_int`
- **Count:** 1
- **Original labels:** {'finance.currency.amount_minor_int': 1}
- **Sample values:** `["220299", "219999", "210399"]`
- **Resolution:** Drop (only 1 row, type not in taxonomy)

### `representation.scientific.metric_prefix`
- **Count:** 8
- **Original labels:** {'representation.scientific.metric_prefix': 8}
- **Sample values:** `["CO2", "\u03b413C-CO2", "\u03b418O-CO2", "\u039414C-CO2", "O2/N2"]`
- **Resolution:** Drop or remap to `representation.discrete.categorical` (8 rows, metric prefixes are categorical values)

### `representation.text.paragraph`
- **Count:** 863
- **Original labels:** {'representation.text.paragraph': 863}
- **Sample values:** `["Branch: Ballygunge, Kolkata 195/4, RASH BEHARI AVENUE, Ballygunge, KOLKATA, Pincode: 700019 City: KOLKATA District: KOLKATA State: WEST BENGAL", "Branch: India Exchange Place, Kolkata P 35, INDIA EXCHANGE PLACE, KOLKATA, Pincode: 700001 City: KOLKATA District: KOLKATA State: WEST BENGAL"]`
- **Resolution:** Remap to `representation.text.plain_text` (paragraph is not in taxonomy)

### `representation.text.sentence`
- **Count:** 3711
- **Original labels:** {'representation.text.sentence': 3677, 'representation.text.description': 30, 'representation.text.title': 4}
- **Sample values:** `["ONLY FOR USERS TO OUR IMPORT AND EXPORT INFORMATION", "ONLY FOR USERS TO OUR IMPORT AND EXPORT INFORMATION", "ONLY FOR USERS TO OUR IMPORT AND EXPORT INFORMATION", "ONLY FOR USERS TO OUR IMPORT AND EXPORT INFORMATION", "ONLY FOR USERS TO OUR IMPORT AND EXPORT INFORMATION"]`
- **Resolution:** Remap to `representation.text.plain_text` (sentence is not in taxonomy, plain_text is the closest match)

## Mislabeling Patterns

Systematic confusion patterns (3+ examples mislabeled the same way):

### geography.address.street_name -> model predicts representation.discrete.categorical (16 examples)

- Model confidence: avg 0.73, range [0.25, 0.96]
- Disambiguation rules applied: {'attractor_demotion_confidence:geography.address.street_name': 10, 'attractor_demotion_cardinality:geography.address.street_name': 6}
- **Cross-domain confusion** — potential mislabeling or very ambiguous values

### identity.person.username -> model predicts representation.discrete.categorical (16 examples)

- Model confidence: avg 0.83, range [0.28, 1.00]
- Disambiguation rules applied: {'attractor_demotion_cardinality:identity.person.username': 13, 'attractor_demotion_confidence:identity.person.username': 2, 'attractor_demotion_validation:identity.person.username': 1}
- **Cross-domain confusion** — potential mislabeling or very ambiguous values

### identity.person.first_name -> model predicts representation.discrete.categorical (15 examples)

- Model confidence: avg 0.84, range [0.34, 1.00]
- Disambiguation rules applied: {'attractor_demotion_cardinality:identity.person.first_name': 14, 'attractor_demotion_cardinality:identity.person.username': 1}
- **Cross-domain confusion** — potential mislabeling or very ambiguous values

### identity.person.gender_code -> model predicts identity.person.gender (14 examples)

- Model confidence: avg 0.64, range [0.59, 0.75]
- Disambiguation rules applied: {'gender_detection': 14}
- **Same domain confusion** — likely ambiguous training data rather than mislabeling

### geography.transportation.icao_code -> model predicts representation.identifier.alphanumeric_id (13 examples)

- Model confidence: avg 0.54, range [0.31, 0.73]
- **Cross-domain confusion** — potential mislabeling or very ambiguous values

### representation.identifier.numeric_code -> model predicts representation.numeric.integer_number (12 examples)

- Model confidence: avg 0.82, range [0.60, 0.98]
- Disambiguation rules applied: {'feature_no_leading_zero:0.00': 1}
- **Same domain confusion** — likely ambiguous training data rather than mislabeling

### representation.discrete.ordinal -> model predicts representation.discrete.categorical (10 examples)

- Model confidence: avg 0.69, range [0.27, 0.93]
- Disambiguation rules applied: {'categorical_low_cardinality': 9, 'categorical_single_char': 1}
- **Same domain confusion** — likely ambiguous training data rather than mislabeling

### representation.file.extension -> model predicts representation.discrete.categorical (9 examples)

- Model confidence: avg 0.95, range [0.60, 1.00]
- Disambiguation rules applied: {'feature_short_code_not_extension:len=3.0,dots=1.00,alpha=1.00': 5, 'feature_short_code_not_extension:len=3.3,dots=1.00,alpha=0.99': 1, 'feature_short_code_not_extension:len=4.0,dots=1.00,alpha=1.00': 1, 'feature_short_code_not_extension:len=3.3,dots=1.00,alpha=1.00': 1, 'feature_short_code_not_extension:len=3.8,dots=1.00,alpha=1.00': 1}
- **Same domain confusion** — likely ambiguous training data rather than mislabeling

### container.array.whitespace_separated -> model predicts representation.text.plain_text (8 examples)

- Model confidence: avg 0.66, range [0.54, 0.80]
- **Cross-domain confusion** — potential mislabeling or very ambiguous values

### finance.currency.amount_nodecimal -> model predicts representation.discrete.categorical (8 examples)

- Model confidence: avg 0.50, range [0.40, 0.67]
- Disambiguation rules applied: {'categorical_low_cardinality': 8}
- **Cross-domain confusion** — potential mislabeling or very ambiguous values

### datetime.period.fiscal_year -> model predicts datetime.component.year (7 examples)

- Model confidence: avg 0.50, range [0.30, 0.74]
- **Same domain confusion** — likely ambiguous training data rather than mislabeling

### representation.numeric.integer_number -> model predicts representation.discrete.ordinal (7 examples)

- Model confidence: avg 0.99, range [0.96, 1.00]
- Disambiguation rules applied: {'small_integer_ordinal': 7}
- **Same domain confusion** — likely ambiguous training data rather than mislabeling

### datetime.date.year_month -> model predicts datetime.component.year (6 examples)

- Model confidence: avg 0.57, range [0.42, 0.95]
- **Same domain confusion** — likely ambiguous training data rather than mislabeling

### geography.location.country_code -> model predicts geography.location.region (6 examples)

- Model confidence: avg 0.85, range [0.82, 0.90]
- **Same domain confusion** — likely ambiguous training data rather than mislabeling

### identity.person.last_name -> model predicts representation.text.entity_name (6 examples)

- Model confidence: avg 0.58, range [0.26, 0.96]
- **Cross-domain confusion** — potential mislabeling or very ambiguous values

### representation.text.plain_text -> model predicts representation.text.entity_name (6 examples)

- Model confidence: avg 0.69, range [0.30, 0.94]
- **Same domain confusion** — likely ambiguous training data rather than mislabeling

### representation.text.word -> model predicts representation.identifier.alphanumeric_id (6 examples)

- Model confidence: avg 0.56, range [0.30, 0.82]
- **Same domain confusion** — likely ambiguous training data rather than mislabeling

### geography.address.street_suffix -> model predicts representation.text.entity_name (5 examples)

- Model confidence: avg 0.39, range [0.24, 0.72]
- **Cross-domain confusion** — potential mislabeling or very ambiguous values

### identity.medical.loinc -> model predicts representation.discrete.categorical (5 examples)

- Model confidence: avg 0.85, range [0.63, 0.98]
- Disambiguation rules applied: {'categorical_low_cardinality': 5}
- **Cross-domain confusion** — potential mislabeling or very ambiguous values

### identity.person.weight -> model predicts representation.scientific.measurement_unit (5 examples)

- Model confidence: avg 0.48, range [0.36, 0.60]
- **Cross-domain confusion** — potential mislabeling or very ambiguous values

### representation.discrete.categorical -> model predicts representation.text.entity_name (5 examples)

- Model confidence: avg 0.62, range [0.30, 0.89]
- **Same domain confusion** — likely ambiguous training data rather than mislabeling

### representation.identifier.increment -> model predicts representation.discrete.ordinal (5 examples)

- Model confidence: avg 0.96, range [0.93, 0.98]
- Disambiguation rules applied: {'small_integer_ordinal': 5}
- **Same domain confusion** — likely ambiguous training data rather than mislabeling

### representation.numeric.integer_number -> model predicts representation.identifier.increment (5 examples)

- Model confidence: avg 0.88, range [0.78, 0.96]
- Disambiguation rules applied: {'numeric_sequential_detection': 5}
- **Same domain confusion** — likely ambiguous training data rather than mislabeling

### representation.text.word -> model predicts representation.text.entity_name (5 examples)

- Model confidence: avg 0.61, range [0.39, 0.90]
- **Same domain confusion** — likely ambiguous training data rather than mislabeling

### representation.text.word -> model predicts representation.discrete.categorical (5 examples)

- Model confidence: avg 0.63, range [0.40, 0.94]
- Disambiguation rules applied: {'categorical_low_cardinality': 3, 'attractor_demotion_cardinality:identity.person.username': 2}
- **Same domain confusion** — likely ambiguous training data rather than mislabeling

### container.array.whitespace_separated -> model predicts representation.discrete.categorical (4 examples)

- Model confidence: avg 0.60, range [0.46, 0.67]
- Disambiguation rules applied: {'categorical_low_cardinality': 4}
- **Cross-domain confusion** — potential mislabeling or very ambiguous values

### datetime.period.fiscal_year -> model predicts representation.discrete.categorical (4 examples)

- Model confidence: avg 0.64, range [0.32, 0.81]
- Disambiguation rules applied: {'categorical_low_cardinality': 4}
- **Cross-domain confusion** — potential mislabeling or very ambiguous values

### datetime.period.fiscal_year -> model predicts datetime.date.year_month (4 examples)

- Model confidence: avg 0.55, range [0.48, 0.63]
- **Same domain confusion** — likely ambiguous training data rather than mislabeling

### datetime.time.hms_24h -> model predicts datetime.time.hm_24h (4 examples)

- Model confidence: avg 0.81, range [0.49, 0.94]
- **Same domain confusion** — likely ambiguous training data rather than mislabeling

### geography.coordinate.latitude -> model predicts representation.numeric.decimal_number (4 examples)

- Model confidence: avg 0.98, range [0.97, 1.00]
- **Cross-domain confusion** — potential mislabeling or very ambiguous values

### geography.transportation.iata_code -> model predicts representation.identifier.alphanumeric_id (4 examples)

- Model confidence: avg 0.44, range [0.42, 0.46]
- **Cross-domain confusion** — potential mislabeling or very ambiguous values

### geography.transportation.iata_code -> model predicts geography.location.country_code (4 examples)

- Model confidence: avg 0.61, range [0.34, 0.98]
- **Same domain confusion** — likely ambiguous training data rather than mislabeling

### identity.commerce.ean -> model predicts identity.commerce.isbn (4 examples)

- Model confidence: avg 0.79, range [0.65, 0.93]
- **Same domain confusion** — likely ambiguous training data rather than mislabeling

### representation.file.excel_format -> model predicts representation.text.plain_text (4 examples)

- Model confidence: avg 0.81, range [0.57, 0.95]
- **Same domain confusion** — likely ambiguous training data rather than mislabeling

### representation.scientific.measurement_unit -> model predicts identity.person.weight (4 examples)

- Model confidence: avg 0.65, range [0.44, 0.98]
- **Cross-domain confusion** — potential mislabeling or very ambiguous values

### representation.text.plain_text -> model predicts representation.discrete.categorical (4 examples)

- Model confidence: avg 0.83, range [0.58, 0.99]
- Disambiguation rules applied: {'categorical_low_cardinality': 4}
- **Same domain confusion** — likely ambiguous training data rather than mislabeling

### technology.development.version -> model predicts representation.numeric.decimal_number (4 examples)

- Model confidence: avg 0.82, range [0.29, 1.00]
- **Cross-domain confusion** — potential mislabeling or very ambiguous values

### container.array.semicolon_separated -> model predicts representation.text.entity_name (3 examples)

- Model confidence: avg 0.63, range [0.57, 0.71]
- **Cross-domain confusion** — potential mislabeling or very ambiguous values

### datetime.duration.iso_8601 -> model predicts representation.discrete.categorical (3 examples)

- Model confidence: avg 0.74, range [0.59, 0.95]
- Disambiguation rules applied: {'categorical_low_cardinality': 3}
- **Cross-domain confusion** — potential mislabeling or very ambiguous values

### datetime.time.hm_12h -> model predicts datetime.time.hm_24h (3 examples)

- Model confidence: avg 0.95, range [0.91, 0.97]
- **Same domain confusion** — likely ambiguous training data rather than mislabeling

### finance.currency.amount_accounting -> model predicts finance.currency.amount (3 examples)

- Model confidence: avg 0.69, range [0.59, 0.78]
- **Same domain confusion** — likely ambiguous training data rather than mislabeling

### finance.currency.amount_comma -> model predicts representation.numeric.decimal_number_comma (3 examples)

- Model confidence: avg 0.56, range [0.46, 0.72]
- **Cross-domain confusion** — potential mislabeling or very ambiguous values

### geography.address.full_address -> model predicts representation.discrete.categorical (3 examples)

- Model confidence: avg 0.71, range [0.62, 0.76]
- Disambiguation rules applied: {'attractor_demotion_confidence:geography.address.street_name': 3}
- **Cross-domain confusion** — potential mislabeling or very ambiguous values

### geography.address.postal_code -> model predicts representation.text.plain_text (3 examples)

- Model confidence: avg 0.83, range [0.64, 0.94]
- **Cross-domain confusion** — potential mislabeling or very ambiguous values

### geography.address.street_suffix -> model predicts representation.text.plain_text (3 examples)

- Model confidence: avg 0.35, range [0.25, 0.42]
- **Cross-domain confusion** — potential mislabeling or very ambiguous values

### geography.address.street_suffix -> model predicts representation.numeric.decimal_number (3 examples)

- Model confidence: avg 1.00, range [0.99, 1.00]
- **Cross-domain confusion** — potential mislabeling or very ambiguous values

### geography.coordinate.coordinates -> model predicts representation.discrete.categorical (3 examples)

- Model confidence: avg 0.59, range [0.40, 0.94]
- Disambiguation rules applied: {'categorical_low_cardinality': 3}
- **Cross-domain confusion** — potential mislabeling or very ambiguous values

### geography.coordinate.dms -> model predicts representation.discrete.categorical (3 examples)

- Model confidence: avg 0.70, range [0.41, 0.92]
- Disambiguation rules applied: {'boolean_override_single_char_categorical': 1, 'categorical_low_cardinality': 2}
- **Cross-domain confusion** — potential mislabeling or very ambiguous values

### identity.person.first_name -> model predicts representation.text.entity_name (3 examples)

- Model confidence: avg 0.50, range [0.37, 0.71]
- **Cross-domain confusion** — potential mislabeling or very ambiguous values

### identity.person.last_name -> model predicts identity.person.full_name (3 examples)

- Model confidence: avg 0.86, range [0.78, 0.94]
- **Same domain confusion** — likely ambiguous training data rather than mislabeling

### representation.boolean.binary -> model predicts representation.discrete.ordinal (3 examples)

- Model confidence: avg 0.70, range [0.56, 0.92]
- Disambiguation rules applied: {'small_integer_ordinal': 3}
- **Same domain confusion** — likely ambiguous training data rather than mislabeling

### representation.boolean.initials -> model predicts representation.discrete.categorical (3 examples)

- Model confidence: avg 0.90, range [0.87, 0.96]
- Disambiguation rules applied: {'boolean_override_single_char_categorical': 3}
- **Same domain confusion** — likely ambiguous training data rather than mislabeling

### representation.discrete.categorical -> model predicts representation.identifier.alphanumeric_id (3 examples)

- Model confidence: avg 0.37, range [0.32, 0.40]
- **Same domain confusion** — likely ambiguous training data rather than mislabeling

### representation.discrete.ordinal -> model predicts representation.text.plain_text (3 examples)

- Model confidence: avg 0.80, range [0.75, 0.87]
- **Same domain confusion** — likely ambiguous training data rather than mislabeling

### representation.file.excel_format -> model predicts datetime.period.fiscal_year (3 examples)

- Model confidence: avg 0.31, range [0.29, 0.33]
- **Cross-domain confusion** — potential mislabeling or very ambiguous values

### representation.file.extension -> model predicts representation.identifier.alphanumeric_id (3 examples)

- Model confidence: avg 0.44, range [0.32, 0.56]
- **Same domain confusion** — likely ambiguous training data rather than mislabeling

### representation.identifier.numeric_code -> model predicts representation.identifier.increment (3 examples)

- Model confidence: avg 0.76, range [0.65, 0.85]
- Disambiguation rules applied: {'numeric_sequential_detection': 3}
- **Same domain confusion** — likely ambiguous training data rather than mislabeling

### representation.text.entity_name -> model predicts identity.person.full_name (3 examples)

- Model confidence: avg 0.62, range [0.55, 0.71]
- **Cross-domain confusion** — potential mislabeling or very ambiguous values


## Recommendations for v16 Training

### 1. Label remapping additions

Add these to `data/label_remap.json`:

```json
{
  "representation.text.sentence": "representation.text.plain_text",
  "representation.text.paragraph": "representation.text.plain_text"
}
```

This fixes the broken remap chain: `description`/`title` currently remap to `sentence`, but `sentence` is not in the taxonomy. After this fix, 4,574 rows (sentence: 3,677 + paragraph: 863 + description: 30 + title: 4) will correctly train as `plain_text`.

### 2. Drop unmappable rows

Drop rows with these labels (total ~31 rows):
- `''` (empty): 20 rows — unlabeled
- `yes`: 2 rows — not a type label
- `finance.currency.amount_minor_int`: 1 row — type not in taxonomy
- `representation.scientific.metric_prefix`: 8 rows — type not in taxonomy

### 3. Genuine mislabeling — drop types (add to `_DROP_ALL_TYPES`)

Manual inspection of sample values confirms these labels contain fully mislabeled data. Add to the `_DROP_ALL_TYPES` set in `prepare_multibranch_data.py`:

- **`finance.banking.swift_bic`** (5 rows): All values are county/country names (`ROSEDALE`, `FRANCE`, `WINNEBAGO`, `SULLIVAN`, `REG_DWORD`). Not a single valid SWIFT/BIC code.
- **`technology.internet.http_method`** (6 rows): All values are random uppercase text (`SAN JOAQUIN`, `GOAT`, `OPERATING`, `IN PROGRESS`, `ENROUTE`). Not a single valid HTTP method (GET, POST, etc.).
- **`representation.file.excel_format`** (16 rows): Values are CLI commands, country names, time durations, fiscal years, file sizes. Not a single valid Excel format string.
- **`identity.medical.loinc`** (7 rows): Values are sports scores (`1-0`, `8-3`, `PG`, `SF`), not LOINC codes (which look like `12345-6`).
- **`identity.medical.cpt`** (4 rows): Mix of 5-digit integers and text like `Early Childhood Education and Teaching`. CPT codes are ambiguous with integers — too noisy to keep at this volume.

Total: 38 rows to add to `_DROP_ALL_TYPES`.

### 4. Partially mislabeled types — needs filtering

These types have a mix of correct and incorrect rows. Need value-pattern filters similar to the existing phone/postal filters:

- **`geography.transportation.icao_code`** (74 rows, 70% disagree): Some rows have valid ICAO codes (`KOTA`, `KCLO`), but others have common English words (`SAKE`, `SAFE`, `SODA`, `SOLO`) or state abbreviations (`ARIZ`, `OREG`). Filter: keep only rows where values match `^[A-Z]{4}$` with known ICAO prefix patterns.
- **`geography.transportation.unlocode`** (9 rows, 78% disagree): Some valid (`ARDRT`, `DEBR4`), but many are county names (`STARK`, `PERRY`, `BLAIR`, `ONTARIO`). Filter: keep only rows with valid UNLOCODE format.
- **`identity.commerce.ean`** (6 rows, 83% disagree): Some valid EAN-13 codes, but one row contains text (`Runt-related transcription factor 2`), and many `978...` codes are actually ISBNs (the model correctly predicts `isbn`). Filter: keep rows with valid EAN-13 checksums.
- **`datetime.duration.iso_8601`** (9 rows, 89% disagree): Only 1 of 9 rows has valid ISO 8601 durations (`P1Y3M5DT7H10M3.3S`). The rest have informal durations (`4:05`, `2h30`) or mislabeled data (`PDF (856K)`). Filter: keep only rows with `P...` pattern.

### 5. Header-dependent types — no action needed

These high-disagreement labels are correctly labeled but require header context for classification. The model's header branch handles this; disagreement without headers is expected. Note: `ordinal`, `increment`, and `categorical` are already excluded from training as `COLUMN_LEVEL_TYPES` in the data pipeline.

- `identity.person.first_name` (181 rows, 100% disagree) — single repeated names classify as `categorical`
- `identity.person.username` (438 rows, 100% disagree) — short strings classify as `categorical`
- `identity.person.last_name` (311 rows, 75% disagree) — names classify as `entity_name`
- `geography.coordinate.latitude` (5 rows, 100% disagree) — decimal numbers, need header "latitude"
- `representation.discrete.ordinal` (404 rows, 100% disagree) — excluded as `COLUMN_LEVEL_TYPE`
- `representation.identifier.numeric_code` (605 rows, 100% disagree) — integers, need header to distinguish from `integer_number`
- `identity.person.gender_code` (589 rows, 85% disagree) — model predicts `gender` (parent type), needs `gender_code` rule
- `geography.location.country_code` (776 rows, 40% disagree) — 2-letter codes overlap with `region`
- `representation.identifier.increment` (191 rows, 40% disagree) — excluded as `COLUMN_LEVEL_TYPE`
- `representation.boolean.initials` (191 rows, 40% disagree) — single chars, need header context

### 6. Ambiguous boundaries — review for consistency

Large-volume labels with moderate disagreement rates. The training data may contain inconsistent labeling across these boundaries:

- **`representation.discrete.categorical` vs `entity_name` vs `plain_text`** (17,063 + 18,232 + 9,998 rows): The three largest labels have significant cross-confusion. 75% of `categorical` examples disagree, mainly predicted as `entity_name`. These labels account for 44% of all training data. Consistent labeling criteria needed.
- **`representation.text.word` vs `entity_name` vs `categorical` vs `alphanumeric_id`** (1,972 rows): 95% disagree — the `word` type overlaps heavily with other text types. Consider whether `word` should remain a distinct type or be merged.
- **`representation.numeric.integer_number` vs `ordinal` vs `increment`** (5,629 rows): 65% disagree — Sharpen rules (`small_integer_ordinal`, `numeric_sequential_detection`) reclassify many integers. Training data may conflict with these rules.

### 7. Low-coverage types to augment

Types with fewer than 10 training rows are under-represented and prone to noise:

```
Label                                      Rows
-----------------------------------------  ----
datetime.date.dmy_dash                        1
datetime.date.dmy_short_slash                 1
datetime.date.short_dmy                       1
datetime.date.ymd_slash                       2
datetime.date.dmy_space_full                  2
datetime.time.iso                             1
geography.coordinate.longitude                2
geography.transportation.hs_code              2
container.object.csv                          1
container.object.yaml                         2
datetime.date.mdy_dash                        7
datetime.duration.iso_8601                    9
datetime.time.hm_12h                          6
finance.banking.swift_bic                     5
finance.currency.amount_accounting            6
finance.currency.amount_code_prefix           1
geography.coordinate.latitude                 5
identity.commerce.ean                         6
identity.government.ssn                       3
identity.medical.cpt                          4
identity.medical.loinc                        7
identity.person.phone_number                  4
technology.internet.http_method               6
technology.internet.url                       6
```

These types need synthetic data augmentation for v16 training. The FineType generator (`finetype generate`) can produce synthetic examples for most of these.
