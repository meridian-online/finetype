# Spike: Rust MCP SDK Evaluation for `finetype mcp`

**Task:** NNFT-240
**Date:** 2026-03-07
**Time budget:** ~1 hour
**Status:** Complete — recommend `rmcp` v1.1.0

## Question

Which Rust MCP library should we use (or should we roll our own) for a stdio-transport MCP server exposing FineType's inference capabilities?

## Candidates Evaluated

### 1. `rmcp` (Official Rust MCP SDK) — RECOMMENDED

| Attribute | Value |
|---|---|
| **Crate** | [rmcp](https://crates.io/crates/rmcp) |
| **Version** | 1.1.0 (stable) |
| **Downloads** | 4.7M total / 3.0M recent |
| **GitHub** | [modelcontextprotocol/rust-sdk](https://github.com/modelcontextprotocol/rust-sdk) — 3.1k stars |
| **Last updated** | 2026-03-04 (3 days ago) |
| **MCP spec** | 2024-11-05 (default), latest supported |
| **Transport** | stdio, SSE, Streamable HTTP |
| **Dependencies** | tokio, serde, schemars, chrono, futures |
| **Transitive deps** | ~141 crates |
| **API style** | Proc macros (`#[tool]`, `#[tool_router]`, `#[tool_handler]`) |

**Key strengths:**
- **Official SDK** — maintained by `modelcontextprotocol` org
- **Macro-driven API** — `#[tool]` auto-generates JSON Schema from `schemars::JsonSchema` derive
- **Clean resource API** — `list_resources` / `read_resource` trait methods with builder pattern
- **Mature** — 39 releases, v1.1.0 stable, active development
- **schemars v1** — uses JSON Schema Draft 2020-12, same as FineType's existing validation
- **tokio-native** — fits FineType's async model

**Concerns:**
- API changed significantly between v0.3→v1.1 (online tutorials are outdated)
- `Annotated<T>` wrapper pattern adds boilerplate for resources
- Heavy transitive dependency count (141), though most overlap with FineType's existing deps (tokio, serde, schemars)

### 2. `rust-mcp-sdk` (Community)

| Attribute | Value |
|---|---|
| **Crate** | [rust-mcp-sdk](https://crates.io/crates/rust-mcp-sdk) |
| **Version** | 0.8.3 (pre-1.0) |
| **Downloads** | 85k total / 34k recent (55x less than rmcp) |
| **GitHub** | 156 stars |
| **Last updated** | 2026-02-01 (1 month ago) |
| **MCP spec** | 2025-11-25 |
| **Status** | "Project is currently under development and should be used at your own risk" |

**Strengths:** Newer spec version, HTTP+SSE support, OAuth providers.
**Weaknesses:** Pre-1.0, ~55x fewer downloads, warning about stability, separate maintainers. No PoC built (clear winner determined without it).

### 3. Raw protocol (JSON-RPC over stdio)

Not evaluated in depth. The rmcp crate's macro system and trait-based handler pattern eliminate enough boilerplate that rolling our own protocol layer is not justified. We'd be reimplementing what rmcp already handles: message framing, request routing, capability negotiation, error codes.

## Proof of Concept Results

Built a minimal MCP server with `rmcp` v1.1.0 — 1 tool + 2 resources over stdio.

**PoC location:** `/tmp/finetype-mcp-spike/` (ephemeral)

### Verified capabilities

| Endpoint | Status | Notes |
|---|---|---|
| `initialize` | PASS | Protocol handshake, capability advertisement (tools + resources) |
| `tools/list` | PASS | Auto-generated JSON Schema from `schemars::JsonSchema` derive |
| `tools/call` (infer) | PASS | `user@example.com` → `identity.person.email` with JSON result |
| `resources/list` | PASS | 2 resources with URI, name, description, mime_type |
| `resources/read` | PASS | Returns JSON content for `finetype://taxonomy` URI |

### API patterns validated

**Tool definition** — clean macro-based pattern:
```rust
#[tool(description = "Infer the semantic type of a string value")]
async fn infer(&self, Parameters(req): Parameters<InferRequest>) -> Result<CallToolResult, ErrorData> {
    Ok(CallToolResult::success(vec![Content::text(json_string)]))
}
```

**Resource definition** — trait methods on `ServerHandler`:
```rust
async fn list_resources(&self, ...) -> Result<ListResourcesResult, ErrorData> {
    Ok(ListResourcesResult { meta: None, next_cursor: None, resources: vec![...] })
}
async fn read_resource(&self, request: ReadResourceRequestParams, ...) -> Result<ReadResourceResult, ErrorData> {
    Ok(ReadResourceResult::new(vec![ResourceContents::text(content, uri)]))
}
```

**Server startup** — 3 lines:
```rust
let service = FineTypeSpike::new().serve(rmcp::transport::stdio()).await?;
service.waiting().await?;
```

### Build metrics

| Metric | Value |
|---|---|
| Build time (cold) | ~55s |
| Build time (incremental) | ~5s |
| Debug binary size | 40MB |
| Direct dependencies | 5 (rmcp, tokio, serde, serde_json, schemars) |
| Transitive dependencies | 141 |

## Recommendation

**Use `rmcp` v1.1.0.** The decision is clear:

1. **Official SDK** with active maintenance and 55x more adoption than alternatives
2. **Clean API** — proc macros eliminate protocol boilerplate, JSON Schema auto-generated
3. **Dependency overlap** — tokio, serde, schemars already in FineType's dependency tree
4. **Validated** — all FineType-relevant MCP capabilities work (tools, resources, stdio)

### Integration plan

1. Add `finetype-mcp` library crate to workspace
2. Depend on `finetype-core` (taxonomy, generators, validation) and `finetype-model` (inference)
3. Add `finetype mcp` subcommand to `finetype-cli`
4. Feature-gate with `features = ["mcp"]` to keep binary size optional

### Cargo.toml for the new crate

```toml
[dependencies]
rmcp = { version = "1.1", features = ["server", "transport-io"] }
schemars = "1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
finetype-core = { path = "../finetype-core" }
finetype-model = { path = "../finetype-model" }
```

### Key API gotcha for implementation

The v1.1.0 API differs from online tutorials (which target v0.3–0.16):
- `Resource = Annotated<RawResource>` — use `Annotated::new(RawResource::new(...), None)`
- `ServerInfo::new(capabilities)` — not `ServerInfo::new(name, version)`
- `ReadResourceResult::new(contents)` — non-exhaustive, use constructor
- `ListResourcesResult` has `meta` field from `paginated_result!` macro
- `ErrorData::invalid_params(msg, data)` takes 2 args
- `Parameters` import: `rmcp::handler::server::wrapper::Parameters`
