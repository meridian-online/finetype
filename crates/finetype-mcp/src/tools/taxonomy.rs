//! `taxonomy` tool — search and browse the type taxonomy, and export
//! per-type JSON Schema.
//!
//! Mirrors the CLI `taxonomy [KEY] [--domain D] [--category C] [-o FORMAT]`:
//! the default `summary` format browses/filters, and `json-schema` emits one
//! Draft-2020-12 schema per matched type via the shared
//! `json_schema::emit_type_schema()` helper — the same path CLI
//! `taxonomy KEY -o json-schema` uses. This absorbs the type-mode of the
//! retired `schema` tool (choice 0101 / 0070); table-mode lives on `profile`.

use crate::{json_schema, FineTypeServer};
use rmcp::model::{CallToolResult, ErrorData};
use serde::Deserialize;
use serde_json::json;

/// Output format for the `taxonomy` tool.
///
/// Mirrors the CLI's `-o` switch (subset: the CLI's plain/csv/markdown/arrow
/// are display-only modes without an MCP-meaningful counterpart).
#[derive(Debug, Default, Deserialize, schemars::JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum TaxonomyFormat {
    /// Default — a list of matched type entries plus a markdown table.
    #[default]
    Summary,
    /// One per-type JSON Schema document (Draft 2020-12) per matched type.
    /// Replaces the type-key branch of the retired `schema` tool.
    JsonSchema,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct TaxonomyRequest {
    /// Type key (e.g. "identity.person.email") or glob ("identity.person.*").
    /// When set it pins the result to that type / prefix and takes precedence
    /// over the domain/category/query filters — mirroring the CLI's positional
    /// KEY. Most useful with `format: "json-schema"`.
    #[schemars(
        description = "Type key or glob (e.g. \"identity.person.email\" or \"identity.*\"); takes precedence over domain/category/query"
    )]
    pub key: Option<String>,

    /// Filter by domain (e.g. "datetime", "identity").
    #[schemars(description = "Filter types by domain name")]
    pub domain: Option<String>,

    /// Filter by category within a domain (e.g. "date", "person").
    #[schemars(description = "Filter types by category name")]
    pub category: Option<String>,

    /// Free-text search query to match type names or descriptions.
    #[schemars(description = "Search query to filter types by name or description")]
    pub query: Option<String>,

    /// Output format. Default `summary` browses/filters; `json-schema` emits
    /// one per-type JSON Schema per matched type (the type-mode of the former
    /// `schema` tool).
    #[schemars(
        description = "Output format: \"summary\" (default, browse/filter) or \"json-schema\" (per-type JSON Schema)"
    )]
    #[serde(default)]
    pub format: TaxonomyFormat,
}

/// True when `label` is selected by an exact key or a glob prefix.
/// Glob shape mirrors the CLI: "domain.*", "domain.category.*", "*".
fn key_matches(label: &str, key: &str) -> bool {
    if key.contains('*') {
        let prefix = key.trim_end_matches(".*").trim_end_matches('*');
        if prefix.is_empty() {
            return true;
        }
        label.starts_with(prefix)
            && (label.len() == prefix.len() || label.as_bytes().get(prefix.len()) == Some(&b'.'))
    } else {
        label == key
    }
}

pub async fn handle(
    server: &FineTypeServer,
    request: TaxonomyRequest,
) -> Result<CallToolResult, ErrorData> {
    let taxonomy = server.taxonomy();
    let labels = taxonomy.labels();

    // Collect the labels that pass the filters. A `key` (exact or glob) is
    // authoritative and ignores domain/category/query — exactly like the CLI's
    // positional KEY.
    let mut matched_labels: Vec<&String> = Vec::new();

    for label in labels {
        let parts: Vec<&str> = label.split('.').collect();
        if parts.len() < 3 {
            continue;
        }
        let domain = parts[0];
        let category = parts[1];

        if let Some(ref key) = request.key {
            if key_matches(label, key) {
                matched_labels.push(label);
            }
            continue;
        }

        if let Some(ref d) = request.domain {
            if !domain.eq_ignore_ascii_case(d) {
                continue;
            }
        }
        if let Some(ref c) = request.category {
            if !category.eq_ignore_ascii_case(c) {
                continue;
            }
        }
        if let Some(ref q) = request.query {
            let q_lower = q.to_lowercase();
            let label_match = label.to_lowercase().contains(&q_lower);
            let desc_match = taxonomy
                .get(label)
                .and_then(|def| def.description.as_ref())
                .map(|desc| desc.to_lowercase().contains(&q_lower))
                .unwrap_or(false);
            if !label_match && !desc_match {
                continue;
            }
        }
        matched_labels.push(label);
    }

    // JSON Schema export — one per-type schema per matched type, via the shared
    // emitter the CLI uses (choice 0101 parity). Empty match → invalid_params,
    // mirroring the CLI's exit-1 on an unknown key / zero-match glob.
    if request.format == TaxonomyFormat::JsonSchema {
        if matched_labels.is_empty() {
            let target = request
                .key
                .as_deref()
                .or(request.query.as_deref())
                .or(request.domain.as_deref())
                .unwrap_or("<filters>");
            return Err(ErrorData::invalid_params(
                format!(
                    "No types matched '{}'. Use the taxonomy tool (summary format) to browse available types.",
                    target
                ),
                None,
            ));
        }

        let schemas: Vec<serde_json::Value> = matched_labels
            .iter()
            .filter_map(|label| {
                taxonomy
                    .get(label)
                    .map(|def| json_schema::emit_type_schema(label, def))
            })
            .collect();

        // Single match → bare object, multiple → array. Matches the schema
        // tool's old single-vs-array shape so existing callers see no change.
        let json_value = if schemas.len() == 1 {
            schemas.into_iter().next().unwrap()
        } else {
            serde_json::Value::Array(schemas)
        };

        let md = format!(
            "## JSON Schema Export\n\n**{} schema(s)** exported.\n",
            matched_labels.len()
        );
        return Ok(super::success_with_summary(&json_value, &md));
    }

    // Summary — list entries + markdown table (the original behaviour).
    let mut matches: Vec<serde_json::Value> = Vec::with_capacity(matched_labels.len());
    for label in &matched_labels {
        let parts: Vec<&str> = label.split('.').collect();
        let domain = parts[0];
        let category = parts[1];
        let type_name = parts[2..].join(".");

        let def = taxonomy.get(label);
        let broad_type = def
            .and_then(|d| d.broad_type.as_deref())
            .unwrap_or("unknown");
        let description = def.and_then(|d| d.description.as_deref()).unwrap_or("");
        let pii = def.and_then(|d| d.pii).unwrap_or(false);

        let mut entry = json!({
            "key": label,
            "domain": domain,
            "category": category,
            "type_name": type_name,
            "broad_type": broad_type,
            "description": description,
        });
        if pii {
            entry["pii"] = json!(true);
        }
        matches.push(entry);
    }

    // Build markdown summary
    let mut md = format!("## Taxonomy Results\n\n**{} types matched**", matches.len());
    if let Some(ref k) = request.key {
        md.push_str(&format!(" for `{}`", k));
    }
    if let Some(ref d) = request.domain {
        md.push_str(&format!(" in domain `{}`", d));
    }
    if let Some(ref c) = request.category {
        md.push_str(&format!(", category `{}`", c));
    }
    if let Some(ref q) = request.query {
        md.push_str(&format!(" matching \"{}\"", q));
    }
    md.push_str("\n\n");

    if !matches.is_empty() {
        md.push_str("| Key | Broad Type | Description |\n");
        md.push_str("|-----|-----------|-------------|\n");
        for entry in &matches {
            let key = entry["key"].as_str().unwrap_or("");
            let bt = entry["broad_type"].as_str().unwrap_or("");
            let desc = entry["description"]
                .as_str()
                .unwrap_or("")
                .chars()
                .take(60)
                .collect::<String>();
            md.push_str(&format!("| `{}` | {} | {} |\n", key, bt, desc));
        }
    }

    let json_value = serde_json::Value::Array(matches);
    Ok(super::success_with_summary(&json_value, &md))
}

#[cfg(test)]
mod tests {
    use super::key_matches;

    #[test]
    fn exact_key_matches_only_itself() {
        assert!(key_matches(
            "identity.person.email",
            "identity.person.email"
        ));
        assert!(!key_matches(
            "identity.person.email",
            "identity.person.name"
        ));
        // A prefix that is not the whole key does NOT match without a glob.
        assert!(!key_matches("identity.person.email", "identity.person"));
    }

    #[test]
    fn glob_matches_on_dot_boundary() {
        assert!(key_matches("identity.person.email", "identity.*"));
        assert!(key_matches("identity.person.email", "identity.person.*"));
        // Boundary-aware: "identity.*" must not match a sibling domain that
        // merely shares the prefix string.
        assert!(!key_matches("identityx.person.email", "identity.*"));
        // Bare "*" matches everything.
        assert!(key_matches("anything.at.all", "*"));
    }
}
