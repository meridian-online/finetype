#!/usr/bin/env bash
# Append-only invariant check — bead finetype-87j ac-02.
#
# Implements halts H08 (failure_log corruption) and H09 (coverage_log
# corruption) from the autonomy contract: the line count of either
# append-only TSV must never drop between cycles. A drop is the only
# way silent state-loss reaches the cron — branches cannot bridge
# cycles, so this is the structural protection.
#
# Usage:
#   scripts/log_integrity_check.sh
#       Reads the most recent cycle_log.jsonl entry's
#       failure_log_lines_after / coverage_log_lines_after, compares
#       them to the current line count, exits 0 if invariant holds,
#       exits 1 with halt code H08/H09 on violation.
#
#   scripts/log_integrity_check.sh --baseline N --file F
#       Direct mode: assert wc -l F >= N. For tests.
#
# Exit codes:
#   0  invariant holds; or no prior cycle to compare against
#   1  invariant violated — halt cycle, surface to human
#   2  arguments / IO error

set -euo pipefail

# Repo root resolves from the script's location by default. The
# FINETYPE_REPO_ROOT env var lets tests point at a tempdir without
# moving the script. The cron preamble does NOT set it.
REPO_ROOT="${FINETYPE_REPO_ROOT:-$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)}"
GITTABLES_DIR="$REPO_ROOT/eval/gittables"
FAILURE_LOG="$GITTABLES_DIR/failure_log.tsv"
COVERAGE_LOG="$GITTABLES_DIR/working_slice_coverage.tsv"
CYCLE_LOG="$GITTABLES_DIR/cycle_log.jsonl"

count_lines() {
    local f="$1"
    if [[ -f "$f" ]]; then
        wc -l <"$f" | tr -d ' '
    else
        echo 0
    fi
}

# Direct mode for tests.
if [[ "${1:-}" == "--baseline" ]]; then
    shift
    BASELINE="$1"; shift
    [[ "${1:-}" == "--file" ]] || { echo "usage: --baseline N --file F" >&2; exit 2; }
    shift
    F="$1"
    CURRENT="$(count_lines "$F")"
    if (( CURRENT < BASELINE )); then
        echo "HALT: line count $CURRENT < baseline $BASELINE for $F" >&2
        exit 1
    fi
    echo "ok: $F line count $CURRENT >= baseline $BASELINE"
    exit 0
fi

# Cycle mode: pull the most recent cycle's line counts from cycle_log.jsonl.
if [[ ! -f "$CYCLE_LOG" ]]; then
    echo "no prior cycle_log; nothing to compare against" >&2
    exit 0
fi

LAST_LINE="$(tail -n 1 "$CYCLE_LOG")"
if [[ -z "$LAST_LINE" ]]; then
    echo "cycle_log empty; nothing to compare against" >&2
    exit 0
fi

# Extract the recorded "lines_after" counts from the previous cycle.
PREV_FAIL_AFTER="$(
    printf '%s\n' "$LAST_LINE" |
        python3 -c 'import json,sys; d=json.loads(sys.stdin.read()); print(d.get("failure_log_lines_after",0))'
)"
PREV_COV_AFTER="$(
    printf '%s\n' "$LAST_LINE" |
        python3 -c 'import json,sys; d=json.loads(sys.stdin.read()); print(d.get("coverage_log_lines_after",0))'
)"

CUR_FAIL="$(count_lines "$FAILURE_LOG")"
CUR_COV="$(count_lines "$COVERAGE_LOG")"

RC=0
if (( CUR_FAIL < PREV_FAIL_AFTER )); then
    echo "H08: failure_log line count dropped: prev=$PREV_FAIL_AFTER cur=$CUR_FAIL" >&2
    RC=1
fi
if (( CUR_COV < PREV_COV_AFTER )); then
    echo "H09: coverage_log line count dropped: prev=$PREV_COV_AFTER cur=$CUR_COV" >&2
    RC=1
fi

if (( RC == 0 )); then
    echo "ok: failure_log $CUR_FAIL >= $PREV_FAIL_AFTER; coverage_log $CUR_COV >= $PREV_COV_AFTER"
fi
exit "$RC"
