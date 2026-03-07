//! `ddl` tool — generate CREATE TABLE DDL from file profiling.

use crate::FineTypeServer;
use rmcp::model::{CallToolResult, ErrorData};
use serde::Deserialize;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DdlRequest {
    /// Path to a CSV, JSON, or NDJSON file.
    #[schemars(description = "Absolute path to the file to generate DDL for")]
    pub path: Option<String>,

    /// Inline CSV data (alternative to path, for small datasets).
    #[schemars(description = "Inline CSV content as a string (alternative to path)")]
    pub data: Option<String>,

    /// Override the table name (default: derived from filename).
    #[schemars(description = "Table name for the CREATE TABLE statement")]
    pub table_name: Option<String>,
}

pub async fn handle(
    _server: &FineTypeServer,
    _request: DdlRequest,
) -> Result<CallToolResult, ErrorData> {
    // TODO: Implement
    Err(ErrorData::internal_error(
        "ddl tool not yet implemented",
        None,
    ))
}
