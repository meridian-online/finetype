//! `ft_validate_text(value, schema)` -- per-cell validation against a JSON Schema fragment.
//!
//! Returns a [`ValueDetail`] (`valid`, `constraint`, `message`, `grade`, `note`); the `ft_validate` table macro
//! calls it once per cell.

use finetype_core::{check_value_against_property, Note, NotesMatcher};
use serde_json::Value as JsonValue;

/// The result of validating one value: the pass/fail verdict with its reason,
/// and the value's grade.
#[derive(Debug, PartialEq)]
pub struct ValueDetail {
    pub valid: bool,
    pub constraint: Option<String>,
    pub message: Option<String>,
    /// `conforms` | `noted` | `fails`; `None` for a skipped (null) value or an
    /// unconstrained column.
    pub grade: Option<&'static str>,
    /// `<kind>: <detail>` when the grade is `noted`.
    pub note: Option<String>,
}

impl ValueDetail {
    /// Valid with no grade: a skipped null value, or a column with no constraint.
    fn skipped() -> Self {
        Self {
            valid: true,
            constraint: None,
            message: None,
            grade: None,
            note: None,
        }
    }

    fn fails(constraint: String, message: String) -> Self {
        Self {
            valid: false,
            constraint: Some(constraint),
            message: Some(message),
            grade: Some("fails"),
            note: None,
        }
    }

    fn schema_error(e: impl std::fmt::Display) -> Self {
        Self::fails("schema".to_string(), format!("schema error: {e}"))
    }
}

/// `<kind snake_case>: <detail>`, the note column's text.
fn note_text(note: &Note) -> String {
    let kind = serde_json::to_value(note.kind)
        .ok()
        .and_then(|k| k.as_str().map(str::to_string))
        .unwrap_or_default();
    format!("{kind}: {}", note.detail)
}

/// Detailed single-value validation for `ft_validate_text`.
///
/// Returns a [`ValueDetail`] mirroring the CLI `RejectRecord` shape (see
/// `finetype_core::table_validator`) plus the value's grade:
///   - `valid` — true if the value passes (or is null-skipped, or carries no
///     constraint).
///   - `constraint` — canonical token of the first failed constraint
///     (`pattern` | `min_length` | `max_length` | `enum` | `type` |
///     `required` | `checksum` | `other`), or `None` when valid.
///   - `message` — human-readable `jsonschema` error, or `None` when valid.
///
/// Scope: `valid` and `grade` reflect the fragment's JSON Schema keywords, its
/// enum matched case-insensitively, the check digit of the taxonomy leaf named
/// by `x-finetype-label` (token `checksum`), and its note extensions
/// (`x-finetype-canonical`, `x-finetype-historic`): the per-value check
/// `finetype validate` applies (`finetype_core::check_value_against_property`).
///
///   - `grade` / `note` — `conforms`, or `noted` with the note the fragment's
///     `x-finetype-canonical` / `x-finetype-historic` extensions give the value
///     (the same matcher `finetype validate` uses), or `fails`.
///
/// Null semantics match `finetype validate`: a value that is empty / whitespace
/// / `"null"` (case-insensitive) is *skipped*, not rejected → `(true, …)`.
/// A null / empty / non-object schema means "no constraint for this column"
/// (e.g. a column absent from `$.properties`) → `(true, …)`, so the table macro
/// reports zero rejects for unconstrained columns rather than rejecting every
/// row.
pub fn validate_value_detail(value: Option<&str>, schema: Option<&str>) -> ValueDetail {
    // Null-value skip (CLI semantics): empty / whitespace / "null" → valid.
    let val = match value {
        None => return ValueDetail::skipped(),
        Some(s) => {
            let t = s.trim();
            if t.is_empty() || t.eq_ignore_ascii_case("null") {
                return ValueDetail::skipped();
            }
            s
        }
    };

    // No-constraint schema: null / empty / not a JSON object → valid.
    let schema_str = match schema {
        None => return ValueDetail::skipped(),
        Some(s) => {
            let t = s.trim();
            if t.is_empty() || !t.starts_with('{') || t.eq_ignore_ascii_case("null") {
                return ValueDetail::skipped();
            }
            s
        }
    };

    let schema_json = match serde_json::from_str::<JsonValue>(schema_str) {
        Ok(v) => v,
        Err(e) => return ValueDetail::schema_error(e),
    };

    // Compiled per call, like the validator above.
    let matcher = match NotesMatcher::from_schema_property(&schema_json) {
        Ok(m) => m,
        Err(e) => return ValueDetail::schema_error(e),
    };

    // The same per-value check `finetype validate` applies: the fragment's JSON
    // Schema keywords, case-folded enum membership, and the check digit of the
    // taxonomy leaf its `x-finetype-label` names.
    let check = check_value_against_property(val, &schema_json);
    if !check.valid {
        return ValueDetail::fails(
            check.constraint.unwrap_or_else(|| "other".to_string()),
            check.message.unwrap_or_default(),
        );
    }
    let note = matcher.and_then(|m| m.note(val));
    ValueDetail {
        valid: true,
        constraint: None,
        message: None,
        grade: Some(if note.is_some() { "noted" } else { "conforms" }),
        note: note.as_ref().map(note_text),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A schema that looks like an object but does not parse is REJECTED with
    /// the `schema` token, not skipped as unconstrained. The two are one
    /// character apart in the input and opposite in the output, and the
    /// no-constraint arm above returns valid for everything that fails the
    /// `starts_with('{')` test — so without this, a malformed schema silently
    /// passing every row would leave the suite green.
    #[test]
    fn malformed_schema_object_is_rejected_not_skipped() {
        let d = validate_value_detail(Some("x"), Some(r#"{"type":"string",}"#));
        assert!(!d.valid);
        assert_eq!(d.constraint.as_deref(), Some("schema"));
        assert!(
            d.message.is_some_and(|m| m.starts_with("schema error:")),
            "the message names the schema as the fault"
        );
    }

    /// ac-04: ft_validate_text detail — a conforming value is valid with no
    /// constraint / message.
    #[test]
    fn test_detail_valid() {
        let d = validate_value_detail(Some("hello"), Some(r#"{"type":"string","minLength":3}"#));
        assert!(d.valid);
        assert_eq!(d.constraint, None);
        assert_eq!(d.message, None);
        assert_eq!(d.grade, Some("conforms"));
        assert_eq!(d.note, None);
    }

    /// ac-04: a failing value names the canonical constraint token and carries
    /// a non-empty message, mirroring the CLI reject_errors shape.
    #[test]
    fn test_detail_constraint_token() {
        let d = validate_value_detail(Some("ab"), Some(r#"{"type":"string","minLength":5}"#));
        assert!(!d.valid);
        assert_eq!(d.constraint.as_deref(), Some("min_length"));
        assert!(d.message.is_some_and(|s| !s.is_empty()));

        let d2 = validate_value_detail(
            Some("nope"),
            Some(r#"{"type":"string","pattern":"^[0-9]+$"}"#),
        );
        assert_eq!(d2.constraint.as_deref(), Some("pattern"));
    }

    /// ac-04: CLI null semantics — an empty / "null" value is skipped (valid),
    /// not rejected.
    #[test]
    fn test_detail_null_value_skipped() {
        for v in ["", "   ", "null", "NULL"] {
            let d = validate_value_detail(Some(v), Some(r#"{"type":"string","minLength":5}"#));
            assert!(d.valid, "value {v:?} should be skipped as null");
            assert_eq!(d.constraint, None);
            assert_eq!(d.grade, None);
        }
        let none = validate_value_detail(None, Some(r#"{"type":"string","minLength":5}"#));
        assert!(none.valid);
        assert_eq!(none.grade, None);
        assert_eq!(none.note, None);
    }

    /// ac-04/ac-05: a NULL / empty / non-object schema means "no constraint"
    /// (column absent from $.properties) → valid, so the table macro reports
    /// zero rejects for unconstrained columns instead of rejecting every row.
    #[test]
    fn test_detail_no_constraint_schema() {
        for s in [None, Some(""), Some("   "), Some("null")] {
            let d = validate_value_detail(Some("anything"), s);
            assert!(d.valid, "schema {s:?} should impose no constraint");
            assert_eq!(d.constraint, None);
        }
    }

    const UUID_FRAGMENT: &str = r#"{"type":"string","x-finetype-canonical":{"kind":"non_canonical","name":"lowercase","pattern":"^[^A-F]*$"}}"#;

    /// A value that passes the schema but misses the canonical form is
    /// accepted with a note; a failing value is graded `fails` with no note.
    #[test]
    fn test_detail_grade_and_note() {
        let d = validate_value_detail(
            Some("550E8400-E29B-41D4-A716-446655440000"),
            Some(UUID_FRAGMENT),
        );
        assert!(d.valid);
        assert_eq!(d.grade, Some("noted"));
        assert_eq!(d.note.as_deref(), Some("non_canonical: lowercase"));

        let ok = validate_value_detail(
            Some("550e8400-e29b-41d4-a716-446655440000"),
            Some(UUID_FRAGMENT),
        );
        assert_eq!(
            (ok.valid, ok.grade, ok.note),
            (true, Some("conforms"), None)
        );

        let bad = validate_value_detail(
            Some("bad"),
            Some(
                r#"{"type":"string","pattern":"^[0-9a-fA-F-]{36}$","x-finetype-canonical":{"kind":"non_canonical","name":"lowercase","pattern":"^[^A-F]*$"}}"#,
            ),
        );
        assert!(!bad.valid);
        assert_eq!(bad.grade, Some("fails"));
        assert_eq!(bad.note, None);
        assert_eq!(bad.constraint.as_deref(), Some("pattern"));
    }

    /// A right-shape ISBN-10 with a bad check digit fails the `checksum`
    /// constraint, as it does under `finetype validate`; the correct digit passes.
    #[test]
    fn test_detail_checksum_label_applied() {
        let frag = r#"{"type":"string","pattern":"^[0-9]{9}[0-9X]$","x-finetype-label":"identity.commerce.isbn"}"#;
        let bad = validate_value_detail(Some("0306406153"), Some(frag)); // correct check digit is 2
        assert!(!bad.valid);
        assert_eq!(bad.grade, Some("fails"));
        assert_eq!(bad.constraint.as_deref(), Some("checksum"));
        assert!(bad.message.is_some_and(|m| m.contains("check digit")));
        let good = validate_value_detail(Some("0306406152"), Some(frag));
        assert!(good.valid);
        assert_eq!(good.grade, Some("conforms"));
    }

    /// Enum membership is case-insensitive, as under `finetype validate`: a
    /// Title-case value conforms to a lower-case enum; a non-member still fails.
    #[test]
    fn test_detail_enum_is_case_folded() {
        let frag = r#"{"type":"string","enum":["male","female"]}"#;
        for v in ["Male", "FEMALE", "male"] {
            let d = validate_value_detail(Some(v), Some(frag));
            assert!(d.valid, "{v} conforms");
            assert_eq!(d.grade, Some("conforms"));
        }
        let d = validate_value_detail(Some("other"), Some(frag));
        assert!(!d.valid);
        assert_eq!(d.constraint.as_deref(), Some("enum"));
        assert_eq!(d.grade, Some("fails"));
    }

    /// A historic value is noted as withdrawn.
    #[test]
    fn test_detail_withdrawn_note() {
        let frag = r#"{"type":"string","x-finetype-historic":[{"value":"OLD","until":"2023-01"}]}"#;
        let d = validate_value_detail(Some("old"), Some(frag));
        assert!(d.valid);
        assert_eq!(d.grade, Some("noted"));
        assert_eq!(d.note.as_deref(), Some("withdrawn: withdrawn 2023-01"));
    }

    /// A malformed note extension is a schema error, like any malformed fragment.
    #[test]
    fn test_detail_malformed_note_extension_is_schema_error() {
        let d = validate_value_detail(
            Some("x"),
            Some(
                r#"{"type":"string","x-finetype-canonical":{"kind":"non_canonical","name":"n","pattern":"("}}"#,
            ),
        );
        assert!(!d.valid);
        assert_eq!(d.constraint.as_deref(), Some("schema"));
        assert_eq!(d.grade, Some("fails"));
    }
}
