#!/usr/bin/env bash
# Does the day-first leaf have legitimate real-world users that the tightening
# would hurt?
#
# This is the question a tightening on THIS leaf has to answer and one on the
# year-first leaf did not. DD-MM-YYYY is the ordering most of the world writes;
# an over-tight rule here would be worse than the disease, stripping the date
# transform off ordinary British, Australian, German, French and Indian date
# columns and shipping them as confident integers. That is the same
# confident-and-wrong failure the change exists to remove, pointed the other
# way — and it is exactly what a century year window did to the year-first leaf
# when a reviewer tried it on 1865-1872 dates.
#
# So: profile realistic non-US date columns, on both sides, and compare the FULL
# emitted record. A column whose record is byte-identical across the change is a
# column the change did not touch.
#
# The four fixtures, and why each is here:
#
#   uk_date_of_birth         COMPACT day-first, EVERY day 13-31. Month-first is
#                            arithmetically impossible for every value, so this
#                            is the least ambiguous genuine `compact_dmy`
#                            column that can be constructed.
#   genealogy_19th_century   COMPACT day-first, all years 1844-1880. The case a
#                            century year window destroys.
#   au_transaction_slash     DD/MM/YYYY. Separator-bearing day-first — a
#                            different leaf, here to catch the change reaching
#                            sideways through the model's validation branch.
#   de_invoice_dot           DD.MM.YYYY, non-English header. Same, other
#                            separator.
#
# Each side is built AND run inside its own label state, for the reason spelled
# out in `probe_compact_date_residual.sh`: `veto_safe.txt` is compiled in and
# `definitions_datetime.yaml` is read from `./labels` at runtime, so a side is
# only honest when both are in place together.
#
# ALWAYS restores the working-tree label files on exit.
#
# Usage: compact_dmy_day_first_reality_check.sh [out-tsv] [base-ref]
set -eo pipefail
cd "$(cd "$(dirname "$0")/.." && pwd)"

OUT="${1:-docs/compact-dmy-day-first-reality-check.tsv}"
BASE_REF="${2:-origin/main}"
YAML=labels/definitions_datetime.yaml
SAFE=labels/veto_safe.txt
BIN=./target/release/finetype

FIXTURES=(
  compact_dmy_uk_date_of_birth
  compact_dmy_genealogy_19th_century
  compact_dmy_au_transaction_slash
  compact_dmy_de_invoice_dot
)
for f in "${FIXTURES[@]}"; do
  [ -f "tests/fixtures/$f.csv" ] || { echo "FAIL: missing tests/fixtures/$f.csv"; exit 1; }
done

WD="$(mktemp -d -t day-first-reality)"
cp "$YAML" "$WD/candidate_yaml_backup"
cp "$SAFE" "$WD/candidate_safe_backup"
restore() {
  git restore --staged "$YAML" "$SAFE" 2>/dev/null || true
  cp "$WD/candidate_yaml_backup" "$YAML"
  cp "$WD/candidate_safe_backup" "$SAFE"
}
trap restore EXIT

for spec in "base:$BASE_REF" "cand:"; do
  side="${spec%%:*}"; ref="${spec#*:}"
  echo "== side $side (${ref:-working tree}) =="
  if [ -n "$ref" ]; then git checkout "$ref" -- "$YAML" "$SAFE"; else restore; fi
  cargo build --release -p finetype-cli
  mkdir -p "$WD/$side"
  for f in "${FIXTURES[@]}"; do
    "$BIN" profile -f "tests/fixtures/$f.csv" -o csv > "$WD/$side/$f.csv" 2>>"$WD/$side.log"
  done
done
restore
cargo build --release -p finetype-cli

{
  printf '# day-first reality check — does the tightening hurt genuine non-US date columns?\n'
  printf '# head_sha\t%s\n' "$(git rev-parse HEAD)"
  printf '# base_ref\t%s\t%s\n' "$BASE_REF" "$(git rev-parse "$BASE_REF")"
  printf '# taxonomy_version\t%s\n' \
    "$(python3 scripts/evidence.py taxonomy-version 2>/dev/null | tr -d '\n' || echo unknown)"
  printf '# identical = the FULL emitted record is byte-for-byte the same on both sides\n'
  printf 'fixture\tidentical\tside\trecord\n'
  for f in "${FIXTURES[@]}"; do
    if cmp -s "$WD/base/$f.csv" "$WD/cand/$f.csv"; then same=YES; else same=NO; fi
    for side in base cand; do
      printf '%s\t%s\t%s\t%s\n' "$f" "$same" "$side" \
        "$(tail -n +2 "$WD/$side/$f.csv" | tr '\n' ' ')"
    done
  done
} > "$OUT"

echo "wrote $OUT"
cat "$OUT"

if grep -qP '\tNO\t' "$OUT" 2>/dev/null || awk -F'\t' '$2=="NO"' "$OUT" | grep -q .; then
  echo
  echo "FINDING: at least one genuine non-US date column MOVED across this change."
  echo "That is the outcome this check exists to catch — read the records above."
fi
