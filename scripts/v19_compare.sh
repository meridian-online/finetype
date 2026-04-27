#!/usr/bin/env bash
# scripts/v19_compare.sh — v19 three-way comparison + MADR 0066 gate evaluation
#
# Evaluates all v19 model candidates against the 448-row eval manifest,
# selects the best seed per architecture, and applies the MADR 0066 hard gate.
#
# Three-way diff: v16 baseline (297/352) vs best-ReLU-v19 vs best-GELU-v19
#
# Usage:
#   ./scripts/v19_compare.sh                   # Full evaluation
#   ./scripts/v19_compare.sh --baseline-only    # Score v16 baseline only
#
# Prerequisites:
#   - Trained models at models/sherlock-v19-{relu,gelu}-s{42,43,44}/
#   - Release binary built (cargo build --release)
#   - eval/datasets/manifest.csv (448 rows)
#   - eval/schema_mapping.csv
#
# Output:
#   diagnostics/v19_gate_results.tsv       — Gate verdicts per architecture
#   diagnostics/v19_per_seed_results.tsv   — Val_acc + profile eval per seed
#   diagnostics/v19_per_column_diff.tsv    — Per-column three-way diff
#   diagnostics/v19_per_domain_delta.tsv   — Per-domain delta table
#
# Spec: orbit/specs/2026-04-25-v19-paired-retrain/spec.yaml
# Decision: 0066 (hard gate), 0068 (GELU+LN revisit)
set -eo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_DIR"

MANIFEST="eval/datasets/manifest.csv"
SCHEMA_MAPPING="eval/schema_mapping.csv"
DIAG_DIR="diagnostics"
mkdir -p "$DIAG_DIR"

BASELINE_MODEL="models/sherlock-v16"
BASELINE_SCORE=297
BASELINE_TOTAL=352

BASELINE_ONLY=false
[[ "${1:-}" == "--baseline-only" ]] && BASELINE_ONLY=true

# ── Build release binary ─────────────────────────────────────────────

FINETYPE="${FINETYPE:-$PROJECT_DIR/target/release/finetype}"
if [ ! -x "$FINETYPE" ]; then
    echo "Building release binary..."
    cargo build --release -p finetype-cli 2>&1
fi

# ── Helper: run profile eval and score against manifest ──────────────

score_model() {
    local model_dir="$1"
    local model_name="$2"
    local output_dir="$DIAG_DIR/v19_eval_${model_name}"
    mkdir -p "$output_dir"

    if [[ ! -d "$model_dir" ]] || [[ ! -f "$model_dir/model.safetensors" ]]; then
        echo "SKIP"
        return
    fi

    # Profile each unique file in the manifest, save raw CSV per dataset
    local raw_dir="$output_dir/raw"
    mkdir -p "$raw_dir"
    local profile_csv="$output_dir/profile_results.csv"

    local seen_files=""
    local errors=0
    local file_count=0

    while IFS=, read -r dataset file_path column_name gt_label _rest; do
        [[ "$dataset" == "dataset" ]] && continue  # skip header

        # Dedup by file_path
        if echo "$seen_files" | grep -qF "|${file_path}|" 2>/dev/null; then
            continue
        fi
        seen_files="${seen_files}|${file_path}|"

        if [[ ! -f "$file_path" ]]; then
            errors=$((errors + 1))
            continue
        fi

        # Profile with the candidate model via FINETYPE_MODEL env var
        # Save raw CSV — DuckDB will parse it properly (handles quoting)
        FINETYPE_MODEL="$model_dir" "$FINETYPE" profile --file "$file_path" -o csv \
            > "$raw_dir/${dataset}.csv" 2>/dev/null
        file_count=$((file_count + 1))

    done < "$MANIFEST"

    if [[ $file_count -eq 0 ]]; then
        echo "SKIP"
        return
    fi

    # Combine raw CSVs into a single profile_results.csv using DuckDB
    # This handles quoted fields correctly; extract dataset from filename
    duckdb -csv -noheader -c "
        COPY (
            SELECT
                regexp_extract(filename, '.*/([^/]+)\\.csv$', 1) AS dataset,
                \"column\" AS column_name,
                type AS predicted_type,
                confidence
            FROM read_csv('$raw_dir/*.csv', auto_detect=true, filename=true)
        ) TO '$profile_csv' (HEADER, DELIMITER ',');
    " 2>/dev/null

    # Score using DuckDB
    if ! command -v duckdb >/dev/null 2>&1; then
        echo "ERROR: duckdb not found"
        return
    fi

    local result
    result=$(duckdb -csv -noheader -c "
        CREATE TABLE gt AS
        SELECT dataset, column_name, gt_label
        FROM read_csv('$MANIFEST',
            columns={'dataset': 'VARCHAR', 'file_path': 'VARCHAR',
                     'column_name': 'VARCHAR', 'gt_label': 'VARCHAR',
                     'source_url': 'VARCHAR', 'licence': 'VARCHAR',
                     'fetched_date': 'VARCHAR'},
            header=true);

        CREATE TABLE preds AS
        SELECT dataset, column_name, predicted_type, confidence
        FROM read_csv('$profile_csv', auto_detect=true);

        CREATE TABLE mapping AS
        SELECT gt_label, finetype_label, finetype_domain
        FROM read_csv('$SCHEMA_MAPPING', auto_detect=true);

        -- Map GT short labels → FineType labels via schema_mapping,
        -- then compare predictions against mapped labels.
        -- A GT label may map to multiple finetype_labels (e.g. boolean → binary/terms).
        -- We keep the best match per (dataset, column_name).
        WITH candidates AS (
            SELECT
                gt.dataset,
                gt.column_name,
                gt.gt_label,
                p.predicted_type,
                p.confidence,
                m.finetype_label,
                m.finetype_domain,
                CASE
                    WHEN p.predicted_type = m.finetype_label THEN 1
                    WHEN m.finetype_label LIKE 'representation.boolean.%'
                         AND p.predicted_type LIKE 'representation.boolean.%' THEN 1
                    ELSE 0
                END AS label_match,
                CASE
                    WHEN split_part(p.predicted_type, '.', 1) = m.finetype_domain THEN 1
                    ELSE 0
                END AS domain_match
            FROM gt
            JOIN preds p USING (dataset, column_name)
            JOIN mapping m ON gt.gt_label = m.gt_label
        ),
        best AS (
            SELECT DISTINCT ON (dataset, column_name)
                dataset, column_name, gt_label, predicted_type, confidence,
                finetype_label, finetype_domain, label_match, domain_match
            FROM candidates
            ORDER BY dataset, column_name, label_match DESC, domain_match DESC
        )
        SELECT
            count(*) AS total,
            sum(label_match) AS label_correct,
            sum(domain_match) AS domain_correct
        FROM best;
    " 2>/dev/null)

    echo "$result"
}

# Score with per-column detail for diff
score_model_detail() {
    local model_dir="$1"
    local model_name="$2"
    local output_dir="$DIAG_DIR/v19_eval_${model_name}"

    if [[ ! -d "$output_dir" ]] || [[ ! -f "$output_dir/profile_results.csv" ]]; then
        return
    fi

    duckdb -csv -c "
        CREATE TABLE gt AS
        SELECT dataset, column_name, gt_label
        FROM read_csv('$MANIFEST',
            columns={'dataset': 'VARCHAR', 'file_path': 'VARCHAR',
                     'column_name': 'VARCHAR', 'gt_label': 'VARCHAR',
                     'source_url': 'VARCHAR', 'licence': 'VARCHAR',
                     'fetched_date': 'VARCHAR'},
            header=true);

        CREATE TABLE preds AS
        SELECT dataset, column_name, predicted_type, confidence
        FROM read_csv('$output_dir/profile_results.csv', auto_detect=true);

        CREATE TABLE mapping AS
        SELECT gt_label, finetype_label, finetype_domain
        FROM read_csv('$SCHEMA_MAPPING', auto_detect=true);

        -- Map GT short labels → FineType labels, keep best match per column
        WITH candidates AS (
            SELECT
                gt.dataset,
                gt.column_name,
                gt.gt_label,
                m.finetype_label,
                m.finetype_domain,
                p.predicted_type AS prediction,
                ROUND(p.confidence, 3) AS confidence,
                CASE
                    WHEN p.predicted_type = m.finetype_label THEN 'MATCH'
                    WHEN m.finetype_label LIKE 'representation.boolean.%'
                         AND p.predicted_type LIKE 'representation.boolean.%' THEN 'MATCH'
                    ELSE 'MISS'
                END AS result,
                CASE
                    WHEN p.predicted_type = m.finetype_label THEN 1
                    WHEN m.finetype_label LIKE 'representation.boolean.%'
                         AND p.predicted_type LIKE 'representation.boolean.%' THEN 1
                    ELSE 0
                END AS label_rank
            FROM gt
            JOIN preds p USING (dataset, column_name)
            JOIN mapping m ON gt.gt_label = m.gt_label
        ),
        best AS (
            SELECT DISTINCT ON (dataset, column_name) *
            FROM candidates
            ORDER BY dataset, column_name, label_rank DESC
        )
        SELECT
            dataset, column_name, gt_label, finetype_label,
            prediction, confidence, result, finetype_domain AS domain
        FROM best
        ORDER BY domain, dataset, column_name;
    " 2>/dev/null
}

# ── Step 1: Gather val_accuracy from results.json ────────────────────

echo "================================================================"
echo " v19 Three-Way Comparison + MADR 0066 Gate"
echo " Started: $(date)"
echo "================================================================"
echo ""

SEEDS=(42 43 44)
ARCHS=("relu" "gelu")

echo "── Val accuracy from training ──"
echo ""
printf "%-30s %-12s %-12s\n" "Model" "Best Val Acc" "Best Epoch"
printf "%-30s %-12s %-12s\n" "-----" "-----------" "----------"

PER_SEED_FILE="$DIAG_DIR/v19_per_seed_results.tsv"
echo -e "model\tarch\tseed\tval_acc\tbest_epoch\tprofile_label\tprofile_domain\tprofile_total" > "$PER_SEED_FILE"

for arch in "${ARCHS[@]}"; do
    for seed in "${SEEDS[@]}"; do
        name="sherlock-v19-${arch}-s${seed}"
        model_dir="models/$name"
        results_json="$model_dir/results.json"

        if [[ -f "$results_json" ]]; then
            best_line=$(duckdb -csv -noheader -c "
                SELECT ROUND(val_accuracy, 4), epoch + 1
                FROM read_json('$results_json', format='array',
                    columns={epoch: 'INTEGER', val_accuracy: 'DOUBLE'})
                ORDER BY val_accuracy DESC
                LIMIT 1;
            " 2>/dev/null || echo ",")
            val_acc=$(echo "$best_line" | cut -d, -f1)
            best_epoch=$(echo "$best_line" | cut -d, -f2)
            printf "%-30s %-12s %-12s\n" "$name" "$val_acc" "$best_epoch"
        else
            val_acc=""
            best_epoch=""
            printf "%-30s %-12s %-12s\n" "$name" "MISSING" "-"
        fi

        # Placeholder for profile eval — filled in next step
        echo -e "${name}\t${arch}\t${seed}\t${val_acc}\t${best_epoch}\t\t\t" >> "$PER_SEED_FILE"
    done
done
echo ""

# ── Step 2: Profile eval each candidate ──────────────────────────────

echo "── Profile evaluation ──"
echo ""

if [[ "$BASELINE_ONLY" == "true" ]]; then
    echo "Baseline-only mode: scoring v16 only"
    CANDIDATES=("v16|$BASELINE_MODEL")
else
    CANDIDATES=()
    for arch in "${ARCHS[@]}"; do
        for seed in "${SEEDS[@]}"; do
            name="sherlock-v19-${arch}-s${seed}"
            CANDIDATES+=("$name|models/$name")
        done
    done
    # Also score baseline for fresh comparison
    CANDIDATES+=("v16|$BASELINE_MODEL")
fi

# Store profile scores in a TSV file (bash 3.2 compatible — no associative arrays)
SCORES_FILE="$DIAG_DIR/v19_profile_scores.tsv"
echo -e "name\tlabel\tdomain\ttotal" > "$SCORES_FILE"

for candidate in "${CANDIDATES[@]}"; do
    IFS='|' read -r cand_name cand_dir <<< "$candidate"
    echo "Scoring $cand_name..."

    result=$(score_model "$cand_dir" "$cand_name")

    if [[ "$result" == "SKIP" ]]; then
        echo "  → SKIPPED (model not found)"
        echo -e "${cand_name}\t\t\t" >> "$SCORES_FILE"
        continue
    elif [[ "$result" == ERROR* ]]; then
        echo "  → $result"
        echo -e "${cand_name}\t\t\t" >> "$SCORES_FILE"
        continue
    fi

    total=$(echo "$result" | cut -d, -f1)
    label=$(echo "$result" | cut -d, -f2)
    domain=$(echo "$result" | cut -d, -f3)

    echo -e "${cand_name}\t${label}\t${domain}\t${total}" >> "$SCORES_FILE"

    label_pct=$(echo "scale=1; $label * 100 / $total" | bc 2>/dev/null || echo "?")
    domain_pct=$(echo "scale=1; $domain * 100 / $total" | bc 2>/dev/null || echo "?")
    echo "  → ${label}/${total} (${label_pct}% label, ${domain_pct}% domain)"
done
echo ""

# Helper: look up a field from the scores TSV
_score_field() {
    # Usage: _score_field <name> <field: label|domain|total>
    local _name="$1" _field="$2" _col
    case "$_field" in
        label) _col=2 ;; domain) _col=3 ;; total) _col=4 ;; *) _col=2 ;;
    esac
    awk -F'\t' -v n="$_name" -v c="$_col" '$1==n {print $c}' "$SCORES_FILE"
}

# Update per-seed file with profile scores
TEMP_SEED="$DIAG_DIR/v19_per_seed_results_tmp.tsv"
head -1 "$PER_SEED_FILE" > "$TEMP_SEED"
while IFS=$'\t' read -r model arch seed val_acc best_epoch _ _ _; do
    [[ "$model" == "model" ]] && continue
    label=$(_score_field "$model" label)
    domain=$(_score_field "$model" domain)
    total=$(_score_field "$model" total)
    echo -e "${model}\t${arch}\t${seed}\t${val_acc}\t${best_epoch}\t${label}\t${domain}\t${total}" >> "$TEMP_SEED"
done < "$PER_SEED_FILE"
mv "$TEMP_SEED" "$PER_SEED_FILE"

# ── Step 3: Select best seed per architecture ────────────────────────

echo "── Best seed selection ──"
echo ""

select_best() {
    local arch="$1"
    local best_name=""
    local best_label=0

    for seed in "${SEEDS[@]}"; do
        local name="sherlock-v19-${arch}-s${seed}"
        local label
        label=$(_score_field "$name" label)
        if [[ -n "$label" ]] && [[ "$label" -gt "$best_label" ]]; then
            best_label="$label"
            best_name="$name"
        fi
    done
    echo "$best_name"
}

BEST_RELU=$(select_best "relu")
BEST_GELU=$(select_best "gelu")

echo "Best ReLU+BN: ${BEST_RELU:-NONE} ($(_score_field "$BEST_RELU" label)/$(_score_field "$BEST_RELU" total))"
echo "Best GELU+LN: ${BEST_GELU:-NONE} ($(_score_field "$BEST_GELU" label)/$(_score_field "$BEST_GELU" total))"
# Update baseline from fresh scoring
BASELINE_SCORE=$(_score_field "v16" label)
BASELINE_TOTAL=$(_score_field "v16" total)
BASELINE_SCORE=${BASELINE_SCORE:-297}
BASELINE_TOTAL=${BASELINE_TOTAL:-352}
echo "Baseline v16: ${BASELINE_SCORE}/${BASELINE_TOTAL}"
echo ""

# ── Step 4: Per-column three-way diff ────────────────────────────────

echo "── Per-column three-way diff ──"
echo ""

DIFF_FILE="$DIAG_DIR/v19_per_column_diff.tsv"

if [[ -n "$BEST_RELU" ]] && [[ -n "$BEST_GELU" ]]; then
    # Generate per-column detail for baseline, best-relu, best-gelu
    for model_name in "v16" "$BEST_RELU" "$BEST_GELU"; do
        local_dir="$DIAG_DIR/v19_eval_${model_name}"
        if [[ -d "$local_dir" ]] && [[ -f "$local_dir/profile_results.csv" ]]; then
            score_model_detail "models/$model_name" "$model_name" > "$DIAG_DIR/v19_detail_${model_name}.csv" 2>/dev/null || true
        fi
    done

    # Three-way join
    if [[ -f "$DIAG_DIR/v19_detail_v16.csv" ]] && \
       [[ -f "$DIAG_DIR/v19_detail_${BEST_RELU}.csv" ]] && \
       [[ -f "$DIAG_DIR/v19_detail_${BEST_GELU}.csv" ]]; then

        duckdb -separator $'\t' -c "
            CREATE TABLE v16 AS SELECT * FROM read_csv('$DIAG_DIR/v19_detail_v16.csv', auto_detect=true);
            CREATE TABLE relu AS SELECT * FROM read_csv('$DIAG_DIR/v19_detail_${BEST_RELU}.csv', auto_detect=true);
            CREATE TABLE gelu AS SELECT * FROM read_csv('$DIAG_DIR/v19_detail_${BEST_GELU}.csv', auto_detect=true);

            SELECT
                v16.column_name,
                v16.domain,
                v16.gt_label,
                v16.prediction AS v16_pred,
                v16.result AS v16_result,
                relu.prediction AS relu_pred,
                relu.result AS relu_result,
                gelu.prediction AS gelu_pred,
                gelu.result AS gelu_result,
                CASE
                    WHEN v16.result = 'MISS' AND relu.result = 'MATCH' THEN 'FIX'
                    WHEN v16.result = 'MATCH' AND relu.result = 'MISS' THEN 'REGRESSION'
                    WHEN v16.result = 'MISS' AND relu.result = 'MISS' THEN 'PERSISTENT'
                    ELSE 'STABLE'
                END AS relu_delta,
                CASE
                    WHEN v16.result = 'MISS' AND gelu.result = 'MATCH' THEN 'FIX'
                    WHEN v16.result = 'MATCH' AND gelu.result = 'MISS' THEN 'REGRESSION'
                    WHEN v16.result = 'MISS' AND gelu.result = 'MISS' THEN 'PERSISTENT'
                    ELSE 'STABLE'
                END AS gelu_delta
            FROM v16
            LEFT JOIN relu ON v16.column_name = relu.column_name
            LEFT JOIN gelu ON v16.column_name = gelu.column_name
            ORDER BY v16.domain, v16.column_name;
        " 2>/dev/null > "$DIFF_FILE" || echo "WARNING: three-way diff query failed"

        echo "Per-column diff saved to $DIFF_FILE"

        # Summary counts
        if [[ -f "$DIFF_FILE" ]] && [[ -s "$DIFF_FILE" ]]; then
            echo ""
            echo "  ReLU+BN changes vs v16:"
            grep -c "FIX" <<< "$(cut -f10 "$DIFF_FILE" 2>/dev/null)" 2>/dev/null | xargs -I{} echo "    Fixes: {}" || true
            grep -c "REGRESSION" <<< "$(cut -f10 "$DIFF_FILE" 2>/dev/null)" 2>/dev/null | xargs -I{} echo "    Regressions: {}" || true
            echo ""
            echo "  GELU+LN changes vs v16:"
            grep -c "FIX" <<< "$(cut -f11 "$DIFF_FILE" 2>/dev/null)" 2>/dev/null | xargs -I{} echo "    Fixes: {}" || true
            grep -c "REGRESSION" <<< "$(cut -f11 "$DIFF_FILE" 2>/dev/null)" 2>/dev/null | xargs -I{} echo "    Regressions: {}" || true
        fi
    else
        echo "WARNING: detail files missing for three-way diff"
    fi
elif [[ -n "$BEST_RELU" ]] || [[ -n "$BEST_GELU" ]]; then
    echo "Only one architecture available — two-way diff only"
fi
echo ""

# ── Step 5: Per-domain delta ─────────────────────────────────────────

echo "── Per-domain delta ──"
echo ""

DOMAIN_FILE="$DIAG_DIR/v19_per_domain_delta.tsv"

compute_domain_delta() {
    local model_name="$1"
    local detail_csv="$DIAG_DIR/v19_detail_${model_name}.csv"

    if [[ ! -f "$detail_csv" ]]; then
        return
    fi

    duckdb -csv -c "
        SELECT
            domain,
            count(*) AS total,
            sum(CASE WHEN result = 'MATCH' THEN 1 ELSE 0 END) AS correct
        FROM read_csv('$detail_csv', auto_detect=true)
        GROUP BY domain
        ORDER BY domain;
    " 2>/dev/null
}

# Compute per-domain for all three
V16_DOMAIN=$(compute_domain_delta "v16")

echo -e "domain\tv16\trelu_v19\tgelu_v19\trelu_delta\tgelu_delta" > "$DOMAIN_FILE"

if [[ -n "$V16_DOMAIN" ]]; then
    # Build domain tables and compute deltas in DuckDB
    V16_DETAIL="$DIAG_DIR/v19_detail_v16.csv"
    RELU_DETAIL="$DIAG_DIR/v19_detail_${BEST_RELU}.csv"
    GELU_DETAIL="$DIAG_DIR/v19_detail_${BEST_GELU}.csv"

    domain_query="
        CREATE TABLE v16d AS
        SELECT domain, count(*) AS total,
            sum(CASE WHEN result='MATCH' THEN 1 ELSE 0 END) AS correct
        FROM read_csv('$V16_DETAIL', auto_detect=true) GROUP BY domain;
    "

    if [[ -f "$RELU_DETAIL" ]]; then
        domain_query+="
        CREATE TABLE relud AS
        SELECT domain, sum(CASE WHEN result='MATCH' THEN 1 ELSE 0 END) AS correct
        FROM read_csv('$RELU_DETAIL', auto_detect=true) GROUP BY domain;
        "
    fi

    if [[ -f "$GELU_DETAIL" ]]; then
        domain_query+="
        CREATE TABLE gelud AS
        SELECT domain, sum(CASE WHEN result='MATCH' THEN 1 ELSE 0 END) AS correct
        FROM read_csv('$GELU_DETAIL', auto_detect=true) GROUP BY domain;
        "
    fi

    domain_query+="
        SELECT
            v.domain,
            v.correct || '/' || v.total AS v16,
    "
    if [[ -f "$RELU_DETAIL" ]]; then
        domain_query+="COALESCE(CAST(r.correct AS VARCHAR), '-') AS relu_v19,
        "
    else
        domain_query+="'-' AS relu_v19,
        "
    fi
    if [[ -f "$GELU_DETAIL" ]]; then
        domain_query+="COALESCE(CAST(g.correct AS VARCHAR), '-') AS gelu_v19,
        "
    else
        domain_query+="'-' AS gelu_v19,
        "
    fi
    if [[ -f "$RELU_DETAIL" ]]; then
        domain_query+="COALESCE(r.correct - v.correct, 0) AS relu_delta,
        "
    else
        domain_query+="0 AS relu_delta,
        "
    fi
    if [[ -f "$GELU_DETAIL" ]]; then
        domain_query+="COALESCE(g.correct - v.correct, 0) AS gelu_delta
        "
    else
        domain_query+="0 AS gelu_delta
        "
    fi
    domain_query+="FROM v16d v"
    if [[ -f "$RELU_DETAIL" ]]; then
        domain_query+=" LEFT JOIN relud r ON v.domain = r.domain"
    fi
    if [[ -f "$GELU_DETAIL" ]]; then
        domain_query+=" LEFT JOIN gelud g ON v.domain = g.domain"
    fi
    domain_query+=" ORDER BY v.domain;"

    duckdb -separator $'\t' -noheader -c "$domain_query" 2>/dev/null >> "$DOMAIN_FILE" || true

    echo "Per-domain delta saved to $DOMAIN_FILE"
    # Display it
    column -t -s $'\t' "$DOMAIN_FILE" 2>/dev/null || cat "$DOMAIN_FILE"
fi
echo ""

# ── Step 6: MADR 0066 gate evaluation ───────────────────────────────

echo "================================================================"
echo " MADR 0066 Hard Gate Evaluation"
echo "================================================================"
echo ""

GATE_FILE="$DIAG_DIR/v19_gate_results.tsv"
echo -e "architecture\tgate_1_seeds\tgate_2_winner\tgate_3_label_delta\tgate_4_domain_delta\tgate_5_max_domain_regression\tgate_6_diff_published\tverdict" > "$GATE_FILE"

evaluate_gate() {
    local arch="$1"         # relu or gelu
    local arch_label="$2"   # ReLU+BN or GELU+LN
    local best_name="$3"

    echo "── $arch_label ──"

    # Gate 1: 3 seeds completed with val_acc >= 0.912
    local seeds_ok=0
    for seed in "${SEEDS[@]}"; do
        local name="sherlock-v19-${arch}-s${seed}"
        local results_json="models/$name/results.json"
        if [[ -f "$results_json" ]]; then
            local val_acc
            val_acc=$(duckdb -csv -noheader -c "
                SELECT ROUND(MAX(val_accuracy), 4)
                FROM read_json('$results_json', format='array',
                    columns={val_accuracy: 'DOUBLE'});
            " 2>/dev/null || echo "0")
            if [[ -n "$val_acc" ]] && (( $(echo "$val_acc >= 0.912" | bc -l 2>/dev/null || echo 0) )); then
                seeds_ok=$((seeds_ok + 1))
            fi
        fi
    done
    local g1="$seeds_ok/3"
    local g1_pass=$( [[ $seeds_ok -eq 3 ]] && echo "PASS" || echo "FAIL" )
    echo "  Gate 1 (3 seeds, val_acc >= 0.912): $g1 $g1_pass"

    # Gate 2: Winner selected
    local g2_pass
    if [[ -n "$best_name" ]]; then
        g2_pass="PASS"
        echo "  Gate 2 (winner selected): $best_name PASS"
    else
        g2_pass="FAIL"
        echo "  Gate 2 (winner selected): NONE FAIL"
    fi

    # Gates 3-6 require profile eval results
    local label domain total
    label=$(_score_field "$best_name" label)
    domain=$(_score_field "$best_name" domain)
    total=$(_score_field "$best_name" total)
    label=${label:-0}; domain=${domain:-0}; total=${total:-$BASELINE_TOTAL}

    # Gate 3: net_label_delta >= +1
    local label_delta=$(( label - BASELINE_SCORE ))
    local g3_pass=$( [[ $label_delta -ge 1 ]] && echo "PASS" || echo "FAIL" )
    echo "  Gate 3 (net_label_delta >= +1): ${label_delta} (${label} vs ${BASELINE_SCORE}) $g3_pass"

    # Gate 4: net_domain_delta >= 0
    local v16_domain
    v16_domain=$(_score_field "v16" domain)
    v16_domain=${v16_domain:-0}
    local domain_delta=$(( domain - v16_domain ))
    local g4_pass=$( [[ $domain_delta -ge 0 ]] && echo "PASS" || echo "FAIL" )
    echo "  Gate 4 (net_domain_delta >= 0): ${domain_delta} $g4_pass"

    # Gate 5: no domain regresses > 3
    local max_regression=0
    local g5_pass="PASS"
    if [[ -f "$DOMAIN_FILE" ]]; then
        local delta_col
        [[ "$arch" == "relu" ]] && delta_col=5 || delta_col=6
        while IFS=$'\t' read -r dom _ _ _ relu_d gelu_d; do
            [[ "$dom" == "domain" ]] && continue
            local d
            [[ "$arch" == "relu" ]] && d="$relu_d" || d="$gelu_d"
            d="${d//[[:space:]]/}"
            if [[ -n "$d" ]] && [[ "$d" != "-" ]]; then
                local abs_d
                if [[ $d -lt 0 ]]; then abs_d=$(( -d )); else abs_d=0; fi
                if [[ $d -lt 0 ]] && [[ $abs_d -gt $max_regression ]]; then
                    max_regression=$abs_d
                fi
            fi
        done < "$DOMAIN_FILE"
    fi
    [[ $max_regression -gt 3 ]] && g5_pass="FAIL"
    echo "  Gate 5 (max domain regression <= 3): ${max_regression} $g5_pass"

    # Gate 6: per-column diff published
    local g6_pass
    if [[ -f "$DIFF_FILE" ]] && [[ -s "$DIFF_FILE" ]]; then
        g6_pass="PASS"
    else
        g6_pass="FAIL"
    fi
    echo "  Gate 6 (per-column diff published): $g6_pass"

    # Overall verdict
    local verdict="PASS"
    for g in "$g1_pass" "$g2_pass" "$g3_pass" "$g4_pass" "$g5_pass" "$g6_pass"; do
        [[ "$g" == "FAIL" ]] && verdict="FAIL"
    done
    echo ""
    echo "  VERDICT: $verdict"
    echo ""

    echo -e "${arch_label}\t${g1}\t${best_name:-NONE}\t${label_delta}\t${domain_delta}\t${max_regression}\t${g6_pass}\t${verdict}" >> "$GATE_FILE"
}

evaluate_gate "relu" "ReLU+BN" "$BEST_RELU"
evaluate_gate "gelu" "GELU+LN" "$BEST_GELU"

# ── Step 7: Final recommendation ────────────────────────────────────

echo "================================================================"
echo " Recommendation"
echo "================================================================"
echo ""

RELU_VERDICT=$(grep "ReLU" "$GATE_FILE" | cut -f8)
GELU_VERDICT=$(grep "GELU" "$GATE_FILE" | cut -f8)

RELU_LABEL=$(_score_field "$BEST_RELU" label); RELU_LABEL=${RELU_LABEL:-0}
GELU_LABEL=$(_score_field "$BEST_GELU" label); GELU_LABEL=${GELU_LABEL:-0}
RELU_TOTAL=$(_score_field "$BEST_RELU" total); RELU_TOTAL=${RELU_TOTAL:-0}
GELU_TOTAL=$(_score_field "$BEST_GELU" total); GELU_TOTAL=${GELU_TOTAL:-0}

if [[ "$RELU_VERDICT" == "PASS" ]] && [[ "$GELU_VERDICT" == "PASS" ]]; then
    # Winner takes all
    if [[ "$GELU_LABEL" -ge "$RELU_LABEL" ]]; then
        echo "WINNER: GELU+LN ($BEST_GELU) — ${GELU_LABEL}/${GELU_TOTAL}"
        echo "Both architectures passed. GELU+LN wins (ties go to GELU per winner-takes-all)."
    else
        echo "WINNER: ReLU+BN ($BEST_RELU) — ${RELU_LABEL}/${RELU_TOTAL}"
        echo "Both architectures passed. ReLU+BN wins on label score."
    fi
elif [[ "$RELU_VERDICT" == "PASS" ]]; then
    echo "WINNER: ReLU+BN ($BEST_RELU) — ${RELU_LABEL}/${RELU_TOTAL}"
    echo "Only ReLU+BN passed the gate."
elif [[ "$GELU_VERDICT" == "PASS" ]]; then
    echo "WINNER: GELU+LN ($BEST_GELU) — ${GELU_LABEL}/${GELU_TOTAL}"
    echo "Only GELU+LN passed the gate."
else
    echo "NO WINNER: Neither architecture passed the MADR 0066 gate."
    echo "Per constraint: no makeup runs. Re-run the full 3-seed set if needed."
fi

echo ""
echo "Gate results: $GATE_FILE"
echo "Per-seed results: $PER_SEED_FILE"
echo "Per-column diff: $DIFF_FILE"
echo "Per-domain delta: $DOMAIN_FILE"
echo ""
echo "================================================================"
