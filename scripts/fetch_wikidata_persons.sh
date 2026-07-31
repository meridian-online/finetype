#!/usr/bin/env bash
# scripts/fetch_wikidata_persons.sh — idempotent Wikidata person-name fetch + register.
#
# Per spec 2026-05-25-v22-boundary-training ac-01.
#
# Pulls three name primitives from Wikidata Query Service to
# $HOME/datasets/wikidata/<YYYY-MM-DD>/:
#   - given_names.tsv     instances of "given name" (Q202444) with English label
#   - family_names.tsv    instances of "family name" (Q101352) with English label
#   - persons.tsv         Q-id-ranged sample of Q5 (Human) entities with English label
#
# Spec target: ≥ 2M person-name records. Achieved via combinatorial
# expansion in ac-02 — N_given × N_family ≫ 2M for the expected list
# sizes — plus the labelled Q5 sample for realism.
#
# Idempotent: re-running with the same date is a no-op once every
# expected file is present. Re-running with a new date fetches a fresh
# snapshot to a new dated directory (old snapshots are preserved).
#
# Usage:
#   ./scripts/fetch_wikidata_persons.sh                # snapshot today
#   ./scripts/fetch_wikidata_persons.sh 2026-05-25     # snapshot a specific date
#   ./scripts/fetch_wikidata_persons.sh --skip-register   # download only
#   ./scripts/fetch_wikidata_persons.sh --persons-target 50000   # raise/lower Q5 sample
#
# Disk: ~20-50 MB.
set -euo pipefail

REPO="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO"

DATE=""
SKIP_REGISTER=false
PERSONS_TARGET="0"  # default 0 — WDQS Q5 dense ranges are unreliable.
# Set to >0 to attempt a Q5 label sample; generator handles absence.
for arg in "$@"; do
    case "$arg" in
        --skip-register) SKIP_REGISTER=true ;;
        --persons-target=*) PERSONS_TARGET="${arg#*=}" ;;
        --persons-target) shift; PERSONS_TARGET="$1" ;;
        --help|-h)
            sed -n '2,/^set -/p' "$0" | grep '^#' | sed 's/^# \?//'; exit 0
            ;;
        -*) echo "Unknown flag: $arg" >&2; exit 1 ;;
        *)  DATE="$arg" ;;
    esac
done
DATE="${DATE:-$(date +%Y-%m-%d)}"

DEST_ROOT="/Users/hugh/datasets/wikidata"
DEST="$DEST_ROOT/$DATE"
mkdir -p "$DEST"

PYTHON="$REPO/eval/gittables/.venv/bin/python"
if [[ ! -x "$PYTHON" ]]; then PYTHON="python3"; fi

echo "── Fetch Wikidata person primitives to $DEST ────────────────────"
"$PYTHON" "$REPO/scripts/_fetch_wikidata_persons_impl.py" \
    --dest "$DEST" --persons-target "$PERSONS_TARGET"

echo
echo "── Inventory ─────────────────────────────────────────────────────"
( cd "$DEST" && du -sh . && find . -type f -printf "  %s\t%P\n" 2>/dev/null \
    || find . -type f | xargs -I{} sh -c 'printf "  %s\t%s\n" "$(stat -f %z "$1")" "${1#./}"' _ {} )

if [[ "$SKIP_REGISTER" == "true" ]]; then
    echo
    echo "── Skipping registration (--skip-register) ──────────────────────"
    exit 0
fi

echo
echo "── Register snapshot ─────────────────────────────────────────────"
cd "$REPO"
python3 scripts/dataset_register.py wikidata_q5 "$DEST" \
    --dataset-version "$DATE" \
    --role train --licence CC0-1.0 \
    --source-url "https://query.wikidata.org/sparql" \
    --attribution "Wikidata Q5 (Human) person-name primitives ($DATE snapshot) — given names (Q202444 instances), family names (Q101352 instances), and a Q-id-ranged sample of Q5 entity labels, fetched from Wikidata Query Service (query.wikidata.org) under the CC0 1.0 Public Domain Dedication. Used by FineType as the v22 training-data source for identity.person.full_name hard negatives per spec 2026-05-25-v22-boundary-training. Attribution: 'Source: Wikidata, dedicated to the public domain under CC0 1.0'."

echo
echo "── Verify ────────────────────────────────────────────────────────"
python3 scripts/dataset_verify.py wikidata_q5
