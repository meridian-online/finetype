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

/// A column of one repeated real TSID (13 Crockford characters, which the old
/// 32-hex rule rejected) is not vetoed or demoted.
#[test]
fn tsid_constant_column_keeps_type() {
    use finetype_core::validation_veto::{audited_safe_labels, evaluate_validation_veto};
    const TSID: &str = "technology.identifier.tsid";
    let tax = super::shipped_taxonomy();
    let values: Vec<String> = vec!["0JMVQ049NFN6K".to_string(); 40];
    let refs: Vec<Option<&str>> = values.iter().map(|s| Some(s.as_str())).collect();
    let veto = evaluate_validation_veto(TSID, &refs, &tax, &audited_safe_labels());
    assert!(!veto.vetoed, "vetoed: {veto:?}");
    assert!(schema_fail_demotion(&values, TSID, &tax).is_none());
}

/// The model still predicts `tsid` for 32-hex columns (trained on the old
/// samples). Such a column fails the new rule, is vetoed (tsid is on the
/// allowlist), and the shape fallback lands it on alphanumeric_id.
#[test]
fn tsid_label_on_32_hex_column_is_vetoed_to_alphanumeric_id() {
    use finetype_core::validation_veto::{
        audited_safe_labels, evaluate_validation_veto, veto_shape_fallback,
    };
    const TSID: &str = "technology.identifier.tsid";
    let tax = super::shipped_taxonomy();
    let values: Vec<String> = (0..40u64)
        .map(|i| {
            format!(
                "{:016x}{:016x}",
                0x0190a1b2c3d4e5f6u64 + i * 7919,
                0xa7b8c9d0e1f2a3b4u64 ^ (i * 104_729)
            )
        })
        .collect();
    let refs: Vec<Option<&str>> = values.iter().map(|s| Some(s.as_str())).collect();
    assert!(audited_safe_labels().contains(TSID));
    let veto = evaluate_validation_veto(TSID, &refs, &tax, &audited_safe_labels());
    assert!(veto.vetoed, "not vetoed: {veto:?}");
    assert_eq!(
        veto_shape_fallback(&refs),
        Some("representation.identifier.alphanumeric_id")
    );
}

/// Asserts a column of 40 copies of `value` is neither vetoed nor demoted for `label`.
fn assert_constant_column_kept(label: &str, value: &str) {
    use finetype_core::validation_veto::{audited_safe_labels, evaluate_validation_veto};
    let tax = super::shipped_taxonomy();
    let values: Vec<String> = vec![value.to_string(); 40];
    let refs: Vec<Option<&str>> = values.iter().map(|s| Some(s.as_str())).collect();
    let veto = evaluate_validation_veto(label, &refs, &tax, &audited_safe_labels());
    assert!(!veto.vetoed, "{value:?}: vetoed: {veto:?}");
    assert!(schema_fail_demotion(&values, label, &tax).is_none());
}

/// A column of one repeated space-padded object (which the old rule rejected)
/// keeps its type. This documents keep-type behaviour; it does not fail on the
/// old rule, because json is not vetoed or demoted under either rule.
#[test]
fn json_constant_column_keeps_type() {
    assert_constant_column_kept("container.object.json", " {\"a\": 1} ");
}

/// A column of one repeated single-line element keeps its type. An
/// empty-element tag (`<root/>`), which the old rule rejected, conforms but
/// does not count for discrimination (the detector keeps the old shape), so a
/// column of it is demoted by design; new-form constants are not pinned here.
#[test]
fn xml_constant_column_keeps_type() {
    assert_constant_column_kept("container.object.xml", "<a>1</a>");
}

/// A column of one repeated `key:` document keeps its type. A `---`-opened
/// document, which the old rule rejected, conforms but does not count for
/// discrimination (the detector keeps the old shape), so a column of it is
/// demoted by design; new-form constants are not pinned here.
#[test]
fn yaml_constant_column_keeps_type() {
    assert_constant_column_kept("container.object.yaml", "name: x\nage: 30");
}

/// A column of one repeated record with an empty field inside a quoted field
/// (which the old rule rejected) keeps its type. The plan's first such value,
/// `a,,c`, is accepted by the published rule but not by the detector, which
/// keeps empty-field text out of discrimination by design, so a column of it
/// is demoted; a quoted field is what the detector keeps. The old rule also
/// rejected `a,,c`, so that demotion is not a regression. RFC 4180 requires
/// the new rejections of a bare quote in an unquoted field and of a line break
/// outside quotes.
#[test]
fn csv_constant_column_keeps_type() {
    assert_constant_column_kept("container.object.csv", "\"a,,b\",1");
}

/// A column of one repeated pairs-only query string keeps its type. A string
/// with a bare name (`debug&page=2`, which the old rule rejected) conforms but
/// does not count for discrimination (the detector keeps the old shape), so a
/// column of it is demoted by design; new-form constants are not pinned here.
#[test]
fn query_string_constant_column_keeps_type() {
    assert_constant_column_kept("container.key_value.query_string", "page=2&q=rust");
}

/// `query_string` now accepts bare names beside a pair, but not a column of
/// bare names alone: ceded-leaf recovery does not promote a word column to it.
#[test]
fn query_string_does_not_recover_word_column() {
    let mut cc =
        ColumnClassifier::with_defaults(Box::new(crate::inference::MockClassifier::new("unknown")));
    cc.set_taxonomy(super::shipped_taxonomy());
    let words: Vec<String> = ["alpha", "beta", "gamma"]
        .iter()
        .cycle()
        .take(42)
        .map(|s| s.to_string())
        .collect();
    let r = cc
        .compose_from_sense("name", &words, "representation.text.word", 1.0)
        .unwrap();
    assert_ne!(r.label, "container.key_value.query_string");
    assert_ne!(
        r.disambiguation_rule.as_deref(),
        Some("ceded_leaf_recovery")
    );
}

/// A column of boolean-expression text is not promoted to `query_string`
/// either: `a == 1 && b == 2` splits on `&` into pieces the widened rule would
/// otherwise read as bare names and pairs.
#[test]
fn query_string_does_not_recover_expression_column() {
    let mut cc =
        ColumnClassifier::with_defaults(Box::new(crate::inference::MockClassifier::new("unknown")));
    cc.set_taxonomy(super::shipped_taxonomy());
    let exprs: Vec<String> = [
        "a == 1 && b == 2",
        "x == 3 && y == 4",
        "status == 1 && ok == 0",
    ]
    .iter()
    .cycle()
    .take(42)
    .map(|s| s.to_string())
    .collect();
    let r = cc
        .compose_from_sense("rule", &exprs, "representation.text.word", 1.0)
        .unwrap();
    assert_ne!(r.label, "container.key_value.query_string");
    assert_ne!(
        r.disambiguation_rule.as_deref(),
        Some("ceded_leaf_recovery")
    );
}

/// A column of HTML pages labelled `html` is not relabelled xml by ceded-leaf
/// recovery: the pages conform to the XML rule (XHTML is XML), but the xml
/// detector keeps them out of discrimination.
#[test]
fn xml_does_not_recover_html_column() {
    let mut cc =
        ColumnClassifier::with_defaults(Box::new(crate::inference::MockClassifier::new("unknown")));
    cc.set_taxonomy(super::shipped_taxonomy());
    let pages: Vec<String> = [
        "<!DOCTYPE html>\n<html><body><p>one</p></body></html>",
        "<!DOCTYPE html>\n<html><head><title>two</title></head><body><div>x</div></body></html>",
        "<!DOCTYPE html>\n<html lang=\"en\"><body><a href=\"/c\">three</a></body></html>",
    ]
    .iter()
    .cycle()
    .take(42)
    .map(|s| s.to_string())
    .collect();
    let r = cc
        .compose_from_sense("page", &pages, "container.object.html", 1.0)
        .unwrap();
    assert_ne!(r.label, "container.object.xml");
}

/// A column of one repeated bare-name query string, labelled `query_string`,
/// is demoted to `word`: the old rule rejected it too, so 0.6.64 did the same.
#[test]
fn query_string_bare_name_column_is_demoted() {
    let tax = super::shipped_taxonomy();
    let values: Vec<String> = vec!["debug&page=2".to_string(); 40];
    let demoted = schema_fail_demotion(&values, "container.key_value.query_string", &tax);
    assert_eq!(
        demoted.map(|(label, _)| label).as_deref(),
        Some("representation.text.word")
    );
}

/// The same function demotes a column of `---`-opened documents labelled
/// `yaml` to `word`, as 0.6.64 did (the old rule rejected them). `yaml` is not
/// on the production demotion list in `value_sharpen`, so only the validation
/// veto reaches such a column in a profile; this pins the function's reading.
#[test]
fn yaml_marker_column_is_demoted_by_the_function() {
    let tax = super::shipped_taxonomy();
    let values: Vec<String> = vec!["---\nname: x".to_string(); 40];
    let demoted = schema_fail_demotion(&values, "container.object.yaml", &tax);
    assert_eq!(
        demoted.map(|(label, _)| label).as_deref(),
        Some("representation.text.word")
    );
}

/// A column of multi-line HTML fragments labelled `html` is not relabelled xml
/// either: they conform to the XML rule but the detector keeps the old
/// single-line shape.
#[test]
fn xml_does_not_recover_html_fragment_column() {
    let mut cc =
        ColumnClassifier::with_defaults(Box::new(crate::inference::MockClassifier::new("unknown")));
    cc.set_taxonomy(super::shipped_taxonomy());
    let fragments: Vec<String> = [
        "<p>Hello</p>\n<p>World</p>",
        "<div>\n  <span>x</span>\n</div>",
        "<ul>\n<li>a</li>\n</ul>",
    ]
    .iter()
    .cycle()
    .take(42)
    .map(|s| s.to_string())
    .collect();
    for label in ["container.object.html", "representation.text.word"] {
        let r = cc
            .compose_from_sense("frag", &fragments, label, 1.0)
            .unwrap();
        assert_ne!(r.label, "container.object.xml", "from {label}");
    }
}

/// A column of one repeated Australian state code, which the old two-letter
/// rule rejected, keeps its type: the detector counts EN_AU codes.
#[test]
fn state_code_constant_column_keeps_type() {
    assert_constant_column_kept("geography.location.state_code", "NSW");
}

/// A column of one repeated upper-case cell index, which the old lower-case
/// rule rejected, keeps its type.
#[test]
fn h3_constant_column_keeps_type() {
    assert_constant_column_kept("geography.index.h3", "881F1D4887FFFFF");
}

/// A column of one repeated mixed-case point, which the old upper-case rule
/// rejected, keeps its type: the detector counts a keyword in any case with a
/// coordinate pair.
#[test]
fn wkt_constant_column_keeps_type() {
    assert_constant_column_kept("geography.format.wkt", "Point (10 10)");
}

/// The same holds for a lower-case keyword with no space before the parenthesis.
#[test]
fn wkt_mixed_case_column_keeps_type() {
    assert_constant_column_kept("geography.format.wkt", "point(1 2)");
}

/// `wkt` is a ceded leaf and now accepts mixed-case keywords, so a column of
/// parenthesised film or product titles must not be promoted to it.
#[test]
fn wkt_does_not_recover_title_column() {
    let mut cc =
        ColumnClassifier::with_defaults(Box::new(crate::inference::MockClassifier::new("unknown")));
    cc.set_taxonomy(super::shipped_taxonomy());
    let titles: Vec<String> = [
        "Triangle (1999)",
        "Point (of sale)",
        "Polygon (regular)",
        "Tin (small)",
        "Point (of sale)",
    ]
    .iter()
    .cycle()
    .take(42)
    .map(|s| s.to_string())
    .collect();
    for label in ["representation.text.word", "representation.text.plain_text"] {
        let r = cc.compose_from_sense("title", &titles, label, 1.0).unwrap();
        assert_ne!(r.label, "geography.format.wkt", "from {label}");
    }
}

/// A column of one repeated plus code keeps its type. Every form the old rule
/// rejected (a bare-plus code, a padded code, lower case) conforms but does not
/// count for discrimination (the detector keeps the first-defined form), so a
/// column of those is demoted by design and has no keep-type test; this value is
/// the first accepted one, which the old rule also took.
#[test]
fn plus_code_constant_column_keeps_type() {
    assert_constant_column_kept("geography.coordinate.plus_code", "849VCWC8+R9");
}

/// A column of one repeated padded plus code, which the old rule rejected, is
/// demoted to `word` by `schema_fail_demotion`, as on 0.6.64.
#[test]
fn plus_code_padded_column_is_demoted() {
    let tax = super::shipped_taxonomy();
    let values: Vec<String> = vec!["6P000000+".to_string(); 40];
    let demoted = schema_fail_demotion(&values, "geography.coordinate.plus_code", &tax);
    assert_eq!(
        demoted.map(|(label, _)| label).as_deref(),
        Some("representation.text.word")
    );
}

/// `plus_code` is a ceded leaf and now takes a padded code with a bare plus, so
/// a column of figures with a trailing plus (`22000000+`) must not be promoted.
#[test]
fn plus_code_does_not_recover_plus_suffixed_figures() {
    let mut cc =
        ColumnClassifier::with_defaults(Box::new(crate::inference::MockClassifier::new("unknown")));
    cc.set_taxonomy(super::shipped_taxonomy());
    let figures: Vec<String> = ["22000000+", "33000000+", "55000000+", "72000000+"]
        .iter()
        .cycle()
        .take(42)
        .map(|s| s.to_string())
        .collect();
    for label in [
        "representation.text.word",
        "representation.identifier.alphanumeric_id",
    ] {
        let r = cc
            .compose_from_sense("downloads", &figures, label, 1.0)
            .unwrap();
        assert_ne!(r.label, "geography.coordinate.plus_code", "from {label}");
        assert_ne!(
            r.disambiguation_rule.as_deref(),
            Some("ceded_leaf_recovery"),
            "from {label}"
        );
    }
}

/// A column of one repeated ordinary grid reference keeps its type (the value
/// the old rule also took; a 16-digit or UPS reference conforms but does not
/// count, so a column of it is demoted by design).
#[test]
fn mgrs_constant_column_keeps_type() {
    assert_constant_column_kept("geography.coordinate.mgrs", "18SUJ2348306479");
}

/// A column of one repeated 16-digit reference, which the old rule rejected, is
/// demoted by `schema_fail_demotion` as on 0.6.64.
#[test]
fn mgrs_sixteen_digit_column_is_demoted() {
    let tax = super::shipped_taxonomy();
    let values: Vec<String> = vec!["18SUJ2348316806479498".to_string(); 40];
    let demoted = schema_fail_demotion(&values, "geography.coordinate.mgrs", &tax);
    assert_eq!(
        demoted.map(|(label, _)| label).as_deref(),
        Some("representation.text.word")
    );
}

/// `mgrs` is a ceded leaf and now takes a UPS reference (a letter, two letters
/// and digits), so a column of product codes shaped `ABC1234` must not be
/// promoted to it.
#[test]
fn mgrs_does_not_recover_product_code_column() {
    let mut cc =
        ColumnClassifier::with_defaults(Box::new(crate::inference::MockClassifier::new("unknown")));
    cc.set_taxonomy(super::shipped_taxonomy());
    let codes: Vec<String> = ["ABC1234", "BCD5678", "AHK0042", "BMW2020", "ZAH1234567890"]
        .iter()
        .cycle()
        .take(42)
        .map(|s| s.to_string())
        .collect();
    for label in [
        "representation.text.word",
        "representation.identifier.alphanumeric_id",
    ] {
        let r = cc.compose_from_sense("sku", &codes, label, 1.0).unwrap();
        assert_ne!(r.label, "geography.coordinate.mgrs", "from {label}");
        assert_ne!(
            r.disambiguation_rule.as_deref(),
            Some("ceded_leaf_recovery"),
            "from {label}"
        );
    }
}

/// A column of one repeated 20-character geohash, which the old rule rejected,
/// keeps its type: the detector counts six to twenty characters.
#[test]
fn geohash_constant_column_keeps_type() {
    assert_constant_column_kept("geography.coordinate.geohash", "9q8yyk8yuvrs4kz5qqvz");
}

/// A column of one repeated short geohash, which the old rule rejected,
/// conforms but does not count (the detector needs six characters), so it is
/// demoted by `schema_fail_demotion` as on 0.6.64.
#[test]
fn geohash_short_column_is_demoted() {
    let tax = super::shipped_taxonomy();
    let values: Vec<String> = vec!["dr5r".to_string(); 40];
    let demoted = schema_fail_demotion(&values, "geography.coordinate.geohash", &tax);
    assert_eq!(
        demoted.map(|(label, _)| label).as_deref(),
        Some("representation.text.word")
    );
}

/// A column of one repeated comma-and-space pair, which the old rule rejected,
/// keeps its type (coordinates has no detector, so conforming is counting).
#[test]
fn coordinates_constant_column_keeps_type() {
    assert_constant_column_kept(
        "geography.coordinate.coordinates",
        "48.8761373390584, 2.3680844979417",
    );
}
