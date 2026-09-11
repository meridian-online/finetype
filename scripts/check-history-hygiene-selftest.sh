#!/usr/bin/env bash
# Regression test for scripts/check-history-hygiene.sh — the gate's credibility.
#
# A hygiene gate fails in two directions and both are silent:
#
#   * it stops matching -- a broken pattern, a rule that errors, a record that no
#     longer covers what it claims -- and reports clean while blind;
#   * it starts matching honest prose, and gets switched off within the week.
#
# So this file exercises both, against real commits in throwaway repositories:
#
#   1. every covered shape in a commit message is caught, and the commit and the
#      rule are named;
#   2. honest text that is one character away from each shape is NOT caught;
#   3. a recorded commit passes, and the record covers that commit and that rule
#      and nothing else -- not another commit, not another rule;
#   4. a record that is reachable and no longer matching is FATAL, and one that
#      has simply left the range is not;
#   5. a malformed record, an abbreviated sha, an undeclared reason key and a
#      total the record states about itself that its entries contradict are each
#      fatal;
#   6. --text catches the same shapes and accepts NOTHING: an identifier in a
#      pull request body fails even when a recorded commit carries the same one;
#   7. a rule that cannot run, an absent rules file and a rules file that
#      declares nothing are fatal, never "clean";
#   8. a range that does not resolve is fatal, never "no commits matched";
#   9. matched text is redacted when CI is set.
#
# Every violating string below is assembled at runtime from separate tokens, and
# every identifier is SYNTHETIC. This file is tracked and the other gate scans
# it, so a literal here would violate the rule it is testing -- and spelling a
# live identifier out to prove it must not be published would publish it.
#
# Usage:  ./scripts/check-history-hygiene-selftest.sh
# Exit:   0 all assertions passed, 1 otherwise.

set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CHECK="$HERE/check-history-hygiene.sh"
RULES="$HERE/public-hygiene-rules.txt"

[[ -x "$CHECK" ]] || {
	echo "selftest: $CHECK is missing or not executable" >&2
	exit 1
}

# Asserted on in both directions, so neither may inherit the ambient value.
unset CI
unset HYGIENE_SHOW_MATCHES

failures=0
TMPROOT="$(mktemp -d)"
trap 'rm -rf "$TMPROOT"' EXIT

ok() { printf '  ok    %s\n' "$1"; }
bad() {
	printf '  FAIL  %s\n' "$1"
	failures=$((failures + 1))
}

# Synthetic planning identifiers, each word and each number a separate token here.
_m=m
_ms=milestone
_card=card
_dec=decision
_ac=AC
_doc=doc
_orbit=orbit
MILESTONE="$(printf '%s-%s' "$_m" 77)"
MILESTONE_SPELLED="$(printf '%s %s-%s' "$_ms" "$_m" 77)"
CARDREF="$(printf '%s %s' "$_card" 0099)"
DECISIONREF="$(printf '%s-%s' "$_dec" 087)"
ACREF="$(printf '%s#%s' "$_ac" 9)"
DOCREF="$(printf '%s-%s' "$_doc" 042)"
ORBITPATH="$(printf '.%s/specs/2026-04-24-a-slug/spec.yaml' "$_orbit")"
HOMEPATH="$(printf '/%s/%s/datasets' Users someone)"

# A throwaway repo carrying the real gate and the real rules, with one commit
# whose message is whatever the caller passes.
new_repo() {
	local msg="${1:-a clean subject line}" d
	d="$(mktemp -d "$TMPROOT/repo.XXXXXX")"
	git init -q "$d"
	mkdir -p "$d/scripts"
	cp "$CHECK" "$d/scripts/check-history-hygiene.sh"
	cp "$RULES" "$d/scripts/public-hygiene-rules.txt"
	: >"$d/scripts/public-hygiene-accepted-history.txt"
	printf 'x\n' >"$d/file.txt"
	git -C "$d" add -A >/dev/null 2>&1
	git -C "$d" -c user.name=t -c user.email=t@e commit -q -m "$msg"
	printf '%s' "$d"
}

commit_more() {
	# commit_more <repo> <message>
	local d="$1" msg="$2"
	printf '%s\n' "$RANDOM" >>"$d/file.txt"
	git -C "$d" add -A >/dev/null 2>&1
	git -C "$d" -c user.name=t -c user.email=t@e commit -q -m "$msg"
}

run_gate() {
	local d="$1"
	shift
	(cd "$d" && ./scripts/check-history-hygiene.sh "$@" 2>&1)
}

# ---------------------------------------------------------------------------
# 1. Every covered shape, in a commit message.
# ---------------------------------------------------------------------------
echo "covered shapes in a commit message:"

check_violation() {
	local desc="$1" label="$2" msg="$3" d out rc sha
	d="$(new_repo "$msg")"
	sha="$(git -C "$d" rev-parse HEAD)"
	out="$(run_gate "$d")"
	rc=$?
	if [[ $rc -ne 1 ]]; then
		bad "$desc — expected exit 1, got $rc"
		printf '%s\n' "$out" | sed 's/^/        /'
		return
	fi
	if ! printf '%s\n' "$out" | grep -q "^$sha: $label\$"; then
		bad "$desc — no '$label' violation reported against $sha"
		printf '%s\n' "$out" | sed 's/^/        /'
		return
	fi
	ok "$desc"
}

check_violation "milestone id in a subject" milestone-id \
	"$(printf 'Close the %s eval-corpus work' "$MILESTONE")"
check_violation "milestone id spelled out" milestone-id \
	"$(printf 'Unblock the retrain, per %s' "$MILESTONE_SPELLED")"
check_violation "card id in a subject" planning-card-id \
	"$(printf 'Land the verbosity contract (%s)' "$CARDREF")"
check_violation "decision record in a body" private-decision-record \
	"$(printf 'Add locale detection\n\nImplements %s, Option B.' "$DECISIONREF")"
check_violation "acceptance-criterion shorthand" acceptance-criterion \
	"$(printf 'Close %s of the promotion spec' "$ACREF")"
check_violation "planning document id" private-doc-id \
	"$(printf 'Record the sequencing argument from %s' "$DOCREF")"
check_violation "planning path in a body" private-planning-path \
	"$(printf 'Add the generator\n\nSpec: %s' "$ORBITPATH")"
check_violation "absolute home path in a body" absolute-home-path \
	"$(printf 'Freeze the sample\n\nRead from %s' "$HOMEPATH")"

# ---------------------------------------------------------------------------
# 2. The direction a hygiene gate usually skips.
# ---------------------------------------------------------------------------
echo "messages that must NOT fire:"

check_silent() {
	local desc="$1" msg="$2" d out rc
	d="$(new_repo "$msg")"
	out="$(run_gate "$d")"
	rc=$?
	if [[ $rc -eq 0 ]]; then
		ok "$desc"
		return
	fi
	bad "$desc — expected exit 0, got $rc"
	printf '%s\n' "$out" | sed 's/^/        /'
}

check_silent "an upper-case letter and a number" \
	"$(printf 'Cycle-1 review-pr LOW finding %s-%s' M 1)"
check_silent "a stage label from a superseded spec" \
	"$(printf 'Data scaling pipeline (%s-%s to %s-%s)' M 3 M 5)"
check_silent "this project's own roadmap milestone" \
	"$(printf '%s %s: Validate and Report ships' Milestone 3)"
check_silent "the word card with no number" \
	"Rework the card component in the renderer"
check_silent "a decision with no number" \
	"Record the decision to defer the retrain"
check_silent "the retired tool's bare name" \
	"$(printf 'Drop the %s/ shim from the build' "$_orbit")"

# ---------------------------------------------------------------------------
# 3-5. The record.
# ---------------------------------------------------------------------------
echo "the accepted-history record:"

record() {
	# record <repo> <line>...
	local d="$1"
	shift
	local n=$#
	{
		printf 'total entries %s\ntotal commits %s\n' "$n" "$n"
		printf 'reason pre-gate-history: on main before the gate existed\n'
		printf '%s\n' "$@"
	} >"$d/scripts/public-hygiene-accepted-history.txt"
}

# 3a. A recorded commit passes.
d="$(new_repo "$(printf 'Close the %s eval-corpus work' "$MILESTONE")")"
sha="$(git -C "$d" rev-parse HEAD)"
record "$d" "$sha | milestone-id | pre-gate-history"
out="$(run_gate "$d")"
rc=$?
if [[ $rc -eq 0 ]] && printf '%s\n' "$out" | grep -q "1 commit-and-rule pair(s) recorded as accepted"; then
	ok "a recorded commit passes and is counted"
else
	bad "a recorded commit did not pass (exit $rc)"
	printf '%s\n' "$out" | sed 's/^/        /'
fi

# 3b. The record covers that RULE only. Without this a one-line entry would be a
# blanket amnesty for a commit, and the commonest real case is a message that
# carries two shapes.
d="$(new_repo "$(printf 'Close %s under %s' "$MILESTONE" "$CARDREF")")"
sha="$(git -C "$d" rev-parse HEAD)"
record "$d" "$sha | milestone-id | pre-gate-history"
out="$(run_gate "$d")"
rc=$?
if [[ $rc -eq 1 ]] && printf '%s\n' "$out" | grep -q "^$sha: planning-card-id\$" &&
	! printf '%s\n' "$out" | grep -q "^$sha: milestone-id\$"; then
	ok "a record covers its own rule and no other rule on the same commit"
else
	bad "the record covered a rule it does not name (exit $rc)"
	printf '%s\n' "$out" | sed 's/^/        /'
fi

# 3c. The record covers that COMMIT only.
d="$(new_repo "$(printf 'Close the %s eval-corpus work' "$MILESTONE")")"
sha="$(git -C "$d" rev-parse HEAD)"
commit_more "$d" "$(printf 'Return the budget to %s' "$MILESTONE")"
sha2="$(git -C "$d" rev-parse HEAD)"
record "$d" "$sha | milestone-id | pre-gate-history"
out="$(run_gate "$d")"
rc=$?
if [[ $rc -eq 1 ]] && printf '%s\n' "$out" | grep -q "^$sha2: milestone-id\$" &&
	! printf '%s\n' "$out" | grep -q "^$sha: milestone-id\$"; then
	ok "a record covers its own commit and no later one"
else
	bad "the record covered a commit it does not name (exit $rc)"
	printf '%s\n' "$out" | sed 's/^/        /'
fi

# 4a. Reachable and no longer matching: the record has stopped being true.
d="$(new_repo "a clean subject line")"
sha="$(git -C "$d" rev-parse HEAD)"
record "$d" "$sha | milestone-id | pre-gate-history"
out="$(run_gate "$d")"
rc=$?
if [[ $rc -eq 2 ]] && printf '%s\n' "$out" | grep -q "no longer matching"; then
	ok "a reachable record that matches nothing is a hard error"
else
	bad "a stale record was not refused (exit $rc)"
	printf '%s\n' "$out" | sed 's/^/        /'
fi

# 4b. Not reachable from this range: not this range's business. A branch cut
# before the entry was written must not redden on somebody else's history.
d="$(new_repo "a clean subject line")"
record "$d" "0000000000000000000000000000000000000000 | milestone-id | pre-gate-history"
out="$(run_gate "$d")"
rc=$?
if [[ $rc -eq 0 ]]; then
	ok "a record outside the range is skipped, not reported"
else
	bad "an unreachable record reddened the gate (exit $rc)"
	printf '%s\n' "$out" | sed 's/^/        /'
fi

# 5a. Two fields.
d="$(new_repo "a clean subject line")"
sha="$(git -C "$d" rev-parse HEAD)"
record "$d" "$sha | milestone-id"
out="$(run_gate "$d")"
rc=$?
if [[ $rc -eq 2 ]] && printf '%s\n' "$out" | grep -q "expected 3 '|'-separated fields"; then
	ok "a record that does not parse is a hard error"
else
	bad "a malformed record was accepted (exit $rc)"
	printf '%s\n' "$out" | sed 's/^/        /'
fi

# 5b. An abbreviated sha compares unequal to what git prints, so the entry would
# quietly cover nothing while looking like coverage.
d="$(new_repo "$(printf 'Close the %s eval-corpus work' "$MILESTONE")")"
sha="$(git -C "$d" rev-parse --short HEAD)"
record "$d" "$sha | milestone-id | pre-gate-history"
out="$(run_gate "$d")"
rc=$?
if [[ $rc -eq 2 ]] && printf '%s\n' "$out" | grep -q "not a full 40-character sha"; then
	ok "an abbreviated sha is refused rather than silently covering nothing"
else
	bad "an abbreviated sha was accepted (exit $rc)"
	printf '%s\n' "$out" | sed 's/^/        /'
fi

# 5c. A reason nobody declared.
d="$(new_repo "$(printf 'Close the %s eval-corpus work' "$MILESTONE")")"
sha="$(git -C "$d" rev-parse HEAD)"
printf 'total entries 1\ntotal commits 1\n%s | milestone-id | some-key\n' "$sha" \
	>"$d/scripts/public-hygiene-accepted-history.txt"
out="$(run_gate "$d")"
rc=$?
if [[ $rc -eq 2 ]] && printf '%s\n' "$out" | grep -q "is never declared"; then
	ok "an entry whose reason key is undeclared is refused"
else
	bad "an entry with no declared reason was accepted (exit $rc)"
	printf '%s\n' "$out" | sed 's/^/        /'
fi

# 5d-5e. The record's own declared totals.
d="$(new_repo "$(printf 'Close the %s eval-corpus work' "$MILESTONE")")"
sha="$(git -C "$d" rev-parse HEAD)"
{
	printf 'total entries 4\ntotal commits 1\n'
	printf 'reason pre-gate-history: on main before the gate existed\n'
	printf '%s | milestone-id | pre-gate-history\n' "$sha"
} >"$d/scripts/public-hygiene-accepted-history.txt"
out="$(run_gate "$d")"
rc=$?
if [[ $rc -eq 2 ]] && printf '%s\n' "$out" | grep -q "declares 'total entries 4', parsed 1"; then
	ok "a wrong entry total is refused"
else
	bad "a wrong entry total was accepted (exit $rc)"
	printf '%s\n' "$out" | sed 's/^/        /'
fi

d="$(new_repo "$(printf 'Close the %s eval-corpus work' "$MILESTONE")")"
sha="$(git -C "$d" rev-parse HEAD)"
{
	printf 'reason pre-gate-history: on main before the gate existed\n'
	printf '%s | milestone-id | pre-gate-history\n' "$sha"
} >"$d/scripts/public-hygiene-accepted-history.txt"
out="$(run_gate "$d")"
rc=$?
if [[ $rc -eq 2 ]] && printf '%s\n' "$out" | grep -q "needs a 'total entries"; then
	ok "a record with entries and no declared totals is refused"
else
	bad "a record with no totals was accepted (exit $rc)"
	printf '%s\n' "$out" | sed 's/^/        /'
fi

# ---------------------------------------------------------------------------
# 6. --text: a pull request title and body.
# ---------------------------------------------------------------------------
echo "pull request title and body:"

d="$(new_repo "a clean subject line")"
printf 'Add the eval corpus\n\nCloses %s and lands %s.\n' "$MILESTONE" "$CARDREF" >"$d/pr.txt"
out="$(run_gate "$d" --text pr.txt)"
rc=$?
if [[ $rc -eq 1 ]] && printf '%s\n' "$out" | grep -q "milestone-id" &&
	printf '%s\n' "$out" | grep -q "planning-card-id"; then
	ok "an identifier in the title or body is caught, and every rule reports"
else
	bad "the pull request text was not scanned (exit $rc)"
	printf '%s\n' "$out" | sed 's/^/        /'
fi

d="$(new_repo "a clean subject line")"
printf 'Add the eval corpus\n\nCloses the corpus-expansion work.\n' >"$d/pr.txt"
out="$(run_gate "$d" --text pr.txt)"
rc=$?
if [[ $rc -eq 0 ]]; then
	ok "clean pull request text passes"
else
	bad "clean pull request text was rejected (exit $rc)"
	printf '%s\n' "$out" | sed 's/^/        /'
fi

# The record is for commits that cannot be changed. A pull request body is being
# written now, so nothing in it is ever pre-existing and nothing may be waved
# through -- not even the identifier of a commit the record accepts.
d="$(new_repo "$(printf 'Close the %s eval-corpus work' "$MILESTONE")")"
sha="$(git -C "$d" rev-parse HEAD)"
record "$d" "$sha | milestone-id | pre-gate-history"
printf 'Body mentioning %s\n' "$MILESTONE" >"$d/pr.txt"
out="$(run_gate "$d" --text pr.txt)"
rc=$?
if [[ $rc -eq 1 ]]; then
	ok "the record does not reach pull request text"
else
	bad "an accepted commit's identifier passed in a pull request body (exit $rc)"
	printf '%s\n' "$out" | sed 's/^/        /'
fi

d="$(new_repo "a clean subject line")"
out="$(run_gate "$d" --text no-such-file.txt)"
rc=$?
if [[ $rc -eq 2 ]]; then
	ok "--text with no readable file is a hard error, not a clean pass"
else
	bad "--text accepted a missing file (exit $rc)"
	printf '%s\n' "$out" | sed 's/^/        /'
fi

# ---------------------------------------------------------------------------
# 7-8. The gate refusing to run, rather than reporting clean.
# ---------------------------------------------------------------------------
echo "the gate refusing to run:"

d="$(new_repo "$(printf 'Close the %s eval-corpus work' "$MILESTONE")")"
rm -f "$d/scripts/public-hygiene-rules.txt"
out="$(run_gate "$d")"
rc=$?
if [[ $rc -eq 2 ]] && printf '%s\n' "$out" | grep -q "no rules to run"; then
	ok "a missing rules file is a hard error"
else
	bad "a missing rules file did not fail loudly (exit $rc)"
	printf '%s\n' "$out" | sed 's/^/        /'
fi

d="$(new_repo "$(printf 'Close the %s eval-corpus work' "$MILESTONE")")"
printf '# every rule commented out\n' >"$d/scripts/public-hygiene-rules.txt"
out="$(run_gate "$d")"
rc=$?
if [[ $rc -eq 2 ]] && printf '%s\n' "$out" | grep -q "declares no rules"; then
	ok "a rules file with no rules is a hard error"
else
	bad "an empty rules file reported a verdict (exit $rc)"
	printf '%s\n' "$out" | sed 's/^/        /'
fi

# A pattern git rejects makes `git log` print nothing, which reads exactly like
# "no commit matched" unless the status is checked per rule.
d="$(new_repo "$(printf 'Close the %s eval-corpus work' "$MILESTONE")")"
perl -0pi -e 's/\Qm-[0-9]+\E/m-[0-9]+(/' "$d/scripts/public-hygiene-rules.txt"
out="$(run_gate "$d")"
rc=$?
if [[ $rc -eq 2 ]] && printf '%s\n' "$out" | grep -q "RULE FAILED TO RUN" &&
	printf '%s\n' "$out" | grep -q "milestone-id"; then
	ok "a broken pattern is fatal and names the rule"
else
	bad "a broken pattern did not fail loudly (exit $rc)"
	printf '%s\n' "$out" | sed 's/^/        /'
fi

d="$(new_repo "$(printf 'Close the %s eval-corpus work' "$MILESTONE")")"
out="$(run_gate "$d" --range no-such-ref)"
rc=$?
if [[ $rc -eq 2 ]] && printf '%s\n' "$out" | grep -q "does not resolve"; then
	ok "a range that does not resolve is fatal, not an empty history"
else
	bad "an unresolvable range was treated as clean (exit $rc)"
	printf '%s\n' "$out" | sed 's/^/        /'
fi

# ---------------------------------------------------------------------------
# 9. Matched text in a public CI log.
# ---------------------------------------------------------------------------
echo "redaction:"

d="$(new_repo "$(printf 'Close the %s eval-corpus work' "$MILESTONE")")"
out="$(cd "$d" && CI=true ./scripts/check-history-hygiene.sh 2>&1)"
rc=$?
if [[ $rc -eq 1 ]] && printf '%s\n' "$out" | grep -q "redacted" &&
	! printf '%s\n' "$out" | grep -qF "$MILESTONE"; then
	ok "under CI the commit and the rule are named and the subject is not"
else
	bad "the commit subject was printed into a CI log (exit $rc)"
	printf '%s\n' "$out" | sed 's/^/        /'
fi

printf 'Body mentioning %s\n' "$MILESTONE" >"$d/pr.txt"
out="$(cd "$d" && CI=true ./scripts/check-history-hygiene.sh --text pr.txt 2>&1)"
rc=$?
if [[ $rc -eq 1 ]] && ! printf '%s\n' "$out" | grep -qF "$MILESTONE"; then
	ok "under CI the pull request text is redacted too"
else
	bad "the pull request text was printed into a CI log (exit $rc)"
	printf '%s\n' "$out" | sed 's/^/        /'
fi

out="$(cd "$d" && ./scripts/check-history-hygiene.sh 2>&1)"
rc=$?
if [[ $rc -eq 1 ]] && printf '%s\n' "$out" | grep -qF "$MILESTONE"; then
	ok "off CI the subject is printed, which is how it gets fixed"
else
	bad "the local run did not print the subject (exit $rc)"
	printf '%s\n' "$out" | sed 's/^/        /'
fi

# ---------------------------------------------------------------------------
# The surface this gate's CI step cannot see: a pull request body EDITED after
# the run that scanned it. ci.yml reads the title and body as of the event that
# started its run and says, in a comment, that a later edit is covered by
# .github/workflows/pr-text-hygiene.yml. That sentence was true and nothing
# checked it: delete that workflow, or its `edited` trigger, and the comment
# still reads as coverage while the surface is unscanned.
#
# Read as text because a workflow trigger is a declaration, and refused rather
# than guessed at when the shape is not the one this knows.
# ---------------------------------------------------------------------------
echo "the edited-body surface:"

PR_WORKFLOW="$(cd "$HERE/.." && pwd)/.github/workflows/pr-text-hygiene.yml"
if [[ ! -f "$PR_WORKFLOW" ]]; then
	bad "ci.yml credits pr-text-hygiene.yml with the edited-body surface and there is no such file"
else
	types_lines="$(grep -cE '^[^#]*types: \[' "$PR_WORKFLOW" || true)"
	if [[ "$types_lines" != "1" ]]; then
		bad "expected exactly one \`types: [...]\` line in pr-text-hygiene.yml, found $types_lines"
	elif ! grep -E '^[^#]*types: \[' "$PR_WORKFLOW" | grep -q 'edited'; then
		bad "pr-text-hygiene.yml does not trigger on \`edited\`, so an edited body is scanned by nothing"
	elif ! grep -qE '^[^#]*\./scripts/check-history-hygiene\.sh --text' "$PR_WORKFLOW"; then
		bad "pr-text-hygiene.yml runs something other than this gate's --text mode on the edited body"
	else
		ok "an edited pull request body is re-scanned: pr-text-hygiene.yml triggers on edited and runs --text"
	fi
fi

echo
if [[ $failures -eq 0 ]]; then
	echo "check-history-hygiene-selftest: all assertions passed."
	exit 0
fi
echo "check-history-hygiene-selftest: $failures assertion(s) failed."
exit 1
