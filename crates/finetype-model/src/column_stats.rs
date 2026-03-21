//! Column-level statistical features (Sherlock-inspired).
//!
//! Computes aggregate statistics over an entire column of string values.
//! Complements the per-value `features.rs` (36-dim) with 27 column-level dimensions:
//!
//! - **Entropy & Cardinality** (4 dims): Shannon entropy, uniqueness, count, emptiness
//! - **Value Length Statistics** (8 dims): mean, variance, min, max, median, skewness, kurtosis, sum
//! - **Character Composition** (10 dims): digit/alpha/special cell fractions, count stats, word counts
//! - **Structural** (5 dims): word count std, case pattern fractions
//!
//! All features are deterministic: same input always produces the same output.
//! Pure computation -- no ML dependencies, no file I/O.

use std::collections::HashMap;

/// Total number of column-level statistical features.
pub const COLUMN_STATS_DIM: usize = 27;

/// Human-readable names for each feature index.
pub const COLUMN_STATS_NAMES: [&str; COLUMN_STATS_DIM] = [
    // Entropy & Cardinality (4)
    "col_entropy",       // 0
    "frac_unique",       // 1
    "n_values",          // 2
    "frac_empty",        // 3
    // Value Length Statistics (8)
    "length_mean",       // 4
    "length_variance",   // 5
    "length_min",        // 6
    "length_max",        // 7
    "length_median",     // 8
    "length_skewness",   // 9
    "length_kurtosis",   // 10
    "length_sum",        // 11
    // Character Composition (10)
    "frac_numeric_cells",  // 12
    "frac_alpha_cells",    // 13
    "frac_special_cells",  // 14
    "avg_digit_count",     // 15
    "std_digit_count",     // 16
    "avg_alpha_count",     // 17
    "std_alpha_count",     // 18
    "avg_special_count",   // 19
    "std_special_count",   // 20
    "avg_word_count",      // 21
    // Structural (5)
    "std_word_count",      // 22
    "frac_starts_upper",   // 23
    "frac_all_upper",      // 24
    "frac_all_lower",      // 25
    "frac_mixed_case",     // 26
];

/// Extract column-level statistical features from a set of values.
///
/// Returns `None` if values is empty.
///
/// # Example
///
/// ```
/// use finetype_model::column_stats::{extract_column_stats, COLUMN_STATS_DIM};
///
/// let values = vec!["hello", "world", "foo"];
/// let stats = extract_column_stats(&values).unwrap();
/// assert_eq!(stats.len(), COLUMN_STATS_DIM);
/// ```
pub fn extract_column_stats(values: &[&str]) -> Option<[f32; COLUMN_STATS_DIM]> {
    if values.is_empty() {
        return None;
    }

    let mut f = [0.0f32; COLUMN_STATS_DIM];
    let total = values.len();

    // Partition into non-empty and empty
    let non_empty: Vec<&str> = values.iter().filter(|v| !v.is_empty()).copied().collect();
    let n_non_empty = non_empty.len();
    let n_empty = total - n_non_empty;

    // ─── Entropy & Cardinality ────────────────────────────────────────────
    // Shannon entropy over value frequencies
    let mut freq: HashMap<&str, usize> = HashMap::new();
    for &v in values {
        *freq.entry(v).or_insert(0) += 1;
    }
    let n_f = total as f64;
    let entropy: f64 = freq
        .values()
        .map(|&count| {
            let p = count as f64 / n_f;
            if p > 0.0 { -p * p.ln() } else { 0.0 }
        })
        .sum();

    f[0] = entropy as f32; // col_entropy
    f[1] = freq.len() as f32 / total as f32; // frac_unique
    f[2] = (total as f64 + 1.0).ln() as f32; // n_values (log-scaled)
    f[3] = n_empty as f32 / total as f32; // frac_empty

    // Guard: if no non-empty values, remaining features stay at 0
    if n_non_empty == 0 {
        return Some(f);
    }

    let n_ne_f = n_non_empty as f64;

    // ─── Per-value stats (single pass) ────────────────────────────────────
    let mut lengths: Vec<f64> = Vec::with_capacity(n_non_empty);
    let mut digit_counts: Vec<f64> = Vec::with_capacity(n_non_empty);
    let mut alpha_counts: Vec<f64> = Vec::with_capacity(n_non_empty);
    let mut special_counts: Vec<f64> = Vec::with_capacity(n_non_empty);
    let mut word_counts: Vec<f64> = Vec::with_capacity(n_non_empty);

    let mut numeric_cells: usize = 0;
    let mut alpha_cells: usize = 0;
    let mut special_cells: usize = 0;
    let mut starts_upper: usize = 0;
    let mut all_upper: usize = 0;
    let mut all_lower: usize = 0;
    let mut mixed_case: usize = 0;

    for &v in &non_empty {
        let chars: Vec<char> = v.chars().collect();
        lengths.push(chars.len() as f64);

        let mut digits: u32 = 0;
        let mut alphas: u32 = 0;
        let mut specials: u32 = 0;
        let mut uppers: u32 = 0;
        let mut lowers: u32 = 0;

        for &c in &chars {
            if c.is_ascii_digit() {
                digits += 1;
            } else if c.is_alphabetic() {
                alphas += 1;
                if c.is_uppercase() {
                    uppers += 1;
                } else if c.is_lowercase() {
                    lowers += 1;
                }
            } else if !c.is_whitespace() {
                specials += 1;
            }
        }

        digit_counts.push(digits as f64);
        alpha_counts.push(alphas as f64);
        special_counts.push(specials as f64);
        word_counts.push(v.split_whitespace().count() as f64);

        if digits > 0 {
            numeric_cells += 1;
        }
        if alphas > 0 {
            alpha_cells += 1;
        }
        if specials > 0 {
            special_cells += 1;
        }

        // Case patterns (only meaningful if there are alphabetic chars)
        if alphas > 0 {
            if let Some(&first) = chars.iter().find(|c| c.is_alphabetic()) {
                if first.is_uppercase() {
                    starts_upper += 1;
                }
            }
            if lowers == 0 {
                all_upper += 1;
            } else if uppers == 0 {
                all_lower += 1;
            } else {
                mixed_case += 1;
            }
        }
    }

    // ─── Value Length Statistics ───────────────────────────────────────────
    let len_mean = mean(&lengths);
    let len_var = variance(&lengths, len_mean);

    f[4] = len_mean as f32; // length_mean
    f[5] = len_var as f32; // length_variance

    // min/max
    let len_min = lengths.iter().copied().fold(f64::INFINITY, f64::min);
    let len_max = lengths.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    f[6] = len_min as f32; // length_min
    f[7] = len_max as f32; // length_max

    f[8] = median(&mut lengths) as f32; // length_median (sorts in place)

    // Reconstruct sorted lengths for skewness/kurtosis -- already sorted by median()
    let len_std = len_var.sqrt();
    f[9] = skewness(&lengths, len_mean, len_std) as f32; // length_skewness
    f[10] = kurtosis(&lengths, len_mean, len_std) as f32; // length_kurtosis

    let len_sum: f64 = lengths.iter().sum();
    f[11] = (len_sum + 1.0).ln() as f32; // length_sum (log-scaled)

    // ─── Character Composition ────────────────────────────────────────────
    f[12] = numeric_cells as f32 / n_ne_f as f32; // frac_numeric_cells
    f[13] = alpha_cells as f32 / n_ne_f as f32; // frac_alpha_cells
    f[14] = special_cells as f32 / n_ne_f as f32; // frac_special_cells

    let digit_mean = mean(&digit_counts);
    f[15] = digit_mean as f32; // avg_digit_count
    f[16] = std_dev(&digit_counts, digit_mean) as f32; // std_digit_count

    let alpha_mean = mean(&alpha_counts);
    f[17] = alpha_mean as f32; // avg_alpha_count
    f[18] = std_dev(&alpha_counts, alpha_mean) as f32; // std_alpha_count

    let special_mean = mean(&special_counts);
    f[19] = special_mean as f32; // avg_special_count
    f[20] = std_dev(&special_counts, special_mean) as f32; // std_special_count

    let word_mean = mean(&word_counts);
    f[21] = word_mean as f32; // avg_word_count

    // ─── Structural ───────────────────────────────────────────────────────
    f[22] = std_dev(&word_counts, word_mean) as f32; // std_word_count
    f[23] = starts_upper as f32 / n_ne_f as f32; // frac_starts_upper
    f[24] = all_upper as f32 / n_ne_f as f32; // frac_all_upper
    f[25] = all_lower as f32 / n_ne_f as f32; // frac_all_lower
    f[26] = mixed_case as f32 / n_ne_f as f32; // frac_mixed_case

    Some(f)
}

// ═══════════════════════════════════════════════════════════════════════════════
// Statistical helpers
// ═══════════════════════════════════════════════════════════════════════════════

/// Arithmetic mean.
fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.iter().sum::<f64>() / values.len() as f64
}

/// Population variance.
fn variance(values: &[f64], mean: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.iter().map(|&x| (x - mean).powi(2)).sum::<f64>() / values.len() as f64
}

/// Population standard deviation.
fn std_dev(values: &[f64], mean: f64) -> f64 {
    variance(values, mean).sqrt()
}

/// Median (sorts the slice in place).
fn median(values: &mut [f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = values.len();
    if n.is_multiple_of(2) {
        (values[n / 2 - 1] + values[n / 2]) / 2.0
    } else {
        values[n / 2]
    }
}

/// Skewness (Fisher's definition). Returns 0 if std_dev is 0 or fewer than 2 values.
fn skewness(values: &[f64], mean: f64, std_dev: f64) -> f64 {
    if values.len() < 2 || std_dev == 0.0 {
        return 0.0;
    }
    let n = values.len() as f64;
    let m3: f64 = values.iter().map(|&x| ((x - mean) / std_dev).powi(3)).sum::<f64>() / n;
    m3
}

/// Excess kurtosis (Fisher's definition). Returns 0 if std_dev is 0 or fewer than 2 values.
fn kurtosis(values: &[f64], mean: f64, std_dev: f64) -> f64 {
    if values.len() < 2 || std_dev == 0.0 {
        return 0.0;
    }
    let n = values.len() as f64;
    let m4: f64 = values.iter().map(|&x| ((x - mean) / std_dev).powi(4)).sum::<f64>() / n;
    m4 - 3.0 // excess kurtosis
}

// ═══════════════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to get a named feature value from column stats.
    fn stat(features: &[f32; COLUMN_STATS_DIM], name: &str) -> f32 {
        let idx = COLUMN_STATS_NAMES
            .iter()
            .position(|&n| n == name)
            .unwrap_or_else(|| panic!("Unknown column stat: {}", name));
        features[idx]
    }

    // ─── Test 1: Dimensions ──────────────────────────────────────────────

    #[test]
    fn test_column_stats_dim_matches_names() {
        assert_eq!(COLUMN_STATS_NAMES.len(), COLUMN_STATS_DIM);
        assert_eq!(COLUMN_STATS_DIM, 27);
        // All names should be unique
        let unique: std::collections::HashSet<&str> = COLUMN_STATS_NAMES.iter().copied().collect();
        assert_eq!(unique.len(), COLUMN_STATS_DIM);
    }

    // ─── Test 2: Known column with hand-calculated values ────────────────

    #[test]
    fn test_known_column_entropy_and_frac_unique() {
        // 10 values: 3 distinct ("a" x4, "b" x3, "c" x3)
        let values: Vec<&str> = vec!["a", "b", "c", "a", "b", "c", "a", "b", "c", "a"];
        let stats = extract_column_stats(&values).unwrap();

        // frac_unique: 3 distinct / 10 total = 0.3
        assert!((stat(&stats, "frac_unique") - 0.3).abs() < 0.001);

        // Shannon entropy: -[4/10 * ln(4/10) + 3/10 * ln(3/10) + 3/10 * ln(3/10)]
        // = -[0.4 * ln(0.4) + 0.3 * ln(0.3) + 0.3 * ln(0.3)]
        // = -[0.4 * (-0.9163) + 0.3 * (-1.2040) + 0.3 * (-1.2040)]
        // = -[-0.3665 + (-0.3612) + (-0.3612)]
        // = -[-1.0889] = 1.0889
        let entropy = stat(&stats, "col_entropy");
        assert!(
            (entropy - 1.0889).abs() < 0.01,
            "Expected entropy ~1.0889, got {}",
            entropy
        );

        // n_values: ln(10 + 1) = ln(11) ~ 2.3979
        let n_values = stat(&stats, "n_values");
        assert!(
            (n_values - 2.3979).abs() < 0.01,
            "Expected n_values ~2.3979, got {}",
            n_values
        );

        // frac_empty: 0/10 = 0
        assert_eq!(stat(&stats, "frac_empty"), 0.0);
    }

    // ─── Test 3: All-identical values (entropy = 0) ──────────────────────

    #[test]
    fn test_all_identical_values() {
        let values: Vec<&str> = vec!["same", "same", "same", "same", "same"];
        let stats = extract_column_stats(&values).unwrap();

        assert_eq!(stat(&stats, "col_entropy"), 0.0);
        assert!((stat(&stats, "frac_unique") - 0.2).abs() < 0.001); // 1/5

        // All lengths are 4, so variance = 0
        assert_eq!(stat(&stats, "length_variance"), 0.0);
        assert_eq!(stat(&stats, "length_skewness"), 0.0);
        assert_eq!(stat(&stats, "length_kurtosis"), 0.0);
        assert_eq!(stat(&stats, "length_mean"), 4.0);
        assert_eq!(stat(&stats, "length_min"), 4.0);
        assert_eq!(stat(&stats, "length_max"), 4.0);
        assert_eq!(stat(&stats, "length_median"), 4.0);
    }

    // ─── Test 4: Single value ────────────────────────────────────────────

    #[test]
    fn test_single_value() {
        let values: Vec<&str> = vec!["hello"];
        let stats = extract_column_stats(&values).unwrap();

        assert_eq!(stat(&stats, "col_entropy"), 0.0); // single value: p=1, -1*ln(1)=0
        assert_eq!(stat(&stats, "frac_unique"), 1.0); // 1/1
        assert_eq!(stat(&stats, "length_variance"), 0.0);
        assert_eq!(stat(&stats, "length_skewness"), 0.0);
        assert_eq!(stat(&stats, "length_kurtosis"), 0.0);
    }

    // ─── Test 5: frac_numeric_cells ──────────────────────────────────────

    #[test]
    fn test_frac_numeric_cells() {
        let values: Vec<&str> = vec!["abc", "123", "a1b"];
        let stats = extract_column_stats(&values).unwrap();

        // "123" has digits, "a1b" has digits => 2/3
        assert!(
            (stat(&stats, "frac_numeric_cells") - 2.0 / 3.0).abs() < 0.001,
            "Expected frac_numeric_cells ~0.667, got {}",
            stat(&stats, "frac_numeric_cells")
        );
    }

    // ─── Test 6: Length stats on simple column ───────────────────────────

    #[test]
    fn test_length_stats_simple() {
        // Lengths: 1, 2, 3, 4, 5
        let values: Vec<&str> = vec!["a", "ab", "abc", "abcd", "abcde"];
        let stats = extract_column_stats(&values).unwrap();

        // Mean: (1+2+3+4+5)/5 = 3.0
        assert!((stat(&stats, "length_mean") - 3.0).abs() < 0.001);

        // Variance: ((1-3)^2 + (2-3)^2 + (3-3)^2 + (4-3)^2 + (5-3)^2) / 5
        //         = (4 + 1 + 0 + 1 + 4) / 5 = 2.0
        assert!((stat(&stats, "length_variance") - 2.0).abs() < 0.001);

        // Min: 1, Max: 5
        assert_eq!(stat(&stats, "length_min"), 1.0);
        assert_eq!(stat(&stats, "length_max"), 5.0);

        // Median: 3 (middle of sorted [1,2,3,4,5])
        assert_eq!(stat(&stats, "length_median"), 3.0);

        // Sum: 15, log-scaled: ln(16) ~ 2.7726
        assert!((stat(&stats, "length_sum") - (16.0_f32).ln()).abs() < 0.01);
    }

    // ─── Test 7: Empty slice returns None ────────────────────────────────

    #[test]
    fn test_empty_values_returns_none() {
        let values: Vec<&str> = vec![];
        assert!(extract_column_stats(&values).is_none());
    }

    // ─── Test 8: All-empty values ────────────────────────────────────────

    #[test]
    fn test_all_empty_values() {
        let values: Vec<&str> = vec!["", "", ""];
        let stats = extract_column_stats(&values).unwrap();

        assert_eq!(stat(&stats, "frac_empty"), 1.0);
        // All remaining features should be 0 (no non-empty values)
        assert_eq!(stat(&stats, "length_mean"), 0.0);
        assert_eq!(stat(&stats, "frac_numeric_cells"), 0.0);
    }

    // ─── Test 9: Mixed empty and non-empty ───────────────────────────────

    #[test]
    fn test_mixed_empty_nonempty() {
        let values: Vec<&str> = vec!["hello", "", "world", ""];
        let stats = extract_column_stats(&values).unwrap();

        assert!((stat(&stats, "frac_empty") - 0.5).abs() < 0.001);
        // Length stats computed over non-empty only: "hello" (5), "world" (5)
        assert_eq!(stat(&stats, "length_mean"), 5.0);
        assert_eq!(stat(&stats, "length_variance"), 0.0);
    }

    // ─── Test 10: Case pattern fractions ─────────────────────────────────

    #[test]
    fn test_case_patterns() {
        let values: Vec<&str> = vec!["Hello", "WORLD", "foo", "Bar", "BAZ", "qux"];
        let stats = extract_column_stats(&values).unwrap();

        // starts_upper: "Hello" (H), "WORLD" (W), "Bar" (B), "BAZ" (B) = 4/6
        assert!(
            (stat(&stats, "frac_starts_upper") - 4.0 / 6.0).abs() < 0.001,
            "Expected frac_starts_upper ~0.667, got {}",
            stat(&stats, "frac_starts_upper")
        );

        // all_upper: "WORLD", "BAZ" = 2/6
        assert!(
            (stat(&stats, "frac_all_upper") - 2.0 / 6.0).abs() < 0.001,
            "Expected frac_all_upper ~0.333, got {}",
            stat(&stats, "frac_all_upper")
        );

        // all_lower: "foo", "qux" = 2/6
        assert!(
            (stat(&stats, "frac_all_lower") - 2.0 / 6.0).abs() < 0.001,
            "Expected frac_all_lower ~0.333, got {}",
            stat(&stats, "frac_all_lower")
        );

        // mixed_case: "Hello", "Bar" = 2/6
        assert!(
            (stat(&stats, "frac_mixed_case") - 2.0 / 6.0).abs() < 0.001,
            "Expected frac_mixed_case ~0.333, got {}",
            stat(&stats, "frac_mixed_case")
        );
    }

    // ─── Test 11: Character composition stats ────────────────────────────

    #[test]
    fn test_character_composition() {
        // "abc" -> 0 digits, 3 alpha, 0 special
        // "123" -> 3 digits, 0 alpha, 0 special
        // "a!b" -> 0 digits, 2 alpha, 1 special
        let values: Vec<&str> = vec!["abc", "123", "a!b"];
        let stats = extract_column_stats(&values).unwrap();

        // avg_digit_count: (0 + 3 + 0) / 3 = 1.0
        assert!((stat(&stats, "avg_digit_count") - 1.0).abs() < 0.001);

        // frac_alpha_cells: all 3 have alpha chars? "abc" yes, "123" no, "a!b" yes => 2/3
        assert!(
            (stat(&stats, "frac_alpha_cells") - 2.0 / 3.0).abs() < 0.001,
            "Expected frac_alpha_cells ~0.667, got {}",
            stat(&stats, "frac_alpha_cells")
        );

        // frac_special_cells: "a!b" has special => 1/3
        assert!(
            (stat(&stats, "frac_special_cells") - 1.0 / 3.0).abs() < 0.001,
            "Expected frac_special_cells ~0.333, got {}",
            stat(&stats, "frac_special_cells")
        );
    }

    // ─── Test 12: Word counts ────────────────────────────────────────────

    #[test]
    fn test_word_counts() {
        let values: Vec<&str> = vec!["one", "two words", "three word sentence"];
        let stats = extract_column_stats(&values).unwrap();

        // avg_word_count: (1 + 2 + 3) / 3 = 2.0
        assert!((stat(&stats, "avg_word_count") - 2.0).abs() < 0.001);

        // std_word_count: variance = ((1-2)^2 + (2-2)^2 + (3-2)^2) / 3 = 2/3, std = sqrt(2/3) ~ 0.8165
        assert!(
            (stat(&stats, "std_word_count") - 0.8165).abs() < 0.01,
            "Expected std_word_count ~0.8165, got {}",
            stat(&stats, "std_word_count")
        );
    }

    // ─── Test 13: Whitespace-only values ─────────────────────────────────

    #[test]
    fn test_whitespace_values() {
        // Whitespace-only values are non-empty but have 0 words
        let values: Vec<&str> = vec!["  ", "hello"];
        let stats = extract_column_stats(&values).unwrap();

        assert_eq!(stat(&stats, "frac_empty"), 0.0); // neither is empty
        // avg_word_count: (0 + 1) / 2 = 0.5
        assert!((stat(&stats, "avg_word_count") - 0.5).abs() < 0.001);
    }

    // ─── Test 14: Determinism ────────────────────────────────────────────

    #[test]
    fn test_deterministic() {
        let values: Vec<&str> = vec!["hello@world.com", "2024-01-15", "42", "New York"];
        let s1 = extract_column_stats(&values).unwrap();
        let s2 = extract_column_stats(&values).unwrap();
        assert_eq!(s1, s2);
    }
}
