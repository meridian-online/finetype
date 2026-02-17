#!/usr/bin/env bash
# Download the active model from HuggingFace.
# Reads models/default symlink to determine which model version to fetch.
# Supports both flat models (single dir) and tiered models (multiple subdirs).
set -euo pipefail

REPO="https://huggingface.co/noon-org/finetype-char-cnn/resolve/main"
MODEL_DIR=$(readlink models/default)

echo "Active model: ${MODEL_DIR}"
mkdir -p "models/${MODEL_DIR}"

# Check if this is a tiered model by looking for a manifest file
MANIFEST_URL="${REPO}/${MODEL_DIR}/manifest.txt"
if curl -sfI "${MANIFEST_URL}" > /dev/null 2>&1; then
  # Tiered model: download manifest then fetch all listed files
  echo "  Detected tiered model — downloading manifest..."
  curl -sfL "${MANIFEST_URL}" -o "models/${MODEL_DIR}/manifest.txt"

  while IFS= read -r file; do
    [ -z "${file}" ] && continue
    dir=$(dirname "${file}")
    mkdir -p "models/${MODEL_DIR}/${dir}"
    echo "  Downloading ${file}..."
    curl -sfL "${REPO}/${MODEL_DIR}/${file}" -o "models/${MODEL_DIR}/${file}"
  done < "models/${MODEL_DIR}/manifest.txt"
else
  # Flat model: download 3 fixed files
  echo "  Flat model — downloading model files..."
  cd "models/${MODEL_DIR}"
  for file in model.safetensors labels.json config.yaml; do
    echo "  Downloading ${file}..."
    curl -sfLO "${REPO}/${MODEL_DIR}/${file}"
  done
  cd ../..
fi

echo "Model files:"
find "models/${MODEL_DIR}" -type f | sort
