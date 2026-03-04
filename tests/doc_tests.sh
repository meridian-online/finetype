#!/usr/bin/env bash
# ═══════════════════════════════════════════════════════════════════════════════
# FineType Doc-Driven CLI Tests
# ═══════════════════════════════════════════════════════════════════════════════
#
# Parses fenced code blocks from docs/*.md, extracts `$ finetype` examples,
# runs them against the actual CLI, and reports parity progress.
#
# These tests are INFORMATIONAL ONLY — they always exit 0.
# They track convergence between documentation and CLI behaviour.
#
# Usage:
#   ./tests/doc_tests.sh                  # build + test all
#   ./tests/doc_tests.sh --skip-build     # test existing binary
#   ./tests/doc_tests.sh --golden-only    # only run golden file tests
#   ./tests/doc_tests.sh --skip-build --golden-only
#
# Dependencies: jq (for JSON subset assertions)

set -euo pipefail

# ── Load shared helpers ─────────────────────────────────────────────────────

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/helpers.sh"

# ── Parse arguments ──────────────────────────────────────────────────────────

SKIP_BUILD=false
GOLDEN_ONLY=false

for arg in "$@"; do
    case "$arg" in
        --skip-build)  SKIP_BUILD=true ;;
        --golden-only) GOLDEN_ONLY=true ;;
    esac
done

if [ "$SKIP_BUILD" = true ]; then
    handle_build --skip-build
else
    handle_build
fi

# ═══════════════════════════════════════════════════════════════════════════════
# PART A: Markdown Parser
# ═══════════════════════════════════════════════════════════════════════════════
#
# Extracts (command, expected_output) pairs from fenced ```bash blocks in
# markdown files. For each block:
#   - Lines starting with "$ finetype" are commands
#   - Subsequent lines until next "$ " or fence close are expected output
#   - Blocks with pipes |, redirects >, or non-finetype $ commands are skipped

# AWK script that extracts (command, expected_output, type) records.
# Outputs tab-delimited lines: command<TAB>expected<TAB>type
# Multi-line expected output uses literal \n escapes (decoded by the runner).
#
# Fence tracking: we must handle non-bash fences (```sql, ```markdown, etc.)
# correctly — their closing ``` must NOT open a bash fence.
DOC_PARSER_AWK='
BEGIN { in_bash = 0; in_other = 0; cmd = ""; expected = ""; collecting = 0 }

# Opening fence: ```bash
/^```bash/ {
    if (!in_bash && !in_other) {
        in_bash = 1
    }
    next
}

# Opening fence: ```<something-else> (sql, markdown, etc.)
/^```[a-zA-Z]/ {
    if (!in_bash && !in_other) {
        in_other = 1
    }
    next
}

# Bare closing fence: ```
/^```$/ {
    if (in_other) {
        # Close a non-bash fence — ignore
        in_other = 0
        next
    }
    if (in_bash) {
        # Close a bash fence — emit any pending test
        if (cmd != "" && expected != "") {
            otype = "plain"
            if (expected ~ /^\{/) otype = "json"
            printf "%s\t%s\t%s\n", cmd, expected, otype
        }
        in_bash = 0; cmd = ""; expected = ""; collecting = 0
    }
    next
}

# Skip if not in a bash fence
!in_bash { next }

# Line starting with "$ finetype" — a command to test
/^\$ finetype/ {
    # Emit previous test if any
    if (cmd != "" && expected != "") {
        otype = "plain"
        if (expected ~ /^\{/) otype = "json"
        printf "%s\t%s\t%s\n", cmd, expected, otype
    }
    # Strip "$ " prefix
    cmd = substr($0, 3)
    expected = ""
    collecting = 1
    next
}

# Another "$ " line (different command) — stop collecting expected output
/^\$ / {
    if (cmd != "" && expected != "") {
        otype = "plain"
        if (expected ~ /^\{/) otype = "json"
        printf "%s\t%s\t%s\n", cmd, expected, otype
    }
    cmd = ""; expected = ""; collecting = 0
    next
}

# Collecting expected output lines
collecting {
    if (expected == "") {
        expected = $0
    } else {
        expected = expected "\\n" $0
    }
}
'

# ── Filter: skip commands with pipes, redirects, or non-simple invocations ──

should_skip_command() {
    local cmd="$1"
    # Skip commands with pipes (multi-command pipelines)
    if echo "$cmd" | grep -qF '|'; then return 0; fi
    # Skip commands with output redirects
    if echo "$cmd" | grep -qF '>'; then return 0; fi
    # Skip commands that reference files we don't have (data.csv, customers.csv, etc.)
    if echo "$cmd" | grep -qE '(data\.csv|customers\.csv|phones\.txt|profile\.json)'; then return 0; fi
    # Skip commands using --mode column with file args (need real data)
    if echo "$cmd" | grep -qE '\-\-mode column'; then return 0; fi
    # Skip profile commands (need real CSV files)
    if echo "$cmd" | grep -qF 'profile'; then return 0; fi
    # Skip generate commands
    if echo "$cmd" | grep -qF 'generate'; then return 0; fi
    return 1
}

# Check if expected output is only comments (lines starting with #)
is_comment_only_output() {
    local expected_raw="$1"
    local decoded
    decoded=$(printf '%b' "$expected_raw")
    # Strip empty lines and check if all remaining lines start with #
    local non_comment
    non_comment=$(echo "$decoded" | grep -v '^$' | grep -v '^#' | head -1)
    [ -z "$non_comment" ]
}

# ═══════════════════════════════════════════════════════════════════════════════
# PART B: Test Runners
# ═══════════════════════════════════════════════════════════════════════════════

run_doc_tests() {
    local md_file="$1"
    local basename
    basename=$(basename "$md_file" .md)

    section "Doc tests: $basename"

    local test_num=0
    local lines
    lines=$(awk "$DOC_PARSER_AWK" "$md_file")

    if [ -z "$lines" ]; then
        printf "  (no testable examples found)\n"
        return
    fi

    # Read tab-delimited records: cmd<TAB>expected<TAB>type
    while IFS=$'\t' read -r cmd expected_raw otype; do
        [ -z "$cmd" ] && continue

        test_num=$((test_num + 1))
        local test_name="${basename}#${test_num}: ${cmd}"

        # Check if we should skip
        if should_skip_command "$cmd"; then
            skip "$test_name"
            continue
        fi

        # Skip tests where expected output is only comments (not real output)
        if is_comment_only_output "$expected_raw"; then
            skip "$test_name (comment-only output)"
            continue
        fi

        # Decode escaped newlines in expected output
        local expected
        expected=$(printf '%b' "$expected_raw")

        # Run the command (replace "finetype" at start with actual binary path)
        local full_cmd
        full_cmd=$(echo "$cmd" | sed "s|^finetype|$FINETYPE|")

        local actual
        actual=$(eval "$full_cmd" 2>/dev/null) || true

        # Strip trailing whitespace from both for comparison
        actual=$(echo "$actual" | sed 's/[[:space:]]*$//')
        expected=$(echo "$expected" | sed 's/[[:space:]]*$//')

        if [ "$otype" = "json" ]; then
            # JSON subset check: every key in expected exists in actual
            assert_json_subset "$test_name" "$actual" "$expected"
        else
            # Plain text: exact match
            assert_eq "$test_name" "$actual" "$expected"
        fi

    done <<< "$lines"
}

# ═══════════════════════════════════════════════════════════════════════════════
# PART C: Golden File Runner
# ═══════════════════════════════════════════════════════════════════════════════

run_golden_tests() {
    local golden_dir="$SCRIPT_DIR/golden"

    if [ ! -d "$golden_dir" ]; then
        printf "  (no golden/ directory found)\n"
        return
    fi

    local found=0
    for cmd_file in "$golden_dir"/*.cmd; do
        [ -f "$cmd_file" ] || continue
        found=$((found + 1))

        local base
        base=$(basename "$cmd_file" .cmd)
        local expected_file="${cmd_file%.cmd}.expected"
        local test_name="golden/$base"

        if [ ! -f "$expected_file" ]; then
            skip "$test_name (no .expected file)"
            continue
        fi

        local cmd
        cmd=$(cat "$cmd_file")
        local expected
        expected=$(cat "$expected_file")

        # Replace "finetype" with actual binary path
        local full_cmd
        full_cmd=$(echo "$cmd" | sed "s|^finetype|$FINETYPE|")

        local actual
        actual=$(eval "$full_cmd" 2>/dev/null) || true

        # Strip trailing whitespace
        actual=$(echo "$actual" | sed 's/[[:space:]]*$//')
        expected=$(echo "$expected" | sed 's/[[:space:]]*$//')

        # Detect JSON vs plain
        if echo "$expected" | head -1 | grep -q '^\s*{'; then
            assert_json_subset "$test_name" "$actual" "$expected"
        else
            assert_eq "$test_name" "$actual" "$expected"
        fi
    done

    if [ "$found" -eq 0 ]; then
        printf "  (no .cmd files in golden/)\n"
    fi
}

# ═══════════════════════════════════════════════════════════════════════════════
# MAIN
# ═══════════════════════════════════════════════════════════════════════════════

if [ "$GOLDEN_ONLY" = false ]; then
    # Run doc tests from all markdown files in docs/
    for md_file in "$REPO_ROOT"/docs/*.md; do
        [ -f "$md_file" ] || continue
        run_doc_tests "$md_file"
    done
fi

section "Golden file tests"
run_golden_tests

# ═══════════════════════════════════════════════════════════════════════════════
# SUMMARY (always exit 0 — these are informational)
# ═══════════════════════════════════════════════════════════════════════════════

TOTAL=$((PASS + FAIL + SKIP))
if [ "$TOTAL" -gt 0 ]; then
    PARITY=$(( (PASS * 100) / TOTAL ))
else
    PARITY=0
fi

print_summary "Doc Test Results"
printf "  Parity: %d%%\n" "$PARITY"

# Always exit 0 — these tests track convergence, not block CI
exit 0
