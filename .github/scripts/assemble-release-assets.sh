#!/usr/bin/env bash
# Assemble a release's flat asset directory from the artifacts a release.yml
# run downloaded via `actions/download-artifact` with NO `merge-multiple` (see
# release.yml's `release` job for why): five CLI tarballs, five DuckDB
# extension binaries (one per duckdb_arch, each sharing the in-artifact
# filename `finetype.duckdb_extension` -- the reusable workflow's own naming,
# not ours), and the taxonomy catalogue + model manifest.
#
# WHY THIS IS A SEPARATE, SELF-TESTED SCRIPT rather than inline release.yml
# bash: release.yml only runs on a pushed tag, so this logic would otherwise
# be exercised a handful of times a year, at the moment it is least welcome to
# be wrong -- the shape scripts/check-formula-asset.sh's own comment names
# for the same reason. `--self-test` proves it on every pull request instead,
# against a synthetic artifacts/ tree standing in for what download-artifact
# actually produces, run through THIS SCRIPT via a real subprocess -- not a
# reimplementation of its logic that could drift from what release.yml calls.
#
# WHY A PLAIN LIST, NOT A `declare -A` ASSOCIATIVE ARRAY: this script is read
# and its logic proved by hand on a contributor's machine, and macOS ships
# bash 3.2 as `/bin/bash` -- no associative arrays. `declare -A` there fails
# with an "unbound variable" error under `set -u` that names none of this.
#
# USAGE
#   assemble-release-assets.sh --artifacts-dir DIR --output-dir DIR \
#       --tag TAG --extension-duckdb-version VERSION
#   assemble-release-assets.sh --self-test
#
# EXIT CODES
#   0  every expected asset assembled
#   1  a platform's extension binary is missing, or fewer than expected were
#      assembled
#   2  bad usage (missing/unreadable required argument)
set -euo pipefail

# arch:target pairs -- the five platforms MainDistributionPipeline.yml builds,
# named by DuckDB's own duckdb_arch on the left and by the Rust target triple
# release.yml's `build` matrix already uses for the SAME five platforms on the
# right. The right-hand column is what a downloader who knows its own Rust
# target triple and the release tag can compute the extension's filename from,
# without learning DuckDB's separate arch-naming scheme.
ARCH_TARGET_PAIRS="
linux_amd64:x86_64-unknown-linux-gnu
linux_arm64:aarch64-unknown-linux-gnu
osx_amd64:x86_64-apple-darwin
osx_arm64:aarch64-apple-darwin
windows_amd64:x86_64-pc-windows-msvc
"

sha256_of() {
	# shasum is macOS/Linux; sha256sum is available in Git Bash on Windows and
	# on every GitHub-hosted Linux runner. Mirrors release.yml's own
	# "Generate SHA256" step for the CLI tarballs.
	if command -v shasum &>/dev/null; then
		shasum -a 256 "$1" >"$1.sha256"
	else
		sha256sum "$1" >"$1.sha256"
	fi
}

assemble() {
	local artifacts_dir="$1" output_dir="$2" tag="$3" ext_version="$4"

	mkdir -p "$output_dir"

	# CLI tarballs/zips/sha256, and the taxonomy catalogue + model manifest —
	# each artifact directory already holds correctly-named files; just
	# flatten. Skip the extension-build artifact directories: they all share
	# the in-artifact filename `finetype.duckdb_extension`, so flattening them
	# here would silently keep only the last one copied.
	local dir base
	for dir in "$artifacts_dir"/finetype-*; do
		[ -d "$dir" ] || continue
		base="$(basename "$dir")"
		case "$base" in
		finetype-"${ext_version}"-extension-*) continue ;;
		esac
		find "$dir" -maxdepth 1 -type f -exec cp {} "$output_dir/" \;
	done

	local found=0 total=0 arch target src dest
	for pair in $ARCH_TARGET_PAIRS; do
		total=$((total + 1))
		arch="${pair%%:*}"
		target="${pair##*:}"
		src="${artifacts_dir}/finetype-${ext_version}-extension-${arch}/finetype.duckdb_extension"
		if [ ! -f "$src" ]; then
			echo "::error::missing extension artifact for ${arch} at ${src}" >&2
			return 1
		fi
		dest="${output_dir}/finetype-${tag}-${target}.duckdb_extension"
		cp "$src" "$dest"
		sha256_of "$dest"
		found=$((found + 1))
	done
	if [ "$found" -ne "$total" ]; then
		echo "::error::expected ${total} extension binaries, assembled ${found}" >&2
		return 1
	fi

	return 0
}

usage() {
	cat >&2 <<'USAGE'
usage: assemble-release-assets.sh --artifacts-dir DIR --output-dir DIR --tag TAG --extension-duckdb-version VERSION
       assemble-release-assets.sh --self-test
USAGE
}

# ══════════════════════════════════════════════════════════════════════════════
# SELF-TEST — a gate that is only known to pass is not known to detect
# ══════════════════════════════════════════════════════════════════════════════

self_test() {
	local script artifacts_dir output_dir target ext archdir failed=0
	script="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/$(basename "${BASH_SOURCE[0]}")"
	# Not `local`, and the trap below is set once here: a `local` variable
	# stops existing the moment this function returns, but an EXIT trap's
	# command string is re-expanded when the trap actually FIRES — after
	# `self_test` has already returned to `main`. Under `set -u` that reads a
	# `local` variable which no longer exists and dies on the way out, after
	# every case has already printed its verdict. `SELFTEST_TMP` survives
	# because it was never local.
	SELFTEST_TMP="$(mktemp -d)"
	trap 'rm -rf "$SELFTEST_TMP"' EXIT
	tmp="$SELFTEST_TMP"

	build_fixture() {
		# $1: directory to build the fixture under
		local root="$1"
		mkdir -p "$root/artifacts"
		local target ext=tar.gz
		for target in x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu x86_64-apple-darwin aarch64-apple-darwin x86_64-pc-windows-msvc; do
			mkdir -p "$root/artifacts/finetype-${target}"
			ext=tar.gz
			case "$target" in *windows*) ext=zip ;; esac
			echo "cli-binary-for-${target}" >"$root/artifacts/finetype-${target}/finetype-vSELFTEST-${target}.${ext}"
			sha256_of "$root/artifacts/finetype-${target}/finetype-vSELFTEST-${target}.${ext}"
		done
		local arch
		for arch in linux_amd64 linux_arm64 osx_amd64 osx_arm64 windows_amd64; do
			mkdir -p "$root/artifacts/finetype-vTEST-extension-${arch}"
			echo "EXTENSION-CONTENT-FOR-${arch}" >"$root/artifacts/finetype-vTEST-extension-${arch}/finetype.duckdb_extension"
		done
		mkdir -p "$root/artifacts/finetype-taxonomy-catalogue"
		echo '[{"x-finetype-label":"a.b.c","pattern":"x"}]' >"$root/artifacts/finetype-taxonomy-catalogue/taxonomy-schemas.json"
		sha256_of "$root/artifacts/finetype-taxonomy-catalogue/taxonomy-schemas.json"
		echo '{"model":"m2v8m-s43"}' >"$root/artifacts/finetype-taxonomy-catalogue/finetype-model.json"
		sha256_of "$root/artifacts/finetype-taxonomy-catalogue/finetype-model.json"
	}

	# ── Control: a complete tree assembles cleanly, no collisions ──────────
	mkdir -p "$tmp/control"
	build_fixture "$tmp/control"
	if ! (cd "$tmp/control" && bash "$script" --artifacts-dir artifacts --output-dir release-assets --tag vSELFTEST --extension-duckdb-version vTEST >/dev/null); then
		echo "  CONTROL FAILED — a complete artifacts tree did not assemble"
		failed=$((failed + 1))
	else
		local n
		n=$(find "$tmp/control/release-assets" -type f | wc -l | tr -d ' ')
		if [ "$n" != "24" ]; then
			echo "  CONTROL FAILED — expected 24 assembled files, got $n"
			failed=$((failed + 1))
		else
			local ok=true
			for arch in linux_amd64 linux_arm64 osx_amd64 osx_arm64 windows_amd64; do
				case "$arch" in
				linux_amd64) target=x86_64-unknown-linux-gnu ;;
				linux_arm64) target=aarch64-unknown-linux-gnu ;;
				osx_amd64) target=x86_64-apple-darwin ;;
				osx_arm64) target=aarch64-apple-darwin ;;
				windows_amd64) target=x86_64-pc-windows-msvc ;;
				esac
				got="$(cat "$tmp/control/release-assets/finetype-vSELFTEST-${target}.duckdb_extension" 2>/dev/null || echo MISSING)"
				want="EXTENSION-CONTENT-FOR-${arch}"
				if [ "$got" != "$want" ]; then
					echo "  CONTROL FAILED — ${target} carries '${got}', expected '${want}' (cross-platform collision)"
					ok=false
				fi
			done
			if [ "$ok" = true ]; then
				echo "  ok   control: 24 files assembled, each extension binary traced to its own platform"
			else
				failed=$((failed + 1))
			fi
		fi
	fi

	# ── Mutation (AC1's failure mode): one platform's extension is missing ──
	mkdir -p "$tmp/missing"
	build_fixture "$tmp/missing"
	rm -rf "$tmp/missing/artifacts/finetype-vTEST-extension-windows_amd64"
	if (cd "$tmp/missing" && bash "$script" --artifacts-dir artifacts --output-dir release-assets --tag vSELFTEST --extension-duckdb-version vTEST >/dev/null 2>&1); then
		echo "  MISS a platform's extension artifact is missing: assembly succeeded anyway"
		failed=$((failed + 1))
	else
		echo "  ok   a platform's extension artifact is missing: assembly refuses rather than shipping four"
	fi

	# ── Mutation: an artifacts directory with only the catalogue (no CLI, no
	#    extensions at all) still refuses rather than silently succeeding with
	#    zero extension binaries. ──
	mkdir -p "$tmp/empty/artifacts/finetype-taxonomy-catalogue"
	echo '[]' >"$tmp/empty/artifacts/finetype-taxonomy-catalogue/taxonomy-schemas.json"
	if (cd "$tmp/empty" && bash "$script" --artifacts-dir artifacts --output-dir release-assets --tag vSELFTEST --extension-duckdb-version vTEST >/dev/null 2>&1); then
		echo "  MISS an artifacts tree with no extension binaries at all: assembly succeeded anyway"
		failed=$((failed + 1))
	else
		echo "  ok   an artifacts tree with no extension binaries at all: assembly refuses"
	fi

	if [ "$failed" -ne 0 ]; then
		echo ""
		echo "self-test FAILED: $failed case(s) not detected correctly"
		return 1
	fi
	echo ""
	echo "self-test passed"
	return 0
}

# ══════════════════════════════════════════════════════════════════════════════

main() {
	local artifacts_dir="" output_dir="" tag="" ext_version=""
	while [ $# -gt 0 ]; do
		case "$1" in
		--self-test)
			self_test
			exit $?
			;;
		--artifacts-dir)
			artifacts_dir="$2"
			shift 2
			;;
		--output-dir)
			output_dir="$2"
			shift 2
			;;
		--tag)
			tag="$2"
			shift 2
			;;
		--extension-duckdb-version)
			ext_version="$2"
			shift 2
			;;
		*)
			echo "error: unknown argument: $1" >&2
			usage
			exit 2
			;;
		esac
	done

	if [ -z "$artifacts_dir" ] || [ -z "$output_dir" ] || [ -z "$tag" ] || [ -z "$ext_version" ]; then
		usage
		exit 2
	fi
	if [ ! -d "$artifacts_dir" ]; then
		echo "error: artifacts dir not found: $artifacts_dir" >&2
		exit 2
	fi

	if assemble "$artifacts_dir" "$output_dir" "$tag" "$ext_version"; then
		echo "Release assets:"
		ls -la "$output_dir/"
		exit 0
	else
		exit 1
	fi
}

main "$@"
