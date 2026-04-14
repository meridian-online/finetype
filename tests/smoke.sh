#!/usr/bin/env bash
# ═══════════════════════════════════════════════════════════════════════════════
# FineType CLI Smoke Tests
# ═══════════════════════════════════════════════════════════════════════════════
#
# End-to-end tests that exercise the compiled release binary.
# These catch regressions like missing embedded models, broken subcommands,
# or path resolution issues that unit tests can't detect.
#
# Usage:
#   ./tests/smoke.sh                  # build + test
#   ./tests/smoke.sh --skip-build     # test existing binary at target/release/finetype
#   FINETYPE=./my-binary ./tests/smoke.sh --skip-build  # test a specific binary

set -euo pipefail

# ── Load shared helpers ─────────────────────────────────────────────────────

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/helpers.sh"

# ── Build ─────────────────────────────────────────────────────────────────────

handle_build "$@"

# ═══════════════════════════════════════════════════════════════════════════════
# TEST SUITE
# ═══════════════════════════════════════════════════════════════════════════════

section "1. Version & Help"

# --version should output "finetype <version>"
VERSION_OUTPUT=$("$FINETYPE" --version 2>&1)
EXPECTED_VERSION=$(grep '^version' "$REPO_ROOT/Cargo.toml" | head -1 | sed 's/.*"\(.*\)"/\1/')
assert_contains "finetype --version output" "$VERSION_OUTPUT" "$EXPECTED_VERSION"

# --help should succeed and mention subcommands
HELP_OUTPUT=$("$FINETYPE" --help 2>&1)
assert_contains "--help mentions infer" "$HELP_OUTPUT" "infer"
assert_contains "--help mentions taxonomy" "$HELP_OUTPUT" "taxonomy"

# ── Infer: Single Value (Column Mode) ────────────────────────────────────────
# Multi-branch model is column-level only, so all infer tests use --mode column.

section "2. Infer — Single Value"

# Single-value column mode works well for distinctive types (email, URL, IPv4).
# Types needing distributional signal (dates, IPv6) are tested with multi-value
# columns in section 3.

OUT=$("$FINETYPE" infer -i "john.doe@example.com" --mode column 2>/dev/null)
assert_contains "email classified" "$OUT" "email"

OUT=$("$FINETYPE" infer -i "192.168.1.1" --mode column 2>/dev/null)
assert_contains "IPv4 classified" "$OUT" "ip_v4"

OUT=$("$FINETYPE" infer -i "https://example.com" --mode column 2>/dev/null)
if echo "$OUT" | grep -qF "url" || echo "$OUT" | grep -qF "uri"; then
    pass "URL classified"
else
    fail "URL classified" "expected output to contain 'url' or 'uri', got '$OUT'"
fi

# ── Infer: Stdin ──────────────────────────────────────────────────────────────

section "3. Infer — Stdin (Column Mode)"

OUT=$(echo "john.doe@example.com" | "$FINETYPE" infer --mode column 2>/dev/null)
assert_contains "stdin email classified" "$OUT" "email"

# Column mode with header hint (realistic usage — header provides disambiguation)
OUT=$(printf "john@example.com\njane@test.org\nbob@company.io\nalice@mail.net\ncharlie@web.co\n" | "$FINETYPE" infer --mode column --header "email" 2>/dev/null)
assert_contains "stdin email column with header" "$OUT" "email"

# Date column with header hint
OUT=$(printf "2026-02-13\n2025-11-01\n2024-06-15\n2023-03-22\n2022-09-30\n" | "$FINETYPE" infer --mode column --header "created_date" 2>/dev/null)
assert_contains "date column classified" "$OUT" "date"

# IPv4 column (no header needed — distinctive pattern)
OUT=$(printf "192.168.1.1\n10.0.0.1\n172.16.0.1\n8.8.8.8\n1.1.1.1\n" | "$FINETYPE" infer --mode column 2>/dev/null)
assert_contains "IPv4 column classified" "$OUT" "ip"

# ── Infer: File Input ────────────────────────────────────────────────────────

section "4. Infer — File Input (Column Mode)"

TMPFILE=$(mktemp /tmp/finetype-smoke-XXXXXX.txt)
trap 'rm -f "$TMPFILE" "${TMPFILE2:-}" "${TMPCSV:-}"' EXIT

cat > "$TMPFILE" <<'EOF'
john.doe@example.com
jane.doe@test.org
bob.smith@company.io
alice.jones@mail.net
charlie.brown@web.co
EOF

# File column mode with header hint (realistic usage)
OUT=$("$FINETYPE" infer --file "$TMPFILE" --mode column --header "email" 2>/dev/null)
assert_contains "file column mode classifies emails" "$OUT" "email"

# ── Infer: Output Formats ────────────────────────────────────────────────────

section "5. Infer — Output Formats"

# JSON output (column mode includes label, confidence, samples_used)
OUT=$("$FINETYPE" infer -i "john.doe@example.com" --mode column -o json 2>/dev/null)
assert_contains "json has label field" "$OUT" '"label"'
assert_contains "json has confidence field" "$OUT" '"confidence"'
assert_contains "json has samples_used field" "$OUT" '"samples_used"'

# CSV output
OUT=$("$FINETYPE" infer -i "john.doe@example.com" --mode column -o csv 2>/dev/null)
assert_contains "csv contains email" "$OUT" "email"

# ── Infer: Column Mode — Homogeneous ────────────────────────────────────────

section "6. Infer — Column Mode (Homogeneous)"

TMPFILE2=$(mktemp /tmp/finetype-smoke-col-XXXXXX.txt)
cat > "$TMPFILE2" <<'EOF'
john@example.com
jane.doe@test.org
bob.smith@company.io
alice@mail.net
charlie@web.co
EOF

OUT=$("$FINETYPE" infer --file "$TMPFILE2" --mode column --header "email" 2>/dev/null)
assert_contains "column mode classifies emails" "$OUT" "email"

# Column mode JSON
OUT=$("$FINETYPE" infer --file "$TMPFILE2" --mode column --header "email" -o json 2>/dev/null)
assert_contains "column mode json has label" "$OUT" '"label"'
assert_contains "column mode json has samples_used" "$OUT" '"samples_used"'

# ── Embedded Model (No models/ dir) ──────────────────────────────────────────

section "7. Embedded Model — Works Without models/ Directory"

# Copy binary to /tmp and run from there — no models/ dir available
TMPBIN=$(mktemp /tmp/finetype-smoke-bin-XXXXXX)
cp "$FINETYPE" "$TMPBIN"
chmod +x "$TMPBIN"

OUT=$("$TMPBIN" infer -i "john.doe@example.com" --mode column 2>/dev/null) || true
if echo "$OUT" | grep -qi "email"; then
    pass "binary works from /tmp without models/ dir"
else
    # Check if it failed with model error
    ERR=$("$TMPBIN" infer -i "john.doe@example.com" --mode column 2>&1) || true
    if echo "$ERR" | grep -qi "model\|taxonomy\|not found"; then
        fail "binary works from /tmp without models/ dir" "model not embedded: $ERR"
    else
        fail "binary works from /tmp without models/ dir" "unexpected output: $OUT / $ERR"
    fi
fi
rm -f "$TMPBIN"

# Also test column mode from /tmp with stdin
TMPBIN2=$(mktemp /tmp/finetype-smoke-bin2-XXXXXX)
cp "$FINETYPE" "$TMPBIN2"
chmod +x "$TMPBIN2"

OUT=$(printf "john@example.com\njane@test.org\nbob@company.io\n" | "$TMPBIN2" infer --mode column 2>/dev/null) || true
if echo "$OUT" | grep -qi "email"; then
    pass "column mode works from /tmp without models/ dir"
else
    fail "column mode works from /tmp without models/ dir" "got: $OUT"
fi

# Profile command with embedded model (no models/ dir)
TMPCSV=$(mktemp /tmp/finetype-smoke-csv-XXXXXX.csv)
cat > "$TMPCSV" <<'CSVEOF'
name,email,age
John Doe,john@example.com,30
Jane Smith,jane@test.org,25
Bob Wilson,bob@company.io,45
CSVEOF

OUT=$("$TMPBIN2" profile -f "$TMPCSV" 2>/dev/null) || true
if echo "$OUT" | grep -qi "email\|Column Profile"; then
    pass "profile works from /tmp without models/ dir"
else
    ERR=$("$TMPBIN2" profile -f "$TMPCSV" 2>&1) || true
    fail "profile works from /tmp without models/ dir" "got: $ERR"
fi
rm -f "$TMPBIN2" "$TMPCSV"

# Taxonomy command with embedded taxonomy (no labels/ dir)
TMPBIN3=$(mktemp /tmp/finetype-smoke-bin3-XXXXXX)
cp "$FINETYPE" "$TMPBIN3"
chmod +x "$TMPBIN3"

OUT=$("$TMPBIN3" taxonomy 2>/dev/null) || true
if echo "$OUT" | grep -qi "Total labels"; then
    pass "taxonomy works from /tmp without labels/ dir"
else
    ERR=$("$TMPBIN3" taxonomy 2>&1) || true
    fail "taxonomy works from /tmp without labels/ dir" "got: $ERR"
fi
rm -f "$TMPBIN3"

# ── Load — DuckDB Integration ────────────────────────────────────────────────

section "8. Load — DuckDB Round-Trip"

if command -v duckdb &>/dev/null; then
    # Create CSV with reserved-word column names (name, type, source)
    # These trigger DuckDB's normalize_names renaming (name→_name) which
    # previously broke the generated SQL.
    LOAD_CSV=$(mktemp /tmp/finetype-smoke-load-XXXXXX.csv)
    cat > "$LOAD_CSV" <<'CSVEOF'
name,type,source,city
John Doe,person,manual,Auckland
Jane Smith,person,import,Wellington
Bob Wilson,business,api,Christchurch
Alice Brown,person,manual,Dunedin
Charlie Lee,person,import,Hamilton
CSVEOF

    # finetype load output should be valid SQL that DuckDB can execute
    LOAD_SQL=$("$FINETYPE" load -f "$LOAD_CSV" --limit 0 2>/dev/null)
    if echo "$LOAD_SQL" | duckdb 2>/dev/null; then
        pass "load | duckdb with reserved-word columns (name, type, source)"
    else
        LOAD_ERR=$(echo "$LOAD_SQL" | duckdb 2>&1 | grep -i "error" | head -3)
        fail "load | duckdb with reserved-word columns" "$LOAD_ERR"
    fi

    # Also test with the preview SELECT (default --limit 10)
    LOAD_SQL_PREVIEW=$("$FINETYPE" load -f "$LOAD_CSV" 2>/dev/null)
    if echo "$LOAD_SQL_PREVIEW" | duckdb 2>/dev/null; then
        pass "load | duckdb with preview SELECT"
    else
        LOAD_ERR=$(echo "$LOAD_SQL_PREVIEW" | duckdb 2>&1 | grep -i "error" | head -3)
        fail "load | duckdb with preview SELECT" "$LOAD_ERR"
    fi

    rm -f "$LOAD_CSV"
else
    skip "load | duckdb round-trip (duckdb not installed)"
    skip "load | duckdb with preview SELECT (duckdb not installed)"
fi

# ── Error Handling ────────────────────────────────────────────────────────────

section "9. Error Handling"

# Missing subcommand should show help (non-zero exit is OK)
OUT=$("$FINETYPE" 2>&1) || true
assert_contains "no subcommand shows usage" "$OUT" "Usage"

# Invalid subcommand
OUT=$("$FINETYPE" nonexistent 2>&1) || true
assert_contains "invalid subcommand shows error" "$OUT" "error"

# ═══════════════════════════════════════════════════════════════════════════════
# SUMMARY
# ═══════════════════════════════════════════════════════════════════════════════

print_summary "Results"

if [ "$FAIL" -gt 0 ]; then
    exit 1
fi

printf "\n\033[32mAll smoke tests passed.\033[0m\n"
