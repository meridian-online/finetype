---
id: m-11
title: "m-11: DuckDB Extension Redesign"
---

## Description

Redesign the DuckDB extension from scalar classifier to higher-level analyst functions.

**Current state:** Flat CharCNN with scalar `finetype()`, `finetype_detail()`, `finetype_cast()`, `finetype_unpack()`, `finetype_version()`. No Sense pipeline, no column-level intelligence, no validation.

**Vision (from architectural review, section 5):**
The extension evolves toward analyst-facing functions closer to the Noon pillars:
- `read_file()` — ingest CSV/JSON/XML with type inference baked in
- `validate_table()` — check data against a spec
- Column-level Sense pipeline adoption (where per-column latency is acceptable)

**Design constraints (established in Sense & Sharpen pivot):**
- Sense model weights embeddable via `include_bytes!` (same as CharCNN)
- `ValueClassifier` trait or new `ColumnSensor` trait usable from both CLI and extension contexts
- Column sampling must work on DuckDB vectors, not just Rust `Vec<String>`

**Scope:**
- Adopt Sense→Sharpen pipeline for column-level functions
- Higher-level analyst functions beyond scalar `finetype()`
- Profile and validate as table functions (not just scalar)
- JSON profiling support (leverage m-9 json_reader infrastructure)

**Non-scope:** This is a FineType extension redesign, not a standalone DuckDB product. Extension stays read-only — it classifies and reports, it doesn't transform data.

**Dependencies:** Benefits from all Phase 1 (Locale Foundation, Format Coverage) and Phase 2 (Validate & Report, JSON Profiling) improvements. Can begin design work independently.

**Reference:** discovery/architectural-pivot/REVIEW.md section 5
