#!/usr/bin/env bash
# Download the active model from HuggingFace.
# Reads models/default symlink to determine which model version to fetch.
set -euo pipefail

REPO="https://huggingface.co/noon-org/finetype-char-cnn/resolve/main"
MODEL_DIR=$(readlink models/default)

echo "Active model: ${MODEL_DIR}"
mkdir -p "models/${MODEL_DIR}"
cd "models/${MODEL_DIR}"

for file in model.safetensors labels.json config.yaml; do
  echo "  Downloading ${file}..."
  curl -sfLO "${REPO}/${MODEL_DIR}/${file}"
done

echo "Model files:"
ls -la
