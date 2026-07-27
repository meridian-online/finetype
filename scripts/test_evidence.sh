#!/usr/bin/env bash
# test_evidence.sh — evidence/ must stay small, text-only and true to the manifest, and
# a fixture must name the label vocabulary it was adjudicated under.
#
# Every case runs against a sandbox repo root (its own labels/, gold TSV and evidence/)
# so a rejection is attributable to the thing the case names rather than to some
# unrelated defect making verify fail for free. The first case proves the sandbox is
# *accepted*; without it every case below would be green whatever it did.
#
# No arguments. Exit 0 = all cases pass.
set -eo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

TMP="$(mktemp -d "${TMPDIR:-/tmp}/evidence-test.XXXXXX")"
trap 'rm -rf "$TMP"' EXIT

PASS=0
FAIL=0
CASE=""

ok()  { PASS=$((PASS + 1)); printf '  ok   %s\n' "$1"; }
bad() { FAIL=$((FAIL + 1)); printf '  FAIL %s\n     %s\n' "$CASE" "$1"; }

EV="scripts/evidence.py"

# ------------------------------------------------------------------ sandbox ----
# A three-type taxonomy and a gold fixture that uses two of them.
SBOX="$TMP/sbox"
mkdir -p "$SBOX/labels" "$SBOX/evidence"
cat > "$SBOX/labels/definitions_alpha.yaml" <<'YAML'
# sandbox taxonomy
alpha.one.thing:
  broad_type: VARCHAR
alpha.two.other:
  broad_type: VARCHAR
YAML
cat > "$SBOX/labels/definitions_beta.yaml" <<'YAML'
beta.three.spare:
  broad_type: VARCHAR
YAML
GOLD="$SBOX/gold.tsv"
{
  printf 'column_name\tcurated_label\n'
  printf 'c1\talpha.one.thing\n'
  printf 'c2\talpha.two.other\n'
  printf 'c3\talpha.one.thing\n'
} > "$GOLD"

MAN="$SBOX/evidence/fixtures.json"
V() { "$EV" --manifest "$MAN" verify --taxonomy-root "$SBOX" --evidence-dir "$SBOX/evidence"; }

"$EV" --manifest "$MAN" register-fixture --id sbox-v1 --path "$GOLD" \
  --label-column curated_label --taxonomy-root "$SBOX" --taxonomy-commit deadbee \
  --note "sandbox" --date 2026-01-01 >/dev/null
"$EV" --manifest "$MAN" record-baseline --fixture sbox-v1 --key "m/p/1.0.0" \
  --correct 2 --scored 3 --model m --binary 1.0.0 --pipeline "p" --source "sandbox" >/dev/null
python3 - "$MAN" <<'PY'
import json, sys
p = sys.argv[1]
d = json.load(open(p))
d["reports"] = ["1.0.0"]
json.dump(d, open(p, "w"), indent=2)
PY
"$EV" --manifest "$MAN" render-release --binary 1.0.0 >/dev/null

# The snapshot every mutation case restores from.
SNAP="$TMP/snap"
cp -R "$SBOX" "$SNAP"
restore() { rm -rf "$SBOX"; cp -R "$SNAP" "$SBOX"; }

# ---------------------------------------------------------- attributability ----
echo "== the sandbox is accepted before anything is broken =="
CASE="baseline sandbox"
if V >/dev/null 2>&1
then ok "a consistent sandbox passes verify"
else bad "verify rejects a sandbox that is actually consistent — every case below is void"; fi

echo "== the committed manifest is self-consistent =="
CASE="committed manifest"
if "$EV" verify >/dev/null 2>&1; then ok "evidence.py verify (repo)"; else bad "repo verify failed"; fi

# --------------------------------------------------------- taxonomy version ----
echo "== taxonomy version is content, not a typed-in number =="
CASE="taxonomy version derivation"
before="$("$EV" taxonomy-version --root "$SBOX")"
printf '\n# a comment nobody thought was semantic\n' >> "$SBOX/labels/definitions_beta.yaml"
after="$("$EV" taxonomy-version --root "$SBOX")"
if [ "$before" != "$after" ]
then ok "editing a definitions file moves the taxonomy version ($before -> $after)"
else bad "a definitions edit left the taxonomy version at $before"; fi
restore

CASE="taxonomy version is derived from its digest"
tsv="$("$EV" taxonomy-version --root "$SBOX" --format tsv)"
ver="$(printf '%s' "$tsv" | cut -f1)"
sha="$(printf '%s' "$tsv" | cut -f2)"
if [ "$ver" = "tax-$(printf '%s' "$sha" | cut -c1-12)" ]
then ok "the printed version is the first 12 hex of its own digest"
else bad "version $ver is not derived from digest $sha"; fi

CASE="taxonomy type count"
n="$(printf '%s' "$tsv" | cut -f3)"
if [ "$n" = "3" ]; then ok "counts 3 types across two files"; else bad "counted $n types, expected 3"; fi

CASE="taxonomy type count agrees with a real YAML parse"
if python3 -c "import yaml" 2>/dev/null; then
  real="$(python3 - "$SBOX" <<'PY'
import pathlib, sys, yaml
keys = set()
for p in sorted((pathlib.Path(sys.argv[1]) / "labels").glob("definitions_*.yaml")):
    keys |= set(yaml.safe_load(p.read_text()) or {})
print(len(keys))
PY
)"
  if [ "$n" = "$real" ]
  then ok "regex count ($n) equals the PyYAML key count ($real)"
  else bad "regex counted $n top-level keys, PyYAML found $real"; fi
else
  ok "skipped: PyYAML not importable"
fi

CASE="repo taxonomy type count agrees with a real YAML parse"
if python3 -c "import yaml" 2>/dev/null; then
  repo_n="$("$EV" taxonomy-version --format tsv | cut -f3)"
  repo_real="$(python3 - "$ROOT" <<'PY'
import pathlib, sys, yaml
keys = set()
for p in sorted((pathlib.Path(sys.argv[1]) / "labels").glob("definitions_*.yaml")):
    keys |= set(yaml.safe_load(p.read_text()) or {})
print(len(keys))
PY
)"
  if [ "$repo_n" = "$repo_real" ]
  then ok "repo: regex count ($repo_n) equals the PyYAML key count ($repo_real)"
  else bad "repo: regex counted $repo_n, PyYAML found $repo_real"; fi
else
  ok "skipped: PyYAML not importable"
fi

# ------------------------------------------------------- manifest guards ----
# Each mutation writes the realistic wrong manifest and asserts verify names it.
mutate() { python3 - "$MAN" "$@"; }

expect_fail() { # CASE-message, expected substring in stderr
  local want="$1"
  local log="$TMP/out.log"
  if V >"$log" 2>&1; then
    bad "verify passed; expected it to reject: $want"
  elif grep -q -- "$want" "$log"; then
    ok "$CASE"
  else
    bad "verify rejected but not for '$want'; said: $(tr '\n' ' ' < "$log" | cut -c1-220)"
  fi
  restore
}

echo "== a fixture must name the vocabulary it was adjudicated under =="
CASE="missing taxonomy block"
mutate <<'PY'
import json, sys
p = sys.argv[1]; d = json.load(open(p))
del d["fixtures"]["sbox-v1"]["taxonomy"]
json.dump(d, open(p, "w"), indent=2)
PY
expect_fail "missing 'taxonomy'"

CASE="hand-written taxonomy version"
mutate <<'PY'
import json, sys
p = sys.argv[1]; d = json.load(open(p))
d["fixtures"]["sbox-v1"]["taxonomy"]["version"] = "tax-v2"
json.dump(d, open(p, "w"), indent=2)
PY
expect_fail "is not derived from its own digest"

CASE="taxonomy digest that is not a digest"
mutate <<'PY'
import json, sys
p = sys.argv[1]; d = json.load(open(p))
d["fixtures"]["sbox-v1"]["taxonomy"]["sha256"] = "nope"
json.dump(d, open(p, "w"), indent=2)
PY
expect_fail "is not a 64-char lowercase hex digest"

CASE="fixture version that disagrees with its id"
mutate <<'PY'
import json, sys
p = sys.argv[1]; d = json.load(open(p))
d["fixtures"]["sbox-v1"]["version"] = "sbox-v2"
json.dump(d, open(p, "w"), indent=2)
PY
expect_fail "a fixture's id is its version"

echo "== a fixture's labels must still be types the taxonomy defines =="
CASE="gold label the taxonomy dropped"
sed -i.bak 's/alpha.two.other/alpha.two.retired/' "$GOLD" && rm -f "$GOLD.bak"
python3 - "$MAN" "$GOLD" <<'PY'
import hashlib, json, sys
p, g = sys.argv[1], sys.argv[2]
d = json.load(open(p))
d["fixtures"]["sbox-v1"]["sha256"] = hashlib.sha256(open(g, "rb").read()).hexdigest()
json.dump(d, open(p, "w"), indent=2)
PY
expect_fail "not types in the checked-out taxonomy"

CASE="distinct-label count that drifted from the file"
mutate <<'PY'
import json, sys
p = sys.argv[1]; d = json.load(open(p))
d["fixtures"]["sbox-v1"]["labels_used"] = 99
json.dump(d, open(p, "w"), indent=2)
PY
expect_fail "distinct labels"

CASE="label column that is not in the fixture"
mutate <<'PY'
import json, sys
p = sys.argv[1]; d = json.load(open(p))
d["fixtures"]["sbox-v1"]["label_column"] = "not_a_column"
json.dump(d, open(p, "w"), indent=2)
PY
expect_fail "has no such column"

# ------------------------------------------------ evidence/ stays small + text ----
echo "== evidence/ holds text and small tabular files only =="
CASE="an oversized file"
python3 -c "open('$SBOX/evidence/big.md','w').write('x' * (200*1024 + 1))"
expect_fail "exceeds the 204800-byte per-file limit"

CASE="a parquet dump"
python3 -c "open('$SBOX/evidence/dump.parquet','wb').write(b'PAR1' + b'\0'*32)"
expect_fail "is not one of"

CASE="a binary artefact wearing a .tsv suffix"
python3 -c "open('$SBOX/evidence/dump.tsv','wb').write(b'\x89PNG\r\n\x1a\n' + bytes(range(256)))"
expect_fail "is not UTF-8 text"

CASE="the directory budget"
python3 - "$SBOX" <<'PY'
import sys, pathlib
d = pathlib.Path(sys.argv[1]) / "evidence"
for i in range(8):
    (d / f"pad{i}.md").write_text("y" * (150 * 1024))
PY
expect_fail "exceeds the 1048576-byte budget"

# ------------------------------------------------------ the report is generated ----
echo "== a listed release report must exist and match the manifest =="
CASE="a deleted release report"
rm -f "$SBOX/evidence/release-1.0.0.md"
expect_fail "is not on disk"

CASE="a hand-edited number in the report"
sed -i.bak 's|2 / 3|3 / 3|' "$SBOX/evidence/release-1.0.0.md" && rm -f "$SBOX/evidence/release-1.0.0.md.bak"
expect_fail "has drifted from the manifest"

CASE="a manifest number changed without re-rendering"
mutate <<'PY'
import json, sys
p = sys.argv[1]; d = json.load(open(p))
b = d["fixtures"]["sbox-v1"]["baselines"]["m/p/1.0.0"]
b["correct"], b["score"] = 3, 1.0
json.dump(d, open(p, "w"), indent=2)
PY
expect_fail "has drifted from the manifest"

# ------------------------------------------------------------ no cross-fixture ----
echo "== the report never subtracts across fixture versions =="
CASE="cross-fixture delta"
# A second fixture version, same pipeline, different ground truth. The delta table must
# stay empty and the pair must be listed as refused instead.
GOLD2="$SBOX/gold2.tsv"
cp "$GOLD" "$GOLD2"; printf 'c4\talpha.one.thing\n' >> "$GOLD2"
"$EV" --manifest "$MAN" register-fixture --id sbox-v2 --path "$GOLD2" \
  --label-column curated_label --taxonomy-root "$SBOX" --date 2026-01-02 >/dev/null
"$EV" --manifest "$MAN" record-baseline --fixture sbox-v2 --key "m/p/1.0.0" \
  --correct 3 --scored 3 --model m --binary 1.0.0 --pipeline "p" --source "sandbox" >/dev/null
"$EV" --manifest "$MAN" render-release --binary 1.0.0 >/dev/null
REPORT="$SBOX/evidence/release-1.0.0.md"
delta_rows="$(sed -n '/^## Same-fixture comparison/,/^## /p' "$REPORT" | grep -c '^| `sbox' || true)"
if [ "$delta_rows" = "0" ]
then ok "no delta row is emitted for two different fixture versions"
else bad "the delta table stated $delta_rows cross-fixture row(s)"; fi

CASE="cross-fixture refusal is stated"
if grep -q "different fixture versions" "$REPORT"
then ok "the pair is listed as a refused comparison"
else bad "the cross-fixture pair was silently dropped instead of refused"; fi

CASE="same-fixture delta is still offered"
"$EV" --manifest "$MAN" record-baseline --fixture sbox-v1 --key "m/p/0.9.0" \
  --correct 1 --scored 3 --model m --binary 0.9.0 --pipeline "p" --source "sandbox" >/dev/null
"$EV" --manifest "$MAN" render-release --binary 1.0.0 >/dev/null
if grep -q '^| `sbox-v1` | `0.9.0` |' "$REPORT"
then ok "same fixture, same pipeline, different binary: the delta is stated"
else bad "a legitimate same-fixture delta was refused too — the rule is over-broad"; fi
restore

# ------------------------------------------------------------ the real report ----
echo "== the committed 0.6.53 report is current =="
CASE="repo release report"
if "$EV" render-release --binary 0.6.53 --check >/dev/null 2>&1
then ok "evidence/release-0.6.53.md matches evidence/fixtures.json"
else bad "the committed report has drifted from the manifest"; fi

CASE="repo report names a fixture version beside every score"
if [ "$(grep -c 'gold-2026-' "$ROOT/evidence/release-0.6.53.md")" -ge 3 ]
then ok "every headline row carries its fixture version"
else bad "the committed report quotes a score without a fixture version"; fi

printf '\n%s passed, %s failed\n' "$PASS" "$FAIL"
[ "$FAIL" -eq 0 ]
