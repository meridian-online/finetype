#!/usr/bin/env bash
# The compact-date residual probe: what the day-first leaf emits, per column
# family, on THREE sides of the compact-date validator history.
#
# Why this exists. The corpus-honest gate certified the year-first tightening
# and could not see what that change did to the day-first sibling: its sample is
# ~3% of GitTables, non-adversarial, and it scores label transitions in
# aggregate. The defect here is not a label count — it is ONE column family
# moving from a low-confidence integer to a HIGH-confidence date with a
# `strptime` transform attached. Six hand-built column families, profiled end to
# end through the CLI, find it in seconds.
#
# FOUR SIDES, because the claim under test was that the year-first tightening
# RELOCATED a defect onto the day-first leaf, and a two-sided probe cannot tell
# a relocation from a defect that was already there:
#
#   shipped_release an EXTERNAL released binary (default: `finetype` on PATH),
#                   run from a scratch directory so no checkout `labels/` can
#                   shadow its embedded taxonomy. This is the only side that
#                   shows what a user actually got. Skipped, with the reason
#                   recorded in the output, when no external binary is found.
#   pre_tightening  the parent of the year-first tightening — both compact
#                   leaves shape-only `^\d{8}$`
#   main            the year-first leaf range-carrying, the day-first leaf
#                   still shape-only
#   candidate       both leaves range-carrying, day-first on the veto-safe
#                   allowlist
#
# WHAT THE FOURTH SIDE SETTLED. The relocation story is FALSE. The shipped
# release and `pre_tightening` agree record for record: three of the six column
# families already emitted `datetime.date.compact_dmy` at high confidence with a
# `%d%m%Y` strptime attached BEFORE the year-first tightening existed. That
# change moved the confidence a little and the label not at all. The defect is
# real, and it is older than the fix it was blamed on — which is why the three
# in-repo sides alone were not enough to establish it, and why this side is
# worth the awkwardness of shelling out to a binary the repo does not build.
#
# The probe reads the FULL emitted record (`profile -o csv`: label, confidence,
# quality band, runner-up, broad type, format string, transform, is_generic,
# disambiguation rule, locale), because a label-only comparison cannot tell a
# fixed column from one that kept its date transform.
#
# LOCALE IS NOISE on this pipeline. The probe therefore ships its own repeat
# control: the same fixture is profiled `REPEATS` times by the SAME binary on
# the candidate side and the distinct locales observed are written to
# `<out>.locale-control`. Read no locale difference between sides as an effect
# of anything unless it exceeds the spread that control shows.
#
# ═══ THE ISOLATION THIS SCRIPT EXISTS TO GET RIGHT ═══════════════════════════
#
# The change spans two files that reach the binary by DIFFERENT routes:
#
#   labels/definitions_datetime.yaml  read from `./labels` AT RUNTIME —
#       `profile` hard-codes `PathBuf::from("labels")` (crates/finetype-cli/
#       src/profile.rs) and `load_taxonomy` prefers that directory over the
#       embedded copy whenever it exists, which in a checkout it always does.
#   labels/veto_safe.txt              `include_str!` at COMPILE time
#       (crates/finetype-core/src/validation_veto.rs, via the
#       crates/finetype-core/data/veto_safe.txt symlink).
#
# So a side is only honest when its label files are on disk BOTH while it is
# built AND while it runs. An earlier revision of this script built each side's
# binary, restored the candidate YAML, and only then ran all sides — which ran
# every baseline binary against the CANDIDATE taxonomy and silently reported
# the two shape-only sides as already fixed. Each side here is built and run
# inside its own label state, and the on-disk blob sha of both files is
# recorded per side so the output names exactly what produced it.
#
# ALWAYS restores the working-tree label files on exit.
#
# Usage:
#   probe_compact_date_residual.sh [out-tsv] [pre-ref] [main-ref] [repeats] [released-bin]
set -eo pipefail
cd "$(cd "$(dirname "$0")/.." && pwd)"

OUT="${1:-docs/compact-date-residual.tsv}"
PRE_REF="${2:-464c18f^}"
MAIN_REF="${3:-origin/main}"
REPEATS="${4:-5}"
RELEASED_BIN="${5:-$(command -v finetype || true)}"
YAML=labels/definitions_datetime.yaml
SAFE=labels/veto_safe.txt
BIN=./target/release/finetype

# One fixture per row of the residual table. Each is a SINGLE column: the model
# is sibling-aware, so bundling the families into one table would let each one's
# verdict depend on its neighbours and the probe would stop being about the
# family under test.
FIXTURES=(
  compact_dmy_ymd_reject_set
  compact_dmy_sequential_ids
  compact_dmy_round_hundred_share_counts
  compact_dmy_unconstrained_eight_digit
  compact_dmy_genuine_ymd_dates
  compact_dmy_genuine_day_first_dates
)

for f in "${FIXTURES[@]}"; do
  [ -f "tests/fixtures/$f.csv" ] || { echo "FAIL: missing tests/fixtures/$f.csv"; exit 1; }
done

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

# One invocation per fixture per format. `profile --files` (which would load the
# model once) is wired only for -o json-schema / -o datapackage, and neither
# carries the confidence, quality band, disambiguation rule or validation pass
# rate this probe exists to compare — so it pays the model load per file.
#
# `-o csv` is the mandated full-record view. `-o json` is taken alongside it for
# `validation_pass_rate` / `validation_advisory_low` / `validation_vetoed`,
# which the CSV writer does not emit and which are the difference between "the
# validator rejected this column" and "the validator rejected it and nothing
# acted on the rejection".
run_side() {  # run_side <tag>
  local tag="$1" f
  mkdir -p "$WD/$tag"
  for f in "${FIXTURES[@]}"; do
    "$BIN" profile -f "tests/fixtures/$f.csv" -o csv  > "$WD/$tag/$f.csv"  2>>"$WD/$tag.log"
    "$BIN" profile -f "tests/fixtures/$f.csv" -o json > "$WD/$tag/$f.json" 2>>"$WD/$tag.log"
  done
}

: > "$WD/side_provenance.tsv"
SIDES=(pre_tightening main candidate)

# ── the shipped release, first, and from OUTSIDE the checkout ────────────────
# `profile` resolves its taxonomy as `PathBuf::from("labels")` relative to the
# CURRENT WORKING DIRECTORY and prefers that directory over its embedded copy
# whenever it exists. Running a released binary from the repo root would
# therefore profile with the working tree's taxonomy while reporting itself as
# the release — the same contamination this script was rewritten to remove, one
# level up. So the fixtures are copied to a scratch directory with no `labels/`
# in it and the binary is run there.
if [ -n "$RELEASED_BIN" ] && [ -x "$RELEASED_BIN" ] \
   && [ "$(cd "$(dirname "$RELEASED_BIN")" && pwd)" != "$(cd "$(dirname "$BIN")" && pwd)" ]; then
  echo "== side shipped_release ($RELEASED_BIN) : profile from a scratch dir =="
  mkdir -p "$WD/shipped_release" "$WD/scratch"
  cp tests/fixtures/compact_dmy_*.csv "$WD/scratch/"
  [ -d "$WD/scratch/labels" ] && { echo "FAIL: scratch dir has a labels/ shadow"; exit 1; }
  for f in "${FIXTURES[@]}"; do
    ( cd "$WD/scratch" \
      && "$RELEASED_BIN" profile -f "$f.csv" -o csv  > "$WD/shipped_release/$f.csv"  2>>"$WD/shipped_release.log" \
      && "$RELEASED_BIN" profile -f "$f.csv" -o json > "$WD/shipped_release/$f.json" 2>>"$WD/shipped_release.log" )
  done
  printf '%s\t%s\t%s\t%s\t%s\n' shipped_release "$RELEASED_BIN" \
    "$("$RELEASED_BIN" --version 2>/dev/null | tr -d '\n')" \
    "(embedded)" "(embedded)" >> "$WD/side_provenance.tsv"
  SIDES=(shipped_release "${SIDES[@]}")
else
  printf '%s\t%s\t%s\t%s\t%s\n' shipped_release \
    "SKIPPED: no external released binary (looked for \`finetype\` on PATH)" \
    - - - >> "$WD/side_provenance.tsv"
fi

for spec in "pre_tightening:$PRE_REF" "main:$MAIN_REF" "candidate:"; do
  side="${spec%%:*}"; ref="${spec#*:}"
  echo "== side $side (${ref:-working tree}) : place label files =="
  if [ -n "$ref" ]; then
    git checkout "$ref" -- "$YAML" "$SAFE"
  else
    restore
  fi
  grep -q "datetime.date.compact_dmy" "$YAML"
  printf '%s\t%s\t%s\t%s\t%s\n' "$side" "${ref:-working-tree}" \
    "$(git rev-parse "${ref:-HEAD}")" \
    "$(git hash-object "$YAML")" "$(git hash-object "$SAFE")" \
    >> "$WD/side_provenance.tsv"
  echo "== side $side : build (veto_safe.txt is compiled in) =="
  cargo build --release -p finetype-cli
  echo "== side $side : profile (definitions_datetime.yaml is read from ./labels) =="
  run_side "$side"
done

echo "== restore the candidate label files and rebuild the candidate binary =="
restore
cargo build --release -p finetype-cli

# Locale repeat control: same binary, same fixture, REPEATS times.
echo "== locale repeat control ($REPEATS repeats, candidate binary) =="
: > "$WD/locale_control.txt"
for _ in $(seq "$REPEATS"); do
  "$BIN" profile -f tests/fixtures/compact_dmy_genuine_ymd_dates.csv -o csv \
    | python3 -c 'import csv,sys; r=list(csv.reader(sys.stdin)); print(dict(zip(r[0],r[1])).get("locale",""))' \
    >> "$WD/locale_control.txt"
done

BIN_VERSION="$("$BIN" --version 2>/dev/null | tr -d '\n')"
TAXONOMY_VERSION="$(python3 scripts/evidence.py taxonomy-version 2>/dev/null | tr -d '\n' || echo unknown)"

{
  printf '# compact-date residual probe\n'
  printf '# binary\t%s\n' "$BIN_VERSION"
  printf '# head_sha\t%s\n' "$(git rev-parse HEAD)"
  printf '# taxonomy_version\t%s\n' "$TAXONOMY_VERSION"
  printf '# side\tref\tcommit-or-version\tdefinitions_datetime.yaml_blob\tveto_safe.txt_blob\n'
  sed 's/^/# /' "$WD/side_provenance.tsv"
  printf '# shipped_release = the released binary, embedded taxonomy, run outside the checkout\n'
  printf '# pre_tightening = both compact leaves ^\\d{8}$\n'
  printf '# main           = year-first range-carrying, day-first still ^\\d{8}$\n'
  printf '# candidate      = day-first range-carrying AND veto-safe\n'
  printf '# locale is NOISE on this pipeline — see %s.locale-control before reading any locale difference\n' "$OUT"
  printf 'fixture\tside\tcolumn\ttype\tconfidence\tquality_band\trunner_up\tbroad_type\tformat_string\ttransform\tis_generic\tsamples_used\tnon_null\tnull\tdisambiguation\tlocale\tvalidation_pass_rate\tvalidation_advisory_low\tvalidation_vetoed\tvetoed_type\n'
  for f in "${FIXTURES[@]}"; do
    for side in "${SIDES[@]}"; do
      # Drop the CSV header row, re-emit tab-separated with the fixture and side
      # prefixed, and append the validation fields the CSV writer omits, joined
      # from the JSON record for the same column.
      python3 -c '
import csv, json, sys

fixture, side, csv_path, json_path = sys.argv[1:5]
with open(json_path) as fh:
    doc = json.load(fh)
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
            rec.get("validation_vetoed", False),
            rec.get("vetoed_type", ""),
        ])
' "$f" "$side" "$WD/$side/$f.csv" "$WD/$side/$f.json"
    done
  done
} > "$OUT"

{
  printf '# locale repeat control — %s repeats of ONE fixture through ONE binary\n' "$REPEATS"
  printf '# fixture\ttests/fixtures/compact_dmy_genuine_ymd_dates.csv\n'
  printf '# distinct locales observed: %s\n' "$(sort -u "$WD/locale_control.txt" | tr '\n' ' ')"
  cat "$WD/locale_control.txt"
} > "$OUT.locale-control"

echo "wrote $OUT and $OUT.locale-control"
column -t -s $'\t' "$OUT"
