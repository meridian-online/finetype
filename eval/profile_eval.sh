#!/usr/bin/env bash
# ═══════════════════════════════════════════════════════════════════════════════
# FineType Profile Evaluation Pipeline
# ═══════════════════════════════════════════════════════════════════════════════
#
# Profiles annotated CSV files with FineType and evaluates predictions against
# ground truth annotations using the schema mapping (NNFT-079).
#
# Usage:
#   ./eval/profile_eval.sh <manifest.csv>
#   ./eval/profile_eval.sh eval/datasets/manifest.csv
#   make eval-profile MANIFEST=eval/datasets/manifest.csv
#
# Manifest format (CSV):
#   dataset,file_path,column_name,gt_label
#   titanic,/path/to/titanic.csv,Survived,boolean
#   titanic,/path/to/titanic.csv,Age,age
#   titanic,/path/to/titanic.csv,Embarked,category
#
# The manifest lists ground truth annotations per column. Each unique
# (dataset, file_path) pair is profiled once; columns without GT annotations
# are still profiled but excluded from accuracy scoring.
#
# Output:
#   eval/eval_output/profile_results.csv  — profile predictions per column
#   eval/eval_output/ground_truth.csv     — GT annotations from manifest
#   Console: full evaluation report from eval/eval_profile.sql

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# ── Arguments ────────────────────────────────────────────────────────────────

MANIFEST="${1:-}"
if [ -z "$MANIFEST" ]; then
    echo "Usage: $0 <manifest.csv>"
    echo ""
    echo "Manifest format (CSV with header):"
    echo "  dataset,file_path,column_name,gt_label"
    echo "  titanic,/path/to/titanic.csv,Survived,boolean"
    echo "  titanic,/path/to/titanic.csv,Age,age"
    exit 1
fi

if [ ! -f "$MANIFEST" ]; then
    echo "ERROR: Manifest not found: $MANIFEST"
    exit 1
fi

# Allow override via env var
FINETYPE="${FINETYPE:-$REPO_ROOT/target/release/finetype}"
if [ ! -x "$FINETYPE" ]; then
    echo "Building release binary..."
    (cd "$REPO_ROOT" && cargo build --release -p finetype-cli 2>&1)
fi

OUTPUT_DIR="$REPO_ROOT/eval/eval_output"
mkdir -p "$OUTPUT_DIR"

# ── Phase 1: Profile all unique datasets ────────────────────────────────────

printf "\n\033[1m═══ Phase 1: Profiling datasets ═══\033[0m\n\n"

# Extract unique (dataset, file_path) pairs from manifest
# Skip header line, deduplicate by dataset+file_path
PROFILE_RESULTS="$OUTPUT_DIR/profile_results.csv"
echo "dataset,column_name,predicted_type,confidence" > "$PROFILE_RESULTS"

PROFILED=0
ERRORS=0

# Track which files we've already profiled (dedup)
# Using newline-separated string for bash 3.2 compatibility (macOS)
SEEN_FILES=""

while IFS=, read -r dataset file_path column_name gt_label; do
    # Skip header
    if [ "$dataset" = "dataset" ]; then continue; fi

    # Skip if already profiled this file
    key="${dataset}:${file_path}"
    if echo "$SEEN_FILES" | grep -q "^${key}$"; then continue; fi
    SEEN_FILES="${SEEN_FILES}${key}"$'\n'

    # Resolve relative paths from repo root
    if [[ ! "$file_path" = /* ]]; then
        file_path="$REPO_ROOT/$file_path"
    fi

    if [ ! -f "$file_path" ]; then
        printf "  \033[33m○\033[0m %s — file not found: %s\n" "$dataset" "$file_path"
        ERRORS=$((ERRORS + 1))
        continue
    fi

    printf "  \033[34m→\033[0m Profiling %s (%s)..." "$dataset" "$(basename "$file_path")"

    # Run finetype profile and parse JSON output
    PROFILE_JSON=$("$FINETYPE" profile -f "$file_path" -o json 2>/dev/null) || {
        printf " \033[31mFAILED\033[0m\n"
        ERRORS=$((ERRORS + 1))
        continue
    }

    # Parse JSON: extract column, type, confidence for each column
    echo "$PROFILE_JSON" | jq -r --arg d "$dataset" \
        '.columns[] | [$d, .column, .type, (.confidence // 0 | tostring)] | join(",")' \
        >> "$PROFILE_RESULTS"

    COL_COUNT=$(echo "$PROFILE_JSON" | jq '.columns | length')
    printf " \033[32m%d columns\033[0m\n" "$COL_COUNT"
    PROFILED=$((PROFILED + 1))

done < "$MANIFEST"

printf "\n  Profiled %d datasets (%d errors)\n" "$PROFILED" "$ERRORS"

# ── Phase 2: Extract ground truth from manifest ─────────────────────────────

printf "\n\033[1m═══ Phase 2: Extracting ground truth ═══\033[0m\n\n"

GT_FILE="$OUTPUT_DIR/ground_truth.csv"
echo "dataset,column_name,gt_label" > "$GT_FILE"

GT_COUNT=0
while IFS=, read -r dataset file_path column_name gt_label; do
    # Skip header
    if [ "$dataset" = "dataset" ]; then continue; fi

    # Write GT annotation
    echo "${dataset},${column_name},${gt_label}" >> "$GT_FILE"
    GT_COUNT=$((GT_COUNT + 1))
done < "$MANIFEST"

printf "  %d ground truth annotations extracted\n" "$GT_COUNT"

# ── Phase 3: Run DuckDB evaluation ──────────────────────────────────────────

printf "\n\033[1m═══ Phase 3: Running evaluation ═══\033[0m\n"

cd "$REPO_ROOT"
duckdb < eval/eval_profile.sql

printf "\n\033[32m═══ Profile evaluation complete ═══\033[0m\n"
printf "  Results: %s\n" "$PROFILE_RESULTS"
printf "  Ground truth: %s\n" "$GT_FILE"
