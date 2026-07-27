#!/usr/bin/env bash
# score_clean_label.sh — composed-gold (reframe) scorer for the clean-label retrain.
#
#   predict_multibranch -> reshape -> compose_predictions (REAL Sharpen)
#                       -> score_gold_anchor.py score --reframe
#
# Two rules this script exists to hold:
#
#   1. Every stage's exit code is checked. A stage that fails takes the whole script
#      down with a readable message and a non-zero exit. Nothing falls back to a
#      substitute artefact, because a broken run that scores "no change" is worse
#      than a run that stops.
#   2. Every score is printed with the id of the gold fixture *version* it was
#      measured on. Bars are not constants in this file: they live in
#      evidence/fixtures.json, keyed by fixture version, and you opt into one with
#      --baseline KEY. A bar recorded on a different fixture version is refused
#      rather than silently applied — ground truth moves, and a stale float
#      rejects good models for the wrong reason.
#
# Usage:
#   scripts/score_clean_label.sh <model_dir> <gold_ftmb> <tag> [options] [predict args...]
#
#     --baseline KEY           compare against this recorded bar on the resolved fixture
#     --fail-below-baseline    exit non-zero when the composed score is under the bar
#     --evidence-dir DIR       also write the headline record here (defaults to the score dir)
#
#   Unrecognised arguments pass straight through to predict_multibranch
#   (e.g. --value-encoder DIR, --zero-char).
#
# Environment overrides (all optional; defaults are the in-repo paths):
#   FINETYPE_PY, FINETYPE_PREDICT_MULTIBRANCH, FINETYPE_GOLD,
#   FINETYPE_SCORE_OUTDIR, FINETYPE_FIXTURE_MANIFEST
#
# Exit codes:
#   0 ok · 1 a stage failed · 2 usage · 3 gold fixture not registered ·
#   4 baseline not recorded on this fixture · 5 row/denominator mismatch ·
#   6 headline unreadable · 7 composed score below the requested bar
set -eo pipefail
cd "$(cd "$(dirname "$0")/.." && pwd)"
export HF_HUB_OFFLINE=1 TRANSFORMERS_OFFLINE=1 HF_HUB_DISABLE_TELEMETRY=1 PYTHONUNBUFFERED=1

PY="${FINETYPE_PY:-eval/gittables/.venv/bin/python}"
PM="${FINETYPE_PREDICT_MULTIBRANCH:-./target/release/predict_multibranch}"
GOLD="${FINETYPE_GOLD:-eval/gold/gold_corpus.tsv}"
OUTDIR="${FINETYPE_SCORE_OUTDIR:-output/clean-label-retrain/scores}"
MANIFEST="${FINETYPE_FIXTURE_MANIFEST:-evidence/fixtures.json}"
EVIDENCE="scripts/evidence.py"

die() {
  code="$1"; shift
  { echo "FAIL: $1"; shift; for l in "$@"; do echo "      $l"; done; } >&2
  exit "$code"
}
note() { echo "$*" >&2; }
usage() { sed -n '2,36p' "$0" >&2; exit 2; }

# ---- arguments -------------------------------------------------------------
[ $# -ge 3 ] || usage
case "$1" in -h|--help) usage;; esac
MODEL="$1"; GOLDFTMB="$2"; TAG="$3"; shift 3
BASELINE=""
FAIL_BELOW=0
EVIDENCE_DIR=""
PM_ARGS=()
while [ $# -gt 0 ]; do
  case "$1" in
    --baseline)   [ $# -ge 2 ] || die 2 "--baseline needs a KEY"; BASELINE="$2"; shift 2;;
    --baseline=*) BASELINE="${1#*=}"; shift;;
    --evidence-dir)   [ $# -ge 2 ] || die 2 "--evidence-dir needs a DIR"; EVIDENCE_DIR="$2"; shift 2;;
    --evidence-dir=*) EVIDENCE_DIR="${1#*=}"; shift;;
    --fail-below-baseline) FAIL_BELOW=1; shift;;
    -h|--help) usage;;
    *) PM_ARGS[${#PM_ARGS[@]}]="$1"; shift;;
  esac
done
[ -n "$BASELINE" ] || [ "$FAIL_BELOW" -eq 0 ] || die 2 "--fail-below-baseline needs --baseline KEY"

[ -x "$PM" ] || die 2 "predict_multibranch not found or not executable at $PM" \
  "build it: cargo build --release --bin predict_multibranch -p finetype-train"
[ -x "$PY" ]       || die 2 "python not found or not executable at $PY"
[ -x "$EVIDENCE" ] || die 2 "fixture manifest tool not executable at $EVIDENCE"
[ -d "$MODEL" ]    || die 2 "model dir not found: $MODEL"
[ -f "$GOLDFTMB" ] || die 2 "gold FTMB not found: $GOLDFTMB"
[ -f "$GOLD" ]     || die 2 "gold corpus not found: $GOLD"

# ---- which ground truth is this? -------------------------------------------
# Resolved by content hash, so an edited gold corpus cannot masquerade as the one a
# recorded bar was measured on.
FIXTURE_TSV=$("$EVIDENCE" --manifest "$MANIFEST" resolve-fixture --path "$GOLD" --format tsv)
FIXTURE_ID=$(printf '%s' "$FIXTURE_TSV" | cut -f1)
FIXTURE_SHA=$(printf '%s' "$FIXTURE_TSV" | cut -f2)
FIXTURE_ROWS=$(printf '%s' "$FIXTURE_TSV" | cut -f3)
[ -n "$FIXTURE_ID" ] || die 3 "could not resolve a fixture version for $GOLD"

mkdir -p "$OUTDIR" "$OUTDIR/$TAG"
[ -n "$EVIDENCE_DIR" ] || EVIDENCE_DIR="$OUTDIR/$TAG"
LOGDIR="$OUTDIR/$TAG/stage-logs"
mkdir -p "$EVIDENCE_DIR" "$LOGDIR"

raw="$OUTDIR/${TAG}_raw.tsv"
sense="$OUTDIR/${TAG}_sense.tsv"
comp="$OUTDIR/${TAG}_composed.tsv"

echo "fixture=$FIXTURE_ID sha256=$FIXTURE_SHA rows=$FIXTURE_ROWS path=$GOLD"

# ---- stage runner: no swallowed failures -----------------------------------
run_stage() {  # <label> <logfile> <cmd...>
  label="$1"; log="$2"; shift 2
  rc=0
  "$@" >"$log" 2>&1 || rc=$?
  if [ "$rc" -ne 0 ]; then
    {
      echo "FAIL: $label exited $rc"
      echo "      command: $*"
      echo "      log:     $log"
      echo "      ---- last 40 lines ----"
      tail -n 40 "$log" 2>/dev/null || true
      echo "      -----------------------"
    } >&2
    exit 1
  fi
}

data_rows() {  # data rows in a headered TSV
  [ -f "$1" ] || die 1 "expected file was never written: $1"
  n=$(wc -l < "$1" | tr -d ' ')
  echo $((n - 1))
}

# ---- predict ---------------------------------------------------------------
note "[$TAG] predict_multibranch"
run_stage "predict_multibranch" "$LOGDIR/predict.log" \
  "$PM" --model "$MODEL" --data "$GOLDFTMB" --out "$raw" ${PM_ARGS[@]+"${PM_ARGS[@]}"}
N_RAW=$(data_rows "$raw")
[ "$N_RAW" -gt 0 ] || die 1 "predict_multibranch wrote no predictions to $raw"

# ---- reshape ---------------------------------------------------------------
note "[$TAG] reshape ($N_RAW rows)"
{ echo -e "file_content_sha256\tcolumn_name\tpredicted_label\tconfidence";
  tail -n +2 "$raw" | awk -F'\t' '{split($1,a,"\037"); print a[1]"\t"a[2]"\t"$2"\t"$3}'; } > "$sense"
N_SENSE=$(data_rows "$sense")
[ "$N_SENSE" -eq "$N_RAW" ] || die 5 \
  "reshape changed the row count: $N_RAW predictions in, $N_SENSE out" \
  "raw: $raw" "reshaped: $sense"

# ---- compose (REAL Sharpen) ------------------------------------------------
# No fallback to $sense. compose_predictions.py drops any column it cannot find
# values for, so a partial compose is a silently different measurement, not a
# smaller one — the row count below is what catches that.
note "[$TAG] compose ($N_SENSE rows, one Sharpen subprocess each — minutes)"
run_stage "compose_predictions.py" "$LOGDIR/compose.log" \
  "$PY" scripts/compose_predictions.py --preds "$sense" --out "$comp"
N_COMP=$(data_rows "$comp")
[ "$N_COMP" -eq "$N_SENSE" ] || die 5 \
  "compose dropped rows: $N_SENSE in, $N_COMP out ($((N_SENSE - N_COMP)) lost)" \
  "a partial compose is a different measurement, not a smaller one" \
  "see $LOGDIR/compose.log"

# ---- score -----------------------------------------------------------------
headline() {  # <report dir> -> "<correct> <scored> <score>"
  hl_dir="$1"
  hl_line=$(grep -m1 -h -F -- '**Headline — column accuracy:**' "$hl_dir"/report_*.md 2>/dev/null) || hl_line=""
  [ -n "$hl_line" ] || die 6 \
    "no headline in any report under $hl_dir" \
    "score_gold_anchor.py exited 0 but wrote no readable accuracy line"
  hl_parsed=$(printf '%s\n' "$hl_line" | sed -E 's#.*\*\* ([0-9]+)/([0-9]+) = ([0-9]+\.[0-9]+).*#\1 \2 \3#')
  set -- $hl_parsed
  [ $# -eq 3 ] || die 6 "could not parse the headline line: $hl_line"
  case "$1$2$3" in *[!0-9.]*) die 6 "non-numeric headline fields in: $hl_line";; esac
  echo "$1 $2 $3"
}

score_one() {  # <predictions tsv> <name> -> "<correct> <scored> <score>"
  so_preds="$1"; so_name="$2"
  rm -f "$OUTDIR/$TAG"/report_*.md
  run_stage "score_gold_anchor.py ($so_name)" "$LOGDIR/score-$so_name.log" \
    "$PY" scripts/score_gold_anchor.py score --gold "$GOLD" --predictions "$so_preds" \
    --model-name "${TAG}-${so_name}" --out-dir "$OUTDIR/$TAG" --reframe
  headline "$OUTDIR/$TAG"
}

note "[$TAG] score sense"
SENSE_HL=$(score_one "$sense" sense)
note "[$TAG] score composed"
COMP_HL=$(score_one "$comp" composed)

set -- $SENSE_HL; SENSE_CORRECT="$1"; SENSE_SCORED="$2"; SENSE_SCORE="$3"
set -- $COMP_HL;  COMP_CORRECT="$1";  COMP_SCORED="$2";  COMP_SCORE="$3"

[ "$SENSE_SCORED" -eq "$COMP_SCORED" ] || die 5 \
  "sense and composed scored different column counts ($SENSE_SCORED vs $COMP_SCORED)" \
  "they must score the same set for the pair to be comparable"

echo "sense(reframe)    $SENSE_CORRECT/$SENSE_SCORED = $SENSE_SCORE   fixture=$FIXTURE_ID"
echo "composed(reframe) $COMP_CORRECT/$COMP_SCORED = $COMP_SCORE   fixture=$FIXTURE_ID"

# ---- baseline (opt-in, fixture-scoped) -------------------------------------
BASE_SCORE=""; BASE_CORRECT=""; BASE_SCORED=""; BASE_MODEL=""; BASE_BINARY=""
DELTA=""; DELTA_COLS=""
if [ -n "$BASELINE" ]; then
  BASE_TSV=$("$EVIDENCE" --manifest "$MANIFEST" get-baseline --fixture "$FIXTURE_ID" --key "$BASELINE" --format tsv)
  BASE_SCORE=$(printf '%s' "$BASE_TSV" | cut -f1)
  BASE_CORRECT=$(printf '%s' "$BASE_TSV" | cut -f2)
  BASE_SCORED=$(printf '%s' "$BASE_TSV" | cut -f3)
  BASE_MODEL=$(printf '%s' "$BASE_TSV" | cut -f4)
  BASE_BINARY=$(printf '%s' "$BASE_TSV" | cut -f5)
  [ "$BASE_SCORED" -eq "$COMP_SCORED" ] || die 5 \
    "the bar was measured over $BASE_SCORED columns, this run scored $COMP_SCORED" \
    "two accuracies over different column sets are not comparable" \
    "baseline=$BASELINE fixture=$FIXTURE_ID"
  DELTA=$(awk -v a="$COMP_SCORE" -v b="$BASE_SCORE" 'BEGIN{printf "%+.3f", a-b}')
  DELTA_COLS=$((COMP_CORRECT - BASE_CORRECT))
  echo "baseline $BASELINE   $BASE_CORRECT/$BASE_SCORED = $BASE_SCORE   fixture=$FIXTURE_ID   model=$BASE_MODEL binary=$BASE_BINARY"
  if [ "$DELTA_COLS" -ge 0 ]; then VERDICT="at-or-above"; else VERDICT="below"; fi
  printf 'delta composed - baseline = %s (%+d columns) -> %s bar   fixture=%s\n' \
    "$DELTA" "$DELTA_COLS" "$VERDICT" "$FIXTURE_ID"
fi

# ---- headline record (kilobytes; the shape evidence/ keeps) ----------------
HEADLINE_JSON="$EVIDENCE_DIR/${TAG}-headline.json"
set -- write-headline --out "$HEADLINE_JSON" \
  --set "tag=$TAG" --set "model=$MODEL" --set "gold_ftmb=$GOLDFTMB" \
  --set "fixture.id=$FIXTURE_ID" --set "fixture.sha256=$FIXTURE_SHA" \
  --set "fixture.rows=$FIXTURE_ROWS" --set "fixture.path=$GOLD" \
  --set "sense.correct=$SENSE_CORRECT" --set "sense.scored=$SENSE_SCORED" --set "sense.score=$SENSE_SCORE" \
  --set "composed.correct=$COMP_CORRECT" --set "composed.scored=$COMP_SCORED" --set "composed.score=$COMP_SCORE" \
  --set "measured=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
if [ -n "$BASELINE" ]; then
  set -- "$@" --set "baseline.key=$BASELINE" --set "baseline.correct=$BASE_CORRECT" \
    --set "baseline.scored=$BASE_SCORED" --set "baseline.score=$BASE_SCORE" \
    --set "baseline.delta_score=$DELTA" --set "baseline.delta_columns=$DELTA_COLS"
fi
"$EVIDENCE" --manifest "$MANIFEST" "$@" || die 1 "could not write the headline record to $HEADLINE_JSON"
note "[$TAG] headline record: $HEADLINE_JSON"

echo "TAG=$TAG fixture=$FIXTURE_ID sense=$SENSE_CORRECT/$SENSE_SCORED=$SENSE_SCORE composed=$COMP_CORRECT/$COMP_SCORED=$COMP_SCORE"

if [ -n "$BASELINE" ] && [ "$FAIL_BELOW" -eq 1 ] && [ "$DELTA_COLS" -lt 0 ]; then
  die 7 "composed $COMP_CORRECT/$COMP_SCORED = $COMP_SCORE is below the bar $BASELINE ($BASE_CORRECT/$BASE_SCORED = $BASE_SCORE) on fixture $FIXTURE_ID"
fi
