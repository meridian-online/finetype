#!/usr/bin/env bash
# Rotating external-data advisory band (v0) — runnable wrapper. SCAFFOLD.
#
# Profiles the on-disk external audit tables with the shipped binary and scores
# them against the adjudicated external gold labels. Advisory, never blocking.
#
# Run from the repo root OR from this directory — paths are resolved relative to
# the repo root regardless.
set -euo pipefail

# --- resolve repo root (two dirs up from output/repo-review/scaffolds/...) ----
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"

BINARY="${BINARY:-$REPO_ROOT/target/release/finetype}"
DATASETS_DIR="${DATASETS_DIR:-$REPO_ROOT/eval/datasets/gold_external}"
LABELS="${LABELS:-$SCRIPT_DIR/held_labels_v0.tsv}"
OUT="${OUT:-$SCRIPT_DIR/report.md}"
ROTATE="${ROTATE:-0}"          # 0 = all tables; e.g. 4 for a 4-table round
SEED="${SEED:-$(date +%Y%m%d)}"

if [[ ! -x "$BINARY" ]]; then
  echo "ERROR: binary not found/executable at $BINARY" >&2
  echo "  build it: cargo build --release -p finetype-cli" >&2
  exit 1
fi

echo "external-band: binary=$BINARY rotate=$ROTATE seed=$SEED"
python3 "$SCRIPT_DIR/run_external_band.py" \
  --binary "$BINARY" \
  --datasets-dir "$DATASETS_DIR" \
  --labels "$LABELS" \
  --rotate "$ROTATE" \
  --seed "$SEED" \
  --out "$OUT"

echo "report -> $OUT"
