#!/usr/bin/env bash
set -euo pipefail

# runpod_sync.sh — Sync files and run commands on the RunPod autoresearch pod
# Covers: ac-13 (push), ac-14 (pull), ac-17 (preemption detection)
#
# Usage:
#   ./scripts/runpod_sync.sh push         # Sync code Beelink -> pod
#   ./scripts/runpod_sync.sh pull         # Sync results pod -> Beelink
#   ./scripts/runpod_sync.sh run CMD...   # Run command on pod via SSH
#
# Exit codes:
#   0 — success
#   1 — general error
#   2 — pod preempted/terminated (caller should handle restart)

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
SSH_FILE="$PROJECT_ROOT/.runpod_ssh.env"

SSH_OPTS="-o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -o ConnectTimeout=15"

log() { echo "[$(date '+%H:%M:%S')] $*"; }
err() { echo "[$(date '+%H:%M:%S')] ERROR: $*" >&2; }

# ─── Load config ────────────────────────────────────────────────────────────

if [ ! -f "$SSH_FILE" ]; then
    err "No SSH config found at $SSH_FILE — is a pod running?"
    exit 1
fi

# shellcheck source=/dev/null
source "$SSH_FILE"

: "${POD_ID:?POD_ID not set in $SSH_FILE}"
: "${POD_HOST:?POD_HOST not set in $SSH_FILE}"
: "${POD_PORT:?POD_PORT not set in $SSH_FILE}"

# ─── Preemption check ───────────────────────────────────────────────────────

check_pod_alive() {
    local POD_INFO POD_STATUS
    POD_INFO=$(runpodctl pod get "$POD_ID" 2>/dev/null || echo "{}")
    POD_STATUS=$(echo "$POD_INFO" | jq -r '.desiredStatus // .status // "UNKNOWN"' 2>/dev/null || echo "UNKNOWN")

    case "$POD_STATUS" in
        RUNNING)
            return 0
            ;;
        EXITED|TERMINATED|STOPPED)
            err "Pod $POD_ID is $POD_STATUS — likely preempted or stopped"
            exit 2
            ;;
        *)
            err "Pod $POD_ID has unexpected status: $POD_STATUS"
            exit 1
            ;;
    esac
}

# ─── Commands ────────────────────────────────────────────────────────────────

do_push() {
    log "Pushing research files to pod..."

    if [ ! -d "$PROJECT_ROOT/research" ]; then
        err "No research/ directory found at $PROJECT_ROOT"
        exit 1
    fi

    # Sync only research code files (not results/checkpoints)
    rsync -az --delete \
        -e "ssh -p $POD_PORT $SSH_OPTS" \
        --include='*.py' \
        --include='*.md' \
        --include='*.toml' \
        --include='*.yaml' \
        --include='*.yml' \
        --include='*.txt' \
        --include='*.cfg' \
        --include='*.ini' \
        --exclude='__pycache__/' \
        --exclude='.venv/' \
        --exclude='*.pyc' \
        --exclude='checkpoints/' \
        --exclude='run.log' \
        --exclude='results.tsv' \
        --exclude='.uv/' \
        "$PROJECT_ROOT/research/" \
        "root@$POD_HOST:/workspace/research/"

    log "  Push complete"
}

do_pull() {
    log "Pulling results from pod..."

    mkdir -p "$PROJECT_ROOT/research"
    mkdir -p "$PROJECT_ROOT/research/checkpoints"

    # Pull run log
    rsync -az \
        -e "ssh -p $POD_PORT $SSH_OPTS" \
        "root@$POD_HOST:/workspace/research/run.log" \
        "$PROJECT_ROOT/research/" 2>/dev/null || log "  No run.log to pull"

    # Pull results TSV
    rsync -az \
        -e "ssh -p $POD_PORT $SSH_OPTS" \
        "root@$POD_HOST:/workspace/research/results.tsv" \
        "$PROJECT_ROOT/research/" 2>/dev/null || log "  No results.tsv to pull"

    # Pull model checkpoints (only model files, not training intermediates)
    rsync -az \
        -e "ssh -p $POD_PORT $SSH_OPTS" \
        --include='*/' \
        --include='*.pt' \
        --include='*.pth' \
        --include='*.safetensors' \
        --include='*.json' \
        --include='*.yaml' \
        --exclude='*' \
        "root@$POD_HOST:/workspace/research/checkpoints/" \
        "$PROJECT_ROOT/research/checkpoints/" 2>/dev/null || log "  No checkpoints to pull"

    log "  Pull complete"
}

do_run() {
    local CMD="$*"

    if [ -z "$CMD" ]; then
        err "Usage: $0 run COMMAND..."
        exit 1
    fi

    log "Running on pod: $CMD"

    # Attempt SSH command; detect preemption on failure
    if ! ssh -p "$POD_PORT" $SSH_OPTS "root@$POD_HOST" "$CMD"; then
        SSH_EXIT=$?
        log "SSH command failed (exit $SSH_EXIT) — checking pod status..."
        check_pod_alive
        # If check_pod_alive returns (pod is RUNNING), it's an SSH/command error
        err "Command failed but pod is still running (SSH exit code: $SSH_EXIT)"
        exit 1
    fi
}

# ─── Main dispatch ───────────────────────────────────────────────────────────

COMMAND="${1:-}"
shift || true

case "$COMMAND" in
    push)
        do_push
        ;;
    pull)
        do_pull
        ;;
    run)
        do_run "$@"
        ;;
    status)
        check_pod_alive
        log "Pod $POD_ID is running"
        ssh -p "$POD_PORT" $SSH_OPTS "root@$POD_HOST" "nvidia-smi --query-gpu=name,utilization.gpu,memory.used,memory.total --format=csv,noheader" 2>/dev/null || true
        ;;
    *)
        echo "Usage: $0 {push|pull|run|status} [args...]"
        echo ""
        echo "Commands:"
        echo "  push         Sync research code from Beelink to pod"
        echo "  pull         Sync results (run.log, results.tsv, checkpoints) from pod"
        echo "  run CMD      Execute CMD on pod via SSH"
        echo "  status       Check pod status and GPU utilization"
        echo ""
        echo "Exit codes:"
        echo "  0  success"
        echo "  1  general error"
        echo "  2  pod preempted/terminated (caller handles restart)"
        exit 1
        ;;
esac
