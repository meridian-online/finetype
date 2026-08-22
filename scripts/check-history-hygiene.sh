#!/usr/bin/env bash
# Public-hygiene gate for the surfaces the tracked-file gate cannot see: COMMIT
# MESSAGES, and a pull request's TITLE and BODY.
#
# scripts/check-public-hygiene.sh reads the content of tracked files. Its own
# header used to say, honestly, that commit messages and pull request text were
# nobody's job. They were the surface the leak used: 117 of the 1,484 commits on
# this repository's main carry a private planning identifier in their message,
# across 5 rule shapes. None of them was stopped by anything, because nothing was
# looking.
#
# THE RULES ARE NOT IN THIS FILE. scripts/public-hygiene-rules.txt holds them and
# both gates read it, so the tracked tree and the history are judged by one list
# and adding a shape to one surface adds it to the other. Two lists inside two
# scripts is how the second surface came to be uncovered in the first place.
#
# Usage:
#
#   ./scripts/check-history-hygiene.sh                 every commit reachable from HEAD
#   ./scripts/check-history-hygiene.sh --range A..B    a commit range
#   ./scripts/check-history-hygiene.sh --text FILE     arbitrary text, no acceptance
#
# Exit codes:
#   0  clean
#   1  one or more violations found
#   2  the gate could not run correctly -- a broken rule, a malformed rules file,
#      a malformed or stale record, or a git without PCRE. ALWAYS a hard failure:
#      a gate that cannot run must never look like a gate that passed.
#
# Its own regression test is scripts/check-history-hygiene-selftest.sh, and CI
# runs that when this file, its rules or its record change.
#
# What this covers, and what it does NOT
# --------------------------------------
# COVERED: the message of every commit reachable from the given range, and any
# text handed to --text. The workflow hands it the pull request title and body.
#
# NOT COVERED, and you have to watch these yourself:
#   * review comments on a pull request,
#   * a pull request body edited after the workflow that reads it last ran,
#   * branch names,
#   * issues, releases, and everything else on the forge.
#
# ---------------------------------------------------------------------------
# WHY A COMMIT MESSAGE CANNOT BE FIXED, AND WHAT THAT MEANS FOR THIS FILE
# ---------------------------------------------------------------------------
# A tracked file can be swept. A published commit message cannot: rewriting it
# rewrites every commit after it, which on a public repository with published
# releases and a downstream package tap breaks every recorded hash, every tag a
# release refers to, and every clone anyone has. So the 117 are recorded in
# scripts/public-hygiene-accepted-history.txt with the reason, and this gate
# refuses the 118th.
#
# That record is a DECLARED SET compared against an OBSERVED SET. It is not a
# pattern matched against prose and it is not a reviewer remembering: the gate
# enumerates what the rules find over the whole history, enumerates what the file
# declares, and reports the difference in both directions. An observed commit
# nobody declared is a violation. A declared commit that no longer matches is a
# hard error, because a record that has stopped being true is a hole that still
# looks like coverage.
# ---------------------------------------------------------------------------

set -uo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)" || {
	echo "check-history-hygiene: not inside a git repository" >&2
	exit 2
}
cd "$REPO_ROOT" || exit 2

RULES_FILE="scripts/public-hygiene-rules.txt"
ACCEPTED="scripts/public-hygiene-accepted-history.txt"

MODE=range
RANGE=HEAD
TEXT_FILE=""
while [[ $# -gt 0 ]]; do
	case "$1" in
	--range)
		MODE=range
		RANGE="${2:-}"
		if [[ -z "$RANGE" ]]; then
			echo "check-history-hygiene: --range needs a value" >&2
			exit 2
		fi
		shift 2
		;;
	--text)
		MODE=text
		TEXT_FILE="${2:-}"
		if [[ -z "$TEXT_FILE" || ! -f "$TEXT_FILE" ]]; then
			echo "check-history-hygiene: --text needs a readable file" >&2
			exit 2
		fi
		shift 2
		;;
	*)
		echo "check-history-hygiene: unknown argument '$1'" >&2
		echo "    usage: $0 [--range <rev-range> | --text <file>]" >&2
		exit 2
		;;
	esac
done

# PCRE, or nothing. A gate that quietly matches nothing is the failure this file
# exists to prevent. git grep exits >1 on an engine it cannot use.
git grep -qP -e 'zzzz(?<!qqqq)' -- . >/dev/null 2>&1
pcre_rc=$?
if [[ $pcre_rc -gt 1 ]]; then
	echo "check-history-hygiene: this git cannot run PCRE patterns (git grep -P exited $pcre_rc)" >&2
	echo "    install a git built with PCRE, or the gate cannot run" >&2
	exit 2
fi

# ---------------------------------------------------------------------------
# The rules, from the file both gates read.
# ---------------------------------------------------------------------------
if [[ ! -f "$RULES_FILE" ]]; then
	echo "check-history-hygiene: $RULES_FILE is missing -- the gate has no rules to run" >&2
	exit 2
fi

declare -a RULES=()
rules_lineno=0
bad_rules=0
while IFS= read -r raw || [[ -n "$raw" ]]; do
	rules_lineno=$((rules_lineno + 1))
	line="${raw%$'\r'}"
	[[ -z "${line//[[:space:]]/}" ]] && continue
	[[ "${line#"${line%%[![:space:]]*}"}" == \#* ]] && continue
	if [[ "$line" != *"|"* ]]; then
		echo "check-history-hygiene: $RULES_FILE:$rules_lineno: expected '<label>|<pattern>'" >&2
		echo "    $line" >&2
		bad_rules=1
		continue
	fi
	r_label="${line%%|*}"
	r_pattern="${line#*|}"
	if [[ -z "$r_label" || -z "$r_pattern" ]]; then
		echo "check-history-hygiene: $RULES_FILE:$rules_lineno: label and pattern are both required" >&2
		bad_rules=1
		continue
	fi
	RULES+=("$r_label|$r_pattern")
done <"$RULES_FILE"
if [[ $bad_rules -ne 0 ]]; then
	exit 2
fi
if [[ ${#RULES[@]} -eq 0 ]]; then
	echo "check-history-hygiene: $RULES_FILE declares no rules -- an empty gate reports clean" >&2
	exit 2
fi

# ---------------------------------------------------------------------------
# Matched text in a public CI log. Same reasoning as the tracked-file gate: this
# repository's Actions logs are world-readable, and a gate that prints the string
# it caught publishes it.
# ---------------------------------------------------------------------------
REDACT=0
if [[ -n "${CI:-}" && "${HYGIENE_SHOW_MATCHES:-}" != "1" ]]; then
	REDACT=1
fi
show() {
	if [[ $REDACT -eq 1 ]]; then
		printf '<redacted: public CI log -- run the gate locally to see it>'
	else
		printf '%s' "$1"
	fi
}

trim() {
	local s="$1"
	s="${s#"${s%%[![:space:]]*}"}"
	s="${s%"${s##*[![:space:]]}"}"
	printf '%s' "$s"
}

# ---------------------------------------------------------------------------
# The declared set.
#
# FORMAT, one entry per line, THREE pipe-separated fields:
#
#     <full 40-character commit sha> | <rule label> | <reason key>
#
# and one or more reason declarations, of the form
#
#     reason <key>: <why this commit is accepted rather than rewritten>
#
# No count and no fingerprint, unlike the tracked-file record: a published commit
# is immutable, so (commit, rule) is exact on its own. No matched text either --
# the message already carries it and writing it here would put it in a second
# place, in a file whose whole point is that the first place cannot be fixed.
#
# An entry naming a key nobody declared is a hard error, so an entry cannot
# arrive without a reason. One reason covers a batch, because 117 copies of one
# sentence is not accountability.
# ---------------------------------------------------------------------------
declare -a ACC_SHA=()
declare -a ACC_RULE=()
declare -a ACC_REASON=()
declare -a ACC_LINENO=()
declare -a ACC_SEEN=()
ACC_REASON_KEYS=""

if [[ -f "$ACCEPTED" ]]; then
	lineno=0
	bad_acc=0
	while IFS= read -r raw || [[ -n "$raw" ]]; do
		lineno=$((lineno + 1))
		line="${raw%$'\r'}"
		trimmed="$(trim "$line")"
		[[ -z "$trimmed" ]] && continue
		[[ "$trimmed" == \#* ]] && continue

		if [[ "$trimmed" == reason\ * ]]; then
			rkey="$(trim "${trimmed#reason }")"
			rkey="${rkey%%:*}"
			rkey="$(trim "$rkey")"
			if [[ "$trimmed" != *:* || -z "$rkey" || -z "$(trim "${trimmed#*:}")" ]]; then
				echo "check-history-hygiene: $ACCEPTED:$lineno: expected 'reason <key>: <text>'" >&2
				echo "    $line" >&2
				bad_acc=1
			fi
			ACC_REASON_KEYS="$ACC_REASON_KEYS$rkey"$'\n'
			continue
		fi

		seps="${line//[^|]/}"
		if [[ ${#seps} -ne 2 ]]; then
			echo "check-history-hygiene: $ACCEPTED:$lineno: expected 3 '|'-separated fields, got $((${#seps} + 1))" >&2
			echo "    $line" >&2
			echo "    format: <commit sha> | <rule label> | <reason key>" >&2
			bad_acc=1
			continue
		fi
		a_rest="${line#*|}"
		a_sha="$(trim "${line%%|*}")"
		a_rule="$(trim "${a_rest%%|*}")"
		a_reason="$(trim "${a_rest#*|}")"
		if [[ -z "$a_sha" || -z "$a_rule" || -z "$a_reason" ]]; then
			echo "check-history-hygiene: $ACCEPTED:$lineno: all three fields are required" >&2
			echo "    $line" >&2
			bad_acc=1
			continue
		fi
		# An abbreviated sha would compare unequal to what git prints and the
		# entry would silently cover nothing, which is the one failure this
		# record cannot afford.
		if [[ ! "$a_sha" =~ ^[0-9a-f]{40}$ ]]; then
			echo "check-history-hygiene: $ACCEPTED:$lineno: '$a_sha' is not a full 40-character sha" >&2
			bad_acc=1
			continue
		fi
		ACC_SHA+=("$a_sha")
		ACC_RULE+=("$a_rule")
		ACC_REASON+=("$a_reason")
		ACC_LINENO+=("$lineno")
		ACC_SEEN+=(0)
	done <"$ACCEPTED"
	if [[ $bad_acc -ne 0 ]]; then
		exit 2
	fi
fi

ACC_N=${#ACC_SHA[@]}

bad_acc=0
for ((i = 0; i < ACC_N; i++)); do
	case $'\n'"$ACC_REASON_KEYS" in
	*$'\n'"${ACC_REASON[$i]}"$'\n'*) ;;
	*)
		echo "check-history-hygiene: $ACCEPTED:${ACC_LINENO[$i]}: reason key '${ACC_REASON[$i]}' is never declared" >&2
		echo "    add a line: reason ${ACC_REASON[$i]}: <why this commit is accepted rather than rewritten>" >&2
		bad_acc=1
		;;
	esac
done
if [[ $bad_acc -ne 0 ]]; then
	exit 2
fi

violations=0
accepted=0
drifted=0

# ---------------------------------------------------------------------------
# --text: arbitrary text, and NOTHING may be accepted.
#
# A pull request body is being written now. There is no such thing as a
# pre-existing one, so the record does not apply and is not consulted.
# ---------------------------------------------------------------------------
if [[ "$MODE" == text ]]; then
	scratch="$(mktemp -d)" || exit 2
	trap 'rm -rf "$scratch"' EXIT
	cp "$TEXT_FILE" "$scratch/text"
	for rule in "${RULES[@]}"; do
		label="${rule%%|*}"
		pattern="${rule#*|}"
		# --no-index so this reads a file that is not in the index, with the
		# same engine and the same patterns as every other scan here.
		out="$(cd "$scratch" && git grep --no-index -PIn -o -e "$pattern" -- text 2>/dev/null)"
		rc=$?
		if [[ $rc -gt 1 ]]; then
			echo "check-history-hygiene: RULE FAILED TO RUN — '$label' (git grep exited $rc)" >&2
			echo "    pattern: $pattern" >&2
			echo "    the gate cannot report clean while a rule is broken — fix the pattern" >&2
			exit 2
		fi
		while IFS= read -r hit; do
			[[ -z "$hit" ]] && continue
			h_rest="${hit#*:}"
			h_line="${h_rest%%:*}"
			h_text="${h_rest#*:}"
			violations=$((violations + 1))
			printf 'line %s: %s: %s\n' "$h_line" "$label" "$(show "$h_text")"
		done <<<"$out"
	done

	if [[ $violations -gt 0 ]]; then
		echo
		echo "check-history-hygiene: FAILED — $violations private planning reference(s) in the text."
		echo
		echo "A pull request title and body are world-readable and permanent. A planning"
		echo "identifier there resolves only inside the private planning tree: it tells a"
		echo "reader of this repository nothing and tells everyone that the work exists."
		echo "Edit the pull request. Keep the descriptive slug, drop the identifier."
		exit 1
	fi
	echo "check-history-hygiene: clean (title and body)."
	exit 0
fi

# ---------------------------------------------------------------------------
# --range: the observed set, from the rules, over every reachable commit.
#
# `git log --perl-regexp --grep` runs the SAME PCRE the tracked-file gate runs,
# against the message of each commit, and prints the ones that match. No temp
# copy of the history, no second regex engine, and no chance of the two gates
# disagreeing about what a pattern means.
# ---------------------------------------------------------------------------
observed="$(mktemp)" || exit 2
errs="$(mktemp)" || exit 2
trap 'rm -f "$observed" "$errs"' EXIT

if ! git rev-parse --quiet --verify "${RANGE%%..*}" >/dev/null 2>&1 &&
	! git rev-parse --quiet --verify "$RANGE" >/dev/null 2>&1; then
	echo "check-history-hygiene: '$RANGE' does not resolve to anything in this repository" >&2
	echo "    a range the gate cannot read is a history the gate did not check" >&2
	exit 2
fi

for rule in "${RULES[@]}"; do
	label="${rule%%|*}"
	pattern="${rule#*|}"

	# The exit code is checked before the output is read, per rule. A pattern
	# git rejects makes `git log` print nothing, which reads exactly like "no
	# commit matched" unless somebody looks at the status.
	matches="$(git log --perl-regexp --grep="$pattern" --format='%H' "$RANGE" 2>"$errs")"
	rc=$?
	if [[ $rc -ne 0 ]]; then
		echo "check-history-hygiene: RULE FAILED TO RUN — '$label' (git log exited $rc)" >&2
		echo "    pattern: $pattern" >&2
		while IFS= read -r errline; do
			[[ -n "$errline" ]] && echo "    $errline" >&2
		done <"$errs"
		echo "    the gate cannot report clean while a rule is broken — fix the pattern" >&2
		exit 2
	fi
	while IFS= read -r sha; do
		[[ -z "$sha" ]] && continue
		printf '%s\t%s\n' "$sha" "$label" >>"$observed"
	done <<<"$matches"
done

# One label may carry several patterns, and a commit matching two of them is one
# fact about that commit, not two. Deduplicated here so the record needs one
# entry per (commit, rule) and the summary count is the number of entries.
LC_ALL=C sort -u -o "$observed" "$observed"

# ---------------------------------------------------------------------------
# The observed set against the declared set, in both directions.
# ---------------------------------------------------------------------------
while IFS=$'\t' read -r o_sha o_label; do
	[[ -z "$o_sha" ]] && continue
	dec=-1
	for ((i = 0; i < ACC_N; i++)); do
		if [[ "${ACC_SHA[$i]}" == "$o_sha" && "${ACC_RULE[$i]}" == "$o_label" ]]; then
			dec=$i
			break
		fi
	done
	if [[ $dec -ge 0 ]]; then
		ACC_SEEN[dec]=1
		accepted=$((accepted + 1))
		continue
	fi
	violations=$((violations + 1))
	subject="$(git log -1 --format='%s' "$o_sha" 2>/dev/null)"
	printf '%s: %s\n' "$o_sha" "$o_label"
	printf '    | %s\n' "$(show "$subject")"
done <"$observed"

# A declared entry that no longer matches. Two different reasons, and only one is
# a defect: the commit may have left this range, which happens on a branch cut
# before the entry was written and is not the gate's business; or it is reachable
# and its message no longer matches, which means the record has stopped being
# true and is a hole that still looks like coverage.
for ((i = 0; i < ACC_N; i++)); do
	[[ ${ACC_SEEN[$i]} -eq 1 ]] && continue
	if ! git merge-base --is-ancestor "${ACC_SHA[$i]}" "${RANGE##*..}" >/dev/null 2>&1; then
		continue
	fi
	echo "check-history-hygiene: $ACCEPTED:${ACC_LINENO[$i]}: recorded, reachable, and no longer matching" >&2
	echo "    ${ACC_SHA[$i]} | ${ACC_RULE[$i]}" >&2
	echo "    a record that has stopped being true is a hole that still looks like" >&2
	echo "    coverage. Delete the entry, or fix the rule it names." >&2
	drifted=1
done

if [[ $drifted -ne 0 ]]; then
	exit 2
fi

if [[ $violations -gt 0 ]]; then
	echo
	echo "check-history-hygiene: FAILED — $violations commit message(s) carry a private planning reference."
	echo
	echo "A commit message on a public repository is permanent and world-readable, and"
	echo "it cannot be swept: rewriting it rewrites every commit after it, which breaks"
	echo "every published release hash and every clone. So fix it BEFORE it is pushed:"
	echo
	echo "    git commit --amend        the tip"
	echo "    git rebase -i <base>      anything behind it, while the branch is yours"
	echo
	echo "Keep the descriptive slug and drop the identifier: a reader of this repository"
	echo "cannot resolve it, and it discloses the shape of work that is not published."
	echo
	echo "$ACCEPTED is NOT the place to put a new one. It records what was already on"
	echo "main when this gate was written, and it only shrinks."
	exit 1
fi

if [[ $accepted -gt 0 ]]; then
	echo "check-history-hygiene: clean ($accepted commit-and-rule pair(s) recorded as accepted)."
else
	echo "check-history-hygiene: clean."
fi
exit 0
