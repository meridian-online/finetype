#!/usr/bin/env bash
# scripts/eval_rule.sh — ONE parameterized eval harness for promotion candidates.
#
# Replaces six near-identical sed-copied scripts (eval_coord_guard, eval_state_code,
# eval_amount_veto, eval_coords_candidate, eval_coords_hier_candidate,
# eval_localefmt_candidate). Those copies drifted — one carried a double-substituted
# model path (sherlock-mfg-coords-hier-hier-s42) that failed an eval run, and another
# kept a stale header comment from its source. One script, two modes, no copy-paste.
#
# TWO MODES
#   --default          RULE eval. Candidate = the freshly-built binary + the shipped
#                      models/default (a Sharpen rule change, not a retrain). Baseline
#                      is whatever the corpus-honest gate reads as its stable oracle.
#                      Runs: gold anchor -> corpus pass -> corpus-honest gate (3 steps).
#
#   --model <prefix>   MODEL eval. Candidate = a trained model dir. Picks the best of
#     [--seeds a,b,c]  N seeds (by val_accuracy in results.json) among
#                      models/<prefix>-s<seed>, sets it as FINETYPE_MODEL, and ALSO runs
#                      the mandatory Sense-distribution drift check vs v19.
#                      Runs: gold anchor -> Sense drift -> corpus pass -> gate (4 steps).
#
# THE VERDICT is always the corpus-honest gate (H05, BLOCKING). Gold + drift are
# advisory inputs printed for the operator.
#
# VENV FOOTGUN (load-bearing): score_gold_anchor.py and snapshot_sense_distribution.py
# import pyarrow/duckdb, which are NOT in system python3 — only in the gittables venv.
# Using plain `python3` for those was a real bug. This script therefore:
#   * calls them via $VPY (eval/gittables/.venv/bin/python) explicitly, and
#   * `source`s the same venv before the corpus pass.
# Do not "simplify" these back to bare python3.
#
# USAGE
#   scripts/eval_rule.sh --label <name> --out-dir <dir> --default
#   scripts/eval_rule.sh --label <name> --out-dir <dir> --model <model-prefix> [--seeds 42,43,44]
#
#   --label   <name>    short name for the gate/gold reports (e.g. coord-guard)
#   --out-dir <dir>     output directory (created if absent), e.g. output/coord-promote-guard
#   --default           rule-eval mode: binary + models/default
#   --model   <prefix>  model-eval mode: pick best seed among models/<prefix>-s<seed>
#   --seeds   a,b,c     seeds to consider in model mode (default 42,43,44)
#   --gate-baseline <parquet>
#                       FRESH transition baseline for the corpus-honest gate:
#                       a columns.parquet from the SAME 33k sample run with the
#                       PRE-change binary, with ydf_prediction_gated joined on
#                       from output/ydf-validation-gate/v19_gated.parquet (the
#                       oracle is column-intrinsic, so the v19-era join is
#                       correct even though v19's PREDICTIONS are retired).
#                       REQUIRED for a valid rule-mode verdict since the m2v8m
#                       swap (2026-06-24): without it the gate defaults its
#                       transition baseline to the retired v19's predictions
#                       and the verdict measures cumulative post-swap history,
#                       not your change — a guaranteed false NO-GO (first hit:
#                       W1 of output/company-reference-audit/, 2026-07-04).
#   -h | --help         print this usage and exit 0

set -uo pipefail

usage() {
    sed -n '2,54p' "$0" | sed 's/^# \{0,1\}//'
}

# ── Argument parsing ────────────────────────────────────────────────────────
LABEL=""
OUT=""
MODEL_PREFIX=""
USE_DEFAULT=0
SEEDS_CSV="42,43,44"
GATE_BASELINE=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        --label)   LABEL="${2:-}"; shift 2 ;;
        --out-dir) OUT="${2:-}"; shift 2 ;;
        --model)   MODEL_PREFIX="${2:-}"; shift 2 ;;
        --seeds)   SEEDS_CSV="${2:-}"; shift 2 ;;
        --default) USE_DEFAULT=1; shift ;;
        --gate-baseline) GATE_BASELINE="${2:-}"; shift 2 ;;
        -h|--help) usage; exit 0 ;;
        *) echo "ERROR: unknown argument: $1" >&2; echo >&2; usage >&2; exit 64 ;;
    esac
done

# ── Validate the argument combination ───────────────────────────────────────
err=0
[[ -n "$LABEL" ]] || { echo "ERROR: --label is required" >&2; err=1; }
[[ -n "$OUT"   ]] || { echo "ERROR: --out-dir is required" >&2; err=1; }
if [[ "$USE_DEFAULT" -eq 1 && -n "$MODEL_PREFIX" ]]; then
    echo "ERROR: --default and --model are mutually exclusive" >&2; err=1
fi
if [[ "$USE_DEFAULT" -eq 0 && -z "$MODEL_PREFIX" ]]; then
    echo "ERROR: pick a mode — either --default or --model <prefix>" >&2; err=1
fi
if [[ "$err" -ne 0 ]]; then echo >&2; usage >&2; exit 64; fi

# ── Resolve repo root and shared paths ──────────────────────────────────────
cd "$(cd "$(dirname "$0")/.." && pwd)"
BIN="./target/release/finetype"
# gold scorer + snapshot need pyarrow/duckdb from the gittables venv, not system python3.
VPY="eval/gittables/.venv/bin/python"
GOLD="eval/gold/gold_corpus.tsv"   # re-adjudicated canonical fixture (spec 2026-06-16-mixed-teacher-gold-readjudication); v1 historical
COLS="eval/gittables/corpus_pass/columns.parquet"
SAMPLE_FILES="output/corpus-honest-gate/stratified_sample.files.txt"
DRIFT_BASELINE="output/destination-drift-precheck/sense_dist_v19fx_s42.json"
DRIFT_FILELIST="output/destination-drift-precheck/sense_dist_v19fx_s42.files.txt"

mkdir -p "$OUT"
LOG="$OUT/eval.log"; exec > >(tee -a "$LOG") 2>&1

# ── Mode-specific setup: choose the model the candidate runs ─────────────────
if [[ "$USE_DEFAULT" -eq 1 ]]; then
    MODE="rule"
    MODEL="models/default"
    SEED_TAG=""
    echo "================================================================"
    echo " RULE eval — '$LABEL'  (binary + models/default)   $(date)"
    echo "================================================================"
else
    MODE="model"
    # Pick the best seed by val_accuracy among models/<prefix>-s<seed>.
    IFS=',' read -r -a SEEDS <<< "$SEEDS_CSV"
    BEST_SEED=""; BEST_ACC=0
    for s in "${SEEDS[@]}"; do
        R="models/${MODEL_PREFIX}-s${s}/results.json"
        [[ -f "$R" ]] || { echo "  seed $s: no results.json at $R (skip)"; continue; }
        ACC=$("$VPY" -c "import json;r=json.load(open('$R'));print(max(e['val_accuracy'] for e in r))" 2>/dev/null)
        echo "  seed $s: best val_acc=$ACC"
        awk "BEGIN{exit !($ACC>$BEST_ACC)}" && { BEST_ACC=$ACC; BEST_SEED=$s; }
    done
    [[ -n "$BEST_SEED" ]] || { echo "FAIL: no trained seed found for models/${MODEL_PREFIX}-s<seed>"; exit 2; }
    MODEL="models/${MODEL_PREFIX}-s${BEST_SEED}"
    SEED_TAG="-s${BEST_SEED}"
    echo "================================================================"
    echo " MODEL eval — '$LABEL'  best seed $BEST_SEED (val_acc=$BEST_ACC)   $(date)"
    echo " -> $MODEL"
    echo "================================================================"
    echo "$BEST_SEED" > "$OUT/best_seed.txt"
fi

# ── 1. Gold anchor (efficacy: per-type precision/recall vs v19) ──────────────
echo ""; echo "── Gold anchor ──"
GOLD_PRED="$OUT/predictions_${LABEL}.tsv"
FINETYPE_MODEL="$MODEL" "$VPY" scripts/score_gold_anchor.py predict \
    --gold "$GOLD" --columns "$COLS" --binary "$BIN" \
    --out "$GOLD_PRED" || echo "  predict failed"

# Representative companion (card 0020 scenario 1): predict on the uniform-random
# representative fixture — the SAME corpus columns parquet ($COLS) is a superset, so
# no extra data source is needed — to surface its PRODUCTION headline next to the
# curated-hard gold number. If the predict fails, the companion is simply omitted.
REPR_GOLD="eval/repr/representative_corpus.tsv"
REPR_PRED="$OUT/predictions_${LABEL}_repr.tsv"
FINETYPE_MODEL="$MODEL" "$VPY" scripts/score_gold_anchor.py predict \
    --gold "$REPR_GOLD" --columns "$COLS" --binary "$BIN" \
    --out "$REPR_PRED" || echo "  repr predict failed (companion omitted)"
COMPANION_ARGS=()
[[ -s "$REPR_PRED" ]] && COMPANION_ARGS=(--companion-gold "$REPR_GOLD" --companion-predictions "$REPR_PRED")

# Reframe (spec 2026-06-17-enum-accuracy-reframe) is the PRIMARY headline: the gold
# fixture migrated `categorical` -> text residual, so the legacy scorer reports a
# phantom regression (model still emits `categorical` until the rules are retired in
# ac-03). The --reframe scorer collapses {categorical, word, plain_text} on both sides
# and is the coherent number. Legacy is kept below for continuity, clearly labelled.
echo "  [PRIMARY] enum-reframe scoring:"
"$VPY" scripts/score_gold_anchor.py score \
    --gold "$GOLD" --predictions "$GOLD_PRED" --reframe \
    ${COMPANION_ARGS[@]+"${COMPANION_ARGS[@]}"} \
    --model-name "${LABEL}${SEED_TAG}-reframe" --out-dir "$OUT" || echo "  reframe score failed"
echo "  [legacy — misleads until ac-03 retires the categorical Sharpen rules]:"
"$VPY" scripts/score_gold_anchor.py score \
    --gold "$GOLD" --predictions "$GOLD_PRED" \
    --model-name "${LABEL}${SEED_TAG}" --out-dir "$OUT" || echo "  score failed"

# ── 2. Sense-distribution drift vs v19 (model mode only; mandatory check) ────
if [[ "$MODE" == "model" ]]; then
    echo ""; echo "── Sense distribution drift vs v19 ──"
    SNAP_LABEL="${LABEL}-full"
    FINETYPE_MODEL="$MODEL" "$VPY" scripts/snapshot_sense_distribution.py \
        --label "$SNAP_LABEL" \
        --file-list "$DRIFT_FILELIST" \
        --out-dir "$OUT" || echo "  snapshot failed"
    "$VPY" scripts/drift_report.py \
        "$DRIFT_BASELINE" \
        "$OUT/sense_dist_${SNAP_LABEL}.json" \
        --abs-floor 0.0040 --rel-mult 3.0 --direction up \
        > "$OUT/drift_report_full.txt" 2>&1 \
        || echo "  (drift_report exit non-zero — see drift_report_full.txt)"
    tail -25 "$OUT/drift_report_full.txt"
fi

# ── 3. Candidate corpus pass (33k stratified sample) ─────────────────────────
echo ""; echo "── Candidate corpus pass (33k stratified sample) ──"
# gittables_corpus_pass.py has no resume: it refuses to run if files.parquet /
# columns.parquet already exist, and an INTERRUPTED prior pass leaves a partial,
# unreadable parquet that the gate then chokes on ("no magic bytes"). Always
# start from a clean output dir — these artifacts are fully regenerated here.
rm -rf "$OUT/sample_pass"
# shellcheck disable=SC1091
source eval/gittables/.venv/bin/activate   # pyarrow/duckdb live here, not in system python3
# --finetype-bin is REQUIRED: without it the corpus pass workers fall back to the
# PATH `finetype` (often a stale installed release), not the freshly-built $BIN the
# gold/drift steps use — a silent binary skew that 100%-errors the pass when the
# installed binary's CSV reader is incompatible with the current duckdb CLI.
FINETYPE_MODEL="$MODEL" python3 scripts/gittables_corpus_pass.py \
    --corpus-index "$SAMPLE_FILES" \
    --finetype-bin "$BIN" \
    --execute --jobs 8 --out-dir "$OUT/sample_pass" || echo "  corpus pass failed"

# ── 4. Corpus-honest gate (H05, BLOCKING — this is the verdict) ──────────────
echo ""; echo "── Corpus-honest gate (H05, BLOCKING) ──"
CAND_PARQUET="$OUT/sample_pass/corpus_pass/columns.parquet"
BASELINE_ARGS=()
if [[ -n "$GATE_BASELINE" ]]; then
    BASELINE_ARGS=(--baseline "$GATE_BASELINE")
elif [[ "$MODE" == "rule" ]]; then
    cat >&2 <<'EOWARN'
  ⚠️  WARNING: no --gate-baseline supplied. The gate's transition baseline
  defaults to the RETIRED v19's predictions, so in rule mode this verdict
  measures the cumulative post-swap history, NOT your rule change — a
  guaranteed false NO-GO since 2026-06-24 (see --help for the fresh-baseline
  protocol; first hit: company-reference audit W1). Treat the verdict below
  as INVALID for ship decisions until re-run with --gate-baseline.
EOWARN
fi
python3 scripts/corpus_honest_gate.py \
    --candidate "$CAND_PARQUET" \
    ${BASELINE_ARGS[@]+"${BASELINE_ARGS[@]}"} \
    --label "${LABEL}${SEED_TAG}" \
    | tee "$OUT/corpus_honest_gate.txt"

echo ""
echo "================================================================"
echo " eval complete: $(date)"
echo " Mode:        $MODE   Model: $MODEL"
[[ "$MODE" == "model" ]] && echo " Drift:       $OUT/drift_report_full.txt"
echo " Gold report: $OUT/report_${LABEL}${SEED_TAG}_*.md"
echo " Honest gate: $OUT/corpus_honest_gate.txt   (this is the verdict)"
echo "================================================================"
