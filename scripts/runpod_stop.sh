#!/usr/bin/env bash
set -euo pipefail

# runpod_stop.sh — Graceful teardown of a RunPod autoresearch pod
# Covers: ac-15

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
SSH_FILE="$PROJECT_ROOT/.runpod_ssh.env"
BUDGET_FILE="$PROJECT_ROOT/budget.json"

SSH_OPTS="-o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -o ConnectTimeout=10"

log() { echo "[$(date '+%H:%M:%S')] $*"; }
err() { echo "[$(date '+%H:%M:%S')] ERROR: $*" >&2; }
die() { err "$*"; exit 1; }

# ─── Load config ────────────────────────────────────────────────────────────

if [ ! -f "$SSH_FILE" ]; then
    die "No SSH config found at $SSH_FILE — is a pod running?"
fi

# shellcheck source=/dev/null
source "$SSH_FILE"

: "${POD_ID:?POD_ID not set in $SSH_FILE}"
: "${POD_HOST:?POD_HOST not set in $SSH_FILE}"
: "${POD_PORT:?POD_PORT not set in $SSH_FILE}"
: "${POD_COST_PER_HR:?POD_COST_PER_HR not set in $SSH_FILE}"
: "${POD_CREATED:?POD_CREATED not set in $SSH_FILE}"

log "Stopping pod $POD_ID..."

# ─── Sync final results back ────────────────────────────────────────────────

log "Syncing final results from pod..."

# Check if pod is still reachable
if ssh -p "$POD_PORT" $SSH_OPTS "root@$POD_HOST" "echo ok" >/dev/null 2>&1; then
    mkdir -p "$PROJECT_ROOT/research"

    # Sync results files
    rsync -az -e "ssh -p $POD_PORT $SSH_OPTS" \
        "root@$POD_HOST:/workspace/research/run.log" \
        "$PROJECT_ROOT/research/" 2>/dev/null || log "  No run.log to sync"

    rsync -az -e "ssh -p $POD_PORT $SSH_OPTS" \
        "root@$POD_HOST:/workspace/research/results.tsv" \
        "$PROJECT_ROOT/research/" 2>/dev/null || log "  No results.tsv to sync"

    # Sync any model checkpoints
    rsync -az -e "ssh -p $POD_PORT $SSH_OPTS" \
        --include='*/' --include='*.pt' --include='*.pth' --include='*.safetensors' --include='*.json' \
        --exclude='*' \
        "root@$POD_HOST:/workspace/research/checkpoints/" \
        "$PROJECT_ROOT/research/checkpoints/" 2>/dev/null || log "  No checkpoints to sync"

    log "  Results synced"
else
    log "  Pod not reachable via SSH — skipping result sync"
fi

# ─── Stop the pod ────────────────────────────────────────────────────────────

log "Sending stop command..."
STOP_OUTPUT=$(runpodctl pod stop "$POD_ID" 2>&1)
log "  $STOP_OUTPUT"

# ─── Update budget ──────────────────────────────────────────────────────────

NOW=$(date +%s)
ELAPSED_SECS=$((NOW - POD_CREATED))
ELAPSED_HOURS=$(echo "scale=2; $ELAPSED_SECS / 3600" | bc)
SESSION_COST=$(echo "scale=2; $ELAPSED_HOURS * $POD_COST_PER_HR" | bc)

if [ -f "$BUDGET_FILE" ]; then
    UPDATED_BUDGET=$(jq --arg pid "$POD_ID" --arg end "$NOW" --arg cost "$SESSION_COST" '
        .sessions = [.sessions[] | if .pod_id == $pid and .end == null then .end = ($end | tonumber) else . end]
        | .total_spend = ([.sessions[] | if .end != null then
            ((.end - .start) / 3600 * .cost_per_hr)
          else
            0
          end] | add // 0)
    ' "$BUDGET_FILE")
    echo "$UPDATED_BUDGET" > "$BUDGET_FILE"
    TOTAL_SPEND=$(echo "$UPDATED_BUDGET" | jq -r '.total_spend')
    log "  Budget updated: session=\$$SESSION_COST, total=\$$TOTAL_SPEND"
fi

# ─── Remove watchdog cron ────────────────────────────────────────────────────

log "Removing watchdog cron job..."
(crontab -l 2>/dev/null | grep -v "runpod_watchdog.sh" || true) | crontab -
log "  Watchdog cron removed"

# ─── Summary ─────────────────────────────────────────────────────────────────

echo ""
echo "════════════════════════════════════════════════════════════════════"
echo "  Pod stopped"
echo "════════════════════════════════════════════════════════════════════"
echo ""
echo "  Pod ID:       $POD_ID"
echo "  Runtime:      ${ELAPSED_HOURS}h"
echo "  Session cost: ~\$$SESSION_COST"
echo "  Total spend:  ~\$${TOTAL_SPEND:-unknown}"
echo ""
echo "  Network volume preserved: $VOLUME_ID"
echo "  Results synced to: $PROJECT_ROOT/research/"
echo ""
echo "  To restart: $SCRIPT_DIR/runpod_launch.sh"
echo "  To delete volume: runpodctl network-volume delete $VOLUME_ID"
echo ""
echo "════════════════════════════════════════════════════════════════════"
