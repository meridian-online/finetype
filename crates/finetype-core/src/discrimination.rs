//! Per-type discrimination detectors.
//!
//! A type's published rule (its taxonomy `validation`) is written to agree with
//! its defining source, and for some types that rule confirms a large share of
//! random input. Where it does, Sharpen and the validation veto need a stricter
//! per-value test to decide whether a column really is the type. A detector is
//! that test; it is internal and never published.
//!
//! A label with no detector falls through to its published rule.

use crate::taxonomy::{Taxonomy, Validation};
use crate::validator::{resolve_label_validator, validate_value, CompiledValidator};

/// The detector for `label`, when one exists. Stricter than the published rule.
pub fn detector_for(label: &str) -> Option<fn(&str) -> bool> {
    match label {
        "identity.person.username" => Some(is_username_shape),
        "identity.medical.npi" => Some(is_issued_npi),
        "technology.identifier.tsid" => Some(is_tsid_shape),
        _ => None,
    }
}

/// Whether `value` passes: the detector when there is one, else the compiled
/// published rule. The one per-value form every discrimination site shares, so a
/// site cannot grow its own reading of "detector else rule".
pub fn passes_detector_or_rule(
    detector: Option<fn(&str) -> bool>,
    rule: &CompiledValidator,
    value: &str,
) -> bool {
    detector.map_or_else(|| rule.is_valid(value), |d| d(value))
}

/// As [`passes_detector_or_rule`], for a site whose taxonomy has no compiled
/// cache: the published rule is compiled per call. A rule that fails to compile
/// counts the value as passing, as `validate_value`'s callers have always read it.
pub fn passes_detector_or_uncompiled_rule(
    detector: Option<fn(&str) -> bool>,
    rule: &Validation,
    value: &str,
) -> bool {
    detector.map_or_else(
        || {
            validate_value(value, rule)
                .map(|r| r.is_valid)
                .unwrap_or(true)
        },
        |d| d(value),
    )
}

/// `username`: 3 to 32 characters of `[a-zA-Z0-9_.-]`, optionally followed by a
/// single `$`. The published rule follows useradd(8) (up to 256 characters),
/// which would confirm 36-character UUIDs and 40/64-character hashes; this
/// keeps the shape analysts meet in web-service names.
fn is_username_shape(value: &str) -> bool {
    let body = value.strip_suffix('$').unwrap_or(value);
    let n = body.chars().count();
    (3..=32).contains(&n)
        && body
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
}

/// `tsid`: the published rule (13 Crockford base32 characters, first character
/// at most F) and at least one letter. The published rule is source-shaped, so it
/// accepts every 13-digit number starting 0 or 1, which is every Unix-millisecond
/// timestamp from 2001 to 2286; a real TSID is all digits about one time in a
/// million, so requiring a letter keeps column discrimination off those.
fn is_tsid_shape(value: &str) -> bool {
    let b = value.as_bytes();
    b.len() == 13
        && b[0].is_ascii_hexdigit()
        && b[1..].iter().all(|c| {
            c.is_ascii_digit()
                || (c.is_ascii_alphabetic()
                    && !matches!(c.to_ascii_uppercase(), b'I' | b'L' | b'O' | b'U'))
        })
        && b.iter().any(u8::is_ascii_alphabetic)
}

/// `npi`: a number in the issued range (first digit 1 or 2) whose check digit
/// passes. The published rule follows the CMS check-digit document, which leaves
/// other first digits reserved rather than invalid, so it accepts any ten digits;
/// this keeps column discrimination at the range NPIs are issued from.
fn is_issued_npi(value: &str) -> bool {
    let b = value.as_bytes();
    b.len() == 10
        && matches!(b[0], b'1' | b'2')
        && b.iter().all(u8::is_ascii_digit)
        && crate::checksum::npi(value)
}

/// Whether `value` passes for `label`: the detector when there is one, else the
/// published rule. A label with no published rule (or an unknown one) never
/// passes, detector or not, matching the veto. The rule is resolved from the taxonomy's compiled cache or compiled on a
/// miss (the same resolution `resolve_label_validator` gives `ceded_leaf_recovery`).
///
/// The profile call sites keep their own validator lookup and call
/// `detector_for` directly, so a failed lookup behaves at each as it did before;
/// this form serves callers with no lookup of their own.
pub fn value_passes_for_discrimination(value: &str, label: &str, tax: &Taxonomy) -> bool {
    let Some(rule) = resolve_label_validator(label, tax) else {
        return false;
    };
    passes_detector_or_rule(detector_for(label), &rule, value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn taxonomy(compiled: bool) -> Taxonomy {
        let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        dir.pop();
        dir.pop();
        let mut tax = Taxonomy::from_directory(dir.join("labels")).expect("load labels");
        if compiled {
            tax.compile_validators();
        }
        tax
    }

    #[test]
    fn username_detector_keeps_the_old_shape_and_allows_trailing_dollar() {
        let detect = detector_for("identity.person.username").expect("username detector");
        for ok in ["alice", "bob.smith", "svc$", "abc", &"a".repeat(32)] {
            assert!(detect(ok), "{ok}");
        }
        for bad in [
            "ab",
            "ab$",
            "a$b",
            "$alice",
            "al ice",
            "123e4567-e89b-12d3-a456-426614174000",
            &"a".repeat(40),
            &"a".repeat(33),
        ] {
            assert!(!detect(bad), "{bad}");
        }
    }

    /// The locale branches of the attractor sites and the fast path consult only
    /// the published rule, so a detector label must never reach them.
    #[test]
    fn detector_labels_have_no_locale_rules_and_are_not_fast_path_leaves() {
        let tax = taxonomy(false);
        let labels = [
            "identity.person.username",
            "identity.medical.npi",
            "technology.identifier.tsid",
        ];
        for label in labels {
            assert!(detector_for(label).is_some(), "{label} has a detector");
            let def = tax.get(label).expect("label in taxonomy");
            assert!(
                def.validation_by_locale.is_none()
                    || def
                        .validation_by_locale
                        .as_ref()
                        .is_some_and(|m| m.is_empty()),
                "{label} has validation_by_locale"
            );
            assert!(
                !crate::fast_path::CONCLUSIVE_LEAVES.contains(&label),
                "{label} is a fast-path leaf"
            );
        }
        // Every registered detector is covered by the list above.
        for (id, _) in tax.definitions() {
            if detector_for(id).is_some() {
                assert!(labels.contains(&id.as_str()), "{id} not listed");
            }
        }
    }

    #[test]
    fn tsid_detector_is_the_published_rule_plus_a_letter() {
        let tax = taxonomy(true);
        let detect = detector_for("technology.identifier.tsid").expect("tsid detector");
        for ok in ["0JMVQ049NFN6K", "0jmvq049nfn6k", "000000000000A"] {
            assert!(detect(ok), "{ok}");
        }
        for bad in [
            "1700000000000",
            "0000000000000",
            "GAWE5HZP3SKTK",
            "0AWE5HZP3SKTU",
            "0JMVQ049NFN6",
            "0190a1b2c3d4e5f6a7b8c9d0e1f2a3b4",
        ] {
            assert!(!detect(bad), "{bad}");
        }
        // A detector never accepts what the published rule rejects.
        let rule = resolve_label_validator("technology.identifier.tsid", &tax).expect("rule");
        for v in [
            "GAWE5HZP3SKTK",
            "0AWE5HZP3SKTU",
            "0JMVQ049NFN6",
            "0JMVQ049NFN6KK",
        ] {
            assert!(!detect(v) && !rule.is_valid(v), "{v}");
        }
    }

    #[test]
    fn no_detector_means_published_rule() {
        let label = "representation.identifier.uuid";
        assert!(detector_for(label).is_none());
        let samples = [
            "123e4567-e89b-12d3-a456-426614174000",
            "00000000-0000-0000-0000-000000000000",
            "123e4567e89b12d3a456426614174000",
            "not-a-uuid",
            "",
        ];
        for compiled in [true, false] {
            let tax = taxonomy(compiled);
            let rule = tax
                .get(label)
                .and_then(|d| d.validation.as_ref())
                .map(|v| crate::validator::CompiledValidator::new(v).unwrap())
                .expect("uuid has a validation");
            for s in samples {
                assert_eq!(
                    value_passes_for_discrimination(s, label, &tax),
                    rule.is_valid(s),
                    "{s:?} (compiled cache: {compiled})"
                );
            }
        }
    }
}
