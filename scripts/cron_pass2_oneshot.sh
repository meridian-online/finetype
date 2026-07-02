#!/usr/bin/env bash
# Pass-2 tabletop brief preparation — one-shot launchd entry point.
#
# Bead finetype-53r "Bonus" section per docs/CRON-LAUNCHD-SETUP.md.
# Sibling to scripts/cron_cycle.sh; this is invoked once on the pinned
# date (2026-05-10 06:03 AEST) by online.meridian.finetype-pass2-2026-05-10
# launchd agent.
#
# Sequence:
#   1. Extract the prompt body from
#      scripts/contracts/2026-05-10-pass2-prep-prompt.md (option B's awk
#      command in that file is the source-of-truth extraction).
#   2. Invoke `claude -p` with bypassPermissions so the agent can read,
#      write, and commit/push the brief.
#   3. Schedule self-cleanup: detached subshell unloads the launchd
#      agent and removes the plist + this wrapper trigger from the
#      LaunchAgents directory ~10s after we exit. The plist's
#      `StartCalendarInterval` is pinned to one specific minute on one
#      specific day, so it physically cannot fire again, but cleanliness
#      argues for the rm anyway (avoids the `launchctl list` noise).
#
# Stdout/stderr go to ~/Library/Logs/finetype-cron/pass2-{stdout,stderr}.log
# per the plist's StandardOutPath / StandardErrorPath. Logs are NOT
# cleaned up; they are the audit trail of the firing.
#
# Manual-override modes:
#   scripts/cron_pass2_oneshot.sh           # production firing
#   scripts/cron_pass2_oneshot.sh --dry-run # print the prompt body, do not invoke claude
#   scripts/cron_pass2_oneshot.sh --no-cleanup
#                                           # skip self-removal (debugging)

set -uo pipefail

REPO_ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

DRY_RUN=0
NO_CLEANUP=0
for arg in "$@"; do
    case "$arg" in
        --dry-run) DRY_RUN=1 ;;
        --no-cleanup) NO_CLEANUP=1 ;;
    esac
done

PROMPT_FILE="$REPO_ROOT/scripts/contracts/2026-05-10-pass2-prep-prompt.md"
PLIST_NAME="online.meridian.finetype-pass2-2026-05-10.plist"
PLIST_PATH="$HOME/Library/LaunchAgents/$PLIST_NAME"
LABEL="online.meridian.finetype-pass2-2026-05-10"

echo "=== cron_pass2_oneshot.sh starting at $(date -u +%Y-%m-%dT%H:%M:%SZ) ===" >&2

if [[ ! -f "$PROMPT_FILE" ]]; then
    echo "error: prompt file not found at $PROMPT_FILE" >&2
    exit 2
fi

# Extract the prompt body — same awk command documented in the prompt
# file's Option B. Stripping the "## Prompt" header line.
PROMPT_BODY="$(awk '/^## Prompt/,0' "$PROMPT_FILE" | tail -n +2)"

if [[ -z "$PROMPT_BODY" ]]; then
    echo "error: extracted empty prompt from $PROMPT_FILE" >&2
    exit 2
fi

if (( DRY_RUN )); then
    echo "--- DRY-RUN: would invoke claude -p with the following prompt ---" >&2
    echo "$PROMPT_BODY"
    echo "--- end DRY-RUN ---" >&2
    exit 0
fi

CLAUDE_BIN="${FINETYPE_CLAUDE_BIN:-claude}"
if ! command -v "$CLAUDE_BIN" >/dev/null 2>&1; then
    echo "error: $CLAUDE_BIN not on PATH" >&2
    exit 2
fi

echo "invoking claude -p (length: ${#PROMPT_BODY} chars)" >&2
RC=0
"$CLAUDE_BIN" -p "$PROMPT_BODY" --permission-mode bypassPermissions || RC=$?
echo "claude -p exited rc=$RC" >&2

# Self-cleanup — detached so launchd doesn't see the unload as a kill
# of an active process. The 10s grace gives our parent shell time to
# exit cleanly. `disown` + `setsid` makes us survive the agent's
# termination.
if (( NO_CLEANUP == 0 )); then
    echo "scheduling self-cleanup of $PLIST_NAME" >&2
    (
        sleep 10
        launchctl unload "$PLIST_PATH" 2>/dev/null || true
        rm -f "$PLIST_PATH"
        echo "[cleanup] removed $PLIST_PATH at $(date -u +%Y-%m-%dT%H:%M:%SZ)" \
            >> "$HOME/Library/Logs/finetype-cron/pass2-stderr.log" 2>/dev/null || true
    ) &
    disown $! 2>/dev/null || true
fi

echo "=== cron_pass2_oneshot.sh finished (rc=$RC) at $(date -u +%Y-%m-%dT%H:%M:%SZ) ===" >&2
exit "$RC"
