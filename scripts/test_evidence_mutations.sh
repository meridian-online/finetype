#!/usr/bin/env bash
# test_evidence_mutations.sh — does scripts/test_evidence.sh actually detect a wrong
# evidence.py, or does it pass on anything that runs?
#
# A test that never reddens is not a test, it is a structural guard: it asserts the
# SHAPE of an answer near an invariant and holds by accident of the fixture. The way to
# tell the two apart is to write the wrong implementation a competent author would
# plausibly write, and require a NAMED case to fail on it.
#
# Each mutation below is a realistic wrong evidence.py — not a syntax error, not a
# deleted function, but a version that runs, renders a report, and states something it
# did not measure. For each one this script asserts:
#
#   1. test_evidence.sh exits non-zero, and
#   2. at least one of the case names the mutation is aimed at appears in the FAIL lines.
#
# (2) is the part that matters. A mutation that reddens some unrelated case proves the
# suite is noisy, not that it is watching the thing this mutation broke.
#
# Usage:  scripts/test_evidence_mutations.sh
# Exit:   0 every mutation was caught by a case aimed at it · 1 a mutation survived
set -uo pipefail
cd "$(cd "$(dirname "$0")/.." && pwd)"

TMP="$(mktemp -d)"
SRC="scripts/evidence.py"
# The mutant has to sit in scripts/ beside the real tool: evidence.py derives REPO_ROOT
# from Path(__file__).parent.parent, so a copy anywhere else resolves the default
# manifest to a directory that does not exist and every case fails for that reason
# instead of for the mutation. Leading dot keeps it out of glob-based tooling; the trap
# removes it on any exit path.
MUTANT="scripts/.evidence-mutant.tmp.py"
trap 'rm -rf "$TMP"; rm -f "$MUTANT"' EXIT
PASS=0
FAIL=0

# apply <python-patch-file> — write a mutant to $MUTANT
apply() {
  cp "$SRC" "$MUTANT"
  python3 "$1" "$MUTANT" || { echo "  patch failed to apply"; return 1; }
  chmod +x "$MUTANT"
}

# expect_caught <mutation name> <case-name-substring>...
expect_caught() {
  local name="$1"; shift
  local log="$TMP/run.log" rc=0
  FINETYPE_EVIDENCE_BIN="$MUTANT" ./scripts/test_evidence.sh >"$log" 2>&1 || rc=$?
  if [ "$rc" -eq 0 ]; then
    FAIL=$((FAIL + 1))
    printf '  SURVIVED  %s\n            the suite passed on a wrong implementation\n' "$name"
    return
  fi
  local fails
  fails="$(grep -E '^  FAIL ' "$log" || true)"
  local hit=""
  for want in "$@"; do
    if printf '%s' "$fails" | grep -qF -- "$want"; then hit="$want"; break; fi
  done
  if [ -n "$hit" ]; then
    PASS=$((PASS + 1))
    printf '  caught    %s\n            by: %s\n' "$name" "$hit"
  else
    FAIL=$((FAIL + 1))
    printf '  MISDIRECT %s\n            reddened, but not by a case aimed at it. FAILs were:\n%s\n' \
      "$name" "$fails"
  fi
}

echo "== realistic wrong implementations of evidence.py =="

# ---------------------------------------------------------------------------
# 1. The renderer goes back to asserting a blanket "not captured at the time" for
#    every release, ignoring any stamp the manifest carries. This is the state the
#    file was in before this release, and it is the most likely regression: the
#    prose reads plausibly and the report still renders.
cat > "$TMP/m1.py" <<'PY'
import sys, pathlib
p = pathlib.Path(sys.argv[1]); s = p.read_text()
old = '    stamped = [(fid, key, b) for fid, _fx, key, b in rows if b.get("taxonomy_measured")]'
new = '    stamped = []'
assert old in s, "anchor 1 missing"
p.write_text(s.replace(old, new))
PY
apply "$TMP/m1.py" && expect_caught \
  "renderer ignores the recorded taxonomy stamp" \
  "a stamped baseline states its taxonomy" \
  "a stamped baseline is not also listed as unstamped" \
  "a mixed manifest names which scores carry a stamp"

# ---------------------------------------------------------------------------
# 2. The renderer claims the stamp for every score as soon as ONE score has it —
#    the flattening a "take the first value" implementation produces. The report
#    then attributes a taxonomy to scores that never recorded one.
cat > "$TMP/m2.py" <<'PY'
import sys, pathlib
p = pathlib.Path(sys.argv[1]); s = p.read_text()
old = '    unstamped = [(fid, key, b) for fid, _fx, key, b in rows if not b.get("taxonomy_measured")]'
new = '    unstamped = []'
assert old in s, "anchor 2 missing"
p.write_text(s.replace(old, new))
PY
apply "$TMP/m2.py" && expect_caught \
  "renderer suppresses the unstamped list" \
  "an unstamped baseline is named as unstamped" \
  "a mixed manifest names which scores carry a stamp"

# ---------------------------------------------------------------------------
# 3. record-baseline accepts --taxonomy and silently drops it. The command exits 0
#    and prints the same success line, so nothing at the call site notices; the
#    stamp simply never reaches the manifest.
cat > "$TMP/m3.py" <<'PY'
import sys, pathlib
p = pathlib.Path(sys.argv[1]); s = p.read_text()
old = 'BASELINE_FIELDS = ("model", "binary", "pipeline", "source", "note", "taxonomy_measured")'
new = 'BASELINE_FIELDS = ("model", "binary", "pipeline", "source", "note")\nBASELINE_FIELDS_ALL = ("model", "binary", "pipeline", "source", "note", "taxonomy_measured")'
assert old in s, "anchor 3 missing"
s = s.replace(old, new)
# the parser still offers the flag, so the CLI surface is unchanged
s = s.replace(
    "    for field in BASELINE_FIELDS:\n        p.add_argument(BASELINE_FLAGS.get(field, f\"--{field}\"), dest=field, default=\"\")",
    "    for field in BASELINE_FIELDS_ALL:\n        p.add_argument(BASELINE_FLAGS.get(field, f\"--{field}\"), dest=field, default=\"\")",
)
p.write_text(s)
PY
apply "$TMP/m3.py" && expect_caught \
  "record-baseline drops --taxonomy on the floor" \
  "a stamped baseline states its taxonomy" \
  "a mixed manifest names which scores carry a stamp"

# ---------------------------------------------------------------------------
# 4. The delta table loses its pipeline column. Every number in it stays correct —
#    this is a cosmetic-looking change — but two rows of one fixture, composed and
#    Sense, become indistinguishable, and a reader attributes the wrong score to
#    the wrong path.
cat > "$TMP/m4.py" <<'PY'
import sys, pathlib
p = pathlib.Path(sys.argv[1]); s = p.read_text()
old = '''        w("| Fixture version | Pipeline | Against | Then | Now | Δ columns | Δ accuracy |")
        w("|---|---|---|---:|---:|---:|---:|")'''
new = '''        w("| Fixture version | Against | Then | Now | Δ columns | Δ accuracy |")
        w("|---|---|---:|---:|---:|---:|")'''
assert old in s, "anchor 4 missing"
s = s.replace(old, new)
old2 = '''                f"| `{fid}` | {b.get('pipeline', '—')} | `{o.get('binary', '—')}` | "
                f"{o['correct']}/{o['scored']} = "'''
new2 = '''                f"| `{fid}` | `{o.get('binary', '—')}` | "
                f"{o['correct']}/{o['scored']} = "'''
assert old2 in s, "anchor 4b missing"
p.write_text(s.replace(old2, new2))
PY
apply "$TMP/m4.py" && expect_caught \
  "delta table drops the pipeline column" \
  "same-fixture delta is still offered"

# ---------------------------------------------------------------------------
# 5. `render-release --check` short-circuits its diff and always reports ok. This is
#    the CLI path CI and the release runbook gate on, and it is a SEPARATE comparison
#    from the one inside verify — so breaking it leaves verify green and every
#    --check caller gating on nothing. This mutation survived the suite until the
#    --check case below stopped delegating its verdict to the thing it was testing.
cat > "$TMP/m5.py" <<'PY'
import sys, pathlib
p = pathlib.Path(sys.argv[1]); s = p.read_text()
old = 'if out.read_text(encoding="utf-8") != text:'
new = 'if False:'
assert old in s, "anchor 5 missing"
p.write_text(s.replace(old, new))
PY
apply "$TMP/m5.py" && expect_caught \
  "render-release --check stops diffing and always says ok" \
  "--check reddens on a drifted report, not just verify"

# ---------------------------------------------------------------------------
# 6. The OTHER comparison: verify stops re-rendering committed reports and only
#    checks they exist. A report and its manifest can then disagree indefinitely,
#    which is the exact failure evidence/ was built to prevent. Kept separate from
#    mutation 5 because the two comparisons are separate code, and a suite that
#    catches one is not thereby watching the other.
cat > "$TMP/m6.py" <<'PY'
import sys, pathlib
p = pathlib.Path(sys.argv[1]); s = p.read_text()
old = '        elif out.read_text(encoding="utf-8") != render_release(doc, binary):'
new = '        elif False:'
assert old in s, "anchor 6 missing"
p.write_text(s.replace(old, new))
PY
apply "$TMP/m6.py" && expect_caught \
  "verify stops diffing a report against the manifest" \
  "a hand-edited number in the report" \
  "a manifest number changed without re-rendering"

printf '\n%s caught, %s survived-or-misdirected\n' "$PASS" "$FAIL"
[ "$FAIL" -eq 0 ]
