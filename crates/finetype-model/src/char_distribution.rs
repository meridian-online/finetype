//! Column-level character distribution feature extractor.
//!
//! Computes Sherlock-style character distribution features for a column of string values.
//! For each of 96 printable ASCII characters (bytes 32–127, space through DEL), calculates 10 aggregation
//! statistics over per-value character frequencies:
//!
//! - `any`: whether the character appears in any value
//! - `all`: whether the character appears in all values
//! - `mean`, `variance`, `min`, `max`, `median`, `sum`: standard statistics
//! - `skewness`: Fisher's skewness (third standardized moment)
//! - `kurtosis`: excess kurtosis (fourth standardized moment minus 3)
//!
//! Output: `[f32; 960]` — deterministic, no external dependencies.

/// 96 printable ASCII characters x 10 statistics = 960 features.
pub const CHAR_DIST_DIM: usize = 960;

/// Number of printable ASCII characters (space through tilde).
const NUM_CHARS: usize = 96;

/// Number of aggregation statistics per character.
const NUM_STATS: usize = 10;

/// Stat indices within each character's 10-element block.
const STAT_ANY: usize = 0;
const STAT_ALL: usize = 1;
const STAT_MEAN: usize = 2;
const STAT_VARIANCE: usize = 3;
const STAT_MIN: usize = 4;
const STAT_MAX: usize = 5;
const STAT_MEDIAN: usize = 6;
const STAT_SUM: usize = 7;
const STAT_SKEWNESS: usize = 8;
const STAT_KURTOSIS: usize = 9;

/// Character distribution feature names for debugging/export.
///
/// Formatted as `"{char_label}_{stat}"`, e.g. `"space_any"`, `"a_mean"`, `"0_variance"`.
/// 960 entries: 96 chars x 10 stats, laid out as char0_any, char0_all, ..., char95_kurtosis.
pub const CHAR_DIST_NAMES: [&str; CHAR_DIST_DIM] = {
    let mut names = [""; CHAR_DIST_DIM];
    let mut ci = 0;
    while ci < NUM_CHARS {
        let mut si = 0;
        while si < NUM_STATS {
            names[ci * NUM_STATS + si] = const_feature_name(ci, si);
            si += 1;
        }
        ci += 1;
    }
    names
};

/// Generate a feature name at compile time.
///
/// We use a lookup table to avoid runtime string allocation. The names are
/// statically allocated string slices built from the 96 x 10 combinations.
const fn const_feature_name(char_idx: usize, stat_idx: usize) -> &'static str {
    // This is a compile-time lookup table. We generate all 960 names as static strings.
    // Since const fn cannot do string concatenation, we use a flat lookup table.
    FEATURE_NAME_TABLE[char_idx * NUM_STATS + stat_idx]
}

// Include the generated name table. This is a 960-entry array of &str built at compile time.
// Rather than attempting const string concatenation, we list them explicitly via a macro.
macro_rules! char_stat_names {
    ($($label:expr),+ $(,)?) => {
        [$( // for each label, emit 10 stat names
            concat!($label, "_any"),
            concat!($label, "_all"),
            concat!($label, "_mean"),
            concat!($label, "_variance"),
            concat!($label, "_min"),
            concat!($label, "_max"),
            concat!($label, "_median"),
            concat!($label, "_sum"),
            concat!($label, "_skewness"),
            concat!($label, "_kurtosis"),
        )+]
    };
}

const FEATURE_NAME_TABLE: [&str; CHAR_DIST_DIM] = char_stat_names![
    "space",
    "exclamation",
    "double_quote",
    "hash",
    "dollar",
    "percent",
    "ampersand",
    "single_quote",
    "open_paren",
    "close_paren",
    "asterisk",
    "plus",
    "comma",
    "dash",
    "period",
    "slash",
    "0",
    "1",
    "2",
    "3",
    "4",
    "5",
    "6",
    "7",
    "8",
    "9",
    "colon",
    "semicolon",
    "less_than",
    "equals",
    "greater_than",
    "question",
    "at",
    "A",
    "B",
    "C",
    "D",
    "E",
    "F",
    "G",
    "H",
    "I",
    "J",
    "K",
    "L",
    "M",
    "N",
    "O",
    "P",
    "Q",
    "R",
    "S",
    "T",
    "U",
    "V",
    "W",
    "X",
    "Y",
    "Z",
    "open_bracket",
    "backslash",
    "close_bracket",
    "caret",
    "underscore",
    "backtick",
    "a",
    "b",
    "c",
    "d",
    "e",
    "f",
    "g",
    "h",
    "i",
    "j",
    "k",
    "l",
    "m",
    "n",
    "o",
    "p",
    "q",
    "r",
    "s",
    "t",
    "u",
    "v",
    "w",
    "x",
    "y",
    "z",
    "open_brace",
    "pipe",
    "close_brace",
    "tilde",
    "del",
];

/// Extract 960-dim character distribution features from a column of values.
///
/// For each of 96 printable ASCII characters (bytes 32–127, space through DEL), computes per-value
/// character frequencies and then aggregates them into 10 column-level statistics.
///
/// Returns `None` if `values` is empty.
///
/// # Example
///
/// ```
/// use finetype_model::char_distribution::{extract_char_distribution, CHAR_DIST_DIM};
///
/// let values = vec!["hello", "world", "test"];
/// let features = extract_char_distribution(&values).unwrap();
/// assert_eq!(features.len(), CHAR_DIST_DIM);
/// ```
pub fn extract_char_distribution(values: &[&str]) -> Option<[f32; CHAR_DIST_DIM]> {
    if values.is_empty() {
        return None;
    }

    let n = values.len();
    let n_f = n as f64;

    // Pre-compute per-value character frequencies: frequencies[char_idx][value_idx]
    // For each value, frequency of char c = count(c in value) / len(value), or 0.0 if empty.
    let mut frequencies = vec![vec![0.0f64; n]; NUM_CHARS];

    for (vi, value) in values.iter().enumerate() {
        let len = value.len();
        if len == 0 {
            continue; // all frequencies stay 0.0
        }
        let len_f = len as f64;
        for &byte in value.as_bytes() {
            if (32..=127).contains(&byte) {
                let ci = (byte - 32) as usize;
                frequencies[ci][vi] += 1.0 / len_f;
            }
        }
    }

    let mut features = [0.0f32; CHAR_DIST_DIM];

    for (ci, freqs) in frequencies.iter_mut().enumerate() {
        let base = ci * NUM_STATS;

        // any: 1.0 if char appears in any value
        let any_present = freqs.iter().any(|&f| f > 0.0);
        features[base + STAT_ANY] = if any_present { 1.0 } else { 0.0 };

        // all: 1.0 if char appears in all values
        let all_present = freqs.iter().all(|&f| f > 0.0);
        features[base + STAT_ALL] = if all_present { 1.0 } else { 0.0 };

        // sum
        let sum: f64 = freqs.iter().sum();
        features[base + STAT_SUM] = sum as f32;

        // mean
        let mean = sum / n_f;
        features[base + STAT_MEAN] = mean as f32;

        // min, max
        let mut min_val = f64::INFINITY;
        let mut max_val = f64::NEG_INFINITY;
        for &f in freqs.iter() {
            if f < min_val {
                min_val = f;
            }
            if f > max_val {
                max_val = f;
            }
        }
        features[base + STAT_MIN] = min_val as f32;
        features[base + STAT_MAX] = max_val as f32;

        // median (sort a copy)
        freqs.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let median = if n % 2 == 1 {
            freqs[n / 2]
        } else {
            (freqs[n / 2 - 1] + freqs[n / 2]) / 2.0
        };
        features[base + STAT_MEDIAN] = median as f32;

        // variance (population), skewness, kurtosis
        let variance = freqs.iter().map(|&f| (f - mean).powi(2)).sum::<f64>() / n_f;
        features[base + STAT_VARIANCE] = variance as f32;

        let (skew, kurt) = skewness_kurtosis(freqs, mean, variance, n_f);
        features[base + STAT_SKEWNESS] = skew as f32;
        features[base + STAT_KURTOSIS] = kurt as f32;
    }

    Some(features)
}

/// Compute Fisher's skewness and excess kurtosis from pre-computed mean and variance.
///
/// - Skewness = E[(x - mean)^3] / stddev^3 (0.0 if variance near zero)
/// - Excess kurtosis = E[(x - mean)^4] / variance^2 - 3 (0.0 if variance near zero)
fn skewness_kurtosis(values: &[f64], mean: f64, variance: f64, n: f64) -> (f64, f64) {
    const EPSILON: f64 = 1e-14;

    if variance < EPSILON {
        return (0.0, 0.0);
    }

    let m3: f64 = values.iter().map(|&x| (x - mean).powi(3)).sum::<f64>() / n;
    let m4: f64 = values.iter().map(|&x| (x - mean).powi(4)).sum::<f64>() / n;

    let stddev = variance.sqrt();
    let skewness = m3 / (stddev * stddev * stddev);
    let kurtosis = m4 / (variance * variance) - 3.0;

    (skewness, kurtosis)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to get a named feature value from the 960-dim vector.
    fn feat(features: &[f32; CHAR_DIST_DIM], name: &str) -> f32 {
        let idx = CHAR_DIST_NAMES
            .iter()
            .position(|&n| n == name)
            .unwrap_or_else(|| panic!("Unknown feature: {}", name));
        features[idx]
    }

    // ─── Test 1: Dimensions ──────────────────────────────────────────────

    #[test]
    fn test_dimensions() {
        assert_eq!(CHAR_DIST_DIM, 960);
        assert_eq!(CHAR_DIST_NAMES.len(), CHAR_DIST_DIM);
        assert_eq!(NUM_CHARS * NUM_STATS, CHAR_DIST_DIM);
    }

    #[test]
    fn test_feature_names_unique() {
        let mut seen = std::collections::HashSet::new();
        for name in &CHAR_DIST_NAMES {
            assert!(seen.insert(name), "Duplicate feature name: {}", name);
        }
    }

    #[test]
    fn test_feature_names_format() {
        // Spot check some names
        assert_eq!(CHAR_DIST_NAMES[0], "space_any");
        assert_eq!(CHAR_DIST_NAMES[1], "space_all");
        assert_eq!(CHAR_DIST_NAMES[9], "space_kurtosis");
        assert_eq!(CHAR_DIST_NAMES[10], "exclamation_any");
        // Last entry
        assert_eq!(CHAR_DIST_NAMES[959], "del_kurtosis");
    }

    // ─── Test 2: Known 10-value column ───────────────────────────────────

    #[test]
    fn test_known_column_spot_check() {
        // 10 values with known character distributions
        let values: Vec<&str> = vec![
            "aaa", "aab", "abc", "bbb", "bbc", "ccc", "cca", "aac", "bba", "cab",
        ];
        let features = extract_char_distribution(&values).unwrap();

        // Character 'a': appears in values 0,1,2,6,7,9 = 8 of 10 values
        // Frequencies: "aaa"=1.0, "aab"=2/3, "abc"=1/3, "bbb"=0, "bbc"=0,
        //              "ccc"=0, "cca"=1/3, "aac"=2/3, "bba"=1/3, "cab"=1/3
        let a_freqs: Vec<f64> = vec![
            1.0,
            2.0 / 3.0,
            1.0 / 3.0,
            0.0,
            0.0,
            0.0,
            1.0 / 3.0,
            2.0 / 3.0,
            1.0 / 3.0,
            1.0 / 3.0,
        ];

        // any: 'a' appears in at least one value
        assert_eq!(feat(&features, "a_any"), 1.0);
        // all: 'a' does NOT appear in all values (bbb, bbc, ccc)
        assert_eq!(feat(&features, "a_all"), 0.0);

        // mean
        let expected_mean: f64 = a_freqs.iter().sum::<f64>() / 10.0;
        assert!(
            (feat(&features, "a_mean") as f64 - expected_mean).abs() < 1e-5,
            "a_mean: got {}, expected {}",
            feat(&features, "a_mean"),
            expected_mean
        );

        // sum
        let expected_sum: f64 = a_freqs.iter().sum();
        assert!(
            (feat(&features, "a_sum") as f64 - expected_sum).abs() < 1e-5,
            "a_sum: got {}, expected {}",
            feat(&features, "a_sum"),
            expected_sum
        );

        // min
        assert!(
            (feat(&features, "a_min") as f64).abs() < 1e-5,
            "a_min should be 0.0"
        );

        // max
        assert!(
            (feat(&features, "a_max") as f64 - 1.0).abs() < 1e-5,
            "a_max should be 1.0"
        );

        // variance (population)
        let expected_var: f64 = a_freqs
            .iter()
            .map(|f| (f - expected_mean).powi(2))
            .sum::<f64>()
            / 10.0;
        assert!(
            (feat(&features, "a_variance") as f64 - expected_var).abs() < 1e-5,
            "a_variance: got {}, expected {}",
            feat(&features, "a_variance"),
            expected_var
        );

        // median: sorted freqs = [0,0,0, 1/3,1/3,1/3,1/3, 2/3,2/3, 1.0]
        // median of 10 values = (freqs[4] + freqs[5]) / 2 = (1/3 + 1/3) / 2 = 1/3
        assert!(
            (feat(&features, "a_median") as f64 - 1.0 / 3.0).abs() < 1e-5,
            "a_median: got {}, expected {}",
            feat(&features, "a_median"),
            1.0 / 3.0
        );

        // Spot check '0' (digit zero, byte 48): none of our values contain '0'
        assert_eq!(feat(&features, "0_any"), 0.0);
        assert_eq!(feat(&features, "0_all"), 0.0);
        assert!((feat(&features, "0_mean") as f64).abs() < 1e-5);

        // Spot check space: none of our values contain spaces
        assert_eq!(feat(&features, "space_any"), 0.0);
        assert_eq!(feat(&features, "space_all"), 0.0);
    }

    // ─── Test 3: All-identical values ────────────────────────────────────

    #[test]
    fn test_all_identical_values() {
        let values: Vec<&str> = vec!["abc"; 5];
        let features = extract_char_distribution(&values).unwrap();

        // 'a' frequency in each value = 1/3, all identical
        assert_eq!(feat(&features, "a_any"), 1.0);
        assert_eq!(feat(&features, "a_all"), 1.0);
        assert!(
            (feat(&features, "a_mean") as f64 - 1.0 / 3.0).abs() < 1e-5,
            "a_mean"
        );
        assert!(
            (feat(&features, "a_variance") as f64).abs() < 1e-5,
            "a_variance should be 0 for identical values"
        );
        assert!(
            (feat(&features, "a_skewness") as f64).abs() < 1e-5,
            "a_skewness should be 0"
        );
        assert!(
            (feat(&features, "a_kurtosis") as f64).abs() < 1e-5,
            "a_kurtosis should be 0"
        );
        assert!(
            (feat(&features, "a_min") as f64 - 1.0 / 3.0).abs() < 1e-5,
            "a_min"
        );
        assert!(
            (feat(&features, "a_max") as f64 - 1.0 / 3.0).abs() < 1e-5,
            "a_max"
        );
    }

    // ─── Test 4: Column with empty strings ───────────────────────────────

    #[test]
    fn test_empty_strings() {
        let values: Vec<&str> = vec!["", "", ""];
        let features = extract_char_distribution(&values).unwrap();

        // All frequencies are 0.0 for every character
        for ci in 0..NUM_CHARS {
            let base = ci * NUM_STATS;
            assert_eq!(features[base + STAT_ANY], 0.0);
            assert_eq!(features[base + STAT_ALL], 0.0);
            assert_eq!(features[base + STAT_MEAN], 0.0);
            assert_eq!(features[base + STAT_VARIANCE], 0.0);
            assert_eq!(features[base + STAT_MIN], 0.0);
            assert_eq!(features[base + STAT_MAX], 0.0);
            assert_eq!(features[base + STAT_MEDIAN], 0.0);
            assert_eq!(features[base + STAT_SUM], 0.0);
            assert_eq!(features[base + STAT_SKEWNESS], 0.0);
            assert_eq!(features[base + STAT_KURTOSIS], 0.0);
        }
    }

    // ─── Test 5: Non-ASCII characters (emoji, CJK) ──────────────────────

    #[test]
    fn test_non_ascii_ignored() {
        // Values with emoji and CJK characters mixed with ASCII
        let values: Vec<&str> = vec!["hello\u{1F600}", "\u{4E16}\u{754C}ab", "\u{2764}xyz"];
        let features = extract_char_distribution(&values).unwrap();

        // Non-ASCII bytes are ignored. Only ASCII portions contribute.
        // "hello\u{1F600}": 'h','e','l','l','o' are ASCII (5 ASCII bytes out of 9 total bytes)
        // But frequency = count / len where len = total byte length including non-ASCII.
        // 'h' frequency = 1/9 for first value (UTF-8 encoded smiley is 4 bytes)

        // Just verify the function completes and produces valid output
        assert_eq!(features.len(), CHAR_DIST_DIM);

        // 'h' should appear in first value
        assert_eq!(feat(&features, "h_any"), 1.0);

        // Emoji/CJK chars (byte > 126) should not show up anywhere.
        // The function only tracks bytes 32–127, space through DEL, so high bytes are simply skipped.
        // Verify no NaN or Inf in output
        for &f in features.iter() {
            assert!(f.is_finite(), "Non-finite value in features");
        }
    }

    // ─── Test 6: Single-value column ─────────────────────────────────────

    #[test]
    fn test_single_value() {
        let values: Vec<&str> = vec!["test"];
        let features = extract_char_distribution(&values).unwrap();

        // 't' frequency = 2/4 = 0.5 (appears twice in "test")
        assert!(
            (feat(&features, "t_mean") as f64 - 0.5).abs() < 1e-5,
            "t_mean"
        );
        assert!(
            (feat(&features, "t_variance") as f64).abs() < 1e-5,
            "t_variance should be 0 for single value"
        );
        assert!(
            (feat(&features, "t_skewness") as f64).abs() < 1e-5,
            "t_skewness should be 0 for single value"
        );
        assert!(
            (feat(&features, "t_kurtosis") as f64).abs() < 1e-5,
            "t_kurtosis should be 0 for single value"
        );
        assert!(
            (feat(&features, "t_min") as f64 - 0.5).abs() < 1e-5,
            "t_min"
        );
        assert!(
            (feat(&features, "t_max") as f64 - 0.5).abs() < 1e-5,
            "t_max"
        );
        assert!(
            (feat(&features, "t_median") as f64 - 0.5).abs() < 1e-5,
            "t_median"
        );

        // 'e' frequency = 1/4 = 0.25
        assert!(
            (feat(&features, "e_mean") as f64 - 0.25).abs() < 1e-5,
            "e_mean"
        );
    }

    // ─── Test 7: Empty values slice ──────────────────────────────────────

    #[test]
    fn test_empty_slice_returns_none() {
        let values: Vec<&str> = vec![];
        assert!(extract_char_distribution(&values).is_none());
    }

    // ─── Determinism ────────────────────────────────────────────────────

    #[test]
    fn test_deterministic() {
        let values: Vec<&str> = vec!["hello", "world", "test123", "$1,234.56", ""];
        let f1 = extract_char_distribution(&values).unwrap();
        let f2 = extract_char_distribution(&values).unwrap();
        assert_eq!(f1, f2, "Features should be deterministic");
    }
}
