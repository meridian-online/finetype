//! MCP resource handlers for FineType taxonomy browsing.
//!
//! Resources expose the taxonomy at these URIs:
//! - `finetype://taxonomy` — Full taxonomy overview
//! - `finetype://taxonomy/{domain}` — Types in a domain
//! - `finetype://taxonomy/{domain}.{category}.{type}` — Single type definition

use finetype_core::Taxonomy;
use rmcp::model::*;

/// List all available taxonomy resources.
pub fn list_resources(taxonomy: &Taxonomy) -> Result<ListResourcesResult, ErrorData> {
    let mut resources = vec![Annotated::new(
        RawResource::new("finetype://taxonomy", "FineType Taxonomy")
            .with_description(
                "Complete type taxonomy overview — domains, categories, and type counts",
            )
            .with_mime_type("application/json"),
        None,
    )];

    // Add per-domain resources
    for domain in taxonomy.domains() {
        resources.push(Annotated::new(
            RawResource::new(
                format!("finetype://taxonomy/{}", domain),
                format!("{} domain", domain),
            )
            .with_description(format!("Type definitions in the {} domain", domain))
            .with_mime_type("application/json"),
            None,
        ));
    }

    Ok(ListResourcesResult {
        meta: None,
        next_cursor: None,
        resources,
    })
}

/// Read a specific taxonomy resource by URI.
pub fn read_resource(taxonomy: &Taxonomy, uri: &str) -> Result<ReadResourceResult, ErrorData> {
    let path = uri
        .strip_prefix("finetype://taxonomy")
        .ok_or_else(|| ErrorData::invalid_params(format!("Unknown resource URI: {}", uri), None))?;

    let json = if path.is_empty() || path == "/" {
        // Root: taxonomy overview
        taxonomy_overview(taxonomy)
    } else {
        let path = path.trim_start_matches('/');
        let parts: Vec<&str> = path.split('.').collect();

        match parts.len() {
            1 => {
                // Domain: e.g., "datetime"
                domain_detail(taxonomy, parts[0])?
            }
            3 => {
                // Full type key: e.g., "datetime.date.iso_8601"
                type_detail(taxonomy, path)?
            }
            _ => {
                return Err(ErrorData::invalid_params(
                    format!(
                        "Invalid taxonomy path: '{}'. Use domain name or full type key (domain.category.type).",
                        path
                    ),
                    None,
                ));
            }
        }
    };

    Ok(ReadResourceResult::new(vec![ResourceContents::text(
        serde_json::to_string_pretty(&json).unwrap_or_default(),
        uri,
    )
    .with_mime_type("application/json")]))
}

fn taxonomy_overview(taxonomy: &Taxonomy) -> serde_json::Value {
    let domains = taxonomy.domains();
    let mut domain_counts = serde_json::Map::new();
    for domain in &domains {
        let count = taxonomy
            .labels()
            .iter()
            .filter(|l| l.starts_with(&format!("{}.", domain)))
            .count();
        domain_counts.insert(domain.clone(), serde_json::Value::from(count));
    }

    serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "total_types": taxonomy.labels().len(),
        "domains": domains,
        "types_per_domain": domain_counts,
    })
}

fn domain_detail(taxonomy: &Taxonomy, domain: &str) -> Result<serde_json::Value, ErrorData> {
    let prefix = format!("{}.", domain);
    let types: Vec<&str> = taxonomy
        .labels()
        .iter()
        .filter(|l| l.starts_with(&prefix))
        .map(|l| l.as_str())
        .collect();

    if types.is_empty() {
        return Err(ErrorData::invalid_params(
            format!("Unknown domain: '{}'", domain),
            None,
        ));
    }

    // Group by category
    let mut categories: std::collections::BTreeMap<String, Vec<&str>> =
        std::collections::BTreeMap::new();
    for key in &types {
        let parts: Vec<&str> = key.split('.').collect();
        if parts.len() >= 2 {
            categories
                .entry(parts[1].to_string())
                .or_default()
                .push(key);
        }
    }

    Ok(serde_json::json!({
        "domain": domain,
        "type_count": types.len(),
        "categories": categories,
    }))
}

fn type_detail(taxonomy: &Taxonomy, type_key: &str) -> Result<serde_json::Value, ErrorData> {
    let definition = taxonomy
        .get(type_key)
        .ok_or_else(|| ErrorData::invalid_params(format!("Unknown type: '{}'", type_key), None))?;

    let parts: Vec<&str> = type_key.split('.').collect();

    Ok(serde_json::json!({
        "key": type_key,
        "domain": parts.first().unwrap_or(&""),
        "category": parts.get(1).unwrap_or(&""),
        "type_name": parts.get(2).unwrap_or(&""),
        "broad_type": definition.broad_type,
        "format_string": definition.format_string,
        "description": definition.description,
        "transform": definition.transform,
        "designation": format!("{:?}", definition.designation),
        "samples": definition.samples,
    }))
}
