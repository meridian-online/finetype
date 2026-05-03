#!/usr/bin/env bash
# Install / uninstall the FineType cron-firing launchd agent.
# Bead finetype-53r.
#
# Usage:
#   scripts/launchd/install.sh install
#       Copies the plist to ~/Library/LaunchAgents, creates the log
#       directory, and runs `launchctl load`.
#
#   scripts/launchd/install.sh uninstall
#       Runs `launchctl unload` and removes the plist. Logs and the
#       /etc/newsyslog.d/finetype-cron.conf entry are left in place
#       (restore them manually if you want a full clean).
#
#   scripts/launchd/install.sh status
#       Shows whether the plist is installed and loaded.
#
#   scripts/launchd/install.sh dry-run-cycle
#       Runs scripts/cron_cycle.sh --dry-run with the same env launchd
#       uses, end-to-end. ac-02 verification.
#
#   scripts/launchd/install.sh install-pass2
#       Installs the pass-2 one-shot launchd plist (fires once at
#       2026-05-10 06:03 AEST, then self-removes).
#
#   scripts/launchd/install.sh uninstall-pass2
#       Removes the pass-2 one-shot plist (cancels the firing).
#
#   scripts/launchd/install.sh status-pass2
#       Shows whether the pass-2 one-shot is installed/loaded.
#
#   scripts/launchd/install.sh dry-run-pass2
#       Prints the prompt body that would be sent to claude -p.
#
# Idempotent: install on a host where it's already loaded reloads.

set -euo pipefail

REPO_ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
PLIST_NAME="online.meridian.finetype-cron.plist"
PLIST_SRC="$REPO_ROOT/scripts/launchd/$PLIST_NAME"
PLIST_DST="$HOME/Library/LaunchAgents/$PLIST_NAME"
LOG_DIR="$HOME/Library/Logs/finetype-cron"
LABEL="online.meridian.finetype-cron"

# Pass-2 one-shot
PASS2_PLIST_NAME="online.meridian.finetype-pass2-2026-05-10.plist"
PASS2_PLIST_SRC="$REPO_ROOT/scripts/launchd/$PASS2_PLIST_NAME"
PASS2_PLIST_DST="$HOME/Library/LaunchAgents/$PASS2_PLIST_NAME"
PASS2_LABEL="online.meridian.finetype-pass2-2026-05-10"

cmd_install() {
    if [[ ! -f "$PLIST_SRC" ]]; then
        echo "error: source plist not found: $PLIST_SRC" >&2
        exit 1
    fi
    mkdir -p "$LOG_DIR"
    mkdir -p "$(dirname "$PLIST_DST")"

    # Reload if already loaded.
    if launchctl list "$LABEL" >/dev/null 2>&1; then
        echo "already loaded — unloading first"
        launchctl unload "$PLIST_DST" || true
    fi

    cp "$PLIST_SRC" "$PLIST_DST"
    launchctl load "$PLIST_DST"
    echo "installed: $PLIST_DST"
    echo "logs:      $LOG_DIR/{stdout,stderr}.log"
    echo
    echo "Verify with: launchctl list | grep finetype-cron"
}

cmd_uninstall() {
    if [[ -f "$PLIST_DST" ]]; then
        launchctl unload "$PLIST_DST" || true
        rm -f "$PLIST_DST"
        echo "uninstalled: $PLIST_DST"
    else
        echo "not installed: $PLIST_DST"
    fi
    if [[ -e /tmp/finetype-cron.lock ]]; then
        echo "stale lockfile present at /tmp/finetype-cron.lock; remove with"
        echo "  scripts/cron_postamble.sh --force"
    fi
}

cmd_status() {
    if [[ -f "$PLIST_DST" ]]; then
        echo "plist installed: $PLIST_DST"
    else
        echo "plist NOT installed"
    fi
    if launchctl list "$LABEL" >/dev/null 2>&1; then
        echo "launchctl: loaded"
        launchctl list "$LABEL" | head -20
    else
        echo "launchctl: NOT loaded"
    fi
    if [[ -e /tmp/finetype-cron.lock ]]; then
        echo "lockfile: present at /tmp/finetype-cron.lock"
        cat /tmp/finetype-cron.lock
    else
        echo "lockfile: not present"
    fi
}

cmd_dry_run_cycle() {
    if [[ -e /tmp/finetype-cron.lock ]]; then
        echo "warning: lockfile present; pre-removing for dry-run" >&2
        rm -f /tmp/finetype-cron.lock
    fi
    cd "$REPO_ROOT"
    exec scripts/cron_cycle.sh --dry-run
}

cmd_install_pass2() {
    if [[ ! -f "$PASS2_PLIST_SRC" ]]; then
        echo "error: source plist not found: $PASS2_PLIST_SRC" >&2
        exit 1
    fi
    mkdir -p "$LOG_DIR"
    mkdir -p "$(dirname "$PASS2_PLIST_DST")"

    # Reload if already loaded.
    if launchctl list "$PASS2_LABEL" >/dev/null 2>&1; then
        echo "already loaded — unloading first"
        launchctl unload "$PASS2_PLIST_DST" || true
    fi

    cp "$PASS2_PLIST_SRC" "$PASS2_PLIST_DST"
    launchctl load "$PASS2_PLIST_DST"
    echo "installed: $PASS2_PLIST_DST"
    echo "fires:     2026-05-10 06:03 AEST (one-shot, self-removes after)"
    echo "logs:      $LOG_DIR/pass2-{stdout,stderr}.log"
    echo
    echo "Verify with: launchctl list | grep finetype-pass2"
}

cmd_uninstall_pass2() {
    if [[ -f "$PASS2_PLIST_DST" ]]; then
        launchctl unload "$PASS2_PLIST_DST" || true
        rm -f "$PASS2_PLIST_DST"
        echo "uninstalled: $PASS2_PLIST_DST"
    else
        echo "not installed: $PASS2_PLIST_DST"
    fi
}

cmd_status_pass2() {
    if [[ -f "$PASS2_PLIST_DST" ]]; then
        echo "pass-2 plist installed: $PASS2_PLIST_DST"
    else
        echo "pass-2 plist NOT installed"
    fi
    if launchctl list "$PASS2_LABEL" >/dev/null 2>&1; then
        echo "launchctl: loaded"
        launchctl list "$PASS2_LABEL" | head -20
    else
        echo "launchctl: NOT loaded"
    fi
}

cmd_dry_run_pass2() {
    cd "$REPO_ROOT"
    exec scripts/cron_pass2_oneshot.sh --dry-run
}

case "${1:-}" in
    install) cmd_install ;;
    uninstall) cmd_uninstall ;;
    status) cmd_status ;;
    dry-run-cycle) cmd_dry_run_cycle ;;
    install-pass2) cmd_install_pass2 ;;
    uninstall-pass2) cmd_uninstall_pass2 ;;
    status-pass2) cmd_status_pass2 ;;
    dry-run-pass2) cmd_dry_run_pass2 ;;
    *)
        echo "usage: $0 {install|uninstall|status|dry-run-cycle|install-pass2|uninstall-pass2|status-pass2|dry-run-pass2}" >&2
        exit 2
        ;;
esac
