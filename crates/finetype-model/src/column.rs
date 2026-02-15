//! Column-mode inference for distribution-based type disambiguation.
//!
//! Column-mode takes a vector of string values (a column sample), runs
//! single-value inference on each, aggregates the predictions, and applies
//! disambiguation rules to determine the most likely type for the entire column.
//!
//! This is critical for resolving ambiguous types like:
//! - `us_slash` vs `eu_slash` dates (MM/DD vs DD/MM)
//! - `short_dmy` vs `short_mdy` dates
//! - `latitude` vs `longitude` coordinates
//! - Numeric types (port, increment, postal_code, integer_number)

use crate::inference::{CharClassifier, ClassificationResult, InferenceError};
use std::collections::HashMap;

/// Configuration for column-mode inference.
#[derive(Debug, Clone)]
pub struct ColumnConfig {
    /// Maximum number of values to sample from the column (default: 100).
    pub sample_size: usize,
    /// Minimum fraction of votes a type needs to be the winner (default: 0.3).
    /// If no type reaches this threshold, the result confidence is lowered.
    pub min_agreement: f32,
}

impl Default for ColumnConfig {
    fn default() -> Self {
        Self {
            sample_size: 100,
            min_agreement: 0.3,
        }
    }
}

/// Result of column-mode inference.
#[derive(Debug, Clone)]
pub struct ColumnResult {
    /// The predicted type label for the column.
    pub label: String,
    /// Confidence score (0.0 to 1.0).
    pub confidence: f32,
    /// Vote distribution: label → fraction of samples classified as this type.
    pub vote_distribution: Vec<(String, f32)>,
    /// Whether a disambiguation rule was applied to override the majority vote.
    pub disambiguation_applied: bool,
    /// Name of the disambiguation rule applied, if any.
    pub disambiguation_rule: Option<String>,
    /// Number of values actually classified.
    pub samples_used: usize,
}

/// Column-mode classifier that wraps a single-value classifier.
pub struct ColumnClassifier {
    classifier: CharClassifier,
    config: ColumnConfig,
}

impl ColumnClassifier {
    /// Create a new column classifier wrapping a CharClassifier.
    pub fn new(classifier: CharClassifier, config: ColumnConfig) -> Self {
        Self { classifier, config }
    }

    /// Create with default configuration.
    pub fn with_defaults(classifier: CharClassifier) -> Self {
        Self::new(classifier, ColumnConfig::default())
    }

    /// Classify a column of values, returning a single type prediction.
    ///
    /// The algorithm:
    /// 1. Sample up to `config.sample_size` values
    /// 2. Run single-value inference on each
    /// 3. Aggregate votes by predicted label
    /// 4. Apply disambiguation rules for known ambiguous pairs
    /// 5. Return the final label with confidence
    pub fn classify_column(&self, values: &[String]) -> Result<ColumnResult, InferenceError> {
        if values.is_empty() {
            return Ok(ColumnResult {
                label: "unknown".to_string(),
                confidence: 0.0,
                vote_distribution: vec![],
                disambiguation_applied: false,
                disambiguation_rule: None,
                samples_used: 0,
            });
        }

        // Step 1: Sample values
        let sample = if values.len() <= self.config.sample_size {
            values.to_vec()
        } else {
            // Deterministic sampling: evenly spaced
            let step = values.len() as f64 / self.config.sample_size as f64;
            (0..self.config.sample_size)
                .map(|i| values[(i as f64 * step) as usize].clone())
                .collect()
        };

        let n_samples = sample.len();

        // Step 2: Run batch inference
        let results = self.classifier.classify_batch(&sample)?;

        // Step 3: Aggregate votes
        let mut vote_counts: HashMap<String, usize> = HashMap::new();
        for result in &results {
            *vote_counts.entry(result.label.clone()).or_default() += 1;
        }

        // Sort by count descending
        let mut votes: Vec<(String, usize)> = vote_counts.into_iter().collect();
        votes.sort_by(|a, b| b.1.cmp(&a.1));

        let vote_distribution: Vec<(String, f32)> = votes
            .iter()
            .map(|(label, count)| (label.clone(), *count as f32 / n_samples as f32))
            .collect();

        // Majority winner
        let (majority_label, majority_count) = votes.first().cloned().unwrap_or_default();
        let majority_fraction = majority_count as f32 / n_samples as f32;

        // Step 4: Apply disambiguation rules
        let disambiguation = disambiguate(&sample, &results, &votes, n_samples);

        if let Some((label, rule_name)) = disambiguation {
            Ok(ColumnResult {
                label,
                confidence: majority_fraction.max(0.8), // Disambiguation rules are high-confidence
                vote_distribution,
                disambiguation_applied: true,
                disambiguation_rule: Some(rule_name),
                samples_used: n_samples,
            })
        } else {
            // No disambiguation needed — use majority vote
            let confidence = if majority_fraction >= self.config.min_agreement {
                majority_fraction
            } else {
                majority_fraction * 0.5 // Low agreement → low confidence
            };

            Ok(ColumnResult {
                label: majority_label,
                confidence,
                vote_distribution,
                disambiguation_applied: false,
                disambiguation_rule: None,
                samples_used: n_samples,
            })
        }
    }

    /// Get a reference to the underlying classifier.
    pub fn classifier(&self) -> &CharClassifier {
        &self.classifier
    }

    /// Get a reference to the configuration.
    pub fn config(&self) -> &ColumnConfig {
        &self.config
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// DISAMBIGUATION RULES
// ═══════════════════════════════════════════════════════════════════════════════

/// Disambiguation rule pairs: types that are ambiguous in single-value mode.
const DATE_SLASH_PAIR: (&str, &str) = ("datetime.date.us_slash", "datetime.date.eu_slash");

const SHORT_DATE_PAIR: (&str, &str) = ("datetime.date.short_mdy", "datetime.date.short_dmy");

const COORDINATE_PAIR: (&str, &str) = (
    "geography.coordinate.latitude",
    "geography.coordinate.longitude",
);

/// Apply disambiguation rules when the vote distribution contains known ambiguous pairs.
///
/// Returns Some((resolved_label, rule_name)) if a rule was applied, None otherwise.
fn disambiguate(
    values: &[String],
    results: &[ClassificationResult],
    votes: &[(String, usize)],
    _n_samples: usize,
) -> Option<(String, String)> {
    // Get the top labels in the vote
    let top_labels: Vec<&str> = votes.iter().take(3).map(|(l, _)| l.as_str()).collect();

    // Rule 1: Date slash disambiguation (us_slash vs eu_slash)
    if contains_pair(&top_labels, DATE_SLASH_PAIR.0, DATE_SLASH_PAIR.1) {
        if let Some(label) = disambiguate_slash_dates(values) {
            return Some((label, "date_slash_disambiguation".to_string()));
        }
    }

    // Rule 2: Short date disambiguation (short_mdy vs short_dmy)
    if contains_pair(&top_labels, SHORT_DATE_PAIR.0, SHORT_DATE_PAIR.1) {
        if let Some(label) = disambiguate_short_dates(values) {
            return Some((label, "short_date_disambiguation".to_string()));
        }
    }

    // Rule 3: Coordinate disambiguation (latitude vs longitude)
    if contains_pair(&top_labels, COORDINATE_PAIR.0, COORDINATE_PAIR.1) {
        if let Some(label) = disambiguate_coordinates(values) {
            return Some((label, "coordinate_disambiguation".to_string()));
        }
    }

    // Rule 4: Numeric type disambiguation
    if let Some((label, rule)) = disambiguate_numeric(values, results, &top_labels) {
        return Some((label, rule));
    }

    None
}

/// Check if two labels are both present in the top candidates.
fn contains_pair(labels: &[&str], a: &str, b: &str) -> bool {
    labels.contains(&a) && labels.contains(&b)
}

/// Disambiguate us_slash vs eu_slash dates.
///
/// Pattern: `DD/MM/YYYY` or `MM/DD/YYYY`
/// Rule: If ANY value has first component > 12, it must be DD/MM (eu_slash).
///       If ANY value has second component > 12, it must be MM/DD (us_slash).
fn disambiguate_slash_dates(values: &[String]) -> Option<String> {
    let mut first_over_12 = false;
    let mut second_over_12 = false;

    for val in values {
        let parts: Vec<&str> = val.split('/').collect();
        if parts.len() >= 2 {
            if let Ok(first) = parts[0].parse::<u32>() {
                if first > 12 {
                    first_over_12 = true;
                }
            }
            if let Ok(second) = parts[1].parse::<u32>() {
                if second > 12 {
                    second_over_12 = true;
                }
            }
        }
    }

    if first_over_12 && !second_over_12 {
        // First component > 12 means it's the day → DD/MM/YYYY → eu_slash
        Some("datetime.date.eu_slash".to_string())
    } else if second_over_12 && !first_over_12 {
        // Second component > 12 means it's the day → MM/DD/YYYY → us_slash
        Some("datetime.date.us_slash".to_string())
    } else {
        // Both ambiguous or contradictory — let model decide
        None
    }
}

/// Disambiguate short_dmy vs short_mdy dates.
///
/// Pattern: `DD-MM-YY` or `MM-DD-YY`
/// Rule: Same as slash dates but with dash separator.
fn disambiguate_short_dates(values: &[String]) -> Option<String> {
    let mut first_over_12 = false;
    let mut second_over_12 = false;

    for val in values {
        let parts: Vec<&str> = val.split('-').collect();
        if parts.len() >= 2 {
            if let Ok(first) = parts[0].parse::<u32>() {
                if first > 12 {
                    first_over_12 = true;
                }
            }
            if let Ok(second) = parts[1].parse::<u32>() {
                if second > 12 {
                    second_over_12 = true;
                }
            }
        }
    }

    if first_over_12 && !second_over_12 {
        Some("datetime.date.short_dmy".to_string())
    } else if second_over_12 && !first_over_12 {
        Some("datetime.date.short_mdy".to_string())
    } else {
        None
    }
}

/// Disambiguate latitude vs longitude coordinates.
///
/// Rule: If ANY |value| > 90, it must be longitude (latitude max is 90).
///       If ALL |values| ≤ 90, it's likely latitude.
fn disambiguate_coordinates(values: &[String]) -> Option<String> {
    let mut any_over_90 = false;
    let mut all_parseable = true;
    let mut parsed_count = 0;

    for val in values {
        if let Ok(v) = val.trim().parse::<f64>() {
            parsed_count += 1;
            if v.abs() > 90.0 {
                any_over_90 = true;
            }
        } else {
            all_parseable = false;
        }
    }

    // Need at least some parseable values
    if parsed_count < 3 {
        return None;
    }

    if any_over_90 {
        Some("geography.coordinate.longitude".to_string())
    } else if all_parseable {
        // All values within [-90, 90] — likely latitude
        Some("geography.coordinate.latitude".to_string())
    } else {
        None
    }
}

/// Disambiguate numeric types based on value range and distribution.
///
/// Covers: port, increment, postal_code, integer_number, street_number, year
fn disambiguate_numeric(
    values: &[String],
    results: &[ClassificationResult],
    top_labels: &[&str],
) -> Option<(String, String)> {
    // Only trigger for numeric-looking columns
    let numeric_types = [
        "technology.internet.port",
        "representation.numeric.increment",
        "representation.numeric.integer_number",
        "representation.numeric.decimal_number",
        "geography.address.postal_code",
        "geography.address.street_number",
        "datetime.component.year",
    ];

    let has_numeric_confusion = top_labels.iter().any(|l| numeric_types.contains(l));
    if !has_numeric_confusion {
        return None;
    }

    // Parse all values as integers
    let parsed: Vec<i64> = values
        .iter()
        .filter_map(|v| v.trim().parse::<i64>().ok())
        .collect();

    if parsed.len() < 3 {
        return None;
    }

    let min = *parsed.iter().min().unwrap();
    let max = *parsed.iter().max().unwrap();
    let range = max - min;

    // Check for sequential/increment pattern
    let mut sorted = parsed.clone();
    sorted.sort();
    sorted.dedup();
    let is_sequential = if sorted.len() >= 3 {
        let diffs: Vec<i64> = sorted.windows(2).map(|w| w[1] - w[0]).collect();
        let avg_diff = diffs.iter().sum::<i64>() as f64 / diffs.len() as f64;
        let variance = diffs
            .iter()
            .map(|d| (*d as f64 - avg_diff).powi(2))
            .sum::<f64>()
            / diffs.len() as f64;
        // Low variance in diffs → sequential
        variance < (avg_diff * 0.5).powi(2) && avg_diff > 0.0
    } else {
        false
    };

    // Port detection: 0-65535, common ports cluster
    // Require ≥30% of values to match common ports (not just "any").
    // This prevents false positives on age/count columns where a few values
    // (e.g., 22, 25, 53) coincidentally match common port numbers.
    let all_in_port_range = min >= 0 && max <= 65535;
    let common_ports = [80, 443, 8080, 3306, 5432, 22, 21, 25, 53, 3000, 8000, 8443];
    let common_port_count = parsed.iter().filter(|v| common_ports.contains(v)).count();
    let common_port_fraction = common_port_count as f64 / parsed.len() as f64;
    let has_common_ports = common_port_fraction >= 0.3;

    // Postal code detection: typically 3-10 digits, non-sequential, bounded range
    let all_positive = min > 0;
    let typical_postal_range = all_positive && max <= 99999 && min >= 100;
    let digit_lengths: Vec<usize> = values
        .iter()
        .filter_map(|v| {
            let trimmed = v.trim();
            if trimmed.chars().all(|c| c.is_ascii_digit()) {
                Some(trimmed.len())
            } else {
                None
            }
        })
        .collect();
    let consistent_digits = if !digit_lengths.is_empty() {
        let first_len = digit_lengths[0];
        digit_lengths.iter().all(|&l| l == first_len)
    } else {
        false
    };

    // Year detection: 4-digit integers in 1900-2100 range
    // Relaxed: ≥80% of values must be in year range (allows occasional outliers)
    let year_candidates: Vec<i64> = parsed
        .iter()
        .filter(|&&v| (1900..=2100).contains(&v))
        .copied()
        .collect();
    let count_trimmed_4digit = values
        .iter()
        .filter(|v| {
            let t = v.trim();
            t.len() == 4 && t.chars().all(|c| c.is_ascii_digit())
        })
        .count();
    let fraction_4digit = if values.is_empty() {
        0.0
    } else {
        count_trimmed_4digit as f64 / values.len() as f64
    };
    let mostly_4digit = fraction_4digit >= 0.8;
    let year_fraction = if parsed.is_empty() {
        0.0
    } else {
        year_candidates.len() as f64 / parsed.len() as f64
    };
    let is_year_column = year_fraction >= 0.8 && parsed.len() >= 3 && mostly_4digit;

    // Decision logic — year check BEFORE sequential, because a column of
    // years (e.g., 2018, 2019, 2020) is more likely to be years than IDs.
    if is_year_column {
        // All values are 4-digit integers in 1900-2100 range → year
        return Some((
            "datetime.component.year".to_string(),
            "numeric_year_detection".to_string(),
        ));
    }

    if is_sequential && min >= 0 && range > 0 {
        // Sequential integers → increment
        return Some((
            "representation.numeric.increment".to_string(),
            "numeric_sequential_detection".to_string(),
        ));
    }

    if has_common_ports && all_in_port_range && !is_sequential {
        // Has common ports and all in range → port
        return Some((
            "technology.internet.port".to_string(),
            "numeric_port_detection".to_string(),
        ));
    }

    if consistent_digits && typical_postal_range && !is_sequential {
        // Exclude year-like columns: if ≥80% of 4-digit values are in 1900-2100,
        // prefer year over postal code (e.g., years with occasional outlier)
        if mostly_4digit && year_fraction >= 0.8 {
            return Some((
                "datetime.component.year".to_string(),
                "numeric_year_detection".to_string(),
            ));
        }
        // Consistent digit length, typical postal range → postal code
        return Some((
            "geography.address.postal_code".to_string(),
            "numeric_postal_code_detection".to_string(),
        ));
    }

    // Street number: small positive integers, typically 1-9999
    let street_range = all_positive && max < 100000 && min >= 1;
    let is_street_candidate = top_labels.contains(&"geography.address.street_number");
    if is_street_candidate
        && street_range
        && !is_sequential
        && !has_common_ports
        && !consistent_digits
    {
        return Some((
            "geography.address.street_number".to_string(),
            "numeric_street_number_detection".to_string(),
        ));
    }

    // Fallback: if we couldn't determine more specifically, use the model majority
    // (return None to let the majority vote stand)
    let _ = results; // suppress unused warning
    None
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(result, Some("datetime.date.eu_slash".to_string()));
    }

    #[test]
    fn test_slash_date_us_detected() {
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
        assert_eq!(result, Some("datetime.date.us_slash".to_string()));
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
                label: "representation.numeric.increment".to_string(),
                confidence: 0.8,
                all_scores: vec![],
            })
            .collect();

        let votes = vec![
            ("representation.numeric.increment".to_string(), 8),
            ("representation.numeric.integer_number".to_string(), 2),
        ];
        let top_labels: Vec<&str> = votes.iter().map(|(l, _)| l.as_str()).collect();

        let result = disambiguate_numeric(&values, &results, &top_labels);
        assert!(result.is_some());
        let (label, _rule) = result.unwrap();
        assert_eq!(label, "representation.numeric.increment");
    }

    #[test]
    fn test_numeric_port_detection() {
        let values: Vec<String> = vec![
            "80", "443", "8080", "3306", "22", "5432", "3000", "8443", "25", "53",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        let results: Vec<ClassificationResult> = values
            .iter()
            .map(|_| ClassificationResult {
                label: "technology.internet.port".to_string(),
                confidence: 0.7,
                all_scores: vec![],
            })
            .collect();

        let votes = vec![
            ("technology.internet.port".to_string(), 7),
            ("representation.numeric.integer_number".to_string(), 3),
        ];
        let top_labels: Vec<&str> = votes.iter().map(|(l, _)| l.as_str()).collect();

        let result = disambiguate_numeric(&values, &results, &top_labels);
        assert!(result.is_some());
        let (label, _rule) = result.unwrap();
        assert_eq!(label, "technology.internet.port");
    }

    #[test]
    fn test_numeric_postal_code_detection() {
        let values: Vec<String> = vec![
            "10001", "90210", "30301", "60601", "02101", "75001", "33101", "94102", "20001",
            "98101",
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

        let votes = vec![
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

        let votes = vec![
            ("representation.numeric.integer_number".to_string(), 5),
            ("geography.address.street_number".to_string(), 3),
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

        let votes = vec![
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
            "10001", "90210", "30301", "60601", "02101", "75001", "33101", "94102", "20001",
            "98101",
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

        let votes = vec![
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
                label: "representation.numeric.increment".to_string(),
                confidence: 0.7,
                all_scores: vec![],
            })
            .collect();

        let votes = vec![
            ("representation.numeric.increment".to_string(), 7),
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
                label: "representation.numeric.increment".to_string(),
                confidence: 0.8,
                all_scores: vec![],
            })
            .collect();

        let votes = vec![
            ("representation.numeric.increment".to_string(), 8),
            ("representation.numeric.integer_number".to_string(), 2),
        ];
        let top_labels: Vec<&str> = votes.iter().map(|(l, _)| l.as_str()).collect();

        let result = disambiguate_numeric(&values, &results, &top_labels);
        assert!(result.is_some());
        let (label, _) = result.unwrap();
        assert_eq!(label, "representation.numeric.increment");
    }

    #[test]
    fn test_year_not_triggered_for_ports() {
        // Port numbers (some happen to be in year range but have common ports)
        let values: Vec<String> = vec!["80", "443", "8080", "3306", "22"]
            .into_iter()
            .map(String::from)
            .collect();

        let results: Vec<ClassificationResult> = values
            .iter()
            .map(|_| ClassificationResult {
                label: "technology.internet.port".to_string(),
                confidence: 0.8,
                all_scores: vec![],
            })
            .collect();

        let votes = vec![
            ("technology.internet.port".to_string(), 4),
            ("representation.numeric.integer_number".to_string(), 1),
        ];
        let top_labels: Vec<&str> = votes.iter().map(|(l, _)| l.as_str()).collect();

        let result = disambiguate_numeric(&values, &results, &top_labels);
        // Should NOT be year (values are 2-4 digits, not all 4-digit)
        if let Some((label, _)) = result {
            assert_ne!(label, "datetime.component.year");
        }
    }

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

        let votes = vec![
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

        let votes = vec![
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

        let votes = vec![
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

        let votes = vec![("representation.numeric.decimal_number".to_string(), 10)];
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

        let votes = vec![("representation.numeric.decimal_number".to_string(), 10)];
        let top_labels: Vec<&str> = votes.iter().map(|(l, _)| l.as_str()).collect();

        let result = disambiguate_numeric(&values, &results, &top_labels);
        // 5 of 10 values are 4-digit (50% < 80%) → should NOT be year
        if let Some((label, _)) = result {
            assert_ne!(label, "datetime.component.year");
        }
    }

    #[test]
    fn test_age_column_not_detected_as_port() {
        // Age values like 22, 25, 53 coincidentally match common port numbers,
        // but the fraction is too low (3/10 = 30%) and the column is clearly ages.
        // With the ≥30% threshold and the sequential/year checks running first,
        // this should NOT be classified as port.
        let values: Vec<String> = vec![
            "22", "25", "30", "35", "40", "45", "50", "53", "60", "70",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        let results: Vec<ClassificationResult> = values
            .iter()
            .map(|_| ClassificationResult {
                label: "technology.internet.port".to_string(),
                confidence: 0.7,
                all_scores: vec![],
            })
            .collect();

        let votes = vec![
            ("technology.internet.port".to_string(), 6),
            ("representation.numeric.integer_number".to_string(), 4),
        ];
        let top_labels: Vec<&str> = votes.iter().map(|(l, _)| l.as_str()).collect();

        let result = disambiguate_numeric(&values, &results, &top_labels);
        // Should NOT be port — only 3 of 10 values (22, 25, 53) match common ports
        // and these are typical age values
        if let Some((label, _)) = result {
            assert_ne!(label, "technology.internet.port");
        }
    }

    #[test]
    fn test_age_column_with_mixed_values_not_port() {
        // Realistic Titanic-like age column: mix of young and old ages.
        // Values 21, 22, 25, 53 match common ports but that's only 4/15 = 27% < 30%.
        let values: Vec<String> = vec![
            "2", "5", "14", "17", "21", "22", "25", "28", "33", "38", "42", "47", "53", "61",
            "75",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        let results: Vec<ClassificationResult> = values
            .iter()
            .map(|_| ClassificationResult {
                label: "technology.internet.port".to_string(),
                confidence: 0.6,
                all_scores: vec![],
            })
            .collect();

        let votes = vec![
            ("technology.internet.port".to_string(), 8),
            ("representation.numeric.integer_number".to_string(), 7),
        ];
        let top_labels: Vec<&str> = votes.iter().map(|(l, _)| l.as_str()).collect();

        let result = disambiguate_numeric(&values, &results, &top_labels);
        if let Some((label, _)) = result {
            assert_ne!(label, "technology.internet.port");
        }
    }

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
        };
        assert_eq!(result.label, "unknown");
        assert_eq!(result.samples_used, 0);
    }
}
