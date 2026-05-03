#!/usr/bin/env bash
# GitTables holdout round-trip gate measurement (entry point).
#
# Wraps scripts/gittables_gate.py so the cron preamble can invoke it
# without knowing the script's location or default flag values. Emits
# the machine-readable JSON summary on stdout (per bead finetype-e6d
# ac-02). Stderr carries human-readable progress.
#
# Usage:
#   scripts/gittables_gate.sh                 # full holdout, sequential
#   scripts/gittables_gate.sh --jobs 4        # parallel
#   scripts/gittables_gate.sh --max-files 20  # smoke test
#   scripts/gittables_gate.sh --output eval/gittables/cycle_NNN.json
#
# Exits:
#   0 — measurement completed (regardless of gate_score)
#   1 — measurement failed (binary missing, holdout absent, IO error)
#   2 — gate_score below floor AND --enforce-floor passed (caller opt-in)
#
# Default invocation matches the contract's per-cycle measurement step.

set -euo pipefail

REPO_ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"

# --enforce-floor is a stripped-out caller option; everything else
# passes through unmodified to the Python harness.
ENFORCE_FLOOR=""
PASSTHRU=()
for arg in "$@"; do
    case "$arg" in
        --enforce-floor=*)
            ENFORCE_FLOOR="${arg#--enforce-floor=}"
            ;;
        *)
            PASSTHRU+=("$arg")
            ;;
    esac
done

cd "$REPO_ROOT"

# Run the harness; capture stdout into the summary file (so we can
# parse gate_score) AND tee to actual stdout for callers piping JSON.
TMP_SUMMARY="$(mktemp -t finetype-gate-summary.XXXXXX.json)"
trap 'rm -f "$TMP_SUMMARY"' EXIT

set +e
python3 scripts/gittables_gate.py "${PASSTHRU[@]}" >"$TMP_SUMMARY"
RC=$?
set -e

# Always echo the summary to stdout so the caller sees it.
cat "$TMP_SUMMARY"

if [[ "$RC" -ne 0 ]]; then
    echo "gittables_gate.sh: harness exited $RC" >&2
    exit 1
fi

# Optional: enforce a floor. The contract gate isn't a CI gate — it's a
# measurement. But callers (a future pre-promotion check) may want one.
if [[ -n "$ENFORCE_FLOOR" ]]; then
    SCORE="$(python3 -c "import json,sys; print(json.load(open('$TMP_SUMMARY'))['gate_score'])")"
    awk -v s="$SCORE" -v f="$ENFORCE_FLOOR" 'BEGIN{exit !(s+0 < f+0)}' && {
        echo "gittables_gate.sh: gate_score=$SCORE < floor=$ENFORCE_FLOOR" >&2
        exit 2
    }
fi
