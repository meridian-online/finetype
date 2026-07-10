use super::super::*;

#[test]
fn test_numeric_sequential_detection() {
    let values: Vec<String> = vec!["1", "2", "3", "4", "5", "6", "7", "8", "9", "10"]
        .into_iter()
        .map(String::from)
        .collect();

    // Create mock results with increment label
    let results: Vec<ClassificationResult> = values
        .iter()
        .map(|_| ClassificationResult {
            label: "representation.identifier.increment".to_string(),
            confidence: 0.8,
            all_scores: vec![],
        })
        .collect();

    let votes = [
        ("representation.identifier.increment".to_string(), 8),
        ("representation.numeric.integer_number".to_string(), 2),
    ];
    let top_labels: Vec<&str> = votes.iter().map(|(l, _)| l.as_str()).collect();

    let result = disambiguate_numeric(&values, &results, &top_labels);
    assert!(result.is_some());
    let (label, _rule) = result.unwrap();
    assert_eq!(label, "representation.identifier.increment");
}

#[test]
fn increment_substance_veto_is_default_on() {
    assert!(!rhh::is_disabled("increment_substance_veto"));
}

#[test]
fn values_form_increment_accepts_genuine_auto_increment() {
    // Contiguous, unique, non-negative run (1..=200) — a real auto-increment.
    let vals: Vec<String> = (1..=200).map(|i| i.to_string()).collect();
    assert_eq!(values_form_increment(&vals), Some(true));
    // Tolerates a few gaps (deleted rows): still ≥80% range fill, ~unique.
    let gapped: Vec<String> = (1..=200)
        .filter(|i| i % 10 != 0)
        .map(|i| i.to_string())
        .collect();
    assert_eq!(values_form_increment(&gapped), Some(true));
}

#[test]
fn values_form_increment_rejects_evenly_spaced_non_increment() {
    // Evenly spaced but sparse (100,200,…,5000): low variance in diffs fools the
    // sample-based sequential check, but distinct/range is tiny → NOT an increment.
    let spaced: Vec<String> = (1..=50).map(|i| (i * 100).to_string()).collect();
    assert_eq!(values_form_increment(&spaced), Some(false));
    // A genuine integer measurement column (duplicated, non-contiguous).
    let measures: Vec<String> = ["3", "7", "3", "42", "7", "108", "3", "9", "256", "42"]
        .into_iter()
        .map(String::from)
        .collect();
    assert_eq!(values_form_increment(&measures), Some(false));
}

#[test]
fn values_form_increment_abstains_on_too_few_or_nonnumeric() {
    assert_eq!(
        values_form_increment(&["1".into(), "2".into(), "3".into()]),
        None
    );
    // 0 integers parsed → too few → None (leave label alone).
    let words: Vec<String> = ["a", "b", "c", "d", "e", "f"]
        .into_iter()
        .map(String::from)
        .collect();
    assert_eq!(values_form_increment(&words), None);
}

// ── SI number override tests ─────────────────────────────

#[test]
fn test_si_number_override_plain_decimals() {
    // Plain decimal values with no SI suffixes → should override to decimal_number
    let values: Vec<String> = vec!["5.1", "3.5", "1.4", "7.9", "0.2", "4.6"]
        .into_iter()
        .map(String::from)
        .collect();
    let result = disambiguate_si_number(&values);
    assert_eq!(
        result,
        Some((
            "representation.numeric.decimal_number".to_string(),
            "si_number_override_no_suffix".to_string()
        ))
    );
}

#[test]
fn test_si_number_override_real_si_values() {
    // Values with SI suffixes → should NOT override
    let values: Vec<String> = vec!["5.1K", "3.5M", "1.4B"]
        .into_iter()
        .map(String::from)
        .collect();
    let result = disambiguate_si_number(&values);
    assert_eq!(result, None);
}

#[test]
fn test_si_number_override_mixed_values() {
    // Even one SI suffix means the column is genuinely SI → no override
    let values: Vec<String> = vec!["5.1", "3.5K", "1.4"]
        .into_iter()
        .map(String::from)
        .collect();
    let result = disambiguate_si_number(&values);
    assert_eq!(result, None);
}

#[test]
fn test_si_number_override_negative_decimals() {
    // Negative decimals with no suffixes → should override
    let values: Vec<String> = vec!["-450.12", "732.57", "-1.003", "98.6"]
        .into_iter()
        .map(String::from)
        .collect();
    let result = disambiguate_si_number(&values);
    assert_eq!(
        result,
        Some((
            "representation.numeric.decimal_number".to_string(),
            "si_number_override_no_suffix".to_string()
        ))
    );
}

#[test]
fn test_si_number_override_empty_values() {
    // Empty values → should override (no SI suffixes found)
    let values: Vec<String> = vec!["", "  ", ""].into_iter().map(String::from).collect();
    let result = disambiguate_si_number(&values);
    assert_eq!(
        result,
        Some((
            "representation.numeric.decimal_number".to_string(),
            "si_number_override_no_suffix".to_string()
        ))
    );
}

// ==========================================================================
// Accuracy improvements — new rule tests
// ==========================================================================

#[test]
fn test_rule19_percentage_no_sign_demotes_to_decimal() {
    // Values that look like percentages by range but have no '%' sign
    // should be classified as decimal_number, not percentage.
    let values: Vec<String> = vec!["5.1", "3.5", "1.4", "0.2", "4.9"]
        .into_iter()
        .map(String::from)
        .collect();

    let results: Vec<ClassificationResult> = values
        .iter()
        .map(|_| ClassificationResult {
            label: "representation.numeric.percentage".to_string(),
            confidence: 0.8,
            all_scores: vec![],
        })
        .collect();

    let votes = vec![
        ("representation.numeric.percentage".to_string(), 4),
        ("representation.numeric.decimal_number".to_string(), 1),
    ];

    let result = disambiguate(&values, &results, &votes, 5, None);
    assert!(
        result.is_some(),
        "Rule 19 should fire for percentage without '%'"
    );
    let (label, rule) = result.unwrap();
    assert_eq!(label, "representation.numeric.decimal_number");
    assert_eq!(rule, "percentage_no_sign");
}

#[test]
fn test_rule19_percentage_with_sign_keeps_percentage() {
    // Values with actual '%' signs should stay as percentage
    let values: Vec<String> = vec!["35.36%", "12.5%", "98.1%", "0.5%", "100%"]
        .into_iter()
        .map(String::from)
        .collect();

    let results: Vec<ClassificationResult> = values
        .iter()
        .map(|_| ClassificationResult {
            label: "representation.numeric.percentage".to_string(),
            confidence: 0.9,
            all_scores: vec![],
        })
        .collect();

    let votes = vec![("representation.numeric.percentage".to_string(), 5)];

    let result = disambiguate(&values, &results, &votes, 5, None);
    // Rule 19 should NOT fire — values have '%'
    assert!(
        result.is_none()
            || result
                .as_ref()
                .map(|(_, r)| r != "percentage_no_sign")
                .unwrap_or(true),
        "Rule 19 should NOT fire when values contain '%'"
    );
}

// ── ColumnFeatures aggregation tests ─────────────────────

#[test]
fn test_aggregate_features_empty() {
    let cf = aggregate_features(&[]);
    assert_eq!(cf.mean, [0.0f32; FEATURE_DIM]);
    assert_eq!(cf.variance, [0.0f32; FEATURE_DIM]);
}

#[test]
fn test_aggregate_features_single_value() {
    let features = extract_features("abc123");
    let cf = aggregate_features(&[features]);
    // Mean should equal the single feature vector
    assert_eq!(cf.mean, features);
    // Variance should be zero (single observation)
    for v in cf.variance {
        assert!(
            v.abs() < 1e-6,
            "variance should be ~0 for single value, got {}",
            v
        );
    }
    // Min/max should equal the single feature vector
    assert_eq!(cf.min, features);
    assert_eq!(cf.max, features);
}

#[test]
fn test_aggregate_features_variance() {
    // Two values with different lengths → non-zero length variance
    let f1 = extract_features("abc"); // length = 3
    let f2 = extract_features("abcdef"); // length = 6
    let cf = aggregate_features(&[f1, f2]);

    let length_idx = feature_idx::LENGTH;
    // Mean length = (3 + 6) / 2 = 4.5
    assert!((cf.mean[length_idx] - 4.5).abs() < 0.01, "mean length");
    // Variance = ((3-4.5)² + (6-4.5)²) / 2 = (2.25 + 2.25) / 2 = 2.25
    assert!(
        (cf.variance[length_idx] - 2.25).abs() < 0.01,
        "variance length"
    );
    // Min/max
    assert!((cf.min[length_idx] - 3.0).abs() < 0.01, "min length");
    assert!((cf.max[length_idx] - 6.0).abs() < 0.01, "max length");
}

#[test]
fn test_uniform_length_hex_zero_variance() {
    // Uniform-length hex strings: all exactly 40 chars → zero length variance
    let shas = [
        "a".repeat(40),
        "b1c2d3e4f5a6b7c8d9e0a1b2c3d4e5f6a7b8c9d0".to_string(),
        "1234567890abcdef1234567890abcdef12345678".to_string(),
        "deadbeefdeadbeefdeadbeefdeadbeefdeadbeef".to_string(),
    ];
    let per_value: Vec<[f32; FEATURE_DIM]> = shas.iter().map(|s| extract_features(s)).collect();
    let cf = aggregate_features(&per_value);

    // Length variance should be exactly 0 (all same length)
    assert!(
        cf.variance[feature_idx::LENGTH] < 0.01,
        "uniform hex length variance should be ~0, got {}",
        cf.variance[feature_idx::LENGTH]
    );
    // All should be hex
    assert!(
        cf.mean[feature_idx::IS_HEX_STRING] >= 0.95,
        "uniform hex strings should all be hex, got {}",
        cf.mean[feature_idx::IS_HEX_STRING]
    );
}

#[test]
fn test_hash_mixed_length_nonzero_variance() {
    // Mixed hashes: MD5 (32 chars) + SHA1 (40 chars) + SHA256 (64 chars)
    let hashes = [
        "d41d8cd98f00b204e9800998ecf8427e".to_string(), // MD5: 32
        "da39a3ee5e6b4b0d3255bfef95601890afd80709".to_string(), // SHA1: 40
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(), // SHA256: 64
    ];
    let per_value: Vec<[f32; FEATURE_DIM]> = hashes.iter().map(|s| extract_features(s)).collect();
    let cf = aggregate_features(&per_value);

    // Length variance should be high (32, 40, 64 → mean=45.33, var=177.56)
    assert!(
        cf.variance[feature_idx::LENGTH] > 100.0,
        "mixed hash length variance should be high, got {}",
        cf.variance[feature_idx::LENGTH]
    );
    // All should still be hex
    assert!(
        cf.mean[feature_idx::IS_HEX_STRING] >= 0.95,
        "all hashes should be hex, got {}",
        cf.mean[feature_idx::IS_HEX_STRING]
    );
}

// Rule F4 tests removed — git_sha collapsed into technology.cryptographic.hash.

#[test]
fn test_rule_f5_numeric_code_demoted_without_leading_zeros() {
    // numeric_code winner with no leading zeros → should demote to integer_number
    // e.g., duration_minutes column with values: 180, 90, 120, 60
    let mut result = ColumnResult {
        label: "representation.identifier.numeric_code".to_string(),
        confidence: 1.0,
        vote_distribution: vec![("representation.identifier.numeric_code".to_string(), 1.0)],
        disambiguation_applied: false,
        disambiguation_rule: None,
        samples_used: 100,
        detected_locale: None,
        is_generic: false,
        column_features: None,
    };

    let mut cf = ColumnFeatures::empty();
    cf.mean[feature_idx::HAS_LEADING_ZERO] = 0.0; // no leading zeros

    let votes = vec![("representation.identifier.numeric_code".to_string(), 100)];

    feature_disambiguate(&mut result, &cf, &votes, 100);

    assert_eq!(result.label, "representation.numeric.integer_number");
    assert!(result.disambiguation_applied);
    assert!(result
        .disambiguation_rule
        .as_ref()
        .unwrap()
        .starts_with("feature_no_leading_zero"));
}

#[test]
fn test_rule_f5_numeric_code_kept_with_leading_zeros() {
    // numeric_code winner with leading zeros → should NOT demote
    // e.g., ZIP-like codes: 00123, 04500
    let mut result = ColumnResult {
        label: "representation.identifier.numeric_code".to_string(),
        confidence: 1.0,
        vote_distribution: vec![("representation.identifier.numeric_code".to_string(), 1.0)],
        disambiguation_applied: false,
        disambiguation_rule: None,
        samples_used: 100,
        detected_locale: None,
        is_generic: false,
        column_features: None,
    };

    let mut cf = ColumnFeatures::empty();
    cf.mean[feature_idx::HAS_LEADING_ZERO] = 0.5; // 50% have leading zeros

    let votes = vec![("representation.identifier.numeric_code".to_string(), 100)];

    feature_disambiguate(&mut result, &cf, &votes, 100);

    assert_eq!(
        result.label, "representation.identifier.numeric_code",
        "should NOT demote when leading zeros are present"
    );
    assert!(!result.disambiguation_applied);
}

#[test]
fn test_rule_f5_numeric_code_with_decimals_becomes_decimal_number() {
    // numeric_code winner with no leading zeros BUT decimal points present
    // e.g., earthquakes gap column: 10.0, 100.0, 101.0
    let mut result = ColumnResult {
        label: "representation.identifier.numeric_code".to_string(),
        confidence: 1.0,
        vote_distribution: vec![("representation.identifier.numeric_code".to_string(), 1.0)],
        disambiguation_applied: false,
        disambiguation_rule: None,
        samples_used: 100,
        detected_locale: None,
        is_generic: false,
        column_features: None,
    };

    let mut cf = ColumnFeatures::empty();
    cf.mean[feature_idx::HAS_LEADING_ZERO] = 0.0; // no leading zeros
    cf.mean[feature_idx::IS_FLOAT] = 1.0; // all values have decimal points

    let votes = vec![("representation.identifier.numeric_code".to_string(), 100)];

    feature_disambiguate(&mut result, &cf, &votes, 100);

    assert_eq!(
        result.label, "representation.numeric.decimal_number",
        "numeric_code with decimal points should become decimal_number"
    );
    assert!(result.disambiguation_applied);
    assert!(result
        .disambiguation_rule
        .as_ref()
        .unwrap()
        .starts_with("feature_decimal_over_numeric_code"));
}

#[test]
fn test_rule_f5b_decimal_demoted_to_integer_when_whole() {
    // BACKLOG #10: decimal_number prediction over whole values (IS_FLOAT≈0) → integer.
    // Lives in feature_sharpen (the composed path; classify_multi_branch:1437 +
    // compose_from_sense:1537 both call it), so the gate and native agree.
    let mut result = ColumnResult {
        label: "representation.numeric.decimal_number".to_string(),
        confidence: 1.0,
        vote_distribution: vec![("representation.numeric.decimal_number".to_string(), 1.0)],
        disambiguation_applied: false,
        disambiguation_rule: None,
        samples_used: 100,
        detected_locale: None,
        is_generic: false,
        column_features: None,
    };
    let mut cf = ColumnFeatures::empty();
    cf.mean[feature_idx::IS_FLOAT] = 0.0; // no fractional values
    feature_sharpen(&mut result, &cf);
    assert_eq!(result.label, "representation.numeric.integer_number");
    assert!(result.disambiguation_applied);
    assert!(result
        .disambiguation_rule
        .as_ref()
        .unwrap()
        .starts_with("feature_decimal_to_integer_is_float"));
}

#[test]
fn test_rule_f5b_keeps_decimal_when_fractional() {
    // Negative: a genuine decimal column (IS_FLOAT=1.0) stays decimal_number.
    let mut result = ColumnResult {
        label: "representation.numeric.decimal_number".to_string(),
        confidence: 1.0,
        vote_distribution: vec![("representation.numeric.decimal_number".to_string(), 1.0)],
        disambiguation_applied: false,
        disambiguation_rule: None,
        samples_used: 100,
        detected_locale: None,
        is_generic: false,
        column_features: None,
    };
    let mut cf = ColumnFeatures::empty();
    cf.mean[feature_idx::IS_FLOAT] = 1.0;
    feature_sharpen(&mut result, &cf);
    assert_eq!(result.label, "representation.numeric.decimal_number");
    assert!(!result.disambiguation_applied);
}

#[test]
fn test_rule_f3_enhanced_float_parseability() {
    // Simulate: decimal_number wins but float-parseability < 1.0 → hs_code override
    let mut result = ColumnResult {
        label: "representation.numeric.decimal_number".to_string(),
        confidence: 0.80,
        vote_distribution: vec![
            ("representation.numeric.decimal_number".to_string(), 0.70),
            ("geography.transportation.hs_code".to_string(), 0.20),
        ],
        disambiguation_applied: false,
        disambiguation_rule: None,
        samples_used: 100,
        detected_locale: None,
        is_generic: false,
        column_features: None,
    };

    let mut cf = ColumnFeatures::empty();
    cf.mean[feature_idx::DIGIT_RATIO] = 0.85;
    cf.mean[feature_idx::SEGMENT_COUNT_DOT] = 1.8; // between 1.5 and 2.0
    cf.mean[feature_idx::IS_FLOAT] = 0.5; // only half parse as float

    let votes = vec![
        ("representation.numeric.decimal_number".to_string(), 70),
        ("geography.transportation.hs_code".to_string(), 20),
    ];

    feature_disambiguate(&mut result, &cf, &votes, 100);

    assert_eq!(result.label, "geography.transportation.hs_code");
    assert!(result.disambiguation_applied);
    assert!(result
        .disambiguation_rule
        .as_ref()
        .unwrap()
        .contains("feature_hs_code"));
}

#[test]
fn test_rule_f6_short_code_demoted_from_file_extension() {
    // Short alphabetic codes like earthquake magType ("mb", "ms", "ml")
    // misclassified as file.extension → should demote to next vote
    let mut result = ColumnResult {
        label: "representation.file.extension".to_string(),
        confidence: 0.85,
        vote_distribution: vec![
            ("representation.file.extension".to_string(), 0.70),
            ("representation.categorical.categorical".to_string(), 0.20),
        ],
        disambiguation_applied: false,
        disambiguation_rule: None,
        samples_used: 100,
        detected_locale: None,
        is_generic: false,
        column_features: None,
    };

    let mut cf = ColumnFeatures::empty();
    cf.mean[feature_idx::LENGTH] = 2.5; // short codes: "mb", "ms", "ml", "mww"
    cf.mean[feature_idx::SEGMENT_COUNT_DOT] = 1.0; // no dots at all
    cf.mean[feature_idx::ALPHA_RATIO] = 1.0; // purely alphabetic

    let votes = vec![
        ("representation.file.extension".to_string(), 70),
        ("representation.categorical.categorical".to_string(), 20),
    ];

    feature_disambiguate(&mut result, &cf, &votes, 100);

    assert_eq!(result.label, "representation.categorical.categorical");
    assert!(result.disambiguation_applied);
    assert!(result
        .disambiguation_rule
        .as_ref()
        .unwrap()
        .contains("feature_short_code_not_extension"));
}

#[test]
fn test_rule_f6_real_extension_not_demoted() {
    // Real file extensions with dots (e.g., ".csv", ".json") should NOT be demoted
    let mut result = ColumnResult {
        label: "representation.file.extension".to_string(),
        confidence: 0.90,
        vote_distribution: vec![("representation.file.extension".to_string(), 1.0)],
        disambiguation_applied: false,
        disambiguation_rule: None,
        samples_used: 100,
        detected_locale: None,
        is_generic: false,
        column_features: None,
    };

    let mut cf = ColumnFeatures::empty();
    cf.mean[feature_idx::LENGTH] = 4.0; // ".csv" length
    cf.mean[feature_idx::SEGMENT_COUNT_DOT] = 2.0; // has dots
    cf.mean[feature_idx::ALPHA_RATIO] = 0.75;

    let votes = vec![("representation.file.extension".to_string(), 90)];

    feature_disambiguate(&mut result, &cf, &votes, 100);

    assert_eq!(
        result.label, "representation.file.extension",
        "should NOT demote when values contain dots"
    );
    assert!(!result.disambiguation_applied);
}

#[test]
fn test_rule_f6_long_values_not_demoted() {
    // Longer strings classified as file.extension should NOT be demoted
    // (e.g., "dockerfile", "makefile" — longer than typical short codes)
    let mut result = ColumnResult {
        label: "representation.file.extension".to_string(),
        confidence: 0.85,
        vote_distribution: vec![("representation.file.extension".to_string(), 1.0)],
        disambiguation_applied: false,
        disambiguation_rule: None,
        samples_used: 100,
        detected_locale: None,
        is_generic: false,
        column_features: None,
    };

    let mut cf = ColumnFeatures::empty();
    cf.mean[feature_idx::LENGTH] = 8.0; // longer values
    cf.mean[feature_idx::SEGMENT_COUNT_DOT] = 1.0; // no dots
    cf.mean[feature_idx::ALPHA_RATIO] = 1.0;

    let votes = vec![("representation.file.extension".to_string(), 85)];

    feature_disambiguate(&mut result, &cf, &votes, 100);

    assert_eq!(
        result.label, "representation.file.extension",
        "should NOT demote when mean length > 4"
    );
    assert!(!result.disambiguation_applied);
}

#[test]
fn test_rule_f6_fallback_to_categorical() {
    // When file.extension is the only vote, should fallback to categorical
    let mut result = ColumnResult {
        label: "representation.file.extension".to_string(),
        confidence: 0.90,
        vote_distribution: vec![("representation.file.extension".to_string(), 1.0)],
        disambiguation_applied: false,
        disambiguation_rule: None,
        samples_used: 100,
        detected_locale: None,
        is_generic: false,
        column_features: None,
    };

    let mut cf = ColumnFeatures::empty();
    cf.mean[feature_idx::LENGTH] = 2.0;
    cf.mean[feature_idx::SEGMENT_COUNT_DOT] = 1.0;
    cf.mean[feature_idx::ALPHA_RATIO] = 1.0;

    // Only file.extension in votes — no second option
    let votes = vec![("representation.file.extension".to_string(), 100)];

    feature_disambiguate(&mut result, &cf, &votes, 100);

    assert_eq!(
        result.label, "representation.categorical.categorical",
        "should fallback to categorical when no second vote exists"
    );
    assert!(result.disambiguation_applied);
}

// AC-2: feature_sharpen — F3 fires on decimal_number without hs_code in votes
#[test]
fn test_sharpen_f3_removed_decimal_stays_decimal() {
    // Even with HS code features, decimal_number should NOT be overridden
    // to hs_code by feature_sharpen. R20 in value_sharpen is now the only
    // path to hs_code.
    let mut result = ColumnResult {
        label: "representation.numeric.decimal_number".to_string(),
        confidence: 0.80,
        vote_distribution: vec![("representation.numeric.decimal_number".to_string(), 0.80)],
        disambiguation_applied: false,
        disambiguation_rule: None,
        samples_used: 50,
        detected_locale: None,
        is_generic: false,
        column_features: None,
    };

    let mut cf = ColumnFeatures::empty();
    cf.mean[feature_idx::DIGIT_RATIO] = 0.85; // ≥ 0.75
    cf.mean[feature_idx::SEGMENT_COUNT_DOT] = 2.5; // ≥ 2.0 (path A)
    cf.mean[feature_idx::IS_FLOAT] = 0.3; // < 1.0
    cf.mean[feature_idx::HAS_NEGATIVE_PREFIX] = 0.0; // no negative prefix
    cf.variance[feature_idx::SEGMENT_COUNT_DOT] = 0.1; // ≤ 0.5

    feature_sharpen(&mut result, &cf);

    assert_eq!(
        result.label, "representation.numeric.decimal_number",
        "F3 removed: decimal_number should NOT be overridden to hs_code by feature_sharpen"
    );
    assert!(
        !result.disambiguation_applied,
        "No disambiguation should fire for decimal_number with HS code features"
    );
}

// AC-2: feature_sharpen — F5 demotes numeric_code (single-entry votes)
#[test]
fn test_sharpen_f5_numeric_code_demoted_single_vote() {
    // Same as existing F5 test but using feature_sharpen (not feature_disambiguate)
    let mut result = ColumnResult {
        label: "representation.identifier.numeric_code".to_string(),
        confidence: 0.90,
        vote_distribution: vec![("representation.identifier.numeric_code".to_string(), 0.90)],
        disambiguation_applied: false,
        disambiguation_rule: None,
        samples_used: 50,
        detected_locale: None,
        is_generic: false,
        column_features: None,
    };

    let mut cf = ColumnFeatures::empty();
    cf.mean[feature_idx::HAS_LEADING_ZERO] = 0.0; // no leading zeros
    cf.mean[feature_idx::IS_FLOAT] = 0.0;

    feature_sharpen(&mut result, &cf);

    assert_eq!(
        result.label, "representation.numeric.integer_number",
        "F5 should demote numeric_code without leading zeros via feature_sharpen"
    );
    assert!(result.disambiguation_applied);
}

// AC-3: value_sharpen — R12 numeric disambiguation with single-entry votes
#[test]
fn test_sharpen_r12_numeric_single_vote() {
    // Year-like values predicted as integer_number
    let values: Vec<String> = vec!["2020", "2021", "2022", "2023", "2024"]
        .into_iter()
        .map(String::from)
        .collect();

    let result = value_sharpen(&values, "representation.numeric.integer_number", 0.80, None);

    assert!(result.is_some(), "R12 should fire for year-like values");
    let (label, _rule) = result.unwrap();
    assert_eq!(label, "datetime.component.year");
}
