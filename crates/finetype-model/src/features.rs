//! Deterministic feature extraction for text values.
//!
//! Extracts a fixed-size feature vector from a string value, organized in 3 tiers:
//!
//! - **Tier 1 — Parse tests** (10 binary features): structural format signals
//! - **Tier 2 — Character stats** (14 features): character counts and ratios
//! - **Tier 3 — Structural** (8 features): delimiter patterns and shape signals
//!
//! All features are deterministic: same input always produces the same output.
//! Designed for fusion with CharCNN at the classifier head (NNFT-248).

use std::collections::HashSet;

/// Total number of features in the feature vector.
pub const FEATURE_DIM: usize = 32;

/// Human-readable names for each feature index, for interpretability and debugging.
pub const FEATURE_NAMES: [&str; FEATURE_DIM] = [
    // Tier 1 — Parse tests (binary 0.0 / 1.0)
    "is_numeric",            // 0
    "is_integer",            // 1
    "is_float",              // 2
    "has_leading_zero",      // 3
    "has_at_sign",           // 4
    "has_protocol_prefix",   // 5
    "is_uuid_like",          // 6
    "is_hex_string",         // 7
    "has_iso_date_sep",      // 8
    "matches_phone_pattern", // 9
    // Tier 2 — Character stats (counts and ratios)
    "length",              // 10
    "digit_count",         // 11
    "alpha_count",         // 12
    "uppercase_count",     // 13
    "lowercase_count",     // 14
    "space_count",         // 15
    "symbol_count",        // 16
    "digit_ratio",         // 17
    "alpha_ratio",         // 18
    "uppercase_ratio",     // 19
    "unique_char_ratio",   // 20
    "max_digit_run",       // 21
    "max_alpha_run",       // 22
    "punctuation_density", // 23
    // Tier 3 — Structural (pattern-derived)
    "segment_count_dot",   // 24
    "segment_count_dash",  // 25
    "segment_count_slash", // 26
    "segment_count_space", // 27
    "has_mixed_case",      // 28
    "starts_with_digit",   // 29
    "ends_with_digit",     // 30
    "length_bucket",       // 31
];

/// Extract a fixed-size feature vector from a string value.
///
/// Returns `[f32; FEATURE_DIM]` — deterministic, allocation-free (except unique char set).
/// Typical runtime: <0.05ms per value.
///
/// # Example
///
/// ```
/// use finetype_model::features::{extract_features, FEATURE_DIM, FEATURE_NAMES};
///
/// let features = extract_features("john.doe@example.com");
/// assert_eq!(features.len(), FEATURE_DIM);
///
/// // has_at_sign should be 1.0
/// assert_eq!(features[4], 1.0);
/// // Feature names are available for debugging
/// assert_eq!(FEATURE_NAMES[4], "has_at_sign");
/// ```
pub fn extract_features(value: &str) -> [f32; FEATURE_DIM] {
    let mut f = [0.0f32; FEATURE_DIM];

    let len = value.len() as f32;
    let chars: Vec<char> = value.chars().collect();
    let char_count = chars.len();

    // ─── Character counting (single pass) ───────────────────────────────
    let mut digit_count: u32 = 0;
    let mut alpha_count: u32 = 0;
    let mut upper_count: u32 = 0;
    let mut lower_count: u32 = 0;
    let mut space_count: u32 = 0;
    let mut symbol_count: u32 = 0;

    // Run tracking
    let mut max_digit_run: u32 = 0;
    let mut cur_digit_run: u32 = 0;
    let mut max_alpha_run: u32 = 0;
    let mut cur_alpha_run: u32 = 0;

    for &c in &chars {
        if c.is_ascii_digit() {
            digit_count += 1;
            cur_digit_run += 1;
            if cur_digit_run > max_digit_run {
                max_digit_run = cur_digit_run;
            }
            cur_alpha_run = 0;
        } else {
            cur_digit_run = 0;
            if c.is_alphabetic() {
                alpha_count += 1;
                cur_alpha_run += 1;
                if cur_alpha_run > max_alpha_run {
                    max_alpha_run = cur_alpha_run;
                }
                if c.is_uppercase() {
                    upper_count += 1;
                } else if c.is_lowercase() {
                    lower_count += 1;
                }
            } else {
                cur_alpha_run = 0;
                if c.is_whitespace() {
                    space_count += 1;
                } else {
                    symbol_count += 1;
                }
            }
        }
    }

    // ─── Tier 1: Parse tests (binary) ───────────────────────────────────

    // 0: is_numeric — parseable as f64
    f[0] = if value.parse::<f64>().is_ok() {
        1.0
    } else {
        0.0
    };

    // 1: is_integer — parseable as i64
    f[1] = if value.parse::<i64>().is_ok() {
        1.0
    } else {
        0.0
    };

    // 2: is_float — contains '.' and parseable as f64
    f[2] = if value.contains('.') && value.parse::<f64>().is_ok() {
        1.0
    } else {
        0.0
    };

    // 3: has_leading_zero — starts with '0' followed by another digit
    // Critical for numeric_code vs postal_code disambiguation
    f[3] = if chars.len() >= 2 && chars[0] == '0' && chars[1].is_ascii_digit() {
        1.0
    } else {
        0.0
    };

    // 4: has_at_sign
    f[4] = if value.contains('@') { 1.0 } else { 0.0 };

    // 5: has_protocol_prefix
    f[5] = if has_protocol_prefix(value) { 1.0 } else { 0.0 };

    // 6: is_uuid_like — 8-4-4-4-12 hex pattern
    f[6] = if is_uuid_like(value) { 1.0 } else { 0.0 };

    // 7: is_hex_string — non-empty, all hex chars
    f[7] = if !value.is_empty() && value.chars().all(|c| c.is_ascii_hexdigit()) {
        1.0
    } else {
        0.0
    };

    // 8: has_iso_date_sep — 'T' between digits (ISO 8601 signal)
    f[8] = if has_iso_date_separator(value) {
        1.0
    } else {
        0.0
    };

    // 9: matches_phone_pattern — starts with '+' and digits, or has parenthesized area code
    f[9] = if matches_phone_pattern(value) {
        1.0
    } else {
        0.0
    };

    // ─── Tier 2: Character stats ────────────────────────────────────────

    // 10: length (raw byte length)
    f[10] = len;

    // 11-16: character counts
    f[11] = digit_count as f32;
    f[12] = alpha_count as f32;
    f[13] = upper_count as f32;
    f[14] = lower_count as f32;
    f[15] = space_count as f32;
    f[16] = symbol_count as f32;

    // 17-20: ratios (safe division)
    let char_count_f = char_count.max(1) as f32;
    f[17] = digit_count as f32 / char_count_f; // digit_ratio
    f[18] = alpha_count as f32 / char_count_f; // alpha_ratio
    f[19] = upper_count as f32 / alpha_count.max(1) as f32; // uppercase_ratio

    // 20: unique_char_ratio
    let unique_chars: HashSet<char> = chars.iter().copied().collect();
    f[20] = unique_chars.len() as f32 / char_count_f;

    // 21-22: max runs
    f[21] = max_digit_run as f32;
    f[22] = max_alpha_run as f32;

    // 23: punctuation_density
    f[23] = symbol_count as f32 / char_count_f;

    // ─── Tier 3: Structural ─────────────────────────────────────────────

    // 24-27: segment counts by delimiter
    f[24] = value.split('.').count() as f32;
    f[25] = value.split('-').count() as f32;
    f[26] = value.split('/').count() as f32;
    f[27] = value.split_whitespace().count() as f32;

    // 28: has_mixed_case — both upper and lower present
    f[28] = if upper_count > 0 && lower_count > 0 {
        1.0
    } else {
        0.0
    };

    // 29: starts_with_digit
    f[29] = if chars.first().is_some_and(|c| c.is_ascii_digit()) {
        1.0
    } else {
        0.0
    };

    // 30: ends_with_digit
    f[30] = if chars.last().is_some_and(|c| c.is_ascii_digit()) {
        1.0
    } else {
        0.0
    };

    // 31: length_bucket — log2(length+1) for scale-invariant length signal
    f[31] = (char_count as f32 + 1.0).log2();

    f
}

// ═══════════════════════════════════════════════════════════════════════════════
// Helper functions
// ═══════════════════════════════════════════════════════════════════════════════

/// Check for common protocol prefixes.
fn has_protocol_prefix(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("ftp://")
        || lower.starts_with("ftps://")
        || lower.starts_with("ssh://")
        || lower.starts_with("s3://")
}

/// Check if value looks like a UUID (8-4-4-4-12 hex pattern).
fn is_uuid_like(value: &str) -> bool {
    let parts: Vec<&str> = value.split('-').collect();
    if parts.len() != 5 {
        return false;
    }
    let expected_lens = [8, 4, 4, 4, 12];
    parts
        .iter()
        .zip(expected_lens.iter())
        .all(|(part, &expected_len)| {
            part.len() == expected_len && part.chars().all(|c| c.is_ascii_hexdigit())
        })
}

/// Check for ISO 8601 'T' separator between digits (e.g., "2024-01-15T10:30:00").
fn has_iso_date_separator(value: &str) -> bool {
    if let Some(t_pos) = value.find('T') {
        // Must have a digit before and after the 'T'
        let before = value.as_bytes().get(t_pos.wrapping_sub(1));
        let after = value.as_bytes().get(t_pos + 1);
        matches!((before, after), (Some(b), Some(a)) if b.is_ascii_digit() && a.is_ascii_digit())
    } else {
        false
    }
}

/// Check for phone number patterns:
/// - Starts with '+' followed by digits
/// - Has parenthesized area code like (123)
fn matches_phone_pattern(value: &str) -> bool {
    let trimmed = value.trim();
    // Pattern 1: international format +XX...
    if let Some(rest) = trimmed.strip_prefix('+') {
        // At least some digits after the +
        return rest.chars().take(3).any(|c| c.is_ascii_digit());
    }
    // Pattern 2: parenthesized area code (XXX)
    if trimmed.starts_with('(') {
        if let Some(close) = trimmed.find(')') {
            let inner = &trimmed[1..close];
            return inner.len() >= 2 && inner.chars().all(|c| c.is_ascii_digit());
        }
    }
    false
}

// ═══════════════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to get a named feature value.
    fn feat(features: &[f32; FEATURE_DIM], name: &str) -> f32 {
        let idx = FEATURE_NAMES
            .iter()
            .position(|&n| n == name)
            .unwrap_or_else(|| panic!("Unknown feature: {}", name));
        features[idx]
    }

    #[test]
    fn test_feature_dim_matches_names() {
        assert_eq!(FEATURE_NAMES.len(), FEATURE_DIM);
        // All names should be unique
        let unique: HashSet<&str> = FEATURE_NAMES.iter().copied().collect();
        assert_eq!(unique.len(), FEATURE_DIM);
    }

    // ─── Tier 1: Parse tests ────────────────────────────────────────────

    #[test]
    fn test_numeric_features() {
        let f = extract_features("42");
        assert_eq!(feat(&f, "is_numeric"), 1.0);
        assert_eq!(feat(&f, "is_integer"), 1.0);
        assert_eq!(feat(&f, "is_float"), 0.0);
        assert_eq!(feat(&f, "has_leading_zero"), 0.0);

        let f = extract_features("3.14");
        assert_eq!(feat(&f, "is_numeric"), 1.0);
        assert_eq!(feat(&f, "is_integer"), 0.0);
        assert_eq!(feat(&f, "is_float"), 1.0);

        let f = extract_features("hello");
        assert_eq!(feat(&f, "is_numeric"), 0.0);
        assert_eq!(feat(&f, "is_integer"), 0.0);
    }

    #[test]
    fn test_leading_zero() {
        // numeric_code: "007", "0123", "00501"
        assert_eq!(feat(&extract_features("007"), "has_leading_zero"), 1.0);
        assert_eq!(feat(&extract_features("0123"), "has_leading_zero"), 1.0);
        assert_eq!(feat(&extract_features("00501"), "has_leading_zero"), 1.0);

        // Not leading zero: "0", "0.5", "100"
        assert_eq!(feat(&extract_features("0"), "has_leading_zero"), 0.0);
        assert_eq!(feat(&extract_features("0.5"), "has_leading_zero"), 0.0);
        assert_eq!(feat(&extract_features("100"), "has_leading_zero"), 0.0);
    }

    #[test]
    fn test_email_features() {
        let f = extract_features("user@example.com");
        assert_eq!(feat(&f, "has_at_sign"), 1.0);
        assert_eq!(feat(&f, "has_protocol_prefix"), 0.0);
        assert!(feat(&f, "segment_count_dot") >= 2.0);
    }

    #[test]
    fn test_url_features() {
        let f = extract_features("https://example.com/path");
        assert_eq!(feat(&f, "has_protocol_prefix"), 1.0);
        assert!(feat(&f, "segment_count_slash") >= 3.0);
    }

    #[test]
    fn test_uuid_features() {
        let f = extract_features("550e8400-e29b-41d4-a716-446655440000");
        assert_eq!(feat(&f, "is_uuid_like"), 1.0);
        assert!(feat(&f, "segment_count_dash") >= 5.0);

        // Not a UUID
        assert_eq!(feat(&extract_features("not-a-uuid"), "is_uuid_like"), 0.0);
    }

    #[test]
    fn test_hex_string() {
        assert_eq!(feat(&extract_features("deadbeef"), "is_hex_string"), 1.0);
        assert_eq!(feat(&extract_features("ABCDEF01"), "is_hex_string"), 1.0);
        assert_eq!(feat(&extract_features("xyz123"), "is_hex_string"), 0.0);
        assert_eq!(feat(&extract_features(""), "is_hex_string"), 0.0);
    }

    #[test]
    fn test_iso_date_separator() {
        assert_eq!(
            feat(&extract_features("2024-01-15T10:30:00"), "has_iso_date_sep"),
            1.0
        );
        assert_eq!(
            feat(&extract_features("2024-01-15"), "has_iso_date_sep"),
            0.0
        );
        assert_eq!(
            feat(&extract_features("The quick"), "has_iso_date_sep"),
            0.0
        );
    }

    #[test]
    fn test_phone_pattern() {
        assert_eq!(
            feat(
                &extract_features("+1-555-123-4567"),
                "matches_phone_pattern"
            ),
            1.0
        );
        assert_eq!(
            feat(
                &extract_features("+44 20 7946 0958"),
                "matches_phone_pattern"
            ),
            1.0
        );
        assert_eq!(
            feat(&extract_features("(555) 123-4567"), "matches_phone_pattern"),
            1.0
        );
        assert_eq!(
            feat(&extract_features("12345"), "matches_phone_pattern"),
            0.0
        );
    }

    // ─── Tier 2: Character stats ────────────────────────────────────────

    #[test]
    fn test_character_counts() {
        let f = extract_features("Hello World 123!");
        assert_eq!(feat(&f, "alpha_count"), 10.0);
        assert_eq!(feat(&f, "digit_count"), 3.0);
        assert_eq!(feat(&f, "uppercase_count"), 2.0);
        assert_eq!(feat(&f, "lowercase_count"), 8.0);
        assert_eq!(feat(&f, "space_count"), 2.0);
        assert_eq!(feat(&f, "symbol_count"), 1.0); // !
    }

    #[test]
    fn test_ratios() {
        let f = extract_features("1234");
        assert_eq!(feat(&f, "digit_ratio"), 1.0);
        assert_eq!(feat(&f, "alpha_ratio"), 0.0);

        let f = extract_features("abcd");
        assert_eq!(feat(&f, "digit_ratio"), 0.0);
        assert_eq!(feat(&f, "alpha_ratio"), 1.0);
    }

    #[test]
    fn test_max_runs() {
        let f = extract_features("abc123def45");
        assert_eq!(feat(&f, "max_digit_run"), 3.0); // "123"
        assert_eq!(feat(&f, "max_alpha_run"), 3.0); // "abc" or "def"
    }

    #[test]
    fn test_empty_string() {
        let f = extract_features("");
        assert_eq!(feat(&f, "length"), 0.0);
        assert_eq!(feat(&f, "digit_ratio"), 0.0);
        assert_eq!(feat(&f, "alpha_ratio"), 0.0);
        assert_eq!(feat(&f, "is_hex_string"), 0.0);
    }

    // ─── Tier 3: Structural ─────────────────────────────────────────────

    #[test]
    fn test_segment_counts() {
        let f = extract_features("192.168.1.1");
        assert_eq!(feat(&f, "segment_count_dot"), 4.0);
        assert_eq!(feat(&f, "segment_count_dash"), 1.0);

        let f = extract_features("2024-01-15");
        assert_eq!(feat(&f, "segment_count_dash"), 3.0);

        let f = extract_features("path/to/file");
        assert_eq!(feat(&f, "segment_count_slash"), 3.0);
    }

    #[test]
    fn test_mixed_case() {
        assert_eq!(feat(&extract_features("Hello"), "has_mixed_case"), 1.0);
        assert_eq!(feat(&extract_features("HELLO"), "has_mixed_case"), 0.0);
        assert_eq!(feat(&extract_features("hello"), "has_mixed_case"), 0.0);
        assert_eq!(feat(&extract_features("12345"), "has_mixed_case"), 0.0);
    }

    #[test]
    fn test_digit_boundaries() {
        let f = extract_features("2024-01-15");
        assert_eq!(feat(&f, "starts_with_digit"), 1.0);
        assert_eq!(feat(&f, "ends_with_digit"), 1.0);

        let f = extract_features("hello");
        assert_eq!(feat(&f, "starts_with_digit"), 0.0);
        assert_eq!(feat(&f, "ends_with_digit"), 0.0);
    }

    #[test]
    fn test_length_bucket() {
        // length_bucket = log2(char_count + 1)
        let f = extract_features("a"); // log2(2) = 1.0
        assert!((feat(&f, "length_bucket") - 1.0).abs() < 0.01);

        let f = extract_features("abcdefg"); // log2(8) = 3.0
        assert!((feat(&f, "length_bucket") - 3.0).abs() < 0.01);
    }

    // ─── Cross-domain representative values ─────────────────────────────

    #[test]
    fn test_datetime_value() {
        let f = extract_features("2024-01-15T10:30:00Z");
        assert_eq!(feat(&f, "has_iso_date_sep"), 1.0);
        assert_eq!(feat(&f, "starts_with_digit"), 1.0);
        assert!(feat(&f, "segment_count_dash") >= 3.0);
    }

    #[test]
    fn test_identity_value() {
        let f = extract_features("john.doe@example.com");
        assert_eq!(feat(&f, "has_at_sign"), 1.0);
        assert_eq!(feat(&f, "has_mixed_case"), 0.0);
        assert!(feat(&f, "segment_count_dot") >= 3.0);
    }

    #[test]
    fn test_geography_value() {
        let f = extract_features("New York City");
        assert_eq!(feat(&f, "has_mixed_case"), 1.0);
        assert_eq!(feat(&f, "is_numeric"), 0.0);
        assert!(feat(&f, "space_count") >= 2.0);
    }

    #[test]
    fn test_finance_value() {
        let f = extract_features("$1,234.56");
        assert_eq!(feat(&f, "is_numeric"), 0.0); // has $ and ,
        assert!(feat(&f, "digit_count") >= 6.0);
        assert!(feat(&f, "symbol_count") >= 2.0); // $ and ,
    }

    #[test]
    fn test_technology_value() {
        let f = extract_features("550e8400-e29b-41d4-a716-446655440000");
        assert_eq!(feat(&f, "is_uuid_like"), 1.0);
        assert_eq!(feat(&f, "has_mixed_case"), 0.0); // all lowercase hex
    }

    #[test]
    fn test_representation_value() {
        // Boolean
        let f = extract_features("true");
        assert_eq!(feat(&f, "is_numeric"), 0.0);
        assert_eq!(feat(&f, "alpha_ratio"), 1.0);

        // Hex color — uppercase hex has no lowercase, so no mixed case
        let f = extract_features("#FF5733");
        assert_eq!(feat(&f, "has_mixed_case"), 0.0);
        assert!(feat(&f, "symbol_count") >= 1.0); // #

        // Mixed-case hex color
        let f = extract_features("#ff5733Ab");
        assert_eq!(feat(&f, "has_mixed_case"), 1.0);
    }

    // ─── Determinism ────────────────────────────────────────────────────

    #[test]
    fn test_deterministic() {
        let values = [
            "hello@world.com",
            "2024-01-15T10:30:00",
            "+1-555-123-4567",
            "007",
            "550e8400-e29b-41d4-a716-446655440000",
            "https://example.com",
            "",
            "42",
        ];
        for value in &values {
            let f1 = extract_features(value);
            let f2 = extract_features(value);
            assert_eq!(f1, f2, "Non-deterministic for: {}", value);
        }
    }

    // ─── Performance ────────────────────────────────────────────────────

    #[test]
    fn test_performance_10k_values() {
        let values: Vec<String> = (0..10_000)
            .map(|i| format!("test-value-{}-example@domain.com", i))
            .collect();

        let start = std::time::Instant::now();
        for v in &values {
            let _ = extract_features(v);
        }
        let elapsed = start.elapsed();

        // 10k values should complete in <1 second (0.1ms/value budget)
        assert!(
            elapsed.as_secs_f64() < 1.0,
            "10k extractions took {:.3}s — exceeds 1s budget",
            elapsed.as_secs_f64()
        );
    }
}
