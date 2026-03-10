---
id: NNFT-268
title: Implement sibling-context attention module and integrate into Sense pipeline
status: Done
assignee:
  - '@nightingale'
created_date: '2026-03-09 00:56'
updated_date: '2026-03-10 03:10'
labels:
  - architecture
  - model
  - context
milestone: m-13
dependencies: []
references:
  - discovery/sense-architecture-challenge/SPIKE_B_SIBLING_CONTEXT.md
  - discovery/sense-architecture-challenge/ARCHITECTURE_EVOLUTION.md
  - discovery/sense-architecture-challenge/sibling_context_spike.rs
  - crates/finetype-model/src/sense.rs
  - crates/finetype-model/src/column.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Phase 3 of the architecture evolution (Spike B findings). Requires multi-column training data (GitTables or similar).

Add a 2-layer pre-norm transformer self-attention module over Model2Vec column embeddings. Spike B confirmed:

- No Candle API blockers — follows same pattern as SenseClassifier cross-attention
- 396,800 params (1.51 MB f32, 0.76 MB f16)
- Latency: 112μs (1 col) to 1.3ms (20 cols) — negligible vs full pipeline
- Graceful degradation: N=1 reduces to self-attention (identity with residual)
- Integration point: between Model2Vec encoding and Sense classification

Training requires multi-column tables with known types. GitTables (1.7M tables) is the natural source. Need to build a data pipeline: sample tables → extract columns → encode with Model2Vec → train attention + downstream classifier jointly.

This is the most impactful change — addresses 3/7 bare-name ambiguity errors that are currently unresolvable.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Implement SiblingContextAttention module in new sibling_context.rs (2-layer, 4-head, 128-dim)
- [x] #2 Integrate into column.rs pipeline between Model2Vec encoding and Sense classification
- [x] #3 Single-column mode produces identical results to current pipeline (no regression)
- [x] #4 Multi-column mode passes context-enriched embeddings to Sense
- [x] #5 Profile eval in single-column mode maintains 179/186 (zero regression)
- [x] #6 Latency overhead measured and documented (<5ms for 20 columns)
- [x] #7 Model weights serializable/deserializable via safetensors
- [x] #8 Training data pipeline for multi-column tables designed (GitTables or similar)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Step 1 complete: SiblingContextAttention module in sibling_context.rs — 2-layer, 4-head, 128-dim with safetensors load/save and 5 passing unit tests (shape preservation, param count=396800, safetensors round-trip, single-column degradation, config defaults).

Step 2 complete: Added classify_with_enriched_header() to SenseClassifier — accepts pre-computed context-enriched header tensor from sibling-context module.

Step 3 complete: Integrated into ColumnClassifier — sibling_context field, set_sibling_context()/has_sibling_context(), classify_columns_with_context() multi-column entry point. classify_sense_sharpen refactored into classify_sense_sharpen_inner with optional enriched header.

Step 4 complete: Module declaration + re-export in lib.rs.

Step 5 complete: CLI wiring — wire_sibling_context() loads from models/sibling-context/ (silent when absent), called at all 4 wire_sense sites. Profile command has multi-column path when sibling context is available.

CI passes: fmt + clippy (0 warnings in finetype-model) + test (298 pass) + taxonomy check (250/250).

AC5 verified: Profile eval 180/186 (96.8% label, 98.4% domain) — identical to baseline, zero regression.

AC6 verified: Release-mode latency — N=1: 108μs, N=5: 489μs, N=10: 845μs, N=20: 1.4ms. Well under 5ms budget.

AC8 documented: Training data pipeline design in plan (GitTables 50K tables, JSONL format, frozen Sense weights, batch padding with attention mask). Follow-up tasks: prepare_sibling_data.sh, sibling_context_train.rs, train_sibling.rs binary.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Implemented the sibling-context self-attention module (NNFT-268), enabling cross-column context for type inference. This is the architectural foundation for resolving bare-name ambiguity errors (3/7 remaining misclassifications) that are structurally unresolvable without seeing sibling columns.

## Changes

**New file: `crates/finetype-model/src/sibling_context.rs`**
- `SiblingContextAttention` — 2-layer pre-norm transformer self-attention (4 heads, 128-dim, 396,800 params)
- Safetensors load/save with JSON config, matching existing model patterns
- Forward: `[N_cols, D] → [N_cols, D]` context-enriched embeddings
- 6 unit tests: shape preservation, param count, safetensors round-trip, single-column degradation, config, latency benchmark

**Modified: `crates/finetype-model/src/sense.rs`**
- Added `classify_with_enriched_header()` — accepts pre-computed context-enriched header tensor instead of encoding from scratch

**Modified: `crates/finetype-model/src/column.rs`**
- Added `sibling_context: Option<SiblingContextAttention>` field + setter/getter
- Added `classify_columns_with_context()` — multi-column entry point that encodes all headers, runs sibling attention, then Sense→Sharpen with enriched headers
- Refactored `classify_sense_sharpen` into `classify_sense_sharpen_inner` with optional enriched header to avoid duplicating ~500 lines of post-Sense logic

**Modified: `crates/finetype-cli/src/main.rs`**
- `wire_sibling_context()` — loads from `models/sibling-context/` (silent when absent)
- Called at all 4 `wire_sense` sites (infer, batch, load, profile)
- Profile command: multi-column batch path when sibling context available, per-column fallback otherwise

## Key design decisions
- **No model = no behavior change.** `load_sibling_context()` returns None, all paths fall back to existing per-column classification.
- **Inner method pattern** avoids duplicating post-Sense disambiguation logic (~500 lines). Single optional parameter switches between normal and enriched paths.
- DuckDB extension, MCP server, single-column callers: completely unaffected.

## Verification
- `cargo test`: 298 pass (5 new sibling_context tests + all existing)
- `cargo clippy -p finetype-model`: 0 warnings
- `make ci`: fmt + clippy + test + check all pass
- Profile eval: 180/186 (96.8% label, 98.4% domain) — identical to baseline
- Release-mode latency: N=1: 108μs, N=5: 489μs, N=10: 845μs, N=20: 1.4ms",
<parameter name="definitionOfDoneCheck">[1, 2, 3]
<!-- SECTION:FINAL_SUMMARY:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [x] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [x] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [ ] #4 Decision record created if plan involved choosing between approaches
- [ ] #5 Daily memory log updated with session outcomes
- [ ] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
