#!/usr/bin/env bash
# scripts/overnight_attractor.sh — t-000133e418 attractor hard-negative retrain pipeline.
#
# Recipe: output/company-reference-audit/retrain_recipe_draft.md (author-approved 2026-07-05).
# Blend (pre-built by build_attractor_negatives_distilled.py) -> candidate FTMB ->
# destination-drift proxy precheck (BLOCKING: NO-GO means the overnight run is NOT
# launched, per spec 2026-06-05-destination-drift-precheck) -> overnight_potion.sh
# 3-seed train + gold score -> post-train Sense-distribution check (mandatory,
# non-fatal here: its verdict is a morning adjudication input, not a launch gate).
#
# The corpus-honest gate (post-train, fresh-vs-fresh vs models/default) is run
# separately after this pipeline — see the recipe draft §gate sequence.
set -eo pipefail
cd "$(cd "$(dirname "$0")/.." && pwd)"
export HF_HUB_OFFLINE=1 TRANSFORMERS_OFFLINE=1 HF_HUB_DISABLE_TELEMETRY=1 PYTHONUNBUFFERED=1

PY="eval/gittables/.venv/bin/python"
TAG="${TAG:-attneg}"
CONFIG="models/m2v8m-247-config.json"
BLEND="output/distillation-attneg/sherlock_distilled_attneg.csv.gz"
FTMB="output/multibranch-training/${TAG}-244.ftmb"   # overnight_potion.sh's --tag path convention
BASE_SNAP="output/destination-drift-precheck/sense_dist_m2v8mfx_s43.json"
FIXED_LIST="output/destination-drift-precheck/sense_dist_v19fx_s42.files.txt"
PROXY_EPOCHS="${PROXY_EPOCHS:-20}"   # round-1 evidence: 20-epoch readouts separated real drift from noise

echo "================ ATTNEG PIPELINE — $(date) ================"
[[ -f "$BLEND" ]] || { echo "FAIL: blend missing: $BLEND (run build_attractor_negatives_distilled.py)"; exit 1; }
[[ -f "$BASE_SNAP" ]] || { echo "FAIL: m2v8m-s43 baseline snapshot missing: $BASE_SNAP"; exit 1; }
[[ -f "$CONFIG" ]] || { echo "FAIL: config missing: $CONFIG"; exit 1; }

echo "--- STEP A: candidate FTMB (potion-8M embeds + hard-negative blend) ---"
if [[ ! -f "$FTMB" ]]; then
  "$PY" scripts/build_ftmb_v5_potion.py --potion minishlab/potion-base-8M \
    --distilled "$BLEND" --output "$FTMB" --workers 8
fi
"$PY" scripts/read_ftmb.py "$FTMB" --stats > output/distillation-attneg/ftmb_stats.txt
grep -E "records|label types|npi|upc|integer_number|plain_text|unix_seconds|decimal_number|\.word" \
  output/distillation-attneg/ftmb_stats.txt || true

echo "--- STEP B: destination-drift proxy precheck (BLOCKING; unix-only flag pre-adjudicated) ---"
PROXY_NAME="${TAG}-proxy${PROXY_EPOCHS}"
if scripts/proxy_pretrain.sh --name "$PROXY_NAME" --ftmb "$FTMB" --epochs "$PROXY_EPOCHS" \
     --baseline "$BASE_SNAP" --config "$CONFIG"; then
  echo "PROXY GO"
else
  # Pre-registered adjudication (derisk review 2026-07-06): datetime.epoch.unix_seconds
  # is the recipe's own targeted destination (verified ~92% Created-epoch corrections);
  # a NO-GO whose flagged set is EXACTLY {unix_seconds} passes. Anything else stops here.
  FLAGGED=$("$PY" -c "
import json
d = json.load(open('output/destination-drift-precheck/proxy_drift_${PROXY_NAME}.json'))
print(','.join(sorted(m['label'] for m in d['movers'] if m.get('flagged'))))")
  if [[ "$FLAGGED" == "datetime.epoch.unix_seconds" ]]; then
    echo "PROXY NO-GO adjudicated: flagged set is exactly {datetime.epoch.unix_seconds} — pre-registered, continuing"
  else
    echo "PROXY NO-GO — overnight NOT launched (mandated stop). Flagged: $FLAGGED"
    echo "Read output/destination-drift-precheck/proxy_drift_${PROXY_NAME}.json, iterate the blend, re-run."
    exit 1
  fi
fi

echo "--- STEP B2: damage-recovery precheck on the proxy model (BLOCKING) ---"
PROXY_DIR="models/sherlock-${PROXY_NAME}-s42"
if "$PY" scripts/damage_recovery_check.py --model "$PROXY_DIR" \
     --out "output/attneg-retrain/damage_recovery_${TAG}.json"; then
  echo "DAMAGE-RECOVERY GO — launching the 3-seed overnight run"
else
  echo "DAMAGE-RECOVERY NO-GO — the repair is not landing at proxy depth; overnight NOT launched."
  exit 1
fi

echo "--- STEP C: overnight 3-seed train + gold score (overnight_potion.sh) ---"
./scripts/overnight_potion.sh --tag "$TAG" --config "$CONFIG"

echo "--- STEP D: post-train Sense-distribution check (mandatory; adjudicated, not fatal) ---"
# Dual-encoder profile-time dependency: train-multi-branch does not save the value
# encoder, but profile-time loading (snapshot / corpus passes) requires it co-located
# in the model dir. The proxy hit exactly this: an encoder-less dir profiles ZERO
# columns and the drift gate reads an empty snapshot as NO-GO.
for D in models/${TAG}-s*; do
  if [[ -f "$D/model.safetensors" && ! -d "$D/value_model2vec" ]]; then
    cp -R models/m2v8m-s43/value_model2vec "$D/"
    "$PY" - "$D/config.json" <<'PYVE'
import json, sys
p = sys.argv[1]; c = json.load(open(p))
c.setdefault("value_embed_model", "value_model2vec")
json.dump(c, open(p, "w"), indent=2); open(p, "a").write("\n")
PYVE
    echo "co-located value encoder into $D"
  fi
done
BEST=$(TAG="$TAG" "$PY" - <<'PYBEST'
import glob, json, os
tag = os.environ.get("TAG", "attneg")
c = []
for p in glob.glob(f"models/{tag}-s*/results.json"):
    try:
        c.append((json.load(open(p)).get("val_accuracy", 0), p.split("/")[-2]))
    except Exception:
        pass
print(sorted(c)[-1][1] if c else "")
PYBEST
)
if [[ -n "$BEST" ]]; then
  echo "best seed by val_accuracy: $BEST"
  FINETYPE_BIN="$PWD/target/release/finetype" FINETYPE_MODEL="models/$BEST" \
    "$PY" scripts/snapshot_sense_distribution.py --label "${TAG}_post" \
    --file-list "$FIXED_LIST" --out-dir output/destination-drift-precheck || true
  if "$PY" scripts/drift_report.py "$BASE_SNAP" \
       "output/destination-drift-precheck/sense_dist_${TAG}_post.json" \
       --abs-floor 0.0040 --rel-mult 3.0 --direction up \
       --json "output/destination-drift-precheck/drift_${TAG}_post.json"; then
    echo "POST-TRAIN DRIFT: GO (no untargeted up-band label)"
  else
    echo "POST-TRAIN DRIFT: band tripped — morning adjudication input (the retrain's own"
    echo "targets move DOWN, so any up-flag is an untargeted destination; read the JSON)."
  fi
else
  echo "WARN: no models/attneg-s*/results.json found; post-train snapshot skipped"
fi

echo "================ ATTNEG PIPELINE done — $(date) ================"
echo "Next (morning): corpus-honest gate fresh-vs-fresh vs models/default + gold/repr review;"
echo "expected signature: collapse on npi+upc ONLY (pre-registered false alarm), nothing else banded."
