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
# The seven mutants are the ways this check plausibly rots:
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
ok() { printf '  caught  %s\n' "$1"; }
bad() {
	printf '  MISSED  %s\n' "$1"
	failures=$((failures + 1))
}

# mutate <dest> <python-regex> <replacement> — writes a mutated copy of the
# check, and exits 2 if the pattern matched nothing.
mutate() {
	python3 - "$CHECK" "$1" "$2" "$3" <<'PY' || exit 2
import re
import sys

src, dst, pattern, replacement = sys.argv[1:5]
with open(src, encoding="utf-8") as fh:
    text = fh.read()
out, n = re.subn(pattern, replacement, text, flags=re.M)
if n == 0:
    sys.stderr.write("mutations: pattern matched nothing: %s\n" % pattern)
    sys.exit(1)
with open(dst, "w", encoding="utf-8") as fh:
    fh.write(out)
PY
	chmod +x "$1"
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

echo
if [[ $failures -ne 0 ]]; then
	echo "check-formula-asset mutations: $failures mutant(s) NOT caught by the self-test" >&2
	exit 1
fi
echo "check-formula-asset mutations: all 7 mutants caught"
