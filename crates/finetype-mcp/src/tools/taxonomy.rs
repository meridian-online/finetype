//! `taxonomy` tool — search and browse the type taxonomy.

use crate::FineTypeServer;
use rmcp::model::{CallToolResult, ErrorData};
use serde::Deserialize;
use serde_json::json;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct TaxonomyRequest {
    /// Filter by domain (e.g. "datetime", "identity").
    #[schemars(description = "Filter types by domain name")]
    pub domain: Option<String>,

    /// Filter by category within a domain (e.g. "date", "person").
    #[schemars(description = "Filter types by category name")]
    pub category: Option<String>,

    /// Free-text search query to match type names or descriptions.
    #[schemars(description = "Search query to filter types by name or description")]
    pub query: Option<String>,
}

pub async fn handle(
    server: &FineTypeServer,
    request: TaxonomyRequest,
) -> Result<CallToolResult, ErrorData> {
    let taxonomy = server.taxonomy();
    let labels = taxonomy.labels();

    // Filter labels based on request parameters
    let mut matches: Vec<serde_json::Value> = Vec::new();

    for label in labels {
        let parts: Vec<&str> = label.split('.').collect();
        if parts.len() < 3 {
            continue;
        }
        let domain = parts[0];
        let category = parts[1];
        let type_name = parts[2..].join(".");

        // Filter by domain
        if let Some(ref d) = request.domain {
            if !domain.eq_ignore_ascii_case(d) {
                continue;
            }
        }

        // Filter by category
        if let Some(ref c) = request.category {
            if !category.eq_ignore_ascii_case(c) {
                continue;
            }
        }

        // Filter by query (case-insensitive substring match on label or description)
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

        // Build result entry
        let def = taxonomy.get(label);
        let broad_type = def
            .and_then(|d| d.broad_type.as_deref())
            .unwrap_or("unknown");
        let description = def.and_then(|d| d.description.as_deref()).unwrap_or("");

        matches.push(json!({
            "key": label,
            "domain": domain,
            "category": category,
            "type_name": type_name,
            "broad_type": broad_type,
            "description": description,
        }));
    }

    // Build markdown summary
    let mut md = format!("## Taxonomy Results\n\n**{} types matched**", matches.len());

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
