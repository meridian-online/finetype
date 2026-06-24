#!/usr/bin/env bash
# scripts/overnight_potion.sh — spec 2026-06-21 ac-02 (bigger-static embed frontier)
#
# Swap the embed branch for a bigger potion (8M/32M, single- or two-view) and measure
# Sense-vs-Sense + composed vs the m2v-244 baseline (FRESH-vs-FRESH, best-of-3).
#
# The embed is computed in PYTHON (build_ftmb_v5_potion.py) at full width to dodge the
# Rust EMBED_DIM=128 truncation; scoring is OFFLINE via predict_multibranch on a potion
# gold FTMB (native `finetype profile` would truncate). One-variable change: embed only —
# 27 stats, 244 valid, v19 data blend, all identical to m2v-244.
#
# Usage:
#   ./scripts/overnight_potion.sh                                  # potion-8M single-view
#   ./scripts/overnight_potion.sh --potion minishlab/potion-base-32M --config models/m2v32m-244-config.json --tag m2v32m
#   ./scripts/overnight_potion.sh --potion minishlab/potion-base-4M --two-view minishlab/potion-base-32M \
#        --config models/m2v-tv-4m32m-244-config.json --tag m2v-tv-4m32m
#   ./scripts/overnight_potion.sh --score-only        # reuse models, just rescore
set -eo pipefail
cd "$(cd "$(dirname "$0")/.." && pwd)"

# Models are fully cached. Force OFFLINE so model2vec's load never makes a network call —
# a flaky HF check hung the first 8M run (main thread blocked on a lock, socket CLOSE_WAIT).
# PYTHONUNBUFFERED so the builder's progress prints are visible through the tee pipe.
export HF_HUB_OFFLINE=1 TRANSFORMERS_OFFLINE=1 HF_HUB_DISABLE_TELEMETRY=1 PYTHONUNBUFFERED=1

PY="eval/gittables/.venv/bin/python"
BIN="./target/release/finetype"
PM="./target/release/predict_multibranch"
GOLD="eval/gold/gold_corpus.tsv"
COLS="eval/gittables/corpus_pass/columns.parquet"

POTION="minishlab/potion-base-8M"
TWO_VIEW=""
CONFIG="models/m2v8m-244-config.json"
TAG="m2v8m"
EPOCHS=100
SCORE_ONLY=false
SEEDS=(42 43 44)
# Override seeds via env for a fast 1-seed pilot, e.g. OVERRIDE_SEEDS="42".
[[ -n "${OVERRIDE_SEEDS:-}" ]] && read -ra SEEDS <<< "$OVERRIDE_SEEDS"

# Per-value attention (choice 0106): when > 0, the builder stores up to this many
# raw value strings per record and writes FTMB v6 instead of v5. 0 = off.
STORE_VALUES="${STORE_VALUES:-0}"
# Local model2vec dir the trainer uses to encode the v6 value strings into per-value
# embeddings (must match the value-branch encoder; potion-8M is staged at models/m2v8m).
VALUE_ENCODER="${VALUE_ENCODER:-models/m2v8m}"

# baseline (ac-01): m2v-244-s44 native Sense 0.521 / composed 0.769
BASE_SENSE="0.521"; BASE_COMPOSED="0.769"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --potion)     POTION="$2"; shift 2 ;;
    --two-view)   TWO_VIEW="$2"; shift 2 ;;
    --config)     CONFIG="$2"; shift 2 ;;
    --tag)        TAG="$2"; shift 2 ;;
    --epochs)     EPOCHS="$2"; shift 2 ;;
    --score-only) SCORE_ONLY=true; shift ;;
    -h|--help)    sed -n '2,/^set -/p' "$0" | grep '^#' | sed 's/^# \?//'; exit 0 ;;
    *) echo "Unknown option: $1" >&2; exit 1 ;;
  esac
done

FTMB="output/multibranch-training/${TAG}-244.ftmb"
GOLDFTMB="output/embed-frontier/gold_${TAG}.ftmb"
RESULT="output/embed-frontier/ac02_${TAG}_result.md"
LOG="output/embed-frontier/overnight_${TAG}.log"
mkdir -p output/embed-frontier output/multibranch-training
exec > >(tee -a "$LOG") 2>&1

EXPECT_EMBED=$("$PY" -c "import json;print(json.load(open('$CONFIG'))['embed_dim'])")
TV_ARG=(); [[ -n "$TWO_VIEW" ]] && TV_ARG=(--two-view "$TWO_VIEW")

echo "================ POTION OVERNIGHT [$TAG] — $(date) ================"
echo " potion: $POTION ${TWO_VIEW:+++ $TWO_VIEW}"
echo " config: $CONFIG (embed_dim=$EXPECT_EMBED)   seeds: ${SEEDS[*]}   epochs: $EPOCHS"
echo " baseline m2v-244-s44: Sense $BASE_SENSE / composed $BASE_COMPOSED"

echo "--- STEP 0: build metal binaries ---"
cargo build --bin finetype -p finetype-cli --no-default-features --features metal --release
cargo build --bin predict_multibranch -p finetype-train --no-default-features --features metal --release
LIVE_TAX=$("$BIN" taxonomy 2>/dev/null | grep -cE "^[a-z]")
[[ "$LIVE_TAX" == "244" ]] || { echo "FAIL: live taxonomy $LIVE_TAX != 244"; exit 1; }
for f in "$CONFIG" "$GOLD" "$COLS"; do [[ -e "$f" ]] || { echo "FAIL: missing $f"; exit 1; }; done

if [[ "$SCORE_ONLY" != "true" ]]; then
  echo "--- STEP 1: build training FTMB (potion embed, full width) ---"
  SV_ARG=(); [[ "$STORE_VALUES" -gt 0 ]] && SV_ARG=(--store-values "$STORE_VALUES")
  [[ "$STORE_VALUES" -gt 0 ]] && echo "  per-value attention: storing $STORE_VALUES values/record (FTMB v6)"
  if [[ ! -f "$FTMB" ]]; then
    "$PY" scripts/build_ftmb_v5_potion.py --potion "$POTION" "${TV_ARG[@]}" "${SV_ARG[@]}" --output "$FTMB" --workers 8
  fi
  # GATE: dims + truncation. A misaligned binary doesn't crash training — it wastes the night.
  VERIFY_OUT=$("$PY" scripts/read_ftmb.py "$FTMB" --stats --verify 2>&1) || { echo "$VERIFY_OUT"; echo "FAIL: read_ftmb verify"; exit 1; }
  echo "$VERIFY_OUT" | grep -q "WARNING: EOF" && { echo "FAIL: FTMB truncated (EOF)"; exit 1; }
  "$PY" - "$FTMB" "$EXPECT_EMBED" "$STORE_VALUES" <<'PYGATE'
import struct, sys
path, expect, store_values = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])
with open(path, "rb") as f:
    assert f.read(4) == b"FTMB"
    ver, = struct.unpack("<I", f.read(4)); n, = struct.unpack("<Q", f.read(8))
    cd, ed, sd, hd = struct.unpack("<HHHH", f.read(8)); f.read(4); vd, = struct.unpack("<H", f.read(2))
print(f"[gate] v{ver} records={n} char={cd} embed={ed} stats={sd} header={hd} valid={vd}")
assert ed == expect, f"FAIL embed={ed} != config {expect} (truncation?)"
assert sd == 27, f"FAIL stats={sd} != 27"
assert vd == 244, f"FAIL valid={vd} != 244"
# per-value attention (choice 0106): a v6 build MUST report stored values, else
# the night trains a value-attention model on empty value lists.
if store_values > 0:
    assert ver == 6, f"FAIL version={ver} != 6 (STORE_VALUES={store_values} but not v6)"
    import subprocess
    out = subprocess.run([sys.executable, "scripts/read_ftmb.py", path, "--records", "20"],
                         capture_output=True, text=True).stdout
    n_with = sum(1 for ln in out.splitlines()
                 if ln.lstrip().startswith("values :") and "n=0 " not in ln)
    assert n_with > 0, "FAIL v6 build stored zero values on all sampled records"
    print(f"[gate] v6 values OK ({n_with}/20 sampled records carry values)")
print("[gate] dims OK")
PYGATE

  echo "--- STEP 2: 3-seed train ---"
  for S in "${SEEDS[@]}"; do
    OUT="models/${TAG}-s${S}"
    [[ -f "$OUT/model.safetensors" ]] && { echo "skip $OUT (exists)"; continue; }
    echo "[train] seed $S -> $OUT  ($(date))"
    VE_ARG=(); [[ "$STORE_VALUES" -gt 0 ]] && VE_ARG=(--value-encoder "$VALUE_ENCODER")
    "$BIN" train-multi-branch --data "$FTMB" --output "$OUT" --model-config "$CONFIG" \
      --epochs "$EPOCHS" --batch-size 32 --lr 0.0001 --weight-decay 0.0001 --dropout 0.35 \
      --seed "$S" --head flat --patience 15 "${VE_ARG[@]}"

    # Post-train: inject type_index_keys (label-order contract for the validation
    # branch at inference). WITHOUT this the native classifier feeds the validation
    # branch zeros and mis-predicts — the bug that made potion-8M look like an 11pp
    # gold regression. Mirrors overnight_m2v_244.sh. (finetype now also derives this
    # from the live taxonomy as a fallback, but persisting pins the order.)
    SAVED_CONFIG="$OUT/config.json"
    if [[ -f "$SAVED_CONFIG" ]] && ! grep -q '"type_index_keys"' "$SAVED_CONFIG"; then
      TYPE_KEYS=$(echo '["test"]' | "$BIN" extract-features --json --header "test" --validation 2>/dev/null | \
        "$PY" -c "import json,sys; print(json.dumps(json.load(sys.stdin)['type_index_keys']))" 2>/dev/null)
      if [[ -n "$TYPE_KEYS" && "$TYPE_KEYS" != "null" ]]; then
        "$PY" -c "
import json
c=json.load(open('$SAVED_CONFIG')); c['type_index_keys']=json.loads('''$TYPE_KEYS''')
json.dump(c,open('$SAVED_CONFIG','w'),indent=2); open('$SAVED_CONFIG','a').write('\n')
print(f'Injected {len(c[\"type_index_keys\"])} type_index_keys')
"
      fi
    fi
  done
fi

echo "--- STEP 3: build gold FTMB (potion embed) ---"
SV_ARG=(); [[ "$STORE_VALUES" -gt 0 ]] && SV_ARG=(--store-values "$STORE_VALUES")
if [[ ! -f "$GOLDFTMB" ]]; then
  "$PY" scripts/build_gold_ftmb_potion.py --gold "$GOLD" --columns "$COLS" --binary "$BIN" \
    --potion "$POTION" "${TV_ARG[@]}" "${SV_ARG[@]}" --out "$GOLDFTMB"
fi

echo "--- STEP 4: score each seed (Sense + composed) ---"
score_one() {  # <preds.tsv> <reframe?> -> headline number
  local preds="$1"; local rf="$2"; rm -f /tmp/potscore/report_*.md 2>/dev/null
  "$PY" scripts/score_gold_anchor.py score --gold "$GOLD" --predictions "$preds" \
    --model-name t --out-dir /tmp/potscore $rf >/dev/null 2>&1 || true
  grep -hE "Headline" /tmp/potscore/report_*.md 2>/dev/null | head -1 | grep -oE "[0-9]+\.[0-9]+" | head -1
}
mkdir -p /tmp/potscore output/embed-frontier/preds
{
echo "# ac-02 [$TAG] — bigger-static embed vs m2v-244 ($(date))"
echo
echo "potion: $POTION ${TWO_VIEW:+++ $TWO_VIEW} | embed_dim $EXPECT_EMBED | Sense-vs-Sense (offline, no truncation)"
echo "baseline m2v-244-s44: Sense $BASE_SENSE / composed $BASE_COMPOSED  (v19: Sense 0.502 / composed 0.793)"
echo
echo "| seed | Sense | composed |"
echo "|---|---|---|"
for S in "${SEEDS[@]}"; do
  OUT="models/${TAG}-s${S}"
  [[ -f "$OUT/model.safetensors" ]] || { echo "| s$S | MISSING | MISSING |"; continue; }
  raw="/tmp/${TAG}_s${S}_raw.tsv"; sense="output/embed-frontier/preds/${TAG}-s${S}_sense.tsv"
  comp="output/embed-frontier/preds/${TAG}-s${S}_composed.tsv"
  PM_VE=(); [[ "$STORE_VALUES" -gt 0 ]] && PM_VE=(--value-encoder "$VALUE_ENCODER")
  "$PM" --model "$OUT" --data "$GOLDFTMB" --out "$raw" "${PM_VE[@]}"
  { echo -e "file_content_sha256\tcolumn_name\tpredicted_label\tconfidence";
    tail -n +2 "$raw" | awk -F'\t' '{split($1,a,"\037"); print a[1]"\t"a[2]"\t"$2"\t"$3}'; } > "$sense"
  "$PY" scripts/compose_predictions.py --preds "$sense" --out "$comp" >/dev/null 2>&1 || cp "$sense" "$comp"
  s=$(score_one "$sense" ""); c=$(score_one "$comp" "")
  echo "| s$S | ${s:-ERR} | ${c:-ERR} |"
done
echo
echo "Decision (pre-registered, ac02_readiness.md): GO if best-of-3 Sense > $BASE_SENSE + CI."
echo "Latency: potion encode ~free (~0.5 ms/col flat 4M->32M, prep); native confirm at ship."
echo "Done $(date)"
} | tee "$RESULT"
echo "================ DONE [$TAG] — $RESULT ================"
