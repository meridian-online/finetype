use super::super::*;

#[test]
fn test_boolean_override_integer_spread() {
    // SibSp-like column: integers 0-8 with >2 unique values
    let values: Vec<String> = vec!["0", "1", "2", "3", "0", "1", "4", "0", "5", "8"]
        .into_iter()
        .map(String::from)
        .collect();
    let top_labels = vec![
        "representation.logical.boolean",
        "representation.numeric.integer_number",
    ];

    let result = disambiguate_boolean_override(&values, &top_labels);
    assert!(result.is_some());
    let (label, rule) = result.unwrap();
    assert_eq!(label, "representation.numeric.integer_number");
    assert_eq!(rule, "boolean_override_integer_spread");
}

#[test]
fn test_boolean_override_preserves_real_boolean() {
    // Actual boolean column: only 0 and 1
    let values: Vec<String> = vec!["0", "1", "0", "1", "1", "0", "0", "1", "0", "1"]
        .into_iter()
        .map(String::from)
        .collect();
    let top_labels = vec!["representation.logical.boolean"];

    let result = disambiguate_boolean_override(&values, &top_labels);
    // Should return None — this IS a boolean column (only 2 unique, spread=1)
    assert!(result.is_none());
}

#[test]
fn test_boolean_override_single_char_categorical() {
    // Embarked-like column: single chars S, C, Q
    let values: Vec<String> = vec!["S", "C", "Q", "S", "S", "C", "Q", "S", "S", "C"]
        .into_iter()
        .map(String::from)
        .collect();
    let top_labels = vec!["representation.logical.boolean", "representation.text.word"];

    let result = disambiguate_boolean_override(&values, &top_labels);
    assert!(result.is_some());
    let (label, rule) = result.unwrap();
    assert_eq!(label, "representation.text.word");
    assert_eq!(rule, "boolean_override_single_char_categorical");
}

#[test]
fn test_boolean_override_preserves_true_false_chars() {
    // T/F single-char boolean values should stay boolean
    let values: Vec<String> = vec!["T", "F", "T", "F", "T", "F", "T", "F"]
        .into_iter()
        .map(String::from)
        .collect();
    let top_labels = vec!["representation.logical.boolean"];

    let result = disambiguate_boolean_override(&values, &top_labels);
    // Should return None — T/F is a valid boolean encoding
    assert!(result.is_none());
}

#[test]
fn binary_vocab_veto_is_default_on() {
    assert!(!rhh::is_disabled("binary_vocab_veto"));
}

// ── Small-integer disambiguation tests ────────────────────

#[test]
fn test_boolean_override_with_current_model_label() {
    // The actual model outputs "technology.development.boolean", not the
    // previously-checked labels. Verify the override fires for this label.
    let values: Vec<String> = vec!["0", "1", "2", "3", "0", "1", "4", "0", "5", "8"]
        .into_iter()
        .map(String::from)
        .collect();
    let top_labels = vec![
        "technology.development.boolean",
        "representation.numeric.integer_number",
    ];

    let result = disambiguate_boolean_override(&values, &top_labels);
    assert!(
        result.is_some(),
        "Boolean override must trigger for technology.development.boolean"
    );
    let (label, rule) = result.unwrap();
    assert_eq!(label, "representation.numeric.integer_number");
    assert_eq!(rule, "boolean_override_integer_spread");
}

#[test]
fn test_boolean_override_preserves_real_boolean_current_label() {
    // Actual {0,1} boolean column with current model label should NOT override
    let values: Vec<String> = vec!["0", "1", "0", "1", "1", "0", "0", "1", "0", "1"]
        .into_iter()
        .map(String::from)
        .collect();
    let top_labels = vec!["technology.development.boolean"];

    let result = disambiguate_boolean_override(&values, &top_labels);
    assert!(
        result.is_none(),
        "Real boolean {{0,1}} must not be overridden"
    );
}

#[test]
fn test_boolean_subtype_terms() {
    let values: Vec<String> = vec![
        "True", "False", "True", "True", "False", "True", "False", "False", "True", "False",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let top_labels = vec!["representation.boolean.terms"];

    let result = disambiguate_boolean_subtype(&values, &top_labels);
    assert!(result.is_some());
    let (label, rule) = result.unwrap();
    assert_eq!(label, "representation.boolean.terms");
    assert_eq!(rule, "boolean_subtype_terms");
}

#[test]
fn test_boolean_subtype_binary() {
    let values: Vec<String> = vec!["0", "1", "0", "1", "1", "0", "0", "1", "0", "1"]
        .into_iter()
        .map(String::from)
        .collect();
    let top_labels = vec!["representation.boolean.binary"];

    let result = disambiguate_boolean_subtype(&values, &top_labels);
    assert!(result.is_some());
    let (label, rule) = result.unwrap();
    assert_eq!(label, "representation.boolean.binary");
    assert_eq!(rule, "boolean_subtype_binary");
}

#[test]
fn test_boolean_subtype_initials() {
    let values: Vec<String> = vec!["T", "F", "T", "F", "T", "T", "F", "F", "T", "F"]
        .into_iter()
        .map(String::from)
        .collect();
    let top_labels = vec!["representation.boolean.initials"];

    let result = disambiguate_boolean_subtype(&values, &top_labels);
    assert!(result.is_some());
    let (label, rule) = result.unwrap();
    assert_eq!(label, "representation.boolean.initials");
    assert_eq!(rule, "boolean_subtype_initials");
}

#[test]
fn test_boolean_subtype_yes_no() {
    let values: Vec<String> = vec!["yes", "no", "yes", "yes", "no", "no", "yes", "no"]
        .into_iter()
        .map(String::from)
        .collect();
    let top_labels = vec!["representation.boolean.terms"];

    let result = disambiguate_boolean_subtype(&values, &top_labels);
    assert!(result.is_some());
    let (label, _) = result.unwrap();
    assert_eq!(label, "representation.boolean.terms");
}

#[test]
fn test_boolean_subtype_override_categorical() {
    // True/False column misclassified as categorical — boolean detection fires
    // because ≥80% of values are boolean-like terms
    let values: Vec<String> = vec![
        "True", "False", "True", "True", "False", "True", "False", "False",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let top_labels = vec!["representation.discrete.categorical"];

    let result = disambiguate_boolean_subtype(&values, &top_labels);
    assert!(
        result.is_some(),
        "Should override categorical for True/False column"
    );
    let (label, _) = result.unwrap();
    assert_eq!(label, "representation.boolean.terms");
}

#[test]
fn test_boolean_subtype_not_triggered_for_mixed() {
    // Mixed values that aren't clearly boolean
    let values: Vec<String> = vec!["yes", "no", "maybe", "yes", "unknown", "no"]
        .into_iter()
        .map(String::from)
        .collect();
    let top_labels = vec!["representation.text.word"];

    let result = disambiguate_boolean_subtype(&values, &top_labels);
    assert!(result.is_none());
}

#[test]
fn test_boolean_subtype_too_few_values() {
    let values: Vec<String> = vec!["True", "False"]
        .into_iter()
        .map(String::from)
        .collect();
    let top_labels = vec!["representation.boolean.terms"];

    let result = disambiguate_boolean_subtype(&values, &top_labels);
    assert!(result.is_none());
}

#[test]
fn test_boolean_subtype_skewed_integers_not_binary() {
    // SibSp-like column: mostly 0s and 1s but with values up to 8
    // Should NOT be classified as binary despite >80% being 0/1
    let values: Vec<String> = vec![
        "0", "1", "0", "1", "0", "0", "1", "0", "0", "1", "2", "0", "3", "0", "1", "0", "0", "1",
        "4", "0",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let top_labels = vec![
        "representation.numeric.integer_number",
        "representation.boolean.binary",
    ];

    let result = disambiguate_boolean_subtype(&values, &top_labels);
    // >2 unique values (0,1,2,3,4) → should NOT fire for binary
    assert!(
        result.is_none(),
        "Skewed integer column (SibSp-like) should not be classified as boolean"
    );
}

#[test]
fn test_boolean_subtype_pure_binary_still_works() {
    // Pure 0/1 column with exactly 2 unique values → should still be binary
    let values: Vec<String> = vec!["0", "1", "1", "0", "0", "1", "0", "1", "1", "0"]
        .into_iter()
        .map(String::from)
        .collect();
    let top_labels = vec!["representation.boolean.binary"];

    let result = disambiguate_boolean_subtype(&values, &top_labels);
    assert!(result.is_some());
    let (label, _) = result.unwrap();
    assert_eq!(label, "representation.boolean.binary");
}

// ═══════════════════════════════════════════════════════════════════════
// Distillation v2 fix tests
// ═══════════════════════════════════════════════════════════════════════

// --- Fix 1: Boolean binary heuristic ---

#[test]
fn test_boolean_override_all_zeros() {
    // All-zero column: not boolean, it's a count/sentinel column
    let values: Vec<String> = vec!["0", "0", "0", "0", "0", "0", "0", "0"]
        .into_iter()
        .map(String::from)
        .collect();
    let top_labels = vec!["representation.boolean.binary"];

    let result = disambiguate_boolean_override(&values, &top_labels);
    assert!(result.is_some(), "All-zero column should be overridden");
    let (label, rule) = result.unwrap();
    assert_eq!(label, "representation.numeric.integer_number");
    assert_eq!(rule, "boolean_override_single_value");
}

#[test]
fn test_boolean_override_all_ones() {
    // All-ones column: also a single-value column, not boolean
    let values: Vec<String> = vec!["1", "1", "1", "1", "1"]
        .into_iter()
        .map(String::from)
        .collect();
    let top_labels = vec!["representation.boolean.binary"];

    let result = disambiguate_boolean_override(&values, &top_labels);
    assert!(result.is_some(), "All-ones column should be overridden");
    let (label, rule) = result.unwrap();
    assert_eq!(label, "representation.numeric.integer_number");
    assert_eq!(rule, "boolean_override_single_value");
}

#[test]
fn test_boolean_override_skewed_95pct() {
    // 19 zeros and 1 one = 95% dominant → borderline, should still be binary
    let mut values: Vec<String> = vec!["0"; 19].into_iter().map(String::from).collect();
    values.push("1".to_string());
    let top_labels = vec!["representation.boolean.binary"];

    let result = disambiguate_boolean_override(&values, &top_labels);
    // At exactly 95% (19/20), dominant_frac = 0.95 which is NOT > 0.95
    assert!(
        result.is_none(),
        "95% skew should still allow binary (boundary)"
    );
}

#[test]
fn test_boolean_override_skewed_above_95pct() {
    // 96 zeros and 4 ones → 96% dominant → too skewed for binary
    let mut values: Vec<String> = vec!["0"; 48].into_iter().map(String::from).collect();
    for _ in 0..2 {
        values.push("1".to_string());
    }
    // 48 zeros, 2 ones = 96% dominant
    let top_labels = vec!["representation.boolean.binary"];

    let result = disambiguate_boolean_override(&values, &top_labels);
    assert!(
        result.is_some(),
        "96% skew should override binary to integer"
    );
    let (label, rule) = result.unwrap();
    assert_eq!(label, "representation.numeric.integer_number");
    assert_eq!(rule, "boolean_override_skewed");
}

#[test]
fn test_boolean_override_balanced_binary_preserved() {
    // Balanced 0/1 column should remain binary
    let values: Vec<String> = vec!["0", "1", "0", "1", "0", "1", "0", "1"]
        .into_iter()
        .map(String::from)
        .collect();
    let top_labels = vec!["representation.boolean.binary"];

    let result = disambiguate_boolean_override(&values, &top_labels);
    assert!(result.is_none(), "Balanced binary should not be overridden");
}

#[test]
fn test_boolean_subtype_all_zeros_rejected() {
    // All-zero column through the subtype path — requires exactly 2 unique values
    let values: Vec<String> = vec!["0", "0", "0", "0", "0"]
        .into_iter()
        .map(String::from)
        .collect();
    let top_labels = vec!["representation.boolean.binary"];

    let result = disambiguate_boolean_subtype(&values, &top_labels);
    // unique_values.len() == 1, not 2, so binary subtype should not fire
    assert!(
        result.is_none(),
        "All-zero through subtype path should return None"
    );
}

// ── binary_vocab_veto full-column feed (BACKLOG #14) ──

#[test]
fn binary_vocab_veto_full_column_catches_rare_count() {
    let cc =
        ColumnClassifier::with_defaults(Box::new(crate::inference::MockClassifier::new("unknown")));

    // Mostly {0,1} with a single rare value above 1 — a count, not a flag.
    let mut counts: Vec<String> = vec!["0".into(); 60];
    counts.push("1".into());
    counts.push("30".into());
    let r = cc
        .compose_from_sense("Comments", &counts, "representation.boolean.binary", 0.5)
        .unwrap();
    assert_eq!(r.label, "representation.numeric.integer_number");

    // A genuine {0,1} binary is untouched.
    let bins: Vec<String> = vec!["0".into(), "1".into(), "0".into(), "1".into(), "1".into()];
    let r2 = cc
        .compose_from_sense("flag", &bins, "representation.boolean.binary", 0.5)
        .unwrap();
    assert_eq!(r2.label, "representation.boolean.binary");
}
