#!/usr/bin/env bash
# Resume distillation v2 — show status and generate launch instructions.
#
# Usage:
#   ./scripts/resume_distillation.sh              # Show status
#   ./scripts/resume_distillation.sh --cleanup     # Remove partial CSVs for re-processing

set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

if [[ "${1:-}" == "--cleanup" ]]; then
    echo "Checking for partial batch CSVs..."
    for jsonl in output/distillation-v2/batch_*.jsonl; do
        batch=$(basename "$jsonl" .jsonl)
        csv="output/distillation-v2/${batch}.csv"
        expected=$(wc -l < "$jsonl")
        if [[ -f "$csv" ]]; then
            actual=$(($(wc -l < "$csv") - 1))
            if [[ $actual -lt $expected ]]; then
                echo "  Removing partial $batch.csv ($actual/$expected columns)"
                rm -f "$csv"
            fi
        fi
    done
    echo "Done. Partial files cleaned up."
    echo ""
fi

python3 scripts/distillation_status.py output/distillation-v2/
