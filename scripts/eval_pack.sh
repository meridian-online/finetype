#!/usr/bin/env bash
# scripts/eval_pack.sh — Pack model evaluation artifacts for transfer
#
# Compresses all artifacts needed to analyse a training run into a single
# tar.gz file. Designed for transferring results from Mac (training) to
# Beelink (analysis) without hunting for individual files.
#
# Contents of the pack:
#   <model>/config.json           — Model architecture config
#   <model>/results.json          — Per-epoch training metrics
#   <model>/label_map.json        — Label index → type name mapping
#   <model>/eval/                 — Full eval suite output
#     profile_results.json        — Profile eval summary (label/domain accuracy)
#     profile_results.csv         — Per-column predictions
#     actionability_results.csv   — Actionability scores
#     ground_truth.csv            — Ground truth for comparison
#     report.md                   — Unified eval dashboard
#   overnight-*.log               — Training log (if found in results/)
#
# Usage:
#   ./scripts/eval_pack.sh models/sherlock-v6-gelu
#   ./scripts/eval_pack.sh models/sherlock-v6-gelu-conservative
#
# Output:
#   results/eval-pack-<model-name>.tar.gz
#
# Unpacking (on target machine):
#   cd /path/to/finetype
#   tar xzf eval-pack-sherlock-v6-gelu-conservative.tar.gz
#   # Creates: models/<name>/eval/*, models/<name>/config.json, etc.
set -euo pipefail

if [[ $# -lt 1 ]] || [[ "$1" == "--help" ]] || [[ "$1" == "-h" ]]; then
    echo "Usage: scripts/eval_pack.sh <model-dir>"
    echo ""
    echo "Pack evaluation artifacts into results/eval-pack-<model-name>.tar.gz"
    echo ""
    echo "Examples:"
    echo "  ./scripts/eval_pack.sh models/sherlock-v6-gelu"
    echo "  ./scripts/eval_pack.sh models/sherlock-v6-gelu-conservative"
    exit 0
fi

MODEL_DIR="$1"
MODEL_NAME="$(basename "$MODEL_DIR")"

if [[ ! -d "$MODEL_DIR" ]]; then
    echo "Error: Model directory not found: $MODEL_DIR" >&2
    exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_DIR"

mkdir -p results

PACK_FILE="results/eval-pack-${MODEL_NAME}.tar.gz"

# Build file list — only include files that exist
FILES=()

# Model config files (lightweight, always useful)
for f in config.json label_map.json results.json; do
    if [[ -f "$MODEL_DIR/$f" ]]; then
        FILES+=("$MODEL_DIR/$f")
    fi
done

# Eval directory (the main payload)
if [[ -d "$MODEL_DIR/eval" ]]; then
    # Add all files in eval/
    while IFS= read -r -d '' file; do
        FILES+=("$file")
    done < <(find "$MODEL_DIR/eval" -type f -print0)
else
    echo "WARN: No eval directory at $MODEL_DIR/eval/ — pack will be config-only"
fi

# Training log (match by model name patterns)
for log in results/overnight-*"${MODEL_NAME}"*.log results/overnight-*.log; do
    if [[ -f "$log" ]]; then
        FILES+=("$log")
        break  # Take the first matching log
    fi
done

if [[ ${#FILES[@]} -eq 0 ]]; then
    echo "Error: No artifacts found to pack for $MODEL_DIR" >&2
    exit 1
fi

# Create the archive
# Use relative paths from project root so unpacking preserves directory structure
tar czf "$PACK_FILE" "${FILES[@]}"

# Summary
PACK_SIZE="$(ls -lh "$PACK_FILE" | awk '{print $5}')"
FILE_COUNT="${#FILES[@]}"

echo "[Eval Pack] Created: $PACK_FILE ($PACK_SIZE, $FILE_COUNT files)"
echo ""
echo "Contents:"
tar tzf "$PACK_FILE" | sed 's/^/  /'
echo ""
echo "To unpack on target machine:"
echo "  cd /path/to/finetype && tar xzf $PACK_FILE"
