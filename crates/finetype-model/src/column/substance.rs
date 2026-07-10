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
