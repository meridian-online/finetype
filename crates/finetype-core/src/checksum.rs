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

/// Validate a US ABA routing transit number by its check digit.
///
/// Nine digits; the weighted sum `3·d₁ + 7·d₂ + 1·d₃` repeating is divisible by
/// 10. The taxonomy pattern checks the Federal Reserve prefix and digit count
/// but not the checksum, so a 9-digit financial integer with a plausible prefix
/// otherwise passes. Hyphens are stripped; a leading sign disqualifies; any
/// length other than 9, or a non-digit, fails.
pub fn aba(value: &str) -> bool {
    let t = value.trim();
    if t.starts_with('-') || t.starts_with('+') {
        return false;
    }
    let digits: Vec<char> = t.chars().filter(|c| *c != '-').collect();
    if digits.len() != 9 {
        return false;
    }
    const WEIGHTS: [u32; 9] = [3, 7, 1, 3, 7, 1, 3, 7, 1];
    let mut sum = 0u32;
    for (i, c) in digits.iter().enumerate() {
        let Some(d) = c.to_digit(10) else {
            return false;
        };
        sum += WEIGHTS[i] * d;
    }
    sum.is_multiple_of(10)
}

/// Character value for CUSIP/SEDOL check digits: digits 0–9, letters A–Z =
/// 10–35. CUSIP also allows `*`/`@`/`#` (36–38). Returns `None` for anything
/// else.
fn alnum_value(c: char) -> Option<u32> {
    if c.is_ascii_digit() {
        c.to_digit(10)
    } else if c.is_ascii_uppercase() {
        Some(c as u32 - 'A' as u32 + 10)
    } else {
        match c {
            '*' => Some(36),
            '@' => Some(37),
            '#' => Some(38),
            _ => None,
        }
    }
}

/// Validate a 9-character CUSIP by its check digit.
///
/// First 8 characters are the issuer + issue; the 9th is a check digit. Each of
/// the first 8 is converted to a value (digits 0–9, A–Z = 10–35); values at even
/// 1-based positions are doubled; the digits of each result are summed; the
/// check digit is `(10 − sum mod 10) mod 10`. The taxonomy pattern checks shape
/// (`^[A-Z0-9]{8}[0-9]$`) but not the check digit.
pub fn cusip(value: &str) -> bool {
    let chars: Vec<char> = value.trim().chars().collect();
    if chars.len() != 9 {
        return false;
    }
    let mut sum = 0u32;
    for (i, &c) in chars[..8].iter().enumerate() {
        let Some(mut v) = alnum_value(c) else {
            return false;
        };
        if i % 2 == 1 {
            v *= 2;
        }
        sum += v / 10 + v % 10;
    }
    let Some(check) = chars[8].to_digit(10) else {
        return false;
    };
    check == (10 - sum % 10) % 10
}

/// Validate a 7-character SEDOL by its check digit.
///
/// First 6 characters (consonants + digits, no vowels) carry weights
/// `1,3,1,7,3,9`; the 7th is a check digit `(10 − weighted sum mod 10) mod 10`.
/// Each character's value is digit 0–9 or letter A–Z = 10–35. The taxonomy
/// pattern checks shape but not the check digit.
pub fn sedol(value: &str) -> bool {
    let chars: Vec<char> = value.trim().chars().collect();
    if chars.len() != 7 {
        return false;
    }
    const WEIGHTS: [u32; 6] = [1, 3, 1, 7, 3, 9];
    let mut sum = 0u32;
    for (i, &c) in chars[..6].iter().enumerate() {
        let Some(v) = alnum_value(c) else {
            return false;
        };
        sum += WEIGHTS[i] * v;
    }
    let Some(check) = chars[6].to_digit(10) else {
        return false;
    };
    check == (10 - sum % 10) % 10
}

/// Resolve a `checksum:` directive name to its validating function.
///
/// Returns `None` for an unknown name so the caller can surface the typo
/// (the validator turns this into a compile-time error). Add a match arm here
/// to enrol a new algo-exists type.
pub fn resolve(name: &str) -> Option<fn(&str) -> bool> {
    match name {
        "isbn" => Some(isbn),
        "aba" => Some(aba),
        "cusip" => Some(cusip),
        "sedol" => Some(sedol),
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
    fn aba_accepts_genuine_routing_numbers() {
        // Real US routing numbers (the taxonomy samples) — all pass the
        // weighted checksum.
        for v in [
            "021000021",
            "111000025",
            "021200025",
            "071000013",
            "011401533",
            "121000358",
        ] {
            assert!(aba(v), "should be a valid ABA routing number: {v}");
        }
    }

    #[test]
    fn aba_rejects_lookalike_numbers() {
        // A 9-digit financial integer with a plausible prefix that fails the
        // checksum (021000021 with the last digit bumped).
        assert!(!aba("021000022"));
        assert!(!aba("123456789")); // arbitrary 9-digit
        assert!(!aba("02100002")); // 8 digits
        assert!(!aba("0210000210")); // 10 digits
        assert!(!aba("-021000021")); // signed
        assert!(!aba("02100002X")); // non-digit
    }

    #[test]
    fn cusip_accepts_and_rejects() {
        for v in ["037833100", "17275R102", "594918104"] {
            assert!(cusip(v), "should be a valid CUSIP: {v}");
        }
        assert!(!cusip("037833101")); // check digit bumped
        assert!(!cusip("17275R103"));
        assert!(!cusip("03783310")); // 8 chars
        assert!(!cusip("037833100X")); // check digit not a digit
    }

    #[test]
    fn sedol_accepts_and_rejects() {
        for v in ["0263494", "B0WNLY7", "3134865"] {
            assert!(sedol(v), "should be a valid SEDOL: {v}");
        }
        assert!(!sedol("0263495")); // check digit bumped
        assert!(!sedol("B0WNLY8"));
        assert!(!sedol("026349")); // 6 chars
        assert!(!sedol("0263494X")); // 8 chars
    }

    #[test]
    fn resolve_known_and_unknown() {
        assert!(resolve("isbn").is_some());
        assert!(resolve("aba").is_some());
        assert!(resolve("cusip").is_some());
        assert!(resolve("sedol").is_some());
        assert!(resolve("not_a_scheme").is_none());
    }
}
