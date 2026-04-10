#!/usr/bin/env bash
set -euo pipefail

# runpod_watchdog.sh — Zombie pod protection cron job
# Covers: ac-23 (zombie protection layer 3)
#
# Checks all running RunPod pods with "finetype" in the name.
# Terminates any pod exceeding MAX_RUNTIME_HOURS.
#
# Designed to run via cron every 30 minutes:
#   */30 * * * * /path/to/scripts/runpod_watchdog.sh >> /var/log/runpod_watchdog.log 2>&1

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
CONFIG_FILE="$PROJECT_ROOT/.runpod_config.env"

# Default max runtime (can be overridden by config)
MAX_RUNTIME_HOURS=10

log() { echo "[$(date '+%Y-%m-%d %H:%M:%S')] WATCHDOG: $*"; }

# Load config if available
if [ -f "$CONFIG_FILE" ]; then
    # shellcheck source=/dev/null
    source "$CONFIG_FILE"
fi

MAX_RUNTIME_SECS=$((MAX_RUNTIME_HOURS * 3600))

# ─── Check for zombie pods ──────────────────────────────────────────────────

# List all running pods
PODS=$(runpodctl pod list 2>/dev/null || echo "[]")

# Filter pods with "finetype" in the name
FINETYPE_PODS=$(echo "$PODS" | jq -c '[.[] | select(.name | test("finetype"; "i"))]' 2>/dev/null || echo "[]")
POD_COUNT=$(echo "$FINETYPE_PODS" | jq 'length' 2>/dev/null || echo "0")

if [ "$POD_COUNT" -eq 0 ]; then
    # No finetype pods running — nothing to do
    exit 0
fi

log "Found $POD_COUNT finetype pod(s) running"

NOW=$(date +%s)

echo "$FINETYPE_PODS" | jq -c '.[]' | while IFS= read -r POD; do
    POD_ID=$(echo "$POD" | jq -r '.id')
    POD_NAME=$(echo "$POD" | jq -r '.name')
    UPTIME_SECS=$(echo "$POD" | jq -r '.runtime.uptimeInSeconds // 0')

    # If uptime is not available from list, try getting pod details
    if [ "$UPTIME_SECS" = "0" ] || [ "$UPTIME_SECS" = "null" ]; then
        POD_DETAIL=$(runpodctl pod get "$POD_ID" 2>/dev/null || echo "{}")
        UPTIME_SECS=$(echo "$POD_DETAIL" | jq -r '.runtime.uptimeInSeconds // 0' 2>/dev/null || echo "0")
    fi

    UPTIME_HOURS=$(echo "scale=1; $UPTIME_SECS / 3600" | bc 2>/dev/null || echo "0")

    if [ "$UPTIME_SECS" -gt "$MAX_RUNTIME_SECS" ]; then
        log "ZOMBIE DETECTED: Pod $POD_ID ($POD_NAME) running for ${UPTIME_HOURS}h (max: ${MAX_RUNTIME_HOURS}h)"
        log "  Terminating pod $POD_ID..."

        STOP_OUTPUT=$(runpodctl pod stop "$POD_ID" 2>&1)
        log "  Stop result: $STOP_OUTPUT"

        # Also update budget.json if it exists
        BUDGET_FILE="$PROJECT_ROOT/budget.json"
        if [ -f "$BUDGET_FILE" ]; then
            UPDATED_BUDGET=$(jq --arg pid "$POD_ID" --arg end "$NOW" '
                .sessions = [.sessions[] | if .pod_id == $pid and .end == null then .end = ($end | tonumber) else . end]
                | .total_spend = ([.sessions[] | if .end != null then
                    ((.end - .start) / 3600 * .cost_per_hr)
                  else
                    0
                  end] | add // 0)
            ' "$BUDGET_FILE" 2>/dev/null || echo "")
            if [ -n "$UPDATED_BUDGET" ]; then
                echo "$UPDATED_BUDGET" > "$BUDGET_FILE"
                log "  Budget updated"
            fi
        fi
    else
        log "Pod $POD_ID ($POD_NAME): ${UPTIME_HOURS}h uptime — OK (max: ${MAX_RUNTIME_HOURS}h)"
    fi
done
