#!/usr/bin/env bash
# LLM distillation: label real-world CSV columns using Ollama
# Usage: ./scripts/llm_label.sh <csv_dir> <output_csv> [options]
#
# Options:
#   --model <name>       Ollama model (default: qwen3:32b)
#   --max-columns <n>    Stop after N columns (default: unlimited)
#   --max-values <n>     Sample N values per column (default: 10)
#   --skip-finetype      Skip FineType comparison
#
# Requires: ollama, python3, jq

set -euo pipefail

# --- Defaults ---
MODEL="qwen3:32b"
MAX_COLUMNS=0  # 0 = unlimited
MAX_VALUES=10
SKIP_FINETYPE=false

# --- Parse args ---
CSV_DIR="${1:-}"
OUTPUT_CSV="${2:-}"
shift 2 2>/dev/null || true

while [[ $# -gt 0 ]]; do
    case "$1" in
        --model) MODEL="$2"; shift 2 ;;
        --max-columns) MAX_COLUMNS="$2"; shift 2 ;;
        --max-values) MAX_VALUES="$2"; shift 2 ;;
        --skip-finetype) SKIP_FINETYPE=true; shift ;;
        *) echo "Unknown option: $1"; exit 1 ;;
    esac
done

if [[ -z "$CSV_DIR" || -z "$OUTPUT_CSV" ]]; then
    echo "Usage: $0 <csv_dir> <output_csv> [--model name] [--max-columns N] [--max-values N] [--skip-finetype]"
    exit 1
fi

# --- Locate taxonomy ---
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
TYPES_FILE="$(mktemp /tmp/finetype_types.XXXXXX)"

# Generate type list from taxonomy
if command -v finetype &>/dev/null; then
    finetype taxonomy --output json 2>/dev/null | python3 -c "
import json, sys
data = json.load(sys.stdin)
for d in data:
    print(d['key'])
" > "$TYPES_FILE"
elif [[ -f "$REPO_ROOT/Cargo.toml" ]]; then
    (cd "$REPO_ROOT" && cargo run --quiet -- taxonomy --output json 2>/dev/null) | python3 -c "
import json, sys
data = json.load(sys.stdin)
for d in data:
    print(d['key'])
" > "$TYPES_FILE"
else
    echo "ERROR: Cannot find finetype binary or Cargo.toml. Run from repo root or install finetype."
    exit 1
fi

TYPE_COUNT=$(wc -l < "$TYPES_FILE" | tr -d ' ')
echo "Loaded $TYPE_COUNT type labels from taxonomy"

# Build type list string for prompt (newline-separated)
TYPE_LIST=$(cat "$TYPES_FILE")

# --- Check Ollama ---
if ! command -v ollama &>/dev/null; then
    echo "ERROR: ollama not found. Install with: brew install ollama"
    exit 1
fi

if ! ollama list 2>/dev/null | grep -q "$MODEL"; then
    echo "Model $MODEL not found. Pull it with: ollama pull $MODEL"
    exit 1
fi

# --- Check FineType ---
HAS_FINETYPE=false
if [[ "$SKIP_FINETYPE" == "false" ]]; then
    if command -v finetype &>/dev/null; then
        HAS_FINETYPE=true
    elif [[ -f "$REPO_ROOT/target/release/finetype" ]]; then
        export PATH="$REPO_ROOT/target/release:$PATH"
        HAS_FINETYPE=true
    fi
fi

# --- Setup output ---
mkdir -p "$(dirname "$OUTPUT_CSV")"
if [[ ! -f "$OUTPUT_CSV" ]]; then
    echo "source_file,column_name,sample_values,llm_label,llm_valid,finetype_label,agreement" > "$OUTPUT_CSV"
fi

# --- Stats ---
TOTAL=0
VALID=0
AGREE=0
INVALID=0

# --- Process CSVs ---
echo "Scanning $CSV_DIR for CSV files..."
CSV_FILES=$(find "$CSV_DIR" -name "*.csv" -type f | sort)
FILE_COUNT=$(echo "$CSV_FILES" | wc -l | tr -d ' ')
echo "Found $FILE_COUNT CSV files"
echo ""

for csv_file in $CSV_FILES; do
    # Extract columns using Python
    python3 -c "
import csv, json, sys, os

filepath = '$csv_file'
max_values = $MAX_VALUES

try:
    with open(filepath, 'r', encoding='utf-8', errors='replace') as f:
        reader = csv.DictReader(f)
        if not reader.fieldnames:
            sys.exit(0)

        # Collect sample values per column
        columns = {h: [] for h in reader.fieldnames}
        row_count = 0
        for row in reader:
            row_count += 1
            if row_count > 200:  # Read at most 200 rows
                break
            for h in reader.fieldnames:
                val = row.get(h, '').strip()
                if val and len(columns[h]) < max_values:
                    columns[h].append(val)

        # Output as JSON lines: {header, values}
        for h in reader.fieldnames:
            if len(columns[h]) >= 3:  # Need at least 3 non-empty values
                print(json.dumps({'header': h, 'values': columns[h][:max_values]}))
except Exception as e:
    print(json.dumps({'error': str(e)}), file=sys.stderr)
" | while IFS= read -r col_json; do
        # Check column limit
        if [[ "$MAX_COLUMNS" -gt 0 && "$TOTAL" -ge "$MAX_COLUMNS" ]]; then
            break 2  # Break out of both loops
        fi

        HEADER=$(echo "$col_json" | python3 -c "import json,sys; print(json.load(sys.stdin)['header'])")
        VALUES=$(echo "$col_json" | python3 -c "import json,sys; print(json.dumps(json.load(sys.stdin)['values']))")

        # Build prompt
        PROMPT="You are a data type classifier. Given a column header and sample values, classify the column into exactly one type from the list below.

RULES:
- Reply with ONLY the type label (e.g., 'identity.person.email')
- Do NOT explain your reasoning
- The label MUST be from the list below — no other labels are valid
- If uncertain, pick the closest match

COLUMN:
Header: $HEADER
Values: $VALUES

VALID TYPE LABELS:
$TYPE_LIST

TYPE LABEL:"

        # Query Ollama
        LLM_RESPONSE=$(ollama run "$MODEL" "$PROMPT" --nowordwrap 2>/dev/null | head -1 | tr -d '[:space:]' | sed 's/^[`"'"'"']//;s/[`"'"'"']$//')

        # Validate response
        LLM_VALID="no"
        if grep -qx "$LLM_RESPONSE" "$TYPES_FILE" 2>/dev/null; then
            LLM_VALID="yes"
            VALID=$((VALID + 1))
        else
            INVALID=$((INVALID + 1))
        fi

        # Get FineType prediction if available
        FT_LABEL=""
        AGREEMENT=""
        if [[ "$HAS_FINETYPE" == "true" && "$LLM_VALID" == "yes" ]]; then
            FT_LABEL=$(echo "$VALUES" | python3 -c "
import json, sys
vals = json.load(sys.stdin)
for v in vals:
    print(v)
" | finetype infer --column --header "$HEADER" 2>/dev/null | head -1 || echo "")

            if [[ -n "$FT_LABEL" ]]; then
                if [[ "$LLM_RESPONSE" == "$FT_LABEL" ]]; then
                    AGREEMENT="yes"
                    AGREE=$((AGREE + 1))
                else
                    AGREEMENT="no"
                fi
            fi
        fi

        TOTAL=$((TOTAL + 1))

        # Escape values for CSV
        ESCAPED_VALUES=$(echo "$VALUES" | sed 's/"/""/g')

        # Append to output
        echo "\"$(basename "$csv_file")\",\"$HEADER\",\"$ESCAPED_VALUES\",\"$LLM_RESPONSE\",\"$LLM_VALID\",\"$FT_LABEL\",\"$AGREEMENT\"" >> "$OUTPUT_CSV"

        # Progress
        VALID_PCT=0
        AGREE_PCT=0
        [[ "$TOTAL" -gt 0 ]] && VALID_PCT=$((VALID * 100 / TOTAL))
        [[ "$VALID" -gt 0 && "$HAS_FINETYPE" == "true" ]] && AGREE_PCT=$((AGREE * 100 / VALID))

        if [[ "$AGREEMENT" == "yes" ]]; then
            echo "[$TOTAL] $(basename "$csv_file"):$HEADER → $LLM_RESPONSE (✓ agree)"
        elif [[ "$AGREEMENT" == "no" ]]; then
            echo "[$TOTAL] $(basename "$csv_file"):$HEADER → $LLM_RESPONSE (✗ FT=$FT_LABEL)"
        elif [[ "$LLM_VALID" == "yes" ]]; then
            echo "[$TOTAL] $(basename "$csv_file"):$HEADER → $LLM_RESPONSE"
        else
            echo "[$TOTAL] $(basename "$csv_file"):$HEADER → INVALID: $LLM_RESPONSE"
        fi

        # Summary every 50 columns
        if [[ $((TOTAL % 50)) -eq 0 ]]; then
            echo "  --- Valid: $VALID/$TOTAL ($VALID_PCT%) | Agreement: $AGREE/$VALID ($AGREE_PCT%) | Invalid: $INVALID ---"
        fi
    done
done

# --- Final summary ---
echo ""
echo "========================================="
echo "  LLM Labelling Complete"
echo "========================================="
echo "  Model:      $MODEL"
echo "  Total:      $TOTAL columns"
echo "  Valid:      $VALID ($((VALID * 100 / (TOTAL > 0 ? TOTAL : 1)))%)"
echo "  Invalid:    $INVALID ($((INVALID * 100 / (TOTAL > 0 ? TOTAL : 1)))%)"
if [[ "$HAS_FINETYPE" == "true" ]]; then
echo "  Agreement:  $AGREE/$VALID ($((AGREE * 100 / (VALID > 0 ? VALID : 1)))%)"
fi
echo "  Output:     $OUTPUT_CSV"
echo "========================================="

# Cleanup
rm -f "$TYPES_FILE"
