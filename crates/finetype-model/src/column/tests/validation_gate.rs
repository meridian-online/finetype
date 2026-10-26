//! `label_validates_sample` — the ≥90% veto-consistency gate.
//!
//! The gate was rewritten for speed: it resolves the leaf's validator once and
//! calls the boolean-only `is_valid` (instead of building a full
//! `ValidationResult` per value and reading one field off it), and it stops as
//! soon as the 0.9 bar is arithmetically out of reach. Both are meant to be
//! answer-preserving, so these tests pin the ANSWER — including the two shapes
//! that a plausible-but-wrong version of each optimisation gets wrong:
//!
//!   * an early exit that bails on `passed / checked < 0.9` (ignoring how many
//!     values are still to come) returns false for a sample whose failures are
//!     all at the front;
//!   * a rewrite that treats "no validator for this leaf" as a pass returns
//!     true where the gate must return false.

use super::super::*;

fn v(xs: &[&str]) -> Vec<String> {
    xs.iter().map(|s| s.to_string()).collect()
}

/// A leaf whose validator accepts exactly `AAA`-shaped values.
fn triple_a_taxonomy() -> Taxonomy {
    let yaml = r#"
representation.identifier.alphanumeric_id:
  title: Alphanumeric ID
  designation: universal
  tier: [VARCHAR, identifier]
  samples: ["AAA"]
  validation:
    type: string
    pattern: '^[A-Z]{3}$'
"#;
    let mut tax = Taxonomy::from_yaml(yaml).unwrap();
    tax.compile_validators();
    tax
}

const LEAF: &str = "representation.identifier.alphanumeric_id";

#[test]
fn gate_passes_a_wholly_valid_sample() {
    let tax = triple_a_taxonomy();
    assert!(label_validates_sample(
        &tax,
        LEAF,
        &v(&["AAA", "BBB", "CCC", "DDD"])
    ));
}

#[test]
fn gate_rejects_a_wholly_invalid_sample() {
    let tax = triple_a_taxonomy();
    assert!(!label_validates_sample(
        &tax,
        LEAF,
        &v(&["aa", "bb", "cc", "dd"])
    ));
}

#[test]
fn gate_accepts_a_sample_whose_only_failure_is_first() {
    // 1 failure then 19 passes = 19/20 = 0.95, over the bar.
    //
    // This is the case an early exit written as "bail when passed/checked drops
    // below 0.9" gets WRONG: after the first value that ratio is 0/1, and the
    // 19 passing values that would have carried the sample over the bar are
    // never looked at. The reachability test has to account for the values still
    // to come, not just the ones already seen.
    let tax = triple_a_taxonomy();
    let mut sample = vec!["zzz".to_string()];
    sample.extend(v(&["AAA"; 19]));
    assert!(
        label_validates_sample(&tax, LEAF, &sample),
        "19/20 = 0.95 is over the 0.9 bar; a front-loaded failure must not decide it"
    );
}

#[test]
fn gate_rejects_at_exactly_one_failure_too_many() {
    // 2 failures then 18 passes = 18/20 = 0.90 — exactly ON the bar, so it
    // passes; 3 failures then 17 = 17/20 = 0.85 fails. Pins the boundary so an
    // early exit cannot quietly move it.
    let tax = triple_a_taxonomy();
    let mut on_bar = v(&["zzz", "yyy"]);
    on_bar.extend(v(&["AAA"; 18]));
    assert!(
        label_validates_sample(&tax, LEAF, &on_bar),
        "18/20 = 0.90 is exactly the bar and must pass"
    );

    let mut under_bar = v(&["zzz", "yyy", "xxx"]);
    under_bar.extend(v(&["AAA"; 17]));
    assert!(
        !label_validates_sample(&tax, LEAF, &under_bar),
        "17/20 = 0.85 is under the bar and must fail"
    );
}

#[test]
fn gate_ignores_empty_values_but_needs_a_non_empty_one() {
    let tax = triple_a_taxonomy();
    // Whitespace-only values are skipped, not counted as failures.
    assert!(label_validates_sample(
        &tax,
        LEAF,
        &v(&["AAA", "", "   ", "BBB"])
    ));
    // A sample with nothing to check cannot assert the leaf.
    assert!(!label_validates_sample(&tax, LEAF, &v(&["", "  ", "\t"])));
    assert!(!label_validates_sample(&tax, LEAF, &[]));
}

#[test]
fn gate_rejects_a_leaf_the_taxonomy_cannot_validate() {
    // No definition, and a definition with no validation schema, both mean "no
    // evidence" — the gate asserts nothing. A rewrite that treats an
    // unresolvable validator as a pass would flip both of these.
    let tax = triple_a_taxonomy();
    assert!(!label_validates_sample(
        &tax,
        "technology.internet.url",
        &v(&["https://example.com", "https://example.org"])
    ));

    let no_schema = Taxonomy::from_yaml(
        r#"
representation.text.word:
  title: Word
  designation: universal
  tier: [VARCHAR, text]
  samples: ["hello"]
"#,
    )
    .unwrap();
    assert!(!label_validates_sample(
        &no_schema,
        "representation.text.word",
        &v(&["hello", "world"])
    ));
}

#[test]
fn gate_answer_matches_the_full_validation_result_it_replaced() {
    // The differential check: for every sample below, the boolean-only gate must
    // agree with the same tally computed through `validate_value_for_label` —
    // the error-collecting call the gate used to make, one per value, whose
    // `is_valid` field was the only thing it read.
    let tax = triple_a_taxonomy();
    let samples: Vec<Vec<String>> = vec![
        v(&["AAA", "BBB", "CCC"]),
        v(&["AAA", "zzz"]),
        v(&[
            "zzz", "AAA", "AAA", "AAA", "AAA", "AAA", "AAA", "AAA", "AAA", "AAA",
        ]),
        v(&[
            "AAA", "AAA", "AAA", "AAA", "AAA", "AAA", "AAA", "AAA", "AAA", "zzz",
        ]),
        v(&["", "AAA", " ", "BBB"]),
        v(&["zzz"]),
        v(&[]),
    ];
    for sample in &samples {
        let mut checked = 0usize;
        let mut passed = 0usize;
        for value in sample {
            let t = value.trim();
            if t.is_empty() {
                continue;
            }
            if let Ok(res) = finetype_core::validator::validate_value_for_label(t, LEAF, &tax) {
                checked += 1;
                if res.is_valid {
                    passed += 1;
                }
            }
        }
        let reference = checked > 0 && (passed as f64) / (checked as f64) >= 0.9;
        assert_eq!(
            label_validates_sample(&tax, LEAF, sample),
            reference,
            "gate disagreed with the full-validation tally on {sample:?} ({passed}/{checked})"
        );
    }
}

/// A column of one repeated upper-case UUID is not vetoed or demoted:
/// RFC 9562 allows upper case, so the published rule accepts it.
#[test]
fn uuid_constant_column_keeps_type() {
    use finetype_core::validation_veto::{audited_safe_labels, evaluate_validation_veto};
    const UUID: &str = "representation.identifier.uuid";
    let tax = super::shipped_taxonomy();
    let first = "550E8400-E29B-41D4-A716-446655440000";
    let values: Vec<String> = vec![first.to_string(); 40];
    let refs: Vec<Option<&str>> = values.iter().map(|s| Some(s.as_str())).collect();
    let veto = evaluate_validation_veto(UUID, &refs, &tax, &audited_safe_labels());
    assert!(!veto.vetoed, "vetoed: {veto:?}");
    assert!(schema_fail_demotion(&values, UUID, &tax).is_none());
}

/// A column of one repeated spaced percentage (`50 %`, which the old rule
/// rejected) is not vetoed or demoted.
#[test]
fn percentage_constant_column_keeps_type() {
    use finetype_core::validation_veto::{audited_safe_labels, evaluate_validation_veto};
    const PERCENTAGE: &str = "representation.numeric.percentage";
    let tax = super::shipped_taxonomy();
    let values: Vec<String> = vec!["50 %".to_string(); 40];
    let refs: Vec<Option<&str>> = values.iter().map(|s| Some(s.as_str())).collect();
    let veto = evaluate_validation_veto(PERCENTAGE, &refs, &tax, &audited_safe_labels());
    assert!(!veto.vetoed, "vetoed: {veto:?}");
    assert!(schema_fail_demotion(&values, PERCENTAGE, &tax).is_none());
}

/// A column of one repeated IEC size (`4KiB`, which the old rule rejected) is
/// not vetoed or demoted.
#[test]
fn file_size_constant_column_keeps_type() {
    use finetype_core::validation_veto::{audited_safe_labels, evaluate_validation_veto};
    const FILE_SIZE: &str = "representation.file.file_size";
    let tax = super::shipped_taxonomy();
    let values: Vec<String> = vec!["4KiB".to_string(); 40];
    let refs: Vec<Option<&str>> = values.iter().map(|s| Some(s.as_str())).collect();
    let veto = evaluate_validation_veto(FILE_SIZE, &refs, &tax, &audited_safe_labels());
    assert!(!veto.vetoed, "vetoed: {veto:?}");
    assert!(schema_fail_demotion(&values, FILE_SIZE, &tax).is_none());
}

/// A column of one repeated UN M.49 region tag (`es-419`, which the old rule
/// rejected) is not vetoed or demoted.
#[test]
fn locale_code_constant_column_keeps_type() {
    use finetype_core::validation_veto::{audited_safe_labels, evaluate_validation_veto};
    const LOCALE_CODE: &str = "technology.code.locale_code";
    let tax = super::shipped_taxonomy();
    let values: Vec<String> = vec!["es-419".to_string(); 40];
    let refs: Vec<Option<&str>> = values.iter().map(|s| Some(s.as_str())).collect();
    let veto = evaluate_validation_veto(LOCALE_CODE, &refs, &tax, &audited_safe_labels());
    assert!(!veto.vetoed, "vetoed: {veto:?}");
    assert!(schema_fail_demotion(&values, LOCALE_CODE, &tax).is_none());
}

/// A column of one repeated `Sept 4, 2023` (the en-GB abbreviation, which the
/// old rule rejected) is not vetoed or demoted.
#[test]
fn abbreviated_month_constant_column_keeps_type() {
    use finetype_core::validation_veto::{audited_safe_labels, evaluate_validation_veto};
    const ABBREVIATED_MONTH: &str = "datetime.date.abbreviated_month";
    let tax = super::shipped_taxonomy();
    for value in ["Sept 4, 2023", "4 Feb 23"] {
        let values: Vec<String> = vec![value.to_string(); 40];
        let refs: Vec<Option<&str>> = values.iter().map(|s| Some(s.as_str())).collect();
        let veto = evaluate_validation_veto(ABBREVIATED_MONTH, &refs, &tax, &audited_safe_labels());
        assert!(!veto.vetoed, "{value}: vetoed: {veto:?}");
        // Vacuous for this label (it is not in schema_fail_demotion's allowlist);
        // the veto assertion above is the one that bites.
        assert!(schema_fail_demotion(&values, ABBREVIATED_MONTH, &tax).is_none());
    }
}

/// A column of one repeated RFC 3339 value with a fraction (or a lower-case
/// `z`), which the old rule rejected, is not vetoed or demoted.
#[test]
fn rfc_3339_constant_column_keeps_type() {
    use finetype_core::validation_veto::{audited_safe_labels, evaluate_validation_veto};
    const RFC_3339: &str = "datetime.timestamp.rfc_3339";
    let tax = super::shipped_taxonomy();
    for value in [
        "2024-01-01 10:00:00.123+05:30",
        "2024-01-01 10:00:00z",
        "2024-01-01 10:00:00 Z",
    ] {
        let values: Vec<String> = vec![value.to_string(); 40];
        let refs: Vec<Option<&str>> = values.iter().map(|s| Some(s.as_str())).collect();
        let veto = evaluate_validation_veto(RFC_3339, &refs, &tax, &audited_safe_labels());
        assert!(!veto.vetoed, "{value}: vetoed: {veto:?}");
        assert!(schema_fail_demotion(&values, RFC_3339, &tax).is_none());
    }
}

/// A column of one repeated `+05` offset (or lower-case `t`), which the old rule
/// rejected, is not vetoed or demoted.
#[test]
fn iso_8601_offset_constant_column_keeps_type() {
    use finetype_core::validation_veto::{audited_safe_labels, evaluate_validation_veto};
    const ISO_8601_OFFSET: &str = "datetime.timestamp.iso_8601_offset";
    let tax = super::shipped_taxonomy();
    for value in ["2024-01-01T10:00:00+05", "2024-01-01t10:00:00+05:30"] {
        let values: Vec<String> = vec![value.to_string(); 40];
        let refs: Vec<Option<&str>> = values.iter().map(|s| Some(s.as_str())).collect();
        let veto = evaluate_validation_veto(ISO_8601_OFFSET, &refs, &tax, &audited_safe_labels());
        assert!(!veto.vetoed, "{value}: vetoed: {veto:?}");
        assert!(schema_fail_demotion(&values, ISO_8601_OFFSET, &tax).is_none());
    }
}

/// A column of one repeated `XCG` (the Caribbean guilder, which the old
/// enum lacked) is not vetoed or demoted.
#[test]
fn currency_code_constant_column_keeps_type() {
    use finetype_core::validation_veto::{audited_safe_labels, evaluate_validation_veto};
    const CURRENCY_CODE: &str = "finance.currency.currency_code";
    let tax = super::shipped_taxonomy();
    let values: Vec<String> = vec!["XCG".to_string(); 40];
    let refs: Vec<Option<&str>> = values.iter().map(|s| Some(s.as_str())).collect();
    let veto = evaluate_validation_veto(CURRENCY_CODE, &refs, &tax, &audited_safe_labels());
    assert!(!veto.vetoed, "vetoed: {veto:?}");
    assert!(schema_fail_demotion(&values, CURRENCY_CODE, &tax).is_none());
}

/// `rfc_3339` now accepts a fraction, so a column of microsecond-offset values
/// validates under both `rfc_3339` and `sql_microseconds_offset`. Both are in
/// `labels/ceded_leaves.txt` (the model's dropped head), but neither is in
/// `CEDED_RECOVERY_LEAVES`: datetime formats belong to `datetime_format_refinement`,
/// so `ceded_leaf_recovery` never weighs them and the double match costs no
/// recall. Pinned so that adding either leaf to the recovery list is a visible
/// decision (the exactly-one-match gate would then defer on such columns).
#[test]
fn microsecond_offset_double_match_does_not_reach_ceded_leaf_recovery() {
    let tax = super::shipped_taxonomy();
    let sample: Vec<String> = [
        "2024-01-01 10:00:00.123456+05:30",
        "2024-02-03 11:12:13.654321-08:00",
        "2024-03-04 00:00:00.000001+00:00",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    assert!(super::super::label_validates_sample(
        &tax,
        "datetime.timestamp.rfc_3339",
        &sample
    ));
    assert!(super::super::label_validates_sample(
        &tax,
        "datetime.timestamp.sql_microseconds_offset",
        &sample
    ));
    for leaf in [
        "datetime.timestamp.rfc_3339",
        "datetime.timestamp.sql_microseconds_offset",
    ] {
        assert!(
            !super::super::CEDED_RECOVERY_LEAVES.contains(&leaf),
            "{leaf} entered CEDED_RECOVERY_LEAVES; the double match now defers recovery"
        );
    }
}

/// A column of one repeated `svc$` (a trailing dollar sign the old rule
/// rejected) is not vetoed or demoted.
#[test]
fn username_constant_column_keeps_type() {
    use finetype_core::validation_veto::{audited_safe_labels, evaluate_validation_veto};
    const USERNAME: &str = "identity.person.username";
    let tax = super::shipped_taxonomy();
    let values: Vec<String> = vec!["svc$".to_string(); 40];
    let refs: Vec<Option<&str>> = values.iter().map(|s| Some(s.as_str())).collect();
    let veto = evaluate_validation_veto(USERNAME, &refs, &tax, &audited_safe_labels());
    assert!(!veto.vetoed, "vetoed: {veto:?}");
    assert!(schema_fail_demotion(&values, USERNAME, &tax).is_none());
}

/// A column of one repeated `AB−` (U+2212, which the old rule rejected) is not
/// vetoed or demoted.
#[test]
fn blood_type_constant_column_keeps_type() {
    use finetype_core::validation_veto::{audited_safe_labels, evaluate_validation_veto};
    const BLOOD_TYPE: &str = "identity.person.blood_type";
    let tax = super::shipped_taxonomy();
    let values: Vec<String> = vec!["AB\u{2212}".to_string(); 40];
    let refs: Vec<Option<&str>> = values.iter().map(|s| Some(s.as_str())).collect();
    let veto = evaluate_validation_veto(BLOOD_TYPE, &refs, &tax, &audited_safe_labels());
    assert!(!veto.vetoed, "vetoed: {veto:?}");
    assert!(schema_fail_demotion(&values, BLOOD_TYPE, &tax).is_none());
}

/// Pins that a valid constant column (40 copies of the CMS worked example) is
/// kept: not vetoed, not demoted. It cannot exercise the loosening: the value
/// was accepted before, and a constant column of 3-start numbers is vetoed by
/// design (see `npi_mixed_column_is_kept_by_the_guard_and_vetoed`).
#[test]
fn npi_valid_constant_column_is_kept() {
    use finetype_core::validation_veto::{audited_safe_labels, evaluate_validation_veto};
    const NPI: &str = "identity.medical.npi";
    let tax = super::shipped_taxonomy();
    let values: Vec<String> = vec!["1234567893".to_string(); 40];
    let refs: Vec<Option<&str>> = values.iter().map(|s| Some(s.as_str())).collect();
    let veto = evaluate_validation_veto(NPI, &refs, &tax, &audited_safe_labels());
    assert!(!veto.vetoed, "vetoed: {veto:?}");
    assert!(schema_fail_demotion(&values, NPI, &tax).is_none());
}
