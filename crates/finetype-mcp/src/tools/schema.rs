//! `schema` tool — export JSON Schema for a type or a CSV file.
//!
//! MCP audit follow-up in v0.6.20: the CLI `schema` verb was retired in
//! v0.6.19 (card 0006 / MADR 0070) — type-mode migrated to
//! `taxonomy KEY -o json-schema`, table-mode to `profile -f FILE -o json-schema`.
//! This MCP tool's type-key branch is RETAINED for v0.6.19 per the
//! visibility-cleanup carve-out (memo 2026-04-27-mcp-surface-audit
//! line 116). The v0.6.20 audit will mirror the CLI fold and surface
//! the equivalent migration on the MCP side.

use crate::FineTypeServer;
use rmcp::model::{CallToolResult, ErrorData};
use serde::Deserialize;
use serde_json::json;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SchemaRequest {
    /// Type key (e.g. "identity.person.email") or glob pattern ("identity.person.*").
    #[schemars(description = "Type key in domain.category.type format, or glob pattern")]
    pub type_key: Option<String>,

    /// DEPRECATED — table-mode JSON Schema export was folded into the
    /// `profile` tool in v0.6.19. Use `profile` with `format: "json-schema"`.
    #[schemars(
        description = "DEPRECATED in v0.6.19 — use profile with format: \"json-schema\" for table-level schema generation"
    )]
    pub path: Option<String>,

    /// DEPRECATED — see `path`.
    #[schemars(
        description = "DEPRECATED in v0.6.19 — use profile with format: \"json-schema\" for table-level schema generation"
    )]
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
    schema.insert("x-finetype-pii".into(), json!(def.pii.unwrap_or(false)));

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

/// Migration string for the deleted table-mode of `schema`.
///
/// Card 0003 (v0.6.19) folds table-mode JSON Schema export into the
/// `profile` tool's new `format: "json-schema"` parameter. The
/// `path`/`data` branch of `schema` therefore hard-errors here. The
/// type-key branch (`type_key: "domain.category.type"`) remains
/// supported until card 0006 deletes the `schema` tool entirely.
const TABLE_MODE_MIGRATION: &str = "Table-mode schema export was folded into profile in v0.6.19. \
     Use profile with format: \"json-schema\" instead.";

pub async fn handle(
    server: &FineTypeServer,
    request: SchemaRequest,
) -> Result<CallToolResult, ErrorData> {
    // Determine mode: file-based or type-key based
    match (&request.type_key, &request.path, &request.data) {
        // File-based: path provided — folded into profile in v0.6.19 (card 0003)
        (_, Some(_), _) => Err(ErrorData::invalid_params(TABLE_MODE_MIGRATION, None)),
        // File-based: inline data provided (no type_key) — same fold
        (None, None, Some(_)) => Err(ErrorData::invalid_params(TABLE_MODE_MIGRATION, None)),
        // Type-key based — preserved (card 0006 owns the eventual deletion)
        (Some(type_key), None, _) => handle_type_key(server, type_key),
        // Neither
        (None, None, None) => Err(ErrorData::invalid_params(
            "Provide 'type_key' for per-type schema lookup. Table-level schema export was folded into the profile tool in v0.6.19 — use profile with format: \"json-schema\" instead.",
            None,
        )),
    }
}
