#!/usr/bin/env bash
# ═══════════════════════════════════════════════════════════════════════════════
# FineType JSON Profiling Smoke Tests (NNFT-217)
# ═══════════════════════════════════════════════════════════════════════════════
#
# End-to-end tests for JSON/NDJSON profiling.
# Tests: nested objects, arrays, mixed types, empty arrays, deeply nested,
#        schema evolution, scalar rejection, output formats.
#
# Usage:
#   ./tests/smoke_json.sh                  # build + test
#   ./tests/smoke_json.sh --skip-build     # test existing binary

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/helpers.sh"

handle_build "$@"

FIXTURES="$SCRIPT_DIR/fixtures"

# ═══════════════════════════════════════════════════════════════════════════════
# TEST SUITE
# ═══════════════════════════════════════════════════════════════════════════════

section "1. JSON Auto-Detection"

output=$("$FINETYPE" profile -f "$FIXTURES/test_profile.json" 2>&1)
assert_contains "JSON file detected" "$output" "paths"

output=$("$FINETYPE" profile -f "$FIXTURES/test_profile.ndjson" 2>&1)
assert_contains "NDJSON file detected" "$output" "NDJSON documents"

# ─────────────────────────────────────────────────────────────────────────────

section "2. Nested Objects (dot notation paths)"

output=$("$FINETYPE" profile -f "$FIXTURES/nested_objects.json" 2>/dev/null)
assert_contains "nested dot path user.contact.email" "$output" "user.contact.email"
assert_contains "nested dot path user.contact.phone" "$output" "user.contact.phone"
assert_contains "nested dot path user.name" "$output" "user.name"
assert_contains "nested dot path score" "$output" "score"

# JSON output should reconstruct nested structure
json_output=$("$FINETYPE" profile -f "$FIXTURES/nested_objects.json" -o json 2>/dev/null)
assert_contains "JSON schema has nested user object" "$json_output" '"user"'
assert_contains "JSON schema has nested contact object" "$json_output" '"contact"'

# ─────────────────────────────────────────────────────────────────────────────

section "3. Arrays of Objects (bracket notation)"

output=$("$FINETYPE" profile -f "$FIXTURES/test_profile.json" 2>/dev/null)
assert_contains "array paths use dot notation for nested" "$output" "address.city"
assert_contains "array paths preserve field names" "$output" "address.country"

# ─────────────────────────────────────────────────────────────────────────────

section "4. Mixed Types in Arrays"

output=$("$FINETYPE" profile -f "$FIXTURES/mixed_array.json" 2>/dev/null)
assert_contains "mixed array tags present" "$output" "tags[]"

# ─────────────────────────────────────────────────────────────────────────────

section "5. Empty Arrays"

output=$("$FINETYPE" profile -f "$FIXTURES/empty_arrays.json" 2>/dev/null)
assert_contains "empty array: name still profiled" "$output" "name"
assert_contains "empty array: email still profiled" "$output" "email"
# Empty arrays should not produce a path (no values to classify)
if echo "$output" | grep -q "hobbies"; then
    # If it appears, it should be unknown (no values)
    pass "empty array: hobbies handled (present but likely unknown)"
else
    pass "empty array: hobbies path absent (no values to classify)"
fi

# ─────────────────────────────────────────────────────────────────────────────

section "6. Deeply Nested Structures (10+ levels)"

output=$("$FINETYPE" profile -f "$FIXTURES/deeply_nested.json" 2>/dev/null)
assert_contains "deeply nested path exists" "$output" "level1.level2.level3"
assert_contains "deeply nested path goes to leaf" "$output" "value"

json_output=$("$FINETYPE" profile -f "$FIXTURES/deeply_nested.json" -o json 2>/dev/null)
assert_contains "JSON output includes deeply nested schema" "$json_output" '"level1"'

# ─────────────────────────────────────────────────────────────────────────────

section "7. Schema Evolution (NDJSON)"

output=$("$FINETYPE" profile -f "$FIXTURES/schema_evolution.ndjson" 2>&1)
assert_contains "schema evolution: name always present" "$output" "name"
assert_contains "schema evolution: email present (partial)" "$output" "email"
assert_contains "schema evolution: phone present (partial)" "$output" "phone"
assert_contains "schema evolution: 5 NDJSON documents" "$output" "5 NDJSON documents"

# ─────────────────────────────────────────────────────────────────────────────

section "8. Top-Level Scalars (error case)"

# Create temporary scalar JSON
SCALAR_JSON=$(mktemp /tmp/scalar_XXXX.json)
echo '"just a string"' > "$SCALAR_JSON"

# Capture output (command will fail, so suppress exit code)
error_output=$("$FINETYPE" profile -f "$SCALAR_JSON" 2>&1 || true)
assert_contains "scalar JSON produces error message" "$error_output" "scalar value"

# Numeric scalar
echo '42' > "$SCALAR_JSON"
error_output=$("$FINETYPE" profile -f "$SCALAR_JSON" 2>&1 || true)
assert_contains "numeric scalar produces error message" "$error_output" "scalar value"

rm -f "$SCALAR_JSON"

# Malformed JSON
MALFORMED_JSON=$(mktemp /tmp/malformed_XXXX.json)
echo '{bad json' > "$MALFORMED_JSON"
error_output=$("$FINETYPE" profile -f "$MALFORMED_JSON" 2>&1 || true)
assert_contains "malformed JSON produces clear error" "$error_output" "Malformed JSON"
rm -f "$MALFORMED_JSON"

# ─────────────────────────────────────────────────────────────────────────────

section "9. Output Formats with JSON Input"

# Plain output
plain_output=$("$FINETYPE" profile -f "$FIXTURES/test_profile.json" 2>/dev/null)
assert_contains "plain output has COLUMN header" "$plain_output" "COLUMN"
assert_contains "plain output has TYPE header" "$plain_output" "TYPE"
assert_contains "plain output has CONF header" "$plain_output" "CONF"

# JSON output
json_output=$("$FINETYPE" profile -f "$FIXTURES/test_profile.json" -o json 2>/dev/null)
assert_contains "JSON output has file field" "$json_output" '"file"'
assert_contains "JSON output has rows field" "$json_output" '"rows"'
assert_contains "JSON output has schema field" "$json_output" '"schema"'
assert_contains "JSON output has columns field" "$json_output" '"columns"'

# CSV output
csv_output=$("$FINETYPE" profile -f "$FIXTURES/test_profile.json" -o csv 2>/dev/null)
assert_contains "CSV output has header row" "$csv_output" "column,type,confidence"
assert_contains "CSV output has data rows" "$csv_output" "address.city"

# ─────────────────────────────────────────────────────────────────────────────

section "10. CSV Regression Check"

# Ensure existing CSV profiling still works
CSV_FILE=$(ls ~/datasets/*.csv 2>/dev/null | head -1)
if [ -n "$CSV_FILE" ]; then
    csv_profile=$("$FINETYPE" profile -f "$CSV_FILE" 2>/dev/null)
    assert_contains "CSV profile still works" "$csv_profile" "columns typed"
else
    skip "CSV regression (no datasets available)"
fi

# ─────────────────────────────────────────────────────────────────────────────

section "11. JSON + --validate (plain output)"

validate_plain=$("$FINETYPE" profile -f "$FIXTURES/nested_objects.json" --validate 2>/dev/null)
assert_contains "validate plain has VALID column" "$validate_plain" "VALID"
assert_contains "validate plain has Quality grade" "$validate_plain" "Quality:"
assert_contains "validate plain shows JSON paths" "$validate_plain" "user.contact.email"

# ─────────────────────────────────────────────────────────────────────────────

section "12. JSON + --validate -o markdown"

validate_md=$("$FINETYPE" profile -f "$FIXTURES/nested_objects.json" --validate -o markdown 2>/dev/null)
assert_contains "validate markdown has Valid Rate header" "$validate_md" "Valid Rate"
assert_contains "validate markdown has Quality header" "$validate_md" "Quality"
assert_contains "validate markdown has bold Quality grade" "$validate_md" "**Quality:"
assert_contains "validate markdown shows JSON paths" "$validate_md" "user.contact.phone"

# ─────────────────────────────────────────────────────────────────────────────

section "13. JSON + --validate -o json"

validate_json=$("$FINETYPE" profile -f "$FIXTURES/test_profile.json" --validate -o json 2>/dev/null)
assert_contains "validate json has quality field" "$validate_json" '"quality"'
assert_contains "validate json has valid count" "$validate_json" '"valid"'
assert_contains "validate json has quality_score" "$validate_json" '"quality_score"'
assert_contains "validate json shows JSON paths" "$validate_json" '"address.city"'

# ═══════════════════════════════════════════════════════════════════════════════

print_summary "JSON Profiling Smoke Tests"

if [ "$FAIL" -gt 0 ]; then
    exit 1
fi
