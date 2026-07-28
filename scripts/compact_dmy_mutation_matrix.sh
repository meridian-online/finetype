#!/usr/bin/env bash
# Does each guard in the day-first compact-date change have a test that DIES
# when it is removed?
#
# Why this exists rather than a line of prose. This repo has repeatedly shipped
# structural guards that pass on broken code — a test asserting a SHAPE near an
# invariant, holding by accident of its fixture. The year-first sibling of this
# very change had a first revision whose REJECT set no single-window mutation
# could redden: deleting a window left the file green. A tightening is only
# defended when a named test dies for each clause, so this script deletes each
# clause in turn and records which tests die.
#
# The mutations are applied to the SHIPPED artefacts — `labels/
# definitions_datetime.yaml` and `labels/veto_safe.txt` — not to a copy, because
# those two files ARE the change. Nothing is recomputed from the pattern: every
# verdict comes from running the real suites.
#
# Both suites are run for every mutation:
#   finetype-core --test precision_tightenings   the validator layer
#   finetype-cli  --test cli_golden -- --ignored the full emitted record
# The CLI suite is `#[ignore]` by file convention (it loads the model) and so
# never runs in CI; that is exactly why it is run here.
#
# A mutation with an empty "died" column is a FINDING, not a pass.
#
# ALWAYS restores both label files on exit.
#
# Usage: compact_dmy_mutation_matrix.sh [out-md]
set -eo pipefail
cd "$(cd "$(dirname "$0")/.." && pwd)"

OUT="${1:-docs/compact-dmy-mutation-matrix.md}"
YAML=labels/definitions_datetime.yaml
SAFE=labels/veto_safe.txt

# The pattern EXACTLY as the YAML stores it — backslashes doubled, because the
# value is a double-quoted YAML scalar. Comparing against the single-backslash
# regex form silently matches nothing and every mutation below reads as a pass.
SHIPPED='^(0[1-9]|[12]\\d|3[01])(0[1-9]|1[0-2])\\d{4}$'

WD="$(mktemp -d -t compact-dmy-mutation)"
cp "$YAML" "$WD/yaml.orig"
cp "$SAFE" "$WD/safe.orig"
restore() { cp "$WD/yaml.orig" "$YAML"; cp "$WD/safe.orig" "$SAFE"; }
trap restore EXIT

grep -qF "$SHIPPED" "$YAML" || {
  echo "FAIL: the shipped day-first pattern is not in $YAML — this script is stale"; exit 1; }
grep -qx "datetime.date.compact_dmy" "$SAFE" || {
  echo "FAIL: compact_dmy is not on $SAFE — this script is stale"; exit 1; }

# name | what it removes | replacement pattern ('' = leave the pattern alone)
# | drop the allowlist line (yes/no)
MUTATIONS=(
  "revert_whole_change|both windows and the allowlist entry — the change undone|^\\\\d{8}\$|yes"
  "drop_day_window|the DAY window only (01-31 -> any two digits)|^\\\\d{2}(0[1-9]|1[0-2])\\\\d{4}\$|no"
  "drop_month_window|the MONTH window only (01-12 -> any two digits)|^(0[1-9]|[12]\\\\d|3[01])\\\\d{2}\\\\d{4}\$|no"
  "drop_year_digits|the YEAR's four-DIGIT requirement (\\\\d{4} -> .{4})|^(0[1-9]|[12]\\\\d|3[01])(0[1-9]|1[0-2]).{4}\$|no"
  "drop_allowlist_entry|the veto-safe allowlist entry only — windows intact||yes"
  "add_century_year_window|nothing; ADDS a year window (the over-tightening)|^(0[1-9]|[12]\\\\d|3[01])(0[1-9]|1[0-2])(19|20)\\\\d{2}\$|no"
)

run_suites() {  # run_suites <tag> -> prints "test_name<TAB>ok|FAILED" lines
  local tag="$1"
  cargo test -p finetype-core --test precision_tightenings \
    > "$WD/$tag.core" 2>&1 || true
  cargo test -p finetype-cli --test cli_golden -- --ignored compact_dmy \
    > "$WD/$tag.cli" 2>&1 || true
  # The DEFAULT libtest format is required here: `--format=terse` prints
  # progress dots instead of one `test <name> ... ok` line per test, the grep
  # below then matches nothing, and under `pipefail` the whole script exits
  # silently at the baseline with every mutation unreported.
  grep -hoE '^test [a-z0-9_]+ \.\.\. (ok|FAILED)' "$WD/$tag.core" "$WD/$tag.cli" \
    | sed 's/^test //; s/ \.\.\. /\t/'
}

echo "== baseline: the shipped change, unmutated =="
run_suites baseline > "$WD/baseline.tsv"
BASE_TOTAL=$(wc -l < "$WD/baseline.tsv" | tr -d ' ')
BASE_FAIL=$(grep -c 'FAILED' "$WD/baseline.tsv" || true)
echo "   $BASE_TOTAL tests, $BASE_FAIL failing"
[ "$BASE_FAIL" = "0" ] || { echo "FAIL: the unmutated tree is not green; nothing below means anything"; exit 1; }

: > "$WD/rows.tsv"
for spec in "${MUTATIONS[@]}"; do
  IFS='|' read -r name what pattern drop_safe <<< "$spec"
  echo "== mutation $name : removes $what =="
  restore
  if [ -n "$pattern" ]; then
    python3 - "$YAML" "$SHIPPED" "$pattern" <<'PY'
import sys
path, old, new = sys.argv[1:4]
with open(path) as fh:
    src = fh.read()
# Both literals are already in the YAML's doubled-backslash form. Requiring
# EXACTLY one occurrence is the guard: the day-first pattern is unique in the
# file, so a zero here means the script has gone stale against the taxonomy and
# every mutation would otherwise report a silent pass.
if src.count(old) != 1:
    sys.exit(f"pattern to mutate occurs {src.count(old)} times in {path}, expected 1")
with open(path, "w") as fh:
    fh.write(src.replace(old, new))
PY
  fi
  if [ "$drop_safe" = yes ]; then
    grep -vx "datetime.date.compact_dmy" "$SAFE" > "$WD/safe.mut" && mv "$WD/safe.mut" "$SAFE"
  fi
  run_suites "$name" > "$WD/$name.tsv"
  died=$(awk -F'\t' '$2=="FAILED"{printf "%s ", $1}' "$WD/$name.tsv")
  n_died=$(awk -F'\t' '$2=="FAILED"' "$WD/$name.tsv" | wc -l | tr -d ' ')
  ran=$(wc -l < "$WD/$name.tsv" | tr -d ' ')
  printf '%s\t%s\t%s\t%s\t%s\n' "$name" "$what" "$ran" "$n_died" "${died:-NONE}" >> "$WD/rows.tsv"
  echo "   $n_died/$ran died: ${died:-NONE}"
done

restore
echo "== restored; re-verifying the tree is green =="
run_suites final > "$WD/final.tsv"
FINAL_FAIL=$(grep -c 'FAILED' "$WD/final.tsv" || true)
[ "$FINAL_FAIL" = "0" ] || { echo "FAIL: the tree did not come back green after mutation"; exit 1; }

{
  echo "# Day-first compact-date change — mutation matrix"
  echo
  echo "Generated by \`scripts/compact_dmy_mutation_matrix.sh\`."
  echo
  echo "- head_sha: \`$(git rev-parse HEAD)\`"
  echo "- taxonomy_version: \`$(python3 scripts/evidence.py taxonomy-version 2>/dev/null | tr -d '\n' || echo unknown)\`"
  echo "- suites: \`finetype-core --test precision_tightenings\` + \`finetype-cli --test cli_golden -- --ignored compact_dmy\`"
  echo "- unmutated tree: **$BASE_TOTAL tests, 0 failing**"
  echo
  echo "Each row deletes ONE clause of the change and reports which tests die."
  echo "**An empty \`died\` column is a finding, not a pass** — it means that"
  echo "clause is load-bearing in production and defended by nothing."
  echo
  echo "| mutation | removes | ran | died | which |"
  echo "|---|---|---:|---:|---|"
  while IFS=$'\t' read -r name what ran n_died died; do
    printf '| `%s` | %s | %s | **%s** | %s |\n' "$name" "$what" "$ran" "$n_died" \
      "$(echo "$died" | sed 's/ $//; s/ /<br>/g')"
  done < "$WD/rows.tsv"
  echo
  echo "Raw per-mutation results are regenerated by re-running the script; the"
  echo "table above is the whole record it produces."
} > "$OUT"

echo "wrote $OUT"
cat "$OUT"
