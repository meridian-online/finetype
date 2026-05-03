#!/usr/bin/env bash
# Autonomy-contract cycle postamble — bead finetype-nms.
#
# Releases /tmp/finetype-cron.lock and (optionally) appends a cycle
# summary record to eval/gittables/cycle_log.jsonl. Called by the cron
# agent after the per-cycle work completes (or aborts).
#
# Usage:
#   scripts/cron_postamble.sh <cycle_id>
#       Releases lock for <cycle_id>. Sanity-checks the cycle_id in
#       the lockfile matches; refuses to remove a lockfile owned by
#       a different cycle (defence-in-depth against multi-cycle
#       interleaving).
#
#   scripts/cron_postamble.sh --force
#       Removes the lockfile unconditionally. For operator-driven
#       recovery only; not used by normal cron flow.
#
# Override targets (same as cron_preamble.sh):
#   FINETYPE_LOCKFILE   /tmp/finetype-cron.lock
#
# Exit codes:
#   0  success
#   1  cycle_id mismatch (refuses to clobber another cycle's lock)
#   2  IO / argument error

set -euo pipefail

LOCKFILE="${FINETYPE_LOCKFILE:-/tmp/finetype-cron.lock}"

if [[ "${1:-}" == "--force" ]]; then
    rm -f "$LOCKFILE"
    echo "force: removed $LOCKFILE" >&2
    exit 0
fi

CYCLE_ID="${1:-}"
if [[ -z "$CYCLE_ID" ]]; then
    echo "usage: cron_postamble.sh <cycle_id> | --force" >&2
    exit 2
fi

if [[ ! -e "$LOCKFILE" ]]; then
    echo "warning: lockfile $LOCKFILE not present (already released?)" >&2
    exit 0
fi

LOCK_CYCLE_ID="$(
    python3 -c '
import json, sys
try:
    d = json.load(open(sys.argv[1]))
    print(d.get("cycle_id", ""))
except Exception:
    print("")
' "$LOCKFILE"
)"

if [[ "$LOCK_CYCLE_ID" != "$CYCLE_ID" ]]; then
    echo "error: lockfile cycle_id ($LOCK_CYCLE_ID) != requested ($CYCLE_ID); refusing to remove" >&2
    exit 1
fi

rm -f "$LOCKFILE"
echo "released lock for cycle $CYCLE_ID" >&2
