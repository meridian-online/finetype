---
id: doc-004
title: 'Decision: Use rmcp v1.1.0 for FineType MCP server'
type: other
created_date: '2026-03-07 03:01'
---
## Context

FineType needs an MCP server to expose type inference capabilities to AI agents. The server will run as `finetype mcp` over stdio transport, exposing 6 tools and taxonomy resources.

## Options Evaluated

1. **rmcp v1.1.0** — Official Rust MCP SDK. 4.7M downloads, v1.1.0 stable, 3.1k GitHub stars, updated 2026-03-04.
2. **rust-mcp-sdk v0.8.3** — Community SDK. 85k downloads, pre-1.0, "use at your own risk" warning.
3. **Raw JSON-RPC protocol** — No external dependency but significant boilerplate.

## Decision

**Use rmcp v1.1.0.**

## Rationale

- Official SDK maintained by modelcontextprotocol org — best long-term support
- 55x more adoption than nearest alternative
- Proc macro API (#[tool], #[tool_router], #[tool_handler]) eliminates protocol boilerplate
- Auto-generates JSON Schema from schemars::JsonSchema (same crate FineType already uses)
- Validated in PoC: all required capabilities work (tools, resources, stdio transport)
- tokio-native async fits FineType's existing architecture

## Risks

- API changed significantly between v0.3 and v1.1 — online tutorials are outdated. Documented gotchas in discovery/mcp-server/SPIKE.md.
- 141 transitive dependencies, though most overlap with FineType's existing dep tree.

## References

- Spike report: discovery/mcp-server/SPIKE.md
- Task: NNFT-240
