#!/usr/bin/env bash
# scripts/overnight_username_veto_gate.sh — ISOLATED corpus-honest gate for the
# full_name->username value veto (spec 2026-06-17-full-name-username-veto ac-03).
#
# WHY ISOLATED: the gate's default baseline is the stale May-2026 v19_gated parquet,
# which scores ~7 months of shipped fixes as the candidate's doing (reframe ac-03
# lesson). So we generate BOTH sides with ONE binary and toggle only the veto via the
# kill switch — the ONLY delta is full_name->username.
#
# REQUIRES an rhh-instrumentation build (the kill switch is a no-op on default release
# builds): cargo build --release -p finetype-cli --features finetype-model/rhh-instrumentation
# then copy it to BIN below. Verified before launch: veto OFF -> full_name, ON -> username.
#
# The candidate/baseline passes write only sense_prediction; the gate pulls its oracle
# from the stable output/ydf-validation-gate parquet, so --fill-ydf is not needed.
#
# NOTE ON INTERPRETATION (do this awake, not here): the veto relocates a SUBSET of
# full_name (handle-shaped) to username; the oracle (gated YDF) emits full_name on
# those. A collapse/oracle_fp trigger on full_name may be a referee-vocabulary false
# alarm (the candidate is more correct than the oracle on handles) — the SAME shape as
# categorical->word, but a blanket --label-remap is WRONG here because real-name
# full_name must still be judged. So treat a NO-GO as "needs subset-aware review",
# cross-check the relocated columns against gold, NOT as an automatic fail.

set -uo pipefail
cd "$(dirname "$0")/.."

BIN="/tmp/finetype-instr"            # rhh-instrumentation release build
MODEL="sherlock-v19-relu-s42"       # shipped default (veto is a Sharpen change)
SAMPLE="output/corpus-honest-gate/stratified_sample.files.txt"
OUT="output/username-veto-gate"
KILL="full_name_username_veto"
VPY="eval/gittables/.venv/bin/python"
LOG="$OUT/run.log"

mkdir -p "$OUT"
exec > >(tee -a "$LOG") 2>&1
echo "================================================================"
echo " username-veto isolated gate — start $(date)"
echo " bin=$BIN model=$MODEL sample=$(wc -l < "$SAMPLE") files"
echo "================================================================"

# Pre-flight: the kill switch must actually toggle on this binary, or the two
# passes would be identical and the gate meaningless.
echo "── Pre-flight: kill-switch toggles ──"
printf 'Author,n\ntptacek,1\npatio11,2\nrms,3\njacquesm,4\ndfens,5\nschof,6\n' > "$OUT/_kw.csv"
ON=$("$BIN" profile -f "$OUT/_kw.csv" 2>/dev/null | grep -i author | grep -o 'identity.person.[a-z_]*' | head -1)
OFF=$(RHH_DISABLE_HINTS="$KILL" "$BIN" profile -f "$OUT/_kw.csv" 2>/dev/null | grep -i author | grep -o 'identity.person.[a-z_]*' | head -1)
echo "  veto ON  -> $ON"
echo "  veto OFF -> $OFF"
if [[ "$ON" != "identity.person.username" || "$OFF" != "identity.person.full_name" ]]; then
    echo "FAIL: kill switch does not toggle as expected — aborting (is BIN an rhh-instrumentation build?)"
    exit 1
fi
rm -f "$OUT/_kw.csv"

# shellcheck disable=SC1091
source eval/gittables/.venv/bin/activate   # pyarrow/duckdb live here

# ── Baseline pass (veto OFF) ─────────────────────────────────────────────────
echo ""; echo "── Baseline corpus pass (veto OFF) — $(date) ──"
rm -rf "$OUT/baseline"
RHH_DISABLE_HINTS="$KILL" FINETYPE_MODEL="$MODEL" python3 scripts/gittables_corpus_pass.py \
    --corpus-index "$SAMPLE" --execute --jobs 8 \
    --finetype-bin "$BIN" --out-dir "$OUT/baseline" || echo "  baseline pass FAILED"

# ── Candidate pass (veto ON) ─────────────────────────────────────────────────
echo ""; echo "── Candidate corpus pass (veto ON) — $(date) ──"
rm -rf "$OUT/candidate"
FINETYPE_MODEL="$MODEL" python3 scripts/gittables_corpus_pass.py \
    --corpus-index "$SAMPLE" --execute --jobs 8 \
    --finetype-bin "$BIN" --out-dir "$OUT/candidate" || echo "  candidate pass FAILED"

BASE_PARQUET="$OUT/baseline/corpus_pass/columns.parquet"
CAND_PARQUET="$OUT/candidate/corpus_pass/columns.parquet"

# ── Relocation volume (sense_prediction delta) ───────────────────────────────
echo ""; echo "── Relocation summary (candidate vs baseline) — $(date) ──"
duckdb -csv -c "
SELECT b.sense_prediction AS baseline, c.sense_prediction AS candidate, count(*) n
FROM '$BASE_PARQUET' b JOIN '$CAND_PARQUET' c
  USING (file_content_sha256, column_name)
WHERE b.sense_prediction <> c.sense_prediction
GROUP BY 1,2 ORDER BY n DESC LIMIT 25;" 2>/dev/null || echo "  (relocation summary failed)"

# ── Corpus-honest gate (ISOLATED: same-binary baseline) ──────────────────────
echo ""; echo "── Corpus-honest gate (isolated) — $(date) ──"
python3 scripts/corpus_honest_gate.py \
    --baseline "$BASE_PARQUET" \
    --candidate "$CAND_PARQUET" \
    --label "username-veto-isolated" \
    --out-dir "$OUT" | tee "$OUT/corpus_honest_gate.txt"

echo ""
echo "================================================================"
echo " done $(date)"
echo " baseline:  $BASE_PARQUET"
echo " candidate: $CAND_PARQUET"
echo " gate:      $OUT/corpus_honest_gate.txt"
echo " REVIEW AWAKE: a full_name trigger may be a subset referee-vocabulary"
echo "   false alarm — cross-check relocated handle columns against gold."
echo "================================================================"
