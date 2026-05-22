---
model_sha: 11f0a570e74aa3d0183e14906da0436e53238469880de0cf1a244c171bddcb57
ydf_sha: 413f2d8767f2a5fc08770dca9a7a2b79648240ad4fd64345a658359481d4d2ce
dbpedia_mapping_sha: ee1f6bbe791156df32826199f3ee0776aff6dae26c2e7a9142e5cb729078e2a7
cascade_version: b931b5936a1e05d1
corpus_index_sha: 27329eda15d6e6b9e2d94231b0c271855489c0463b5a5e9611b2478104e750ec
corpus_pass_id: e86d9af603afb71715aa9566297afbc97b692bb821a3e5dc7ba0f5a7f7bd1a4e
---

# Gittables multi-lens corpus diagnostic — report

This report is the deliverable of `.orbit/specs/2026-05-20-gittables-multi-lens-diagnostic/`. It surfaces and ranks; it does not fix. Runtime metadata (timings, error counts, version provenance beyond the frontmatter) lives in `progress.md` — by design, this document carries no wall-clock timestamp so byte-identical re-runs are detectable via `corpus_pass_id`.

## Part 1 — Corroborated gaps (the headline diagnostic)

Each section below is one `(criterion × mechanism)` cell. Top-10 ranked gap clusters per cell, where a *gap cluster* groups columns sharing the same mechanism, taxonomy prediction, and value shape signature. Each cluster has been independently flagged by **both** lenses (YDF + cascade) — single-lens signals are quarantined to `single_lens_signals.tsv`.

### Criterion: `non_trivial_floor`

#### Mechanism: `format_diversity_path_a`

> no corroborated gaps found

#### Mechanism: `format_diversity_path_b`

> **demoted** by ac-12 attestation (2026-05-23) — 2336 clusters routed to `single_lens_signals.tsv`. See `spot_check_prescreen.md` for the per-gap reasoning and `progress.md` (2026-05-23 entry) for the demotion rationale.

#### Mechanism: `code_vs_canonical_path_a`

> no corroborated gaps found

#### Mechanism: `code_vs_canonical_path_b`

> no corroborated gaps found

#### Mechanism: `enum_overfit`

> no corroborated gaps found

#### Mechanism: `misclassification`

Total clusters in cell: **30865** (126796 distinct columns affected). Top-10 below cover 24430 columns (19.3% of cell).

##### Rank #1 — `e0756c995049…` — 17610 columns — action: `training_data_addition`

- **Corroborating lenses**: **ydf** = `datetime.component.year` (conf 0.57) · **cascade** = `misclassification` (conf 1.00)
- **Candidate spec slug**: _(none — to be assigned downstream)_

| file | column | sense | ydf | samples | dbpedia |
|---|---|---|---|---|---|
| `parent/section4all_xls_40300B%20Ann$_31.parquet` | `Table 4` | `representation.numeric.decimal_number` | `datetime.component.year` | `2008.0` `1277.0` | `` |
| `parent/section4all_xls_40300B%20Ann$_37.parquet` | `Table 4` | `representation.numeric.decimal_number` | `datetime.component.year` | `2008.0` `1277.0` | `` |
| `parent/section4all_xls_40300B%20Ann$_39.parquet` | `Table 4` | `representation.numeric.decimal_number` | `datetime.component.year` | `2008.0` `1277.0` | `` |
| `parent/section4all_xls_40300B%20Ann$_40.parquet` | `Table 4` | `representation.numeric.decimal_number` | `datetime.component.year` | `2008.0` `1277.0` | `` |
| `parent/section4all_xls_40300B%20Ann$_69.parquet` | `__index_level_25__` | `representation.numeric.decimal_number` | `datetime.component.year` | `2007.0` `1164.0` | `` |

##### Rank #2 — `f4c025140768…` — 1826 columns — action: `training_data_addition`

- **Corroborating lenses**: **ydf** = `representation.text.plain_text` (conf 0.52) · **cascade** = `misclassification` (conf 1.00)
- **Candidate spec slug**: _(none — to be assigned downstream)_

| file | column | sense | ydf | samples | dbpedia |
|---|---|---|---|---|---|
| `id/PARTICIPANT_ITC_SDRD_Builder_2020-04-07_09h53.25.817.parquet` | `spacebar2.rt` | `representation.numeric.decimal_number` | `representation.text.plain_text` | `0.7` | `` |
| `time_interval/participant101keysequencedata_2020-01-15-09_15_16-joachim.parquet` | `InterTrialIntervalSeconds` | `representation.numeric.decimal_number` | `representation.text.plain_text` | `2.5` | `http://dbpedia.org/ontology/minTime` |
| `time_interval/participant101keysequencedata_2020-01-15-09_15_16-joachim.parquet` | `TargetFabInputRate` | `representation.numeric.decimal_number` | `representation.text.plain_text` | `0.3` | `` |
| `time_interval/participant101keysequencedata_2020-01-15-09_31_57-joachim.parquet` | `InterTrialIntervalSeconds` | `representation.numeric.decimal_number` | `representation.text.plain_text` | `2.5` | `http://dbpedia.org/ontology/minTime` |
| `time_interval/participant101keysequencedata_2020-01-15-09_31_57-joachim_1.parquet` | `InterTrialIntervalSeconds` | `representation.numeric.decimal_number` | `representation.text.plain_text` | `2.5` | `http://dbpedia.org/ontology/minTime` |

##### Rank #3 — `9a5a75088c13…` — 1088 columns — action: `training_data_addition`

- **Corroborating lenses**: **ydf** = `representation.text.entity_name` (conf 0.76) · **cascade** = `misclassification` (conf 1.00)
- **Candidate spec slug**: _(none — to be assigned downstream)_

| file | column | sense | ydf | samples | dbpedia |
|---|---|---|---|---|---|
| `data_rate/CWEGF.parquet` | `quoteSourceName` | `representation.text.plain_text` | `representation.text.entity_name` | `Delayed Quote` `Delayed Quote` `Delayed Quote` | `http://dbpedia.org/ontology/quote` |
| `data_rate/CWGL.parquet` | `quoteSourceName` | `representation.text.plain_text` | `representation.text.entity_name` | `Delayed Quote` `Delayed Quote` `Delayed Quote` | `http://dbpedia.org/ontology/quote` |
| `data_rate/CWNR.parquet` | `quoteSourceName` | `representation.text.plain_text` | `representation.text.entity_name` | `Delayed Quote` `Delayed Quote` `Delayed Quote` | `http://dbpedia.org/ontology/quote` |
| `data_rate/CWSFF.parquet` | `quoteSourceName` | `representation.text.plain_text` | `representation.text.entity_name` | `Delayed Quote` `Delayed Quote` `Delayed Quote` | `http://dbpedia.org/ontology/quote` |
| `data_rate/CWVLF.parquet` | `quoteSourceName` | `representation.text.plain_text` | `representation.text.entity_name` | `Delayed Quote` `Delayed Quote` `Delayed Quote` | `http://dbpedia.org/ontology/quote` |

##### Rank #4 — `d10050d245ea…` — 877 columns — action: `training_data_addition`

- **Corroborating lenses**: **ydf** = `representation.text.entity_name` (conf 0.57) · **cascade** = `misclassification` (conf 1.00)
- **Candidate spec slug**: _(none — to be assigned downstream)_

| file | column | sense | ydf | samples | dbpedia |
|---|---|---|---|---|---|
| `processing_time/E03-1005_sweta_1.parquet` | `Discourse Facet` | `representation.text.plain_text` | `representation.text.entity_name` | `Method Citation` `Method Citation` `Method Citation` | `` |
| `processing_time/E03-1005_sweta_101.parquet` | `Discourse Facet` | `representation.text.plain_text` | `representation.text.entity_name` | `Method Citation` `Method Citation` `Method Citation` | `` |
| `processing_time/E03-1005_sweta_104.parquet` | `Discourse Facet` | `representation.text.plain_text` | `representation.text.entity_name` | `Method Citation` `Method Citation` `Method Citation` | `` |
| `processing_time/E03-1005_sweta_108.parquet` | `Discourse Facet` | `representation.text.plain_text` | `representation.text.entity_name` | `Method Citation` `Method Citation` `Method Citation` | `` |
| `processing_time/E03-1005_sweta_11.parquet` | `Discourse Facet` | `representation.text.plain_text` | `representation.text.entity_name` | `Method Citation` `Method Citation` `Method Citation` | `` |

##### Rank #5 — `644d4c0fb5e4…` — 673 columns — action: `training_data_addition`

- **Corroborating lenses**: **ydf** = `representation.numeric.integer_number` (conf 0.98) · **cascade** = `misclassification` (conf 1.00)
- **Candidate spec slug**: _(none — to be assigned downstream)_

| file | column | sense | ydf | samples | dbpedia |
|---|---|---|---|---|---|
| `seek_time/PL2311LAG1TTXC.parquet` | `Smart 194: Temperature_Celsius (Raw Value)` | `representation.numeric.decimal_number` | `representation.numeric.integer_number` | `26` `26` `25` | `` |
| `seek_time/PL2311LAG1TYJC.parquet` | `Smart 194: Temperature_Celsius (Raw Value)` | `representation.numeric.decimal_number` | `representation.numeric.integer_number` | `29` `28` `27` | `` |
| `seek_time/PL2311LAG1UBKC.parquet` | `Smart 194: Temperature_Celsius (Raw Value)` | `representation.numeric.decimal_number` | `representation.numeric.integer_number` | `28` `27` `26` | `` |
| `seek_time/PL2311LAG1Y9SC_1.parquet` | `Smart 194: Temperature_Celsius (Raw Value)` | `representation.numeric.decimal_number` | `representation.numeric.integer_number` | `28` `28` `28` | `` |
| `seek_time/PL2321LAGASNRJ.parquet` | `Smart 194: Temperature_Celsius (Raw Value)` | `representation.numeric.decimal_number` | `representation.numeric.integer_number` | `30` `30` `30` | `` |

##### Rank #6 — `022debba8a26…` — 508 columns — action: `training_data_addition`

- **Corroborating lenses**: **ydf** = `representation.text.entity_name` (conf 0.57) · **cascade** = `misclassification` (conf 1.00)
- **Candidate spec slug**: _(none — to be assigned downstream)_

| file | column | sense | ydf | samples | dbpedia |
|---|---|---|---|---|---|
| `show_time/example_101.parquet` | `Scale values` | `representation.text.plain_text` | `representation.text.entity_name` | `Not yet competent,Competent` | `http://dbpedia.org/ontology/scale` |
| `show_time/example_108.parquet` | `Scale values` | `representation.text.plain_text` | `representation.text.entity_name` | `Not yet competent,Competent` | `http://dbpedia.org/ontology/scale` |
| `show_time/example_111.parquet` | `Scale values` | `representation.text.plain_text` | `representation.text.entity_name` | `Not yet competent,Competent` | `http://dbpedia.org/ontology/scale` |
| `show_time/example_115.parquet` | `Scale values` | `representation.text.plain_text` | `representation.text.entity_name` | `Not yet competent,Competent` | `http://dbpedia.org/ontology/scale` |
| `show_time/example_121.parquet` | `Scale values` | `representation.text.plain_text` | `representation.text.entity_name` | `Not yet competent,Competent` | `http://dbpedia.org/ontology/scale` |

##### Rank #7 — `8ad55f498be9…` — 508 columns — action: `training_data_addition`

- **Corroborating lenses**: **ydf** = `representation.text.sentence` (conf 0.63) · **cascade** = `misclassification` (conf 1.00)
- **Candidate spec slug**: _(none — to be assigned downstream)_

| file | column | sense | ydf | samples | dbpedia |
|---|---|---|---|---|---|
| `show_time/example_101.parquet` | `Description` | `representation.text.plain_text` | `representation.text.sentence` | `<p>The Core Competencies summarise the capabilities that ar…` `<p>Each level of the Core Competencies has behavioural indi…` `<p>Level 1 is typically associated with jobs such as Assist…` | `http://dbpedia.org/ontology/description` |
| `show_time/example_108.parquet` | `Description` | `representation.text.plain_text` | `representation.text.sentence` | `<p>The Core Competencies summarise the capabilities that ar…` `<p>Each level of the Core Competencies has behavioural indi…` `<p>Level 1 is typically associated with jobs such as Assist…` | `http://dbpedia.org/ontology/description` |
| `show_time/example_111.parquet` | `Description` | `representation.text.plain_text` | `representation.text.sentence` | `<p>The Core Competencies summarise the capabilities that ar…` `<p>Each level of the Core Competencies has behavioural indi…` `<p>Level 1 is typically associated with jobs such as Assist…` | `http://dbpedia.org/ontology/description` |
| `show_time/example_115.parquet` | `Description` | `representation.text.plain_text` | `representation.text.sentence` | `<p>The Core Competencies summarise the capabilities that ar…` `<p>Each level of the Core Competencies has behavioural indi…` `<p>Level 1 is typically associated with jobs such as Assist…` | `http://dbpedia.org/ontology/description` |
| `show_time/example_121.parquet` | `Description` | `representation.text.plain_text` | `representation.text.sentence` | `<p>The Core Competencies summarise the capabilities that ar…` `<p>Each level of the Core Competencies has behavioural indi…` `<p>Level 1 is typically associated with jobs such as Assist…` | `http://dbpedia.org/ontology/description` |

##### Rank #8 — `e0fa5a1ea315…` — 455 columns — action: `training_data_addition`

- **Corroborating lenses**: **ydf** = `representation.text.sentence` (conf 0.54) · **cascade** = `misclassification` (conf 1.00)
- **Candidate spec slug**: _(none — to be assigned downstream)_

| file | column | sense | ydf | samples | dbpedia |
|---|---|---|---|---|---|
| `abstraction/Designite_01.%20Mobile_DesignSmells.parquet` | `Cause` | `representation.text.plain_text` | `representation.text.sentence` | `The tool detected the smell in this class because the class…` `The tool detected the smell in this class because the class…` `The tool detected the smell in this class because this clas…` | `http://dbpedia.org/ontology/deathCause` |
| `abstraction/Designite_02.%20DatabaseChange_DesignSmells.parquet` | `Cause` | `representation.text.plain_text` | `representation.text.sentence` | `The tool detected the smell in this class because this clas…` `The tool detected the smell in this class because this clas…` `The tool detected the smell in this class because this clas…` | `http://dbpedia.org/ontology/deathCause` |
| `abstraction/Designite_02.%20SubstringModule.Web_DesignSmells.parquet` | `Cause` | `representation.text.plain_text` | `representation.text.sentence` | `The tool detected the smell in this class because this clas…` `The tool detected the smell in this class because the class…` | `http://dbpedia.org/ontology/deathCause` |
| `abstraction/Designite_1TobiiTobii_DesignSmells.parquet` | `Cause` | `representation.text.plain_text` | `representation.text.sentence` | `The tool detected the smell in this class because this clas…` `The tool detected the smell in this class because the class…` | `http://dbpedia.org/ontology/deathCause` |
| `abstraction/Designite_A1_DesignSmells.parquet` | `Cause` | `representation.text.plain_text` | `representation.text.sentence` | `The tool detected the smell in this class because the class…` `The tool detected the smell in this class because this clas…` `The tool detected the smell in this class because this clas…` | `http://dbpedia.org/ontology/deathCause` |

##### Rank #9 — `414130dd67c2…` — 443 columns — action: `training_data_addition`

- **Corroborating lenses**: **ydf** = `representation.text.entity_name` (conf 0.55) · **cascade** = `misclassification` (conf 1.00)
- **Candidate spec slug**: _(none — to be assigned downstream)_

| file | column | sense | ydf | samples | dbpedia |
|---|---|---|---|---|---|
| `id/ir.model.access_14425.parquet` | `group_id:id` | `representation.text.plain_text` | `representation.text.entity_name` | `base.group_system` `base.group_user` `base.group_system` | `http://dbpedia.org/ontology/elementGroup` |
| `id/ir.model.access_14670.parquet` | `group_id:id` | `representation.text.plain_text` | `representation.text.entity_name` | `base.group_system` `base.group_user` `base.group_system` | `http://dbpedia.org/ontology/elementGroup` |
| `id/ir.model.access_14788.parquet` | `group_id:id` | `representation.text.plain_text` | `representation.text.entity_name` | `base.group_system` `base.group_user` `base.group_system` | `http://dbpedia.org/ontology/elementGroup` |
| `id/ir.model.access_21606.parquet` | `group_id:id` | `representation.text.plain_text` | `representation.text.entity_name` | `base.group_system` `base.group_system` `base.group_user` | `http://dbpedia.org/ontology/elementGroup` |
| `id/ir.model.access_21768.parquet` | `group_id:id` | `representation.text.plain_text` | `representation.text.entity_name` | `base.group_system` `base.group_system` `base.group_user` | `http://dbpedia.org/ontology/elementGroup` |

##### Rank #10 — `7e0ecb31fabb…` — 442 columns — action: `training_data_addition`

- **Corroborating lenses**: **ydf** = `representation.text.sentence` (conf 0.70) · **cascade** = `misclassification` (conf 1.00)
- **Candidate spec slug**: _(none — to be assigned downstream)_

| file | column | sense | ydf | samples | dbpedia |
|---|---|---|---|---|---|
| `id/mdl_question_10.parquet` | `questiontext` | `representation.text.plain_text` | `representation.text.sentence` | `What is the purpose of life?` `What is the purpose of life?` `What is the purpose of life?` | `` |
| `id/mdl_question_100.parquet` | `questiontext` | `representation.text.plain_text` | `representation.text.sentence` | `What is the purpose of life?` `What is the purpose of life?` `What is the purpose of life?` | `` |
| `id/mdl_question_102.parquet` | `questiontext` | `representation.text.plain_text` | `representation.text.sentence` | `What is the purpose of life?` `What is the purpose of life?` `What is the purpose of life?` | `` |
| `id/mdl_question_105.parquet` | `questiontext` | `representation.text.plain_text` | `representation.text.sentence` | `What is the purpose of life?` `What is the purpose of life?` `What is the purpose of life?` | `` |
| `id/mdl_question_113.parquet` | `questiontext` | `representation.text.plain_text` | `representation.text.sentence` | `What is the purpose of life?` `What is the purpose of life?` `What is the purpose of life?` | `` |

#### Mechanism: `validator_widening`

> no corroborated gaps found

#### Mechanism: `unknown_no_fit`

> no corroborated gaps found

#### Mechanism: `fallthrough`

> no corroborated gaps found

### Criterion: `reject_rate_ceil`

#### Mechanism: `format_diversity_path_a`

> **demoted** by ac-12 attestation (2026-05-23) — 8 clusters routed to `single_lens_signals.tsv`. See `spot_check_prescreen.md` for the per-gap reasoning and `progress.md` (2026-05-23 entry) for the demotion rationale.

#### Mechanism: `format_diversity_path_b`

Total clusters in cell: **767** (5339 distinct columns affected). Top-10 below cover 3222 columns (60.3% of cell).

##### Rank #1 — `2129fe8b10c7…` — 1320 columns — action: `model_retrain`

- **Corroborating lenses**: **ydf** = `representation.discrete.categorical` (conf 0.70) · **cascade** = `format_diversity_path_b` (conf 1.00)
- **Candidate spec slug**: _(none — to be assigned downstream)_

| file | column | sense | ydf | samples | dbpedia |
|---|---|---|---|---|---|
| `time_slot/20151128BKNCLE.parquet` | `period` | `datetime.period.quarter` | `representation.discrete.categorical` | `Q4` `Q4` `Q4` | `http://dbpedia.org/ontology/period` |
| `time_slot/20160118HOULAC.parquet` | `period` | `datetime.period.quarter` | `representation.discrete.categorical` | `Q4` `Q4` `Q4` | `http://dbpedia.org/ontology/period` |
| `show_time/20150405CHICLE.parquet` | `period` | `datetime.period.quarter` | `representation.discrete.categorical` | `Q4` `Q4` `Q4` | `http://dbpedia.org/ontology/period` |
| `show_time/20151118MINORL.parquet` | `period` | `datetime.period.quarter` | `representation.discrete.categorical` | `Q4` `Q4` `Q4` | `http://dbpedia.org/ontology/period` |
| `show_time/20151130BOSMIA.parquet` | `period` | `datetime.period.quarter` | `representation.discrete.categorical` | `Q4` `Q4` `Q4` | `http://dbpedia.org/ontology/period` |

##### Rank #2 — `1343378923ae…` — 530 columns — action: `model_retrain`

- **Corroborating lenses**: **ydf** = `representation.numeric.decimal_number` (conf 0.98) · **cascade** = `format_diversity_path_b` (conf 1.00)
- **Candidate spec slug**: _(none — to be assigned downstream)_

| file | column | sense | ydf | samples | dbpedia |
|---|---|---|---|---|---|
| `time_interval/covid-19-nursing-home-dataset-week-ending-2020-11-22-AZ-2020-05-31.parquet` | `Number of Ventilators in Facility` | `representation.numeric.integer_number` | `representation.numeric.decimal_number` | `3.0` | `http://dbpedia.org/ontology/numberOfRooms` |
| `time_interval/covid-19-nursing-home-dataset-week-ending-2020-11-22-AZ-2020-05-31.parquet` | `Number of Ventilators in Use for COVID-19` | `representation.numeric.integer_number` | `representation.numeric.decimal_number` | `0.0` | `` |
| `time_interval/covid-19-nursing-home-dataset-week-ending-2020-11-22-AZ-2020-06-14.parquet` | `Number of Ventilators in Facility` | `representation.numeric.integer_number` | `representation.numeric.decimal_number` | `3.0` | `http://dbpedia.org/ontology/numberOfRooms` |
| `time_interval/covid-19-nursing-home-dataset-week-ending-2020-11-22-AZ-2020-06-14.parquet` | `Number of Ventilators in Use for COVID-19` | `representation.numeric.integer_number` | `representation.numeric.decimal_number` | `2.0` | `` |
| `time_interval/covid-19-nursing-home-dataset-week-ending-2020-11-22-AZ-2020-08-09.parquet` | `Number of Ventilators in Use for COVID-19` | `representation.numeric.integer_number` | `representation.numeric.decimal_number` | `0.0` | `` |

##### Rank #3 — `667cd51c18f7…` — 504 columns — action: `model_retrain`

- **Corroborating lenses**: **ydf** = `identity.person.gender_code` (conf 0.98) · **cascade** = `format_diversity_path_b` (conf 1.00)
- **Candidate spec slug**: _(none — to be assigned downstream)_

| file | column | sense | ydf | samples | dbpedia |
|---|---|---|---|---|---|
| `parent/student_75.parquet` | `gender` | `identity.person.gender` | `identity.person.gender_code` | `M` `M` `M` | `http://dbpedia.org/ontology/gender` |
| `parent/student_dataset.parquet` | `gender` | `identity.person.gender` | `identity.person.gender_code` | `M` `M` `M` | `http://dbpedia.org/ontology/gender` |
| `parent/test_639.parquet` | `Provider Gender Code` | `identity.person.gender` | `identity.person.gender_code` | `M` `M` `M` | `http://dbpedia.org/ontology/code` |
| `parent/train_83.parquet` | `gender` | `identity.person.gender` | `identity.person.gender_code` | `M` `F` `M` | `http://dbpedia.org/ontology/gender` |
| `parent/uds-saarland_participants.parquet` | `gender` | `identity.person.gender` | `identity.person.gender_code` | `M` `M` `F` | `http://dbpedia.org/ontology/gender` |

##### Rank #4 — `20f7a63b946a…` — 258 columns — action: `model_retrain`

- **Corroborating lenses**: **ydf** = `representation.numeric.decimal_number` (conf 0.99) · **cascade** = `format_diversity_path_b` (conf 1.00)
- **Candidate spec slug**: _(none — to be assigned downstream)_

| file | column | sense | ydf | samples | dbpedia |
|---|---|---|---|---|---|
| `time_interval/covid-19-nursing-home-dataset-week-ending-2020-11-22-AZ-2020-05-31.parquet` | `Number of All Beds` | `representation.numeric.integer_number` | `representation.numeric.decimal_number` | `120.0` `120.0` `90.0` | `http://dbpedia.org/ontology/numberOfRooms` |
| `time_interval/covid-19-nursing-home-dataset-week-ending-2020-11-22-AZ-2020-06-14.parquet` | `Number of All Beds` | `representation.numeric.integer_number` | `representation.numeric.decimal_number` | `31.0` `60.0` `66.0` | `http://dbpedia.org/ontology/numberOfRooms` |
| `time_interval/covid-19-nursing-home-dataset-week-ending-2020-11-22-AZ-2020-07-26.parquet` | `Number of All Beds` | `representation.numeric.integer_number` | `representation.numeric.decimal_number` | `191.0` `128.0` `58.0` | `http://dbpedia.org/ontology/numberOfRooms` |
| `time_interval/covid-19-nursing-home-dataset-week-ending-2020-11-22-AZ-2020-08-02.parquet` | `Number of All Beds` | `representation.numeric.integer_number` | `representation.numeric.decimal_number` | `180.0` `109.0` `115.0` | `http://dbpedia.org/ontology/numberOfRooms` |
| `time_interval/covid-19-nursing-home-dataset-week-ending-2020-11-22-AZ-2020-08-09.parquet` | `Number of All Beds` | `representation.numeric.integer_number` | `representation.numeric.decimal_number` | `100.0` `24.0` `103.0` | `http://dbpedia.org/ontology/numberOfRooms` |

##### Rank #5 — `8684ba9e2904…` — 206 columns — action: `model_retrain`

- **Corroborating lenses**: **ydf** = `geography.location.region` (conf 0.77) · **cascade** = `format_diversity_path_b` (conf 1.00)
- **Candidate spec slug**: _(none — to be assigned downstream)_

| file | column | sense | ydf | samples | dbpedia |
|---|---|---|---|---|---|
| `id/Practitioner_1.parquet` | `address_state` | `geography.address.full_address` | `geography.location.region` | `IL` | `http://dbpedia.org/ontology/state` |
| `speed_of_light/2016-09-02.parquet` | `ADDRESS STATE` | `geography.address.full_address` | `geography.location.region` | `TX` `TX` `TX` | `http://dbpedia.org/ontology/state` |
| `speed_of_light/2016-09-24.parquet` | `ADDRESS STATE` | `geography.address.full_address` | `representation.discrete.categorical` | `TX` `TX` `TX` | `http://dbpedia.org/ontology/state` |
| `speed_of_light/2016-10-01.parquet` | `ADDRESS STATE` | `geography.address.full_address` | `representation.discrete.categorical` | `TX` `TX` `TX` | `http://dbpedia.org/ontology/state` |
| `speed_of_light/2016-10-14.parquet` | `ADDRESS STATE` | `geography.address.full_address` | `representation.discrete.categorical` | `TX` `TX` `TX` | `http://dbpedia.org/ontology/state` |

##### Rank #6 — `5d497fbaeac4…` — 100 columns — action: `model_retrain`

- **Corroborating lenses**: **ydf** = `representation.discrete.categorical` (conf 0.75) · **cascade** = `format_diversity_path_b` (conf 1.00)
- **Candidate spec slug**: _(none — to be assigned downstream)_

| file | column | sense | ydf | samples | dbpedia |
|---|---|---|---|---|---|
| `id/ScheduleG_PAC.parquet` | `Middle Name` | `identity.person.full_name` | `representation.discrete.categorical` | `N` `N` `N` | `http://dbpedia.org/ontology/name` |
| `show_time/ASL_FormA_data.parquet` | `Child's Own Name` | `identity.person.full_name` | `representation.discrete.categorical` | `P` `U` `P` | `http://dbpedia.org/ontology/personName` |
| `show_time/ASL_FormC_data.parquet` | `Child's Own Name` | `identity.person.full_name` | `representation.boolean.initials` | `Y` `N` `N` | `http://dbpedia.org/ontology/personName` |
| `object/DM_Attributes_1.parquet` | `Entity_Name` | `identity.person.full_name` | `representation.discrete.categorical` | `P` `P` `P` | `http://dbpedia.org/ontology/name` |
| `id/contract_89.parquet` | `class_name` | `identity.person.full_name` | `identity.person.gender_code` | `F` `F` `F` | `http://dbpedia.org/ontology/class` |

##### Rank #7 — `c1101aeb2474…` — 97 columns — action: `model_retrain`

- **Corroborating lenses**: **ydf** = `representation.numeric.decimal_number` (conf 0.98) · **cascade** = `format_diversity_path_b` (conf 1.00)
- **Candidate spec slug**: _(none — to be assigned downstream)_

| file | column | sense | ydf | samples | dbpedia |
|---|---|---|---|---|---|
| `time_interval/covid-19-nursing-home-dataset-week-ending-2020-11-22-AZ-2020-08-09.parquet` | `Number of Ventilators in Facility` | `representation.numeric.integer_number` | `representation.numeric.decimal_number` | `10.0` | `http://dbpedia.org/ontology/numberOfRooms` |
| `time_interval/covid-19-nursing-home-dataset-week-ending-2020-11-22-ID-2020-10-25.parquet` | `Number of Ventilators in Facility` | `representation.numeric.integer_number` | `representation.numeric.decimal_number` | `12.0` | `http://dbpedia.org/ontology/numberOfRooms` |
| `time_interval/covid-19-nursing-home-dataset-week-ending-2020-11-22-ID-2020-11-22.parquet` | `Number of All Beds` | `representation.numeric.integer_number` | `representation.numeric.decimal_number` | `48.0` `83.0` `88.0` | `http://dbpedia.org/ontology/numberOfRooms` |
| `time_interval/covid-19-nursing-home-dataset-week-ending-2020-11-22-NV-2020-06-14.parquet` | `Number of Ventilators in Facility` | `representation.numeric.integer_number` | `representation.numeric.decimal_number` | `19.0` | `http://dbpedia.org/ontology/numberOfRooms` |
| `time_interval/covid-19-nursing-home-dataset-week-ending-2020-11-22-NV-2020-06-14.parquet` | `Number of Ventilators in Use for COVID-19` | `representation.numeric.integer_number` | `representation.numeric.decimal_number` | `17.0` | `` |

##### Rank #8 — `5a7732688996…` — 76 columns — action: `model_retrain`

- **Corroborating lenses**: **ydf** = `representation.discrete.categorical` (conf 0.52) · **cascade** = `format_diversity_path_b` (conf 1.00)
- **Candidate spec slug**: _(none — to be assigned downstream)_

| file | column | sense | ydf | samples | dbpedia |
|---|---|---|---|---|---|
| `lead_time/JUG_UA_events_past.parquet` | `venue_country` | `geography.location.country_code` | `representation.discrete.categorical` | `ua` `ua` `ua` | `http://dbpedia.org/ontology/country` |
| `lead_time/streams_events_past.parquet` | `venue_country` | `geography.location.country_code` | `representation.discrete.categorical` | `us` `us` `us` | `http://dbpedia.org/ontology/country` |
| `processing_time/Big-Data-Developers-in-Denver_events_past.parquet` | `venue_country` | `geography.location.country_code` | `representation.discrete.categorical` | `us` `us` `us` | `http://dbpedia.org/ontology/country` |
| `processing_time/Big-Data-Developers-in-Detroit_events_past.parquet` | `venue_country` | `geography.location.country_code` | `representation.discrete.categorical` | `us` `us` `us` | `http://dbpedia.org/ontology/country` |
| `processing_time/Big-Data-Developers-in-Houston_events_past.parquet` | `venue_country` | `geography.location.country_code` | `representation.discrete.categorical` | `us` `us` `us` | `http://dbpedia.org/ontology/country` |

##### Rank #9 — `f007ca304919…` — 71 columns — action: `model_retrain`

- **Corroborating lenses**: **ydf** = `geography.location.region` (conf 0.60) · **cascade** = `format_diversity_path_b` (conf 1.00)
- **Candidate spec slug**: _(none — to be assigned downstream)_

| file | column | sense | ydf | samples | dbpedia |
|---|---|---|---|---|---|
| `time_interval/covid-19-nursing-home-dataset-week-ending-2020-11-22-ND-2020-05-24.parquet` | `Provider State` | `geography.location.country_code` | `geography.location.region` | `ND` `ND` `ND` | `http://dbpedia.org/ontology/state` |
| `time_interval/covid-19-nursing-home-dataset-week-ending-2020-11-22-ND-2020-05-31.parquet` | `Provider State` | `geography.location.country_code` | `geography.location.region` | `ND` `ND` `ND` | `http://dbpedia.org/ontology/state` |
| `time_interval/covid-19-nursing-home-dataset-week-ending-2020-11-22-ND-2020-06-14.parquet` | `Provider State` | `geography.location.country_code` | `geography.location.region` | `ND` `ND` `ND` | `http://dbpedia.org/ontology/state` |
| `time_interval/covid-19-nursing-home-dataset-week-ending-2020-11-22-ND-2020-06-21.parquet` | `Provider State` | `geography.location.country_code` | `geography.location.region` | `ND` `ND` `ND` | `http://dbpedia.org/ontology/state` |
| `time_interval/covid-19-nursing-home-dataset-week-ending-2020-11-22-ND-2020-06-28.parquet` | `Provider State` | `geography.location.country_code` | `geography.location.region` | `ND` `ND` `ND` | `http://dbpedia.org/ontology/state` |

##### Rank #10 — `0c95caa04538…` — 60 columns — action: `model_retrain`

- **Corroborating lenses**: **ydf** = `representation.numeric.decimal_number` (conf 0.98) · **cascade** = `format_diversity_path_b` (conf 1.00)
- **Candidate spec slug**: _(none — to be assigned downstream)_

| file | column | sense | ydf | samples | dbpedia |
|---|---|---|---|---|---|
| `time_interval/covid-19-nursing-home-dataset-week-ending-2020-11-22-NV-2020-07-12.parquet` | `Number of All Beds` | `representation.numeric.integer_number` | `representation.numeric.decimal_number` | `144.0` `49.0` `98.0` | `http://dbpedia.org/ontology/numberOfRooms` |
| `time_interval/covid-19-nursing-home-dataset-week-ending-2020-11-22-NV-2020-07-19.parquet` | `Number of All Beds` | `representation.numeric.integer_number` | `representation.numeric.decimal_number` | `47.0` `42.0` `2.0` | `http://dbpedia.org/ontology/numberOfRooms` |
| `time_interval/covid-19-nursing-home-dataset-week-ending-2020-11-22-NV-2020-08-30.parquet` | `Number of All Beds` | `representation.numeric.integer_number` | `representation.numeric.decimal_number` | `25.0` `125.0` `47.0` | `http://dbpedia.org/ontology/numberOfRooms` |
| `time_interval/covid-19-nursing-home-dataset-week-ending-2020-11-22-NV-2020-10-04.parquet` | `Number of All Beds` | `representation.numeric.integer_number` | `representation.numeric.decimal_number` | `95.0` `42.0` `2.0` | `http://dbpedia.org/ontology/numberOfRooms` |
| `time_interval/covid-19-nursing-home-dataset-week-ending-2020-11-22-NV-2020-10-25.parquet` | `Number of All Beds` | `representation.numeric.integer_number` | `representation.numeric.decimal_number` | `96.0` `170.0` `182.0` | `http://dbpedia.org/ontology/numberOfRooms` |

#### Mechanism: `code_vs_canonical_path_a`

> **demoted** by ac-12 attestation (2026-05-23) — 1 clusters routed to `single_lens_signals.tsv`. See `spot_check_prescreen.md` for the per-gap reasoning and `progress.md` (2026-05-23 entry) for the demotion rationale.

#### Mechanism: `code_vs_canonical_path_b`

> no corroborated gaps found

#### Mechanism: `enum_overfit`

Total clusters in cell: **4** (5 distinct columns affected). Top-4 below cover 5 columns (100.0% of cell).

##### Rank #1 — `ecd834154222…` — 2 columns — action: `validator_widening`

- **Corroborating lenses**: **ydf** = `representation.discrete.categorical` (conf 0.53) · **cascade** = `enum_overfit` (conf 1.00)
- **Candidate spec slug**: _(none — to be assigned downstream)_

| file | column | sense | ydf | samples | dbpedia |
|---|---|---|---|---|---|
| `beats_per_minute/data.parquet` | `unit.symbol` | `representation.scientific.measurement_unit` | `representation.discrete.categorical` | `kg` `g` `lbs` | `` |
| `data_rate/data_4.parquet` | `unit.symbol` | `representation.scientific.measurement_unit` | `representation.discrete.categorical` | `kg` `g` `lbs` | `` |

##### Rank #2 — `211c7d391798…` — 1 columns — action: `validator_widening`

- **Corroborating lenses**: **ydf** = `representation.discrete.categorical` (conf 0.63) · **cascade** = `enum_overfit` (conf 1.00)
- **Candidate spec slug**: _(none — to be assigned downstream)_

| file | column | sense | ydf | samples | dbpedia |
|---|---|---|---|---|---|
| `show_time/client_globals_2.parquet` | `BooleanValue` | `representation.boolean.terms` | `representation.discrete.categorical` | `boolean` `FALSE` | `http://dbpedia.org/ontology/value` |

##### Rank #3 — `cad66a91096d…` — 1 columns — action: `validator_widening`

- **Corroborating lenses**: **ydf** = `representation.discrete.categorical` (conf 0.62) · **cascade** = `enum_overfit` (conf 1.00)
- **Candidate spec slug**: _(none — to be assigned downstream)_

| file | column | sense | ydf | samples | dbpedia |
|---|---|---|---|---|---|
| `time_interval/client_globals_26.parquet` | `BooleanValue` | `representation.boolean.terms` | `representation.discrete.categorical` | `boolean` `TRUE` | `http://dbpedia.org/ontology/value` |

##### Rank #4 — `cdf4571a30e4…` — 1 columns — action: `validator_widening`

- **Corroborating lenses**: **ydf** = `representation.discrete.categorical` (conf 0.68) · **cascade** = `enum_overfit` (conf 1.00)
- **Candidate spec slug**: _(none — to be assigned downstream)_

| file | column | sense | ydf | samples | dbpedia |
|---|---|---|---|---|---|
| `id/elora_unit_conversion_mapping.parquet` | `display_unit` | `representation.scientific.measurement_unit` | `representation.discrete.categorical` | `kg` `g` `dg` | `http://dbpedia.org/ontology/militaryUnitSize` |

#### Mechanism: `misclassification`

Total clusters in cell: **30203** (251335 distinct columns affected). Top-10 below cover 84920 columns (33.8% of cell).

##### Rank #1 — `721b890ea74d…` — 22952 columns — action: `training_data_addition`

- **Corroborating lenses**: **ydf** = `representation.discrete.categorical` (conf 0.98) · **cascade** = `misclassification` (conf 1.00)
- **Candidate spec slug**: _(none — to be assigned downstream)_

| file | column | sense | ydf | samples | dbpedia |
|---|---|---|---|---|---|
| `id/0021600721.parquet` | `START_POSITION` | `identity.person.gender_code` | `representation.discrete.categorical` | `F` `F` `C` | `http://dbpedia.org/ontology/start` |
| `id/0021600721_2.parquet` | `START_POSITION` | `identity.person.gender_code` | `representation.discrete.categorical` | `F` `F` `C` | `http://dbpedia.org/ontology/start` |
| `id/0021600721_3.parquet` | `START_POSITION` | `identity.person.gender_code` | `representation.discrete.categorical` | `F` `F` `C` | `http://dbpedia.org/ontology/start` |
| `id/0021600722_3_1.parquet` | `START_POSITION` | `identity.person.gender_code` | `representation.discrete.categorical` | `F` `F` `C` | `http://dbpedia.org/ontology/start` |
| `id/0021600723.parquet` | `START_POSITION` | `identity.person.gender_code` | `representation.discrete.categorical` | `F` `F` `C` | `http://dbpedia.org/ontology/start` |

##### Rank #2 — `1b858e0d073b…` — 21956 columns — action: `training_data_addition`

- **Corroborating lenses**: **ydf** = `representation.numeric.integer_number` (conf 0.73) · **cascade** = `misclassification` (conf 1.00)
- **Candidate spec slug**: _(none — to be assigned downstream)_

| file | column | sense | ydf | samples | dbpedia |
|---|---|---|---|---|---|
| `abstraction/00-01_28.parquet` | `Comments` | `datetime.offset.utc` | `representation.numeric.integer_number` | `0` `0` `0` | `http://dbpedia.org/ontology/comment` |
| `abstraction/00-01_36.parquet` | `Points` | `datetime.offset.utc` | `representation.numeric.integer_number` | `8` `0` `0` | `http://dbpedia.org/ontology/careerPoints` |
| `abstraction/00-01_41.parquet` | `Comments` | `datetime.offset.utc` | `representation.boolean.binary` | `0` `0` `1` | `http://dbpedia.org/ontology/comment` |
| `abstraction/00-01_51.parquet` | `Comments` | `datetime.offset.utc` | `representation.numeric.integer_number` | `0` `0` `0` | `http://dbpedia.org/ontology/comment` |
| `abstraction/00-01_52.parquet` | `Comments` | `datetime.offset.utc` | `representation.numeric.integer_number` | `0` `0` `0` | `http://dbpedia.org/ontology/comment` |

##### Rank #3 — `20803deffbad…` — 9779 columns — action: `training_data_addition`

- **Corroborating lenses**: **ydf** = `representation.numeric.integer_number` (conf 0.73) · **cascade** = `misclassification` (conf 1.00)
- **Candidate spec slug**: _(none — to be assigned downstream)_

| file | column | sense | ydf | samples | dbpedia |
|---|---|---|---|---|---|
| `seek_time/cassandra_B_features.parquet` | `F6-java.util.concurrent.ConcurrentLinkedQueue<java.lang.Long>` | `technology.internet.url` | `representation.numeric.integer_number` | `0` `0` `0` | `` |
| `seek_time/cassandra_B_features.parquet` | `F71-eevans@sym-link.com` | `technology.internet.url` | `representation.boolean.binary` | `0` `0` `1` | `` |
| `time_interval/synthesized_info.parquet` | `linked pc num` | `technology.internet.url` | `representation.numeric.integer_number` | `2` `1` `4` | `` |
| `abstraction/ir.model.access_8.parquet` | `perm_unlink` | `technology.internet.url` | `representation.numeric.integer_number` | `1` `1` `1` | `` |
| `time_slot/ir.model.access_2.parquet` | `perm_unlink` | `technology.internet.url` | `representation.boolean.binary` | `1` `0` `1` | `` |

##### Rank #4 — `81b63a52e3ef…` — 8649 columns — action: `training_data_addition`

- **Corroborating lenses**: **ydf** = `representation.numeric.integer_number` (conf 0.73) · **cascade** = `misclassification` (conf 1.00)
- **Candidate spec slug**: _(none — to be assigned downstream)_

| file | column | sense | ydf | samples | dbpedia |
|---|---|---|---|---|---|
| `time_interval/PDS4_PARTICLE_1E00_2000.parquet` | `Maximum Cardinality` | `representation.boolean.binary` | `representation.numeric.integer_number` | `1` `1` | `http://dbpedia.org/ontology/maximumTemperature` |
| `time_interval/PDS4_PARTICLE_1F00_2010_1.parquet` | `Maximum Cardinality` | `representation.boolean.binary` | `representation.numeric.integer_number` | `1` `1` | `http://dbpedia.org/ontology/maximumTemperature` |
| `time_interval/PDS4_PARTICLE_1G00_2010.parquet` | `Maximum Cardinality` | `representation.boolean.binary` | `representation.numeric.integer_number` | `1` `1` | `http://dbpedia.org/ontology/maximumTemperature` |
| `time_interval/PDS4_PARTICLE_1G00_2010_1.parquet` | `Maximum Cardinality` | `representation.boolean.binary` | `representation.numeric.integer_number` | `1` `1` | `http://dbpedia.org/ontology/maximumTemperature` |
| `time_interval/StructureDefinition-cdm-appointment.parquet` | `Base Path` | `representation.boolean.binary` | `representation.numeric.integer_number` | `0` `0` `0` | `http://dbpedia.org/ontology/routeLine` |

##### Rank #5 — `cdde5d05b73a…` — 5764 columns — action: `training_data_addition`

- **Corroborating lenses**: **ydf** = `representation.discrete.categorical` (conf 0.88) · **cascade** = `misclassification` (conf 1.00)
- **Candidate spec slug**: _(none — to be assigned downstream)_

| file | column | sense | ydf | samples | dbpedia |
|---|---|---|---|---|---|
| `abstraction/00-01_24.parquet` | `Type` | `datetime.component.periodicity` | `representation.discrete.categorical` | `comment` `comment` `comment` | `http://dbpedia.org/ontology/type` |
| `abstraction/02-03_25.parquet` | `Type` | `datetime.component.periodicity` | `representation.discrete.categorical` | `comment` `comment` `comment` | `http://dbpedia.org/ontology/type` |
| `abstraction/02-03_32.parquet` | `Type` | `datetime.component.periodicity` | `representation.discrete.categorical` | `comment` `comment` `comment` | `http://dbpedia.org/ontology/type` |
| `abstraction/03-04_27.parquet` | `Type` | `datetime.component.periodicity` | `representation.discrete.categorical` | `comment` `story` `comment` | `http://dbpedia.org/ontology/type` |
| `abstraction/03-04_43.parquet` | `Type` | `datetime.component.periodicity` | `representation.discrete.categorical` | `comment` `comment` `story` | `http://dbpedia.org/ontology/type` |

##### Rank #6 — `3f2aa8465552…` — 4835 columns — action: `training_data_addition`

- **Corroborating lenses**: **ydf** = `representation.discrete.categorical` (conf 0.96) · **cascade** = `misclassification` (conf 1.00)
- **Candidate spec slug**: _(none — to be assigned downstream)_

| file | column | sense | ydf | samples | dbpedia |
|---|---|---|---|---|---|
| `id/0021600721_2.parquet` | `TEAM_ABBREVIATION` | `representation.identifier.alphanumeric_id` | `representation.discrete.categorical` | `DET` `DET` `DET` | `http://dbpedia.org/ontology/teamName` |
| `id/0021600721_3.parquet` | `TEAM_ABBREVIATION` | `representation.identifier.alphanumeric_id` | `representation.discrete.categorical` | `DET` `DET` `DET` | `http://dbpedia.org/ontology/teamName` |
| `id/0021600722_3_1.parquet` | `TEAM_ABBREVIATION` | `representation.identifier.alphanumeric_id` | `representation.discrete.categorical` | `CLE` `CLE` `CLE` | `http://dbpedia.org/ontology/teamName` |
| `id/0021600723.parquet` | `TEAM_ABBREVIATION` | `representation.identifier.alphanumeric_id` | `representation.discrete.categorical` | `MEM` `MEM` `MEM` | `http://dbpedia.org/ontology/teamName` |
| `id/0021600723_3.parquet` | `TEAM_ABBREVIATION` | `representation.identifier.alphanumeric_id` | `representation.discrete.categorical` | `MEM` `MEM` `MEM` | `http://dbpedia.org/ontology/teamName` |

##### Rank #7 — `cc9a17251cae…` — 3457 columns — action: `training_data_addition`

- **Corroborating lenses**: **ydf** = `representation.numeric.integer_number` (conf 0.97) · **cascade** = `misclassification` (conf 1.00)
- **Candidate spec slug**: _(none — to be assigned downstream)_

| file | column | sense | ydf | samples | dbpedia |
|---|---|---|---|---|---|
| `abstraction/01-02_35.parquet` | `Points` | `datetime.offset.utc` | `representation.numeric.integer_number` | `0` `0` `28` | `http://dbpedia.org/ontology/careerPoints` |
| `abstraction/03-04_58.parquet` | `Comments` | `datetime.offset.utc` | `representation.numeric.integer_number` | `0` `0` `0` | `http://dbpedia.org/ontology/comment` |
| `abstraction/04-05_131.parquet` | `Points` | `datetime.offset.utc` | `representation.numeric.integer_number` | `1` `0` `9` | `http://dbpedia.org/ontology/careerPoints` |
| `abstraction/04-05_49.parquet` | `Comments` | `datetime.offset.utc` | `representation.numeric.integer_number` | `0` `0` `0` | `http://dbpedia.org/ontology/comment` |
| `abstraction/05-06_134.parquet` | `Comments` | `datetime.offset.utc` | `representation.numeric.integer_number` | `0` `16` `3` | `http://dbpedia.org/ontology/comment` |

##### Rank #8 — `7285286a0655…` — 2869 columns — action: `training_data_addition`

- **Corroborating lenses**: **ydf** = `representation.discrete.categorical` (conf 0.97) · **cascade** = `misclassification` (conf 1.00)
- **Candidate spec slug**: _(none — to be assigned downstream)_

| file | column | sense | ydf | samples | dbpedia |
|---|---|---|---|---|---|
| `lead_time/cenlab-index_1.parquet` | `nationality` | `identity.person.blood_type` | `representation.discrete.categorical` | `a` `b` `b` | `http://dbpedia.org/ontology/nationality` |
| `lead_time/cenlab-index_7.parquet` | `nationality` | `identity.person.blood_type` | `representation.discrete.categorical` | `a` `b` `b` | `http://dbpedia.org/ontology/nationality` |
| `parent/tokens_1.parquet` | `s` | `identity.person.blood_type` | `representation.discrete.categorical` | `s` `s` `s` | `` |
| `parent/tokens_11.parquet` | `Unnamed: 2` | `identity.person.blood_type` | `representation.discrete.categorical` | `s` `s` `s` | `` |
| `parent/tokens_18.parquet` | `s` | `identity.person.blood_type` | `representation.discrete.categorical` | `s` `s` `s` | `` |

##### Rank #9 — `08f6c4cbfefd…` — 2488 columns — action: `training_data_addition`

- **Corroborating lenses**: **ydf** = `representation.numeric.decimal_number` (conf 0.99) · **cascade** = `misclassification` (conf 1.00)
- **Candidate spec slug**: _(none — to be assigned downstream)_

| file | column | sense | ydf | samples | dbpedia |
|---|---|---|---|---|---|
| `time_interval/TEI-SQL-TABLE_1.parquet` | `ref_num` | `representation.numeric.integer_number` | `representation.numeric.decimal_number` | `1.0` `2.0` `3.0` | `http://dbpedia.org/ontology/gradNum` |
| `time_interval/TO_E1UdvSojKjM_comment.parquet` | `num_likes` | `representation.numeric.integer_number` | `representation.numeric.decimal_number` | `3.0` `2.0` `0.0` | `` |
| `time_interval/TO_UnTLIViVzTk_comment.parquet` | `num_likes` | `representation.numeric.integer_number` | `representation.numeric.decimal_number` | `0.0` `2.0` `0.0` | `` |
| `lead_time/JUG_UA_events_past.parquet` | `waitlist_count` | `representation.numeric.integer_number` | `representation.numeric.decimal_number` | `0.0` `0.0` `0.0` | `http://dbpedia.org/ontology/elevatorCount` |
| `lead_time/Loc-Overlapping-Synset-Lemma.parquet` | `Count_s_A` | `representation.numeric.integer_number` | `representation.numeric.decimal_number` | `1.0` `0.0` `1.0` | `http://dbpedia.org/ontology/aSide` |

##### Rank #10 — `9f5141836073…` — 2171 columns — action: `training_data_addition`

- **Corroborating lenses**: **ydf** = `representation.discrete.categorical` (conf 0.72) · **cascade** = `misclassification` (conf 1.00)
- **Candidate spec slug**: _(none — to be assigned downstream)_

| file | column | sense | ydf | samples | dbpedia |
|---|---|---|---|---|---|
| `lead_time/Michelle_Wander_1_reviews.parquet` | `unUsefulGrouping` | `datetime.component.day_of_week` | `representation.discrete.categorical` | `people` `people` `people` | `http://dbpedia.org/ontology/unNumber` |
| `lead_time/Michelle_Wander_1_reviews.parquet` | `usefulGrouping` | `datetime.component.day_of_week` | `representation.discrete.categorical` | `people` `people` `people` | `` |
| `lead_time/Sam_Slaven_1_reviews.parquet` | `unUsefulGrouping` | `datetime.component.day_of_week` | `representation.discrete.categorical` | `people` `people` `people` | `http://dbpedia.org/ontology/unNumber` |
| `lead_time/Sam_Slaven_1_reviews.parquet` | `usefulGrouping` | `datetime.component.day_of_week` | `representation.discrete.categorical` | `people` `people` `people` | `` |
| `lead_time/William_Hope_1_reviews.parquet` | `unUsefulGrouping` | `datetime.component.day_of_week` | `representation.discrete.categorical` | `people` `people` `people` | `http://dbpedia.org/ontology/unNumber` |

#### Mechanism: `validator_widening`

> **demoted** by ac-12 attestation (2026-05-23) — 381 clusters routed to `single_lens_signals.tsv`. See `spot_check_prescreen.md` for the per-gap reasoning and `progress.md` (2026-05-23 entry) for the demotion rationale.

#### Mechanism: `unknown_no_fit`

> no corroborated gaps found

#### Mechanism: `fallthrough`

> no corroborated gaps found


## Part 2 — Candidate taxonomy gaps from DBpedia coverage

DBpedia classes flagged as `no_finetype_equivalent` in the curated mapping table, surfaced when they appear in ≥10 columns AND those columns were predicted as trivial (`plain_text` / `decimal_number`) by Sense. These are candidate FineType taxonomy additions — real-world semantic patterns that DBpedia models but FineType doesn't. **Part 2 is independent of Part 1** — entries here are not corroborated by the lens stack.

| DBpedia class | columns affected | sample (file:column) |
|---|---:|---|
| `http://dbpedia.org/ontology/title` | 130419 | `00-01_10.parquet:Title`<br>`00-01_101.parquet:Title`<br>`00-01_11.parquet:Title` |
| `http://dbpedia.org/ontology/rating` | 81367 | `2009_saturn_vue_modified.parquet:rating_value`<br>`imdb-top-rated_5.parquet:IMDb Rating`<br>`top_1000_IMDB_movies.parquet:imdb_rating` |
| `http://dbpedia.org/ontology/populationPctMen` | 61422 | `recent-grads_10.parquet:Men`<br>`recent-grads_120.parquet:Men`<br>`recent-grads_142.parquet:Men` |
| `http://dbpedia.org/ontology/dfE` | 50899 | `2012_raw.parquet:nel e nel`<br>`hooks.parquet:e`<br>`hooks.parquet:g_li` |
| `http://dbpedia.org/ontology/sourceText` | 24158 | `Lec18_MIT6.00IntroductiontoComputerScienceandProgramming,Fall2008-QJ_MPc0TobI.en.parquet:text`<br>`TopicModel-2-1-document.parquet:Text`<br>`TopicModel-2-1-document_1.parquet:Text` |
| `http://dbpedia.org/ontology/ratio` | 14891 | `CWEGF.parquet:Current Ratio (mrq)`<br>`CWGL.parquet:Current Ratio (mrq)`<br>`CWNR.parquet:Current Ratio (mrq)` |
| `http://dbpedia.org/ontology/ra` | 14372 | `CombinedLog2021_02_22_16_46_33_423.parquet:TA`<br>`CombinedLog2021_04_04_10_57_40_539.parquet:TA`<br>`sunshine-daydream.parquet:Su` |
| `http://dbpedia.org/ontology/day` | 9447 | `GOOG.parquet:Day`<br>`CWEGF.parquet:fiftyDayAverage`<br>`CWEGF.parquet:fiftyDayAverageChange` |
| `http://dbpedia.org/ontology/model` | 8276 | `Edmund_M=-Clarke_2.parquet:introduction model checking`<br>`GWAGUIPropList_1.parquet:Model Default Unit`<br>`GWAGUIPropList_2.parquet:Model Default Unit` |
| `http://dbpedia.org/ontology/closeTo` | 7809 | `WU.parquet:LONDON (Reuters) - Western Union is hopeful it will be able to resume offering money transfer services to Cuba as it looks forward to policy steps beneficial to the firm under the U.S. Biden administration, its chief financial officer said on Tuesday.`<br>`S04E21_script.parquet:Almost time to wake up`<br>`NSL_1.parquet:Investors said markets were continuing to react to the Fed's meeting and Chairman Jerome Powell's press conference, as the central bank pledged to keep its foot on the gas despite an expected surge of inflation. read more` |
| `http://dbpedia.org/ontology/medlinePlus` | 7277 | `eplustbl_1.parquet:EnergyPlus`<br>`eplustbl_8.parquet:EnergyPlus`<br>`eplustbl_9.parquet:EnergyPlus` |
| `http://dbpedia.org/ontology/reference` | 7067 | `W11-2123_6.parquet:Reference Text`<br>`Questionnaire_July%2031,%202019_10.47.parquet:ExternalReference`<br>`A00-2018.annv3.parquet:Reference Text` |
| `http://dbpedia.org/ontology/description` | 6490 | `Designite_Abstraction_ImpSmells_1.parquet:Description`<br>`Designite_Reign.Audio.API_ImpSmells.parquet:Description`<br>`output_1.parquet:description` |
| `http://dbpedia.org/ontology/temperature` | 6384 | `20466.6TRAO.html_1.parquet:Temperature`<br>`Globtherm2_within_species_SO.parquet:pretreatment_temp`<br>`MCU_PCB_V2.parquet:OPERATING-TEMPERATURE` |
| `http://dbpedia.org/ontology/fileExtension` | 6248 | `Designite_01.%20Calculator%20(ASP.NET%20MVC)_DesignSmells.parquet:File`<br>`Designite_01.%20Mobile_DesignSmells.parquet:File`<br>`Designite_01.%20RealTimeChart_DesignSmells.parquet:File` |
| `http://dbpedia.org/ontology/elementGroup` | 6151 | `ir.model.access_782.parquet:group_id:id`<br>`ir.model.access_861.parquet:group_id:id`<br>`ir.model.access_862.parquet:group_id:id` |
| `http://dbpedia.org/ontology/speedLimit` | 5804 | `data_science-data.parquet:air_speed`<br>`data_science-data.parquet:wind_speed`<br>`data_science-data_1.parquet:air_speed` |
| `http://dbpedia.org/ontology/deathCause` | 4317 | `Designite_01.%20Calculator%20(ASP.NET%20MVC)_DesignSmells.parquet:Cause`<br>`Designite_01.%20Mobile_DesignSmells.parquet:Cause`<br>`Designite_01.%20RealTimeChart_DesignSmells.parquet:Cause` |
| `http://dbpedia.org/ontology/openAccessContent` | 4271 | `01-02.parquet:Content`<br>`01-02_94.parquet:Content`<br>`02-03_108.parquet:Content` |
| `http://dbpedia.org/ontology/fatalityRate` | 4146 | `answer_1_2_3.parquet:Attrition_rate`<br>`Anne_Arundel_County_Crime_Rate_By_Type.parquet:PROPERTY CRIME RATE`<br>`Anne_Arundel_County_Crime_Rate_By_Type.parquet:TOTAL CRIME RATE` |
| `http://dbpedia.org/ontology/max` | 4091 | `study_variablelist.parquet:Field max`<br>`CombinedLog2020_12_10__24_04_314.parquet:maxPing`<br>`CombinedLog2020_12_10__33_08_140.parquet:maxPing` |
| `http://dbpedia.org/ontology/other` | 3722 | `IntoValue2_extended_DC_CTgov_changes.parquet:other_comments`<br>`audit_format_map.parquet:other`<br>`structure_audit.parquet:other` |
| `http://dbpedia.org/ontology/min` | 3697 | `study_variablelist.parquet:Field min`<br>`CombinedLog2020_12_10__33_08_140.parquet:minPing`<br>`CombinedLog2020_12_10__46_09_314.parquet:minPing` |
| `http://dbpedia.org/ontology/dam` | 3687 | `skarbonka_7.parquet:dam`<br>`Configuration1_window10_split10_typetypetest_projectvelocity1.5.parquet:dam`<br>`Configuration1_window10_split11_typetest_projectvelocity1.5.parquet:dam` |
| `http://dbpedia.org/ontology/sentence` | 3554 | `text_11.parquet:Sentence`<br>`text_12.parquet:Sentence`<br>`text_13.parquet:Sentence` |
| `http://dbpedia.org/ontology/previousPopulationTotal` | 3317 | `CWEGF.parquet:totalCurrentAssets`<br>`CWEGF.parquet:totalCurrentLiabilities`<br>`CWGL.parquet:Total Cash Per Share (mrq)` |
| `http://dbpedia.org/ontology/sea` | 3245 | `example_187.parquet:dpm_sea`<br>`example_187.parquet:hs_sea`<br>`example_187.parquet:tp_sea` |
| `http://dbpedia.org/ontology/boilingPoint` | 3244 | `periodic-table-data.parquet:boiling_point`<br>`weather-2012-01-01_41.parquet:Dew PointF`<br>`weather-2012-01-06_39.parquet:Dew PointF` |
| `http://dbpedia.org/ontology/type` | 3169 | `nphys-volume08-issue01_1.parquet:contentType`<br>`chart_label_to_concept.parquet:measurement_type_concept_id`<br>`chart_label_to_concept_1.parquet:measurement_type_concept_id` |
| `http://dbpedia.org/ontology/equity` | 3109 | `2017-04-26_1.parquet:Arconic sheds Alcoa stake through debt-for-equity swap`<br>`CWEGF.parquet:Total Debt/Equity (mrq)`<br>`CWEGF.parquet:totalStockholderEquity` |
| `http://dbpedia.org/ontology/value` | 2960 | `study_variablelist.parquet:Field dependency value`<br>`PDS4_IMG_SURFACE_1D00_1210.parquet:Maximum Value`<br>`PDS4_IMG_SURFACE_1D00_1210.parquet:Minimum Value` |
| `http://dbpedia.org/ontology/passengersPerDay` | 2797 | `FeatureTable_ROM_VIC_Normalized_Female.parquet:COG_PER`<br>`CWEGF.parquet:fiftyDayAverageChangePercent`<br>`CWEGF.parquet:twoHundredDayAverageChangePercent` |
| `http://dbpedia.org/ontology/score` | 2744 | `AhmedBB.parquet:avgScore`<br>`AhmedBB.parquet:reviewScore`<br>`movies2017.parquet:IMDB_score` |
| `http://dbpedia.org/ontology/length` | 2638 | `prepared_data_1.parquet:Length.`<br>`prepared_data_2.parquet:Length.`<br>`DataDictionary.parquet:length` |
| `http://dbpedia.org/ontology/recordedIn` | 2588 | `name_13.parquet:published_in_id`<br>`2phase_energy_out.parquet:Es_in`<br>`2phase_energy_out.parquet:Ts_in` |
| `http://dbpedia.org/ontology/stockExchange` | 2570 | `2017-04-26_1.parquet:BRIEF-Arconic announces debt-for-equity exchange for Alcoa Corp common stock`<br>`CWEGF.parquet:commonStock`<br>`CWEGF.parquet:repurchaseOfStock` |
| `http://dbpedia.org/ontology/operatingIncome` | 2490 | `CWEGF.parquet:Operating Cash Flow (ttm)`<br>`CWEGF.parquet:totalCashFromOperatingActivities`<br>`CWEGF.parquet:totalCashflowsFromInvestingActivities` |
| `http://dbpedia.org/ontology/longName` | 2395 | `pi_project.parquet:long_title`<br>`C08.parquet:InstrumentLongName_en`<br>`CWEGF.parquet:longBusinessSummary` |
| `http://dbpedia.org/ontology/vaporPressure` | 2351 | `NORDIC_SENSOR_HUB_PRESSURE_1.parquet:pressure_hpa`<br>`NORDIC_SENSOR_HUB_PRESSURE_2.parquet:pressure_hpa`<br>`data_science-data.parquet:differential_pressure` |
| `http://dbpedia.org/ontology/assets` | 2336 | `CWEGF.parquet:totalAssets`<br>`CWGL.parquet:investments`<br>`CWNR.parquet:totalAssets` |
| `http://dbpedia.org/ontology/netIncome` | 2286 | `CWEGF.parquet:netBorrowings`<br>`CWEGF.parquet:netReceivables`<br>`CWGL.parquet:netBorrowings` |
| `http://dbpedia.org/ontology/width` | 2128 | `arts_r3_properties.parquet:width_ms`<br>`sourcezone_parameters.parquet:approx_unit_source_width`<br>`si2017_si.parquet:cel width (μm)` |
| `http://dbpedia.org/ontology/lyrics` | 2095 | `Captain%20Insano%20Lyrics.parquet:Lyrics`<br>`Ex%20Lyrics.parquet:Lyrics`<br>`Hourglass%20(Prod.%20by%20MetaMorph)%20Lyrics.parquet:Lyrics` |
| `http://dbpedia.org/ontology/odor` | 2060 | `Designite_8.%20LazyInitialization_DesignSmells.parquet:Design smell`<br>`Designite_AdalDesktopTestApp_DesignSmells.parquet:Design smell`<br>`Designite_Alarm%20clock_DesignSmells.parquet:Design smell` |
| `http://dbpedia.org/ontology/meanRadius` | 2059 | `item_id_stat.parquet:mean`<br>`level2_stat.parquet:mean`<br>`summary.throughput.parquet:mean` |
| `http://dbpedia.org/ontology/comment` | 1935 | `T1_mT4GJ97JLg8_comment.parquet:comment`<br>`T3_36OzeRwP-SU_comment.parquet:comment`<br>`TO_5031-rmWWgs_comment.parquet:comment` |
| `http://dbpedia.org/ontology/worldOpen` | 1842 | `CWEGF.parquet:regularMarketOpen`<br>`CWGL.parquet:regularMarketOpen`<br>`CWNR.parquet:regularMarketOpen` |
| `http://dbpedia.org/ontology/ableToGrind` | 1796 | `2009_saturn_vue_modified.parquet:Fun_To_Drive`<br>`al_hazan1511.parquet:Don't you know what it means to me to be a Marine, Dad? Ever...`<br>`katexoxoxo.parquet:We began to recognize in them a strange obsession. After all...` |
| `http://dbpedia.org/ontology/visitorsPercentageChange` | 1493 | `Beach%20Point%20Capital%20Management.parquet:% Change`<br>`CWEGF.parquet:regularMarketChangePercent`<br>`CWGL.parquet:regularMarketChangePercent` |
| `http://dbpedia.org/ontology/numberOfUseOfProperty` | 1472 | `housing_data.parquet:Proportion of non-retail business acres`<br>`BAX.parquet:Details on other terms of the pact with Moderna were not disclosed.`<br>`Beach%20Point%20Capital%20Management.parquet:% of Portfolio` |
| `http://dbpedia.org/ontology/previousWork` | 1412 | `CWEGF.parquet:regularMarketPreviousClose`<br>`CWGL.parquet:regularMarketPreviousClose`<br>`CWNR.parquet:regularMarketPreviousClose` |
| `http://dbpedia.org/ontology/otherActivity` | 1377 | `CWEGF.parquet:otherCashflowsFromFinancingActivities`<br>`CWEGF.parquet:otherCashflowsFromInvestingActivities`<br>`CWSFF.parquet:otherCashflowsFromFinancingActivities` |
| `http://dbpedia.org/ontology/keyPerson` | 1362 | `PDS4_IMG_SURFACE_1D00_1210.parquet:Sort Key`<br>`20667.2.html.parquet:Key`<br>`20667.2.html_1.parquet:Key` |
| `http://dbpedia.org/ontology/number` | 1344 | `scrapedData.parquet:commentsNumber`<br>`A00-2018.annv3.parquet:Citance Number`<br>`A00-2018.annv3_10.parquet:Citance Number` |
| `http://dbpedia.org/ontology/signName` | 1331 | `account.tax.template_17.parquet:base_sign`<br>`account.tax.template_17.parquet:ref_base_sign`<br>`account.tax.template_17.parquet:ref_tax_sign` |
| `http://dbpedia.org/ontology/totalCargo` | 1326 | `CWEGF.parquet:totalLiab`<br>`CWNR.parquet:totalLiab`<br>`CWVLF.parquet:totalLiab` |
| `http://dbpedia.org/ontology/book` | 1309 | `CWEGF.parquet:bookValue`<br>`CWGL.parquet:bookValue`<br>`CWNR.parquet:bookValue` |
| `http://dbpedia.org/ontology/averageAnnualGeneration` | 1302 | `CWVLF.parquet:trailingAnnualDividendRate`<br>`CWVLF.parquet:trailingAnnualDividendYield`<br>`CWXZF.parquet:trailingAnnualDividendRate` |
| `http://dbpedia.org/ontology/scale` | 1282 | `Michael-Stonebraker.parquet:kyrix interactive pan/zoom visualization scale`<br>`1601391354_Linux_runs.parquet:process_parallel__pipeline_numeric__scale_column_wise__quantile_range`<br>`1601391356_Linux_runs.parquet:process_parallel__pipeline_numeric__scale_column_wise__quantile_range` |
| `http://dbpedia.org/ontology/notes` | 1265 | `pubs.parquet:Notes`<br>`Extended_Data_Table_5.parquet:Notes`<br>`07b5853ca41b79766229b16ad8d43b59b7dcf0f6bdde3578c3ba8325bf165d37_8.parquet:Notes` |
| `http://dbpedia.org/ontology/frequency` | 1236 | `frequency_6.parquet:Frequency`<br>`joy_words.parquet:frequency`<br>`male_10.parquet:Word Frequency` |
| `http://dbpedia.org/ontology/minTime` | 1233 | `faucets_1.parquet:interval_minutes`<br>`Generators_1.parquet:Min Down Time (h)`<br>`Generators_1.parquet:Min Up Time (h)` |
| `http://dbpedia.org/ontology/isPartOf` | 1128 | `ace0.parquet:Imagine a vast sheet of paper on which straight Lines, Trian...`<br>`ACS_UCL.parquet:A: How is the data item collected`<br>`meanings_1.parquet:Hyperesthesia is a condition that involves an abnormal increase in sensitivity to stimuli of the sense.` |
| `http://dbpedia.org/ontology/quote` | 1095 | `CWEGF.parquet:quoteSourceName`<br>`CWGL.parquet:quoteSourceName`<br>`CWNR.parquet:quoteSourceName` |
| `http://dbpedia.org/ontology/operatingSystem` | 1079 | `srds87.parquet:Reliability Issues in Distributed Operating Systems.`<br>`CWEGF.parquet:changeToOperatingActivities`<br>`CWGL.parquet:changeToOperatingActivities` |
| `http://dbpedia.org/ontology/orderInOffice` | 1019 | `CombinedLog2021_02_22_16_46_33_423.parquet:receiverOutOfOrder`<br>`CWGL.parquet:changeInCash`<br>`CWNR.parquet:changeInCash` |
| `http://dbpedia.org/ontology/meltingPoint` | 1017 | `data_science-data.parquet:dew_point`<br>`data_science-data_1.parquet:dew_point`<br>`periodic-table-data.parquet:melting_point` |
| `http://dbpedia.org/ontology/equipment` | 979 | `CWEGF.parquet:propertyPlantEquipment`<br>`CWVLF.parquet:propertyPlantEquipment`<br>`CX.parquet:propertyPlantEquipment` |
| `http://dbpedia.org/ontology/status` | 968 | `07b5853ca41b79766229b16ad8d43b59b7dcf0f6bdde3578c3ba8325bf165d37_8.parquet:OwnershipStatus`<br>`79cdc3e1c43097bd770fcf43fbd54109178ef6c6e8ed0aa7903914919bb3094e_1.parquet:OwnershipStatus`<br>`79cdc3e1c43097bd770fcf43fbd54109178ef6c6e8ed0aa7903914919bb3094e_6.parquet:OwnershipStatus` |
| `http://dbpedia.org/ontology/volume` | 926 | `nphys-volume08-issue01_1.parquet:volume`<br>`nnano-volume14-issue05_1.parquet:volume`<br>`lsa-volume06-issue12.parquet:volume` |
| `http://dbpedia.org/ontology/notSolubleIn` | 909 | `csfw1998.parquet:Weakly Secret Bit Commitment: Applications to Lotteries and Fair Exchange.`<br>`list_actions_time.parquet:Miniclip NOT Visible`<br>`more%20riddles.parquet:With thieves I consort, With the Vilest, in short, I'm quite at ease in depravity, Yet all divines use me, And savants can't lose me, For I am the century of gravity.` |
| `http://dbpedia.org/ontology/minimumTemperature` | 868 | `PDS4_IMG_SURFACE_1D00_1210.parquet:Minimum Cardinality`<br>`nyc-listings.parquet:minimum_nights`<br>`nyc-listings_2.parquet:minimum_nights` |
| `http://dbpedia.org/ontology/depth` | 862 | `Configurations.parquet:A_Depth`<br>`configurations_16.parquet:regressor:adaboost:max_depth`<br>`configurations_16.parquet:regressor:mlp:hidden_layer_depth` |
| `http://dbpedia.org/ontology/mostDownPoint` | 828 | `WU.parquet:“We’re hopeful that some of the policies and things that they’ve talked about come into play because those things may be beneficial to Western Union.”`<br>`cv_20190612_bermuda_test.parquet:you want the moon?`<br>`0413351b-ef9e-444a-a7e2-87ef904a0d50_tags.parquet:small school\willing\top notch\communicative\approachable\available\spacious\many other\clean\good reason\good enough reason\independent study\good way\good possible way` |
| `http://dbpedia.org/ontology/bSide` | 804 | `top_2010s.parquet:dB`<br>`boston_77.parquet:B`<br>`boston_2.parquet:B` |
| `http://dbpedia.org/ontology/areaCode` | 800 | `account.tax.template_17.parquet:base_code_id:id`<br>`account.tax.template_17.parquet:ref_base_code_id:id`<br>`account.tax.template_2313.parquet:base_code_id:id` |
| `http://dbpedia.org/ontology/atPage` | 770 | `nphys-volume08-issue01_1.parquet:publishedAt`<br>`nnano-volume14-issue05_1.parquet:publishedAt`<br>`lsa-volume06-issue12.parquet:publishedAt` |
| `http://dbpedia.org/ontology/populationTotal` | 740 | `imp_909_predict_0203-1.py.parquet:total`<br>`imp_910_predict_0204-1.py.parquet:total`<br>`imp_919_predict_0215-3.py.parquet:total` |
| `http://dbpedia.org/ontology/isPartOfAnatomicalStructure` | 711 | `Robin-Milner_1.parquet:An inductive characterization of matching in binding bigraphs.`<br>`ccs2013.parquet:A security framework for the analysis and design of software attestation.`<br>`iclr2018.xml.parquet:Learning One-hidden-layer Neural Networks with Landscape Design.` |
| `http://dbpedia.org/ontology/definition` | 693 | `blueboxes_1.parquet:definition`<br>`cognitiveChallenges.parquet:definition`<br>`Data%20Dictionary%20-%20external.parquet:Definition` |
| `http://dbpedia.org/ontology/isPartOfName` | 671 | `codemyname.parquet:She may be the face I can't forget. The trace of pleasure or...`<br>`ljrobispado.parquet:Adios rock band that we loved the most, this is a toast to w...`<br>`WU.parquet:U.S. President-elect Joe Biden has promised to roll back some sanctions on remittances, although in a move that could complicate any efforts by Biden to revive ties with Havana, the Trump administration last week announced it was returning Cuba to the U.S. list of state sponsors of terrorism.` |
| `http://dbpedia.org/ontology/numberOfPersonBornInPlace` | 668 | `part-00002-138dbd91-623d-400c-a199-4a93a7d20304.parquet:First Ranger, you have command of Castle Black`<br>`%E5%A4%96%E7%A0%94%E7%89%88%E9%AB%98%E4%B8%AD%E8%8B%B1%E8%AF%AD%20%E9%80%89%E4%BF%AE11.parquet:n a person who settles in a new colony or moves into new country`<br>`MSNBC.201807_1.parquet:someone who believes in climate change and takes it seriously for the benefit of all of us including our children. so, i would urge you to resign before your scandals push you out. i want to bring into the conversation emily holden, politico's climate change and` |
| `http://dbpedia.org/ontology/ndlId` | 666 | `TableS5.parquet:ord_gc`<br>`6_1527.parquet:Bwd Pkt Len Std`<br>`6_1527.parquet:Pkt Len Var` |
| `http://dbpedia.org/ontology/drugbank` | 623 | `configurations_16.parquet:regressor:libsvm_svr:degree`<br>`configurations_16.parquet:regressor:libsvm_svr:gamma`<br>`configurations_46.parquet:regressor:libsvm_svr:degree` |
| `http://dbpedia.org/ontology/loadLimit` | 587 | `MCU_PCB_V2.parquet:LOAD_CAPACITANCE`<br>`data_science-data.parquet:heating_load`<br>`data_science-data.parquet:load_profile` |
| `http://dbpedia.org/ontology/version` | 578 | `train_filtrado.parquet:Census_FirmwareVersionIdentifier`<br>`eplustbl_1.parquet:Program Version:`<br>`ESDL_metadata_variables.parquet:orig_version` |
| `http://dbpedia.org/ontology/detectionMethod` | 569 | `Designite_Helios.FsCheck.Tests_ImpSmells_1.parquet:Method`<br>`Designite_Metrics.Tests_MethodMetrics.parquet:Method`<br>`Designite_Abot.Tests.Unit_MethodMetrics.parquet:Method` |
| `http://dbpedia.org/ontology/source` | 569 | `CPRD_Test_Table_Usagi.parquet:sourceFrequency`<br>`ksu_news%20January%2029,%202020.parquet:Source`<br>`ksu_news%20January%2029,%202020_2.parquet:Source` |
| `http://dbpedia.org/ontology/affiliation` | 559 | `nphys-volume08-issue01_1.parquet:affiliations`<br>`nnano-volume14-issue05_1.parquet:affiliations`<br>`lsa-volume06-issue12.parquet:affiliations` |
| `http://dbpedia.org/ontology/idNumber` | 534 | `manifest.parquet:unique_data_id`<br>`8_1750.parquet:internal_chunk_id`<br>`999_4.parquet:internal_chunk_id` |
| `http://dbpedia.org/ontology/carbohydrate` | 526 | `sugar-black-rose.parquet:Sugar`<br>`LactateProducers.parquet:Sugars`<br>`sugar-black-rose.parquet:Sugar` |
| `http://dbpedia.org/ontology/giniCoefficientAsOf` | 514 | `data_science-data.parquet:coefficient_of_variation`<br>`data_science-data_1.parquet:coefficient_of_variation`<br>`bulk-moisture-density%20(15).parquet:Bottom Depth DSF, MSF, WSF and CSF-A [m]` |
| `http://dbpedia.org/ontology/programmingLanguage` | 510 | `Leslie-Lamport.parquet:youre writing program dont use programming language`<br>`ML_Projects.parquet:Technologies/Languages`<br>`books.essential_1.parquet:The C Programming Language` |
| `http://dbpedia.org/ontology/maxTime` | 497 | `hay_capture-01.kismet.parquet:MaxSeenRate`<br>`hay_capture-02.kismet.parquet:MaxSeenRate`<br>`hay_capture-07.kismet.parquet:MaxSeenRate` |
| `http://dbpedia.org/ontology/work` | 496 | `141c0c29-fe93-4533-93f0-a15ae1f0f700_tags.parquet:overall learning experience\pretty fair\whenever available\too much work\much online work\able\full time job`<br>`StandardMetropolitanAreasData_train_data.parquet:work_force`<br>`RP3_2016-03-27_09-23-29.parquet:work_per_pulse` |
| `http://dbpedia.org/ontology/publication` | 492 | `field_level_data_Alice_Classen_Coffea_arabica_Tanzania_2011_2012.parquet:Publication`<br>`TSCRP.parquet:Publication`<br>`TSCRP.parquet:Publication` |
| `http://dbpedia.org/ontology/bloodType` | 489 | `ReinforceParamProtector.parquet:resistBloodRate`<br>`001_storage-sig.parquet:blood_pulse_wave`<br>`10011_episode1_timeseries.parquet:Mean blood pressure` |
| `http://dbpedia.org/ontology/map` | 479 | `StructureDefinition-VA.MHV.medication.parquet:Mapping: RIM Mapping`<br>`review_69353_extracted_data_csv_20201227072740.parquet:Baseline map_sd_sr`<br>`2021-03-31_NCM3722_glucose_growth_params.parquet:map_val` |
| `http://dbpedia.org/ontology/filename` | 474 | `output_1.parquet:filename`<br>`categories_1.parquet:FILENAME`<br>`manifest.parquet:filepath` |
| `http://dbpedia.org/ontology/bnfId` | 472 | `ThemisIda.parquet:RX MODBUS ID`<br>`Themisv1.parquet:RX MODBUS ID`<br>`arts_r3_properties.parquet:freq_id_low` |
| `http://dbpedia.org/ontology/component` | 468 | `Reqcycle_2016-01-01_2020-12-01_6.parquet:Component`<br>`density_pure.parquet:N Components`<br>`density_pure_10.parquet:N Components` |
| `http://dbpedia.org/ontology/totalMass` | 457 | `l36%20(Kestrel%20Cruiser%20C%20-%20NORMAL%20AE).parquet:TOTAL SCRAP COLLECTED`<br>`ACsampleselect.parquet:Total.Carbon`<br>`Genomic-GC-Manifest-Workflow-Test-1.parquet:Total Concentration (ng/ul)` |
| `http://dbpedia.org/ontology/free` | 454 | `CWEGF.parquet:Levered Free Cash Flow (ttm)`<br>`CWGL.parquet:Levered Free Cash Flow (ttm)`<br>`CWSFF.parquet:Levered Free Cash Flow (ttm)` |
| `http://dbpedia.org/ontology/hasKMLData` | 446 | `indicators.en.parquet:data`<br>`2019GDP.parquet:data`<br>`FieldDefinitions.parquet:Data` |
| `http://dbpedia.org/ontology/utcOffset` | 445 | `PHiLIP_map_t_1_0_2.parquet:bit_offset`<br>`PHiLIP_map_t_1_2_0.parquet:bit_offset`<br>`PHiLIP_map_t_1_0_1.parquet:bit_offset` |
| `http://dbpedia.org/ontology/convictionPenalty` | 443 | `mdl_question_10.parquet:penalty`<br>`mdl_question_100.parquet:penalty`<br>`mdl_question_102.parquet:penalty` |
| `http://dbpedia.org/ontology/birthSign` | 442 | `account.tax.template_17.parquet:tax_sign`<br>`account.tax.template_3779.parquet:tax_sign`<br>`account.tax.template_3785.parquet:tax_sign` |
| `http://dbpedia.org/ontology/coastLine` | 438 | `END_RESULT_DATAFRAME_9_pypownet_240.parquet:Worsened line`<br>`END_RESULT_DATAFRAME_G2OP_CASE14_REALISTIC.parquet:Worsened line`<br>`END_RESULT_DATAFRAME_for_test_with_line_9_cut_withoutthermallimits.parquet:Worsened line` |
| `http://dbpedia.org/ontology/hasAnnotation` | 431 | `faucets_1.parquet:has_ref_program`<br>`iot_hue_lightoff.parquet:answer_annotation`<br>`copo_schema.parquet:dc.relation HasMetadata` |
| `http://dbpedia.org/ontology/numberOfResourceWithType` | 430 | `infocom2002.parquet:Simple models of network access, with applications to the design of joint rate and admission control.`<br>`trainData.parquet:In some situations, third parties' terms may apply to your use of GitHub. For example, you may be a member of an organization on GitHub with its own terms or license agreements; you may download an application that integrates with GitHub; or you may use GitHub to authenticate to another service. Please be aware that while these Terms are our full agreement with you, other parties' terms govern their relationships with you.`<br>`tdsc8.parquet:A Distributed Algorithm for Finding All Best Swap Edges of a Minimum-Diameter Spanning Tree.` |
| `http://dbpedia.org/ontology/case` | 408 | `StateData.parquet:Averages Cases Since First Case`<br>`2017-06-23.parquet:BRIEF-Parkervision says German court decision in Parkervision V. Apple infringement case awaits validity ruling`<br>`accuracyTestCases_1.parquet:test_case_description` |
| `http://dbpedia.org/ontology/power` | 401 | `UDS_3.parquet:power`<br>`UDS_2.parquet:power`<br>`UDS_4.parquet:power` |
| `http://dbpedia.org/ontology/gross` | 393 | `movies2017.parquet:gross`<br>`top_1000_IMDB_movies.parquet:gross_earning`<br>`CWVLF.parquet:Gross Profit (ttm)` |
| `http://dbpedia.org/ontology/aSide` | 389 | `20170206_articles.parquet:Let’s plant a garden with eco-friendly pens`<br>`Lund_content_word_counting_NN.parquet:a`<br>`20170206_articles.parquet:Let’s plant a garden with eco-friendly pens` |
| `http://dbpedia.org/ontology/lowest` | 387 | `input_3.parquet:low`<br>`data_3005.parquet:low`<br>`data_3055.parquet:low` |
| `http://dbpedia.org/ontology/highest` | 383 | `input_3.parquet:high`<br>`data_3005.parquet:high`<br>`data_3055.parquet:high` |
| `http://dbpedia.org/ontology/routeDirection` | 378 | `GAMUTMeasureInformat_DATA.parquet:direction_higher`<br>`StructureDefinition-VA.MHV.medication.parquet:Path`<br>`data_science-data.parquet:wind_direction` |
| `http://dbpedia.org/ontology/review` | 373 | `nyc-listings.parquet:review_scores_accuracy`<br>`nyc-listings.parquet:review_scores_checkin`<br>`nyc-listings.parquet:review_scores_cleanliness` |
| `http://dbpedia.org/ontology/ethnicGroup` | 360 | `Big-Data-Developers-in-Austin_events_past.parquet:group_lat`<br>`Big-Data-Developers-in-Austin_events_past.parquet:group_lon`<br>`Big-Data-Developers-in-Miami_events_past.parquet:group_lat` |
| `http://dbpedia.org/ontology/starRating` | 360 | `04_8.parquet:Stars`<br>`05.parquet:Stars`<br>`final%20data%20with%20predictions.parquet:starTeffKelvin` |
| `http://dbpedia.org/ontology/locusSupplementaryData` | 349 | `ACS_UCL.parquet:Data Dictionary Element`<br>`train_filtrado.parquet:Census_InternalPrimaryDisplayResolutionHorizontal`<br>`ACS_UCL.parquet:Data Dictionary Element` |
| `http://dbpedia.org/ontology/computingInput` | 335 | `MCU_PCB_V2.parquet:INPUT-VOLTAGE`<br>`Sequestration_Rates_4.parquet:Raw Data Input`<br>`10-10.parquet:Ikaros Input` |
| `http://dbpedia.org/ontology/supply` | 335 | `InstrumentModel.parquet:im/power_supply_voltage_minimum`<br>`MCU_PCB_V2.parquet:OPERATING-SUPPLY-VOLTAGE`<br>`MCU_PCB_V2.parquet:OPERATING_SUPPLY_VOLTAGE` |
| `http://dbpedia.org/ontology/origin` | 330 | `periodic-table-data.parquet:name_origin`<br>`account_invoice_121.parquet:origin`<br>`many_vulns_new_format_10.parquet:Component origin id` |
| `http://dbpedia.org/ontology/maximumTemperature` | 327 | `nyc-listings.parquet:maximum_nights`<br>`nyc-listings_2.parquet:maximum_nights`<br>`nyc-listings_new.parquet:maximum_nights` |
| `http://dbpedia.org/ontology/mapDescription` | 325 | `ReporterDescriptionDescriptor_2.parquet:ReporterDescriptionMap`<br>`aranet2-aragwas-MERGED-AMW-v2_091319_nodeTable.parquet:MapMan_description`<br>`aranet2-aragwas-MERGED-AMW-v2_091319_nodeTable_1.parquet:MapMan_descr` |
| `http://dbpedia.org/ontology/setupTime` | 323 | `data-1370-1375.parquet:A tool to quickly mock out end points, setup delays and more...`<br>`1601391354_Linux_runs.parquet:std_fit_time`<br>`funcs_by_name_122.parquet:_GetSystemTime` |
| `http://dbpedia.org/ontology/signature` | 323 | `classica_df.parquet:time_signature`<br>`BennyCarter_It'sAWonderfulWorld-2_Solo.parquet:signature`<br>`BennyCarter_LongAgoAndFarAway-2_Solo.parquet:signature` |
| `http://dbpedia.org/ontology/overallRecord` | 321 | `Ashley_Spies_1_reviews.parquet:rOverall`<br>`Claire_Crowston_1_reviews.parquet:rOverall`<br>`Daniel_Arena.parquet:rOverall` |
| `http://dbpedia.org/ontology/tag` | 315 | `dev.parquet:tag`<br>`dev_5.parquet:tag`<br>`2020-02-19-2020-02-19-firefox-creator-answers-desktop-all-locales.parquet:tags` |
| `http://dbpedia.org/ontology/language` | 312 | `human_flow.parquet:In beautiful visual language`<br>`human_flow.parquet:In beautiful visual language`<br>`csd-contribution-1110.parquet:Language_glottocode` |
| `http://dbpedia.org/ontology/canBaggageChecked` | 305 | `post_questionnaire.parquet:If you had any trouble, could you describe what happened?`<br>`post_questionnaire.parquet:If you had any troubles, could you please describe what happened?`<br>`Npc_Road_020.parquet:What have you got?` |
| `http://dbpedia.org/ontology/college` | 300 | `894dea03-9fed-4e0a-9af8-0b0c10267008_tags.parquet:online learning\other college`<br>`Calculus%20and%20Analytic%20Geometry%20I_3.parquet:College Algebra`<br>`Introduction%20to%20Microprocessors_1.parquet:College Algebra` |
| `http://dbpedia.org/ontology/davisCup` | 298 | `wc-20140609-140000_10.parquet:cup`<br>`wc-20140609-140000_5.parquet:cup`<br>`wc-20140609-140000_7.parquet:cup` |
| `http://dbpedia.org/ontology/firstWin` | 292 | `wc-20140609-140000_10.parquet:win_group`<br>`wc-20140609-140000_5.parquet:win_group`<br>`wc-20140609-140000_7.parquet:win_group` |
| `http://dbpedia.org/ontology/superbowlWin` | 291 | `wc-20140609-140000_10.parquet:win`<br>`wc-20140609-140000_5.parquet:win`<br>`wc-20140609-140000_7.parquet:win` |
| `http://dbpedia.org/ontology/numberOfDeaths` | 285 | `totalsyear.parquet:All_Overdose_deaths`<br>`2020-04-27_2020-05-02_us_errs.parquet:actual_addl_deaths`<br>`2020-05-04_2020-05-09_us_errs.parquet:actual_addl_deaths` |
| `http://dbpedia.org/ontology/numberOfRooms` | 278 | `covid-19-nursing-home-dataset-week-ending-2020-11-08-NM-2020-11-08.parquet:Total Number of Occupied Beds`<br>`covid-19-nursing-home-dataset-week-ending-2020-11-15-NM-2020-05-31.parquet:Total Number of Occupied Beds`<br>`covid-19-nursing-home-dataset-week-ending-2020-11-15-NM-2020-06-14.parquet:Total Number of Occupied Beds` |
| `http://dbpedia.org/ontology/zodiacSign` | 276 | `account.tax.code.template_128.parquet:sign`<br>`account.tax.code.template_129.parquet:sign`<br>`account.tax.code.template_133.parquet:sign` |
| `http://dbpedia.org/ontology/draft` | 275 | `projections_2020-09-14_2020-09-19.parquet:PSI-DRAFT`<br>`projections_2020-09-14_2020-09-19.parquet:error-PSI-DRAFT`<br>`projections_2020-09-14_2020-09-26.parquet:PSI-DRAFT` |
| `http://dbpedia.org/ontology/access` | 273 | `DataDictionary.parquet:fieldPublicAccessLevel`<br>`EDITEDpermissionsDesc.parquet:android.permission.ACCESS_ALL_DOWNLOADS`<br>`scopus(1)_1.parquet:Access Type` |
| `http://dbpedia.org/ontology/range` | 272 | `metrics_A_BP.parquet:range`<br>`metrics_A_CacheA.parquet:range`<br>`metrics_A_CacheBS.parquet:range` |
| `http://dbpedia.org/ontology/colorChart` | 267 | `account.account.template_25.parquet:chart_template_id:id`<br>`account.account.template_269.parquet:chart_template_id:id`<br>`account.account.template_381.parquet:chart_template_id:id` |
| `http://dbpedia.org/ontology/series` | 266 | `CountryData.IND.parquet:Series`<br>`copo_schema.parquet:series`<br>`projections_2020-09-14_2020-09-19.parquet:CMU-TimeSeries` |
| `http://dbpedia.org/ontology/result` | 265 | `AllData_LGBM_GS.parquet:result`<br>`AllData_v2_LGBM_GS.parquet:result`<br>`AllData_v4_AddFeat_LGBM_GS.parquet:result` |
| `http://dbpedia.org/ontology/alias` | 263 | `StructureDefinition-VA.MHV.medication.parquet:Alias(s)`<br>`StructureDefinition-ClinicalDischargeType.parquet:Alias(s)`<br>`StructureDefinition-ConsultationsAmountKlinic.parquet:Alias(s)` |
| `http://dbpedia.org/ontology/registration` | 258 | `2018-11-17-events_111.parquet:registration`<br>`2018-11-17-events_139.parquet:registration`<br>`2018-11-17-events_171.parquet:registration` |
| `http://dbpedia.org/ontology/order` | 256 | `codelists.parquet:Order`<br>`20210409-hipster-shop-sl-agg.parquet:Average order`<br>`Soils_data.parquet:Variable_Order` |
| `http://dbpedia.org/ontology/maxApparentMagnitude` | 254 | `Parameters_Global_V2_20200718.parquet:Jump Magnitude`<br>`2020-09-08.parquet:apparentTemperatureMax`<br>`2020-09-08_1.parquet:apparentTemperatureMax` |
| `http://dbpedia.org/ontology/reportingMark` | 254 | `LocalCrimeJurisbyJuris_6.parquet:Reporting`<br>`LocalCrimeJurisbyJuris_7.parquet:Reporting`<br>`covid-19-nursing-home-dataset-week-ending-2020-11-22-NM-2020-08-16.parquet:Reporting Interval` |
| `http://dbpedia.org/ontology/numberOfClassesWithResource` | 249 | `8551db0a-9973-4b7f-bbb7-26f437e41b76_text.parquet:Excellent gorgeous campus with staff that is willing to help any students and their needs. Great counselors and many extra curricular activities and groups you can join`<br>`LEN_3.parquet:(Reuters) - Lennar Corp said on Wednesday it plans to spin-off all or parts of its non-core businesses to become a pure-play homebuilder and financial services company, while also creating a joint venture to provide single family homes for rent.`<br>`9c167073-7445-4d86-8364-0700d2ab0a4f_tags.parquet:many instructor\use pre\pre - lecture\- recorded lecture\online office hour\many student\other country\use asynchronous lecture\want flexible schedule` |
| `http://dbpedia.org/ontology/commandModule` | 247 | `billing_apps_merge.parquet:.iso.org.dod.internet.private.enterprises.roamware.csvProcessor-mob.csvProcessorTrapsModule.csvProcessorTrapsGroup.csvProcessorSysUp`<br>`X-Keys_SF-260D.parquet:Command Logic Test`<br>`X-Keys_TBM900_1.parquet:Command Logic Test` |
| `http://dbpedia.org/ontology/dec` | 245 | `adjusted_unemployment_3.parquet:Apr`<br>`adjusted_unemployment_3.parquet:Aug`<br>`adjusted_unemployment_3.parquet:Dec` |
| `http://dbpedia.org/ontology/subjectTerm` | 243 | `excel.parquet:Subject`<br>`Nebraska_District_all_1.parquet:Subject`<br>`Nevada_District_all_5.parquet:Subject` |
| `http://dbpedia.org/ontology/isPartOfMilitaryConflict` | 242 | `CVI.parquet:“The reasons why Yemin was granted that interest are unclear, given the extraordinary existing compensation he was already receiving for his service as Chairman of the Board, President and Chief Executive Officer of both Delek and Logistics,” the lawsuit states.`<br>`FHN_1.parquet:** The U.S. Federal Reserve announced it had approved a merger between First Horizon National Corp and IBERIABANK Corp.`<br>`CNN.200907_2.parquet:crisis like the one that's taken place will never happen again. reporter: the president is also trying to move aggressively to deal with another potential crisis, climate change. he helped lead the group to support a reduction in greenhouse gas emissions among` |
| `http://dbpedia.org/ontology/numberOfSettlementsInCountry` | 237 | `totalsyear.parquet:females_in_agriculture`<br>`totalsyear.parquet:indiv_in_poverty`<br>`totalsyear.parquet:males_in_agriculture` |
| `http://dbpedia.org/ontology/imageSize` | 233 | `Comparison_of_image_viewers-5_1.parquet:adjust image (functions)`<br>`Comparison_of_image_viewers_5.parquet:adjust image (functions)`<br>`790_reward0_cond6510_run1.parquet:f_image` |
| `http://dbpedia.org/ontology/density` | 232 | `periodic-table-data.parquet:density`<br>`periodic-table-data.parquet:density_predicted`<br>`planets%2Bradii.parquet:density` |
| `http://dbpedia.org/ontology/projectKeyword` | 231 | `TopicModel-2-1-document.parquet:Keywords`<br>`TopicModel-2-1-document_1.parquet:Keywords`<br>`faucets_1.parquet:meta_keywords` |
| `http://dbpedia.org/ontology/school` | 230 | `house_prices.parquet:pupil-teacher ratio by town`<br>`mlr10_2.parquet:% Free school lunch`<br>`totalsyear.parquet:indiv_in_school` |
| `http://dbpedia.org/ontology/galleryItem` | 224 | `NAICS_data_1048_27.parquet:Item`<br>`NAICS_data_1048_40.parquet:Item`<br>`NAICS_data_1048_60.parquet:Item` |
| `http://dbpedia.org/ontology/race` | 224 | `totalsyear.parquet:asian_race`<br>`totalsyear.parquet:black_race`<br>`totalsyear.parquet:hispanic_race` |
| `http://dbpedia.org/ontology/information` | 217 | `GAMUTMeasureInformat_DATA.parquet:measure_information_complete`<br>`IDSD.d00b_2.parquet:ACCOMMODATION ALLOCATION INFORMATION`<br>`IDSD.d00b_4.parquet:ACCOMMODATION ALLOCATION INFORMATION` |
| `http://dbpedia.org/ontology/perCapitaIncomeAsOf` | 217 | `%20central%20african%20republic.parquet:Foreign direct investment, net (BoP, current US$)`<br>`Bosnia%20and%20Herzegovina.parquet:Broad money (% of GDP)`<br>`Bosnia%20and%20Herzegovina.parquet:Current health expenditure (% of GDP)` |
| `http://dbpedia.org/ontology/routeLine` | 216 | `MW_21_HIDS_3.parquet:data.win.eventdata.commandLine`<br>`ga_demo_1.parquet:Line Destination`<br>`Bosnia%20and%20Herzegovina.parquet:Rail lines (total route-km)` |
| `http://dbpedia.org/ontology/areaTotal` | 215 | `lil.parquet:Total Inches`<br>`joined_toronto_neighborhood_4final_analysis-a.parquet:Total private dwellings`<br>`meta_2020-11-24_aw.parquet:total_hosp` |
| `http://dbpedia.org/ontology/category` | 215 | `Example1.parquet:Categories`<br>`MVS_parameters_list.parquet:category`<br>`afg0national.parquet:CharacteristicCategory` |
| `http://dbpedia.org/ontology/targetAirport` | 214 | `ontokin.parquet:Target`<br>`table_2_thesis_marketing_literature.parquet:Targets`<br>`tbox-example.parquet:Target` |
| `http://dbpedia.org/ontology/glycemicIndex` | 213 | `Factors.parquet:Diversity index`<br>`Factors_1.parquet:Diversity index`<br>`data_science-data.parquet:thermal_comfort_index` |
| `http://dbpedia.org/ontology/security` | 213 | `2009_saturn_vue_modified.parquet:Safety`<br>`2009_nissan_gt-r_modified.parquet:Safety`<br>`2009_kia_borrego_modified.parquet:Safety` |
| `http://dbpedia.org/ontology/topic` | 213 | `ieee_trans_mob_comput-top-terms.parquet:Topic`<br>`ieee_transactions_on_information_forensics_and_security-top-terms.parquet:Topic`<br>`InteractiveTools_Master.parquet:TopicDetail` |
| `http://dbpedia.org/ontology/humanDevelopmentIndexAsOf` | 212 | `Yoshua-Bengio_1.parquet:Equivalence of Equilibrium Propagation and Recurrent Backpropagation.`<br>`2018-03-21_1.parquet:BRIEF-AbbVie and International Myeloma Foundation Partner To Study Role Of Genetic Mutation`<br>`Yoshua-Bengio_3.parquet:Equivalence of Equilibrium Propagation and Recurrent Backpropagation.` |
| `http://dbpedia.org/ontology/numberOfResourceOfType` | 212 | `BUFR_TableD_en_11.parquet:CategoryOfSequences_en`<br>`Data%20Dictionary%20-%20external.parquet:Range of Time`<br>`category_paper_mappings_20180614_1.parquet:Summary of results` |
| `http://dbpedia.org/ontology/wikiPageID` | 212 | `ga_demo_1.parquet:Page ID`<br>`valid_data.parquet:Avg_PageDist`<br>`0fa538b1-2a2f-4874-9234-ed8b884cedc2.parquet:Wikipage page ID` |
| `http://dbpedia.org/ontology/taste` | 210 | `UDS_3.parquet:flavor`<br>`UDS_2.parquet:flavor`<br>`UDS_4.parquet:flavor` |
| `http://dbpedia.org/ontology/impactFactor` | 208 | `FeatureTable_ROM_VIC_Normalized_Female.parquet:AFFECT`<br>`FeatureImportance_M1.parquet:cumulative_importance`<br>`FeatureImportance_Simple.parquet:cumulative_importance` |
| `http://dbpedia.org/ontology/listItemOf` | 206 | `indicators_list.parquet:chart_list`<br>`TBT_RewardsApi.parquet:Fetch a list of orders`<br>`definition.parquet:_list_child_element_keys` |
| `http://dbpedia.org/ontology/areaOfSearch` | 204 | `accuracyTestCases_1.parquet:search_parameters`<br>`accuracyTestCases_4.parquet:search_parameters`<br>`accuracyTestCases_7.parquet:search_parameters` |
| `http://dbpedia.org/ontology/timeInSpace` | 203 | `shuffled_tweets_1-140,751-800.parquet:t_in-flight`<br>`responses.parquet:About how many hours a week do you engage in programming?`<br>`responses.parquet:If you've worked in industry, what was your position and how long did you spend in this role?` |
| `http://dbpedia.org/ontology/usSales` | 203 | `%20central%20african%20republic.parquet:GDP (current US$)`<br>`%20central%20african%20republic.parquet:Merchandise imports (current US$)`<br>`ANF_3.parquet:“Our distribution centers remained operational, enabling us to fulfill digital customer demand globally, partially mitigating lost sales from temporary store closures,” said Chief Executive Officer Fran Horowitz.` |
| `http://dbpedia.org/ontology/mass` | 202 | `data_science-data.parquet:mass_flow_rate`<br>`data_science-data.parquet:thermal_mass`<br>`data_science-data_1.parquet:mass_flow_rate` |
| `http://dbpedia.org/ontology/shortProgCompetition` | 201 | `StructureDefinition-VA.MHV.medication.parquet:Short`<br>`hull_mods_17.parquet:short`<br>`Project%20Manager%20Survey%20(Responses)%20-%20Form%20Responses%201.parquet:Short Bio` |
| `http://dbpedia.org/ontology/nameAsOf` | 198 | `ACCOUNT_1.parquet:﻿"フィールド名/Field Name"`<br>`ADGROUP.parquet:﻿"フィールド名/Field Name"`<br>`ADGROUP_1.parquet:﻿"フィールド名/Field Name"` |
| `http://dbpedia.org/ontology/episode` | 195 | `tt0208614.parquet:episode`<br>`tt1486217.parquet:episode`<br>`tt0208614.parquet:episode` |
| `http://dbpedia.org/ontology/note` | 193 | `BUFRCREX_CodeFlag_en_33.parquet:Note_en`<br>`ACCOUNT_1.parquet:備考/Note`<br>`ADGROUP.parquet:備考/Note` |
| `http://dbpedia.org/ontology/leftTributary` | 192 | `Tabela_Preditiva.parquet:par_left`<br>`angle-changes-attack-patch10pgd001.parquet:left_elbow`<br>`angle-changes-attack-patch10pgd001.parquet:left_hip` |
| `http://dbpedia.org/ontology/facilityId` | 191 | `chart_label_to_concept.parquet:unit_concept_id`<br>`chart_label_to_concept_1.parquet:unit_concept_id`<br>`Big-Data-Developers-in-Austin_events_past.parquet:venue_id` |
| `http://dbpedia.org/ontology/militaryUnit` | 185 | `GWAGUIPropList_1.parquet:Unit`<br>`GWAGUIPropList_2.parquet:Unit`<br>`USDA_cost_data_6.parquet:Unit` |
| `http://dbpedia.org/ontology/runtime` | 184 | `007_edited.parquet:Runtime`<br>`dbo_runtime-104-small.parquet:http://dbpedia.org/ontology/runtime`<br>`dbo_runtime-109-small-10var.parquet:http://dbpedia.org/ontology/runtime` |
| `http://dbpedia.org/ontology/winsAtLET` | 184 | `2017-09-08.parquet:Apple lawsuits against Qualcomm can proceed, U.S. judge rules`<br>`shuuru.parquet:He hugged her close, then let go to shake hands with Joe's o...`<br>`CT.parquet:ah , i guess biden rally goers cant afford horns . and if biden wins , they maybe can buy whistles if they save up` |
| `http://dbpedia.org/ontology/openingTheme` | 180 | `da-DK_10.parquet:'Opening'`<br>`da-DK_105.parquet:'Opening'`<br>`da-DK_106.parquet:'Opening'` |
| `http://dbpedia.org/ontology/alpsSection` | 177 | `blueboxes_1.parquet:section`<br>`help_142.parquet:SECTION`<br>`help_144.parquet:SECTION` |
| `http://dbpedia.org/ontology/co2Emission` | 177 | `data_science-data.parquet:carbon_dioxide`<br>`data_science-data_1.parquet:carbon_dioxide`<br>`scenario_uniq_values.parquet:soln_emissions_per_funit` |
| `http://dbpedia.org/ontology/cpu` | 176 | `competitors_bsd_construct.parquet:cpu_time`<br>`competitors_bsd_count.parquet:cpu_time`<br>`competitors_impala_count.parquet:cpu_time` |
| `http://dbpedia.org/ontology/lunarOrbitTime` | 174 | `2020-09-08.parquet:moonPhase`<br>`2020-09-08_1.parquet:moonPhase`<br>`2020-09-08_100.parquet:moonPhase` |
| `http://dbpedia.org/ontology/product` | 171 | `S03E03_script.parquet:Advertise your product or brand here`<br>`en_US_1009.parquet:Product`<br>`lac_201027.parquet:coefficient_product` |
| `http://dbpedia.org/ontology/event` | 168 | `dwbfpricesukwin16072013.parquet:EVENT_DT`<br>`adverse-event-information.parquet:adverseEventId`<br>`COVID19_Metadata_v0.95_Sample_Complete_v5.parquet:exposure event` |
| `http://dbpedia.org/ontology/service` | 168 | `shuffled_tweets_1-140,751-800.parquet:t_customer_service`<br>`review_train.parquet:Service`<br>`university-of-virginia-medical-center.parquet:service` |
| `http://dbpedia.org/ontology/humanDevelopmentIndex` | 166 | `79cdc3e1c43097bd770fcf43fbd54109178ef6c6e8ed0aa7903914919bb3094e_6.parquet:NonHousingDevelopment`<br>`8652da2117d959f3e0b90b743ae39b4f6a1c53079ff3660b6a5b5790bc8c9cd5_4.parquet:NonHousingDevelopment`<br>`DCI.parquet:researchDevelopment` |
| `http://dbpedia.org/ontology/fuelSystem` | 164 | `l36%20(Kestrel%20Cruiser%20C%20-%20NORMAL%20AE).parquet:DRONE CONTROL SYSTEM CAPACITY`<br>`l36%20(Kestrel%20Cruiser%20C%20-%20NORMAL%20AE).parquet:DRONE CONTROL SYSTEM DAMAGE`<br>`l36%20(Kestrel%20Cruiser%20C%20-%20NORMAL%20AE).parquet:MEDBAY SYSTEM POWER` |
| `http://dbpedia.org/ontology/branchFrom` | 163 | `morning22_alerts_equityalert_com.parquet:From`<br>`risksolutions_standardandpoors_com.parquet:From`<br>`weekly_text_lists_smartmoney_com.parquet:From` |
| `http://dbpedia.org/ontology/initiallyUsedFor` | 162 | `humaneval_master.parquet:Test used`<br>`heyyitsadam.parquet:She said that she had just joined up as a new recruit. She s...`<br>`CVI.parquet:CVR also claims that Delek twice rejected CVR’s request for the additional documents establishing Yemin’s pay.` |
| `http://dbpedia.org/ontology/limit` | 161 | `measurements_1.parquet:limits`<br>`measurements.parquet:limits`<br>`BuildingInfo.parquet:TargetLimit` |
| `http://dbpedia.org/ontology/gdpPerCapita` | 160 | `us_states_misc_stats.parquet:population_per_sq_mi`<br>`%20central%20african%20republic.parquet:GDP growth (annual %)`<br>`%20central%20african%20republic.parquet:GDP per capita, PPP (current international $)` |
| `http://dbpedia.org/ontology/homeArena` | 160 | `auth_user_102.parquet:homeAction`<br>`auth_user_104.parquet:homeAction`<br>`auth_user_105.parquet:homeAction` |
| `http://dbpedia.org/ontology/dutchNAIdentifier` | 159 | `train_filtrado.parquet:IeVerIdentifier`<br>`non_thermal_springs.parquet:Na`<br>`Parties_Groups_1.parquet:NLA_Party_Identifier` |
| `http://dbpedia.org/ontology/material` | 158 | `data_science-data.parquet:material_properties`<br>`data_science-data_1.parquet:material_properties`<br>`datasets_8.parquet:dv.relatedMaterial` |
| `http://dbpedia.org/ontology/distance` | 157 | `BostonHousing.parquet:distance`<br>`RP3_2016-03-27_09-23-29.parquet:distance`<br>`RP3_2016-03-27_09-23-29.parquet:distance_per_stroke` |
| `http://dbpedia.org/ontology/average` | 155 | `Movie_Clean.parquet:vote_average`<br>`City_of_Seattle_Wages___Comparison_by_Gender__Wage_Progression_Job_Titles_3.parquet:Total Avg Hrly Rate`<br>`City_of_Seattle_Wages___Comparison_by_Gender__Wage_Progression_Job_Titles_4.parquet:Total Avg Hrly Rate` |
| `http://dbpedia.org/ontology/meanTemperature` | 152 | `male_10.parquet:Arousal Mean`<br>`male_10.parquet:Dominance Mean`<br>`male_10.parquet:Valence Mean` |
| `http://dbpedia.org/ontology/wikiPageOutDegree` | 152 | `import_media_two_stores_1032.parquet:hide_from_product_page`<br>`import_media_two_stores_1115.parquet:hide_from_product_page`<br>`import_media_two_stores_1451.parquet:hide_from_product_page` |
| `http://dbpedia.org/ontology/lastPosition` | 151 | `prediction-20170128-091541.parquet:last close`<br>`prediction-20170130-091911.parquet:last close`<br>`prediction-20170131-092137.parquet:last close` |
| `http://dbpedia.org/ontology/age` | 148 | `Factors.parquet:Age dependency ratio, %`<br>`Factors_1.parquet:Age dependency ratio, %`<br>`John_L=-Hennessy.parquet:new golden age computer architecture` |
| `http://dbpedia.org/ontology/area` | 145 | `Factors.parquet:Area`<br>`Factors_1.parquet:Area`<br>`20170324_16sd0_r1_0.02Hz_shock_1.parquet:area` |
| `http://dbpedia.org/ontology/participatingIn` | 144 | `OAH%20Program%20Observation%20Form%20for%20TPP%20Grantees.parquet:How actively did the group members participate in discussions and activities?`<br>`Tabela_Preditiva.parquet:in`<br>`permit-2005-1109_formatted.parquet:in` |
| `http://dbpedia.org/ontology/kindOfCriminalAction` | 141 | `totalsyear.parquet:violent_crime_for_county`<br>`%E4%BA%BA%E6%95%99%E7%89%88%E9%AB%98%E4%B8%AD%E8%8B%B1%E8%AF%AD4%20-%20%E5%BF%85%E4%BF%AE.parquet:n the action of accomplishing something`<br>`Parameters_Global_20200614.parquet:Rate of Action` |
| `http://dbpedia.org/ontology/relatedMeanOfTransportation` | 141 | `ACS_UCL.parquet:C. How would you describe the existing coverage?`<br>`MSNBC.201707.parquet:we talked about these and other matters and challenges facing southern mayors around climate change.`<br>`ACS_UCL.parquet:C. How would you describe the existing coverage?` |
| `http://dbpedia.org/ontology/feature` | 140 | `imp_909_predict_0203-1.py.parquet:feature`<br>`imp_910_predict_0204-1.py.parquet:feature`<br>`imp_931_predict_0225-2.py.parquet:feature` |
| `http://dbpedia.org/ontology/aircraftTransport` | 139 | `EDCD.d02b_3.parquet:TRANSPORT MEANS`<br>`EDCD.d03a_4.parquet:TRANSPORT MEANS`<br>`EDCD.d03b_3.parquet:TRANSPORT MEANS` |
| `http://dbpedia.org/ontology/impactFactorAsOf` | 138 | `BK.parquet:Chong cautioned that narrowing spreads do not correlate directly with outflows. Factors such as diversification support continued flows into CGBs.`<br>`How_does_temperature_and_humidity_affect_the_tramsmission_of_2019_nCoV_.parquet:Factors Described`<br>`MOS_5.parquet:Rising potash demand comes as BHP Group is expected to make an investment decision soon on completing its Jansen, Saskatchewan mine.` |
| `http://dbpedia.org/ontology/capacityFactor` | 137 | `scenario_uniq_values.parquet:soln_energy_efficiency_factor`<br>`unit_of_measure_100.parquet:factor_c`<br>`unit_of_measure_113.parquet:factor_c` |
| `http://dbpedia.org/ontology/systemRequirements` | 136 | `Basic_and_Speciality_Actions.parquet:Requirements`<br>`StructureDefinition-ClinicalDischargeType.parquet:Requirements`<br>`StructureDefinition-ConsultationsAmountKlinic.parquet:Requirements` |
| `http://dbpedia.org/ontology/interest` | 134 | `DCI.parquet:minorityInterest`<br>`DEF.DE.parquet:minorityInterest`<br>`DIRV.parquet:minorityInterest` |
| `http://dbpedia.org/ontology/numberOfVisitorsAsOf` | 132 | `infocom1997-1.parquet:Design of a Gigabit ATM Switch.`<br>`words-without-force-positions_6.parquet:percent_of_d_speeches`<br>`%20central%20african%20republic.parquet:Exports of goods and services (BoP, current US$)` |
| `http://dbpedia.org/ontology/drainsFrom` | 131 | `nl-NL.parquet:Remove from compare`<br>`nl-NL_3.parquet:Remove from compare`<br>`nl-NL_4.parquet:Remove from compare` |
| `http://dbpedia.org/ontology/maximumDepth` | 129 | `PDS4_IMG_SURFACE_1D00_1210.parquet:Maximum Characters`<br>`PDS4_CHAN1_1E00_1110.parquet:Maximum Characters`<br>`PDS4_CHAN1_1F00_1110.parquet:Maximum Characters` |
| `http://dbpedia.org/ontology/significantDesign` | 128 | `data_science-data.parquet:passive_design_strategies`<br>`data_science-data_1.parquet:passive_design_strategies`<br>`RealTime%20Embedded%20Systems%20Programming_test_2_1.parquet:Object Oriented Analysis and Design` |
| `http://dbpedia.org/ontology/nationalTopographicSystemMapNumber` | 127 | `StructureDefinition-VA.MHV.medication.parquet:Mapping: FiveWs Pattern Mapping`<br>`review_69353_extracted_data_csv_20201227072740.parquet:Baseline map_mean_primary`<br>`review_69353_extracted_data_csv_20201227072740.parquet:Baseline map_sd_primary` |
| `http://dbpedia.org/ontology/codeIndex` | 125 | `-1605911493-0-nodes.parquet:DataRateIndex`<br>`numeo.parquet:Crime Index`<br>`numeo.parquet:Exp Pollution Index` |
| `http://dbpedia.org/ontology/areaLand` | 124 | `scenario_uniq_values.parquet:include_unprotected_land_in_regrowth_calcs`<br>`%20central%20african%20republic.parquet:Land area (sq. km)`<br>`%20central%20african%20republic.parquet:Rural land area (sq. km)` |
| `http://dbpedia.org/ontology/escapeVelocity` | 124 | `CombinedLog2021_01_25_12_19_16_757.parquet:velocity`<br>`CombinedLog2021_01_25_12_19_16_757.parquet:velocityAccuracy`<br>`CombinedLog2021_02_22_16_46_33_423.parquet:velocity` |
| `http://dbpedia.org/ontology/field` | 124 | `study_variablelist.parquet:Field info`<br>`study_variablelist.parquet:Field units`<br>`ClassicDatabase_DataDictionary_2021-03-31.parquet:Field Label` |
| `http://dbpedia.org/ontology/rgbCoordinateBlue` | 124 | `blue-dragon.parquet:Blue`<br>`purple-elephant.parquet:Purple`<br>`purple-panty-dropper.parquet:Purple` |
| `http://dbpedia.org/ontology/tvComId` | 122 | `932867inns1_wickets.parquet:com`<br>`PrenatalCareRecord13PC_SubAb_grid.parquet:PC_SACom`<br>`SocialServicesRecord17.parquet:SS_BarACom` |
| `http://dbpedia.org/ontology/noteOnPlaceOfBurial` | 120 | `tissec15.parquet:Guest Editorial: Special Issue on Computer and Communications Security.`<br>`MI_2.parquet:i think after the election trump will begin laying waste to the anarchist assholes , in and out of government . 🙏 🇺 🇸 🇺 🇸 🇺 🇸 🇺 🇸`<br>`WLL_1.parquet:Shares of the company’s new common stock will start trading on the New York Stock Exchange under the ticker symbol “WLL” on Wednesday.` |
| `http://dbpedia.org/ontology/proTeam` | 118 | `Citi_glassdoor_ratings.parquet:pros`<br>`WellsFargo_glassdoor_ratings.parquet:pros`<br>`modified_flux_ds.parquet:pro__L` |
| `http://dbpedia.org/ontology/firstGame` | 117 | `2012-GOAT-OneUp.parquet:GAME`<br>`2019-GOAT-PopularMechanics.parquet:GAME`<br>`29414811_13_8724394428539174350_10.parquet:game` |
| `http://dbpedia.org/ontology/governingBody` | 115 | `2019-10-27.parquet:body`<br>`2019-12-25.parquet:body`<br>`2019-10-27.parquet:body` |
| `http://dbpedia.org/ontology/nonFictionSubject` | 115 | `books.essential_1.parquet:Books > Non-Fiction > Essential programming`<br>`books.essential_124.parquet:Books > Non-Fiction > Essential programming`<br>`books.essential_16.parquet:Books > Non-Fiction > Essential programming` |
| `http://dbpedia.org/ontology/currentStatus` | 113 | `ESU_Results%202020-11-10.parquet:FutureStatus (see methods)`<br>`Example1.parquet:CurrentProb`<br>`ProgramParticipation.parquet:HousingStatusAtExit` |
| `http://dbpedia.org/ontology/followedBy` | 112 | `scopus(1)_3.parquet:Cited by`<br>`DatabaseInfo.parquet:identifiedBy`<br>`DatabaseInfo_1.parquet:identifiedBy` |
| `http://dbpedia.org/ontology/strength` | 112 | `UDS_3.parquet:toughness`<br>`UDS_2.parquet:toughness`<br>`UDS_4.parquet:toughness` |
| `http://dbpedia.org/ontology/position` | 111 | `-1605911493-0-nodes.parquet:Position x`<br>`-1605911493-0-nodes.parquet:Position y`<br>`ADMISSIONS_DM.parquet:hasPosition` |
| `http://dbpedia.org/ontology/numberOfPiersInWater` | 109 | `Bosnia%20and%20Herzegovina.parquet:Debt service on external debt, total (TDS, current US$)`<br>`Company_info.parquet:shortPercentOfFloat`<br>`eretrea.parquet:Debt service on external debt, total (TDS, current US$)` |
| `http://dbpedia.org/ontology/owner` | 108 | `core.parquet:OwnerUserId`<br>`InstrumentModel.parquet:owner_id`<br>`2007-03-08_1.parquet:folder_owner_addrhouse` |
| `http://dbpedia.org/ontology/passengersUsedSystem` | 108 | `l36%20(Kestrel%20Cruiser%20C%20-%20NORMAL%20AE).parquet:NEARBY SHIP ARTILLERY SYSTEM CAPACITY`<br>`l36%20(Kestrel%20Cruiser%20C%20-%20NORMAL%20AE).parquet:NEARBY SHIP CLOAKING SYSTEM CAPACITY`<br>`l36%20(Kestrel%20Cruiser%20C%20-%20NORMAL%20AE).parquet:NEARBY SHIP CLONEBAY SYSTEM DAMAGE` |
| `http://dbpedia.org/ontology/child` | 106 | `coldwell_2006_1.parquet:child_anger`<br>`coldwell_2006_1.parquet:child_warmth`<br>`Colorado_1.parquet:flu children` |
| `http://dbpedia.org/ontology/currentProduction` | 106 | `stock_inventoryLine_10.parquet:currentQty`<br>`stock_inventoryLine_12.parquet:currentQty`<br>`stock_inventoryLine_13.parquet:currentQty` |
| `http://dbpedia.org/ontology/superFamily` | 106 | `2b8005cb-0a16-479e-b298-a35dab6d336f_tags.parquet:great school\excellent staff\nice\too big academic\usually interesting\informative staff\always willing\very nice`<br>`1dcb0fa0-183e-4c9e-a662-0a83b00f7f0e_tags.parquet:great staff\great environment\good\amazing college`<br>`1b97db4e-7ded-4e00-8f41-4aef72e23bcc_tags.parquet:overall professor\super nice\favorite professor\super approachable\very organized course` |
| `http://dbpedia.org/ontology/badGuy` | 105 | `2012_raw.parquet:BadKen`<br>`5643d911-95bf-44e4-bd26-0c77a87a9fe6_tags.parquet:very good\think online learning\hard\other\good job\get lonely\mental health`<br>`e534c86b-e20f-4b9d-8254-d92a6a228018_tags.parquet:south bend\able\many people\good thing\small school\great job\amazing job` |
| `http://dbpedia.org/ontology/otherInformation` | 105 | `S-T,%20medical%20abbreviations.parquet:More Info`<br>`Q-R,%20medical%20abbreviations.parquet:More Info`<br>`S-T,%20medical%20abbreviations.parquet:More Info` |
| `http://dbpedia.org/ontology/derivedWord` | 104 | `joy_words.parquet:word`<br>`blueboxes_1.parquet:word`<br>`anniversary_phrases.parquet:word` |
| `http://dbpedia.org/ontology/hasOutsidePlace` | 104 | `2018-01-30.parquet:LIVE MARKETS-Tech outperforms, but Apple and suppliers remain under pressure`<br>`2018-01-30_1.parquet:LIVE MARKETS-Tech outperforms, but Apple and suppliers remain under pressure`<br>`AEO_1.parquet:Aerie, which sells work-from-home favorites including lingerie and loungewear, has been exceeding the company's expectations, Schottenstein added.` |
| `http://dbpedia.org/ontology/hasInput` | 103 | `covid-tools-dashboards.parquet:Accept Public Input`<br>`errors-dms-rej-list.parquet:Obligation Error: Mandatory Data Element has not been provided`<br>`web-files-small-metadata.parquet:hasAudio` |
| `http://dbpedia.org/ontology/output` | 103 | `Extended_Data_Table_5.parquet:OutputMarkers`<br>`28_1.parquet:Output Growth Rate %`<br>`ourpresident_100000_small.parquet:output` |
| `http://dbpedia.org/ontology/percentage` | 102 | `TopicModel-2-1-document.parquet:Percentage`<br>`TopicModel-2-1-document_1.parquet:Percentage`<br>`faucets_1.parquet:ref_payout_percent` |
| `http://dbpedia.org/ontology/grossDomesticProductAsOf` | 101 | `%E5%8C%97%E5%B8%88%E5%A4%A7%E7%89%88%E9%AB%98%E4%B8%AD%E8%8B%B1%E8%AF%AD%E9%80%89%E4%BF%AE%E6%A8%A1%E5%9D%979.parquet:n growth to a global or worldwide scale`<br>`%E5%8C%97%E5%B8%88%E5%A4%A7%E7%89%88%E9%AB%98%E4%B8%AD%E8%8B%B1%E8%AF%AD%E9%80%89%E4%BF%AE%E6%A8%A1%E5%9D%979.parquet:n growth to a global or worldwide scale`<br>`Bosnia%20and%20Herzegovina.parquet:Gross savings (% of GDP)` |
| `http://dbpedia.org/ontology/topSpeed` | 101 | `ieee_trans_on_cad_of_integrated_circuits_and_systems-top-terms.parquet:Top Terms`<br>`topic_keys60-july_cognitive_2.parquet:Top words`<br>`topics.parquet:TopTerms` |
| `http://dbpedia.org/ontology/numberOfCollectionItems` | 100 | `Globtherm2_within_species_SO.parquet:elevation_of_collection`<br>`Globtherm2_within_species_SO.parquet:lat_of_collection`<br>`Globtherm2_within_species_SO.parquet:long_of_collection` |
| `http://dbpedia.org/ontology/tree` | 100 | `uber_alltime_4.parquet:Construct Quad Tree`<br>`sw_0194_3595.utt.parquet:parse_tree`<br>`CAP206_GAG_1_all_hap.fasta_classification.parquet:partistic_tree_confidence` |
| `http://dbpedia.org/ontology/fuelConsumption` | 98 | `data_science-data.parquet:energy_consumption`<br>`data_science-data_1.parquet:energy_consumption`<br>`scenario_uniq_values.parquet:conv_fuel_consumed_per_funit` |
| `http://dbpedia.org/ontology/generationUnits` | 98 | `totalsyear.parquet:housing_units`<br>`Biomass_Prices.parquet:Common Units`<br>`Biomass_Prices.parquet:Original Units` |
| `http://dbpedia.org/ontology/systemOfLaw` | 98 | `InProgressItems.parquet:System.IterationPath`<br>`AndroidSupport.Merged.dll.TR_3.parquet:System.Object`<br>`AndroidX.merged.dll.TR.parquet:System.Object` |
| `http://dbpedia.org/ontology/stateOfOrigin` | 96 | `source_17.parquet:motionState_radiusOfCurvature`<br>`source_18.parquet:motionState_radiusOfCurvature`<br>`source_20.parquet:motionState_radiusOfCurvature` |
| `http://dbpedia.org/ontology/training` | 96 | `extraction_alpha_18.parquet:training_time`<br>`extraction_alpha_21.parquet:training_time`<br>`extraction_alpha_25.parquet:training_time` |
| `http://dbpedia.org/ontology/allcinemaId` | 95 | `account_accountManagement_100.parquet:product.importId`<br>`account_accountManagement_111.parquet:product.importId`<br>`account_accountManagement_113.parquet:product.importId` |
| `http://dbpedia.org/ontology/damage` | 95 | `gear.parquet:Magic damage`<br>`ArtifactInfo_1.parquet:DamageBonus`<br>`ArtifactInfo_1.parquet:DamageBonus` |
| `http://dbpedia.org/ontology/meaning` | 95 | `n4_2.parquet:meaning`<br>`S-T,%20medical%20abbreviations.parquet:Meaning`<br>`kanji-J5.parquet:meanings` |
| `http://dbpedia.org/ontology/leftChild` | 94 | `20150410TORORL.parquet:seconds_left`<br>`20151213UTAOKC.parquet:seconds_left`<br>`20151217OKCCLE.parquet:seconds_left` |
| `http://dbpedia.org/ontology/education` | 93 | `ec954ebb-8e1b-4605-a716-cf11ab3d3530_tags.parquet:not applicable\online education`<br>`Final%20-%20Copy.parquet:Basic-school Entrepreneurial Education and training`<br>`Final%20-%20Copy.parquet:Post-school entrepreneurial education and training` |
| `http://dbpedia.org/ontology/abstract` | 92 | `BiPlot.parquet:Abstraction`<br>`FeatureTable_ROM_VIC_Normalized_Female.parquet:ABSTRACTION`<br>`causality_matrix_DtoA.parquet:Multifaceted Abstraction` |
| `http://dbpedia.org/ontology/generalManager` | 92 | `DLAKF.parquet:sellingGeneralAdministrative`<br>`DLAKY.parquet:sellingGeneralAdministrative`<br>`DNSKF.parquet:sellingGeneralAdministrative` |
| `http://dbpedia.org/ontology/matchPoint` | 92 | `DT_01__per_pair_results.parquet:f_match`<br>`DT_01__per_pair_results.parquet:p_match`<br>`DT_01__per_pair_results.parquet:r_match` |
| `http://dbpedia.org/ontology/format` | 91 | `DataDictionary.parquet:dataFormat`<br>`copo_schema.parquet:dc.relation hasFormat`<br>`Comparison_of_data_serialization_formats_2.parquet:Format` |
| `http://dbpedia.org/ontology/minimumElevation` | 91 | `PDS4_CHAN1_1E00_1110.parquet:Minimum Characters`<br>`PDS4_CHAN1_1F00_1110.parquet:Minimum Characters`<br>`PDS4_DISP_1D00_1500_1.parquet:Minimum Characters` |
| `http://dbpedia.org/ontology/teachingStaff` | 91 | `nctq_2020.parquet:Student Teaching/Clinical Practice (General Teacher Preparation Policy)`<br>`nctq_2020.parquet:Teaching Reading (Elementary Teacher Preparation Policy)`<br>`table_columns_1.parquet:practice_students` |
| `http://dbpedia.org/ontology/maxAbsoluteMagnitude` | 90 | `NIST2_Sandia_Helium_Plume_dataplot_config.parquet:Exp_Error_Absolute`<br>`evaluations_metrics.parquet:absolute_se`<br>`RagdollParam.parquet:maxAngularVelocity` |
| `http://dbpedia.org/ontology/causeOfDeath` | 88 | `Parameters_Global_20200509.parquet:Rate of Death`<br>`Parameters_Global_20200509_4.parquet:Rate of Death`<br>`Parameters_Global_20200511_4.parquet:Rate of Death` |
| `http://dbpedia.org/ontology/isPartOfWineRegion` | 88 | `ACS_UCL.parquet:B. How is the data item stored, within the centre?`<br>`ACS_UCL.parquet:B. How is the data item stored, within the centre?`<br>`BMO_2.parquet:The wealth management industry is seeing some shakeup. Societe Generale is in exclusive talks to sell most of asset manager Lyxor to Amundi, while Wells Fargo & Co in February agreed to sell its asset management business.` |
| `http://dbpedia.org/ontology/isRouteStop` | 86 | `faucets_1.parquet:is_paused`<br>`xgboost_level0.parquet:stop_iter`<br>`seliu93.parquet:If an exoplanet happens to transit its star, we can often de...` |
| `http://dbpedia.org/ontology/populationAsOf` | 86 | `house_prices.parquet:% lower status of the population`<br>`mlr10_2.parquet:% Change in population`<br>`BK.parquet:SHANGHAI (Reuters) - Investors have become net sellers of Chinese government bonds (CGBs) in recent weeks, a BNY Mellon indicator showed, as rising U.S. yields and an aggressive American vaccination drive eat into the appeal of Chinese bonds.` |
| `http://dbpedia.org/ontology/careerPoints` | 85 | `pt_PT_7.parquet:Points`<br>`Agriculture%20and%20Forestry_4.parquet:Entry points`<br>`Agriculture%20and%20Forestry_7.parquet:Entry points` |
| `http://dbpedia.org/ontology/nameInCantoneseChinese` | 85 | `vectors_topic_2_by_languages.parquet:[Chinese, (Sim.)]_Freq`<br>`vectors_topic_4_by_languages.parquet:[Chinese, (Sim.)]_Freq`<br>`007.parquet:ENGLISH-A` |
| `http://dbpedia.org/ontology/powerOutput` | 84 | `InstrumentModel.parquet:im/power_supply_voltage_maximum`<br>`InstrumentModel.parquet:im/power_supply_voltage_maximum`<br>`data_12109.parquet:input_buildings_electricity_demand` |
| `http://dbpedia.org/ontology/careerPrizeMoney` | 83 | `quantity_theory_open_data_H.parquet:money growth`<br>`quantity_theory_open_data_oecd.parquet:money growth`<br>`amazon-movies_&_tv-cv-1-bigrams-normalized.parquet:your money` |
| `http://dbpedia.org/ontology/confirmedCases` | 83 | `joined_toronto_neighborhood_4final_analysis-a.parquet:COVID Cases`<br>`tested_numbers_icmr_data_2.parquet:Individuals Tested Per Confirmed Case`<br>`tested_numbers_icmr_data_2.parquet:Positive cases from samples reported` |
| `http://dbpedia.org/ontology/noContest` | 82 | `90.parquet:No`<br>`Aizen90.parquet:no`<br>`AnArousedPanda.parquet:no` |
| `http://dbpedia.org/ontology/superOrder` | 82 | `super-snow-dog.parquet:Super`<br>`crop_research.parquet:super`<br>`6f79daed-7608-4b55-9847-8091f782e7c2_tags.parquet:very quick\super helpful\very efficient\very knowledgeable\willing` |
| `http://dbpedia.org/ontology/currentlyUsedFor` | 81 | `programming-basic.parquet:What does SOLID stand for?`<br>`modified_flux_ds.parquet:for_std`<br>`Episode-5-Hinode-Bridge.parquet:For someone who can see the future` |
| `http://dbpedia.org/ontology/locationIdentifier` | 81 | `train_filtrado.parquet:CityIdentifier`<br>`train_filtrado.parquet:OrganizationIdentifier`<br>`review_69353_extracted_data_csv_20201227072740.parquet:Study Identifier` |
| `http://dbpedia.org/ontology/numberOfFederalDeputies` | 81 | `Bosnia%20and%20Herzegovina.parquet:Central government debt, total (% of GDP)`<br>`mexico.parquet:Central government debt, total (% of GDP)`<br>`san%20mareno.parquet:Central government debt, total (% of GDP)` |
| `http://dbpedia.org/ontology/lunarSampleMass` | 80 | `What%20is%20the%20best%20method%20to%20combat%20the%20hypercoagulable%20state%20seen%20in%20COVID-19__1.parquet:Sample`<br>`What%20is%20the%20efficacy%20of%20novel%20therapeutics%20being%20tested%20currently_.parquet:Sample`<br>`unit_test_1.parquet:Sample` |
| `http://dbpedia.org/ontology/comparable` | 78 | `conditional_BF_df.parquet:comparison`<br>`full-robot-template.parquet:Equivalent`<br>`full-robot-template_1.parquet:Equivalent` |
| `http://dbpedia.org/ontology/production` | 78 | `Bosnia%20and%20Herzegovina.parquet:Aquaculture production (metric tons)`<br>`Bosnia%20and%20Herzegovina.parquet:Cereal production (metric tons)`<br>`eretrea.parquet:Aquaculture production (metric tons)` |
| `http://dbpedia.org/ontology/capacity` | 77 | `scenario_uniq_values.parquet:conv_lifetime_capacity`<br>`scenario_uniq_values.parquet:soln_lifetime_capacity`<br>`periodic-table-data.parquet:heat_capacity` |
| `http://dbpedia.org/ontology/countryWithFirstSatellite` | 77 | `SHI_1.parquet:Huang also said the coronavirus crisis had forced the company to cut fuel exports in the second quarter.`<br>`oracle.parquet:What Happens When Americans Join the Global Internet`<br>`2017-12-12.parquet:BRIEF-Advance Auto Parts Announces Partnership with Interstate Batteries` |
| `http://dbpedia.org/ontology/management` | 77 | `jdg_nrm.parquet:Learnability of management team`<br>`jdg_nrm_1.parquet:Learnability of management team`<br>`2015-03-31%2006_35_01.parquet:maize_mgt_practices` |
| `http://dbpedia.org/ontology/rightTributary` | 77 | `angle-changes-attack-patch10pgd001.parquet:right_elbow`<br>`angle-changes-attack-patch10pgd001.parquet:right_shoulder`<br>`angle-changes-attack-patch10pgd001_2.parquet:right_elbow` |
| `http://dbpedia.org/ontology/architecturalStyle` | 76 | `Designite_BlankAppTemplateXaml.Android_ArchSmells.parquet:Architecture smell`<br>`Designite_CSharpExpressionCompiler_ArchSmells.parquet:Architecture smell`<br>`Designite_CircuitEditor.vs2010_ArchSmells.parquet:Architecture smell` |
| `http://dbpedia.org/ontology/languageCode` | 75 | `help_67.parquet:SYNTAX`<br>`help_93.parquet:SYNTAX`<br>`train_filtrado.parquet:Census_OSInstallLanguageIdentifier` |
| `http://dbpedia.org/ontology/sudocId` | 75 | `ForC.GROA.potential.duplicate.measurement.list.parquet:is.duplicate.of.ForC.measurement.ID`<br>`ForC.GROA.potential.duplicate.measurement.list_1.parquet:is.duplicate.of.ForC.measurement.ID`<br>`account_accountManagement_100.parquet:paymentMode.importId` |
| `http://dbpedia.org/ontology/shipBeam` | 74 | `l36%20(Kestrel%20Cruiser%20C%20-%20NORMAL%20AE).parquet:HULL`<br>`l36%20(Kestrel%20Cruiser%20C%20-%20NORMAL%20AE).parquet:NEARBY SHIP CLOAKING SYSTEM DAMAGE`<br>`l36%20(Kestrel%20Cruiser%20C%20-%20NORMAL%20AE).parquet:NEARBY SHIP HULL` |
| `http://dbpedia.org/ontology/tuition` | 73 | `1992.parquet:Tuition`<br>`1993.parquet:Tuition`<br>`1995.parquet:Tuition` |
| `http://dbpedia.org/ontology/visitorsPerDay` | 73 | `prepared_data_1.parquet:Beats.Per.Minute`<br>`prepared_data_2.parquet:Beats.Per.Minute`<br>`aggregate%20of%20nr%20stools%20per%207%20days.parquet:mean_per_week` |
| `http://dbpedia.org/ontology/lineLength` | 72 | `NIST2_Sandia_Helium_Plume_dataplot_config.parquet:Exp_Line_Width`<br>`valid_data.parquet:Avg_LineDist`<br>`CardiganOutParAllV1UP.parquet:Step length Left [m]` |
| `http://dbpedia.org/ontology/rightChild` | 72 | `PARTICIPANT_ITC_SDRD_Builder_2019-11-05_10h59.07.106.parquet:Right_Choice_Text`<br>`PARTICIPANT_ITC_SDRD_Builder_2020-04-07_09h53.25.817.parquet:Right_Choice_Text`<br>`trials_phase-testing_session-plusmin.parquet:right_face` |
| `http://dbpedia.org/ontology/animal` | 71 | `tdata_var.parquet:plant_animal`<br>`0509-Critical%20event%20that%20made%20crash%20imminent.parquet:Object or animal`<br>`1014-Critical%20event%20that%20made%20crash%20imminent.parquet:Object or animal` |
| `http://dbpedia.org/ontology/basedOn` | 71 | `%E4%BA%BA%E6%95%99%E7%89%88%E9%AB%98%E4%B8%AD%E8%8B%B1%E8%AF%AD%E9%80%89%E6%8B%A9%E6%80%A7%E5%BF%85%E4%BF%AE%E7%AC%AC%E5%9B%9B%E5%86%8C_1.parquet:n. a literary work based on the imagination and not necessarily on fact`<br>`indicators_details.parquet:Details on Scoring`<br>`BasedOnBooks.old.07.12.2020_1.parquet:[Based on a Book]` |
| `http://dbpedia.org/ontology/criteria` | 71 | `lac_201027.parquet:criteria`<br>`lac_201027_1.parquet:criteria`<br>`CloseFrameworkAgreementUA.parquet:awardCriteriaDetails` |
| `http://dbpedia.org/ontology/discontinued` | 71 | `CX.parquet:discontinuedOperations`<br>`CXMSF.parquet:discontinuedOperations`<br>`DD.parquet:discontinuedOperations` |
| `http://dbpedia.org/ontology/otherFuelType` | 71 | `erratic_ked.parquet:It takes more than a calorie of fossil fuel energy to produc...`<br>`linearised.parquet:Rotating components such as jet-engine blades and gas turbin...`<br>`account_accountingBatch_10.parquet:directDebitDataTypeSelect` |
| `http://dbpedia.org/ontology/otherSportsExperience` | 71 | `b1552ccb-7fde-422c-a49a-ac4a8a70584d_tags.parquet:most wonderful experience\so involved\caring heart`<br>`beaecf25-c64e-45b3-836d-d1c0bc13c62c_tags.parquet:mostly positive experience\great part\small school\able\online`<br>`d2971843-e90f-413d-9896-4bd16e6937dc_tags.parquet:great experience` |
| `http://dbpedia.org/ontology/atRowNumber` | 70 | `hipc_ctf_19029902_1_1.parquet:row_key`<br>`hipc_ctf_23591775_4.parquet:row_key`<br>`hipc_ctf_24495909_1.parquet:row_key` |
| `http://dbpedia.org/ontology/circuitName` | 70 | `Introduction%20to%20Microprocessors_1.parquet:Circuit Modeling I`<br>`Logic%20and%20Digital%20Design_2.parquet:Circuit Modeling I`<br>`RealTime%20Embedded%20Systems%20Programming_test_2_1.parquet:Circuit Modeling I` |
| `http://dbpedia.org/ontology/deadInFightPlace` | 70 | `abilities_1.parquet:Genji looses three deadly throwing stars in quick succession.`<br>`hugofromstatefarm.parquet:When beetles fight these battles in a bottle with their padd...`<br>`jpgranger.parquet:We hold our rifles in missing hands. We stand tall on missin...` |
| `http://dbpedia.org/ontology/denomination` | 70 | `density_pure_101.parquet:Denom`<br>`density_pure_102.parquet:Denom`<br>`density_pure_103.parquet:Denom` |
| `http://dbpedia.org/ontology/arm` | 69 | `believer.parquet:left_arm_elbow_bend`<br>`believer.parquet:left_arm_elbow_rotate`<br>`believer.parquet:left_arm_gripper_finger` |
| `http://dbpedia.org/ontology/grossDomesticProductNominalPerCapita` | 69 | `Bosnia%20and%20Herzegovina.parquet:Domestic general government health expenditure per capita, PPP (current international $)`<br>`Bosnia%20and%20Herzegovina.parquet:External health expenditure per capita, PPP (current international $)`<br>`Bosnia%20and%20Herzegovina.parquet:Gross national expenditure (constant LCU)` |
| `http://dbpedia.org/ontology/role` | 69 | `bank-payment_permission_1.parquet:roleName`<br>`bank-payment_permission_11.parquet:roleName`<br>`bank-payment_permission_14.parquet:roleName` |
| `http://dbpedia.org/ontology/water` | 69 | `data_science-data.parquet:air-to-water`<br>`data_science-data.parquet:chilled_water`<br>`data_science-data.parquet:hot_water` |
| `http://dbpedia.org/ontology/typeOfStorage` | 68 | `PDS4_IMG_SURFACE_1D00_1210.parquet:Unit of Measure Type`<br>`PDS4_DISP_1D00_1500_1.parquet:Unit of Measure Type`<br>`PDS4_DISP_1E00_1500_1.parquet:Unit of Measure Type` |
| `http://dbpedia.org/ontology/network` | 67 | `hay_capture-01.kismet.parquet:Network`<br>`hay_capture-02.kismet.parquet:Network`<br>`hay_capture-07.kismet.parquet:Network` |
| `http://dbpedia.org/ontology/requirement` | 66 | `2014-02-24-checklist-iemma-morando-osella.parquet:Requirement`<br>`execution_commands_python.parquet:mandatory parameters`<br>`expedition.parquet:requirement` |
| `http://dbpedia.org/ontology/numberOfVolunteers` | 65 | `Argentina_5.parquet:volunteers`<br>`Argentina_6.parquet:volunteers`<br>`Australia_1.parquet:volunteers` |
| `http://dbpedia.org/ontology/titleDouble` | 64 | `Test_AppendTable_ColumnsMatch_ColumnFilters_out_1.parquet:Double`<br>`Test_AppendTable_ColumnsMatch_out.parquet:Double`<br>`Test_AppendTable_IncludeColumns_ColumnMap_ColumnFilters_out.parquet:Double` |
| `http://dbpedia.org/ontology/wikiPageRevisionID` | 64 | `0fa538b1-2a2f-4874-9234-ed8b884cedc2.parquet:Wikipage revision ID`<br>`0fa538b1-2a2f-4874-9234-ed8b884cedc2_1.parquet:Wikipage revision ID`<br>`19b692d3-2ab3-4f66-88f9-960f9e7e17e6.parquet:Wikipage revision ID` |
| `http://dbpedia.org/ontology/numberOfPersonInOccupation` | 63 | `%E4%BA%BA%E6%95%99%E7%89%88%E9%AB%98%E4%B8%AD%E8%8B%B1%E8%AF%AD7%20-%20%E9%80%89%E4%BF%AE.parquet:n. the condition of being unable to perform as a consequence of physical or mental unfitness`<br>`words_13.parquet:Noun:a particular geographical region of indefinite boundary (usually serving some special purpose or distinguished by its people or culture or geography`<br>`14322425_0_281964118534741951.parquet:Average number of persons per household` |
| `http://dbpedia.org/ontology/numberOfRun` | 63 | `gru_16.parquet:run`<br>`gru_16_1.parquet:run`<br>`gru_32.parquet:run` |
| `http://dbpedia.org/ontology/percentageLiteracyWomen` | 63 | `recent-grads_10.parquet:ShareWomen`<br>`recent-grads_120.parquet:ShareWomen`<br>`recent-grads_142.parquet:ShareWomen` |
| `http://dbpedia.org/ontology/species` | 63 | `%20central%20african%20republic.parquet:Plant species (higher), threatened`<br>`Bosnia%20and%20Herzegovina.parquet:Plant species (higher), threatened`<br>`Greenland.parquet:Plant species (higher), threatened` |
| `http://dbpedia.org/ontology/vehicle` | 63 | `CrimeOneYearofData_clean_1.parquet:Motor vehicle theft rate`<br>`CrimeOneYearofData_clean_6.parquet:Motor vehicle theft rate`<br>`crime_data_48.parquet:motor vehicle theft` |
| `http://dbpedia.org/ontology/averageSpeed` | 62 | `lac_201027.parquet:max_average_uptake`<br>`lac_201027_1.parquet:max_average_uptake`<br>`BSM_Trip_Summary_File_04_11_13.parquet:AverageSpeed (m/s)` |
| `http://dbpedia.org/ontology/deathAge` | 62 | `meta_2020-05-19_aw.parquet:death_gender`<br>`meta_2020-05-20_aw.parquet:death_gender`<br>`meta_2020-05-21_aw.parquet:death_gender` |
| `http://dbpedia.org/ontology/severeCases` | 62 | `COPD_91.parquet:Severe`<br>`COPD_91.parquet:Severe Adjusted`<br>`COPD_92.parquet:Severe` |
| `http://dbpedia.org/ontology/activity` | 61 | `duckduckgo_Android_develop.parquet:activity_add_widget_instructions`<br>`top_news_analysis.parquet:activityHotness`<br>`Final%20-%20Copy.parquet:Improvement-Driven Opportunity Entrepreneurial Activity: Relative Prevalence` |
| `http://dbpedia.org/ontology/credit` | 60 | `events-all.parquet:credit`<br>`translate_287.parquet:Credit Card Type`<br>`translate_323.parquet:Credit Card Type` |
| `http://dbpedia.org/ontology/skinColor` | 60 | `asteroids.parquet:texture`<br>`planets_3000bc_to_3000ad.parquet:texture`<br>`ddf--concepts.parquet:color` |
| `http://dbpedia.org/ontology/treatment` | 60 | `Consolidated-Orius-Development-JB2020-Raw-Development-Openrefined.parquet:Treatment`<br>`vaccine%20study.parquet:Treatment`<br>`Arizona_1.parquet:flu treatment` |
| `http://dbpedia.org/ontology/hairColor` | 59 | `%E6%80%BB%E8%A3%85.SLDASM.parquet:Color Blue`<br>`%E6%80%BB%E8%A3%85.SLDASM.parquet:Color Green`<br>`3_DOF_ARM_description.parquet:Color Blue` |
| `http://dbpedia.org/ontology/upperAge` | 59 | `COPD_91.parquet:Fatality upper bound`<br>`COPD_91.parquet:Severe upper bound`<br>`COPD_92.parquet:Fatality upper bound` |
| `http://dbpedia.org/ontology/associationOfLocalGovernment` | 58 | `uss2012.parquet:PharmaLeaks: Understanding the Business of Online Pharmaceutical Affiliate Programs.`<br>`Final%20-%20Copy.parquet:Governmental support and policies`<br>`NSL_1.parquet:“It's all about the Federal Reserve meeting driving the markets today,” said Brad Peterson, regional portfolio adviser at Northern Trust Wealth Management.` |
| `http://dbpedia.org/ontology/features` | 58 | `covid-tools-dashboards.parquet:Desirable Features/Recommendations`<br>`configurations_16.parquet:regressor:extra_trees:max_features`<br>`configurations_16.parquet:regressor:random_forest:max_features` |
| `http://dbpedia.org/ontology/disease` | 57 | `What%20is%20the%20best%20method%20to%20combat%20the%20hypercoagulable%20state%20seen%20in%20COVID-19__1.parquet:Severity of Disease`<br>`What%20is%20the%20efficacy%20of%20novel%20therapeutics%20being%20tested%20currently_.parquet:Severity of Disease`<br>`20200614_05-03-57_IL1b_1.parquet:Disease` |
| `http://dbpedia.org/ontology/originalTitle` | 57 | `pairs_icehockey2.parquet:original_utt_prev`<br>`Lookup_StandardizeVariableNames.parquet:Original`<br>`10_cloverfield_lane.parquet:Original` |
| `http://dbpedia.org/ontology/southWestPlace` | 57 | `northernberry.parquet:Northern`<br>`U.S.%20Census%20Data_1.parquet:North Carolina`<br>`U.S.%20Census%20Data_1.parquet:South Carolina` |
| `http://dbpedia.org/ontology/winsInEurope` | 57 | `KEP.parquet:Korea's KHNP keen on clean energy, nuclear in Poland`<br>`PPG_1.parquet:Akzo Nobel concedes to PPG in battle over Finland's Tikkurila`<br>`Dhairya_XBOX_GameSales_processed.parquet:Europe` |
| `http://dbpedia.org/ontology/population` | 56 | `crime_data_48.parquet:population`<br>`joined_toronto_neighborhood_4final_analysis-a.parquet:Population`<br>`test_state_data_basic.parquet:Population` |
| `http://dbpedia.org/ontology/technique` | 56 | `HSSingLog20180225-1.parquet:Technique`<br>`HSSingLog20180227-1.parquet:Technique`<br>`HSSingLog20180301-1.parquet:Technique` |
| `http://dbpedia.org/ontology/partner` | 55 | `copo_schema.parquet:dc.contributor type=Partner`<br>`crm_event_12.parquet:clientPartner.importId`<br>`crm_event_12.parquet:contactPartner.importId` |
| `http://dbpedia.org/ontology/votesAgainst` | 55 | `oogl.parquet:votes`<br>`tt0208614.parquet:votes`<br>`tt1486217.parquet:votes` |
| `http://dbpedia.org/ontology/elementBlock` | 54 | `loading_profile_11.parquet:WebCore::RenderBlock::insertIntoFloatingObjectMaps (period)`<br>`loading_profile_11.parquet:WebCore::RenderBlock::logicalRightFloatOffsetForLine const (period)`<br>`loading_profile_11.parquet:WebCore::RenderBlock::nextFloatLogicalBottomBelow const (period)` |
| `http://dbpedia.org/ontology/genre` | 54 | `ClaDan.parquet:genre`<br>`ClaDan_1.parquet:genre`<br>`Movie_Clean.parquet:genre_ids` |
| `http://dbpedia.org/ontology/lowerAge` | 54 | `COPD_91.parquet:Severe lower bound`<br>`COPD_92.parquet:Severe lower bound`<br>`Diabetes_2.parquet:Severe lower bound` |
| `http://dbpedia.org/ontology/shipCrew` | 53 | `l36%20(Kestrel%20Cruiser%20C%20-%20NORMAL%20AE).parquet:NEARBY SHIP DRONE PARTS`<br>`l36%20(Kestrel%20Cruiser%20C%20-%20NORMAL%20AE).parquet:NEARBY SHIP MEDBAY SYSTEM CAPACITY`<br>`l36%20(Kestrel%20Cruiser%20C%20-%20NORMAL%20AE).parquet:NEARBY SHIP PILOT SYSTEM DAMAGE` |
| `http://dbpedia.org/ontology/skills` | 53 | `elements_mapping_features.parquet:abilities_elements`<br>`elements_mapping_features.parquet:skills_encoding`<br>`Cognitive_Abilities.parquet:Cognitive Abilities` |
| `http://dbpedia.org/ontology/rightAscension` | 52 | `metadata_98.parquet:right`<br>`angle-changes-attack-patch10pgd001.parquet:right_knee`<br>`angle-changes-attack-patch10pgd001_2.parquet:right_knee` |
| `http://dbpedia.org/ontology/fuel` | 51 | `scenario_uniq_values.parquet:conv_fuel_emissions_factor`<br>`scenario_uniq_values.parquet:soln_fuel_efficiency_factor`<br>`scenario_uniq_values.parquet:soln_fuel_emissions_factor` |
| `http://dbpedia.org/ontology/head` | 51 | `believer.parquet:head_pan`<br>`believer.parquet:head_sidetilt`<br>`believer_4.parquet:head_pan` |
| `http://dbpedia.org/ontology/populationUrban` | 51 | `%20central%20african%20republic.parquet:Urban population`<br>`Bosnia%20and%20Herzegovina.parquet:Urban population`<br>`Greenland.parquet:Urban population` |
| `http://dbpedia.org/ontology/firstWinner` | 50 | `test_full_csv_10.parquet:winner`<br>`test_full_csv_10.parquet:winner_dialogue`<br>`test_full_csv_11.parquet:winner` |
| `http://dbpedia.org/ontology/nameInJapanese` | 50 | `table2.parquet:Variable name in CDS`<br>`japanese.parquet:Japanese`<br>`leeds-tagged-100-f.parquet:CD_IN_NN` |
| `http://dbpedia.org/ontology/protein` | 50 | `Scott2010_chlor_inhibition_minimal.parquet:RNA_protein_ratio`<br>`si2017_si.parquet:RNA/protein`<br>`lab_502.parquet:creatine_kinase` |
| `http://dbpedia.org/ontology/schoolNumber` | 50 | `d0543f4e-166d-4a2c-a8ef-9456c3cad10d_tags.parquet:good school\close family\same friend\same good friend\major\great school\favorite part\high acceptance rate\large university`<br>`eretrea.parquet:Adjusted net enrollment rate, primary (% of primary school age children)`<br>`korea%20dem.%20people%20rep..parquet:Adjusted net enrollment rate, primary (% of primary school age children)` |
| `http://dbpedia.org/ontology/iconographicAttributes` | 49 | `tariff_property_list_1.parquet:demand attributes`<br>`ibm-fun-21-05-14.parquet:attribute`<br>`CALLOUT_DM_1.parquet:Attribute` |
| `http://dbpedia.org/ontology/specialEffects` | 49 | `How_does_temperature_and_humidity_affect_the_tramsmission_of_2019_nCoV_.parquet:Effect`<br>`Seasonality_of_transmission.parquet:Effect`<br>`disparity_results_for_paper_mgbm_xnn_hmda_simu.parquet:Marginal Effects` |
| `http://dbpedia.org/ontology/diameter` | 48 | `Solar%20System.parquet:Diameter (km)`<br>`Solar_System.parquet:Diameter (km)`<br>`planets%2Bradii.parquet:diameter` |
| `http://dbpedia.org/ontology/engineer` | 48 | `lda_speciality_keywords.parquet:Software Engineering`<br>`lda_speciality_keywords.parquet:Software Engineering`<br>`PREDICTION_Software%20Engineering%20Project%20I.parquet:Intermediate Software Engineering` |
| `http://dbpedia.org/ontology/purpose` | 48 | `dataset-req-attrs.parquet:Purpose`<br>`dataset-req-attrs_1.parquet:Purpose`<br>`dataset-req-attrs.parquet:Purpose` |
| `http://dbpedia.org/ontology/statusManager` | 48 | `Citi_glassdoor_ratings.parquet:employee_status`<br>`WellsFargo_glassdoor_ratings.parquet:employee_status`<br>`Zurich_glassdoor_ratings.parquet:employee_status` |
| `http://dbpedia.org/ontology/woRMS` | 48 | `Test%20Code.parquet:RMS Error`<br>`malawi.parquet:wi`<br>`SpawnSeisHydrophoneDeploymentMetaData.parquet:LONmin` |
| `http://dbpedia.org/ontology/areaTotalRanking` | 47 | `2007-03-08_1.parquet:total_job_valuation`<br>`2007-04-12.parquet:total_job_valuation`<br>`2009-01-26_1.parquet:total_job_valuation` |
| `http://dbpedia.org/ontology/knownFor` | 47 | `failed_companies_autopsyio_dataset_3.parquet:Reason_for_Failure`<br>`modified_flux_ds.parquet:for`<br>`GB2_xtab.parquet:for` |
| `http://dbpedia.org/ontology/totalTracks` | 47 | `tested_numbers_icmr_data_2.parquet:Total Samples Tested`<br>`tests_day_wise_5.parquet:Total Samples Tested`<br>`tests_day_wise_6.parquet:Total Samples Tested` |
| `http://dbpedia.org/ontology/voltageOfElectrification` | 47 | `InstrumentModel.parquet:im/voltage`<br>`MCU_PCB_V2.parquet:INSOLATION-VOLTAGE`<br>`MCU_PCB_V2.parquet:VOLTAGE` |
| `http://dbpedia.org/ontology/collection` | 46 | `energy-term-datapoints.parquet:Collection`<br>`energy-term-datapoints_1.parquet:Collection`<br>`COVID19_Metadata_v0.95_Sample_Complete_v5.parquet:collection protocol` |
| `http://dbpedia.org/ontology/land` | 46 | `housing_11.parquet:land_zone`<br>`housing_12.parquet:land_zone`<br>`%20central%20african%20republic.parquet:Arable land (hectares)` |
| `http://dbpedia.org/ontology/personFunction` | 46 | `74b7f0fe-68d3-4412-b14c-f52bff042c9a_tags.parquet:really applicable\limit online function`<br>`Analysis%20Log.parquet:function`<br>`DQA_Check_Type_Inventory_2.parquet:function` |
| `http://dbpedia.org/ontology/tempPlace` | 46 | `dat-RESULTS2018-05-09.parquet:temp_CV`<br>`dat-RESULTS2018-05-09.parquet:temp_SD`<br>`tdata_var.parquet:temp` |
| `http://dbpedia.org/ontology/tvShow` | 46 | `funcs_by_address_197.parquet:memShow_FW`<br>`funcs_by_address_199.parquet:memShow_FW`<br>`funcs_by_address_242.parquet:memShow_FW` |
| `http://dbpedia.org/ontology/chemicalFormula` | 45 | `2015-03-31%2006_35_01.parquet:beans_chemical_benefits`<br>`2015-03-31%2006_35_01.parquet:beans_chemical_supplier`<br>`2015-03-31%2006_35_01.parquet:beans_chemical_use` |
| `http://dbpedia.org/ontology/militaryBranch` | 45 | `randoop-600.parquet:branch_coverage`<br>`Top%20Tree.parquet:Branch`<br>`randoop-180.parquet:branch_coverage` |
| `http://dbpedia.org/ontology/populationRural` | 45 | `%20central%20african%20republic.parquet:Rural population`<br>`Bosnia%20and%20Herzegovina.parquet:Rural population`<br>`Greenland.parquet:Rural population` |
| `http://dbpedia.org/ontology/relative` | 45 | `NIST2_Sandia_Helium_Plume_dataplot_config.parquet:Exp_Error_Relative`<br>`data_science-data.parquet:relative_humidity`<br>`data_science-data_1.parquet:relative_humidity` |
| `http://dbpedia.org/ontology/stateOfOriginYear` | 45 | `%20central%20african%20republic.parquet:Over-age students, primary (% of enrollment)`<br>`Bosnia%20and%20Herzegovina.parquet:Over-age students, primary (% of enrollment)`<br>`andorra.parquet:Over-age students, primary (% of enrollment)` |
| `http://dbpedia.org/ontology/usedInWar` | 45 | `MO.parquet:ramping up the “ war on drugs ” locking up non - violent offenders for life . biden did not oppose these things the democratic party used it as a platform to show they were tough - on - crime`<br>`scottdonaldson.parquet:I remember when we used to sit in the government yard in Tre...`<br>`HUN.parquet:BRIEF-Huntsman Says Will Begin Making Hydro Alcoholic Solution To Produce Hand Sanitizer In Swiss Canton Of Vaud And General Hospital In Lausanne, Switzerland` |
| `http://dbpedia.org/ontology/brainInfoType` | 44 | `Contactlab_Template_1.parquet:Template Type Information`<br>`TC_366.parquet:Avg_deviceInformationId DoubleType`<br>`TC_366_2.parquet:Avg_deviceInformationId DoubleType` |
| `http://dbpedia.org/ontology/building` | 44 | `data_science-data.parquet:building_envelope`<br>`data_science-data.parquet:building_form`<br>`data_science-data.parquet:building_geometry` |
| `http://dbpedia.org/ontology/chemSpiderId` | 44 | `2012_raw.parquet:spider_j`<br>`projections_2020-11-09_2020-11-14.parquet:JHU_UNC_GAS-StatMechPool`<br>`projections_2020-10-26_2020-10-31.parquet:JHU_UNC_GAS-StatMechPool` |
| `http://dbpedia.org/ontology/domain` | 44 | `wikipedia.org.parquet:domain`<br>`gender_equality_index_gpg_2005.parquet:Sub-domain score`<br>`gender_equality_index_gpg_2012.parquet:Sub-domain score` |
| `http://dbpedia.org/ontology/media` | 44 | `Final%20-%20Copy.parquet:corr_media`<br>`DFS.parquet:media`<br>`dfs-prepos.parquet:media` |
| `http://dbpedia.org/ontology/acceleration` | 43 | `List_of_Intel_graphics_processing_units-2.parquet:Hardware acceleration`<br>`List_of_Intel_graphics_processing_units-3.parquet:Hardware acceleration`<br>`List_of_Intel_graphics_processing_units_10.parquet:Hardware acceleration` |
| `http://dbpedia.org/ontology/course` | 43 | `39154acd-95a7-47a5-bc9e-714be9e95f39_tags.parquet:online course`<br>`53911552-0352-4bed-979f-b0367cbbdd0b_tags.parquet:online course\more rigorous`<br>`f20e78ac-604d-45fc-9e12-11aff808c62d_tags.parquet:online course\online course\genal education course\interested\online experience\as easy\bearable` |
| `http://dbpedia.org/ontology/eurobabeIndexId` | 43 | `test_data.parquet:cpi_idx`<br>`test_data_1.parquet:cpi_idx`<br>`training_data.parquet:cpi_idx` |
| `http://dbpedia.org/ontology/filmColourType` | 43 | `tab_resume.parquet:black box type`<br>`tab_resume_1.parquet:black box type`<br>`tab_resume_11.parquet:black box type` |
| `http://dbpedia.org/ontology/numberOfResource` | 43 | `%20central%20african%20republic.parquet:Total natural resources rents (% of GDP)`<br>`Bosnia%20and%20Herzegovina.parquet:Total natural resources rents (% of GDP)`<br>`Greenland.parquet:Total natural resources rents (% of GDP)` |
| `http://dbpedia.org/ontology/related` | 43 | `Frederick-P-Brooks-Jr.parquet:survey presence related concept`<br>`Frederick-P-Brooks-Jr.parquet:survey presence related concept`<br>`Complaint.parquet:relatedLot` |
| `http://dbpedia.org/ontology/subjectOfPlay` | 43 | `snixtho.parquet:The rules of the Hunger Games are simple. In punishment for ...`<br>`snixtho.parquet:The rules of the Hunger Games are simple. In punishment for ...`<br>`2001_73.parquet:All Four Subject Areas (percent)` |
| `http://dbpedia.org/ontology/dutchWinkelID` | 42 | `dutch-dragon.parquet:Dutch`<br>`dutch-haze.parquet:Dutch`<br>`vectors_topic_2_by_languages.parquet:[Dutch]_Freq` |
| `http://dbpedia.org/ontology/filmAudioType` | 42 | `yt_comments_test.parquet:video`<br>`Exp1_CodeBook.parquet:Video watched. FRONTAL VIDEO or PROFILE VIDEO`<br>`web-files-small-metadata.parquet:audioCompressor` |
| `http://dbpedia.org/ontology/government` | 42 | `andorra.parquet:Government expenditure on education, total (% of government expenditure)`<br>`cleanedEconFreedomData.parquet:Government Integrity`<br>`cleanedEconFreedomData_2.parquet:Government Integrity` |
| `http://dbpedia.org/ontology/britishOpen` | 41 | `2014-02-24-checklist-iemma-morando-osella.parquet:Open-DAI`<br>`SampleMarketHoursDatabase_2.parquet:fri_open`<br>`SampleMarketHoursDatabase_2.parquet:thu_open` |
| `http://dbpedia.org/ontology/countryWithFirstAstronaut` | 41 | `squarecuber.parquet:And Aunt Zelda all the women looked like you and Uncle Bob a...`<br>`AIW.parquet:"The first quarter marked an activity inflection for the international markets, while North America continued to stage a healthy recovery," Halliburton Chief Executive Jeff Miller said.`<br>`b8c16d9f-d907-47c5-9c0a-1d2a179e36bc_tags.parquet:native history\native american history\native history\native american history\native government\native american government\easy access\amazing\right area\busy\grateful` |
| `http://dbpedia.org/ontology/hipSize` | 41 | `Praesepe-tgas-apassdr9-members.parquet:hip`<br>`Praesepe-tgas-apassdr9-members_1.parquet:hip`<br>`angle-changes-attack-patch10pgd001.parquet:right_hip` |
| `http://dbpedia.org/ontology/retentionTime` | 41 | `Dementia_cpdMetadata.parquet:Retention Time`<br>`bulk-moisture-density%20(15).parquet:moisture and density::registration time-stamp`<br>`bulk-moisture-density%20(24).parquet:moisture and density::registration time-stamp` |
| `http://dbpedia.org/ontology/symptom` | 41 | `Arizona_1.parquet:symptoms`<br>`Arizona_1.parquet:the flu symptoms`<br>`Colorado_1.parquet:flu a symptoms` |
| `http://dbpedia.org/ontology/perCapitaIncome` | 40 | `house_prices.parquet:per capita crime rate by town`<br>`housing_data.parquet:Per Capita Crime Rate`<br>`scenario_uniq_values.parquet:seq_rate_per_regime` |
| `http://dbpedia.org/ontology/resolution` | 40 | `ESDL_metadata_variables.parquet:time_coverage_resolution`<br>`MCU_PCB_V2.parquet:ADC-RESOLUTION`<br>`Complaint.parquet:resolution` |
| `http://dbpedia.org/ontology/serviceModule` | 40 | `01_firm_2.parquet:service_subspecialty_assignment_id`<br>`HaitiLNSP_Role_Module_1.parquet:system_module_id`<br>`HaitiLNSP_Role_Module_2.parquet:system_module_id` |
| `http://dbpedia.org/ontology/vehiclesPerDay` | 40 | `scenario_uniq_values.parquet:conv_emissions_per_funit`<br>`scenario_uniq_values.parquet:conv_emissions_per_funit`<br>`standard_player_pitching_stats_1872.parquet:bases_on_balls_per_nine` |
| `http://dbpedia.org/ontology/world` | 40 | `Dhairya_XBOX_GameSales_processed.parquet:Rest of World`<br>`Dhairya_XBOX_GameSales_processed_1.parquet:Rest of World`<br>`Dhairya_XBOX_GameSales_processed_2.parquet:Rest of World` |
| `http://dbpedia.org/ontology/absoluteMagnitude` | 39 | `evaluations_metrics.parquet:absolute_diff`<br>`ArtifactInfo_1.parquet:EnchantmentMagnitude`<br>`ArtifactInfo_1.parquet:EnchantmentMagnitude` |
| `http://dbpedia.org/ontology/hraState` | 39 | `projections_2020-04-27_2020-05-02_1.parquet:MIT_Sak_State`<br>`projections_2020-04-27_2020-05-09_1.parquet:MIT_Sak_State`<br>`projections_2020-05-11_2020-05-23_1.parquet:MIT_Sak_State` |
| `http://dbpedia.org/ontology/nextEvent` | 39 | `CloseFrameworkAgreementUA.parquet:next_check`<br>`CloseFrameworkAgreementUA_1.parquet:next_check`<br>`CloseFrameworkAgreementUA_3.parquet:next_check` |
| `http://dbpedia.org/ontology/populationPctWomen` | 39 | `recent-grads_10.parquet:Women`<br>`recent-grads_120.parquet:Women`<br>`recent-grads_142.parquet:Women` |
| `http://dbpedia.org/ontology/calculationNeeds` | 38 | `Biomass_Prices.parquet:Conversion calculation`<br>`Growth_Rate_of_Land_Degradation.parquet:Conversion calculation`<br>`SOLUTION_First_Cost_per_Implementation_Unit_1.parquet:Conversion calculation` |
| `http://dbpedia.org/ontology/contest` | 38 | `moves_40.parquet:contest_effect_id`<br>`moves_63.parquet:contest_effect_id`<br>`moves_79.parquet:contest_effect_id` |
| `http://dbpedia.org/ontology/hasInsidePlace` | 38 | `catalyst0435.parquet:I suddenly had the impression that I had been left all alone...`<br>`hanlel.parquet:Everything that has transpired has done so according to my d...`<br>`mumeigaf.parquet:Everything we hold dear likewise resolves into its original ...` |
| `http://dbpedia.org/ontology/industry` | 38 | `BostonHousing.parquet:industry`<br>`EWI.parquet:industry`<br>`EWI_3.parquet:industry` |
| `http://dbpedia.org/ontology/isHandicappedAccessible` | 38 | `train_filtrado.parquet:Census_IsFlightsDisabled`<br>`train_filtrado.parquet:IsProtected`<br>`29bbbb2e-4110-40e5-ac83-90507d82acbc_tags.parquet:amazing\so inviting\beautiful\safe\relatable\other student\motivated` |
| `http://dbpedia.org/ontology/isoCode` | 38 | `out-0.05.parquet:IsoRegression`<br>`out-0.05_4.parquet:IsoRegression`<br>`out-0.05_6.parquet:IsoRegression` |
| `http://dbpedia.org/ontology/magazine` | 38 | `1009028042208-EW.parquet:mag`<br>`1018049019401-RRab.parquet:mag`<br>`1018059003526-RRab.parquet:mag` |
| `http://dbpedia.org/ontology/numberOfResourceOfClass` | 38 | `What%20is%20the%20efficacy%20of%20novel%20therapeutics%20being%20tested%20currently_.parquet:Primary Endpoint(s) of Study`<br>`Bosnia%20and%20Herzegovina.parquet:Benefit incidence of social protection and labor programs to poorest quintile (% of total SPL benefits)`<br>`Bosnia%20and%20Herzegovina.parquet:Total debt service (% of GNI)` |
| `http://dbpedia.org/ontology/subtitle` | 38 | `Home%20Page_2.parquet:Subtitle`<br>`Technology%20News.parquet:Subtitle`<br>`NIST2_Sandia_Helium_Plume_dataplot_config.parquet:Plot_Subtitle` |
| `http://dbpedia.org/ontology/highestBuildingInYear` | 36 | `answer_1_2_3.parquet:Average_tenure_in_days`<br>`SID_1.parquet:In a recent earnings conference call, executives said CSN had earlier this month carved out a separate subsidiary for the cement operations.`<br>`HDB_1.parquet:India on Monday reported its tenth record daily increase in coronavirus cases in eleven days, with the capital city of New Delhi — currently under a weekend curfew — reporting a shortage in critical-care beds.` |
| `http://dbpedia.org/ontology/producedBy` | 36 | `afg0national.parquet:ByVariableLabel`<br>`copo_schema.parquet:dc.relation isReferencedBy`<br>`R05_CSV12_TOURIST_ARRIVAL_AND_AVERAGE_LENGTH_OF_STAY_(1964-2020).parquet:by_air_percent` |
| `http://dbpedia.org/ontology/restriction` | 36 | `Chemical%20and%20Biomolecular%20Engineering%20(CBE)-courses.parquet:restriction`<br>`RawDataProcessor_params_2.parquet:restrictions`<br>`parameters_28.parquet:restrictions` |
| `http://dbpedia.org/ontology/shipDisplacement` | 36 | `l36%20(Kestrel%20Cruiser%20C%20-%20NORMAL%20AE).parquet:NEARBY SHIP ARTILLERY SYSTEM DAMAGE`<br>`l36%20(Kestrel%20Cruiser%20C%20-%20NORMAL%20AE).parquet:NEARBY SHIP ENGINES CAPACITY`<br>`l36%20(Kestrel%20Cruiser%20C%20-%20NORMAL%20AE).parquet:NEARBY SHIP ENGINES DAMAGE` |
| `http://dbpedia.org/ontology/barPassRate` | 35 | `1601391354_Linux_runs.parquet:pass_custom`<br>`1601391356_Linux_runs.parquet:pass_custom`<br>`metrics_aggregates_1.parquet:false_negative_rate_at_optimal_threshold` |
| `http://dbpedia.org/ontology/command` | 35 | `Metrics2019-08-16-10-30-24-574_1.parquet:COMMAND_RATE`<br>`Metrics2019-08-16-10-30-24-574_1.parquet:Commands`<br>`Metrics2019-08-25-12-13-05-802.parquet:COMMAND_RATE` |
| `http://dbpedia.org/ontology/homeStadium` | 35 | `nba_20141105_total.parquet:home_bookmaker_line`<br>`nba_20150220_total.parquet:home_bookmaker_line`<br>`nba_20151127_total.parquet:home_bookmaker_line` |
| `http://dbpedia.org/ontology/numberOfFilms` | 35 | `ComingOfAge.parquet:-= Coming of Age Films`<br>`ComingOfAge_10.parquet:-= Coming of Age Films`<br>`ComingOfAge_11.parquet:-= Coming of Age Films` |
| `http://dbpedia.org/ontology/symbol` | 35 | `DUE.DE.parquet:symbol`<br>`E8X.DE.parquet:symbol`<br>`ENUR.DE.parquet:symbol` |
| `http://dbpedia.org/ontology/actScore` | 34 | `1996.parquet:Average ACT composite scores`<br>`1998_13.parquet:Average ACT composite scores`<br>`2001_73.parquet:Average ACT composite scores` |
| `http://dbpedia.org/ontology/documentDesignation` | 34 | `Agreement_10.parquet:documents`<br>`Agreement_13.parquet:documents`<br>`Award.parquet:documents` |
| `http://dbpedia.org/ontology/projectObjective` | 34 | `covid-tools-dashboards.parquet:Description or Aims/Objectives`<br>`ContainerDesign.parquet:objective`<br>`lesson_16.parquet:lesson_objectives` |
| `http://dbpedia.org/ontology/show` | 34 | `activity_template.parquet:is_show`<br>`days_of_week.parquet:show_order`<br>`days_of_week.parquet:show_select_match` |
| `http://dbpedia.org/ontology/appearance` | 33 | `survey_274.parquet:appearance`<br>`survey_276.parquet:appearance`<br>`survey_278.parquet:appearance` |
| `http://dbpedia.org/ontology/fileSize` | 33 | `Questions_11.parquet:imageFile`<br>`SCORESeasonal_allvars_1.parquet:dataFile`<br>`SCORESeasonal_conversion.parquet:dataFile` |
| `http://dbpedia.org/ontology/projectType` | 33 | `failure_startup_autopsy.parquet:Last_funding_type`<br>`37c83e866049a80b0b525cb002568ee8f9950e01c9be065488bab30a8206f725.parquet:planning-permission-type`<br>`9a85055294c6d5217adf020b1d07ae91d7fe0e121f2bd95e090b91684d93a7c0.parquet:developer-agreement-type` |
| `http://dbpedia.org/ontology/spokenIn` | 33 | `citeSents_5.parquet:In fact`<br>`stats_24.parquet:No._words_in_minutes`<br>`stats_27.parquet:No._words_in_minutes` |
| `http://dbpedia.org/ontology/statValue` | 33 | `WorldBankIndicators_prod.parquet:ExistingStatVar`<br>`WorldBankIndicators_prod.parquet:ExistingStatVar`<br>`evaluations_metrics.parquet:test_stat` |
| `http://dbpedia.org/ontology/unknownOutcomes` | 33 | `ardupilot_texts_V3.3.parquet:MAV_SEVERITY_UNKNOWN`<br>`ardupilot_texts_V3.3_1.parquet:MAV_SEVERITY_UNKNOWN`<br>`dictionary_1_manual.parquet:outcome` |
| `http://dbpedia.org/ontology/approximateCalories` | 32 | `kindergartendish_kindergartendish.parquet:calories`<br>`2019-04-30.parquet:Calories (kcal)`<br>`2019-05-25.parquet:Calories (kcal)` |
| `http://dbpedia.org/ontology/cluster` | 32 | `keywords_sample_1.parquet:Cluster`<br>`baseline_glcm.parquet:ClusterShade`<br>`baseline_glcm.parquet:ClusterTendency` |
| `http://dbpedia.org/ontology/dateAgreement` | 32 | `2017-10-06.parquet:BRIEF-American Airlines - ‍Entered into amended note purchase agreement`<br>`CNN.201606_1.parquet:negotiated the landmark agreement on climate change`<br>`CloseFrameworkAgreementUA.parquet:agreements` |
| `http://dbpedia.org/ontology/imdbId` | 32 | `source_17.parquet:actorConfig_id`<br>`source_18.parquet:actorConfig_id`<br>`source_20.parquet:actorConfig_id` |
| `http://dbpedia.org/ontology/locatedInArea` | 32 | `TableS5.parquet:in_deg`<br>`Test_CheckReservoirStations_out_ContentAreaSeepage.parquet:AREA (ACRE)`<br>`Test_FillDiversionStationsFromHydroBase_ge20200720_out.parquet:AREA (ACRE)` |
| `http://dbpedia.org/ontology/ons` | 32 | `225_81.parquet:Ons`<br>`225_82.parquet:Ons`<br>`263_29.parquet:Ons` |
| `http://dbpedia.org/ontology/season` | 32 | `allDCUniverseEpisodesSorted.parquet:Season`<br>`allDCUniverseEpisodesSorted2020.parquet:Season`<br>`tt0208614.parquet:season` |
| `http://dbpedia.org/ontology/stateOfOriginPoint` | 32 | `source_17.parquet:motionState_timeToCollision`<br>`source_18.parquet:motionState_timeToCollision`<br>`source_20.parquet:motionState_timeToCollision` |
| `http://dbpedia.org/ontology/surfaceArea` | 32 | `data_science-data.parquet:surface`<br>`data_science-data_1.parquet:surface`<br>`metrics_aggregates_1.parquet:area_under_pr_curve` |
| `http://dbpedia.org/ontology/blackLongDistancePisteNumber` | 31 | `dickrichards.parquet:The quick brown fox jumps over the lazy dog.`<br>`lonequid.parquet:The quick brown fox jumps over the lazy dog.`<br>`snakypoutz.parquet:The quick brown fox jumps over the lazy dog.` |
| `http://dbpedia.org/ontology/forces` | 31 | `Bosnia%20and%20Herzegovina.parquet:Armed forces personnel, total`<br>`eretrea.parquet:Armed forces personnel, total`<br>`korea%20dem.%20people%20rep..parquet:Armed forces personnel, total` |
| `http://dbpedia.org/ontology/majorShrine` | 31 | `merged_stars.parquet:semiMajorAxisAu`<br>`baseline_shape.parquet:MajorAxis`<br>`five_valid_2.parquet:semi_major_axis` |
| `http://dbpedia.org/ontology/organSystem` | 31 | `HaitiLNSP_Role_Module_1.parquet:system_role_id`<br>`HaitiLNSP_Role_Module_2.parquet:system_role_id`<br>`Haiti_Role_Module_Rev1.parquet:system_role_id` |
| `http://dbpedia.org/ontology/originalName` | 31 | `pairs_icehockey2.parquet:original_utt`<br>`therps_qaqc_3.parquet:therps_property (ORIGINAL NAME)`<br>`therps_qaqc_3.parquet:therps_property (ORIGINAL NAME)` |
| `http://dbpedia.org/ontology/previousEntity` | 31 | `CALLOUT_DM_1.parquet:Entity`<br>`D_CPT_DM.parquet:Entity`<br>`D_ITEMS_DM.parquet:Entity` |
| `http://dbpedia.org/ontology/academicDiscipline` | 30 | `1c527ae2-eabe-44a1-ade8-39082b0093a2_tags.parquet:academically rigorous\diverse curriculum`<br>`phrase_definitions_3.parquet:academic_doi`<br>`dcac5188-e8f9-49f7-95b0-820d9e90a3d2_tags.parquet:academic college\inclusive environment` |
| `http://dbpedia.org/ontology/notableIdea` | 30 | `6605e427-6846-4779-ad89-f4d25b5ab3cc_tags.parquet:great school\really great way\involved\easy`<br>`failed_companies_autopsyio_dataset_3.parquet:Idea`<br>`subtopic_3.parquet:essential idea` |
| `http://dbpedia.org/ontology/numberOfEmployees` | 30 | `Bosnia%20and%20Herzegovina.parquet:Employers, total (% of total employment) (modeled ILO estimate)`<br>`eretrea.parquet:Employers, total (% of total employment) (modeled ILO estimate)`<br>`korea%20dem.%20people%20rep..parquet:Employers, total (% of total employment) (modeled ILO estimate)` |
| `http://dbpedia.org/ontology/second` | 30 | `iperf_bmx.parquet:bits_per_second (median)`<br>`tf-idf.parquet:half`<br>`catalogue_1.parquet:second` |
| `http://dbpedia.org/ontology/solventWithGoodSolubility` | 30 | `26889b86-f95f-4c46-9156-316289baaa54_tags.parquet:good school\very high standard\extra resource\free\most\lenient\reasonable`<br>`26889b86-f95f-4c46-9156-316289baaa54_tags.parquet:good school\very high standard\extra resource\free\most\lenient\reasonable`<br>`results_with_adjusted_scores.parquet:consistency_with_fov_mor` |
| `http://dbpedia.org/ontology/subOrder` | 30 | `receive_order_base.parquet:inventory_constant_sub`<br>`receive_order_base.parquet:order_point_sub`<br>`es_cl_10.parquet:Cannot use sub-query in order by` |
| `http://dbpedia.org/ontology/topLevelDomain` | 30 | `MCU_PCB_V2.parquet:DRIVE-LEVEL`<br>`data_science-data.parquet:noise_level`<br>`data_science-data_1.parquet:noise_level` |
| `http://dbpedia.org/ontology/usOpenMixed` | 30 | `9da8a7ad-2870-4da1-b5a2-9c4f87f13c9c_tags.parquet:very friendly\open\new idea`<br>`bivhitscar.parquet:Some have a positive vocation for breaking open safes: from ...`<br>`SampleMarketHoursDatabase_2.parquet:sat_ex_open` |
| `http://dbpedia.org/ontology/bestFinish` | 29 | `ee099613-bc91-430e-ab38-47c31a81b9e2_tags.parquet:second semester freshman\illustrious\good choice\good\hard\make sure\proud`<br>`hay_capture-01.kismet.parquet:BestQuality`<br>`hay_capture-02.kismet.parquet:BestQuality` |
| `http://dbpedia.org/ontology/cableCar` | 29 | `csv_lines_12.parquet:cables`<br>`csv_lines_13.parquet:cables`<br>`csv_lines_27.parquet:cables` |
| `http://dbpedia.org/ontology/gradName` | 29 | `grad-students_10.parquet:Grad_share`<br>`grad-students_113.parquet:Grad_share`<br>`grad-students_116.parquet:Grad_share` |
| `http://dbpedia.org/ontology/gradNum` | 29 | `grad-students_10.parquet:Grad_median`<br>`grad-students_113.parquet:Grad_median`<br>`grad-students_116.parquet:Grad_median` |
| `http://dbpedia.org/ontology/hasJunctionWith` | 29 | `waysidekoi.parquet:Common knowledge has bicyclists always riding into the wind,...`<br>`admin_26.parquet:A config with that path already exists`<br>`IVZ_1.parquet:The filing said Trian has held “constructive” discussion with Invesco’s chief executive officer Martin Flanagan and chief financial officer Allison Dukes. At Janus Henderson, Trian has spoken with non-executive chairman Richard Gillingwater and intends to speak with the board and management about strategic and operational initiatives, the filing said.` |
| `http://dbpedia.org/ontology/perimeter` | 29 | `BreastCancer.parquet:perimeter_error`<br>`BreastCancer.parquet:worst_perimeter`<br>`Test_ReadTableFromDBF_2_out.parquet:PERIMETER` |
| `http://dbpedia.org/ontology/redListIdNL` | 29 | `enclosure_lego_parts.parquet:LDrawColorId`<br>`filter_1.parquet:wordIDList`<br>`match_rostersdata.parquet:h_red_card` |
| `http://dbpedia.org/ontology/specialTrial` | 29 | `-1_criterion_02152019_152055.parquet:trial`<br>`1234_criterion_02132019_125303.parquet:trial`<br>`sub-0790_ses06_task-perc-criterion_09062019_114010.parquet:trial` |
| `http://dbpedia.org/ontology/thumbnail` | 29 | `artinSTEM2020.parquet:thumbnail`<br>`CVE-2020-5776_5.parquet:thumbnail`<br>`items_141.parquet:Thumbnail` |
| `http://dbpedia.org/ontology/wikidataSplitIri` | 29 | `Acceleration.parquet:Wikidata`<br>`SCORESeasonal_conversion.parquet:IRI`<br>`Q7.11.parquet:footnote-iri` |
| `http://dbpedia.org/ontology/focus` | 28 | `List%20of%20ideology-related%20papers.parquet:Focus`<br>`nnano-volume14-issue07_1.parquet:focus`<br>`nphys-volume15-issue02.parquet:focus` |
| `http://dbpedia.org/ontology/participant` | 28 | `Azhar%20Satar.parquet:Participant OS`<br>`Azhar%20Satar.parquet:Participant Private ID`<br>`Haikal%20Sharudin.parquet:Participant OS` |
| `http://dbpedia.org/ontology/prominence` | 28 | `prepared_data_1.parquet:Popularity`<br>`prepared_data_2.parquet:Popularity`<br>`Movie_Clean.parquet:popularity` |
| `http://dbpedia.org/ontology/sales` | 28 | `summary_stats.parquet:Days Sales Outstanding`<br>`summary_stats.parquet:Investment purchases and sales`<br>`25_284.parquet:catalog_sales_profit` |
| `http://dbpedia.org/ontology/thumbnailCaption` | 28 | `drawings.parquet:caption`<br>`Scratch_Assay_T=25_Bin2x2_analyzed%20Region%20Single_Scratch_1.parquet:ImageSceneColumn::Image Scene Column Index!!I`<br>`combined_hashtag.parquet:Caption` |
| `http://dbpedia.org/ontology/bedCount` | 27 | `nyc-listings.parquet:beds`<br>`nyc-listings_2.parquet:beds`<br>`nyc-listings_new.parquet:beds` |
| `http://dbpedia.org/ontology/flag` | 27 | `Network_Test_Traffic.parquet:ACK Flag Cnt`<br>`Network_Test_Traffic.parquet:ECE Flag Cnt`<br>`Network_Test_Traffic.parquet:FIN Flag Cnt` |
| `http://dbpedia.org/ontology/grossDomesticProductPurchasingPowerParityPerCapita` | 27 | `data_728.parquet:households_final_demand_solar_thermal_demand`<br>`data_728.parquet:households_final_demand_wood_pellets_households_final_demand_for_space_heating_wood_pellets_parent_share`<br>`data_809.parquet:households_final_demand_wood_pellets_households_final_demand_for_space_heating_wood_pellets_parent_share` |
| `http://dbpedia.org/ontology/notifyDate` | 27 | `leeds-tagged-100-f.parquet:_notify_`<br>`leeds-tagged-100-oq.parquet:_notify_`<br>`reqview-tagged-100-of.parquet:_notify_` |
| `http://dbpedia.org/ontology/block` | 26 | `Consolidated-Orius-Development-JB2020-Raw-Development-Openrefined.parquet:Block`<br>`16-to-19_2021-to-2022_published-12-09-2021_casterbridge-college_12.parquet:Block`<br>`instrumentconfig.parquet:PerBlock` |
| `http://dbpedia.org/ontology/codeSettlement` | 26 | `Covid_Probabilityv1.IAA-02eab525-39de-42ac-a27f-ded7ef4fe780-Tags.parquet:coding_perc_agreement`<br>`Covid_Probabilityv1.IAA-1ec3b603-ba25-42f7-8404-bf47fcdf1f2b-Tags.parquet:coding_perc_agreement`<br>`Covid_Probabilityv1.IAA-3b5cdc96-0bb8-4b25-90b6-6766682b70b9-Tags.parquet:coding_perc_agreement` |
| `http://dbpedia.org/ontology/distanceToLondon` | 26 | `Solar%20System.parquet:Mean Distance from Sun (AU)`<br>`Solar_System.parquet:Mean Distance from Sun (AU)`<br>`solar.parquet:Mean Distance from Sun (AU)` |
| `http://dbpedia.org/ontology/educationSystem` | 26 | `c07e3b65-edb5-4ff1-b272-c679c9cd3750_tags.parquet:online learning system\online orientation\other`<br>`InProgressItems.parquet:System.Effort`<br>`nctq_2019.parquet:Program Performance Measures (General Teacher Preparation Policy)` |
| `http://dbpedia.org/ontology/nameInWadeGilesChinese` | 26 | `SID_1.parquet:JPMorgan, Bradesco working on IPO of Brazilian steelmaker CSN's cement unit, sources say`<br>`2017-05-15.parquet:BRIEF-Omega Advisors Inc takes share stake in Alcoa, Netflix`<br>`2018-05-10.parquet:BRIEF-Apple, Goldman Sachs Team Up On New Credit Card - WSJ` |
| `http://dbpedia.org/ontology/nciId` | 26 | `CombinedLog2021_02_22_16_46_33_423.parquet:NCI`<br>`OutpatientClaims.parquet:NCH_PRMRY_PYR_CLM_PD_AMT`<br>`OutpatientClaims_1.parquet:NCH_PRMRY_PYR_CLM_PD_AMT` |
| `http://dbpedia.org/ontology/asWikiText` | 25 | `train_30.parquet:text_a`<br>`train_31.parquet:text_a`<br>`train_32.parquet:text_a` |
| `http://dbpedia.org/ontology/center` | 25 | `Results-2020-09-14.parquet:CENTER_LAT`<br>`Results-2020-09-14.parquet:CENTER_LON`<br>`Results-2020-09-16.parquet:CENTER_LAT` |
| `http://dbpedia.org/ontology/enemy` | 25 | `l36%20(Kestrel%20Cruiser%20C%20-%20NORMAL%20AE).parquet:ENEMY BOARDING ATTEMPTS`<br>`l36%20(Kestrel%20Cruiser%20C%20-%20NORMAL%20AE).parquet:ENEMY BOARDING STRENGTH`<br>`l28%20(Zoltan%20Cruiser%20B%20-%20NORMAL%20AE).parquet:ENEMY BOARDING ATTEMPTS` |
| `http://dbpedia.org/ontology/entrezgene` | 25 | `configurations_16.parquet:regressor:gaussian_process:alpha`<br>`configurations_46.parquet:regressor:gaussian_process:alpha`<br>`configurations_48.parquet:regressor:gaussian_process:alpha` |
| `http://dbpedia.org/ontology/fat` | 25 | `turkish.parquet:fat`<br>`turkish.parquet:saturated-fat`<br>`turkish.parquet:trans-fat` |
| `http://dbpedia.org/ontology/parkingLotsCars` | 25 | `CloseFrameworkAgreementUA.parquet:lots`<br>`CloseFrameworkAgreementUA_1.parquet:lots`<br>`CloseFrameworkAgreementUA_3.parquet:lots` |
| `http://dbpedia.org/ontology/recoveryCases` | 25 | `data_science-data.parquet:heat_recovery`<br>`data_science-data_1.parquet:heat_recovery`<br>`Arizona_1.parquet:flu recovery` |
| `http://dbpedia.org/ontology/statisticValue` | 25 | `COPD_91.parquet:Fatality p-value`<br>`COPD_92.parquet:Fatality p-value`<br>`Diabetes_2.parquet:Fatality p-value` |
| `http://dbpedia.org/ontology/aircraftAttack` | 24 | `gear.parquet:Slash attack`<br>`gear.parquet:Stab attack`<br>`Vocabulary_set_3.parquet:attack verbally` |
| `http://dbpedia.org/ontology/bodyStyle` | 24 | `sms.template_11.parquet:template_body`<br>`sms.template_5.parquet:template_body`<br>`sms.template_7.parquet:template_body` |
| `http://dbpedia.org/ontology/championInSingleMale` | 24 | `City_of_Seattle_Wages___Comparison_by_Gender__Wage_Progression_Job_Titles_3.parquet:Female to male % rate`<br>`City_of_Seattle_Wages___Comparison_by_Gender__Wage_Progression_Job_Titles_4.parquet:Female to male % rate`<br>`totalsyear.parquet:males_in_manufacturing` |
| `http://dbpedia.org/ontology/enginePower` | 24 | `POR_1.parquet:Portland General Electric shuts Oregon's only coal-fired power plant`<br>`l36%20(Kestrel%20Cruiser%20C%20-%20NORMAL%20AE).parquet:ENGINES POWER`<br>`l36%20(Kestrel%20Cruiser%20C%20-%20NORMAL%20AE).parquet:NEARBY SHIP ENGINES POWER` |
| `http://dbpedia.org/ontology/firstMention` | 24 | `CompiledData-earnings.parquet:Mention_instructions`<br>`Comparison_of_programming_languages_(object-oriented_programming)-5_2.parquet:read-only`<br>`Comparison_of_programming_languages_(object-oriented_programming)-5_2.parquet:write-only` |
| `http://dbpedia.org/ontology/infantMortality` | 24 | `epidemic.parquet:mortality`<br>`NECN_Functional_Table.parquet:MonthlyWoodMortality`<br>`NECN_Functional_Table_1.parquet:MonthlyWoodMortality` |
| `http://dbpedia.org/ontology/largestWin` | 24 | `tradessummary_supertrend14wx5_9_trail_SL_in_system_percentSL1_0.parquet:Largest Loss`<br>`tradessummary_supertrend14wx5_9_trail_SL_in_system_percentSL1_0.parquet:Largest Profit`<br>`tradessummary_supertrend29wx5_5_trail_SL_in_system.parquet:Largest Loss` |
| `http://dbpedia.org/ontology/locationCountry` | 24 | `source_17.parquet:motionState_worldLocationLat`<br>`source_17.parquet:motionState_worldLocationLong`<br>`source_18.parquet:motionState_worldLocationLat` |
| `http://dbpedia.org/ontology/numberSold` | 24 | `goodreads_library_export.parquet:Owned Copies`<br>`2015-03-31%2006_35_01.parquet:beans_sold`<br>`2015-03-31%2006_35_01.parquet:maize_sold` |
| `http://dbpedia.org/ontology/office` | 24 | `regression.parquet:office`<br>`regression.parquet:office_old`<br>`007.parquet:OFFICE AUTOMATION TOOLS` |
| `http://dbpedia.org/ontology/orthologousGene` | 24 | `MG1655-M9-NC_000913_3gb-stationary-37-D-lyxose2_mut.parquet:Gene`<br>`MgSa_GOPathways_1.parquet:Genes`<br>`PGI_mut.parquet:Gene` |
| `http://dbpedia.org/ontology/passengersPerYear` | 24 | `tested_numbers_icmr_data_2.parquet:Tests per million`<br>`tests_day_wise_5.parquet:Tests per million`<br>`tests_day_wise_6.parquet:Tests per million` |
| `http://dbpedia.org/ontology/settlementAttached` | 24 | `data_prep_fp_biomass_resilience.Rmd.parquet:attached_pkgs`<br>`data_prep_mapp_resilience.Rmd.parquet:attached_pkgs`<br>`goal_prep_lsp.Rmd.parquet:attached_pkgs` |
| `http://dbpedia.org/ontology/skiLift` | 24 | `lift_c12.parquet:lift`<br>`colic0-114-noExtend-A-mci=-1_1.parquet:lift`<br>`colic0-120-noExtend-mci=-1_1.parquet:lift` |
| `http://dbpedia.org/ontology/space` | 24 | `nyc-listings.parquet:space`<br>`nyc-listings_2.parquet:space`<br>`nyc-listings_new.parquet:space` |
| `http://dbpedia.org/ontology/synonym` | 24 | `crop_research.parquet:synonym`<br>`eco-robot-template.parquet:Exact Synonym`<br>`eco-robot-template.parquet:Narrow Synonym` |
| `http://dbpedia.org/ontology/climate` | 23 | `numeo.parquet:Climate Index`<br>`2015-03-31%2006_35_01.parquet:implemented_mgt_climate_action`<br>`2015-04-02%2003_27_21.parquet:implemented_mgt_climate_action` |
| `http://dbpedia.org/ontology/configuration` | 23 | `Klarna_Core.parquet:Configure`<br>`Workflow.parquet:configuration`<br>`en_GB_90.parquet:Configure` |
| `http://dbpedia.org/ontology/lastWin` | 23 | `TwoStageBetween_12-attributes_predictions.parquet:last_stage_favours`<br>`TwoStageBetween_19-alternatives_predictions.parquet:last_stage_favours`<br>`TwoStageWithin_12-alternatives_predictions.parquet:last_stage_favours` |
| `http://dbpedia.org/ontology/maintainedBy` | 23 | `copo_schema.parquet:dc.relation isReplacedBy`<br>`copo_schema.parquet:dc.relation isRequiredBy`<br>`importAnalyzerTest_28.parquet:modifiedByUser` |
| `http://dbpedia.org/ontology/officialName` | 23 | `IntoValue2_extended_DC_CTgov_changes.parquet:official_title`<br>`double_check_CTgov_original_results_IRR2%2B3.parquet:official_title`<br>`double_check_CTgov_results_IRR1.parquet:official_title` |
| `http://dbpedia.org/ontology/picture` | 23 | `Project%20Manager%20Survey%20(Responses)%20-%20Form%20Responses%201.parquet:Photo`<br>`kindergartendish_kindergartendish.parquet:picture`<br>`%22airborne_SIGINT%22_2.parquet:pic_path` |
| `http://dbpedia.org/ontology/powerType` | 23 | `results_38.parquet:<nm_connection_get_type@@Base>`<br>`log_7.parquet:Total Reactive Power Present Demand (kVAR)`<br>`mb-010.5C4F18F1_1.log_1.parquet:Total Reactive Power Present Demand (kVAR)` |
| `http://dbpedia.org/ontology/shareSource` | 23 | `newness_representation_1794.parquet:ipcc_share`<br>`newness_representation_1794.parquet:share`<br>`newness_representation_1804.parquet:ipcc_share` |
| `http://dbpedia.org/ontology/wptFinalTable` | 23 | `custom_fetch_apple_search_ads_campaigns.parquet:flx_table`<br>`custom_fetch_apple_search_ads_campaigns.parquet:flx_table`<br>`custom_parameters.parquet:table` |
| `http://dbpedia.org/ontology/closed` | 22 | `public-apis_public-apis_issues.parquet:closed`<br>`public-apis_public-apis_issues.parquet:closedA`<br>`Illegal%20Posting%20of%20Signs.parquet:ClosedPhoto` |
| `http://dbpedia.org/ontology/computingMedia` | 22 | `topics_60.parquet:Social Sciences`<br>`dsn2002.parquet:Dependable Computing and Communications`<br>`007.parquet:COMPUTER NETWORKS AND INTERNET TECHNOLOGIES` |
| `http://dbpedia.org/ontology/documentNumber` | 22 | `individual_sheets_FW.parquet:sheet_step_number`<br>`exampleNaicsScraping.parquet:Document.type`<br>`services_27.parquet:required_documents` |
| `http://dbpedia.org/ontology/fuelCapacity` | 22 | `PBF.parquet:PBF Energy posts smaller loss as travel recovery boosts fuel demand`<br>`data_science-data.parquet:energy_storage`<br>`data_science-data_1.parquet:energy_storage` |
| `http://dbpedia.org/ontology/goldMedalSingle` | 22 | `platinum-purple-kush.parquet:Platinum`<br>`82-84__cologne-berlin.parquet:gold_relationship`<br>`82-84__cologne-berlin_1.parquet:gold_relationship` |
| `http://dbpedia.org/ontology/highestBreak` | 22 | `ship_data_7.parquet:breakProb`<br>`Arizona_1.parquet:break a fever`<br>`Colorado_1.parquet:break a fever` |
| `http://dbpedia.org/ontology/icaoLocationIdentifier` | 22 | `train_filtrado.parquet:Census_OEMModelIdentifier`<br>`linked.parquet:AltIdentifier.local`<br>`thumbs_4.parquet:AltIdentifier.local` |
| `http://dbpedia.org/ontology/inflow` | 22 | `national-sector-wide-20160225-full.parquet:annualCashInflows`<br>`summary_stats.parquet:freeCashFlowOperatingCashFlowRatio`<br>`END_RESULT_DATAFRAME_G2OP_CASE14_REALISTIC.parquet:Delta flows` |
| `http://dbpedia.org/ontology/musicalKey` | 22 | `GlobalLeaderboard0.parquet:cross_experiment_key`<br>`GlobalLeaderboard0.parquet:hyperparameter_key`<br>`RetroCI_Roles_13.parquet:display_key` |
| `http://dbpedia.org/ontology/presentName` | 22 | `test_case_1.parquet:PresentMode`<br>`dice_germanrfc_result.parquet:origin_cf_present_emp_since`<br>`dice_germanrfc_result.parquet:origin_cf_present_res_since` |
| `http://dbpedia.org/ontology/relation` | 22 | `measurements_1.parquet:fact_relationship`<br>`measurements.parquet:fact_relationship`<br>`ADMISSIONS_DM.parquet:inRelationTo` |
| `http://dbpedia.org/ontology/selection` | 22 | `Comparison_of_image_viewers-5_1.parquet:selection (styles)`<br>`Comparison_of_image_viewers_5.parquet:selection (styles)`<br>`disparity_results_mgbm_xnn_hmda_simu.parquet:Selected` |
| `http://dbpedia.org/ontology/templateName` | 22 | `web-files-small-metadata.parquet:template`<br>`LNCaP_CAF_DMSO_vs_docetaxel_linux.parquet:imageFileName`<br>`PageEntity_2.parquet:structureTemplateId` |
| `http://dbpedia.org/ontology/approach` | 21 | `Tim-Berners=Lee.parquet:fair trade framework assessing decentralised data solution`<br>`Tim-Berners=Lee.parquet:fair trade framework assessing decentralised data solution`<br>`tradessummary_supertrend14wx5_9_trail_SL_in_system_percentSL1_0.parquet:Strategy` |
| `http://dbpedia.org/ontology/educationPlace` | 21 | `92ad84d2-9fee-4827-bcf5-b7bfcdf216d8_tags.parquet:good\most quality\high education`<br>`7319d2e3-697a-4c26-a9cb-49266518134d_tags.parquet:wonderful experience\good education\go green`<br>`92ad84d2-9fee-4827-bcf5-b7bfcdf216d8_tags.parquet:good\most quality\high education` |
| `http://dbpedia.org/ontology/medicalDiagnosis` | 21 | `test_data_tissue.parquet:diagnosis`<br>`train_20.parquet:psychiatric_or_mental_illness`<br>`train_23.parquet:psychiatric_or_mental_illness` |
| `http://dbpedia.org/ontology/medication` | 21 | `top_differences.parquet:DRUG`<br>`Arizona_1.parquet:flu medicine`<br>`Colorado_1.parquet:flu medicine` |
| `http://dbpedia.org/ontology/mood` | 21 | `ClaDan.parquet:mood`<br>`ClaDan_1.parquet:mood`<br>`ClaAggDarInsFasNot.parquet:mood` |
| `http://dbpedia.org/ontology/numberOfParkingSpaces` | 21 | `out-0.05.parquet:RandomSubSpaces`<br>`out-0.05_6.parquet:RandomSubSpaces`<br>`out-0.05_7.parquet:RandomSubSpaces` |
| `http://dbpedia.org/ontology/part` | 21 | `copo_schema.parquet:dc.relation hasPart`<br>`barbershop_the_next_cut_2016.parquet:For the most part`<br>`adult_in_scanner_auditory_behavioral_all_runs_08_08_19.parquet:auditory_part_id` |
| `http://dbpedia.org/ontology/particularSign` | 21 | `how_going_4.parquet:Any particular reason for your mood? (optional)`<br>`how_going_4_1.parquet:Any particular reason for your mood? (optional)`<br>`ed839.parquet:They had been given a sign - a powerful, lucid sign - that u...` |
| `http://dbpedia.org/ontology/politicalPartyOfLeader` | 21 | `China%20Front_1.parquet:Commanders and leaders.third party`<br>`010_1.parquet:POLITICS OF GLOBALIZATION`<br>`010_3.parquet:POLITICS OF GLOBALIZATION` |
| `http://dbpedia.org/ontology/battery` | 20 | `comments_data.parquet:Battery MD`<br>`comments_data.parquet:Battery Python`<br>`l36%20(Kestrel%20Cruiser%20C%20-%20NORMAL%20AE).parquet:BATTERY SYSTEM DAMAGE` |
| `http://dbpedia.org/ontology/causedBy` | 20 | `X-Keys_SF-260D.parquet:Multiply By`<br>`X-Keys_TBM900_1.parquet:Multiply By`<br>`ETS_5.parquet:ReplacedBy` |
| `http://dbpedia.org/ontology/differentialDiagnosis` | 20 | `Martin_E=-Hellman.parquet:differential-linear cryptanalysis`<br>`diagnosis_endOfLife.parquet:diagnosisRef`<br>`data-energy_efficiency.parquet:fault_detection_and_diagnosis` |
| `http://dbpedia.org/ontology/eTeatrId` | 20 | `hou_q3.parquet:e_Hamag`<br>`Circuito%20Lixeira%20Inteligente%20NodeMCU.parquet:ID da p´ágina`<br>`Flowmap_Cities_one_to_one_1.parquet:e_pop` |
| `http://dbpedia.org/ontology/ko` | 20 | `AkumaKaze.parquet:ka`<br>`100034.parquet:ko`<br>`100034_2.parquet:ko` |
| `http://dbpedia.org/ontology/settingOfPlay` | 20 | `spider_man_homecoming.parquet:Spider-Man: Homecoming plays like a coming of age tale in an action-packed superhero setting`<br>`RNG_2.parquet:The teams exchanged short field goals in the third quarter, setting the stage for a dramatic ending.`<br>`down0595--awskms-key.template.parquet:Parameters.CreateSimpleRoles.Description,Set` |
| `http://dbpedia.org/ontology/trackNumber` | 20 | `2018-12-07_CH23_15uM.parquet:Track  (um)`<br>`2019-01-31_CH12_15uM.parquet:Track  (um)`<br>`Genomic-GC-Manifest-Workflow-Test-1.parquet:Tracking Number` |
| `http://dbpedia.org/ontology/veneratedIn` | 20 | `baselineSS.parquet:In Plum`<br>`scaling.parquet:In Plum`<br>`2phase_energy_pbr_all_vars_out.parquet:P_in` |
| `http://dbpedia.org/ontology/award` | 19 | `007_edited.parquet:Awards`<br>`CloseFrameworkAgreementUA.parquet:awardCriteria`<br>`CloseFrameworkAgreementUA.parquet:awardPeriod` |
| `http://dbpedia.org/ontology/buildingType` | 19 | `loading_profile_11.parquet:WebCore::StyleBuilder::applyProperty (period)`<br>`loading_profile_13.parquet:WebCore::StyleBuilder::applyProperty (period)`<br>`loading_profile_29.parquet:WebCore::StyleBuilder::applyProperty (period)` |
| `http://dbpedia.org/ontology/deFactoLanguage` | 19 | `MARTUTENE%20PM%209%20SOLAR_6_20190117120000.parquet:Intervalo de registro`<br>`MARTUTENE%20PM%20COCHE%20ELECTRICO_7_20190117120100.parquet:ID de tipo de dispositivo`<br>`MARTUTENE%20PM%20COCHE%20ELECTRICO_7_20190117120100.parquet:Intervalo de registro` |
| `http://dbpedia.org/ontology/film` | 19 | `dbo_gross-959-small-10var.parquet:Film`<br>`dbo_gross-959-small-10var_1.parquet:Film`<br>`dbo_gross-959-small-5var_1.parquet:Film` |
| `http://dbpedia.org/ontology/followingEvent` | 19 | `DUK.parquet:Following`<br>`test_53.parquet:user_following`<br>`mike_pence_users_2.parquet:Following` |
| `http://dbpedia.org/ontology/goldMedalist` | 19 | `24k-gold.parquet:Gold`<br>`monsters_1.parquet:gold`<br>`white-gold.parquet:Gold` |
| `http://dbpedia.org/ontology/heightAttack` | 19 | `gear.parquet:Attack speed`<br>`gear.parquet:Crush attack`<br>`gear.parquet:Magic attack` |
| `http://dbpedia.org/ontology/measurements` | 19 | `WorldBankIndicators_prod.parquet:measurementDenominator`<br>`WorldBankIndicators_prod.parquet:measurementDenominator`<br>`data_science-data.parquet:performance_measurement_and_verification` |
| `http://dbpedia.org/ontology/nonProfessionalCareer` | 19 | `tbl_courses.parquet:Professional Practice and Responsibility`<br>`0251507d-9160-4ecc-9954-08f463a49e0f_tags.parquet:high standard\high academic standard\friendly environment\financial aid\timely manner\pleasant experience`<br>`Final%20-%20Copy.parquet:Commercial and professional infrastructure` |
| `http://dbpedia.org/ontology/picturesCommonsCategory` | 19 | `bulkbundleproductupload.parquet:images`<br>`bulkbundleproductupload_2.parquet:images`<br>`product_list_3.parquet:images` |
| `http://dbpedia.org/ontology/place` | 19 | `a2befbe3-0afb-47a5-bfe5-fa2a2cd37a03_tags.parquet:great place`<br>`Complaint.parquet:reviewPlace`<br>`regression.parquet:place` |
| `http://dbpedia.org/ontology/populationDensity` | 19 | `Factors.parquet:Population density`<br>`Factors_1.parquet:Population density`<br>`StateData.parquet:Population Density` |
| `http://dbpedia.org/ontology/team` | 19 | `jdg_nrm.parquet:Team dynamics`<br>`jdg_nrm_1.parquet:Team dynamics`<br>`Sample%20Data_4.parquet:TeamId` |
| `http://dbpedia.org/ontology/tradeMark` | 19 | `cleanedEconFreedomData.parquet:Trade Freedom`<br>`cleanedEconFreedomData_2.parquet:Trade Freedom`<br>`econFreedomCorrMatrix_1.parquet:Trade Freedom` |
| `http://dbpedia.org/ontology/usOpenSingle` | 19 | `test_id_duplicates_bloomfield_input.parquet:open_to_spec_group`<br>`hours_and_location_intents_10.parquet:When do you open`<br>`hours_and_location_intents_8.parquet:When do you open` |
| `http://dbpedia.org/ontology/declination` | 18 | `2001_2.parquet:declination`<br>`2003.parquet:declination`<br>`2004.parquet:declination` |
| `http://dbpedia.org/ontology/fight` | 18 | `Arizona_1.parquet:fight the flu`<br>`Colorado_1.parquet:fight the flu`<br>`Connecticut_1.parquet:fight the flu` |
| `http://dbpedia.org/ontology/frozen` | 18 | `ice-cream-cake.parquet:Ice`<br>`ice-wreck.parquet:Ice`<br>`data-1420-1425.parquet:ice` |
| `http://dbpedia.org/ontology/house` | 18 | `nyc-listings.parquet:house_rules`<br>`nyc-listings_2.parquet:house_rules`<br>`nyc-listings_new.parquet:house_rules` |
| `http://dbpedia.org/ontology/internationalAffiliation` | 18 | `infocom83.parquet:International Data Communications`<br>`007.parquet:INTRODUCTION TO INTERNATIONAL RELATIONS`<br>`008.parquet:INTRODUCTION TO INTERNATIONAL RELATIONS` |
| `http://dbpedia.org/ontology/mainFamilyBranch` | 18 | `competitors_bsd_construct.parquet:branch-misses`<br>`competitors_bsd_count.parquet:branch-misses`<br>`competitors_impala_count.parquet:branch-misses` |
| `http://dbpedia.org/ontology/meetingBuilding` | 18 | `leeds-tagged-100-f.parquet:_meeting_`<br>`leeds-tagged-100-oq.parquet:_meeting_`<br>`reqview-tagged-100-of.parquet:_meeting_` |
| `http://dbpedia.org/ontology/nameInTraditionalChinese` | 18 | `Lesotho_426_married_example.parquet:se_log_r_traditional_no_use`<br>`Lesotho_426_married_example.parquet:se_traditional`<br>`007.parquet:NATIONALISTIC THOUGHT IN SANSKRIT LITERATURE` |
| `http://dbpedia.org/ontology/surfaceGravity` | 18 | `2012_raw.parquet:gravity`<br>`planets%2Bradii.parquet:gravity`<br>`planets%2Bradii_1.parquet:gravity` |
| `http://dbpedia.org/ontology/third` | 18 | `RNG_2.parquet:EditorsNote: rewords third and fourth grafs`<br>`summary_features_participants_classification_th10.parquet:std_thirdRule_y`<br>`summary_features_participants_classification_th20.parquet:std_thirdRule_y` |
| `http://dbpedia.org/ontology/throwingSide` | 18 | `DiverseReport.parquet:MISSING ELEMENTS IN RIGHT SIDE`<br>`Delaware_1.parquet:flu shot side effects`<br>`Kansas_1.parquet:flu shot side effects` |
| `http://dbpedia.org/ontology/visitorsTotal` | 18 | `nyc-listings_new.parquet:total_photos`<br>`ScheduleG.parquet:LoansPaidTotal`<br>`ScheduleG_3.parquet:LoansPaidTotal` |
| `http://dbpedia.org/ontology/weapon` | 18 | `unique_classeses.dbpedia.org_resource_.nt.gz.hdt.parquet://dbpedia.org/ontology/Weapon`<br>`unregistered_classeses.dbpedia.org_resource_.nt.gz.hdt.parquet://dbpedia.org/ontology/Weapon`<br>`l36%20(Kestrel%20Cruiser%20C%20-%20NORMAL%20AE).parquet:NEARBY SHIP WEAPONS SYSTEM DAMAGE` |
| `http://dbpedia.org/ontology/annualTemperature` | 17 | `discom_consumption.parquet:annual_growth_rate`<br>`Test_FillDiversionStationsFromHydroBase_ge20200720_out.parquet:EFFICIENCY ANNUAL (%)`<br>`Test_FillDiversionStationsFromHydroBase_le20180529_out.parquet:EFFICIENCY ANNUAL (%)` |
| `http://dbpedia.org/ontology/board` | 17 | `DPSGY.parquet:messageBoardId`<br>`DR0.DE_1.parquet:messageBoardId`<br>`DTGI.parquet:messageBoardId` |
| `http://dbpedia.org/ontology/certification` | 17 | `1519141640_allp_1.parquet:Certification`<br>`1519147593_allp_2.parquet:Certification`<br>`1519204864_allp_2.parquet:Certification` |
| `http://dbpedia.org/ontology/climbUpNumber` | 17 | `6_1527.parquet:Down/Up Ratio`<br>`Network_Test_Traffic.parquet:Down/Up Ratio`<br>`tf-idf.parquet:up` |
| `http://dbpedia.org/ontology/club` | 17 | `shuffled_tweets_1-140,751-800.parquet:t_club`<br>`shuffled_tweets_421-560.parquet:t_club`<br>`scaling.parquet:Devil's club` |
| `http://dbpedia.org/ontology/deathYear` | 17 | `GOT-character-deaths_10.parquet:Death Chapter`<br>`GOT-character-deaths_6.parquet:Death Chapter`<br>`GoT-character-deaths_11.parquet:Death Chapter` |
| `http://dbpedia.org/ontology/family` | 17 | `taxtable.parquet:family`<br>`taxtable_1.parquet:family`<br>`in_cell_line_breaks.parquet:Premium Family` |
| `http://dbpedia.org/ontology/finalFlight` | 17 | `shuffled_tweets_1-140,751-800.parquet:t_flight_delays`<br>`shuffled_tweets_421-560.parquet:t_flight_delays`<br>`shuffled_tweets_1-140,751-800.parquet:t_flight_delays` |
| `http://dbpedia.org/ontology/fullScore` | 17 | `artinSTEM2020.parquet:full`<br>`MW_15_HIDS_3.parquet:full_log`<br>`MW_17_HIDS_3.parquet:full_log` |
| `http://dbpedia.org/ontology/guest` | 17 | `nyc-listings.parquet:guests_included`<br>`nyc-listings_2.parquet:guests_included`<br>`nyc-listings_new.parquet:guests_included` |
| `http://dbpedia.org/ontology/highestAltitude` | 17 | `2019-10-15.parquet:High latitude (deg)`<br>`2019-10-18.parquet:High latitude (deg)`<br>`2019-11-05.parquet:High latitude (deg)` |
| `http://dbpedia.org/ontology/lastFlight` | 17 | `shuffled_tweets_1-140,751-800.parquet:t_flight_bookings`<br>`shuffled_tweets_421-560.parquet:t_flight_bookings`<br>`shuffled_tweets_1-140,751-800.parquet:t_flight_bookings` |
| `http://dbpedia.org/ontology/mergedWith` | 17 | `units_1.parquet:convertsWith`<br>`Chemical%20and%20Biomolecular%20Engineering%20(CBE)-courses.parquet:overlaps with`<br>`Political%20Science%20(POL%20SCI)-courses.parquet:overlaps with` |
| `http://dbpedia.org/ontology/tessitura` | 17 | `ClaDan.parquet:timbre`<br>`ClaDan_1.parquet:timbre`<br>`ClaAggDarInsFasNot.parquet:timbre` |
| `http://dbpedia.org/ontology/wordBefore` | 17 | `END_RESULT_DATAFRAME_9_pypownet_240.parquet:Flows before`<br>`END_RESULT_DATAFRAME_G2OP_CASE14_REALISTIC.parquet:Flows before`<br>`END_RESULT_DATAFRAME_for_test_with_line_9_cut_withoutthermallimits.parquet:Flows before` |
| `http://dbpedia.org/ontology/apparentMagnitude` | 16 | `songs_13.parquet:sentimentMagnitude`<br>`alignment.parquet:Magnitude`<br>`final%20data%20with%20predictions.parquet:magnitude` |
| `http://dbpedia.org/ontology/countryWithFirstSpaceflight` | 16 | `FOXNEWS.201707_2.parquet:approach. rich, thank you. lawmakers in california have voted to extend landmark climate change legislation, despite the u.s. withdrawal from an international global warming treaty.`<br>`Colorado_1.parquet:medicine for the flu`<br>`District%20of%20Columbia_1.parquet:medicine for the flu` |
| `http://dbpedia.org/ontology/ethnicity` | 16 | `20200614_05-03-57_IL1b_1.parquet:Patient race/ethnicity`<br>`20200614_05-12-07_CCL2.parquet:Patient race/ethnicity`<br>`DaveClientTest.parquet:Ethnicity` |
| `http://dbpedia.org/ontology/giniCoefficient` | 16 | `hate_crimes.parquet:gini_index`<br>`hate_crimes_1.parquet:gini_index`<br>`us_states_misc_stats.parquet:gini_coefficient` |
| `http://dbpedia.org/ontology/giniCoefficientRanking` | 16 | `Bosnia%20and%20Herzegovina.parquet:Gini index (World Bank estimate)`<br>`mexico.parquet:Gini index (World Bank estimate)`<br>`montenegro.parquet:Gini index (World Bank estimate)` |
| `http://dbpedia.org/ontology/hasVariant` | 16 | `humaneval_master.parquet:HasTest`<br>`copo_schema.parquet:dc.relation hasVersion`<br>`Simple.parquet:has_variants` |
| `http://dbpedia.org/ontology/instrument` | 16 | `competitors_425.parquet:TexasInstrumentsIncorporated`<br>`competitors_463.parquet:TexasInstrumentsIncorporated`<br>`competitors_471.parquet:TexasInstrumentsIncorporated` |
| `http://dbpedia.org/ontology/island` | 16 | `U.S.%20Census%20Data_1.parquet:Rhode Island`<br>`U.S.%20Census%20Data_14.parquet:Rhode Island`<br>`U.S.%20Census%20Data_31.parquet:Rhode Island` |
| `http://dbpedia.org/ontology/landingSite` | 16 | `data_science-data.parquet:site`<br>`data_science-data_1.parquet:site`<br>`1a93f47c44a4e68037e88f645b09ba99201d81f7a263c27b8e7dcfac3d8bbf4b_5.parquet:Site UPRN` |
| `http://dbpedia.org/ontology/lastSeason` | 16 | `tf-idf.parquet:last`<br>`Test_CalculateTimeSeriesStatistic_LastNonmissing_AnalysisPeriod_AnalysisWindow_out.parquet:LastNonmissing`<br>`Test_CalculateTimeSeriesStatistic_LastNonmissing_AnalysisPeriod_AnalysisWindow_out_1.parquet:LastNonmissing` |
| `http://dbpedia.org/ontology/latestElection` | 16 | `cdph.ca.gov-crime-1.parquet:metatab-latest`<br>`cdph.ca.gov-alcohol_licenses-1.parquet:metatab-latest`<br>`cdph.ca.gov-crime-1.parquet:metatab-latest` |
| `http://dbpedia.org/ontology/mother` | 16 | `education.student_13.parquet:mother_tongue/id`<br>`education.student_21.parquet:mother_tongue/id`<br>`education.student_5.parquet:mother_tongue/id` |
| `http://dbpedia.org/ontology/orbitalEccentricity` | 16 | `Solar%20System.parquet:Orbital Eccentricity`<br>`Solar_System.parquet:Orbital Eccentricity`<br>`solar.parquet:Orbital Eccentricity` |
| `http://dbpedia.org/ontology/orderDate` | 16 | `account_accountingBatch_10.parquet:interbankPaymentOrderTypeSelect`<br>`account_accountingBatch_11.parquet:interbankPaymentOrderTypeSelect`<br>`account_accountingBatch_12.parquet:interbankPaymentOrderTypeSelect` |
| `http://dbpedia.org/ontology/plant` | 16 | `2015-03-31%2006_35_01.parquet:maize_fertilizer_supplier`<br>`2015-03-31%2006_35_01.parquet:maize_seed_supplier`<br>`2015-04-02%2003_27_21.parquet:maize_fertilizer_supplier` |
| `http://dbpedia.org/ontology/similar` | 16 | `test_queries_largeDataset_0.02_Ayat_manualEval.parquet:isSimilar`<br>`tf-idf.parquet:different`<br>`individual_sheets_FW.parquet:group_sheet_similar` |
| `http://dbpedia.org/ontology/structuralSystem` | 16 | `830737f5-ced0-4b7b-9ee5-c2522ab8253a_tags.parquet:unique system\equal`<br>`bloomberg_1year_4.parquet:Design Underground System`<br>`bloomberg_2year_4.parquet:Design Underground System` |
| `http://dbpedia.org/ontology/unNumber` | 16 | `extraction.parquet:[[Van der Waerden number]]`<br>`Mage_Eav_354.parquet:Inténto de añadir un objeto inválido`<br>`Mage_Eav_376.parquet:Inténto de añadir un objeto inválido` |
| `http://dbpedia.org/ontology/votesFor` | 16 | `export_datacosupplychaindataset.parquet:days for shipment (scheduled)`<br>`NSW_LGA_NEXIS_201212_15.parquet:NEED FOR ASSISTANCE %`<br>`NSW_LGA_NEXIS_201212_20.parquet:NEED FOR ASSISTANCE %` |
| `http://dbpedia.org/ontology/addressInRoad` | 15 | `source_17.parquet:motionState_vehicleInFrontId`<br>`source_18.parquet:motionState_vehicleInFrontId`<br>`source_20.parquet:motionState_vehicleInFrontId` |
| `http://dbpedia.org/ontology/brand` | 15 | `example_3.parquet:brand_selector`<br>`null_examples.parquet:int_card_brand`<br>`example_2_1.parquet:brand_selector` |
| `http://dbpedia.org/ontology/championInSingleFemale` | 15 | `PoblacionMundial.parquet:PopFemale`<br>`train_20.parquet:female`<br>`train_23.parquet:female` |
| `http://dbpedia.org/ontology/channel` | 15 | `hay_capture-01.kismet.parquet:Channel`<br>`hay_capture-02.kismet.parquet:Channel`<br>`hay_capture-07.kismet.parquet:Channel` |
| `http://dbpedia.org/ontology/construction` | 15 | `emp_industries_p.parquet:construction`<br>`tv003_umur.parquet:annee_construction`<br>`Comparison_of_programming_languages_(object-oriented_programming)-0_1.parquet:construction` |
| `http://dbpedia.org/ontology/effectiveRadiatedPower` | 15 | `86df1213-44fa-46a7-84db-cb7d164a4006_tags.parquet:effective`<br>`isca2021-pcinfo.parquet:topic: Power, energy, and thermal management`<br>`people_124.parquet:effective_tax` |
| `http://dbpedia.org/ontology/livingPlace` | 15 | `top_2010s.parquet:live`<br>`Q27d_1.parquet:Living Situation - Youth`<br>`q15_1.parquet:Living Situation` |
| `http://dbpedia.org/ontology/nationalRanking` | 15 | `12-5-1.parquet:National metadata`<br>`16-2-1.parquet:National metadata`<br>`3-2-2.parquet:National metadata` |
| `http://dbpedia.org/ontology/neighbourhood` | 15 | `nyc-listings.parquet:neighborhood_overview`<br>`nyc-listings_2.parquet:neighborhood_overview`<br>`nyc-listings_new.parquet:neighborhood_overview` |
| `http://dbpedia.org/ontology/otherFunction` | 15 | `certified-connected-thermostats-2019-04-04.parquet:Communication Method Other`<br>`github_178.parquet:functionParameters`<br>`spacy_noun_phrases.parquet:the functionality` |
| `http://dbpedia.org/ontology/partitionCoefficient` | 15 | `picking_predictors.parquet:coefficients`<br>`coeff_DecisionTreeReg.parquet:Coefficients`<br>`coeff_ranf.parquet:Coefficients` |
| `http://dbpedia.org/ontology/politicalFunction` | 15 | `Final%20-%20Copy.parquet:Cultural and social norms`<br>`financing_rates_1.parquet:social_indicator`<br>`financing_rates_2.parquet:social_indicator` |
| `http://dbpedia.org/ontology/populationTotalReference` | 15 | `summary_stats.parquet:totalDebtToCapitalization`<br>`blue1_expression.parquet:Total RPKM to gene models`<br>`blue2_expression.parquet:Total RPKM to gene models` |
| `http://dbpedia.org/ontology/relatedPlaces` | 15 | `16-to-19_2021-to-2022_published-12-09-2021_casterbridge-college.parquet:Places`<br>`16-to-19_2021-to-2022_published-12-09-2021_casterbridge-college_11.parquet:Places`<br>`16-to-19_2021-to-2022_published-12-09-2021_casterbridge-college_12.parquet:Places` |
| `http://dbpedia.org/ontology/setDesigner` | 15 | `main_cross_question_validations.parquet:conditional_set`<br>`main_cross_question_validations.parquet:set`<br>`input_template_1.parquet:AVAILABILITY_SET` |
| `http://dbpedia.org/ontology/sharingOutPopulationYear` | 15 | `BMO_2.parquet:Reuters reported last week that Swiss bank Credit Suisse is exploring options for its asset management arm.`<br>`Company_info.parquet:sharesPercentSharesOut`<br>`Final%20-%20Copy.parquet:Total early-stage Entrepreneurial Activity for Female Working Age Population` |
| `http://dbpedia.org/ontology/shoot` | 15 | `weapon_data.parquet:spread/shot`<br>`weapon_data_1.parquet:spread/shot`<br>`leeds-tagged-100-f.parquet:_shot_` |
| `http://dbpedia.org/ontology/usingCountry` | 15 | `nn_result.parquet:scaled_input_native-country`<br>`leeds-tagged-100-f.parquet:_using_`<br>`leeds-tagged-100-oq.parquet:_using_` |
| `http://dbpedia.org/ontology/activeCases` | 14 | `6_1527.parquet:Active Mean`<br>`6_1527.parquet:Active Std`<br>`Network_Test_Traffic.parquet:Active Mean` |
| `http://dbpedia.org/ontology/areaOfCatchment` | 14 | `Blood_Law_1.parquet:Area of Effect`<br>`london-borough-profile_13.parquet:Inland_Area_(Hectares)`<br>`london-borough-profiles_12.parquet:Inland_Area_(Hectares)` |
| `http://dbpedia.org/ontology/availableSmartCard` | 14 | `data_science-data.parquet:smart_meters`<br>`data_science-data_1.parquet:smart_meters`<br>`ceedb667-5d12-4672-8570-6822b4bd647a_tags.parquet:online learning\great\easy\available professor\few hiccup\available student` |
| `http://dbpedia.org/ontology/callSign` | 14 | `AoU_DRCB_GEN_2020-07-11-00-00-00.parquet:drc_call_rate`<br>`Xproject_Final%20Parameters%20Pivoted.parquet:AUCall`<br>`template_57.parquet:call_screen` |
| `http://dbpedia.org/ontology/chEBI` | 14 | `HyPiX_ClappHornberger_Constrained_%20HyDroParam.parquet:Ψch`<br>`HyPiX_ClappHornberger_Constrained_%20HyDroParam.parquet:λch`<br>`stdy_kinase_xtal.200321.conf.SK_nn_kinfo_classify_1.parquet:dist_CH` |
| `http://dbpedia.org/ontology/codeStockExchange` | 14 | `import-20180616081850-1_18_Laptop12345.parquet:use_config_notify_stock_qty`<br>`magento-1-import.parquet:use_config_notify_stock_qty`<br>`products_multiple_stores_86.parquet:use_config_notify_stock_qty` |
| `http://dbpedia.org/ontology/complications` | 14 | `Arizona_1.parquet:flu complications`<br>`Delaware_1.parquet:flu complications`<br>`District%20of%20Columbia_1.parquet:flu complications` |
| `http://dbpedia.org/ontology/connectsReferencedTo` | 14 | `copo_schema.parquet:dc.relation conformsTo`<br>`overview_errors.parquet:Bases corresponding to M operator in CIGAR extend beyond reference`<br>`data-1016-1019.parquet:TTS (text to speech) for node.js. send text from node.js to your speakers.` |
| `http://dbpedia.org/ontology/demographicsAsOf` | 14 | `John_W=-Backus_1.parquet:The History of Fortran I, II, and III.`<br>`dsn2007.parquet:Science and Engineering: A Collusion of Cultures.`<br>`BK.parquet:Wee Khoon Chong, senior Asia Pacific market strategist at BNY Mellon Markets, told Reuters that narrowing spreads between Chinese and U.S. bonds and a flat Chinese yield curve weighed on demand. The relatively rapid rollout of vaccines in the United States has also dimmed the appeal of Chinese bonds relative to U.S. assets as expectations rise for a strong U.S. recovery.` |
| `http://dbpedia.org/ontology/drama` | 14 | `genre-tag_vectors.parquet:Drama`<br>`007.parquet:SANSKRIT DRAMA`<br>`008.parquet:SANSKRIT DRAMA` |
| `http://dbpedia.org/ontology/firstPlace` | 14 | `en_GB_44.parquet:Europe First Priority`<br>`en_US_8317.parquet:Europe First Priority`<br>`en_US_8328.parquet:Europe First Priority` |
| `http://dbpedia.org/ontology/freeFlightTime` | 14 | `weapon_data.parquet:flight time`<br>`coupang_1year_4.parquet:Employee Free Time`<br>`coupang_6months_4.parquet:Employee Free Time` |
| `http://dbpedia.org/ontology/kindOfRock` | 14 | `jdg_nrm.parquet:Humility / arrogance of founders`<br>`jdg_nrm_1.parquet:Humility / arrogance of founders`<br>`permit-2011-1894_formatted.parquet:Rock` |
| `http://dbpedia.org/ontology/legalArticle` | 14 | `category_paper_mappings_20180614_1.parquet:Article`<br>`article_40.parquet:Articles`<br>`data_395.parquet:news_article` |
| `http://dbpedia.org/ontology/meshNumber` | 14 | `expected_trials_data_1.parquet:condition_mesh`<br>`export2021.03.29-05.53.35.parquet:Mesh_Terms`<br>`Image_22.parquet:Mean_MacroCells_AreaShape_EulerNumber` |
| `http://dbpedia.org/ontology/orientation` | 14 | `data_science-data.parquet:building_orientation`<br>`data_science-data_1.parquet:building_orientation`<br>`test2_2021_Jan_27_1148.parquet:orientation` |
| `http://dbpedia.org/ontology/patent` | 14 | `Bosnia%20and%20Herzegovina.parquet:Patent applications, residents`<br>`andorra.parquet:Patent applications, residents`<br>`korea%20dem.%20people%20rep..parquet:Patent applications, residents` |
| `http://dbpedia.org/ontology/peopleName` | 14 | `tf-idf.parquet:people`<br>`com.sun.star.comp.forms.OGridControlModel_10.parquet:getByName()`<br>`com.sun.star.comp.configuration.OInnerGroupUpdateAccess_2.parquet:getByName()` |
| `http://dbpedia.org/ontology/politicalPartyInLegislature` | 14 | `007.parquet:THEMES IN COMPARATIVE POLITICAL THEORY`<br>`008.parquet:THEMES IN COMPARATIVE POLITICAL THEORY`<br>`029.parquet:POLITICAL THOUGHT IN SANSKRIT` |
| `http://dbpedia.org/ontology/procedure` | 14 | `QA-Grid%20view-2.parquet:Procedure`<br>`QA_UTF8.parquet:Procedure`<br>`conditions.parquet:procedure` |
| `http://dbpedia.org/ontology/splitFromParty` | 14 | `imp_909_predict_0203-1.py.parquet:split`<br>`imp_910_predict_0204-1.py.parquet:split`<br>`imp_919_predict_0215-3.py.parquet:split` |
| `http://dbpedia.org/ontology/timeZone` | 14 | `data_science-data.parquet:zone`<br>`data_science-data_1.parquet:zone`<br>`root-data-global.parquet:ApicalZone` |
| `http://dbpedia.org/ontology/winterTemperature` | 14 | `data_science-data.parquet:outdoor_air_temperature`<br>`data_science-data_1.parquet:outdoor_air_temperature`<br>`Baltimore7DayForecast_1.parquet:weather` |
| `http://dbpedia.org/ontology/worstDefeat` | 14 | `amazon-books-cv-4-bigrams-normalized.parquet:the worst`<br>`amazon-books-cv-4-bigrams-normalized_1.parquet:the worst`<br>`amazon-books-cv-4-bigrams.parquet:the worst` |
| `http://dbpedia.org/ontology/alpsSubgroup` | 13 | `hipc_ctf_23591775_2.parquet:subgroup`<br>`hipc_gene_21357945_5.parquet:subgroup`<br>`hipc_gene_21357945_5_1.parquet:subgroup` |
| `http://dbpedia.org/ontology/bigPoolRecord` | 13 | `tf-idf.parquet:pool`<br>`big-sky-og.parquet:Big`<br>`big-sky-og_1.parquet:Big` |
| `http://dbpedia.org/ontology/bird` | 13 | `blue-heron.parquet:Heron`<br>`permit-2011-3695_formatted.parquet:Eagle`<br>`Delaware_1.parquet:bird flu` |
| `http://dbpedia.org/ontology/cargoFuel` | 13 | `l36%20(Kestrel%20Cruiser%20C%20-%20NORMAL%20AE).parquet:NEARBY SHIP FUEL`<br>`ship_data_7.parquet:cargo`<br>`l28%20(Zoltan%20Cruiser%20B%20-%20NORMAL%20AE).parquet:NEARBY SHIP FUEL` |
| `http://dbpedia.org/ontology/complexity` | 13 | `BiPlot.parquet:Richness`<br>`OmgFunctionsTechnical_1.parquet:Effort complexity`<br>`OmgFunctionsTechnical_2.parquet:Effort complexity` |
| `http://dbpedia.org/ontology/dateCompleted` | 13 | `%E3%82%BF%E3%82%B9%E3%82%AFlist_1.parquet:diff_days_to_task_completed`<br>`%E3%82%BF%E3%82%B9%E3%82%AFlist_2.parquet:diff_days_to_task_completed`<br>`conquest_warrior_transformation_3.parquet:completed_episode_id` |
| `http://dbpedia.org/ontology/digitalChannel` | 13 | `Comparison_of_radio_systems_0.parquet:Digital subchannels`<br>`Digital_Journal.parquet:digital`<br>`Robert_E=-Kahn_1.parquet:block digital entity standard perspective` |
| `http://dbpedia.org/ontology/eMedicineSubject` | 13 | `dtb_mail_template_12.parquet:mail_subject`<br>`dtb_mail_template_16.parquet:mail_subject`<br>`dtb_mail_template_24.parquet:mail_subject` |
| `http://dbpedia.org/ontology/engine` | 13 | `cars%20-%20Copy.parquet:engineCap`<br>`turbo-mind-warp.parquet:Turbo`<br>`turbo-mind-warp_1.parquet:Turbo` |
| `http://dbpedia.org/ontology/ethnicGroupsInYear` | 13 | `25915251_results.parquet:Edit Groups (EditId, Matched Groups)`<br>`44880776_results.parquet:Edit Groups (EditId, Matched Groups)`<br>`9554482_results.parquet:Edit Groups (EditId, Matched Groups)` |
| `http://dbpedia.org/ontology/gun` | 13 | `state_data.parquet:ANTI-GUN`<br>`state_data.parquet:PRO-GUN`<br>`state_data.parquet:federal_ANTI-GUN` |
| `http://dbpedia.org/ontology/hand` | 13 | `Lokad_Item.parquet:StockOnHand`<br>`Lokad_Items.parquet:StockOnHand`<br>`MRPTests_4.parquet:QtyOnHand` |
| `http://dbpedia.org/ontology/legalForm` | 13 | `EAP700M.parquet:Legal`<br>`3.data%20records_2.parquet:Home - Find a Lawyer - Law Bulletin Boards - Legal Forms - Search CAN I USE BANKRUPTCY TO PROTECT MY ASSETS? Free legal information for asset protection law @ FreeAdvice.com. ...`<br>`impact_level_description.parquet:legal_and_compliance` |
| `http://dbpedia.org/ontology/licenceLetter` | 13 | `shuffled04%20A%20Young(Remix)%20Lyrics.parquet:Rhyme Scheme Letter`<br>`Data_Tidy.parquet:sup_letter`<br>`4%20A%20Young(Remix)%20Lyrics.parquet:Rhyme Scheme Letter` |
| `http://dbpedia.org/ontology/littlePoolRecord` | 13 | `track-artist-uri-00-cleaned_1.parquet:The Little Boy Blues`<br>`2009_volkswagen_eos.parquet:Great Little car`<br>`track-artist-uri-00-cleaned_1.parquet:The Little Boy Blues` |
| `http://dbpedia.org/ontology/playingTime` | 13 | `Episode-60-.parquet:Play enough Basketball and you too`<br>`Episode-60-In-Order-to-Win.parquet:Play enough Basketball and you too`<br>`aggregated_master.parquet:time_played` |
| `http://dbpedia.org/ontology/populationPctChildren` | 13 | `joint_tour_composition_5.parquet:adults`<br>`Delaware_1.parquet:flu in children`<br>`District%20of%20Columbia_1.parquet:flu in children` |
| `http://dbpedia.org/ontology/river` | 13 | `2012_raw.parquet:Rivers`<br>`housing_data.parquet:Bounds Charles River?`<br>`valley-girl.parquet:Valley` |
| `http://dbpedia.org/ontology/speaker` | 13 | `62_51.parquet:speaker`<br>`62_207.parquet:speaker`<br>`gordon.parquet:speaker` |
| `http://dbpedia.org/ontology/specialist` | 13 | `lift_values_query_6.parquet:expert`<br>`lift_values_query_8.parquet:expert`<br>`lift_values_query_1.parquet:expert` |
| `http://dbpedia.org/ontology/surfaceFormOccurrenceOffset` | 13 | `1601391354_Linux_runs.parquet:process_parallel__pipeline_numeric__impute__gaps`<br>`1601391354_Linux_runs.parquet:process_parallel__pipeline_numeric__transform_normal`<br>`1601391356_Linux_runs.parquet:process_parallel__pipeline_numeric__impute__gaps` |
| `http://dbpedia.org/ontology/ward` | 13 | `Automotive%20Noise%20Disturbance.parquet:ward`<br>`Big%20Buildings%20Online%20Request.parquet:ward`<br>`Illegal%20Posting%20of%20Signs.parquet:ward` |
| `http://dbpedia.org/ontology/wholeArea` | 13 | `da2e1d80-8be4-493b-a4fc-bc30936f23e1_tags.parquet:whole semester\online`<br>`Batch_AC.parquet:Cottonseed_Whole`<br>`Batch_AC3.parquet:Cottonseed_Whole` |
| `http://dbpedia.org/ontology/airDate` | 12 | `data_science-data.parquet:air-to-air`<br>`data_science-data_1.parquet:air-to-air`<br>`nitrogendioxide.parquet:Air` |
| `http://dbpedia.org/ontology/assetUnderManagement` | 12 | `summary_stats.parquet:Asset Growth`<br>`summary_stats.parquet:assetTurnover`<br>`summary_stats.parquet:fixedAssetTurnover` |
| `http://dbpedia.org/ontology/background` | 12 | `reaTestData_1.parquet:backgroundSaltConc`<br>`questions_expert.parquet:background`<br>`test_data_age.parquet:background` |
| `http://dbpedia.org/ontology/brainInfoNumber` | 12 | `meta_climate_test.parquet:no_temporal_info`<br>`BugCrowd-export.parquet:extra_info`<br>`down0299--resource-template.json.parquet:Parameters.AdditionalReportInfoWriteCapacityUnits.Description` |
| `http://dbpedia.org/ontology/demographics` | 12 | `dictionary_1_manual.parquet:demographic`<br>`longitudinal-data-import_1.parquet:demographics_complete`<br>`longitudinal-data-import_2.parquet:demographics_complete` |
| `http://dbpedia.org/ontology/elementAbove` | 12 | `1997_keywords.parquet:earth element`<br>`Extended_Data_Table_5.parquet:Celltypes_above_threshold`<br>`base.element_1.parquet:element_float` |
| `http://dbpedia.org/ontology/europeanParliamentGroup` | 12 | `Human%20phenomenological%20groups.parquet:Mervyn_group`<br>`covid19_data_description_dashboard-wave-1.parquet:group_english`<br>`group_info_april.parquet:group_english` |
| `http://dbpedia.org/ontology/formula` | 12 | `Acceleration.parquet:Identifier / Formula`<br>`Astronomical_spectroscopy.parquet:FormulaConceptDB`<br>`Angular_velocity.parquet:FormulaConceptDB` |
| `http://dbpedia.org/ontology/geneReviewsId` | 12 | `lac_201027.parquet:Reaction_gene_ (ID_ and/or)`<br>`lac_201027_1.parquet:Reaction_gene_ (ID_ and/or)`<br>`pathway_table_up_LUAD_B_lineage_1.parquet:geneID` |
| `http://dbpedia.org/ontology/highestPoint` | 12 | `survey-data.parquet:evaluation_usefulness-critical-service-highest-effort`<br>`allConfigurations.parquet:thresholdPointInPoly`<br>`variable_ranges_15.parquet:OUTLIER HIGH` |
| `http://dbpedia.org/ontology/layingDown` | 12 | `Alerts-1.parquet:alerts_down`<br>`financing_rates_1.parquet:down_payment`<br>`financing_rates_2.parquet:down_payment` |
| `http://dbpedia.org/ontology/linguisticsTradition` | 12 | `007.parquet:PHILOSOPHY, RELIGION AND CULTURE IN SANSKRIT TRADITION`<br>`008.parquet:SANSKRIT LITERATURE`<br>`029.parquet:PHILOSOPHY, RELIGION AND CULTURE IN SANSKRIT TRADITION` |
| `http://dbpedia.org/ontology/lowerEarthOrbitPayload` | 12 | `solarSystemData_1.parquet:EARTH`<br>`SolarSystemData_1.parquet:EARTH`<br>`SolarSystemData_3.parquet:EARTH` |
| `http://dbpedia.org/ontology/lowestAltitude` | 12 | `2019-05-30.parquet:Low latitude (deg)`<br>`2019-08-24.parquet:Low latitude (deg)`<br>`2020-01-06.parquet:Low latitude (deg)` |
| `http://dbpedia.org/ontology/lunarRover` | 12 | `2012_raw.parquet:MoonDragon`<br>`solarSystemData_1.parquet:MOON`<br>`martian-mean-green.parquet:Martian` |
| `http://dbpedia.org/ontology/names` | 12 | `COMPAS_BNN_RATE.parquet:var_names`<br>`ARImportProfileErrors.parquet:CC Names - Report`<br>`ARImportProfileInvalidFilename.parquet:CC Names - Report` |
| `http://dbpedia.org/ontology/northWestPlace` | 12 | `U.S.%20Census%20Data_1.parquet:North Dakota`<br>`U.S.%20Census%20Data_14.parquet:North Dakota`<br>`U.S.%20Census%20Data_31.parquet:North Dakota` |
| `http://dbpedia.org/ontology/numberOfAcademicStaff` | 12 | `3dc146f6-9101-4fe6-9656-54e9ab26da71_tags.parquet:take online course\great\deliver high quality\supportive understanding\very appreciative\academic understanding`<br>`cc1981ab-b8f3-46a7-a905-8b5aa29deabf_tags.parquet:most caring environment\efficient\great collaboration\academic center\accessible\academic journey`<br>`nctq_2019.parquet:Measures of Professional Practice (Teacher and Principal Evaluation Policy)` |
| `http://dbpedia.org/ontology/numberOfLines` | 12 | `stats_24.parquet:no._of_words`<br>`stats_27.parquet:no._of_words`<br>`stats_31.parquet:no._of_words` |
| `http://dbpedia.org/ontology/numberOfPropertiesUsed` | 12 | `citeSents_14.parquet:A similar list of markers have been shown to be excellent discriminating features between original and translated texts (from several European languages`<br>`brownswitcherinos.parquet:Directories contain elements that describe objects in the wo...`<br>`Comparison_of_documentation_generators-2_1.parquet:parameter types extracted` |
| `http://dbpedia.org/ontology/oldName` | 12 | `Num_anal_abstracts.parquet:old_file`<br>`regression.parquet:aerialway_old`<br>`regression.parquet:aeroway_old` |
| `http://dbpedia.org/ontology/penaltiesTeamA` | 12 | `archmealfiller.parquet:A computer needs a manager to administer its operations, jus...`<br>`drewden123.parquet:A computer needs a manager to administer its operations, jus...`<br>`specialist.parquet:A computer needs a manager to administer its operations, jus...` |
| `http://dbpedia.org/ontology/percentageOfAreaWater` | 12 | `ICPIM_FC_data_areainfo.parquet:SnowPercentOfPrec`<br>`baseline_glszm_1.parquet:SmallAreaHighGrayLevelEmphasis`<br>`xs_sample_bridge_blockage_1.parquet:blockage_proportion` |
| `http://dbpedia.org/ontology/primaryFuelType` | 12 | `top_differences.parquet:drive_primary`<br>`distributions_1.parquet:Analysis_type_secondary`<br>`distributions_2.parquet:Analysis_type_secondary` |
| `http://dbpedia.org/ontology/road` | 12 | `20170124_articles.parquet:Ring Road flyover ‘sags’`<br>`08_V1_merge.parquet:lane`<br>`cars%20-%20Copy.parquet:roadTax` |
| `http://dbpedia.org/ontology/sexualOrientation` | 12 | `train_20.parquet:other_sexual_orientation`<br>`train_20.parquet:sexual_explicit`<br>`train_23.parquet:other_sexual_orientation` |
| `http://dbpedia.org/ontology/shipLaunch` | 12 | `l36%20(Kestrel%20Cruiser%20C%20-%20NORMAL%20AE).parquet:NEARBY SHIP MISSILES`<br>`l28%20(Zoltan%20Cruiser%20B%20-%20NORMAL%20AE).parquet:NEARBY SHIP MISSILES`<br>`28.09.15(RockCruiserC-HARD-AE).parquet:NEARBY SHIP MISSILES` |
| `http://dbpedia.org/ontology/silverMedalist` | 12 | `silver-kush.parquet:Silver`<br>`silver-train.parquet:Silver`<br>`silver-surfer.parquet:Silver` |
| `http://dbpedia.org/ontology/statistic` | 12 | `disparity_results_mgbm_xnn_hmda_simu.parquet:T-Statistic`<br>`glm_summary.parquet:statistic`<br>`light-monod-params-direct.parquet:statistic` |
| `http://dbpedia.org/ontology/unitedStatesNationalBridgeId` | 12 | `competitors_225.parquet:PeoplesUnitedFinancialInc.`<br>`019_11.parquet:UNITED NATIONS AND GLOBAL CONFLICTS`<br>`019_3.parquet:UNITED NATIONS AND GLOBAL CONFLICTS` |
| `http://dbpedia.org/ontology/winsAtJapan` | 12 | `Dhairya_XBOX_GameSales_processed.parquet:Japan`<br>`Dhairya_XBOX_GameSales_processed_1.parquet:Japan`<br>`Dhairya_XBOX_GameSales_processed_2.parquet:Japan` |
| `http://dbpedia.org/ontology/clade` | 11 | `CAP206_GAG_1_all_hap.fasta_classification.parquet:clade_support`<br>`CAP206_GAG_1_all_hap.fasta_classification.parquet:clade_tree_cofidence`<br>`CAP217_ENV_3_all_hap.fasta_classification_1.parquet:clade_support` |
| `http://dbpedia.org/ontology/connotation` | 11 | `reddit_comments_race_black_biased_valid_reduced.parquet:bias_phrase`<br>`reddit_comments_religion1_jews_biased_valid_reduced_1.parquet:bias_phrase`<br>`reddit_comments_orientation_lgbtq_biased_valid_reduced_1.parquet:bias_phrase` |
| `http://dbpedia.org/ontology/contractAward` | 11 | `Agreement_10.parquet:contracts`<br>`Agreement_13.parquet:contracts`<br>`Agreement_1.parquet:contracts` |
| `http://dbpedia.org/ontology/currentRank` | 11 | `MCU_PCB_V2.parquet:MAXIMUM_DC_CURRENT`<br>`MCU_PCB_V2.parquet:RATED-CURRENT`<br>`MCU_PCB_V2.parquet:RATED_CURRENT` |
| `http://dbpedia.org/ontology/distanceToNearestCity` | 11 | `housing_11.parquet:distance_to_employment_center`<br>`housing_12.parquet:distance_to_employment_center`<br>`housing_data.parquet:Distance to employment centers` |
| `http://dbpedia.org/ontology/dryCargo` | 11 | `SCBI_all_traits_table_indvidual_level.parquet:PLA_dry_percent`<br>`EnglishWS_Byers_data.parquet:wet`<br>`wet-dream.parquet:Wet` |
| `http://dbpedia.org/ontology/episodeNumber` | 11 | `allDCUniverseEpisodesSorted.parquet:Episode Number`<br>`sample_tv_show_1.parquet:Episode Number`<br>`sample_tv_show.parquet:Episode Number` |
| `http://dbpedia.org/ontology/faaLocationIdentifier` | 11 | `test_manage_script_1.parquet:Dublin Core:Identifier`<br>`test_manage_script_10.parquet:Dublin Core:Identifier`<br>`test_manage_script_11.parquet:Dublin Core:Identifier` |
| `http://dbpedia.org/ontology/finalLostDouble` | 11 | `ref_test_IncludeHeader_csv.parquet:TestDouble`<br>`ref_test_IncludeHeader_csv_1.parquet:TestDouble`<br>`ref_test_IncludeHeader_delimited.parquet:TestDouble` |
| `http://dbpedia.org/ontology/flagCaption` | 11 | `101-120_2.parquet:phot_variable_flag`<br>`21-40.parquet:phot_variable_flag`<br>`61-80_1.parquet:phot_variable_flag` |
| `http://dbpedia.org/ontology/goldenCalfAward` | 11 | `break-times.orig.parquet:Golden`<br>`test-out.parquet:golden intent`<br>`test-out_1.parquet:golden intent` |
| `http://dbpedia.org/ontology/grossDomesticProductPerPeople` | 11 | `house_prices.parquet:non-retail business acres`<br>`%20central%20african%20republic.parquet:Listed domestic companies, total`<br>`cayman%20iceland.parquet:Listed domestic companies, total` |
| `http://dbpedia.org/ontology/hasSurfaceForm` | 11 | `job_62.parquet:A new contact form has been sent for the following offer:`<br>`job_64.parquet:A new contact form has been sent for the following offer:`<br>`job_65.parquet:A new contact form has been sent for the following offer:` |
| `http://dbpedia.org/ontology/historicalRegion` | 11 | `regression.parquet:historic`<br>`007.parquet:HISTORY AND TOURISM`<br>`008.parquet:HISTORY AND TOURISM` |
| `http://dbpedia.org/ontology/kindOfCriminal` | 11 | `build-in-fs.parquet:kind`<br>`build-in-fs_1.parquet:kind`<br>`blukki.parquet:Stupid human propaganda! The very concept of a superior alie...` |
| `http://dbpedia.org/ontology/movie` | 11 | `movies_50.parquet:Movie`<br>`53822652_0_5767892317858575530_5.parquet:movie`<br>`53822652_0_5767892317858575530_6.parquet:movie` |
| `http://dbpedia.org/ontology/numberOfPlatformLevels` | 11 | `s3r1a-Weights.parquet:Does the business model has high level of operating leverage & scalability potential ?`<br>`OAH%20Program%20Observation%20Form%20for%20TPP%20Grantees.parquet:Level of enthusiasm`<br>`WM_forwidedata.parquet:SearchesThreetoEight_Efficiency_InRespectiveTrial_AcrossBothPlatforms` |
| `http://dbpedia.org/ontology/operator` | 11 | `stepcriteria.parquet:Operator__c`<br>`test_cross_question_validations_1.parquet:operator`<br>`633_7.parquet:Operator` |
| `http://dbpedia.org/ontology/parentMountainPeak` | 11 | `e15434-fd-photom.parquet:peak`<br>`e29670-fd-photom_1.parquet:peak`<br>`e30080-fd-photom.parquet:peak` |
| `http://dbpedia.org/ontology/releaseDate` | 11 | `top_pad_19.parquet:Release`<br>`top_pad_21.parquet:Release`<br>`top_pad_24.parquet:Release` |
| `http://dbpedia.org/ontology/secretaryGeneral` | 11 | `metadata_101.parquet:General comments`<br>`GDP_Population_Military_Health_Metadata.parquet:General comments`<br>`clusters_1.parquet:general` |
| `http://dbpedia.org/ontology/translator` | 11 | `bkk_training_evaluation_id.parquet:Translation`<br>`BRZ.parquet:translations`<br>`BRZ_1.parquet:translations` |
| `http://dbpedia.org/ontology/waterPercentage` | 11 | `bulk-moisture-density%20(15).parquet:moisture and density::pore water mass [g]::number`<br>`bulk-moisture-density%20(24).parquet:moisture and density::pore water mass [g]::number`<br>`bulk-moisture-density%20(25).parquet:moisture and density::pore water mass [g]::number` |
| `http://dbpedia.org/ontology/wikiPageInDegree` | 11 | `lab-notes-claimant.parquet:Page in prototype`<br>`lab-notes-claimant.parquet:Page in prototype`<br>`citeSents.parquet:In bibliography entries CITATION` |
| `http://dbpedia.org/ontology/wptTitle` | 11 | `VMA_info.parquet:Title on xls`<br>`VMA_info_10.parquet:Title on xls`<br>`VMA_info_12.parquet:Title on xls` |
| `http://dbpedia.org/ontology/alternativeName` | 10 | `Simple.parquet:allow_alternative_item`<br>`Simple_3.parquet:allow_alternative_item`<br>`Comparison_of_layout_engines_(HTML)_17.parquet:Alternative solution` |
| `http://dbpedia.org/ontology/altitude` | 10 | `Flight_3226_C.parquet:GPS_ALTITUDE`<br>`data_219.parquet:thirtycharacterthirtycharacter-Altitude`<br>`data_219.parquet:thirtyonecharactersvariablehere-Altitude` |
| `http://dbpedia.org/ontology/areaWater` | 10 | `factbook-oceania2.parquet:area_water`<br>`factbook_6.parquet:area_water`<br>`Results-2020-09-14.parquet:LAND COVER ELEMENTS:WATER>LAKE/PONDED/CONTAINER` |
| `http://dbpedia.org/ontology/artery` | 10 | `%E6%A9%9F%E5%B7%A7%E5%B0%91%E5%A5%B3%E3%81%AF%E5%82%B7%E3%81%A4%E3%81%8B%E3%81%AA%E3%81%84.parquet:artery`<br>`lab_502.parquet:arterial_ph`<br>`%E6%A9%9F%E5%B7%A7%E5%B0%91%E5%A5%B3%E3%81%AF%E5%82%B7%E3%81%A4%E3%81%8B%E3%81%AA%E3%81%84.parquet:artery` |
| `http://dbpedia.org/ontology/artificialSnowArea` | 10 | `snow-monster.parquet:Snow`<br>`snow-white.parquet:Snow`<br>`snow-white.parquet:Snow` |
| `http://dbpedia.org/ontology/chorusCharacterInPlay` | 10 | `rebecca261.parquet:Hey Mr. Tambourine Man, play a song for me, I'm not sleepy a...`<br>`sensationinnerspace.parquet:Hey Mr. Tambourine Man, play a song for me, I'm not sleepy a...`<br>`rebecca261.parquet:Hey Mr. Tambourine Man, play a song for me, I'm not sleepy a...` |
| `http://dbpedia.org/ontology/distanceToDouglas` | 10 | `HRB_1.parquet:H&R Block adds to bargain-hunter rally inspired by bets on reopening, Biden`<br>`LZB.parquet:Longtime La-Z-Boy CEO Darrow to retire, CFO to take helm`<br>`trackLog-2019-ott-08_12-59-20_1.parquet:Distance to empty (Estimated)(km)` |
| `http://dbpedia.org/ontology/father` | 10 | `FatherMatches_1.parquet:FatherReliability`<br>`FatherMatches_1.parquet:exp.mmrateFather`<br>`FatherMatches_1.parquet:mmrateFather` |
| `http://dbpedia.org/ontology/finalLost` | 10 | `test2_2021_Jan_27_1148.parquet:finalOpacity`<br>`LTAR_SoilVarList_20210513.parquet:Final attribute`<br>`LTAR_SoilVarList_20210513.parquet:Final attribute` |
| `http://dbpedia.org/ontology/fuelTypeName` | 10 | `Black_Carbon.parquet:GeoTypeName`<br>`terrain.parquet:waterTypeID`<br>`mail_types.parquet:typeName` |
| `http://dbpedia.org/ontology/gridReference` | 10 | `metaParameters.parquet:mapGridXDimension`<br>`metaParameters.parquet:townGridDimension`<br>`OxfordBAI_cons.parquet:gridRef` |
| `http://dbpedia.org/ontology/headOfFamily` | 10 | `ProgramParticipation.parquet:RelationshipToHeadOfHousehold`<br>`misslyons.parquet:My severe injuries had healed and the sweet taste of blood c...`<br>`007.parquet:NUTRITION FOR THE FAMILY` |
| `http://dbpedia.org/ontology/inclination` | 10 | `Solar%20System.parquet:Inclination of Axis (degrees)`<br>`Solar_System.parquet:Inclination of Axis (degrees)`<br>`solar.parquet:Inclination of Axis (degrees)` |
| `http://dbpedia.org/ontology/leadYear` | 10 | `Lokad_Item.parquet:SupplierLeadTime`<br>`Lokad_Items.parquet:SupplierLeadTime`<br>`MRPTests_4.parquet:LeadTime` |
| `http://dbpedia.org/ontology/literaryGenre` | 10 | `031.parquet:DETECTIVE LITERATURE`<br>`031_9.parquet:DETECTIVE LITERATURE`<br>`books.computers-in-fiction_17.parquet:Books > Fiction > Computers in Literature` |
| `http://dbpedia.org/ontology/mainCharacter` | 10 | `Data_82.parquet:main_image`<br>`article_complete_1.parquet:main`<br>`Comparison_of_programming_paradigms-0_1.parquet:Main traits` |
| `http://dbpedia.org/ontology/mayorArticle` | 10 | `url-versions-2015-06-14-clean-test-fold-5_3.parquet:articleHeadline`<br>`url-versions-2015-06-14-clean-test-fold-10.parquet:articleHeadline`<br>`url-versions-2015-06-14-clean-test-fold-5_3.parquet:articleHeadline` |
| `http://dbpedia.org/ontology/nextEntity` | 10 | `Agreement_10.parquet:procuringEntity`<br>`Agreement_13.parquet:procuringEntity`<br>`CloseFrameworkAgreementUA.parquet:procuringEntity` |
| `http://dbpedia.org/ontology/nlaId` | 10 | `C_zp.lgt.parquet:Vop = vivre`<br>`down0689--iot-services.yaml.json.parquet:IotCaCertId`<br>`five_valid.parquet:henry_draper_catalog_id` |
| `http://dbpedia.org/ontology/numberOfCity` | 10 | `%E4%BA%BA%E6%95%99%E7%89%88%E9%AB%98%E4%B8%AD%E8%8B%B1%E8%AF%AD11%20-%20%E9%80%89%E4%BF%AE.parquet:n. the largest city and principal port of New Zealand`<br>`%E4%BA%BA%E6%95%99%E7%89%88%E9%AB%98%E4%B8%AD%E8%8B%B1%E8%AF%AD11%20-%20%E9%80%89%E4%BF%AE.parquet:n. the largest city and principal port of New Zealand`<br>`%E4%BA%BA%E6%95%99%E7%89%88%E9%AB%98%E4%B8%AD%E8%8B%B1%E8%AF%AD11%20-%20%E9%80%89%E4%BF%AE.parquet:n. the largest city and principal port of New Zealand` |
| `http://dbpedia.org/ontology/numberOfTurns` | 10 | `stats_24.parquet:no._of_pauses`<br>`stats_27.parquet:no._of_pauses`<br>`stats_31.parquet:no._of_pauses` |
| `http://dbpedia.org/ontology/percentageFat` | 10 | `nodes_metadata_1.parquet:percent_loss`<br>`nodes_metadata_2.parquet:percent_loss`<br>`raw_3cb_feats_1.parquet:lipid_lesion_percent` |
| `http://dbpedia.org/ontology/staff` | 10 | `districts-2021-05-03.parquet:inperson_staff`<br>`districts-2021-05-03.parquet:total_staff`<br>`clusters_1.parquet:staff` |
| `http://dbpedia.org/ontology/style` | 10 | `NIST2_Sandia_Helium_Plume_dataplot_config.parquet:Cmp_Marker_Style`<br>`gis_layer_feature_18.parquet:Style`<br>`gis_layer_feature_2.parquet:Style` |
| `http://dbpedia.org/ontology/summerTemperature` | 10 | `data_science-data.parquet:cooling_degree_days`<br>`data_science-data.parquet:heating_degree_days`<br>`data_science-data_1.parquet:cooling_degree_days` |
| `http://dbpedia.org/ontology/workArea` | 10 | `Influenza_Benchmarks_Crosswalk_05272020.parquet:WHO Benchmarks Technical Area`<br>`bf93f92d-47fb-41db-bffa-f9c2236b28a4_tags.parquet:local area\close community\make sure\great program\make sure student\ready\choose western`<br>`bf93f92d-47fb-41db-bffa-f9c2236b28a4_tags.parquet:local area\close community\make sure\great program\make sure student\ready\choose western` |

Total `no_finetype_equivalent` classes surfaced: **762**.
