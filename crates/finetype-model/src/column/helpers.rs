//! Small stateless predicates shared by the sharpen/disambiguate modules —
//! pure value/label checks with no dependency on the classifier types.

/// Check if two labels are both present in the top candidates.
pub(crate) fn contains_pair(labels: &[&str], a: &str, b: &str) -> bool {
    labels.contains(&a) && labels.contains(&b)
}
/// Detect IPv4 addresses via dotted-quad pattern.
///
/// Check whether a value matches HS code format: 4+ digits optionally separated
/// into 2-digit groups by dots. Valid: "8471.30", "847130", "8471.30.00".
/// Invalid: "3.14", "0.887", "-12.5" (plain decimals).
pub(crate) fn is_hs_code_format(s: &str) -> bool {
    let s = s.trim();
    if s.is_empty() {
        return false;
    }
    // HS codes are never negative
    if s.starts_with('-') || s.starts_with('+') {
        return false;
    }
    // Split on dots and check structure
    let parts: Vec<&str> = s.split('.').collect();
    match parts.len() {
        // No dots: must be 4-10 pure digits (e.g., "847130")
        1 => {
            let len = parts[0].len();
            (4..=10).contains(&len) && parts[0].chars().all(|c| c.is_ascii_digit())
        }
        // Dotted: first group 4 digits, subsequent groups exactly 2 digits
        2..=4 => {
            parts[0].len() == 4
                && parts[0].chars().all(|c| c.is_ascii_digit())
                && parts[1..]
                    .iter()
                    .all(|p| p.len() == 2 && p.chars().all(|c| c.is_ascii_digit()))
        }
        _ => false,
    }
}
/// Check whether a value matches ISIN format: 2-letter country code followed by
/// 9 alphanumeric characters and 1 check digit. Total 12 characters.
/// Valid: "US0378331005", "GB0002634946", "DE0007164600".
/// Invalid: "US-Z03-98-12345" (ISRC format with dashes).
///
/// Key distinction from ISRC (also 12-char, 2-letter prefix): ISIN positions 2-4
/// are the start of the NSIN and are typically all digits (e.g., US**037**8331005).
/// ISRC positions 2-4 are the registrant code and typically contain letters
/// (e.g., SE**3YX**3859059). Requiring all-digit positions 2-4 avoids false matches
/// on ISRC values.
pub(crate) fn is_isin_format(s: &str) -> bool {
    let s = s.trim();
    if s.len() != 12 {
        return false;
    }
    let bytes = s.as_bytes();
    // First 2 chars: uppercase letters (country code)
    if !bytes[0].is_ascii_uppercase() || !bytes[1].is_ascii_uppercase() {
        return false;
    }
    // Positions 2-4: must be all digits (distinguishes from ISRC registrant codes)
    if !bytes[2..5].iter().all(|b| b.is_ascii_digit()) {
        return false;
    }
    // Remaining positions 5-10: alphanumeric
    if !bytes[5..11].iter().all(|b| b.is_ascii_alphanumeric()) {
        return false;
    }
    // Last char: digit (check digit)
    bytes[11].is_ascii_digit()
}
/// Check whether a value matches ISSN format: DDDD-DDD[DX] (4 digits, dash,
/// 3 digits, 1 digit or 'X'). Total 9 characters with dash at position 4.
/// Valid: "1781-2253", "8371-5342", "0317-839X".
/// Distinguishes from EIN format: DD-DDDDDDD (2 digits, dash, 7 digits).
pub(crate) fn is_issn_format(s: &str) -> bool {
    let s = s.trim();
    let bytes = s.as_bytes();
    // Exactly 9 chars with dash at position 4
    bytes.len() == 9
        && bytes[4] == b'-'
        && bytes[..4].iter().all(|b| b.is_ascii_digit())
        && bytes[5..8].iter().all(|b| b.is_ascii_digit())
        && (bytes[8].is_ascii_digit() || bytes[8] == b'X')
}
