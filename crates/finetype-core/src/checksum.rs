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

/// Validate a GS1 GTIN (EAN-8, UPC-A, EAN-13, GTIN-14) by its check digit.
///
/// From the rightmost digit, weights alternate 1 (the check digit itself),
/// 3, 1, 3, …; the total is divisible by 10. Only the four GS1 lengths are
/// admitted — an arbitrary digit run of another length is not a barcode.
/// Spaces and hyphens are stripped; a leading sign or a non-digit fails.
pub fn gs1(value: &str) -> bool {
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
    if !matches!(digits.len(), 8 | 12 | 13 | 14) {
        return false;
    }
    let mut sum = 0u32;
    for (i, &d) in digits.iter().rev().enumerate() {
        sum += if i % 2 == 1 { 3 * d } else { d };
    }
    sum.is_multiple_of(10)
}

/// Validate a 10-digit US National Provider Identifier by its check digit.
///
/// The NPI check digit is Luhn computed over the 9-digit base prefixed with
/// the ISO issuer prefix `80840` (healthcare, US) — so the full 15-digit
/// string `80840` + NPI must pass Luhn. A bare 10-digit number that passes
/// plain Luhn is NOT necessarily a valid NPI and vice versa. Spaces and
/// hyphens are stripped; any other length, a leading sign, or a non-digit
/// fails.
pub fn npi(value: &str) -> bool {
    let t = value.trim();
    if t.starts_with('-') || t.starts_with('+') {
        return false;
    }
    let digits: String = t.chars().filter(|c| *c != '-' && *c != ' ').collect();
    if digits.len() != 10 || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return false;
    }
    luhn(&format!("80840{digits}"))
}

/// Validate an 11-digit Australian Business Number by its ATO modulus-89 check.
///
/// Subtract 1 from the first digit, apply weights 10,1,3,5,7,9,11,13,15,17,19
/// and confirm the weighted sum is divisible by 89. ABNs print as
/// `NN NNN NNN NNN`; spaces are stripped. Any other length, a leading sign,
/// a first digit of 0 (would go negative), or a non-digit fails.
pub fn abn(value: &str) -> bool {
    let t = value.trim();
    if t.starts_with('-') || t.starts_with('+') {
        return false;
    }
    let Some(digits) = t
        .chars()
        .filter(|c| *c != ' ')
        .map(|c| c.to_digit(10))
        .collect::<Option<Vec<u32>>>()
    else {
        return false;
    };
    if digits.len() != 11 || digits[0] == 0 {
        return false;
    }
    const WEIGHTS: [u32; 11] = [10, 1, 3, 5, 7, 9, 11, 13, 15, 17, 19];
    let mut sum = 0u32;
    for (i, &d) in digits.iter().enumerate() {
        let d = if i == 0 { d - 1 } else { d };
        sum += WEIGHTS[i] * d;
    }
    sum.is_multiple_of(89)
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
    let rearranged: Vec<char> = chars[4..]
        .iter()
        .chain(chars[..4].iter())
        .copied()
        .collect();
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

/// Validate an 8-digit ISSN by its ISO 7064 Mod 11-2 check digit.
///
/// The first seven digits carry weights 8,7,6,5,4,3,2; the eighth is a check
/// digit `(11 − weighted sum mod 11) mod 11`, printed as `X` when 10. ISSNs
/// print as `NNNN-NNNC`; the hyphen is stripped. Any other length, or a
/// non-digit where a digit is required, fails.
pub fn issn(value: &str) -> bool {
    let chars: Vec<char> = value.trim().chars().filter(|c| *c != '-').collect();
    if chars.len() != 8 {
        return false;
    }
    let mut sum = 0u32;
    for (i, &c) in chars[..7].iter().enumerate() {
        let Some(d) = c.to_digit(10) else {
            return false;
        };
        sum += (8 - i as u32) * d;
    }
    let check = (11 - sum % 11) % 11;
    let given = match chars[7] {
        'X' | 'x' => 10,
        c => match c.to_digit(10) {
            Some(d) => d,
            None => return false,
        },
    };
    check == given
}

/// Validate a 16-character ORCID iD by its ISO 7064 Mod 11-2 check digit.
///
/// The 15 leading digits fold `total = (total + digit)·2 mod 11`; the check
/// digit is `(12 − total) mod 11`, printed as `X` when 10. ORCIDs print as
/// `NNNN-NNNN-NNNN-NNNC`; hyphens are stripped. (Same scheme as ISNI.) Any other
/// length, or a non-digit in the 15-digit base, fails.
pub fn orcid(value: &str) -> bool {
    let chars: Vec<char> = value.trim().chars().filter(|c| *c != '-').collect();
    if chars.len() != 16 {
        return false;
    }
    let mut total = 0u32;
    for &c in &chars[..15] {
        let Some(d) = c.to_digit(10) else {
            return false;
        };
        total = ((total + d) * 2) % 11;
    }
    let check = (12 - total) % 11;
    let given = match chars[15] {
        'X' | 'x' => 10,
        c => match c.to_digit(10) {
            Some(d) => d,
            None => return false,
        },
    };
    check == given
}

/// Validate a CAS Registry Number by its check digit.
///
/// Format `N…N-NN-N` (2–7, 2, 1 digits); the final digit is a check equal to
/// `(Σ position·digit) mod 10`, positions counting from the right of the
/// non-check portion (rightmost = 1). Hyphens are stripped. A non-digit, or
/// fewer than the 5 minimum digits, fails.
pub fn cas(value: &str) -> bool {
    let Some(digits) = value
        .trim()
        .chars()
        .filter(|c| *c != '-')
        .map(|c| c.to_digit(10))
        .collect::<Option<Vec<u32>>>()
    else {
        return false;
    };
    if digits.len() < 5 {
        return false;
    }
    let (body, check) = digits.split_at(digits.len() - 1);
    let mut sum = 0u32;
    for (i, &d) in body.iter().rev().enumerate() {
        sum += (i as u32 + 1) * d;
    }
    sum % 10 == check[0]
}

/// Validate an 11-character ISO 6346 shipping-container code by its check digit.
///
/// Owner (3 letters) + category (U/J/Z) + 6-digit serial + 1 check digit. Each
/// of the first 10 characters takes a value (digits 0–9; letters A=10..Z=38 with
/// every multiple of 11 skipped) weighted by `2^position` (leftmost = 2^0); the
/// check digit is `sum mod 11`, with 10 mapped to 0. Any other length, or an
/// invalid character, fails.
pub fn iso6346(value: &str) -> bool {
    // A..Z letter values with multiples of 11 (11, 22, 33) skipped.
    const LV: [u32; 26] = [
        10, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 34, 35,
        36, 37, 38,
    ];
    let chars: Vec<char> = value.trim().chars().collect();
    if chars.len() != 11 {
        return false;
    }
    let mut sum = 0u32;
    for (i, &c) in chars[..10].iter().enumerate() {
        let v = if c.is_ascii_uppercase() {
            LV[(c as u32 - 'A' as u32) as usize]
        } else if let Some(d) = c.to_digit(10) {
            d
        } else {
            return false;
        };
        sum += v * (1u32 << i);
    }
    let Some(check) = chars[10].to_digit(10) else {
        return false;
    };
    // Check digit is `sum mod 11`; a remainder of 10 is written as 0.
    (sum % 11) % 10 == check
}

/// Validate a DEA registration number by its check digit.
///
/// Two letters (registrant type + a name initial) then seven digits; the seventh
/// digit is a check equal to `((d1+d3+d5) + 2·(d2+d4+d6)) mod 10`. Only the
/// digits enter the checksum. Any other length, a non-letter in the first two
/// positions, or a non-digit in the numeric part, fails.
pub fn dea(value: &str) -> bool {
    let chars: Vec<char> = value.trim().chars().collect();
    if chars.len() != 9 || !chars[0].is_ascii_alphabetic() || !chars[1].is_ascii_alphabetic() {
        return false;
    }
    let Some(d) = chars[2..9]
        .iter()
        .map(|c| c.to_digit(10))
        .collect::<Option<Vec<u32>>>()
    else {
        return false;
    };
    let sum = (d[0] + d[2] + d[4]) + 2 * (d[1] + d[3] + d[5]);
    sum % 10 == d[6]
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
        "gs1" => Some(gs1),
        "npi" => Some(npi),
        "abn" => Some(abn),
        "issn" => Some(issn),
        "orcid" => Some(orcid),
        "cas" => Some(cas),
        "iso6346" => Some(iso6346),
        "dea" => Some(dea),
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
        for v in [
            "US0378331005",
            "GB0002634946",
            "JP3633400001",
            "DE0007164600",
        ] {
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
    fn gs1_accepts_all_four_lengths() {
        assert!(gs1("73513537")); // EAN-8
        assert!(gs1("036000291452")); // UPC-A
        assert!(gs1("4006381333931")); // EAN-13
        assert!(gs1("00036000291452")); // GTIN-14 (zero-padded UPC-A)
    }

    #[test]
    fn gs1_rejects_lookalikes() {
        assert!(!gs1("4006381333932")); // check digit bumped
        assert!(!gs1("036000291453"));
        assert!(!gs1("73513538"));
        assert!(!gs1("400638133393")); // 12 digits but not a valid UPC
        assert!(!gs1("40063813339")); // 11 digits — not a GS1 length
        assert!(!gs1("4006381333")); // 10 digits — not a GS1 length
        assert!(!gs1("-4006381333931")); // signed
        assert!(!gs1("400638133393a")); // non-digit
    }

    #[test]
    fn npi_accepts_and_rejects() {
        // 1234567893: Luhn over 80840123456789 yields check digit 3.
        assert!(npi("1234567893"));
        assert!(!npi("1234567894")); // check digit bumped
                                     // Passes plain Luhn (79927398713) but is 11 digits — not an NPI.
        assert!(!npi("79927398713"));
        assert!(!npi("123456789")); // 9 digits
        assert!(!npi("-1234567893")); // signed
        assert!(!npi("123456789X")); // non-digit
    }

    #[test]
    fn abn_accepts_genuine_numbers() {
        // The ATO's own ABN and Telstra's — canonical published examples.
        assert!(abn("51824753556"));
        assert!(abn("33051775556"));
        // Printed grouping NN NNN NNN NNN.
        assert!(abn("51 824 753 556"));
    }

    #[test]
    fn abn_rejects_lookalikes() {
        assert!(!abn("51824753557")); // check broken
        assert!(!abn("33051775557"));
        assert!(!abn("5182475355")); // 10 digits
        assert!(!abn("518247535561")); // 12 digits
        assert!(!abn("01824753556")); // first digit 0 — no valid ABN starts with 0
        assert!(!abn("-51824753556")); // signed
        assert!(!abn("5182475355X")); // non-digit
    }

    #[test]
    fn issn_accepts_and_rejects() {
        // Canonical valid ISSNs (incl. an X check digit).
        for v in ["0378-5955", "2049-3630", "0028-0836", "2434-561X"] {
            assert!(issn(v), "should be a valid ISSN: {v}");
        }
        assert!(issn("03785955")); // hyphen optional
        assert!(!issn("0378-5956")); // check digit bumped
        assert!(!issn("2049-3631"));
        assert!(!issn("0378-595")); // 7 digits
        assert!(!issn("0378-595Y")); // bad check char
    }

    #[test]
    fn orcid_accepts_and_rejects() {
        // The canonical ORCID support example, and one with an X check digit.
        for v in [
            "0000-0002-1825-0097",
            "0000-0001-5109-3700",
            "0000-0002-1694-233X",
        ] {
            assert!(orcid(v), "should be a valid ORCID: {v}");
        }
        assert!(!orcid("0000-0002-1825-0098")); // check digit bumped
        assert!(!orcid("0000-0002-1825-009")); // 15 chars
        assert!(!orcid("0000-0002-1825-00Y7")); // non-digit base
    }

    #[test]
    fn cas_accepts_and_rejects() {
        // Water, formaldehyde, toluene, benzene — published CAS numbers.
        for v in ["7732-18-5", "50-00-0", "108-88-3", "71-43-2"] {
            assert!(cas(v), "should be a valid CAS number: {v}");
        }
        assert!(!cas("7732-18-6")); // check digit bumped
        assert!(!cas("108-88-4"));
        assert!(!cas("50-0-0")); // too short
        assert!(!cas("7732-18-X")); // non-digit
    }

    #[test]
    fn iso6346_accepts_and_rejects() {
        // The ISO 6346 worked example plus other valid container numbers.
        for v in ["CSQU3054383", "MSKU0000006", "HLXU1234561"] {
            assert!(iso6346(v), "should be a valid ISO 6346 code: {v}");
        }
        assert!(!iso6346("CSQU3054384")); // check digit bumped
        assert!(!iso6346("CSQU305438")); // 10 chars
        assert!(!iso6346("CSQ_3054383")); // invalid char
    }

    #[test]
    fn dea_accepts_and_rejects() {
        // Valid DEA numbers (the check is over the 7 digits only).
        for v in ["BB1388568", "AB1234563"] {
            assert!(dea(v), "should be a valid DEA number: {v}");
        }
        assert!(!dea("BB1388569")); // check digit bumped
        assert!(!dea("AB1234564"));
        assert!(!dea("1B1388568")); // first position not a letter
        assert!(!dea("BB138856")); // 8 chars
        assert!(!dea("BB138856X")); // non-digit in numeric part
    }

    #[test]
    fn resolve_known_and_unknown() {
        for name in [
            "isbn", "aba", "cusip", "sedol", "luhn", "isin", "lei", "iban", "figi", "gs1", "npi",
            "abn", "issn", "orcid", "cas", "iso6346", "dea",
        ] {
            assert!(resolve(name).is_some(), "{name} should resolve");
        }
        assert!(resolve("not_a_scheme").is_none());
    }
}
