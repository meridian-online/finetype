#!/usr/bin/env bash
# Prove that the assets a Homebrew formula publishes actually exist and hash to
# the checksums written beside them.
#
# The release workflow generates Formula/finetype.rb wholesale and pushes it to
# the tap. Until this script existed, the four checksums in that file came from
# the `.sha256` sidecars downloaded next to the assets, and nothing ever fetched
# the assets themselves — so a URL that resolved to nothing, or a checksum
# written beside the wrong URL, produced a green release and failed later on a
# stranger's machine at `brew install`. The tap has no pull-request CI of its
# own, so nothing downstream looked either.
#
# What this proves and what it does not:
#
#   PROVES  the URL in the formula resolves to a downloadable asset, and that
#           asset hashes to the sha256 written beside it in the same file —
#           the two facts `brew install` establishes before it unpacks
#           anything.
#   DOES NOT PROVE  that the binary inside the asset runs. That needs a host of
#           the target architecture. The formula publishes four: arm and x86_64
#           for macOS and for Linux, and three of the four are produced by a
#           runner that is not that architecture.
#
# Usage:
#
#   scripts/check-formula-asset.sh [--min-pairs N] <path/to/formula.rb>
#
# --min-pairs N (default 1) is the refusal-to-pass-on-zero guard: a formula that
# lost a platform block would otherwise check the blocks that remain and report
# clean. The release workflow passes 4, one per published target.
#
# Exit codes:
#   0  every url/sha256 pair in the file fetched and matched
#   1  an asset did not resolve, or its digest disagreed with the formula
#   2  the check could not run — bad usage, an unreadable or unparseable
#      formula, fewer pairs than --min-pairs, or no sha256 tool on PATH. ALWAYS
#      a hard failure: a check that cannot run must never look like one that
#      passed.
#
# Its own regression test is scripts/check-formula-asset-selftest.sh, and the
# mutation harness that proves the self-test still detects is
# scripts/check-formula-asset-mutations.sh. Both run on every pull request via
# .github/workflows/ci.yml. This script needs published release assets, so it
# can only run on a tag; those two are what keep it honest in between.
set -uo pipefail

usage() {
	cat >&2 <<'USAGE'
usage: check-formula-asset.sh [--min-pairs N] <path/to/formula.rb>

Fetches each url in the formula and compares its sha256 to the checksum written
beside it. Exit 0 clean, 1 a missing or mismatched asset, 2 the check could not
run.
USAGE
}

MIN_PAIRS=1

while [[ $# -gt 0 ]]; do
	case "$1" in
	--min-pairs)
		if [[ $# -lt 2 ]]; then
			echo "check-formula-asset: --min-pairs needs a value" >&2
			exit 2
		fi
		MIN_PAIRS="$2"
		shift 2
		;;
	--min-pairs=*)
		MIN_PAIRS="${1#*=}"
		shift
		;;
	-h | --help)
		usage
		exit 2
		;;
	--)
		shift
		break
		;;
	-*)
		echo "check-formula-asset: unknown option: $1" >&2
		usage
		exit 2
		;;
	*)
		break
		;;
	esac
done

if [[ ! "$MIN_PAIRS" =~ ^[0-9]+$ ]] || [[ "$MIN_PAIRS" -lt 1 ]]; then
	echo "check-formula-asset: --min-pairs must be a positive integer, got '$MIN_PAIRS'" >&2
	exit 2
fi

if [[ $# -ne 1 ]]; then
	usage
	exit 2
fi

FORMULA="$1"

if [[ ! -f "$FORMULA" ]]; then
	echo "check-formula-asset: no such formula: $FORMULA" >&2
	exit 2
fi

command -v curl >/dev/null 2>&1 || {
	echo "check-formula-asset: curl is not on PATH" >&2
	exit 2
}

# shasum on macOS, sha256sum on most Linux images; both are present on the
# GitHub-hosted runners this runs on. Neither is assumed.
digest_of() {
	local file="$1" out
	if command -v shasum >/dev/null 2>&1; then
		out="$(shasum -a 256 "$file")" || return 1
	elif command -v sha256sum >/dev/null 2>&1; then
		out="$(sha256sum "$file")" || return 1
	else
		return 2
	fi
	# The tool's own exit code is read above; the field split cannot mask it.
	printf '%s' "${out%% *}"
}

if ! command -v shasum >/dev/null 2>&1 && ! command -v sha256sum >/dev/null 2>&1; then
	echo "check-formula-asset: neither shasum nor sha256sum is on PATH" >&2
	exit 2
fi

# ---------------------------------------------------------------------------
# Parse the formula into (url, sha256) pairs, in file order.
#
# A Homebrew formula writes the checksum on the line after the url it belongs
# to, inside whichever platform branch selected it. Pairing by adjacency is what
# makes the check address the exact string a `brew install` would resolve —
# reconstructing the URL from the tag instead would re-derive the same mistake
# the generator made and agree with it. The generator IS the thing under
# suspicion here: it composes all four urls and all four checksums from the tag
# and the sidecars, and it is the only writer of the file.
# ---------------------------------------------------------------------------
declare -a URLS=()
declare -a SHAS=()
declare -a LINES=()

pending_url=""
pending_line=0
lineno=0

while IFS= read -r line || [[ -n "$line" ]]; do
	lineno=$((lineno + 1))
	if [[ "$line" =~ ^[[:space:]]*url[[:space:]]+\"([^\"]*)\" ]]; then
		if [[ -n "$pending_url" ]]; then
			echo "check-formula-asset: $FORMULA:$pending_line: url has no sha256 beside it" >&2
			echo "    $pending_url" >&2
			exit 2
		fi
		pending_url="${BASH_REMATCH[1]}"
		pending_line=$lineno
	elif [[ "$line" =~ ^[[:space:]]*sha256[[:space:]]+\"([^\"]*)\" ]]; then
		if [[ -z "$pending_url" ]]; then
			echo "check-formula-asset: $FORMULA:$lineno: sha256 with no url above it" >&2
			exit 2
		fi
		URLS+=("$pending_url")
		SHAS+=("${BASH_REMATCH[1]}")
		LINES+=("$pending_line")
		pending_url=""
	fi
done <"$FORMULA"

if [[ -n "$pending_url" ]]; then
	echo "check-formula-asset: $FORMULA:$pending_line: url has no sha256 beside it" >&2
	echo "    $pending_url" >&2
	exit 2
fi

pairs=${#URLS[@]}

if [[ $pairs -lt $MIN_PAIRS ]]; then
	echo "check-formula-asset: $FORMULA carries $pairs url/sha256 pair(s), expected at least $MIN_PAIRS" >&2
	echo "    a formula that lost a platform block must not report clean on the blocks that remain" >&2
	exit 2
fi

TMP="$(mktemp -d)" || exit 2
trap 'rm -rf "$TMP"' EXIT

failures=0

echo "check-formula-asset: $FORMULA — $pairs url/sha256 pair(s)"

i=0
while [[ $i -lt $pairs ]]; do
	url="${URLS[$i]}"
	want="${SHAS[$i]}"
	at="${LINES[$i]}"
	i=$((i + 1))

	if [[ ! "$want" =~ ^[0-9a-f]{64}$ ]]; then
		echo "FAIL  $FORMULA:$at: the sha256 beside this url is not 64 lowercase hex digits" >&2
		echo "        url:      $url" >&2
		echo "        declared: $want" >&2
		failures=$((failures + 1))
		continue
	fi

	asset="$TMP/asset.$i"
	# No -f. Without it curl reports the status line instead of exiting 22 on
	# every 4xx alike, so a red run names 404 (the asset is not there) apart
	# from 403 (it is there and unreadable). --retry does not retry a 404.
	code="$(curl -sSL --retry 3 --retry-delay 2 --max-time 600 \
		-o "$asset" -w '%{http_code}' "$url" 2>"$TMP/curl.err")"
	rc=$?

	if [[ $rc -ne 0 ]]; then
		echo "FAIL  the formula's url could not be fetched (curl exit $rc)" >&2
		echo "        url: $url" >&2
		sed 's/^/        /' "$TMP/curl.err" >&2
		failures=$((failures + 1))
		continue
	fi

	if [[ "$code" != "200" ]]; then
		echo "FAIL  the formula's url does not resolve to an asset (HTTP $code)" >&2
		echo "        url: $url" >&2
		echo "        the checksum beside it was published against a file nobody can download" >&2
		failures=$((failures + 1))
		continue
	fi

	got="$(digest_of "$asset")"
	drc=$?
	if [[ $drc -ne 0 ]] || [[ ! "$got" =~ ^[0-9a-f]{64}$ ]]; then
		echo "check-formula-asset: could not hash the asset downloaded from $url" >&2
		exit 2
	fi

	bytes="$(wc -c <"$asset" | tr -d '[:space:]')"

	if [[ "$got" != "$want" ]]; then
		echo "FAIL  the asset does not match the checksum published beside it" >&2
		echo "        url:      $url" >&2
		echo "        declared: $want  ($FORMULA:$((at + 1)))" >&2
		echo "        actual:   $got  ($bytes bytes fetched)" >&2
		failures=$((failures + 1))
		continue
	fi

	echo "  ok  $url"
	echo "      $got  ($bytes bytes)"
done

if [[ $failures -ne 0 ]]; then
	echo "check-formula-asset: $failures of $pairs pair(s) failed — do not publish this formula" >&2
	exit 1
fi

echo "check-formula-asset: $pairs of $pairs pair(s) fetched and matched"
exit 0
