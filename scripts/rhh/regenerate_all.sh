#!/usr/bin/env bash
# regenerate_all.sh — reproducibility gate for rhh diagnostics (ac-09).
#
# Runs every step of the rhh pipeline in order and fingerprints outputs.
# Two modes:
#
#   ./regenerate_all.sh            — full regeneration (including ac-04,
#                                    which is slow: baseline + 22 disabled
#                                    profile runs on the 448-row manifest).
#   ./regenerate_all.sh --fast     — skip ac-04. Useful when you've already
#                                    generated rhh_counterfactual.tsv and
#                                    just want to re-derive downstream
#                                    artefacts (ac-05/06/07).
#
# Byte-identical contract (same machine, same model, same manifest):
#   rhh_family_inventory.tsv  — byte-identical across runs.
#   rhh_hit_counts.tsv        — byte-identical (profile outputs are
#                               deterministic given the pinned model).
#   rhh_counterfactual.tsv    — byte-identical on same machine; FP drift
#                               permitted cross-machine (see methodology).
#   rhh_classification.tsv    — byte-identical (derived from counterfactual).
#   rhh_domain_rollup.tsv     — byte-identical.
#   rhh_roadmap.tsv           — byte-identical.
#
# If drift is observed, diffs are written to diagnostics/rhh_drift_log.txt
# and the script exits 1.
#
# Prerequisites:
#   - models/default resolves to a multi-branch model directory.
#   - eval/datasets/manifest.csv is populated.
#   - For ac-04: finetype binary built with
#     `cargo build --release -p finetype-cli --features finetype-model/rhh-instrumentation`.
#     The script auto-detects target/release/finetype and warns if the
#     feature isn't compiled in.

set -euo pipefail

# Deterministic environment for the Python pipeline scripts. The ac-09
# gate's byte-identical contract depends on hash-set iteration order
# being pinned; PYTHONHASHSEED=0 achieves that. ac-04's finetype profile
# invocation is already deterministic given a pinned model + manifest.
export PYTHONHASHSEED=0

# Compat note: this script runs on bash 3.2 (stock macOS /bin/bash).
# No associative arrays; parallel indexed arrays instead.

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DIAG_DIR="${REPO_ROOT}/diagnostics"
SCRIPTS="${REPO_ROOT}/scripts/rhh"
FP_FILE="${DIAG_DIR}/rhh_fingerprints.sha256"
DRIFT_LOG="${DIAG_DIR}/rhh_drift_log.txt"
FAST=0

for arg in "$@"; do
    case "$arg" in
        --fast) FAST=1 ;;
        -h|--help)
            sed -n '2,40p' "${BASH_SOURCE[0]}"
            exit 0
            ;;
        *)
            echo "unknown argument: $arg" >&2
            exit 2
            ;;
    esac
done

cd "${REPO_ROOT}"

# ----- pre-flight -----
if [[ ! -f "eval/datasets/manifest.csv" ]]; then
    echo "ERROR: eval/datasets/manifest.csv missing — cannot profile." >&2
    exit 1
fi
if [[ ! -e "models/default" ]]; then
    echo "ERROR: models/default missing — set up the default model first." >&2
    exit 1
fi

# Files subject to byte-identical check.
ARTEFACTS=(
    "diagnostics/rhh_family_inventory.tsv"
    "diagnostics/rhh_hit_counts.tsv"
    "diagnostics/rhh_counterfactual.tsv"
    "diagnostics/rhh_counterfactual_summary.tsv"
    "diagnostics/rhh_classification.tsv"
    "diagnostics/rhh_domain_rollup.tsv"
    "diagnostics/rhh_roadmap.tsv"
)

# ----- capture pre-run fingerprints -----
# bash 3.2 compat: parallel indexed arrays (PRE_KEYS + PRE_HASHES) in
# place of an associative array. Lookup is via linear scan — negligible
# for |ARTEFACTS|=7.
PRE_KEYS=()
PRE_HASHES=()
for f in "${ARTEFACTS[@]}"; do
    PRE_KEYS+=("$f")
    if [[ -f "$f" ]]; then
        PRE_HASHES+=("$(shasum -a 256 "$f" | awk '{print $1}')")
    else
        PRE_HASHES+=("ABSENT")
    fi
done

# Lookup helper — emits the pre-hash for a given key, or empty string.
lookup_pre() {
    local key="$1"
    local i
    for (( i = 0; i < ${#PRE_KEYS[@]}; i++ )); do
        if [[ "${PRE_KEYS[$i]}" == "$key" ]]; then
            echo "${PRE_HASHES[$i]}"
            return 0
        fi
    done
    echo ""
}

# ----- run the pipeline -----
echo "==> ac-01 family inventory"
python3 "${SCRIPTS}/ac01_inventory.py"

echo "==> ac-03 hit counts"
python3 "${SCRIPTS}/ac03_hit_counts.py"

if [[ $FAST -eq 0 ]]; then
    echo "==> ac-04 counterfactual (slow — baseline + 22 disabled runs)"
    python3 "${SCRIPTS}/ac04_counterfactual.py"
else
    echo "==> ac-04 counterfactual SKIPPED (--fast); using existing TSV"
    if [[ ! -f "diagnostics/rhh_counterfactual.tsv" ]]; then
        echo "ERROR: --fast requires existing rhh_counterfactual.tsv" >&2
        exit 1
    fi
fi

echo "==> ac-05 classification"
python3 "${SCRIPTS}/ac05_classify.py"

echo "==> ac-06 domain rollup"
python3 "${SCRIPTS}/ac06_domain_rollup.py"

echo "==> ac-07 roadmap"
python3 "${SCRIPTS}/ac07_roadmap.py"

# ----- capture post-run fingerprints + compute diffs -----
: > "${DRIFT_LOG}"
drift=0
{
    echo "# rhh_fingerprints.sha256 — produced by scripts/rhh/regenerate_all.sh"
    echo "# Artefact sha256 fingerprints after the last pipeline regeneration."
    echo "# Byte-identical re-run is the ac-09 gate."
    for f in "${ARTEFACTS[@]}"; do
        hash_post="$(shasum -a 256 "$f" | awk '{print $1}')"
        echo "${hash_post}  ${f}"
        hash_pre="$(lookup_pre "$f")"
        if [[ -n "${hash_pre}" && "${hash_pre}" != "ABSENT" && "${hash_pre}" != "${hash_post}" ]]; then
            {
                echo "DRIFT: ${f}"
                echo "  pre:  ${hash_pre}"
                echo "  post: ${hash_post}"
            } >> "${DRIFT_LOG}"
            drift=1
        fi
    done
} > "${FP_FILE}"

if [[ $drift -eq 1 ]]; then
    echo
    echo "DRIFT DETECTED — see ${DRIFT_LOG}"
    cat "${DRIFT_LOG}"
    exit 1
fi

echo
echo "==> byte-identical: OK (fingerprints in ${FP_FILE})"
