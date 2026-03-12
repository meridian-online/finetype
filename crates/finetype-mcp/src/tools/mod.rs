//! MCP tool implementations for FineType.
//!
//! Each tool is in its own module with:
//! - A request struct deriving `schemars::JsonSchema` for auto-generated input schema
//! - A `handle()` function containing the tool logic
//! - Response formatted as JSON primary + markdown summary

pub mod ddl;
pub mod generate;
pub mod infer;
pub mod profile;
pub mod schema;
pub mod taxonomy;
pub mod validate;

use rmcp::model::{CallToolResult, Content};

/// Helper to create a successful tool result with JSON + markdown summary.
#[allow(dead_code)]
pub(crate) fn success_with_summary(json: &serde_json::Value, summary: &str) -> CallToolResult {
    CallToolResult::success(vec![
        Content::text(serde_json::to_string_pretty(json).unwrap_or_default()),
        Content::text(summary.to_string()),
    ])
}
