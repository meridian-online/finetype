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

# Overridable so a mutation harness can point this suite at a deliberately-broken copy
# of the tool and check that a NAMED case reddens. A suite that cannot be aimed at a
# wrong implementation has never been shown to detect one.
EV="${FINETYPE_EVIDENCE_BIN:-scripts/evidence.py}"

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

# `verify` and `render-release --check` are two separate comparisons of the same two
# things, and only one of them was ever exercised on a DRIFTED report. A build whose
# --check short-circuits its diff returns "ok" for a report that has drifted, and every
# caller that gates on --check — CI, the release runbook — is then gating on nothing.
CASE="--check reddens on a drifted report, not just verify"
sed -i.bak 's|2 / 3|3 / 3|' "$SBOX/evidence/release-1.0.0.md" && rm -f "$SBOX/evidence/release-1.0.0.md.bak"
chk_rc=0
"$EV" --manifest "$MAN" render-release --binary 1.0.0 --check >"$TMP/check.log" 2>&1 || chk_rc=$?
if [ "$chk_rc" -eq 0 ]; then
  bad "--check reported ok on a report whose number was edited by hand"
elif grep -q 'has drifted from' "$TMP/check.log"; then
  ok "--check names the drift and exits $chk_rc"
else
  bad "--check exited $chk_rc, said: $(tr "\n" " " < "$TMP/check.log" | cut -c1-200)"
fi
restore

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
# A second fixture version carrying a *different binary* on the same pipeline and the
# same denominator — the shape that tempts a subtraction. Everything a naive pair loop
# needs is present except the one thing that matters: the ground truth is not the same.
# The delta table must stay empty and the pair must be listed as refused instead.
GOLD2="$SBOX/gold2.tsv"
cp "$GOLD" "$GOLD2"; printf 'c4\talpha.one.thing\n' >> "$GOLD2"
"$EV" --manifest "$MAN" register-fixture --id sbox-v2 --path "$GOLD2" \
  --label-column curated_label --taxonomy-root "$SBOX" --date 2026-01-02 >/dev/null
"$EV" --manifest "$MAN" record-baseline --fixture sbox-v2 --key "m/p/0.9.0" \
  --correct 3 --scored 3 --model m --binary 0.9.0 --pipeline "p" --source "sandbox" >/dev/null
"$EV" --manifest "$MAN" render-release --binary 1.0.0 >/dev/null
REPORT="$SBOX/evidence/release-1.0.0.md"
delta_rows="$(sed -n '/^## Same-fixture comparison/,/^### /p' "$REPORT" | grep -c '^| `sbox' || true)"
if [ "$delta_rows" = "0" ]
then ok "no delta row is emitted for two different fixture versions"
else bad "the delta table stated $delta_rows cross-fixture row(s)"; fi

CASE="cross-fixture refusal is stated"
# Matched against the refused bullet itself, naming both fixture ids. A looser pattern
# matches this report's own intro prose about fixture versions and passes on a renderer
# that dropped the refusals entirely.
if grep -q '^- `sbox-v1` .* vs `sbox-v2` .*different fixture versions' "$REPORT"
then ok "the pair is listed as a refused comparison, naming both fixtures"
else bad "the cross-fixture pair was silently dropped instead of refused"; fi

CASE="same-fixture delta is still offered"
"$EV" --manifest "$MAN" record-baseline --fixture sbox-v1 --key "m/p/0.9.0" \
  --correct 1 --scored 3 --model m --binary 0.9.0 --pipeline "p" --source "sandbox" >/dev/null
"$EV" --manifest "$MAN" render-release --binary 1.0.0 >/dev/null
# The pipeline column is pinned too: two rows of one fixture (composed and Sense) are
# indistinguishable without it, and a reader cannot tell which number is which path.
if grep -q '^| `sbox-v1` | p | `0.9.0` |' "$REPORT"
then ok "same fixture, same pipeline, different binary: the delta is stated, pipeline named"
else bad "a legitimate same-fixture delta was refused too — the rule is over-broad"; fi
restore

# ------------------------------- the measuring binary's taxonomy, when it is known ----
# Two different taxonomies, and a report that conflates them is asserting something it
# did not measure: the one a fixture's labels were ADJUDICATED under, and the one the
# MEASURING BINARY was compiled with. The renderer used to emit a blanket "not captured
# at the time" for every release, which stopped being true the moment a run stamped it —
# a generated file stating a falsehood about its own numbers.
echo "== the report states the measuring binary's taxonomy only when the manifest carries it =="

CASE="an unstamped baseline is named as unstamped"
"$EV" --manifest "$MAN" render-release --binary 1.0.0 >/dev/null
if grep -q '^- `sbox-v1` · `m/p/1.0.0`$' "$REPORT" \
   && grep -q 'do \*\*not\*\* carry it' "$REPORT"
then ok "the score with no taxonomy stamp is listed under the not-carried heading"
else bad "an unstamped score was not declared unstamped"; fi

CASE="an unstamped baseline is not claimed to be stamped"
if grep -q 'measured by a binary built with taxonomy' "$REPORT"
then bad "the report asserted a measuring taxonomy no baseline records"
else ok "no measuring taxonomy is asserted when none was recorded"; fi

CASE="a stamped baseline states its taxonomy"
"$EV" --manifest "$MAN" record-baseline --force --fixture sbox-v1 --key "m/p/1.0.0" \
  --correct 2 --scored 3 --model m --binary 1.0.0 --pipeline "p" --source "sandbox" \
  --taxonomy tax-deadbeef1234 >/dev/null
"$EV" --manifest "$MAN" render-release --binary 1.0.0 >/dev/null
if grep -q 'measured by a binary built with taxonomy \*\*`tax-deadbeef1234`\*\*' "$REPORT"
then ok "the recorded taxonomy is stated verbatim"
else bad "the stamp was recorded but the report did not state it"; fi

CASE="a stamped baseline is not also listed as unstamped"
if grep -q 'do \*\*not\*\* carry it' "$REPORT"
then bad "a stamped score is still being declared unstamped"
else ok "the not-carried list is absent when every score carries a stamp"; fi

CASE="a mixed manifest names which scores carry a stamp and which do not"
"$EV" --manifest "$MAN" record-baseline --fixture sbox-v1 --key "m/q/1.0.0" \
  --correct 1 --scored 3 --model m --binary 1.0.0 --pipeline "q" --source "sandbox" >/dev/null
"$EV" --manifest "$MAN" render-release --binary 1.0.0 >/dev/null
if grep -q '^- `sbox-v1` · `m/p/1.0.0` — measured under `tax-deadbeef1234`$' "$REPORT" \
   && grep -q '^- `sbox-v1` · `m/q/1.0.0`$' "$REPORT"
then ok "each score is attributed to the stamp it actually has"
else bad "a mixed manifest was flattened into one blanket claim"; fi
restore

# ------------------------------------ a manifest that does not resolve stops it ----
# Everything below drives a *command* and asserts three things: the exit code the caller
# branches on, a named reason on stderr, and the absence of a Python traceback. The
# traceback check is the point. A manifest of the wrong shape used to reach code that
# assumed it had resolved — `doc.get` on a list, `.get` on a fixture that is a string —
# so the failure surfaced as an AttributeError several frames from its cause, reading
# like a bug in the tool rather than "your manifest is wrong". A crash is the good
# outcome here; the bad one is a run that carries on holding a value nobody measured.
BADDIR="$TMP/broken"
BAD="$BADDIR/fixtures.json"
mkdir -p "$BADDIR"

expect_cmd_fail() { # want-exit, want-substring, command...
  local want_code="$1" want="$2"
  shift 2
  local log="$TMP/cmd.log" got=0
  set +e
  "$@" >"$log" 2>&1
  got=$?
  set -e
  if [ "$got" -eq 0 ]; then
    bad "exited 0 — it proceeded on a manifest it should have refused (wanted $want_code: $want)"
  elif grep -q 'Traceback (most recent call last)' "$log"; then
    bad "crashed with a Python traceback instead of a named failure: $(tr '\n' ' ' < "$log" | cut -c1-200)"
  elif [ "$got" -ne "$want_code" ]; then
    bad "exited $got, expected $want_code; said: $(tr '\n' ' ' < "$log" | cut -c1-200)"
  elif ! grep -q -- "$want" "$log"; then
    bad "exited $want_code but not for '$want'; said: $(tr '\n' ' ' < "$log" | cut -c1-200)"
  else
    ok "$CASE"
  fi
}

# Each case starts from the good manifest and breaks one thing, so the rejection is
# attributable to that thing. $MAN is never touched; nothing here needs restore.
break_manifest() { python3 - "$MAN" "$BAD"; }

VBAD() { "$EV" --manifest "$BAD" verify --taxonomy-root "$SBOX" --evidence-dir "$SBOX/evidence"; }

echo "== a manifest that cannot be resolved is refused, not worked around =="

CASE="a manifest that is not JSON"
printf '{ "schema": broken,\n' > "$BAD"
expect_cmd_fail 5 "is not valid JSON" VBAD

CASE="a manifest that is valid JSON but not an object"
printf '[]\n' > "$BAD"
expect_cmd_fail 5 "not an object" VBAD

CASE="a manifest that is not UTF-8"
python3 -c "import sys; open(sys.argv[1],'wb').write(b'\xff\xfe{\x00\"s\x00')" "$BAD"
expect_cmd_fail 5 "could not be read as UTF-8" VBAD

CASE="a manifest declaring another schema"
break_manifest <<'PY'
import json, sys
d = json.load(open(sys.argv[1])); d["schema"] = "somebody/else@1"
json.dump(d, open(sys.argv[2], "w"), indent=2)
PY
expect_cmd_fail 5 "declares schema" VBAD

CASE="a 'fixtures' that is not an object"
break_manifest <<'PY'
import json, sys
d = json.load(open(sys.argv[1])); d["fixtures"] = []
json.dump(d, open(sys.argv[2], "w"), indent=2)
PY
expect_cmd_fail 5 "has no 'fixtures' object" VBAD

CASE="a fixture entry that is not an object"
break_manifest <<'PY'
import json, sys
d = json.load(open(sys.argv[1])); d["fixtures"]["sbox-v1"] = "gold-corpus"
json.dump(d, open(sys.argv[2], "w"), indent=2)
PY
expect_cmd_fail 5 "not an object" VBAD

CASE="a 'reports' that is a bare string"
# The shape that silently weakens the check rather than failing it: iterating "1.0.0"
# yields five one-character release names, none of which is the report that must exist.
break_manifest <<'PY'
import json, sys
d = json.load(open(sys.argv[1])); d["reports"] = "1.0.0"
json.dump(d, open(sys.argv[2], "w"), indent=2)
PY
expect_cmd_fail 5 "not a list of version strings" VBAD

CASE="a 'reports' that is a list of non-strings"
# The half of the same check the case above cannot reach. `["1.0.0"]` and `[1]` are both
# lists, so `isinstance(reports, list)` accepts each; only the element check separates
# them. Without it a report named `1` is looked for as a release and never found, and the
# renderer's "the committed report is current" check silently stops checking.
break_manifest <<'PY'
import json, sys
d = json.load(open(sys.argv[1])); d["reports"] = [1]
json.dump(d, open(sys.argv[2], "w"), indent=2)
PY
expect_cmd_fail 5 "not a list of version strings" VBAD

# ------------------------------------ a lookup that finds nothing stops the run ----
echo "== a lookup that resolves to nothing is a failure, not an empty value =="

CASE="resolve-fixture on an unregistered file"
expect_cmd_fail 3 "is not a registered fixture version" \
  "$EV" --manifest "$MAN" resolve-fixture --path "$SBOX/labels/definitions_beta.yaml"

CASE="resolve-fixture --format tsv on an unregistered file"
expect_cmd_fail 3 "is not a registered fixture version" \
  "$EV" --manifest "$MAN" resolve-fixture --path "$SBOX/labels/definitions_beta.yaml" --format tsv

CASE="get-baseline on a fixture that is not registered"
expect_cmd_fail 3 "no fixture 'sbox-v9'" \
  "$EV" --manifest "$MAN" get-baseline --fixture sbox-v9 --key "m/p/1.0.0"

CASE="record-baseline against a fixture that is not registered"
expect_cmd_fail 3 "no fixture 'sbox-v9'" \
  "$EV" --manifest "$MAN" record-baseline --fixture sbox-v9 --key k --correct 1 --scored 2 \
  --source sandbox

CASE="get-baseline for a key with no recorded score"
expect_cmd_fail 4 "has no recorded score" \
  "$EV" --manifest "$MAN" get-baseline --fixture sbox-v1 --key "m/p/never-run"

CASE="get-baseline --format tsv for a key with no recorded score"
# The tsv branch is the one the scorer calls. It must not emit a row of empty fields for
# a bar that was never measured — a blank line still gets `cut` into a comparison.
expect_cmd_fail 4 "has no recorded score" \
  "$EV" --manifest "$MAN" get-baseline --fixture sbox-v1 --key "m/p/never-run" --format tsv

# ---------------------------------------- an incomplete bar is never emitted ----
echo "== an incomplete bar is refused, not emitted with blank fields =="

GB() { "$EV" --manifest "$BAD" get-baseline --fixture sbox-v1 --key "m/p/1.0.0" --format tsv; }

CASE="a bar missing its column count"
break_manifest <<'PY'
import json, sys
d = json.load(open(sys.argv[1]))
del d["fixtures"]["sbox-v1"]["baselines"]["m/p/1.0.0"]["correct"]
json.dump(d, open(sys.argv[2], "w"), indent=2)
PY
expect_cmd_fail 5 "is missing correct" GB

CASE="a bar whose score is a string"
break_manifest <<'PY'
import json, sys
d = json.load(open(sys.argv[1]))
d["fixtures"]["sbox-v1"]["baselines"]["m/p/1.0.0"]["score"] = "0.667"
json.dump(d, open(sys.argv[2], "w"), indent=2)
PY
expect_cmd_fail 5 "is not a number" GB

CASE="a bar whose score is a JSON boolean"
# `bool` is a subclass of `int`, so `isinstance(True, (int, float))` is True and the
# type check alone lets it through. `format(True, '.3f')` then prints 1.000 — a bar of
# perfect accuracy conjured out of a typo.
break_manifest <<'PY'
import json, sys
d = json.load(open(sys.argv[1]))
d["fixtures"]["sbox-v1"]["baselines"]["m/p/1.0.0"]["score"] = True
json.dump(d, open(sys.argv[2], "w"), indent=2)
PY
expect_cmd_fail 5 "records score=True, which is not a number" GB

CASE="a bar whose column count is a string"
break_manifest <<'PY'
import json, sys
d = json.load(open(sys.argv[1]))
d["fixtures"]["sbox-v1"]["baselines"]["m/p/1.0.0"]["scored"] = "3"
json.dump(d, open(sys.argv[2], "w"), indent=2)
PY
expect_cmd_fail 5 "not a whole number of columns" GB

CASE="a bar whose column count is a JSON boolean"
# The load-bearing one, and the reason the bool exclusion is not pedantry. `bool` is a
# subclass of `int`, so an `int` check alone accepts `true`; get-baseline then emits
# `True` in field 2, score_clean_label.sh cuts it out (line 203) and feeds it to
# `$((COMP_CORRECT - BASE_CORRECT))` (line 212), where bash reads an unset name as 0.
# The run reports its whole column count as the delta, calls that at-or-above, and
# passes — a confident comparison against nothing, which is the failure this manifest
# exists to prevent.
break_manifest <<'PY'
import json, sys
d = json.load(open(sys.argv[1]))
d["fixtures"]["sbox-v1"]["baselines"]["m/p/1.0.0"]["correct"] = True
json.dump(d, open(sys.argv[2], "w"), indent=2)
PY
expect_cmd_fail 5 "records correct=True, which is not a whole number of columns" GB

CASE="a 'baselines' that is not an object"
break_manifest <<'PY'
import json, sys
d = json.load(open(sys.argv[1]))
d["fixtures"]["sbox-v1"]["baselines"] = ["m/p/1.0.0"]
json.dump(d, open(sys.argv[2], "w"), indent=2)
PY
expect_cmd_fail 5 "not an object" GB

CASE="a bar that is not a measurement"
break_manifest <<'PY'
import json, sys
d = json.load(open(sys.argv[1]))
d["fixtures"]["sbox-v1"]["baselines"]["m/p/1.0.0"] = 0.667
json.dump(d, open(sys.argv[2], "w"), indent=2)
PY
expect_cmd_fail 5 "not a recorded measurement" GB

CASE="record-baseline reporting a malformed prior measurement"
break_manifest <<'PY'
import json, sys
d = json.load(open(sys.argv[1]))
d["fixtures"]["sbox-v1"]["baselines"]["m/p/1.0.0"] = {"score": 0.667}
json.dump(d, open(sys.argv[2], "w"), indent=2)
PY
expect_cmd_fail 5 "is missing correct, scored" \
  "$EV" --manifest "$BAD" record-baseline --fixture sbox-v1 --key "m/p/1.0.0" \
  --correct 3 --scored 3 --source sandbox

# The positive control for the whole section. Without it every case above is satisfied
# by a get-baseline that refuses everything, which would break the scorer outright.
# `set +e` around the capture is load-bearing: under `set -e` a failing command
# substitution aborts the whole script, so a refuse-everything mutation would end the
# run here instead of reddening a named case — red either way, but unattributable.
emitted() { # capture stdout+stderr and exit code without tripping set -e
  set +e
  GOT="$("$@" 2>&1)"
  RC=$?
  set -e
}

CASE="a complete bar is still emitted in full"
emitted "$EV" --manifest "$MAN" get-baseline --fixture sbox-v1 --key "m/p/1.0.0" --format tsv
want="$(printf '0.667\t2\t3\tm\t1.0.0\tp\tsandbox')"
if [ "$RC" -ne 0 ]; then
  bad "get-baseline refused a complete bar (exit $RC): $(printf '%s' "$GOT" | tr '\n' ' ' | cut -c1-160)"
elif [ "$GOT" = "$want" ]; then
  ok "score, correct, scored, model, binary, pipeline and source all survive"
else
  bad "the emitted record changed: got '$GOT', want '$want'"
fi

CASE="a complete bar is still emitted as a bare score"
emitted "$EV" --manifest "$MAN" get-baseline --fixture sbox-v1 --key "m/p/1.0.0"
if [ "$RC" -ne 0 ]; then
  bad "get-baseline refused a complete bar (exit $RC): $(printf '%s' "$GOT" | tr '\n' ' ' | cut -c1-160)"
elif [ "$GOT" = "0.667" ]; then
  ok "the default format still prints the score"
else
  bad "got '$GOT', want 0.667"
fi

# ------------------------------------------------------------ the real reports ----
# Every release listed under `reports` in the real manifest, not a hardcoded
# version. A per-release edit here is a step someone forgets, and the release it
# is forgotten on is the one whose report goes unchecked.
REPORTS=$(python3 -c 'import json;print(" ".join(json.load(open("'"$ROOT"'/evidence/fixtures.json")).get("reports",[])))')
# What is on disk, independently. `verify` only re-renders the versions the
# manifest lists, so a committed report the manifest forgot is a report nobody
# checks — comparing the two sets is what surfaces that.
ONDISK=$(ls "$ROOT"/evidence/release-*.md 2>/dev/null \
  | sed -e 's#.*/release-##' -e 's#\.md$##' | sort | tr '\n' ' ')

echo "== every committed release report is current =="
CASE="the manifest lists at least one release report"
if [ -n "$REPORTS" ]
then ok "reports: $REPORTS"
else bad "evidence/fixtures.json lists no release reports — nothing below is checked"; fi

CASE="the manifest's report list matches the reports on disk"
WANT=$(printf '%s\n' $REPORTS | sort | tr '\n' ' ')
if [ "$WANT" = "$ONDISK" ]
then ok "manifest and evidence/ agree: $ONDISK"
else bad "manifest lists [$WANT] but evidence/ holds [$ONDISK] — an unlisted report is one verify never re-renders"; fi

for V in $REPORTS; do
  # The comparison is done HERE, against a freshly rendered copy, rather than by
  # trusting `--check` to report on itself. `--check` is the thing under test: a build
  # where its diff is short-circuited returns 0 for a drifted report, and a case that
  # only asks `--check` passes on exactly that build. Measured — mutation 5 in
  # scripts/test_evidence_mutations.sh survived this suite until this case stopped
  # delegating its own verdict.
  CASE="repo release report $V"
  FRESH="$TMP/rendered-$V.md"
  "$EV" --manifest "$ROOT/evidence/fixtures.json" render-release --binary "$V" \
    --out "$FRESH" >/dev/null 2>&1
  if cmp -s "$FRESH" "$ROOT/evidence/release-$V.md"
  then ok "evidence/release-$V.md matches a fresh render from evidence/fixtures.json"
       DRIFTED=0
  else bad "evidence/release-$V.md has drifted from the manifest"; DRIFTED=1; fi

  CASE="--check agrees with an independent diff for $V"
  CHECK_RC=0
  "$EV" --manifest "$ROOT/evidence/fixtures.json" render-release --binary "$V" --check \
    >/dev/null 2>&1 || CHECK_RC=$?
  if { [ "$DRIFTED" -eq 0 ] && [ "$CHECK_RC" -eq 0 ]; } \
     || { [ "$DRIFTED" -eq 1 ] && [ "$CHECK_RC" -ne 0 ]; }
  then ok "--check and the byte comparison reach the same verdict"
  else bad "--check said rc=$CHECK_RC while the byte comparison said drifted=$DRIFTED"; fi

  CASE="report $V names a fixture version beside every score"
  # One `gold-` id per headline row, plus the rows of the fixture table. A report
  # that quotes a score with no fixture id beside it is the thing evidence/ exists
  # to stop, so the floor is the row count of its own headline table.
  ROWS=$(awk '/^\| `gold-/ {n++} END {print n+0}' "$ROOT/evidence/release-$V.md")
  IDS=$(grep -c 'gold-2026-' "$ROOT/evidence/release-$V.md")
  if [ "$ROWS" -ge 1 ] && [ "$IDS" -ge "$ROWS" ]
  then ok "$ROWS headline/fixture rows, $IDS fixture ids"
  else bad "report $V quotes a score without a fixture version ($ROWS rows, $IDS ids)"; fi
done

printf '\n%s passed, %s failed\n' "$PASS" "$FAIL"
[ "$FAIL" -eq 0 ]
