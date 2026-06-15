//! Substance checksums for algorithmic identifier types (Precision Principle).
//!
//! Many identifier types carry a self-validating check digit — a column of
//! arbitrary numbers the same *length* as an ISBN is not a column of ISBNs.
//! A length/format-only validation "confirms 90% of random input"; the
//! checksum is what distinguishes "is this type" from "is not this type".
//!
//! This module is the canonical home for that arithmetic, shared by the
//! validator (so the taxonomy's `checksum:` directive makes a type's
//! validation substance-checking) and by the model's post-sharpen
//! `checksum_substance_guard`. Previously the ISBN math lived hand-rolled
//! inside a per-type veto; here it is wired into the validator instead, and
//! each future algo-exists type (aba, luhn, npi, ean, upc, imei, …) enrols by
//! adding one entry to [`resolve`] plus a one-line `checksum:` directive in
//! its YAML — no new bespoke veto.
//!
//! Each function is a total `fn(&str) -> bool`: `true` when the value carries a
//! valid check digit for that scheme, `false` otherwise (including malformed
//! input). They are deliberately lenient about surrounding format — hyphens are
//! stripped — because the validator's `pattern` already constrains shape; the
//! checksum adds the substance check on top.

/// Validate an ISBN-10 or ISBN-13 by its check digit (not just digit count).
///
/// ISBN-10: weighted sum `Σ (i+1)·dᵢ` (i = 0..9, final digit may be `X` = 10)
/// is divisible by 11. ISBN-13: weighted sum with alternating 1/3 weights is
/// divisible by 10. Hyphens are stripped; a leading sign disqualifies (ISBNs
/// are never signed). Any other length, or a non-digit where a digit is
/// required, fails.
pub fn isbn(value: &str) -> bool {
    let t = value.trim();
    if t.starts_with('-') || t.starts_with('+') {
        return false;
    }
    let digits: Vec<char> = t.chars().filter(|c| *c != '-').collect();
    match digits.len() {
        10 => {
            let mut sum = 0u32;
            for (i, c) in digits.iter().enumerate() {
                let v = if i == 9 && (*c == 'X' || *c == 'x') {
                    10
                } else if let Some(d) = c.to_digit(10) {
                    d
                } else {
                    return false;
                };
                sum += (i as u32 + 1) * v;
            }
            sum.is_multiple_of(11)
        }
        13 => {
            let mut sum = 0u32;
            for (i, c) in digits.iter().enumerate() {
                let Some(d) = c.to_digit(10) else {
                    return false;
                };
                sum += if i.is_multiple_of(2) { d } else { 3 * d };
            }
            sum.is_multiple_of(10)
        }
        _ => false,
    }
}

/// Resolve a `checksum:` directive name to its validating function.
///
/// Returns `None` for an unknown name so the caller can surface the typo
/// (the validator turns this into a compile-time error). Add a match arm here
/// to enrol a new algo-exists type.
pub fn resolve(name: &str) -> Option<fn(&str) -> bool> {
    match name {
        "isbn" => Some(isbn),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn isbn_accepts_genuine_codes() {
        // ISBN-10, ISBN-10 with X check digit, ISBN-13, hyphenated ISBN-13.
        for v in [
            "0306406152",
            "043942089X",
            "9780306406157",
            "978-3-16-148410-0",
        ] {
            assert!(isbn(v), "should be a valid ISBN: {v}");
        }
    }

    #[test]
    fn isbn_rejects_lookalike_numbers() {
        // Financial figures the model has mislabelled as ISBN (marketCap and
        // friends) are 10/13 digits long but fail the check digit.
        for v in ["5150000128", "6965100000", "7586000000", "1041000000"] {
            assert!(!isbn(v), "should NOT be a valid ISBN: {v}");
        }
        // Leading sign disqualifies; wrong length fails.
        assert!(!isbn("-1617000000"));
        assert!(!isbn("12345678901")); // 11 digits
        assert!(!isbn("abcdefghij")); // non-digit
    }

    #[test]
    fn resolve_known_and_unknown() {
        assert!(resolve("isbn").is_some());
        assert!(resolve("not_a_scheme").is_none());
    }
}
