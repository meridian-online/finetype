# FineType Roadmap: Milestones

**Date:** 2026-03-04
**Context:** Post v0.5.2 release. 163 types across 7 domains. Sense+Sharpen pipeline, CharCNN v10, pure Rust stack.

## Vision

FineType evolves in two phases:

1. **Inference depth** (Phase 1) — Make type detection genuinely locale-aware and cover the long tail of real-world formats. FineType becomes the definitive answer to "what type is this column?"
2. **Data quality pipeline** (Phase 2) — Layer validation, reporting, and schema export on top of inference. FineType becomes a data onboarding tool: detect → validate → report → export.

Milestones are theme-based, not time-boxed. Ship when the theme is complete.

## Phase 1: Inference Depth

### Milestone 1 — Locale Foundation

**Goal:** Make locale a structural part of FineType, not just a metadata field.

**Current state:** 14 postal locales, 15 phone locales, 6 month/day locales. Post-hoc locale detection via `validation_by_locale`. CLDR data downloaded but not yet integrated as upstream source.

**Scope:**
- CLDR as the single upstream data source for all locale-specific types
- Expand postal_code validation to 50+ locales
- Expand phone_number validation to 40+ locales
- Locale-aware generators: CLDR pattern permutation for training data
- Locale shapes output — validation, format_string, and transform are locale-specific
- Model retrain on CLDR-enriched training data

**Existing tasks:** NNFT-058 (date/time CLDR permutation), NNFT-060 (CLDR upstream), NNFT-133 (model retrain with CLDR data)

**Key risk:** CLDR data is vast. Need to scope which locales are "Tier 1" (high data volume, common in datasets) vs "Tier 2" (complete but lower priority).

### Milestone 2 — Format Coverage

**Goal:** Grow the taxonomy to handle the long tail of real-world formats cheaply.

**Current state:** 45 datetime types, 16 finance types. Actionability at 98.7%. JSON Schema validation powers type contracts.

**Scope:**
- Datetime: ambiguous formats (dd/MM vs MM/dd — requires column-level context), more ISO 8601 variants, epoch timestamps (seconds, milliseconds), natural language dates
- Currency: locale-specific formatting (comma vs period decimal, symbol placement), accounting notation (parentheses for negatives)
- Growth mechanism: YAML definition + JSON Schema contract + generator = new format covered. No model retrain needed for the long tail.
- Each new format must have a validation pattern that *rejects* non-matches — precision principle applies

**Metrics:** Actionability ≥95% as types are added. Per-predicted-type precision ≥95%.

**Design tension:** dd/MM vs MM/dd ambiguity is not solvable per-value. Requires column-level context (locale hint from header, other columns, or dataset metadata) or statistical disambiguation (e.g., values >12 in first position prove dd/MM). This needs a design decision within the milestone.

**Dependency:** Locale Foundation provides the CLDR infrastructure that makes format expansion scalable.

## Phase 2: Data Quality Pipeline

### Milestone 3 — Validate & Report

**Goal:** `profile → validate → report` as a first-class pipeline with actionable output.

**Current state:** `finetype validate` exists (NNFT-014). JSON Schema validation is baked into the inference pipeline (NNFT-013/116). Validation signals are used internally but not surfaced as user-facing reports.

**Scope:**
- `finetype validate <file>` → per-column quality report
- Output: invalid row counts, offending values with row numbers, quality scores (% valid, % null, % type-conforming)
- Machine-readable output (JSON) alongside human-readable (table/markdown)
- Powered by existing JSON Schema contracts — the work is surfacing internal signals, not building new validation

**Non-scope:** Cross-column constraints, referential integrity, cardinality anomalies. These are future work if there's demand.

**Key insight:** Most of the validation machinery exists. This milestone is about *presentation and workflow*, not plumbing.

### Milestone 4 — JSON Profiling

**Goal:** Profile JSON documents with the same accuracy as CSV.

**Current state:** `finetype_unpack(json)` in the DuckDB extension recursively classifies JSON fields. CLI supports CSV and Parquet.

**Scope:**
- JSON → flatten to table → profile with existing column pipeline
- Handle the common shape: array-of-objects (API responses, NDJSON)
- Preserve field path in output (`data.users[].email` → `identity.person.email`)
- Start with JSON only. XML/YAML are future work if there's demand.

**Implementation path:** DuckDB's `read_json` handles flattening. The work is gluing it to the existing profiler and handling edge cases (nested arrays, mixed types within a field).

**Independence:** This milestone has no dependencies on Phase 1 or other Phase 2 milestones. Could run in parallel.

### Milestone 5 — Schema Export

**Goal:** Turn type inference into actionable DDL.

**Current state:** `finetype schema <key>` exports JSON Schema per type. Each taxonomy definition specifies `broad_type` and `transform` SQL.

**Scope:**
- `finetype schema-for <file>` → DuckDB `CREATE TABLE` statement with correct column types
- Arrow schema output (for Parquet/IPC toolchains)
- Uses `broad_type` + `transform` from taxonomy contracts
- Read-only — FineType outputs the schema, user executes it

**Non-scope:** FineType does not write Parquet files or execute DDL. It stays read-only. "Write programs that do one thing and do it well."

**Dependency:** Benefits from Validate & Report (validated types → trustworthy DDL), but could ship a basic version without it.

## Dependency Graph

```
Phase 1:
  Locale Foundation ──→ Format Coverage

Phase 2:
  Format Coverage ──→ Validate & Report ──→ Schema Export
                      JSON Profiling (independent, can run in parallel)
```

## Success Criteria

| Milestone | Key metric |
|---|---|
| Locale Foundation | 50+ postal locales, 40+ phone locales, CLDR integrated |
| Format Coverage | Taxonomy covers 90%+ of formats seen in GitTables/SOTAB |
| Validate & Report | `finetype validate` produces actionable per-column reports |
| JSON Profiling | JSON profiling accuracy matches CSV profiling accuracy |
| Schema Export | Generated DDL is correct for all supported broad_types |
