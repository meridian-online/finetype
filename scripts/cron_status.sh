#!/usr/bin/env bash
# At-a-glance autonomy-contract status — escalations, halts, gate trend.
#
# Authority: contract artefacts at eval/gittables/cycle_log.jsonl etc.
# This is a read-only view; no state change. Use after `bd ready` or
# whenever you want to know what the cron-firing agent has been doing.
#
# Usage:
#   scripts/cron_status.sh             # full report
#   scripts/cron_status.sh trend       # gate-score series only
#   scripts/cron_status.sh escalations # escalation summary only
#   scripts/cron_status.sh notes       # full escalation note bodies
#   scripts/cron_status.sh halts       # halts that fired
#   scripts/cron_status.sh now         # is a cycle currently running?

set -euo pipefail

REPO_ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
CYCLE_LOG="$REPO_ROOT/eval/gittables/cycle_log.jsonl"
LOCKFILE="${FINETYPE_LOCKFILE:-/tmp/finetype-cron.lock}"

if [[ ! -f "$CYCLE_LOG" ]]; then
    echo "no cycles logged yet — $CYCLE_LOG does not exist" >&2
    exit 0
fi

show_now() {
    if [[ -e "$LOCKFILE" ]]; then
        echo "cycle in flight:"
        cat "$LOCKFILE"
    else
        echo "no cycle currently running"
    fi
}

show_trend() {
    echo "cycle_start                  cycle_id  gate_score  passed/total  errored  branches"
    jq -r '
        [.cycle_start,
         .cycle_id[:8],
         (.gate_score | tostring),
         "\(.files_passed)/\(.files_total)",
         (.gate_files_errored // 0 | tostring),
         (.branches_taken // [] | join(","))
        ] | @tsv
    ' "$CYCLE_LOG" | column -t -s $'\t'
}

show_escalations() {
    local count
    count="$(jq -r 'select(.escalations_raised // [] | length > 0)' "$CYCLE_LOG" | wc -l | tr -d ' ')"
    if (( count == 0 )); then
        echo "no escalations raised across $(wc -l <"$CYCLE_LOG" | tr -d ' ') cycles"
        return
    fi
    echo "cycle_start                  cycle_id  escalations"
    jq -r '
        select(.escalations_raised // [] | length > 0)
        | [.cycle_start, .cycle_id[:8], (.escalations_raised | join(","))]
        | @tsv
    ' "$CYCLE_LOG" | column -t -s $'\t'
}

show_notes() {
    jq -r '
        select(.escalation_notes)
        | "\n=== \(.cycle_start) (\(.cycle_id[:8])) ===",
          (.escalation_notes | to_entries[] | "\n[\(.key)]\n\(.value)")
    ' "$CYCLE_LOG"
}

show_halts() {
    local count
    count="$(jq -r 'select(.halts_fired // [] | length > 0)' "$CYCLE_LOG" | wc -l | tr -d ' ')"
    if (( count == 0 )); then
        echo "no halts fired across $(wc -l <"$CYCLE_LOG" | tr -d ' ') cycles"
        return
    fi
    echo "cycle_start                  cycle_id  halts"
    jq -r '
        select(.halts_fired // [] | length > 0)
        | [.cycle_start, .cycle_id[:8], (.halts_fired | join(","))]
        | @tsv
    ' "$CYCLE_LOG" | column -t -s $'\t'
}

case "${1:-all}" in
    now) show_now ;;
    trend) show_trend ;;
    escalations) show_escalations ;;
    notes) show_notes ;;
    halts) show_halts ;;
    all)
        echo "── now ──"
        show_now
        echo
        echo "── gate trend ──"
        show_trend
        echo
        echo "── escalations ──"
        show_escalations
        echo
        echo "── halts ──"
        show_halts
        echo
        echo "(scripts/cron_status.sh notes for full escalation note bodies)"
        ;;
    *)
        echo "usage: $0 {all|now|trend|escalations|notes|halts}" >&2
        exit 2
        ;;
esac
