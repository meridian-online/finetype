#!/usr/bin/env bash
# Download the active model from HuggingFace.
# Resolution precedence:
#   1. FINETYPE_CI_MODEL env var (if set AND non-empty after whitespace trim) — CI authoritative source.
#   2. readlink models/default — runtime default on Linux/macOS.
#   3. cat models/default — fallback for Windows, where git may check out
#      symlinks as plain text files.
# If all three yield an empty/whitespace name, the script exits non-zero
# with a clear error — no malformed URLs reach curl.
#
# FINETYPE_CI_MODEL is read ONLY by this script. CLI binary, MCP server,
# DuckDB extension, and all eval scripts ignore it. See spec
# specs/2026-04-20-ci-decouple-default-symlink/spec.yaml.
set -euo pipefail

REPO="https://huggingface.co/meridian-online/finetype-model/resolve/main"

# Use `${VAR:-}` (colon-dash) so empty string is treated as unset under `set -u`.
MODEL_DIR="${FINETYPE_CI_MODEL:-}"
if [ -z "${MODEL_DIR}" ]; then
  # Fall back to models/default. readlink on Linux/macOS, cat on Windows.
  MODEL_DIR=$(readlink models/default 2>/dev/null || cat models/default 2>/dev/null || true)
fi
# Strip CRLF (from Windows-checked-out plain-text symlinks) and surrounding whitespace.
MODEL_DIR=$(printf '%s' "${MODEL_DIR}" | tr -d '\r' | sed -e 's/^[[:space:]]*//' -e 's/[[:space:]]*$//')

if [ -z "${MODEL_DIR}" ]; then
  echo "ERROR: cannot resolve model — FINETYPE_CI_MODEL unset/empty AND models/default missing or empty." >&2
  echo "       Set FINETYPE_CI_MODEL=<model-dir> or create the models/default symlink before running this script." >&2
  exit 1
fi

echo "Active model: ${MODEL_DIR}"
mkdir -p "models/${MODEL_DIR}"

# Check if this is a tiered model by looking for a manifest file
MANIFEST_URL="${REPO}/${MODEL_DIR}/manifest.txt"
if curl -sfI --retry 3 --retry-all-errors "${MANIFEST_URL}" > /dev/null 2>&1; then
  # Tiered model: download manifest then fetch all listed files
  echo "  Detected tiered model — downloading manifest..."
  curl -sfL --retry 5 --retry-delay 2 --retry-all-errors --retry-connrefused "${MANIFEST_URL}" -o "models/${MODEL_DIR}/manifest.txt"

  while IFS= read -r file; do
    [ -z "${file}" ] && continue
    dir=$(dirname "${file}")
    mkdir -p "models/${MODEL_DIR}/${dir}"
    echo "  Downloading ${file}..."
    curl -sfL --retry 5 --retry-delay 2 --retry-all-errors --retry-connrefused "${REPO}/${MODEL_DIR}/${file}" -o "models/${MODEL_DIR}/${file}"
  done < "models/${MODEL_DIR}/manifest.txt"
else
  # Multi-branch flat model: download 3 fixed files
  echo "  Flat model — downloading model files..."
  cd "models/${MODEL_DIR}"
  for file in model.safetensors label_map.json config.json; do
    echo "  Downloading ${file}..."
    curl -sfLO --retry 5 --retry-delay 2 --retry-all-errors --retry-connrefused "${REPO}/${MODEL_DIR}/${file}"
  done
  # Dual-encoder: if config.json declares a co-located value-branch encoder
  # (value_embed_model, e.g. potion-8M for the value-aggregation branch), fetch
  # its files too. Header + semantic/entity/sense classifiers use the shared
  # model2vec; the value branch needs this second encoder, so without it the model
  # fails to load. Single-encoder models have no value_embed_model → skip.
  VEM=$(sed -n 's/.*"value_embed_model"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' config.json 2>/dev/null | head -1)
  if [ -n "${VEM}" ]; then
    echo "  Dual-encoder — downloading value encoder ${VEM}/..."
    mkdir -p "${VEM}"
    for vf in model.safetensors tokenizer.json; do
      echo "  Downloading ${VEM}/${vf}..."
      curl -sfL --retry 5 --retry-delay 2 --retry-all-errors --retry-connrefused \
        "${REPO}/${MODEL_DIR}/${VEM}/${vf}" -o "${VEM}/${vf}"
    done
  fi
  cd ../..
fi

echo "Model files:"
find "models/${MODEL_DIR}" -type f | sort

# ── Sibling-context model (optional) ──────────────────────────────────────
# Download the sibling-context attention model for cross-column enrichment.
# The build gracefully degrades if these are absent.
echo ""
echo "Downloading sibling-context model..."
mkdir -p models/sibling-context
SC_OK=true
for file in model.safetensors config.json; do
  echo "  Downloading sibling-context/${file}..."
  if ! curl -sfL --retry 5 --retry-delay 2 --retry-all-errors --retry-connrefused "${REPO}/sibling-context/${file}" -o "models/sibling-context/${file}"; then
    echo "  WARNING: Failed to download sibling-context/${file} — sibling context will be disabled"
    SC_OK=false
    break
  fi
done

if [ "${SC_OK}" = true ]; then
  echo "Sibling-context files:"
  find models/sibling-context -type f | sort
else
  echo "Sibling-context download failed — continuing without cross-column enrichment"
  rm -rf models/sibling-context
fi

# ── Model2Vec semantic hint classifier (optional) ──────────────────────────
# Download the Model2Vec artifacts for the semantic column name classifier.
# The build gracefully degrades if these are absent (HAS_MODEL2VEC=false).
echo ""
echo "Downloading Model2Vec semantic hint classifier..."
mkdir -p models/model2vec
M2V_OK=true
for file in model.safetensors type_embeddings.safetensors tokenizer.json label_index.json; do
  echo "  Downloading model2vec/${file}..."
  if ! curl -sfL --retry 5 --retry-delay 2 --retry-all-errors --retry-connrefused "${REPO}/model2vec/${file}" -o "models/model2vec/${file}"; then
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
  if ! curl -sfL --retry 5 --retry-delay 2 --retry-all-errors --retry-connrefused "${REPO}/entity-classifier/${file}" -o "models/entity-classifier/${file}"; then
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
