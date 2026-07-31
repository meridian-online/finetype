#!/usr/bin/env bash
# scripts/sharpen_ablation.sh — Measure per-rule Sharpen impact via ablation
#
# Compares raw model output (--raw-model) against Sharpened output for v19-relu-s42.
# For each column where Sharpen changes the prediction, determines if the change
# helped or hurt relative to ground truth. Groups by disambiguation rule name.
#
# Output:
#   diagnostics/sharpen_ablation.tsv    — per-rule impact summary
#   diagnostics/sharpen_per_column.tsv  — per-column raw vs sharpened vs GT
#
# Usage:
#   ./scripts/sharpen_ablation.sh [model_dir]
#
# Spec: 2026-04-27-sharpen-rule-audit (ac-01)
set -eo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_DIR"

MODEL_DIR="${1:-models/sherlock-v19-relu-s42}"
MODEL_NAME="$(basename "$MODEL_DIR")"
MANIFEST="eval/datasets/manifest.csv"
SCHEMA_MAPPING="eval/schema_mapping.csv"
DIAG_DIR="diagnostics"
mkdir -p "$DIAG_DIR"

FINETYPE="${FINETYPE:-$PROJECT_DIR/target/release/finetype}"
if [ ! -x "$FINETYPE" ]; then
    echo "Building release binary..."
    cargo build --release -p finetype-cli 2>&1
fi

echo "================================================================"
echo " Sharpen Ablation Study"
echo " Model: $MODEL_DIR"
echo " Started: $(date)"
echo "================================================================"
echo ""

# ── Step 1: Profile with and without Sharpen ────────────────────────

RAW_DIR="$DIAG_DIR/ablation_raw"
SHARP_DIR="$DIAG_DIR/ablation_sharp"
mkdir -p "$RAW_DIR/csv" "$SHARP_DIR/csv"

echo "── Step 1: Profiling all datasets (raw + sharpened) ──"

seen_files=""
file_count=0

while IFS=, read -r dataset file_path column_name gt_label _rest; do
    [[ "$dataset" == "dataset" ]] && continue

    if echo "$seen_files" | grep -qF "|${file_path}|" 2>/dev/null; then
        continue
    fi
    seen_files="${seen_files}|${file_path}|"

    if [[ ! -f "$file_path" ]]; then
        continue
    fi

    # Raw model (no Sharpen)
    FINETYPE_MODEL="$MODEL_DIR" "$FINETYPE" profile --file "$file_path" --raw-model -o csv \
        > "$RAW_DIR/csv/${dataset}.csv" 2>/dev/null

    # Sharpened (normal pipeline)
    FINETYPE_MODEL="$MODEL_DIR" "$FINETYPE" profile --file "$file_path" -o csv \
        > "$SHARP_DIR/csv/${dataset}.csv" 2>/dev/null

    file_count=$((file_count + 1))
done < "$MANIFEST"

echo "  Profiled $file_count datasets"
echo ""

# ── Step 2: Combine and score ───────────────────────────────────────

echo "── Step 2: Combining results and scoring ──"

# Combine raw CSVs
duckdb -csv -noheader -c "
    COPY (
        SELECT
            regexp_extract(filename, '.*/([^/]+)\\.csv\$', 1) AS dataset,
            \"column\" AS column_name,
            type AS predicted_type,
            confidence,
            disambiguation
        FROM read_csv('$RAW_DIR/csv/*.csv', auto_detect=true, filename=true)
    ) TO '$RAW_DIR/combined.csv' (HEADER, DELIMITER ',');
" 2>/dev/null

# Combine sharpened CSVs
duckdb -csv -noheader -c "
    COPY (
        SELECT
            regexp_extract(filename, '.*/([^/]+)\\.csv\$', 1) AS dataset,
            \"column\" AS column_name,
            type AS predicted_type,
            confidence,
            disambiguation
        FROM read_csv('$SHARP_DIR/csv/*.csv', auto_detect=true, filename=true)
    ) TO '$SHARP_DIR/combined.csv' (HEADER, DELIMITER ',');
" 2>/dev/null

# ── Step 3: Per-column comparison ───────────────────────────────────

echo "── Step 3: Per-column raw vs sharpened vs GT ──"

PER_COL_FILE="$DIAG_DIR/sharpen_per_column.tsv"

duckdb -c "
    CREATE TABLE gt AS
    SELECT dataset, column_name, gt_label
    FROM read_csv('$MANIFEST',
        columns={'dataset': 'VARCHAR', 'file_path': 'VARCHAR',
                 'column_name': 'VARCHAR', 'gt_label': 'VARCHAR',
                 'source_url': 'VARCHAR', 'licence': 'VARCHAR',
                 'fetched_date': 'VARCHAR'},
        header=true);

    CREATE TABLE mapping AS
    SELECT gt_label, finetype_label, finetype_domain
    FROM read_csv('$SCHEMA_MAPPING', auto_detect=true);

    CREATE TABLE raw AS
    SELECT dataset, column_name, predicted_type AS raw_pred, confidence AS raw_conf,
           disambiguation AS raw_rule
    FROM read_csv('$RAW_DIR/combined.csv', auto_detect=true);

    CREATE TABLE sharp AS
    SELECT dataset, column_name, predicted_type AS sharp_pred, confidence AS sharp_conf,
           disambiguation AS sharp_rule
    FROM read_csv('$SHARP_DIR/combined.csv', auto_detect=true);

    -- Build valid label sets per (dataset, column_name): a prediction matches
    -- GT if it equals ANY finetype_label for that gt_label (many-to-many mapping).
    -- Also match if both are in the boolean.% family (interchangeable).
    CREATE TABLE valid_labels AS
    SELECT gt.dataset, gt.column_name, gt.gt_label,
           list(DISTINCT m.finetype_label) AS valid_labels,
           first(m.finetype_domain) AS finetype_domain,
           first(m.finetype_label) AS finetype_label  -- representative for display
    FROM gt
    JOIN mapping m ON gt.gt_label = m.gt_label
    GROUP BY gt.dataset, gt.column_name, gt.gt_label;

    CREATE TABLE joined AS
    SELECT
        g.dataset, g.column_name, g.gt_label, g.finetype_label,
        r.raw_pred, r.raw_conf, r.raw_rule,
        s.sharp_pred, s.sharp_conf, s.sharp_rule,
        -- Did raw match GT? (any valid label or boolean family)
        CASE WHEN list_contains(g.valid_labels, r.raw_pred) THEN 1
             WHEN len(list_filter(g.valid_labels, x -> x LIKE 'representation.boolean.%')) > 0
                  AND r.raw_pred LIKE 'representation.boolean.%' THEN 1
             ELSE 0
        END AS raw_match,
        -- Did sharpened match GT?
        CASE WHEN list_contains(g.valid_labels, s.sharp_pred) THEN 1
             WHEN len(list_filter(g.valid_labels, x -> x LIKE 'representation.boolean.%')) > 0
                  AND s.sharp_pred LIKE 'representation.boolean.%' THEN 1
             ELSE 0
        END AS sharp_match,
        -- Did Sharpen change the prediction?
        CASE WHEN r.raw_pred != s.sharp_pred THEN 1 ELSE 0 END AS sharpen_changed,
        -- What was the effect?
        CASE
            WHEN r.raw_pred = s.sharp_pred THEN 'UNCHANGED'
            WHEN list_contains(g.valid_labels, r.raw_pred)
                 AND NOT list_contains(g.valid_labels, s.sharp_pred) THEN 'HURT'
            WHEN NOT list_contains(g.valid_labels, r.raw_pred)
                 AND list_contains(g.valid_labels, s.sharp_pred) THEN 'HELPED'
            WHEN NOT list_contains(g.valid_labels, r.raw_pred)
                 AND NOT list_contains(g.valid_labels, s.sharp_pred) THEN 'CHANGED_BOTH_WRONG'
            -- Handle boolean interchangeability
            WHEN len(list_filter(g.valid_labels, x -> x LIKE 'representation.boolean.%')) > 0
                 AND r.raw_pred LIKE 'representation.boolean.%'
                 AND NOT (s.sharp_pred LIKE 'representation.boolean.%') THEN 'HURT'
            WHEN len(list_filter(g.valid_labels, x -> x LIKE 'representation.boolean.%')) > 0
                 AND NOT (r.raw_pred LIKE 'representation.boolean.%')
                 AND s.sharp_pred LIKE 'representation.boolean.%' THEN 'HELPED'
            ELSE 'CHANGED_BOTH_WRONG'
        END AS sharpen_effect
    FROM valid_labels g
    JOIN raw r USING (dataset, column_name)
    JOIN sharp s USING (dataset, column_name);

    COPY (
        SELECT * FROM joined ORDER BY sharpen_effect DESC, dataset, column_name
    ) TO '$PER_COL_FILE' (DELIMITER '\t', HEADER);
" 2>/dev/null

total_cols=$(duckdb -noheader -list -c "SELECT count(*) FROM read_csv('$PER_COL_FILE', sep='\t', auto_detect=true)" 2>/dev/null | tr -d '[:space:]')
changed_cols=$(duckdb -noheader -list -c "SELECT count(*) FROM read_csv('$PER_COL_FILE', sep='\t', auto_detect=true) WHERE sharpen_changed = 1" 2>/dev/null | tr -d '[:space:]')
echo "  Total columns: $total_cols"
echo "  Sharpen changed prediction: $changed_cols"
echo ""

# ── Step 4: Per-rule ablation summary ───────────────────────────────

echo "── Step 4: Per-rule impact summary ──"

ABLATION_FILE="$DIAG_DIR/sharpen_ablation.tsv"

duckdb -c "
    CREATE TABLE per_col AS
    SELECT * FROM read_csv('$PER_COL_FILE', sep='\t', auto_detect=true);

    -- Group by the sharpened rule name for columns where Sharpen changed the prediction
    COPY (
        SELECT
            sharp_rule AS rule_id,
            count(*) FILTER (WHERE sharpen_effect = 'HELPED') AS fixes,
            count(*) FILTER (WHERE sharpen_effect = 'HURT') AS regressions,
            count(*) FILTER (WHERE sharpen_effect = 'CHANGED_BOTH_WRONG') AS changed_both_wrong,
            count(*) FILTER (WHERE sharpen_effect = 'HELPED')
                - count(*) FILTER (WHERE sharpen_effect = 'HURT') AS net_delta,
            count(*) AS total_fires,
            string_agg(
                CASE WHEN sharpen_effect != 'UNCHANGED' THEN column_name ELSE NULL END,
                ', '
            ) AS affected_columns
        FROM per_col
        WHERE sharpen_changed = 1
        GROUP BY sharp_rule
        ORDER BY net_delta ASC, regressions DESC
    ) TO '$ABLATION_FILE' (DELIMITER '\t', HEADER);
" 2>/dev/null

echo ""
echo "── Results ──"
echo ""

# Print summary
duckdb -c "SELECT * FROM read_csv('$ABLATION_FILE', sep='\t', auto_detect=true) ORDER BY net_delta ASC, regressions DESC;" 2>/dev/null

echo ""

# Summary stats
raw_score=$(duckdb -noheader -list -c "SELECT sum(raw_match) FROM read_csv('$PER_COL_FILE', sep='\t', auto_detect=true)" 2>/dev/null | tr -d '[:space:]')
sharp_score=$(duckdb -noheader -list -c "SELECT sum(sharp_match) FROM read_csv('$PER_COL_FILE', sep='\t', auto_detect=true)" 2>/dev/null | tr -d '[:space:]')
echo "Raw model score:      $raw_score / $total_cols"
echo "Sharpened score:      $sharp_score / $total_cols"
echo "Sharpen net effect:   $(( sharp_score - raw_score ))"
echo ""
echo "Ablation summary:     $ABLATION_FILE"
echo "Per-column detail:    $PER_COL_FILE"
echo ""
echo "================================================================"
