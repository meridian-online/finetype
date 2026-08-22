#!/usr/bin/env bash
# Regression test for scripts/check-public-hygiene.sh — the gate's own credibility.
#
# A hygiene gate fails in two directions and both are silent:
#
#   * it stops matching (a broken pattern, a rule that errors, an allowlist entry
#     that no longer suppresses anything) and reports clean while blind;
#   * it starts matching honest prose and gets switched off within the week.
#
# So this file exercises both. It builds a throwaway git repository, drops the
# real checker into it, and asserts on exit codes and output:
#
#   1. every covered shape is caught, named, and located;
#   2. the tracked innocent-strings fixture produces silence;
#   3. a rule that cannot run is FATAL and names itself, never "clean";
#   4. an allowlist entry that does not parse is fatal;
#   5. an allowlist entry that suppresses nothing is fatal;
#   6. a well-formed allowlist entry actually suppresses its match;
#   7. a GLOB entry suppresses across a subtree — and does not reach outside it;
#   8. the rules file IS the rule list — an absent or malformed one is fatal, and
#      a gate with no rules never reports clean;
#   9. the accepted-tree record behaves as a ratchet in every direction: an exact
#      record passes, growth is refused, a same-count SUBSTITUTION is refused, a
#      record that matches nothing is refused, and an entry whose reason key
#      nobody declared is refused;
#  10. matched text is redacted when CI is set, and printed when it is not.
#
# Every violating string below is assembled at runtime from harmless pieces
# (`printf '%s%s' . orbit/`), never written as a literal. This file is tracked and
# the gate scans it, so a literal here would violate the very rule it tests. The
# planning identifiers it builds are SYNTHETIC — the shapes are real, the numbers
# are nobody's — because a test that spells a live identifier out leaks it into
# this repository in the course of proving that it must not be.
#
# Usage:  ./scripts/check-public-hygiene-selftest.sh
# Exit:   0 all assertions passed, 1 otherwise.

set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CHECK="$HERE/check-public-hygiene.sh"
RULES="$HERE/public-hygiene-rules.txt"
FIXTURE="$HERE/public-hygiene-innocent-strings.txt"

# The gate redacts matched text when CI is set, and this file asserts on that
# text in both directions. Neither assertion may inherit whichever environment it
# happens to run in, so every case sets CI for itself.
unset CI
unset HYGIENE_SHOW_MATCHES

# The same digest the gate computes, so no fixture below hardcodes a hash.
if command -v sha256sum >/dev/null 2>&1; then
	hash_stdin() { sha256sum | cut -c1-12; }
else
	hash_stdin() { shasum -a 256 | cut -c1-12; }
fi

[[ -x "$CHECK" ]] || {
	echo "selftest: $CHECK is missing or not executable" >&2
	exit 1
}

failures=0
TMPROOT="$(mktemp -d)"
trap 'rm -rf "$TMPROOT"' EXIT

ok() { printf '  ok    %s\n' "$1"; }
bad() {
	printf '  FAIL  %s\n' "$1"
	failures=$((failures + 1))
}

# The offending shapes, never written literally and never a real identifier.
ORBIT="$(printf '%s%s' . 'orbit/')"
HOME_ABS="$(printf '/%s/%s' Users hugh)"
HOME_LINUX="$(printf '/%s/%s' home hugh)"

# Synthetic planning identifiers. Each word and each number is a separate token
# in this file's source, so the file carries none of the shapes it assembles.
_m=m
_ms=milestone
_card=card
_dec=decision
_ac=AC
_doc=doc
MILESTONE="$(printf '%s-%s' "$_m" 77)"
MILESTONE2="$(printf '%s-%s' "$_m" 78)"
MILESTONE_SPELLED="$(printf '%s %s-%s' "$_ms" "$_m" 77)"
CARDREF="$(printf '%s %s' "$_card" 0099)"
DECISIONREF="$(printf '%s-%s' "$_dec" 087)"
ACREF="$(printf '%s#%s' "$_ac" 9)"
DOCREF="$(printf '%s-%s' "$_doc" 042)"

# A throwaway repo with the real checker and an empty allowlist.
new_repo() {
	local d
	d="$(mktemp -d "$TMPROOT/repo.XXXXXX")"
	git init -q "$d"
	mkdir -p "$d/scripts"
	cp "$CHECK" "$d/scripts/check-public-hygiene.sh"
	# The real rules file, not a stand-in. A self-test that invents its own
	# rules proves a gate nobody runs.
	cp "$RULES" "$d/scripts/public-hygiene-rules.txt"
	: >"$d/scripts/public-hygiene-allowlist.txt"
	: >"$d/scripts/public-hygiene-accepted-tree.txt"
	printf '%s' "$d"
}

# The fingerprint the gate computes for one file and one rule: the distinct
# matched strings, sorted with LC_ALL=C, newline-terminated, SHA-256, 12 chars.
fingerprint_of() {
	printf '%s\n' "$@" | LC_ALL=C sort -u | hash_stdin
}

# run_gate <repo> -> prints combined output, returns the gate's exit code.
run_gate() {
	local d="$1"
	git -C "$d" add -A >/dev/null 2>&1
	(cd "$d" && ./scripts/check-public-hygiene.sh 2>&1)
}

# ---------------------------------------------------------------------------
# 1. Every covered shape is caught.
# ---------------------------------------------------------------------------
echo "covered shapes:"

check_violation() {
	local desc="$1" label="$2" line="$3" d out rc
	d="$(new_repo)"
	printf '%s\n' "$line" >"$d/subject.txt"
	out="$(run_gate "$d")"
	rc=$?
	if [[ $rc -ne 1 ]]; then
		bad "$desc — expected exit 1, got $rc"
		printf '%s\n' "$out" | sed 's/^/        /'
		return
	fi
	if ! printf '%s\n' "$out" | grep -q "^subject.txt:1: $label: "; then
		bad "$desc — no '$label' violation reported at subject.txt:1"
		printf '%s\n' "$out" | sed 's/^/        /'
		return
	fi
	ok "$desc"
}

check_violation "planning spec path" private-planning-path \
	"$(printf 'see %sspecs/2026-04-24-amount-variant-generators/spec.yaml' "$ORBIT")"
check_violation "planning choice path" private-planning-path \
	"$(printf '// verbatim from %schoices/0065-a-decision.md' "$ORBIT")"
check_violation "planning path in a doc comment" private-planning-path \
	"$(printf '/// Spec: `%s`.' "${ORBIT}specs/2026-04-20-a-slug/")"
check_violation "absolute macOS home path" absolute-home-path \
	"$(printf 'DATASETS = "%s/datasets/"' "$HOME_ABS")"
check_violation "absolute Linux home path" absolute-home-path \
	"$(printf 'cd %s/github/project || exit 1' "$HOME_LINUX")"
check_violation "home path inside a data value" absolute-home-path \
	"$(printf '  "local_path": "%s/datasets/geonames/2026-05-24",' "$HOME_ABS")"
check_violation "milestone id in prose" milestone-id \
	"$(printf 'Returns the R&D budget to %s (eval-corpus expansion).' "$MILESTONE")"
check_violation "milestone id in a doc comment" milestone-id \
	"$(printf '/// Baseline taken from the %s corpus pass.' "$MILESTONE")"
check_violation "milestone id spelled out" milestone-id \
	"$(printf 'blocked on %s, per the retrain hold' "$MILESTONE_SPELLED")"
check_violation "card id in a comment" planning-card-id \
	"$(printf '// Deterministic fast-path (%s): a conclusive sample.' "$CARDREF")"
check_violation "decision record in a doc comment" private-decision-record \
	"$(printf '/// Implements post-hoc locale detection (%s, Option B).' "$DECISIONREF")"
check_violation "acceptance-criterion shorthand" acceptance-criterion \
	"$(printf 'closes %s of the promotion spec' "$ACREF")"
check_violation "planning document id" private-doc-id \
	"$(printf 'see %s for the sequencing argument' "$DOCREF")"

# The two-directional half, and the half a hygiene gate usually skips. Each
# string below is what the rule above would match if it were one character
# looser, and each is honest text this repository actually contains.
echo "shapes that must NOT fire:"

check_silent() {
	local desc="$1" line="$2" d out rc
	d="$(new_repo)"
	printf '%s\n' "$line" >"$d/subject.txt"
	out="$(run_gate "$d")"
	rc=$?
	if [[ $rc -eq 0 ]]; then
		ok "$desc"
		return
	fi
	bad "$desc — expected exit 0, got $rc"
	printf '%s\n' "$out" | sed 's/^/        /'
}

check_silent "an upper-case M and a number is not a milestone id" \
	"$(printf 'Heliplataforma Equipo Modular %s-%s Heliport,-52.5,-68.3' M 10)"
check_silent "this project's own roadmap heading" \
	"$(printf '### %s %s — Validate & Report' Milestone 3)"
check_silent "a version or a matrix cell is not a milestone id" \
	"$(printf 'Data scaling pipeline (%s-%s to %s-%s)' M 3 M 5)"
check_silent "the word card without a number" \
	"a card in the renderer, and the card component it uses"
check_silent "a two-digit card-like number" \
	"$(printf 'see %s %s of the deck' "$_card" 42)"
check_silent "a decision without a number" \
	"the decision to defer was recorded in the changelog"
check_silent "a hex digest ending in the acceptance-criterion shape" \
	"$(printf 'rev = "%s#%s"' "$_ac" 4afea48)"

# ---------------------------------------------------------------------------
# 2. The innocent-strings fixture stays silent.
# ---------------------------------------------------------------------------
echo "innocent strings:"
d="$(new_repo)"
cp "$FIXTURE" "$d/subject.txt"
out="$(run_gate "$d")"
rc=$?
if [[ $rc -eq 0 ]]; then
	ok "fixture of innocent strings produces no violations"
else
	bad "fixture of innocent strings tripped the gate (exit $rc)"
	printf '%s\n' "$out" | sed 's/^/        /'
fi

# ---------------------------------------------------------------------------
# 3. A rule that cannot run is fatal and names itself.
# ---------------------------------------------------------------------------
echo "broken rule:"
d="$(new_repo)"
printf '%s\n' "$(printf 'see %sspecs/a/spec.yaml' "$ORBIT")" >"$d/subject.txt"
# Corrupt exactly one rule's pattern into something PCRE rejects: an unclosed
# group. git grep then exits 128 and prints nothing to stdout — indistinguishable
# from "no violations" unless the exit code is checked per rule.
perl -0pi -e 's/\Q[.]orbit\E/[.]orbit(/' "$d/scripts/public-hygiene-rules.txt"
out="$(run_gate "$d")"
rc=$?
if [[ $rc -eq 2 ]] && printf '%s\n' "$out" | grep -q "RULE FAILED TO RUN" &&
	printf '%s\n' "$out" | grep -q "private-planning-path"; then
	ok "a broken pattern is fatal and names the rule"
else
	bad "a broken pattern did not fail loudly (exit $rc)"
	printf '%s\n' "$out" | sed 's/^/        /'
fi

# ---------------------------------------------------------------------------
# 4-7. Allowlist behaviour.
# ---------------------------------------------------------------------------
echo "allowlist:"

subject_line="$(printf 'DATASETS = "%s/datasets/"' "$HOME_ABS")"

# 4. Unparseable entry.
d="$(new_repo)"
printf '%s\n' "$subject_line" >"$d/subject.txt"
printf 'subject.txt | %s\n' "$HOME_ABS" >"$d/scripts/public-hygiene-allowlist.txt"
out="$(run_gate "$d")"
rc=$?
if [[ $rc -eq 2 ]] && printf '%s\n' "$out" | grep -q "expected 3 '|'-separated fields"; then
	ok "an entry that does not parse is a hard error"
else
	bad "an unparseable allowlist entry was not rejected (exit $rc)"
	printf '%s\n' "$out" | sed 's/^/        /'
fi

# 4b. Parses, but the explanation is empty.
d="$(new_repo)"
printf '%s\n' "$subject_line" >"$d/subject.txt"
printf 'subject.txt | %s |\n' "$HOME_ABS" >"$d/scripts/public-hygiene-allowlist.txt"
out="$(run_gate "$d")"
rc=$?
if [[ $rc -eq 2 ]] && printf '%s\n' "$out" | grep -q "all required"; then
	ok "an entry with no written reason is a hard error"
else
	bad "a reasonless allowlist entry was not rejected (exit $rc)"
	printf '%s\n' "$out" | sed 's/^/        /'
fi

# 5. Stale entry — well formed, matches nothing.
d="$(new_repo)"
printf '%s\n' "$subject_line" >"$d/subject.txt"
printf 'subject.txt | %s | quoted from a file that has since moved\n' \
	"$(printf '/%s/%s' Users someone-else)" >"$d/scripts/public-hygiene-allowlist.txt"
out="$(run_gate "$d")"
rc=$?
if [[ $rc -eq 2 ]] && printf '%s\n' "$out" | grep -q "stale entry"; then
	ok "an entry that suppresses nothing is a hard error"
else
	bad "a stale allowlist entry was not rejected (exit $rc)"
	printf '%s\n' "$out" | sed 's/^/        /'
fi

# 6. Legitimate entry — must actually suppress.
d="$(new_repo)"
printf '%s\n' "$subject_line" >"$d/subject.txt"
printf 'subject.txt | %s | a dataset root this script opens\n' \
	"$HOME_ABS" >"$d/scripts/public-hygiene-allowlist.txt"
out="$(run_gate "$d")"
rc=$?
if [[ $rc -eq 0 ]] && printf '%s\n' "$out" | grep -q "1 allowlisted match"; then
	ok "a well-formed entry suppresses its match"
else
	bad "a legitimate allowlist entry did not suppress (exit $rc)"
	printf '%s\n' "$out" | sed 's/^/        /'
fi

# 7. A glob entry covers a subtree — and stops at its edge. Without the second
# half, `output/*` would be indistinguishable from `*` and the allowlist could
# wave the whole tree through while still reporting "clean (N allowlisted)".
d="$(new_repo)"
mkdir -p "$d/output/run-a" "$d/output/run-b" "$d/crates/thing/src"
printf '%s\n' "$subject_line" >"$d/output/run-a/manifest.json"
printf '%s\n' "$subject_line" >"$d/output/run-b/manifest.json"
printf '%s\n' "$subject_line" >"$d/crates/thing/src/lib.rs"
: >"$d/subject.txt"
printf 'output/*.json | %s | frozen run records\n' \
	"$HOME_ABS" >"$d/scripts/public-hygiene-allowlist.txt"
out="$(run_gate "$d")"
rc=$?
if [[ $rc -eq 1 ]] &&
	printf '%s\n' "$out" | grep -q "^crates/thing/src/lib.rs:1: absolute-home-path" &&
	! printf '%s\n' "$out" | grep -q "^output/"; then
	ok "a glob entry suppresses its subtree and nothing outside it"
else
	bad "the glob entry did not scope correctly (exit $rc)"
	printf '%s\n' "$out" | sed 's/^/        /'
fi

# ---------------------------------------------------------------------------
# 8. The rules file IS the rule list.
# ---------------------------------------------------------------------------
echo "rules file:"

subject_ms="$(printf 'Returns the R&D budget to %s.' "$MILESTONE")"

# 8a. Absent.
d="$(new_repo)"
printf '%s\n' "$subject_ms" >"$d/subject.txt"
rm -f "$d/scripts/public-hygiene-rules.txt"
out="$(run_gate "$d")"
rc=$?
if [[ $rc -eq 2 ]] && printf '%s\n' "$out" | grep -q "no rules to run"; then
	ok "a missing rules file is a hard error, not a clean tree"
else
	bad "a missing rules file did not fail loudly (exit $rc)"
	printf '%s\n' "$out" | sed 's/^/        /'
fi

# 8b. Present, parses, but declares nothing. The shape that makes a gate report
# clean over a tree it never looked at.
d="$(new_repo)"
printf '%s\n' "$subject_ms" >"$d/subject.txt"
printf '# every rule commented out\n' >"$d/scripts/public-hygiene-rules.txt"
out="$(run_gate "$d")"
rc=$?
if [[ $rc -eq 2 ]] && printf '%s\n' "$out" | grep -q "declares no rules"; then
	ok "a rules file with no rules is a hard error"
else
	bad "an empty rules file reported a verdict (exit $rc)"
	printf '%s\n' "$out" | sed 's/^/        /'
fi

# 8c. A line with no separator.
d="$(new_repo)"
printf '%s\n' "$subject_ms" >"$d/subject.txt"
printf 'milestone-id\n' >"$d/scripts/public-hygiene-rules.txt"
out="$(run_gate "$d")"
rc=$?
if [[ $rc -eq 2 ]] && printf '%s\n' "$out" | grep -q "expected '<label>|<pattern>'"; then
	ok "a malformed rule line is a hard error"
else
	bad "a malformed rule line was accepted (exit $rc)"
	printf '%s\n' "$out" | sed 's/^/        /'
fi

# ---------------------------------------------------------------------------
# 9. The accepted-tree record, in every direction it can be wrong.
#
# This is a RATCHET, so "it passes" is only one of five things it has to do. A
# record that waves through whatever it happens to find is not a record; it is
# the allowlist with extra steps and no reason field.
# ---------------------------------------------------------------------------
echo "accepted-tree record:"

accept_repo() {
	# accept_repo <count> <declared-count> <declared-fingerprint>
	local n="$1" dc="$2" dfp="$3" d i
	d="$(new_repo)"
	: >"$d/subject.txt"
	for ((i = 0; i < n; i++)); do
		printf 'the %s corpus pass, run %s\n' "$MILESTONE" "$i" >>"$d/subject.txt"
	done
	{
		printf 'reason pre-gate-tree: published before the rule existed\n'
		printf 'subject.txt | milestone-id | %s | %s | pre-gate-tree\n' "$dfp" "$dc"
	} >"$d/scripts/public-hygiene-accepted-tree.txt"
	printf '%s' "$d"
}

FP_ONE="$(fingerprint_of "$MILESTONE")"

# 9a. Exact record — passes, and says so.
d="$(accept_repo 3 3 "$FP_ONE")"
out="$(run_gate "$d")"
rc=$?
if [[ $rc -eq 0 ]] && printf '%s\n' "$out" | grep -q "3 recorded as accepted"; then
	ok "an exact record passes and is counted separately from the allowlist"
else
	bad "an exact accepted record did not pass (exit $rc)"
	printf '%s\n' "$out" | sed 's/^/        /'
fi

# 9b. It grew. The whole point.
d="$(accept_repo 4 3 "$FP_ONE")"
out="$(run_gate "$d")"
rc=$?
if [[ $rc -eq 2 ]] && printf '%s\n' "$out" | grep -q "GREW"; then
	ok "one more occurrence than recorded is refused, and named as growth"
else
	bad "growth past the recorded count was not refused (exit $rc)"
	printf '%s\n' "$out" | sed 's/^/        /'
fi

# 9c. Same count, different identifier. A count-only ratchet passes this, which
# is why the fingerprint is there.
d="$(new_repo)"
printf 'the %s corpus pass\n' "$MILESTONE2" >"$d/subject.txt"
{
	printf 'reason pre-gate-tree: published before the rule existed\n'
	printf 'subject.txt | milestone-id | %s | 1 | pre-gate-tree\n' "$FP_ONE"
} >"$d/scripts/public-hygiene-accepted-tree.txt"
out="$(run_gate "$d")"
rc=$?
if [[ $rc -eq 2 ]] && printf '%s\n' "$out" | grep -q "no longer describes the tree"; then
	ok "a same-count substitution is refused"
else
	bad "swapping one identifier for another at the same count passed (exit $rc)"
	printf '%s\n' "$out" | sed 's/^/        /'
fi

# 9d. Recorded, and the file has since been swept. A record of a leak that is
# gone is a hole nobody is watching.
d="$(new_repo)"
printf 'the corpus pass, no identifier here\n' >"$d/subject.txt"
{
	printf 'reason pre-gate-tree: published before the rule existed\n'
	printf 'subject.txt | milestone-id | %s | 1 | pre-gate-tree\n' "$FP_ONE"
} >"$d/scripts/public-hygiene-accepted-tree.txt"
out="$(run_gate "$d")"
rc=$?
if [[ $rc -eq 2 ]] && printf '%s\n' "$out" | grep -q "nothing matches it now"; then
	ok "a record that matches nothing is refused"
else
	bad "a stale accepted record was not refused (exit $rc)"
	printf '%s\n' "$out" | sed 's/^/        /'
fi

# 9e. An entry whose reason nobody wrote down.
d="$(new_repo)"
printf 'the %s corpus pass\n' "$MILESTONE" >"$d/subject.txt"
printf 'subject.txt | milestone-id | %s | 1 | some-key\n' "$FP_ONE" \
	>"$d/scripts/public-hygiene-accepted-tree.txt"
out="$(run_gate "$d")"
rc=$?
if [[ $rc -eq 2 ]] && printf '%s\n' "$out" | grep -q "is never declared"; then
	ok "an entry whose reason key is undeclared is refused"
else
	bad "an entry with no declared reason was accepted (exit $rc)"
	printf '%s\n' "$out" | sed 's/^/        /'
fi

# 9f. Four fields instead of five.
d="$(new_repo)"
printf 'the %s corpus pass\n' "$MILESTONE" >"$d/subject.txt"
{
	printf 'reason pre-gate-tree: published before the rule existed\n'
	printf 'subject.txt | milestone-id | %s | 1\n' "$FP_ONE"
} >"$d/scripts/public-hygiene-accepted-tree.txt"
out="$(run_gate "$d")"
rc=$?
if [[ $rc -eq 2 ]] && printf '%s\n' "$out" | grep -q "expected 5 '|'-separated fields"; then
	ok "an accepted entry that does not parse is refused"
else
	bad "a malformed accepted entry was not refused (exit $rc)"
	printf '%s\n' "$out" | sed 's/^/        /'
fi

# 9g. A record does NOT cover a different file. Without this, one entry would be
# indistinguishable from a repo-wide waiver.
d="$(new_repo)"
printf 'the %s corpus pass\n' "$MILESTONE" >"$d/subject.txt"
printf 'the %s corpus pass\n' "$MILESTONE" >"$d/elsewhere.txt"
{
	printf 'reason pre-gate-tree: published before the rule existed\n'
	printf 'subject.txt | milestone-id | %s | 1 | pre-gate-tree\n' "$FP_ONE"
} >"$d/scripts/public-hygiene-accepted-tree.txt"
out="$(run_gate "$d")"
rc=$?
if [[ $rc -eq 1 ]] && printf '%s\n' "$out" | grep -q "^elsewhere.txt:1: milestone-id" &&
	! printf '%s\n' "$out" | grep -q "^subject.txt:"; then
	ok "a record covers its own file and no other"
else
	bad "the accepted record leaked outside its file (exit $rc)"
	printf '%s\n' "$out" | sed 's/^/        /'
fi

# ---------------------------------------------------------------------------
# 10. Matched text in a public CI log.
#
# This repository's Actions logs are world-readable. A gate that prints the
# identifier it caught publishes it, which is the harm the gate exists to
# prevent, arriving through the gate.
# ---------------------------------------------------------------------------
echo "redaction:"

d="$(new_repo)"
printf 'the %s corpus pass\n' "$MILESTONE" >"$d/subject.txt"
git -C "$d" add -A >/dev/null 2>&1
out="$(cd "$d" && CI=true ./scripts/check-public-hygiene.sh 2>&1)"
rc=$?
if [[ $rc -eq 1 ]] && printf '%s\n' "$out" | grep -q "redacted" &&
	! printf '%s\n' "$out" | grep -qF "$MILESTONE"; then
	ok "under CI the rule and the location are named and the text is not"
else
	bad "the matched text was printed into a CI log (exit $rc)"
	printf '%s\n' "$out" | sed 's/^/        /'
fi

out="$(cd "$d" && CI=true HYGIENE_SHOW_MATCHES=1 ./scripts/check-public-hygiene.sh 2>&1)"
rc=$?
if [[ $rc -eq 1 ]] && printf '%s\n' "$out" | grep -qF "$MILESTONE"; then
	ok "the override prints the text, so the redaction is not a dead end"
else
	bad "HYGIENE_SHOW_MATCHES did not restore the text (exit $rc)"
	printf '%s\n' "$out" | sed 's/^/        /'
fi

out="$(cd "$d" && ./scripts/check-public-hygiene.sh 2>&1)"
rc=$?
if [[ $rc -eq 1 ]] && printf '%s\n' "$out" | grep -qF "$MILESTONE"; then
	ok "off CI the text is printed, which is how it gets fixed"
else
	bad "the local run did not print the matched text (exit $rc)"
	printf '%s\n' "$out" | sed 's/^/        /'
fi

echo
if [[ $failures -eq 0 ]]; then
	echo "check-public-hygiene-selftest: all assertions passed."
	exit 0
fi
echo "check-public-hygiene-selftest: $failures assertion(s) failed."
exit 1
