# Doc-Driven CLI Tests — Design

**Date:** 2026-03-05
**Status:** Implemented

## Problem

The documentation in `docs/LOCALE_GUIDE.md` and `docs/SENSE_AND_SHARPEN_PIPELINE.md` describes target CLI behavior that doesn't yet match the actual implementation. For example:

- JSON output uses `"class"` not `"label"`
- No `locale` or `broad_type` fields in JSON output
- Plain text output doesn't append `.LOCALE` suffix

We need a test harness that tracks convergence between docs and CLI without blocking CI.

## Solution

A three-part test script (`tests/doc_tests.sh`) that:

1. **Parses markdown** — Extracts `$ finetype` examples from fenced code blocks in `docs/*.md`
2. **Runs tests** — Compares actual CLI output against documented expected output
3. **Reports parity** — Always exits 0, prints pass/fail/skip counts and parity percentage

### Architecture

```
tests/
  helpers.sh       # Shared helpers (extracted from smoke.sh)
  smoke.sh         # Existing smoke tests (sources helpers.sh)
  doc_tests.sh     # Doc-driven tests (sources helpers.sh)
  golden/          # Edge-case tests not covered in docs
    *.cmd          # Command to run (one per file)
    *.expected     # Expected output (companion to .cmd)
```

### Markdown Parser

The awk-based parser walks each `docs/*.md` file:
- Tracks fence open/close state (` ```bash ` blocks)
- Extracts lines starting with `$ finetype` as commands
- Collects subsequent lines as expected output
- Outputs NUL-delimited records: `command\0expected\0type\0`

### Skip Criteria

Commands are skipped when they:
- Contain pipes (`|`) or redirects (`>`)
- Reference files we don't have (`data.csv`, `customers.csv`)
- Use `--mode column` (requires real columnar data)
- Use `profile` or `generate` subcommands

### Assertion Types

- **Plain text**: Exact string match (`assert_eq`)
- **JSON subset**: Every key in expected exists in actual with same value (`assert_json_subset` via `jq`)

### Golden Files

Edge cases not covered in documentation, stored as `.cmd`/`.expected` pairs:

| File | Tests |
|------|-------|
| `locale-phone-ambiguous` | UK-format phone without country code |
| `locale-postal-numeric` | Numeric postal code (DE 5-digit) |
| `locale-month-unicode` | Non-Latin month name (Russian январь) |

### Makefile Targets

| Target | Description |
|--------|-------------|
| `make test-smoke` | Run smoke tests only |
| `make test-docs` | Run doc + golden tests only |
| `make test-golden` | Run golden file tests only |
| `make test-cli` | Run smoke + doc tests (full CLI test suite) |

## Test Inventory

| Source | Testable | Skipped | Notes |
|--------|----------|---------|-------|
| LOCALE_GUIDE.md | ~10 | ~3 | JSON subset + plain text |
| SENSE_AND_SHARPEN_PIPELINE.md | 0 | ~3 | No expected output in examples |
| Golden files | 3 | 0 | Edge cases |
| **Total** | **~13** | **~6** | |

## Expected Initial Parity

~0% — the CLI doesn't match the documented target behavior yet. As CLI features are implemented (locale in output, label→class rename, broad_type field), parity will increase toward 100%.

## Design Decisions

1. **Always exit 0** — These tests inform, not block. CI smoke tests remain the gate.
2. **Source helpers.sh** — Shared with smoke.sh to avoid duplication.
3. **NUL-delimited awk output** — Handles multi-line expected output safely.
4. **JSON subset, not equality** — CLI may return extra fields; we only check documented ones.
5. **jq dependency** — Already available in dev environments; simpler than hand-rolled JSON parsing.
