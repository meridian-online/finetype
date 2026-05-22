#!/usr/bin/env bash
# ac-13 — reproducibility test for the gittables corpus pass.
#
# Runs the corpus pass twice on a 100-file CALIBRATE-half sample
# (`file_content_sha256 % 2 == 0`) and asserts byte-identical
# outputs across all four ac-13 artefacts:
#
#   (a) columns.parquet
#   (b) mechanism_decomposition.parquet
#   (c) report.md
#   (d) corpus_pass_id (frontmatter value, derived from input hashes)
#
# Output goes to eval/gittables/corpus_pass_calibrate_reprocheck/
# (run_a/ + run_b/ subdirs) — distinct from the measure-half corpus
# pass to prevent cross-contamination.
#
# USAGE
#   source eval/gittables/.venv/bin/activate
#   bash scripts/test_corpus_pass_reproducibility.sh
#
# Exit code: 0 on success, non-zero on any byte-identity assertion fail.

set -euo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO"

OUT_ROOT="eval/gittables/corpus_pass_calibrate_reprocheck"
PATHS_FILE="eval/gittables/corpus_paths_calibrate_100.txt"
RUN_A="$OUT_ROOT/run_a"
RUN_B="$OUT_ROOT/run_b"

# --- (0) Build the 100-file calibrate paths sample ----------------------
if [[ ! -f "$PATHS_FILE" ]]; then
    echo "[ac-13] building calibrate-half paths sample..."
    python3 scripts/build_calibrate_paths_sample.py
fi
n_paths=$(wc -l < "$PATHS_FILE" | tr -d ' ')
echo "[ac-13] using $n_paths calibrate-half paths"

# --- (1) Run corpus pass twice ------------------------------------------
run_one() {
    local out_dir="$1"
    local label="$2"
    echo "[ac-13] run $label — corpus pass into $out_dir..."
    rm -rf "$out_dir"
    mkdir -p "$out_dir"
    python3 scripts/gittables_corpus_pass.py \
        --corpus-index "$PATHS_FILE" \
        --out-dir "$out_dir" \
        --partition calibrate \
        --jobs 8 \
        --execute \
        > "$out_dir/execute.log" 2>&1
    # Pass B — fill YDF predictions (same data flow as the full ac-06).
    python3 scripts/gittables_corpus_pass.py \
        --corpus-index "$PATHS_FILE" \
        --out-dir "$out_dir" \
        --partition calibrate \
        --jobs 1 \
        --fill-ydf \
        >> "$out_dir/execute.log" 2>&1

    echo "[ac-13] run $label — mechanism decomposition..."
    # The decomposition script reads from a fixed location; we make
    # symlinks in its expected location for the run, then move outputs.
    # For ac-13 we want to point everything at this run's dir.
    python3 scripts/build_mechanism_decomposition.py \
        --files-parquet "$out_dir/corpus_pass/files.parquet" \
        --columns-parquet "$out_dir/corpus_pass/columns.parquet" \
        --per-column-rejects "$out_dir/corpus_pass/per_column_rejects.parquet" \
        --out "$out_dir/corpus_pass/mechanism_decomposition.parquet" \
        --a-only \
        > "$out_dir/decomp.log" 2>&1 || true
    # ^ --a-only because the 100-file sample has no per_column_rejects
    # extract; the (b)-side is exercised at full scale in ac-08 and is
    # not part of the reproducibility contract per spec verification
    # (which names columns.parquet, mechanism_decomposition.parquet,
    # report.md, corpus_pass_id — none of which require the (b)-side
    # to be populated).

    echo "[ac-13] run $label — corroborated gaps..."
    python3 scripts/build_corroborated_gaps.py \
        --columns-parquet "$out_dir/corpus_pass/columns.parquet" \
        --mechanism-decomposition "$out_dir/corpus_pass/mechanism_decomposition.parquet" \
        --out-corroborated "$out_dir/corpus_pass/corroborated_gaps.parquet" \
        --out-single-lens "$out_dir/corpus_pass/single_lens_signals.tsv" \
        > "$out_dir/corrob.log" 2>&1

    echo "[ac-13] run $label — diagnostic report..."
    python3 scripts/build_diagnostic_report.py \
        --out "$out_dir/corpus_pass/report.md" \
        --corroborated "$out_dir/corpus_pass/corroborated_gaps.parquet" \
        --files-parquet "$out_dir/corpus_pass/files.parquet" \
        --columns-parquet "$out_dir/corpus_pass/columns.parquet" \
        > "$out_dir/report.log" 2>&1
}

run_one "$RUN_A" "A"
run_one "$RUN_B" "B"

# --- (2) Byte-identical assertions --------------------------------------
echo "[ac-13] comparing artefacts..."
fail=0
check_identical() {
    local rel="$1"
    if ! cmp -s "$RUN_A/$rel" "$RUN_B/$rel"; then
        echo "  ✗ $rel — NOT byte-identical"
        fail=1
    else
        echo "  ✓ $rel"
    fi
}

check_identical corpus_pass/columns.parquet
check_identical corpus_pass/mechanism_decomposition.parquet
check_identical corpus_pass/report.md

# (d) corpus_pass_id frontmatter — extract and compare
id_a=$(awk '/^---$/{n++; next} n==1 && /^corpus_pass_id:/{print $2; exit}' "$RUN_A/corpus_pass/report.md")
id_b=$(awk '/^---$/{n++; next} n==1 && /^corpus_pass_id:/{print $2; exit}' "$RUN_B/corpus_pass/report.md")
if [[ "$id_a" == "$id_b" && -n "$id_a" ]]; then
    echo "  ✓ corpus_pass_id (both = $id_a)"
else
    echo "  ✗ corpus_pass_id mismatch: A=$id_a B=$id_b"
    fail=1
fi

if [[ $fail -ne 0 ]]; then
    echo "[ac-13] FAIL — at least one assertion did not hold."
    exit 1
fi

echo "[ac-13] PASS — all four assertions hold."
