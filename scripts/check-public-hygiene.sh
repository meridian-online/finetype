#!/usr/bin/env bash
# Public-hygiene gate: stop paths into the private planning repo, and absolute
# home-directory paths, reaching this public repository.
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
# NOT COVERED, and you have to watch these yourself:
#   * commit messages,
#   * PR titles, PR descriptions and review comments,
#   * branch names,
#   * anything in git history that is no longer in the current tree,
#   * issues, releases, and everything else that lives on the forge.
# Those are real leak vectors and nothing in this file inspects them.
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
# Rules: "<label>|<PCRE>". Anything git grep -P accepts.
#
# Matches are deduplicated per (label, file, line), so one offending line is
# reported once per rule even when two of a rule's patterns fire on it.
#
# A pattern may never match a `|`: the allowlist format is pipe-separated and
# relies on matched text being unable to collide with the separator.
#
# The patterns are deliberately narrow, and each stops at the first path
# separator after the interesting part. That is what makes the allowlist usable:
# every match of `absolute-home-path` in a data file is the SAME six-to-eleven
# characters, so one entry covers a file with sixty thousand occurrences instead
# of needing sixty thousand entries.
# ---------------------------------------------------------------------------
RULES=(
	# The private planning tree's directory prefix. Anchored on the dot so a
	# bare `orbit/` — a real word, and the name of a retired tool this repo's
	# history mentions — is left alone. Written as a character class so that
	# this file, which is scanned like any other, does not contain the literal
	# prefix and go red on itself.
	'private-planning-path|(?<![A-Za-z0-9_.-])[.]orbit/'

	# An absolute home directory, macOS or Linux, stopped at the user component
	# so the matched text is stable and one allowlist entry can cover a file
	# with thousands of occurrences.
	#
	# Case-SENSITIVE, and the user component must start with a LETTER. Both
	# guards were bought: the first cut was `(?i)` over a digit-friendly class
	# and it matched `/users/862` — a URL path segment — several thousand times
	# across the generated fixtures in eval/datasets/csv. The residual is that a
	# document writing `/users/hugh` in lower case is missed; that is the
	# cheaper mistake, because a gate drowning in generated CSV gets switched
	# off within the week.
	'absolute-home-path|/(?:Users|home)/(?![Ss]hared/)[A-Za-z][A-Za-z0-9._-]*'
)

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
# Scan.
# ---------------------------------------------------------------------------
violations=0
allowed=0
# Newline-delimited "<label>:<file>:<line>" keys already reported. A plain string
# rather than an associative array on purpose: macOS still ships bash 3.2, which
# has no `declare -A`, and this gate has to run on a developer's machine as
# readily as on CI.
seen_keys=""

hits="$(mktemp)" || exit 2
errs="$(mktemp)" || exit 2
trap 'rm -f "$hits" "$errs"' EXIT

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

		key="$label:$file:$line"
		case $'\n'"$seen_keys" in
		*$'\n'"$key"$'\n'*) continue ;;
		esac
		seen_keys="$seen_keys$key"$'\n'

		violations=$((violations + 1))
		printf '%s:%s: %s: %s\n' "$file" "$line" "$label" "$text"
		# Show the offending source line so the fix is obvious without opening
		# the file. Capped, because one of this tree's data files runs to a
		# megabyte on a line.
		src="$(sed -n "${line}p" -- "$file" 2>/dev/null | cut -c1-200)"
		[[ -n "$src" ]] && printf '    | %s\n' "$src"
	done <"$hits"
done

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
	echo "check-public-hygiene: FAILED — $violations private path(s) in tracked files."
	echo
	echo "A path into the private planning repo does not resolve for anyone reading this"
	echo "repository, and an absolute home path is never what a second reader should run."
	echo "Rewrite the sentence: keep the descriptive slug, drop the path."
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
	exit 1
fi

if [[ $allowed -gt 0 ]]; then
	echo "check-public-hygiene: clean ($allowed allowlisted match(es))."
else
	echo "check-public-hygiene: clean."
fi
exit 0
