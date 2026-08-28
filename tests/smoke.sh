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

# Single-value column mode works well for very distinctive types (URL, IPv4).
# Types that need distributional signal (email, dates, IPv6) are tested with
# multi-value columns in section 3 — that's the realistic CLI usage anyway.

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

# Multi-value email via stdin — the realistic CLI usage
OUT=$(printf "john@example.com\njane@test.org\nbob@company.io\nalice@mail.net\ncharlie@web.co\n" | "$FINETYPE" infer --mode column 2>/dev/null)
assert_contains "stdin email column" "$OUT" "email"

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

# JSON output (column mode includes label, confidence, samples_used).
# Using URL since it's a reliable N=1 classifier — the purpose of this
# section is to test output schema, not the model's classification.
OUT=$("$FINETYPE" infer -i "https://example.com" --mode column -o json 2>/dev/null)
assert_contains "json has label field" "$OUT" '"label"'
assert_contains "json has confidence field" "$OUT" '"confidence"'
assert_contains "json has samples_used field" "$OUT" '"samples_used"'

# CSV output
OUT=$("$FINETYPE" infer -i "https://example.com" --mode column -o csv 2>/dev/null)
if echo "$OUT" | grep -qF "url" || echo "$OUT" | grep -qF "uri"; then
    pass "csv contains url label"
else
    fail "csv contains url label" "expected 'url' or 'uri', got '$OUT'"
fi

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

# Copy binary to /tmp and run from there — no models/ dir available.
# Use URL since it's reliable at N=1; this section tests model embedding,
# not the classifier's behaviour on short emails.
TMPBIN=$(mktemp /tmp/finetype-smoke-bin-XXXXXX)
cp "$FINETYPE" "$TMPBIN"
chmod +x "$TMPBIN"

OUT=$("$TMPBIN" infer -i "https://example.com" --mode column 2>/dev/null) || true
if echo "$OUT" | grep -qiE "url|uri"; then
    pass "binary works from /tmp without models/ dir"
else
    # Check if it failed with model error
    ERR=$("$TMPBIN" infer -i "https://example.com" --mode column 2>&1) || true
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

# ── Load subcommand removed — should error via clap unknown-subcommand ──────

section "8. finetype load — removed in v0.6.19 (MADR 0071)"

# `finetype load …` must error with exit 2 via clap's unknown-subcommand
# handler. No shim, no warning, no carve-out. The typed-CTAS path now
# lives on `finetype validate --db --table` (covered by validate_cli.rs).
LOAD_OUT=$("$FINETYPE" load -f /tmp/anything.csv 2>&1) || LOAD_EXIT=$?
if [ "${LOAD_EXIT:-0}" -eq 2 ]; then
    pass "finetype load exits 2 via clap unknown-subcommand"
else
    fail "finetype load exits 2 via clap unknown-subcommand" "got exit ${LOAD_EXIT:-0}: $LOAD_OUT"
fi

# ── Column order ──────────────────────────────────────────────────────────────

section "9. Profile — Column Order"

# `profile` emits one entry per input column and every consumer reads that
# sequence POSITIONALLY — the eval fixtures, the DuckDB extension, the
# json-schema/datapackage writers. Column classification runs in parallel, and a
# parallel collect into an unordered container permutes results while leaving the
# SET intact, so order has to be asserted, not assumed.
#
# Two different things are checked, because checking only the first is not enough.
# The emitted column NAMES are read from the input file, and the LABELS from the
# parallel results, so a lost ordering inside the classifier scrambles which label
# lands on which column while the names stay in perfect file order — a name-only
# check passes on that build. So: the name sequence AND each column's own type.
#
# The columns are deliberately of eight different types; a file whose columns all
# classify the same way would satisfy any permutation.
TMPORDDIR=$(mktemp -d /tmp/finetype-smoke-order-XXXXXX)
TMPORD="$TMPORDDIR/column_order.csv"
cat > "$TMPORD" <<'CSVEOF'
zeta_email,alpha_created_at,mike_ip,bravo_uuid,yankee_amount,charlie_url,delta_country,echo_id
ada@example.com,2024-01-05T09:30:00Z,192.168.0.1,550e8400-e29b-41d4-a716-446655440000,12.50,https://a.example.com,US,A1
grace@example.org,2024-02-11T18:04:22Z,10.0.0.255,6ba7b810-9dad-11d1-80b4-00c04fd430c8,88.25,https://b.example.org,GB,B2
alan@example.net,2024-03-30T00:00:01Z,172.16.4.9,6ba7b811-9dad-11d1-80b4-00c04fd430c8,3.75,https://c.example.net,DE,C3
edsger@example.com,2024-04-01T12:12:12Z,8.8.8.8,6ba7b812-9dad-11d1-80b4-00c04fd430c8,0.50,https://d.example.io,FR,D4
CSVEOF

EXPECTED_ORDER=$(head -1 "$TMPORD" | tr ',' '\n' | tr -d '\r' | paste -sd, -)

# Default path (header hints + sibling context).
ORDER_JSON=$("$FINETYPE" profile -f "$TMPORD" -o json 2>/dev/null)
ACTUAL_ORDER=$(printf '%s' "$ORDER_JSON" | grep -o '"column": "[^"]*"' | sed 's/"column": "//; s/"$//' | paste -sd, -)
assert_eq "profile emits columns in file order" "$ACTUAL_ORDER" "$EXPECTED_ORDER"

# Header-hint-free path — a different per-column loop, same contract.
ORDER_JSON=$("$FINETYPE" profile -f "$TMPORD" -o json --no-header-hint 2>/dev/null)
ACTUAL_ORDER=$(printf '%s' "$ORDER_JSON" | grep -o '"column": "[^"]*"' | sed 's/"column": "//; s/"$//' | paste -sd, -)
assert_eq "profile --no-header-hint emits columns in file order" "$ACTUAL_ORDER" "$EXPECTED_ORDER"

# datapackage writes the columns as an ordered `fields` array; same check through
# the interoperable envelope.
ORDER_DP=$("$FINETYPE" profile -f "$TMPORD" -o datapackage 2>/dev/null)
ACTUAL_ORDER=$(printf '%s' "$ORDER_DP" | grep -o '"name": "[^"]*"' | sed 's/"name": "//; s/"$//' | grep -v '^column_order$' | paste -sd, -)
assert_eq "datapackage fields follow file order" "$ACTUAL_ORDER" "$EXPECTED_ORDER"

# Each column keeps its OWN type. `-o csv` emits `column,type` per row, so this
# reads the pairing directly. A classifier that returns its results in some
# schedule-determined order sends `charlie_url`'s label to `delta_country` and
# vice versa, which every check above still passes.
COLUMN_TYPE_PAIRS() {  # <extra profile args...>
    "$FINETYPE" profile -f "$TMPORD" -o csv "$@" 2>/dev/null \
        | tail -n +2 | cut -d, -f1,2 | tr -d '"'
}

# column:a fragment unique to the type that column's values are
EXPECT_TYPES="zeta_email:email
alpha_created_at:iso_8601
mike_ip:ip_v4
bravo_uuid:uuid
yankee_amount:decimal_number
charlie_url:url
delta_country:country_code
echo_id:alphanumeric_id"

check_pairing() {  # <label> <pairs>
    local what="$1" pairs="$2" col frag got bad=""
    while IFS= read -r spec; do
        col="${spec%%:*}"
        frag="${spec#*:}"
        got=$(printf '%s\n' "$pairs" | grep "^$col," | cut -d, -f2)
        case "$got" in
            *"$frag"*) ;;
            *) bad="$bad $col=>'$got' (wanted *$frag*)" ;;
        esac
    done <<< "$EXPECT_TYPES"
    if [ -z "$bad" ]; then
        pass "$what"
    else
        fail "$what" "mis-paired:$bad"
    fi
}

PAIRS_HINT=$(COLUMN_TYPE_PAIRS)
check_pairing "each column keeps its own type" "$PAIRS_HINT"

PAIRS_NOHINT=$(COLUMN_TYPE_PAIRS --no-header-hint)
check_pairing "each column keeps its own type (--no-header-hint)" "$PAIRS_NOHINT"

# Repeat runs must not reshuffle: thread scheduling varies, the answer must not.
# Compares the whole column,type sequence, so this catches a permutation without
# knowing anything about which labels the current model produces.
STABLE=1
for _ in 1 2 3; do
    [ "$(COLUMN_TYPE_PAIRS)" = "$PAIRS_HINT" ] || STABLE=0
done
if [ "$STABLE" -eq 1 ]; then
    pass "column,type sequence is stable across runs"
else
    fail "column,type sequence is stable across runs" "repeat runs disagreed; first run was: $PAIRS_HINT"
fi

rm -rf "$TMPORDDIR"

# ── Column count is the file's, not the sniffer's ─────────────────────────────

section "10. Profile — the sniffer cannot widen the schema"

# `read_csv_input` passed `null_padding=true` alongside `auto_detect=true`.
# `null_padding` pads a short ragged row with NULLs, which is why it is there.
# It ALSO lets the sniffer widen the schema: with row widths no longer required
# to agree, a delimiter that splits only some rows becomes acceptable. Measured
# on duckdb v1.5.5, the two prose fixtures below reported EIGHT and FIVE columns
# respectively under that option, the extra ones named `column1`…`columnN` and
# carrying labels and confidences into the descriptor as though they were real.
#
# The fix sniffs the shape first and reads with the column list the sniff
# pinned, so the two properties are separable: the count comes from the sniff,
# the padding from the read. Every assertion here is on `profile`'s own output.

# ── The real-data case: 459 NAICS descriptions, one column, semicolon-heavy
# prose, written by duckdb's own COPY. Under the defect: 8 columns.
NAICS="$REPO_ROOT/tests/fixtures/label_stability/naics_description.csv"
NAICS_ERR=$("$FINETYPE" profile -f "$NAICS" -o json 2>&1 >/dev/null) || true
assert_contains "single-column prose CSV profiles as one column" \
    "$NAICS_ERR" 'Found 1 columns: ["description"]'

# ── The same defect on a fixture small enough to assert a VALUE against. Its
# descriptions carry 0, 1, 2, 3 and 4 semicolons — the inconsistency is what
# moves the sniffer — and there are few enough distinct values that
# `x-finetype-enum` publishes the domain verbatim, which is the only surface on
# which `profile` shows what it actually read. Under the defect this column is
# cut at the first semicolon and the domain holds fragments.
PROSE="$REPO_ROOT/tests/fixtures/prose_semicolons.csv"
PROSE_ERR=$("$FINETYPE" profile -f "$PROSE" -o json 2>&1 >/dev/null) || true
assert_contains "semicolon-heavy prose profiles as one column" \
    "$PROSE_ERR" 'Found 1 columns: ["description"]'

PROSE_SCHEMA=$("$FINETYPE" profile -f "$PROSE" -o json-schema 2>/dev/null)
assert_contains "a semicolon-bearing value is read back whole" \
    "$PROSE_SCHEMA" \
    "cutting timber; transporting timber; and producing wood chips in the field"

# ── The property `null_padding` was added for, which the fix has to keep: a
# genuinely ragged file — rows with FEWER fields than the header — still
# profiles, and the short rows are padded rather than the run failing. Two of
# the 40 rows are missing their third field.
#
# Both halves are asserted. The column count alone is not enough: a strict sniff
# with no padding at all finds no delimiter whose widths agree on this file and
# collapses it to ONE column named `id,name,city`, so a fix that only stopped
# the widening would report a wrong count here and pass any count-free check.
# The null count is what proves the pad happened.
RAGGED="$REPO_ROOT/tests/fixtures/ragged_short_rows.csv"
RAGGED_ERR=$("$FINETYPE" profile -f "$RAGGED" -o json 2>&1 >/dev/null) || true
assert_contains "ragged CSV keeps its header's column count" \
    "$RAGGED_ERR" 'Found 3 columns: ["id", "name", "city"]'

RAGGED_JSON=$("$FINETYPE" profile -f "$RAGGED" -o json 2>/dev/null)
# `|| echo none` matters: under `set -o pipefail` a grep that matches nothing
# aborts the whole script, which would end the suite before this assertion could
# report — the run still exits non-zero, but the assertion that pins the padding
# never speaks. It has to be able to FAIL, not to kill the harness.
RAGGED_NULLS=$(printf '%s\n' "$RAGGED_JSON" | tr -d ' ' \
    | grep -A20 '"column":"city"' | grep '"null":' | head -1 \
    | sed 's/.*"null"://; s/,$//' || echo none)
assert_eq "short rows are padded, not dropped" "$RAGGED_NULLS" "2"


# ── Error Handling ────────────────────────────────────────────────────────────

section "11. Error Handling"

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
