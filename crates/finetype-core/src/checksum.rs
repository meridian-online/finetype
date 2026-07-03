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

/// Alphanumeric value A–Z/0–9 for ISIN/LEI/IBAN: digits 0–9, letters A–Z =
/// 10–35. Returns `None` for anything else. (Unlike [`alnum_value`] this admits
/// no CUSIP `*`/`@`/`#` extras — these schemes are strictly `[A-Z0-9]`.)
fn alnum36(c: char) -> Option<u32> {
    if c.is_ascii_digit() {
        c.to_digit(10)
    } else if c.is_ascii_uppercase() {
        Some(c as u32 - 'A' as u32 + 10)
    } else {
        None
    }
}

/// Fold an alphanumeric slice to its ISO 7064 Mod 97-10 remainder.
///
/// Each digit contributes one decimal digit; each letter A–Z its two-digit
/// value (10–35). The whole is reduced mod 97 digit-by-digit, so the running
/// remainder is always < 97 and nothing overflows however long the input.
/// Returns `None` if any character is not `[A-Z0-9]`.
fn mod97(chars: &[char]) -> Option<u32> {
    let mut r = 0u32;
    for &c in chars {
        let v = alnum36(c)?;
        if v >= 10 {
            r = (r * 10 + v / 10) % 97;
            r = (r * 10 + v % 10) % 97;
        } else {
            r = (r * 10 + v) % 97;
        }
    }
    Some(r)
}

/// Validate a numeric string by its Luhn (mod-10) check digit.
///
/// Standard Luhn over the whole value: from the rightmost digit, every second
/// digit is doubled (subtracting 9 when the result exceeds 9); the total is
/// divisible by 10. Spaces and hyphens are stripped; a leading sign, a
/// non-digit, or fewer than two digits fails. This is the shared primitive the
/// generator's `luhn_check_digit` computes against, exposed here so algo-exists
/// numeric types (credit card, IMEI, NPI, …) enrol with a `checksum: luhn`
/// directive rather than a bespoke veto.
pub fn luhn(value: &str) -> bool {
    let t = value.trim();
    if t.starts_with('-') || t.starts_with('+') {
        return false;
    }
    let Some(digits) = t
        .chars()
        .filter(|c| *c != '-' && *c != ' ')
        .map(|c| c.to_digit(10))
        .collect::<Option<Vec<u32>>>()
    else {
        return false;
    };
    if digits.len() < 2 {
        return false;
    }
    let mut sum = 0u32;
    for (i, &d) in digits.iter().rev().enumerate() {
        let mut d = d;
        if i % 2 == 1 {
            d *= 2;
            if d > 9 {
                d -= 9;
            }
        }
        sum += d;
    }
    sum.is_multiple_of(10)
}

/// Validate a 12-character ISIN by its Luhn check digit (ISO 6166).
///
/// Two-letter country code + 9-char NSIN + 1 check digit. Every character is
/// expanded to its value (digits 0–9, letters A–Z = 10–35) as decimal digits,
/// then the whole is Luhn-validated. The taxonomy pattern checks shape
/// (`^[A-Z]{2}[A-Z0-9]{9}[0-9]$`) but not the check digit — a 12-char
/// alphanumeric with a plausible final digit otherwise passes. Spaces/hyphens
/// are stripped; any other length, or a non-`[A-Z0-9]` character, fails.
pub fn isin(value: &str) -> bool {
    let chars: Vec<char> = value
        .trim()
        .chars()
        .filter(|c| *c != '-' && *c != ' ')
        .collect();
    if chars.len() != 12 {
        return false;
    }
    let mut expanded = String::with_capacity(24);
    for &c in &chars {
        match alnum36(c) {
            Some(v) if v >= 10 => {
                expanded.push((b'0' + (v / 10) as u8) as char);
                expanded.push((b'0' + (v % 10) as u8) as char);
            }
            Some(v) => expanded.push((b'0' + v as u8) as char),
            None => return false,
        }
    }
    luhn(&expanded)
}

/// Validate a 20-character LEI by its ISO 7064 Mod 97-10 check digits (ISO 17442).
///
/// The full 20 characters (letters A–Z = 10–35, expanded to two digits each)
/// reduce to 1 mod 97. The taxonomy pattern checks shape
/// (`^[0-9]{4}[A-Z0-9]{14}[0-9]{2}$`) but not the check digits. Spaces are
/// stripped; any other length, or a non-`[A-Z0-9]` character, fails.
pub fn lei(value: &str) -> bool {
    let chars: Vec<char> = value.trim().chars().filter(|c| *c != ' ').collect();
    if chars.len() != 20 {
        return false;
    }
    mod97(&chars) == Some(1)
}

/// Validate an IBAN by its ISO 7064 Mod 97-10 check digits (ISO 13616).
///
/// Move the first four characters (country code + 2 check digits) to the end,
/// expand letters (A–Z = 10–35), and confirm the result is 1 mod 97. Spaces are
/// stripped and letters upper-cased (IBANs print in groups of four). Length must
/// be 15–34, the first two characters letters and the next two digits; anything
/// else fails.
pub fn iban(value: &str) -> bool {
    let chars: Vec<char> = value
        .trim()
        .chars()
        .filter(|c| *c != ' ')
        .map(|c| c.to_ascii_uppercase())
        .collect();
    if chars.len() < 15 || chars.len() > 34 {
        return false;
    }
    if !(chars[0].is_ascii_uppercase()
        && chars[1].is_ascii_uppercase()
        && chars[2].is_ascii_digit()
        && chars[3].is_ascii_digit())
    {
        return false;
    }
    // Rearranged = BBAN + country code + check digits (first four moved to end).
    let rearranged: Vec<char> = chars[4..].iter().chain(chars[..4].iter()).copied().collect();
    mod97(&rearranged) == Some(1)
}

/// Compute the FIGI check digit for the first 11 characters (OMG FIGI /
/// OpenFIGI "Modulus 10 Double Add Double").
///
/// Digits map to their value and letters to A=10..Z=35. Working right-to-left,
/// every second value (0-based odd index over the eleven) is doubled; the
/// decimal digits of *every* result — including the two-digit letter values —
/// are summed; the check digit is `(10 - sum mod 10) mod 10`. Because the
/// doubling and digit-sum happen at the *character* level (not over a letters-
/// expanded digit string, as ISIN does), FIGI deliberately yields a different
/// digit than an ISIN-style Luhn. Returns `None` unless given exactly 11
/// `[A-Z0-9]` characters. Shared by [`figi`] and the FIGI generator so the two
/// can never drift.
pub(crate) fn figi_check_digit(first11: &[char]) -> Option<u8> {
    if first11.len() != 11 {
        return None;
    }
    let mut sum = 0u32;
    for (i, &c) in first11.iter().enumerate() {
        let mut v = alnum36(c)?;
        if i % 2 == 1 {
            v *= 2;
        }
        // v < 100 (max 35*2 = 70), so this sums its decimal digits.
        sum += v / 10 + v % 10;
    }
    Some(((10 - sum % 10) % 10) as u8)
}

/// Validate a 12-character FIGI by its check digit (OMG FIGI / OpenFIGI).
///
/// Positions 1–11 are the identifier (`BB` provider + `G` + no-vowel body), the
/// 12th a numeric check digit computed by [`figi_check_digit`]. The taxonomy
/// pattern checks shape but not the check digit; letters are upper-cased for
/// leniency. Any other length, a non-digit check position, or a non-`[A-Z0-9]`
/// identifier character fails.
pub fn figi(value: &str) -> bool {
    let chars: Vec<char> = value
        .trim()
        .chars()
        .map(|c| c.to_ascii_uppercase())
        .collect();
    if chars.len() != 12 {
        return false;
    }
    let Some(check) = chars[11].to_digit(10) else {
        return false;
    };
    figi_check_digit(&chars[..11]) == Some(check as u8)
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
        "luhn" => Some(luhn),
        "isin" => Some(isin),
        "lei" => Some(lei),
        "iban" => Some(iban),
        "figi" => Some(figi),
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
    fn luhn_accepts_and_rejects() {
        // Canonical Luhn vectors (a test credit-card PAN and the Wikipedia example).
        assert!(luhn("4532015112830366"));
        assert!(luhn("79927398713"));
        // Bumping the check digit (rightmost, never doubled) breaks the mod-10.
        assert!(!luhn("4532015112830367"));
        assert!(!luhn("79927398714"));
        assert!(!luhn("-4532015112830366")); // signed
        assert!(!luhn("4")); // single digit
        assert!(!luhn("4532a15112830366")); // non-digit
    }

    #[test]
    fn isin_accepts_taxonomy_samples() {
        // The finance.securities.isin `samples:` are curated valid ISINs.
        for v in ["US0378331005", "GB0002634946", "JP3633400001", "DE0007164600"] {
            assert!(isin(v), "should be a valid ISIN: {v}");
        }
    }

    #[test]
    fn isin_rejects_lookalikes() {
        assert!(!isin("US0378331004")); // check digit bumped
        assert!(!isin("GB0002634947"));
        assert!(!isin("US037833100")); // 11 chars
        assert!(!isin("US03783310055")); // 13 chars
        assert!(!isin("US037833100$")); // non-alnum
    }

    #[test]
    fn iban_accepts_taxonomy_samples() {
        // The finance.banking.iban `samples:` are curated valid IBANs.
        for v in [
            "GB29NWBK60161331926819",
            "DE89370400440532013000",
            "FR7630006000011234567890189",
            "NL91ABNA0417164300",
            "ES9121000418450200051332",
            "IT60X0542811101000000123456",
        ] {
            assert!(iban(v), "should be a valid IBAN: {v}");
        }
        // Printed in groups of four (as IBANs usually are) — still valid.
        assert!(iban("GB29 NWBK 6016 1331 9268 19"));
        // Lower-case tolerated (transform upper-cases anyway).
        assert!(iban("gb29nwbk60161331926819"));
    }

    #[test]
    fn iban_rejects_lookalikes() {
        assert!(!iban("GB30NWBK60161331926819")); // check digits bumped
        assert!(!iban("DE88370400440532013000"));
        assert!(!iban("GB2")); // too short
        assert!(!iban("1B29NWBK60161331926819")); // country not letters
        assert!(!iban("GBA0NWBK60161331926819")); // check position not digits
    }

    #[test]
    fn lei_accepts_taxonomy_samples() {
        // The finance.securities.lei `samples:` are curated valid LEIs.
        for v in [
            "529900T8BM49AURSDO55",
            "213800WSGIIZCXF1P572",
            "549300MLUDYVRQOOXS22",
        ] {
            assert!(lei(v), "should be a valid LEI: {v}");
        }
    }

    #[test]
    fn lei_rejects_lookalikes() {
        assert!(!lei("529900T8BM49AURSDO56")); // check digits bumped
        assert!(!lei("529900T8BM49AURSDO5")); // 19 chars
        assert!(!lei("529900T8BM49AURSDO555")); // 21 chars
        assert!(!lei("529900T8BM49AURSDO5$")); // non-alnum
    }

    #[test]
    fn figi_check_digit_matches_spec_example() {
        // The OpenFIGI worked example: BBG000BLNQ1 -> check digit 6.
        let first11: Vec<char> = "BBG000BLNQ1".chars().collect();
        assert_eq!(figi_check_digit(&first11), Some(6));
    }

    #[test]
    fn figi_accepts_taxonomy_samples() {
        // The finance.securities.figi `samples:` are curated valid FIGIs.
        for v in [
            "BBG000BLNQ16",
            "BBG000B9XRY4",
            "BBG000BVPV84",
            "BBG000BPH459",
            "BBG000GZQ728",
        ] {
            assert!(figi(v), "should be a valid FIGI: {v}");
        }
    }

    #[test]
    fn figi_rejects_lookalikes() {
        assert!(!figi("BBG000BLNQ15")); // check digit bumped
        assert!(!figi("BBG000B9XRY5"));
        assert!(!figi("BBG000BLNQ1")); // 11 chars
        assert!(!figi("BBG000BLNQ160")); // 13 chars
        assert!(!figi("BBG000BLNQ1X")); // check position not a digit
    }

    #[test]
    fn resolve_known_and_unknown() {
        assert!(resolve("isbn").is_some());
        assert!(resolve("aba").is_some());
        assert!(resolve("cusip").is_some());
        assert!(resolve("sedol").is_some());
        assert!(resolve("luhn").is_some());
        assert!(resolve("isin").is_some());
        assert!(resolve("lei").is_some());
        assert!(resolve("iban").is_some());
        assert!(resolve("figi").is_some());
        assert!(resolve("not_a_scheme").is_none());
    }
}
