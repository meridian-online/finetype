//! Integration tests for the JSON Schema enum-emission gate.
//!
//! ac-09 of spec `2026-04-28-validate-precision-corpus` ships two distinct
//! sub-fixes around the enum-emission gate; these three tests pin both:
//!
//!   T1 (`pvc_enum_omitted_for_non_categorical_label`):
//!     a non-enum-eligible label (e.g. `datetime.offset.iana`) never gets
//!     an `enum` array, even with cardinality well under the threshold.
//!     Pins the pre-existing label-family gate from v0.6.19.
//!
//!   T2 (`pvc_enum_kept_for_boolean_terms_label`):
//!     a `representation.boolean.terms` column with low cardinality DOES
//!     receive an `enum` array under the new sub-fix (b) — boolean labels
//!     gain gate parity with categorical.
//!
//!   T3 (`pvc_enum_omitted_when_cardinality_exceeds_default`):
//!     an enum-eligible label with cardinality 40 (over the new default
//!     of 32) gets no `enum`. Pins sub-fix (a) — the lowered default —
//!     by hard-coding both the threshold value and the comparison.
//!
//! These tests verify the shared library primitive exported from
//! `finetype_cli::enum_emission`, which is the same primitive the
//! `finetype profile -o json-schema` binary calls. Drift between binary
//! and tests is therefore impossible.

use finetype_cli::enum_emission::{collect_unique_values_if_categorical, label_is_enum_eligible};

/// The clap default for `finetype profile --enum-threshold` after ac-09.
const PVC_DEFAULT_ENUM_THRESHOLD: usize = 32;

#[test]
fn pvc_enum_omitted_for_non_categorical_label() {
    // datetime.offset.iana is open-domain — no enum should ever attach,
    // even though 12 distinct values is well under the threshold.
    let values: Vec<String> = [
        "Australia/Sydney",
        "Europe/London",
        "America/New_York",
        "America/Los_Angeles",
        "Asia/Tokyo",
        "Asia/Shanghai",
        "Africa/Lagos",
        "Africa/Cairo",
        "America/Chicago",
        "Europe/Paris",
        "Europe/Berlin",
        "America/Sao_Paulo",
    ]
    .iter()
    .map(|s| (*s).to_string())
    .collect();

    assert!(!label_is_enum_eligible("datetime.offset.iana"));
    let result = collect_unique_values_if_categorical(
        "datetime.offset.iana",
        &values,
        PVC_DEFAULT_ENUM_THRESHOLD,
    );
    assert!(
        result.is_none(),
        "expected None for non-enum-eligible label, got {:?}",
        result
    );
}

#[test]
fn pvc_enum_kept_for_boolean_terms_label() {
    // representation.boolean.terms with 6 distinct yes/no values — sub-fix
    // (b) of ac-09 makes this enum-eligible alongside categorical.
    let values: Vec<String> = [
        "yes", "no", "y", "n", "true", "false", "yes", "yes", "no", "no",
    ]
    .iter()
    .map(|s| (*s).to_string())
    .collect();

    assert!(label_is_enum_eligible("representation.boolean.terms"));
    let result = collect_unique_values_if_categorical(
        "representation.boolean.terms",
        &values,
        PVC_DEFAULT_ENUM_THRESHOLD,
    );
    let enum_values = result.expect("expected Some(enum) for boolean.terms under threshold");
    assert_eq!(
        enum_values,
        vec!["false", "n", "no", "true", "y", "yes"],
        "enum values should be sorted unique"
    );
}

#[test]
fn pvc_enum_omitted_when_cardinality_exceeds_default() {
    // 40 distinct values labelled categorical → over default 32 → no enum.
    // Pins both the lowered default and the cardinality cap.
    let values: Vec<String> = (0..40).map(|i| format!("category_{i:02}")).collect();

    assert!(label_is_enum_eligible(
        "representation.discrete.categorical"
    ));
    let result = collect_unique_values_if_categorical(
        "representation.discrete.categorical",
        &values,
        PVC_DEFAULT_ENUM_THRESHOLD,
    );
    assert!(
        result.is_none(),
        "expected None when cardinality (40) > threshold (32), got {:?}",
        result.as_ref().map(|v| v.len())
    );

    // And confirm the exact threshold edge — 32 distinct passes, 33 fails.
    let v32: Vec<String> = (0..32).map(|i| format!("c_{i:02}")).collect();
    let v33: Vec<String> = (0..33).map(|i| format!("c_{i:02}")).collect();
    assert!(collect_unique_values_if_categorical(
        "representation.discrete.categorical",
        &v32,
        PVC_DEFAULT_ENUM_THRESHOLD
    )
    .is_some());
    assert!(collect_unique_values_if_categorical(
        "representation.discrete.categorical",
        &v33,
        PVC_DEFAULT_ENUM_THRESHOLD
    )
    .is_none());
}
