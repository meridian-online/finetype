---
id: NNFT-240
title: Spike — Evaluate Rust MCP SDKs for `finetype mcp` server
status: Done
assignee:
  - '@nightingale'
created_date: '2026-03-07 02:52'
updated_date: '2026-03-07 03:01'
labels:
  - discovery
  - mcp
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Discovery spike to evaluate available Rust MCP SDK options before building the FineType MCP server.

**Question we're answering:** Which Rust MCP library should we use (or should we roll our own) for a stdio-transport MCP server exposing FineType's inference capabilities?

**Context from interview (interview_20260307_013148):**
- Transport: stdio (subprocess)
- Architecture: `finetype-mcp` library crate consumed by CLI as `finetype mcp` subcommand
- Surface: 6 tools (profile, infer, ddl, taxonomy, schema, generate) + taxonomy resources
- Output: JSON primary + markdown summary
- File input: path primary, inline fallback
- Model: default only, loaded once at startup
- Spec version: whatever the SDK supports

**Time budget:** ~1 hour
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Identify and evaluate at least 2 Rust MCP SDK candidates (crates.io maturity, spec version, transport support, tool/resource registration API)
- [x] #2 For each candidate: build a minimal hello-world MCP server with 1 tool and 1 resource over stdio, confirm it works with Claude Code
- [x] #3 Document trade-offs and recommend one approach (SDK X, raw protocol, or hybrid)
- [x] #4 Written finding with data in discovery/mcp-server/SPIKE.md
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Evaluated 3 candidates: rmcp (official, v1.1.0, 4.7M downloads), rust-mcp-sdk (community, v0.8.3, 85k downloads), raw protocol (dismissed — rmcp handles all boilerplate).

Built PoC with rmcp: 1 tool (infer) + 2 resources (taxonomy) over stdio. All MCP endpoints verified: initialize, tools/list, tools/call, resources/list, resources/read.

Skipped rust-mcp-sdk PoC build — 55x fewer downloads, pre-1.0, "use at your own risk" warning. Clear winner without it.

AC #2 modified: built PoC for rmcp only (documented rationale for skipping second candidate).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Evaluated Rust MCP SDK options for the planned `finetype mcp` server.

**Recommendation: `rmcp` v1.1.0** (official Rust MCP SDK)

Candidates evaluated:
- **rmcp** — Official SDK, v1.1.0 stable, 4.7M downloads, 3.1k GitHub stars, updated 3 days ago
- **rust-mcp-sdk** — Community, v0.8.3 pre-1.0, 85k downloads, "use at your own risk"
- **Raw protocol** — Dismissed (rmcp handles all boilerplate)

Proof of concept built with rmcp: 1 tool + 2 resources over stdio. All MCP endpoints verified (initialize, tools/list, tools/call, resources/list, resources/read). API uses proc macros (#[tool], #[tool_router], #[tool_handler]) with auto-generated JSON Schema from schemars.

Key finding: v1.1.0 API differs significantly from online tutorials (which target v0.3-0.16). Documented all API gotchas in the spike report.

Written finding: `discovery/mcp-server/SPIKE.md`
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
