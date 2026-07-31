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
#   7. a GLOB entry suppresses across a subtree — and does not reach outside it.
#
# Every violating string below is assembled at runtime from harmless pieces
# (`printf '%s%s' . orbit/`), never written as a literal. This file is tracked and
# the gate scans it, so a literal here would violate the very rule it tests.
#
# Usage:  ./scripts/check-public-hygiene-selftest.sh
# Exit:   0 all assertions passed, 1 otherwise.

set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CHECK="$HERE/check-public-hygiene.sh"
FIXTURE="$HERE/public-hygiene-innocent-strings.txt"

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

# The two offending shapes, never written literally.
ORBIT="$(printf '%s%s' . 'orbit/')"
HOME_ABS="$(printf '/%s/%s' Users hugh)"
HOME_LINUX="$(printf '/%s/%s' home hugh)"

# A throwaway repo with the real checker and an empty allowlist.
new_repo() {
	local d
	d="$(mktemp -d "$TMPROOT/repo.XXXXXX")"
	git init -q "$d"
	mkdir -p "$d/scripts"
	cp "$CHECK" "$d/scripts/check-public-hygiene.sh"
	: >"$d/scripts/public-hygiene-allowlist.txt"
	printf '%s' "$d"
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
perl -0pi -e 's/\Q[.]orbit\E/[.]orbit(/' "$d/scripts/check-public-hygiene.sh"
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

echo
if [[ $failures -eq 0 ]]; then
	echo "check-public-hygiene-selftest: all assertions passed."
	exit 0
fi
echo "check-public-hygiene-selftest: $failures assertion(s) failed."
exit 1
