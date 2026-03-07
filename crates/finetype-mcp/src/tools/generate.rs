//! `generate` tool — generate synthetic sample data for a type.

use crate::FineTypeServer;
use rmcp::model::{CallToolResult, ErrorData};
use serde::Deserialize;

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
    _server: &FineTypeServer,
    _request: GenerateRequest,
) -> Result<CallToolResult, ErrorData> {
    // TODO: Implement
    Err(ErrorData::internal_error(
        "generate tool not yet implemented",
        None,
    ))
}
