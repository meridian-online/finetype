#!/usr/bin/env bash
# scripts/package.sh — Package a trained model for distribution
#
# Usage:
#   ./scripts/package.sh models/char-cnn-v13
#   ./scripts/package.sh models/char-cnn-v13 --output dist/
#   ./scripts/package.sh --help
set -euo pipefail

# ─── Defaults ───────────────────────────────────────────────────────
MODEL_DIR=""
OUTPUT_DIR="."

# ─── Usage ──────────────────────────────────────────────────────────
usage() {
    cat <<'USAGE'
Usage: scripts/package.sh MODEL_DIR [OPTIONS]

Bundle a trained model directory into a distributable .tar.gz archive.

Arguments:
  MODEL_DIR       Path to model directory (e.g., models/char-cnn-v13)

Options:
  --output DIR    Directory for the archive (default: current directory)
  --help          Show this help

The archive contains:
  model.safetensors   Trained model weights
  config.yaml         Model configuration
  labels.json         Label index
  manifest.json       Training provenance (if present)

Output:
  Prints file size and SHA256 checksum (useful for Homebrew tap updates).

Examples:
  ./scripts/package.sh models/char-cnn-v13
  ./scripts/package.sh models/char-cnn-v13 --output dist/
USAGE
    exit 0
}

# ─── Parse arguments ────────────────────────────────────────────────
while [[ $# -gt 0 ]]; do
    case "$1" in
        --output)  OUTPUT_DIR="$2"; shift 2 ;;
        --help|-h) usage ;;
        -*)        echo "Unknown option: $1" >&2; exit 1 ;;
        *)
            if [[ -z "$MODEL_DIR" ]]; then
                MODEL_DIR="$1"; shift
            else
                echo "Error: Unexpected argument: $1" >&2; exit 1
            fi
            ;;
    esac
done

if [[ -z "$MODEL_DIR" ]]; then
    echo "Error: MODEL_DIR is required" >&2
    echo "Usage: scripts/package.sh MODEL_DIR [--output DIR]" >&2
    exit 1
fi

# ─── Validate model directory ──────────────────────────────────────
# Resolve symlinks
MODEL_DIR="$(realpath "$MODEL_DIR")"

if [[ ! -d "$MODEL_DIR" ]]; then
    echo "Error: Not a directory: ${MODEL_DIR}" >&2
    exit 1
fi

MODEL_NAME="$(basename "$MODEL_DIR")"

# Required files
REQUIRED_FILES=(model.safetensors config.yaml labels.json)
OPTIONAL_FILES=(manifest.json)

for f in "${REQUIRED_FILES[@]}"; do
    if [[ ! -f "${MODEL_DIR}/${f}" ]]; then
        echo "Error: Missing required file: ${MODEL_DIR}/${f}" >&2
        exit 1
    fi
done

# ─── Build file list ───────────────────────────────────────────────
FILES_TO_PACK=()
for f in "${REQUIRED_FILES[@]}" "${OPTIONAL_FILES[@]}"; do
    if [[ -f "${MODEL_DIR}/${f}" ]]; then
        FILES_TO_PACK+=("${f}")
    fi
done

# ─── Create archive ────────────────────────────────────────────────
mkdir -p "$OUTPUT_DIR"
ARCHIVE_NAME="finetype-${MODEL_NAME}.tar.gz"
ARCHIVE_PATH="${OUTPUT_DIR}/${ARCHIVE_NAME}"

echo "Packaging ${MODEL_NAME}..."
echo "  Files: ${FILES_TO_PACK[*]}"

tar -czf "$ARCHIVE_PATH" -C "$MODEL_DIR" "${FILES_TO_PACK[@]}"

# ─── Summary ────────────────────────────────────────────────────────
FILE_SIZE=$(du -h "$ARCHIVE_PATH" | cut -f1)

# Compute SHA256
if command -v sha256sum &>/dev/null; then
    SHA256=$(sha256sum "$ARCHIVE_PATH" | cut -d' ' -f1)
elif command -v shasum &>/dev/null; then
    SHA256=$(shasum -a 256 "$ARCHIVE_PATH" | cut -d' ' -f1)
else
    SHA256="(sha256sum not available)"
fi

echo ""
echo "Archive: ${ARCHIVE_PATH}"
echo "  Size:   ${FILE_SIZE}"
echo "  SHA256: ${SHA256}"
echo ""
echo "Contents:"
tar -tzf "$ARCHIVE_PATH" | sed 's/^/  /'
echo ""
echo "To upload to HuggingFace:"
echo "  huggingface-cli upload meridian-online/finetype ${ARCHIVE_PATH}"
