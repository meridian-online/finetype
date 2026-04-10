#!/usr/bin/env bash
set -euo pipefail

# runpod_budget.sh — Budget tracking for RunPod autoresearch sessions
# Covers: ac-16
#
# Usage:
#   ./scripts/runpod_budget.sh check    # Exit 0 if under threshold, 1 if over
#   ./scripts/runpod_budget.sh update   # Update current session elapsed time
#   ./scripts/runpod_budget.sh status   # Print human-readable budget status

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
SSH_FILE="$PROJECT_ROOT/.runpod_ssh.env"
BUDGET_FILE="$PROJECT_ROOT/budget.json"

log() { echo "[$(date '+%H:%M:%S')] $*"; }
err() { echo "[$(date '+%H:%M:%S')] ERROR: $*" >&2; }

# ─── Ensure budget.json exists ──────────────────────────────────────────────

if [ ! -f "$BUDGET_FILE" ]; then
    cat > "$BUDGET_FILE" <<EOF
{
  "budget_cap": 40.00,
  "alert_threshold": 35.00,
  "sessions": [],
  "total_spend": 0.00
}
EOF
    log "Created new budget.json"
fi

# ─── Compute current spend ──────────────────────────────────────────────────

compute_spend() {
    local NOW
    NOW=$(date +%s)

    # Calculate spend for all sessions, treating open sessions as running until now
    jq --arg now "$NOW" '
        .total_spend = ([.sessions[] |
            if .end != null then
                ((.end - .start) / 3600 * .cost_per_hr)
            else
                (($now | tonumber) - .start) / 3600 * .cost_per_hr
            end
        ] | add // 0)
        | .total_spend = (.total_spend * 100 | round / 100)
    ' "$BUDGET_FILE"
}

# ─── Commands ────────────────────────────────────────────────────────────────

do_check() {
    local BUDGET_DATA TOTAL_SPEND THRESHOLD
    BUDGET_DATA=$(compute_spend)
    TOTAL_SPEND=$(echo "$BUDGET_DATA" | jq -r '.total_spend')
    THRESHOLD=$(echo "$BUDGET_DATA" | jq -r '.alert_threshold')

    # Compare using bc (floating point)
    OVER=$(echo "$TOTAL_SPEND >= $THRESHOLD" | bc -l)

    if [ "$OVER" -eq 1 ]; then
        err "BUDGET ALERT: \$$TOTAL_SPEND spent (threshold: \$$THRESHOLD)"
        exit 1
    else
        exit 0
    fi
}

do_update() {
    local BUDGET_DATA
    BUDGET_DATA=$(compute_spend)
    echo "$BUDGET_DATA" > "$BUDGET_FILE"

    local TOTAL_SPEND
    TOTAL_SPEND=$(echo "$BUDGET_DATA" | jq -r '.total_spend')
    log "Budget updated: \$$TOTAL_SPEND spent"
}

do_status() {
    local NOW BUDGET_DATA
    NOW=$(date +%s)
    BUDGET_DATA=$(compute_spend)

    local TOTAL_SPEND BUDGET_CAP THRESHOLD REMAINING SESSION_COUNT
    TOTAL_SPEND=$(echo "$BUDGET_DATA" | jq -r '.total_spend')
    BUDGET_CAP=$(echo "$BUDGET_DATA" | jq -r '.budget_cap')
    THRESHOLD=$(echo "$BUDGET_DATA" | jq -r '.alert_threshold')
    REMAINING=$(echo "$BUDGET_CAP - $TOTAL_SPEND" | bc)
    SESSION_COUNT=$(echo "$BUDGET_DATA" | jq '.sessions | length')

    # Calculate hours remaining at current rate
    local ACTIVE_RATE HOURS_REMAINING
    ACTIVE_RATE=$(echo "$BUDGET_DATA" | jq '[.sessions[] | select(.end == null) | .cost_per_hr] | add // 0')
    if [ "$(echo "$ACTIVE_RATE > 0" | bc -l)" -eq 1 ]; then
        HOURS_REMAINING=$(echo "scale=1; $REMAINING / $ACTIVE_RATE" | bc)
    else
        HOURS_REMAINING="N/A (no active session)"
    fi

    echo ""
    echo "════════════════════════════════════════════════════════════════════"
    echo "  RunPod Budget Status"
    echo "════════════════════════════════════════════════════════════════════"
    echo ""
    echo "  Total spend:      \$$TOTAL_SPEND"
    echo "  Budget cap:       \$$BUDGET_CAP"
    echo "  Alert threshold:  \$$THRESHOLD"
    echo "  Remaining:        \$$REMAINING"
    echo "  Hours remaining:  $HOURS_REMAINING"
    echo "  Sessions:         $SESSION_COUNT"
    echo ""

    # Show active sessions
    local ACTIVE_SESSIONS
    ACTIVE_SESSIONS=$(echo "$BUDGET_DATA" | jq -c '[.sessions[] | select(.end == null)]')
    ACTIVE_COUNT=$(echo "$ACTIVE_SESSIONS" | jq 'length')

    if [ "$ACTIVE_COUNT" -gt 0 ]; then
        echo "  Active sessions:"
        echo "$ACTIVE_SESSIONS" | jq -c '.[]' | while IFS= read -r SESSION; do
            local SID SSTART SCOST SELAPSED SCOST_SO_FAR
            SID=$(echo "$SESSION" | jq -r '.pod_id')
            SSTART=$(echo "$SESSION" | jq -r '.start')
            SCOST=$(echo "$SESSION" | jq -r '.cost_per_hr')
            SELAPSED=$(echo "scale=1; ($NOW - $SSTART) / 3600" | bc)
            SCOST_SO_FAR=$(echo "scale=2; $SELAPSED * $SCOST" | bc)
            echo "    Pod $SID: ${SELAPSED}h @ \$$SCOST/hr = \$$SCOST_SO_FAR"
        done
        echo ""
    fi

    # Show completed sessions
    local COMPLETED_SESSIONS
    COMPLETED_SESSIONS=$(echo "$BUDGET_DATA" | jq -c '[.sessions[] | select(.end != null)]')
    COMPLETED_COUNT=$(echo "$COMPLETED_SESSIONS" | jq 'length')

    if [ "$COMPLETED_COUNT" -gt 0 ]; then
        echo "  Completed sessions:"
        echo "$COMPLETED_SESSIONS" | jq -c '.[]' | while IFS= read -r SESSION; do
            local SID SSTART SEND SCOST SELAPSED SCOST_FINAL
            SID=$(echo "$SESSION" | jq -r '.pod_id')
            SSTART=$(echo "$SESSION" | jq -r '.start')
            SEND=$(echo "$SESSION" | jq -r '.end')
            SCOST=$(echo "$SESSION" | jq -r '.cost_per_hr')
            SELAPSED=$(echo "scale=1; ($SEND - $SSTART) / 3600" | bc)
            SCOST_FINAL=$(echo "scale=2; $SELAPSED * $SCOST" | bc)
            echo "    Pod $SID: ${SELAPSED}h @ \$$SCOST/hr = \$$SCOST_FINAL"
        done
        echo ""
    fi

    echo "════════════════════════════════════════════════════════════════════"

    # Also return exit code for scripting
    local OVER
    OVER=$(echo "$TOTAL_SPEND >= $THRESHOLD" | bc -l)
    if [ "$OVER" -eq 1 ]; then
        echo ""
        err "WARNING: Spend has reached the alert threshold!"
        exit 1
    fi
}

# ─── Main dispatch ───────────────────────────────────────────────────────────

COMMAND="${1:-status}"

case "$COMMAND" in
    check)
        do_check
        ;;
    update)
        do_update
        ;;
    status)
        do_status
        ;;
    *)
        echo "Usage: $0 {check|update|status}"
        echo ""
        echo "Commands:"
        echo "  check    Exit 0 if under budget threshold, 1 if over"
        echo "  update   Recalculate and persist current spend"
        echo "  status   Print human-readable budget summary"
        exit 1
        ;;
esac
