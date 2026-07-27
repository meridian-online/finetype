#!/usr/bin/env bash
# run_clean_label_augment.sh — the CLEAN test (spec 2026-06-28-clean-label-retrain).
#
# The REPLACE run regressed (composed 0.774 against the s43 bar of the day, both on the
# gold-2026-06-28 fixture) by swapping real semantic columns for synthetic clean positives —
# gold cratered on exactly the synthesised families (city 0.958->0.458, continent 1.000->
# 0.000). That confounds label-cleanliness with a synthetic distribution shift. AUGMENT keeps
# ALL real v3 (real-column formats preserved) and ADDS clean generator positives, isolating
# the label variable. Go/no-go = composed gold (reframe) against the s43 bar recorded in
# evidence/fixtures.json for the gold fixture version that is actually checked out.
#
# 8M only (shipped architecture); reuses the already-built 245 gold FTMB. Idempotent.
set -eo pipefail
cd "$(cd "$(dirname "$0")/.." && pwd)"
export HF_HUB_OFFLINE=1 TRANSFORMERS_OFFLINE=1 HF_HUB_DISABLE_TELEMETRY=1 PYTHONUNBUFFERED=1

PY="eval/gittables/.venv/bin/python"
BIN="./target/release/finetype"
BLEND="output/clean-label-retrain/clean_label_blend_augment.csv.gz"
FTMB="output/clean-label-retrain/augment_m2v8m-245.ftmb"
CONFIG="models/m2v8m-245-config.json"
OUT="models/clean8m-aug-s42"
GOLDFTMB="output/clean-label-retrain/gold_m2v8m_245.ftmb"
LOG="output/clean-label-retrain/augment.log"
exec > >(tee -a "$LOG") 2>&1

echo "================ CLEAN-LABEL AUGMENT (8M) — $(date) ================"
[[ -f "$BLEND" ]] || { echo "FAIL: missing $BLEND"; exit 1; }
[[ -f "$GOLDFTMB" ]] || { echo "FAIL: missing 245 gold FTMB $GOLDFTMB (build the REPLACE run first)"; exit 1; }

echo "--- build augment training FTMB (potion-8M) ---"
if [[ ! -f "$FTMB" ]]; then
  "$PY" scripts/build_ftmb_v5_potion.py --potion minishlab/potion-base-8M \
    --distilled "$BLEND" --output "$FTMB" --workers 8
else echo "skip (exists): $FTMB"; fi

# valid_dim guard (taxonomy-drift)
cfg_vd=$("$PY" -c "import json;print(json.load(open('$CONFIG'))['valid_dim'])")
ftmb_vd=$("$PY" -c "import struct;f=open('$FTMB','rb');f.read(4);f.read(4);f.read(8);f.read(8);f.read(4);print(struct.unpack('<H',f.read(2))[0])")
[[ "$cfg_vd" == "$ftmb_vd" ]] || { echo "FAIL: config valid_dim=$cfg_vd != FTMB valid_dim=$ftmb_vd"; exit 1; }
echo "valid_dim OK ($cfg_vd)"

echo "--- train seed 42 ---"
if [[ ! -f "$OUT/model.safetensors" ]]; then
  "$BIN" train-multi-branch --data "$FTMB" --output "$OUT" --model-config "$CONFIG" \
    --epochs 100 --batch-size 32 --lr 0.0001 --weight-decay 0.0001 --dropout 0.35 \
    --seed 42 --head flat --patience 15
  saved="$OUT/config.json"
  if [[ -f "$saved" ]] && ! grep -q '"type_index_keys"' "$saved"; then
    keys=$(echo '["test"]' | "$BIN" extract-features --json --header "test" --validation 2>/dev/null | \
      "$PY" -c "import json,sys; print(json.dumps(json.load(sys.stdin)['type_index_keys']))" 2>/dev/null)
    if [[ -n "$keys" && "$keys" != "null" ]]; then
      "$PY" -c "
import json
c=json.load(open('$saved')); c['type_index_keys']=json.loads('''$keys''')
json.dump(c,open('$saved','w'),indent=2); open('$saved','a').write('\n')
print(f'Injected {len(c[\"type_index_keys\"])} type_index_keys')"
    fi
  fi
else echo "skip (exists): $OUT/model.safetensors"; fi

BAR="m2v8m-s43/composed-reframe/0.6.53"
echo "================ DECISIVE (AUGMENT): 8M composed(reframe) vs $BAR — $(date) ================"
scripts/score_clean_label.sh "$OUT" "$GOLDFTMB" clean8m_aug --baseline "$BAR"
echo "================ DONE — $(date) ================"
