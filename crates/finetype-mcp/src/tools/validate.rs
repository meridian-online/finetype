//! `validate` tool — validate CSV data against a JSON Schema.

use crate::FineTypeServer;
use finetype_core::validate_table;
use rmcp::model::{CallToolResult, ErrorData};
use serde::Deserialize;
use serde_json::json;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ValidateRequest {
    /// Path to a CSV file to validate.
    #[schemars(description = "Absolute path to the CSV file to validate")]
    pub path: Option<String>,

    /// Inline CSV data (alternative to path, for small datasets).
    #[schemars(description = "Inline CSV content as a string (alternative to path)")]
    pub data: Option<String>,

    /// JSON Schema to validate against (as a JSON string).
    #[schemars(description = "JSON Schema document as a string to validate the data against")]
    pub schema: String,
}

/// Headers and row data parsed from CSV.
type CsvData = (Vec<String>, Vec<Vec<Option<String>>>);

/// Parse CSV data into headers and rows (with Option<String> cells for null handling).
fn parse_csv_rows(csv_data: &str) -> Result<CsvData, ErrorData> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(csv_data.as_bytes());

    let headers: Vec<String> = reader
        .headers()
        .map_err(|e| ErrorData::invalid_params(format!("Failed to parse CSV headers: {e}"), None))?
        .iter()
        .map(|h| h.to_string())
        .collect();

    let rows: Vec<Vec<Option<String>>> = reader
        .records()
        .filter_map(|r| r.ok())
        .map(|record| {
            (0..headers.len())
                .map(|i| {
                    let val = record.get(i).unwrap_or("").to_string();
                    if val.is_empty() {
                        None
                    } else {
                        Some(val)
                    }
                })
                .collect()
        })
        .collect();

    Ok((headers, rows))
}

pub async fn handle(
    _server: &FineTypeServer,
    request: ValidateRequest,
) -> Result<CallToolResult, ErrorData> {
    // Read CSV data from path or inline
    let csv_data = match (&request.path, &request.data) {
        (Some(path), _) => std::fs::read_to_string(path)
            .map_err(|e| ErrorData::invalid_params(format!("Failed to read file: {e}"), None))?,
        (_, Some(data)) => data.clone(),
        (None, None) => {
            return Err(ErrorData::invalid_params(
                "Either 'path' or 'data' must be provided",
                None,
            ));
        }
    };

    // Parse the JSON Schema
    let schema: serde_json::Value = serde_json::from_str(&request.schema).map_err(|e| {
        ErrorData::invalid_params(format!("Failed to parse JSON Schema: {e}"), None)
    })?;

    // Parse CSV into headers and rows
    let (headers, rows) = parse_csv_rows(&csv_data)?;

    // Run validation
    let result = validate_table(&headers, &rows, &schema)
        .map_err(|e| ErrorData::invalid_params(format!("Validation error: {e}"), None))?;

    // Build JSON output
    let columns_json: Vec<serde_json::Value> = result
        .columns
        .iter()
        .map(|c| {
            json!({
                "name": c.name,
                "total": c.total,
                "valid": c.valid,
                "invalid": c.invalid,
                "null": c.null,
                "pass_rate": (c.pass_rate * 1000.0).round() / 1000.0,
            })
        })
        .collect();

    // Limit row errors in output to first 50 to avoid huge responses
    let max_errors = 50;
    let truncated = result.row_errors.len() > max_errors;
    let errors_json: Vec<serde_json::Value> = result
        .row_errors
        .iter()
        .take(max_errors)
        .map(|re| {
            json!({
                "row": re.row_index,
                "errors": re.errors.iter().map(|e| json!({
                    "column": e.column,
                    "value": e.value,
                    "error": e.error,
                })).collect::<Vec<_>>(),
            })
        })
        .collect();

    let mut json_result = json!({
        "total_rows": result.total_rows,
        "valid_rows": result.valid_rows,
        "invalid_rows": result.invalid_rows,
        "grade": result.grade,
        "columns": columns_json,
        "errors": errors_json,
    });

    if !result.missing_columns.is_empty() {
        json_result["missing_columns"] = json!(result.missing_columns);
    }
    if truncated {
        json_result["errors_truncated"] = json!(true);
        json_result["total_error_rows"] = json!(result.row_errors.len());
    }

    // Build markdown summary
    let valid_pct = if result.total_rows > 0 {
        result.valid_rows as f64 / result.total_rows as f64 * 100.0
    } else {
        0.0
    };

    let mut md = format!(
        "## Validation Results\n\n\
         **Grade:** {} | **Rows:** {} total, {} valid ({:.1}%), {} invalid\n\n",
        result.grade, result.total_rows, result.valid_rows, valid_pct, result.invalid_rows,
    );

    if !result.missing_columns.is_empty() {
        md.push_str(&format!(
            "**Missing columns:** {}\n\n",
            result.missing_columns.join(", ")
        ));
    }

    md.push_str("| Column | Valid | Invalid | Null | Pass Rate |\n");
    md.push_str("|--------|-------|---------|------|-----------|\n");
    for col in &result.columns {
        md.push_str(&format!(
            "| {} | {} | {} | {} | {:.1}% |\n",
            col.name,
            col.valid,
            col.invalid,
            col.null,
            col.pass_rate * 100.0,
        ));
    }

    if !result.row_errors.is_empty() {
        md.push_str(&format!(
            "\n**{} row(s) with errors**",
            result.row_errors.len()
        ));
        if truncated {
            md.push_str(&format!(" (showing first {})", max_errors));
        }
        md.push('\n');
    }

    Ok(super::success_with_summary(&json_result, &md))
}
