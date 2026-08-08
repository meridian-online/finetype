#!/usr/bin/env bash
# Regression test for scripts/check-formula-asset.sh — the check's credibility.
#
# The check itself can only run on a tag: it needs a published release and a
# rewritten tap formula to address. That is exactly the shape that rots, because
# nothing exercises it between releases and a release is the worst moment to
# discover the check stopped reddening. So this file stands a throwaway HTTP
# server in front of throwaway assets, writes formulae against it, and asserts
# on exit codes and on what the messages say — on a pull request, where a
# regression is cheap.
#
# The fixture carries FOUR platform pairs, the number the real formula carries,
# and that is the point rather than a detail. With two pairs, "first" and "last"
# exhaust the file, so a check that read only the first and the last would pass
# a two-pair fixture while skipping half of a four-pair formula. Each of the
# four is given its own wrong-checksum case here.
#
# What is asserted:
#
#   1. the four real url/checksum pairs pass, the output names every url and
#      every digest it fetched, and the summary says four of four — so a check
#      that silently examined fewer than all of them cannot look green;
#   2. a wrong checksum on the FIRST pair fails, and separately on the second,
#      the third and the LAST — no early break, no last-match-only read, and no
#      first-and-last-only read;
#   3. every one of those messages names the url, the declared digest and the
#      actual one;
#   4. a url that 404s fails, and the message says 404 — asserted on the first
#      and on the last pair;
#   5. a checksum that is not a sha256 fails;
#   6. a formula with no pairs, a url with no checksum beside it, and a checksum
#      with no url above it are each hard failures rather than vacuous passes;
#   7. --min-pairs refuses a formula that lost a platform block.
#
# Usage:  ./scripts/check-formula-asset-selftest.sh
#
# CHECK_FORMULA_ASSET may point at a different copy of the check. That is how
# scripts/check-formula-asset-mutations.sh runs this file against deliberately
# broken copies; nothing else should set it.
#
# Exit:   0 all assertions passed, 1 otherwise.
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CHECK="${CHECK_FORMULA_ASSET:-$HERE/check-formula-asset.sh}"

[[ -x "$CHECK" ]] || {
	echo "selftest: $CHECK is missing or not executable" >&2
	exit 1
}

# python3 serves the fixture assets. It is present on the hosted runner this
# workflow uses and on a developer macOS box; if it ever is not, this must be a
# loud failure rather than a skip. A self-test that skips is a self-test that
# reports green while proving nothing.
command -v python3 >/dev/null 2>&1 || {
	echo "selftest: python3 is required to serve the fixture assets" >&2
	exit 1
}

failures=0
ok() { printf '  ok    %s\n' "$1"; }
bad() {
	printf '  FAIL  %s\n' "$1"
	failures=$((failures + 1))
}

TMP="$(mktemp -d)" || exit 1
SERVER_PID=""
cleanup() {
	if [[ -n "$SERVER_PID" ]]; then
		# `wait` after the kill so the shell reaps the job here rather than
		# printing "Terminated" over the last line of the report.
		kill "$SERVER_PID" 2>/dev/null
		wait "$SERVER_PID" 2>/dev/null
	fi
	rm -rf "$TMP"
}
trap cleanup EXIT

# ---------------------------------------------------------------------------
# Fixture assets and a server in front of them.
#
# One per published target, with distinct bytes so no two share a digest — two
# identical assets would let a check that paired the wrong url with the wrong
# checksum pass anyway. The digests below are computed from the files rather
# than pasted: a pasted digest would make case 1 pass for the wrong reason the
# moment the fixture changed.
# ---------------------------------------------------------------------------
mkdir -p "$TMP/assets"
printf 'fixture asset, macos arm leg\n' >"$TMP/assets/tool-macos-arm.tar.gz"
printf 'fixture asset, macos intel leg, a different size on purpose\n' >"$TMP/assets/tool-macos-intel.tar.gz"
printf 'fixture asset, linux arm leg, different again\n' >"$TMP/assets/tool-linux-arm.tar.gz"
printf 'fixture asset, linux x86_64 leg, and different once more so no two digests collide\n' >"$TMP/assets/tool-linux-intel.tar.gz"

digest_of() {
	local out
	if command -v shasum >/dev/null 2>&1; then
		out="$(shasum -a 256 "$1")" || return 1
	else
		out="$(sha256sum "$1")" || return 1
	fi
	printf '%s' "${out%% *}"
}

hash_fixture() {
	local sha
	sha="$(digest_of "$TMP/assets/$1")" || {
		echo "selftest: could not hash the fixture asset $1" >&2
		exit 1
	}
	printf '%s' "$sha"
}

MACOS_ARM_SHA="$(hash_fixture tool-macos-arm.tar.gz)"
MACOS_INTEL_SHA="$(hash_fixture tool-macos-intel.tar.gz)"
LINUX_ARM_SHA="$(hash_fixture tool-linux-arm.tar.gz)"
LINUX_INTEL_SHA="$(hash_fixture tool-linux-intel.tar.gz)"

# A digest of the right shape that belongs to no file here: the last hex digit
# rotated. These are the wrong-digest fixtures, and they are derived rather than
# written so one can never accidentally become the right one.
rotate_last() {
	local sha="$1" last rotated
	last="${sha: -1}"
	case "$last" in
	0) rotated=1 ;;
	*) rotated=0 ;;
	esac
	printf '%s' "${sha:0:63}$rotated"
}

WRONG_MACOS_ARM_SHA="$(rotate_last "$MACOS_ARM_SHA")"
WRONG_MACOS_INTEL_SHA="$(rotate_last "$MACOS_INTEL_SHA")"
WRONG_LINUX_ARM_SHA="$(rotate_last "$LINUX_ARM_SHA")"
WRONG_LINUX_INTEL_SHA="$(rotate_last "$LINUX_INTEL_SHA")"

cat >"$TMP/serve.py" <<'PY'
"""Serve a directory on an ephemeral loopback port and print the port."""
import http.server
import os
import socketserver
import sys

os.chdir(sys.argv[1])


class Quiet(http.server.SimpleHTTPRequestHandler):
    def log_message(self, *args):
        pass


socketserver.TCPServer.allow_reuse_address = True
with socketserver.TCPServer(("127.0.0.1", 0), Quiet) as httpd:
    print(httpd.server_address[1], flush=True)
    httpd.serve_forever()
PY

python3 "$TMP/serve.py" "$TMP/assets" >"$TMP/port" 2>"$TMP/server.err" &
SERVER_PID=$!

PORT=""
waited=0
while [[ $waited -lt 100 ]]; do
	PORT="$(tr -d '[:space:]' <"$TMP/port")"
	[[ -n "$PORT" ]] && break
	sleep 0.1
	waited=$((waited + 1))
done

if [[ -z "$PORT" ]]; then
	echo "selftest: the fixture server never reported a port" >&2
	cat "$TMP/server.err" >&2
	exit 1
fi

BASE="http://127.0.0.1:$PORT"
MACOS_ARM_URL="$BASE/tool-macos-arm.tar.gz"
MACOS_INTEL_URL="$BASE/tool-macos-intel.tar.gz"
LINUX_ARM_URL="$BASE/tool-linux-arm.tar.gz"
LINUX_INTEL_URL="$BASE/tool-linux-intel.tar.gz"
MISSING_URL="$BASE/tool-never-published.tar.gz"

# Prove the server is really answering before any assertion depends on it —
# otherwise every "goes red" case below would pass for the wrong reason.
if ! curl -sSfL -o /dev/null "$MACOS_ARM_URL"; then
	echo "selftest: the fixture server did not serve the first asset" >&2
	cat "$TMP/server.err" >&2
	exit 1
fi

# ---------------------------------------------------------------------------
# Formula fixtures. The shape is the one the release workflow generates: two
# platform blocks with two architecture branches each, every url followed by its
# checksum on the next line.
# ---------------------------------------------------------------------------
formula() { # formula <path> <url1> <sha1> <url2> <sha2> <url3> <sha3> <url4> <sha4>
	cat >"$1" <<RB
class Finetype < Formula
  desc "Fixture"
  homepage "https://example.invalid/"
  license "MIT"
  version "0.0.0"

  depends_on "duckdb"

  on_macos do
    if Hardware::CPU.arm?
      url "$2"
      sha256 "$3"
    else
      url "$4"
      sha256 "$5"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "$6"
      sha256 "$7"
    else
      url "$8"
      sha256 "$9"
    end
  end

  def install
    bin.install "finetype"
  end
end
RB
}

# run <expected-exit> <description> -- <args...>  ; leaves output in $OUT
OUT=""
run() {
	local want="$1" desc="$2"
	shift 3 # want, desc, and the literal --
	OUT="$("$CHECK" "$@" 2>&1)"
	rc=$?
	if [[ $rc -ne $want ]]; then
		bad "$desc: expected exit $want, got $rc"
		printf '%s\n' "$OUT" | sed 's/^/          /'
		return 1
	fi
	ok "$desc (exit $rc)"
	return 0
}

says() { # says <description> <needle>
	if printf '%s' "$OUT" | grep -Fq -- "$2"; then
		ok "$1"
	else
		bad "$1 — output did not contain: $2"
		printf '%s\n' "$OUT" | sed 's/^/          /'
	fi
}

echo "the four real pairs pass, and say what they fetched:"
formula "$TMP/good.rb" \
	"$MACOS_ARM_URL" "$MACOS_ARM_SHA" \
	"$MACOS_INTEL_URL" "$MACOS_INTEL_SHA" \
	"$LINUX_ARM_URL" "$LINUX_ARM_SHA" \
	"$LINUX_INTEL_URL" "$LINUX_INTEL_SHA"
if run 0 "all four platform pairs fetch and match" -- --min-pairs 4 "$TMP/good.rb"; then
	says "the output names the macos arm url" "$MACOS_ARM_URL"
	says "the output names the macos intel url" "$MACOS_INTEL_URL"
	says "the output names the linux arm url" "$LINUX_ARM_URL"
	says "the output names the linux x86_64 url" "$LINUX_INTEL_URL"
	says "the output names the macos arm digest" "$MACOS_ARM_SHA"
	says "the output names the macos intel digest" "$MACOS_INTEL_SHA"
	says "the output names the linux arm digest" "$LINUX_ARM_SHA"
	says "the output names the linux x86_64 digest" "$LINUX_INTEL_SHA"
	# The count, not just the urls. A check that fetched three of four and
	# reported clean would satisfy fewer of the assertions above but this one
	# states the arithmetic outright.
	says "the summary states four of four" "4 of 4 pair(s) fetched and matched"
fi

echo
echo "a wrong checksum goes red on EVERY pair, not just the ones at the ends:"

# Pair 1 of 4. A check that only ever looked at the last pair passes the file
# below; this case is what refuses it.
formula "$TMP/wrong-1.rb" \
	"$MACOS_ARM_URL" "$WRONG_MACOS_ARM_SHA" \
	"$MACOS_INTEL_URL" "$MACOS_INTEL_SHA" \
	"$LINUX_ARM_URL" "$LINUX_ARM_SHA" \
	"$LINUX_INTEL_URL" "$LINUX_INTEL_SHA"
if run 1 "pair 1 of 4 — the macos arm checksum disagrees" -- --min-pairs 4 "$TMP/wrong-1.rb"; then
	says "the message names the url" "$MACOS_ARM_URL"
	says "the message names the declared digest" "$WRONG_MACOS_ARM_SHA"
	says "the message names the actual digest" "$MACOS_ARM_SHA"
fi

# Pair 2 of 4, and pair 3 below. Neither is at an end of the file, so a check
# that read the first pair and the last pair and nothing between them passes
# every other case here and fails these two.
formula "$TMP/wrong-2.rb" \
	"$MACOS_ARM_URL" "$MACOS_ARM_SHA" \
	"$MACOS_INTEL_URL" "$WRONG_MACOS_INTEL_SHA" \
	"$LINUX_ARM_URL" "$LINUX_ARM_SHA" \
	"$LINUX_INTEL_URL" "$LINUX_INTEL_SHA"
if run 1 "pair 2 of 4 — the macos x86_64 checksum disagrees" -- --min-pairs 4 "$TMP/wrong-2.rb"; then
	says "the message names the url" "$MACOS_INTEL_URL"
	says "the message names the declared digest" "$WRONG_MACOS_INTEL_SHA"
	says "the message names the actual digest" "$MACOS_INTEL_SHA"
fi

formula "$TMP/wrong-3.rb" \
	"$MACOS_ARM_URL" "$MACOS_ARM_SHA" \
	"$MACOS_INTEL_URL" "$MACOS_INTEL_SHA" \
	"$LINUX_ARM_URL" "$WRONG_LINUX_ARM_SHA" \
	"$LINUX_INTEL_URL" "$LINUX_INTEL_SHA"
if run 1 "pair 3 of 4 — the linux arm checksum disagrees" -- --min-pairs 4 "$TMP/wrong-3.rb"; then
	says "the message names the url" "$LINUX_ARM_URL"
	says "the message names the declared digest" "$WRONG_LINUX_ARM_SHA"
	says "the message names the actual digest" "$LINUX_ARM_SHA"
fi

# Pair 4 of 4. A check that stopped after the first pair passes the file below;
# this case is what refuses it.
formula "$TMP/wrong-4.rb" \
	"$MACOS_ARM_URL" "$MACOS_ARM_SHA" \
	"$MACOS_INTEL_URL" "$MACOS_INTEL_SHA" \
	"$LINUX_ARM_URL" "$LINUX_ARM_SHA" \
	"$LINUX_INTEL_URL" "$WRONG_LINUX_INTEL_SHA"
if run 1 "pair 4 of 4 — the linux x86_64 checksum disagrees" -- --min-pairs 4 "$TMP/wrong-4.rb"; then
	says "the message names the url" "$LINUX_INTEL_URL"
	says "the message names the declared digest" "$WRONG_LINUX_INTEL_SHA"
	says "the message names the actual digest" "$LINUX_INTEL_SHA"
fi

echo
echo "a url nobody can download goes red, and the status is in the message:"
# The status assertion carries its own weight. A copy of the check with the HTTP
# status test deleted still exits 1 here, because curl writes the 404 body to
# the file and its digest cannot match — so the exit code alone does not
# distinguish "the asset is not there" from "the asset is wrong", and only the
# message does.
formula "$TMP/missing-last.rb" \
	"$MACOS_ARM_URL" "$MACOS_ARM_SHA" \
	"$MACOS_INTEL_URL" "$MACOS_INTEL_SHA" \
	"$LINUX_ARM_URL" "$LINUX_ARM_SHA" \
	"$MISSING_URL" "$LINUX_INTEL_SHA"
if run 1 "the last url resolves to nothing" -- --min-pairs 4 "$TMP/missing-last.rb"; then
	says "the message names the url" "$MISSING_URL"
	says "the message names the status" "HTTP 404"
fi

formula "$TMP/missing-first.rb" \
	"$MISSING_URL" "$MACOS_ARM_SHA" \
	"$MACOS_INTEL_URL" "$MACOS_INTEL_SHA" \
	"$LINUX_ARM_URL" "$LINUX_ARM_SHA" \
	"$LINUX_INTEL_URL" "$LINUX_INTEL_SHA"
if run 1 "the first url resolves to nothing" -- --min-pairs 4 "$TMP/missing-first.rb"; then
	says "the message names the url" "$MISSING_URL"
	says "the message names the status" "HTTP 404"
fi

echo
echo "a checksum that is not a sha256 goes red:"
formula "$TMP/notasha.rb" \
	"$MACOS_ARM_URL" "$MACOS_ARM_SHA" \
	"$MACOS_INTEL_URL" "$MACOS_INTEL_SHA" \
	"$LINUX_ARM_URL" "$LINUX_ARM_SHA" \
	"$LINUX_INTEL_URL" "deadbeef"
run 1 "a truncated checksum is refused" -- --min-pairs 4 "$TMP/notasha.rb"

echo
echo "nothing to check is a hard failure, never a pass:"
cat >"$TMP/empty.rb" <<'RB'
class Finetype < Formula
  desc "A formula with no downloads at all"
  homepage "https://example.invalid/"
end
RB
run 2 "a formula with no url/sha256 pairs" -- "$TMP/empty.rb"

cat >"$TMP/nosha.rb" <<RB
class Finetype < Formula
  on_macos do
    url "$MACOS_ARM_URL"
  end
end
RB
run 2 "a url with no checksum beside it" -- "$TMP/nosha.rb"

cat >"$TMP/nourl.rb" <<RB
class Finetype < Formula
  on_macos do
    sha256 "$MACOS_ARM_SHA"
  end
end
RB
run 2 "a checksum with no url above it" -- "$TMP/nourl.rb"

cat >"$TMP/threepairs.rb" <<RB
class Finetype < Formula
  on_macos do
    if Hardware::CPU.arm?
      url "$MACOS_ARM_URL"
      sha256 "$MACOS_ARM_SHA"
    else
      url "$MACOS_INTEL_URL"
      sha256 "$MACOS_INTEL_SHA"
    end
  end

  on_linux do
    url "$LINUX_ARM_URL"
    sha256 "$LINUX_ARM_SHA"
  end
end
RB
run 2 "three platform pairs where four were required" -- --min-pairs 4 "$TMP/threepairs.rb"
run 0 "the same three, when three is all that was required" -- --min-pairs 3 "$TMP/threepairs.rb"

run 2 "a formula path that does not exist" -- "$TMP/definitely-not-here.rb"

echo
if [[ $failures -ne 0 ]]; then
	echo "check-formula-asset self-test: $failures assertion(s) FAILED" >&2
	exit 1
fi
echo "check-formula-asset self-test: ok"
