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

/// A column of one repeated geohash keeps its type. Every form the old rule
/// rejected (1 to 5 and 13 to 20 characters) conforms but does not count for
/// discrimination (the detector keeps the old rule), so this is the first
/// accepted value, which the old rule also took.
#[test]
fn geohash_constant_column_keeps_type() {
    assert_constant_column_kept("geography.coordinate.geohash", "9q8yyk8yuv");
}

/// A column of one repeated short geohash, which the old rule rejected,
/// conforms but does not count (the detector needs six to twelve characters), so
/// it is demoted by `schema_fail_demotion` as on 0.6.64.
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

/// A column of one repeated `Crescent`, which the old enum rejected, keeps its
/// type: the detector counts the added full words. The national abbreviations
/// (`CRT`, `BVD`, ...) conform but do not count, so a column of them is demoted.
#[test]
fn street_suffix_constant_column_keeps_type() {
    assert_constant_column_kept("geography.address.street_suffix", "Crescent");
}

/// A column of one repeated `CRT`, which the old enum rejected, conforms but
/// does not count (it is also an airport code), so it is demoted by
/// `schema_fail_demotion` as on 0.6.64.
#[test]
fn street_suffix_abbreviation_column_is_demoted() {
    let tax = super::shipped_taxonomy();
    let values: Vec<String> = vec!["CRT".to_string(); 40];
    let demoted = schema_fail_demotion(&values, "geography.address.street_suffix", &tax);
    assert_eq!(
        demoted.map(|(label, _)| label).as_deref(),
        Some("representation.text.word")
    );
}

/// `street_suffix` is a ceded leaf and now takes `BD`, `AV`, `CRT`, `BVD`, so
/// columns of country, currency and airport codes must not be promoted to it.
/// (A Sense assertion of `street_suffix` on such a column is not vetoed, as on
/// 0.6.64, and is not tested here.)
#[test]
fn street_suffix_does_not_recover_code_columns() {
    let mut cc =
        ColumnClassifier::with_defaults(Box::new(crate::inference::MockClassifier::new("unknown")));
    cc.set_taxonomy(super::shipped_taxonomy());
    let columns: [(&str, [&str; 6]); 4] = [
        ("code", ["BD", "AV", "CRT", "BVD", "BOUL", "BD"]),
        ("country", ["BD", "FR", "DE", "CA", "AU", "BD"]),
        ("currency", ["BDT", "AUD", "CAD", "EUR", "BVD", "CRT"]),
        ("airport", ["CRT", "AVL", "BOU", "BVD", "AVP", "CRT"]),
    ];
    for (header, codes) in columns {
        let values: Vec<String> = codes
            .iter()
            .cycle()
            .take(42)
            .map(|s| s.to_string())
            .collect();
        for label in [
            "representation.text.word",
            "representation.identifier.alphanumeric_id",
        ] {
            let r = cc.compose_from_sense(header, &values, label, 1.0).unwrap();
            assert_ne!(
                r.label, "geography.address.street_suffix",
                "{header} from {label}"
            );
            assert_ne!(
                r.disambiguation_rule.as_deref(),
                Some("ceded_leaf_recovery"),
                "{header} from {label}"
            );
        }
    }
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

/// A column of one repeated 20-character geohash conforms but is demoted too,
/// as on 0.6.64.
#[test]
fn geohash_long_column_is_demoted() {
    let tax = super::shipped_taxonomy();
    let values: Vec<String> = vec!["9q8yyk8yuvrs4kz5qqvz".to_string(); 40];
    let demoted = schema_fail_demotion(&values, "geography.coordinate.geohash", &tax);
    assert_eq!(
        demoted.map(|(label, _)| label).as_deref(),
        Some("representation.text.word")
    );
}

/// A column of one repeated keycap, which the old class rejected (a digit is
/// not a symbol), keeps its type (emoji has no detector, so conforming is
/// counting).
#[test]
fn emoji_constant_column_keeps_type() {
    assert_constant_column_kept("representation.text.emoji", "1\u{fe0f}\u{20e3}");
}

/// `emoji` is a ceded leaf and now takes keycap sequences, which need U+20E3,
/// so columns of bare digits, `#` and `*` must not be promoted to it.
#[test]
fn emoji_does_not_recover_digit_columns() {
    let mut cc =
        ColumnClassifier::with_defaults(Box::new(crate::inference::MockClassifier::new("unknown")));
    cc.set_taxonomy(super::shipped_taxonomy());
    let columns: [(&str, [&str; 6]); 3] = [
        ("digit", ["1", "2", "3", "4", "5", "1"]),
        ("symbol", ["#", "*", "#", "*", "#", "*"]),
        ("rank", ["10", "2", "#3", "*", "7", "1"]),
    ];
    for (header, codes) in columns {
        let values: Vec<String> = codes
            .iter()
            .cycle()
            .take(42)
            .map(|s| s.to_string())
            .collect();
        for label in [
            "representation.text.word",
            "representation.identifier.alphanumeric_id",
        ] {
            let r = cc.compose_from_sense(header, &values, label, 1.0).unwrap();
            assert_ne!(
                r.label, "representation.text.emoji",
                "{header} from {label}"
            );
        }
    }
}

/// A column of one repeated `CC(=O)O` keeps its type. Every form the old rule
/// rejected (`*`, `$`) conforms but is not counted by the detector, so no new
/// form can pin keep-type.
#[test]
fn smiles_constant_column_keeps_type() {
    assert_constant_column_kept("representation.scientific.smiles", "CC(=O)O");
}

/// A column of one repeated wildcard SMILES, which the old rule rejected,
/// conforms but is not counted (the detector keeps the old shape), so
/// `schema_fail_demotion` demotes it as on 0.6.64. `smiles` is not on the veto
/// allowlist, so only the function is pinned.
#[test]
fn smiles_wildcard_column_is_demoted() {
    let tax = super::shipped_taxonomy();
    let values: Vec<String> = vec!["[*]CC".to_string(); 40];
    let demoted = schema_fail_demotion(&values, "representation.scientific.smiles", &tax);
    assert_eq!(
        demoted.map(|(label, _)| label).as_deref(),
        Some("representation.text.word")
    );
}

/// The detector keeps the old class, so every ACCEPT form the old class
/// rejected (`MKVBZU`) is not counted; no such value can pin keep-type. A
/// column of one repeated standard-letter sequence, which both rules accept,
/// keeps its type.
#[test]
fn protein_sequence_constant_column_keeps_type() {
    assert_constant_column_kept(
        "representation.scientific.protein_sequence",
        "MKVLLIVGSLLIVGKQ",
    );
}

/// A column of one repeated `MKVBZU`, which the old rule rejected, conforms but
/// does not count (the detector keeps the old class), so it is demoted by
/// `schema_fail_demotion` as on 0.6.64.
#[test]
fn protein_sequence_ambiguity_column_is_demoted() {
    let tax = super::shipped_taxonomy();
    let values: Vec<String> = vec!["MKVBZU".to_string(); 40];
    let demoted = schema_fail_demotion(&values, "representation.scientific.protein_sequence", &tax);
    assert_eq!(
        demoted.map(|(label, _)| label).as_deref(),
        Some("representation.text.word")
    );
}

/// `protein_sequence` now takes B, Z, U, O and `-`, but a column of uppercase
/// words is not kept as protein_sequence by `schema_fail_demotion` (the
/// detector keeps the old class, which rejects `HELLO`'s O, `WORLD`'s O and
/// `BOOK`'s B and O).
#[test]
fn protein_sequence_does_not_recover_uppercase_words() {
    let tax = super::shipped_taxonomy();
    let values: Vec<String> = ["HELLO", "WORLD", "BOOK"]
        .iter()
        .cycle()
        .take(42)
        .map(|s| s.to_string())
        .collect();
    let demoted = schema_fail_demotion(&values, "representation.scientific.protein_sequence", &tax);
    assert!(
        demoted.is_some(),
        "uppercase words kept as protein_sequence"
    );
}

/// A column of one repeated `rgba(0, 0, 255, 0.5)` (the old rule rejected the
/// `rgba` head and the alpha) keeps its type.
#[test]
fn color_rgb_constant_column_keeps_type() {
    assert_constant_column_kept("representation.format.color_rgb", "rgba(0, 0, 255, 0.5)");
}

/// A column of one repeated modern-syntax `hsl(120deg 75% 25% / 60%)` (the old
/// rule took only comma-separated percentages) keeps its type.
#[test]
fn color_hsl_constant_column_keeps_type() {
    assert_constant_column_kept(
        "representation.format.color_hsl",
        "hsl(120deg 75% 25% / 60%)",
    );
}

/// `color_hsl` is a ceded-recovery leaf and now takes the space-separated
/// syntax, but the rule is anchored by `hsl(` / `hsla(`, so a neighbouring
/// colour function (rgb, or hsl with a component missing) is not promoted.
#[test]
fn color_hsl_does_not_recover_neighbouring_colour_columns() {
    let mut cc =
        ColumnClassifier::with_defaults(Box::new(crate::inference::MockClassifier::new("unknown")));
    cc.set_taxonomy(super::shipped_taxonomy());
    let columns: [(&str, [&str; 3]); 3] = [
        (
            "fill",
            ["rgb(255 0 0)", "rgb(0 128 0 / 50%)", "rgb(10 20 30)"],
        ),
        ("tint", ["hsl(120 75%)", "hsl(240 50%)", "hsl(0 100%)"]),
        (
            "shade",
            ["hwb(120 10% 20%)", "lab(50% 40 59)", "oklch(60% 0.2 30)"],
        ),
    ];
    for (header, codes) in columns {
        let values: Vec<String> = codes
            .iter()
            .cycle()
            .take(42)
            .map(|s| s.to_string())
            .collect();
        for label in [
            "representation.text.word",
            "representation.identifier.alphanumeric_id",
        ] {
            let r = cc.compose_from_sense(header, &values, label, 1.0).unwrap();
            assert_ne!(
                r.label, "representation.format.color_hsl",
                "{header} from {label}"
            );
        }
    }
}

/// A column of one repeated `#0000ffcc` (the old rule took 3 or 6 digits only)
/// keeps its type.
#[test]
fn color_hex_constant_column_keeps_type() {
    assert_constant_column_kept("representation.format.color_hex", "#0000ffcc");
}

/// `color_hex` is a ceded leaf (it is not on the ceded-recovery list, its
/// bare form being permissive) and now takes 4- and 8-digit forms, but only
/// with a `#`, so a column of 4-digit years is not promoted to it.
#[test]
fn color_hex_does_not_recover_year_column() {
    // It is only safe off the recovery list because its rule is anchored on
    // `#` for the 4- and 8-digit forms; adding it there needs a detector.
    assert!(
        !CEDED_RECOVERY_LEAVES.contains(&"representation.format.color_hex"),
        "color_hex is on CEDED_RECOVERY_LEAVES: add a detector first"
    );
    let mut cc =
        ColumnClassifier::with_defaults(Box::new(crate::inference::MockClassifier::new("unknown")));
    cc.set_taxonomy(super::shipped_taxonomy());
    let values: Vec<String> = ["2019", "2020", "2021"]
        .iter()
        .cycle()
        .take(42)
        .map(|s| s.to_string())
        .collect();
    for label in [
        "representation.text.word",
        "representation.identifier.alphanumeric_id",
        "datetime.component.year",
    ] {
        let r = cc.compose_from_sense("year", &values, label, 1.0).unwrap();
        assert_ne!(r.label, "representation.format.color_hex", "from {label}");
    }
}

/// A column of one repeated compressed address (the old rule took only the
/// eight-group form) keeps its type.
#[test]
fn ip_v6_constant_column_keeps_type() {
    assert_constant_column_kept("technology.internet.ip_v6", "2001:db8:85a3::8a2e:370:7334");
}

/// `ip_v6` is a ceded-recovery leaf and now takes `::`-compressed values, so
/// columns that are not addresses but share colons (MAC addresses, clock times,
/// C++ and Rust paths with an upper-case or longer-than-four-hex segment) must
/// not be promoted. All-hex `::` names such as `cafe::babe` are valid IPv6 and
/// would conform, so they are not used as neighbours here.
#[test]
fn ip_v6_does_not_recover_neighbouring_colon_columns() {
    let mut cc =
        ColumnClassifier::with_defaults(Box::new(crate::inference::MockClassifier::new("unknown")));
    cc.set_taxonomy(super::shipped_taxonomy());
    let columns: [(&str, [&str; 3]); 4] = [
        (
            "hw",
            [
                "00:1a:2b:3c:4d:5e",
                "12:34:56:78:9a:bc",
                "aa:bb:cc:dd:ee:ff",
            ],
        ),
        ("at", ["10:30", "12:45:10", "23:59:59"]),
        (
            "symbol",
            ["std::vector", "Foo::bar", "crate::module::function"],
        ),
        ("ratio", ["1:2", "3:4:5", "16:9"]),
    ];
    for (header, codes) in columns {
        let values: Vec<String> = codes
            .iter()
            .cycle()
            .take(42)
            .map(|s| s.to_string())
            .collect();
        for label in [
            "representation.text.word",
            "representation.identifier.alphanumeric_id",
        ] {
            let r = cc.compose_from_sense(header, &values, label, 1.0).unwrap();
            assert_ne!(
                r.label, "technology.internet.ip_v6",
                "{header} from {label}"
            );
        }
    }
}

/// A column of one repeated fully qualified name with its root dot (the old
/// rule rejected the dot) keeps its type: the detector counts a stop after a
/// dotted name.
#[test]
fn hostname_constant_column_keeps_type() {
    assert_constant_column_kept("technology.internet.hostname", "example.com.");
}

/// `hostname` now takes a trailing full stop. It is neither a ceded leaf nor an
/// attractor, but the stop is common at the end of a word, an abbreviation or a
/// file name, so a column of those must not be promoted to it by the recoveries
/// that read the validator.
#[test]
fn hostname_does_not_recover_dotted_word_columns() {
    let mut cc =
        ColumnClassifier::with_defaults(Box::new(crate::inference::MockClassifier::new("unknown")));
    cc.set_taxonomy(super::shipped_taxonomy());
    let columns: [(&str, [&str; 4]); 3] = [
        ("remark", ["done.", "pending.", "see note.", "ok."]),
        ("abbr", ["inc.", "corp.", "ltd.", "approx."]),
        (
            "attachment",
            ["report.csv.", "readme.md.", "data.json.", "notes.txt."],
        ),
    ];
    for (header, codes) in columns {
        let values: Vec<String> = codes
            .iter()
            .cycle()
            .take(42)
            .map(|s| s.to_string())
            .collect();
        for label in [
            "representation.text.word",
            "representation.text.plain_text",
            "representation.text.entity_name",
            "representation.identifier.alphanumeric_id",
            "unknown",
        ] {
            let r = cc.compose_from_sense(header, &values, label, 1.0).unwrap();
            assert_ne!(
                r.label, "technology.internet.hostname",
                "{header} from {label}"
            );
        }
    }
}

/// A column of one repeated lone word with a stop, labelled `hostname`, is
/// demoted to `word` by `schema_fail_demotion`, as on 0.6.64 (the detector does
/// not count `done.`).
#[test]
fn hostname_lone_word_with_stop_column_is_demoted() {
    let tax = super::shipped_taxonomy();
    let values: Vec<String> = vec!["done.".to_string(); 40];
    let demoted = schema_fail_demotion(&values, "technology.internet.hostname", &tax);
    assert_eq!(
        demoted.map(|(label, _)| label).as_deref(),
        Some("representation.text.word")
    );
}

/// A column of one repeated empty data URI (the old rule needed at least one
/// character of data) keeps its type.
#[test]
fn data_uri_constant_column_keeps_type() {
    assert_constant_column_kept("technology.internet.data_uri", "data:,");
}

/// `data_uri` is a ceded-recovery leaf and now takes empty data, but still needs
/// the comma, so a column of bare `data:` heads or media types is not promoted.
#[test]
fn data_uri_does_not_recover_headless_columns() {
    let mut cc =
        ColumnClassifier::with_defaults(Box::new(crate::inference::MockClassifier::new("unknown")));
    cc.set_taxonomy(super::shipped_taxonomy());
    let columns: [(&str, [&str; 3]); 3] = [
        ("scheme", ["data:", "data:text/plain", "data:image/png"]),
        (
            "mime",
            ["text/plain;base64", "image/png;base64", "text/html"],
        ),
        ("note", ["data: pending", "data: n/a", "data: see file"]),
    ];
    for (header, codes) in columns {
        let values: Vec<String> = codes
            .iter()
            .cycle()
            .take(42)
            .map(|s| s.to_string())
            .collect();
        for label in [
            "representation.text.word",
            "representation.identifier.alphanumeric_id",
        ] {
            let r = cc.compose_from_sense(header, &values, label, 1.0).unwrap();
            assert_ne!(
                r.label, "technology.internet.data_uri",
                "{header} from {label}"
            );
        }
    }
}

/// A column of one repeated access-point URI (the old rule took only a bucket
/// name) keeps its type.
#[test]
fn s3_uri_constant_column_keeps_type() {
    assert_constant_column_kept(
        "technology.cloud.s3_uri",
        "s3://arn:aws:s3:us-west-2:123456789012:accesspoint/myaccesspoint/mykey",
    );
}

/// `s3_uri` is a ceded-recovery leaf and now takes an access-point ARN after
/// `s3://`, so a column of bare ARNs (aws_arn's own form) or of malformed
/// access points is not promoted to it.
#[test]
fn s3_uri_does_not_recover_arn_columns() {
    let mut cc =
        ColumnClassifier::with_defaults(Box::new(crate::inference::MockClassifier::new("unknown")));
    cc.set_taxonomy(super::shipped_taxonomy());
    let columns: [(&str, [&str; 3]); 3] = [
        (
            "arn",
            [
                "arn:aws:s3:us-west-2:123456789012:accesspoint/myaccesspoint",
                "arn:aws:s3:::my-bucket/key",
                "arn:aws:iam::123456789012:role/admin",
            ],
        ),
        (
            "target",
            [
                "s3://arn:aws:s3:us-west-2:1234:accesspoint/x",
                "s3://arn:aws:ec2:us-west-2:123456789012:accesspoint/abc",
                "s3://A",
            ],
        ),
        (
            "path",
            [
                "s3:/bucket/key",
                "gs://bucket/key",
                "https://x.s3.amazonaws.com/k",
            ],
        ),
    ];
    for (header, codes) in columns {
        let values: Vec<String> = codes
            .iter()
            .cycle()
            .take(42)
            .map(|s| s.to_string())
            .collect();
        for label in [
            "representation.text.word",
            "representation.identifier.alphanumeric_id",
        ] {
            let r = cc.compose_from_sense(header, &values, label, 1.0).unwrap();
            assert_ne!(r.label, "technology.cloud.s3_uri", "{header} from {label}");
        }
    }
}

/// A bare `::` is a valid address that the detector does not count, so a column
/// of it (alone, or 90% of it with times) is not promoted to `ip_v6`, and an
/// all-`::` column labelled `ip_v6` is demoted to `word`.
#[test]
fn ip_v6_does_not_recover_placeholder_column() {
    let mut cc =
        ColumnClassifier::with_defaults(Box::new(crate::inference::MockClassifier::new("unknown")));
    cc.set_taxonomy(super::shipped_taxonomy());
    let all: Vec<String> = vec!["::".to_string(); 40];
    let mixed: Vec<String> = (0..40)
        .map(|i| if i < 36 { "::" } else { "12:30:00" }.to_string())
        .collect();
    for (header, values) in [("addr", &all), ("seen", &mixed)] {
        for label in [
            "representation.text.word",
            "representation.identifier.alphanumeric_id",
        ] {
            let r = cc.compose_from_sense(header, values, label, 1.0).unwrap();
            assert_ne!(
                r.label, "technology.internet.ip_v6",
                "{header} from {label}"
            );
        }
    }
    let tax = super::shipped_taxonomy();
    let demoted = schema_fail_demotion(&all, "technology.internet.ip_v6", &tax);
    assert_eq!(
        demoted.map(|(label, _)| label).as_deref(),
        Some("representation.text.word")
    );
}

/// A column of one repeated hyphenated pre-release version (which the old rule
/// rejected) keeps its type.
#[test]
fn version_constant_column_keeps_type() {
    assert_constant_column_kept("technology.development.version", "1.2.3-alpha-1");
}

/// A column of one repeated single-digit-month, third-component calver (which the
/// old rule rejected) keeps its type. The detector counts it: it has a third
/// component. A column of `2024.1`, which the detector does not count, is
/// demoted instead (see `calver_decimal_column_is_demoted`).
#[test]
fn calver_constant_column_keeps_type() {
    assert_constant_column_kept("technology.development.calver", "2024.2.9");
}

/// A column of one repeated `2024.1` conforms to calver but is a decimal; the
/// detector does not count it, so schema_fail_demotion demotes it, as on 0.6.64
/// (the old rule rejected it outright).
#[test]
fn calver_decimal_column_is_demoted() {
    let tax = super::shipped_taxonomy();
    let values: Vec<String> = vec!["2024.1".to_string(); 40];
    assert!(
        schema_fail_demotion(&values, "technology.development.calver", &tax).is_some(),
        "a column of 2024.1 is demoted"
    );
}

/// calver is not a ceded leaf, so nothing promotes a column to it; the real guard
/// for decimals is `calver_decimal_column_is_demoted` (the detector does not count
/// them). This pins that no promotion path exists.
///
/// A column of decimals that look like a calendar version (`2024.1`, `2020.5`)
/// is not promoted to `calver`, from the decimal, the integer or the unknown
/// label, with a version-like or a date-like header.
#[test]
fn calver_does_not_recover_decimal_columns() {
    let mut cc =
        ColumnClassifier::with_defaults(Box::new(crate::inference::MockClassifier::new("unknown")));
    cc.set_taxonomy(super::shipped_taxonomy());
    let decimals: Vec<String> = ["2024.1", "2020.5", "1999.9", "2024.7", "2021.3", "2022.8"]
        .iter()
        .cycle()
        .take(60)
        .map(|s| s.to_string())
        .collect();
    for header in ["value", "version", "release", "year"] {
        for from in [
            "representation.numeric.decimal_number",
            "representation.numeric.integer_number",
            "unknown",
        ] {
            let r = cc.compose_from_sense(header, &decimals, from, 1.0).unwrap();
            assert_ne!(
                r.label, "technology.development.calver",
                "{header} / {from}: {:?}",
                r.disambiguation_rule
            );
        }
    }
}

/// A column of one repeated sha512-pinned image (which the old rule rejected)
/// keeps its type. The 128 hex digits are built in code.
#[test]
fn docker_ref_constant_column_keeps_type() {
    let value = format!("app@sha512:{}", "a".repeat(128));
    assert_constant_column_kept("technology.development.docker_ref", &value);
}

/// RFC 7519 section 6.1: the example Unsecured JWT (empty signature).
const JWT_UNSECURED: &str = "eyJhbGciOiJub25lIn0.eyJpc3MiOiJqb2UiLA0KICJleHAiOjEzMDA4MTkzODAsDQogImh0dHA6Ly9leGFtcGxlLmNvbS9pc19yb290Ijp0cnVlfQ.";

/// RFC 7516 appendix A.1.7: the complete JWE Compact Serialization example.
const JWT_JWE: &str = "eyJhbGciOiJSU0EtT0FFUCIsImVuYyI6IkEyNTZHQ00ifQ.OKOawDo13gRp2ojaHV7LFpZcgV7T6DVZKTyKOMTYUmKoTCVJRgckCL9kiMT03JGeipsEdY3mx_etLbbWSrFr05kLzcSr4qKAq7YN7e9jwQRb23nfa6c9d-StnImGyFDbSv04uVuxIp5Zms1gNxKKK2Da14B8S4rzVRltdYwam_lDp5XnZAYpQdb76FdIKLaVmqgfwX7XWRxv2322i-vDxRfqNzo_tETKzpVLzfiwQyeyPGLBIO56YJ7eObdv0je81860ppamavo35UgoRdbYaBcoh9QcfylQr66oc6vFWXRcZ_ZT2LawVCWTIy3brGPi6UklfCpIMfIjf7iGdXKHzg.48V1_ALb6US04U3b.5eym8TW_c8SuK0ltJ3rpYIzOeDQz7TALvtu6UG9oMo4vpzs9tX_EFShS8iB7j6jiSdiwkIr3ajwQzaBtQD_A.XFBoMYUZodetZdvTiFvSkQ";

/// A column of one repeated unsecured JWT (which the old rule rejected) keeps its
/// type: its header decodes to JSON with `alg`, so the detector counts it.
#[test]
fn jwt_constant_column_keeps_type() {
    assert_constant_column_kept("technology.cryptographic.jwt", JWT_UNSECURED);
}

/// A column of RFC 7516 JWEs labelled `jwt` keeps its type through the veto and
/// the substance guard.
#[test]
fn jwt_jwe_column_keeps_type() {
    assert_constant_column_kept("technology.cryptographic.jwt", JWT_JWE);
    let mut cc =
        ColumnClassifier::with_defaults(Box::new(crate::inference::MockClassifier::new("unknown")));
    cc.set_taxonomy(super::shipped_taxonomy());
    let tokens: Vec<String> = vec![JWT_JWE.to_string(), JWT_UNSECURED.to_string()]
        .into_iter()
        .cycle()
        .take(42)
        .collect();
    let r = cc
        .compose_from_sense("token", &tokens, "technology.cryptographic.jwt", 0.9)
        .unwrap();
    assert_eq!(r.label, "technology.cryptographic.jwt");
}

/// `jwt` is a ceded leaf and now takes a trailing dot and five parts, so columns of
/// dotted names must not be promoted to it.
#[test]
fn jwt_does_not_recover_dotted_name_columns() {
    let mut cc =
        ColumnClassifier::with_defaults(Box::new(crate::inference::MockClassifier::new("unknown")));
    cc.set_taxonomy(super::shipped_taxonomy());
    let columns: [Vec<String>; 3] = [
        [
            "internationalization.configuration.",
            "authentication.middleware.",
            "documentation.reference.",
        ]
        .iter()
        .cycle()
        .take(42)
        .map(|s| s.to_string())
        .collect(),
        [
            "springframework.web.configuration.mvcdispatcher.DispatcherServlet",
            "jakartaservlet.http.configuration.HttpSessionListener.Handlerclass",
        ]
        .iter()
        .cycle()
        .take(42)
        .map(|s| s.to_string())
        .collect(),
        [
            "Introduction.Background.",
            "Conclusionsection.Appendixmaterial.",
            "Methodology.Participants.",
        ]
        .iter()
        .cycle()
        .take(42)
        .map(|s| s.to_string())
        .collect(),
    ];
    for values in &columns {
        for label in [
            "representation.text.word",
            "representation.text.plain_text",
            "unknown",
        ] {
            let r = cc.compose_from_sense("name", values, label, 1.0).unwrap();
            assert_ne!(
                r.label, "technology.cryptographic.jwt",
                "from {label}: {values:?}"
            );
        }
    }
}

/// A column of one repeated whole-second PostgreSQL timestamp (`+03`), which the
/// old rule rejected, is not vetoed or demoted.
#[test]
fn pg_short_offset_constant_column_keeps_type() {
    assert_constant_column_kept(
        "datetime.timestamp.pg_short_offset",
        "2022-01-06 18:00:00+03",
    );
}

/// A column of one repeated ISO 8601 duration with a decimal comma (which the old
/// rule rejected) keeps its type.
#[test]
fn iso_duration_constant_column_keeps_type() {
    assert_constant_column_kept("datetime.duration.iso_8601", "P0,5Y");
}

/// So does one repeated alternative-format duration.
#[test]
fn iso_duration_alternative_format_constant_column_keeps_type() {
    assert_constant_column_kept("datetime.duration.iso_8601", "P0003-06-04T12:30:05");
}

/// A column of one repeated upper-case month-year (which the old rule rejected)
/// keeps its type.
#[test]
fn month_year_abbrev_constant_column_keeps_type() {
    assert_constant_column_kept("datetime.date.month_year_abbrev", "JAN 2024");
}

/// The BLS's `Sept` is the same.
#[test]
fn month_year_abbrev_sept_constant_column_keeps_type() {
    assert_constant_column_kept("datetime.date.month_year_abbrev", "Sept 2026");
}

/// Full month names are `month_year_full`, which is what a column of them reads
/// as today (profile on 60 rows of `January 2024` gives `month_year_full`). The
/// abbreviated rule must not take them, whichever leaf the model proposes.
#[test]
fn full_month_year_column_keeps_month_year_full() {
    let mut cc =
        ColumnClassifier::with_defaults(Box::new(crate::inference::MockClassifier::new("unknown")));
    cc.set_taxonomy(super::shipped_taxonomy());
    let months = [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ];
    let values: Vec<String> = months
        .iter()
        .cycle()
        .take(48)
        .enumerate()
        .map(|(i, m)| format!("{m} {}", 2015 + i % 10))
        .collect();
    let r = cc
        .compose_from_sense("period", &values, "datetime.date.month_year_full", 0.9)
        .unwrap();
    assert_eq!(r.label, "datetime.date.month_year_full");
    let tax = super::shipped_taxonomy();
    assert!(
        !label_validates_sample(&tax, "datetime.date.month_year_abbrev", &values),
        "the full-name column fails month_year_abbrev as a whole (only May, June and July values conform on their own)"
    );
}

/// A column of one repeated week-number written without its leading zero, the
/// Noted form (the old rule accepted every week, so no ACCEPT value was newly
/// accepted), keeps its type.
#[test]
fn iso_week_constant_column_keeps_type() {
    assert_constant_column_kept("datetime.date.iso_week", "2024-W3");
}

/// A column of one repeated zero-padded date (which the old rule rejected) keeps
/// its type.
#[test]
fn chinese_ymd_constant_column_keeps_type() {
    assert_constant_column_kept(
        "datetime.date.chinese_ymd",
        "2024\u{5E74}01\u{6708}11\u{65E5}",
    );
}

/// A column of one repeated hyphenated zone (which the old rule rejected) keeps
/// its type.
#[test]
fn iana_constant_column_keeps_type() {
    assert_constant_column_kept("datetime.offset.iana", "America/Port-au-Prince");
}

/// So does one repeated fixed-offset zone.
#[test]
fn iana_etc_gmt_constant_column_keeps_type() {
    assert_constant_column_kept("datetime.offset.iana", "Etc/GMT+4");
}

/// Hyphens and dots in a location widen the shape, so columns of other
/// `Word/Word` paths and `A-B/C_D` code names must not be recovered as `iana`,
/// from any label the model might give them.
#[test]
fn iana_does_not_recover_path_or_code_columns() {
    let mut cc =
        ColumnClassifier::with_defaults(Box::new(crate::inference::MockClassifier::new("unknown")));
    cc.set_taxonomy(super::shipped_taxonomy());
    let columns: [Vec<String>; 5] = [
        // Under real tz areas, where the widened class (not the area list) is
        // what decides: file names and versioned names are not zones.
        [
            "Europe/Sales-Report.pdf",
            "Asia/Tokyo.png",
            "America/data.v2",
            "Europe/Q3-Summary.xlsx",
            "Asia/Osaka.jpg",
        ]
        .iter()
        .cycle()
        .take(70)
        .map(|s| s.to_string())
        .collect(),
        [
            "Sales/North",
            "Sales/South-East",
            "Marketing/Emea",
            "Ops/Core.Net",
        ]
        .iter()
        .cycle()
        .take(40)
        .map(|s| s.to_string())
        .collect(),
        ["AB-12/C_D", "XY-9/P_Q", "Part-A/Code_B", "Lot-7/Bin_3"]
            .iter()
            .cycle()
            .take(40)
            .map(|s| s.to_string())
            .collect(),
        [
            "Users/hugh-cameron",
            "Users/jane.doe",
            "Home/john-smith",
            "Home/ann_lee",
        ]
        .iter()
        .cycle()
        .take(40)
        .map(|s| s.to_string())
        .collect(),
        ["GMT+4", "GMT-5", "UTC+1", "GMT+10"]
            .iter()
            .cycle()
            .take(40)
            .map(|s| s.to_string())
            .collect(),
    ];
    for values in &columns {
        for label in [
            "representation.text.word",
            "representation.text.plain_text",
            "representation.text.entity_name",
            "geography.location.continent",
            "geography.location.city",
            "unknown",
        ] {
            let r = cc.compose_from_sense("name", values, label, 1.0).unwrap();
            assert_ne!(r.label, "datetime.offset.iana", "from {label}: {values:?}");
        }
    }
}

/// A column of real zone names the model read as a place is still recovered.
#[test]
fn iana_recovers_hyphenated_and_etc_zones() {
    let mut cc =
        ColumnClassifier::with_defaults(Box::new(crate::inference::MockClassifier::new("unknown")));
    cc.set_taxonomy(super::shipped_taxonomy());
    let values: Vec<String> = [
        "America/Port-au-Prince",
        "Etc/GMT+4",
        "Etc/GMT-14",
        "America/Argentina/Buenos_Aires",
    ]
    .iter()
    .cycle()
    .take(40)
    .map(|s| s.to_string())
    .collect();
    let r = cc
        .compose_from_sense("name", &values, "geography.location.continent", 0.9)
        .unwrap();
    assert_eq!(r.label, "datetime.offset.iana");
}

/// A column of one repeated Czech amount with the symbol after the number
/// (which the old rule rejected) keeps its type.
#[test]
fn amount_multisym_constant_column_keeps_type() {
    assert_constant_column_kept("finance.currency.amount_multisym", "1 234,56 K\u{10d}");
}

/// A column of one repeated rupee amount with a space after the sign (which the
/// old rule rejected) keeps its type.
#[test]
fn amount_lakh_constant_column_keeps_type() {
    assert_constant_column_kept("finance.currency.amount_lakh", "\u{20b9} 5,00,000");
}

/// A column of one repeated alphabetic ticker keeps its type. The values the old
/// rule rejected (`5`, `0005`) conform but the detector does not count them, so a
/// column of them is demoted by design (see
/// `ticker_numeric_constant_column_is_demoted`); a form the detector keeps is
/// what is pinned here.
#[test]
fn ticker_constant_column_keeps_type() {
    assert_constant_column_kept("finance.securities.ticker", "BRK-B");
}

/// A column of one repeated `5` conforms to ticker but is a number; the detector
/// does not count it, so schema_fail_demotion demotes it, as on 0.6.64 (the old
/// rule rejected it outright).
#[test]
fn ticker_numeric_constant_column_is_demoted() {
    let tax = super::shipped_taxonomy();
    for value in ["5", "0005"] {
        let values: Vec<String> = vec![value.to_string(); 40];
        assert!(
            schema_fail_demotion(&values, "finance.securities.ticker", &tax).is_some(),
            "a column of {value} is demoted"
        );
    }
}

/// A column of small integers is not kept as, or promoted to, `ticker`, from a
/// ticker label or a numeric one, under a ticker-like header or a neutral one.
#[test]
fn ticker_does_not_keep_or_recover_small_integer_columns() {
    let mut cc =
        ColumnClassifier::with_defaults(Box::new(crate::inference::MockClassifier::new("unknown")));
    cc.set_taxonomy(super::shipped_taxonomy());
    let ints: Vec<String> = ["5", "12", "7", "300", "0005", "42", "9", "1500"]
        .iter()
        .cycle()
        .take(60)
        .map(|s| s.to_string())
        .collect();
    for header in ["value", "ticker", "symbol", "code", "count"] {
        for from in [
            "finance.securities.ticker",
            "representation.numeric.integer_number",
            "representation.identifier.numeric_code",
            "unknown",
        ] {
            let r = cc.compose_from_sense(header, &ints, from, 1.0).unwrap();
            assert_ne!(
                r.label, "finance.securities.ticker",
                "{header} / {from}: {:?}",
                r.disambiguation_rule
            );
        }
    }
}

/// A column of one repeated private placement number with its check digit
/// (which the old rule rejected) keeps its type. The check digit is computed by
/// the library's checker.
#[test]
fn cusip_constant_column_keeps_type() {
    let value = (0..=9)
        .map(|d| format!("12345*10{d}"))
        .find(|c| finetype_core::checksum::cusip(c))
        .unwrap();
    assert_constant_column_kept("finance.securities.cusip", &value);
}

/// The same placement number with a wrong check digit conforms to the rule but
/// the detector does not count it, so a column of it is demoted.
#[test]
fn cusip_placement_number_with_wrong_check_digit_column_is_demoted() {
    let tax = super::shipped_taxonomy();
    let value = (0..=9)
        .map(|d| format!("12345*10{d}"))
        .find(|c| !finetype_core::checksum::cusip(c))
        .unwrap();
    let values: Vec<String> = vec![value; 40];
    assert!(schema_fail_demotion(&values, "finance.securities.cusip", &tax).is_some());
}

/// `cusip` is an attractor leaf and now takes `*`, `@` and `#`, so a column of
/// codes that use them without a valid check digit is not kept as cusip.
#[test]
fn cusip_does_not_keep_reserved_character_codes_without_a_check_digit() {
    let mut cc =
        ColumnClassifier::with_defaults(Box::new(crate::inference::MockClassifier::new("unknown")));
    cc.set_taxonomy(super::shipped_taxonomy());
    let values: Vec<String> = ["ABC*1234", "XYZ@5678", "QRS#9012", "LMN*3456", "TUV@7890"]
        .iter()
        .cycle()
        .take(40)
        .map(|b| {
            (0..=9)
                .map(|d| format!("{b}{d}"))
                .find(|c| !finetype_core::checksum::cusip(c))
                .unwrap()
        })
        .collect();
    let r = cc
        .compose_from_sense("code", &values, "finance.securities.cusip", 0.9)
        .unwrap();
    assert_ne!(
        r.label, "finance.securities.cusip",
        "{:?}",
        r.disambiguation_rule
    );
}

/// A column of one repeated `M` keeps its type. `<`, which the old rule rejected,
/// conforms to the rule but the detector does not count it, so a column of it is
/// demoted by design (see `gender_code_filler_constant_column_is_demoted`).
#[test]
fn gender_code_constant_column_keeps_type() {
    assert_constant_column_kept("identity.person.gender_code", "M");
}

/// A column of one repeated `<` conforms to gender_code but is a lone punctuation
/// mark; the detector does not count it, so schema_fail_demotion demotes it, as on
/// 0.6.64 (the old rule rejected it outright).
#[test]
fn gender_code_filler_constant_column_is_demoted() {
    let tax = super::shipped_taxonomy();
    let values: Vec<String> = vec!["<".to_string(); 40];
    assert!(schema_fail_demotion(&values, "identity.person.gender_code", &tax).is_some());
}

/// A machine readable zone column (mostly M/F with a few `<`) keeps its type: the
/// detector still counts the codes, and demotion needs a majority to fail.
#[test]
fn gender_code_mrz_column_with_fillers_is_not_demoted() {
    let tax = super::shipped_taxonomy();
    let values: Vec<String> = (0..40)
        .map(|i| {
            if i % 8 == 0 {
                "<"
            } else if i % 2 == 0 {
                "M"
            } else {
                "F"
            }
        })
        .map(String::from)
        .collect();
    assert!(schema_fail_demotion(&values, "identity.person.gender_code", &tax).is_none());
}

/// `gender_code` now takes the ICAO filler `<`. It is not a ceded leaf and not on
/// the fast path, and `disambiguate_gender` reads only M/F/X, so nothing promotes a
/// column of fillers (or a comparison-operator column) to it.
#[test]
fn gender_code_is_not_promoted_for_filler_columns() {
    for values in [
        vec!["<"; 40],
        vec!["<", ">", "<", ">", "<", ">"],
        vec!["<", "M", "F", "<", "M", "<"],
    ] {
        let values: Vec<String> = values.iter().map(|s| s.to_string()).collect();
        assert_ne!(
            disambiguate_gender(&values),
            Some("identity.person.gender_code".to_string()),
            "{values:?}"
        );
    }
    let tax = super::shipped_taxonomy();
    assert_eq!(
        finetype_core::fast_path::deterministic_fast_path(&tax, &vec!["<".to_string(); 40]),
        None
    );
    let mut cc =
        ColumnClassifier::with_defaults(Box::new(crate::inference::MockClassifier::new("unknown")));
    cc.set_taxonomy(tax);
    for label in [
        "representation.text.word",
        "representation.text.entity_name",
    ] {
        let r = cc
            .compose_from_sense("gender", &vec!["<".to_string(); 40], label, 0.9)
            .unwrap();
        assert_ne!(
            r.label, "identity.person.gender_code",
            "{label}: {:?}",
            r.disambiguation_rule
        );
    }
}

/// A column of one repeated NDC in a form the detector keeps (the old shape)
/// keeps its type. The six-digit-labeler forms the old rule rejected conform to
/// the rule but the detector does not count them, so a column of them is demoted
/// by design, as on 0.6.64; see `ndc_six_digit_labeler_column_is_demoted`.
#[test]
fn ndc_constant_column_keeps_type() {
    assert_constant_column_kept("identity.medical.ndc", "0002-1433-80");
}

#[test]
fn ndc_six_digit_labeler_column_is_demoted() {
    let tax = super::shipped_taxonomy();
    for value in ["123456-7890-1", "123456-789-01", "123456-7890-12"] {
        let values: Vec<String> = vec![value.to_string(); 40];
        assert!(
            schema_fail_demotion(&values, "identity.medical.ndc", &tax).is_some(),
            "a column of {value} is demoted"
        );
    }
}

/// `ndc` is an attractor leaf. A column of twelve-digit UPCs is still demoted from
/// it by attractor demotion, and a hyphenated six-digit-labeler column is not kept.
#[test]
fn ndc_attractor_demotion_still_demotes_upc_columns() {
    let mut cc =
        ColumnClassifier::with_defaults(Box::new(crate::inference::MockClassifier::new("unknown")));
    cc.set_taxonomy(super::shipped_taxonomy());
    let upcs: Vec<String> = (0..40)
        .map(|i| format!("{:012}", 36_000_291_452u64 + i * 7_919))
        .collect();
    let hyphenated: Vec<String> = (0..40)
        .map(|i| format!("{:06}-{:04}-{}", 100_000 + i * 31, 1_000 + i, i % 10))
        .collect();
    for (name, values) in [("upc", upcs), ("six-digit labeler", hyphenated)] {
        let r = cc
            .compose_from_sense("code", &values, "identity.medical.ndc", 0.9)
            .unwrap();
        assert_ne!(
            r.label, "identity.medical.ndc",
            "{name}: {:?}",
            r.disambiguation_rule
        );
    }
}

/// A column of one repeated Canadian wholesale code (sector 41, which the old rule
/// rejected) keeps its type.
#[test]
fn naics_constant_column_keeps_type() {
    assert_constant_column_kept("identity.industry.naics", "411110");
}

/// A column of one repeated hyphenated EIN keeps its type. The bare nine digits,
/// which the old rule rejected, conform to the rule but the detector does not count
/// them, so a column of them is demoted (`ein_bare_digit_column_is_demoted`).
#[test]
fn ein_constant_column_keeps_type() {
    assert_constant_column_kept("identity.government.ein", "12-3456789");
}

/// A column of bare nine-digit numbers labelled `ein` is demoted, as on 0.6.64.
#[test]
fn ein_bare_digit_column_is_demoted() {
    let tax = super::shipped_taxonomy();
    let constant: Vec<String> = vec!["010548657".to_string(); 40];
    let varied: Vec<String> = (0..40)
        .map(|i| format!("{:09}", 10_548_657 + i * 997))
        .collect();
    for values in [constant, varied] {
        assert!(
            schema_fail_demotion(&values, "identity.government.ein", &tax).is_some(),
            "{:?}",
            &values[..2]
        );
    }
}

/// A column of one repeated space-separated ISBN (which the old rule rejected)
/// keeps its type.
#[test]
fn isbn_constant_column_keeps_type() {
    assert_constant_column_kept("identity.commerce.isbn", "978 0 13 110362 7");
}

/// A column of space-grouped numbers that fail the check digit is not kept as
/// isbn by the checksum guard, so the widened rule does not open isbn to them.
#[test]
fn isbn_space_separated_with_wrong_check_digit_is_not_kept() {
    let mut cc =
        ColumnClassifier::with_defaults(Box::new(crate::inference::MockClassifier::new("unknown")));
    cc.set_taxonomy(super::shipped_taxonomy());
    let values: Vec<String> = (0..40)
        .map(|i| format!("978 0 13 {:06} {}", 110_000 + i * 13, 4))
        .filter(|v| !finetype_core::checksum::isbn(v))
        .collect();
    assert!(values.len() >= 30);
    let r = cc
        .compose_from_sense("isbn", &values, "identity.commerce.isbn", 0.9)
        .unwrap();
    assert_ne!(
        r.label, "identity.commerce.isbn",
        "{:?}",
        r.disambiguation_rule
    );
}

/// A genuine space-separated ISBN column is recovered by the header guard, which
/// reads the check digit through `checksum::isbn`.
#[test]
fn isbn_header_recovery_reads_space_separated_codes() {
    let mut cc =
        ColumnClassifier::with_defaults(Box::new(crate::inference::MockClassifier::new("unknown")));
    cc.set_taxonomy(super::shipped_taxonomy());
    let values: Vec<String> = ["978 0 13 110362 7", "0 13 110362 8", "978 3 16 148410 0"]
        .iter()
        .cycle()
        .take(30)
        .map(|s| s.to_string())
        .collect();
    let r = cc
        .compose_from_sense(
            "isbn",
            &values,
            "representation.identifier.alphanumeric_id",
            0.9,
        )
        .unwrap();
    assert_eq!(
        r.label, "identity.commerce.isbn",
        "{:?}",
        r.disambiguation_rule
    );
}
