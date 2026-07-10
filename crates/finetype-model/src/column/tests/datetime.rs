use super::super::*;

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
