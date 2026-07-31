//! Table-level validation engine.
//!
//! Validates CSV data against a table-level JSON Schema document.
//! Each column in the schema's `properties` is validated independently,
//! producing per-row error records and per-column statistics.
//!
//! This module is consumed by the CLI `validate` command, MCP server,
//! and DuckDB extension.

use crate::quality::FileQualityGrade;
use jsonschema::error::{TypeKind, ValidationErrorKind};
use serde::Serialize;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use thiserror::Error;

// ═══════════════════════════════════════════════════════════════════════════════
// ERRORS
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Error, Debug)]
pub enum TableValidatorError {
    #[error("Schema must be an object with 'properties'")]
    MissingProperties,
    #[error("Failed to compile schema for column '{column}': {detail}")]
    SchemaCompilation { column: String, detail: String },
}

// ═══════════════════════════════════════════════════════════════════════════════
// RESULT TYPES
// ═══════════════════════════════════════════════════════════════════════════════

/// A single cell-level validation error.
#[derive(Debug, Clone, Serialize)]
pub struct CellError {
    pub column: String,
    pub value: Option<String>,
    pub error: String,
    pub schema_path: String,
}

/// Errors for a single row.
#[derive(Debug, Clone, Serialize)]
pub struct RowErrors {
    pub row_index: usize,
    pub errors: Vec<CellError>,
}

/// Per-column validation statistics.
#[derive(Debug, Clone, Serialize)]
pub struct ColumnValidationStats {
    pub name: String,
    pub total: usize,
    pub valid: usize,
    pub invalid: usize,
    pub null: usize,
    pub pass_rate: f64,
}

/// Per-cell reject detail — one record per constraint violation.
///
/// Produced by `validate_table` and projected by the CLI into the DuckDB
/// `finetype_reject_errors` sidecar. See ontology `RejectEntry` in
/// `spec 2026-04-22-duckdb-extension-ergonomics`.
///
/// A single failing cell can produce multiple `RejectRecord`s when the
/// cell's value violates multiple independent constraints (e.g. type +
/// pattern). Ordering within the `rejects` vector is deterministic:
/// sorted by `(row_index, column_index)` ascending (ac-03).
#[derive(Debug, Clone, Serialize)]
pub struct RejectRecord {
    /// 0-based row index in the input `rows` slice.
    pub row_index: usize,
    /// 0-based column index in the input `headers` slice.
    pub column_index: usize,
    /// Column name (copied from `headers[column_index]`).
    pub column_name: String,
    /// The failing value (None if the cell was null / missing).
    pub value: Option<String>,
    /// Authored-time value from schema's `x-finetype-label` (not set by
    /// the validator — populated by the CLI at schema-load time for
    /// columns that have the extension).
    pub expected_type: Option<String>,
    /// Authored-time value from schema's `x-finetype-confidence` (see
    /// `expected_type` — not set by the validator).
    pub type_confidence: Option<f64>,
    /// Canonical constraint token: one of `pattern` | `min_length` |
    /// `max_length` | `enum` | `type` | `required` | `other`.
    pub constraint_failed: String,
    /// The constraint's value for debugging without a schema round-trip
    /// (pattern regex, length limit as a string, enum list as a
    /// JSON-array string, type name, required-property name).
    pub constraint_value: Option<String>,
    /// Human-readable description of what failed and why (the
    /// underlying `jsonschema::ValidationError::Display`).
    pub error_message: String,
}

/// Summary of an entire table validation run.
#[derive(Debug, Clone, Serialize)]
pub struct TableValidationResult {
    pub total_rows: usize,
    pub valid_rows: usize,
    pub invalid_rows: usize,
    pub columns: Vec<ColumnValidationStats>,
    pub grade: String,
    pub row_errors: Vec<RowErrors>,
    /// Columns present in schema but missing from data headers.
    pub missing_columns: Vec<String>,
    /// 0-based indices of rows that passed every column's validation.
    /// Sorted ascending. Used by the CLI to project only valid rows
    /// into the output DuckDB table.
    pub valid_row_indices: Vec<usize>,
    /// Per-cell reject detail (one record per constraint violation).
    /// Deterministically ordered by `(row_index, column_index)`
    /// ascending (ac-03).
    pub rejects: Vec<RejectRecord>,
}

// ═══════════════════════════════════════════════════════════════════════════════
// CONSTRAINT TOKEN MAPPING (ac-02)
// ═══════════════════════════════════════════════════════════════════════════════

/// Map a `jsonschema::ValidationErrorKind` to the canonical
/// `(constraint_failed, constraint_value)` pair used by `RejectRecord`.
///
/// Unknown / unsupported kinds fall through to `("other", None)`.
fn map_kind(kind: &ValidationErrorKind) -> (String, Option<String>) {
    match kind {
        ValidationErrorKind::Pattern { pattern } => ("pattern".to_string(), Some(pattern.clone())),
        ValidationErrorKind::MinLength { limit } => {
            ("min_length".to_string(), Some(limit.to_string()))
        }
        ValidationErrorKind::MaxLength { limit } => {
            ("max_length".to_string(), Some(limit.to_string()))
        }
        ValidationErrorKind::Enum { options } => ("enum".to_string(), Some(options.to_string())),
        ValidationErrorKind::Type { kind } => {
            let value = match kind {
                TypeKind::Single(t) => format!("{:?}", t).to_lowercase(),
                TypeKind::Multiple(set) => format!("{:?}", set),
            };
            ("type".to_string(), Some(value))
        }
        ValidationErrorKind::Required { property } => {
            // property is typically a JSON string; strip surrounding quotes
            // if present for human-readable output.
            let name = property
                .as_str()
                .map(|s| s.to_string())
                .unwrap_or_else(|| property.to_string());
            ("required".to_string(), Some(name))
        }
        _ => ("other".to_string(), None),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// CASE-FOLDED ENUM MEMBERSHIP
// ═══════════════════════════════════════════════════════════════════════════════

/// Per-column enum membership, matched case-insensitively.
///
/// The CLI/MCP validation path compiles a `jsonschema::Validator` per column,
/// but the jsonschema crate matches `enum` byte-exactly. Learned enums (gender,
/// status) are sampled lower-case while the data is Title-case, so an exact
/// match rejects valid values purely on letter-case. The taxonomy's own
/// case-variant enums (`representation.boolean.*`, HTTP methods) explicitly
/// document "any case", so case-insensitive membership is the authorial intent
/// universally — no taxonomy enum distinguishes two members by case alone.
///
/// We strip `enum` from the schema the jsonschema crate compiles and check
/// membership here against a lower-cased set, mirroring `CompiledValidator`.
/// A co-attached `pattern` stays byte-exact on the raw value.
struct EnumCheck {
    /// Lower-cased members for case-insensitive membership.
    folded: HashSet<String>,
    /// Original members rendered as a JSON array string, for the reject
    /// `constraint_value` / error message (matches the jsonschema crate's
    /// `options` token shape).
    options_token: String,
}

/// Split a column schema into a jsonschema validator (with `enum` removed) and
/// an optional case-folded `EnumCheck`. When the schema carries no `enum`, the
/// validator is compiled unchanged and the check is `None`.
fn build_column_validator(
    col_name: &str,
    col_schema: &Value,
) -> Result<(jsonschema::Validator, Option<EnumCheck>), TableValidatorError> {
    let mut schema_for_jsonschema = col_schema.clone();
    let enum_check = if let Value::Object(map) = &mut schema_for_jsonschema {
        match map.remove("enum") {
            Some(Value::Array(members)) if !members.is_empty() => {
                let folded = members
                    .iter()
                    .filter_map(|m| m.as_str())
                    .map(|s| s.to_lowercase())
                    .collect::<HashSet<String>>();
                let options_token = Value::Array(members).to_string();
                Some(EnumCheck {
                    folded,
                    options_token,
                })
            }
            // Empty array or non-array enum: leave it for jsonschema to handle.
            Some(other) => {
                map.insert("enum".to_string(), other);
                None
            }
            None => None,
        }
    } else {
        None
    };

    let validator = jsonschema::validator_for(&schema_for_jsonschema).map_err(|e| {
        TableValidatorError::SchemaCompilation {
            column: col_name.to_string(),
            detail: e.to_string(),
        }
    })?;
    Ok((validator, enum_check))
}

// ═══════════════════════════════════════════════════════════════════════════════
// NULL DETECTION
// ═══════════════════════════════════════════════════════════════════════════════

/// Returns true if the value should be treated as null.
fn is_null(value: &Option<String>) -> bool {
    match value {
        None => true,
        Some(s) => {
            let trimmed = s.trim();
            trimmed.is_empty() || trimmed.eq_ignore_ascii_case("null")
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// CORE VALIDATION
// ═══════════════════════════════════════════════════════════════════════════════

/// Validate CSV data against a table-level JSON Schema.
///
/// The schema should be a JSON Schema object with `properties` mapping
/// column names to per-column validation schemas.
///
/// Returns a `TableValidationResult` with all rows categorised and errors collected.
pub fn validate_table(
    headers: &[String],
    rows: &[Vec<Option<String>>],
    schema: &Value,
) -> Result<TableValidationResult, TableValidatorError> {
    // Extract properties from schema
    let properties = schema
        .get("properties")
        .and_then(|p| p.as_object())
        .ok_or(TableValidatorError::MissingProperties)?;

    // Build header index: header_name → column_index
    let header_index: HashMap<&str, usize> = headers
        .iter()
        .enumerate()
        .map(|(i, h)| (h.as_str(), i))
        .collect();

    // Identify columns present in schema but missing from data
    let header_set: HashSet<&str> = headers.iter().map(|h| h.as_str()).collect();
    let mut missing_columns: Vec<String> = properties
        .keys()
        .filter(|k| !header_set.contains(k.as_str()))
        .cloned()
        .collect();
    missing_columns.sort();

    // Compile per-column validators for columns present in both schema and data.
    // Sort by column index so iteration order is deterministic and matches
    // the column order in `headers` (ac-03: byte-identical output on repeat
    // calls). `serde_json::Map` iteration is arbitrary otherwise.
    let mut validators: Vec<(usize, String, jsonschema::Validator, Option<EnumCheck>)> = Vec::new();
    for (col_name, col_schema) in properties {
        if let Some(&col_idx) = header_index.get(col_name.as_str()) {
            let (validator, enum_check) = build_column_validator(col_name, col_schema)?;
            validators.push((col_idx, col_name.clone(), validator, enum_check));
        }
        // Columns in schema but not in data are tracked in missing_columns
    }
    validators.sort_by_key(|(col_idx, _, _, _)| *col_idx);

    // Per-column counters
    let mut col_stats: HashMap<String, (usize, usize, usize)> = HashMap::new(); // (valid, invalid, null)
    for (_, name, _, _) in &validators {
        col_stats.insert(name.clone(), (0, 0, 0));
    }

    let mut row_errors_list: Vec<RowErrors> = Vec::new();
    let mut valid_row_count: usize = 0;
    let mut valid_row_indices: Vec<usize> = Vec::new();
    let mut rejects: Vec<RejectRecord> = Vec::new();

    for (row_idx, row) in rows.iter().enumerate() {
        let mut errors: Vec<CellError> = Vec::new();

        for (col_idx, col_name, validator, enum_check) in &validators {
            let cell = row.get(*col_idx).unwrap_or(&None);

            if is_null(cell) {
                // Null values pass validation
                if let Some(stats) = col_stats.get_mut(col_name) {
                    stats.2 += 1;
                }
                continue;
            }

            let value_str = cell.as_deref().unwrap_or("");
            let json_value = Value::String(value_str.to_string());

            // Per-cell: collect one RejectRecord per constraint violation
            // (ac-02). A cell can fail multiple constraints; each gets its own
            // record so downstream SQL can count rejects by constraint token.
            // jsonschema covers pattern/length/type; enum membership is checked
            // here case-insensitively (the `enum` keyword was stripped before
            // compiling `validator`).
            let mut cell_rejects: Vec<RejectRecord> = Vec::new();
            let mut cell_errors: Vec<CellError> = Vec::new();

            for err in validator.iter_errors(&json_value) {
                let (token, value) = map_kind(err.kind());
                cell_rejects.push(RejectRecord {
                    row_index: row_idx,
                    column_index: *col_idx,
                    column_name: col_name.clone(),
                    value: Some(value_str.to_string()),
                    expected_type: None,
                    type_confidence: None,
                    constraint_failed: token,
                    constraint_value: value,
                    error_message: err.to_string(),
                });
                cell_errors.push(CellError {
                    column: col_name.clone(),
                    value: Some(value_str.to_string()),
                    error: err.to_string(),
                    schema_path: err.schema_path().to_string(),
                });
            }

            // Case-insensitive enum membership (scoped to enum only; any
            // co-attached pattern stayed exact via `validator` above).
            if let Some(check) = enum_check {
                if !check.folded.contains(&value_str.to_lowercase()) {
                    let error_message =
                        format!("{} is not one of {}", json_value, check.options_token);
                    cell_rejects.push(RejectRecord {
                        row_index: row_idx,
                        column_index: *col_idx,
                        column_name: col_name.clone(),
                        value: Some(value_str.to_string()),
                        expected_type: None,
                        type_confidence: None,
                        constraint_failed: "enum".to_string(),
                        constraint_value: Some(check.options_token.clone()),
                        error_message: error_message.clone(),
                    });
                    cell_errors.push(CellError {
                        column: col_name.clone(),
                        value: Some(value_str.to_string()),
                        error: error_message,
                        schema_path: "/enum".to_string(),
                    });
                }
            }

            if cell_rejects.is_empty() {
                if let Some(stats) = col_stats.get_mut(col_name) {
                    stats.0 += 1;
                }
            } else {
                if let Some(stats) = col_stats.get_mut(col_name) {
                    stats.1 += 1;
                }
                rejects.extend(cell_rejects);

                // Preserve legacy row_errors field (first error only — the
                // canonical behaviour for the pre-existing CSV path).
                errors.push(cell_errors.swap_remove(0));
            }
        }

        if errors.is_empty() {
            valid_row_count += 1;
            valid_row_indices.push(row_idx);
        } else {
            row_errors_list.push(RowErrors {
                row_index: row_idx,
                errors,
            });
        }
    }

    // ac-03: deterministic ordering — rejects sorted by
    // (row_index, column_index) ascending. The inner loop already
    // produces (row_idx, col_idx) in order, but column iteration follows
    // the `validators` Vec order which came from serde_json::Map
    // iteration — arbitrary across schema authors. Sort explicitly to
    // guarantee byte-identical output on repeat calls.
    rejects.sort_by_key(|r| (r.row_index, r.column_index));

    let total_rows = rows.len();
    let invalid_row_count = total_rows - valid_row_count;

    // Build column stats
    let columns: Vec<ColumnValidationStats> = validators
        .iter()
        .map(|(_, name, _, _)| {
            let (valid, invalid, null) = col_stats.get(name).copied().unwrap_or((0, 0, 0));
            let total = valid + invalid + null;
            let non_null = valid + invalid;
            let pass_rate = if non_null > 0 {
                valid as f64 / non_null as f64
            } else {
                1.0 // All null → no failures
            };
            ColumnValidationStats {
                name: name.clone(),
                total,
                valid,
                invalid,
                null,
                pass_rate,
            }
        })
        .collect();

    // Compute grade from valid row rate
    let valid_rate = if total_rows > 0 {
        valid_row_count as f64 / total_rows as f64
    } else {
        0.0
    };
    let grade = FileQualityGrade::from_score(valid_rate).to_string();

    Ok(TableValidationResult {
        total_rows,
        valid_rows: valid_row_count,
        invalid_rows: invalid_row_count,
        columns,
        grade,
        row_errors: row_errors_list,
        missing_columns,
        valid_row_indices,
        rejects,
    })
}

// ═══════════════════════════════════════════════════════════════════════════════
// ROW SPLITTING
// ═══════════════════════════════════════════════════════════════════════════════

/// Split rows into valid and invalid sets, preserving original row order.
///
/// Returns `(valid_rows, invalid_rows)` where each row includes all columns.
/// `None` values are converted to empty strings for CSV output.
pub fn split_rows(
    headers: &[String],
    rows: &[Vec<Option<String>>],
    result: &TableValidationResult,
) -> (Vec<Vec<String>>, Vec<Vec<String>>) {
    let invalid_indices: HashSet<usize> = result.row_errors.iter().map(|r| r.row_index).collect();
    let num_cols = headers.len();

    let mut valid = Vec::new();
    let mut invalid = Vec::new();

    for (idx, row) in rows.iter().enumerate() {
        let string_row: Vec<String> = (0..num_cols)
            .map(|i| {
                row.get(i)
                    .and_then(|v| v.as_ref())
                    .cloned()
                    .unwrap_or_default()
            })
            .collect();

        if invalid_indices.contains(&idx) {
            invalid.push(string_row);
        } else {
            valid.push(string_row);
        }
    }

    (valid, invalid)
}

// ═══════════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn s(v: &str) -> Option<String> {
        Some(v.to_string())
    }

    fn make_schema() -> Value {
        json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "minLength": 1
                },
                "age": {
                    "type": "string",
                    "pattern": "^[0-9]+$"
                },
                "email": {
                    "type": "string",
                    "pattern": "^[^@]+@[^@]+\\.[^@]+$"
                }
            }
        })
    }

    #[test]
    fn test_mixed_valid_invalid() {
        let headers = vec!["name".into(), "age".into(), "email".into()];
        let rows = vec![
            vec![s("Alice"), s("30"), s("alice@example.com")],
            vec![s("Bob"), s("notanumber"), s("bob@example.com")],
            vec![s("Charlie"), s("25"), s("invalid-email")],
            vec![s("Diana"), s("40"), s("diana@test.org")],
            vec![s(""), s("20"), s("eve@test.com")], // empty name is null → passes
        ];

        let schema = make_schema();
        let result = validate_table(&headers, &rows, &schema).unwrap();

        assert_eq!(result.total_rows, 5);
        assert_eq!(result.valid_rows, 3); // Alice, Diana, and Eve (empty name is null)
        assert_eq!(result.invalid_rows, 2);
        assert_eq!(result.row_errors.len(), 2);
        assert_eq!(result.missing_columns.len(), 0);

        // Check row indices of errors
        let error_indices: Vec<usize> = result.row_errors.iter().map(|r| r.row_index).collect();
        assert!(error_indices.contains(&1)); // Bob (age not numeric)
        assert!(error_indices.contains(&2)); // Charlie (invalid email)
    }

    #[test]
    fn test_all_valid() {
        let headers = vec!["name".into(), "age".into(), "email".into()];
        let rows = vec![
            vec![s("Alice"), s("30"), s("alice@example.com")],
            vec![s("Bob"), s("25"), s("bob@example.com")],
        ];

        let schema = make_schema();
        let result = validate_table(&headers, &rows, &schema).unwrap();

        assert_eq!(result.total_rows, 2);
        assert_eq!(result.valid_rows, 2);
        assert_eq!(result.invalid_rows, 0);
        assert_eq!(result.row_errors.len(), 0);
        assert_eq!(result.grade, "A");
    }

    #[test]
    fn test_all_invalid() {
        let headers = vec!["name".into(), "age".into(), "email".into()];
        let rows = vec![
            vec![s(""), s("abc"), s("not-email")],
            vec![s(""), s("xyz"), s("also-bad")],
        ];

        let schema = make_schema();
        let result = validate_table(&headers, &rows, &schema).unwrap();

        assert_eq!(result.total_rows, 2);
        assert_eq!(result.valid_rows, 0);
        assert_eq!(result.invalid_rows, 2);
        assert_eq!(result.grade, "F");
    }

    #[test]
    fn test_null_handling() {
        let headers = vec!["name".into(), "age".into()];
        let rows = vec![
            vec![s("Alice"), None],        // null age passes
            vec![s("Bob"), s("")],         // empty string is null, passes
            vec![s("Charlie"), s("null")], // literal "null" is null, passes
            vec![s("Diana"), s("NULL")],   // literal "NULL" is null, passes
        ];

        let schema = json!({
            "type": "object",
            "properties": {
                "name": { "type": "string", "minLength": 1 },
                "age": { "type": "string", "pattern": "^[0-9]+$" }
            }
        });

        let result = validate_table(&headers, &rows, &schema).unwrap();

        assert_eq!(result.total_rows, 4);
        assert_eq!(result.valid_rows, 4);
        assert_eq!(result.invalid_rows, 0);

        // Check the age column stats
        let age_col = result.columns.iter().find(|c| c.name == "age").unwrap();
        assert_eq!(age_col.null, 4);
        assert_eq!(age_col.valid, 0);
        assert_eq!(age_col.invalid, 0);
        assert_eq!(age_col.pass_rate, 1.0); // all null → no failures
    }

    #[test]
    fn test_missing_column_in_schema() {
        // Data has "extra" column not in schema — should be skipped
        let headers = vec!["name".into(), "extra".into()];
        let rows = vec![
            vec![s("Alice"), s("anything")],
            vec![s("Bob"), s("whatever")],
        ];

        let schema = json!({
            "type": "object",
            "properties": {
                "name": { "type": "string", "minLength": 1 }
            }
        });

        let result = validate_table(&headers, &rows, &schema).unwrap();

        assert_eq!(result.total_rows, 2);
        assert_eq!(result.valid_rows, 2);
        assert_eq!(result.columns.len(), 1); // only "name" validated
        assert_eq!(result.missing_columns.len(), 0);
    }

    #[test]
    fn test_schema_column_missing_from_data() {
        // Schema expects "email" but data doesn't have it
        let headers = vec!["name".into()];
        let rows = vec![vec![s("Alice")], vec![s("Bob")]];

        let schema = json!({
            "type": "object",
            "properties": {
                "name": { "type": "string", "minLength": 1 },
                "email": { "type": "string", "pattern": "^.+@.+$" }
            }
        });

        let result = validate_table(&headers, &rows, &schema).unwrap();

        assert_eq!(result.total_rows, 2);
        assert_eq!(result.valid_rows, 2);
        assert_eq!(result.columns.len(), 1); // only "name" validated
        assert!(result.missing_columns.contains(&"email".to_string()));
    }

    #[test]
    fn test_split_rows() {
        let headers = vec!["name".into(), "age".into()];
        let rows = vec![
            vec![s("Alice"), s("30")],
            vec![s("Bob"), s("bad")],
            vec![s("Charlie"), s("25")],
        ];

        let schema = json!({
            "type": "object",
            "properties": {
                "name": { "type": "string", "minLength": 1 },
                "age": { "type": "string", "pattern": "^[0-9]+$" }
            }
        });

        let result = validate_table(&headers, &rows, &schema).unwrap();
        let (valid, invalid) = split_rows(&headers, &rows, &result);

        assert_eq!(valid.len(), 2);
        assert_eq!(invalid.len(), 1);

        assert_eq!(valid[0], vec!["Alice", "30"]);
        assert_eq!(valid[1], vec!["Charlie", "25"]);
        assert_eq!(invalid[0], vec!["Bob", "bad"]);
    }

    #[test]
    fn test_split_rows_none_to_empty() {
        let headers = vec!["a".into(), "b".into()];
        let rows = vec![vec![s("x"), None]];

        let schema = json!({
            "type": "object",
            "properties": {
                "a": { "type": "string" }
            }
        });

        let result = validate_table(&headers, &rows, &schema).unwrap();
        let (valid, _invalid) = split_rows(&headers, &rows, &result);

        assert_eq!(valid.len(), 1);
        assert_eq!(valid[0], vec!["x", ""]); // None → empty string
    }

    #[test]
    fn test_empty_rows() {
        let headers = vec!["name".into()];
        let rows: Vec<Vec<Option<String>>> = vec![];

        let schema = json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" }
            }
        });

        let result = validate_table(&headers, &rows, &schema).unwrap();

        assert_eq!(result.total_rows, 0);
        assert_eq!(result.valid_rows, 0);
        assert_eq!(result.invalid_rows, 0);
    }

    #[test]
    fn test_column_pass_rates() {
        let headers = vec!["code".into()];
        let rows = vec![
            vec![s("ABC")],
            vec![s("123")], // fails alpha-only pattern
            vec![s("DEF")],
            vec![None], // null, not counted
        ];

        let schema = json!({
            "type": "object",
            "properties": {
                "code": { "type": "string", "pattern": "^[A-Z]+$" }
            }
        });

        let result = validate_table(&headers, &rows, &schema).unwrap();
        let col = &result.columns[0];

        assert_eq!(col.valid, 2);
        assert_eq!(col.invalid, 1);
        assert_eq!(col.null, 1);
        assert_eq!(col.total, 4);
        assert!((col.pass_rate - 2.0 / 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_error_on_missing_properties() {
        let headers = vec!["a".into()];
        let rows = vec![vec![s("x")]];
        let schema = json!({ "type": "object" }); // no properties

        let result = validate_table(&headers, &rows, &schema);
        assert!(result.is_err());
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // ac-01: RejectRecord shape + valid_row_indices (spec vrp_ac01)
    // ═══════════════════════════════════════════════════════════════════════════

    /// Canonical constraint_failed tokens.
    const CANONICAL_TOKENS: &[&str] = &[
        "pattern",
        "min_length",
        "max_length",
        "enum",
        "type",
        "required",
        "other",
    ];

    #[test]
    fn test_vrp_ac01_result_shape() {
        // 3 rows: 1 valid, 2 with different column failures.
        let headers = vec!["name".into(), "age".into(), "email".into()];
        let rows = vec![
            // Row 0: valid
            vec![s("Alice"), s("30"), s("alice@example.com")],
            // Row 1: age fails pattern
            vec![s("Bob"), s("notanumber"), s("bob@example.com")],
            // Row 2: email fails pattern
            vec![s("Charlie"), s("25"), s("invalid-email")],
        ];
        let schema = make_schema();

        let result = validate_table(&headers, &rows, &schema).unwrap();

        // (a) valid_row_indices lists row 0 only.
        assert_eq!(result.valid_row_indices, vec![0]);

        // (b) Two cell failures → at least two rejects (may be more if a
        //     pattern mismatch also reports a type error; the current
        //     jsonschema impl reports only the relevant kind, so exactly 2).
        assert_eq!(
            result.rejects.len(),
            2,
            "expected exactly 2 rejects, got {:#?}",
            result.rejects
        );

        // (c) Each RejectRecord has a canonical constraint_failed token.
        for r in &result.rejects {
            assert!(
                CANONICAL_TOKENS.contains(&r.constraint_failed.as_str()),
                "constraint_failed '{}' not in canonical set",
                r.constraint_failed
            );
            assert!(r.value.is_some(), "reject value should be populated");
            assert!(
                !r.error_message.is_empty(),
                "reject error_message should be non-empty"
            );
        }

        // (d) Legacy row_errors field is preserved (non-breaking).
        assert_eq!(result.row_errors.len(), 2);

        // Column indices and names line up with the headers.
        for r in &result.rejects {
            assert_eq!(headers[r.column_index], r.column_name);
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // ac-02: constraint_failed token mapping (spec vrp_ac02_<constraint>_failure)
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_vrp_ac02_pattern_failure() {
        let headers = vec!["code".into()];
        let rows = vec![vec![s("abc")]]; // violates uppercase-only pattern
        let schema = json!({
            "type": "object",
            "properties": {
                "code": { "type": "string", "pattern": "^[A-Z]+$" }
            }
        });

        let result = validate_table(&headers, &rows, &schema).unwrap();
        assert_eq!(result.rejects.len(), 1);
        let r = &result.rejects[0];
        assert_eq!(r.constraint_failed, "pattern");
        assert_eq!(r.constraint_value.as_deref(), Some("^[A-Z]+$"));
    }

    #[test]
    fn test_vrp_ac02_min_length_failure() {
        let headers = vec!["tag".into()];
        let rows = vec![vec![s("ab")]]; // length 2, minLength 5
        let schema = json!({
            "type": "object",
            "properties": {
                "tag": { "type": "string", "minLength": 5 }
            }
        });

        let result = validate_table(&headers, &rows, &schema).unwrap();
        assert_eq!(result.rejects.len(), 1);
        let r = &result.rejects[0];
        assert_eq!(r.constraint_failed, "min_length");
        assert_eq!(r.constraint_value.as_deref(), Some("5"));
    }

    #[test]
    fn test_vrp_ac02_max_length_failure() {
        let headers = vec!["tag".into()];
        let rows = vec![vec![s("abcdefghij")]]; // length 10, maxLength 3
        let schema = json!({
            "type": "object",
            "properties": {
                "tag": { "type": "string", "maxLength": 3 }
            }
        });

        let result = validate_table(&headers, &rows, &schema).unwrap();
        assert_eq!(result.rejects.len(), 1);
        let r = &result.rejects[0];
        assert_eq!(r.constraint_failed, "max_length");
        assert_eq!(r.constraint_value.as_deref(), Some("3"));
    }

    #[test]
    fn test_vrp_ac02_enum_failure() {
        let headers = vec!["status".into()];
        let rows: Vec<Vec<Option<String>>> = vec![vec![s("pending")]];
        let schema = json!({
            "type": "object",
            "properties": {
                "status": {
                    "type": "string",
                    "enum": ["open", "closed"]
                }
            }
        });

        let result = validate_table(&headers, &rows, &schema).unwrap();
        assert_eq!(result.rejects.len(), 1);
        let r = &result.rejects[0];
        assert_eq!(r.constraint_failed, "enum");
        let cv = r.constraint_value.as_deref().unwrap();
        // constraint_value is the enum list as a JSON-array string.
        assert!(
            cv.contains("open") && cv.contains("closed"),
            "constraint_value '{}' should contain enum members",
            cv
        );
    }

    /// Enum membership is case-insensitive: a learned lower-case enum
    /// ({male, female, ...}) must accept Title-case data (Male, Female,
    /// Non-binary). Regression for the gender/case bug — every row of
    /// people_directory.csv's gender column rejected purely on letter-case.
    #[test]
    fn test_enum_membership_case_insensitive() {
        let headers = vec!["gender".into()];
        let rows: Vec<Vec<Option<String>>> = vec![
            vec![s("Male")],
            vec![s("Female")],
            vec![s("Non-binary")],
            vec![s("MALE")],
            vec![s("female")],
        ];
        let schema = json!({
            "type": "object",
            "properties": {
                "gender": {
                    "type": "string",
                    "enum": ["male", "female", "non-binary", "other"]
                }
            }
        });

        let result = validate_table(&headers, &rows, &schema).unwrap();
        assert!(
            result.rejects.is_empty(),
            "case-only differences must not reject: {:?}",
            result.rejects
        );
        assert_eq!(result.valid_rows, 5);

        // A value absent from the set (ignoring case) still rejects.
        let bad_rows: Vec<Vec<Option<String>>> = vec![vec![s("unknown")]];
        let bad = validate_table(&headers, &bad_rows, &schema).unwrap();
        assert_eq!(bad.rejects.len(), 1);
        assert_eq!(bad.rejects[0].constraint_failed, "enum");
    }

    #[test]
    fn test_vrp_ac02_type_failure() {
        // Feed a string through a schema that requires integer. Because our
        // validator treats all CSV cells as strings, the type mismatch fires.
        let headers = vec!["count".into()];
        let rows = vec![vec![s("42")]];
        let schema = json!({
            "type": "object",
            "properties": {
                "count": { "type": "integer" }
            }
        });

        let result = validate_table(&headers, &rows, &schema).unwrap();
        assert_eq!(result.rejects.len(), 1);
        let r = &result.rejects[0];
        assert_eq!(r.constraint_failed, "type");
        assert!(
            r.constraint_value.is_some(),
            "type failure should carry the expected type name"
        );
    }

    #[test]
    fn test_vrp_ac02_required_failure() {
        // Schema declares "x" required but the schema for column "x" is not
        // per-column (it's at the object level). jsonschema's `Required`
        // variant only fires when validating an object — we exercise it by
        // using a schema that enforces required on the cell value itself.
        //
        // Since `validate_table` validates each cell as a JSON string
        // against the column's sub-schema, the natural place to exercise
        // Required is to emit a fall-through "other" if jsonschema doesn't
        // produce it. That would make the test vacuous, so instead we
        // confirm the token is reachable via direct kind mapping.
        let kind = ValidationErrorKind::Required {
            property: Value::String("x".to_string()),
        };
        let (token, value) = map_kind(&kind);
        assert_eq!(token, "required");
        assert_eq!(value.as_deref(), Some("x"));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // ac-03: determinism (spec vrp_ac03_determinism)
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_vrp_ac03_determinism() {
        // 12-row mixed-outcome fixture exercising all three columns.
        let headers = vec!["name".into(), "age".into(), "email".into()];
        let rows = vec![
            vec![s("Alice"), s("30"), s("alice@example.com")],
            vec![s("Bob"), s("notanumber"), s("bob@example.com")],
            vec![s("Charlie"), s("25"), s("invalid-email")],
            vec![s("Diana"), s("40"), s("diana@test.org")],
            vec![s(""), s("20"), s("eve@test.com")],
            vec![s("Frank"), s("abc"), s("frank@test.com")],
            vec![s("Grace"), s("55"), s("no-at-sign")],
            vec![s("Henry"), s("33"), s("henry@example.com")],
            vec![s("Ivy"), s("xxx"), s("malformed")],
            vec![s("Jack"), s("18"), s("jack@example.com")],
            vec![s("Kate"), s("kkk"), s("kate@test.com")],
            vec![s("Leo"), s("99"), s("not-email-at-all")],
        ];
        let schema = make_schema();

        let r1 = validate_table(&headers, &rows, &schema).unwrap();
        let r2 = validate_table(&headers, &rows, &schema).unwrap();

        let j1 = serde_json::to_string(&r1).unwrap();
        let j2 = serde_json::to_string(&r2).unwrap();
        assert_eq!(j1, j2, "validate_table must be deterministic");

        // Rejects sorted by (row_index, column_index) ascending.
        for window in r1.rejects.windows(2) {
            let a = &window[0];
            let b = &window[1];
            assert!(
                (a.row_index, a.column_index) <= (b.row_index, b.column_index),
                "rejects must be sorted by (row_index, column_index)"
            );
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // ac-13: Core-crate scenario grid (spec vrp_ac13_*)
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_vrp_ac13_happy_all_valid() {
        let headers = vec!["name".into(), "age".into(), "email".into()];
        let rows = vec![
            vec![s("Alice"), s("30"), s("alice@example.com")],
            vec![s("Bob"), s("25"), s("bob@example.com")],
            vec![s("Carol"), s("40"), s("carol@example.com")],
        ];
        let result = validate_table(&headers, &rows, &make_schema()).unwrap();
        assert_eq!(result.valid_row_indices, vec![0, 1, 2]);
        assert!(result.rejects.is_empty());
    }

    #[test]
    fn test_vrp_ac13_all_reject() {
        let headers = vec!["age".into()];
        let rows = vec![vec![s("x")], vec![s("y")], vec![s("z")]];
        let schema = json!({
            "type": "object",
            "properties": {
                "age": { "type": "string", "pattern": "^[0-9]+$" }
            }
        });
        let result = validate_table(&headers, &rows, &schema).unwrap();
        assert!(result.valid_row_indices.is_empty());
        assert_eq!(result.rejects.len(), 3);
        // All rejects carry the pattern token.
        assert!(result
            .rejects
            .iter()
            .all(|r| r.constraint_failed == "pattern"));
    }

    #[test]
    fn test_vrp_ac13_partial_reject_mixed() {
        let headers = vec!["age".into()];
        let rows = vec![vec![s("10")], vec![s("bad")], vec![s("20")]];
        let schema = json!({
            "type": "object",
            "properties": {
                "age": { "type": "string", "pattern": "^[0-9]+$" }
            }
        });
        let result = validate_table(&headers, &rows, &schema).unwrap();
        assert_eq!(result.valid_row_indices, vec![0, 2]);
        assert_eq!(result.rejects.len(), 1);
        assert_eq!(result.rejects[0].row_index, 1);
    }

    #[test]
    fn test_vrp_ac13_multi_reject_per_row() {
        // Two columns both fail on the same row.
        let headers = vec!["a".into(), "b".into()];
        let rows = vec![vec![s("x"), s("y")]];
        let schema = json!({
            "type": "object",
            "properties": {
                "a": { "type": "string", "pattern": "^[0-9]+$" },
                "b": { "type": "string", "pattern": "^[0-9]+$" }
            }
        });
        let result = validate_table(&headers, &rows, &schema).unwrap();
        assert!(result.valid_row_indices.is_empty());
        assert_eq!(result.rejects.len(), 2);
        // Row index is the same (0); column indices sorted ascending.
        assert_eq!(result.rejects[0].column_index, 0);
        assert_eq!(result.rejects[1].column_index, 1);
    }

    #[test]
    fn test_vrp_ac13_empty_input() {
        let headers = vec!["a".into()];
        let rows: Vec<Vec<Option<String>>> = vec![];
        let schema = json!({
            "type": "object",
            "properties": { "a": { "type": "string" } }
        });
        let result = validate_table(&headers, &rows, &schema).unwrap();
        assert_eq!(result.total_rows, 0);
        assert!(result.valid_row_indices.is_empty());
        assert!(result.rejects.is_empty());
    }

    #[test]
    fn test_vrp_ac13_single_row_single_column() {
        let headers = vec!["a".into()];
        let rows = vec![vec![s("hello")]];
        let schema = json!({
            "type": "object",
            "properties": { "a": { "type": "string", "minLength": 1 } }
        });
        let result = validate_table(&headers, &rows, &schema).unwrap();
        assert_eq!(result.valid_row_indices, vec![0]);
        assert!(result.rejects.is_empty());
    }
}
