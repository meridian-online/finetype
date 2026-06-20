#!/usr/bin/env bash
# Two-view embed overnight (spec 2026-06-20-gte-tiny-embed-branch-swap, ac-03 two-view bet).
#
# Embed slot = frozen gte-tiny (1536) ++ fine-tuned gte-small (1536) = 3072, so the
# multi-branch sees a format-preserving view AND a semantic view and learns to weight
# each per type. Chases the oracle ceiling (0.599 standalone) the ac-03 ablation found,
# vs the frozen floor (0.532). NO model-code change — trainer/predict are dim-agnostic.
#
# Pipeline: build binaries -> build two-view FTMB (guarded) -> 3-seed train -> build
# two-view gold FTMB -> score each seed standalone gold vs the floor. Idempotent: skips
# any step whose artefact already exists, so it is safe to re-run.
set -eo pipefail
cd "$(cd "$(dirname "$0")/.." && pwd)"

VENV=output/fine-tuned-encoder-discovery/.venv/bin/python
BIN=./target/release/finetype
FT=output/gte-tiny-embed-swap/gte_small_ft.pt
FTMB=output/multibranch-training/v5-two-view.ftmb
GOLDFTMB=output/gte-tiny-embed-swap/gold_twoview.ftmb
CFG=models/gte-config-3072.json
GOLD=eval/gold/gold_corpus.tsv
COLS=eval/gittables/corpus_pass/columns.parquet
LOG=output/gte-tiny-embed-swap/overnight_two_view.log
SEEDS=(42 43 44)
mkdir -p "$(dirname "$LOG")"
exec > >(tee -a "$LOG") 2>&1

echo "================ TWO-VIEW OVERNIGHT — $(date) ================"

echo "--- STEP 0: build metal binaries ---"
cargo build --bin finetype -p finetype-cli --no-default-features --features metal --release
cargo build --bin predict_multibranch -p finetype-train --no-default-features --features metal --release
for f in "$FT" "$CFG" "$GOLD" "$COLS"; do [ -e "$f" ] || { echo "FATAL: missing $f"; exit 1; }; done

echo "--- STEP 1: build two-view FTMB (frozen ++ ft, 3072) ---"
if [ ! -f "$FTMB" ]; then
  $VENV scripts/build_ftmb_v5_gte.py --two-view-ft "$FT" --output "$FTMB" --workers 8
fi
# Guard: a misaligned binary does not crash training — it silently wastes the night.
python3 - "$FTMB" <<'PY'
import struct, sys
f = open(sys.argv[1], "rb"); assert f.read(4) == b"FTMB"
ver, = struct.unpack("<I", f.read(4)); n, = struct.unpack("<Q", f.read(8))
cd, ed, sd, hd = struct.unpack("<HHHH", f.read(8)); f.read(4); vd, = struct.unpack("<H", f.read(2))
assert ed == 3072 and vd == 244, f"FATAL bad dims: embed={ed} valid={vd}"
print(f"GUARD OK: v{ver} {n} records, embed={ed}, valid={vd}")
PY

echo "--- STEP 2: 3-seed train ---"
for S in "${SEEDS[@]}"; do
  OUT=models/gte-twoview-s$S
  if [ -f "$OUT/model.safetensors" ]; then echo "skip $OUT (exists)"; continue; fi
  echo "[train] seed $S -> $OUT  ($(date))"
  $BIN train-multi-branch --data "$FTMB" --output "$OUT" --model-config "$CFG" \
    --epochs 100 --batch-size 32 --lr 0.0001 --weight-decay 0.0001 --dropout 0.35 \
    --seed "$S" --head flat --patience 15
done

echo "--- STEP 3: build two-view gold FTMB ---"
if [ ! -f "$GOLDFTMB" ]; then
  $VENV scripts/build_gold_ftmb.py --gold "$GOLD" --columns "$COLS" --binary "$BIN" \
    --two-view-ft "$FT" --out "$GOLDFTMB"
fi

echo "--- STEP 4: score each seed (standalone gold) ---"
echo "baseline: frozen floor 0.532 | release(ft) 0.518 | two-view oracle ceiling 0.599"
for S in "${SEEDS[@]}"; do
  OUT=models/gte-twoview-s$S
  [ -f "$OUT/model.safetensors" ] || { echo "seed $S: no model"; continue; }
  ./target/release/predict_multibranch --model "$OUT" --data "$GOLDFTMB" --out "/tmp/tv_s${S}_raw.tsv"
  { echo -e "file_content_sha256\tcolumn_name\tpredicted_label\tconfidence";
    tail -n +2 "/tmp/tv_s${S}_raw.tsv" | awk -F'\t' '{split($1,a,"\037"); print a[1]"\t"a[2]"\t"$2"\t"$3}'; } > "/tmp/tv_s${S}_pred.tsv"
  mkdir -p "/tmp/tvscore_s$S"
  H=$($VENV scripts/score_gold_anchor.py score --gold "$GOLD" --predictions "/tmp/tv_s${S}_pred.tsv" \
        --model-name "twoview-s$S" --out-dir "/tmp/tvscore_s$S" 2>&1 | grep -i "Headline")
  echo "SEED $S TWO-VIEW STANDALONE GOLD: $H"
done

echo "================ DONE — $(date) ================"
echo "Models: models/gte-twoview-s{42,43,44}  | scores above vs floor 0.532"
echo "GO if a seed clears the floor by >CI (~+0.03) toward 0.599 -> then ac-04 (compose)."
