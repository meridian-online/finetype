use super::*;

/// Normalize boolean sub-types based on actual value content.
///
/// When the top prediction or any boolean label appears in the top 3 votes,
/// examine the actual values to determine the correct boolean sub-type:
/// - 0/1 integers → `representation.boolean.binary`
/// - true/false/yes/no/on/off text → `representation.boolean.terms`
/// - T/F/Y/N single characters → `representation.boolean.initials`
///
/// Also detects boolean-valued columns that were misclassified as non-boolean
/// types (e.g., categorical), overriding when ≥80% of values match.
pub(crate) fn disambiguate_boolean_subtype(
    values: &[String],
    top_labels: &[&str],
) -> Option<(String, String)> {
    let non_empty = non_empty_trimmed(values);

    if non_empty.len() < 3 {
        return None;
    }

    // Check if values are boolean-like
    let binary_values: &[&str] = &["0", "1"];
    let terms_values: &[&str] = &[
        "true", "false", "True", "False", "TRUE", "FALSE", "yes", "no", "Yes", "No", "YES", "NO",
        "on", "off", "On", "Off", "ON", "OFF",
    ];
    let initials_values: &[&str] = &["T", "F", "t", "f", "Y", "N", "y", "n"];

    let binary_count = non_empty
        .iter()
        .filter(|v| binary_values.contains(v))
        .count();
    let terms_count = non_empty
        .iter()
        .filter(|v| terms_values.contains(v))
        .count();
    let initials_count = non_empty
        .iter()
        .filter(|v| initials_values.contains(v))
        .count();

    let n = non_empty.len() as f64;
    let binary_frac = binary_count as f64 / n;
    let terms_frac = terms_count as f64 / n;
    let initials_frac = initials_count as f64 / n;

    // Only fire if a boolean type is in the top predictions, OR if the values
    // themselves are overwhelmingly boolean (catches cases where model predicted
    // categorical/other for True/False columns)
    let has_boolean_vote = top_labels.iter().any(|l| BOOLEAN_LABELS.contains(l));
    let max_frac = binary_frac.max(terms_frac).max(initials_frac);

    if !has_boolean_vote && max_frac < 0.8 {
        return None;
    }

    // Pick the best matching sub-type (must have ≥80% of values matching)
    //
    // For binary (0/1), also require ≤2 unique values to avoid false positives
    // on skewed integer columns (e.g. SibSp: mostly 0s and 1s, but range 0-8).
    if terms_frac >= 0.8 {
        Some((
            "representation.boolean.terms".to_string(),
            "boolean_subtype_terms".to_string(),
        ))
    } else if initials_frac >= 0.8 {
        Some((
            "representation.boolean.initials".to_string(),
            "boolean_subtype_initials".to_string(),
        ))
    } else if binary_frac >= 0.8 {
        let unique_values: std::collections::HashSet<&str> = non_empty.iter().copied().collect();
        // Require exactly 2 unique values (both 0 AND 1 present). All-zero or all-one
        // columns are count/sentinel data, not boolean. Also reject when one value
        // dominates >95% of samples — not meaningful boolean variation.
        if unique_values.len() == 2 {
            let dominant_frac = non_empty
                .iter()
                .filter(|&&v| v == "0")
                .count()
                .max(non_empty.iter().filter(|&&v| v == "1").count())
                as f64
                / n;
            if dominant_frac <= 0.95 {
                Some((
                    "representation.boolean.binary".to_string(),
                    "boolean_subtype_binary".to_string(),
                ))
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    }
}
/// Detect gender columns by checking if all values match a known gender value set.
///
/// Rule: If ALL non-empty values are in the gender set → identity.person.gender
pub(crate) fn disambiguate_gender(values: &[String]) -> Option<String> {
    const GENDER_VALUES: &[&str] = &[
        "male",
        "female",
        "m",
        "f",
        "Male",
        "Female",
        "M",
        "F",
        "MALE",
        "FEMALE",
        "man",
        "woman",
        "Man",
        "Woman",
        "MAN",
        "WOMAN",
        "boy",
        "girl",
        "Boy",
        "Girl",
        // Inclusive gender values
        "non-binary",
        "Non-binary",
        "Non-Binary",
        "NON-BINARY",
        "nonbinary",
        "Nonbinary",
        "other",
        "Other",
        "OTHER",
        "prefer not to say",
        "Prefer not to say",
        "unknown",
        "Unknown",
        "UNKNOWN",
        "x",
        "X",
        "genderqueer",
        "Genderqueer",
        "agender",
        "Agender",
        "transgender",
        "Transgender",
    ];

    let non_empty = non_empty_trimmed(values);

    if non_empty.len() < 3 {
        return None;
    }

    // Single-character alpha sex codes (ICAO Doc 9303: M/F/X) are gender_code,
    // not gender. The gender enum is the word set [male,female,other,unknown],
    // so a column of bare M/F validates against gender_code's enum
    // [M,F,X,0,1,2,9] and is hard-vetoed against gender. Alpha-only: the
    // ISO-5218 numeric codes (0/1/2/9) are deliberately NOT a value-only
    // trigger — they would over-emit gender_code onto every boolean/ordinal
    // numeric column (boolean_subtype owns 0/1 and runs first). Per the
    // deterministic-layer audit false-veto resolution (spec
    // 2026-06-12-false-veto-trio-resolution).
    let all_code = non_empty
        .iter()
        .all(|v| matches!(v.to_ascii_uppercase().as_str(), "M" | "F" | "X"));
    if all_code {
        return Some("identity.person.gender_code".to_string());
    }

    let all_gender = non_empty.iter().all(|v| GENDER_VALUES.contains(v));
    if all_gender {
        Some("identity.person.gender".to_string())
    } else {
        None
    }
}
/// Override boolean classification when the column has small integer values
/// with more than 2 unique values and a spread > 1.
///
/// Rule: If majority vote is boolean but values are integers with >2 unique values
///       spanning 0-N where N > 1, override to integer_number.
pub(crate) fn disambiguate_boolean_override(
    values: &[String],
    top_labels: &[&str],
) -> Option<(String, String)> {
    // Only trigger when boolean is in the top predictions
    let has_boolean = top_labels.iter().any(|l| BOOLEAN_LABELS.contains(l));
    if !has_boolean {
        return None;
    }

    let non_empty = non_empty_trimmed(values);
    if non_empty.len() < 3 {
        return None;
    }

    // Check single-character non-numeric values first (e.g., Embarked: S, C, Q)
    let all_single_char = non_empty.iter().all(|v| v.chars().count() == 1);
    let all_digits = non_empty
        .iter()
        .all(|v| v.chars().all(|c| c.is_ascii_digit()));
    if all_single_char && !all_digits {
        let mut unique_chars: Vec<&str> = non_empty.clone();
        unique_chars.sort();
        unique_chars.dedup();
        if unique_chars.len() >= 2 {
            // Single chars that aren't just 0/1 or T/F → categorical
            let is_boolean_set = unique_chars.len() == 2 && {
                let set: std::collections::HashSet<&str> = unique_chars.iter().copied().collect();
                set.contains("0") && set.contains("1")
                    || set.contains("T") && set.contains("F")
                    || set.contains("t") && set.contains("f")
                    || set.contains("Y") && set.contains("N")
                    || set.contains("y") && set.contains("n")
            };
            if !is_boolean_set {
                return Some((
                    "representation.text.word".to_string(),
                    "boolean_override_single_char_categorical".to_string(),
                ));
            }
        }
    }

    // Parse values as integers — check for small integer spread
    let parsed: Vec<i64> = values
        .iter()
        .filter_map(|v| v.trim().parse::<i64>().ok())
        .collect();

    if parsed.len() >= 3 {
        let mut unique: Vec<i64> = parsed.clone();
        unique.sort();
        unique.dedup();
        let n_unique = unique.len();
        let min = *unique.first().unwrap();
        let max = *unique.last().unwrap();

        // All-zero or single-value columns are not boolean — they're counts,
        // sentinels, or constant columns. Require ≥2 distinct values for
        // boolean classification to stand. (Distillation v2: 175 cases)
        if n_unique < 2 {
            return Some((
                "representation.numeric.integer_number".to_string(),
                "boolean_override_single_value".to_string(),
            ));
        }

        // If one value dominates >95% of samples, it's not meaningful boolean data.
        // For n_unique==2, one of {count(==first), count(!=first)} is the majority;
        // .max() picks whichever is larger regardless of which value came first in
        // the parsed vector.
        let dominant_count = parsed
            .iter()
            .filter(|&&v| v == parsed[0])
            .count()
            .max(parsed.iter().filter(|&&v| v != parsed[0]).count());
        let dominant_frac = dominant_count as f64 / parsed.len() as f64;
        if n_unique == 2 && dominant_frac > 0.95 {
            return Some((
                "representation.numeric.integer_number".to_string(),
                "boolean_override_skewed".to_string(),
            ));
        }

        // If >2 unique integer values and spread > 1, it's not boolean
        if n_unique > 2 && (max - min) > 1 {
            return Some((
                "representation.numeric.integer_number".to_string(),
                "boolean_override_integer_spread".to_string(),
            ));
        }
    }

    None
}
