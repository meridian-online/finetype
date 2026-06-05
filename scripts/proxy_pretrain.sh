#!/usr/bin/env bash
# scripts/proxy_pretrain.sh — cheap destination-drift pre-check
# (spec 2026-06-05-destination-drift-precheck ac-02/ac-03).
#
# The expensive instrument is a full 3-seed / 50-epoch overnight retrain
# followed by a corpus pass. This proxy answers ONE cheaper question first:
# will the candidate hard-negative blend destabilise an UNTARGETED label
# boundary? It trains a single seed for a handful of epochs on the candidate's
# ALREADY-BUILT FTMB, snapshots that proxy model's Sense distribution on the
# fixed corpus file list, and runs the calibrated drift_report.py gate against
# the pre-train baseline snapshot. GO/NO-GO falls straight out of the gate's
# exit code.
#
# RANKING fidelity, not converged accuracy, is the goal: a 10-epoch proxy does
# not match the 50-epoch model's val_acc, but ac-03 shows it preserves the
# DIRECTION and RANK of the destination drift — v23's categorical explosion and
# v24's coordinate.latitude explosion both surface at proxy depth. So the proxy
# is a true pre-filter: a NO-GO here means do not spend the overnight run.
#
# Not forked from scripts/overnight_v24_numeric.sh — it reuses the same
# train-multi-branch invocation and the same FTMB the overnight run consumes;
# it just trims seeds (3 -> 1) and epochs (50 -> --epochs, default 10) and
# bolts the snapshot+drift gate onto the end.
#
# Usage:
#   scripts/proxy_pretrain.sh --name v24-proxy \
#       --ftmb output/multibranch-training/v24-numeric-blend.ftmb \
#       --baseline output/destination-drift-precheck/sense_dist_v19fx_s42.json
#
#   # knobs: --epochs N (default 10), --seed S (default 42),
#   #        --config PATH (default models/sherlock-v13-config.json),
#   #        --file-list PATH (default the s42 fixed list),
#   #        --out-dir DIR (default output/destination-drift-precheck)
#
# Exit: 0 = GO (no untargeted boundary tripped), 1 = NO-GO (drift detected),
#       2 = usage/precondition error.
set -eo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_DIR"

NAME=""
FTMB=""
BASELINE=""
EPOCHS=10
SEED=42
CONFIG="models/sherlock-v13-config.json"
OUT_DIR="output/destination-drift-precheck"
FILE_LIST="output/destination-drift-precheck/sense_dist_v19fx_s42.files.txt"
BATCH_SIZE=32
PATIENCE=99   # do not early-stop before the proxy epoch budget is spent

while [[ $# -gt 0 ]]; do
    case "$1" in
        --name)      NAME="$2"; shift 2 ;;
        --ftmb)      FTMB="$2"; shift 2 ;;
        --baseline)  BASELINE="$2"; shift 2 ;;
        --epochs)    EPOCHS="$2"; shift 2 ;;
        --seed)      SEED="$2"; shift 2 ;;
        --config)    CONFIG="$2"; shift 2 ;;
        --file-list) FILE_LIST="$2"; shift 2 ;;
        --out-dir)   OUT_DIR="$2"; shift 2 ;;
        --help|-h)   sed -n '2,/^set -/p' "$0" | grep '^#' | sed 's/^# \?//'; exit 0 ;;
        *) echo "Unknown option: $1" >&2; exit 2 ;;
    esac
done

[[ -z "$NAME" || -z "$FTMB" || -z "$BASELINE" ]] && {
    echo "FAIL: --name, --ftmb and --baseline are required" >&2; exit 2; }
[[ -f "$FTMB" ]]      || { echo "FAIL: FTMB not found: $FTMB" >&2; exit 2; }
[[ -f "$BASELINE" ]]  || { echo "FAIL: baseline snapshot not found: $BASELINE" >&2; exit 2; }
[[ -f "$CONFIG" ]]    || { echo "FAIL: config not found: $CONFIG" >&2; exit 2; }
[[ -f "$FILE_LIST" ]] || { echo "FAIL: file list not found: $FILE_LIST" >&2; exit 2; }

mkdir -p "$OUT_DIR"
MODEL_DIR="models/sherlock-${NAME}-s${SEED}"
PROXY_SNAP="$OUT_DIR/sense_dist_${NAME}.json"
DRIFT_JSON="$OUT_DIR/proxy_drift_${NAME}.json"

echo "================================================================"
echo " Destination-drift proxy pre-check: $NAME"
echo " FTMB:      $FTMB"
echo " Baseline:  $BASELINE"
echo " Proxy:     seed=$SEED epochs=$EPOCHS config=$CONFIG"
echo " Model dir: $MODEL_DIR"
echo "================================================================"

echo "[1/4] Building finetype (Metal)..."
cargo build --bin finetype --no-default-features --features metal --release 2>&1 | tail -2
BIN="$PROJECT_DIR/target/release/finetype"

echo "[2/4] Training proxy (single seed, $EPOCHS epochs)..."
TRAIN_START=$(date +%s)
if [[ -f "$MODEL_DIR/model.safetensors" ]]; then
    echo "  reusing existing $MODEL_DIR (delete to retrain)"
else
    cargo run --bin finetype --no-default-features --features metal --release -- \
        train-multi-branch \
        --data "$FTMB" \
        --output "$MODEL_DIR" \
        --model-config "$CONFIG" \
        --epochs "$EPOCHS" \
        --batch-size "$BATCH_SIZE" \
        --lr 0.0001 \
        --weight-decay 0.0001 \
        --dropout 0.35 \
        --seed "$SEED" \
        --head flat \
        --patience "$PATIENCE" 2>&1 | tail -8
    # type_index_keys patch (same as the overnight script) so the saved model
    # profiles correctly.
    SAVED_CONFIG="$MODEL_DIR/config.json"
    if [[ -f "$SAVED_CONFIG" ]] && ! grep -q '"type_index_keys"' "$SAVED_CONFIG"; then
        TYPE_KEYS=$(echo '["test"]' | "$BIN" extract-features --json --header "test" --validation 2>/dev/null | \
            python3 -c "import json,sys; print(json.dumps(json.load(sys.stdin)['type_index_keys']))" 2>/dev/null)
        if [[ -n "$TYPE_KEYS" && "$TYPE_KEYS" != "null" ]]; then
            python3 -c "
import json
with open('$SAVED_CONFIG') as f: c = json.load(f)
c['type_index_keys'] = json.loads('$TYPE_KEYS')
with open('$SAVED_CONFIG','w') as f: json.dump(c, f, indent=2); f.write('\n')
"
        fi
    fi
fi
TRAIN_END=$(date +%s)
TRAIN_MIN=$(( (TRAIN_END - TRAIN_START) / 60 ))
echo "  proxy train wall-clock: ${TRAIN_MIN} min (single seed, $EPOCHS epochs)"

echo "[3/4] Snapshotting proxy Sense distribution on the fixed file list..."
FINETYPE_BIN="$BIN" FINETYPE_MODEL="$MODEL_DIR" \
    python3 scripts/snapshot_sense_distribution.py \
        --label "$NAME" --file-list "$FILE_LIST" --out-dir "$OUT_DIR" >/dev/null
mv -f "$OUT_DIR/sense_dist_${NAME}.json" "$PROXY_SNAP" 2>/dev/null || true

echo "[4/4] Drift gate (calibrated band) vs baseline..."
set +e
python3 scripts/drift_report.py "$BASELINE" "$PROXY_SNAP" --json "$DRIFT_JSON"
GATE_RC=$?
set -e

echo ""
echo "================================================================"
echo " Proxy pre-check $NAME: $([ $GATE_RC -eq 0 ] && echo GO || echo NO-GO)"
echo " proxy train wall-clock: ${TRAIN_MIN} min   drift report: $DRIFT_JSON"
echo "================================================================"
exit $GATE_RC
