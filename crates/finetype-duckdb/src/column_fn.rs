//! Column-level classification helpers for the `finetype()` and `finetype_detail()` overloads.
//!
//! When `finetype()` receives a `LIST<VARCHAR>` input (via `list(col)`), it delegates
//! to these helpers which use the ColumnClassifier's disambiguation rules (date formats,
//! coordinates, boolean subtypes, categorical detection, etc.) to produce a single
//! semantic type for the whole column.
//!
//! Usage:
//! ```sql
//! -- Column classification (LIST<VARCHAR> overload)
//! SELECT col_name, finetype(list(col_value))
//! FROM values_table GROUP BY col_name;
//!
//! -- With header hint for better disambiguation
//! SELECT col_name, finetype(list(col_value), col_name)
//! FROM values_table GROUP BY col_name;
//!
//! -- Full detail (JSON output)
//! SELECT col_name, finetype_detail(list(col_value))
//! FROM values_table GROUP BY col_name;
//! ```

use crate::get_classifier;
use crate::type_mapping;

use duckdb::core::{DataChunkHandle, Inserter};
use duckdb::vtab::arrow::WritableVector;
use std::error::Error;
use std::ffi::CString;
use std::sync::OnceLock;

use finetype_model::inference::InferenceError;
use finetype_model::{ClassificationResult, ColumnClassifier, ColumnResult, ValueClassifier};

// ═══════════════════════════════════════════════════════════════════════════════
// GLOBAL COLUMN CLASSIFIER
// ═══════════════════════════════════════════════════════════════════════════════

/// Thin delegate that implements ValueClassifier by forwarding to the global CharClassifier.
struct GlobalClassifierDelegate;

impl ValueClassifier for GlobalClassifierDelegate {
    fn classify(&self, text: &str) -> std::result::Result<ClassificationResult, InferenceError> {
        get_classifier().classify(text)
    }

    fn classify_batch(
        &self,
        texts: &[String],
    ) -> std::result::Result<Vec<ClassificationResult>, InferenceError> {
        get_classifier().classify_batch(texts)
    }
}

static COLUMN_CLASSIFIER: OnceLock<ColumnClassifier> = OnceLock::new();

fn get_column_classifier() -> &'static ColumnClassifier {
    COLUMN_CLASSIFIER
        .get_or_init(|| ColumnClassifier::with_defaults(Box::new(GlobalClassifierDelegate)))
}

/// Run column classification on a slice of strings.
///
/// Used by both the scalar path (chunk-as-sample) and the list path (explicit list).
pub fn classify_column(
    values: &[String],
) -> std::result::Result<ColumnResult, finetype_model::inference::InferenceError> {
    get_column_classifier().classify_column(values)
}

// ═══════════════════════════════════════════════════════════════════════════════
// INPUT TYPE DETECTION
// ═══════════════════════════════════════════════════════════════════════════════

/// Check whether the first column of a data chunk is a LIST type.
///
/// Used by the unified `finetype()` / `finetype_detail()` to dispatch between
/// single-value (VARCHAR) and column-level (LIST<VARCHAR>) classification.
pub unsafe fn is_list_input(input: &mut DataChunkHandle) -> bool {
    use libduckdb_sys::*;

    let raw_chunk = input.get_ptr();
    let vector = duckdb_data_chunk_get_vector(raw_chunk, 0);
    let mut logical_type = duckdb_vector_get_column_type(vector);
    let type_id = duckdb_get_type_id(logical_type);
    duckdb_destroy_logical_type(&mut logical_type);
    type_id == DUCKDB_TYPE_DUCKDB_TYPE_LIST
}

// ═══════════════════════════════════════════════════════════════════════════════
// LIST<VARCHAR> READER
// ═══════════════════════════════════════════════════════════════════════════════

/// Read a LIST<VARCHAR> value from a DuckDB data chunk at a specific column and row.
///
/// Returns None if the list is NULL, Some(vec) with the string values otherwise.
/// NULL and empty elements within the list are skipped.
unsafe fn read_list_varchar(
    input: &mut DataChunkHandle,
    col_idx: usize,
    row_idx: usize,
) -> Option<Vec<String>> {
    use libduckdb_sys::*;

    let raw_chunk = input.get_ptr();
    let vector = duckdb_data_chunk_get_vector(raw_chunk, col_idx as idx_t);

    // Check list-level validity (NULL check)
    let validity = duckdb_vector_get_validity(vector);
    if !validity.is_null() {
        let entry_idx = row_idx / 64;
        let bit = row_idx % 64;
        let mask = *validity.add(entry_idx);
        if (mask >> bit) & 1 == 0 {
            return None;
        }
    }

    // Read list entry (offset + length)
    let list_data = duckdb_vector_get_data(vector) as *const duckdb_list_entry;
    let list_entry = *list_data.add(row_idx);

    let offset = list_entry.offset as usize;
    let length = list_entry.length as usize;

    if length == 0 {
        return Some(vec![]);
    }

    // Get child vector (VARCHAR entries)
    let child_vector = duckdb_list_vector_get_child(vector);
    let child_validity = duckdb_vector_get_validity(child_vector);
    let child_data = duckdb_vector_get_data(child_vector) as *const duckdb_string_t;

    let mut values = Vec::with_capacity(length);

    for i in 0..length {
        let child_idx = offset + i;

        // Check child validity
        if !child_validity.is_null() {
            let entry_idx = child_idx / 64;
            let bit = child_idx % 64;
            let mask = *child_validity.add(entry_idx);
            if (mask >> bit) & 1 == 0 {
                continue; // Skip NULL entries
            }
        }

        // Read string value
        let str_val = *child_data.add(child_idx);

        let (ptr, len) = if duckdb_string_is_inlined(str_val) {
            (
                str_val.value.inlined.inlined.as_ptr() as *const u8,
                str_val.value.inlined.length as usize,
            )
        } else {
            (
                str_val.value.pointer.ptr as *const u8,
                str_val.value.pointer.length as usize,
            )
        };

        if !ptr.is_null() && len > 0 {
            if let Ok(s) = std::str::from_utf8(std::slice::from_raw_parts(ptr, len)) {
                let trimmed = s.trim();
                if !trimmed.is_empty() {
                    values.push(trimmed.to_string());
                }
            }
        }
    }

    Some(values)
}

// ═══════════════════════════════════════════════════════════════════════════════
// COLUMN-LEVEL INVOKE HELPERS
// ═══════════════════════════════════════════════════════════════════════════════

/// Column-level classification: LIST<VARCHAR> → label VARCHAR.
///
/// Called by `FineType::invoke()` when the first argument is a LIST<VARCHAR>.
pub unsafe fn invoke_column_label(
    input: &mut DataChunkHandle,
    output: &mut dyn WritableVector,
) -> std::result::Result<(), Box<dyn Error>> {
    let col_classifier = get_column_classifier();
    let len = input.len();
    let n_cols = input.num_columns();
    let mut output_vec = output.flat_vector();

    for i in 0..len {
        if let Some(values) = read_list_varchar(input, 0, i) {
            if values.is_empty() {
                let cstr = CString::new("unknown")?;
                output_vec.insert(i, cstr);
                continue;
            }

            // Check for optional header argument (2-arg overload)
            let result = if n_cols >= 2 {
                if let Some(header) = crate::read_varchar(input, 1, i) {
                    if header.is_empty() {
                        col_classifier.classify_column(&values)
                    } else {
                        col_classifier.classify_column_with_header(&values, &header)
                    }
                } else {
                    col_classifier.classify_column(&values)
                }
            } else {
                col_classifier.classify_column(&values)
            };

            match result {
                Ok(col_result) => {
                    let cstr = CString::new(col_result.label.as_str())?;
                    output_vec.insert(i, cstr);
                }
                Err(_) => {
                    let cstr = CString::new("unknown")?;
                    output_vec.insert(i, cstr);
                }
            }
        } else {
            // NULL list → NULL output
            output_vec.set_null(i);
        }
    }

    Ok(())
}

/// Column-level classification with full detail: LIST<VARCHAR> → JSON VARCHAR.
///
/// Called by `FineTypeDetail::invoke()` when the first argument is a LIST<VARCHAR>.
pub unsafe fn invoke_column_detail(
    input: &mut DataChunkHandle,
    output: &mut dyn WritableVector,
) -> std::result::Result<(), Box<dyn Error>> {
    let col_classifier = get_column_classifier();
    let len = input.len();
    let n_cols = input.num_columns();
    let output_vec = output.flat_vector();

    for i in 0..len {
        if let Some(values) = read_list_varchar(input, 0, i) {
            if values.is_empty() {
                let cstr = CString::new(
                    r#"{"type":"unknown","confidence":0.0,"duckdb_type":"VARCHAR","samples":0}"#,
                )?;
                output_vec.insert(i, cstr);
                continue;
            }

            // Check for optional header argument
            let result = if n_cols >= 2 {
                if let Some(header) = crate::read_varchar(input, 1, i) {
                    if header.is_empty() {
                        col_classifier.classify_column(&values)
                    } else {
                        col_classifier.classify_column_with_header(&values, &header)
                    }
                } else {
                    col_classifier.classify_column(&values)
                }
            } else {
                col_classifier.classify_column(&values)
            };

            match result {
                Ok(col_result) => {
                    let json = format_column_result_json(&col_result);
                    let cstr = CString::new(json)?;
                    output_vec.insert(i, cstr);
                }
                Err(e) => {
                    let json = format!(
                        r#"{{"type":"unknown","confidence":0.0,"duckdb_type":"VARCHAR","samples":0,"error":"{}"}}"#,
                        e.to_string().replace('"', "'")
                    );
                    let cstr = CString::new(json)?;
                    output_vec.insert(i, cstr);
                }
            }
        } else {
            // NULL list → NULL output
            output.flat_vector().set_null(i);
        }
    }

    Ok(())
}

/// Format a ColumnResult as a JSON string.
pub fn format_column_result_json(result: &ColumnResult) -> String {
    let duckdb_type = type_mapping::to_duckdb_type(&result.label);

    // Build top-N votes as a JSON object
    let votes: Vec<String> = result
        .vote_distribution
        .iter()
        .take(5) // Top 5 candidates
        .map(|(label, frac)| format!(r#""{}": {:.3}"#, label, frac))
        .collect();
    let votes_json = format!("{{{}}}", votes.join(", "));

    let disambiguation = if let Some(ref rule) = result.disambiguation_rule {
        format!(r#", "disambiguation": "{}""#, rule)
    } else {
        String::new()
    };

    format!(
        r#"{{"type": "{}", "confidence": {:.3}, "duckdb_type": "{}", "samples": {}{}, "votes": {}}}"#,
        result.label,
        result.confidence,
        duckdb_type,
        result.samples_used,
        disambiguation,
        votes_json,
    )
}
