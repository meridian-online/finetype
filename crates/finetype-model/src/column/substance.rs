//! Shared substance-guard machinery.
//!
//! A *substance guard* demotes a Sense assertion to `unknown` when too few of a
//! column's values actually carry the certainty its label claims — asserting only
//! the negative certainty (not-an-X), never guessing which text type it really is
//! (that stays the model's job). The jwt / mime_type / locale_code / password
//! guards are all this one shape, differing only in the per-value predicate; the
//! per-guard rationale lives on each wrapper in `mod.rs`.

use super::*;

/// Trimmed, non-empty values from a column sample — the input every substance
/// guard measures its predicate over.
pub(crate) fn non_empty_trimmed(sample: &[String]) -> Vec<&str> {
    sample
        .iter()
        .map(|v| v.trim())
        .filter(|v| !v.is_empty())
        .collect()
}

/// Like [`non_empty_trimmed`] but each value is trimmed **and lowercased** — for
/// the case-insensitive value scans (day/month-name detection).
pub(crate) fn non_empty_lower(sample: &[String]) -> Vec<String> {
    sample
        .iter()
        .map(|v| v.trim().to_lowercase())
        .filter(|v| !v.is_empty())
        .collect()
}

/// Demote `result` to `unknown` when fewer than half of the column's non-empty
/// values satisfy `predicate`. No-op unless `result.label == label`; demote-only,
/// RHH-disableable via `rule`. This is the shared body of the substance guards —
/// the predicate is the only thing that varies between them.
pub(crate) fn demote_when_substance_fails(
    result: &mut ColumnResult,
    sample: &[String],
    label: &str,
    rule: &str,
    predicate: impl Fn(&str) -> bool,
) {
    if rhh::is_disabled(rule) {
        return;
    }
    if result.label != label {
        return;
    }
    let non_empty = non_empty_trimmed(sample);
    if non_empty.len() < 3 {
        return;
    }
    let valid = non_empty.iter().filter(|v| predicate(v)).count();
    // Majority carry the certainty → keep. Otherwise the label is wrong.
    if valid * 2 >= non_empty.len() {
        return;
    }
    result.label = "unknown".to_string();
    result.confidence = result.confidence.min(0.6);
    result.disambiguation_applied = true;
    result.disambiguation_rule = Some(rule.to_string());
    result.detected_locale = None;
}

/// Demote `result` by VALUE SHAPE when fewer than half the non-empty values
/// satisfy `predicate` — the shared body of the checksum and membership
/// substance guards (which demote to a numeric/identifier SHAPE, not `unknown`).
/// Every checksum/membership-bearing type is itself an identifier, so a failing
/// lookalike is overwhelmingly another identifier: bare-number columns go to the
/// numeric family, everything else to `alphanumeric_id` (unconditional, not
/// cardinality-split — gold confirms citation_id/case_number/… as alphanumeric_id).
/// Tagged `{rule}:{old label}`. Directive resolution and the label gate stay
/// caller-side (the caller resolves its `checksum:`/`membership:` fn and bails
/// when the label carries none).
pub(crate) fn demote_by_shape_when_substance_fails(
    result: &mut ColumnResult,
    sample: &[String],
    taxonomy: &Taxonomy,
    rule: &str,
    predicate: impl Fn(&str) -> bool,
) {
    let non_empty = non_empty_trimmed(sample);
    if non_empty.len() < 3 {
        return;
    }
    let valid = non_empty.iter().filter(|v| predicate(v)).count();
    // Majority carry the substance → genuine identifiers, keep. Otherwise these
    // values are wearing the wrong label.
    if valid * 2 >= non_empty.len() {
        return;
    }
    let (is_bare, any_decimal) = values_look_like_bare_numbers(sample);
    let demoted_from = result.label.clone();
    result.label = if is_bare {
        if any_decimal {
            "representation.numeric.decimal_number".to_string()
        } else {
            "representation.numeric.integer_number".to_string()
        }
    } else {
        "representation.identifier.alphanumeric_id".to_string()
    };
    result.confidence = result.confidence.min(0.6);
    result.disambiguation_applied = true;
    result.disambiguation_rule = Some(format!("{rule}:{demoted_from}"));
    result.detected_locale = detect_locale_from_validation(sample, &result.label, taxonomy);
}

/// Demote a bare-number column that a header hint mislabelled, to its true
/// numeric type — the shared tail of the url / utc / amount `*_bare_number_veto`s.
/// The caller has already confirmed the column is bare numbers and decided
/// `any_decimal` (utc computes it over the whole sample, not the 16-value window
/// `values_look_like_bare_numbers` scans). Tagged `{rule}:{header}`.
pub(crate) fn demote_bare_to_numeric(
    result: &mut ColumnResult,
    sample: &[String],
    taxonomy: Option<&Taxonomy>,
    any_decimal: bool,
    rule: &str,
    header: &str,
) {
    result.label = if any_decimal {
        "representation.numeric.decimal_number".to_string()
    } else {
        "representation.numeric.integer_number".to_string()
    };
    result.confidence = result.confidence.min(0.6);
    result.disambiguation_applied = true;
    result.disambiguation_rule = Some(format!("{rule}:{}", header.to_lowercase()));
    if let Some(taxonomy) = taxonomy {
        result.detected_locale = detect_locale_from_validation(sample, &result.label, taxonomy);
    }
}
