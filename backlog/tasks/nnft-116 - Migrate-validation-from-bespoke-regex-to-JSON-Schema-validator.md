---
id: NNFT-116
title: Migrate validation from bespoke regex to JSON Schema validator
status: To Do
assignee:
  - '@nightingale'
created_date: '2026-02-24 06:52'
labels:
  - architecture
  - validation
  - ecosystem
dependencies: []
documentation:
  - 'https://github.com/sourcemeta/blaze'
  - 'https://github.com/Stranger6667/jsonschema-rs'
  - 'https://json-schema.org/specification'
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The taxonomy YAML validation fields use JSON Schema keywords (pattern, minLength, maxLength, minimum, maximum, enum) but the implementation in finetype-core::validator uses hand-rolled Rust regex matching instead of a proper JSON Schema validator.

**Why this matters:**
- Ecosystem integration: JSON Schema is a standard; proper compliance means FineType validation schemas can be consumed by any JSON Schema-aware tool
- Documentation: Users can reference JSON Schema spec docs rather than our bespoke interpretation
- Future validation: We'll want richer keywords (format, oneOf, if/then, $ref) as accuracy work progresses
- Regex dialect gap: JSON Schema specifies ECMA-262 regex; we use Rust regex crate. Currently no conflicts (patterns use basic constructs), but future patterns could diverge.

**Current state:**
- 169 type definitions with validation schemas in labels/definitions_*.yaml
- ~70 types have regex patterns (checked: all currently compatible between ECMA-262 and Rust regex)
- validator.rs: validate_value(), validate_value_for_label(), validate_column() — all use bespoke logic
- Used by: finetype check command, NNFT-115 attractor demotion (planned)

**Research needed:**
- Evaluate Blaze (https://github.com/sourcemeta/blaze) — high performance JSON Schema validator with published benchmarks. C++ based, may have Rust bindings or FFI options.
- Evaluate jsonschema Rust crate (https://github.com/Stranger6667/jsonschema-rs) — pure Rust, widely used
- Benchmark: validate_value() is called per-value during column classification. Need sub-microsecond per validation for hot path. Current regex approach is fast; JSON Schema overhead needs measurement.
- Compatibility: Ensure all 169 existing validation schemas are valid JSON Schema drafts
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Benchmark current validate_value() performance (calls/sec) as baseline
- [ ] #2 Evaluate JSON Schema validator options (Blaze via FFI, jsonschema-rs, others) for correctness and performance
- [ ] #3 Replace bespoke validator with JSON Schema-compliant implementation
- [ ] #4 All 169 existing validation schemas pass without modification (or document required changes)
- [ ] #5 Performance regression < 2x on validate_value() hot path
- [ ] #6 validator.rs public API preserved (validate_value, validate_column, etc.)
<!-- AC:END -->
