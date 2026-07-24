//! FineType MCP — library types and the (deprecated) MCP server role.
//!
//! Exposes FineType's type inference capabilities via the Model Context Protocol (MCP).
//!
//! # Deprecation — the MCP *server role* has moved
//!
//! Running an MCP server from this crate is **deprecated**. The MCP server role is
//! superseded by arcform's `arc mcp` entrypoint — point MCP clients there instead.
//!
//! This crate is **retained for its library types**: the datapackage / JSON-schema
//! emitters (`datapackage`, `json_schema`), taxonomy resources, and tool request/
//! response types remain supported and are unaffected. Only the server lifecycle
//! (`FineTypeServer::new` to build a server and `FineTypeServer::serve_stdio` to run
//! one) is deprecated.
//!
//! # Architecture
//!
//! - `FineTypeServer` is the main server struct implementing `ServerHandler`
//! - Tools are defined via rmcp's `#[tool]` macro in the `tools` module
//! - Resources expose the taxonomy at `finetype://taxonomy/...` URIs
//!
//! # Usage
//!
//! To run an MCP server, use arcform's `arc mcp`. The former server lifecycle in
//! this crate is deprecated:
//!
//! ```ignore
//! // Deprecated — prefer `arc mcp` (arcform):
//! let server = FineTypeServer::new(column_classifier, taxonomy);
//! server.serve_stdio().await?;
//! ```

pub mod datapackage;
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
        name = "taxonomy",
        description = "Search and browse the FineType type taxonomy. Filter by domain, category, key, or search query to discover type definitions. Set format: \"json-schema\" with a key/glob to export per-type JSON Schema (Draft 2020-12)."
    )]
    async fn taxonomy_tool(
        &self,
        Parameters(request): Parameters<tools::taxonomy::TaxonomyRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        tools::taxonomy::handle(self, request).await
    }

    #[tool(
        description = "Validate CSV data against a JSON Schema. Returns per-row and per-column validation results with a quality grade. When the corresponding CLI is run with --db/--table, valid rows are materialised into a typed DuckDB table (per-column transforms applied via TRY-wrapped projection) and rejects land in the finetype_reject_errors sidecar — error_type='SEMANTIC_TYPE' for engine-level rejects, error_type='TRANSFORM_FAILED' (constraint_failed='transform') for cells that passed validation but failed the typed cast. Staging-NULL → typed-NULL is not a transform failure."
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
        // CLI↔MCP surface parity (choice 0101): the MCP tool set mirrors the
        // CLI's capability surface. The former `schema` tool folded into
        // `taxonomy` (type-mode, format: "json-schema") + `profile`
        // (table-mode) per choice 0070; the `ddl` tool was retired
        // (parity-down — the CLI has no DDL command; use validate --db/--table).
        .with_instructions(
            "FineType — semantic type inference engine for tabular data.\n\n\
             Tools: profile (file -> column types, or table-level JSON Schema), \
             infer (values -> type), taxonomy (browse types, or per-type JSON Schema), \
             validate (CSV + schema -> quality report), generate (sample data).\n\n\
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
    ///
    /// **Deprecated** — part of the MCP server role, which has moved to arcform's
    /// `arc mcp`. This crate is retained for its library types, not for running a
    /// server.
    #[deprecated(
        note = "the MCP server role is superseded by `arc mcp` in arcform; this crate \
                is retained for its library types (datapackage / JSON-schema helpers), \
                not for running an MCP server"
    )]
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
    ///
    /// **Deprecated** — this is the MCP server run entry point, which has moved to
    /// arcform's `arc mcp`. This crate is retained for its library types, not for
    /// running a server.
    #[deprecated(
        note = "the MCP server role is superseded by `arc mcp` in arcform; this crate \
                is retained for its library types (datapackage / JSON-schema helpers), \
                not for running an MCP server"
    )]
    pub async fn serve_stdio(self) -> Result<()> {
        let service = self.serve(rmcp::transport::stdio()).await?;
        service.waiting().await?;
        Ok(())
    }
}

#[cfg(test)]
mod parity_tests {
    use super::*;

    /// CLI↔MCP surface-parity manifest (choice 0101).
    ///
    /// Every registered MCP tool MUST appear here, each annotated with the CLI
    /// capability it maps to. The rule is `{MCP tools} ⊆ {CLI capabilities,
    /// visible ∪ intentionally-hidden}` — the CLI is the canonical surface, and
    /// MCP must not exceed it. The reverse is allowed: maintainer-internal
    /// hidden CLI subcommands (`check`, `train-multi-branch`, …) need no MCP
    /// tool.
    ///
    /// To ADD an MCP tool, add it here with its CLI mapping — which forces the
    /// parity decision (does the CLI carry this capability?) to be explicit, not
    /// accidental. The `parity_guard` test fails until this manifest matches the
    /// server's registered tool set exactly.
    const PARITY_MANIFEST: &[(&str, &str)] = &[
        ("infer", "mirrors CLI `infer`"),
        ("profile", "mirrors CLI `profile` (incl. -o json-schema table-mode)"),
        (
            "taxonomy",
            "mirrors CLI `taxonomy` (incl. KEY -o json-schema type-mode; absorbed the retired `schema` tool, choice 0070/0101)",
        ),
        ("validate", "mirrors CLI `validate`"),
        (
            "generate",
            "intentional MCP-public surface of the HIDDEN CLI `generate` subcommand (card 0010)",
        ),
    ];

    #[test]
    fn parity_guard_mcp_tools_equal_manifest() {
        let router = FineTypeServer::tool_router();
        let mut registered: Vec<String> = router
            .list_all()
            .into_iter()
            .map(|t| t.name.into_owned())
            .collect();
        registered.sort();

        let mut expected: Vec<String> = PARITY_MANIFEST
            .iter()
            .map(|(name, _)| name.to_string())
            .collect();
        expected.sort();

        assert_eq!(
            registered, expected,
            "MCP tool set drifted from the CLI↔MCP parity manifest (choice 0101).\n\
             registered = {registered:?}\n\
             manifest   = {expected:?}\n\
             A new/removed/renamed MCP tool must be reflected in PARITY_MANIFEST \
             with its CLI mapping — which forces the parity decision to be explicit."
        );
    }

    /// The two retired tools must not reappear (choice 0101): `schema` folded
    /// into `taxonomy`/`profile`; `ddl` retired parity-down.
    #[test]
    fn retired_tools_absent() {
        let router = FineTypeServer::tool_router();
        let names: Vec<String> = router
            .list_all()
            .into_iter()
            .map(|t| t.name.into_owned())
            .collect();
        for retired in ["schema", "ddl"] {
            assert!(
                !names.contains(&retired.to_string()),
                "`{retired}` was retired by choice 0101 but is still registered"
            );
        }
    }
}
