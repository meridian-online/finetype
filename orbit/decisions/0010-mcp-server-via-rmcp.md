---
status: accepted
date-created: 2026-03-07
date-modified: 2026-03-11
---
# 0010. MCP server via rmcp — official Rust MCP SDK

## Context and Problem Statement

FineType needed to expose its type inference capabilities to AI agents via the Model Context Protocol (MCP). This required choosing a Rust MCP library or building a custom implementation. The server would run as a subprocess (`finetype mcp`) over stdio transport.

A spike (NNFT-240) evaluated available Rust MCP SDKs.

## Considered Options

- **rmcp (Official Rust MCP SDK)** — v1.1.0, maintained by the `modelcontextprotocol` org. Macro-driven API (`#[tool]`), schemars v1 (JSON Schema Draft 2020-12), tokio-native. 3.1K GitHub stars, 4.7M crate downloads.
- **mcp-server (community)** — Less mature, fewer downloads, not officially maintained.
- **Custom implementation** — Roll our own stdio JSON-RPC handler. Maximum control but significant maintenance burden.

## Decision Outcome

Chosen option: **rmcp v1.1.0**, because it is the official SDK maintained by the MCP organization, uses the same JSON Schema draft (2020-12) as FineType's existing validation infrastructure, and provides a clean macro-driven API that auto-generates tool schemas from `schemars::JsonSchema` derives.

The result: `finetype mcp` subcommand with 6 tools (infer, profile, ddl, taxonomy, schema, generate) and taxonomy resources. JSON + markdown dual output. New `finetype-mcp` library crate.

### Consequences

- Good, because the official SDK tracks MCP spec changes — reduced maintenance burden
- Good, because `schemars` v1 alignment means tool parameter schemas use the same patterns as FineType's type validation
- Good, because the macro-driven API minimizes boilerplate — tool implementations are plain async functions
- Bad, because rmcp's API changed significantly between v0.3 and v1.1 — online tutorials are largely outdated
- Bad, because 141 transitive dependencies, though most overlap with FineType's existing dependency tree (tokio, serde, schemars)
- Neutral, because stdio transport is the simplest option — SSE/HTTP transport available if needed later
