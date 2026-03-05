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
            FEATURES="metal"
            DEVICE_NAME="Metal"
            # Try to get chip name
            local chip
            chip="$(sysctl -n machdep.cpu.brand_string 2>/dev/null || echo "Apple Silicon")"
            DEVICE_DETAIL="$chip"
            ;;
        Linux)
            if command -v nvidia-smi &>/dev/null && nvidia-smi &>/dev/null; then
                FEATURES="cuda"
                DEVICE_NAME="CUDA"
                DEVICE_DETAIL="$(nvidia-smi --query-gpu=name --format=csv,noheader 2>/dev/null | head -1 || echo "NVIDIA GPU")"
            else
                FEATURES="cpu"
                DEVICE_NAME="CPU"
                DEVICE_DETAIL="$(lscpu 2>/dev/null | grep 'Model name' | sed 's/.*: *//' || echo "Unknown")"
            fi
            ;;
        *)
            FEATURES="cpu"
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
    cargo run -p finetype-cli --features "${FEATURES}" --release -- generate \
        --samples "$SAMPLES" \
        --seed "$SEED" \
        --output "$TRAINING_DATA" 2>&1 | while IFS= read -r line; do
        # Show build progress, suppress verbose output
        if [[ "$line" == *"Compiling"* ]] || [[ "$line" == *"Finished"* ]]; then
            echo "  $line"
        fi
    done
    echo "  Done: $(wc -l < "$TRAINING_DATA") training samples"
fi

# ─── Step 2: Build CLI with correct features ───────────────────────
echo ""
echo "[2/3] Building with --features ${FEATURES}..."
cargo build -p finetype-cli --features "${FEATURES}" --release 2>&1 | while IFS= read -r line; do
    if [[ "$line" == *"Compiling"* ]] || [[ "$line" == *"Finished"* ]]; then
        echo "  $line"
    fi
done

# ─── Step 3: Train ─────────────────────────────────────────────────
echo ""
echo "[3/3] Training CharCNN..."
mkdir -p "$MODEL_DIR"

# Build the train command
TRAIN_CMD=(
    cargo run -p finetype-cli --features "${FEATURES}" --release --
    train
    --data "$TRAINING_DATA"
    --output "$MODEL_DIR"
    --epochs "$EPOCHS"
    --seed "$SEED"
    --device "${DEVICE_NAME,,}"  # lowercase
)

EPOCH_START_TIME=""
TRAIN_START_TIME=$(date +%s)
TOTAL_EPOCHS="$EPOCHS"

# Run training, parsing stderr for progress display
"${TRAIN_CMD[@]}" 2>&1 | tee "$LOG_FILE" | while IFS= read -r line; do
    # Parse epoch completion lines
    if [[ "$line" =~ Epoch\ ([0-9]+)/([0-9]+):\ loss=([0-9.]+),\ accuracy=([0-9.]+)% ]]; then
        epoch="${BASH_REMATCH[1]}"
        total="${BASH_REMATCH[2]}"
        loss="${BASH_REMATCH[3]}"
        acc="${BASH_REMATCH[4]}"

        now=$(date +%s)
        elapsed=$((now - TRAIN_START_TIME))
        elapsed_min=$((elapsed / 60))
        elapsed_sec=$((elapsed % 60))

        if [[ "$epoch" -gt 0 ]]; then
            per_epoch=$((elapsed / epoch))
            remaining=$(( per_epoch * (total - epoch) ))
            eta_min=$((remaining / 60))
            eta_sec=$((remaining % 60))
            eta_str="${eta_min}m$(printf '%02d' ${eta_sec})s"
        else
            eta_str="--"
        fi

        # Progress bar
        filled=$((epoch * 40 / total))
        empty=$((40 - filled))
        bar=$(printf '%0.s=' $(seq 1 $filled 2>/dev/null) || true)
        space=$(printf '%0.s ' $(seq 1 $empty 2>/dev/null) || true)

        printf '\r  [%s%s] %d/%d  loss=%.4f  acc=%.1f%%  elapsed=%dm%02ds  ETA=%s     ' \
            "$bar" "$space" "$epoch" "$total" "$loss" "$acc" \
            "$elapsed_min" "$elapsed_sec" "$eta_str"
    elif [[ "$line" == *"Starting epoch"* ]]; then
        # Just mark epoch start, don't print (the completion line is more useful)
        :
    elif [[ "$line" == *"Compiling"* ]] || [[ "$line" == *"Finished"* ]]; then
        echo "  $line"
    elif [[ "$line" == *"Batch"* ]]; then
        # Suppress batch-level output (too noisy)
        :
    elif [[ "$line" == *"Saved"* ]] || [[ "$line" == *"saved"* ]] || [[ "$line" == *"Best"* ]]; then
        echo ""
        echo "  $line"
    elif [[ "$line" == *"WARNING"* ]] || [[ "$line" == *"warning"* ]]; then
        :  # Suppress build warnings
    elif [[ "$line" == *"model"* ]] || [[ "$line" == *"Model"* ]]; then
        echo "  $line"
    fi
done

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
