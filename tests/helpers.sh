#!/usr/bin/env bash
# ═══════════════════════════════════════════════════════════════════════════════
# FineType CLI Test Helpers
# ═══════════════════════════════════════════════════════════════════════════════
#
# Shared functions for all CLI test scripts.
# Source this file: source "$(dirname "${BASH_SOURCE[0]}")/helpers.sh"

# ── Configuration ─────────────────────────────────────────────────────────────

HELPERS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$HELPERS_DIR/.." && pwd)"

PASS=0
FAIL=0
SKIP=0
ERRORS=()

# Allow override via env var
FINETYPE="${FINETYPE:-$REPO_ROOT/target/release/finetype}"

# ── Helpers ───────────────────────────────────────────────────────────────────

pass() {
    PASS=$((PASS + 1))
    printf "  \033[32m✓\033[0m %s\n" "$1"
}

fail() {
    FAIL=$((FAIL + 1))
    ERRORS+=("$1: $2")
    printf "  \033[31m✗\033[0m %s — %s\n" "$1" "$2"
}

skip() {
    SKIP=$((SKIP + 1))
    printf "  \033[33m○\033[0m %s (skipped)\n" "$1"
}

section() {
    printf "\n\033[1m%s\033[0m\n" "$1"
}

# Assert output equals expected
assert_eq() {
    local name="$1" actual="$2" expected="$3"
    if [ "$actual" = "$expected" ]; then
        pass "$name"
    else
        fail "$name" "expected '$expected', got '$actual'"
    fi
}

# Assert output contains expected substring
assert_contains() {
    local name="$1" actual="$2" expected="$3"
    if echo "$actual" | grep -qF "$expected"; then
        pass "$name"
    else
        fail "$name" "expected output to contain '$expected', got '$actual'"
    fi
}

# Assert command succeeds (exit code 0)
assert_ok() {
    local name="$1"
    shift
    if output=$("$@" 2>&1); then
        pass "$name"
        echo "$output"
    else
        fail "$name" "command failed with exit $?: $output"
        echo ""
    fi
}

# Assert command fails (non-zero exit)
assert_fail() {
    local name="$1"
    shift
    if output=$("$@" 2>&1); then
        fail "$name" "expected failure but command succeeded: $output"
    else
        pass "$name"
    fi
}

# Assert that expected JSON is a subset of actual JSON (every key in expected
# exists in actual with the same value). Requires jq.
assert_json_subset() {
    local name="$1" actual_json="$2" expected_json="$3"

    # Validate both are parseable JSON
    if ! echo "$actual_json" | jq empty 2>/dev/null; then
        fail "$name" "actual output is not valid JSON: $actual_json"
        return
    fi
    if ! echo "$expected_json" | jq empty 2>/dev/null; then
        fail "$name" "expected output is not valid JSON: $expected_json"
        return
    fi

    local missing
    missing=$(jq -n --argjson exp "$expected_json" --argjson act "$actual_json" \
        '[($exp | to_entries[]) | select(.value != ($act[.key] // null))] | length')

    if [ "$missing" = "0" ]; then
        pass "$name"
    else
        local diff
        diff=$(jq -n --argjson exp "$expected_json" --argjson act "$actual_json" \
            '[($exp | to_entries[]) | select(.value != ($act[.key] // null)) | {key, expected: .value, actual: ($act[.key] // "MISSING")}]')
        fail "$name" "mismatched fields: $diff"
    fi
}

# ── Build Helper ──────────────────────────────────────────────────────────────

# Call: handle_build "$@"
# Handles --skip-build flag and verifies binary exists.
handle_build() {
    if [[ "${1:-}" != "--skip-build" ]]; then
        section "Building release binary..."
        (cd "$REPO_ROOT" && cargo build --release -p finetype-cli 2>&1)
        printf "  Binary: %s\n" "$FINETYPE"
    fi

    if [ ! -x "$FINETYPE" ]; then
        printf "\033[31mERROR: Binary not found at %s\033[0m\n" "$FINETYPE"
        exit 1
    fi
}

# ── Summary ───────────────────────────────────────────────────────────────────

print_summary() {
    local label="${1:-Results}"
    section "$label"
    local total=$((PASS + FAIL + SKIP))
    printf "  %d passed, %d failed, %d skipped (of %d)\n" "$PASS" "$FAIL" "$SKIP" "$total"

    if [ "$FAIL" -gt 0 ]; then
        printf "\n\033[31mFailures:\033[0m\n"
        for err in "${ERRORS[@]}"; do
            printf "  - %s\n" "$err"
        done
    fi
}
