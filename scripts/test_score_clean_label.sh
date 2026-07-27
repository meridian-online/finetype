#!/usr/bin/env bash
# test_score_clean_label.sh — the scorer must not swallow a failure, and must never
# print a score without naming the fixture version it was measured on.
#
# Runs scripts/score_clean_label.sh against stub stand-ins for predict_multibranch,
# compose_predictions.py and score_gold_anchor.py, so a full pass costs milliseconds
# instead of the four minutes the real pipeline takes. Every stub failure mode here
# is one the real pipeline has produced.
#
# No arguments. Exit 0 = all cases pass.
set -eo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

TMP="$(mktemp -d "${TMPDIR:-/tmp}/score-clean-label-test.XXXXXX")"
trap 'rm -rf "$TMP"' EXIT

PASS=0
FAIL=0
CASE=""

ok()   { PASS=$((PASS + 1)); printf '  ok   %s\n' "$1"; }
bad()  { FAIL=$((FAIL + 1)); printf '  FAIL %s\n     %s\n' "$CASE" "$1"; }

# ---------------------------------------------------------------- stubs ----
mkdir -p "$TMP/bin" "$TMP/model" "$TMP/out"
: > "$TMP/gold.ftmb"

cat > "$TMP/bin/predict_multibranch" <<'STUB'
#!/usr/bin/env bash
set -eo pipefail
out=""
while [ $# -gt 0 ]; do
  case "$1" in --out) out="$2"; shift 2;; *) shift;; esac
done
if [ "$STUB_PM_FAIL" = 1 ]; then echo "stub predict_multibranch: CUDA is on fire" >&2; exit 9; fi
{
  printf 'key\tpredicted_label\tconfidence\n'
  i=0
  while [ "$i" -lt "${STUB_ROWS:-8}" ]; do
    printf 'sha%02d\037col%02d\tlabel%02d\t0.9\n' "$i" "$i" "$i"
    i=$((i + 1))
  done
} > "$out"
STUB

cat > "$TMP/bin/python" <<'STUB'
#!/usr/bin/env bash
set -eo pipefail
script="$1"; shift
case "$script" in
  *compose_predictions.py)
    preds=""; out=""
    while [ $# -gt 0 ]; do
      case "$1" in --preds) preds="$2"; shift 2;; --out) out="$2"; shift 2;; *) shift;; esac
    done
    if [ "$STUB_COMPOSE_FAIL" = 1 ]; then
      echo "stub compose: finetype profile exited 101 on column col03" >&2
      exit 3
    fi
    if [ "$STUB_COMPOSE_DROP" = 1 ]; then sed '$d' "$preds" > "$out"; else cp "$preds" "$out"; fi
    ;;
  *score_gold_anchor.py)
    preds=""; outdir=""; name=""
    while [ $# -gt 0 ]; do
      case "$1" in
        --predictions) preds="$2"; shift 2;;
        --out-dir) outdir="$2"; shift 2;;
        --model-name) name="$2"; shift 2;;
        *) shift;;
      esac
    done
    if [ "$STUB_SCORE_FAIL" = 1 ]; then
      echo "stub score_gold_anchor: KeyError 'curated_label'" >&2
      exit 4
    fi
    mkdir -p "$outdir"
    n=$(( $(wc -l < "$preds") - 1 ))
    case "$name" in
      *-sense)    miss="${STUB_MISS_SENSE:-4}";;
      *-composed) miss="${STUB_MISS_COMPOSED:-1}";;
      *)          miss=0;;
    esac
    correct=$((n - miss))
    if [ "$STUB_SCORE_NO_HEADLINE" = 1 ]; then
      printf '# report\n\nnothing useful here\n' > "$outdir/report_${name}_stub.md"
    else
      acc=$(awk -v c="$correct" -v n="$n" 'BEGIN{printf "%.3f", c/n}')
      printf '# report\n\n**Headline — column accuracy:** %d/%d = %s (95%% CI 0.800-0.900)  \n' \
        "$correct" "$n" "$acc" > "$outdir/report_${name}_stub.md"
    fi
    ;;
  *) echo "stub python: unexpected script $script" >&2; exit 127;;
esac
STUB

chmod +x "$TMP/bin/predict_multibranch" "$TMP/bin/python"

# ------------------------------------------------------------- fixtures ----
GOLD="$TMP/gold_corpus.tsv"
printf 'family\tfile_content_sha256\tcolumn_name\tcurated_label\n' > "$GOLD"
i=0
while [ "$i" -lt 8 ]; do
  printf 'fam\tsha%02d\tcol%02d\tlabel%02d\n' "$i" "$i" "$i" >> "$GOLD"
  i=$((i + 1))
done

MANIFEST="$TMP/fixtures.json"
scripts/evidence.py --manifest "$MANIFEST" register-fixture \
  --id test-gold-v1 --path "$GOLD" --note "harness fixture" >/dev/null
scripts/evidence.py --manifest "$MANIFEST" record-baseline \
  --fixture test-gold-v1 --key "stub/composed/v1" --correct 6 --scored 8 \
  --model stub --binary v1 --source "harness" >/dev/null
scripts/evidence.py --manifest "$MANIFEST" record-baseline \
  --fixture test-gold-v1 --key "stub/wrong-denominator/v1" --correct 900 --scored 931 \
  --model stub --binary v1 --source "harness" >/dev/null

UNREGISTERED="$TMP/gold_unregistered.tsv"
cp "$GOLD" "$UNREGISTERED"
printf 'fam\tsha99\tcol99\tlabel99\n' >> "$UNREGISTERED"

# ---------------------------------------------------------------- driver ----
# run <case-name> <expected-exit> [env assignments...] -- [extra script args...]
STDOUT=""
STDERR=""
RC=0
run() {
  CASE="$1"; expect="$2"; shift 2
  env_args=()
  while [ $# -gt 0 ] && [ "$1" != "--" ]; do env_args[${#env_args[@]}]="$1"; shift; done
  [ $# -eq 0 ] || shift
  rm -rf "$TMP/out"; mkdir -p "$TMP/out"
  RC=0
  env FINETYPE_PY="$TMP/bin/python" \
      FINETYPE_PREDICT_MULTIBRANCH="$TMP/bin/predict_multibranch" \
      FINETYPE_GOLD="${GOLD_OVERRIDE:-$GOLD}" \
      FINETYPE_SCORE_OUTDIR="$TMP/out" \
      FINETYPE_FIXTURE_MANIFEST="$MANIFEST" \
      ${env_args[@]+"${env_args[@]}"} \
      scripts/score_clean_label.sh "$TMP/model" "$TMP/gold.ftmb" t1 "$@" \
      >"$TMP/stdout" 2>"$TMP/stderr" || RC=$?
  STDOUT="$(cat "$TMP/stdout")"
  STDERR="$(cat "$TMP/stderr")"
  if [ "$RC" -ne "$expect" ]; then
    bad "expected exit $expect, got $RC. stderr: $(printf '%s' "$STDERR" | tail -n 5 | tr '\n' '|')"
    return 1
  fi
  return 0
}

want_out() { case "$STDOUT" in *"$1"*) ok "$CASE: stdout has '$1'";; *) bad "stdout lacks '$1'";; esac; }
no_out()   { case "$STDOUT" in *"$1"*) bad "stdout should not contain '$1'";; *) ok "$CASE: stdout free of '$1'";; esac; }
want_err() { case "$STDERR" in *"$1"*) ok "$CASE: stderr explains '$1'";; *) bad "stderr lacks '$1'; got: $(printf '%s' "$STDERR" | tail -n 3 | tr '\n' '|')";; esac; }

echo "== the committed manifest is self-consistent =="
if scripts/evidence.py verify >/dev/null; then ok "evidence.py verify"; else bad "evidence.py verify failed"; fi

echo "== happy path: every score names its fixture version =="
if run "happy path" 0 -- ; then
  want_out "fixture=test-gold-v1"
  want_out "sense(reframe)    4/8 = 0.500   fixture=test-gold-v1"
  want_out "composed(reframe) 7/8 = 0.875   fixture=test-gold-v1"
  want_out "TAG=t1 fixture=test-gold-v1 sense=4/8=0.500 composed=7/8=0.875"
  if [ -f "$TMP/out/t1/t1-headline.json" ]; then
    ok "happy path: headline record written"
    if grep -q '"id": "test-gold-v1"' "$TMP/out/t1/t1-headline.json"; then
      ok "happy path: headline record carries the fixture id"
    else bad "headline record has no fixture id"; fi
  else bad "no headline record written"; fi
  # Not one printed score may appear without a fixture id beside it.
  offenders=$(printf '%s\n' "$STDOUT" | grep -E '[0-9]+/[0-9]+ = [0-9]' | grep -cv 'fixture=' || true)
  if [ "$offenders" -eq 0 ]; then ok "happy path: no unstamped score lines"; else bad "$offenders score lines printed without a fixture id"; fi
fi

echo "== a failing composer must not fall back to the uncomposed predictions =="
if run "compose fails" 1 STUB_COMPOSE_FAIL=1 -- ; then
  want_err "compose_predictions.py exited 3"
  want_err "finetype profile exited 101"
  no_out "composed(reframe)"
  no_out "TAG=t1"
fi

echo "== a composer that silently drops rows is caught by the row count =="
if run "compose drops a row" 5 STUB_COMPOSE_DROP=1 -- ; then
  want_err "compose dropped rows: 8 in, 7 out"
  no_out "composed(reframe)"
fi

echo "== a failing scorer must not be reported as a score =="
if run "scorer fails" 1 STUB_SCORE_FAIL=1 -- ; then
  want_err "score_gold_anchor.py (sense) exited 4"
  want_err "KeyError"
  no_out "sense(reframe)"
fi

echo "== a scorer that writes no headline is a failure, not an ERR string =="
if run "no headline" 6 STUB_SCORE_NO_HEADLINE=1 -- ; then
  want_err "no headline in any report"
  no_out "ERR"
fi

echo "== a failing predictor stops the run =="
if run "predict fails" 1 STUB_PM_FAIL=1 -- ; then
  want_err "predict_multibranch exited 9"
  want_err "CUDA is on fire"
  no_out "sense(reframe)"
fi

echo "== an unregistered gold corpus is refused =="
GOLD_OVERRIDE="$UNREGISTERED"
if run "unregistered fixture" 3 -- ; then
  want_err "is not a registered fixture version"
  want_err "register-fixture"
  no_out "sense(reframe)"
fi
GOLD_OVERRIDE=""

echo "== a recorded bar is quoted with its fixture version =="
if run "baseline present" 0 -- --baseline "stub/composed/v1"; then
  want_out "baseline stub/composed/v1   6/8 = 0.750   fixture=test-gold-v1   model=stub binary=v1"
  want_out "delta composed - baseline = +0.125 (+1 columns) -> at-or-above bar   fixture=test-gold-v1"
  if grep -q '"key": "stub/composed/v1"' "$TMP/out/t1/t1-headline.json"
  then ok "$CASE: headline record carries the bar it was compared against"
  else bad "headline record has no baseline block"; fi
fi

echo "== a bar that was never measured on this fixture is refused, not applied =="
if run "baseline missing" 4 -- --baseline "stub/composed/v9"; then
  want_err "has no recorded score on fixture 'test-gold-v1'"
  no_out "delta composed"
fi

echo "== a bar over a different column count is refused =="
if run "denominator mismatch" 5 -- --baseline "stub/wrong-denominator/v1"; then
  want_err "the bar was measured over 931 columns, this run scored 8"
fi

echo "== --fail-below-baseline turns a below-bar result into a non-zero exit =="
if run "below bar" 7 STUB_MISS_COMPOSED=3 -- --baseline "stub/composed/v1" --fail-below-baseline; then
  want_err "is below the bar stub/composed/v1"
  want_out "delta composed - baseline = -0.125 (-1 columns) -> below bar"
fi
if run "below bar, no gate" 0 STUB_MISS_COMPOSED=3 -- --baseline "stub/composed/v1"; then
  want_out "-> below bar"
fi

echo "== manifest guards =="
CASE="manifest guards"
if scripts/evidence.py --manifest "$MANIFEST" register-fixture --id test-gold-v1 --path "$GOLD" >/dev/null 2>&1
then bad "re-registering an existing id should fail"; else ok "$CASE: duplicate fixture id refused"; fi
if scripts/evidence.py --manifest "$MANIFEST" register-fixture --id other --path "$GOLD" >/dev/null 2>&1
then bad "re-registering the same content under a new id should fail"; else ok "$CASE: duplicate content refused"; fi
if scripts/evidence.py --manifest "$MANIFEST" record-baseline --fixture test-gold-v1 \
     --key "stub/composed/v1" --correct 1 --scored 8 --source h >/dev/null 2>&1
then bad "overwriting a recorded baseline should fail"; else ok "$CASE: recorded baseline is immutable"; fi
sed 's/"score": 0.875/"score": 0.999/' "$MANIFEST" > "$TMP/tampered.json"
if scripts/evidence.py --manifest "$TMP/tampered.json" verify >/dev/null 2>&1
then bad "verify should reject a score that is not correct/scored"; else ok "$CASE: verify catches a doctored score"; fi
printf 'fam\tsha77\tcol77\tlabel77\n' >> "$GOLD"
if scripts/evidence.py --manifest "$MANIFEST" verify >/dev/null 2>&1
then bad "verify should reject an edited-but-unregistered fixture file"; else ok "$CASE: verify catches an unregistered edit"; fi

echo
echo "passed $PASS, failed $FAIL"
[ "$FAIL" -eq 0 ]
