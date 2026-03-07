//! `taxonomy` tool — search and browse the type taxonomy.

use crate::FineTypeServer;
use rmcp::model::{CallToolResult, ErrorData};
use serde::Deserialize;

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
    _server: &FineTypeServer,
    _request: TaxonomyRequest,
) -> Result<CallToolResult, ErrorData> {
    // TODO: Implement
    Err(ErrorData::internal_error(
        "taxonomy tool not yet implemented",
        None,
    ))
}
