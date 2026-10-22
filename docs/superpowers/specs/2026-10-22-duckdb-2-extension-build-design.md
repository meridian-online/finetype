# finetype 0.6.63: the extension is built and tested for DuckDB 2.0

Short design, small change. Agreed in conversation on 2026-10-22.

## Intent

DuckDB 2.0.0 shipped on 2026-10-21. finetype's own build, CI and release workflow still build and test the DuckDB extension with the 1.5.3 toolchain and a 1.5.3 CLI. Release 0.6.63 moves them to 2.0, so finetype's release carries extension builds for DuckDB 2.0 and arcform's DuckDB 2.0 release can rely on them. The function registration stays on the v1 C API as it is today. Registering functions with names and docs through the v2 C API is a later release, held until the v2 C API has stopped moving between DuckDB patch releases.

## What we already know (measured 2026-10-22, macOS arm64)

- The community registry already serves finetype 0.6.62 for DuckDB v2.0.0, and it loads and runs on a 2.0.0 CLI. That works because the extension uses the stable `C_STRUCT` ABI at a v1.2.0 floor, so one binary loads on 1.5 and 2.0. Release 0.6.63 changes finetype's own toolchain, tests and docs, not whether 2.0 users can install finetype.
- On 2.0.0, `ft_profile(x ORDER BY x DESC)` and `ft_detail(x ORDER BY x)` return results over 200,000 rows in 7 groups. On 1.5.6 the same `ft_profile` query crashes the process (exit 139). The upstream fix (duckdb #26455) is on the 2.0 line only.
- `duckdb/extension-ci-tools` has no `v2.0.0` tag or branch, only the moving `v2.0-cyanoptera` branch (head `78bd1e4a`, 2026-10-20).
- DuckDB 2.0 renamed its CLI release assets, for example `duckdb_cli-linux-amd64.zip` became `duckdb-cli-linux-amd64.tar.gz`, and the macOS build is `duckdb-cli-osx-arm64.tar.gz`.
- The 2.0 CLI switches to an agent output mode when an agent's environment variable is set and stdout is not a terminal, unless an output-mode flag such as `-csv` or `-no-agent` is given.

## Goals

- `release.yml`, `MainDistributionPipeline.yml` and `weekly-duckdb-compat.yml` call extension-ci-tools' `_extension_distribution.yml` at a commit on `v2.0-cyanoptera`, with `duckdb_version: v2.0.0`. `EXTENSION_DUCKDB_VERSION` and the `finetype-v…-extension-*` artifact pattern follow.
- The `extension-ci-tools` submodule moves to the same commit, and `.gitmodules` records the branch.
- Every DuckDB CLI install in `ci.yml` and `release.yml` downloads the v2.0.0 CLI under its new asset name.
- `scripts/check_extension_stamp.py`, which requires a DuckDB CLI install step before each `--load` step and recognises one by `duckdb_cli-`, also recognises 2.0's `duckdb-cli-` name. Its self-test fixtures cover the new name.
- A new SQL test file under `test/sql/` runs `ft_profile` and `ft_detail` under an aggregate-level `ORDER BY`, with and without `GROUP BY`, on enough rows to have crashed 1.5. extension-ci-tools runs it against 2.0 on every distribution build.
- The docs say the ORDER BY limitation is lifted on DuckDB 2.0 and still holds on 1.5: `docs/DEVELOPMENT.md`, the README's `ft_detail` caveat, and the module docs in `crates/finetype-duckdb/src/{lib,profile_agg,column_fn}.rs`. The places that name 1.5.3 as the tested or pinned version (`docs/DEVELOPMENT.md` lines 11 and 141) name 2.0.0.
- The README's registry version table gains a 2.0.0 row (`finetype 0.6.62`, measured 2026-10-22). `scripts/check_readme_version_table.py` downloads 2.0-and-later CLIs under the new asset names.
- Version 0.6.63: `Cargo.toml`'s workspace version moves, and the changelog's `[Unreleased]` section becomes `[0.6.63]`. That section already describes the removal of the MCP server and agent skills. It also gains an entry for the 2.0 toolchain move and the ORDER BY change.

## Non-goals

- Nothing for DuckDB 1.5. The registry entry's 1.5 key stays as it is, we ask the registry maintainers for no 1.5 build, and there is no 1.5 release line.
- No v2 C API registration and no function docs in `duckdb_functions()`.
- No change to the `duckdb` / `libduckdb-sys` crate pins (`=1.10503.1`). The stable C API floor makes the binary independent of the DuckDB it is tested on, and duckdb-rs has not published a 2.0 crate.
- No tag or release in this branch. The branch stops at a ready pull request. Hugh merges it, then tags `v0.6.63`.
- The registry pull request that moves finetype's `ref` to the 0.6.63 commit is drafted for Hugh to open, not opened by the agent.

## Anti-goals

- Moving any 1.5 test or build to the ORDER BY test. The test must not run on a 1.5 DuckDB, where it would crash the test runner.
- A green CI that tested nothing on 2.0. The CI log has to show the 2.0.0 CLI version and the new ORDER BY test running.

## Constraints

- arcform's 0.2.2 release waits on this one and is due on 31 October 2026.
- One cargo build at a time on Hugh's Mac. CI is the full gate.
- finetype is a public repo, so no planning references go in tracked files, commits or the pull request.
- `test/sql/finetype.test` is also what the community registry runs. The ORDER BY test goes in its own file so that a future build of an old ref, or a 1.5 build, never meets it inside the main smoke test.

## Approaches

1. **Move the whole extension toolchain to 2.0, with extension-ci-tools pinned to a commit on `v2.0-cyanoptera`.** Chosen. Release assets are named and tested for v2.0.0, which is what the issue's "done when" requires, and the ORDER BY test runs on 2.0 in every distribution build. The cost is pinning to a branch DuckDB is still moving; a commit pin keeps builds reproducible, and the pin moves to a v2.0.0 tag once one exists.
2. **Wait for an extension-ci-tools v2.0.0 tag.** Dropped: no date for the tag, and the 31 October deadline.
3. **Keep the 1.5.3 toolchain and move only CI's test CLI and the docs.** Dropped: the release would still be named and tested for 1.5.3, which does not meet the "done when", and the weekly compatibility job would test against a toolchain we no longer target.

Hugh picked approach 1 and the version 0.6.63 (not 0.7.0, given finetype's few users).

## Left to the builder

- The exact `v2.0-cyanoptera` commit to pin. Use the branch head on the day of the build and record it in a comment beside each `uses:` line, as the existing pins do.
- Whether the 2.0 branch's `c_api_extensions/base.Makefile` still accepts the root `Makefile`'s contract targets unchanged. If it does not, adapt the root `Makefile` and say what changed in the pull request.
- How many rows the ORDER BY test uses, provided it would have crashed 1.5.6. 200,000 rows in 7 groups did.
- Whether any SQL in `test/sql/finetype.test` or CI's smoke steps behaves differently under 2.0's parser and operator changes. CI shows that; fix what breaks.
