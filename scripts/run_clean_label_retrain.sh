#!/usr/bin/env bash
# run_clean_label_retrain.sh — spec 2026-06-28-clean-label-retrain
#
# Single-seed retrain on the clean-label blend (vocab-membership labels for the semantic
# families) holding the SHIPPED architecture + Sharpen FIXED. One variable: the training
# labels for geo/person. Go/no-go = composed gold (reframe) vs the s43 bar recorded in
# evidence/fixtures.json for whichever gold fixture version is checked out — the bar is
# looked up, never typed in here, because a float copied into a script goes stale the
# moment the ground truth under it is re-adjudicated.
#
# Two encoders on the SAME clean blend:
#   - 8M (primary): config m2v8m-244-config.json — the shipped architecture exactly.
#   - 4M (speed track): config m2v-244-config.json — does a clean blend make the smaller,
#     latency-cheaper encoder viable at equal composed accuracy?
#
# Idempotent: skips any artefact that already exists. Resumable.
set -eo pipefail
cd "$(cd "$(dirname "$0")/.." && pwd)"
export HF_HUB_OFFLINE=1 TRANSFORMERS_OFFLINE=1 HF_HUB_DISABLE_TELEMETRY=1 PYTHONUNBUFFERED=1

PY="eval/gittables/.venv/bin/python"
BIN="./target/release/finetype"
PM="./target/release/predict_multibranch"
GOLD="eval/gold/gold_corpus.tsv"
COLS="eval/gittables/corpus_pass/columns.parquet"
BLEND="output/clean-label-retrain/clean_label_blend.csv.gz"
SEED=42
LOG="output/clean-label-retrain/retrain.log"
exec > >(tee -a "$LOG") 2>&1

echo "================ CLEAN-LABEL RETRAIN — $(date) ================"
[[ -f "$BLEND" ]] || { echo "FAIL: missing clean blend $BLEND"; exit 1; }

train_one() {  # <potion> <config> <train_ftmb> <model_out> <gold_ftmb>
  local potion="$1" config="$2" ftmb="$3" out="$4" goldftmb="$5"
  echo "--- [$out] build training FTMB ($potion) ---"
  if [[ ! -f "$ftmb" ]]; then
    "$PY" scripts/build_ftmb_v5_potion.py --potion "$potion" \
      --distilled "$BLEND" --output "$ftmb" --workers 8
  else echo "skip (exists): $ftmb"; fi

  # Pre-flight: config.valid_dim MUST equal the FTMB's valid_dim, else the validation
  # branch matmul mismatches and the whole train wastes hours (taxonomy-drift guard).
  local cfg_vd ftmb_vd
  cfg_vd=$("$PY" -c "import json;print(json.load(open('$config'))['valid_dim'])")
  ftmb_vd=$("$PY" -c "import struct;f=open('$ftmb','rb');f.read(4);f.read(4);f.read(8);f.read(8);f.read(4);print(struct.unpack('<H',f.read(2))[0])")
  if [[ "$cfg_vd" != "$ftmb_vd" ]]; then
    echo "FAIL: config valid_dim=$cfg_vd != FTMB valid_dim=$ftmb_vd ($config vs $ftmb)"; exit 1
  fi
  echo "[$out] valid_dim OK ($cfg_vd)"

  echo "--- [$out] train seed $SEED ---"
  if [[ ! -f "$out/model.safetensors" ]]; then
    "$BIN" train-multi-branch --data "$ftmb" --output "$out" --model-config "$config" \
      --epochs 100 --batch-size 32 --lr 0.0001 --weight-decay 0.0001 --dropout 0.35 \
      --seed "$SEED" --head flat --patience 15
    # post-train: inject type_index_keys (label-order contract for validation branch)
    local saved="$out/config.json"
    if [[ -f "$saved" ]] && ! grep -q '"type_index_keys"' "$saved"; then
      local keys
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
  else echo "skip (exists): $out/model.safetensors"; fi

  echo "--- [$out] build gold FTMB ($potion) ---"
  if [[ ! -f "$goldftmb" ]]; then
    "$PY" scripts/build_gold_ftmb_potion.py --gold "$GOLD" --columns "$COLS" --binary "$BIN" \
      --potion "$potion" --out "$goldftmb"
  else echo "skip (exists): $goldftmb"; fi
}

# Taxonomy drifted 244->245 since s43 (datetime.offset.timezone_abbreviation, a Sharpen-
# recovered leaf with ZERO training rows). The recorded baseline was measured on THIS
# 245-binary (its Sharpen recovers the 6 gold tz_abbr columns), so 245 is the reference
# environment. We train the candidate at 245 — the extra leaf is inert (no training data,
# Sharpen-recovered in both baseline and candidate) so it does not affect the geo/person
# variable under test.

# The go/no-go bar: the shipped model's composed score, looked up in the fixture manifest
# under whichever gold version is checked out. If gold has moved since the bar was measured,
# score_clean_label.sh refuses the comparison instead of judging this candidate against
# ground truth that no longer exists.
BAR="m2v8m-s43/composed-reframe/0.6.53"

# ---- 8M primary (shipped architecture) — DECISIVE go/no-go ----
train_one minishlab/potion-base-8M models/m2v8m-245-config.json \
  output/clean-label-retrain/clean_m2v8m-244.ftmb models/clean8m-s42 \
  output/clean-label-retrain/gold_m2v8m_245.ftmb
echo "================ DECISIVE: 8M composed(reframe) vs $BAR — $(date) ================"
scripts/score_clean_label.sh models/clean8m-s42 output/clean-label-retrain/gold_m2v8m_245.ftmb \
  clean8m --baseline "$BAR"

# ---- 4M speed track (does clean data make the smaller encoder viable?) ----
train_one minishlab/potion-base-4M models/m2v-245-config.json \
  output/clean-label-retrain/clean_m2v4m-244.ftmb models/clean4m-s42 \
  output/clean-label-retrain/gold_m2v4m.ftmb
echo "================ SPEED TRACK: 4M composed(reframe) — $(date) ================"
scripts/score_clean_label.sh models/clean4m-s42 output/clean-label-retrain/gold_m2v4m.ftmb \
  clean4m --baseline "$BAR"
echo "================ DONE — $(date) ================"
