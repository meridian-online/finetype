---
attestor: @hughcameron
attest_date: TBD
sampling_seed: 20260520
samples_per_cell: 3
pass_rate_threshold: 0.9
---

# ac-12 — Per-cell spot-check on Part 1 (corroborated gaps)

For each gap below, flip the verdict line to **PASS** or **FAIL**. A gap passes only if **all three** of the following hold:

1. **(a) Mechanism fits the evidence.** The sample_evidence rows exhibit the column failure pattern that the assigned `mechanism_token` describes per MADRs 0075 / 0081.
2. **(b) Lenses genuinely disagree with Sense.** Both YDF and the cascade independently point at a different answer than Sense's prediction (not spurious — i.e. not e.g. both lenses happen to be wrong in the same direction by coincidence).
3. **(c) Token is correct.** The assigned `mechanism_token` is the closest-fitting one from the closed 10-token set.

Partial failures (any one of 3 conditions failing) count as full failures. Per-cell threshold: pass_rate ≥ 0.9. With 3 samples per cell, that means all 3 must pass.

**Failure consequence (per spec):** if a cell's pass rate falls below threshold, all gaps in that cell are demoted to `single_lens_signals.tsv` and the demotion is logged in `progress.md`.

## Cell: `non_trivial_floor` × `format_diversity_path_b`

Sampled 3 of 2336 gaps in this cell.

### Sample 1 — `a57a3a01a7a6…`

- **gap_id**: `a57a3a01a7a680485f4206d68bf7cf063f51f17099b5ba77effc9059ecb93365`
- **affected_column_count**: 2
- **recommended_action_class**: `model_retrain`
- **corroborating_lenses**:
  - **ydf**: `representation.text.sentence` (conf 0.65)
  - **cascade**: `format_diversity_path_b` (conf 1.00)

**Sample evidence**:

| file | column | sense | ydf | samples | dbpedia |
|---|---|---|---|---|---|
| `show_time/COVID-19_Expansion_cards_black_.parquet` | `text` | `representation.text.plain_text` | `representation.text.sentence` | `16 people. 39 days of ______. One Survivor.` `A recent laboratory study shows that undergraduates have 50…` `A shocking new poll reveals 96% of Americans support ______.` `A successful job interview begins with a firm handshake and…` `After years of ______, my wife is finally divorcing me.` | `http://dbpedia.org/ontology/sourceText` |
| `parent/COVID-19_Expansion_cards_black_.parquet` | `text` | `representation.text.plain_text` | `representation.text.sentence` | `16 people. 39 days of ______. One Survivor.` `A recent laboratory study shows that undergraduates have 50…` `A shocking new poll reveals 96% of Americans support ______.` `A successful job interview begins with a firm handshake and…` `After years of ______, my wife is finally divorcing me.` | `http://dbpedia.org/ontology/sourceText` |

**Verdict**: ☐ PASS  ☐ FAIL

**Reason (if FAIL)**: _to be filled by attestor_

### Sample 2 — `0c2f31b51309…`

- **gap_id**: `0c2f31b51309d0faa998691926d5b1afdc489b805d0dcc38ced3aff744e97667`
- **affected_column_count**: 1
- **recommended_action_class**: `model_retrain`
- **corroborating_lenses**:
  - **ydf**: `representation.text.entity_name` (conf 0.61)
  - **cascade**: `format_diversity_path_b` (conf 1.00)

**Sample evidence**:

| file | column | sense | ydf | samples | dbpedia |
|---|---|---|---|---|---|
| `question_time/jeff_leath_enron_com.parquet` | `Text` | `representation.text.plain_text` | `representation.text.entity_name` | `Strategic Sourcing ConferenceForwarded by Jeff LeathNAEnron…` `Strategic Sourcing ConferenceForwarded by Jeff LeathNAEnron…` | `http://dbpedia.org/ontology/sourceText` |

**Verdict**: ☐ PASS  ☐ FAIL

**Reason (if FAIL)**: _to be filled by attestor_

### Sample 3 — `e0eb6890ae08…`

- **gap_id**: `e0eb6890ae08060426de5965621dba73eab998509593650a807923c4a6e1e36a`
- **affected_column_count**: 2
- **recommended_action_class**: `model_retrain`
- **corroborating_lenses**:
  - **ydf**: `representation.text.sentence` (conf 0.57)
  - **cascade**: `format_diversity_path_b` (conf 1.00)

**Sample evidence**:

| file | column | sense | ydf | samples | dbpedia |
|---|---|---|---|---|---|
| `living_thing/pGPX_6.parquet` | `text` | `representation.text.plain_text` | `representation.text.sentence` | `Whenever Spiritmonger deals damage to a creature, put a +1/…` `Put a 3/3 green Elephant creature token onto the battlefiel…` `Imprint — When Chrome Mox enters the battlefield, you may e…` `Whenever equipped creature deals combat damage, put two cha…` `Destroy target nonland permanent and all other permanents w…` | `http://dbpedia.org/ontology/sourceText` |
| `thing/pGPX_3.parquet` | `text` | `representation.text.plain_text` | `representation.text.sentence` | `Whenever Spiritmonger deals damage to a creature, put a +1/…` `Put a 3/3 green Elephant creature token onto the battlefiel…` `Imprint — When Chrome Mox enters the battlefield, you may e…` `Whenever equipped creature deals combat damage, put two cha…` `Destroy target nonland permanent and all other permanents w…` | `http://dbpedia.org/ontology/sourceText` |

**Verdict**: ☐ PASS  ☐ FAIL

**Reason (if FAIL)**: _to be filled by attestor_

## Cell: `non_trivial_floor` × `misclassification`

Sampled 3 of 30865 gaps in this cell.

### Sample 1 — `63db51c7e35b…`

- **gap_id**: `63db51c7e35b3164a95e9dfcf50d24bfec071efb9d9a07be167c1f0a491d96c7`
- **affected_column_count**: 4
- **recommended_action_class**: `training_data_addition`
- **corroborating_lenses**:
  - **ydf**: `representation.text.entity_name` (conf 0.55)
  - **cascade**: `misclassification` (conf 1.00)

**Sample evidence**:

| file | column | sense | ydf | samples | dbpedia |
|---|---|---|---|---|---|
| `living_thing/13-14_420.parquet` | `Title` | `representation.text.plain_text` | `representation.text.entity_name` | `Continuous Deployment of Wordpress Plugins Using Kernl` | `http://dbpedia.org/ontology/title` |
| `whole/13-14_814.parquet` | `Title` | `representation.text.plain_text` | `representation.text.entity_name` | `Continuous Deployment of Wordpress Plugins Using Kernl` | `http://dbpedia.org/ontology/title` |
| `dead_air/13-14_63.parquet` | `Title` | `representation.text.plain_text` | `representation.text.entity_name` | `Continuous Deployment of Wordpress Plugins Using Kernl` | `http://dbpedia.org/ontology/title` |
| `growth_rate/13-14_111.parquet` | `Title` | `representation.text.plain_text` | `representation.text.entity_name` | `Continuous Deployment of Wordpress Plugins Using Kernl` | `http://dbpedia.org/ontology/title` |

**Verdict**: ☐ PASS  ☐ FAIL

**Reason (if FAIL)**: _to be filled by attestor_

### Sample 2 — `08593c1937d3…`

- **gap_id**: `08593c1937d3089dfbdee4630ede36eba262520fc6d34c9896b92f1cf293a914`
- **affected_column_count**: 1
- **recommended_action_class**: `training_data_addition`
- **corroborating_lenses**:
  - **ydf**: `representation.text.sentence` (conf 0.61)
  - **cascade**: `misclassification` (conf 1.00)

**Sample evidence**:

| file | column | sense | ydf | samples | dbpedia |
|---|---|---|---|---|---|
| `question_time/Robert_W=-Floyd.parquet` | `addition machine` | `representation.text.plain_text` | `representation.text.sentence` | `compilation regular expression integrated circuit` `compilation regular expression integrated circuit extended …` `paradigm programming` `exact approximate membership tester` `expected time bound selection` | `` |

**Verdict**: ☐ PASS  ☐ FAIL

**Reason (if FAIL)**: _to be filled by attestor_

### Sample 3 — `b4128af44888…`

- **gap_id**: `b4128af448881d541d374e6c280fcfcc9417e9d1e67e55adeef107f495ab3a82`
- **affected_column_count**: 2
- **recommended_action_class**: `training_data_addition`
- **corroborating_lenses**:
  - **ydf**: `representation.text.sentence` (conf 0.67)
  - **cascade**: `misclassification` (conf 1.00)

**Sample evidence**:

| file | column | sense | ydf | samples | dbpedia |
|---|---|---|---|---|---|
| `whole/cherish_x3.parquet` | `We often believe the more we practice a skill or technique -...` | `representation.text.plain_text` | `representation.text.sentence` | `A computer needs a manager to administer its operations, ju…` `It may seem quite novelistic to you, and I am willing to ag…` `King, eh? Very nice. And how'd you get that, eh? By exploit…` `Warmer water increases the moisture content of storms, and …` `The wine list looks so imposing that you finally give up la…` | `http://dbpedia.org/ontology/isPartOf` |
| `real_time/cherish_x3.parquet` | `We often believe the more we practice a skill or technique -...` | `representation.text.plain_text` | `representation.text.sentence` | `A computer needs a manager to administer its operations, ju…` `It may seem quite novelistic to you, and I am willing to ag…` `King, eh? Very nice. And how'd you get that, eh? By exploit…` `Warmer water increases the moisture content of storms, and …` `The wine list looks so imposing that you finally give up la…` | `http://dbpedia.org/ontology/isPartOf` |

**Verdict**: ☐ PASS  ☐ FAIL

**Reason (if FAIL)**: _to be filled by attestor_

## Cell: `reject_rate_ceil` × `code_vs_canonical_path_a`

Sampled 1 of 1 gaps in this cell.

### Sample 1 — `839ed8e4221b…`

- **gap_id**: `839ed8e4221b6057d2f59457ba0b7ce4888e3c3596a1e378f0f7c6536a10d47a`
- **affected_column_count**: 2
- **recommended_action_class**: `model_retrain`
- **corroborating_lenses**:
  - **ydf**: `representation.discrete.categorical` (conf 0.61)
  - **cascade**: `code_vs_canonical_path_a` (conf 1.00)

**Sample evidence**:

| file | column | sense | ydf | samples | dbpedia |
|---|---|---|---|---|---|
| `id/dna_filt.parquet` | `atom_id` | `representation.identifier.alphanumeric_id` | `representation.discrete.categorical` | `C1'` `C2` `C2'` `C3'` `C4` | `http://dbpedia.org/ontology/id` |
| `id/dna_full.parquet` | `atom_id` | `representation.identifier.alphanumeric_id` | `representation.discrete.categorical` | `C1'` `C2` `C2'` `C3'` `C4` | `http://dbpedia.org/ontology/id` |

**Verdict**: ☐ PASS  ☐ FAIL

**Reason (if FAIL)**: _to be filled by attestor_

## Cell: `reject_rate_ceil` × `enum_overfit`

Sampled 3 of 4 gaps in this cell.

### Sample 1 — `cdf4571a30e4…`

- **gap_id**: `cdf4571a30e4eb2c4592080e1cdebe6bfe432e9def25d081debbc68b12cb3712`
- **affected_column_count**: 1
- **recommended_action_class**: `validator_widening`
- **corroborating_lenses**:
  - **ydf**: `representation.discrete.categorical` (conf 0.68)
  - **cascade**: `enum_overfit` (conf 1.00)

**Sample evidence**:

| file | column | sense | ydf | samples | dbpedia |
|---|---|---|---|---|---|
| `id/elora_unit_conversion_mapping.parquet` | `display_unit` | `representation.scientific.measurement_unit` | `representation.discrete.categorical` | `kg` `g` `dg` `cg` `mg` | `http://dbpedia.org/ontology/militaryUnitSize` |

**Verdict**: ☐ PASS  ☐ FAIL

**Reason (if FAIL)**: _to be filled by attestor_

### Sample 2 — `211c7d391798…`

- **gap_id**: `211c7d391798b5e26ce6f36cc384fbdfcc00a39239fff8353aacabe4e1e19ee7`
- **affected_column_count**: 1
- **recommended_action_class**: `validator_widening`
- **corroborating_lenses**:
  - **ydf**: `representation.discrete.categorical` (conf 0.63)
  - **cascade**: `enum_overfit` (conf 1.00)

**Sample evidence**:

| file | column | sense | ydf | samples | dbpedia |
|---|---|---|---|---|---|
| `show_time/client_globals_2.parquet` | `BooleanValue` | `representation.boolean.terms` | `representation.discrete.categorical` | `boolean` `FALSE` | `http://dbpedia.org/ontology/value` |

**Verdict**: ☐ PASS  ☐ FAIL

**Reason (if FAIL)**: _to be filled by attestor_

### Sample 3 — `ecd834154222…`

- **gap_id**: `ecd83415422201be3008b87a241eff9c68a39b09f9182aff05a39b43c29cac06`
- **affected_column_count**: 2
- **recommended_action_class**: `validator_widening`
- **corroborating_lenses**:
  - **ydf**: `representation.discrete.categorical` (conf 0.53)
  - **cascade**: `enum_overfit` (conf 1.00)

**Sample evidence**:

| file | column | sense | ydf | samples | dbpedia |
|---|---|---|---|---|---|
| `beats_per_minute/data.parquet` | `unit.symbol` | `representation.scientific.measurement_unit` | `representation.discrete.categorical` | `kg` `g` `lbs` `s` `h` | `` |
| `data_rate/data_4.parquet` | `unit.symbol` | `representation.scientific.measurement_unit` | `representation.discrete.categorical` | `kg` `g` `lbs` `s` `h` | `` |

**Verdict**: ☐ PASS  ☐ FAIL

**Reason (if FAIL)**: _to be filled by attestor_

## Cell: `reject_rate_ceil` × `format_diversity_path_a`

Sampled 3 of 8 gaps in this cell.

### Sample 1 — `7925dc9e13e9…`

- **gap_id**: `7925dc9e13e9691e16cdfbbb09edcf7c8301c54f0ccd34c3c6c5a617a0e595e6`
- **affected_column_count**: 2
- **recommended_action_class**: `validator_widening`
- **corroborating_lenses**:
  - **ydf**: `representation.text.entity_name` (conf 0.70)
  - **cascade**: `format_diversity_path_a` (conf 1.00)

**Sample evidence**:

| file | column | sense | ydf | samples | dbpedia |
|---|---|---|---|---|---|
| `id/payment_intent_data_1.parquet` | `customer_id` | `representation.identifier.alphanumeric_id` | `representation.text.entity_name` | `cus_I0JJf8lbCaJBZS` `cus_I0JJf8lbCaJBZS` `cus_I0JJf8lbCaJBZS` `cus_IrzdXtpzmRAOIT` `cus_IrzdXtpzmRAOIT` | `http://dbpedia.org/ontology/id` |
| `id/payment_intent_data_3.parquet` | `customer_id` | `representation.identifier.alphanumeric_id` | `representation.text.entity_name` | `cus_I0JJf8lbCaJBZS` `cus_I0JJf8lbCaJBZS` `cus_I0JJf8lbCaJBZS` `cus_IrzdXtpzmRAOIT` `cus_IrzdXtpzmRAOIT` | `http://dbpedia.org/ontology/id` |

**Verdict**: ☐ PASS  ☐ FAIL

**Reason (if FAIL)**: _to be filled by attestor_

### Sample 2 — `2453b24e2e5e…`

- **gap_id**: `2453b24e2e5eb6a34ac8a9cfba1df2193747c9fe9a68be9abb4b60b939e051c1`
- **affected_column_count**: 15
- **recommended_action_class**: `validator_widening`
- **corroborating_lenses**:
  - **ydf**: `representation.text.plain_text` (conf 0.55)
  - **cascade**: `format_diversity_path_a` (conf 1.00)

**Sample evidence**:

| file | column | sense | ydf | samples | dbpedia |
|---|---|---|---|---|---|
| `data_rate/2019-02-09_software_table.parquet` | `VERSION_DATE` | `datetime.date.iso` | `representation.text.plain_text` | `2017-01-27` `TRUE` | `http://dbpedia.org/ontology/version` |
| `data_rate/2019-04-19_software_table.parquet` | `VERSION_DATE` | `datetime.date.iso` | `representation.text.plain_text` | `2017-01-27` `TRUE` | `http://dbpedia.org/ontology/version` |
| `data_rate/2019-05-01_software_table.parquet` | `VERSION_DATE` | `datetime.date.iso` | `representation.text.plain_text` | `2017-01-27` `TRUE` | `http://dbpedia.org/ontology/version` |
| `data_rate/2019-06-02_software_table.parquet` | `VERSION_DATE` | `datetime.date.iso` | `representation.text.plain_text` | `2017-01-27` `TRUE` | `http://dbpedia.org/ontology/version` |
| `data_rate/2019-12-10_software_table.parquet` | `VERSION_DATE` | `datetime.date.iso` | `representation.text.plain_text` | `2017-01-27` `TRUE` | `http://dbpedia.org/ontology/version` |

**Verdict**: ☐ PASS  ☐ FAIL

**Reason (if FAIL)**: _to be filled by attestor_

### Sample 3 — `cd8ec93a4780…`

- **gap_id**: `cd8ec93a478017266f0d29f30e92cfd6534e265bb31d2c2b57f12b50b230220c`
- **affected_column_count**: 1
- **recommended_action_class**: `validator_widening`
- **corroborating_lenses**:
  - **ydf**: `representation.numeric.decimal_number` (conf 0.84)
  - **cascade**: `format_diversity_path_a` (conf 1.00)

**Sample evidence**:

| file | column | sense | ydf | samples | dbpedia |
|---|---|---|---|---|---|
| `id/Test_CheckReservoirStations_out_EvapStations.parquet` | `WEIGHT (%)` | `identity.person.weight` | `representation.numeric.decimal_number` | `-90.0` `100.0` | `http://dbpedia.org/ontology/weight` |

**Verdict**: ☐ PASS  ☐ FAIL

**Reason (if FAIL)**: _to be filled by attestor_

## Cell: `reject_rate_ceil` × `format_diversity_path_b`

Sampled 3 of 767 gaps in this cell.

### Sample 1 — `7555f3869e52…`

- **gap_id**: `7555f3869e52c6b59d5b95c3eec56746e21d575b4aa1c075f951a0fd09e1328a`
- **affected_column_count**: 1
- **recommended_action_class**: `model_retrain`
- **corroborating_lenses**:
  - **ydf**: `representation.text.entity_name` (conf 0.51)
  - **cascade**: `format_diversity_path_b` (conf 1.00)

**Sample evidence**:

| file | column | sense | ydf | samples | dbpedia |
|---|---|---|---|---|---|
| `id/SOLUTION_Operating_Cost_per_Functional_Unit_per_Annum_5.parquet` | `World / Drawdown Region` | `geography.location.continent` | `representation.text.entity_name` | `Middle East and Africa` `Middle East and Africa` `Middle East and Africa` `Middle East and Africa` `OECD90` | `http://dbpedia.org/ontology/region` |

**Verdict**: ☐ PASS  ☐ FAIL

**Reason (if FAIL)**: _to be filled by attestor_

### Sample 2 — `0a86df7d3263…`

- **gap_id**: `0a86df7d326343afbe16bc794090bd4e6864f2b7178585a7be7ec1e1619b35f5`
- **affected_column_count**: 1
- **recommended_action_class**: `model_retrain`
- **corroborating_lenses**:
  - **ydf**: `geography.address.full_address` (conf 0.77)
  - **cascade**: `format_diversity_path_b` (conf 1.00)

**Sample evidence**:

| file | column | sense | ydf | samples | dbpedia |
|---|---|---|---|---|---|
| `parent/2018-06-11.parquet` | `ADDRESS ZIP` | `geography.address.postal_code` | `geography.address.full_address` | `Unit 2096 Box 2290 DPO AP 50285` `Unit 8043 Box 6177 DPO AE 57755` `08901 Mays Spring South Alyssa, SC 09712` `51917 Samantha Manors Suite 786 Port Brian, OK 49125` `34266 Walker Fall Suite 120 Dennistown, HI 98842` | `http://dbpedia.org/ontology/address` |

**Verdict**: ☐ PASS  ☐ FAIL

**Reason (if FAIL)**: _to be filled by attestor_

### Sample 3 — `ddb54a6cdf58…`

- **gap_id**: `ddb54a6cdf58d424e65e0b46d0f7431c2d034b586caea4c9b71df13392bbaefb`
- **affected_column_count**: 1
- **recommended_action_class**: `model_retrain`
- **corroborating_lenses**:
  - **ydf**: `geography.address.full_address` (conf 0.81)
  - **cascade**: `format_diversity_path_b` (conf 1.00)

**Sample evidence**:

| file | column | sense | ydf | samples | dbpedia |
|---|---|---|---|---|---|
| `parent/2018-03-31_1.parquet` | `ADDRESS ZIP` | `geography.address.postal_code` | `geography.address.full_address` | `64094 Bennett Courts Gonzalezburgh, HI 06374` `5749 Brandon Flats West Danielville, CT 06653` `9339 Sullivan Mountain Suite 302 Lake Brian, FL 53353` `75151 Megan Grove Suite 227 Davisberg, TX 91986` `883 Mcgrath Shoals Herrerashire, VT 66143` | `http://dbpedia.org/ontology/address` |

**Verdict**: ☐ PASS  ☐ FAIL

**Reason (if FAIL)**: _to be filled by attestor_

## Cell: `reject_rate_ceil` × `misclassification`

Sampled 3 of 30203 gaps in this cell.

### Sample 1 — `66010133df53…`

- **gap_id**: `66010133df53e20aab2e686688433a64024f78277ce33e8c7445309277233483`
- **affected_column_count**: 1
- **recommended_action_class**: `training_data_addition`
- **corroborating_lenses**:
  - **ydf**: `representation.discrete.categorical` (conf 0.71)
  - **cascade**: `misclassification` (conf 1.00)

**Sample evidence**:

| file | column | sense | ydf | samples | dbpedia |
|---|---|---|---|---|---|
| `then/REV.parquet` | `loses` | `representation.text.word` | `representation.discrete.categorical` | `(Reuters)` `(Reuters)` `Inc` `Inc` `filed` | `http://dbpedia.org/ontology/wins` |

**Verdict**: ☐ PASS  ☐ FAIL

**Reason (if FAIL)**: _to be filled by attestor_

### Sample 2 — `0860c99acb10…`

- **gap_id**: `0860c99acb10938deada953a9445f98f7a3db1153073e81a491f0ff2b250ec86`
- **affected_column_count**: 3
- **recommended_action_class**: `training_data_addition`
- **corroborating_lenses**:
  - **ydf**: `representation.text.entity_name` (conf 0.72)
  - **cascade**: `misclassification` (conf 1.00)

**Sample evidence**:

| file | column | sense | ydf | samples | dbpedia |
|---|---|---|---|---|---|
| `object/Designite_Dapper.SimpleCRUDTests(netcoreapp2.0)_MethodMetrics.parquet` | `Namespace` | `representation.text.word` | `representation.text.entity_name` | `Dapper.SimpleCRUDTests` `Dapper.SimpleCRUDTests` `Dapper.SimpleCRUDTests` `Dapper.SimpleCRUDTests` `Dapper.SimpleCRUDTests` | `` |
| `id/Designite_Dapper.SimpleCRUDTests(netcoreapp2.0)_DesignSmells.parquet` | `Namespace` | `representation.text.word` | `representation.text.entity_name` | `Dapper.SimpleCRUDTests` `Dapper.SimpleCRUDTests` `Dapper.SimpleCRUDTests` `Dapper.SimpleCRUDTests` `Dapper.SimpleCRUDTests` | `` |
| `id/Designite_Dapper.SimpleCRUDTests(netcoreapp2.0)_MethodMetrics.parquet` | `Namespace` | `representation.text.word` | `representation.text.entity_name` | `Dapper.SimpleCRUDTests` `Dapper.SimpleCRUDTests` `Dapper.SimpleCRUDTests` `Dapper.SimpleCRUDTests` `Dapper.SimpleCRUDTests` | `` |

**Verdict**: ☐ PASS  ☐ FAIL

**Reason (if FAIL)**: _to be filled by attestor_

### Sample 3 — `b863f8172da1…`

- **gap_id**: `b863f8172da1150c0b797f11ac00514549d596cc9d4c5c3ced966b6a7bde96aa`
- **affected_column_count**: 4
- **recommended_action_class**: `training_data_addition`
- **corroborating_lenses**:
  - **ydf**: `representation.text.entity_name` (conf 0.62)
  - **cascade**: `misclassification` (conf 1.00)

**Sample evidence**:

| file | column | sense | ydf | samples | dbpedia |
|---|---|---|---|---|---|
| `id/Sympodina.parquet` | `authors` | `identity.person.full_name` | `representation.text.entity_name` | `Subram. & Lodha` `Subram. & Lodha` | `http://dbpedia.org/ontology/author` |
| `id/Adhogamina.parquet` | `authors` | `identity.person.full_name` | `representation.text.entity_name` | `Subram. & Lodha` `Subram. & Lodha` | `http://dbpedia.org/ontology/author` |
| `id/Angulimaya.parquet` | `authors` | `identity.person.full_name` | `representation.text.entity_name` | `Subram. & Lodha` `Subram. & Lodha` | `http://dbpedia.org/ontology/author` |
| `id/Janannfeldtia.parquet` | `authors` | `identity.person.full_name` | `representation.text.entity_name` | `Subram. & Sekar` `Subram. & Sekar` | `http://dbpedia.org/ontology/author` |

**Verdict**: ☐ PASS  ☐ FAIL

**Reason (if FAIL)**: _to be filled by attestor_

## Cell: `reject_rate_ceil` × `validator_widening`

Sampled 3 of 381 gaps in this cell.

### Sample 1 — `75550339ad4b…`

- **gap_id**: `75550339ad4baf715cca5ef40c15f35718a823a6c55592e980e7af8fe2ca1f40`
- **affected_column_count**: 1
- **recommended_action_class**: `validator_widening`
- **corroborating_lenses**:
  - **ydf**: `geography.address.full_address` (conf 0.72)
  - **cascade**: `validator_widening` (conf 1.00)

**Sample evidence**:

| file | column | sense | ydf | samples | dbpedia |
|---|---|---|---|---|---|
| `object/1609422084917.parquet` | `EMAIL` | `identity.person.email` | `geography.address.full_address` | `42294 Foster Plaza West Danny, IA 06826` `6546 Cory Orchard Rogersmouth, NJ 15706` `45229 Drake Route Apt. 113 North Paul, MO 73439` `9361 Robinson Green Apt. 635 North Lynntown, NC 59694` `76430 Cindy Cove South Nicholas, FL 14230` | `http://dbpedia.org/ontology/address` |

**Verdict**: ☐ PASS  ☐ FAIL

**Reason (if FAIL)**: _to be filled by attestor_

### Sample 2 — `099beb68241d…`

- **gap_id**: `099beb68241d4b9b0d90c44b8cf6cebb25554633f78248669f8ccf6501d376c7`
- **affected_column_count**: 1
- **recommended_action_class**: `validator_widening`
- **corroborating_lenses**:
  - **ydf**: `geography.address.full_address` (conf 0.65)
  - **cascade**: `validator_widening` (conf 1.00)

**Sample evidence**:

| file | column | sense | ydf | samples | dbpedia |
|---|---|---|---|---|---|
| `half_life/ccpcurrentcsv_2.parquet` | `EMAIL` | `identity.person.email` | `geography.address.full_address` | `4812 Medina Cliffs South Jodyport, SC 90572` `404 Richard Creek Port James, DE 78158` `79919 Haley Walks Apt. 982 Amystad, KY 54622` `5525 Jones Fall Anthonychester, FL 90788` `3687 Johnson Mission South Christina, NE 46072` | `http://dbpedia.org/ontology/address` |

**Verdict**: ☐ PASS  ☐ FAIL

**Reason (if FAIL)**: _to be filled by attestor_

### Sample 3 — `dda85e6d07f2…`

- **gap_id**: `dda85e6d07f2661e833d53e9126e6e9aba582736e500d55847f44bc403460f33`
- **affected_column_count**: 1
- **recommended_action_class**: `validator_widening`
- **corroborating_lenses**:
  - **ydf**: `representation.numeric.integer_number` (conf 0.53)
  - **cascade**: `validator_widening` (conf 1.00)

**Sample evidence**:

| file | column | sense | ydf | samples | dbpedia |
|---|---|---|---|---|---|
| `id/page-data_3.parquet` | `URL` | `technology.internet.url` | `representation.numeric.integer_number` | `57954` `13400` `50270` `22829` `51777` | `http://dbpedia.org/ontology/fileURL` |

**Verdict**: ☐ PASS  ☐ FAIL

**Reason (if FAIL)**: _to be filled by attestor_

---

## Per-cell summary

| criterion | mechanism | sampled | passed | pass_rate | meets threshold? |
|---|---|---:|---:|---:|---|
| `non_trivial_floor` | `format_diversity_path_b` | 3 | TBD | TBD | TBD |
| `non_trivial_floor` | `misclassification` | 3 | TBD | TBD | TBD |
| `reject_rate_ceil` | `code_vs_canonical_path_a` | 1 | TBD | TBD | TBD |
| `reject_rate_ceil` | `enum_overfit` | 3 | TBD | TBD | TBD |
| `reject_rate_ceil` | `format_diversity_path_a` | 3 | TBD | TBD | TBD |
| `reject_rate_ceil` | `format_diversity_path_b` | 3 | TBD | TBD | TBD |
| `reject_rate_ceil` | `misclassification` | 3 | TBD | TBD | TBD |
| `reject_rate_ceil` | `validator_widening` | 3 | TBD | TBD | TBD |

**Spec close blocks** until every cell either passes (0.9 threshold) OR every failing cell is demoted per the spec's failure-consequence procedure.
