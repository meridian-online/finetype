#!/usr/bin/env bash
# scripts/train.sh — End-to-end CharCNN training with hardware detection
#
# Usage:
#   ./scripts/train.sh --samples 1000 --size small --epochs 5
#   ./scripts/train.sh --samples 5000 --size large --epochs 15 --seed 42
#   ./scripts/train.sh --help
set -euo pipefail

# ─── Defaults ───────────────────────────────────────────────────────
SAMPLES=1000
SIZE="small"
EPOCHS=10
SEED=42
EMBED_DIM=""
NUM_FILTERS=""
HIDDEN_DIM=""
MODEL_NAME=""
DATA_FILE=""

# ─── Architecture presets (bash 3.2 compatible — no associative arrays) ──
preset_values() {
    # Usage: preset_values <size> → sets PRESET_EMBED PRESET_FILTERS PRESET_HIDDEN
    case "$1" in
        small)  PRESET_EMBED=32;  PRESET_FILTERS=64;  PRESET_HIDDEN=128 ;;
        medium) PRESET_EMBED=64;  PRESET_FILTERS=128; PRESET_HIDDEN=256 ;;
        large)  PRESET_EMBED=128; PRESET_FILTERS=256; PRESET_HIDDEN=512 ;;
        *) return 1 ;;
    esac
}

# ─── Usage ──────────────────────────────────────────────────────────
usage() {
    cat <<'USAGE'
Usage: scripts/train.sh [OPTIONS]

End-to-end CharCNN training: generate data, build with correct features, train model.

Options:
  --samples N         Samples per type for data generation (default: 1000)
  --size PRESET       Architecture preset: small|medium|large (default: small)
  --epochs N          Number of training epochs (default: 10)
  --seed N            Random seed (default: 42)
  --embed-dim N       Override embedding dimension
  --num-filters N     Override number of CNN filters
  --hidden-dim N      Override hidden layer dimension
  --model-name NAME   Output model directory name (default: auto char-cnn-vN)
  --data FILE         Use existing NDJSON training data (skip generation)
  --help              Show this help

Architecture presets:
  small   embed=32   filters=64   hidden=128
  medium  embed=64   filters=128  hidden=256
  large   embed=128  filters=256  hidden=512

Examples:
  ./scripts/train.sh --samples 100 --size small --epochs 2    # Quick test
  ./scripts/train.sh --samples 5000 --size large --epochs 15  # Full training (M1 Metal)
USAGE
    exit 0
}

# ─── Parse arguments ────────────────────────────────────────────────
while [[ $# -gt 0 ]]; do
    case "$1" in
        --samples)   SAMPLES="$2";     shift 2 ;;
        --size)      SIZE="$2";        shift 2 ;;
        --epochs)    EPOCHS="$2";      shift 2 ;;
        --seed)      SEED="$2";        shift 2 ;;
        --embed-dim) EMBED_DIM="$2";   shift 2 ;;
        --num-filters) NUM_FILTERS="$2"; shift 2 ;;
        --hidden-dim) HIDDEN_DIM="$2"; shift 2 ;;
        --model-name) MODEL_NAME="$2"; shift 2 ;;
        --data)      DATA_FILE="$2";   shift 2 ;;
        --help|-h)   usage ;;
        *) echo "Unknown option: $1" >&2; exit 1 ;;
    esac
done

# Validate size preset and resolve defaults
if ! preset_values "$SIZE"; then
    echo "Error: Unknown size preset '$SIZE'. Use small, medium, or large." >&2
    exit 1
fi

# Resolve architecture params (overrides take precedence)
EMBED_DIM="${EMBED_DIM:-$PRESET_EMBED}"
NUM_FILTERS="${NUM_FILTERS:-$PRESET_FILTERS}"
HIDDEN_DIM="${HIDDEN_DIM:-$PRESET_HIDDEN}"

# ─── Hardware detection ─────────────────────────────────────────────
detect_hardware() {
    local os
    os="$(uname -s)"
    case "$os" in
        Darwin)
            CARGO_FEATURES="--no-default-features --features metal"
            DEVICE_NAME="Metal"
            # Try to get chip name
            local chip
            chip="$(sysctl -n machdep.cpu.brand_string 2>/dev/null || echo "Apple Silicon")"
            DEVICE_DETAIL="$chip"
            ;;
        Linux)
            if command -v nvidia-smi &>/dev/null && nvidia-smi &>/dev/null; then
                CARGO_FEATURES="--no-default-features --features cuda"
                DEVICE_NAME="CUDA"
                DEVICE_DETAIL="$(nvidia-smi --query-gpu=name --format=csv,noheader 2>/dev/null | head -1 || echo "NVIDIA GPU")"
            else
                CARGO_FEATURES="--no-default-features --features cpu"
                DEVICE_NAME="CPU"
                DEVICE_DETAIL="$(lscpu 2>/dev/null | grep 'Model name' | sed 's/.*: *//' || echo "Unknown")"
            fi
            ;;
        *)
            CARGO_FEATURES="--no-default-features --features cpu"
            DEVICE_NAME="CPU"
            DEVICE_DETAIL="Unknown"
            ;;
    esac
}

detect_hardware

# ─── Auto-detect model name ────────────────────────────────────────
if [[ -z "$MODEL_NAME" ]]; then
    # Find highest existing char-cnn-vN and increment (ignore snapshots)
    LATEST=$(ls -d models/char-cnn-v* 2>/dev/null | sed 's/.*char-cnn-v//' | grep -E '^[0-9]+$' | sort -n | tail -1 || echo "0")
    LATEST="${LATEST:-0}"
    NEXT=$((LATEST + 1))
    MODEL_NAME="char-cnn-v${NEXT}"
fi

MODEL_DIR="models/${MODEL_NAME}"
LOG_FILE="${MODEL_DIR}/train.log"

# ─── Banner ─────────────────────────────────────────────────────────
echo ""
echo "CharCNN Training -- ${DEVICE_NAME} (${DEVICE_DETAIL}) -- ${SIZE} (${EMBED_DIM}/${NUM_FILTERS}/${HIDDEN_DIM})"
echo "Model: ${MODEL_NAME} | Samples: ${SAMPLES}/type | Epochs: ${EPOCHS} | Seed: ${SEED}"
echo ""

# ─── Step 1: Generate training data ────────────────────────────────
if [[ -n "$DATA_FILE" ]]; then
    echo "[1/3] Using existing training data: ${DATA_FILE}"
    TRAINING_DATA="$DATA_FILE"
else
    TRAINING_DATA="training.ndjson"
    echo "[1/3] Generating training data (${SAMPLES} samples/type)..."
    cargo run -p finetype-cli ${CARGO_FEATURES} --release -- generate \
        --samples "$SAMPLES" \
        --seed "$SEED" \
        --output "$TRAINING_DATA"
    echo "  Done: $(wc -l < "$TRAINING_DATA") training samples"
fi

# ─── Step 2: Build CLI with correct features ───────────────────────
echo ""
echo "[2/3] Building with ${CARGO_FEATURES}..."
cargo build -p finetype-cli ${CARGO_FEATURES} --release

# ─── Step 3: Train ─────────────────────────────────────────────────
echo ""
echo "[3/3] Training CharCNN..."
mkdir -p "$MODEL_DIR"

# Build the train command
TRAIN_CMD=(
    cargo run -p finetype-cli ${CARGO_FEATURES} --release --
    train
    --data "$TRAINING_DATA"
    --output "$MODEL_DIR"
    --epochs "$EPOCHS"
    --seed "$SEED"
    --device "$(echo "$DEVICE_NAME" | tr '[:upper:]' '[:lower:]')"
)

TRAIN_START_TIME=$(date +%s)

# Run training — output goes to terminal and log file
"${TRAIN_CMD[@]}" 2>&1 | tee "$LOG_FILE"

echo ""
echo ""

# ─── Summary ────────────────────────────────────────────────────────
END_TIME=$(date +%s)
TOTAL_ELAPSED=$((END_TIME - TRAIN_START_TIME))
TOTAL_MIN=$((TOTAL_ELAPSED / 60))
TOTAL_SEC=$((TOTAL_ELAPSED % 60))

echo "Training complete: ${MODEL_DIR}"
echo "  Time: ${TOTAL_MIN}m${TOTAL_SEC}s"
echo "  Log: ${LOG_FILE}"
if [[ -f "${MODEL_DIR}/model.safetensors" ]]; then
    MODEL_SIZE=$(du -h "${MODEL_DIR}/model.safetensors" | cut -f1)
    echo "  Model size: ${MODEL_SIZE}"
fi
echo ""
echo "Next steps:"
echo "  ./scripts/eval.sh --model ${MODEL_DIR}    # Evaluate"
echo "  ./scripts/package.sh ${MODEL_DIR}          # Package for distribution"
