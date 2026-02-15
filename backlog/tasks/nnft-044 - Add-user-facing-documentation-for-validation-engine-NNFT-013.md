---
id: NNFT-044
title: Add user-facing documentation for validation engine (NNFT-013)
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-13 10:39'
updated_date: '2026-02-15 08:35'
labels:
  - documentation
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
NNFT-013 implemented a validation engine (validator.rs) with single-value validation, column validation with 4 strategies (quarantine, set-null, ffill, bfill), and taxonomy integration. However there's no user-facing documentation explaining how to use it.

Add documentation covering:
- The Infer → Validate → Transform pipeline concept
- How to use `finetype validate` CLI command
- Validation strategies and when to use each
- JSON Schema fragment format used by type definitions
- API usage examples for Rust library consumers
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 README or DEVELOPMENT.md section explaining the validation engine
- [x] #2 CLI help text for finetype validate is clear and complete
- [x] #3 Example usage for each validation strategy documented
- [x] #4 API documentation (rustdoc) for public validation types
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Added comprehensive validation documentation to README.md:

1. AC #1: New 'Data Validation' section in README covering Infer → Validate → Transform pipeline, CLI usage, strategies, schema format, and library API
2. AC #2: CLI help text already clear and complete (finetype validate --help shows all options with descriptions)
3. AC #3: Strategy comparison table with use-case guidance (quarantine for manual review, null for NULL-tolerant, ffill for time-series, bfill for backward scenarios)
4. AC #4: validator.rs already has complete rustdoc on all public types: ValidatorError, ValidationResult, ValidationError, ValidationCheck, InvalidStrategy, QuarantinedValue, ColumnStats, ColumnValidationResult, validate_value, validate_column, validate_value_for_label, validate_column_for_label"
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added user-facing documentation for the validation engine to README.md.

Changes:
- New 'Data Validation' section in README covering:
  - Infer → Validate → Transform pipeline concept
  - CLI usage examples for all modes (NDJSON, plain text with label, strategy selection, output formats)
  - Strategy comparison table with use-case guidance (quarantine, null, ffill, bfill)
  - JSON Schema fragment format explanation with YAML example
  - Library API usage examples for single-value and column validation

The existing rustdoc in validator.rs is already comprehensive for API consumers. The CLI help text is already clear and complete."
<!-- SECTION:FINAL_SUMMARY:END -->
