//! `profile` tool — profile all columns in a CSV/JSON file.

use crate::FineTypeServer;
use rmcp::model::{CallToolResult, Content, ErrorData};
use serde::Deserialize;
use serde_json::json;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ProfileRequest {
    /// Path to a CSV, JSON, or NDJSON file to profile.
    #[schemars(description = "Absolute path to the file to profile")]
    pub path: Option<String>,

    /// Inline CSV data (alternative to path, for small datasets).
    #[schemars(description = "Inline CSV content as a string (alternative to path)")]
    pub data: Option<String>,

    /// Run JSON Schema validation on classified columns for data quality metrics.
    #[schemars(
        description = "Enable validation for data quality report (% valid, failing values)"
    )]
    #[serde(default)]
    pub validate: bool,
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
    let classifier = server.classifier().read().await;

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
