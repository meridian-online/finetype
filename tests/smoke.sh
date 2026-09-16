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

# ── A 100-value SAMPLE of `naics_description.csv` that widened even though the
# whole file does not: a candidate sniff confirmed itself by reading the row
# its own `SkipRows` lands on, and on this exact window that row is the file's
# own last line, which happens to split into three under `;`. This is window 2
# of seed `20260828`, drawn the way `scripts/check_label_stability.py` draws it
# (see `tests/fixtures/label_stability/BASELINE.md`), persisted here — OUTSIDE
# `tests/fixtures/label_stability/`, whose every CSV `check_label_stability.py`
# requires a baseline row for — so the defect it exposed stays under CI rather
# than only under an occasional draw.
NAICS_W2="$REPO_ROOT/tests/fixtures/naics_description_window2.csv"
NAICS_W2_ERR=$("$FINETYPE" profile -f "$NAICS_W2" -o json 2>&1 >/dev/null) || true
assert_contains "a 100-value sample that used to widen profiles as one column" \
    "$NAICS_W2_ERR" 'Found 1 columns: ["description"]'
assert_contains "and keeps all 100 of its rows, not the one a skipped read would leave" \
    "$NAICS_W2_ERR" 'Read 100 rows'


# ── Error Handling ────────────────────────────────────────────────────────────

section "11. Error Handling"

# Missing subcommand should show help (non-zero exit is OK)
OUT=$("$FINETYPE" 2>&1) || true
assert_contains "no subcommand shows usage" "$OUT" "Usage"

# Invalid subcommand
OUT=$("$FINETYPE" nonexistent 2>&1) || true
assert_contains "invalid subcommand shows error" "$OUT" "error"

# ── Nominations ──────────────────────────────────────────────────────────────
#
# A nomination declares what a column IS. Everything below runs the release
# binary end to end, because the flag reaching the binary and the descriptor it
# writes are exactly the parts a unit test over the emitter cannot see.

section "12. Nominations"

NOM_CSV="$REPO_ROOT/tests/fixtures/nominated_corpus.csv"
NOM_FILE="$REPO_ROOT/tests/fixtures/nominated_corpus.nominations.finetype.json"
NOMDIR=$(mktemp -d /tmp/finetype-smoke-nominations-XXXXXX)

# The fixture carries 160 data rows on purpose. A file short enough for a
# hundred-row sniff to widen would make its column count a property of the read
# path rather than of the file, and every assertion below would then be about
# the sniff.
NOM_ROWS=$(( $(wc -l < "$NOM_CSV") - 1 ))
if [ "$NOM_ROWS" -gt 100 ]; then
    pass "the nominations fixture is clear of the sniff window ($NOM_ROWS rows)"
else
    fail "the nominations fixture is clear of the sniff window" "only $NOM_ROWS rows"
fi

HELP_OUTPUT=$("$FINETYPE" profile --help 2>&1)
# `nominations <FILE>` rather than `--nominations`: assert_contains passes its
# needle to `grep -qF` without a `--` terminator, so a needle beginning with a
# dash is read as an option and the assertion fails for a reason that has
# nothing to do with the binary.
assert_contains "profile --help lists the nominations flag" "$HELP_OUTPUT" "nominations <FILE>"

# One field of the emitted descriptor, as sorted JSON.
dp_field() {  # <descriptor> <field name>
    printf '%s' "$1" | python3 -c '
import json, sys
fields = json.load(sys.stdin)["resources"][0]["schema"]["fields"]
for f in fields:
    if f["name"] == sys.argv[1]:
        print(json.dumps(f, sort_keys=True))
        break
else:
    print("{}")
' "$2"
}

NOM_DP=$("$FINETYPE" profile -f "$NOM_CSV" --nominations "$NOM_FILE" -o datapackage 2>/dev/null)
BASE_JSON=$("$FINETYPE" profile -f "$NOM_CSV" -o json 2>/dev/null)

# The four things a nominated field publishes, and the one it must not.
NOM_CORPUS=$(dp_field "$NOM_DP" corpus)
GOT=$(printf '%s' "$NOM_CORPUS" | python3 -c '
import json, sys
f = json.load(sys.stdin)
print("type=%s label=%s nominated=%s constraints=%s confidence=%s" % (
    f.get("type"),
    f.get("x-finetype-label"),
    f.get("x-finetype-nominated"),
    json.dumps(f.get("constraints"), sort_keys=True),
    "present" if "x-finetype-confidence" in f else "absent",
))')
assert_eq "a nominated field publishes its label, its bounds and no confidence" \
    "$GOT" \
    'type=string label=representation.text.plain_text nominated=True constraints={"maxLength": 65536, "minLength": 1} confidence=absent'

# An undeclared column in the same run is inferred, and still publishes one.
GOT=$(dp_field "$NOM_DP" record_id | python3 -c '
import json, sys
f = json.load(sys.stdin)
print("nominated=%s confidence=%s" % (
    f.get("x-finetype-nominated", False),
    "present" if "x-finetype-confidence" in f else "absent"))')
assert_eq "an undeclared column in the same run is still inferred" "$GOT" "nominated=False confidence=present"

# THE ASSERTION THAT CAN TELL A DECLARATION FROM A GUESS.
#
# Inference types this fixture's `corpus` column as `representation.text.plain_text`
# on its own — it is prose, and that is the right answer. So the assertion above,
# which is the acceptance criterion as written, would pass just as well against a
# build that ignored the nomination and published the guess. It pins what a
# nominated field CARRIES; it cannot pin that the nomination was USED.
#
# `record_id` is where that becomes decidable: inference types it as an
# identifier, so declaring it as text asks for a label inference does not give.
# The premise is measured in the same breath rather than assumed.
cat > "$NOMDIR/override.json" <<'JSONEOF'
{"resources": {"nominated_corpus": {"record_id": {"label": "representation.text.plain_text"}}}}
JSONEOF
INFERRED_ID=$(printf '%s' "$BASE_JSON" | python3 -c '
import json, sys
cols = {c["column"]: c for c in json.load(sys.stdin)["columns"]}
print(cols["record_id"]["type"])')
if [ "$INFERRED_ID" != "representation.text.plain_text" ]; then
    pass "the premise holds — inference calls record_id '$INFERRED_ID', not text"
else
    fail "the premise holds — inference does not already call record_id text" \
        "inference gave 'representation.text.plain_text', so the next assertion proves nothing"
fi
OVERRIDE_JSON=$("$FINETYPE" profile -f "$NOM_CSV" --nominations "$NOMDIR/override.json" -o json 2>/dev/null)
GOT=$(printf '%s' "$OVERRIDE_JSON" | python3 -c '
import json, sys
cols = {c["column"]: c for c in json.load(sys.stdin)["columns"]}
print("%s nominated=%s" % (cols["record_id"]["type"], cols["record_id"].get("nominated")))')
assert_eq "the declared label is published, not the one inference would give" "$GOT" \
    "representation.text.plain_text nominated=True"

OVERRIDE_DP=$("$FINETYPE" profile -f "$NOM_CSV" --nominations "$NOMDIR/override.json" -o datapackage 2>/dev/null)
GOT=$(dp_field "$OVERRIDE_DP" record_id | python3 -c '
import json, sys
f = json.load(sys.stdin)
print("%s bounds=%s" % (f.get("x-finetype-label"), json.dumps(f.get("constraints"), sort_keys=True)))')
assert_eq "the descriptor carries the declared label and its bounds" "$GOT" \
    'representation.text.plain_text bounds={"maxLength": 65536, "minLength": 1}'

# The declared type survives to `plain` and `json` too.
NOM_PLAIN=$("$FINETYPE" profile -f "$NOM_CSV" --nominations "$NOM_FILE" -o plain 2>/dev/null)
CORPUS_ROW=$(printf '%s\n' "$NOM_PLAIN" | grep '^  corpus ' || true)
assert_contains "plain marks the nominated row CONF as decl" "$CORPUS_ROW" "decl"
assert_contains "plain gives the nominated row its declared label" "$CORPUS_ROW" "representation.text.plain_text"

NOM_JSON=$("$FINETYPE" profile -f "$NOM_CSV" --nominations "$NOM_FILE" -o json 2>/dev/null)
GOT=$(printf '%s' "$NOM_JSON" | python3 -c '
import json, sys
cols = {c["column"]: c for c in json.load(sys.stdin)["columns"]}
c = cols["corpus"]
dropped = [k for k in ("confidence", "quality_band", "runner_up", "validation_pass_rate",
                       "validation_vetoed", "vetoed_type", "validation_advisory_low") if k in c]
print("type=%s nominated=%s leaked=%s" % (c["type"], c.get("nominated"), ",".join(dropped) or "none"))')
assert_eq "json marks the nominated column and drops the classifier answer" "$GOT" \
    "type=representation.text.plain_text nominated=True leaked=none"

# `-o json-schema` carries the marker beside the label.
NOM_JS=$("$FINETYPE" profile -f "$NOM_CSV" --nominations "$NOM_FILE" -o json-schema 2>/dev/null)
GOT=$(printf '%s' "$NOM_JS" | python3 -c '
import json, sys
props = json.load(sys.stdin)["properties"]
print("label=%s nominated=%s other=%s" % (
    props["corpus"]["x-finetype-label"],
    props["corpus"].get("x-finetype-nominated"),
    props["record_id"].get("x-finetype-nominated", False)))')
assert_eq "json-schema marks the nominated property and only that one" "$GOT" \
    "label=representation.text.plain_text nominated=True other=False"

# Nominating one column changes that column's field object and nothing else.
# `$.created` comes off `Utc::now()`, so two runs a second apart differ there
# with no nomination involved.
BASE_DP=$("$FINETYPE" profile -f "$NOM_CSV" -o datapackage 2>/dev/null)
printf '%s' "$NOM_DP" > "$NOMDIR/with.json"
printf '%s' "$BASE_DP" > "$NOMDIR/without.json"
GOT=$(python3 - "$NOMDIR/with.json" "$NOMDIR/without.json" <<'PYEOF'
import json, sys

def load(p):
    d = json.load(open(p))
    d.pop("created", None)
    return d

a, b = load(sys.argv[1]), load(sys.argv[2])
nominated = {"corpus"}

def strip(d):
    d = json.loads(json.dumps(d))
    d["resources"][0]["schema"]["fields"] = [
        f for f in d["resources"][0]["schema"]["fields"] if f["name"] not in nominated
    ]
    return d

outside = "same" if strip(a) == strip(b) else "differs"

def field(d, name):
    return next(f for f in d["resources"][0]["schema"]["fields"] if f["name"] == name)

inside = "differs" if field(a, "corpus") != field(b, "corpus") else "same"
print("outside=%s inside=%s" % (outside, inside))
PYEOF
)
assert_eq "nominating a column changes its own field object and nothing else" "$GOT" \
    "outside=same inside=differs"

# ── Refusals. Every one of these stops the run; a nomination that is silently
#    dropped is an inference wearing a declaration's marker.

# A label the taxonomy does not carry.
cat > "$NOMDIR/unknown-label.json" <<'JSONEOF'
{"resources": {"nominated_corpus": {"corpus": {"label": "representation.text.plain_txet"}}}}
JSONEOF
OUT=$("$FINETYPE" profile -f "$NOM_CSV" --nominations "$NOMDIR/unknown-label.json" -o datapackage 2>&1) && \
    fail "an unknown nominated label is refused" "the run exited 0" || \
    pass "an unknown nominated label is refused"
assert_contains "the refusal names the unknown label" "$OUT" "representation.text.plain_txet"
assert_contains "the refusal names the column" "$OUT" "corpus"

# The one label of 251 whose Frictionless type the v2 profile does not admit.
cat > "$NOMDIR/list-type.json" <<'JSONEOF'
{"resources": {"nominated_corpus": {"corpus": {"label": "container.array.comma_separated"}}}}
JSONEOF
OUT=$("$FINETYPE" profile -f "$NOM_CSV" --nominations "$NOMDIR/list-type.json" -o datapackage 2>&1) && \
    fail "a nominated type the v2 profile does not admit is refused" "the run exited 0" || \
    pass "a nominated type the v2 profile does not admit is refused"
assert_contains "the refusal names the type the profile rejects" "$OUT" "list"

# …while an INFERRED list still emits. The asymmetry is deliberate: an inferred
# answer is about data the caller cannot change mid-run, a nomination is a
# declaration made before any work starts.
OUT=$("$FINETYPE" profile -f "$NOM_CSV" -o datapackage 2>&1) && \
    pass "the same file with no nomination still profiles" || \
    fail "the same file with no nomination still profiles" "$OUT"

# An unknown key, at the level a typo actually lands on.
cat > "$NOMDIR/typo.json" <<'JSONEOF'
{"resources": {"nominated_corpus": {"corpus": {"lable": "representation.text.plain_text"}}}}
JSONEOF
OUT=$("$FINETYPE" profile -f "$NOM_CSV" --nominations "$NOMDIR/typo.json" -o datapackage 2>&1) && \
    fail "an unknown key in a nomination is refused" "the run exited 0" || \
    pass "an unknown key in a nomination is refused"
assert_contains "the refusal names the offending key" "$OUT" "lable"
assert_contains "the refusal names the JSON path to it" "$OUT" '$.resources["nominated_corpus"]["corpus"]'

# A column object with no `label`.
cat > "$NOMDIR/no-label.json" <<'JSONEOF'
{"resources": {"nominated_corpus": {"corpus": {"why": "I forgot the label"}}}}
JSONEOF
OUT=$("$FINETYPE" profile -f "$NOM_CSV" --nominations "$NOMDIR/no-label.json" -o datapackage 2>&1) && \
    fail "a nomination with no label is refused" "the run exited 0" || \
    pass "a nomination with no label is refused"
assert_contains "the refusal names the missing key" "$OUT" "label"

# A declared column the file does not have — the renamed-column case.
cat > "$NOMDIR/absent-column.json" <<'JSONEOF'
{"resources": {"nominated_corpus": {"korpus": {"label": "representation.text.plain_text"}}}}
JSONEOF
OUT=$("$FINETYPE" profile -f "$NOM_CSV" --nominations "$NOMDIR/absent-column.json" -o datapackage 2>&1) && \
    fail "a nomination naming an absent column is refused" "the run exited 0" || \
    pass "a nomination naming an absent column is refused"
assert_contains "the refusal names the absent column" "$OUT" "korpus"
assert_contains "the refusal names the stem" "$OUT" "nominated_corpus"
assert_contains "the refusal names the file" "$OUT" "nominated_corpus.csv"

# A declared stem no input matches.
cat > "$NOMDIR/absent-stem.json" <<'JSONEOF'
{"resources": {"no_such_file": {"corpus": {"label": "representation.text.plain_text"}}}}
JSONEOF
OUT=$("$FINETYPE" profile -f "$NOM_CSV" --nominations "$NOMDIR/absent-stem.json" -o datapackage 2>&1) && \
    fail "a nomination naming an absent stem is refused" "the run exited 0" || \
    pass "a nomination naming an absent stem is refused"
assert_contains "the refusal names the absent stem" "$OUT" "no_such_file"

rm -rf "$NOMDIR"

# ═══════════════════════════════════════════════════════════════════════════════
# SUMMARY
# ═══════════════════════════════════════════════════════════════════════════════

print_summary "Results"

if [ "$FAIL" -gt 0 ]; then
    exit 1
fi

printf "\n\033[32mAll smoke tests passed.\033[0m\n"
