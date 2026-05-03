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
# Idempotent: install on a host where it's already loaded reloads.

set -euo pipefail

REPO_ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
PLIST_NAME="online.meridian.finetype-cron.plist"
PLIST_SRC="$REPO_ROOT/scripts/launchd/$PLIST_NAME"
PLIST_DST="$HOME/Library/LaunchAgents/$PLIST_NAME"
LOG_DIR="$HOME/Library/Logs/finetype-cron"
LABEL="online.meridian.finetype-cron"

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

case "${1:-}" in
    install) cmd_install ;;
    uninstall) cmd_uninstall ;;
    status) cmd_status ;;
    dry-run-cycle) cmd_dry_run_cycle ;;
    *)
        echo "usage: $0 {install|uninstall|status|dry-run-cycle}" >&2
        exit 2
        ;;
esac
