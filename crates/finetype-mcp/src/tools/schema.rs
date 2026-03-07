//! `schema` tool — export JSON Schema for a type.

use crate::FineTypeServer;
use rmcp::model::{CallToolResult, ErrorData};
use serde::Deserialize;

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

pub async fn handle(
    _server: &FineTypeServer,
    _request: SchemaRequest,
) -> Result<CallToolResult, ErrorData> {
    // TODO: Implement
    Err(ErrorData::internal_error(
        "schema tool not yet implemented",
        None,
    ))
}
