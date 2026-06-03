//! `finetype_validate(value, schema_json)` -- Validate a value against a JSON Schema fragment.
//!
//! Returns 'valid' if the value passes validation, or the first error message if it fails.
//! The schema is compiled once per unique schema string and cached for performance.

use std::collections::HashMap;
use std::sync::Mutex;

use jsonschema::error::ValidationErrorKind;
use serde_json::Value as JsonValue;

/// Cached result of schema compilation: either a parsed JSON value (we re-compile
/// cheaply per chunk) or an error message for permanently invalid schemas.
enum CachedSchema {
    /// Schema JSON parsed successfully and compiles as a valid JSON Schema.
    Valid(JsonValue),
    /// Schema is permanently invalid (parse error or schema compilation error).
    Invalid(String),
}

/// Thread-safe cache of parsed JSON Schema values, keyed by schema string.
///
/// DuckDB calls the scalar function once per chunk (~2048 rows). The schema argument
/// is typically constant across all rows (from a literal in the SQL query), so caching
/// avoids re-parsing the schema JSON for every row.
static SCHEMA_CACHE: std::sync::LazyLock<Mutex<HashMap<String, CachedSchema>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

/// Validate a value string against a JSON Schema fragment string.
///
/// Returns `"valid"` on success, or the first validation error message on failure.
pub fn validate_value(value: &str, schema_str: &str) -> String {
    // Fast path: check cache
    {
        let cache = SCHEMA_CACHE.lock().unwrap();
        if let Some(cached) = cache.get(schema_str) {
            return match cached {
                CachedSchema::Invalid(err) => format!("schema error: {}", err),
                CachedSchema::Valid(schema_json) => run_validation(value, schema_json),
            };
        }
    }

    // Slow path: parse and validate the schema, then cache
    let schema_json = match serde_json::from_str::<JsonValue>(schema_str) {
        Ok(v) => v,
        Err(e) => {
            let err_msg = e.to_string();
            let mut cache = SCHEMA_CACHE.lock().unwrap();
            cache.insert(
                schema_str.to_string(),
                CachedSchema::Invalid(err_msg.clone()),
            );
            return format!("schema error: {}", err_msg);
        }
    };

    // Verify the schema compiles before caching as valid
    if let Err(e) = jsonschema::validator_for(&schema_json) {
        let err_msg = e.to_string();
        let mut cache = SCHEMA_CACHE.lock().unwrap();
        cache.insert(
            schema_str.to_string(),
            CachedSchema::Invalid(err_msg.clone()),
        );
        return format!("schema error: {}", err_msg);
    }

    let result = run_validation(value, &schema_json);

    let mut cache = SCHEMA_CACHE.lock().unwrap();
    cache.insert(schema_str.to_string(), CachedSchema::Valid(schema_json));

    result
}

/// Run validation of a value against a pre-parsed JSON Schema.
///
/// Compiles the validator from the cached JSON value. The `jsonschema` crate's
/// compilation is fast (~microseconds for simple schemas) so this is acceptable
/// overhead compared to the cost of re-parsing the JSON string.
fn run_validation(value: &str, schema_json: &JsonValue) -> String {
    let validator = match jsonschema::validator_for(schema_json) {
        Ok(v) => v,
        Err(e) => return format!("schema error: {}", e),
    };

    let json_value = JsonValue::String(value.to_string());

    let result = match validator.iter_errors(&json_value).next() {
        None => "valid".to_string(),
        Some(error) => error.to_string(),
    };
    result
}

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

    /// ac-07: the existing scalar `finetype_validate(value, schema_json)` is
    /// preserved unchanged by spec v1.2. 'valid' on conforming input;
    /// non-'valid' error string on non-conforming input.
    #[test]
    fn test_vrp_ac07_scalar_unchanged() {
        let schema = r#"{"type":"string"}"#;
        assert_eq!(validate_value("hello", schema), "valid");

        // A string-typed schema with a minLength of 5 rejects a 3-char value.
        let schema2 = r#"{"type":"string","minLength":5}"#;
        let err = validate_value("abc", schema2);
        assert_ne!(err, "valid");
        assert!(
            !err.is_empty(),
            "non-conforming input should produce a non-empty error"
        );
    }

    /// Schema-error surfacing: malformed JSON schema strings produce an
    /// error prefix rather than 'valid'. Preserves the existing contract.
    #[test]
    fn test_vrp_ac07_schema_error_prefixed() {
        let err = validate_value("x", "not-json");
        assert!(err.starts_with("schema error:"), "got: {}", err);
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
