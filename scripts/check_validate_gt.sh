#!/bin/bash
# Validate iter-2 GT sidecars: cardinality + label validity.
#
# AC-02 of .orbit/specs/2026-04-28-validate-corpus-curation/spec.yaml.
#
# Two invariants per sidecar:
#   (a) cardinality — `columns:` map size equals the CSV's header column count
#   (b) label validity — each `expected_label` value resolves to a real
#       FineType taxonomy type (rejects typos like geography.location.us_state)
#
# Label validity is checked via content-grep on `finetype taxonomy <label>`
# stderr/stdout for `^Error: unknown type` rather than relying on the exit
# code, because the binary exits 0 for prefix-matching typos that resolve
# to a valid namespace but unknown leaf (verified empirically on v0.6.19).
#
# Exit codes:
#   0   all sidecars + labels validated
#   1   at least one cardinality or label validation failed
#   2   tooling problem (binary missing, sidecar missing, yq missing)
#
# Run from the repo root.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

SIDECARS=(
    eval/datasets/validate_corpus/pokemon.gt.yaml
    eval/datasets/validate_corpus/rio2016_athletes.gt.yaml
    eval/datasets/validate_corpus/us_baby_names.gt.yaml
    eval/datasets/validate_corpus/co2_emissions_by_nation.gt.yaml
    eval/datasets/validate_corpus/world_population.gt.yaml
    eval/datasets/validate_corpus/un_locode.gt.yaml
    eval/datasets/validate_corpus/global_temp_annual.gt.yaml
    eval/datasets/validate_corpus/sp500_constituents.gt.yaml
    eval/datasets/validate_corpus/nyc_taxi.gt.yaml
    eval/datasets/validate_corpus/gdelt_events.gt.yaml
    eval/datasets/validate_corpus/fifa_players.gt.yaml
    eval/datasets/validate_corpus/oecd_employment.gt.yaml
)

# Locate the finetype binary — prefer release build, fall back to debug, fall
# back to PATH.
if [ -x "target/release/finetype" ]; then
    FINETYPE=target/release/finetype
elif [ -x "target/debug/finetype" ]; then
    FINETYPE=target/debug/finetype
elif command -v finetype >/dev/null 2>&1; then
    FINETYPE=finetype
else
    echo "FAIL: finetype binary not found (target/{release,debug}/finetype or PATH)" >&2
    exit 2
fi

fail_count=0

for sidecar in "${SIDECARS[@]}"; do
    if [ ! -f "$sidecar" ]; then
        echo "FAIL: sidecar missing: $sidecar" >&2
        fail_count=$((fail_count + 1))
        continue
    fi

    csv="eval/datasets/validate_corpus/csv/$(basename "$sidecar" .gt.yaml).csv"
    if [ ! -f "$csv" ]; then
        echo "FAIL: $sidecar: csv missing at $csv" >&2
        fail_count=$((fail_count + 1))
        continue
    fi

    # (a) cardinality
    csv_cols=$(head -1 "$csv" | python3 -c "import sys,csv; print(len(next(csv.reader(sys.stdin))))")
    yaml_cols=$(python3 -c "
import yaml,sys
data = yaml.safe_load(open('$sidecar')) or {}
cols = data.get('columns') or {}
print(len(cols))
")
    if [ "$csv_cols" -ne "$yaml_cols" ]; then
        echo "FAIL: $sidecar cardinality: csv=$csv_cols yaml=$yaml_cols" >&2
        fail_count=$((fail_count + 1))
        continue
    fi

    # (b) label validity — content-grep on 'Error: unknown type'
    labels=$(python3 -c "
import yaml
data = yaml.safe_load(open('$sidecar')) or {}
cols = data.get('columns') or {}
for v in cols.values():
    print(v)
")
    while IFS= read -r label; do
        [ -z "$label" ] && continue
        out=$("$FINETYPE" taxonomy "$label" -o json-schema 2>&1 || true)
        if echo "$out" | grep -qE '^Error: unknown type'; then
            echo "FAIL: $sidecar bad label: $label" >&2
            fail_count=$((fail_count + 1))
        fi
    done <<<"$labels"
done

if [ "$fail_count" -gt 0 ]; then
    echo "check_validate_gt.sh: $fail_count failures" >&2
    exit 1
fi

echo "check_validate_gt.sh: all ${#SIDECARS[@]} sidecars validated (cardinality + labels)"
exit 0
