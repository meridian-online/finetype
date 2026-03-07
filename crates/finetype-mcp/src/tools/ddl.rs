//! `ddl` tool — generate CREATE TABLE DDL from file profiling.

use crate::FineTypeServer;
use rmcp::model::{CallToolResult, Content, ErrorData};
use serde::Deserialize;
use serde_json::json;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DdlRequest {
    /// Path to a CSV, JSON, or NDJSON file.
    #[schemars(description = "Absolute path to the file to generate DDL for")]
    pub path: Option<String>,

    /// Inline CSV data (alternative to path, for small datasets).
    #[schemars(description = "Inline CSV content as a string (alternative to path)")]
    pub data: Option<String>,

    /// Override the table name (default: derived from filename).
    #[schemars(description = "Table name for the CREATE TABLE statement")]
    pub table_name: Option<String>,
}

/// Check if a column name needs quoting in SQL.
fn needs_quoting(name: &str) -> bool {
    name.contains(' ')
        || name.contains('-')
        || name.contains('.')
        || name.contains('(')
        || name.contains(')')
        || name.contains(',')
        || name.starts_with(|c: char| c.is_ascii_digit())
}

/// Quote a column name with double quotes if needed.
fn quote_column(name: &str) -> String {
    if needs_quoting(name) {
        format!("\"{}\"", name.replace('"', "\"\""))
    } else {
        name.to_string()
    }
}

pub async fn handle(
    server: &FineTypeServer,
    request: DdlRequest,
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

    // Parse CSV
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(csv_data.as_bytes());

    let headers: Vec<String> = reader
        .headers()
        .map_err(|e| ErrorData::invalid_params(format!("Failed to parse CSV headers: {e}"), None))?
        .iter()
        .map(|h| h.to_string())
        .collect();

    let records: Vec<csv::StringRecord> = reader.records().filter_map(|r| r.ok()).collect();

    let mut columns: Vec<Vec<String>> = vec![Vec::new(); headers.len()];
    for record in &records {
        for (col_idx, col) in columns.iter_mut().enumerate() {
            let val = record.get(col_idx).unwrap_or("").to_string();
            if !val.is_empty() {
                col.push(val);
            }
        }
    }

    // Determine table name
    let table_name = request.table_name.unwrap_or_else(|| {
        request
            .path
            .as_ref()
            .and_then(|p| {
                std::path::Path::new(p)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .map(|s| s.to_string())
            })
            .unwrap_or_else(|| "data".to_string())
    });

    let classifier = server.classifier().read().await;
    let taxonomy = server.taxonomy();

    let mut col_details = Vec::new();
    let mut ddl_columns = Vec::new();

    for (col_idx, header) in headers.iter().enumerate() {
        let values = &columns[col_idx];

        let (label, sql_type, broad_type) = if values.is_empty() {
            (
                "unknown".to_string(),
                "VARCHAR".to_string(),
                "VARCHAR".to_string(),
            )
        } else {
            let result = classifier
                .classify_column_with_header(values, header)
                .map_err(|e| {
                    ErrorData::internal_error(format!("Classification error: {e}"), None)
                })?;

            let ddl = taxonomy.ddl_info(&result.label);
            let sql_type = ddl
                .as_ref()
                .map(|d| d.duckdb_type.clone())
                .unwrap_or_else(|| "VARCHAR".to_string());
            let broad = taxonomy
                .get(&result.label)
                .and_then(|d| d.broad_type.clone())
                .unwrap_or_else(|| "VARCHAR".to_string());

            (result.label, sql_type, broad)
        };

        let quoted = quote_column(header);
        ddl_columns.push(format!(
            "    {} {} -- finetype: {}",
            quoted, sql_type, label
        ));

        col_details.push(json!({
            "name": header,
            "finetype_label": label,
            "sql_type": sql_type,
            "broad_type": broad_type,
        }));
    }

    let ddl = format!(
        "CREATE TABLE {} (\n{}\n);",
        quote_column(&table_name),
        ddl_columns.join(",\n")
    );

    let result_json = json!({
        "ddl": ddl,
        "columns": col_details,
    });

    let md = format!("## Generated DDL\n\n```sql\n{}\n```\n", ddl);

    Ok(CallToolResult::success(vec![
        Content::text(serde_json::to_string_pretty(&result_json).unwrap()),
        Content::text(md),
    ]))
}
