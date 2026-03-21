#!/usr/bin/env bash
# Overnight distillation runner — launches Claude Code sessions in a loop.
#
# Each session: checks status, commits completed batches, launches 5 new waves.
# Resume-safe via .done markers. Safe to kill (Ctrl+C) at any time.
#
# Usage:
#   ./scripts/distill_overnight.sh              # Run until all Sherlock batches done
#   ./scripts/distill_overnight.sh --waves 3    # Run 3 waves per session (default: 5)
#   ./scripts/distill_overnight.sh --dry-run    # Show what would run without executing
#
# Requires: claude CLI on PATH
# Weekly budget: check with `claude usage` before starting

set -euo pipefail

REPO_DIR="$HOME/github/meridian-online/finetype"
WAVES_PER_SESSION="${WAVES:-5}"
PAUSE_BETWEEN_SESSIONS=10   # seconds between sessions
MAX_SESSIONS=1200           # safety cap (~6000 batches at 5/session, covers full plan)
DRY_RUN=false

# Parse args
while [[ $# -gt 0 ]]; do
    case "$1" in
        --waves) WAVES_PER_SESSION="$2"; shift 2 ;;
        --dry-run) DRY_RUN=true; shift ;;
        --max-sessions) MAX_SESSIONS="$2"; shift 2 ;;
        --pause) PAUSE_BETWEEN_SESSIONS="$2"; shift 2 ;;
        *) echo "Unknown arg: $1"; exit 1 ;;
    esac
done

SESSION_PROMPT='You are Nightingale, the FineType distillation specialist. Work in /home/hugh/github/meridian-online/finetype/.

## Task: Continue the distillation v3 rolling pipeline

### Step 1: Status check
Run: python3 scripts/distill_run.py status
If all Sherlock batches are done, say "SHERLOCK COMPLETE" and stop.

### Step 2: Get next batches
Run: python3 scripts/distill_run.py next --source sherlock --count '"$WAVES_PER_SESSION"'
This prints JSON lines with batch_id, jsonl_path, offset, limit.

### Step 3: Generate prompts and launch agents
For each batch from Step 3:
1. Generate prompt: python3 scripts/distill_agent_prompt.py --batch-id <batch_id> --jsonl <jsonl_path> --offset <offset> --limit <limit> > /tmp/distill_prompt_<batch_num>.txt
2. Launch agent: Agent(prompt="Read /tmp/distill_prompt_<batch_num>.txt and follow those instructions exactly. You are working in /home/hugh/github/meridian-online/finetype/. Complete all steps: blind classification, run finetype, adjudicate, write CSV and .done marker.", mode=bypassPermissions, model=sonnet, run_in_background=true)

Launch ALL agents in a SINGLE message (parallel launch).

### Step 4: Wait for agents
Wait for all agents to complete. Report status as they finish.

### Step 5: When all agents complete
Report final status. Do NOT commit or push — batch files stay untracked until concat.

### Critical rules
- Use --dangerously-skip-permissions equivalent (mode=bypassPermissions) for agents
- Do NOT commit batch output files to git — they are rolled up during concat
- If an agent fails, skip that batch — it will be retried next session
- Do NOT launch more than '"$WAVES_PER_SESSION"' agents total'

log() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $*"
}

progress_bar() {
    # Parse done count from distill_run.py status output
    local status_output
    status_output=$(python3 scripts/distill_run.py status 2>/dev/null)
    local total done pct
    # Extract TOTAL line: "TOTAL        5,436      168  ..."
    total=$(echo "$status_output" | awk '/^TOTAL/ {gsub(/,/,"",$2); print $2}')
    done=$(echo "$status_output" | awk '/^TOTAL/ {gsub(/,/,"",$3); print $3}')

    if [[ -z "$total" || "$total" -eq 0 ]]; then
        return
    fi

    pct=$((done * 100 / total))
    local bar_width=30
    local filled=$((pct * bar_width / 100))
    local empty=$((bar_width - filled))
    local bar=""
    for ((i=0; i<filled; i++)); do bar+="▓"; done
    for ((i=0; i<empty; i++)); do bar+="░"; done

    # ETA calculation
    local eta_str="--"
    if [[ -n "$START_TIME" && "$done" -gt "$START_DONE" ]]; then
        local elapsed=$(( $(date +%s) - START_TIME ))
        local batches_this_run=$((done - START_DONE))
        local remaining=$((total - done))
        if [[ "$batches_this_run" -gt 0 ]]; then
            local secs_per_batch=$((elapsed / batches_this_run))
            local eta_secs=$((remaining * secs_per_batch))
            local eta_h=$((eta_secs / 3600))
            local eta_m=$(( (eta_secs % 3600) / 60 ))
            eta_str="${eta_h}h ${eta_m}m"
        fi
    fi

    log "[${done}/${total} ${bar} ${pct}%] ETA: ${eta_str}"
}

INSTANCE_ID="$$"  # PID as unique instance identifier
START_TIME=$(date +%s)
# Capture starting done count for ETA
cd "$REPO_DIR"
START_DONE=$(python3 scripts/distill_run.py status 2>/dev/null | awk '/^TOTAL/ {gsub(/,/,"",$3); print $3}')
START_DONE="${START_DONE:-0}"

log "Distillation overnight runner starting"
log "Waves per session: $WAVES_PER_SESSION"
log "Pause between sessions: ${PAUSE_BETWEEN_SESSIONS}s"
log "Max sessions: $MAX_SESSIONS"
progress_bar

for ((session=1; session<=MAX_SESSIONS; session++)); do
    # Check if all batches are done
    if python3 scripts/distill_run.py next --source sherlock --count 1 2>&1 | grep -q "ALL BATCHES COMPLETE"; then
        log "All batches complete! 🎉"
        progress_bar
        break
    fi

    log "=== Session $session/$MAX_SESSIONS ==="

    if $DRY_RUN; then
        log "[DRY RUN] Would launch: claude -p '<prompt>' --dangerously-skip-permissions"
        log "[DRY RUN] Sleeping ${PAUSE_BETWEEN_SESSIONS}s"
        sleep 2
        continue
    fi

    # Launch Claude Code session
    # Parallel instances may occasionally duplicate a batch — .done markers
    # prevent data issues, and duplicates are rare (only if two sessions
    # call `next` before either writes a .done marker).
    log "Launching Claude Code session..."
    claude -p "$SESSION_PROMPT" \
        --model haiku \
        --dangerously-skip-permissions \
        --verbose 2>&1 | tee "/tmp/distill_${INSTANCE_ID}_session_${session}.log" || {
        log "Session $session failed (exit $?). Continuing after pause."
    }

    progress_bar
    log "Pausing ${PAUSE_BETWEEN_SESSIONS}s..."
    sleep "$PAUSE_BETWEEN_SESSIONS"
done

log "Overnight runner finished after $session sessions."
progress_bar
