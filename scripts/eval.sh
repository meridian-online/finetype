#!/usr/bin/env bash
# scripts/eval.sh — Evaluate a trained model against the full eval suite
#
# Usage:
#   ./scripts/eval.sh                        # Evaluate current default model
#   ./scripts/eval.sh --model models/char-cnn-v13  # Evaluate a specific model
#   ./scripts/eval.sh --help
set -euo pipefail

# ─── Defaults ───────────────────────────────────────────────────────
MODEL_DIR=""

# ─── Usage ──────────────────────────────────────────────────────────
usage() {
    cat <<'USAGE'
Usage: scripts/eval.sh [OPTIONS]

Run the full evaluation suite (profile + actionability + report) against a model.

Options:
  --model DIR    Model directory to evaluate (default: models/default)
  --help         Show this help

When --model is specified, the script temporarily re-points models/default to
the target model, runs the eval suite, then restores the original symlink.

Output:
  eval/eval_output/profile_results.csv        Profile predictions
  eval/eval_output/actionability_results.csv  Actionability scores
  eval/eval_output/report.md                  Unified dashboard

Examples:
  ./scripts/eval.sh                                # Eval current default
  ./scripts/eval.sh --model models/char-cnn-v13    # Eval specific model
USAGE
    exit 0
}

# ─── Parse arguments ────────────────────────────────────────────────
while [[ $# -gt 0 ]]; do
    case "$1" in
        --model)  MODEL_DIR="$2"; shift 2 ;;
        --help|-h) usage ;;
        *) echo "Unknown option: $1" >&2; exit 1 ;;
    esac
done

# ─── Resolve model ──────────────────────────────────────────────────
SYMLINK_PATH="models/default"
ORIGINAL_TARGET=""
NEEDS_RESTORE=false

if [[ -n "$MODEL_DIR" ]]; then
    # Validate model directory
    if [[ ! -d "$MODEL_DIR" ]]; then
        echo "Error: Model directory not found: ${MODEL_DIR}" >&2
        exit 1
    fi
    if [[ ! -f "${MODEL_DIR}/model.safetensors" ]]; then
        echo "Error: No model.safetensors in ${MODEL_DIR}" >&2
        exit 1
    fi

    # Save and replace symlink
    if [[ -L "$SYMLINK_PATH" ]]; then
        ORIGINAL_TARGET="$(readlink "$SYMLINK_PATH")"
    elif [[ -e "$SYMLINK_PATH" ]]; then
        echo "Error: models/default exists but is not a symlink" >&2
        exit 1
    fi

    # Get relative path for symlink
    MODEL_BASENAME="$(basename "$MODEL_DIR")"
    rm -f "$SYMLINK_PATH"
    ln -s "$MODEL_BASENAME" "$SYMLINK_PATH"
    NEEDS_RESTORE=true
    echo "Evaluating: ${MODEL_DIR} (temporarily linked as default)"
else
    if [[ -L "$SYMLINK_PATH" ]]; then
        ACTUAL="$(readlink "$SYMLINK_PATH")"
        echo "Evaluating: models/${ACTUAL} (current default)"
    else
        echo "Evaluating: models/default"
    fi
fi

# ─── Cleanup trap ───────────────────────────────────────────────────
cleanup() {
    if [[ "$NEEDS_RESTORE" == true ]] && [[ -n "$ORIGINAL_TARGET" ]]; then
        rm -f "$SYMLINK_PATH"
        ln -s "$ORIGINAL_TARGET" "$SYMLINK_PATH"
        echo ""
        echo "Restored models/default -> ${ORIGINAL_TARGET}"
    fi
}
trap cleanup EXIT

# ─── Auto-detect model type ───────────────────────────────────
# If the model directory contains label_map.json + config.json with
# multi-branch fields, export FINETYPE_MODEL_TYPE for profile_eval.sh.
ACTUAL_MODEL_DIR="${MODEL_DIR:-models/default}"
if [[ -L "$ACTUAL_MODEL_DIR" ]]; then
    ACTUAL_MODEL_DIR="models/$(readlink "$ACTUAL_MODEL_DIR")"
fi
if [[ -f "${ACTUAL_MODEL_DIR}/label_map.json" ]] && [[ -f "${ACTUAL_MODEL_DIR}/config.json" ]]; then
    if grep -q '"char_dim"' "${ACTUAL_MODEL_DIR}/config.json" 2>/dev/null; then
        export FINETYPE_MODEL_TYPE="multi-branch"
        echo "Auto-detected model type: multi-branch"
    fi
fi

echo ""

# ─── Step 1: Profile eval ──────────────────────────────────────────
echo "[1/3] Running profile evaluation..."
make eval-profile 2>&1
echo ""

# ─── Step 2: Actionability eval ────────────────────────────────────
echo "[2/3] Running actionability evaluation..."
make eval-actionability 2>&1
echo ""

# ─── Step 3: Report generation ─────────────────────────────────────
echo "[3/3] Generating evaluation report..."
# eval-report depends on eval-profile and eval-actionability, but we already ran them
# Run the report binary directly to avoid re-running prerequisites
EVAL_RUN="cargo run --release -p finetype-eval --bin"
${EVAL_RUN} eval-report -- \
    --profile-results eval/eval_output/profile_results.csv \
    --actionability-results eval/eval_output/actionability_results.csv \
    --labels-dir labels \
    --output eval/eval_output/report.md 2>&1
echo ""

# ─── Summary ────────────────────────────────────────────────────────
echo "========================================"
echo "Evaluation complete"
echo "========================================"

# Extract key metrics from report
REPORT="eval/eval_output/report.md"
if [[ -f "$REPORT" ]]; then
    # Try to extract label accuracy, domain accuracy, and actionability
    LABEL_ACC=$(grep -oP 'Label accuracy.*?(\d+\.?\d*%)' "$REPORT" 2>/dev/null | head -1 || true)
    DOMAIN_ACC=$(grep -oP 'Domain accuracy.*?(\d+\.?\d*%)' "$REPORT" 2>/dev/null | head -1 || true)
    ACTION=$(grep -oP 'Actionability.*?(\d+\.?\d*%)' "$REPORT" 2>/dev/null | head -1 || true)

    if [[ -n "$LABEL_ACC" ]]; then echo "  $LABEL_ACC"; fi
    if [[ -n "$DOMAIN_ACC" ]]; then echo "  $DOMAIN_ACC"; fi
    if [[ -n "$ACTION" ]]; then echo "  $ACTION"; fi

    echo ""
    echo "Full report: ${REPORT}"
fi
