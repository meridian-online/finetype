#!/usr/bin/env bash
# Autonomy-contract cycle entry point — bead finetype-53r.
#
# launchd invokes this script every 2h. It:
#   1. Runs scripts/cron_preamble.sh — acquires lockfile, checks disk
#   2. Invokes the cron-firing agent (`claude -p`) with the contract
#      cycle prompt (or, with --dry-run, runs a smoke-test gate)
#   3. Always runs scripts/cron_postamble.sh — releases lockfile
#
# All output (stdout + stderr) is intended to be redirected by launchd
# to ~/Library/Logs/finetype-cron/. The plist's StandardOutPath and
# StandardErrorPath are the canonical sinks.
#
# Modes:
#   scripts/cron_cycle.sh
#       Production mode — invokes `claude -p` with the cycle prompt.
#   scripts/cron_cycle.sh --dry-run
#       Smoke test — runs gate harness against a 3-file slice of the
#       holdout, exercises preamble + postamble. Proves the wiring
#       works end-to-end without an active REPL (53r ac-02).
#
# Exit codes:
#   0  cycle completed (regardless of gate_score)
#   1  preamble or postamble halted
#   2  cycle work itself failed
#
# The lockfile is ALWAYS released by the postamble even on failure,
# so a crash mid-cycle does not strand the lock.

set -uo pipefail

REPO_ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

DRY_RUN=0
for arg in "$@"; do
    case "$arg" in
        --dry-run) DRY_RUN=1 ;;
    esac
done

echo "=== cron_cycle.sh starting at $(date -u +%Y-%m-%dT%H:%M:%SZ) ===" >&2

# 1. Preamble — captures cycle_id, checks disk, acquires lock.
PREAMBLE_OUT="$(scripts/cron_preamble.sh)" || {
    echo "preamble halted:" >&2
    echo "$PREAMBLE_OUT" >&2
    exit 1
}
echo "preamble:" >&2
echo "$PREAMBLE_OUT" >&2

CYCLE_ID="$(printf '%s\n' "$PREAMBLE_OUT" | python3 -c 'import json,sys; print(json.loads(sys.stdin.read())["cycle_id"])')"
if [[ -z "$CYCLE_ID" ]]; then
    echo "error: no cycle_id from preamble" >&2
    exit 2
fi

# 2. Cycle body — production mode invokes claude; dry-run mode runs a
#    tiny gate measurement to prove the wiring.
CYCLE_RC=0
if (( DRY_RUN )); then
    echo "--- DRY-RUN: gate harness on 3 files ---" >&2
    HOLDOUT="$REPO_ROOT/eval/gittables/holdout_paths.txt"
    if [[ -f "$HOLDOUT" ]]; then
        scripts/gittables_gate.sh --max-files 3 --no-per-file --quiet >/dev/null || CYCLE_RC=$?
    else
        # No frozen holdout yet — synthesise a 3-file slice.
        TMP_HOLDOUT="$(mktemp -t finetype-dry-run.XXXXXX.txt)"
        find /Users/hugh/datasets/gittables -maxdepth 2 -name '*.parquet' 2>/dev/null \
            | head -3 > "$TMP_HOLDOUT"
        if [[ -s "$TMP_HOLDOUT" ]]; then
            scripts/gittables_gate.sh --holdout "$TMP_HOLDOUT" --no-per-file --quiet >/dev/null || CYCLE_RC=$?
        else
            echo "warning: no parquet files; dry-run gate skipped" >&2
        fi
        rm -f "$TMP_HOLDOUT"
    fi
else
    CLAUDE_BIN="${FINETYPE_CLAUDE_BIN:-claude}"
    if ! command -v "$CLAUDE_BIN" >/dev/null 2>&1; then
        echo "error: $CLAUDE_BIN not on PATH" >&2
        CYCLE_RC=2
    else
        # Per contract §2 standing orders: cron-firing agent reads the
        # contract at cycle start. Pass the cycle_id so the agent's
        # log entries are correlated with the lockfile.
        PROMPT="You are the FineType cron-firing agent. Read .orbit/contracts/2026-05-03-gittables-90-percent-roundtrip.yaml and run one cycle per §2 standing orders. Cycle ID: $CYCLE_ID. Halt and surface (do not improvise) on any unmatched state."
        "$CLAUDE_BIN" -p "$PROMPT" --permission-mode bypassPermissions || CYCLE_RC=$?
    fi
fi

# 3. Postamble — release lockfile no matter what.
echo "--- postamble ---" >&2
scripts/cron_postamble.sh "$CYCLE_ID" || {
    echo "postamble error" >&2
    # Don't mask cycle exit code with postamble error.
    if (( CYCLE_RC == 0 )); then CYCLE_RC=1; fi
}

echo "=== cron_cycle.sh finished (rc=$CYCLE_RC) at $(date -u +%Y-%m-%dT%H:%M:%SZ) ===" >&2
exit "$CYCLE_RC"
