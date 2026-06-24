//! `profile` tool — profile all columns in a CSV/JSON file.

use crate::datapackage;
use crate::json_schema;
use crate::FineTypeServer;
use rmcp::model::{CallToolResult, Content, ErrorData};
use serde::Deserialize;
use serde_json::json;

/// Output format for the `profile` tool.
///
/// Mirrors the CLI's `-o json | json-schema` switch (subset; the other
/// CLI formats — plain/csv/markdown/arrow — are CLI-only display modes
/// without an MCP-meaningful counterpart).
#[derive(Debug, Default, Deserialize, schemars::JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ProfileFormat {
    /// Default profile JSON shape — one object per column with type,
    /// confidence, domain, locale.
    #[default]
    Json,
    /// Table-level JSON Schema document (Draft 2020-12). Replaces the
    /// `path`/`data` branch of the legacy `schema` tool.
    JsonSchema,
    /// Frictionless Data Package descriptor (choice 0105) — the interoperable
    /// family-standard envelope; mirrors the CLI's `-o datapackage`.
    Datapackage,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ProfileRequest {
    /// Path to a CSV, JSON, or NDJSON file to profile.
    #[schemars(description = "Absolute path to the file to profile")]
    pub path: Option<String>,

    /// Inline CSV data (alternative to path, for small datasets).
    #[schemars(description = "Inline CSV content as a string (alternative to path)")]
    pub data: Option<String>,

    /// Output format. Default `json` returns one object per column. Set
    /// to `json-schema` to receive a table-level JSON Schema document
    /// suitable for direct input to `validate`.
    #[schemars(
        description = "Output format: \"json\" (default, profile object per column), \"json-schema\" (table-level JSON Schema), or \"datapackage\" (Frictionless Data Package descriptor)"
    )]
    #[serde(default)]
    pub format: ProfileFormat,

    /// Attach observed-data constraints to JSON Schema output
    /// (`minLength`/`maxLength`, `minimum`/`maximum`, `enum`,
    /// `x-finetype-null-rate`, `x-finetype-cardinality`). Only meaningful
    /// when `format == "json-schema"`.
    #[schemars(
        description = "When format is \"json-schema\", attach observed-data constraints (minLength/maxLength, minimum/maximum, enum, null-rate, cardinality)"
    )]
    #[serde(default)]
    pub stats: bool,

    /// Cardinality threshold for the JSON Schema `enum` keyword. Only
    /// meaningful when `format == "json-schema"` AND `stats: true`. 0
    /// disables `enum` emission. Default 50, matching the CLI.
    #[schemars(
        description = "Cardinality threshold for ENUM emission in JSON Schema output (0 disables; default 50)"
    )]
    #[serde(default = "default_enum_threshold")]
    pub enum_threshold: usize,

    /// Run JSON Schema validation on classified columns for data quality metrics.
    #[schemars(
        description = "Enable validation for data quality report (% valid, failing values)"
    )]
    #[serde(default)]
    pub validate: bool,
}

fn default_enum_threshold() -> usize {
    50
}

/// Parse CSV data from a string, returning (headers, columns_of_values).
fn parse_csv(csv_data: &str) -> Result<(Vec<String>, Vec<Vec<String>>), ErrorData> {
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

    Ok((headers, columns))
}

pub async fn handle(
    server: &FineTypeServer,
    request: ProfileRequest,
) -> Result<CallToolResult, ErrorData> {
    // ac-04 mirror: --stats / enum-threshold are only meaningful with
    // json-schema output. Don't error here — the CLI gates on conflict via
    // clap, and MCP callers can pass these defensively. Just ignore them
    // when format is Json.

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

    let (headers, columns) = parse_csv(&csv_data)?;
    let row_count = columns.iter().map(|c| c.len()).max().unwrap_or(0);
    let classifier = server.classifier().read().await;

    // ── json-schema branch ───────────────────────────────────────────────
    // Routes through the same `emit_table_schema` helper as the CLI's
    // `profile -o json-schema` so output is byte-identical across surfaces.
    if request.format == ProfileFormat::JsonSchema {
        let taxonomy = server.taxonomy();

        // Classify each column once; reuse the result for the schema input.
        let mut labels: Vec<String> = Vec::with_capacity(headers.len());
        for (col_idx, header) in headers.iter().enumerate() {
            let values = &columns[col_idx];
            if values.is_empty() {
                labels.push("unknown".to_string());
                continue;
            }
            let result = classifier
                .classify_column_with_header(values, header)
                .map_err(|e| {
                    ErrorData::internal_error(format!("Classification error: {e}"), None)
                })?;
            labels.push(result.label);
        }

        // Project into the helper's borrowed-input shape.
        let cols: Vec<json_schema::TableSchemaColumn<'_>> = headers
            .iter()
            .enumerate()
            .map(|(i, name)| {
                let values: &[String] = columns.get(i).map(|v| v.as_slice()).unwrap_or(&[]);
                let null_count = row_count.saturating_sub(values.len());
                json_schema::TableSchemaColumn {
                    name,
                    label: &labels[i],
                    values,
                    null_count,
                }
            })
            .collect();

        // Title / id are derived from the path when available.
        let (file_stem, file_id) = match &request.path {
            Some(p) => {
                let path = std::path::Path::new(p);
                let stem = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("table")
                    .to_string();
                let id = path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("data.csv")
                    .to_string();
                (stem, id)
            }
            None => ("table".to_string(), "data.csv".to_string()),
        };

        let schema = json_schema::emit_table_schema(
            &cols,
            &file_stem,
            &file_id,
            taxonomy,
            request.stats,
            request.enum_threshold,
        );

        let md = format!(
            "## Table Schema\n\n**{}** ({} columns, {} rows)\n",
            file_stem,
            headers.len(),
            row_count
        );
        return Ok(super::success_with_summary(&schema, &md));
    }

    // ── datapackage branch ───────────────────────────────────────────────
    // Routes through the same `emit_datapackage` helper as the CLI's
    // `profile -o datapackage` so output is identical across surfaces
    // (modulo the `created` timestamp). Locale enrichment is CLI-only, so
    // `x-finetype-locale` is omitted here.
    if request.format == ProfileFormat::Datapackage {
        let taxonomy = server.taxonomy();

        let mut labels: Vec<String> = Vec::with_capacity(headers.len());
        let mut confidences: Vec<Option<f32>> = Vec::with_capacity(headers.len());
        for (col_idx, header) in headers.iter().enumerate() {
            let values = &columns[col_idx];
            if values.is_empty() {
                labels.push("unknown".to_string());
                confidences.push(None);
                continue;
            }
            let result = classifier
                .classify_column_with_header(values, header)
                .map_err(|e| {
                    ErrorData::internal_error(format!("Classification error: {e}"), None)
                })?;
            confidences.push(Some(result.confidence));
            labels.push(result.label);
        }

        let cols: Vec<datapackage::DatapackageColumn<'_>> = headers
            .iter()
            .enumerate()
            .map(|(i, name)| {
                let values: &[String] = columns.get(i).map(|v| v.as_slice()).unwrap_or(&[]);
                datapackage::DatapackageColumn {
                    name,
                    label: &labels[i],
                    values,
                    confidence: confidences[i],
                    locale: None,
                }
            })
            .collect();

        let resource = match &request.path {
            Some(p) => {
                datapackage::ResourceMeta::for_path(std::path::Path::new(p), csv_data.as_bytes())
            }
            None => datapackage::ResourceMeta::for_inline(csv_data.as_bytes()),
        };

        let descriptor =
            datapackage::emit_datapackage(&cols, &resource, taxonomy, request.enum_threshold);
        let md = format!(
            "## Data Package\n\n**{}** ({} columns, {} rows)\n",
            resource.name,
            headers.len(),
            row_count
        );
        return Ok(super::success_with_summary(&descriptor, &md));
    }

    // ── default json branch — unchanged from prior behaviour ─────────────
    let mut profiles = Vec::new();

    for (col_idx, header) in headers.iter().enumerate() {
        let values = &columns[col_idx];
        if values.is_empty() {
            profiles.push(json!({
                "name": header,
                "type": "unknown",
                "confidence": 0.0,
                "domain": "unknown",
                "is_generic": true,
                "samples_used": 0,
                "detected_locale": null,
            }));
            continue;
        }

        let result = classifier
            .classify_column_with_header(values, header)
            .map_err(|e| ErrorData::internal_error(format!("Classification error: {e}"), None))?;

        let domain = result.label.split('.').next().unwrap_or("unknown");

        let mut col_json = json!({
            "name": header,
            "type": result.label,
            "confidence": (result.confidence * 1000.0).round() / 1000.0,
            "domain": domain,
            "is_generic": result.is_generic,
            "samples_used": result.samples_used,
            "detected_locale": result.detected_locale,
        });

        // Validation quality metrics
        if request.validate {
            if let Some(validator) = server.taxonomy().get_validator(&result.label) {
                let mut valid = 0usize;
                let mut invalid = 0usize;
                let mut invalid_samples: Vec<String> = Vec::new();

                for v in values.iter() {
                    if validator.is_valid(v) {
                        valid += 1;
                    } else {
                        invalid += 1;
                        if invalid_samples.len() < 5 {
                            invalid_samples.push(v.clone());
                        }
                    }
                }

                let total = valid + invalid;
                let pct = if total > 0 {
                    (valid as f64 / total as f64 * 1000.0).round() / 10.0
                } else {
                    0.0
                };

                col_json.as_object_mut().unwrap().insert(
                    "validation".to_string(),
                    json!({
                        "valid": valid,
                        "invalid": invalid,
                        "valid_pct": pct,
                        "invalid_samples": invalid_samples,
                    }),
                );
            }
        }

        profiles.push(col_json);
    }

    // Build markdown summary
    let mut md = String::from("## Profile Summary\n\n");
    md.push_str("| Column | Type | Confidence | Domain |\n");
    md.push_str("|--------|------|------------|--------|\n");
    for p in &profiles {
        md.push_str(&format!(
            "| {} | `{}` | {:.1}% | {} |\n",
            p["name"].as_str().unwrap_or(""),
            p["type"].as_str().unwrap_or(""),
            p["confidence"].as_f64().unwrap_or(0.0) * 100.0,
            p["domain"].as_str().unwrap_or(""),
        ));
    }

    let json_value = json!(profiles);
    Ok(CallToolResult::success(vec![
        Content::text(serde_json::to_string_pretty(&json_value).unwrap()),
        Content::text(md),
    ]))
}
