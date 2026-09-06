#!/usr/bin/env bash
# Assemble a release's flat asset directory from the artifacts a release.yml
# run downloaded via `actions/download-artifact` with NO `merge-multiple` (see
# release.yml's `release` job for why): four CLI tarballs and one zip, five
# DuckDB extension binaries (one per duckdb_arch, each sharing the in-artifact
# filename `finetype.duckdb_extension` -- the reusable workflow's own naming,
# not ours), and the taxonomy catalogue + model manifest.
#
# WHY THIS IS A SEPARATE, SELF-TESTED SCRIPT rather than inline release.yml
# bash: release.yml only runs on a pushed tag, so this logic would otherwise
# be exercised a handful of times a year, at the moment it is least welcome to
# be wrong -- the shape scripts/check-formula-asset.sh's own comment names
# for the same reason. `--self-test` proves it instead, against a synthetic
# artifacts/ tree standing in for what download-artifact actually produces,
# run through THIS SCRIPT via a real subprocess -- not a reimplementation of
# its logic that could drift from what release.yml calls.
#
# WHEN THAT PROOF RUNS, precisely: on a pull request whose diff touches this
# script or .github/workflows/release.yml, and on any pull request that
# changes the routing itself. NOT on every pull request -- the row for this
# gate in .github/gate-self-tests.tsv is what selects it, and a diff that
# touches neither path skips it. release.yml is in that row because it is this
# script's only caller and the likeliest place to break the property.
#
# WHY A PLAIN LIST, NOT A `declare -A` ASSOCIATIVE ARRAY: this script is read
# and its logic proved by hand on a contributor's machine, and macOS ships
# bash 3.2 as `/bin/bash` -- no associative arrays. `declare -A` there prints
# `declare: -A: invalid option` and exits 2.
#
# USAGE
#   assemble-release-assets.sh --artifacts-dir DIR --output-dir DIR \
#       --tag TAG --extension-duckdb-version VERSION
#   assemble-release-assets.sh --self-test
#
# EXIT CODES
#   0  every expected asset assembled
#   1  a platform's extension binary is missing from the artifacts tree
#   2  bad usage (missing/unreadable required argument)
#   3  the artifacts tree holds an extension binary for a duckdb_arch that
#      ARCH_TARGET_PAIRS below does not name, which would otherwise be dropped
#      from the release in silence
#   4  the taxonomy catalogue or the model manifest did not reach the output
#      directory, so the release would carry the extension and no type source
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
	# Writes `<hash>  <BARE FILENAME>` beside $1. shasum records the path
	# exactly AS GIVEN, so hashing "$output_dir/finetype-..." would write a
	# sidecar naming a path that does not exist in the directory a person
	# downloaded the release into, and `shasum -a 256 -c` there would report
	# "FAILED open or read" and exit 1. Running from inside the file's own
	# directory is what makes the sidecar say the bare filename -- the form the
	# CLI tarball sidecars this repo already publishes use, which is what the
	# sibling release notes tell a downloader to run `-c` against.
	#
	# shasum is macOS/Linux; sha256sum is available in Git Bash on Windows and
	# on the GitHub-hosted Linux runners. Mirrors release.yml's own
	# "Generate SHA256" step for the CLI tarballs, which gets the same form by
	# running in the archive's own directory.
	local dir base
	dir="$(dirname "$1")"
	base="$(basename "$1")"
	if command -v shasum &>/dev/null; then
		(cd "$dir" && shasum -a 256 "$base" >"$base.sha256")
	else
		(cd "$dir" && sha256sum "$base" >"$base.sha256")
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

	# The catalogue and the model manifest are the assets a consumer resolves a
	# tag FOR, and the flatten above cannot tell an artifact that arrived empty
	# from one that arrived full: `actions/upload-artifact` warns rather than
	# fails when its path matches nothing, so an upload whose glob stopped
	# matching leaves a green job and an empty directory. Without this, the
	# release would carry five extension binaries and no type source, and
	# `softprops/action-gh-release` would skip the unmatched entries with a
	# warning. Named files, not a count: a count is satisfied by any four
	# files.
	local required missing_assets=""
	for required in taxonomy-schemas.json taxonomy-schemas.json.sha256 finetype-model.json finetype-model.json.sha256; do
		[ -f "${output_dir}/${required}" ] || missing_assets="${missing_assets} ${required}"
	done
	if [ -n "$missing_assets" ]; then
		echo "::error::the taxonomy catalogue artifact did not deliver:${missing_assets} -- check the taxonomy-catalogue job's upload paths" >&2
		return 4
	fi

	# An extension artifact for a duckdb_arch this script does not name is a
	# FAILURE, not a silence. ARCH_TARGET_PAIRS above and release.yml's
	# `exclude_archs` input are two independent lists with nothing tying them
	# together: they agree today, but a platform that upstream starts building
	# -- or one dropped from `exclude_archs` -- would otherwise be built,
	# downloaded, and then left off the release while every step reported
	# success. What a downloader gets is a 404 for a platform the project
	# believes it ships. Checked BEFORE the mapping is used, because the
	# question is whether the mapping is still complete.
	local arch target src dest pair uncovered="" covered
	for dir in "$artifacts_dir"/finetype-"${ext_version}"-extension-*; do
		[ -d "$dir" ] || continue
		base="$(basename "$dir")"
		arch="${base#finetype-${ext_version}-extension-}"
		covered=false
		for pair in $ARCH_TARGET_PAIRS; do
			if [ "${pair%%:*}" = "$arch" ]; then
				covered=true
				break
			fi
		done
		[ "$covered" = true ] || uncovered="${uncovered} ${arch}"
	done
	if [ -n "$uncovered" ]; then
		echo "::error::extension artifact for duckdb_arch not named by ARCH_TARGET_PAIRS:${uncovered} -- add each with its Rust target triple, or exclude it in release.yml's exclude_archs" >&2
		return 3
	fi

	for pair in $ARCH_TARGET_PAIRS; do
		arch="${pair%%:*}"
		target="${pair##*:}"
		src="${artifacts_dir}/finetype-${ext_version}-extension-${arch}/finetype.duckdb_extension"
		if [ ! -f "$src" ]; then
			echo "::error::missing extension artifact for ${arch} at ${src}" >&2
			return 1
		fi
		# A zero-byte binary is an artifact that arrived and a platform that
		# did not. It copies, checksums and uploads exactly like a real one,
		# and the first thing that notices is a downloader's LOAD failing.
		if [ ! -s "$src" ]; then
			echo "::error::empty extension artifact for ${arch} at ${src}" >&2
			return 1
		fi
		dest="${output_dir}/finetype-${tag}-${target}.duckdb_extension"
		cp "$src" "$dest"
		sha256_of "$dest"
	done

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

	# Deliberately NOT sha256_of: the fixture stands in for the sidecars
	# release.yml's own `build` and `taxonomy-catalogue` jobs upload, which are
	# written by a different code path (their inline "Generate SHA256" steps,
	# running in the file's own directory). A fixture built by the function
	# under test would inherit that function's defects and could not detect
	# them -- the sidecar-form check below would pass against a broken
	# sha256_of because the expectation had moved with it.
	fixture_sha256() {
		local dir base
		dir="$(dirname "$1")"
		base="$(basename "$1")"
		if command -v shasum &>/dev/null; then
			(cd "$dir" && shasum -a 256 "$base" >"$base.sha256")
		else
			(cd "$dir" && sha256sum "$base" >"$base.sha256")
		fi
	}

	# Verifies one sidecar the way a downloader does: from inside the directory
	# holding the asset, with the checker the release notes name.
	verify_sidecar() {
		if command -v shasum &>/dev/null; then
			shasum -a 256 -c "$1"
		else
			sha256sum -c "$1"
		fi
	}

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
			fixture_sha256 "$root/artifacts/finetype-${target}/finetype-vSELFTEST-${target}.${ext}"
		done
		local arch
		for arch in linux_amd64 linux_arm64 osx_amd64 osx_arm64 windows_amd64; do
			mkdir -p "$root/artifacts/finetype-vTEST-extension-${arch}"
			echo "EXTENSION-CONTENT-FOR-${arch}" >"$root/artifacts/finetype-vTEST-extension-${arch}/finetype.duckdb_extension"
		done
		mkdir -p "$root/artifacts/finetype-taxonomy-catalogue"
		echo '[{"x-finetype-label":"a.b.c","pattern":"x"}]' >"$root/artifacts/finetype-taxonomy-catalogue/taxonomy-schemas.json"
		fixture_sha256 "$root/artifacts/finetype-taxonomy-catalogue/taxonomy-schemas.json"
		echo '{"model":"m2v8m-s43"}' >"$root/artifacts/finetype-taxonomy-catalogue/finetype-model.json"
		fixture_sha256 "$root/artifacts/finetype-taxonomy-catalogue/finetype-model.json"
	}

	# Runs the script under test, echoes its exit code and leaves its output in
	# $root/assembly.log, so a case can assert WHICH refusal fired and for
	# which reason rather than only that one did.
	run_assembly() {
		local root="$1" rc=0
		(cd "$root" && bash "$script" --artifacts-dir artifacts --output-dir release-assets --tag vSELFTEST --extension-duckdb-version vTEST) >"$root/assembly.log" 2>&1 || rc=$?
		echo "$rc"
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

	# ── Control, second property: what each sidecar SAYS ───────────────────
	# Counting files and comparing binary payloads (above) cannot see a wrong
	# filename inside a sidecar, which is how five extension sidecars naming
	# `release-assets/finetype-...` reached a review while every check was
	# green. A sidecar is only useful if the command the release notes name
	# works in the directory a person downloaded the assets into, so that is
	# the assertion: `-c` from inside `release-assets`, plus the recorded name
	# read back and compared to the bare filename.
	local sidecar sidecar_name recorded sidecars=0
	local sidecars_ok=true
	for sidecar in "$tmp/control/release-assets"/*.sha256; do
		[ -f "$sidecar" ] || continue
		sidecars=$((sidecars + 1))
		sidecar_name="$(basename "$sidecar")"
		recorded="$(awk 'NR==1 {print $2}' "$sidecar")"
		if [ "$recorded" != "${sidecar_name%.sha256}" ]; then
			echo "  MISS ${sidecar_name} records '${recorded}', expected the bare filename '${sidecar_name%.sha256}'"
			sidecars_ok=false
		fi
		if ! (cd "$tmp/control/release-assets" && verify_sidecar "$sidecar_name" >/dev/null 2>&1); then
			echo "  MISS ${sidecar_name} does not verify with -c from inside the download directory"
			sidecars_ok=false
		fi
	done
	if [ "$sidecars" -ne 12 ]; then
		echo "  MISS expected 12 sidecars in the assembled directory, found ${sidecars}"
		sidecars_ok=false
	fi
	if [ "$sidecars_ok" = true ]; then
		echo "  ok   every one of the ${sidecars} sidecars names its bare filename and verifies with -c from the download directory"
	else
		failed=$((failed + 1))
	fi

	# ── Mutation (AC1's failure mode): one platform's extension is missing ──
	local rc
	mkdir -p "$tmp/missing"
	build_fixture "$tmp/missing"
	rm -rf "$tmp/missing/artifacts/finetype-vTEST-extension-windows_amd64"
	rc="$(run_assembly "$tmp/missing")"
	if [ "$rc" != "1" ]; then
		echo "  MISS a platform's extension artifact is missing: exit ${rc}, expected 1"
		failed=$((failed + 1))
	else
		echo "  ok   a platform's extension artifact is missing: assembly refuses with exit 1 rather than shipping four"
	fi

	# ── Mutation: an artifacts directory with only the catalogue (no CLI, no
	#    extensions at all) still refuses rather than silently succeeding with
	#    zero extension binaries. ──
	#    The catalogue artifact is COMPLETE here on purpose: the exit-4 rung
	#    runs first, so an incomplete one would have this case pass for the
	#    wrong reason and leave the extension rung unexercised.
	mkdir -p "$tmp/empty/artifacts/finetype-taxonomy-catalogue"
	echo '[]' >"$tmp/empty/artifacts/finetype-taxonomy-catalogue/taxonomy-schemas.json"
	fixture_sha256 "$tmp/empty/artifacts/finetype-taxonomy-catalogue/taxonomy-schemas.json"
	echo '{"model":"m2v8m-s43"}' >"$tmp/empty/artifacts/finetype-taxonomy-catalogue/finetype-model.json"
	fixture_sha256 "$tmp/empty/artifacts/finetype-taxonomy-catalogue/finetype-model.json"
	rc="$(run_assembly "$tmp/empty")"
	if [ "$rc" != "1" ]; then
		echo "  MISS an artifacts tree with no extension binaries at all: exit ${rc}, expected 1"
		failed=$((failed + 1))
	else
		echo "  ok   an artifacts tree with no extension binaries at all: assembly refuses with exit 1"
	fi

	# ── Mutation: a SIXTH platform, built and downloaded, that this script's
	#    ARCH_TARGET_PAIRS does not name. The complete-tree control above
	#    passes with it present unless the script looks for it, so without this
	#    case a platform added upstream ships as a 404. Asserted as exit 3 and
	#    not merely non-zero: exit 1 here would mean the missing-platform rung
	#    caught it for the wrong reason, which is a different defect wearing
	#    this one's result. ──
	mkdir -p "$tmp/sixth"
	build_fixture "$tmp/sixth"
	mkdir -p "$tmp/sixth/artifacts/finetype-vTEST-extension-linux_amd64_musl"
	echo "EXTENSION-CONTENT-FOR-linux_amd64_musl" >"$tmp/sixth/artifacts/finetype-vTEST-extension-linux_amd64_musl/finetype.duckdb_extension"
	rc="$(run_assembly "$tmp/sixth")"
	if [ "$rc" != "3" ]; then
		echo "  MISS an extension artifact for a platform ARCH_TARGET_PAIRS does not name: exit ${rc}, expected 3"
		failed=$((failed + 1))
	elif ! grep -q "linux_amd64_musl" "$tmp/sixth/assembly.log"; then
		echo "  WRONG an extension artifact for a platform ARCH_TARGET_PAIRS does not name: refused with exit 3 without naming the platform"
		failed=$((failed + 1))
	else
		echo "  ok   an extension artifact for a platform ARCH_TARGET_PAIRS does not name: assembly refuses with exit 3 and names it rather than dropping it"
	fi

	# ── Mutation: a platform's extension binary arrives ZERO BYTES. It copies,
	#    checksums and uploads exactly like a real one, so nothing before a
	#    downloader's LOAD would notice. The reason is asserted as well as the
	#    rung: "empty" and "missing" are both exit 1, and a case that reads
	#    only the code cannot tell which one fired. ──
	mkdir -p "$tmp/emptybin"
	build_fixture "$tmp/emptybin"
	: >"$tmp/emptybin/artifacts/finetype-vTEST-extension-osx_arm64/finetype.duckdb_extension"
	rc="$(run_assembly "$tmp/emptybin")"
	if [ "$rc" != "1" ]; then
		echo "  MISS a zero-byte extension binary: exit ${rc}, expected 1"
		failed=$((failed + 1))
	elif ! grep -q "empty extension artifact for osx_arm64" "$tmp/emptybin/assembly.log"; then
		echo "  WRONG a zero-byte extension binary: refused with exit 1 but not as empty — $(cat "$tmp/emptybin/assembly.log")"
		failed=$((failed + 1))
	else
		echo "  ok   a zero-byte extension binary: assembly refuses with exit 1 and names it empty, not missing"
	fi

	# ── Mutation: the taxonomy-catalogue artifact does not deliver, the shape
	#    `actions/upload-artifact` produces when its path stops matching (it
	#    warns rather than fails). Every extension binary is present, so the
	#    release would carry five platforms of extension and no type source —
	#    AC2 and AC5 unmet on a green run.
	#
	#    ONE CASE PER ASSET, each removing only that file. Removing all four at
	#    once proves the rung fires and not which assets it requires: with a
	#    single case, shortening the required list to two still passed. AC5
	#    rides on finetype-model.json specifically, so the case that pins AC5
	#    has to be the one where finetype-model.json is the only thing gone. ──
	local absent
	for absent in taxonomy-schemas.json taxonomy-schemas.json.sha256 finetype-model.json finetype-model.json.sha256; do
		rm -rf "$tmp/nocatalogue"
		mkdir -p "$tmp/nocatalogue"
		build_fixture "$tmp/nocatalogue"
		rm -f "$tmp/nocatalogue/artifacts/finetype-taxonomy-catalogue/${absent}"
		rc="$(run_assembly "$tmp/nocatalogue")"
		if [ "$rc" != "4" ]; then
			echo "  MISS the catalogue artifact delivered everything but ${absent}: exit ${rc}, expected 4"
			failed=$((failed + 1))
		elif ! grep -q "${absent}" "$tmp/nocatalogue/assembly.log"; then
			echo "  WRONG the catalogue artifact delivered everything but ${absent}: refused with exit 4 without naming it"
			failed=$((failed + 1))
		else
			echo "  ok   the catalogue artifact delivered everything but ${absent}: assembly refuses with exit 4 and names it"
		fi
	done

	# ── Mutation: the catalogue artifact arrives under a directory name the
	#    flatten's `finetype-*` glob does not match, so all four files exist in
	#    the artifacts tree and none of them is copied. This is the case the
	#    rung is FOR, and the only one that separates "present in the output
	#    directory" from "present somewhere under artifacts/": replacing the
	#    rung's `[ -f "${output_dir}/${required}" ]` with a `find` over
	#    "$artifacts_dir" leaves the four cases above green, because they
	#    delete the file and both forms agree on a file that is gone. Here the
	#    forms disagree, and the wrong one ships a release with no type source.
	mkdir -p "$tmp/misnamed"
	build_fixture "$tmp/misnamed"
	mv "$tmp/misnamed/artifacts/finetype-taxonomy-catalogue" "$tmp/misnamed/artifacts/taxonomy-catalogue"
	rc="$(run_assembly "$tmp/misnamed")"
	if [ "$rc" != "4" ]; then
		echo "  MISS the catalogue artifact arrived under a name the flatten does not match: exit ${rc}, expected 4"
		failed=$((failed + 1))
	elif ! grep -q "taxonomy-schemas.json" "$tmp/misnamed/assembly.log"; then
		echo "  WRONG the catalogue artifact arrived under a name the flatten does not match: refused with exit 4 without naming the absent assets"
		failed=$((failed + 1))
	else
		echo "  ok   the catalogue artifact arrived under a name the flatten does not match: assembly refuses with exit 4 rather than reading it where it was not copied"
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

	# Propagate assemble's own code rather than collapsing every refusal to 1:
	# "a platform is missing" (1) and "a platform is present that this script
	# cannot name" (3) are different defects with different fixes, and a caller
	# -- including the self-test below -- that cannot tell them apart cannot
	# tell whether the rung it meant to exercise is the one that fired.
	local status=0
	assemble "$artifacts_dir" "$output_dir" "$tag" "$ext_version" || status=$?
	if [ "$status" -ne 0 ]; then
		exit "$status"
	fi
	echo "Release assets:"
	ls -la "$output_dir/"
	exit 0
}

main "$@"
