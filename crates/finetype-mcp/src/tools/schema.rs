//! `schema` tool — export JSON Schema for a type or a CSV file.

use crate::FineTypeServer;
use rmcp::model::{CallToolResult, ErrorData};
use serde::Deserialize;
use serde_json::json;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SchemaRequest {
    /// Type key (e.g. "identity.person.email") or glob pattern ("identity.person.*").
    /// Omit when using `path` or `data` for table-level schema generation.
    #[schemars(description = "Type key in domain.category.type format, or glob pattern")]
    pub type_key: Option<String>,

    /// Path to a CSV file for table-level JSON Schema generation.
    #[schemars(description = "Absolute path to a CSV file to generate a table-level JSON Schema")]
    pub path: Option<String>,

    /// Inline CSV data for table-level JSON Schema generation.
    #[schemars(description = "Inline CSV content for table-level JSON Schema generation")]
    pub data: Option<String>,

    /// Pretty-print the JSON output.
    #[schemars(description = "Pretty-print the JSON Schema output")]
    #[serde(default)]
    pub pretty: bool,
}

/// Build a JSON Schema document for a type definition, matching the CLI's output.
fn build_json_schema(key: &str, def: &finetype_core::Definition) -> serde_json::Value {
    let mut schema = serde_json::Map::new();

    // Standard JSON Schema metadata
    schema.insert(
        "$schema".into(),
        json!("https://json-schema.org/draft/2020-12/schema"),
    );
    schema.insert(
        "$id".into(),
        json!(format!("https://meridian.online/schemas/{}", key)),
    );

    if let Some(title) = &def.title {
        schema.insert("title".into(), json!(title));
    }
    if let Some(desc) = &def.description {
        schema.insert("description".into(), json!(desc.trim()));
    }

    // Merge validation keywords from the type's validation schema
    if let Some(validation) = &def.validation {
        let val_schema = validation.to_json_schema();
        if let serde_json::Value::Object(val_obj) = val_schema {
            for (k, v) in val_obj {
                schema.insert(k, v);
            }
        }
    } else {
        schema.insert("type".into(), json!("string"));
    }

    // FineType DDL extension fields
    schema.insert("x-finetype-key".into(), json!(key));
    if let Some(broad_type) = &def.broad_type {
        schema.insert("x-finetype-broad-type".into(), json!(broad_type));
    }
    if let Some(transform) = &def.transform {
        schema.insert("x-finetype-transform".into(), json!(transform));
    }
    if let Some(fmt) = &def.format_string {
        schema.insert("x-finetype-format-string".into(), json!(fmt));
    }
    if let Some(alt) = &def.format_string_alt {
        schema.insert("x-format-string-alt".into(), json!(alt));
    }

    serde_json::Value::Object(schema)
}

/// Handle type-key based schema lookup (original behaviour).
fn handle_type_key(server: &FineTypeServer, type_key: &str) -> Result<CallToolResult, ErrorData> {
    let taxonomy = server.taxonomy();

    let schemas: Vec<(String, serde_json::Value)> = if type_key.contains('*') {
        // Glob pattern matching
        let prefix = type_key.trim_end_matches(".*").trim_end_matches('*');

        let mut matched: Vec<(String, serde_json::Value)> = taxonomy
            .labels()
            .iter()
            .filter(|k| {
                if prefix.is_empty() {
                    true
                } else {
                    k.starts_with(prefix)
                        && (k.len() == prefix.len()
                            || k.as_bytes().get(prefix.len()) == Some(&b'.'))
                }
            })
            .filter_map(|k| {
                taxonomy
                    .get(k)
                    .map(|def| (k.clone(), build_json_schema(k, def)))
            })
            .collect();
        matched.sort_by(|(a, _), (b, _)| a.cmp(b));
        matched
    } else {
        // Exact match
        match taxonomy.get(type_key) {
            Some(def) => {
                vec![(type_key.to_string(), build_json_schema(type_key, def))]
            }
            None => {
                return Err(ErrorData::invalid_params(
                    format!(
                        "Unknown type: '{}'. Use the taxonomy tool to browse available types.",
                        type_key
                    ),
                    None,
                ));
            }
        }
    };

    if schemas.is_empty() {
        return Err(ErrorData::invalid_params(
            format!("No types matching pattern '{}'", type_key),
            None,
        ));
    }

    // Build the JSON output
    let json_output = if schemas.len() == 1 {
        schemas[0].1.clone()
    } else {
        serde_json::Value::Array(schemas.iter().map(|(_, s)| s.clone()).collect())
    };

    // Build markdown summary
    let mut md = format!(
        "## JSON Schema Export\n\n**{} schema(s)** for `{}`\n\n",
        schemas.len(),
        type_key
    );

    md.push_str("| Type Key | Broad Type | Has Validation |\n");
    md.push_str("|----------|-----------|----------------|\n");
    for (key, schema) in &schemas {
        let bt = schema
            .get("x-finetype-broad-type")
            .and_then(|v| v.as_str())
            .unwrap_or("---");
        let has_val = schema.get("pattern").is_some()
            || schema.get("enum").is_some()
            || schema.get("minimum").is_some();
        md.push_str(&format!(
            "| `{}` | {} | {} |\n",
            key,
            bt,
            if has_val { "Yes" } else { "No" }
        ));
    }

    Ok(super::success_with_summary(&json_output, &md))
}

/// Handle file-based table-level schema generation.
async fn handle_file(
    server: &FineTypeServer,
    csv_data: &str,
    file_name: Option<&str>,
) -> Result<CallToolResult, ErrorData> {
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
    let row_count = records.len();

    let mut columns: Vec<Vec<String>> = vec![Vec::new(); headers.len()];
    for record in &records {
        for (col_idx, col) in columns.iter_mut().enumerate() {
            let val = record.get(col_idx).unwrap_or("").to_string();
            if !val.is_empty() {
                col.push(val);
            }
        }
    }

    let classifier = server.classifier().read().await;
    let taxonomy = server.taxonomy();

    // Derive table name from file_name
    let table_title = file_name
        .and_then(|f| {
            std::path::Path::new(f)
                .file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| "table".to_string());

    let schema_id = file_name.unwrap_or("data.csv");

    let mut properties = serde_json::Map::new();
    let mut required: Vec<String> = Vec::new();
    let mut col_summary: Vec<(String, String, f32)> = Vec::new();

    for (col_idx, header) in headers.iter().enumerate() {
        let values = &columns[col_idx];
        let null_count = row_count - values.len();

        if values.is_empty() {
            let mut prop = serde_json::Map::new();
            prop.insert("type".into(), json!("string"));
            prop.insert("x-finetype-label".into(), json!("unknown"));
            properties.insert(header.clone(), serde_json::Value::Object(prop));
            col_summary.push((header.clone(), "unknown".to_string(), 0.0));
            continue;
        }

        let result = classifier
            .classify_column_with_header(values, header)
            .map_err(|e| ErrorData::internal_error(format!("Classification error: {e}"), None))?;

        let mut prop = serde_json::Map::new();

        if let Some(def) = taxonomy.get(&result.label) {
            // Merge validation keywords from the type definition
            if let Some(validation) = &def.validation {
                let val_schema = validation.to_json_schema();
                if let serde_json::Value::Object(val_obj) = val_schema {
                    for (k, v) in val_obj {
                        prop.insert(k, v);
                    }
                }
            } else {
                prop.insert("type".into(), json!("string"));
            }

            // x-finetype extension fields
            prop.insert("x-finetype-label".into(), json!(result.label));
            let domain = result.label.split('.').next().unwrap_or("");
            prop.insert("x-finetype-domain".into(), json!(domain));
            prop.insert(
                "x-finetype-confidence".into(),
                json!((result.confidence * 1000.0).round() / 1000.0),
            );
            if let Some(broad_type) = &def.broad_type {
                let duckdb_type = finetype_core::DdlInfo::duckdb_type_from_broad_type(broad_type);
                prop.insert("x-finetype-broad-type".into(), json!(duckdb_type));
            }
            if let Some(transform) = &def.transform {
                prop.insert("x-finetype-transform".into(), json!(transform));
            }
            if let Some(fmt) = &def.format_string {
                prop.insert("x-finetype-format-string".into(), json!(fmt));
            }
        } else {
            prop.insert("type".into(), json!("string"));
            prop.insert("x-finetype-label".into(), json!(result.label));
            let domain = result.label.split('.').next().unwrap_or("");
            prop.insert("x-finetype-domain".into(), json!(domain));
            prop.insert(
                "x-finetype-confidence".into(),
                json!((result.confidence * 1000.0).round() / 1000.0),
            );
        }

        // Columns with no nulls are required
        if null_count == 0 {
            required.push(header.clone());
        }

        col_summary.push((header.clone(), result.label.clone(), result.confidence));
        properties.insert(header.clone(), serde_json::Value::Object(prop));
    }

    // Build the table-level JSON Schema
    let mut schema = serde_json::Map::new();
    schema.insert(
        "$schema".into(),
        json!("https://json-schema.org/draft/2020-12/schema"),
    );
    schema.insert("$id".into(), json!(format!("finetype://{}", schema_id)));
    schema.insert("title".into(), json!(table_title));
    schema.insert("type".into(), json!("object"));
    schema.insert("properties".into(), serde_json::Value::Object(properties));
    if !required.is_empty() {
        let mut req_sorted = required;
        req_sorted.sort();
        schema.insert("required".into(), json!(req_sorted));
    }

    let schema_value = serde_json::Value::Object(schema);

    // Build markdown summary
    let mut md = format!(
        "## Table Schema\n\n**{}** ({} columns, {} rows)\n\n",
        table_title,
        headers.len(),
        row_count
    );
    md.push_str("| Column | Type | Confidence |\n");
    md.push_str("|--------|------|------------|\n");
    for (name, label, conf) in &col_summary {
        md.push_str(&format!(
            "| {} | `{}` | {:.1}% |\n",
            name,
            label,
            conf * 100.0,
        ));
    }

    Ok(super::success_with_summary(&schema_value, &md))
}

pub async fn handle(
    server: &FineTypeServer,
    request: SchemaRequest,
) -> Result<CallToolResult, ErrorData> {
    // Determine mode: file-based or type-key based
    match (&request.type_key, &request.path, &request.data) {
        // File-based: path provided
        (_, Some(path), _) => {
            let csv_data = std::fs::read_to_string(path).map_err(|e| {
                ErrorData::invalid_params(format!("Failed to read file: {e}"), None)
            })?;
            let file_name = std::path::Path::new(path)
                .file_name()
                .and_then(|s| s.to_str())
                .map(|s| s.to_string());
            handle_file(server, &csv_data, file_name.as_deref()).await
        }
        // File-based: inline data provided (no type_key)
        (None, None, Some(data)) => handle_file(server, data, None).await,
        // Type-key based
        (Some(type_key), None, _) => handle_type_key(server, type_key),
        // Neither
        (None, None, None) => Err(ErrorData::invalid_params(
            "Provide either 'type_key' for type schema lookup, or 'path'/'data' for table-level schema generation",
            None,
        )),
    }
}
