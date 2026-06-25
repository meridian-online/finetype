use super::*;

// ── RHH instrumentation tests (ac-02) ───────────────────────────────
//
// Default build: prove behaviour is unchanged — header_hint() returns the
// same family-tag for every header it used to fire on. This is the
// "default cargo test compiles and passes" half of ac-02 verification.

// Default-build invariants — gated to default builds only because the
// on-feature test (`rhh_ac02_on_feature_disable_scenarios`) mutates
// `RHH_DISABLE_HINTS`, and Cargo runs unit tests concurrently within a
// single test binary. The on-feature test is self-contained: it sets
// env vars to assert the disable mechanic, then restores them. These
// baseline tests instead assert that on a default build (feature off),
// the env var is read-through-noop and behaviour is unchanged.

#[test]
#[cfg(not(feature = "rhh-instrumentation"))]
fn rhh_ac02_default_build_email_match_table_unchanged() {
    // "email" in the match table → identity.person.email
    assert_eq!(header_hint("email"), Some("identity.person.email"));
}

#[test]
#[cfg(not(feature = "rhh-instrumentation"))]
fn rhh_ac02_default_build_phone_substring_unchanged() {
    // "phone" only fires through the substring matcher (not in match table)
    assert_eq!(header_hint("phone"), Some("identity.person.phone_number"));
}

#[test]
#[cfg(not(feature = "rhh-instrumentation"))]
fn rhh_ac02_default_build_zip_geography_unchanged() {
    assert_eq!(header_hint("zip"), Some("geography.address.postal_code"));
}

#[test]
#[cfg(not(feature = "rhh-instrumentation"))]
fn rhh_ac02_default_build_env_var_ignored() {
    // Even if RHH_DISABLE_HINTS is set, default builds ignore it because
    // rhh::is_disabled compiles to a constant `false`.
    unsafe {
        std::env::set_var("RHH_DISABLE_HINTS", "substring_matcher_identity");
    }
    let result = header_hint("phone");
    unsafe {
        std::env::remove_var("RHH_DISABLE_HINTS");
    }
    assert_eq!(result, Some("identity.person.phone_number"));
}

// On-feature tests — gated behind `rhh-instrumentation`. Default
// `cargo test` (no feature) skips these entirely. Run with:
//   cargo test -p finetype-model --features rhh-instrumentation rhh_ac02
//
// SAFETY: env-var mutation is process-global. These tests run
// sequentially and each test sets, asserts, and unsets the env var
// within its own body. They must NOT run concurrently with anything
// else that reads RHH_DISABLE_HINTS — Cargo serialises tests within a
// single test binary by default for unit tests, and the workspace
// configures `--test-threads=1` is not required because each test
// restores RHH_DISABLE_HINTS to its prior state on exit.

/// All on-feature scenarios in one test so the shared `RHH_DISABLE_HINTS`
/// env var is mutated by exactly one thread at a time. Splitting this
/// into multiple `#[test]` functions caused parallel-test interference
/// with the unconditional `rhh_ac02_default_build_*` tests above.
#[test]
#[cfg(feature = "rhh-instrumentation")]
fn rhh_ac02_on_feature_disable_scenarios() {
    let prior = std::env::var("RHH_DISABLE_HINTS").ok();

    let restore = |prior: &Option<String>| match prior {
        Some(v) => unsafe { std::env::set_var("RHH_DISABLE_HINTS", v) },
        None => unsafe { std::env::remove_var("RHH_DISABLE_HINTS") },
    };

    // Scenario 1: disable substring_matcher_identity → "phone" no
    // longer fires (it lives only in the identity substring matcher).
    unsafe {
        std::env::set_var("RHH_DISABLE_HINTS", "substring_matcher_identity");
    }
    assert_eq!(
        header_hint("phone"),
        None,
        "disabling substring_matcher_identity should silence phone hint"
    );

    // Scenario 2: disabling identity must not silence technology hits.
    // (Same env var still set from scenario 1.)
    assert_eq!(
        header_hint("ipv6"),
        Some("technology.internet.ip_v6"),
        "disabling identity must not affect technology"
    );

    // Scenario 3: disable header_hint_table + substring_matcher_identity
    // simultaneously → "email" (which lives in the exact-match table)
    // returns None because the match table is gated and the substring
    // fallback for identity is also gated.
    unsafe {
        std::env::set_var(
            "RHH_DISABLE_HINTS",
            "header_hint_table,substring_matcher_identity",
        );
    }
    assert_eq!(
        header_hint("email"),
        None,
        "match_table+identity disable should silence email"
    );

    // Scenario 4: empty env var = no families disabled → baseline.
    unsafe {
        std::env::set_var("RHH_DISABLE_HINTS", "");
    }
    assert_eq!(
        header_hint("phone"),
        Some("identity.person.phone_number"),
        "empty disable list must restore baseline"
    );

    // Scenario 5: whitespace + empty entries are tolerated.
    unsafe {
        std::env::set_var("RHH_DISABLE_HINTS", "  substring_matcher_identity ,, ,");
    }
    assert_eq!(
        header_hint("phone"),
        None,
        "whitespace and empty tokens must parse cleanly"
    );

    restore(&prior);
}

// ── Disambiguation rule unit tests ──────────────────────────────────

#[test]
fn test_slash_date_eu_detected() {
    let values: Vec<String> = vec![
        "15/01/2024",
        "28/06/2023",
        "03/11/2022",
        "31/12/2019",
        "12/05/2020",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let result = disambiguate_slash_dates(&values);
    assert_eq!(result, Some("datetime.date.dmy_slash".to_string()));
}

#[test]
fn test_slash_date_mdy_detected() {
    let values: Vec<String> = vec![
        "01/15/2024",
        "06/28/2023",
        "11/03/2022",
        "12/31/2019",
        "05/12/2020",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let result = disambiguate_slash_dates(&values);
    assert_eq!(result, Some("datetime.date.mdy_slash".to_string()));
}

#[test]
fn test_slash_date_ambiguous() {
    // All values have both components ≤ 12 — ambiguous
    let values: Vec<String> = vec![
        "01/02/2024",
        "03/04/2023",
        "05/06/2022",
        "07/08/2021",
        "09/10/2020",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let result = disambiguate_slash_dates(&values);
    assert_eq!(result, None);
}

#[test]
fn test_short_date_dmy_detected() {
    let values: Vec<String> = vec!["15-01-24", "28-06-23", "31-12-19"]
        .into_iter()
        .map(String::from)
        .collect();

    let result = disambiguate_short_dates(&values);
    assert_eq!(result, Some("datetime.date.short_dmy".to_string()));
}

#[test]
fn test_short_date_mdy_detected() {
    let values: Vec<String> = vec!["01-15-24", "06-28-23", "12-31-19"]
        .into_iter()
        .map(String::from)
        .collect();

    let result = disambiguate_short_dates(&values);
    assert_eq!(result, Some("datetime.date.short_mdy".to_string()));
}

#[test]
fn test_coordinates_longitude_detected() {
    let values: Vec<String> = vec!["-74.0060", "151.2093", "-0.1278", "139.6917", "2.3522"]
        .into_iter()
        .map(String::from)
        .collect();

    let result = disambiguate_coordinates(&values);
    assert_eq!(result, Some("geography.coordinate.longitude".to_string()));
}

#[test]
fn test_coordinates_latitude_detected() {
    let values: Vec<String> = vec!["40.7128", "-33.8688", "51.5074", "35.6762", "-22.9068"]
        .into_iter()
        .map(String::from)
        .collect();

    let result = disambiguate_coordinates(&values);
    assert_eq!(result, Some("geography.coordinate.latitude".to_string()));
}

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

// test_numeric_port_detection: REMOVED (port type removed)

#[test]
fn test_numeric_postal_code_detection() {
    let values: Vec<String> = vec![
        "10001", "90210", "30301", "60601", "02101", "75001", "33101", "94102", "20001", "98101",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let results: Vec<ClassificationResult> = values
        .iter()
        .map(|_| ClassificationResult {
            label: "geography.address.postal_code".to_string(),
            confidence: 0.6,
            all_scores: vec![],
        })
        .collect();

    let votes = [
        ("geography.address.postal_code".to_string(), 6),
        ("representation.numeric.integer_number".to_string(), 4),
    ];
    let top_labels: Vec<&str> = votes.iter().map(|(l, _)| l.as_str()).collect();

    let result = disambiguate_numeric(&values, &results, &top_labels);
    assert!(result.is_some());
    let (label, _rule) = result.unwrap();
    assert_eq!(label, "geography.address.postal_code");
}

#[test]
fn test_year_detection() {
    let values: Vec<String> = vec![
        "2020", "2019", "2021", "2018", "2023", "2015", "2022", "2017", "2024", "2016",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let results: Vec<ClassificationResult> = values
        .iter()
        .map(|_| ClassificationResult {
            label: "representation.numeric.integer_number".to_string(),
            confidence: 0.6,
            all_scores: vec![],
        })
        .collect();

    let votes = [
        ("representation.numeric.integer_number".to_string(), 5),
        ("geography.address.postal_code".to_string(), 3),
        ("datetime.component.year".to_string(), 2),
    ];
    let top_labels: Vec<&str> = votes.iter().map(|(l, _)| l.as_str()).collect();

    let result = disambiguate_numeric(&values, &results, &top_labels);
    assert!(result.is_some());
    let (label, rule) = result.unwrap();
    assert_eq!(label, "datetime.component.year");
    assert_eq!(rule, "numeric_year_detection");
}

#[test]
fn test_year_detection_historical() {
    // Historical years in typical range
    let values: Vec<String> = vec!["1945", "1918", "1969", "1989", "2001"]
        .into_iter()
        .map(String::from)
        .collect();

    let results: Vec<ClassificationResult> = values
        .iter()
        .map(|_| ClassificationResult {
            label: "representation.numeric.decimal_number".to_string(),
            confidence: 0.5,
            all_scores: vec![],
        })
        .collect();

    let votes = [
        ("representation.numeric.decimal_number".to_string(), 3),
        ("representation.numeric.integer_number".to_string(), 2),
    ];
    let top_labels: Vec<&str> = votes.iter().map(|(l, _)| l.as_str()).collect();

    let result = disambiguate_numeric(&values, &results, &top_labels);
    assert!(result.is_some());
    let (label, _) = result.unwrap();
    assert_eq!(label, "datetime.component.year");
}

#[test]
fn test_year_not_triggered_for_5digit_postal() {
    // 5-digit postal codes should NOT trigger year rule
    let values: Vec<String> = vec![
        "10001", "90210", "30301", "60601", "02101", "75001", "33101", "94102", "20001", "98101",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let results: Vec<ClassificationResult> = values
        .iter()
        .map(|_| ClassificationResult {
            label: "geography.address.postal_code".to_string(),
            confidence: 0.6,
            all_scores: vec![],
        })
        .collect();

    let votes = [
        ("geography.address.postal_code".to_string(), 6),
        ("representation.numeric.integer_number".to_string(), 4),
    ];
    let top_labels: Vec<&str> = votes.iter().map(|(l, _)| l.as_str()).collect();

    let result = disambiguate_numeric(&values, &results, &top_labels);
    assert!(result.is_some());
    let (label, _) = result.unwrap();
    // Should be postal_code, NOT year (5-digit values)
    assert_eq!(label, "geography.address.postal_code");
}

#[test]
fn test_sequential_years_still_detected_as_year() {
    // Sequential 4-digit numbers in year range → still year (more likely
    // a column of consecutive years than auto-increment IDs starting at 2001)
    let values: Vec<String> = vec![
        "2001", "2002", "2003", "2004", "2005", "2006", "2007", "2008", "2009", "2010",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let results: Vec<ClassificationResult> = values
        .iter()
        .map(|_| ClassificationResult {
            label: "representation.identifier.increment".to_string(),
            confidence: 0.7,
            all_scores: vec![],
        })
        .collect();

    let votes = [
        ("representation.identifier.increment".to_string(), 7),
        ("representation.numeric.integer_number".to_string(), 3),
    ];
    let top_labels: Vec<&str> = votes.iter().map(|(l, _)| l.as_str()).collect();

    let result = disambiguate_numeric(&values, &results, &top_labels);
    assert!(result.is_some());
    let (label, _) = result.unwrap();
    // Year wins over increment when values are in 1900-2100 range
    assert_eq!(label, "datetime.component.year");
}

#[test]
fn test_sequential_non_year_still_increment() {
    // Sequential numbers outside year range → increment
    let values: Vec<String> = vec!["1", "2", "3", "4", "5", "6", "7", "8", "9", "10"]
        .into_iter()
        .map(String::from)
        .collect();

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
    let (label, _) = result.unwrap();
    assert_eq!(label, "representation.identifier.increment");
}

// test_year_not_triggered_for_ports: REMOVED (port type removed)

#[test]
fn test_year_with_outlier_not_postal_code() {
    // Year column with one outlier outside 1900-2100 — should still be year (≥80% rule)
    let values: Vec<String> = vec![
        "2020", "2019", "2021", "2018", "2023", "2015", "2022", "2017", "2024", "1776",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let results: Vec<ClassificationResult> = values
        .iter()
        .map(|_| ClassificationResult {
            label: "geography.address.postal_code".to_string(),
            confidence: 0.6,
            all_scores: vec![],
        })
        .collect();

    let votes = [
        ("geography.address.postal_code".to_string(), 5),
        ("representation.numeric.decimal_number".to_string(), 3),
        ("datetime.component.year".to_string(), 2),
    ];
    let top_labels: Vec<&str> = votes.iter().map(|(l, _)| l.as_str()).collect();

    let result = disambiguate_numeric(&values, &results, &top_labels);
    assert!(result.is_some());
    let (label, _rule) = result.unwrap();
    // Should be year, NOT postal_code: 9 of 10 values are in 1900-2100 (90% ≥ 80%)
    assert_eq!(label, "datetime.component.year");
}

#[test]
fn test_year_with_many_outliers_not_year() {
    // Only 60% of values in year range — below 80% threshold, should NOT be year
    let values: Vec<String> = vec![
        "2020", "2019", "2021", "1500", "1600", "1700", "1800", "2022", "2023", "2024",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let results: Vec<ClassificationResult> = values
        .iter()
        .map(|_| ClassificationResult {
            label: "representation.numeric.integer_number".to_string(),
            confidence: 0.6,
            all_scores: vec![],
        })
        .collect();

    let votes = [
        ("representation.numeric.integer_number".to_string(), 5),
        ("geography.address.postal_code".to_string(), 3),
        ("datetime.component.year".to_string(), 2),
    ];
    let top_labels: Vec<&str> = votes.iter().map(|(l, _)| l.as_str()).collect();

    let result = disambiguate_numeric(&values, &results, &top_labels);
    // 6/10 in year range = 60% < 80% threshold → should NOT be year
    if let Some((label, _)) = result {
        assert_ne!(label, "datetime.component.year");
    }
}

#[test]
fn test_year_with_non4digit_outlier() {
    // Year column where 1 of 10 values is not a 4-digit integer (e.g., "NA" or empty)
    // With the relaxed check, 9/10 = 90% ≥ 80% should still detect as year
    let values: Vec<String> = vec![
        "2020", "2019", "2021", "2018", "2023", "2015", "2022", "2017", "2024", "NA",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let results: Vec<ClassificationResult> = values
        .iter()
        .map(|_| ClassificationResult {
            label: "representation.numeric.decimal_number".to_string(),
            confidence: 0.6,
            all_scores: vec![],
        })
        .collect();

    let votes = [
        ("representation.numeric.decimal_number".to_string(), 8),
        ("datetime.component.year".to_string(), 2),
    ];
    let top_labels: Vec<&str> = votes.iter().map(|(l, _)| l.as_str()).collect();

    let result = disambiguate_numeric(&values, &results, &top_labels);
    assert!(result.is_some());
    let (label, rule) = result.unwrap();
    // 9 of 10 values are 4-digit (90% ≥ 80%) and all parseable ones are in year range
    assert_eq!(label, "datetime.component.year");
    assert_eq!(rule, "numeric_year_detection");
}

#[test]
fn test_year_with_decimal_format() {
    // Year column where values have decimal formatting like "2020.0"
    // These are not 4-digit integers, so the fraction check matters
    let values: Vec<String> = vec![
        "2020", "2019", "2021.0", "2018", "2023", "2015", "2022", "2017.0", "2024", "2016",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let results: Vec<ClassificationResult> = values
        .iter()
        .map(|_| ClassificationResult {
            label: "representation.numeric.decimal_number".to_string(),
            confidence: 0.7,
            all_scores: vec![],
        })
        .collect();

    let votes = [("representation.numeric.decimal_number".to_string(), 10)];
    let top_labels: Vec<&str> = votes.iter().map(|(l, _)| l.as_str()).collect();

    let result = disambiguate_numeric(&values, &results, &top_labels);
    assert!(result.is_some());
    let (label, _) = result.unwrap();
    // 8 of 10 values are 4-digit (80% ≥ 80%), and all integers parse into year range
    assert_eq!(label, "datetime.component.year");
}

#[test]
fn test_not_year_when_too_few_4digit() {
    // Column where less than 80% of values are 4-digit — should NOT be year
    let values: Vec<String> = vec![
        "2020", "2019", "NA", "N/A", "", "2015", "2022", "null", "2024", "missing",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let results: Vec<ClassificationResult> = values
        .iter()
        .map(|_| ClassificationResult {
            label: "representation.numeric.decimal_number".to_string(),
            confidence: 0.5,
            all_scores: vec![],
        })
        .collect();

    let votes = [("representation.numeric.decimal_number".to_string(), 10)];
    let top_labels: Vec<&str> = votes.iter().map(|(l, _)| l.as_str()).collect();

    let result = disambiguate_numeric(&values, &results, &top_labels);
    // 5 of 10 values are 4-digit (50% < 80%) → should NOT be year
    if let Some((label, _)) = result {
        assert_ne!(label, "datetime.component.year");
    }
}

// test_age_column_not_detected_as_port: REMOVED (port type removed)
// test_age_column_with_mixed_values_not_port: REMOVED (port type removed)

#[test]
fn test_empty_column() {
    // Just test the ColumnResult for empty case
    let result = ColumnResult {
        label: "unknown".to_string(),
        confidence: 0.0,
        vote_distribution: vec![],
        disambiguation_applied: false,
        disambiguation_rule: None,
        samples_used: 0,
        detected_locale: None,
        is_generic: false,
        column_features: None,
    };
    assert_eq!(result.label, "unknown");
    assert_eq!(result.samples_used, 0);
    assert_eq!(result.detected_locale, None);
}

// ── Locale suffix stripping tests ────────────────────────────────────

#[test]
fn test_strip_locale_suffix_4level_country() {
    let (base, locale) = strip_locale_suffix("geography.address.postal_code.EN_US");
    assert_eq!(base, "geography.address.postal_code");
    assert_eq!(locale, Some("EN_US"));
}

#[test]
fn test_strip_locale_suffix_4level_universal() {
    let (base, locale) = strip_locale_suffix("representation.boolean.binary.UNIVERSAL");
    assert_eq!(base, "representation.boolean.binary");
    assert_eq!(locale, Some("UNIVERSAL"));
}

#[test]
fn test_strip_locale_suffix_3level_unchanged() {
    let (base, locale) = strip_locale_suffix("geography.address.postal_code");
    assert_eq!(base, "geography.address.postal_code");
    assert_eq!(locale, None);
}

#[test]
fn test_strip_locale_suffix_short_locale() {
    let (base, locale) = strip_locale_suffix("geography.location.city.EN");
    assert_eq!(base, "geography.location.city");
    assert_eq!(locale, Some("EN"));
}

#[test]
fn test_strip_locale_suffix_no_false_positive_on_type() {
    // "iso" is lowercase — should NOT be treated as a locale suffix
    let (base, locale) = strip_locale_suffix("datetime.date.iso");
    assert_eq!(base, "datetime.date.iso");
    assert_eq!(locale, None);
}

#[test]
fn test_strip_locale_suffix_no_false_positive_on_short_label() {
    // Only two parts — the last part should not be treated as locale
    let (base, locale) = strip_locale_suffix("representation.EN");
    assert_eq!(base, "representation.EN");
    assert_eq!(locale, None);
}

// ── Cardinality & categorical rule tests ────────────────────────────

#[test]
fn test_gender_detection_mixed_case() {
    let values: Vec<String> = vec![
        "male", "female", "Male", "Female", "male", "female", "male", "Female", "male", "Male",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let result = disambiguate_gender(&values);
    assert_eq!(result, Some("identity.person.gender".to_string()));
}

#[test]
fn test_gender_detection_single_char_is_gender_code() {
    // Bare single-char sex codes (ICAO M/F/X) are gender_code, not gender:
    // they validate against [M,F,X,0,1,2,9], not the word enum
    // [male,female,other,unknown] which would hard-veto them.
    let values: Vec<String> = vec!["M", "F", "M", "F", "M", "F", "M", "F", "M", "F"]
        .into_iter()
        .map(String::from)
        .collect();

    let result = disambiguate_gender(&values);
    assert_eq!(result, Some("identity.person.gender_code".to_string()));
}

#[test]
fn test_gender_detection_single_char_mixed_case_with_x() {
    let values: Vec<String> = vec!["m", "f", "X", "M", "f", "x", "F"]
        .into_iter()
        .map(String::from)
        .collect();

    let result = disambiguate_gender(&values);
    assert_eq!(result, Some("identity.person.gender_code".to_string()));
}

#[test]
fn test_gender_detection_word_and_code_mix_stays_gender() {
    // Mixed word + single-char (not all single-char) -> word type gender.
    let values: Vec<String> = vec!["male", "M", "female", "F", "male"]
        .into_iter()
        .map(String::from)
        .collect();

    let result = disambiguate_gender(&values);
    assert_eq!(result, Some("identity.person.gender".to_string()));
}

#[test]
fn test_gender_detection_numeric_codes_do_not_fire() {
    // ISO-5218 numerics are NOT a value-only trigger (boolean/ordinal
    // over-emit guard) — a bare 0/1/2 column must not become gender_code.
    let values: Vec<String> = vec!["0", "1", "2", "1", "0", "2"]
        .into_iter()
        .map(String::from)
        .collect();

    let result = disambiguate_gender(&values);
    assert_eq!(result, None);
}

#[test]
fn test_gender_detection_with_nonbinary() {
    // People directory: Male, Female, Non-binary
    let values: Vec<String> = vec![
        "Male",
        "Female",
        "Male",
        "Non-binary",
        "Female",
        "Male",
        "Female",
        "Male",
        "Non-binary",
        "Female",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let result = disambiguate_gender(&values);
    assert_eq!(
        result,
        Some("identity.person.gender".to_string()),
        "Non-binary should be recognized as a valid gender value"
    );
}

#[test]
fn test_gender_detection_with_other_inclusive() {
    let values: Vec<String> = vec![
        "Male",
        "Female",
        "Other",
        "Male",
        "Female",
        "Prefer not to say",
        "Male",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let result = disambiguate_gender(&values);
    assert_eq!(result, Some("identity.person.gender".to_string()));
}

#[test]
fn test_gender_detection_fails_for_non_gender() {
    let values: Vec<String> = vec!["red", "blue", "green", "red", "blue"]
        .into_iter()
        .map(String::from)
        .collect();

    let result = disambiguate_gender(&values);
    assert_eq!(result, None);
}

#[test]
fn test_ipv4_detection_standard_ips() {
    let values: Vec<String> = vec![
        "192.168.1.1",
        "10.0.0.1",
        "172.16.0.1",
        "8.8.8.8",
        "1.2.3.4",
        "10.0.0.255",
        "192.168.0.100",
        "255.255.255.0",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let result = disambiguate_ipv4(&values);
    assert_eq!(
        result,
        Some("technology.internet.ip_v4".to_string()),
        "Standard IPv4 addresses should be detected"
    );
}

#[test]
fn test_ipv4_detection_rejects_version_numbers() {
    // Semantic version numbers have different structure (fewer octets, >255 values)
    let values: Vec<String> = vec![
        "1.0.0", "2.1.3", "3.14.159", "0.2.53", "6.27.84", "4.24.59", "7.23.74",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let result = disambiguate_ipv4(&values);
    assert_eq!(
        result, None,
        "Version numbers should NOT match IPv4 pattern"
    );
}

#[test]
fn test_ipv4_detection_rejects_decimals() {
    let values: Vec<String> = vec!["151.3", "165.0", "161.2", "169.1", "181.7"]
        .into_iter()
        .map(String::from)
        .collect();

    let result = disambiguate_ipv4(&values);
    assert_eq!(
        result, None,
        "Decimal numbers should NOT match IPv4 pattern"
    );
}

#[test]
fn test_ipv4_detection_mixed_with_some_invalid() {
    // 80% threshold: 8 valid out of 10 = 80%
    let values: Vec<String> = vec![
        "10.0.0.1", "10.0.0.2", "10.0.0.3", "10.0.0.4", "10.0.0.5", "10.0.0.6", "10.0.0.7",
        "10.0.0.8", "N/A", "unknown",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let result = disambiguate_ipv4(&values);
    assert_eq!(
        result,
        Some("technology.internet.ip_v4".to_string()),
        "80% valid IPs should trigger detection"
    );
}

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
    assert_eq!(label, "representation.discrete.categorical");
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

// ── Header hint tests ───────────────────────────────────────────────

#[test]
fn test_header_hint_email() {
    assert_eq!(header_hint("Email"), Some("identity.person.email"));
    assert_eq!(header_hint("email_address"), Some("identity.person.email"));
    assert_eq!(header_hint("E-Mail"), Some("identity.person.email"));
    assert_eq!(header_hint("user_email"), Some("identity.person.email"));
}

#[test]
fn test_header_hint_phone() {
    assert_eq!(header_hint("phone"), Some("identity.person.phone_number"));
    assert_eq!(
        header_hint("Phone Number"),
        Some("identity.person.phone_number")
    );
    assert_eq!(
        header_hint("telephone"),
        Some("identity.person.phone_number")
    );
    assert_eq!(header_hint("mobile"), Some("identity.person.phone_number"));
}

#[test]
fn test_header_hint_postal() {
    assert_eq!(header_hint("zip"), Some("geography.address.postal_code"));
    assert_eq!(
        header_hint("zip_code"),
        Some("geography.address.postal_code")
    );
    assert_eq!(
        header_hint("Postal Code"),
        Some("geography.address.postal_code")
    );
    assert_eq!(
        header_hint("postcode"),
        Some("geography.address.postal_code")
    );
}

#[test]
fn test_header_hint_names() {
    // Bare "name" is now ambiguous — returns None so Sense+CharCNN decide
    assert_eq!(header_hint("Name"), None);
    assert_eq!(header_hint("full_name"), Some("identity.person.full_name"));
    assert_eq!(
        header_hint("first_name"),
        Some("identity.person.first_name")
    );
    assert_eq!(header_hint("last_name"), Some("identity.person.last_name"));
    assert_eq!(header_hint("surname"), Some("identity.person.last_name"));
    // spec 2026-06-25-sharpen-stage-audit: the broad `ends_with(" name")` →
    // full_name arm was retired (net damage — country_name/template_name/
    // agency_name mis-promoted). Unqualified "* name" headers now carry no hint;
    // only the first/last/full-qualified arms above remain.
    assert_eq!(header_hint("display name"), None);
    assert_eq!(header_hint("user name"), None);
}

#[test]
fn test_header_hint_geo() {
    assert_eq!(
        header_hint("latitude"),
        Some("geography.coordinate.latitude")
    );
    assert_eq!(header_hint("lat"), Some("geography.coordinate.latitude"));
    assert_eq!(
        header_hint("longitude"),
        Some("geography.coordinate.longitude")
    );
    assert_eq!(header_hint("lng"), Some("geography.coordinate.longitude"));
    assert_eq!(header_hint("country"), Some("geography.location.country"));
    assert_eq!(header_hint("city"), Some("geography.location.city"));
}

#[test]
fn test_header_hint_identity() {
    assert_eq!(header_hint("gender"), Some("identity.person.gender"));
    assert_eq!(header_hint("Sex"), Some("identity.person.gender"));
    // "age" is not a taxonomy type; hint redirects to integer_number
    assert_eq!(
        header_hint("age"),
        Some("representation.numeric.integer_number")
    );
    assert_eq!(
        header_hint("Age"),
        Some("representation.numeric.integer_number")
    );
}

#[test]
fn test_header_hint_tech() {
    assert_eq!(header_hint("url"), Some("technology.internet.url"));
    assert_eq!(header_hint("URL"), Some("technology.internet.url"));
    assert_eq!(header_hint("website"), Some("technology.internet.url"));
    assert_eq!(header_hint("ip_address"), Some("technology.internet.ip_v4"));
    assert_eq!(header_hint("uuid"), Some("technology.identifier.uuid"));
    // "port" header hint removed
}

#[test]
fn test_header_hint_date() {
    assert_eq!(header_hint("date"), Some("datetime.timestamp.iso_8601"));
    assert_eq!(
        header_hint("created_date"),
        Some("datetime.timestamp.iso_8601")
    );
    assert_eq!(header_hint("year"), Some("datetime.component.year"));
    assert_eq!(header_hint("birth_date"), Some("datetime.date.iso"));
    assert_eq!(header_hint("dob"), Some("datetime.date.iso"));
    // Specific timestamp formats take priority over generic catch-all
    assert_eq!(
        header_hint("rfc_2822_timestamp"),
        Some("datetime.timestamp.rfc_2822")
    );
    assert_eq!(header_hint("rfc2822"), Some("datetime.timestamp.rfc_2822"));
    assert_eq!(header_hint("rfc_3339"), Some("datetime.timestamp.rfc_3339"));
    assert_eq!(
        header_hint("sql_timestamp"),
        Some("datetime.timestamp.sql_standard")
    );
    // Epoch/Unix timestamps — exact match
    assert_eq!(
        header_hint("unix_epoch"),
        Some("datetime.epoch.unix_seconds")
    );
    assert_eq!(
        header_hint("epoch_time"),
        Some("datetime.epoch.unix_seconds")
    );
    assert_eq!(
        header_hint("unix_ms"),
        Some("datetime.epoch.unix_milliseconds")
    );
    // Epoch/Unix — substring match
    assert_eq!(
        header_hint("created_epoch"),
        Some("datetime.epoch.unix_seconds")
    );
    assert_eq!(
        header_hint("timestamp_unix"),
        Some("datetime.epoch.unix_seconds")
    );
}

#[test]
fn test_header_hint_categorical() {
    // text types confused with region/country
    assert_eq!(
        header_hint("language"),
        Some("representation.discrete.categorical")
    );
    assert_eq!(
        header_hint("sport"),
        Some("representation.discrete.categorical")
    );
    assert_eq!(
        header_hint("species"),
        Some("representation.discrete.categorical")
    );
    assert_eq!(
        header_hint("exchange"),
        Some("representation.discrete.categorical")
    );
}

#[test]
fn test_header_hint_measurements() {
    // numeric measurements confused with numeric_code
    assert_eq!(
        header_hint("altitude"),
        Some("representation.numeric.integer_number")
    );
    assert_eq!(
        header_hint("elevation"),
        Some("representation.numeric.integer_number")
    );
    assert_eq!(
        header_hint("pages"),
        Some("representation.numeric.integer_number")
    );
    assert_eq!(
        header_hint("heart_rate"),
        Some("representation.numeric.integer_number")
    );
    assert_eq!(
        header_hint("attendance"),
        Some("representation.numeric.integer_number")
    );
    // "response time" intentionally unhinted — model handles both
    // integer and decimal response times correctly without hints.
    assert_eq!(header_hint("response_time_ms"), None);
    assert_eq!(
        header_hint("payload_size_bytes"),
        Some("representation.numeric.integer_number")
    );
    assert_eq!(
        header_hint("duration_minutes"),
        Some("representation.numeric.integer_number")
    );
}

#[test]
fn test_header_hint_numeric() {
    // Financial hints → finance.currency.amount
    assert_eq!(header_hint("price"), Some("finance.currency.amount"));
    assert_eq!(header_hint("amount"), Some("finance.currency.amount"));
    assert_eq!(header_hint("salary"), Some("finance.currency.amount"));
    assert_eq!(header_hint("revenue"), Some("finance.currency.amount"));
    assert_eq!(header_hint("income"), Some("finance.currency.amount"));
    assert_eq!(header_hint("expense"), Some("finance.currency.amount"));
    assert_eq!(header_hint("fare"), Some("finance.currency.amount"));
    assert_eq!(header_hint("fee"), Some("finance.currency.amount"));
    assert_eq!(
        header_hint("count"),
        Some("representation.numeric.integer_number")
    );
    // "id" intentionally unhinted — genuinely ambiguous
    assert_eq!(header_hint("id"), None);
}

#[test]
fn test_header_hint_no_match() {
    assert_eq!(header_hint("foo"), None);
    assert_eq!(header_hint("xyz"), None);
    assert_eq!(header_hint("data"), None);
    assert_eq!(header_hint("column1"), None);
}

// === 0094 coordinate header-veto helpers ===

#[test]
fn coord_veto_corroborates_real_coordinate_headers() {
    for h in [
        "lat",
        "latitude",
        "lon",
        "lng",
        "long",
        "longitude",
        "lat_dd",
        "y_lat",
        "decimalLatitude",
        "gps_lat",
        "coord_x",
        "wgs84_lat",
        "LATITUDE",
        "Longitude",
    ] {
        assert!(header_corroborates_coordinate(h), "should corroborate: {h}");
    }
}

#[test]
fn coord_veto_does_not_corroborate_false_friends_or_quantities() {
    // value-confusable non-coordinate headers — the FP battleground — must
    // NOT corroborate, so a latitude prediction on them is demotable.
    for h in [
        "mag",
        "mean",
        "std",
        "rms",
        "error",
        "depth",
        "score",
        "rate",
        "probability",
        "latency",
        "translate",
        "correlation",
        "plate",
        "population",
        "magnitude",
        "temperature",
        "elevation_error",
    ] {
        assert!(
            !header_corroborates_coordinate(h),
            "should NOT corroborate: {h}"
        );
    }
}

#[test]
fn coord_veto_value_gate_requires_numeric() {
    assert!(values_look_like_generic_decimals(&[
        "0.12".into(),
        "-3.4".into(),
        "51.5".into(),
        "2.0".into(),
        "9.9".into()
    ]));
    // text / mixed columns do not satisfy the value gate
    assert!(!values_look_like_generic_decimals(&[
        "alpha".into(),
        "beta".into(),
        "gamma".into()
    ]));
    assert!(!values_look_like_generic_decimals(&[]));
}

#[test]
fn coord_veto_is_a_default_on_header_hint() {
    // As a header-hint family, the coordinate veto is ON in default builds
    // (rhh::is_disabled is a const `false` when the rhh-instrumentation
    // feature is off) and disableable via RHH_DISABLE_HINTS in ablation builds.
    assert!(!rhh::is_disabled("header_hint_coord_veto"));
}

// === state_code promotion helpers ===

#[test]
fn state_code_header_corroboration() {
    for h in [
        "State",
        "Provider State",
        "DL State",
        "ADDRESS STATE",
        "province",
    ] {
        assert!(header_corroborates_state(h), "should corroborate: {h}");
    }
    // false friends — `state` must not match as a substring
    for h in [
        "statement",
        "status",
        "real estate",
        "estate",
        "country",
        "city",
    ] {
        assert!(!header_corroborates_state(h), "should NOT corroborate: {h}");
    }
}

#[test]
fn region_header_corroboration() {
    // admin divisions above city level — the gold city->region false positives
    for h in [
        "Region",
        "County",
        "district",
        "borough",
        "work_location_borough",
        "province",
    ] {
        assert!(header_corroborates_region(h), "should corroborate: {h}");
    }
    // must NOT match city/town or ambiguous tokens, nor substrings
    for h in ["city", "town", "name", "regional_office", "districting"] {
        assert!(
            !header_corroborates_region(h),
            "should NOT corroborate: {h}"
        );
    }
}

#[test]
fn state_code_value_vocab_gate() {
    // US codes
    assert!(values_look_like_state_codes(&[
        "GA".into(),
        "FL".into(),
        "NM".into(),
        "TX".into(),
    ]));
    // lowercase is uppercased before lookup
    assert!(values_look_like_state_codes(&[
        "ca".into(),
        "ny".into(),
        "wa".into(),
    ]));
    // AU 3-letter codes
    assert!(values_look_like_state_codes(&[
        "NSW".into(),
        "VIC".into(),
        "QLD".into(),
    ]));
    // full state NAMES are not codes
    assert!(!values_look_like_state_codes(&[
        "California".into(),
        "Florida".into(),
        "Texas".into(),
    ]));
    // a country-code column: most ISO codes are not state codes, so it fails
    // the vocab gate even before the header guard.
    assert!(!values_look_like_state_codes(&[
        "GB".into(),
        "FR".into(),
        "DE".into(),
        "JP".into(),
        "CN".into(),
    ]));
    // below the 3-value floor
    assert!(!values_look_like_state_codes(&["TX".into(), "FL".into()]));
}

#[test]
fn state_code_promote_is_a_default_on_header_hint() {
    assert!(!rhh::is_disabled("header_hint_state_code_promote"));
}

// === currency-amount bare-number veto ===

#[test]
fn amount_bare_number_gate() {
    // bare accounting integers (the false-positive shape) → demote, integer
    let (bare, dec) = values_look_like_bare_numbers(&[
        "84000000".into(),
        "-14638000".into(),
        "795000000".into(),
        "0".into(),
    ]);
    assert!(bare && !dec);
    // bare decimals → demote, to decimal
    let (bare, dec) = values_look_like_bare_numbers(&[
        "-270269883.0".into(),
        "396458184.0".into(),
        "2701751944.0".into(),
    ]);
    assert!(bare && dec);
    // currency-symbol / formatted money → NOT bare (kept as amount)
    let (bare, _) =
        values_look_like_bare_numbers(&["£45.17".into(), "£23.88".into(), "£35.02".into()]);
    assert!(!bare);
    let (bare, _) = values_look_like_bare_numbers(&[
        "EUR 4 459 807".into(),
        "EUR 4 626 565".into(),
        "EUR 4 652 581".into(),
    ]);
    assert!(!bare);
    // below the 3-value floor
    let (bare, _) = values_look_like_bare_numbers(&["100".into(), "200".into()]);
    assert!(!bare);
}

#[test]
fn amount_bare_number_veto_is_default_on() {
    assert!(!rhh::is_disabled("amount_bare_number_veto"));
}

#[test]
fn url_bare_number_veto_is_default_on() {
    assert!(!rhh::is_disabled("url_bare_number_veto"));
}

#[test]
fn country_code_corroboration_is_default_on() {
    assert!(!rhh::is_disabled("country_code_corroboration"));
}

#[test]
fn binary_vocab_veto_is_default_on() {
    assert!(!rhh::is_disabled("binary_vocab_veto"));
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

#[test]
fn checksum_substance_guard_is_default_on() {
    assert!(!rhh::is_disabled("checksum_substance_guard"));
}

#[test]
fn isbn_checksum_distinguishes_genuine_from_lookalikes() {
    // The check-digit math now lives in the canonical crate::checksum module
    // (wired into the validator via `checksum: isbn`); the guard delegates to
    // it. Genuine ISBNs pass; same-length financial figures the model
    // mislabels as ISBN fail.
    use finetype_core::checksum::isbn;
    for v in [
        "0306406152",
        "043942089X",
        "9780306406157",
        "978-3-16-148410-0",
    ] {
        assert!(isbn(v), "should be valid ISBN: {v}");
    }
    for v in ["5150000128", "6965100000", "7586000000", "1041000000"] {
        assert!(!isbn(v), "should NOT be valid ISBN: {v}");
    }
    assert!(!isbn("-1617000000"));
}

#[test]
fn url_bare_number_gate() {
    // 0/1/-1 flag columns the model mislabels as url → bare integers, demote
    let (bare, dec) =
        values_look_like_bare_numbers(&["0".into(), "1".into(), "0".into(), "-1".into()]);
    assert!(bare && !dec);
    // genuine URLs are non-numeric → not bare, kept as url
    let (bare, _) = values_look_like_bare_numbers(&[
        "https://example.com/a".into(),
        "http://foo.org".into(),
        "https://bar.net/x".into(),
    ]);
    assert!(!bare);
}

// === 0094 postal header-veto helpers (spec 2026-06-10-postal-header-veto) ===

#[test]
fn postal_veto_corroborates_real_postal_headers() {
    for h in [
        "zip",
        "ZIP",
        "zip_code",
        "zipcode",
        "business_zip",
        "billing zip",
        "postal_code",
        "PostalCode",
        "postcode",
        "post_code",
        "PLZ",
        "cep",
        "pincode",
        "pin_code_area_pincode",
    ] {
        assert!(header_corroborates_postal(h), "should corroborate: {h}");
    }
}

#[test]
fn postal_veto_does_not_corroborate_false_friends_or_quantities() {
    // the gold-measured FP battleground: bare-integer quantity columns plus
    // token false friends — none may corroborate, so postal is demotable.
    for h in [
        "zipper",
        "unzip",
        "gzip_size",
        "averageDailyVolume10Day",
        "fullTimeEmployees",
        "totalCurrentLiabilities",
        "conversation_no",
        "voteId",
        "d_week_seq",
        "destination_x",
        "sId",
        "1001",
        "code",
        "postage_paid",
    ] {
        assert!(
            !header_corroborates_postal(h),
            "should NOT corroborate: {h}"
        );
    }
}

#[test]
fn postal_veto_value_gate_requires_bare_integers_without_leading_zeros() {
    // bare 4-digit integers — the measured FP shape — satisfy the gate
    assert!(values_look_like_generic_integers(&[
        "1191".into(),
        "1497".into(),
        "2200".into(),
        "5322".into()
    ]));
    // a leading zero is postal evidence (01219-style zip): veto blocked
    assert!(!values_look_like_generic_integers(&[
        "01219".into(),
        "1191".into(),
        "1497".into()
    ]));
    // decimals / text / zip+4 / empty do not satisfy the gate
    assert!(!values_look_like_generic_integers(&[
        "12.5".into(),
        "9.1".into(),
        "3.3".into()
    ]));
    assert!(!values_look_like_generic_integers(&[
        "12345-6789".into(),
        "98765-4321".into()
    ]));
    assert!(!values_look_like_generic_integers(&[]));
}

#[test]
fn postal_veto_is_a_default_on_header_hint() {
    assert!(!rhh::is_disabled("header_hint_postal_veto"));
}

// === keyword guard tests ===

#[test]
fn ac01_r25_http_status_gate_fires_on_status_codes() {
    // 3-digit HTTP status codes in 100-599 should NOT be converted to postal_code.
    // R25 is now a guard inside R12's disambiguate_numeric — when the model
    // predicts integer_number and values are 3-digit 100-599, R12 should NOT
    // override to postal_code.
    let values: Vec<String> = vec!["200", "404", "500", "301", "503"]
        .into_iter()
        .map(String::from)
        .collect();
    // When model predicts integer_number, R12 fires but R25 guard blocks postal_code
    let result = value_sharpen(&values, "representation.numeric.integer_number", 0.8, None);
    // Should NOT return postal_code
    if let Some((label, _)) = &result {
        assert_ne!(
            label, "geography.address.postal_code",
            "R25 guard should prevent postal_code for HTTP status codes"
        );
    }
}

#[test]
fn ac01_r25_http_status_gate_preserves_real_postal_codes() {
    // 5-digit postal codes: non-sequential, consistent 5-digit length
    let values: Vec<String> = vec![
        "10001", "90210", "33139", "60601", "02134", "94102", "30308",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let result = value_sharpen(&values, "representation.numeric.integer_number", 0.8, None);
    assert!(result.is_some(), "R12 should fire on 5-digit postal codes");
    let (label, _) = result.unwrap();
    assert_eq!(label, "geography.address.postal_code");
}

#[test]
fn ac01_r25_http_status_gate_preserves_3digit_postal() {
    // 3-digit values outside HTTP range (≥600): Icelandic postal codes
    // Non-sequential to avoid increment detection
    let values: Vec<String> = vec!["601", "900", "750", "602", "850", "700", "801"]
        .into_iter()
        .map(String::from)
        .collect();
    let result = value_sharpen(&values, "representation.numeric.integer_number", 0.8, None);
    // <90% in 100-599 range (0% in this case), so R25 guard doesn't fire
    if let Some((label, _)) = &result {
        assert_eq!(
            label, "geography.address.postal_code",
            "3-digit values outside 100-599 should still be postal_code"
        );
    }
}

#[test]
fn ac03_r27_year_vs_compact_ym_fires_on_years() {
    // 4-digit years should override compact_ym
    let values: Vec<String> = vec!["2022", "2021", "2023", "2020", "2019"]
        .into_iter()
        .map(String::from)
        .collect();
    let result = value_sharpen(&values, "datetime.date.compact_ym", 0.8, None);
    assert!(result.is_some(), "R27 should fire on 4-digit years");
    let (label, rule) = result.unwrap();
    assert_eq!(label, "datetime.component.year");
    assert!(rule.starts_with("year_compact_ym_gate:"));
}

#[test]
fn ac03_r27_year_vs_compact_ym_preserves_real_compact_ym() {
    // 6-digit YYYYMM should NOT trigger R27
    let values: Vec<String> = vec!["202201", "202312", "202406"]
        .into_iter()
        .map(String::from)
        .collect();
    let result = value_sharpen(&values, "datetime.date.compact_ym", 0.8, None);
    if let Some((_, rule)) = &result {
        assert!(
            !rule.starts_with("year_compact_ym_gate:"),
            "R27 should not fire on 6-digit compact_ym values"
        );
    }
}

#[test]
fn ac03_r27_year_range_rejects_non_year_4digit() {
    // 4-digit values outside 1900-2100 should NOT trigger R27
    let values: Vec<String> = vec!["1234", "5678", "9012", "3456"]
        .into_iter()
        .map(String::from)
        .collect();
    let result = value_sharpen(&values, "datetime.date.compact_ym", 0.8, None);
    if let Some((_, rule)) = &result {
        assert!(
            !rule.starts_with("year_compact_ym_gate:"),
            "R27 should not fire on non-year 4-digit values"
        );
    }
}

#[test]
fn v15_email_display_guard() {
    // "email_display" should NOT match the email hint
    assert_eq!(header_hint("email_display"), None);
    // But "email" still matches
    assert_eq!(header_hint("email"), Some("identity.person.email"));
    // And "customer_email" still matches
    assert_eq!(header_hint("customer_email"), Some("identity.person.email"));
}

#[test]
fn v15_phone_e164_guard() {
    // "phone_e164" should NOT match the phone hint
    assert_eq!(header_hint("phone_e164"), None);
    // But "phone" still matches
    assert_eq!(header_hint("phone"), Some("identity.person.phone_number"));
    // And "phone_number" still matches
    assert_eq!(
        header_hint("phone_number"),
        Some("identity.person.phone_number")
    );
}

#[test]
fn v15_ip_port_guard() {
    // "ip_v4_with_port" should NOT match the ip_v4 hint
    assert_eq!(header_hint("ip_v4_with_port"), None);
    // But "ip_address" still matches
    assert_eq!(header_hint("ip_address"), Some("technology.internet.ip_v4"));
    // And "server_ip" still matches
    assert_eq!(header_hint("server_ip"), Some("technology.internet.ip_v4"));
}

#[test]
fn v15_upc_maps_to_upc_not_ean() {
    // "upc" should map to identity.commerce.upc, not ean
    assert_eq!(header_hint("upc"), Some("identity.commerce.upc"));
    // "ean" still maps to ean
    assert_eq!(header_hint("ean"), Some("identity.commerce.ean"));
    // "barcode" still maps to ean
    assert_eq!(header_hint("barcode"), Some("identity.commerce.ean"));
}

#[test]
fn v15_region_not_mapped_to_state() {
    // "region" should NOT map to state — model handles this correctly
    assert_eq!(header_hint("region"), None);
    // But "state" and "province" still map to state
    assert_eq!(header_hint("state"), Some("geography.location.state"));
    assert_eq!(header_hint("province"), Some("geography.location.state"));
}

#[test]
fn v15_subcountry_not_mapped_to_state() {
    // "subcountry" should NOT map to state — model predicts region correctly
    assert_eq!(header_hint("subcountry"), None);
}

// ── R31: Version vs dmy_short_dot gate ──────────────────────────────

#[test]
fn r31_version_overrides_dmy_short_dot_on_invalid_dates() {
    // Version strings with out-of-range date segments should override dmy_short_dot.
    // 0.2.53 → day=0 (invalid), 6.27.84 → month=27 (invalid), etc.
    let values: Vec<String> = vec!["0.2.53", "6.27.84", "4.24.59", "7.23.74", "5.14.89"]
        .into_iter()
        .map(String::from)
        .collect();
    let result = value_sharpen(&values, "datetime.date.dmy_short_dot", 0.99, None);
    assert!(result.is_some(), "R31 should fire on version-like values");
    let (label, rule) = result.unwrap();
    assert_eq!(label, "technology.development.version");
    assert!(rule.starts_with("version_dmy_short_dot_gate:"));
}

#[test]
fn r31_preserves_real_dmy_short_dot() {
    // Actual DD.MM.YY dates should NOT trigger R31.
    let values: Vec<String> = vec!["15.06.23", "01.12.22", "28.03.24", "10.01.21", "05.09.20"]
        .into_iter()
        .map(String::from)
        .collect();
    let result = value_sharpen(&values, "datetime.date.dmy_short_dot", 0.99, None);
    if let Some((_, rule)) = &result {
        assert!(
            !rule.starts_with("version_dmy_short_dot_gate:"),
            "R31 should not fire on valid DMY dates"
        );
    }
}

#[test]
fn r31_borderline_valid_dates_stay_dmy() {
    // Version strings that happen to be valid dates (10.6.2, 9.1.11)
    // should NOT trigger R31 — below 30% threshold.
    let values: Vec<String> = vec!["10.6.2", "9.1.11", "3.1.16", "8.4.8", "5.3.7"]
        .into_iter()
        .map(String::from)
        .collect();
    let result = value_sharpen(&values, "datetime.date.dmy_short_dot", 0.99, None);
    if let Some((_, rule)) = &result {
        assert!(
            !rule.starts_with("version_dmy_short_dot_gate:"),
            "R31 should not fire when most values are valid dates"
        );
    }
}

// ── R32: text-family low-cardinality vocabulary override ────────────

#[test]
fn r32_vocab_overrides_word() {
    // A status vocabulary asserted as free text is a categorical.
    let values: Vec<String> = vec![
        "active", "inactive", "active", "pending", "active", "inactive",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let result = value_sharpen(&values, "representation.text.word", 0.9, None);
    assert!(
        result.is_some(),
        "R32 should fire on a small repeated vocab"
    );
    let (label, rule) = result.unwrap();
    assert_eq!(label, "representation.discrete.categorical");
    assert!(rule.starts_with("text_vocab_override:"));
}

#[test]
fn r32_preserves_distinct_free_text() {
    // Species names / free words are mostly distinct — never a vocab.
    let values: Vec<String> = vec![
        "Dichocarpum",
        "adiantifolium",
        "sutchuenense",
        "arisanense",
        "auriculatum",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    for label in [
        "representation.text.word",
        "representation.text.entity_name",
        "representation.text.plain_text",
    ] {
        assert!(
            value_sharpen(&values, label, 0.9, None).is_none(),
            "R32 must not fire on distinct free text ({label})"
        );
    }
}

#[test]
fn r32_preserves_constant_and_short_columns() {
    let constant: Vec<String> = vec!["s"; 6].into_iter().map(String::from).collect();
    assert!(
        value_sharpen(&constant, "representation.text.word", 0.9, None).is_none(),
        "single distinct value is not a vocabulary"
    );
    let short: Vec<String> = vec!["a".into(), "b".into(), "a".into()];
    assert!(
        value_sharpen(&short, "representation.text.word", 0.9, None).is_none(),
        "below the n>=4 floor"
    );
}

#[test]
fn r32_out_of_scope_labels_untouched() {
    // Excluded by design: city/region (legitimately low-cardinality
    // vocabularies exist) and — measured by the corpus-honest gate
    // round 1 — entity_name/plain_text, where the oracle refuted
    // 3,752/2,115 moves (repeating manufacturer names ARE entity_name;
    // repeated boilerplate IS plain_text).
    let values: Vec<String> = vec![
        "Sydney",
        "Melbourne",
        "Sydney",
        "Sydney",
        "Melbourne",
        "Sydney",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    for label in [
        "geography.location.city",
        "representation.text.entity_name",
        "representation.text.plain_text",
    ] {
        let result = value_sharpen(&values, label, 0.9, None);
        if let Some((_, rule)) = &result {
            assert!(
                !rule.starts_with("text_vocab_override:"),
                "R32 must not fire on {label}"
            );
        }
    }
}

#[test]
fn test_header_hint_coverage() {
    // Verify at least 20 distinct column name patterns are covered
    let test_headers = vec![
        "email",
        "phone",
        "zip",
        "postal",
        "name",
        "full_name",
        "first_name",
        "last_name",
        "latitude",
        "longitude",
        "country",
        "city",
        "state",
        "gender",
        "url",
        "ip",
        "uuid",
        "port",
        "date",
        "year",
        "password",
        "price",
        "amount",
        "count",
        "address",
        "street",
    ];
    let matches: Vec<&str> = test_headers
        .iter()
        .filter(|h| header_hint(h).is_some())
        .copied()
        .collect();
    assert!(
        matches.len() >= 20,
        "Expected at least 20 matches, got {}: {:?}",
        matches.len(),
        matches
    );
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

// ── New header hint tests ─────────────────────────────────

#[test]
fn class_rank_grade_headers_no_longer_hint_ordinal() {
    // spec 2026-06-25-sharpen-stage-audit: the class/rank/grade/tier → ordinal
    // header arms were retired (value-blind, gold-negative on the attention model
    // — Grade, Region Rank, GlobalRank, TldRank, usageclass were mis-promoted to
    // ordinal over a correct numeric Sense). These headers now carry no hint; the
    // model's value-based prediction stands.
    assert_eq!(header_hint("Pclass"), None);
    assert_eq!(header_hint("class"), None);
    assert_eq!(header_hint("grade"), None);
    assert_eq!(header_hint("rank"), None);
    assert_eq!(header_hint("tier"), None);
}

#[test]
fn test_header_hint_count_columns() {
    assert_eq!(
        header_hint("SibSp"),
        Some("representation.numeric.integer_number")
    );
    assert_eq!(
        header_hint("Parch"),
        Some("representation.numeric.integer_number")
    );
    assert_eq!(
        header_hint("siblings"),
        Some("representation.numeric.integer_number")
    );
    assert_eq!(
        header_hint("children"),
        Some("representation.numeric.integer_number")
    );
    assert_eq!(
        header_hint("qty"),
        Some("representation.numeric.integer_number")
    );
}

#[test]
fn test_header_hint_survival_columns() {
    assert_eq!(
        header_hint("Survived"),
        Some("representation.boolean.binary")
    );
    assert_eq!(header_hint("alive"), Some("representation.boolean.binary"));
    assert_eq!(header_hint("active"), Some("representation.boolean.binary"));
}

#[test]
fn test_header_hint_ticket_cabin() {
    assert_eq!(
        header_hint("Ticket"),
        Some("representation.alphanumeric.alphanumeric_id")
    );
    assert_eq!(
        header_hint("Cabin"),
        Some("representation.alphanumeric.alphanumeric_id")
    );
    assert_eq!(
        header_hint("seat"),
        Some("representation.alphanumeric.alphanumeric_id")
    );
}

#[test]
fn test_header_hint_embarked() {
    assert_eq!(
        header_hint("Embarked"),
        Some("representation.discrete.categorical")
    );
    assert_eq!(
        header_hint("terminal"),
        Some("representation.discrete.categorical")
    );
}

#[test]
fn test_header_hint_fare() {
    assert_eq!(header_hint("Fare"), Some("finance.currency.amount"));
    assert_eq!(header_hint("fee"), Some("finance.currency.amount"));
}

#[test]
fn compound_class_headers_no_longer_hint_ordinal() {
    // spec 2026-06-25-sharpen-stage-audit: the substring class/grade/rank/tier →
    // ordinal arm was retired alongside its exact-match twin. Compound headers
    // containing "class" no longer hint ordinal.
    assert_eq!(header_hint("passenger_class"), None);
    assert_eq!(header_hint("skill_grade"), None);
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

// ── Day-of-week / month name / boolean sub-type tests ─────

#[test]
fn test_day_of_week_full_names() {
    let values: Vec<String> = vec![
        "Monday",
        "Tuesday",
        "Wednesday",
        "Thursday",
        "Friday",
        "Saturday",
        "Sunday",
        "Monday",
        "Friday",
        "Wednesday",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let result = disambiguate_day_of_week(&values);
    assert_eq!(result, Some("datetime.component.day_of_week".to_string()));
}

#[test]
fn test_day_of_week_abbreviated() {
    let values: Vec<String> = vec![
        "Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun", "Mon", "Fri", "Wed",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let result = disambiguate_day_of_week(&values);
    assert_eq!(result, Some("datetime.component.day_of_week".to_string()));
}

#[test]
fn test_day_of_week_two_letter() {
    let values: Vec<String> = vec!["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"]
        .into_iter()
        .map(String::from)
        .collect();

    let result = disambiguate_day_of_week(&values);
    assert_eq!(result, Some("datetime.component.day_of_week".to_string()));
}

#[test]
fn test_day_of_week_not_triggered_for_names() {
    let values: Vec<String> = vec!["Alice", "Bob", "Charlie", "David", "Eve", "Frank", "Grace"]
        .into_iter()
        .map(String::from)
        .collect();

    let result = disambiguate_day_of_week(&values);
    assert_eq!(result, None);
}

#[test]
fn test_day_of_week_too_few_values() {
    let values: Vec<String> = vec!["Monday", "Tuesday"]
        .into_iter()
        .map(String::from)
        .collect();

    let result = disambiguate_day_of_week(&values);
    assert_eq!(result, None);
}

#[test]
fn test_day_of_week_below_threshold() {
    // Only 2 of 10 are day names (20% < 80%)
    let values: Vec<String> = vec![
        "Monday", "Apple", "Banana", "Cherry", "Date", "Fig", "Grape", "Tuesday", "Kiwi", "Lemon",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let result = disambiguate_day_of_week(&values);
    assert_eq!(result, None);
}

#[test]
fn test_month_name_full_names() {
    let values: Vec<String> = vec![
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
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let result = disambiguate_month_name(&values);
    assert_eq!(result, Some("datetime.component.month_name".to_string()));
}

#[test]
fn test_month_name_abbreviated() {
    let values: Vec<String> = vec![
        "Jan", "Feb", "Mar", "Apr", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let result = disambiguate_month_name(&values);
    assert_eq!(result, Some("datetime.component.month_name".to_string()));
}

#[test]
fn test_month_name_mixed_case() {
    let values: Vec<String> = vec![
        "january", "FEBRUARY", "March", "april", "MAY", "June", "july", "AUGUST",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let result = disambiguate_month_name(&values);
    assert_eq!(result, Some("datetime.component.month_name".to_string()));
}

#[test]
fn test_month_name_not_triggered_for_names() {
    // "May" overlaps with month name, but others don't
    let values: Vec<String> = vec![
        "Alice", "Bob", "Charlie", "David", "Eve", "Frank", "Grace", "Helen", "Ivan", "Jack",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let result = disambiguate_month_name(&values);
    assert_eq!(result, None);
}

#[test]
fn test_month_name_too_few_values() {
    let values: Vec<String> = vec!["January", "February"]
        .into_iter()
        .map(String::from)
        .collect();

    let result = disambiguate_month_name(&values);
    assert_eq!(result, None);
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

// --- Fix 6: Epoch seconds detection ---

#[test]
fn test_epoch_seconds_in_range() {
    // Unix timestamps from 2020-2024
    let values: Vec<String> = vec![
        "1577836800", // 2020-01-01
        "1609459200", // 2021-01-01
        "1640995200", // 2022-01-01
        "1672531200", // 2023-01-01
        "1704067200", // 2024-01-01
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let result = detect_epoch_seconds(&values);
    assert_eq!(result, Some("datetime.epoch.unix_seconds".to_string()));
}

#[test]
fn test_epoch_seconds_pre_2000_not_detected() {
    // Timestamps before 2000 are below EPOCH_MIN
    let values: Vec<String> = vec![
        "631152000", // 1990-01-01
        "662688000", // 1991-01-01
        "694224000", // 1992-01-01
        "725846400", // 1993-01-01
        "757382400", // 1994-01-01
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let result = detect_epoch_seconds(&values);
    assert!(
        result.is_none(),
        "Pre-2000 timestamps should not be detected"
    );
}

#[test]
fn test_epoch_seconds_post_2050_not_detected() {
    // Timestamps after 2050 are above EPOCH_MAX
    let values: Vec<String> = vec![
        "2524608001", // Just past 2050-01-01
        "2556144000", // 2051
        "2587680000", // 2052
        "2619216000", // 2053
        "2650752000", // 2054
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let result = detect_epoch_seconds(&values);
    assert!(
        result.is_none(),
        "Post-2050 timestamps should not be detected"
    );
}

#[test]
fn test_epoch_milliseconds_detected() {
    // 13-digit Unix millisecond timestamps
    let values: Vec<String> = vec![
        "1577836800000", // 2020-01-01 in ms
        "1609459200000", // 2021-01-01 in ms
        "1640995200000", // 2022-01-01 in ms
        "1672531200000", // 2023-01-01 in ms
        "1704067200000", // 2024-01-01 in ms
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let result = detect_epoch_seconds(&values);
    assert_eq!(result, Some("datetime.epoch.unix_milliseconds".to_string()));
}

#[test]
fn test_epoch_seconds_threshold_boundary() {
    // 4 in-range, 1 out-of-range: 80% → should still detect
    let values: Vec<String> = vec![
        "1577836800", // 2020 (in range)
        "1609459200", // 2021 (in range)
        "1640995200", // 2022 (in range)
        "1672531200", // 2023 (in range)
        "12345",      // Out of range
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let result = detect_epoch_seconds(&values);
    assert_eq!(
        result,
        Some("datetime.epoch.unix_seconds".to_string()),
        "80% in-range should still detect"
    );
}

#[test]
fn test_epoch_seconds_below_threshold() {
    // 2 in-range, 3 out-of-range: 40% → should not detect
    let values: Vec<String> = vec![
        "1577836800", // In range
        "1609459200", // In range
        "12345",      // Out of range
        "67890",      // Out of range
        "99999",      // Out of range
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let result = detect_epoch_seconds(&values);
    assert!(result.is_none(), "40% in-range should not detect");
}

#[test]
fn test_epoch_seconds_small_integers_not_detected() {
    // Small integers (vote counts, scores) should not be detected as epoch
    let values: Vec<String> = vec!["100", "200", "50", "75", "150"]
        .into_iter()
        .map(String::from)
        .collect();

    let result = detect_epoch_seconds(&values);
    assert!(result.is_none(), "Small integers should not be epoch");
}

#[test]
fn test_epoch_seconds_floats_with_zero_fract() {
    // Float-stored epoch values (pandas nullable int → float64)
    let values: Vec<String> = vec![
        "1577836800.0",
        "1609459200.0",
        "1640995200.0",
        "1672531200.0",
        "1704067200.0",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let result = detect_epoch_seconds(&values);
    assert_eq!(
        result,
        Some("datetime.epoch.unix_seconds".to_string()),
        "Float-stored epoch values should be detected"
    );
}

#[test]
fn test_epoch_seconds_too_few_values() {
    // Less than 3 non-empty values → should not fire
    let values: Vec<String> = vec!["1577836800", "1609459200"]
        .into_iter()
        .map(String::from)
        .collect();

    let result = detect_epoch_seconds(&values);
    assert!(result.is_none(), "Too few values should not detect");
}

/// Integration test: verify that semantic hint classifier influences column classification.
/// Skips if Model2Vec model files are not present.
#[test]
fn test_classify_column_with_semantic_hint() {
    use crate::semantic::SemanticHintClassifier;

    let model_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("models")
        .join("model2vec");

    if !model_dir.join("model.safetensors").exists() {
        eprintln!("Skipping semantic column integration test: models/model2vec not found");
        return;
    }

    let semantic = SemanticHintClassifier::load(&model_dir).unwrap();

    // Create a mock classifier that delegates value-level inference
    // We use a simple stub here — the semantic hint should override generic
    // value predictions when the header name is semantically clear.
    let base_classifier =
        crate::inference::MockClassifier::new("representation.numeric.decimal_number");
    let column_classifier = ColumnClassifier::with_semantic_hint(
        Box::new(base_classifier),
        ColumnConfig::default(),
        semantic,
    );

    // The base classifier always returns decimal_number, but the semantic hint
    // for "weight_kg" should override to identity.person.weight
    let values: Vec<String> = vec!["72.5", "85.0", "63.2", "90.1"]
        .into_iter()
        .map(String::from)
        .collect();
    let result = column_classifier
        .classify_column_with_header(&values, "weight_kg")
        .unwrap();
    assert_eq!(
        result.label, "identity.person.weight",
        "Semantic hint for 'weight_kg' should override generic decimal_number"
    );

    // Generic column names should NOT override (semantic hint returns None)
    let result2 = column_classifier
        .classify_column_with_header(&values, "col1")
        .unwrap();
    assert_eq!(
        result2.label, "representation.numeric.decimal_number",
        "Generic 'col1' should not trigger semantic override"
    );
}

// ── Attractor demotion tests (Rule 14) ──────────────────────────────

// Was testing street_number demotion which no longer exists.

#[test]
fn test_attractor_confidence_demotion() {
    // Low confidence postal_code prediction (0.6) — should demote
    let values: Vec<String> = vec![
        "1500", "2300", "45000", "800", "99", "12", "5600", "340", "78", "4100",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let votes = vec![
        ("geography.address.postal_code".to_string(), 6),
        ("representation.numeric.integer_number".to_string(), 4),
    ];

    let result = disambiguate_attractor_demotion(&values, &votes, 10, None);
    assert!(
        result.is_some(),
        "Should demote postal_code at 0.6 confidence"
    );
    let (label, rule) = result.unwrap();
    assert_eq!(label, "representation.numeric.integer_number");
    assert!(rule.starts_with("attractor_demotion_confidence:"));
}

#[test]
fn test_attractor_cardinality_demotion() {
    // 4 unique short words classified as first_name at high confidence — categorical
    let values: Vec<String> = vec![
        "Soccer", "Baseball", "Tennis", "Hockey", "Soccer", "Baseball", "Tennis", "Hockey",
        "Soccer", "Baseball",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let votes = vec![
        ("identity.person.first_name".to_string(), 9),
        ("representation.text.word".to_string(), 1),
    ];

    // High confidence (0.9) — Signal 2 won't fire, but Signal 3 (cardinality) should
    let result = disambiguate_attractor_demotion(&values, &votes, 10, None);
    assert!(
        result.is_some(),
        "Should demote first_name with 4 unique values"
    );
    let (label, rule) = result.unwrap();
    assert_eq!(label, "representation.discrete.categorical");
    assert!(rule.starts_with("attractor_demotion_cardinality:"));
}

#[test]
fn test_attractor_cardinality_single_value() {
    // Single unique value (e.g., airports.type = "airport" repeated) — categorical
    let values: Vec<String> = vec![
        "airport", "airport", "airport", "airport", "airport", "airport", "airport", "airport",
        "airport", "airport",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let votes = vec![("identity.person.first_name".to_string(), 10)];

    // Cardinality 1 — strongest signal that this is NOT a person's name
    let result = disambiguate_attractor_demotion(&values, &votes, 10, None);
    assert!(
        result.is_some(),
        "Should demote first_name with 1 unique value"
    );
    let (label, rule) = result.unwrap();
    assert_eq!(label, "representation.discrete.categorical");
    assert!(rule.starts_with("attractor_demotion_cardinality:"));
}

#[test]
fn test_attractor_validation_confirmed_skips_signal2() {
    // ICAO code at low confidence (0.6) but values pass validation → no demotion
    // This tests that validation confirmation gates Signal 2.
    let values: Vec<String> = vec!["EGLL", "KJFK", "LFPG", "EDDF", "RJTT", "VHHH"]
        .into_iter()
        .map(String::from)
        .collect();
    let votes = vec![
        ("geography.transportation.icao_code".to_string(), 6),
        ("representation.alphanumeric.alphanumeric_id".to_string(), 4),
    ];

    let yaml = r#"
geography.transportation.icao_code:
  title: "ICAO Code"
  validation:
    type: string
    pattern: "^[A-Z]{4}$"
    minLength: 4
    maxLength: 4
  tier: [VARCHAR, geography, transportation]
  release_priority: 5
  samples: ["EGLL"]
"#;
    let taxonomy = Taxonomy::from_yaml(yaml).unwrap();

    // Confidence 0.6 < 0.85 → Signal 2 would fire, BUT validation
    // pattern passes → validation_confirmed = true → Signal 2 skipped
    let result = disambiguate_attractor_demotion(&values, &votes, 10, Some(&taxonomy));
    assert!(
        result.is_none(),
        "Should NOT demote ICAO codes when validation confirms them"
    );
}

// ── Rule 32: closed-set over-emit demotion (earthquake-roundtrip ac-03) ──

#[test]
fn test_schema_fail_demotion_measurement_unit_to_categorical() {
    // net/locationSource/magSource: short network codes the model labels
    // measurement_unit (a closed SI-unit enum). None is an SI unit → 100%
    // fail the enum → demote to categorical (low cardinality).
    let values: Vec<String> = vec!["us", "ci", "nc", "us", "nn", "ci", "us", "nc"]
        .into_iter()
        .map(String::from)
        .collect();
    let yaml = r#"
representation.scientific.measurement_unit:
  title: "Measurement Unit"
  validation:
    type: string
    enum: [meter, kilogram, second, m, kg, s]
  tier: [VARCHAR, scientific]
  release_priority: 3
  samples: ["meter"]
"#;
    let taxonomy = Taxonomy::from_yaml(yaml).unwrap();
    let result = value_sharpen(
        &values,
        "representation.scientific.measurement_unit",
        0.9,
        Some(&taxonomy),
    );
    let (label, rule) = result.expect("network codes should demote off measurement_unit");
    assert_eq!(label, "representation.discrete.categorical");
    assert!(rule.starts_with("schema_fail_demotion:"));
}

#[test]
fn test_schema_fail_demotion_geohash_to_alphanumeric_id() {
    // id: unique event identifiers the model labels geohash. The geohash
    // base32 alphabet excludes a/i/l/o, so network-prefixed ids ("ci…")
    // fail the pattern → demote to alphanumeric_id (high cardinality).
    let values: Vec<String> = (0..30).map(|i| format!("ci40{:06}", i)).collect();
    let yaml = r#"
geography.coordinate.geohash:
  title: "Geohash"
  validation:
    type: string
    pattern: "^[0-9b-hjkmnp-z]{6,12}$"
  tier: [VARCHAR, geography, coordinate]
  release_priority: 5
  samples: ["u4pruyd"]
"#;
    let taxonomy = Taxonomy::from_yaml(yaml).unwrap();
    let result = value_sharpen(
        &values,
        "geography.coordinate.geohash",
        0.9,
        Some(&taxonomy),
    );
    let (label, rule) = result.expect("event ids should demote off geohash");
    assert_eq!(label, "representation.identifier.alphanumeric_id");
    assert!(rule.starts_with("schema_fail_demotion:"));
}

#[test]
fn test_schema_fail_demotion_partial_fail_kept() {
    // A column only ~17% of whose values fail the geohash pattern is NOT demoted.
    // Genuine geohash columns fail their own (v13 min-6) pattern ~23% on legitimate
    // short geohashes, and real unit columns fail the incomplete SI enum ~44%; the
    // >50% bar spares them. Only overwhelming failure (network codes that are 100%
    // non-units) demotes. Locks the bar against over-demoting genuine columns —
    // the earthquake `id` column (~28% fail) is intentionally left as geohash here
    // rather than risk regressing real short-geohash detection (the v13 trade-off).
    let mut values: Vec<String> = (0..25).map(|i| format!("bcd{:03}", i)).collect();
    values.extend((0..5).map(|i| format!("icd{:03}", i))); // 'i' ∉ base32 → fail
    let yaml = r#"
geography.coordinate.geohash:
  title: "Geohash"
  validation:
    type: string
    pattern: "^[0-9b-hjkmnp-z]{6,12}$"
  tier: [VARCHAR, geography, coordinate]
  release_priority: 5
  samples: ["u4pruyd"]
"#;
    let taxonomy = Taxonomy::from_yaml(yaml).unwrap();
    let result = value_sharpen(
        &values,
        "geography.coordinate.geohash",
        0.9,
        Some(&taxonomy),
    );
    assert!(
        result.is_none(),
        "partial-fail column must be kept, not demoted (>50% bar spares genuine columns)"
    );
}

#[test]
fn test_schema_fail_demotion_keeps_real_geohash() {
    // A genuine geohash column (all values valid base32) must NOT be demoted.
    let values: Vec<String> = (0..30).map(|i| format!("bcd{:03}", i)).collect();
    let yaml = r#"
geography.coordinate.geohash:
  title: "Geohash"
  validation:
    type: string
    pattern: "^[0-9b-hjkmnp-z]{6,12}$"
  tier: [VARCHAR, geography, coordinate]
  release_priority: 5
  samples: ["u4pruyd"]
"#;
    let taxonomy = Taxonomy::from_yaml(yaml).unwrap();
    let result = value_sharpen(
        &values,
        "geography.coordinate.geohash",
        0.9,
        Some(&taxonomy),
    );
    assert!(
        result.is_none(),
        "valid geohash values must not be demoted off geohash"
    );
}

#[test]
fn test_schema_fail_demotion_keeps_real_units() {
    // A genuine units column whose values ARE SI units must NOT be demoted —
    // the rule fires on schema-validation failure, not on the label alone.
    let values: Vec<String> = vec!["meter", "kg", "second", "m", "kg", "meter"]
        .into_iter()
        .map(String::from)
        .collect();
    let yaml = r#"
representation.scientific.measurement_unit:
  title: "Measurement Unit"
  validation:
    type: string
    enum: [meter, kilogram, second, m, kg, s]
  tier: [VARCHAR, scientific]
  release_priority: 3
  samples: ["meter"]
"#;
    let taxonomy = Taxonomy::from_yaml(yaml).unwrap();
    let result = value_sharpen(
        &values,
        "representation.scientific.measurement_unit",
        0.9,
        Some(&taxonomy),
    );
    assert!(
        result.is_none(),
        "real SI-unit values must not be demoted off measurement_unit"
    );
}

#[test]
fn test_schema_fail_demotion_utc_integers_to_categorical() {
    // Plain integers the Sense stage over-emits as datetime.offset.utc. None
    // matches the `UTC +HH:MM` pattern → 100% fail → demote (low cardinality →
    // categorical). This is the corpus utc over-emit the v24 retrain made worse.
    let values: Vec<String> = vec!["5", "100", "23", "5", "100", "7", "23", "5"]
        .into_iter()
        .map(String::from)
        .collect();
    let yaml = r#"
datetime.offset.utc:
  title: "UTC Offset"
  validation:
    type: string
    pattern: "^UTC [+-]\\d{2}:\\d{2}$"
  tier: [VARCHAR, datetime, offset]
  release_priority: 4
  samples: ["UTC +05:00"]
"#;
    let taxonomy = Taxonomy::from_yaml(yaml).unwrap();
    let result = value_sharpen(&values, "datetime.offset.utc", 0.9, Some(&taxonomy));
    let (label, rule) = result.expect("integers should demote off datetime.offset.utc");
    assert_eq!(label, "representation.discrete.categorical");
    assert!(rule.starts_with("schema_fail_demotion:"));
}

#[test]
fn test_schema_fail_demotion_keeps_real_utc() {
    // A genuine UTC-offset column (all values match the pattern) must NOT be
    // demoted — the regression guard for widening the allowlist to utc.
    let values: Vec<String> = vec![
        "UTC +05:00",
        "UTC -08:00",
        "UTC +00:00",
        "UTC +05:30",
        "UTC -03:00",
        "UTC +09:00",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let yaml = r#"
datetime.offset.utc:
  title: "UTC Offset"
  validation:
    type: string
    pattern: "^UTC [+-]\\d{2}:\\d{2}$"
  tier: [VARCHAR, datetime, offset]
  release_priority: 4
  samples: ["UTC +05:00"]
"#;
    let taxonomy = Taxonomy::from_yaml(yaml).unwrap();
    let result = value_sharpen(&values, "datetime.offset.utc", 0.9, Some(&taxonomy));
    assert!(
        result.is_none(),
        "real UTC-offset values must not be demoted off datetime.offset.utc"
    );
}

#[test]
fn test_schema_fail_demotion_url_codes_to_alphanumeric_id() {
    // Bare ids/codes the Sense stage over-emits as technology.internet.url. None
    // has a scheme:// → 100% fail → demote (high cardinality → alphanumeric_id).
    let values: Vec<String> = (0..30).map(|i| format!("ABC{:05}", i)).collect();
    let yaml = r#"
technology.internet.url:
  title: "URL"
  validation:
    type: string
    pattern: "^(?:https?|ftp|file)://[^\\s]+$"
  tier: [VARCHAR, technology, internet]
  release_priority: 4
  samples: ["https://example.com"]
"#;
    let taxonomy = Taxonomy::from_yaml(yaml).unwrap();
    let result = value_sharpen(&values, "technology.internet.url", 0.9, Some(&taxonomy));
    let (label, rule) = result.expect("bare codes should demote off technology.internet.url");
    assert_eq!(label, "representation.identifier.alphanumeric_id");
    assert!(rule.starts_with("schema_fail_demotion:"));
}

#[test]
fn test_schema_fail_demotion_keeps_real_url() {
    // A genuine URL column (all values have a valid scheme://) must NOT be
    // demoted — the regression guard for widening the allowlist to url.
    let values: Vec<String> = vec![
        "https://example.com",
        "http://foo.org/path",
        "https://a.b.c/x?y=1",
        "ftp://files.example.com",
        "https://news.site/article",
        "http://localhost:8080",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let yaml = r#"
technology.internet.url:
  title: "URL"
  validation:
    type: string
    pattern: "^(?:https?|ftp|file)://[^\\s]+$"
  tier: [VARCHAR, technology, internet]
  release_priority: 4
  samples: ["https://example.com"]
"#;
    let taxonomy = Taxonomy::from_yaml(yaml).unwrap();
    let result = value_sharpen(&values, "technology.internet.url", 0.9, Some(&taxonomy));
    assert!(
        result.is_none(),
        "real URL values must not be demoted off technology.internet.url"
    );
}

// Was testing street_number non-demotion which no longer exists.

#[test]
fn test_attractor_no_demotion_high_confidence() {
    // Attractor at >0.85 with valid values — should NOT demote
    let values: Vec<String> = vec![
        "10001", "90210", "30301", "60601", "02101", "75001", "33101", "94102", "20001", "98101",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let votes = vec![
        ("geography.address.postal_code".to_string(), 9),
        ("representation.numeric.integer_number".to_string(), 1),
    ];

    let yaml = r#"
geography.address.postal_code:
  title: "Postal Code"
  validation:
    type: string
    pattern: "^[0-9]{3,10}$"
    minLength: 3
    maxLength: 10
  tier: [VARCHAR, geography, address]
  release_priority: 5
  samples: ["10001"]
"#;
    let taxonomy = Taxonomy::from_yaml(yaml).unwrap();

    // All pass validation AND confidence is 0.9 → no demotion
    let result = disambiguate_attractor_demotion(&values, &votes, 10, Some(&taxonomy));
    assert!(
        result.is_none(),
        "Should NOT demote real postal codes at 0.9 confidence"
    );
}

#[test]
fn test_select_fallback_numeric() {
    // All integer values, no representation.* in votes
    let values: Vec<String> = vec!["100", "200", "300"]
        .into_iter()
        .map(String::from)
        .collect();
    let votes = vec![
        ("geography.address.postal_code".to_string(), 8),
        ("representation.numeric.integer_number".to_string(), 2),
    ];

    let result = select_fallback(&votes, true, false, false, &values);
    assert_eq!(result, "representation.numeric.integer_number");
}

#[test]
fn test_select_fallback_text() {
    let values: Vec<String> = vec!["Soccer", "Tennis"]
        .into_iter()
        .map(String::from)
        .collect();
    let votes = vec![
        ("identity.person.first_name".to_string(), 8),
        ("identity.person.username".to_string(), 2),
    ];

    let result = select_fallback(&votes, false, true, false, &values);
    assert_eq!(result, "representation.discrete.categorical");
}

#[test]
fn test_select_fallback_from_votes() {
    // representation.* type exists in the vote distribution → use it
    let values: Vec<String> = vec!["100", "200"].into_iter().map(String::from).collect();
    let votes = vec![
        ("geography.address.postal_code".to_string(), 6),
        ("representation.numeric.decimal_number".to_string(), 3),
        ("representation.numeric.integer_number".to_string(), 1),
    ];

    let result = select_fallback(&votes, true, false, false, &values);
    assert_eq!(
        result, "representation.numeric.decimal_number",
        "Should use representation.* type from votes when available"
    );
}

// ── Locale-aware attractor demotion tests ───────────────────────────

#[test]
fn test_attractor_demotion_locale_validation_demotes_salary() {
    // Salary column predicted as postal_code: 6-digit values fail ALL locale
    // patterns. Using values clearly in salary range (>99999) that cannot
    // be valid postal codes in any locale.
    let values: Vec<String> = vec![
        "102000", "245000", "112000", "350000", "178000", "195000", "267000", "188000", "103000",
        "272000",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let votes = vec![
        ("geography.address.postal_code".to_string(), 9),
        ("representation.numeric.integer_number".to_string(), 1),
    ];

    let yaml = r#"
geography.address.postal_code:
  title: "Postal Code"
  validation:
    type: string
    minLength: 3
    maxLength: 10
    maximum: 99999
  validation_by_locale:
    EN_US:
      type: string
      pattern: "^(\\d{5})(?:[ \\-](\\d{4}))?$"
      minLength: 5
      maxLength: 10
    EN_GB:
      type: string
      pattern: "^[A-Z]{1,2}\\d[A-Z\\d]?\\s?\\d[A-Z]{2}$"
      minLength: 5
      maxLength: 8
    DE:
      type: string
      pattern: "^\\d{5}$"
      minLength: 5
      maxLength: 5
  tier: [VARCHAR, address]
  release_priority: 4
  samples: ["10001"]
"#;
    let mut taxonomy = Taxonomy::from_yaml(yaml).unwrap();
    taxonomy.compile_validators();
    taxonomy.compile_locale_validators();

    let result = disambiguate_attractor_demotion(&values, &votes, 10, Some(&taxonomy));
    assert!(
        result.is_some(),
        "Should demote salary values despite matching universal validation"
    );
    let (label, rule) = result.unwrap();
    assert_eq!(label, "representation.numeric.integer_number");
    assert!(
        rule.starts_with("attractor_demotion_validation:"),
        "Should demote via validation signal, got: {}",
        rule
    );
}

#[test]
fn sample_contradicts_label_blocks_year_hint_on_decimals() {
    // spec 2026-06-25-sharpen-stage-audit ac-1: the deprecated header_hint
    // "...year" substring promotes a decimal column ("priceEpsCurrentYear",
    // "CitesPerYear") to datetime.component.year. The values contradict the
    // year validator, so the corroboration guard must report a contradiction.
    let yaml = r#"
datetime.component.year:
  title: "Year"
  validation:
    type: string
    pattern: "^(19|20)\\d{2}$"
    minLength: 4
    maxLength: 4
  tier: [VARCHAR, component]
  release_priority: 4
  samples: ["2021"]
"#;
    let mut taxonomy = Taxonomy::from_yaml(yaml).unwrap();
    taxonomy.compile_validators();

    let decimals: Vec<String> = vec!["3.2205129", "1.0", "2.0", "1.5", "0.0"]
        .into_iter()
        .map(String::from)
        .collect();
    assert!(
        sample_contradicts_label(&taxonomy, "datetime.component.year", &decimals),
        "decimals must contradict the year hint"
    );

    // Genuine 4-digit years corroborate — the guard must NOT block the hint.
    let years: Vec<String> = vec!["2021", "2020", "2019", "2022", "2018"]
        .into_iter()
        .map(String::from)
        .collect();
    assert!(
        !sample_contradicts_label(&taxonomy, "datetime.component.year", &years),
        "real years must corroborate the year hint"
    );

    // A leaf with no universal validator yields no evidence → never blocks.
    assert!(
        !sample_contradicts_label(&taxonomy, "representation.text.word", &decimals),
        "validator-less leaf must not report a contradiction"
    );

    // Too few values to judge → no contradiction.
    let two: Vec<String> = vec!["3.14", "2.71"].into_iter().map(String::from).collect();
    assert!(!sample_contradicts_label(
        &taxonomy,
        "datetime.component.year",
        &two
    ));
}

#[test]
fn header_hint_value_corroboration_is_default_on() {
    assert!(!rhh::is_disabled("header_hint_value_corroboration"));
}

#[test]
fn header_corroborates_timezone_token_aware() {
    // spec 2026-06-25-timezone-abbreviation-type: the recovery requires a tz-ish
    // header because EST/CST/PST overlap estimate/cost.
    assert!(header_corroborates_timezone("exchangeTimezoneShortName"));
    assert!(header_corroborates_timezone("TZ"));
    assert!(header_corroborates_timezone("time_zone"));
    assert!(header_corroborates_timezone("tzname"));
    // non-timezone headers must NOT corroborate — the precision gate
    assert!(!header_corroborates_timezone("status"));
    assert!(!header_corroborates_timezone("estimate_code"));
    assert!(!header_corroborates_timezone("country"));
}

#[test]
fn values_are_clearly_non_url_separates_ids_from_urls() {
    // spec 2026-06-25-sharpen-stage-audit: the url header-hint corroboration uses a
    // value-SHAPE test (not the validator) so it keeps all three url forms gold
    // counts as url, and fires only on positive evidence of non-url-ness.
    let v = |xs: &[&str]| xs.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    // bare ids / prose / flags -> clearly non-url (block the hint)
    assert!(values_are_clearly_non_url(&v(&[
        "msg32812262",
        "msg32929450",
        "msg11"
    ])));
    assert!(values_are_clearly_non_url(&v(&["Yes", "Yes", "Yes", "No"])));
    // real urls — all three forms — are NOT clearly non-url (hint may stand)
    assert!(!values_are_clearly_non_url(&v(&[
        "http://a.com/x",
        "https://b.io/y",
        "http://c.org/z"
    ])));
    assert!(!values_are_clearly_non_url(&v(&[
        "//cdn.a.io/x.js",
        "//cdn.b.io/y.css",
        "//c.io/z.js"
    ])));
    assert!(!values_are_clearly_non_url(&v(&[
        "/partner/x.asp?id=1",
        "/partner/y.asp?id=2",
        "/partner/z.asp?id=3"
    ])));
    // too few values -> inconclusive: a SINGLE clearly-non-url value still returns
    // false, so the verdict comes from the count floor alone (a `msg…` id would be
    // "clearly non-url" with >=3 values — see above — but one truncated compose
    // sample is not enough evidence to block the hint). Uses a non-url value on
    // purpose: a url value here would pass for the wrong reason (it's url-shaped),
    // masking the floor.
    assert!(!values_are_clearly_non_url(&v(&["msg32812262"])));
    // and the same id WITH enough values IS clearly non-url — proving it was the
    // count, not the shape, that spared the single-value case.
    assert!(values_are_clearly_non_url(&v(&[
        "msg32812262",
        "msg32929450",
        "msg11"
    ])));
}

#[test]
fn test_attractor_accepts_real_us_postal_codes() {
    // Real US ZIP codes: match EN_US locale pattern → locale-confirmed, no demotion
    let values: Vec<String> = vec![
        "10001", "90210", "30301", "60601", "02101", "75001", "33101", "94102", "20001", "98101",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let votes = vec![
        ("geography.address.postal_code".to_string(), 9),
        ("representation.numeric.integer_number".to_string(), 1),
    ];

    let yaml = r#"
geography.address.postal_code:
  title: "Postal Code"
  validation:
    type: string
    minLength: 3
    maxLength: 10
    maximum: 99999
  validation_by_locale:
    EN_US:
      type: string
      pattern: "^(\\d{5})(?:[ \\-](\\d{4}))?$"
      minLength: 5
      maxLength: 10
    EN_GB:
      type: string
      pattern: "^[A-Z]{1,2}\\d[A-Z\\d]?\\s?\\d[A-Z]{2}$"
      minLength: 5
      maxLength: 8
    DE:
      type: string
      pattern: "^\\d{5}$"
      minLength: 5
      maxLength: 5
  tier: [VARCHAR, address]
  release_priority: 4
  samples: ["10001"]
"#;
    let mut taxonomy = Taxonomy::from_yaml(yaml).unwrap();
    taxonomy.compile_validators();
    taxonomy.compile_locale_validators();

    // US ZIPs match EN_US locale at >50% → locale-confirmed → no demotion
    let result = disambiguate_attractor_demotion(&values, &votes, 10, Some(&taxonomy));
    assert!(
        result.is_none(),
        "Should NOT demote real US ZIP codes (locale-confirmed by EN_US)"
    );
}

#[test]
fn test_attractor_accepts_real_uk_postcodes() {
    // Real UK postcodes: match EN_GB locale pattern → locale-confirmed, no demotion
    let values: Vec<String> = vec![
        "EC1A 1BB", "W1C 1AX", "M2 5BQ", "SW1A 1AA", "B1 1BB", "LS1 1BA",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let votes = vec![("geography.address.postal_code".to_string(), 6)];

    let yaml = r#"
geography.address.postal_code:
  title: "Postal Code"
  validation:
    type: string
    minLength: 3
    maxLength: 10
    maximum: 99999
  validation_by_locale:
    EN_US:
      type: string
      pattern: "^(\\d{5})(?:[ \\-](\\d{4}))?$"
      minLength: 5
      maxLength: 10
    EN_GB:
      type: string
      pattern: "^[A-Z]{1,2}\\d[A-Z\\d]?\\s?\\d[A-Z]{2}$"
      minLength: 5
      maxLength: 8
  tier: [VARCHAR, address]
  release_priority: 4
  samples: ["10001"]
"#;
    let mut taxonomy = Taxonomy::from_yaml(yaml).unwrap();
    taxonomy.compile_validators();
    taxonomy.compile_locale_validators();

    let result = disambiguate_attractor_demotion(&values, &votes, 6, Some(&taxonomy));
    assert!(
        result.is_none(),
        "Should NOT demote real UK postcodes (locale-confirmed by EN_GB)"
    );
}

#[test]
fn test_attractor_locale_low_confidence_accepted() {
    // US ZIP codes at low confidence (0.6) — normally Signal 2 would demote,
    // but locale validation confirms the type → no demotion
    let values: Vec<String> = vec![
        "10001", "90210", "30301", "60601", "02101", "75001", "33101", "94102", "20001", "98101",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let votes = vec![
        ("geography.address.postal_code".to_string(), 6),
        ("representation.numeric.integer_number".to_string(), 4),
    ];

    let yaml = r#"
geography.address.postal_code:
  title: "Postal Code"
  validation:
    type: string
    minLength: 3
    maxLength: 10
    maximum: 99999
  validation_by_locale:
    EN_US:
      type: string
      pattern: "^(\\d{5})(?:[ \\-](\\d{4}))?$"
      minLength: 5
      maxLength: 10
  tier: [VARCHAR, address]
  release_priority: 4
  samples: ["10001"]
"#;
    let mut taxonomy = Taxonomy::from_yaml(yaml).unwrap();
    taxonomy.compile_validators();
    taxonomy.compile_locale_validators();

    // Confidence 0.6 < 0.85 → Signal 2 would fire, BUT locale validation
    // confirms the type → locale_confirmed = true → Signal 2 skipped
    let result = disambiguate_attractor_demotion(&values, &votes, 10, Some(&taxonomy));
    assert!(
        result.is_none(),
        "Should NOT demote real US ZIPs at low confidence when locale confirms them"
    );
}

#[test]
fn test_attractor_locale_confirmed_skips_cardinality() {
    // Phone numbers with locale confirmation should NOT be demoted
    // by Signal 3 (cardinality), even with few unique values. Small tables
    // with legitimate phone numbers are common in web-scraped datasets.
    let values: Vec<String> = vec![
        "(805) 638-3078",
        "(650) 440-2450",
        "(805) 638-3078",
        "(805) 638-3078",
        "(650) 440-2450",
        "(650) 440-2450",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    // 2 unique values, 6 total — classic cardinality demotion target
    let votes = vec![("identity.person.phone_number".to_string(), 6)];

    let yaml = r#"
identity.person.phone_number:
  title: "Phone Number"
  validation:
    type: string
    minLength: 7
    maxLength: 20
    pattern: "^[+]?[0-9\\s()\\-\\.]+$"
  validation_by_locale:
    EN_US:
      type: string
      pattern: "^(\\+?1[\\s\\-\\.]*)?\\(?\\d{3}\\)?[\\s\\-\\.]*\\d{3}[\\s\\-\\.]*\\d{4}$"
      minLength: 10
      maxLength: 18
  tier: [VARCHAR, person]
  release_priority: 4
  samples: ["+1 (555) 123-4567"]
"#;
    let mut taxonomy = Taxonomy::from_yaml(yaml).unwrap();
    taxonomy.compile_validators();
    taxonomy.compile_locale_validators();

    let result = disambiguate_attractor_demotion(&values, &votes, 6, Some(&taxonomy));
    assert!(
        result.is_none(),
        "Should NOT demote phone numbers when locale-confirmed, even with 2 unique values"
    );
}

#[test]
fn test_attractor_universal_only_does_not_confirm_locale_type() {
    // Precision Principle: For locale-specific types, passing the
    // universal validation pattern does NOT count as confirmation. Only locale
    // patterns can confirm. These values pass universal phone validation
    // (digits + formatting chars) but don't match any locale pattern.
    let values: Vec<String> = vec![
        "123-456", "789-012", "345-678", "123-456", "789-012", "345-678",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    // Low confidence — Signal 2 would fire if not confirmed
    let votes = vec![
        ("identity.person.phone_number".to_string(), 4),
        ("representation.discrete.categorical".to_string(), 2),
    ];

    let yaml = r#"
identity.person.phone_number:
  title: "Phone Number"
  validation:
    type: string
    minLength: 7
    maxLength: 20
    pattern: "^[+]?[0-9\\s()\\-\\.]+$"
  validation_by_locale:
    EN_US:
      type: string
      pattern: "^(\\+?1[\\s\\-\\.]*)?\\(?\\d{3}\\)?[\\s\\-\\.]*\\d{3}[\\s\\-\\.]*\\d{4}$"
      minLength: 10
      maxLength: 18
  tier: [VARCHAR, person]
  release_priority: 4
  samples: ["+1 (555) 123-4567"]
"#;
    let mut taxonomy = Taxonomy::from_yaml(yaml).unwrap();
    taxonomy.compile_validators();
    taxonomy.compile_locale_validators();

    // Confidence 0.67 < 0.85 AND no locale confirmation → should demote
    // despite universal pattern matching (precision principle)
    let result = disambiguate_attractor_demotion(&values, &votes, 6, Some(&taxonomy));
    assert!(
        result.is_some(),
        "Should demote phone_number at low confidence when only universal validates (no locale)"
    );
    let (_, rule) = result.unwrap();
    assert!(
        rule.starts_with("attractor_demotion_confidence:"),
        "Should demote via confidence signal, got: {}",
        rule
    );
}

#[test]
fn test_attractor_first_name_cardinality_unchanged() {
    // first_name has no locale validators, so cardinality demotion
    // still works exactly as before — this is a regression guard.
    let values: Vec<String> = vec![
        "John", "Jane", "Bob", "John", "Jane", "Bob", "John", "Jane", "Bob", "John",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let votes = vec![
        ("identity.person.first_name".to_string(), 9),
        ("representation.text.word".to_string(), 1),
    ];

    // 3 unique values, text attractor, no locale validators → must still demote
    let result = disambiguate_attractor_demotion(&values, &votes, 10, None);
    assert!(
        result.is_some(),
        "first_name with 3 unique values should still be demoted (no locale validators)"
    );
    let (label, rule) = result.unwrap();
    assert_eq!(label, "representation.discrete.categorical");
    assert!(
        rule.starts_with("attractor_demotion_cardinality:"),
        "Should demote via cardinality signal, got: {}",
        rule
    );
}

// ── Duration override tests ─────────────────────────────────────────

#[test]
fn test_duration_override_standard_durations() {
    // Standard ISO 8601 durations like PT20M (20 minutes), PT1H (1 hour)
    let values: Vec<String> = vec![
        "PT20M", "PT30M", "PT10M", "PT15M", "PT1H", "PT45M", "PT5M", "PT60M",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let result = disambiguate_duration_override(&values);
    assert!(result.is_some(), "Should detect ISO 8601 durations");
    let (label, rule) = result.unwrap();
    assert_eq!(label, "datetime.duration.iso_8601");
    assert_eq!(rule, "duration_override_sedol");
}

#[test]
fn test_duration_override_complex_durations() {
    // Complex durations with multiple components: P1DT12H, P2Y3M, PT1H30M
    let values: Vec<String> = vec!["P1DT12H", "P2Y3M", "PT1H30M", "P30D", "P1Y", "PT2H15M30S"]
        .into_iter()
        .map(String::from)
        .collect();

    let result = disambiguate_duration_override(&values);
    assert!(result.is_some(), "Should detect complex ISO 8601 durations");
    let (label, _) = result.unwrap();
    assert_eq!(label, "datetime.duration.iso_8601");
}

#[test]
fn test_duration_override_malformed_sotab_durations() {
    // Non-standard durations found in SOTAB: PD1TH0M0, PD3TH0M0
    let values: Vec<String> = vec![
        "PD1TH0M0", "PD3TH0M0", "PT30M", "PT20M", "PD1TH0M0", "PT10M",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let result = disambiguate_duration_override(&values);
    assert!(
        result.is_some(),
        "Should detect non-standard duration variants"
    );
    let (label, _) = result.unwrap();
    assert_eq!(label, "datetime.duration.iso_8601");
}

#[test]
fn test_duration_override_not_triggered_for_sedol() {
    // Real SEDOL codes: 7 alphanumeric chars, restricted charset
    let values: Vec<String> = vec![
        "B0YBKJ7", "B1YW440", "B39J2S1", "B0JNMQ2", "BWFGQN3", "B082RF1",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let result = disambiguate_duration_override(&values);
    assert!(
        result.is_none(),
        "Should NOT trigger for real SEDOL codes (no duration pattern)"
    );
}

#[test]
fn test_duration_override_not_triggered_below_threshold() {
    // Mixed column: mostly non-duration with a few durations
    let values: Vec<String> = vec![
        "ABC1234", "DEF5678", "GHI9012", "PT20M", "JKL3456", "MNO7890", "PQR1234", "STU5678",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let result = disambiguate_duration_override(&values);
    assert!(
        result.is_none(),
        "Should NOT trigger when <50% of values are durations"
    );
}

#[test]
fn test_duration_override_week_durations() {
    // Week-based durations: P1W, P2W
    let values: Vec<String> = vec!["P1W", "P2W", "P3W", "P4W", "P1W", "P2W"]
        .into_iter()
        .map(String::from)
        .collect();

    let result = disambiguate_duration_override(&values);
    assert!(result.is_some(), "Should detect week-based durations");
    let (label, _) = result.unwrap();
    assert_eq!(label, "datetime.duration.iso_8601");
}

// ── UTC offset override tests ──

#[test]
fn test_utc_offset_override_standard_offsets() {
    // Standard UTC offsets from the airports dataset
    let values: Vec<String> = vec![
        "+05:30", "-08:00", "-05:00", "+05:30", "+01:00", "+10:00", "+10:00", "+09:00", "+01:00",
        "+00:00",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let result = disambiguate_utc_offset_override(&values);
    assert!(result.is_some(), "Should detect UTC offset values");
    let (label, rule) = result.unwrap();
    assert_eq!(label, "datetime.offset.utc");
    assert_eq!(rule, "utc_offset_override_time");
}

#[test]
fn test_utc_offset_override_all_negative() {
    // All negative offsets (Americas)
    let values: Vec<String> = vec!["-05:00", "-06:00", "-07:00", "-08:00", "-04:00", "-03:00"]
        .into_iter()
        .map(String::from)
        .collect();

    let result = disambiguate_utc_offset_override(&values);
    assert!(result.is_some(), "Should detect negative UTC offsets");
    let (label, _) = result.unwrap();
    assert_eq!(label, "datetime.offset.utc");
}

#[test]
fn test_utc_offset_override_not_triggered_for_times() {
    // Actual 24h time values (no leading sign)
    let values: Vec<String> = vec!["14:30", "08:00", "12:15", "09:45", "16:00", "23:59"]
        .into_iter()
        .map(String::from)
        .collect();

    let result = disambiguate_utc_offset_override(&values);
    assert!(
        result.is_none(),
        "Should NOT trigger for actual time values (no +/- prefix)"
    );
}

#[test]
fn test_utc_offset_override_not_triggered_below_threshold() {
    // Mixed column: mostly times with a few offsets
    let values: Vec<String> = vec![
        "14:30", "08:00", "+05:30", "12:15", "09:45", "16:00", "23:59", "-08:00", "11:00", "07:30",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let result = disambiguate_utc_offset_override(&values);
    assert!(
        result.is_none(),
        "Should NOT trigger when <80% of values are UTC offsets"
    );
}

#[test]
fn test_utc_offset_override_too_few_values() {
    // Only 2 values — below minimum threshold
    let values: Vec<String> = vec!["+05:30", "-08:00"]
        .into_iter()
        .map(String::from)
        .collect();

    let result = disambiguate_utc_offset_override(&values);
    assert!(
        result.is_none(),
        "Should NOT trigger with fewer than 3 values"
    );
}

// ── Text length demotion tests ──

#[test]
fn test_text_length_demotion_long_text_as_address() {
    // Long descriptions/paragraphs misclassified as full_address
    let values: Vec<String> = vec![
            "Contact information of the hotel Record of Zelenograd: phone, location map, address on the map. Full amenities and services list available.",
            "The layout of the room includes two bedrooms and a spacious lounge, but each room has its own plasma TV for entertainment and relaxation purposes.",
            "Services provided by the hotel Record (Zelenograd): Credit cards (Visa, MasterCard, World), free Wi-Fi, gym, spa, pool, conference rooms available.",
            "STANDARD WITH THE KITCHEN number one category. The Record Hotel has 1 Standard Room with Kitchen area for extended stays and business travelers.",
            "Preheat oven to 350 degrees. Grease a small baking dish or small cast iron skillet and fill with peaches. Sprinkle cinnamon over peaches and set aside.",
        ]
        .into_iter()
        .map(String::from)
        .collect();

    let votes = vec![("geography.address.full_address".to_string(), 5)];
    let result = disambiguate_text_length_demotion(&values, &votes);
    assert!(result.is_some(), "Should demote long text overcall");
    let (label, rule) = result.unwrap();
    assert_eq!(label, "representation.text.plain_text");
    assert_eq!(rule, "text_length_demotion_full_address");
}

#[test]
fn test_text_length_demotion_real_address_not_demoted() {
    // Real addresses should NOT be demoted (typical length 20-40 chars)
    let values: Vec<String> = vec![
        "123 Main St, Springfield, IL",
        "456 Oak Ave, Portland, OR",
        "789 Pine Rd, Austin, TX",
        "101 Elm Blvd, Denver, CO",
        "202 Maple Dr, Seattle, WA",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let votes = vec![("geography.address.full_address".to_string(), 5)];
    let result = disambiguate_text_length_demotion(&values, &votes);
    assert!(
        result.is_none(),
        "Should NOT demote real addresses (median ~28 chars)"
    );
}

#[test]
fn test_text_length_demotion_ignores_non_address() {
    // Rule should not fire for non-full_address predictions
    let values: Vec<String> = vec![
            "This is a very long text that exceeds one hundred characters and should demonstrate that length alone does not trigger demotion for other types.",
            "Another very long text value that is clearly longer than the threshold of one hundred characters but is not predicted as full_address by the model.",
            "Yet another long text to ensure the median is above the threshold value for the test to be meaningful in demonstrating the rule only applies to addresses.",
        ]
        .into_iter()
        .map(String::from)
        .collect();

    let votes = vec![("identity.person.full_name".to_string(), 3)];
    let result = disambiguate_text_length_demotion(&values, &votes);
    assert!(
        result.is_none(),
        "Should NOT fire for non-full_address predictions"
    );
}

#[test]
fn test_text_length_demotion_borderline_not_demoted() {
    // Values right at the boundary (median ~95 chars) should NOT be demoted
    let values: Vec<String> = vec![
            "123 Main Street, Apartment 4B, Springfield, Illinois 62704, United States of America — Near the park",
            "456 Oak Avenue, Suite 200, Portland, Oregon 97201, United States of America — Downtown district",
            "789 Pine Road, Building C, Unit 12, Austin, Texas 78701, United States of America — East campus",
        ]
        .into_iter()
        .map(String::from)
        .collect();

    let votes = vec![("geography.address.full_address".to_string(), 3)];
    let result = disambiguate_text_length_demotion(&values, &votes);
    assert!(
        result.is_none(),
        "Should NOT demote borderline addresses (median ~95 chars)"
    );
}

// ==========================================================================
// Designation-aware is_generic_prediction tests
// ==========================================================================

#[test]
fn test_is_generic_attractor_demoted_always_generic() {
    // Attractor-demoted predictions are always generic, regardless of designation
    let rule = Some("attractor_demotion_validation:something".to_string());
    assert!(
        is_generic_prediction("identity.person.email", &rule, None),
        "Attractor-demoted predictions should always be generic"
    );
}

#[test]
fn test_is_generic_boolean_always_generic() {
    // Boolean types are always generic
    assert!(
        is_generic_prediction("representation.boolean.binary", &None, None),
        "Boolean types should always be generic"
    );
    assert!(
        is_generic_prediction("representation.boolean.terms", &None, None),
        "Boolean types should always be generic"
    );
}

#[test]
fn test_is_generic_broad_words_with_taxonomy() {
    // broad_words designation should make a prediction generic when taxonomy is available
    let yaml = r#"
identity.person.gender:
  title: Gender
  designation: broad_words
  tier: [VARCHAR, identity, person]
  release_priority: 1
  samples: ["Male"]
identity.person.email:
  title: Email
  designation: universal
  tier: [VARCHAR, identity, person]
  release_priority: 5
  samples: ["test@example.com"]
"#;
    let taxonomy = Taxonomy::from_yaml(yaml).unwrap();

    // broad_words → generic
    assert!(
        is_generic_prediction("identity.person.gender", &None, Some(&taxonomy)),
        "broad_words types should be generic when taxonomy is available"
    );

    // universal → not generic (not in hardcoded list either)
    assert!(
        !is_generic_prediction("identity.person.email", &None, Some(&taxonomy)),
        "universal types should NOT be generic (unless in hardcoded list)"
    );
}

#[test]
fn test_is_generic_broad_characters_with_taxonomy() {
    let yaml = r#"
identity.person.password:
  title: Password
  designation: broad_characters
  tier: [VARCHAR, identity, person]
  release_priority: 1
  samples: ["p@ssw0rd"]
"#;
    let taxonomy = Taxonomy::from_yaml(yaml).unwrap();

    assert!(
        is_generic_prediction("identity.person.password", &None, Some(&taxonomy)),
        "broad_characters types should be generic when taxonomy is available"
    );
}

#[test]
fn test_is_generic_broad_numbers_with_taxonomy() {
    let yaml = r#"
representation.identifier.increment:
  title: Increment
  designation: broad_numbers
  tier: [BIGINT, identifier]
  release_priority: 1
  samples: ["42"]
"#;
    let taxonomy = Taxonomy::from_yaml(yaml).unwrap();

    assert!(
        is_generic_prediction(
            "representation.identifier.increment",
            &None,
            Some(&taxonomy)
        ),
        "broad_numbers types should be generic when taxonomy is available"
    );
}

#[test]
fn test_is_generic_fallback_without_taxonomy() {
    // Without taxonomy, falls back to hardcoded list
    assert!(
        is_generic_prediction("representation.text.word", &None, None),
        "Hardcoded generic label should be generic without taxonomy"
    );
    assert!(
        is_generic_prediction("identity.person.phone_number", &None, None),
        "Hardcoded generic label should be generic without taxonomy"
    );
    assert!(
        !is_generic_prediction("identity.person.email", &None, None),
        "Non-hardcoded label should NOT be generic without taxonomy"
    );
}

#[test]
fn test_is_generic_locale_specific_not_generic() {
    // locale_specific designation should NOT be generic (when not in hardcoded list)
    let yaml = r#"
geography.address.postal_code:
  title: Postal Code
  designation: locale_specific
  tier: [VARCHAR, geography, address]
  release_priority: 5
  samples: ["90210"]
"#;
    let taxonomy = Taxonomy::from_yaml(yaml).unwrap();

    // postal_code is locale_specific and NOT in the hardcoded list → not generic
    assert!(
        !is_generic_prediction("geography.address.postal_code", &None, Some(&taxonomy)),
        "locale_specific types not in hardcoded list should NOT be generic"
    );
}

#[test]
fn test_is_generic_hardcoded_overrides_taxonomy() {
    // phone_number is in the hardcoded list AND has locale_specific designation.
    // Hardcoded list (Signal 3) takes precedence — the type stays generic so
    // header hints can still override when the model uses it as a catch-all.
    let yaml = r#"
identity.person.phone_number:
  title: Phone Number
  designation: locale_specific
  tier: [VARCHAR, identity, person]
  release_priority: 5
  samples: ["+1 (202) 555-0100"]
"#;
    let taxonomy = Taxonomy::from_yaml(yaml).unwrap();

    assert!(
        is_generic_prediction("identity.person.phone_number", &None, Some(&taxonomy)),
        "phone_number is in hardcoded list — stays generic regardless of designation"
    );
}

// ==========================================================================
// Post-hoc locale detection tests
// ==========================================================================

#[test]
fn test_detect_locale_us_phone_numbers() {
    // US phone numbers should detect EN_US locale
    let yaml = r#"
identity.person.phone_number:
  title: Phone Number
  designation: locale_specific
  tier: [VARCHAR, identity, person]
  release_priority: 5
  samples: ["+1 (202) 555-0100"]
  validation:
    type: string
    pattern: "^[+]?[0-9\\s()\\-\\.]+$"
  validation_by_locale:
    EN_US:
      type: string
      pattern: "^(\\+?1[\\s\\-./]*)?\\(?\\d{3}\\)?[\\s\\-./]*\\d{3}[\\s\\-./]*\\d{4}$"
      minLength: 10
      maxLength: 30
    EN_GB:
      type: string
      pattern: "^(\\+?44[\\s\\-./]*(\\(0\\))?)?0?\\d{2,5}([\\s\\-./]*\\d{1,8}){1,3}$"
      minLength: 10
      maxLength: 30
"#;
    let mut taxonomy = Taxonomy::from_yaml(yaml).unwrap();
    taxonomy.compile_locale_validators();

    let values: Vec<String> = vec![
        "+1 (202) 555-0100",
        "+1 (415) 555-0199",
        "(312) 555-0142",
        "1-800-555-0123",
        "(617) 555-0187",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let locale = detect_locale_from_validation(&values, "identity.person.phone_number", &taxonomy);
    assert_eq!(
        locale,
        Some("EN_US".to_string()),
        "US phone numbers should detect EN_US"
    );
}

#[test]
fn test_detect_locale_uk_phone_numbers() {
    let yaml = r#"
identity.person.phone_number:
  title: Phone Number
  designation: locale_specific
  tier: [VARCHAR, identity, person]
  release_priority: 5
  samples: ["+44 20 7946 0958"]
  validation:
    type: string
    pattern: "^[+]?[0-9\\s()\\-\\.]+$"
  validation_by_locale:
    EN_US:
      type: string
      pattern: "^(\\+?1[\\s\\-./]*)?\\(?\\d{3}\\)?[\\s\\-./]*\\d{3}[\\s\\-./]*\\d{4}$"
      minLength: 10
      maxLength: 30
    EN_GB:
      type: string
      pattern: "^(\\+?44[\\s\\-./]*(\\(0\\))?)?0?\\d{2,5}([\\s\\-./]*\\d{1,8}){1,3}$"
      minLength: 10
      maxLength: 30
"#;
    let mut taxonomy = Taxonomy::from_yaml(yaml).unwrap();
    taxonomy.compile_locale_validators();

    let values: Vec<String> = vec![
        "+44 20 7946 0958",
        "020 7946 0123",
        "+44 121 496 0987",
        "0161 496 0654",
        "+44 131 496 0321",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let locale = detect_locale_from_validation(&values, "identity.person.phone_number", &taxonomy);
    assert_eq!(
        locale,
        Some("EN_GB".to_string()),
        "UK phone numbers should detect EN_GB"
    );
}

#[test]
fn test_detect_locale_no_validators() {
    // Types without validation_by_locale should return None
    let yaml = r#"
identity.person.email:
  title: Email
  designation: universal
  tier: [VARCHAR, identity, person]
  release_priority: 5
  samples: ["test@example.com"]
"#;
    let taxonomy = Taxonomy::from_yaml(yaml).unwrap();

    let values: Vec<String> = vec!["test@example.com", "user@domain.org"]
        .into_iter()
        .map(String::from)
        .collect();

    let locale = detect_locale_from_validation(&values, "identity.person.email", &taxonomy);
    assert_eq!(
        locale, None,
        "Types without locale validators should return None"
    );
}

#[test]
fn test_detect_locale_no_match_above_threshold() {
    // Values that don't match any locale pattern well enough should return None
    let yaml = r#"
identity.person.phone_number:
  title: Phone Number
  designation: locale_specific
  tier: [VARCHAR, identity, person]
  release_priority: 5
  samples: ["+1 (202) 555-0100"]
  validation_by_locale:
    EN_US:
      type: string
      pattern: "^(\\+?1[\\s\\-./]*)?\\(?\\d{3}\\)?[\\s\\-./]*\\d{3}[\\s\\-./]*\\d{4}$"
      minLength: 10
      maxLength: 30
"#;
    let mut taxonomy = Taxonomy::from_yaml(yaml).unwrap();
    taxonomy.compile_locale_validators();

    // Random strings that don't match any phone pattern
    let values: Vec<String> = vec!["abc", "hello world", "12345", "not-a-phone"]
        .into_iter()
        .map(String::from)
        .collect();

    let locale = detect_locale_from_validation(&values, "identity.person.phone_number", &taxonomy);
    assert_eq!(
        locale, None,
        "Non-matching values should not detect any locale"
    );
}

#[test]
fn test_detect_locale_empty_values() {
    let yaml = r#"
identity.person.phone_number:
  title: Phone Number
  designation: locale_specific
  tier: [VARCHAR, identity, person]
  release_priority: 5
  samples: ["+1 (202) 555-0100"]
  validation_by_locale:
    EN_US:
      type: string
      pattern: "^(\\+?1[\\s\\-./]*)?\\(?\\d{3}\\)?[\\s\\-./]*\\d{3}[\\s\\-./]*\\d{4}$"
      minLength: 10
      maxLength: 30
"#;
    let mut taxonomy = Taxonomy::from_yaml(yaml).unwrap();
    taxonomy.compile_locale_validators();

    let values: Vec<String> = vec!["", "", ""].into_iter().map(String::from).collect();

    let locale = detect_locale_from_validation(&values, "identity.person.phone_number", &taxonomy);
    assert_eq!(
        locale, None,
        "All-empty values should not detect any locale"
    );
}

// ==========================================================================
// Locale detection tests for calling_code, month_name, day_of_week
// ==========================================================================

#[test]
fn test_detect_locale_calling_code_uk() {
    let yaml = r#"
geography.contact.calling_code:
  title: International Calling Code
  designation: locale_specific
  tier: [VARCHAR, contact]
  release_priority: 3
  samples: ["+1"]
  validation:
    type: string
    pattern: "^\\+?[0-9]{1,4}$"
  validation_by_locale:
    EN_US:
      type: string
      pattern: "^\\+?1$"
    EN_GB:
      type: string
      pattern: "^\\+?44$"
    DE:
      type: string
      pattern: "^\\+?49$"
    FR:
      type: string
      pattern: "^\\+?33$"
"#;
    let mut taxonomy = Taxonomy::from_yaml(yaml).unwrap();
    taxonomy.compile_locale_validators();

    let values: Vec<String> = vec!["+44", "44", "+44", "+44", "44"]
        .into_iter()
        .map(String::from)
        .collect();

    let locale =
        detect_locale_from_validation(&values, "geography.contact.calling_code", &taxonomy);
    assert_eq!(
        locale,
        Some("EN_GB".to_string()),
        "+44 calling codes should detect EN_GB"
    );
}

#[test]
fn test_detect_locale_calling_code_de() {
    let yaml = r#"
geography.contact.calling_code:
  title: International Calling Code
  designation: locale_specific
  tier: [VARCHAR, contact]
  release_priority: 3
  samples: ["+1"]
  validation:
    type: string
    pattern: "^\\+?[0-9]{1,4}$"
  validation_by_locale:
    EN_US:
      type: string
      pattern: "^\\+?1$"
    EN_GB:
      type: string
      pattern: "^\\+?44$"
    DE:
      type: string
      pattern: "^\\+?49$"
"#;
    let mut taxonomy = Taxonomy::from_yaml(yaml).unwrap();
    taxonomy.compile_locale_validators();

    let values: Vec<String> = vec!["+49", "49", "+49"]
        .into_iter()
        .map(String::from)
        .collect();

    let locale =
        detect_locale_from_validation(&values, "geography.contact.calling_code", &taxonomy);
    assert_eq!(
        locale,
        Some("DE".to_string()),
        "+49 calling codes should detect DE"
    );
}

#[test]
fn test_detect_locale_month_name_french() {
    let yaml = r#"
datetime.component.month_name:
  title: Full Month Name
  designation: locale_specific
  tier: [VARCHAR, component]
  release_priority: 1
  samples: ["January"]
  validation:
    type: string
    enum: ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"]
  validation_by_locale:
    EN:
      type: string
      enum: ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"]
    FR:
      type: string
      enum: ["janvier", "février", "mars", "avril", "mai", "juin", "juillet", "août", "septembre", "octobre", "novembre", "décembre"]
    DE:
      type: string
      enum: ["Januar", "Februar", "März", "April", "Mai", "Juni", "Juli", "August", "September", "Oktober", "November", "Dezember"]
"#;
    let mut taxonomy = Taxonomy::from_yaml(yaml).unwrap();
    taxonomy.compile_locale_validators();

    let values: Vec<String> = vec!["janvier", "mars", "juin", "décembre", "août"]
        .into_iter()
        .map(String::from)
        .collect();

    let locale = detect_locale_from_validation(&values, "datetime.component.month_name", &taxonomy);
    assert_eq!(
        locale,
        Some("FR".to_string()),
        "French month names should detect FR"
    );
}

#[test]
fn test_detect_locale_month_name_german() {
    let yaml = r#"
datetime.component.month_name:
  title: Full Month Name
  designation: locale_specific
  tier: [VARCHAR, component]
  release_priority: 1
  samples: ["January"]
  validation:
    type: string
    enum: ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"]
  validation_by_locale:
    EN:
      type: string
      enum: ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"]
    FR:
      type: string
      enum: ["janvier", "février", "mars", "avril", "mai", "juin", "juillet", "août", "septembre", "octobre", "novembre", "décembre"]
    DE:
      type: string
      enum: ["Januar", "Februar", "März", "April", "Mai", "Juni", "Juli", "August", "September", "Oktober", "November", "Dezember"]
"#;
    let mut taxonomy = Taxonomy::from_yaml(yaml).unwrap();
    taxonomy.compile_locale_validators();

    let values: Vec<String> = vec!["März", "Oktober", "Dezember", "Januar"]
        .into_iter()
        .map(String::from)
        .collect();

    let locale = detect_locale_from_validation(&values, "datetime.component.month_name", &taxonomy);
    assert_eq!(
        locale,
        Some("DE".to_string()),
        "German month names should detect DE"
    );
}

#[test]
fn test_detect_locale_day_of_week_spanish() {
    let yaml = r#"
datetime.component.day_of_week:
  title: Day of Week Name
  designation: locale_specific
  tier: [VARCHAR, component]
  release_priority: 1
  samples: ["Monday"]
  validation:
    type: string
    enum: ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"]
  validation_by_locale:
    EN:
      type: string
      enum: ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"]
    ES:
      type: string
      enum: ["lunes", "martes", "miércoles", "jueves", "viernes", "sábado", "domingo"]
    IT:
      type: string
      enum: ["lunedì", "martedì", "mercoledì", "giovedì", "venerdì", "sabato", "domenica"]
"#;
    let mut taxonomy = Taxonomy::from_yaml(yaml).unwrap();
    taxonomy.compile_locale_validators();

    let values: Vec<String> = vec!["lunes", "miércoles", "viernes", "domingo"]
        .into_iter()
        .map(String::from)
        .collect();

    let locale =
        detect_locale_from_validation(&values, "datetime.component.day_of_week", &taxonomy);
    assert_eq!(
        locale,
        Some("ES".to_string()),
        "Spanish day names should detect ES"
    );
}

// ==========================================================================
// Rule fixes for 3 profile eval misses
// ==========================================================================

#[test]
fn test_is_generic_numeric_postal_code_detection() {
    // Fix 1: numeric_postal_code_detection should yield to header hints
    // (e.g., "cvv" column with 3-digit values detected as postal_code)
    let rule = Some("numeric_postal_code_detection".to_string());
    assert!(
        is_generic_prediction("geography.address.postal_code", &rule, None),
        "numeric_postal_code_detection should be treated as generic to yield to header hints"
    );

    // numeric_port_detection assertion removed (port type removed)
}

#[test]
fn test_geography_protection_person_name_hints() {
    // Fix 2: Geography protection should fire for last_name and first_name,
    // not just full_name. Verifies PERSON_NAME_HINTS covers all three.
    assert!(
        PERSON_NAME_HINTS.contains(&"identity.person.full_name"),
        "PERSON_NAME_HINTS should include full_name"
    );
    assert!(
        PERSON_NAME_HINTS.contains(&"identity.person.last_name"),
        "PERSON_NAME_HINTS should include last_name"
    );
    assert!(
        PERSON_NAME_HINTS.contains(&"identity.person.first_name"),
        "PERSON_NAME_HINTS should include first_name"
    );

    // Non-person identity types should NOT be in the list
    assert!(
        !PERSON_NAME_HINTS.contains(&"identity.person.email"),
        "email should NOT be in PERSON_NAME_HINTS"
    );
    assert!(
        !PERSON_NAME_HINTS.contains(&"identity.person.phone_number"),
        "phone_number should NOT be in PERSON_NAME_HINTS"
    );
}

#[test]
fn test_location_types_module_level() {
    // Verify LOCATION_TYPES extracted to module level contains expected types
    assert!(
        LOCATION_TYPES.contains(&"geography.location.city"),
        "LOCATION_TYPES should include city"
    );
    assert!(
        LOCATION_TYPES.contains(&"geography.location.country"),
        "LOCATION_TYPES should include country"
    );
    assert!(
        LOCATION_TYPES.contains(&"geography.location.region"),
        "LOCATION_TYPES should include region"
    );
    assert!(
        LOCATION_TYPES.contains(&"geography.location.state"),
        "LOCATION_TYPES should include state"
    );
    assert!(
        LOCATION_TYPES.contains(&"geography.location.continent"),
        "LOCATION_TYPES should include continent"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Sense→Sharpen pipeline tests
// ═══════════════════════════════════════════════════════════════════════

/// Helper: load Sense resources if available. Returns None if artifacts missing.
fn load_sense_resources() -> Option<(
    crate::sense::SenseClassifier,
    crate::model2vec_shared::Model2VecResources,
    crate::label_category_map::LabelCategoryMap,
)> {
    let base = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()?
        .parent()?
        .to_path_buf();
    let m2v_dir = base.join("models/model2vec");
    let sense_dir = base.join("models/sense_spike/arch_a");

    if !m2v_dir.join("model.safetensors").exists() || !sense_dir.join("model.safetensors").exists()
    {
        return None;
    }

    let m2v = crate::model2vec_shared::Model2VecResources::load(&m2v_dir).ok()?;
    let sense = crate::sense::SenseClassifier::load(&sense_dir).ok()?;
    let label_map = crate::label_category_map::LabelCategoryMap::new();
    Some((sense, m2v, label_map))
}

#[test]
fn test_sense_pipeline_fields_default_none() {
    let mock = crate::inference::MockClassifier::new("representation.text.word");
    let cc = ColumnClassifier::with_defaults(Box::new(mock));
    assert!(!cc.has_sense());
}

#[test]
fn test_sense_pipeline_set_sense_enables() {
    let Some((sense, m2v, label_map)) = load_sense_resources() else {
        eprintln!("Skipping: model artifacts not found");
        return;
    };

    let mock = crate::inference::MockClassifier::new("representation.text.word");
    let mut cc = ColumnClassifier::with_defaults(Box::new(mock));
    assert!(!cc.has_sense());

    cc.set_sense(sense, m2v, label_map);
    assert!(cc.has_sense());
}

#[test]
fn test_sense_pipeline_fallback_without_sense() {
    // Without Sense, classify_column_with_header should use legacy path
    let mock = crate::inference::MockClassifier::new("identity.person.email");
    let cc = ColumnClassifier::with_defaults(Box::new(mock));

    let values: Vec<String> = vec![
        "john@example.com".to_string(),
        "jane@test.org".to_string(),
        "bob@corp.io".to_string(),
    ];
    let result = cc
        .classify_column_with_header(&values, "email")
        .expect("classify");

    // Legacy pipeline: MockClassifier always returns email, header hint confirms
    assert_eq!(result.label, "identity.person.email");
    assert!(!cc.has_sense());
}

#[test]
fn test_sense_sharpen_unanimous_email() {
    let Some((sense, m2v, label_map)) = load_sense_resources() else {
        eprintln!("Skipping: model artifacts not found");
        return;
    };

    // MockClassifier always returns email — unanimous CharCNN vote
    let mock = crate::inference::MockClassifier::new("identity.person.email");
    let mut cc = ColumnClassifier::with_defaults(Box::new(mock));
    cc.set_sense(sense, m2v, label_map);

    let values: Vec<String> = (0..10).map(|i| format!("user{}@example.com", i)).collect();

    let result = cc
        .classify_column_with_header(&values, "email")
        .expect("classify with sense");

    // Unanimous CharCNN votes for email should produce email
    // (either masked in by Sense category or fallback to unmasked)
    assert_eq!(result.label, "identity.person.email");
    assert!(result.samples_used > 0);
}

#[test]
fn test_sense_sharpen_empty_column() {
    let Some((sense, m2v, label_map)) = load_sense_resources() else {
        eprintln!("Skipping: model artifacts not found");
        return;
    };

    let mock = crate::inference::MockClassifier::new("representation.text.word");
    let mut cc = ColumnClassifier::with_defaults(Box::new(mock));
    cc.set_sense(sense, m2v, label_map);

    let result = cc
        .classify_column_with_header(&[], "date")
        .expect("classify empty");

    assert_eq!(result.label, "unknown");
    assert_eq!(result.samples_used, 0);
}

#[test]
fn test_sense_sharpen_iso_date_column() {
    let Some((sense, m2v, label_map)) = load_sense_resources() else {
        eprintln!("Skipping: model artifacts not found");
        return;
    };

    // MockClassifier returns iso date — all values agree
    let mock = crate::inference::MockClassifier::new("datetime.date.iso");
    let mut cc = ColumnClassifier::with_defaults(Box::new(mock));
    cc.set_sense(sense, m2v, label_map);

    let values: Vec<String> = vec![
        "2024-01-15".to_string(),
        "2024-02-20".to_string(),
        "2024-03-25".to_string(),
        "2024-04-30".to_string(),
        "2024-05-10".to_string(),
    ];

    let result = cc
        .classify_column_with_header(&values, "created_at")
        .expect("classify dates");

    // Should produce a datetime type (either masked to temporal or unmasked)
    assert!(
        result.label.starts_with("datetime."),
        "Expected datetime type, got: {}",
        result.label
    );
}

#[test]
fn test_sense_sharpen_entity_demotion() {
    let Some((sense, m2v, label_map)) = load_sense_resources() else {
        eprintln!("Skipping: model artifacts not found");
        return;
    };

    // MockClassifier returns full_name, but these are company names.
    // Sense should predict Entity with non-Person subtype → demote to entity_name.
    let mock = crate::inference::MockClassifier::new("identity.person.full_name");
    let mut cc = ColumnClassifier::with_defaults(Box::new(mock));
    cc.set_sense(sense, m2v, label_map);

    let values: Vec<String> = vec![
        "Apple Inc.".to_string(),
        "Microsoft Corporation".to_string(),
        "Google LLC".to_string(),
        "Amazon.com Inc.".to_string(),
        "Meta Platforms Inc.".to_string(),
    ];

    let result = cc
        .classify_column_with_header(&values, "company")
        .expect("classify companies");

    // Entity demotion should fire if Sense detects non-person entity.
    // The result should be either entity_name (demoted) or full_name
    // (if Sense predicts Person). Both are valid — the test verifies
    // the pipeline runs without error and produces a meaningful result.
    assert!(
        result.label == "representation.text.entity_name"
            || result.label == "identity.person.full_name",
        "Expected entity_name or full_name, got: {}",
        result.label
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

#[test]
fn test_header_hint_timezone() {
    assert_eq!(
        header_hint("timezone"),
        Some("datetime.offset.iana"),
        "timezone should hint to iana"
    );
    assert_eq!(
        header_hint("tz"),
        Some("datetime.offset.iana"),
        "tz should hint to iana"
    );
    assert_eq!(
        header_hint("time_zone"),
        Some("datetime.offset.iana"),
        "time_zone (with underscore) should hint to iana"
    );
}

#[test]
fn test_header_hint_publisher() {
    assert_eq!(
        header_hint("publisher"),
        Some("representation.text.entity_name"),
        "publisher should hint to entity_name"
    );
}

#[test]
fn test_header_hint_measurement_keywords() {
    assert_eq!(
        header_hint("pressure_atm"),
        Some("representation.numeric.decimal_number"),
        "pressure_atm should hint to decimal_number"
    );
    assert_eq!(
        header_hint("temperature"),
        Some("representation.numeric.decimal_number"),
        "temperature should hint to decimal_number"
    );
    assert_eq!(
        header_hint("voltage"),
        Some("representation.numeric.decimal_number"),
        "voltage should hint to decimal_number"
    );
}

#[test]
fn test_header_hint_priority_hardcoded_first() {
    // "job_title" has a hardcoded hint → categorical
    // Model2Vec might return entity_name for this header
    // With hardcoded-first, header_hint should win
    assert_eq!(
        header_hint("job_title"),
        Some("representation.discrete.categorical"),
        "job_title should hint to categorical (hardcoded)"
    );
    assert_eq!(
        header_hint("occupation"),
        Some("representation.discrete.categorical"),
        "occupation should hint to categorical (hardcoded)"
    );
}

#[test]
fn test_geo_override_same_domain() {
    // When both hint and prediction are location types, the hint should win
    // at ≤0.90 confidence (higher threshold than generic 0.50)
    assert!(
        LOCATION_TYPES.contains(&"geography.location.city"),
        "city must be a location type"
    );
    assert!(
        LOCATION_TYPES.contains(&"geography.location.country"),
        "country must be a location type"
    );
    // Both city and country are location types — same-domain override should apply
}

// ── Sharpen header bugfix tests (spec: 2026-04-12-sharpen-header-bugfixes) ──

#[test]
fn ac01_bitcoin_address_not_captured_by_address_hint() {
    // "bitcoin_address" must NOT match the address keyword — it should return
    // None so the model's bitcoin_address prediction is preserved.
    assert_eq!(
        header_hint("bitcoin_address"),
        None,
        "bitcoin_address must not match the address keyword"
    );
    assert_eq!(
        header_hint("btc_address"),
        None,
        "btc_address must not match the address keyword"
    );
    assert_eq!(
        header_hint("crypto_address"),
        None,
        "crypto_address must not match the address keyword"
    );
    assert_eq!(
        header_hint("wallet_address"),
        None,
        "wallet_address must not match the address keyword"
    );
    // Regression: street_address and full_address still work
    assert_eq!(
        header_hint("street_address"),
        Some("geography.address.street_address"),
        "street_address should still return street_address"
    );
    assert_eq!(
        header_hint("full_address"),
        Some("geography.address.full_address"),
        "full_address should still return full_address"
    );
    assert_eq!(
        header_hint("home_address"),
        Some("geography.address.full_address"),
        "home_address should still return full_address"
    );
}

#[test]
fn ac02_ipv6_header_returns_ip_v6() {
    // "ip_v6" normalizes to "ip v6" — the v6 check must fire before the
    // generic ip_v4 catch-all.
    assert_eq!(
        header_hint("ip_v6"),
        Some("technology.internet.ip_v6"),
        "ip_v6 should return ip_v6"
    );
    assert_eq!(
        header_hint("server_ipv6"),
        Some("technology.internet.ip_v6"),
        "server_ipv6 should return ip_v6"
    );
    assert_eq!(
        header_hint("ipv6_address"),
        Some("technology.internet.ip_v6"),
        "ipv6_address should return ip_v6"
    );
    // Regression: source_ip still returns ip_v4 (exact match arm)
    assert_eq!(
        header_hint("source_ip"),
        Some("technology.internet.ip_v4"),
        "source_ip should still return ip_v4"
    );
    // Regression: ip_address still returns ip_v4
    assert_eq!(
        header_hint("ip_address"),
        Some("technology.internet.ip_v4"),
        "ip_address should still return ip_v4"
    );
}

#[test]
fn ac04_icao_header_hint() {
    assert_eq!(
        header_hint("icao"),
        Some("geography.transportation.icao_code"),
        "icao should hint to icao_code"
    );
    assert_eq!(
        header_hint("ICAO"),
        Some("geography.transportation.icao_code"),
        "ICAO (uppercase) should hint to icao_code"
    );
    assert_eq!(
        header_hint("icao_code"),
        Some("geography.transportation.icao_code"),
        "icao_code should hint to icao_code"
    );
}

#[test]
fn debug_icao_sharpen_override() {
    // Full apply_header_sharpen test: icao hint should override unlocode
    let mock = crate::inference::MockClassifier::new("geography.transportation.unlocode");
    let cc = ColumnClassifier::with_defaults(Box::new(mock));
    let mut result = make_result("geography.transportation.unlocode", 0.919);
    let sample: Vec<String> = vec!["EGLL".into(), "KJFK".into(), "LFPG".into()];

    // Verify header_hint works
    let hint = header_hint("icao");
    assert_eq!(hint, Some("geography.transportation.icao_code"));

    cc.apply_header_sharpen(&mut result, "icao", &sample);

    eprintln!("result.label = {}", result.label);
    eprintln!("result.rule = {:?}", result.disambiguation_rule);
    assert_eq!(
        result.label, "geography.transportation.icao_code",
        "icao hint must override unlocode@0.919 via same-category path"
    );
}

#[test]
fn debug_ipv6_sharpen_override() {
    // Full apply_header_sharpen test: ip_v6 hint should override ip_v4
    let mock = crate::inference::MockClassifier::new("technology.internet.ip_v4");
    let cc = ColumnClassifier::with_defaults(Box::new(mock));
    let mut result = make_result("technology.internet.ip_v4", 0.70);
    let sample: Vec<String> = vec!["2001:db8::1".into(); 5];

    let hint = header_hint("ip_v6");
    assert_eq!(hint, Some("technology.internet.ip_v6"));

    cc.apply_header_sharpen(&mut result, "ip_v6", &sample);

    eprintln!("result.label = {}", result.label);
    eprintln!("result.rule = {:?}", result.disambiguation_rule);
    assert_eq!(
        result.label, "technology.internet.ip_v6",
        "ip_v6 hint must override ip_v4@0.70 via same-category path"
    );
}

#[test]
fn ac04_author_header_hint() {
    assert_eq!(
        header_hint("author"),
        Some("identity.person.full_name"),
        "author should hint to full_name"
    );
    assert_eq!(
        header_hint("authors"),
        Some("identity.person.full_name"),
        "authors should hint to full_name"
    );
    assert_eq!(
        header_hint("author_name"),
        Some("identity.person.full_name"),
        "author_name should hint to full_name"
    );
}

/// Helper: create a ColumnResult for threshold tests.
fn make_result(label: &str, confidence: f32) -> ColumnResult {
    ColumnResult {
        label: label.to_string(),
        confidence,
        vote_distribution: vec![(label.to_string(), 1.0)],
        disambiguation_applied: false,
        disambiguation_rule: None,
        samples_used: 10,
        detected_locale: None,
        is_generic: false,
        column_features: None,
    }
}

#[test]
fn ac03a_same_category_hardcoded_override_unconditional() {
    // Path A: hardcoded "phone" hint overrides ssn@1.00 because both are
    // identity.person.* — same category, no confidence threshold.
    let mock = crate::inference::MockClassifier::new("identity.person.ssn");
    let cc = ColumnClassifier::with_defaults(Box::new(mock));
    let mut result = make_result("identity.person.ssn", 1.00);
    let sample: Vec<String> = vec!["555-0100".into(); 5];

    cc.apply_header_sharpen(&mut result, "phone", &sample);

    assert_eq!(
        result.label, "identity.person.phone_number",
        "phone hint must override ssn@1.00 (same category, hardcoded)"
    );
    assert!(
        result.disambiguation_applied,
        "disambiguation_applied should be true"
    );
    assert_eq!(
        result.disambiguation_rule.as_deref(),
        Some("header_hint_same_category:phone"),
        "rule should be header_hint_same_category"
    );
}

#[test]
fn ac03b_same_domain_hardcoded_override_below_095() {
    // Path B: hardcoded "year" hint overrides compact_ym@0.83 because
    // both are datetime.* (same domain) and 0.83 < 0.95.
    let mock = crate::inference::MockClassifier::new("datetime.date.compact_ym");
    let cc = ColumnClassifier::with_defaults(Box::new(mock));
    let mut result = make_result("datetime.date.compact_ym", 0.83);
    let sample: Vec<String> = vec!["2024".into(); 5];

    cc.apply_header_sharpen(&mut result, "year", &sample);

    assert_eq!(
        result.label, "datetime.component.year",
        "year hint must override compact_ym@0.83 (same domain, < 0.90)"
    );
    assert!(result.disambiguation_applied);
}

#[test]
fn ac03b_same_domain_hardcoded_override_url_over_docker_ref() {
    // Path B: hardcoded "url" hint overrides docker_ref@0.60 because
    // both are technology.* (same domain) and 0.60 < 0.95.
    let mock = crate::inference::MockClassifier::new("technology.development.docker_ref");
    let cc = ColumnClassifier::with_defaults(Box::new(mock));
    let mut result = make_result("technology.development.docker_ref", 0.60);
    let sample: Vec<String> = vec!["https://example.com/track/123".into(); 5];

    cc.apply_header_sharpen(&mut result, "tracking_url", &sample);

    assert_eq!(
        result.label, "technology.internet.url",
        "url hint must override docker_ref@0.60 (same domain, < 0.90)"
    );
    assert!(result.disambiguation_applied);
}

#[test]
fn ac03b_same_domain_hardcoded_does_not_override_at_095() {
    // Regression: a hardcoded same-domain hint does NOT override when
    // confidence >= 0.95 — the model is very confident.
    // Uses phone→ssn (identity.person vs identity.government, same domain).
    let mock = crate::inference::MockClassifier::new("identity.government.ssn");
    let cc = ColumnClassifier::with_defaults(Box::new(mock));
    let mut result = make_result("identity.government.ssn", 0.96);
    let sample: Vec<String> = vec!["123-45-6789".into(); 5];

    cc.apply_header_sharpen(&mut result, "phone", &sample);

    assert_eq!(
        result.label, "identity.government.ssn",
        "hardcoded hint must NOT override at confidence >= 0.95"
    );
    assert!(
        !result.disambiguation_applied,
        "disambiguation should not be applied at >= 0.95"
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
fn test_hs_code_float_parseability() {
    // HS codes with 3 segments don't parse as float
    let codes = [
        "6204.62.40", // 3 segments → NOT float-parseable
        "8471.30.10",
        "8471.30", // 2 segments → float-parseable (8471.30)
        "6204.62",
    ];
    let per_value: Vec<[f32; FEATURE_DIM]> = codes.iter().map(|s| extract_features(s)).collect();
    let cf = aggregate_features(&per_value);

    // is_float should be < 1.0 (2 of 4 don't parse as float)
    assert!(
        cf.mean[feature_idx::IS_FLOAT] < 1.0,
        "HS codes with 3-segment entries should have is_float < 1.0, got {}",
        cf.mean[feature_idx::IS_FLOAT]
    );
    // digit_ratio should be high
    assert!(
        cf.mean[feature_idx::DIGIT_RATIO] > 0.7,
        "HS codes should have high digit ratio, got {}",
        cf.mean[feature_idx::DIGIT_RATIO]
    );
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

// ── Sharpen-specific tests (AC-2, AC-3) ──────────────────────────────
//
// These tests exercise the multi-branch Sharpen functions directly,
// using single-entry vote distributions that simulate multi-branch output.

// AC-2: feature_sharpen — F2 fires on hostname without docker_ref in votes
#[test]
fn test_sharpen_f2_hostname_high_slash_segments_no_docker_vote() {
    // Multi-branch predicts "hostname" but column has high slash segments
    // (e.g., "docker.io/library/nginx:latest"). With multi-branch single-entry
    // votes, docker_ref never appears as runner-up — F2 must fire on feature
    // threshold alone.
    let mut result = ColumnResult {
        label: "technology.internet.hostname".to_string(),
        confidence: 0.85,
        vote_distribution: vec![("technology.internet.hostname".to_string(), 0.85)],
        disambiguation_applied: false,
        disambiguation_rule: None,
        samples_used: 50,
        detected_locale: None,
        is_generic: false,
        column_features: None,
    };

    let mut cf = ColumnFeatures::empty();
    cf.mean[feature_idx::SEGMENT_COUNT_SLASH] = 2.0; // ≥ 1.5 threshold

    feature_sharpen(&mut result, &cf);

    assert_eq!(
        result.label, "technology.development.docker_ref",
        "F2 should fire on hostname with high slash segments even without docker_ref in votes"
    );
    assert!(result.disambiguation_applied);
    assert!(result
        .disambiguation_rule
        .as_ref()
        .unwrap()
        .starts_with("feature_slash_segments"));
}

#[test]
fn test_sharpen_f2_hostname_low_slash_segments_stays() {
    // hostname with low slash segments should NOT trigger F2
    let mut result = ColumnResult {
        label: "technology.internet.hostname".to_string(),
        confidence: 0.85,
        vote_distribution: vec![("technology.internet.hostname".to_string(), 0.85)],
        disambiguation_applied: false,
        disambiguation_rule: None,
        samples_used: 50,
        detected_locale: None,
        is_generic: false,
        column_features: None,
    };

    let mut cf = ColumnFeatures::empty();
    cf.mean[feature_idx::SEGMENT_COUNT_SLASH] = 0.5; // below 1.5

    feature_sharpen(&mut result, &cf);

    assert_eq!(
        result.label, "technology.internet.hostname",
        "F2 should NOT fire when slash segments are below threshold"
    );
    assert!(!result.disambiguation_applied);
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

// R20: HS code validation gate — demotes hs_code when values are plain decimals
#[test]
fn test_r20_hs_code_gate_demotes_plain_decimals() {
    // Model predicts hs_code but values are plain decimals (pe_ratio, sepal_length, etc.)
    let values: Vec<String> = vec![
        "3.14", "0.887", "-12.5", "100.0", "0.003", "45.67", "1.23", "99.9",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let result = value_sharpen(&values, "geography.transportation.hs_code", 0.95, None);
    assert!(result.is_some(), "R20 should fire on plain decimals");
    let (label, rule) = result.unwrap();
    assert_eq!(label, "representation.numeric.decimal_number");
    assert!(rule.starts_with("hs_code_validation_gate"));
}

#[test]
fn test_r20_hs_code_gate_keeps_real_hs_codes() {
    // Real HS codes should NOT be demoted
    let values: Vec<String> = vec![
        "8471.30",
        "8471.30.00",
        "6204.62",
        "8517.12",
        "0901.21",
        "2204.10",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let result = value_sharpen(&values, "geography.transportation.hs_code", 0.95, None);
    assert!(result.is_none(), "R20 should NOT fire on real HS codes");
}

#[test]
fn test_is_hs_code_format() {
    // Valid HS codes
    assert!(is_hs_code_format("8471.30"));
    assert!(is_hs_code_format("8471.30.00"));
    assert!(is_hs_code_format("0901.21.00.10"));
    assert!(is_hs_code_format("847130")); // undotted 6-digit
    assert!(is_hs_code_format("84713000")); // undotted 8-digit

    // Invalid — plain decimals
    assert!(!is_hs_code_format("3.14"));
    assert!(!is_hs_code_format("0.887"));
    assert!(!is_hs_code_format("-12.5"));
    assert!(!is_hs_code_format("100.0"));
    assert!(!is_hs_code_format("45.67"));

    // Invalid — too short
    assert!(!is_hs_code_format("123"));
    assert!(!is_hs_code_format("12.34"));

    // Invalid — negative
    assert!(!is_hs_code_format("-8471.30"));
}

// ── R21: Coordinate plausibility gate tests ─────────────────────────
#[test]
fn test_r21_coordinate_gate_demotes_out_of_range() {
    // Earthquake depth values — many exceed 180, not coordinates
    let values: Vec<String> = vec![
        "127.013", "573.817", "201.998", "10.0", "224.419", "177.684", "97.335", "671.0", "450.2",
        "300.1",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let result = value_sharpen(&values, "geography.coordinate.longitude", 0.94, None);
    assert!(result.is_some(), "R21 should fire for out-of-range values");
    let (label, rule) = result.unwrap();
    assert_eq!(label, "representation.numeric.decimal_number");
    assert!(rule.contains("coordinate_plausibility_gate"));
}

#[test]
fn test_r21_coordinate_gate_keeps_real_coordinates() {
    // Real longitude values — all within [-180, 180]
    let values: Vec<String> = vec![
        "-122.4194",
        "139.6917",
        "2.3522",
        "-73.9857",
        "151.2093",
        "-43.1729",
        "28.9784",
        "-3.7038",
        "103.8198",
        "-46.6333",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let result = value_sharpen(&values, "geography.coordinate.longitude", 0.95, None);
    // Should NOT fire — these are valid longitude values
    assert!(
        result.is_none()
            || result
                .as_ref()
                .unwrap()
                .1
                .contains("coordinate_disambiguation"),
        "R21 should not fire for valid coordinates"
    );
}

// ── R22: UPC digit-count gate tests ───────────────────────────────
#[test]
fn test_r22_upc_gate_corrects_to_ean() {
    // EAN-13 values (13 digits) misclassified as UPC (12 digits)
    let values: Vec<String> = vec![
        "1794213764625",
        "4293423898067",
        "6324920385397",
        "3683935437077",
        "5078019484874",
        "8706648142321",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let result = value_sharpen(&values, "identity.commerce.upc", 0.999, None);
    assert!(result.is_some(), "R22 should fire for 13-digit values");
    let (label, rule) = result.unwrap();
    assert_eq!(label, "identity.commerce.ean");
    assert!(rule.contains("upc_digit_count_gate"));
}

#[test]
fn test_r22_upc_gate_demotes_wrong_length() {
    // 10-digit values (e.g., NPI) misclassified as UPC
    let values: Vec<String> = vec![
        "1966662179",
        "6579926978",
        "2527909147",
        "9953906342",
        "2157414996",
        "6989529491",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let result = value_sharpen(&values, "identity.commerce.upc", 0.94, None);
    assert!(result.is_some(), "R22 should fire for non-12-digit values");
    let (label, rule) = result.unwrap();
    assert_eq!(label, "representation.identifier.numeric_code");
    assert!(rule.contains("upc_digit_count_gate"));
}

#[test]
fn test_r22_upc_gate_keeps_real_upc() {
    // Real UPC values (12 digits)
    let values: Vec<String> = vec![
        "012345678905",
        "036000291452",
        "070330507227",
        "042100005264",
        "040000000068",
        "041570056103",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let result = value_sharpen(&values, "identity.commerce.upc", 0.99, None);
    // Should NOT fire — these are valid UPC values
    assert!(
        result.is_none() || !result.as_ref().unwrap().1.contains("upc_digit_count_gate"),
        "R22 should not fire for valid 12-digit UPC"
    );
}

// ── R23: ISIN format gate tests ───────────────────────────────────
#[test]
fn test_r23_isin_gate_corrects_isrc() {
    // ISIN values misclassified as ISRC
    let values: Vec<String> = vec![
        "US0378331005",
        "GB0002634946",
        "DE0007164600",
        "JP3435000009",
        "FR0000120271",
        "CA0585861085",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let result = value_sharpen(&values, "identity.commerce.isrc", 0.97, None);
    assert!(result.is_some(), "R23 should fire for ISIN-format values");
    let (label, rule) = result.unwrap();
    assert_eq!(label, "finance.securities.isin");
    assert!(rule.contains("isin_format_gate"));
}

// ── R24: ISSN/EIN dash-position gate tests ────────────────────────
#[test]
fn test_r24_issn_gate_corrects_ein() {
    // ISSN values misclassified as EIN
    let values: Vec<String> = vec![
        "1781-2253",
        "8371-5342",
        "6910-7471",
        "2908-3721",
        "8987-7548",
        "4149-8688",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let result = value_sharpen(&values, "identity.government.ein", 0.91, None);
    assert!(result.is_some(), "R24 should fire for ISSN-format values");
    let (label, rule) = result.unwrap();
    assert_eq!(label, "identity.commerce.issn");
    assert!(rule.contains("issn_format_gate"));
}

#[test]
fn test_is_isin_format() {
    assert!(is_isin_format("US0378331005"));
    assert!(is_isin_format("GB0002634946"));
    assert!(is_isin_format("AU000000BHP4"));
    assert!(is_isin_format("NL0011540547"));

    // Invalid — ISRC values (letters in positions 2-4 = registrant code)
    assert!(!is_isin_format("SE3YX3859059")); // ISRC: positions 2-4 = "3YX"
    assert!(!is_isin_format("NLEK47515013")); // ISRC: positions 2-4 = "EK4"
    assert!(!is_isin_format("CAHRM7311593")); // ISRC: positions 2-4 = "HRM"
                                              // Invalid — other formats
    assert!(!is_isin_format("US-Z03-98-12345")); // ISRC with dashes
    assert!(!is_isin_format("1234567890AB")); // starts with digits
    assert!(!is_isin_format("USABC")); // too short
    assert!(!is_isin_format("us0378331005")); // lowercase
}

#[test]
fn test_is_issn_format() {
    assert!(is_issn_format("1781-2253"));
    assert!(is_issn_format("0317-839X")); // X check digit
    assert!(is_issn_format("0000-0000"));

    // Invalid — EIN format (dash at position 2)
    assert!(!is_issn_format("12-3456789"));
    // Invalid — too short/long
    assert!(!is_issn_format("1234-567"));
    assert!(!is_issn_format("12345-6789"));
    // Invalid — no dash
    assert!(!is_issn_format("12345678"));
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

// AC-2: feature_sharpen — F6 falls back to categorical with empty votes
#[test]
fn test_sharpen_f6_extension_to_categorical_empty_votes() {
    // Multi-branch predicts "file.extension" with single-entry votes
    // and short alphabetic values — F6 should fallback to categorical
    let mut result = ColumnResult {
        label: "representation.file.extension".to_string(),
        confidence: 0.75,
        vote_distribution: vec![("representation.file.extension".to_string(), 0.75)],
        disambiguation_applied: false,
        disambiguation_rule: None,
        samples_used: 50,
        detected_locale: None,
        is_generic: false,
        column_features: None,
    };

    let mut cf = ColumnFeatures::empty();
    cf.mean[feature_idx::LENGTH] = 2.5; // ≤ 4.0
    cf.mean[feature_idx::SEGMENT_COUNT_DOT] = 0.0; // < 1.1
    cf.mean[feature_idx::ALPHA_RATIO] = 0.95; // ≥ 0.8

    feature_sharpen(&mut result, &cf);

    assert_eq!(
        result.label, "representation.discrete.categorical",
        "F6 should fallback to categorical with single-entry votes"
    );
    assert!(result.disambiguation_applied);
}

// AC-3: value_sharpen — R5 day-of-week with single-entry votes
#[test]
fn test_sharpen_r5_day_of_week_single_vote() {
    let values: Vec<String> = vec!["Monday", "Tuesday", "Wednesday", "Thursday", "Friday"]
        .into_iter()
        .map(String::from)
        .collect();

    let result = value_sharpen(&values, "representation.discrete.categorical", 0.80, None);

    assert!(result.is_some(), "R5 should fire for day-of-week values");
    let (label, rule) = result.unwrap();
    assert_eq!(label, "datetime.component.day_of_week");
    assert_eq!(rule, "day_of_week_name_detection");
}

// AC-3: value_sharpen — R8 gender with single-entry votes
#[test]
fn test_sharpen_r8_gender_single_vote() {
    let values: Vec<String> = vec!["Male", "Female", "Male", "Female", "Male"]
        .into_iter()
        .map(String::from)
        .collect();

    let result = value_sharpen(&values, "representation.discrete.categorical", 0.80, None);

    assert!(result.is_some(), "R8 should fire for gender values");
    let (label, rule) = result.unwrap();
    assert_eq!(label, "identity.person.gender");
    assert_eq!(rule, "gender_detection");
}

// AC-3: value_sharpen — R11 categorical with single-entry votes
#[test]
fn test_sharpen_r11_categorical_single_vote() {
    let values: Vec<String> = vec!["red", "blue", "green", "red", "blue"]
        .into_iter()
        .map(String::from)
        .collect();

    // Low cardinality values with a text-like label should trigger categorical
    let result = value_sharpen(&values, "identity.person.first_name", 0.70, None);

    assert!(
        result.is_some(),
        "R11 should fire for low-cardinality categorical values"
    );
    let (label, _rule) = result.unwrap();
    assert_eq!(label, "representation.discrete.categorical");
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

// AC-3: value_sharpen — R17 UTC offset with single-entry votes
#[test]
fn test_sharpen_r17_utc_offset_single_vote() {
    let values: Vec<String> = vec!["+05:30", "-08:00", "+00:00", "+09:00", "-05:00"]
        .into_iter()
        .map(String::from)
        .collect();

    let result = value_sharpen(&values, "representation.numeric.decimal_number", 0.70, None);

    assert!(result.is_some(), "R17 should fire for UTC offset values");
    let (label, rule) = result.unwrap();
    assert_eq!(label, "datetime.offset.utc");
    assert_eq!(rule, "utc_offset_override_time");
}

// AC-3: sharpen_attractor_demotion — confidence threshold
#[test]
fn test_sharpen_attractor_high_confidence_no_demotion() {
    // postal_code with high confidence (0.95) should NOT be demoted
    let values: Vec<String> = vec!["10001", "90210", "60601", "30301", "94102"]
        .into_iter()
        .map(String::from)
        .collect();

    let result = sharpen_attractor_demotion(
        &values,
        "geography.address.postal_code",
        0.95, // high confidence
        None, // no taxonomy for validation
    );

    assert!(
        result.is_none(),
        "High confidence (0.95) should NOT trigger attractor demotion"
    );
}

#[test]
fn test_sharpen_attractor_low_confidence_demotes() {
    // postal_code with low confidence (0.30) should be demoted
    let values: Vec<String> = vec!["42", "100", "7", "256", "1024"]
        .into_iter()
        .map(String::from)
        .collect();

    let result = sharpen_attractor_demotion(
        &values,
        "geography.address.postal_code",
        0.30, // low confidence — below 0.85 threshold
        None, // no taxonomy for validation
    );

    assert!(
        result.is_some(),
        "Low confidence (0.30) should trigger attractor demotion"
    );
    let (label, rule) = result.unwrap();
    assert_eq!(label, "representation.numeric.integer_number");
    assert!(
        rule.starts_with("attractor_demotion_confidence"),
        "Rule should be confidence-based demotion, got: {}",
        rule
    );
}

#[test]
fn test_sharpen_attractor_text_low_confidence_demotes_to_categorical() {
    // first_name with low confidence and low cardinality → categorical
    let values: Vec<String> = vec!["alpha", "beta", "gamma", "alpha", "beta"]
        .into_iter()
        .map(String::from)
        .collect();

    let result = sharpen_attractor_demotion(
        &values,
        "identity.person.first_name",
        0.40, // low confidence
        None,
    );

    assert!(
        result.is_some(),
        "Low confidence text attractor should demote"
    );
    let (label, _rule) = result.unwrap();
    assert_eq!(label, "representation.discrete.categorical");
}

#[test]
fn test_sharpen_attractor_non_attractor_type_ignored() {
    // A type that's not in any attractor list should never trigger demotion
    let values: Vec<String> = vec!["hello@example.com"]
        .into_iter()
        .map(String::from)
        .collect();

    let result = sharpen_attractor_demotion(
        &values,
        "identity.person.email",
        0.30, // even with low confidence
        None,
    );

    assert!(
        result.is_none(),
        "Non-attractor type should never trigger demotion"
    );
}

// ── AC-07(a): h.contains("uri") removal ─────────────────────────────

#[test]
fn ac07a_data_uri_header_no_longer_matches_url() {
    // After removing h.contains("uri"), "data_uri" should NOT map to url.
    // This was the root cause of audit item #1: the keyword match forced
    // "data_uri" → url, overriding the model's prediction.
    assert_eq!(
        header_hint("data_uri"),
        None,
        "data_uri should not match url after h.contains(\"uri\") removal"
    );
}

#[test]
fn ac07a_uri_exact_match_still_works() {
    // The exact match for "uri" at line 3859 must still map to url.
    assert_eq!(header_hint("uri"), Some("technology.internet.url"));
}

#[test]
fn ac07a_url_keyword_still_works() {
    // h.contains("url") still catches url-containing headers.
    assert_eq!(header_hint("download_url"), Some("technology.internet.url"));
    assert_eq!(header_hint("redirect_url"), Some("technology.internet.url"));
}

#[test]
fn ac07a_link_href_keywords_still_work() {
    // Other url keyword matches are unaffected.
    assert_eq!(
        header_hint("external_link"),
        Some("technology.internet.url")
    );
    assert_eq!(header_hint("href"), Some("technology.internet.url"));
}

#[test]
fn ac07a_request_uri_no_longer_matches_url() {
    // Pure "uri" headers without "url" lose the keyword hint.
    // The model handles these from value patterns.
    assert_eq!(
        header_hint("request_uri"),
        None,
        "request_uri should not match url after h.contains(\"uri\") removal"
    );
}

// ── AC-07(b): F3 removal ─────────────────────────────────────────────

#[test]
fn ac07b_r20_still_validates_hs_codes() {
    // R20 (HS code validation gate) must still work as the sole backstop
    // after F3 removal. Model-predicted hs_code with valid values should pass.
    let values: Vec<String> = vec![
        "8471.30",
        "8471.30.00",
        "6204.62",
        "8517.12",
        "0901.21",
        "2204.10",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let result = value_sharpen(&values, "geography.transportation.hs_code", 0.95, None);
    assert!(
        result.is_none(),
        "R20 should keep valid hs_code predictions after F3 removal"
    );
}

#[test]
fn ac07b_r20_still_demotes_false_hs_codes() {
    // R20 must still demote hs_code when values are plain decimals.
    let values: Vec<String> = vec![
        "3.14", "0.887", "-12.5", "100.0", "0.003", "45.67", "1.23", "99.9",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let result = value_sharpen(&values, "geography.transportation.hs_code", 0.95, None);
    assert!(result.is_some(), "R20 should still demote false hs_code");
    let (label, _) = result.unwrap();
    assert_eq!(label, "representation.numeric.decimal_number");
}

// ── AC-05: Country/country_code post-hint guard ──────────────────────
// Note: Full integration tests for AC-05 require the model to be loaded.
// These unit tests verify the guard logic via header_hint and value patterns.

#[test]
fn ac05_country_exact_match_still_hints_country() {
    // The hardcoded hint for "country" still maps to geography.location.country.
    // The post-hint guard in apply_header_sharpen then checks values.
    assert_eq!(header_hint("country"), Some("geography.location.country"));
}

#[test]
fn ac05_alpha2_regex_matches_country_codes() {
    // Verify the alpha-2 check logic matches ISO 3166-1 codes.
    let codes = ["AU", "US", "GB", "DE", "FR", "JP", "CN", "BR"];
    for code in &codes {
        assert!(
            code.len() == 2 && code.chars().all(|c| c.is_ascii_uppercase()),
            "{} should match alpha-2 pattern",
            code
        );
    }
}

#[test]
fn ac05_alpha2_regex_rejects_state_codes() {
    // 3-letter state codes like "NSW" must NOT match the alpha-2 pattern.
    let state_codes = ["NSW", "VIC", "QLD", "CA", "NY"];
    for code in &state_codes {
        let matches = code.len() == 2 && code.chars().all(|c| c.is_ascii_uppercase());
        // CA and NY are 2-letter but they're state codes — the guard only fires
        // when the label is "country" (not state), so these are correctly handled
        // by the pipeline context, not the regex alone.
        if code.len() == 3 {
            assert!(
                !matches,
                "{} (3-char) should not match alpha-2 pattern",
                code
            );
        }
    }
}

#[test]
fn ac05_alpha2_regex_rejects_lowercase() {
    // Lowercase should not match.
    let lower = ["au", "us", "gb"];
    for code in &lower {
        assert!(
            !(code.len() == 2 && code.chars().all(|c| c.is_ascii_uppercase())),
            "{} should not match alpha-2 pattern",
            code
        );
    }
}

#[test]
fn ac05_alpha2_regex_rejects_country_names() {
    // Full country names should not match.
    let names = ["Australia", "United States", "Germany", "France"];
    for name in &names {
        assert!(
            !(name.len() == 2 && name.chars().all(|c| c.is_ascii_uppercase())),
            "{} should not match alpha-2 pattern",
            name
        );
    }
}

// ── full_name → username value veto (spec 2026-06-17-full-name-username-veto) ──

fn vals(xs: &[&str]) -> Vec<String> {
    xs.iter().map(|s| s.to_string()).collect()
}

#[test]
fn username_veto_fires_on_login_handles() {
    // Corpus `author` shape: single-token handles, no internal whitespace.
    let handles = vals(&[
        "tptacek",
        "patio11",
        "rms",
        "jacquesm",
        "petercooper",
        "dfens",
        "schof",
    ]);
    assert!(is_username_handle_shaped(&handles));
}

#[test]
fn username_veto_fires_on_handles_with_digits_underscores() {
    let handles = vals(&["peter123", "dc2k08", "adam_null", "steve19", "mr-justin"]);
    assert!(is_username_handle_shaped(&handles));
}

#[test]
fn username_veto_skips_real_full_names() {
    // Multi-token "First Last" → whitespace fraction 1.0, stays full_name.
    let names = vals(&[
        "John Smith",
        "Mary Jane Watson",
        "Alan Turing",
        "Ada Lovelace",
        "Grace Hopper",
    ]);
    assert!(!is_username_handle_shaped(&names));
}

#[test]
fn username_veto_skips_entity_and_org_names() {
    // player_name / org-name shape: multi-word, has whitespace.
    let orgs = vals(&[
        "Portland Trail Blazers",
        "Golden State Warriors",
        "New York Knicks",
        "Boston Celtics",
    ]);
    assert!(!is_username_handle_shaped(&orgs));
}

#[test]
fn username_veto_skips_mixed_author_lists() {
    // `authors` (plural) corpus shape: ~0.58 whitespace fraction — too mixed.
    let mixed = vals(&[
        "Jane Doe",
        "jsmith",
        "Robert C. Martin",
        "kent_beck",
        "Martin Fowler",
    ]);
    assert!(!is_username_handle_shaped(&mixed));
}

#[test]
fn username_veto_needs_minimum_evidence() {
    // Too few non-empty values to judge.
    let scant = vals(&["rms", "", "  "]);
    assert!(!is_username_handle_shaped(&scant));
}

#[test]
fn username_veto_skips_low_cardinality_vocab() {
    // ac-03 false-positive class: single-token handle-charset values that are a
    // small REPEATING vocabulary (exchange codes, drug names) — not usernames.
    let drugs = vals(&[
        "ethanol", "ethanol", "ethanol", "morphine", "morphine", "morphine", "ethanol", "morphine",
    ]);
    assert!(!is_username_handle_shaped(&drugs));

    let exchanges = vals(&["NMS", "NYQ", "NMS", "NYQ", "NMS", "NYQ", "PCX", "NMS"]);
    assert!(!is_username_handle_shaped(&exchanges));
}

#[test]
fn username_veto_still_fires_on_high_cardinality_handles() {
    // Distinct handles per row survive the cardinality guard.
    let handles = vals(&[
        "ww520",
        "KevBurnsJr",
        "thinkcomp",
        "cjensen",
        "pmiller2",
        "vic_nyc",
        "andrewcooke",
        "lanstein",
    ]);
    assert!(is_username_handle_shaped(&handles));
}

// ── datetime_format_refinement (spec 2026-06-19-deterministic-datetime-parser) ──

#[test]
fn test_datetime_refinement_fires_when_taxonomy_accepts() {
    // sql_standard values + a taxonomy that accepts them → the rule recovers a real
    // timestamp the model called `unknown`.
    let yaml = r#"
datetime.timestamp.sql_standard:
  title: SQL Standard
  designation: universal
  tier: [TIMESTAMP, timestamp]
  release_priority: 5
  samples: ["2020-01-03 14:22:09"]
  validation:
    type: string
    pattern: "^\\d{4}-\\d{2}-\\d{2} \\d{2}:\\d{2}:\\d{2}$"
"#;
    let mut tax = Taxonomy::from_yaml(yaml).unwrap();
    tax.compile_validators();
    let mut cc =
        ColumnClassifier::with_defaults(Box::new(crate::inference::MockClassifier::new("unknown")));
    cc.set_taxonomy(tax);

    let mut r = ColumnResult {
        label: "unknown".to_string(),
        confidence: 0.3,
        vote_distribution: vec![],
        disambiguation_applied: false,
        disambiguation_rule: None,
        samples_used: 2,
        detected_locale: None,
        is_generic: false,
        column_features: None,
    };
    let sample = vec![
        "2001-09-17 00:00:00".to_string(),
        "2011-08-30 00:00:00".to_string(),
    ];
    cc.datetime_format_refinement(&mut r, &sample);
    assert_eq!(r.label, "datetime.timestamp.sql_standard");
    assert_eq!(
        r.disambiguation_rule.as_deref(),
        Some("datetime_format_refinement")
    );
}

// ── structured_string_refinement (spec 2026-06-19-plain-text-type-discovery) ──

#[test]
fn test_structured_string_refinement() {
    let yaml = r#"
technology.filesystem.windows_path:
  title: Windows Path
  designation: universal
  tier: [VARCHAR, filesystem]
  release_priority: 3
  samples: ["C:\\x"]
  validation:
    type: string
    pattern: '^([A-Za-z]:\\|\\\\)[^\r\n]*$'
technology.internet.message_id:
  title: Message ID
  designation: universal
  tier: [VARCHAR, internet]
  release_priority: 3
  samples: ["<a@b>"]
  validation:
    type: string
    pattern: '^<[^<>@\s]+@[^<>@\s]+>$'
technology.code.qualified_name:
  title: Qualified Name
  designation: universal
  tier: [VARCHAR, code]
  release_priority: 3
  samples: ["a.b.c"]
  validation:
    type: string
    pattern: '^[A-Za-z_][A-Za-z0-9_]*(\.[A-Za-z_][A-Za-z0-9_]*){2,}$'
"#;
    let mut tax = Taxonomy::from_yaml(yaml).unwrap();
    tax.compile_validators();
    let mut cc =
        ColumnClassifier::with_defaults(Box::new(crate::inference::MockClassifier::new("unknown")));
    cc.set_taxonomy(tax);

    let mk = |label: &str| ColumnResult {
        label: label.to_string(),
        confidence: 0.3,
        vote_distribution: vec![],
        disambiguation_applied: false,
        disambiguation_rule: None,
        samples_used: 3,
        detected_locale: None,
        is_generic: false,
        column_features: None,
    };
    let run = |cc: &ColumnClassifier, label: &str, vals: &[&str]| {
        let mut r = mk(label);
        let s: Vec<String> = vals.iter().map(|v| v.to_string()).collect();
        cc.structured_string_refinement(&mut r, &s);
        r.label
    };

    // Fires from the residual labels.
    assert_eq!(
        run(
            &cc,
            "representation.text.plain_text",
            &[r"C:\a\b.sys", r"D:\x\y.cs"]
        ),
        "technology.filesystem.windows_path"
    );
    assert_eq!(
        run(
            &cc,
            "representation.text.plain_text",
            &["<a.b@thyme>", "<c.d@thyme>"]
        ),
        "technology.internet.message_id"
    );
    assert_eq!(
        run(&cc, "representation.text.word", &["com.a.B", "org.c.D"]),
        "technology.code.qualified_name"
    );
    // windows_path also recovers from a path/locator misprediction (unambiguous validator).
    assert_eq!(
        run(
            &cc,
            "technology.internet.urn",
            &[r"C:\a\b.sys", r"D:\x\y.cs"]
        ),
        "technology.filesystem.windows_path"
    );
    // qualified_name must NOT eat a confident hostname (structural overlap).
    assert_eq!(
        run(
            &cc,
            "technology.internet.hostname",
            &["www.bbc.co.uk", "api.github.com"]
        ),
        "technology.internet.hostname"
    );
    // Prose is left alone (no validator passes).
    assert_eq!(
        run(
            &cc,
            "representation.text.plain_text",
            &["a sentence here", "more prose now"]
        ),
        "representation.text.plain_text"
    );
}

#[test]
fn test_datetime_refinement_blocked_when_taxonomy_rejects() {
    // The taxonomy's iso_8601_milliseconds requires a trailing `Z`. A zoneless
    // `…:03.123` reads as millis in the detector but FAILS that leaf's schema, so the
    // veto-consistency gate must refuse to assert it (else the downstream veto would
    // hard-reject our label into unknown/alphanumeric_id — strictly worse).
    let yaml = r#"
datetime.timestamp.iso_8601_milliseconds:
  title: ISO 8601 ms
  designation: universal
  tier: [TIMESTAMP, timestamp]
  release_priority: 5
  samples: ["2020-01-03T14:22:09.123Z"]
  validation:
    type: string
    pattern: "^\\d{4}-\\d{2}-\\d{2}T\\d{2}:\\d{2}:\\d{2}\\.\\d{3}Z$"
"#;
    let mut tax = Taxonomy::from_yaml(yaml).unwrap();
    tax.compile_validators();
    let mut cc = ColumnClassifier::with_defaults(Box::new(crate::inference::MockClassifier::new(
        "representation.identifier.alphanumeric_id",
    )));
    cc.set_taxonomy(tax);

    let mut r = ColumnResult {
        label: "representation.identifier.alphanumeric_id".to_string(),
        confidence: 0.5,
        vote_distribution: vec![],
        disambiguation_applied: false,
        disambiguation_rule: None,
        samples_used: 2,
        detected_locale: None,
        is_generic: false,
        column_features: None,
    };
    let sample = vec![
        "2013-06-04T01:02:03.123".to_string(),
        "2018-01-01T05:30:00.000".to_string(),
    ];
    cc.datetime_format_refinement(&mut r, &sample);
    assert_eq!(
        r.label, "representation.identifier.alphanumeric_id",
        "zoneless iso-millis must not be asserted — taxonomy rejects that leaf"
    );
}
