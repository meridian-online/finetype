# Discovery: Full Taxonomy Export & Schema Command

**Date**: 2026-02-27
**Author**: @nightingale
**Status**: Ready to implement
**Related**: noon-org/web task NNWB-015 (type registry page)

## Context

The website is getting a browsable type registry page (NNWB-015). For v1 we've exported the YAML definitions to a static JSON file manually. But for ongoing freshness, FineType's CLI should be the authoritative export tool.

Two gaps in the current CLI:

### Gap 1: `finetype taxonomy` exports only 7 of 16 fields

**Currently exported**: key, title, broad_type, designation, priority, transform, locales

**Missing**: description, validation (JSON Schema), samples, decompose, aliases, references, notes, format_string, transform_ext, tier

The full definition data is all loaded at runtime (it's compiled into the binary from YAML). It just isn't serialised in the JSON output path.

### Gap 2: No per-type JSON Schema export

Analysts want to grab the JSON Schema for a specific type — to validate data in their own pipelines, use in form builders, or integrate with other tools. There's no CLI path for this today.

The `Validation::to_json_schema()` method already exists in `finetype-core`. It just needs a CLI surface.

## Proposed changes

### 1. Add `--full` flag to `finetype taxonomy`

```bash
# Current (unchanged)
finetype taxonomy --output json
# → 7 fields per type

# New
finetype taxonomy --full --output json
# → All fields per type, including description, validation, samples, decompose, etc.
```

**Implementation**:
- In `crates/finetype-cli/src/main.rs`, the taxonomy JSON serialisation path
- Add a `--full` flag to the taxonomy subcommand args
- When `--full`, serialise the complete `Definition` struct instead of the slim subset
- Consider a `TaxonomyFullEntry` serde struct that maps all fields

**Fields to include in full export**:
```
key, title, description, domain, category, type,
broad_type, designation, transform, transform_ext,
format_string, locales, samples, validation (as JSON Schema),
decompose, aliases, tier, release_priority, references
```

**Estimated effort**: ~2 hours. The data is already loaded; this is serialisation work.

### 2. Add `finetype schema` subcommand

```bash
# Single type
finetype schema identity.person.email
# → Outputs complete JSON Schema object

# With formatting
finetype schema identity.person.email --pretty
# → Pretty-printed JSON Schema

# Multiple types (glob)
finetype schema "identity.person.*"
# → Array of JSON Schema objects

# Pipe-friendly
finetype schema identity.person.email | pbcopy
finetype schema identity.person.email | jq .pattern
```

**Output format** (single type):
```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://noon.sh/schemas/identity.person.email",
  "title": "Email Address",
  "description": "Standard email address format (RFC 5322)...",
  "type": "string",
  "pattern": "^[a-zA-Z0-9.!#$%&'*+/=?^_`{|}~-]+@...",
  "minLength": 5,
  "maxLength": 254,
  "examples": ["john.smith@example.com", "user+tag@domain.org"]
}
```

**Implementation**:
- New subcommand in `crates/finetype-cli/src/main.rs`
- Lookup type by key via `Taxonomy::get()`
- Use existing `Validation::to_json_schema()` and enrich with `$schema`, `$id`, `title`, `description`, `examples`
- Add `--pretty` flag (default: compact)
- Add glob support for pattern matching multiple types
- Exit code 1 if type not found, with helpful error suggesting similar types

**Estimated effort**: ~2 hours. Most logic exists; this is a new CLI surface.

### 3. (Future) CI automation

Once both commands exist, a GitHub Action on FineType release tags can:
```bash
finetype taxonomy --full --output json > taxonomy.json
# Open PR on noon-org/web to update src/data/taxonomy.json
```

This keeps the website registry fresh automatically. Not needed for v1.

## Design notes

- The `--full` flag is additive — the default `finetype taxonomy --output json` stays unchanged for backward compatibility
- JSON Schema output uses `$id` based on `https://noon.sh/schemas/{type.key}` — these URLs don't need to resolve, they're identifiers per JSON Schema spec
- The `schema` command is read-only and fast — it loads the compiled taxonomy and does a lookup, no inference involved
- Glob matching (`identity.person.*`) reuses the existing `Taxonomy::by_category()` filter path

## Files to modify

| File | Change |
|------|--------|
| `crates/finetype-cli/src/main.rs` | Add `--full` flag, add `schema` subcommand |
| `crates/finetype-core/src/taxonomy.rs` | Add `TaxonomyFullEntry` serde struct (or extend existing) |
| `crates/finetype-core/src/validator.rs` | Possibly enrich `to_json_schema()` with `$id`, `examples` |

## Acceptance criteria

- [ ] `finetype taxonomy --full --output json` exports all 16+ fields per type
- [ ] `finetype schema <type_key>` outputs a valid JSON Schema document
- [ ] `finetype schema <type_key> --pretty` outputs formatted JSON
- [ ] `finetype schema "domain.category.*"` supports glob patterns
- [ ] Unknown type key returns exit code 1 with helpful error
- [ ] Existing `finetype taxonomy` output (without `--full`) is unchanged
