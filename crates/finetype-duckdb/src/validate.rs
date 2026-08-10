//! `ft_validate_text(value, schema)` -- per-cell validation against a JSON Schema fragment.
//!
//! Returns `(valid, constraint_token, message)`; the `ft_validate` table macro
//! calls it once per cell.

use jsonschema::error::ValidationErrorKind;
use serde_json::Value as JsonValue;

/// Detailed single-value validation for `ft_validate_text`.
///
/// Returns `(valid, constraint_token, message)` mirroring the CLI
/// `RejectRecord` shape (see `finetype_core::table_validator`):
///   - `valid` — true if the value passes (or is null-skipped, or carries no
///     constraint).
///   - `constraint_token` — canonical token of the first failed constraint
///     (`pattern` | `min_length` | `max_length` | `enum` | `type` |
///     `required` | `other`), or `None` when valid.
///   - `message` — human-readable `jsonschema` error, or `None` when valid.
///
/// Null semantics match `finetype validate`: a value that is empty / whitespace
/// / `"null"` (case-insensitive) is *skipped*, not rejected → `(true, …)`.
/// A null / empty / non-object schema means "no constraint for this column"
/// (e.g. a column absent from `$.properties`) → `(true, …)`, so the table macro
/// reports zero rejects for unconstrained columns rather than rejecting every
/// row.
pub fn validate_value_detail(
    value: Option<&str>,
    schema: Option<&str>,
) -> (bool, Option<String>, Option<String>) {
    // Null-value skip (CLI semantics): empty / whitespace / "null" → valid.
    let val = match value {
        None => return (true, None, None),
        Some(s) => {
            let t = s.trim();
            if t.is_empty() || t.eq_ignore_ascii_case("null") {
                return (true, None, None);
            }
            s
        }
    };

    // No-constraint schema: null / empty / not a JSON object → valid.
    let schema_str = match schema {
        None => return (true, None, None),
        Some(s) => {
            let t = s.trim();
            if t.is_empty() || !t.starts_with('{') || t.eq_ignore_ascii_case("null") {
                return (true, None, None);
            }
            s
        }
    };

    let schema_json = match serde_json::from_str::<JsonValue>(schema_str) {
        Ok(v) => v,
        Err(e) => {
            return (
                false,
                Some("schema".to_string()),
                Some(format!("schema error: {e}")),
            )
        }
    };

    let validator = match jsonschema::validator_for(&schema_json) {
        Ok(v) => v,
        Err(e) => {
            return (
                false,
                Some("schema".to_string()),
                Some(format!("schema error: {e}")),
            )
        }
    };

    let json_value = JsonValue::String(val.to_string());
    let out = match validator.iter_errors(&json_value).next() {
        None => (true, None, None),
        Some(error) => {
            let token = constraint_token(error.kind());
            (false, Some(token), Some(error.to_string()))
        }
    };
    out
}

/// Map a `jsonschema::ValidationErrorKind` to the canonical constraint token.
///
/// Mirror of the private `map_kind` in `finetype_core::table_validator`
/// (kept in sync by hand — the extension validates independently of the CLI
/// path). Unknown kinds fall through to `other`.
fn constraint_token(kind: &ValidationErrorKind) -> String {
    match kind {
        ValidationErrorKind::Pattern { .. } => "pattern",
        ValidationErrorKind::MinLength { .. } => "min_length",
        ValidationErrorKind::MaxLength { .. } => "max_length",
        ValidationErrorKind::Enum { .. } => "enum",
        ValidationErrorKind::Type { .. } => "type",
        ValidationErrorKind::Required { .. } => "required",
        _ => "other",
    }
    .to_string()
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
        let (valid, constraint, message) =
            validate_value_detail(Some("x"), Some(r#"{"type":"string",}"#));
        assert!(!valid);
        assert_eq!(constraint.as_deref(), Some("schema"));
        assert!(
            message.is_some_and(|m| m.starts_with("schema error:")),
            "the message names the schema as the fault"
        );
    }

    /// ac-04: ft_validate_text detail — a conforming value is valid with no
    /// constraint / message.
    #[test]
    fn test_detail_valid() {
        let (valid, c, m) =
            validate_value_detail(Some("hello"), Some(r#"{"type":"string","minLength":3}"#));
        assert!(valid);
        assert_eq!(c, None);
        assert_eq!(m, None);
    }

    /// ac-04: a failing value names the canonical constraint token and carries
    /// a non-empty message, mirroring the CLI reject_errors shape.
    #[test]
    fn test_detail_constraint_token() {
        let (valid, c, m) =
            validate_value_detail(Some("ab"), Some(r#"{"type":"string","minLength":5}"#));
        assert!(!valid);
        assert_eq!(c.as_deref(), Some("min_length"));
        assert!(m.is_some_and(|s| !s.is_empty()));

        let (_, c2, _) = validate_value_detail(
            Some("nope"),
            Some(r#"{"type":"string","pattern":"^[0-9]+$"}"#),
        );
        assert_eq!(c2.as_deref(), Some("pattern"));
    }

    /// ac-04: CLI null semantics — an empty / "null" value is skipped (valid),
    /// not rejected.
    #[test]
    fn test_detail_null_value_skipped() {
        for v in ["", "   ", "null", "NULL"] {
            let (valid, c, _) =
                validate_value_detail(Some(v), Some(r#"{"type":"string","minLength":5}"#));
            assert!(valid, "value {v:?} should be skipped as null");
            assert_eq!(c, None);
        }
        let (valid_none, _, _) =
            validate_value_detail(None, Some(r#"{"type":"string","minLength":5}"#));
        assert!(valid_none);
    }

    /// ac-04/ac-05: a NULL / empty / non-object schema means "no constraint"
    /// (column absent from $.properties) → valid, so the table macro reports
    /// zero rejects for unconstrained columns instead of rejecting every row.
    #[test]
    fn test_detail_no_constraint_schema() {
        for s in [None, Some(""), Some("   "), Some("null")] {
            let (valid, c, _) = validate_value_detail(Some("anything"), s);
            assert!(valid, "schema {s:?} should impose no constraint");
            assert_eq!(c, None);
        }
    }
}
