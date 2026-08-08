#!/usr/bin/env bash
# Mutation harness for scripts/check-formula-asset.sh — proof that its self-test
# still DETECTS, not merely that it passes.
#
# scripts/check-formula-asset-selftest.sh asserts that the check goes red on a
# wrong checksum, on an unreachable url, and on a formula that lost a platform
# block. Every one of those assertions would keep passing if the check were
# quietly replaced by `exit 0`, because a self-test is only as good as the
# failures it can still see. So this file writes deliberately broken copies of
# the check, points the self-test at each in turn, and requires the self-test to
# fail on every one of them.
#
# The mutants below are the ways this check plausibly rots:
#
#   always-exit-0            the check reports clean whatever it found
#   digest-comparison-gone   it fetches the asset and never compares the hash
#   first-pair-only          it stops after the first platform pair
#   http-status-ignored      it accepts a 404 body as if it were the asset
#   min-pairs-guard-gone     a formula that lost platform blocks passes
#   last-pair-only           the parser keeps only the final pair it read
#   first-and-last-only      it checks the ends of the file and skips the middle
#
# The last two are why the self-test fixture carries four platform pairs rather
# than two. With two pairs, "first", "last" and "all" are the same set, and both
# of those mutants would survive.
#
# The closing reconciliation in the check — `checked + failures == pairs` before
# it may report clean — cannot be proved this way, and the last section of this
# file proves it a second way. See the comment above that section for why the
# self-test cannot reach it.
#
# A mutation that fails to apply is a hard error here, not a skip: an unapplied
# mutation leaves the original script in place, the self-test passes, and the
# harness would otherwise report a mutant "caught" that was never created. The
# unmutated check is run through the same self-test first, or every catch below
# is noise.
#
# Usage:  ./scripts/check-formula-asset-mutations.sh
# Exit:   0 every mutant was caught, 1 otherwise, 2 the harness could not run.
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CHECK="$HERE/check-formula-asset.sh"
SELFTEST="$HERE/check-formula-asset-selftest.sh"

for f in "$CHECK" "$SELFTEST"; do
	[[ -x "$f" ]] || {
		echo "mutations: $f is missing or not executable" >&2
		exit 2
	}
done

command -v python3 >/dev/null 2>&1 || {
	echo "mutations: python3 is required to write the mutants" >&2
	exit 2
}

TMP="$(mktemp -d)" || exit 2
trap 'rm -rf "$TMP"' EXIT

failures=0
caught=0
proved=0
ok() {
	printf '  caught  %s\n' "$1"
	caught=$((caught + 1))
}
bad() {
	printf '  MISSED  %s\n' "$1"
	failures=$((failures + 1))
}
# The second section asserts on a mutant's behaviour directly rather than on
# whether the self-test saw it, so it reports in its own words: one of its three
# mutants is expected to reach exit 0, and "caught" would be the wrong word for
# an assertion that it did.
asserted() {
	printf '  ok      %s\n' "$1"
	proved=$((proved + 1))
}
wrong() {
	printf '  FAIL    %s\n' "$1"
	failures=$((failures + 1))
}

# mutate <dest> <python-regex> <replacement> [<regex> <replacement> …] — writes a
# mutated copy of the check with each edit applied in order, and exits 2 if any
# one of them matched nothing.
#
# More than one edit because a guard that is unreachable while the rest of the
# check is correct can only be shown to matter alongside the defect it exists to
# catch; a single-edit harness can express the defect or the missing guard, but
# never the difference between them.
mutate() {
	local dest="$1"
	shift
	python3 - "$CHECK" "$dest" "$@" <<'PY' || exit 2
import re
import sys

src, dst = sys.argv[1:3]
edits = sys.argv[3:]
if not edits or len(edits) % 2:
    sys.stderr.write("mutations: mutate takes pattern/replacement pairs\n")
    sys.exit(1)
with open(src, encoding="utf-8") as fh:
    text = fh.read()
for pattern, replacement in zip(edits[::2], edits[1::2]):
    text, n = re.subn(pattern, replacement, text, flags=re.M)
    if n == 0:
        sys.stderr.write("mutations: pattern matched nothing: %s\n" % pattern)
        sys.exit(1)
with open(dst, "w", encoding="utf-8") as fh:
    fh.write(text)
PY
	chmod +x "$dest"
	# A mutant bash refuses to parse fails everything downstream for a reason
	# that has nothing to do with the defect, and this file would read that as a
	# catch. Reject it here instead.
	if ! bash -n "$dest" 2>"$TMP/syntax.err"; then
		echo "mutations: the mutant $dest is not valid bash" >&2
		sed 's/^/    /' "$TMP/syntax.err" >&2
		exit 2
	fi
}

# The self-test is noisy by design; each run's transcript goes to a file so a
# failing mutant can be explained without burying the report.
run_selftest() { # run_selftest <path-to-check> <log>
	CHECK_FORMULA_ASSET="$1" "$SELFTEST" >"$2" 2>&1
	return $?
}

echo "control: the unmutated check must pass its own self-test"
if run_selftest "$CHECK" "$TMP/control.log"; then
	printf '  ok      the check as written passes\n'
else
	echo "mutations: the UNMUTATED check fails its self-test — fix that first," >&2
	echo "           because every 'caught' below would be meaningless." >&2
	sed 's/^/    /' "$TMP/control.log" >&2
	exit 1
fi

echo
echo "each deliberately broken copy must be caught:"

# --- always-exit-0 -----------------------------------------------------------
# Every refusal becomes a pass. This is the mutant a self-test exists for: the
# check still runs, still prints, still fetches, and reports clean regardless.
mutate "$TMP/m-always-exit-0.sh" '^([ \t]*)exit [12]$' '\1exit 0'
if run_selftest "$TMP/m-always-exit-0.sh" "$TMP/m1.log"; then
	bad "always-exit-0"
	sed 's/^/          /' "$TMP/m1.log"
else
	ok "always-exit-0"
fi

# --- digest comparison removed ----------------------------------------------
# The asset is fetched and its digest computed, and then the comparison against
# the formula's checksum is never made. The check would still prove the url
# resolves, which is the half that was already easy.
mutate "$TMP/m-digest-gone.sh" '^\tif \[\[ "\$got" != "\$want" \]\]; then$' '\tif false; then'
if run_selftest "$TMP/m-digest-gone.sh" "$TMP/m2.log"; then
	bad "digest-comparison-gone"
	sed 's/^/          /' "$TMP/m2.log"
else
	ok "digest-comparison-gone"
fi

# --- loop stopped after the first pair ---------------------------------------
# The classic early break. Three of the four published platforms go unchecked
# and the summary still reads clean.
mutate "$TMP/m-first-pair-only.sh" '^while \[\[ \$i -lt \$pairs \]\]; do$' 'while [[ $i -lt 1 ]]; do'
if run_selftest "$TMP/m-first-pair-only.sh" "$TMP/m3.log"; then
	bad "first-pair-only"
	sed 's/^/          /' "$TMP/m3.log"
else
	ok "first-pair-only"
fi

# --- HTTP status ignored ------------------------------------------------------
# Worth reading closely: this mutant does NOT make the 404 case exit 0, because
# curl writes the 404 body to the file and its digest cannot match the formula's
# checksum. The run stays at exit 1 by a second, accidental path. Only the
# self-test's assertion that the message says "HTTP 404" catches it — which is
# why that assertion is not decoration, and why a refactor that trusts the
# digest comparison alone silently loses the status check.
mutate "$TMP/m-status-ignored.sh" '^\tif \[\[ "\$code" != "200" \]\]; then$' '\tif false; then'
if run_selftest "$TMP/m-status-ignored.sh" "$TMP/m4.log"; then
	bad "http-status-ignored"
	sed 's/^/          /' "$TMP/m4.log"
else
	ok "http-status-ignored"
fi

# --- minimum-pairs guard removed ---------------------------------------------
# A formula that lost a platform block reports clean on the blocks that remain.
mutate "$TMP/m-min-pairs-gone.sh" '^if \[\[ \$pairs -lt \$MIN_PAIRS \]\]; then$' 'if false; then'
if run_selftest "$TMP/m-min-pairs-gone.sh" "$TMP/m5.log"; then
	bad "min-pairs-guard-gone"
	sed 's/^/          /' "$TMP/m5.log"
else
	ok "min-pairs-guard-gone"
fi

# --- parser keeps only the last pair ------------------------------------------
# `+=` becomes `=`, so each pair overwrites the one before it and the check ends
# up addressing the final platform block alone.
mutate "$TMP/m-last-pair-only.sh" '^(\t\t)(URLS|SHAS|LINES)\+=\(' '\1\2=('
if run_selftest "$TMP/m-last-pair-only.sh" "$TMP/m6.log"; then
	bad "last-pair-only"
	sed 's/^/          /' "$TMP/m6.log"
else
	ok "last-pair-only"
fi

# --- first and last pair only -------------------------------------------------
# The mutant a two-pair fixture cannot see. It checks the ends of the formula
# and skips everything between them, which on a four-platform formula means half
# the published assets are never fetched.
mutate "$TMP/m-first-and-last-only.sh" '^(\ti=\$\(\(i \+ 1\)\))$' '\1\n\t[[ $i -eq 2 || $i -eq 3 ]] && continue'
if run_selftest "$TMP/m-first-and-last-only.sh" "$TMP/m7.log"; then
	bad "first-and-last-only"
	sed 's/^/          /' "$TMP/m7.log"
else
	ok "first-and-last-only"
fi

# ---------------------------------------------------------------------------
# The reconciliation guard, which the self-test cannot reach.
#
# The check closes by requiring `checked + failures == pairs` before it may
# report clean. No formula can make that fire: the fetch loop has four
# `continue`s and each is preceded by `failures=$((failures + 1))`, the one path
# that increments neither counter is a hard `exit 2`, and the only fall-through
# to the bottom of the body increments `checked`. The arithmetic therefore
# closes for every input, `if false` in front of the guard changes nothing the
# self-test can observe, and run_selftest reports that mutant MISSED. The same
# is true of moving `checked=$((checked + 1))` to the top of the loop body:
# while no iteration skips, counting attempts and counting successes give the
# same number.
#
# So the guard is defence-in-depth against a future loop that skips, and it is
# proved the way defence-in-depth has to be — against a check already broken in
# the way the guard exists to catch, with the assertions made here rather than
# through the self-test. The base defect is a loop that advances past every pair
# without fetching anything. Three copies of it, one assertion each, under a
# control that pins the fixture at four pairs:
#
#   the guard as written        refuses at exit 2 and says the loop skipped
#   the guard as `if false`     reports clean at exit 0 having fetched nothing
#   `checked` counted at the    the arithmetic closes on attempts instead of
#     top of the loop body      matches, and the summary line goes back to
#                               being true by construction
#
# The second and third assert exit 0 on purpose. They are the differential: what
# the check does when the guard is gone is the measure of what the guard is for.
#
# Nothing in this section reaches the network. The control is refused by
# --min-pairs before the fetch loop is entered, and in each mutant the inserted
# `continue` sits above the curl, so the fixture's urls are never addressed and
# need not resolve.
# ---------------------------------------------------------------------------
echo
echo "the reconciliation guard, asserted directly because the self-test cannot reach it:"

cat >"$TMP/four-pairs.rb" <<'RB'
class Finetype < Formula
  desc "Fixture for the reconciliation assertions; its urls are never fetched"
  homepage "https://example.invalid/"
  version "0.0.0"

  on_macos do
    if Hardware::CPU.arm?
      url "https://example.invalid/tool-macos-arm.tar.gz"
      sha256 "1111111111111111111111111111111111111111111111111111111111111111"
    else
      url "https://example.invalid/tool-macos-intel.tar.gz"
      sha256 "2222222222222222222222222222222222222222222222222222222222222222"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://example.invalid/tool-linux-arm.tar.gz"
      sha256 "3333333333333333333333333333333333333333333333333333333333333333"
    else
      url "https://example.invalid/tool-linux-intel.tar.gz"
      sha256 "4444444444444444444444444444444444444444444444444444444444444444"
    end
  end
end
RB

# Advance past every pair without fetching it: no failure is recorded, and no
# success either.
SKIP_PATTERN='^(\ti=\$\(\(i \+ 1\)\))$'
SKIP_REPLACEMENT='\1\n\tcontinue'
RECONCILIATION='^if \[\[ \$\(\(checked \+ failures\)\) -ne \$pairs \]\]; then$'

# assert_run <check> <min-pairs> <expected-exit> <needle> <description>
assert_run() {
	local mutant="$1" min="$2" want="$3" needle="$4" desc="$5" out rc
	out="$("$mutant" --min-pairs "$min" "$TMP/four-pairs.rb" 2>&1)"
	rc=$?
	if [[ $rc -ne $want ]]; then
		wrong "$desc — expected exit $want, got $rc"
		printf '%s\n' "$out" | sed 's/^/          /'
		return 1
	fi
	if ! printf '%s\n' "$out" | grep -Fq -- "$needle"; then
		wrong "$desc — exit $rc was right, but the output never said: $needle"
		printf '%s\n' "$out" | sed 's/^/          /'
		return 1
	fi
	asserted "$desc (exit $rc)"
}

# "0 of 4" and "4 of 4" below are only worth reading if the fixture really holds
# four pairs. Pin that against the check as written, and pin it through the
# --min-pairs refusal so the fetch loop is never entered: a fixture that parsed
# as one pair would make the exit-2 assertion under it pass for the wrong
# reason.
assert_run "$CHECK" 5 2 \
	"carries 4 url/sha256 pair(s), expected at least 5" \
	"the fixture parses as four pairs under the check as written"

mutate "$TMP/m-fetches-nothing.sh" "$SKIP_PATTERN" "$SKIP_REPLACEMENT"
assert_run "$TMP/m-fetches-nothing.sh" 4 2 \
	"the loop did not reach every pair in the formula" \
	"a loop that fetches nothing is refused, and the reconciliation says why"

mutate "$TMP/m-reconciliation-gone.sh" \
	"$SKIP_PATTERN" "$SKIP_REPLACEMENT" \
	"$RECONCILIATION" 'if false; then'
assert_run "$TMP/m-reconciliation-gone.sh" 4 0 \
	"0 of 4 pair(s) fetched and matched" \
	"without the reconciliation the same loop reports clean having fetched nothing"

# `checked` moved from the success path to the top of the loop body, so it
# counts iterations entered rather than assets matched. The removal runs before
# the insertion; the other order would delete the line it had just written.
mutate "$TMP/m-checked-counts-attempts.sh" \
	"$SKIP_PATTERN" "$SKIP_REPLACEMENT" \
	'\n\tchecked=\$\(\(checked \+ 1\)\)\n' '\n' \
	'^while \[\[ \$i -lt \$pairs \]\]; do$' 'while [[ $i -lt $pairs ]]; do\n\tchecked=$((checked + 1))'
assert_run "$TMP/m-checked-counts-attempts.sh" 4 0 \
	"4 of 4 pair(s) fetched and matched" \
	"counting checked on entry closes the arithmetic and restores 'four of four'"

echo
if [[ $failures -ne 0 ]]; then
	echo "check-formula-asset mutations: $failures failure(s) — a mutant the self-test" >&2
	echo "    did not catch, or an assertion about the reconciliation guard that did" >&2
	echo "    not hold" >&2
	exit 1
fi
echo "check-formula-asset mutations: $caught mutant(s) caught by the self-test, and"
echo "    $proved assertion(s) about the reconciliation guard held"
