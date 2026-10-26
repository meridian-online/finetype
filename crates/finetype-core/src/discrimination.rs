//! Per-type discrimination detectors.
//!
//! A type's published rule (its taxonomy `validation`) is written to agree with
//! its defining source, and for some types that rule confirms a large share of
//! random input. Where it does, Sharpen and the validation veto need a stricter
//! per-value test to decide whether a column really is the type. A detector is
//! that test; it is internal and never published.
//!
//! A label with no detector falls through to its published rule.

use crate::taxonomy::Taxonomy;
use crate::validator::resolve_label_validator;

/// The detector for `label`, when one exists. Stricter than the published rule.
pub fn detector_for(label: &str) -> Option<fn(&str) -> bool> {
    match label {
        "identity.person.username" => Some(is_username_shape),
        _ => None,
    }
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
    match detector_for(label) {
        Some(detect) => detect(value),
        None => rule.is_valid(value),
    }
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
