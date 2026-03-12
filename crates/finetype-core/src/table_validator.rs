//! Table-level validation engine.
//!
//! Validates CSV data against a table-level JSON Schema document.
//! Each column in the schema's `properties` is validated independently,
//! producing per-row error records and per-column statistics.
//!
//! This module is consumed by the CLI `validate` command, MCP server,
//! and DuckDB extension.

use crate::quality::FileQualityGrade;
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
    let missing_columns: Vec<String> = properties
        .keys()
        .filter(|k| !header_set.contains(k.as_str()))
        .cloned()
        .collect();

    // Compile per-column validators for columns present in both schema and data
    let mut validators: Vec<(usize, String, jsonschema::Validator)> = Vec::new();
    for (col_name, col_schema) in properties {
        if let Some(&col_idx) = header_index.get(col_name.as_str()) {
            let validator = jsonschema::validator_for(col_schema).map_err(|e| {
                TableValidatorError::SchemaCompilation {
                    column: col_name.clone(),
                    detail: e.to_string(),
                }
            })?;
            validators.push((col_idx, col_name.clone(), validator));
        }
        // Columns in schema but not in data are tracked in missing_columns
    }

    // Per-column counters
    let mut col_stats: HashMap<String, (usize, usize, usize)> = HashMap::new(); // (valid, invalid, null)
    for (_, name, _) in &validators {
        col_stats.insert(name.clone(), (0, 0, 0));
    }

    let mut row_errors_list: Vec<RowErrors> = Vec::new();
    let mut valid_row_count: usize = 0;

    for (row_idx, row) in rows.iter().enumerate() {
        let mut errors: Vec<CellError> = Vec::new();

        for (col_idx, col_name, validator) in &validators {
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

            let validation_result = validator.validate(&json_value);
            if validation_result.is_ok() {
                if let Some(stats) = col_stats.get_mut(col_name) {
                    stats.0 += 1;
                }
            } else {
                if let Some(stats) = col_stats.get_mut(col_name) {
                    stats.1 += 1;
                }
                // Collect first error for this cell
                let err_msg = validator
                    .iter_errors(&json_value)
                    .next()
                    .map(|e| e.to_string())
                    .unwrap_or_else(|| "validation failed".to_string());
                let schema_path = validator
                    .iter_errors(&json_value)
                    .next()
                    .map(|e| e.schema_path().to_string())
                    .unwrap_or_default();

                errors.push(CellError {
                    column: col_name.clone(),
                    value: Some(value_str.to_string()),
                    error: err_msg,
                    schema_path,
                });
            }
        }

        if errors.is_empty() {
            valid_row_count += 1;
        } else {
            row_errors_list.push(RowErrors {
                row_index: row_idx,
                errors,
            });
        }
    }

    let total_rows = rows.len();
    let invalid_row_count = total_rows - valid_row_count;

    // Build column stats
    let columns: Vec<ColumnValidationStats> = validators
        .iter()
        .map(|(_, name, _)| {
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
}
