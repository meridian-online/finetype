//! `profile` tool — profile all columns in a CSV/JSON file.

use crate::FineTypeServer;
use rmcp::model::{CallToolResult, ErrorData};
use serde::Deserialize;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ProfileRequest {
    /// Path to a CSV, JSON, or NDJSON file to profile.
    #[schemars(description = "Absolute path to the file to profile")]
    pub path: Option<String>,

    /// Inline CSV data (alternative to path, for small datasets).
    #[schemars(description = "Inline CSV content as a string (alternative to path)")]
    pub data: Option<String>,

    /// Run JSON Schema validation on classified columns for data quality metrics.
    #[schemars(
        description = "Enable validation for data quality report (% valid, failing values)"
    )]
    #[serde(default)]
    pub validate: bool,
}

pub async fn handle(
    _server: &FineTypeServer,
    _request: ProfileRequest,
) -> Result<CallToolResult, ErrorData> {
    // TODO: Implement
    Err(ErrorData::internal_error(
        "profile tool not yet implemented",
        None,
    ))
}
