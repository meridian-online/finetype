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
//! let server = FineTypeServer::new(classifier, taxonomy, semantic);
//! server.serve_stdio().await?;
//! ```

pub mod resources;
pub mod tools;

use anyhow::Result;
use finetype_core::Taxonomy;
use finetype_model::{
    CharClassifier, ColumnClassifier, ColumnConfig, SemanticHintClassifier, ValueClassifier,
};
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
        description = "Export the JSON Schema validation contract for a specific type. Returns the schema that defines valid values for the given type key."
    )]
    async fn schema(
        &self,
        Parameters(request): Parameters<tools::schema::SchemaRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        tools::schema::handle(self, request).await
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
        .with_instructions(
            "FineType — semantic type inference engine for tabular data.\n\n\
             Tools: profile (file -> column types), infer (values -> type), ddl (file -> CREATE TABLE), \
             taxonomy (browse types), schema (JSON Schema export), generate (sample data).\n\n\
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
    /// Create a new server with loaded models.
    ///
    /// All models are loaded once at startup and shared across requests.
    pub fn new(
        char_classifier: CharClassifier,
        taxonomy: Taxonomy,
        semantic: Option<SemanticHintClassifier>,
    ) -> Self {
        let config = ColumnConfig {
            sample_size: 100,
            ..Default::default()
        };

        let mut column_classifier = if let Some(semantic) = semantic {
            ColumnClassifier::with_semantic_hint(
                Box::new(char_classifier) as Box<dyn ValueClassifier>,
                config,
                semantic,
            )
        } else {
            ColumnClassifier::new(
                Box::new(char_classifier) as Box<dyn ValueClassifier>,
                config,
            )
        };

        // Set taxonomy for validation-based disambiguation
        let mut tax = taxonomy;
        tax.compile_validators();
        tax.compile_locale_validators();
        column_classifier.set_taxonomy(tax.clone());

        Self {
            tool_router: Self::tool_router(),
            classifier: Arc::new(RwLock::new(column_classifier)),
            tax: Arc::new(tax),
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
