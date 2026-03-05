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

# ── Infer: Single Value ──────────────────────────────────────────────────────

section "2. Infer — Single Value"

OUT=$("$FINETYPE" infer -i "john.doe@example.com" 2>/dev/null)
assert_contains "email classified" "$OUT" "email"

OUT=$("$FINETYPE" infer -i "2026-02-13" 2>/dev/null)
assert_contains "date classified" "$OUT" "date"

OUT=$("$FINETYPE" infer -i "192.168.1.1" 2>/dev/null)
assert_contains "IPv4 classified" "$OUT" "ip_v4"

OUT=$("$FINETYPE" infer -i "bc89:60a9:23b8:c1e9:3924:56de:3eb1:3b90" 2>/dev/null)
assert_contains "IPv6 classified" "$OUT" "ip_v6"

OUT=$("$FINETYPE" infer -i "https://example.com" 2>/dev/null)
if echo "$OUT" | grep -qF "url" || echo "$OUT" | grep -qF "uri"; then
    pass "URL classified"
else
    fail "URL classified" "expected output to contain 'url' or 'uri', got '$OUT'"
fi

# ── Infer: Stdin ──────────────────────────────────────────────────────────────

section "3. Infer — Stdin"

OUT=$(echo "john.doe@example.com" | "$FINETYPE" infer 2>/dev/null)
assert_contains "stdin email classified" "$OUT" "email"

# Multiple values via stdin
OUT=$(printf "john@example.com\n192.168.1.1\n2026-02-13\n" | "$FINETYPE" infer 2>/dev/null)
LINE_COUNT=$(echo "$OUT" | wc -l | tr -d ' ')
assert_eq "stdin multi-line produces 3 lines" "$LINE_COUNT" "3"

# ── Infer: File Input ────────────────────────────────────────────────────────

section "4. Infer — File Input"

TMPFILE=$(mktemp /tmp/finetype-smoke-XXXXXX.txt)
trap 'rm -f "$TMPFILE" "${TMPFILE2:-}" "${TMPCSV:-}"' EXIT

cat > "$TMPFILE" <<'EOF'
john.doe@example.com
192.168.1.1
https://example.com
2026-02-13
EOF

OUT=$("$FINETYPE" infer --file "$TMPFILE" 2>/dev/null)
LINE_COUNT=$(echo "$OUT" | wc -l | tr -d ' ')
assert_eq "file input produces 4 lines" "$LINE_COUNT" "4"

# ── Infer: Output Formats ────────────────────────────────────────────────────

section "5. Infer — Output Formats"

# JSON output
OUT=$("$FINETYPE" infer -i "john@example.com" -o json 2>/dev/null)
assert_contains "json has label field" "$OUT" '"label"'

# JSON with confidence
OUT=$("$FINETYPE" infer -i "john@example.com" -o json --confidence 2>/dev/null)
assert_contains "json has confidence field" "$OUT" '"confidence"'

# JSON with value
OUT=$("$FINETYPE" infer -i "john@example.com" -o json -v 2>/dev/null)
assert_contains "json has input field" "$OUT" '"input"'

# CSV output
OUT=$("$FINETYPE" infer -i "john@example.com" -o csv 2>/dev/null)
assert_contains "csv contains email" "$OUT" "email"

# Plain with confidence (tab-separated)
OUT=$("$FINETYPE" infer -i "john@example.com" --confidence 2>/dev/null)
if echo "$OUT" | grep -qP '\t'; then
    pass "plain+confidence is tab-separated"
else
    # BSD grep may not support -P, check for tab via awk
    if echo "$OUT" | awk -F'\t' 'NF>1{found=1} END{exit !found}'; then
        pass "plain+confidence is tab-separated"
    else
        fail "plain+confidence is tab-separated" "no tab found in: $OUT"
    fi
fi

# ── Infer: Column Mode ──────────────────────────────────────────────────────

section "6. Infer — Column Mode"

TMPFILE2=$(mktemp /tmp/finetype-smoke-col-XXXXXX.txt)
cat > "$TMPFILE2" <<'EOF'
john@example.com
jane.doe@test.org
bob.smith@company.io
alice@mail.net
charlie@web.co
EOF

OUT=$("$FINETYPE" infer --file "$TMPFILE2" --mode column 2>/dev/null)
assert_contains "column mode classifies emails" "$OUT" "email"

# Column mode JSON
OUT=$("$FINETYPE" infer --file "$TMPFILE2" --mode column -o json 2>/dev/null)
assert_contains "column mode json has label" "$OUT" '"label"'
assert_contains "column mode json has samples_used" "$OUT" '"samples_used"'

# ── Embedded Model (No models/ dir) ──────────────────────────────────────────

section "7. Embedded Model — Works Without models/ Directory"

# Copy binary to /tmp and run from there — no models/ dir available
TMPBIN=$(mktemp /tmp/finetype-smoke-bin-XXXXXX)
cp "$FINETYPE" "$TMPBIN"
chmod +x "$TMPBIN"

OUT=$("$TMPBIN" infer -i "john@example.com" 2>/dev/null) || true
if echo "$OUT" | grep -qi "email"; then
    pass "binary works from /tmp without models/ dir"
else
    # Check if it failed with model error
    ERR=$("$TMPBIN" infer -i "john@example.com" 2>&1) || true
    if echo "$ERR" | grep -qi "model\|taxonomy\|not found"; then
        fail "binary works from /tmp without models/ dir" "model not embedded: $ERR"
    else
        fail "binary works from /tmp without models/ dir" "unexpected output: $OUT / $ERR"
    fi
fi
rm -f "$TMPBIN"

# Also test column mode from /tmp
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

# ── Error Handling ────────────────────────────────────────────────────────────

section "8. Error Handling"

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
