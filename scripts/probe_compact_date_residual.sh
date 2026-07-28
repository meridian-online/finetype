#!/usr/bin/env bash
# The compact-date residual probe: what the day-first leaf emits, per column
# family, on BOTH sides of a `datetime.date.compact_dmy` validator change.
#
# Why this exists. The corpus-honest gate certified the year-first tightening
# and could not see what that change did to the day-first sibling: its sample is
# ~3% of GitTables, non-adversarial, and it scores label transitions in
# aggregate. The defect here is not a label count — it is ONE column family
# moving from a low-confidence integer to a HIGH-confidence date with a
# `strptime` transform attached. Six hand-built column families, profiled
# end to end through the CLI, find it in seconds.
#
# The probe reads the FULL emitted record (`profile -o csv`: label, confidence,
# quality band, runner-up, broad type, format string, transform, is_generic,
# disambiguation rule, locale), because a label-only comparison cannot tell a
# fixed column from one that kept its date transform.
#
# LOCALE IS NOISE on this pipeline (eight profiles of one fixed column returned
# six different locales), so the locale field is emitted for completeness and
# must not be read as an effect of anything without a same-input repeat control.
#
# ALWAYS restores the working-tree YAML on exit.
#
# Usage:
#   probe_compact_date_residual.sh [out-tsv] [base-ref]
#     out-tsv  : default docs/compact-date-residual.tsv
#     base-ref : git ref holding the PRE-change validator (default origin/main)
set -eo pipefail
cd "$(cd "$(dirname "$0")/.." && pwd)"

OUT="${1:-docs/compact-date-residual.tsv}"
BASE_REF="${2:-origin/main}"
YAML=labels/definitions_datetime.yaml
SAFE=labels/veto_safe.txt
BIN=./target/release/finetype

# One fixture per row of the residual table. Each is a SINGLE column: the
# model is sibling-aware, so bundling the families into one table would let
# each one's verdict depend on its neighbours and the probe would stop being
# about the family under test.
FIXTURES=(
  compact_dmy_ymd_reject_set
  compact_dmy_sequential_ids
  compact_dmy_round_hundred_share_counts
  compact_dmy_unconstrained_eight_digit
  compact_dmy_genuine_ymd_dates
  compact_dmy_genuine_day_first_dates
)

WD="$(mktemp -d -t compact-date-probe)"
cp "$YAML" "$WD/candidate_yaml_backup"
cp "$SAFE" "$WD/candidate_safe_backup"
restore() {
  # `git checkout <ref> -- <path>` writes the INDEX as well as the worktree, so
  # a plain `cp` back would leave the files staged-modified against HEAD.
  git restore --staged "$YAML" "$SAFE" 2>/dev/null || true
  cp "$WD/candidate_yaml_backup" "$YAML"
  cp "$WD/candidate_safe_backup" "$SAFE"
}
trap restore EXIT

: > "$WD/files.txt"
for f in "${FIXTURES[@]}"; do
  [ -f "tests/fixtures/$f.csv" ] || { echo "FAIL: missing tests/fixtures/$f.csv"; exit 1; }
  echo "tests/fixtures/$f.csv" >> "$WD/files.txt"
done

# One invocation per fixture per format. `profile --files` (which would load the
# model once) is wired only for -o json-schema / -o datapackage, and neither
# carries the confidence, quality band, disambiguation rule or validation pass
# rate this probe exists to compare — so it pays the model load per file.
#
# `-o csv` is the mandated full-record view. `-o json` is taken alongside it for
# `validation_pass_rate` / `validation_advisory_low`, which the CSV writer does
# not emit and which are the difference between "the validator rejected this
# column" and "the validator rejected it and nothing acted on the rejection".
run_side() {  # run_side <tag> <binary>
  local tag="$1" bin="$2" f
  mkdir -p "$WD/$tag"
  for f in "${FIXTURES[@]}"; do
    "$bin" profile -f "tests/fixtures/$f.csv" -o csv  > "$WD/$tag/$f.csv"  2>>"$WD/$tag.log"
    "$bin" profile -f "tests/fixtures/$f.csv" -o json > "$WD/$tag/$f.json" 2>>"$WD/$tag.log"
  done
}

# TWO BINARIES, not one binary with the YAML swapped. Half of this change is
# `labels/definitions_datetime.yaml`, which `load_taxonomy` reads from disk at
# runtime. The other half is `labels/veto_safe.txt`, which `finetype-core` pulls
# in with `include_str!` at COMPILE time. Swapping only the YAML would run the
# baseline side with the candidate's hard-veto scope already in force, and the
# row this probe exists for — a column the validator rejects at 0.0 that ships
# as a confident date anyway — would not appear on it.
echo "== build the CANDIDATE binary (working tree) =="
cargo build --release -p finetype-cli
cp "$BIN" "$WD/finetype-candidate"

echo "== build the BASELINE binary ($BASE_REF label files) =="
git checkout "$BASE_REF" -- "$YAML" "$SAFE"
grep -q "datetime.date.compact_dmy" "$YAML"
cargo build --release -p finetype-cli
cp "$BIN" "$WD/finetype-baseline"

echo "== restore the candidate label files (explicit; trap is the backstop) =="
restore
cargo build --release -p finetype-cli

echo "== baseline side ($BASE_REF) =="
run_side baseline "$WD/finetype-baseline"

echo "== candidate side (working tree) =="
run_side candidate "$WD/finetype-candidate"

BIN_VERSION="$("$BIN" --version 2>/dev/null | tr -d '\n')"
HEAD_SHA="$(git rev-parse HEAD)"
BASE_SHA="$(git rev-parse "$BASE_REF")"
TAXONOMY_VERSION="$(python3 scripts/evidence.py taxonomy-version 2>/dev/null | tr -d '\n' || echo unknown)"

{
  echo "# compact-date residual probe"
  echo "# binary\t$BIN_VERSION"
  echo "# head_sha\t$HEAD_SHA"
  echo "# base_ref\t$BASE_REF\t$BASE_SHA"
  echo "# taxonomy_version\t$TAXONOMY_VERSION"
  echo "# baseline = day-first validator ^\\d{8}\$ ; candidate = day+month windows"
  echo "# locale is NOISE on this pipeline — do not read a locale difference as an effect"
  printf 'fixture\tside\tcolumn\ttype\tconfidence\tquality_band\trunner_up\tbroad_type\tformat_string\ttransform\tis_generic\tsamples_used\tnon_null\tnull\tdisambiguation\tlocale\tvalidation_pass_rate\tvalidation_advisory_low\n'
  for f in "${FIXTURES[@]}"; do
    for side in baseline candidate; do
      # Drop the CSV header row, strip the quoting `-o csv` applies, re-emit
      # tab-separated with the fixture and side prefixed, and append the two
      # validation fields the CSV writer omits, joined from the JSON record.
      python3 -c '
import csv, json, sys

fixture, side, csv_path, json_path = sys.argv[1:5]
doc = json.load(open(json_path))
by_col = {c["column"]: c for c in doc.get("columns", [])}

w = csv.writer(sys.stdout, delimiter="\t", lineterminator="\n")
with open(csv_path, newline="") as fh:
    reader = csv.reader(fh)
    next(reader, None)  # header
    for row in reader:
        if not row:
            continue
        rec = by_col.get(row[0], {})
        w.writerow([
            fixture, side, *row,
            rec.get("validation_pass_rate", ""),
            rec.get("validation_advisory_low", False),
        ])
' "$f" "$side" "$WD/$side/$f.csv" "$WD/$side/$f.json"
    done
  done
} > "$OUT"

echo "wrote $OUT"
column -t -s $'\t' "$OUT" | sed -n '1,40p'
