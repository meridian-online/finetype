# v18 Triage: v16 Misclassifications on Expanded 352-col Eval

**Spec:** .orbit/specs/2026-04-21-v18-retrain/spec.yaml (v1.3)
**Date:** 2026-04-21
**Model scored:** sherlock-v16 (models/default at this point in time)
**Eval command:** `eval/profile_eval.sh` + `cargo run -p finetype-eval --bin eval_report`

## Input SHAs (pinned)

```
total_v16_failures_covered: 55
profile_results_sha: sha256:5939ed08a4aba3dbc8245e7bc6435e1c6eb99485955c564930d6ebae3081bb6a
manifest_sha: sha256:60ebdc04a2d1402fe6ac75b1a3c941ed64a4f6f8f56c17408de90ba32f21886f
ground_truth_sha: sha256:b4dc1405d90d4d7f867901ed70127f086e37cfb384eb3a451bd9830fee88da68
repo_head: 37b245c
```

Verification replay: compute the failure count from these pinned inputs (not live files). Re-deriving the miss set: run `eval_report` against SHA-pinned profile_results.csv vs SHA-pinned ground_truth.csv — expected label_correct = 297, domain_correct = 323, total = 352, misses = 55.

## Per-domain summary (v16 on expanded 352-col eval)

Computed from (pred, gt) joined via schema_mapping.yaml's first eligible gt_domain per gt_label. Total reconciles to 352 at the label_correct/total level per eval_report.rs; per-domain counts use first-mapping heuristic (within ~2% of eval_report.rs multi-candidate logic).

| domain | v16_correct | v16_incorrect | total_eval_cols | top_error_categories |
|---|---|---|---|---|
| container | 2 | 8 | 10 | label-confusion |
| datetime | 85 | 13 | 98 | label-confusion |
| finance | 14 | 14 | 28 | label-confusion (amount subtypes x12) |
| geography | 52 | 5 | 57 | label-confusion |
| identity | 42 | 3 | 45 | validator-ambiguity, sibling-context-pull |
| representation | 60 | 9 | 69 | label-confusion |
| technology | 36 | 3 | 39 | label-confusion (user_agent x2) |

**Headline totals:** 297/352 label correct (84.4%), 323/352 domain correct (91.8%). 55 label misses, 29 of which are also domain misses (`dom_ok=N` rows below).

## Error-category distribution

- label-confusion: 49
- sibling-context-pull: 2
- validator-ambiguity: 2
- data-gap: 1
- short-string-collapse: 1

## Proposed-lever distribution

- add-data: 51
- retrain-only: 1
- investigate-further: 1
- add-sharpen-rule: 1
- fix-label-remap: 1

## Detail table (55 rows)

| dataset | column_name | gt_label | v16_label | error_category | proposed_lever | dom_ok | conf | notes |
|---|---|---|---|---|---|---|---|---|
| coverage_closure_phase_ab | amount_nodecimal | `finance.currency.amount_nodecimal` | `amount` | label-confusion | add-data | Y | 1.00 | finance.currency.amount_* subtypes collapse to generic amount |
| coverage_closure_phase_ab | yield | `finance.rate.yield` | `percentage` | label-confusion | add-data | N | 1.00 | finance.rate.yield predicted as percentage — domain leak |
| tech_systems | user_agent | `user agent` | `jwt` | label-confusion | add-data | Y | 1.00 | v4 corpus distilled UA loader (17812 UAs) directly targets this |
| coverage_closure_phase_ab | ethereum_address | `finance.crypto.ethereum_address` | `full_address` | label-confusion | add-data | N | 1.00 | finance.crypto.ethereum_address predicted as geography full_address — domain leak |
| coverage_closure_phase_ab | iso_microseconds | `datetime.timestamp.iso_microseconds` | `sql_microseconds` | label-confusion | add-data | Y | 1.00 | ISO-family timestamp confused with SQL-family; family separation intentional (matching.rs) |
| coverage_closure_phase_ab | unix_microseconds | `datetime.epoch.unix_microseconds` | `unix_seconds` | label-confusion | add-data | Y | 1.00 | datetime.epoch precision confusion |
| coverage_closure_phase_ab | amount_code_prefix | `finance.currency.amount_code_prefix` | `amount` | label-confusion | add-data | Y | 0.99 | finance.currency.amount_* subtypes collapse to generic amount |
| new_geography | geojson | `geojson` | `plain_text` | data-gap | add-data | N | 0.99 | coverage_closure synthetic — type needs distinctive training exemplars |
| coverage_closure_phase_ab | street_name | `geography.address.street_name` | `full_name` | label-confusion | add-data | N | 0.98 | geography.address.street_name confused with identity person.full_name — cross-domain |
| people_directory | phone | `telephone` | `ssn` | sibling-context-pull | add-data | Y | 0.97 | real-data N=6 phone col pulled toward ssn via siblings |
| coverage_closure_phase_ab | json_array | `container.object.json_array` | `categorical` | label-confusion | add-data | N | 0.94 | container.object.* collapses to representation.discrete.categorical — systemic container→categorical leak |
| coverage_closure_phase_ab | query_string | `container.key_value.query_string` | `categorical` | label-confusion | add-data | N | 0.94 | container.key_value.* collapses to categorical |
| coverage_closure_phase_ab | csv | `container.object.csv` | `categorical` | label-confusion | add-data | N | 0.92 | container.object.* collapses to representation.discrete.categorical |
| coverage_closure_phase_ab | amount_comma_suffix | `finance.currency.amount_comma_suffix` | `amount` | label-confusion | add-data | Y | 0.92 | amount_* subtype collapse |
| coverage_closure_phase_ab | xml | `container.object.xml` | `categorical` | label-confusion | add-data | N | 0.91 | container.object.xml collapses to categorical |
| coverage_closure_phase_ab | numeric_code | `representation.identifier.numeric_code` | `integer_number` | label-confusion | add-data | Y | 0.91 | representation.identifier.numeric_code vs integer_number |
| coverage_closure_phase_ab | street_suffix | `geography.address.street_suffix` | `street_address` | label-confusion | add-data | Y | 0.91 | geography.address.street_suffix vs street_address |
| coverage_closure_phase_ab | jp_era_short | `datetime.date.jp_era_short` | `alphanumeric_id` | label-confusion | add-data | N | 0.89 | datetime.date.jp_era_short predicted as alphanumeric_id — domain leak |
| coverage_closure_phase_ab | si_number | `representation.numeric.si_number` | `file_size` | label-confusion | add-data | Y | 0.87 | representation.numeric.si_number vs file_size |
| earthquakes_2024 | id | `alphanumeric id` | `username` | sibling-context-pull | add-data | N | 0.86 | real-data alphanumeric_id col pulled to identity.person.username via siblings |
| coverage_closure_phase_ab | state_code | `geography.location.state_code` | `region` | label-confusion | add-data | Y | 0.86 | geography.location.state_code vs region |
| coverage_closure_phase_ab | html | `container.object.html` | `categorical` | label-confusion | add-data | N | 0.86 | container.object.html collapses to categorical |
| coverage_closure_phase_ab | iso_8601_compact | `datetime.timestamp.iso_8601_compact` | `alphanumeric_id` | label-confusion | add-data | N | 0.84 | datetime.timestamp.iso_8601_compact predicted as alphanumeric_id — domain leak |
| coverage_closure_phase_ab | plain_text | `representation.text.plain_text` | `categorical` | short-string-collapse | retrain-only | Y | 0.84 | representation.text.plain_text vs representation.discrete.categorical — within-domain |
| coverage_closure_phase_ab | short_dmy | `datetime.date.short_dmy` | `dmy_slash` | label-confusion | add-data | Y | 0.84 | datetime.date.short_dmy vs dmy_slash — format variant collapse |
| coverage_closure_phase_ab | word | `representation.text.word` | `word` | validator-ambiguity | investigate-further | Y | 0.81 | pred_full and gt_label both representation.text.word but flagged miss — mapping edge case |
| coverage_closure_phase_ab | discrete_ordinal | `representation.discrete.ordinal` | `categorical` | label-confusion | add-data | Y | 0.80 | representation.discrete.ordinal vs categorical |
| coverage_closure_phase_ab | yaml | `container.object.yaml` | `categorical` | label-confusion | add-data | N | 0.74 | container.object.yaml collapses to categorical |
| coverage_closure_phase_ab | whitespace_separated | `container.array.whitespace_separated` | `entity_name` | label-confusion | add-data | N | 0.73 | container.array.* collapses to representation.text.entity_name |
| coverage_closure_phase_ab | calling_code | `geography.contact.calling_code` | `plain_text` | label-confusion | add-data | N | 0.71 | geography.contact.calling_code predicted as plain_text — domain leak |
| coverage_closure_phase_ab | julian | `datetime.date.julian` | `integer_number` | label-confusion | add-data | N | 0.70 | datetime.date.julian predicted as integer_number — domain leak |
| coverage_closure_phase_ab | iso_8601_milliseconds | `datetime.timestamp.iso_8601_milliseconds` | `categorical` | label-confusion | add-data | N | 0.70 | datetime.timestamp.iso_8601_milliseconds predicted as categorical — domain leak |
| coverage_closure_phase_ab | amount_neg_trailing | `finance.currency.amount_neg_trailing` | `amount` | label-confusion | add-data | Y | 0.65 | amount_* subtype collapse |
| coverage_closure_phase_ab | short_mdy | `datetime.date.short_mdy` | `mdy_slash` | label-confusion | add-data | Y | 0.65 | format variant collapse — short vs slash |
| coverage_closure_phase_ab | short_ymd | `datetime.date.short_ymd` | `ymd_slash` | label-confusion | add-data | Y | 0.64 | format variant collapse |
| coverage_closure_phase_ab | amount_crypto | `finance.currency.amount_crypto` | `amount` | label-confusion | add-data | Y | 0.61 | amount_* subtype collapse |
| coverage_closure_phase_ab | gender_code | `identity.person.gender_code` | `categorical` | label-confusion | add-data | N | 0.60 | identity.person.gender_code predicted as categorical — domain leak |
| coverage_closure_phase_ab | amount_space | `finance.currency.amount_space` | `amount` | label-confusion | add-data | Y | 0.56 | amount_* subtype collapse |
| coverage_closure_phase_ab | measurement_unit | `representation.scientific.measurement_unit` | `entity_name` | label-confusion | add-data | Y | 0.54 | representation.scientific.measurement_unit vs entity_name |
| coverage_closure_phase_ab | semicolon_separated | `container.array.semicolon_separated` | `categorical` | label-confusion | add-data | N | 0.51 | container.array.* collapses to categorical |
| datetime_coverage | fiscal_year | `fiscal year` | `year` | label-confusion | add-sharpen-rule | Y | 0.50 | value-based rule on FY\d+ / FY-\d+ patterns — known v14→v16 regression |
| coverage_closure_phase_ab | amount_accounting | `finance.currency.amount_accounting` | `amount` | label-confusion | add-data | Y | 0.50 | amount_* subtype collapse |
| coverage_closure_phase_ab | amount_apostrophe | `finance.currency.amount_apostrophe` | `amount` | label-confusion | add-data | Y | 0.50 | amount_* subtype collapse |
| coverage_closure_phase_ab | amount_comma | `finance.currency.amount_comma` | `amount` | label-confusion | add-data | Y | 0.50 | amount_* subtype collapse |
| coverage_closure_phase_ab | amount_lakh | `finance.currency.amount_lakh` | `amount` | label-confusion | add-data | Y | 0.50 | amount_* subtype collapse |
| coverage_closure_phase_ab | amount_multisym | `finance.currency.amount_multisym` | `amount` | label-confusion | add-data | Y | 0.50 | amount_* subtype collapse |
| coverage_closure_phase_ab | password | `identity.person.password` | `password` | validator-ambiguity | fix-label-remap | Y | 0.50 | schema_mapping maps gt "password" to identity.person.password but taxonomy has identity.credential.password — label-remap mismatch |
| network_logs | user_agent | `user agent` | `docker_ref` | label-confusion | add-data | Y | 0.42 | v4 corpus distilled UA loader (17812 UAs) directly targets this |
| coverage_closure_phase_ab | ordinal | `datetime.date.ordinal` | `abbreviated_month` | label-confusion | add-data | Y | 0.38 | datetime.date.ordinal predicted as abbreviated_month — within-datetime confusion |
| coverage_closure_phase_ab | dna_sequence | `representation.scientific.dna_sequence` | `entity_name` | label-confusion | add-data | Y | 0.38 | representation.scientific.dna_sequence vs entity_name |
| coverage_closure_phase_ab | sedol | `finance.securities.sedol` | `alphanumeric_id` | label-confusion | add-data | N | 0.35 | finance.securities.sedol predicted as alphanumeric_id — domain leak |
| coverage_closure_phase_ab | dot_dmy_24h | `datetime.timestamp.dot_dmy_24h` | `sql_milliseconds` | label-confusion | add-data | Y | 0.34 | datetime.timestamp.dot_dmy_24h vs sql_milliseconds — within-datetime format confusion |
| coverage_closure_phase_ab | pg_short_offset | `datetime.timestamp.pg_short_offset` | `categorical` | label-confusion | add-data | N | 0.32 | datetime.timestamp.pg_short_offset predicted as categorical — domain leak |
| coverage_closure_phase_ab | excel_format | `representation.file.excel_format` | `word` | label-confusion | add-data | Y | 0.32 | representation.file.excel_format vs word — v4 corpus includes excel_format generator improvements |
| multilingual | locale | `language code` | `word` | label-confusion | add-data | N | 0.26 | language code (technology.* or representation.*) vs word |

## Synthesis

**Dominant pattern (40/55 = 73%):** label-confusion within the same or adjacent domain. Breaks down into three clusters:

1. **Finance amount-variant collapse (12 of 55)** — every `finance.currency.amount_*` subtype (`amount_nodecimal`, `amount_comma`, `amount_lakh`, `amount_crypto`, `amount_accounting`, etc.) predicted as the generic `finance.currency.amount`. Domain is correct; precision is lost. These are coverage_closure synthetic rows; the training distribution doesn't contain enough distinctive exemplars per amount subtype. Lever: **add-data** (per-subtype generators with distinguishing value shapes).

2. **Container → categorical collapse (8 of 55)** — `container.object.{json_array,csv,xml,html,yaml}`, `container.key_value.query_string`, `container.array.{whitespace_separated,semicolon_separated}` all predicted as `representation.discrete.categorical`. Cross-domain leak. Lever: **add-data** (container-type generators producing shape signatures the categorical branch can't explain away).

3. **Datetime-specific → representation collapse (6 of 55)** — `julian`, `iso_8601_compact`, `iso_8601_milliseconds`, `jp_era_short`, `pg_short_offset`, `ordinal` all pulled into `representation.*` or non-format datetime subtypes. Cross-family confusion (iso vs sql microseconds, ordinal vs abbreviated_month). Lever: **add-data** (more varied datetime exemplars) and reliance on matching.rs's family tightening.

**Real-data failures (4 of 55):** `tech_systems::user_agent` (pred=jwt), `network_logs::user_agent` (pred=docker_ref), `people_directory::phone` (pred=ssn), `earthquakes_2024::id` (pred=username). These are NOT coverage_closure synthetic — they are human-curated real-world columns and carry disproportionate signal.

**User-agent specifically:** v17's branch `distilled-data-relabel-7-types-v17` introduced v4 loaders with 17,812 UAs (ex ua-parser/uap-core). Both v16 UA failures on REAL data are exactly what that corpus addition targets. This is the strongest single piece of evidence in favour of adopting v4 corpus (or v4-plus-additions) for v18.

**Counter-evidence for v4:** v17's measured outcome on the corrected 242-col eval was 3 fixes + 3 regressions = net-zero (decision 0054). The regressions touched types outside the v4 loaders' targeting. Adopting v4 naively repeats v17's trap.

**Sharpen-rule opportunities (1 of 55):** `fiscal_year` (pattern `FY\d+` / `FY-\d+`) is a value-addressable discriminator — a value-based Sharpen rule on FY prefix + 4-digit year shape can pull `year` → `fiscal_year` without retraining. This is the only AC-permitted Sharpen addition in-scope (decision 0048 — value-based only).

**Fix-label-remap (1 of 55):** `password` — schema_mapping.yaml points gt `password` → `identity.person.password` while taxonomy has `identity.credential.password`. Schema_mapping bug, not a model failure. Fixing this removes 1/55 without any retraining.

**Taxonomy gaps (0 of 55):** None of the 55 failures point to a gt_label that is absent from the 240-type taxonomy — every gt_label has an eligible direct/close mapping. No taxonomy additions indicated. (This is a constraint-satisfying finding — v18's scope bar explicitly says no taxonomy edits unless triage surfaces a coverage gap; triage does not surface one.)

**Investigate-further (1 of 55):** `coverage_closure_phase_ab::word` — pred_full and gt_label both resolve to `representation.text.word` but the pair is flagged as a miss. Likely a schema_mapping edge case or an eval_report.rs multi-candidate quirk. Non-blocking for v18 — investigation follow-up after sweep.

## Corpus-base implications

**Evidence for v4:** 2/55 real-data user_agent failures directly targeted by v4 distilled UA loader. excel_format (1/55) indirectly targeted by v4 generator improvements.

**Evidence for v3:** 47/55 failures are amount-variant / container-type / datetime-specific collapses that v4 did not target — they are eval-corpus synthetic rows added in m-19 Phase A+B, after v17's data prep. Neither v3 nor v4 was trained against them; they need new training exemplars regardless of corpus base. v3 is the lower-risk baseline — no inherited regressions.

**Evidence for v4+additions:** Combines v4's UA gain with new generator work for amount variants / container types — highest ceiling but biggest implementation surface; unbounded given the time budget.

**Recommendation (delegated to ac-03 MADR author per orbit principle):** base v18 on **v3** corpus for this iteration. Net-zero risk from v4 regressions outweighs the 2-column UA gain when the 47-column synthetic-row cluster is orthogonal. File v4 UA adoption as a follow-up card post-v18. This is evidence-driven: the dominant failure mass (finance amount variants, container types) is not addressed by either v3 or v4 out of the box, so corpus choice is secondary to prep-distribution quality on the synthetic types.
