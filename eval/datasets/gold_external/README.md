# Gold External Datasets

This directory contains curated external datasets used for evaluation and validation.

## gleif_entities.csv

A registry slice of 200 entity records from GLEIF (Global Legal Entity Identifier Foundation), used as a regression fixture for LEI (Legal Entity Identifier) validation.

### Known Data Anomalies

Two LEI values in this file fail the ISO 7064 MOD 97-10 check digit validation while passing the ISO 17442 shape validation (20 uppercase alphanumeric characters). These are kept deliberately as test fixtures to distinguish pattern (shape) validation from checksum (substance) validation:

- `0292001629A3Q7XJ0D13` (line 115)
- `0292001684F9TE5J9417` (line 122)

These anomalies are asserted in `crates/finetype-core/tests/precision_widenings.rs` under the test `pvc_widening_lei_validates_the_committed_registry_slice()`. They remain in the fixture to ensure the precision principle is maintained: a valid LEI must pass both shape validation and checksum validation, not just one.
