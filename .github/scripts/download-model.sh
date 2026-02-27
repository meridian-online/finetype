#!/usr/bin/env bash
# Download the active model from HuggingFace.
# Reads models/default symlink to determine which model version to fetch.
# Supports both flat models (single dir) and tiered models (multiple subdirs).
set -euo pipefail

REPO="https://huggingface.co/noon-org/finetype-char-cnn/resolve/main"
# readlink works on Linux/macOS; fall back to cat for Windows where
# git may check out symlinks as plain text files.
MODEL_DIR=$(readlink models/default 2>/dev/null || cat models/default)
MODEL_DIR=$(echo "${MODEL_DIR}" | tr -d '\r')

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

# ── Model2Vec semantic hint classifier (optional) ──────────────────────────
# Download the Model2Vec artifacts for the semantic column name classifier.
# The build gracefully degrades if these are absent (HAS_MODEL2VEC=false).
echo ""
echo "Downloading Model2Vec semantic hint classifier..."
mkdir -p models/model2vec
M2V_OK=true
for file in model.safetensors type_embeddings.safetensors tokenizer.json label_index.json; do
  echo "  Downloading model2vec/${file}..."
  if ! curl -sfL "${REPO}/model2vec/${file}" -o "models/model2vec/${file}"; then
    echo "  WARNING: Failed to download model2vec/${file} — semantic hints will be disabled"
    M2V_OK=false
    break
  fi
done

if [ "${M2V_OK}" = true ]; then
  echo "Model2Vec files:"
  find models/model2vec -type f | sort
else
  echo "Model2Vec download failed — continuing without semantic hints"
  rm -rf models/model2vec
fi

# ── Entity classifier (optional) ────────────────────────────────────────
# Download the entity classifier model for entity_name demotion (Rule 18).
# The build gracefully degrades if these are absent (HAS_ENTITY_CLASSIFIER=false).
echo ""
echo "Downloading entity classifier model..."
mkdir -p models/entity-classifier
EC_OK=true
for file in model.safetensors config.json label_index.json; do
  echo "  Downloading entity-classifier/${file}..."
  if ! curl -sfL "${REPO}/entity-classifier/${file}" -o "models/entity-classifier/${file}"; then
    echo "  WARNING: Failed to download entity-classifier/${file} — entity demotion will be disabled"
    EC_OK=false
    break
  fi
done

if [ "${EC_OK}" = true ]; then
  echo "Entity classifier files:"
  find models/entity-classifier -type f | sort
else
  echo "Entity classifier download failed — continuing without entity demotion"
  rm -rf models/entity-classifier
fi
