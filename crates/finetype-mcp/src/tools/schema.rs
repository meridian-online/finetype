//! `schema` tool — export JSON Schema for a type.

use crate::FineTypeServer;
use rmcp::model::{CallToolResult, ErrorData};
use serde::Deserialize;
use serde_json::json;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SchemaRequest {
    /// Type key (e.g. "identity.person.email") or glob pattern ("identity.person.*").
    #[schemars(description = "Type key in domain.category.type format, or glob pattern")]
    pub type_key: String,

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

pub async fn handle(
    server: &FineTypeServer,
    request: SchemaRequest,
) -> Result<CallToolResult, ErrorData> {
    let taxonomy = server.taxonomy();

    let schemas: Vec<(String, serde_json::Value)> = if request.type_key.contains('*') {
        // Glob pattern matching
        let prefix = request
            .type_key
            .trim_end_matches(".*")
            .trim_end_matches('*');

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
        match taxonomy.get(&request.type_key) {
            Some(def) => {
                vec![(
                    request.type_key.clone(),
                    build_json_schema(&request.type_key, def),
                )]
            }
            None => {
                return Err(ErrorData::invalid_params(
                    format!(
                        "Unknown type: '{}'. Use the taxonomy tool to browse available types.",
                        request.type_key
                    ),
                    None,
                ));
            }
        }
    };

    if schemas.is_empty() {
        return Err(ErrorData::invalid_params(
            format!("No types matching pattern '{}'", request.type_key),
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
        request.type_key
    );

    md.push_str("| Type Key | Broad Type | Has Validation |\n");
    md.push_str("|----------|-----------|----------------|\n");
    for (key, schema) in &schemas {
        let bt = schema
            .get("x-finetype-broad-type")
            .and_then(|v| v.as_str())
            .unwrap_or("—");
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
