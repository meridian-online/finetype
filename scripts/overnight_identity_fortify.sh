#!/usr/bin/env bash
# scripts/overnight_identity_fortify.sh — identity fortification retrain
# (spec 2026-06-18-identity-header-hint-fortification ac-03).
#
# Trains 3 seeds x 50 epochs on the identity FTMB (v3 base + author/login
# header-paired identity additions). Additive over v19's training distribution
# (v3 distilled — the exact base shipped v19 used), so the only delta is the
# identity data. Drift proxy GATE: GO at 25 epochs (ac-02) — user_agent
# converged, identity in-band (username +0.12pp, full_name +1.23pp).
#
# TRAINING ONLY — no profile/eval step, so duckdb is NOT needed here (training
# reads the binary FTMB). The post-train gates (gold, corpus-honest) run
# separately tomorrow with duckdb on PATH. The type_index_keys config patch per
# seed is required so the saved model profiles correctly later.
set -uo pipefail
cd "$(dirname "$0")/.."
export PATH="$HOME/.duckdb/cli/1.5.3:$PATH"   # defensive: in case any step shells duckdb

FTMB="output/multibranch-training/identity-blend.ftmb"
CONFIG="models/sherlock-v13-config.json"
BIN="target/release/finetype"
EPOCHS=50
SEEDS=(42 43 44)
LOG="output/identity-fortification/overnight.log"
mkdir -p output/identity-fortification
exec > >(tee -a "$LOG") 2>&1

echo "================================================================"
echo " identity fortification retrain — 3 seeds x ${EPOCHS}ep — start $(date)"
echo " FTMB: $FTMB"
echo "================================================================"
[[ -f "$FTMB" ]] || { echo "FAIL: FTMB missing"; exit 1; }

for SEED in "${SEEDS[@]}"; do
    MODEL_DIR="models/sherlock-identity-fortify-relu-s${SEED}"
    echo ""; echo "── seed ${SEED} -> ${MODEL_DIR} — $(date) ──"
    if [[ -f "$MODEL_DIR/model.safetensors" ]]; then
        echo "  reusing existing $MODEL_DIR (delete to retrain)"
    else
        cargo run --bin finetype --no-default-features --features metal --release -- \
            train-multi-branch \
            --data "$FTMB" \
            --output "$MODEL_DIR" \
            --model-config "$CONFIG" \
            --epochs "$EPOCHS" \
            --batch-size 32 \
            --lr 0.0001 \
            --weight-decay 0.0001 \
            --dropout 0.35 \
            --seed "$SEED" \
            --head flat \
            --patience 99 2>&1 | tail -12
        # type_index_keys patch so the saved model profiles correctly.
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
                echo "  patched type_index_keys"
            fi
        fi
    fi
    [[ -f "$MODEL_DIR/results.json" ]] && python3 -c "import json; r=json.load(open('$MODEL_DIR/results.json')); print(f\"  seed ${SEED}: best_val_acc={r.get('best_val_accuracy', r.get('val_accuracy','?'))}\")" 2>/dev/null
done

echo ""
echo "================================================================"
echo " identity fortification retrain DONE — $(date)"
echo " models: models/sherlock-identity-fortify-relu-s{42,43,44}"
echo " NEXT (awake, duckdb on PATH): pick best seed by val_acc, then the"
echo "   ac-04 gold(--reframe)+author-relocation gate and ac-05 corpus-honest."
echo "================================================================"
