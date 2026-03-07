//! `generate` tool — generate synthetic sample data for a type.

use crate::FineTypeServer;
use finetype_core::Generator;
use rmcp::model::{CallToolResult, Content, ErrorData};
use serde::Deserialize;
use serde_json::json;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GenerateRequest {
    /// Type key (e.g. "identity.person.email").
    #[schemars(description = "Type key in domain.category.type format")]
    pub type_key: String,

    /// Number of samples to generate (default: 10).
    #[schemars(description = "Number of sample values to generate")]
    pub count: Option<usize>,

    /// Locale for locale-specific types (e.g. "en_US", "de_DE").
    #[schemars(description = "Locale code for locale-specific generation")]
    pub locale: Option<String>,
}

pub async fn handle(
    server: &FineTypeServer,
    request: GenerateRequest,
) -> Result<CallToolResult, ErrorData> {
    let type_key = &request.type_key;
    let count = request.count.unwrap_or(10);

    // Verify the type exists in taxonomy
    if server.taxonomy().get(type_key).is_none() {
        return Err(ErrorData::invalid_params(
            format!(
                "Unknown type key: '{type_key}'. Use the taxonomy tool to browse available types."
            ),
            None,
        ));
    }

    // Create a generator from the taxonomy
    // We need to clone the taxonomy since Generator takes ownership
    let taxonomy_clone = server.taxonomy().as_ref().clone();
    let mut generator = Generator::new(taxonomy_clone);

    let mut samples = Vec::with_capacity(count);
    for _ in 0..count {
        match generator.generate_value(type_key) {
            Ok(value) => samples.push(value),
            Err(e) => {
                return Err(ErrorData::internal_error(
                    format!("Generation failed for '{type_key}': {e}"),
                    None,
                ));
            }
        }
    }

    let result_json = json!({
        "type_key": type_key,
        "count": samples.len(),
        "samples": samples,
    });

    // Markdown summary
    let mut md = format!("## Generated Samples for `{type_key}`\n\n");
    for (i, sample) in samples.iter().enumerate() {
        md.push_str(&format!("{}. `{}`\n", i + 1, sample));
    }

    Ok(CallToolResult::success(vec![
        Content::text(serde_json::to_string_pretty(&result_json).unwrap()),
        Content::text(md),
    ]))
}
