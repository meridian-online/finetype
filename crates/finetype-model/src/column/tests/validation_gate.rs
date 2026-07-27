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
        v(&["zzz", "AAA", "AAA", "AAA", "AAA", "AAA", "AAA", "AAA", "AAA", "AAA"]),
        v(&["AAA", "AAA", "AAA", "AAA", "AAA", "AAA", "AAA", "AAA", "AAA", "zzz"]),
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
