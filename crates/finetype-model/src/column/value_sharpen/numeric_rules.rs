use super::*;

/// Disambiguate numeric types based on value range and distribution.
///
/// Covers: port, increment, postal_code, integer_number, year
pub(crate) fn disambiguate_numeric(
    values: &[String],
    results: &[ClassificationResult],
    top_labels: &[&str],
) -> Option<(String, String)> {
    // Only trigger for numeric-looking columns
    let numeric_types = [
        "representation.identifier.increment",
        "representation.numeric.integer_number",
        "representation.numeric.decimal_number",
        "geography.address.postal_code",
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
    // Span + sequential/increment detection. Both `range` and `is_sequential` are
    // consulted only for non-negative columns — line ~1600 gates the increment
    // branch on `min >= 0`, and the postal branch's `!is_sequential` sits behind
    // `typical_postal_range` (which requires `min >= 100`). Only there is the i64
    // span arithmetic overflow-free: a negative sentinel like `i64::MIN` alongside
    // positive values makes `max - min` and the sorted diffs exceed `i64::MAX`
    // (debug-panic; silent wrap in release). Computing them solely when `min >= 0`
    // removes that whole overflow class and is a no-op — the values are dead when
    // `min < 0`, and when `min >= 0` the diffs telescope to `max - min ≤ i64::MAX`,
    // so nothing inside can overflow.
    let (range, is_sequential) = if min >= 0 {
        let range = max - min;
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
        (range, is_sequential)
    } else {
        // Negative columns never reach either consumer (increment branch is gated
        // on `min >= 0`; postal on `min >= 100`), so these values are dead.
        (0i64, false)
    };

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
            "representation.identifier.increment".to_string(),
            "numeric_sequential_detection".to_string(),
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
        // R25 guard: HTTP status code gate (v15, Option C).
        // If all values are 3-digit and ≥90% are in 100-599 (HTTP status range),
        // these are status codes, not postal codes. Keep the model's prediction.
        if digit_lengths.iter().all(|&l| l == 3) {
            let status_count = parsed.iter().filter(|&&v| (100..=599).contains(&v)).count();
            let status_rate = status_count as f64 / parsed.len() as f64;
            if status_rate >= 0.90 {
                // Don't convert to postal_code — return None to keep model prediction
                return None;
            }
        }
        // Consistent digit length, typical postal range → postal code
        return Some((
            "geography.address.postal_code".to_string(),
            "numeric_postal_code_detection".to_string(),
        ));
    }

    // Fallback: if we couldn't determine more specifically, use the model majority
    // (return None to let the majority vote stand)
    let _ = results; // suppress unused warning
    None
}
/// SI number override: plain decimals misclassified as si_number.
///
/// The T2_DOUBLE_numeric model sometimes predicts `si_number` for columns of
/// plain decimals (e.g. "5.1", "3.5") because the numeric prefix of SI values
/// (before the suffix like K, M, G) looks identical. If no sampled values
/// contain an SI suffix, override to `decimal_number`.
pub(crate) fn disambiguate_si_number(values: &[String]) -> Option<(String, String)> {
    // SI suffixes: K/k (kilo), M/m (mega), B/b (billion), T/t (tera/trillion),
    // G/g (giga). Also check for % which would be percentage.
    const SI_SUFFIXES: &[char] = &['K', 'k', 'M', 'm', 'B', 'b', 'T', 't', 'G', 'g'];

    let has_si_suffix = values.iter().any(|v| {
        let trimmed = v.trim();
        if trimmed.is_empty() {
            return false;
        }
        // Check if the last character (ignoring trailing whitespace) is an SI suffix
        trimmed
            .chars()
            .last()
            .is_some_and(|c| SI_SUFFIXES.contains(&c))
    });

    if !has_si_suffix {
        Some((
            "representation.numeric.decimal_number".to_string(),
            "si_number_override_no_suffix".to_string(),
        ))
    } else {
        None
    }
}
