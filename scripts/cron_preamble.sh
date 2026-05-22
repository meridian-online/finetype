#!/usr/bin/env bash
# Autonomy-contract cycle preamble — bead finetype-nms.
#
# launchd fires this script every 2h. The preamble:
#   1. Reads the active contract; records its checksum
#   2. Acquires /tmp/finetype-cron.lock (halt H04 if already held)
#   3. Verifies free disk >= 20 GB (halt H01 otherwise)
#   4. Captures pre-cycle line counts of failure_log + coverage_log
#      (the H08/H09 invariant baseline for the next cycle)
#   5. Emits a cycle preamble JSON record on stdout — the cycle agent
#      uses this as its starting state.
#
# The lockfile contains:
#   {cycle_id: <uuid>, started: <iso8601>, pid: <pid>}
# Released by scripts/cron_postamble.sh on cycle completion.
#
# Expected to complete in < 1s under normal conditions; the budget is
# 2s combined preamble + postamble (ac-05) — so 1s here, 1s there.
#
# Override targets for tests:
#   FINETYPE_LOCKFILE     /tmp/finetype-cron.lock
#   FINETYPE_DISK_TARGET  /
#   FINETYPE_DISK_FLOOR_GB  20
#   FINETYPE_REPO_ROOT    repo root (resolved from script location by default)
#   FINETYPE_DISK_CMD     df command (test mock)
#
# Exit codes:
#   0  success — lockfile created, JSON preamble on stdout
#   1  halt fired (H04 lock-present, H01 disk-floor) — JSON halt on stdout
#   2  IO / argument error

set -euo pipefail

REPO_ROOT="${FINETYPE_REPO_ROOT:-$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)}"
LOCKFILE="${FINETYPE_LOCKFILE:-/tmp/finetype-cron.lock}"
DISK_TARGET="${FINETYPE_DISK_TARGET:-/}"
DISK_FLOOR_GB="${FINETYPE_DISK_FLOOR_GB:-20}"
DISK_CMD="${FINETYPE_DISK_CMD:-df -g}"
CONTRACT="$REPO_ROOT/.orbit/contracts/2026-05-03-gittables-90-percent-roundtrip.yaml"
GITTABLES_DIR="$REPO_ROOT/eval/gittables"
FAILURE_LOG="$GITTABLES_DIR/failure_log.tsv"
COVERAGE_LOG="$GITTABLES_DIR/working_slice_coverage.tsv"
INFERENCE_SIGNALS="$GITTABLES_DIR/inference_signals.tsv"
FINETYPE_BIN="${FINETYPE_BIN:-$REPO_ROOT/target/release/finetype}"
INFERENCE_SIGNALS_HEADER="cycle_id	timestamp	file_path	file_content_sha256	column_name	predicted_type	inferred_correct_type	confidence	mechanism	validator_pass_rate	header_match	top_k_json"

iso_utc() {
    date -u +%Y-%m-%dT%H:%M:%SZ
}

emit_halt() {
    local id="$1"
    local message="$2"
    python3 - "$id" "$message" <<'PY'
import json, sys
print(json.dumps({
    "status": "halt",
    "halt_id": sys.argv[1],
    "message": sys.argv[2],
}))
PY
}

# ── 1. Contract presence + checksum ────────────────────────────────────
if [[ ! -f "$CONTRACT" ]]; then
    emit_halt "H_NO_CONTRACT" "contract not found at $CONTRACT"
    exit 2
fi
CONTRACT_SHA="$(shasum -a 256 "$CONTRACT" | awk '{print $1}')"

# ── 2. Lockfile acquire (H04) ──────────────────────────────────────────
if [[ -e "$LOCKFILE" ]]; then
    LOCK_PAYLOAD="$(cat "$LOCKFILE" 2>/dev/null || echo '<unreadable>')"
    emit_halt "H04" "concurrent cycle: lockfile already exists at $LOCKFILE — payload: $LOCK_PAYLOAD"
    exit 1
fi

CYCLE_ID="$(uuidgen | tr '[:upper:]' '[:lower:]')"
CYCLE_START="$(iso_utc)"

# Atomic create-or-fail via O_CREAT|O_EXCL via `set -C` + `>`.
(
    set -C
    printf '{"cycle_id":"%s","started":"%s","pid":%d}\n' \
        "$CYCLE_ID" "$CYCLE_START" "$$" > "$LOCKFILE"
) || {
    emit_halt "H04" "concurrent cycle: lockfile creation race at $LOCKFILE"
    exit 1
}

# Even on subsequent failures, leave the lockfile in place so the
# cron runner can see what was happening — the postamble removes it.
# H01 is intentionally non-removing because the operator may want to
# inspect the lock payload alongside the disk-floor signal.

# ── 3. Disk floor (H01) ────────────────────────────────────────────────
# Use `df -g` (GB units, BSD-style) on macOS; default on Linux is `df -BG`.
# Both produce: header line + data line(s); col 4 is "Available".
DF_OUT="$($DISK_CMD "$DISK_TARGET")"
FREE_GB="$(printf '%s\n' "$DF_OUT" | awk 'NR==2 {print $4}' | tr -d 'G')"
if ! [[ "$FREE_GB" =~ ^[0-9]+$ ]]; then
    emit_halt "H01" "could not parse df output: $DF_OUT"
    exit 1
fi
if (( FREE_GB < DISK_FLOOR_GB )); then
    emit_halt "H01" "free disk ${FREE_GB}G < floor ${DISK_FLOOR_GB}G on $DISK_TARGET"
    exit 1
fi

# ── 4. Pre-cycle log line counts ───────────────────────────────────────
count_lines() {
    if [[ -f "$1" ]]; then
        wc -l <"$1" | tr -d ' '
    else
        echo 0
    fi
}
FAIL_LINES_BEFORE="$(count_lines "$FAILURE_LOG")"
COV_LINES_BEFORE="$(count_lines "$COVERAGE_LOG")"

# ── 4a. inference_signals.tsv header init (ac-12) ──────────────────────
# Sidecar `inference_signals.tsv` is append-only from creation. Initialise
# the 12-column header on first cycle if the file does not yet exist.
if [[ ! -f "$INFERENCE_SIGNALS" ]]; then
    mkdir -p "$(dirname "$INFERENCE_SIGNALS")"
    printf '%s\n' "$INFERENCE_SIGNALS_HEADER" > "$INFERENCE_SIGNALS"
fi

# ── 4b. Type-inference smoke gate (finetype-7zi ac-06) ─────────────────
# Smoke fixture: canonical email column. Locked weights w_v=0.4, w_h=0.6.
# Math (overlap-on-min header match): h={email}∩{identity,person,email}/
# min(1,3) = 1.0; v=1.0 (validator passes canonical addresses); score = 1.0.
# Rule 8 (prediction_confirmed) fires because predicted==inferred AND
# validator pass-rate ≥ 0.7. Failure halts the cycle (H05 family).
if [[ -x "$FINETYPE_BIN" ]]; then
    SMOKE_INPUT='{"column_name":"email","predicted_type":"identity.person.email","samples":["alice@example.com","bob@example.com","carol@example.com","dave@example.org","eve@example.net","frank@example.com","grace@example.io","henry@example.com"]}'
    SMOKE_OUT="$(printf '%s\n' "$SMOKE_INPUT" | "$FINETYPE_BIN" infer --mode column --batch --explain 2>/dev/null || true)"
    SMOKE_OK="$(printf '%s' "$SMOKE_OUT" | python3 -c "
import json, sys
try:
    out = json.loads(sys.stdin.read())
except Exception as e:
    print(f'parse_error:{e}'); sys.exit(0)
if out.get('inferred_correct_type') != 'identity.person.email':
    print(f\"wrong_type:{out.get('inferred_correct_type')}\"); sys.exit(0)
conf = float(out.get('confidence', 0.0))
if conf < 0.5:
    print(f'low_confidence:{conf}'); sys.exit(0)
if out.get('mechanism') != 'prediction_confirmed':
    print(f\"wrong_mechanism:{out.get('mechanism')}\"); sys.exit(0)
print('ok')
")"
    if [[ "$SMOKE_OK" != "ok" ]]; then
        emit_halt "H05" "infer --explain smoke gate failed: $SMOKE_OK (raw output: $SMOKE_OUT)"
        exit 1
    fi
fi

# ── 5. Emit preamble JSON ─────────────────────────────────────────────
python3 - <<PY
import json
print(json.dumps({
    "status": "ok",
    "cycle_id": "$CYCLE_ID",
    "cycle_start": "$CYCLE_START",
    "contract_path": "$CONTRACT",
    "contract_sha256": "$CONTRACT_SHA",
    "lockfile": "$LOCKFILE",
    "free_disk_gb_start": $FREE_GB,
    "failure_log_lines_before": $FAIL_LINES_BEFORE,
    "coverage_log_lines_before": $COV_LINES_BEFORE,
}))
PY
