# FineType

FineType is a type inference engine that detects and classifies data types in tabular datasets. It's the core analytical engine of the Meridian project.

## The Meridian Pillars

Every decision in this repo should reflect these principles:

1. **Spark joy for analysts** — Type inference should feel magical, not tedious. Clear output, helpful error messages, sensible defaults.
2. **Write programs that do one thing and do it well** — Each command has one job: `profile` discovers, `taxonomy` generates schema, `validate` enforces and materialises typed output. Separate concerns for separate tools.
3. **Design for the future, for it will be here sooner than you think** — The type taxonomy, model architecture, and extension interfaces should accommodate new data types and formats without breaking existing behaviour.

### Precision Principle

Precision is what makes FineType valuable. Every validation pattern, locale rule, and disambiguation heuristic must meaningfully distinguish "is this type" from "is not this type."

- Prefer precise locale-specific validation over permissive universal patterns. If a type is `designation: locale_specific`, its real validation lives in `validation_by_locale`, not the universal `validation` block.
- A validation that confirms 90% of random input is not a validation.
- Expanding locale coverage is the path to accuracy, not relaxing heuristics.

## Architectural direction (settled — do not re-ask)

- **Multi-branch implements the Sense stage** (decision 0041): The Sense→Sharpen pipeline has two stages — Sense (broad classification) and Sharpen (rule-based post-processing). The Sense stage is currently implemented by the multi-branch model; historically it was implemented by the original Sense model. Both remain in code; multi-branch is the v0.6.19 default.
- **Regex header hints deprecated** (decision 0042): Hardcoded regex `header_hint()` rules are deprecated in favour of learned approaches — multi-branch header branch (Model2Vec), sibling-context attention, semantic matching.
- **Value-based rules only** (decision 0048): New disambiguation rules check actual column values, not header metadata.
- **Strength through simplification** (decision 0038): Prefer retraining over adding disambiguation rules. Rules are a last resort.

## Project state

**Version:** 0.6.19
**Taxonomy:** 240 definitions across 7 domains (container 11, datetime 84, finance 28, geography 25, identity 33, representation 33, technology 26) — all generators pass, 100% alignment.
**Default Sense-stage model:** Multi-branch (sherlock-v19-relu-s42) inside the Sense→Sharpen pipeline. 5-branch: char+embed+stats+header+validation, ReLU+BatchNorm, val_acc 0.9173. Single forward pass per column. Profile eval **369/448 (82.4% label)** on 36 datasets. Original Sense implementation remains in code as an alternative.
**Codebase:** ~20k lines of Rust across 9 crates. Zero Python dependencies (build + runtime).
**Distribution:** GitHub releases (Linux x86/arm, macOS x86/arm, Windows), Homebrew tap, crates.io (core + model), DuckDB community extension (v0.2.0 merged), MCP server.

## Current sprint

**m-19 IN FLIGHT — eval-corpus expansion (Phase A+B).** Three deliverables: realism standard + pre-screen, coverage floor (240/240 types), train/eval leakage firewall (sources.yaml + row-hash SHA256). v18 retrain block enforced until Phase A+B ships. Spec: `orbit/specs/2026-04-21-eval-expansion/`. Decisions 0055/0056/0057.

## Decision register

48 architectural decisions in `orbit/decisions/` (MADR format). Browse: `ls orbit/decisions/` or Ctrl+B (fzf + glow preview).

## Tier-2 references — load on demand

**Before modifying the engine, model pipeline, taxonomy, MCP, DuckDB extension, training, or eval infrastructure:** Read `docs/ARCHITECTURE.md`.

**Before changing CLI surface, env vars, or build/test commands:** Read `docs/DEVELOPMENT.md`.

**Before promoting a model or cutting a release:** Read `docs/RELEASE.md`.

**For shipped feature history and release notes:** See `CHANGELOG.md`.
