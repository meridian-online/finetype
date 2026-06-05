#!/bin/bash
# Round-trip coherence metrics for a CSV: profile -> self-generated schema -> validate.
#
# Emits two metrics that together gate a coherent profile->validate round-trip:
#
#   non_trivial_pct        = (# columns NOT typed as the catch-all) / (# columns)
#                            Recall: how many columns got a real, enforceable type
#                            rather than the plain_text/unknown VARCHAR fallback.
#
#   reject_rate_non_trivial = (rejected cells) / (non_trivial_cols * total_rows)
#                            Precision: over the columns we DID type, how many cells
#                            does the validator reject. Restricting to non-trivial
#                            columns blocks the "type everything VARCHAR" cheat —
#                            catch-all columns carry no pattern so reject nothing.
#                            (Rejects only ever come from columns with a validatable
#                            pattern, i.e. the non-trivial ones, so the summary
#                            reject count is already scoped correctly.)
#
# Targets (blog round-trip): non_trivial_pct >= 0.80 AND reject_rate_non_trivial <= 0.01
#
# Model selection: honours FINETYPE_MODEL_DIR (a path to the model dir, e.g.
# models/sherlock-v23-precision-relu-s42) so a retrained model can be swapped in
# without touching this script. The CLI reads its model path from FINETYPE_MODEL,
# so the harness translates FINETYPE_MODEL_DIR -> FINETYPE_MODEL for the CLI;
# FINETYPE_MODEL is still honoured directly if set. Binary overridable via
# FINETYPE_BIN (defaults to the finetype on PATH — the shipping CLI).
#
# Usage: scripts/roundtrip_metrics.sh <input.csv>

set -euo pipefail

BIN="${FINETYPE_BIN:-finetype}"
CSV="${1:?usage: roundtrip_metrics.sh <input.csv>}"

# AC contract: honour FINETYPE_MODEL_DIR. The CLI consumes FINETYPE_MODEL, so
# map the dir onto it (FINETYPE_MODEL_DIR wins when both are set).
if [[ -n "${FINETYPE_MODEL_DIR:-}" ]]; then
  export FINETYPE_MODEL="$FINETYPE_MODEL_DIR"
fi

if ! command -v "$BIN" >/dev/null 2>&1 && [[ ! -x "$BIN" ]]; then
  echo "error: finetype binary not found/executable at $BIN (build it, install it, or set FINETYPE_BIN)" >&2
  exit 3
fi
if [[ ! -f "$CSV" ]]; then
  echo "error: input csv not found: $CSV" >&2
  exit 3
fi

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

# 1. Profile (per-column labels) and generate a schema from those same labels.
"$BIN" profile -f "$CSV" -o csv          > "$TMP/profile.csv"   2>/dev/null
"$BIN" profile -f "$CSV" -o json-schema  > "$TMP/schema.json"   2>/dev/null

# 2. Validate the data against its own generated schema (check-only). Exit code is
#    1 when rejects exist; that is expected, so don't let set -e abort on it.
"$BIN" validate "$CSV" "$TMP/schema.json" -o json > "$TMP/validate.json" 2>/dev/null || true

# 3. Compute metrics.
python3 - "$TMP/profile.csv" "$TMP/validate.json" <<'PY'
import csv, json, sys

profile_csv, validate_json = sys.argv[1], sys.argv[2]

CATCH_ALL = {"representation.text.plain_text", "unknown"}

total_cols = 0
non_trivial_cols = 0
with open(profile_csv, newline="") as f:
    for row in csv.DictReader(f):
        total_cols += 1
        if row["type"] not in CATCH_ALL:
            non_trivial_cols += 1

with open(validate_json) as f:
    v = json.load(f)
total_rows = v["total_rows"]
rejects    = v["rejects"]
grade      = v["grade"]

non_trivial_pct = (non_trivial_cols / total_cols) if total_cols else 0.0
denom = non_trivial_cols * total_rows
reject_rate_non_trivial = (rejects / denom) if denom else 0.0

pass_a = non_trivial_pct >= 0.80
pass_b = reject_rate_non_trivial <= 0.01

print(f"non_trivial_pct={non_trivial_pct:.4f}")
print(f"reject_rate_non_trivial={reject_rate_non_trivial:.4f}")
print(f"non_trivial_cols={non_trivial_cols}")
print(f"total_cols={total_cols}")
print(f"total_rows={total_rows}")
print(f"rejects={rejects}")
print(f"grade={grade}")
print(f"target_a_non_trivial_pct>=0.80: {'PASS' if pass_a else 'FAIL'}")
print(f"target_b_reject_rate<=0.01: {'PASS' if pass_b else 'FAIL'}")
print(f"OVERALL: {'PASS' if (pass_a and pass_b) else 'FAIL'}")
PY
