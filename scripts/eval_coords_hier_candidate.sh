#!/usr/bin/env bash
# scripts/eval_localefmt_candidate.sh — ac-06 honest evaluation of the locale-format
# mining-factory candidate. Run AFTER overnight_mfg_localefmt.sh completes.
#
# Picks the best of the 3 ReLU seeds by val_accuracy, then runs the full ac-06 bar:
#   1. gold anchor  — per-family precision/recall vs v19 (efficacy)
#   2. Sense pre/post — snapshot candidate Sense dist + drift_report vs v19 (mandatory)
#   3. corpus pass  — candidate predictions on the 33k stratified sample
#   4. corpus-honest gate — BLOCKING relocation detector (H05)
#
# Emits a consolidated summary; the GO/NO-GO verdict is the corpus-honest gate's.
set -uo pipefail

cd "$(cd "$(dirname "$0")/.." && pwd)"
LFDIR="output/mining-factory/coord-only"
BIN="./target/release/finetype"
GOLD="eval/gold/gold_corpus_v1.tsv"
COLS="eval/gittables/corpus_pass/columns.parquet"
SEEDS=(42 43 44)
LOG="$LFDIR/eval.log"
# gold scorer + snapshot need pyarrow/duckdb from the gittables venv, not system python3.
VPY="eval/gittables/.venv/bin/python"
exec > >(tee -a "$LOG") 2>&1

echo "================================================================"
echo " ac-06 honest evaluation — coord-only candidate   $(date)"
echo "================================================================"

# ── Pick best seed by val_accuracy ──────────────────────────────────────
BEST_SEED=""; BEST_ACC=0
for s in "${SEEDS[@]}"; do
    R="models/sherlock-mfg-coords-hier-s${s}/results.json"
    [[ -f "$R" ]] || { echo "  seed $s: no results.json (skip)"; continue; }
    ACC=$(python3 -c "import json;r=json.load(open('$R'));print(max(e['val_accuracy'] for e in r))" 2>/dev/null)
    echo "  seed $s: best val_acc=$ACC"
    awk "BEGIN{exit !($ACC>$BEST_ACC)}" && { BEST_ACC=$ACC; BEST_SEED=$s; }
done
[[ -n "$BEST_SEED" ]] || { echo "FAIL: no trained seed found"; exit 2; }
MODEL="models/sherlock-mfg-coords-hier-s${BEST_SEED}"
echo ""
echo ">> Best seed: $BEST_SEED  (val_acc=$BEST_ACC)  -> $MODEL"
echo "$BEST_SEED" > "$LFDIR/best_seed.txt"

# ── 1. Gold anchor ──────────────────────────────────────────────────────
echo ""; echo "── [1/4] Gold anchor ──"
GOLD_PRED="$LFDIR/predictions_mfg-coords-hier.tsv"
FINETYPE_MODEL="$MODEL" "$VPY" scripts/score_gold_anchor.py predict \
    --gold "$GOLD" --columns "$COLS" --binary "$BIN" --out "$GOLD_PRED" || echo "  predict failed"
"$VPY" scripts/score_gold_anchor.py score \
    --gold "$GOLD" --predictions "$GOLD_PRED" \
    --model-name "mfg-coords-hier-s${BEST_SEED}" --out-dir "$LFDIR" || echo "  score failed"

# ── 2. Sense pre/post (mandatory post-train check) ──────────────────────
echo ""; echo "── [2/4] Sense distribution drift vs v19 ──"
FINETYPE_MODEL="$MODEL" "$VPY" scripts/snapshot_sense_distribution.py \
    --label mfg-coords-hier-full \
    --file-list output/destination-drift-precheck/sense_dist_v19fx_s42.files.txt \
    --out-dir "$LFDIR" || echo "  snapshot failed"
python3 scripts/drift_report.py \
    output/destination-drift-precheck/sense_dist_v19fx_s42.json \
    "$LFDIR/sense_dist_mfg-coords-hier-full.json" \
    --abs-floor 0.0040 --rel-mult 3.0 --direction up \
    > "$LFDIR/drift_report_full.txt" 2>&1 || echo "  (drift_report exit non-zero — see drift_report_full.txt)"
tail -25 "$LFDIR/drift_report_full.txt"

# ── 3. Candidate corpus pass on the stratified sample ───────────────────
echo ""; echo "── [3/4] Candidate corpus pass (33k stratified sample) ──"
# shellcheck disable=SC1091
source eval/gittables/.venv/bin/activate
FINETYPE_MODEL="$MODEL" python3 scripts/gittables_corpus_pass.py \
    --corpus-index output/corpus-honest-gate/stratified_sample.files.txt \
    --execute --jobs 8 --out-dir "$LFDIR/sample_pass_hier" || echo "  corpus pass failed"

# ── 4. Corpus-honest gate (BLOCKING) ────────────────────────────────────
echo ""; echo "── [4/4] Corpus-honest gate (H05, BLOCKING) ──"
CAND_PARQUET="$LFDIR/sample_pass_hier/corpus_pass/columns.parquet"
python3 scripts/corpus_honest_gate.py \
    --candidate "$CAND_PARQUET" \
    --label "mfg-coords-hier-s${BEST_SEED}" \
    | tee "$LFDIR/corpus_honest_gate.txt"

echo ""
echo "================================================================"
echo " ac-06 evaluation complete: $(date)"
echo " Best seed: $BEST_SEED  Model: $MODEL"
echo " Gold report:  $LFDIR/report_mfg-coords-hier-s${BEST_SEED}_*.md"
echo " Drift:        $LFDIR/drift_report_full.txt"
echo " Honest gate:  $LFDIR/corpus_honest_gate.txt  (this is the verdict)"
echo "================================================================"
