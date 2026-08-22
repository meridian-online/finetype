#!/usr/bin/env bash
# Public-hygiene gate: stop private planning references, and absolute
# home-directory paths, reaching this public repository's tracked files.
#
# The shapes it looks for are NOT in this file. They are in
# scripts/public-hygiene-rules.txt, which scripts/check-history-hygiene.sh reads
# too, so the tracked tree and the commit messages are judged by one list.
#
# This repository is public. The planning that drives it is not. A path like
# `<planning>/specs/2026-04-24-amount-variant-generators/spec.yaml` resolves only
# inside the private planning tree — to a reader here it is a dead pointer that
# leaks the shape of private work. An absolute `/Users/<name>/…` path is the
# other half of the same problem: it publishes one machine's layout and it is
# never what a second reader should run.
#
# The two sibling public repositories have run a gate of this shape since
# 2026-07-21. This one is ported from theirs, with two rules of its own and one
# extension to the allowlist format (see ALLOWLIST below).
#
# Usage (no arguments, from anywhere inside the repo):
#
#   ./scripts/check-public-hygiene.sh
#   make hygiene          # runs the self-test first, which is the right order
#
# Exit codes:
#   0  clean
#   1  one or more violations found
#   2  the gate could not run correctly — a broken rule, a malformed allowlist
#      entry, a stale allowlist entry, or a git without PCRE. ALWAYS a hard
#      failure: a gate that cannot run must never look like a gate that passed.
#
# Its own regression test is scripts/check-public-hygiene-selftest.sh, and CI
# runs that FIRST. A hygiene gate fails silently in both directions — it can stop
# matching, or start matching honest prose — so "the gate still works" has to be
# established before "the tree is clean" means anything.
#
# What this covers, and what it does NOT
# --------------------------------------
# COVERED: the content of TRACKED files in the current checkout, via `git grep`.
# Build artifacts, target/, models/ and everything else ignored are invisible by
# construction, so a working tree full of generated files can never make the gate
# cry wolf.
#
# COVERED BY A SIBLING, scripts/check-history-hygiene.sh, which reads the same
# scripts/public-hygiene-rules.txt this file reads:
#   * commit messages, over the whole history reachable from HEAD;
#   * a pull request's title and body, passed to it as text.
# This list used to say those were nobody's job, and it was right for as long as
# it stood: 117 commits on this repository's main carry a private planning
# identifier in their message and none of them were stopped by anything. Neither
# gate can undo that -- see scripts/public-hygiene-accepted-history.txt, which
# records them -- and both stop the next one.
#
# STILL NOT COVERED, and you have to watch these yourself:
#   * review comments on a pull request,
#   * a pull request body EDITED after its last CI run, unless the workflow that
#     scans it also triggers on `edited` (.github/workflows/pr-text-hygiene.yml
#     does; .github/workflows/ci.yml does not),
#   * branch names,
#   * content in git history that is no longer in the current tree and no longer
#     in any reachable commit message -- a blob from a deleted file, say,
#   * issues, releases, and everything else that lives on the forge.
# Those are real leak vectors and nothing in either gate inspects them.
#
# ---------------------------------------------------------------------------
# THE RULE THAT MATTERS MOST HERE
# ---------------------------------------------------------------------------
# A hygiene sweep may rewrite text that DESCRIBES the work. It may never rewrite
# text that IS the data.
#
# This is not a style preference, and the cost of getting it wrong is specific:
# rewriting a key on one side of a join and not the others does not error. The
# join simply matches fewer rows, and every gate that depends on it returns a
# credible-looking figure. A sweep is the most attractive place for that to
# happen, because a sweep is mechanical, high-volume, and reviewed as a diff of
# obviously-harmless path strings.
#
# So: if a matched string is a value some other artefact joins on, or a path a
# program opens, allowlist it and say why. If it is a sentence about the work,
# rewrite the sentence.
#
# ---------------------------------------------------------------------------
# On false positives
# ---------------------------------------------------------------------------
# A gate that cries wolf gets disabled within a week, which is worse than no
# gate. Both patterns below were measured against the real tree before they were
# committed, and scripts/public-hygiene-innocent-strings.txt is a tracked fixture
# of innocent-but-similar-looking strings the gate must stay silent on. That
# fixture is scanned like any other tracked file, so a pattern that starts biting
# real prose turns the gate red on its own fixture.
#
# If a pattern flags something legitimate, FIX THE PATTERN and add the innocent
# string to the fixture. The allowlist is for genuine content that must stay, not
# for papering over a bad regex.
# ---------------------------------------------------------------------------

set -uo pipefail

# Run from the repo root regardless of where the caller invoked us.
REPO_ROOT="$(git rev-parse --show-toplevel)" || {
	echo "check-public-hygiene: not inside a git repository" >&2
	exit 2
}
cd "$REPO_ROOT" || exit 2

ALLOWLIST="scripts/public-hygiene-allowlist.txt"

# The rules use PCRE lookarounds, which git only offers when it was built with
# PCRE. Fail loudly rather than silently matching nothing — a gate that quietly
# no-ops is the failure mode this whole file exists to prevent.
# git grep exits 0 on a match, 1 on no match, and >1 on an error such as "cannot
# use Perl-compatible regexes when not compiled with USE_LIBPCRE".
git grep -qP -e 'zzzz(?<!qqqq)' -- . >/dev/null 2>&1
pcre_rc=$?
if [[ $pcre_rc -gt 1 ]]; then
	echo "check-public-hygiene: this git cannot run PCRE patterns (git grep -P exited $pcre_rc)" >&2
	echo "    install a git built with PCRE, or the gate cannot run" >&2
	exit 2
fi

# ---------------------------------------------------------------------------
# Rules. They are NOT in this file.
#
# scripts/public-hygiene-rules.txt holds them, as "<label>|<PCRE>" lines, and
# scripts/check-history-hygiene.sh reads the same file for commit messages and
# pull request text. They were two lists inside two scripts until a leak used the
# surface only one of them read; a list that lives inside a script is a list the
# other script does not have.
#
# Matches are deduplicated per (label, file, line), so one offending line is
# reported once per rule even when two of a rule's patterns fire on it.
#
# A pattern may never match a `|`: the allowlist format is pipe-separated and
# relies on matched text being unable to collide with the separator.
# ---------------------------------------------------------------------------
RULES_FILE="scripts/public-hygiene-rules.txt"

if [[ ! -f "$RULES_FILE" ]]; then
	echo "check-public-hygiene: $RULES_FILE is missing -- the gate has no rules to run" >&2
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
		echo "check-public-hygiene: $RULES_FILE:$rules_lineno: expected '<label>|<pattern>'" >&2
		echo "    $line" >&2
		bad_rules=1
		continue
	fi
	r_label="${line%%|*}"
	r_pattern="${line#*|}"
	if [[ -z "$r_label" || -z "$r_pattern" ]]; then
		echo "check-public-hygiene: $RULES_FILE:$rules_lineno: label and pattern are both required" >&2
		bad_rules=1
		continue
	fi
	RULES+=("$r_label|$r_pattern")
done <"$RULES_FILE"
if [[ $bad_rules -ne 0 ]]; then
	exit 2
fi
if [[ ${#RULES[@]} -eq 0 ]]; then
	echo "check-public-hygiene: $RULES_FILE declares no rules -- an empty gate reports clean" >&2
	exit 2
fi

# ---------------------------------------------------------------------------
# Matched text in a CI log.
#
# This repository is public and so are its Actions logs. A gate that prints the
# string it caught publishes that string to everyone, which is most of the harm
# the gate exists to prevent -- arriving through the gate. So when CI is set, the
# matched text and the offending source line are replaced by a marker; the file,
# the line number and the rule are still named, which is what a reader needs to
# find it. Run the gate locally, or set HYGIENE_SHOW_MATCHES=1, to see the text.
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

# ---------------------------------------------------------------------------
# ALLOWLIST.
#
# Format, one entry per line, THREE pipe-separated fields:
#
#     <tracked file path or glob> | <exact offending text> | <why this is legitimate>
#
# `|` is the separator precisely because no rule pattern can ever match a `|`,
# so the offending text can never collide with the separator. All three fields
# are required and none may be empty. Anything that does not parse is a hard
# error (exit 2), so the escape hatch cannot be used silently or by accident.
#
# THE PATH FIELD ACCEPTS A GLOB — the one deliberate divergence from the sibling
# repos' version of this file, where it is an exact path. `*` matches any run of
# characters INCLUDING `/`, so `output/*.json` covers a whole subtree. This tree
# has families of frozen run records that carry the same recorded path in the
# same field, and the alternative was forty entries repeating one reason. A glob
# still has to earn its place: it must match something, and it carries a reason
# like any other entry. Prefer the narrowest glob that does the job.
#
# Line numbers are deliberately NOT part of an entry — they drift on every edit.
# Instead every entry must MATCH SOMETHING: an entry that suppresses nothing is
# also a hard error, so a stale allowlist cannot rot into fake coverage.
#
# Blank lines and whole-line `#` comments are ignored.
# ---------------------------------------------------------------------------
declare -a ALLOW_PATH=()
declare -a ALLOW_TEXT=()
declare -a ALLOW_LINENO=()
declare -a ALLOW_HITS=()

trim() {
	local s="$1"
	s="${s#"${s%%[![:space:]]*}"}"
	s="${s%"${s##*[![:space:]]}"}"
	printf '%s' "$s"
}

if [[ -f "$ALLOWLIST" ]]; then
	lineno=0
	bad_allow=0
	while IFS= read -r raw || [[ -n "$raw" ]]; do
		lineno=$((lineno + 1))
		# Strip a trailing CR, in case someone edits on Windows.
		line="${raw%$'\r'}"
		trimmed="$(trim "$line")"
		[[ -z "$trimmed" ]] && continue
		[[ "$trimmed" == \#* ]] && continue

		# Split on `|` into exactly three fields, counting the separators
		# rather than reading into an array: `read -r -a` DISCARDS a trailing
		# empty field, so `path | text |` — an entry whose author could not
		# think of a reason, which is precisely the shape the next check
		# exists to refuse — arrived as two fields and was reported as a
		# malformed line instead of a reasonless one.
		seps="${line//[^|]/}"
		if [[ ${#seps} -ne 2 ]]; then
			echo "check-public-hygiene: $ALLOWLIST:$lineno: expected 3 '|'-separated fields, got $((${#seps} + 1))" >&2
			echo "    $line" >&2
			echo "    format: <tracked/file/path or glob> | <exact offending text> | <why this is legitimate>" >&2
			bad_allow=1
			continue
		fi
		rest="${line#*|}"
		a_path="$(trim "${line%%|*}")"
		a_text="$(trim "${rest%%|*}")"
		a_reason="$(trim "${rest#*|}")"
		if [[ -z "$a_path" || -z "$a_text" || -z "$a_reason" ]]; then
			echo "check-public-hygiene: $ALLOWLIST:$lineno: path, text and explanation are all required" >&2
			echo "    $line" >&2
			bad_allow=1
			continue
		fi
		ALLOW_PATH+=("$a_path")
		ALLOW_TEXT+=("$a_text")
		ALLOW_LINENO+=("$lineno")
		ALLOW_HITS+=(0)
	done <"$ALLOWLIST"
	if [[ $bad_allow -ne 0 ]]; then
		exit 2
	fi
fi

ALLOW_COUNT=${#ALLOW_PATH[@]}

# Returns 0 — and marks the entry used — when this file/text pair is allowlisted.
# The path comparison is an unquoted RHS inside [[ ]], which is bash pattern
# matching: an entry with no metacharacters behaves as an exact match.
is_allowed() {
	local file="$1" text="$2" i
	for ((i = 0; i < ALLOW_COUNT; i++)); do
		# shellcheck disable=SC2053  # glob match is the point
		if [[ "$file" == ${ALLOW_PATH[$i]} && "${ALLOW_TEXT[$i]}" == "$text" ]]; then
			ALLOW_HITS[i]=$((ALLOW_HITS[i] + 1))
			return 0
		fi
	done
	return 1
}

# ---------------------------------------------------------------------------
# ACCEPTED, which is not the same thing as allowlisted, and the difference is the
# whole point of having two files.
#
#   the ALLOWLIST says "this text in this file is LEGITIMATE and stays" -- a
#       dataset value, a path a program opens, a key another artefact joins on;
#   this file says "this is a REAL leak, it is already published, it is recorded
#       rather than silently tolerated, and it may not grow."
#
# A hygiene rule turned on over a tree that already violates it has three
# possible endings. Sweep first, which for the two shapes recorded here is 137
# rewrites across 52 file-and-rule pairs, most of them inside frozen evidence
# records whose wording is somebody's past judgement. Waive them into the
# allowlist, which spells every leaked identifier out in a tracked file and
# permanently declares it fine. Or record them, refuse anything new, and let the
# list only shrink. This is the third.
#
# FORMAT, one entry per line, FIVE pipe-separated fields:
#
#     <tracked file path> | <rule label> | <fingerprint> | <count> | <reason key>
#
# and one or more reason declarations, which are lines of the form
#
#     reason <key>: <why this set is accepted rather than swept>
#
# Every entry names a declared key. One reason usually covers a whole batch, and
# 137 copies of one sentence is not accountability, it is wallpaper -- but an
# entry whose key is not declared is a hard error, so an entry cannot arrive
# without one.
#
# THE FINGERPRINT is the first 12 hex characters of the SHA-256 of the distinct
# matched strings for that file and rule, sorted and newline-joined. It discloses
# nothing that is not already in the named file in plain text; it is here so that
# REPLACING one identifier with another, which leaves the count untouched, is not
# silent. The count catches a repeat of an identifier already there.
#
# THE RATCHET ONLY TIGHTENS. An entry that no longer matches, matches fewer
# times, or matches different text is a HARD ERROR naming the new figures -- not
# a pass. That is what stops the file drifting into a description of a tree that
# has moved on, and it is the same reasoning as the stale-allowlist rule above.
# When you fix one of these, the gate tells you exactly what to write or delete.
# ---------------------------------------------------------------------------
ACCEPTED="scripts/public-hygiene-accepted-tree.txt"

declare -a ACC_PATH=()
declare -a ACC_RULE=()
declare -a ACC_FP=()
declare -a ACC_COUNT=()
declare -a ACC_REASON=()
declare -a ACC_LINENO=()
declare -a ACC_SEEN=()
ACC_REASON_KEYS=""

# The fingerprint hash, from whichever of the two portable tools is present.
# macOS ships `shasum`, Linux ships `sha256sum`, and a machine with neither
# cannot run this gate -- which is exit 2, never a quiet pass.
if command -v sha256sum >/dev/null 2>&1; then
	hash_stdin() { sha256sum | cut -c1-12; }
elif command -v shasum >/dev/null 2>&1; then
	hash_stdin() { shasum -a 256 | cut -c1-12; }
else
	echo "check-public-hygiene: no sha256sum and no shasum -- $ACCEPTED cannot be verified" >&2
	exit 2
fi

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
			rtext="$(trim "${trimmed#*:}")"
			if [[ -z "$rkey" || -z "$rtext" || "$trimmed" != *:* ]]; then
				echo "check-public-hygiene: $ACCEPTED:$lineno: expected 'reason <key>: <text>'" >&2
				echo "    $line" >&2
				bad_acc=1
			fi
			ACC_REASON_KEYS="$ACC_REASON_KEYS$rkey"$'\n'
			continue
		fi

		seps="${line//[^|]/}"
		if [[ ${#seps} -ne 4 ]]; then
			echo "check-public-hygiene: $ACCEPTED:$lineno: expected 5 '|'-separated fields, got $((${#seps} + 1))" >&2
			echo "    $line" >&2
			echo "    format: <path> | <rule label> | <fingerprint> | <count> | <reason key>" >&2
			bad_acc=1
			continue
		fi
		a_f1="$(trim "${line%%|*}")"
		a_rest="${line#*|}"
		a_f2="$(trim "${a_rest%%|*}")"
		a_rest="${a_rest#*|}"
		a_f3="$(trim "${a_rest%%|*}")"
		a_rest="${a_rest#*|}"
		a_f4="$(trim "${a_rest%%|*}")"
		a_f5="$(trim "${a_rest#*|}")"
		if [[ -z "$a_f1" || -z "$a_f2" || -z "$a_f3" || -z "$a_f4" || -z "$a_f5" ]]; then
			echo "check-public-hygiene: $ACCEPTED:$lineno: all five fields are required" >&2
			echo "    $line" >&2
			bad_acc=1
			continue
		fi
		if [[ ! "$a_f4" =~ ^[0-9]+$ ]]; then
			echo "check-public-hygiene: $ACCEPTED:$lineno: count '$a_f4' is not a number" >&2
			bad_acc=1
			continue
		fi
		ACC_PATH+=("$a_f1")
		ACC_RULE+=("$a_f2")
		ACC_FP+=("$a_f3")
		ACC_COUNT+=("$a_f4")
		ACC_REASON+=("$a_f5")
		ACC_LINENO+=("$lineno")
		ACC_SEEN+=(0)
	done <"$ACCEPTED"
	if [[ $bad_acc -ne 0 ]]; then
		exit 2
	fi
fi

ACC_COUNT_N=${#ACC_PATH[@]}

# Every entry's reason key has to have been declared. An entry carrying a key
# nobody wrote down is an entry with no reason, which is the shape this file
# exists to refuse.
bad_acc=0
for ((i = 0; i < ACC_COUNT_N; i++)); do
	case $'\n'"$ACC_REASON_KEYS" in
	*$'\n'"${ACC_REASON[$i]}"$'\n'*) ;;
	*)
		echo "check-public-hygiene: $ACCEPTED:${ACC_LINENO[$i]}: reason key '${ACC_REASON[$i]}' is never declared" >&2
		echo "    add a line: reason ${ACC_REASON[$i]}: <why this is accepted rather than swept>" >&2
		bad_acc=1
		;;
	esac
done
if [[ $bad_acc -ne 0 ]]; then
	exit 2
fi

# ---------------------------------------------------------------------------
# Scan.
# ---------------------------------------------------------------------------
violations=0
allowed=0
accepted=0
drifted=0
# Newline-delimited "<label>:<file>:<line>" keys already reported. A plain string
# rather than an associative array on purpose: macOS still ships bash 3.2, which
# has no `declare -A`, and this gate has to run on a developer's machine as
# readily as on CI.
seen_keys=""

hits="$(mktemp)" || exit 2
errs="$(mktemp)" || exit 2
# Every match that survived the allowlist, as "<label>\t<file>\t<line>\t<text>".
# Collected rather than reported on sight, because whether a match is a violation
# is not decided per match: it is decided by comparing the whole observed set
# against the whole declared set in $ACCEPTED, which is a question no single hit
# can answer.
observed="$(mktemp)" || exit 2
groups="$(mktemp)" || exit 2
trap 'rm -f "$hits" "$errs" "$observed" "$groups"' EXIT

for rule in "${RULES[@]}"; do
	label="${rule%%|*}"
	pattern="${rule#*|}"

	# -I skips binary files, -n gives line numbers, -o prints just the match.
	#
	# The allowlist is excluded from the scan: by construction it quotes the
	# exact text it is waving through, so scanning it would make every entry
	# self-violating. It is the one file with that property — this script is
	# scanned, and its patterns contain no literal offending strings.
	#
	# The exit code is checked BEFORE the output is read, and it is checked per
	# rule. git grep exits 0 on a match, 1 on no match, and >1 on an error — a
	# broken pattern exits 128 and prints nothing to stdout, which without this
	# check reads exactly like "no violations" and lets the gate report clean
	# while blind. Anything above 1 is fatal and names the rule.
	git grep -PIn -o -e "$pattern" -- . ":(exclude)$ALLOWLIST" >"$hits" 2>"$errs"
	grep_rc=$?
	if [[ $grep_rc -gt 1 ]]; then
		echo "check-public-hygiene: RULE FAILED TO RUN — '$label' (git grep exited $grep_rc)" >&2
		echo "    pattern: $pattern" >&2
		while IFS= read -r errline; do
			[[ -n "$errline" ]] && echo "    $errline" >&2
		done <"$errs"
		echo "    the gate cannot report clean while a rule is broken — fix the pattern" >&2
		exit 2
	fi

	while IFS= read -r hit; do
		[[ -z "$hit" ]] && continue
		file="${hit%%:*}"
		rest="${hit#*:}"
		line="${rest%%:*}"
		text="${rest#*:}"

		if is_allowed "$file" "$text"; then
			allowed=$((allowed + 1))
			continue
		fi

		printf '%s\t%s\t%s\t%s\n' "$label" "$file" "$line" "$text" >>"$observed"
	done <"$hits"
done

# ---------------------------------------------------------------------------
# The observed set against the declared set.
#
# One group per (rule, file). Its count is the number of matches, its fingerprint
# the digest of its distinct matched strings. Three outcomes and each is a
# different exit code, because they are three different mistakes:
#
#   observed and NOT declared    a leak this diff introduced           -> exit 1
#   observed and declared, drifted    the record has stopped being true -> exit 2
#   declared and NOT observed    the record has stopped being true      -> exit 2
# ---------------------------------------------------------------------------
cut -f1,2 "$observed" | sort -u >"$groups"

while IFS=$'\t' read -r g_label g_file; do
	[[ -z "$g_label" ]] && continue
	g_count="$(awk -F'\t' -v l="$g_label" -v f="$g_file" '$1==l && $2==f' "$observed" | wc -l | tr -d ' ')"
	# LC_ALL=C, because the digest is over a SORTED list and a locale that
	# collates differently produces a different digest for the same tree. A
	# fingerprint that depends on the machine reading it is not a fingerprint.
	g_fp="$(awk -F'\t' -v l="$g_label" -v f="$g_file" '$1==l && $2==f {print $4}' "$observed" | LC_ALL=C sort -u | hash_stdin)"

	dec=-1
	for ((i = 0; i < ACC_COUNT_N; i++)); do
		if [[ "${ACC_PATH[$i]}" == "$g_file" && "${ACC_RULE[$i]}" == "$g_label" ]]; then
			dec=$i
			break
		fi
	done

	if [[ $dec -ge 0 ]]; then
		ACC_SEEN[dec]=1
		if [[ "${ACC_COUNT[$dec]}" == "$g_count" && "${ACC_FP[$dec]}" == "$g_fp" ]]; then
			accepted=$((accepted + g_count))
			continue
		fi
		echo "check-public-hygiene: $ACCEPTED:${ACC_LINENO[$dec]}: the record no longer describes the tree" >&2
		echo "    $g_file | $g_label" >&2
		echo "    recorded: fingerprint ${ACC_FP[$dec]}, count ${ACC_COUNT[$dec]}" >&2
		echo "    observed: fingerprint $g_fp, count $g_count" >&2
		if [[ "${ACC_COUNT[$dec]}" -lt "$g_count" ]]; then
			echo "    this shape GREW. The accepted set only shrinks; take the new one out." >&2
		else
			echo "    if you removed some, write the observed figures back. If you removed" >&2
			echo "    all of them, delete the entry." >&2
		fi
		drifted=1
		continue
	fi

	# Undeclared: report every distinct line, once per (rule, file, line).
	while IFS=$'\t' read -r _ v_file v_line v_text; do
		key="$g_label:$v_file:$v_line"
		case $'\n'"$seen_keys" in
		*$'\n'"$key"$'\n'*) continue ;;
		esac
		seen_keys="$seen_keys$key"$'\n'

		violations=$((violations + 1))
		printf '%s:%s: %s: %s\n' "$v_file" "$v_line" "$g_label" "$(show "$v_text")"
		# Show the offending source line so the fix is obvious without opening
		# the file. Capped, because one of this tree's data files runs to a
		# megabyte on a line.
		src="$(sed -n "${v_line}p" -- "$v_file" 2>/dev/null | cut -c1-200)"
		[[ -n "$src" ]] && printf '    | %s\n' "$(show "$src")"
	done < <(awk -F'\t' -v l="$g_label" -v f="$g_file" '$1==l && $2==f' "$observed")
done <"$groups"

# A declared entry that matched nothing is the same defect as a stale allowlist
# entry: it looks like coverage and covers nothing. It also means the tree got
# BETTER, which is the direction this file is supposed to move -- so the message
# says "delete it" rather than "something is wrong".
for ((i = 0; i < ACC_COUNT_N; i++)); do
	if [[ ${ACC_SEEN[$i]} -eq 0 ]]; then
		echo "check-public-hygiene: $ACCEPTED:${ACC_LINENO[$i]}: recorded, but nothing matches it now" >&2
		echo "    ${ACC_PATH[$i]} | ${ACC_RULE[$i]}" >&2
		echo "    the file was swept, moved or deleted. Delete this entry -- the record" >&2
		echo "    of a leak that is gone is a hole nobody is watching." >&2
		drifted=1
	fi
done

if [[ $drifted -ne 0 ]]; then
	exit 2
fi

# A stale allowlist entry is a hole nobody is watching: it says "this exact text
# in this exact file is fine", and once the text has moved or gone it suppresses
# nothing while still looking like coverage. Hard error.
stale=0
for ((i = 0; i < ALLOW_COUNT; i++)); do
	if [[ ${ALLOW_HITS[$i]} -eq 0 ]]; then
		echo "check-public-hygiene: $ALLOWLIST:${ALLOW_LINENO[$i]}: stale entry — it suppresses nothing" >&2
		echo "    ${ALLOW_PATH[$i]} | ${ALLOW_TEXT[$i]}" >&2
		stale=1
	fi
done
if [[ $stale -ne 0 ]]; then
	echo "    the text or the file has changed. Correct the entry, or delete it." >&2
	exit 2
fi

if [[ $violations -gt 0 ]]; then
	echo
	echo "check-public-hygiene: FAILED — $violations private reference(s) in tracked files."
	echo
	echo "A path into the private planning repo does not resolve for anyone reading this"
	echo "repository; a planning identifier resolves only inside it and discloses the"
	echo "shape of private work; an absolute home path is never what a second reader"
	echo "should run. Rewrite the sentence: keep the descriptive slug, drop the pointer."
	echo
	echo "    <planning-dir>/specs/2026-04-24-amount-variant-generators/spec.yaml"
	echo " -> spec 2026-04-24-amount-variant-generators"
	echo
	echo "But if the matched string is a VALUE — something a program opens, or a key"
	echo "another artefact joins on — do NOT rewrite it. Rewriting one side of a join"
	echo "does not error; it matches fewer rows and returns a plausible wrong number."
	echo "Add a line to $ALLOWLIST in the form:"
	echo
	echo "    path/or/glob | <exact matched text> | why this is legitimate"
	echo
	echo "$ACCEPTED is NOT the place to put a new one. It records what was already"
	echo "published when the rule was turned on, and it only shrinks."
	exit 1
fi

summary="check-public-hygiene: clean"
[[ $allowed -gt 0 ]] && summary="$summary ($allowed allowlisted match(es)"
[[ $allowed -gt 0 && $accepted -gt 0 ]] && summary="$summary, $accepted recorded as accepted"
[[ $allowed -eq 0 && $accepted -gt 0 ]] && summary="$summary ($accepted recorded as accepted"
[[ $allowed -gt 0 || $accepted -gt 0 ]] && summary="$summary)"
echo "$summary."
exit 0
