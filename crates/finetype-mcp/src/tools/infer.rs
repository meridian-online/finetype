//! `infer` tool — classify string values into semantic types.

use crate::FineTypeServer;
use rmcp::model::{CallToolResult, ErrorData};
use serde::Deserialize;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct InferRequest {
    /// One or more string values to classify. For column-mode inference, pass multiple values.
    #[schemars(description = "String values to classify (single value or list for column-mode)")]
    pub values: Vec<String>,

    /// Optional column header name for context-aware classification.
    #[schemars(description = "Column header name for disambiguation (e.g. 'email', 'country')")]
    pub header: Option<String>,
}

pub async fn handle(
    _server: &FineTypeServer,
    _request: InferRequest,
) -> Result<CallToolResult, ErrorData> {
    // TODO: Implement — teammate will fill this in
    Err(ErrorData::internal_error(
        "infer tool not yet implemented",
        None,
    ))
}
