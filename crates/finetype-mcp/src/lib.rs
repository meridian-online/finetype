//! FineType MCP Server
//!
//! Exposes FineType's type inference capabilities via the Model Context Protocol (MCP).
//! Designed to be consumed by AI agents over stdio transport.
//!
//! # Architecture
//!
//! - `FineTypeServer` is the main server struct implementing `ServerHandler`
//! - Tools are defined via rmcp's `#[tool]` macro in the `tools` module
//! - Resources expose the taxonomy at `finetype://taxonomy/...` URIs
//!
//! # Usage
//!
//! ```ignore
//! // From CLI: finetype mcp
//! // Or programmatically:
//! let server = FineTypeServer::new(column_classifier, taxonomy);
//! server.serve_stdio().await?;
//! ```

pub mod json_schema;
pub mod resources;
pub mod tools;

use anyhow::Result;
use finetype_core::Taxonomy;
use finetype_model::ColumnClassifier;
use rmcp::{
    handler::server::router::tool::ToolRouter, handler::server::wrapper::Parameters, model::*,
    service::RequestContext, tool, tool_handler, tool_router, RoleServer, ServerHandler,
    ServiceExt,
};
use std::sync::Arc;
use tokio::sync::RwLock;

/// The FineType MCP server.
///
/// Holds the loaded model and taxonomy, and routes MCP tool/resource requests.
pub struct FineTypeServer {
    tool_router: ToolRouter<Self>,
    /// Column classifier with all models loaded (CharCNN, Sense, Entity, Model2Vec)
    classifier: Arc<RwLock<ColumnClassifier>>,
    /// Taxonomy with compiled validators
    tax: Arc<Taxonomy>,
}

// ColumnClassifier doesn't impl Debug, so we do it manually
impl std::fmt::Debug for FineTypeServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FineTypeServer")
            .field("tool_router", &self.tool_router)
            .field("taxonomy_types", &self.tax.labels().len())
            .finish_non_exhaustive()
    }
}

#[tool_router]
impl FineTypeServer {
    // ─── Tool implementations are in tools/*.rs ─────────────────────────

    #[tool(
        description = "Infer the semantic type of string values. Pass a single value or a list of values with an optional column header for context-aware classification."
    )]
    async fn infer(
        &self,
        Parameters(request): Parameters<tools::infer::InferRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        tools::infer::handle(self, request).await
    }

    #[tool(
        description = "Profile all columns in a CSV or JSON file. Detects semantic types, confidence scores, and domains for each column. Use the validate flag for data quality metrics."
    )]
    async fn profile(
        &self,
        Parameters(request): Parameters<tools::profile::ProfileRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        tools::profile::handle(self, request).await
    }

    #[tool(
        description = "Generate a CREATE TABLE DDL statement from file profiling. Infers column types and maps them to appropriate SQL types."
    )]
    async fn ddl(
        &self,
        Parameters(request): Parameters<tools::ddl::DdlRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        tools::ddl::handle(self, request).await
    }

    #[tool(
        name = "taxonomy",
        description = "Search and browse the FineType type taxonomy. Filter by domain, category, or search query to discover available type definitions."
    )]
    async fn taxonomy_tool(
        &self,
        Parameters(request): Parameters<tools::taxonomy::TaxonomyRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        tools::taxonomy::handle(self, request).await
    }

    #[tool(
        description = "Export JSON Schema for a type key or a CSV file. Pass a type_key for per-type schema, or path/data for table-level schema generation via column profiling."
    )]
    async fn schema(
        &self,
        Parameters(request): Parameters<tools::schema::SchemaRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        tools::schema::handle(self, request).await
    }

    #[tool(
        description = "Validate CSV data against a JSON Schema. Returns per-row and per-column validation results with a quality grade."
    )]
    async fn validate(
        &self,
        Parameters(request): Parameters<tools::validate::ValidateRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        tools::validate::handle(self, request).await
    }

    #[tool(
        description = "Generate synthetic sample data for a given type. Useful for testing, documentation, or understanding what values a type accepts."
    )]
    async fn generate(
        &self,
        Parameters(request): Parameters<tools::generate::GenerateRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        tools::generate::handle(self, request).await
    }
}

#[tool_handler]
impl ServerHandler for FineTypeServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .build(),
        )
        .with_server_info(Implementation::from_build_env())
        // MCP audit follow-up in v0.6.20: card 0006 retired the CLI
        // `schema` verb (type-mode → `taxonomy KEY -o json-schema`,
        // table-mode → `profile -f FILE -o json-schema`). The MCP
        // `schema` tool's type-key branch is RETAINED for v0.6.19 per
        // the visibility-cleanup carve-out (memo
        // 2026-04-27-mcp-surface-audit). The v0.6.20 audit will mirror
        // the CLI fold and remove the asymmetry surfaced here.
        .with_instructions(
            "FineType — semantic type inference engine for tabular data.\n\n\
             Tools: profile (file -> column types), infer (values -> type), ddl (file -> CREATE TABLE), \
             taxonomy (browse types), schema (type or file -> JSON Schema), validate (CSV + schema -> quality report), \
             generate (sample data).\n\n\
             Resources: finetype://taxonomy for browsing type definitions.",
        )
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, ErrorData> {
        resources::list_resources(&self.tax)
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, ErrorData> {
        resources::read_resource(&self.tax, &request.uri)
    }
}

impl FineTypeServer {
    /// Create a new server with a fully-configured column classifier.
    ///
    /// The caller is responsible for building the `ColumnClassifier` with all
    /// desired models wired up (multi-branch, sibling context, semantic hints,
    /// taxonomy, etc.). The taxonomy is extracted from the classifier's state
    /// and also stored separately for resource/tool access.
    pub fn new(column_classifier: ColumnClassifier, taxonomy: Taxonomy) -> Self {
        Self {
            tool_router: Self::tool_router(),
            classifier: Arc::new(RwLock::new(column_classifier)),
            tax: Arc::new(taxonomy),
        }
    }

    /// Get a reference to the column classifier.
    pub fn classifier(&self) -> &Arc<RwLock<ColumnClassifier>> {
        &self.classifier
    }

    /// Get a reference to the taxonomy.
    pub fn taxonomy(&self) -> &Arc<Taxonomy> {
        &self.tax
    }

    /// Start serving over stdio transport.
    pub async fn serve_stdio(self) -> Result<()> {
        let service = self.serve(rmcp::transport::stdio()).await?;
        service.waiting().await?;
        Ok(())
    }
}
