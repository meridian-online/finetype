#!/usr/bin/env bash
# scripts/overnight_mfg_localefmt.sh — locale-format mining-factory candidate train.
#
# Spec 2026-06-07-reference-data-mining-factory ac-05/ac-06. Trains the
# locale-format manufactured-blend candidate on the ALREADY-BUILT FTMB
# (output/multibranch-training/mfg-blend-localefmt.ftmb), v19 ReLU recipe verbatim:
# 3 seeds x ReLU+BN, sherlock-v13-config, 100 epochs / patience 15, lr/wd 1e-4,
# dropout 0.35, head flat. Mirrors scripts/overnight_v19_paired.sh's train
# invocation; does NOT rebuild the FTMB (that was built + proxy-gated already).
#
# The destination-drift proxy was NO-GO on two minor boundaries (json_array prose
# relabel + small isbn over-emit); the author authorised a documented exception to
# take this candidate to the real blocking gate (corpus-honest). See memory
# mfg-localefmt-proxy-exception.
#
# Usage: scripts/overnight_mfg_localefmt.sh
set -eo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_DIR"

FTMB_FILE="output/multibranch-training/mfg-blend-localefmt.ftmb"
RELU_CONFIG="models/sherlock-v13-config.json"
EPOCHS=100
PATIENCE=15
BATCH_SIZE=32
SEEDS=(42 43 44)
BIN="$PROJECT_DIR/target/release/finetype"
LOG="output/mining-factory/locale-format/train.log"

exec > >(tee -a "$LOG") 2>&1

echo "================================================================"
echo " locale-format candidate train — 3 ReLU seeds, v19 recipe"
echo " FTMB:   $FTMB_FILE"
echo " Config: $RELU_CONFIG  epochs=$EPOCHS patience=$PATIENCE batch=$BATCH_SIZE"
echo " Started: $(date)"
echo "================================================================"

[[ -f "$FTMB_FILE" ]] || { echo "FAIL: FTMB not found: $FTMB_FILE"; exit 2; }

echo "[build] cargo build (metal release)..."
cargo build --bin finetype --no-default-features --features metal --release 2>&1 | tail -2

for seed in "${SEEDS[@]}"; do
    name="sherlock-mfg-localefmt-relu-s${seed}"
    MODEL_DIR="models/${name}"
    echo ""
    echo "────────────────────────────────────────────────────────────"
    echo " Seed ${seed} -> ${MODEL_DIR}   $(date)"
    echo "────────────────────────────────────────────────────────────"
    if [[ -f "$MODEL_DIR/model.safetensors" ]]; then
        echo "[skip] exists — delete to retrain"
        continue
    fi
    START=$(date +%s)
    "$BIN" train-multi-branch \
        --data "$FTMB_FILE" \
        --output "$MODEL_DIR" \
        --model-config "$RELU_CONFIG" \
        --epochs "$EPOCHS" \
        --batch-size "$BATCH_SIZE" \
        --lr 0.0001 \
        --weight-decay 0.0001 \
        --dropout 0.35 \
        --seed "$seed" \
        --head flat \
        --patience "$PATIENCE"
    ELAPSED=$(( ($(date +%s) - START) / 60 ))
    echo "[seed ${seed}] trained in ${ELAPSED} min"

    # type_index_keys patch (same as overnight_v19_paired.sh) so the model profiles.
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
print(f'  injected {len(c[\"type_index_keys\"])} type_index_keys')
"
        fi
    fi

    # Best val accuracy from results.json.
    RES="$MODEL_DIR/results.json"
    if [[ -f "$RES" ]]; then
        python3 -c "
import json
r=json.load(open('$RES'))
rows=r if isinstance(r,list) else r.get('epochs',r.get('history',[]))
best=max(rows,key=lambda e:e.get('val_accuracy',0)) if rows else {}
print(f'  best val_acc={best.get(\"val_accuracy\",0)*100:.2f}% @ epoch {best.get(\"epoch\",\"?\")}')" 2>/dev/null || true
    fi
done

echo ""
echo "================================================================"
echo " All seeds done: $(date)"
for seed in "${SEEDS[@]}"; do
    echo "  models/sherlock-mfg-localefmt-relu-s${seed}"
done
echo "================================================================"
