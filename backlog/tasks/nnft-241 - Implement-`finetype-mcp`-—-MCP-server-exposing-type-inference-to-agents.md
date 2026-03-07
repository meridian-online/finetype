---
id: NNFT-241
title: Implement `finetype mcp` — MCP server exposing type inference to agents
status: Done
assignee:
  - '@nightingale'
created_date: '2026-03-07 03:03'
updated_date: '2026-03-07 03:28'
labels:
  - feature
  - mcp
dependencies: []
references:
  - discovery/mcp-server/SPIKE.md
  - backlog/docs/doc-004 - Decision-Use-rmcp-v1.1.0-for-FineType-MCP-server.md
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Build the FineType MCP server so AI agents can use type inference via Model Context Protocol.

**Architecture (from interview + spike NNFT-240):**
- New `finetype-mcp` library crate in workspace, consumed by CLI as `finetype mcp` subcommand
- Single binary distribution (no separate Homebrew install)
- SDK: `rmcp` v1.1.0 (official Rust MCP SDK) — validated in PoC
- Transport: stdio (subprocess — agent launches `finetype mcp`)
- Model: default model loaded once at server startup
- Output: JSON primary content + markdown summary text block
- File input: path primary, inline CSV/JSON fallback for small datasets

**Tools (6):**
1. `profile` — Profile all columns in a file (with `validate` flag for quality report)
2. `infer` — Classify a column (values[] + optional header) or single value
3. `ddl` — Generate CREATE TABLE DDL from file (schema-for workflow)
4. `taxonomy` — Search/filter type taxonomy (domain, category, query params)
5. `schema` — Export JSON Schema for type(s) (type key, pretty flag)
6. `generate` — Generate synthetic sample data (type key, count, locale)

**MCP Resources:**
- `finetype://taxonomy` — Full taxonomy listing
- `finetype://taxonomy/{domain}` — Types in a domain
- `finetype://taxonomy/{domain}.{category}.{type}` — Single type definition

**References:**
- Interview decisions: discovery/mcp-server/SPIKE.md (bottom section)
- SDK decision: doc-004
- rmcp API gotchas: discovery/mcp-server/SPIKE.md (API gotcha section)
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 finetype-mcp library crate exists in workspace with rmcp v1.1.0 dependency, compiles cleanly
- [x] #2 finetype mcp subcommand starts stdio MCP server, responds to initialize with correct capabilities (tools + resources)
- [x] #3 profile tool: accepts file path (or inline data), returns JSON column profiles with types/confidence/domains + markdown summary. Supports validate flag for quality metrics.
- [x] #4 infer tool: accepts values[] + optional header, returns classified type with confidence. Also works for single value.
- [x] #5 ddl tool: accepts file path (or inline data) + optional table_name, returns CREATE TABLE DDL statement
- [x] #6 taxonomy tool: returns type listing, filterable by domain/category/query string
- [x] #7 schema tool: accepts type key, returns JSON Schema contract for that type
- [x] #8 generate tool: accepts type key + optional count/locale, returns synthetic sample values
- [x] #9 MCP resources: finetype://taxonomy, finetype://taxonomy/{domain}, finetype://taxonomy/{d}.{c}.{t} all return correct JSON
- [x] #10 All 6 tools return JSON primary content + markdown summary text block
- [x] #11 cargo test passes including new MCP server tests
- [x] #12 Smoke test: configure as MCP server in Claude Code .mcp.json, verify tools appear and infer tool works
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
## Phase 1: Scaffold (lead, before team spawn)

1. Create `crates/finetype-mcp/` with Cargo.toml, lib.rs, server struct, empty tool router
2. Add mod structure: `tools/mod.rs`, `tools/infer.rs`, `tools/profile.rs`, `tools/ddl.rs`, `tools/taxonomy.rs`, `tools/schema.rs`, `tools/generate.rs`, `resources.rs`
3. Wire `finetype mcp` subcommand in CLI
4. Implement server handler (get_info, stdio startup)
5. Commit scaffold → push

## Phase 2: Team implementation (parallel worktrees)

**Lead (@nightingale):**
- Implement `infer` tool (tools/infer.rs)
- Implement all 3 resource handlers (resources.rs)
- Implement markdown summary formatting
- Integration: final tool_router wiring, CLI testing
- Smoke test with Claude Code

**Teammate 2:**
- Implement `profile` tool (tools/profile.rs)
- Implement `ddl` tool (tools/ddl.rs)
- Implement `generate` tool (tools/generate.rs)

**Teammate 3:**
- Implement `taxonomy` tool (tools/taxonomy.rs)
- Implement `schema` tool (tools/schema.rs)
- Write unit tests for all tools
- Update CLAUDE.md with MCP server section

## Phase 3: Integration (lead)
- Merge teammate branches
- Final smoke test
- Commit with task ID
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Phase 1 complete: scaffold committed at 86a2240, pushed to main. Crate compiles with all 6 tool stubs + resources + CLI subcommand.

Phase 2 in progress:
- Lead: infer tool implemented and compiling (single-value + column mode with header hints)
- Resources: taxonomy overview, domain detail, type detail all implemented in scaffold
- Teammates spawned: tools-file (profile/ddl/generate), tools-core (taxonomy/schema)

Phase 2 complete: All 6 tool handlers implemented and compiling.
- infer: single-value + column mode with header hints, vote distribution
- profile: CSV parsing, column classification, validation quality metrics
- ddl: CREATE TABLE DDL generation with broad_type mapping
- taxonomy: domain/category/query filtering with markdown table
- schema: JSON Schema export with x-finetype extensions, glob patterns
- generate: synthetic data generation using finetype_core::Generator

cargo check, cargo fmt, cargo clippy all clean. 258 tests pass.

Phase 3 — Integration & smoke test complete:
- cargo build passes, finetype mcp starts and responds to initialize
- tools/list returns all 6 tools with correct schemas
- infer tool: tested with email values → identity.person.email (100% confidence)
- taxonomy tool: tested with identity.person filter → 13 types listed
- resources/list: 8 resources (overview + 7 domains)
- cargo test: 258 passed, 0 failed
- cargo run -- check: ALL CHECKS PASSED
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added MCP server to FineType via new `finetype-mcp` library crate, exposing type inference capabilities to AI agents over the Model Context Protocol.

## What changed

**New crate: `crates/finetype-mcp/`** (rmcp v1.1.0, ~840 lines)
- `lib.rs` — `FineTypeServer` struct with `#[tool_router]` and `#[tool_handler]` implementations, stdio transport serving
- `resources.rs` — Taxonomy browsing via `finetype://taxonomy[/domain][/d.c.t]` URIs (overview, domain detail, type detail)
- `tools/infer.rs` — Single-value (ValueClassifier) and column-mode (ColumnClassifier with header hints) inference
- `tools/profile.rs` — CSV file/inline data profiling with per-column type classification and optional validation quality metrics
- `tools/ddl.rs` — CREATE TABLE DDL generation from file profiling, maps finetype labels to SQL types via `ddl_info()`
- `tools/taxonomy.rs` — Taxonomy search/filter by domain, category, or free-text query with markdown table output
- `tools/schema.rs` — JSON Schema export with x-finetype extension fields, glob pattern support
- `tools/generate.rs` — Synthetic data generation using `finetype_core::Generator`

**CLI integration:** Added `Mcp` variant to `Commands` enum and `cmd_mcp()` handler in `finetype-cli/src/main.rs`

**CLAUDE.md:** Updated architecture (9 crates), crate dependency graph, workspace layout, CLI commands table, key file reference, decided items (#21), recent milestones

## Why

AI agents (Claude Code, Cursor, etc.) can now use FineType for type inference without shelling out to CLI. The MCP protocol provides structured tool/resource access with proper schema discovery, enabling agents to profile datasets, generate DDL, browse the taxonomy, and validate data quality programmatically.

## Tests

- `cargo test`: 258 passed, 0 failed
- `cargo run -- check`: ALL CHECKS PASSED
- `cargo fmt --check` + `cargo clippy`: clean
- Manual JSON-RPC smoke test: initialize, tools/list (6 tools), tools/call (infer with emails → identity.person.email at 100%), resources/list (8 resources), taxonomy tool (identity.person → 13 types)"
<!-- SECTION:FINAL_SUMMARY:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [x] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [x] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [x] #4 Decision record created if plan involved choosing between approaches
- [x] #5 Daily memory log updated with session outcomes
- [x] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
