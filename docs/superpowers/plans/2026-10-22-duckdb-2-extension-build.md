# finetype 0.6.63: DuckDB 2.0 extension build — implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** finetype's CI and release workflow build and test the DuckDB extension for DuckDB 2.0.0, the ORDER BY limitation is documented as lifted on 2.0 with a test, and the tree is versioned 0.6.63.

**Architecture:** Workflow and script changes only; no Rust source changes beyond module doc comments. The extension keeps its stable `C_STRUCT` ABI at the v1.2.0 floor, so the binary is the same kind of artefact as before. Only the toolchain that builds and tests it moves.

**Tech Stack:** GitHub Actions, duckdb/extension-ci-tools reusable workflow, sqllogictest, Python 3 stdlib scripts.

**Spec:** `docs/superpowers/specs/2026-10-22-duckdb-2-extension-build-design.md`

## Global Constraints

- extension-ci-tools pin: commit `78bd1e4abbd543822166d5b07644751ba1fc6e8d` on branch `v2.0-cyanoptera` (head on 2026-10-22). It is used for every `uses: duckdb/extension-ci-tools/...@` ref, every `ci_tools_version:`, and the submodule.
- `duckdb_version: v2.0.0` everywhere it is a literal; `EXTENSION_DUCKDB_VERSION: v2.0.0`; artifact pattern `finetype-v2.0.0-extension-*`.
- DuckDB CLI on Linux CI: `https://github.com/duckdb/duckdb/releases/download/v2.0.0/duckdb-cli-linux-amd64.tar.gz`, extracted with `tar xzf`.
- Every `_extension_distribution.yml` call passes `reduced_ci_mode: 'disabled'`. The 2.0 workflow's default `auto` can skip platforms, and the stamp and release jobs require all five binaries.
- Nothing changes for DuckDB 1.5: no 1.5 key in the registry entry, no 1.5 build.
- Rust crate pins `duckdb`/`libduckdb-sys = "=1.10503.1"` are unchanged.
- Public repo: no planning ids in tracked files, commits or the pull request.
- One cargo build at a time locally.

## Review Focus

1. **A platform silently dropped by the 2.0 workflow.** Expect the distribution run to produce five binaries; the stamp job's count check (`-ne 5`) catches it in CI. Task 1 sets `reduced_ci_mode: 'disabled'`.
2. **The wiring gate passing on the old CLI name only.** `check_extension_stamp.py --release-wiring` must accept a `curl` of `duckdb-cli-` and still refuse a comment naming it. Task 1 updates the self-test's fixture strings to the new pin.
3. **A 2.0 CLI in agent mode.** It switches only when no output-mode flag is given. finetype's shell-out and the README checker's probe both pass `-csv`, so neither changes; a new invocation added in this branch passes `-csv` or `-no-agent` too.
4. **SQL that 2.0 parses differently in `test/sql/finetype.test`.** The distribution build runs it on 2.0. Fix whatever reddens; do not delete assertions to pass.
5. **The ORDER BY test reaching a 1.5 runner.** It lives in its own file, and nothing in this repo runs sqllogictest on 1.5 after Task 1.

---

### Task 1: Move the extension toolchain and CI CLI to DuckDB 2.0

**Files:**
- Modify: `.github/workflows/release.yml` (build-extension `uses`/inputs ~168-171, `EXTENSION_DUCKDB_VERSION` ~264, CLI install ~351, the comments naming the pins)
- Modify: `.github/workflows/MainDistributionPipeline.yml` (~33-39, artifact pattern ~80, the floor comment)
- Modify: `.github/workflows/weekly-duckdb-compat.yml` (both calls ~53-67 and the header comment about the pin)
- Modify: `.github/workflows/ci.yml` (three CLI installs ~440, ~934, ~973)
- Modify: `.gitmodules` (`branch = v2.0-cyanoptera`), `extension-ci-tools` submodule to the pinned commit
- Modify: `scripts/check_extension_stamp.py` (`DUCKDB_CLI_MARK` ~731 and the self-test fixture strings ~2133-2139)

**Interfaces:**
- Produces: `DUCKDB_CLI_MARK = "duckdb-cli-"`. The release job's install step fetches that name, so the wiring rung keeps matching.

- [ ] **Step 1:** Change `DUCKDB_CLI_MARK` to `"duckdb-cli-"`, and change the self-test's `sub(...)` source string to the new install line (`curl -fsSL -o duckdb.tar.gz https://github.com/duckdb/duckdb/releases/download/v2.0.0/duckdb-cli-linux-amd64.tar.gz`) and its replacement comment to name `duckdb-cli-linux-amd64.tar.gz`. Update the prose comments at ~731-740, ~2130 and ~1255 that quote `duckdb_cli-`.
- [ ] **Step 2:** Run `scripts/check_extension_stamp.py --release-wiring`. Expected: exit 1 or 2 naming the missing `duckdb-cli-` fetch, because release.yml still fetches the old name. This is the failing state.
- [ ] **Step 3:** Rewrite each of the four CLI install steps (`ci.yml` ×3, `release.yml` ×1) as `curl -fsSL -o duckdb.tar.gz <v2.0.0 URL>` / `tar xzf duckdb.tar.gz` / `sudo mv duckdb /usr/local/bin/duckdb` / `duckdb --version`.
- [ ] **Step 4:** Move every extension-ci-tools ref, `ci_tools_version` and `duckdb_version: v1.5.3` literal to the pin values, add `reduced_ci_mode: 'disabled'` to all four calls, change `EXTENSION_DUCKDB_VERSION` and the artifact pattern, and rewrite comments that say "1.5.3" or "1.2 through 1.5+" so they describe the 2.0 pin (one line beside each `uses:` naming `v2.0-cyanoptera` and the date). In `weekly-duckdb-compat.yml`, keep `duckdb_version: ${{ needs.resolve-stable.outputs.tag }}` and `main` as they are.
- [ ] **Step 5:** `git -C extension-ci-tools fetch origin v2.0-cyanoptera && git -C extension-ci-tools checkout 78bd1e4abbd543822166d5b07644751ba1fc6e8d`; set `.gitmodules` `branch = v2.0-cyanoptera`.
- [ ] **Step 6:** Verify:
  - `scripts/check_extension_stamp.py --release-wiring` exits 0.
  - `.github/scripts/gate-self-tests.py plan --base main` exits 0.
  - `rg -n "v1\.5\.3|duckdb_cli-" .github/ scripts/check_extension_stamp.py` prints nothing.
  - `make -n build-extension` resolves (no "No rule to make target").
  - `make build-extension` builds, then `$SCRATCH/duckdb2/duckdb -unsigned -no-agent -batch -c "LOAD 'target/release/finetype.duckdb_extension'; SELECT ft_version();"` prints `finetype 0.6.62`. This is the one local cargo build in this task.
- [ ] **Step 7:** Commit `Build and test the DuckDB extension on DuckDB 2.0.0`, then `roborev wait <sha>`.

### Task 2: Test and document ORDER BY on DuckDB 2.0

**Files:**
- Create: `test/sql/finetype_order_by.test`
- Modify: `docs/DEVELOPMENT.md:191` (the ORDER BY paragraph), `:11` and `:141` (tested/pinned version)
- Modify: `README.md:185-186` (the `ft_detail` caveat comment)
- Modify: module docs `crates/finetype-duckdb/src/lib.rs:~21`, `profile_agg.rs:~57-64`, `column_fn.rs:~22`

- [ ] **Step 1:** Write `test/sql/finetype_order_by.test` (header names it as needing DuckDB 2.0, with the upstream fix #26455 and the reason it is a separate file). `require finetype`, then build `CREATE TABLE emails AS SELECT 'user' || i || '@example.com' AS x, i % 7 AS g FROM range(200000) r(i);`, and assert:
  - `SELECT count(*) FROM (SELECT ft_profile(x ORDER BY x DESC) FROM emails GROUP BY g)` → `7`
  - `SELECT ft_profile(x ORDER BY x).type IS NOT NULL FROM emails` → `true`
  - `SELECT json_valid(ft_detail(x ORDER BY x)) FROM emails` → `true`
  - `SELECT count(*) FROM (SELECT ft_detail(x ORDER BY x DESC) FROM emails GROUP BY g)` → `7`
  - No type labels are asserted (model-dependent, as `finetype.test`'s header explains).
- [ ] **Step 2:** Verify that it fails on 1.5 and passes on 2.0 by running the statements through each CLI against the Task 1 artifact. On `duckdb -unsigned -batch` (1.5.6) the GROUP BY query exits 139 or errors. On `$SCRATCH/duckdb2/duckdb -unsigned -no-agent -batch` it prints 7.
- [ ] **Step 3:** Rewrite the ORDER BY paragraph in `docs/DEVELOPMENT.md`. The heading becomes: an aggregate-level `ORDER BY` works on DuckDB 2.0 and crashes on 1.5. Keep the mechanism sentence, then add: fixed upstream in DuckDB 2.0 (duckdb #26455) and not backported to 1.5, and `test/sql/finetype_order_by.test` runs on 2.0 in every distribution build. Change lines 11 and 141 to v2.0.0 and `v2.0-cyanoptera`. The README comment and the three module docs say the same thing in one line each.
- [ ] **Step 4:** Verify: `rg -n -i "order by" README.md docs/DEVELOPMENT.md crates/finetype-duckdb/src` shows no remaining "not supported" wording without the 1.5 qualifier; `cargo fmt --check` passes.
- [ ] **Step 5:** Commit `ft_profile and ft_detail take an aggregate ORDER BY on DuckDB 2.0`, then `roborev wait <sha>`.

### Task 3: README version table measures DuckDB 2.0

**Files:**
- Modify: `scripts/check_readme_version_table.py` (`cli_asset` ~176, `fetch_cli` ~186, `--self-test` cases)
- Modify: `README.md:112-128` (date line and a new top row)

**Interfaces:**
- Produces: `cli_asset(version: str) -> str` and archive extraction chosen per version. For major ≥ 2: `duckdb-cli-osx-universal.tar.gz`, `duckdb-cli-linux-arm64.tar.gz`, `duckdb-cli-linux-amd64.tar.gz`, extracted with `tarfile`; below 2, today's `.zip` names.

- [ ] **Step 1:** Add self-test cases: `cli_asset("2.0.0")` on Darwin → `duckdb-cli-osx-universal.tar.gz`; `cli_asset("1.5.6")` on Darwin → `duckdb_cli-osx-universal.zip`; and a README row `| 2.0.0 | \`finetype 0.6.62\` |` parses to a version outcome.
- [ ] **Step 2:** `python3 scripts/check_readme_version_table.py --self-test`. Expected: FAIL on the new cases.
- [ ] **Step 3:** Implement the per-version asset name and the tar extraction.
- [ ] **Step 4:** `--self-test` passes. Then add the README row `| 2.0.0 | \`finetype 0.6.62\` |` at the top of the table, and change the date line to "Measured 2026-10-22 on `osx_arm64`" only if `--measure` confirms every other cell. If it doesn't, keep the old date and add "2.0.0 measured 2026-10-22" beside the row.
- [ ] **Step 5:** `python3 scripts/check_readme_version_table.py --measure` (network, about 15 CLI downloads, a few minutes). Expected: exit 0, or a report showing only cells the registry has since republished. Record what it says in the commit message.
- [ ] **Step 6:** Commit `README version table: DuckDB 2.0.0 serves finetype 0.6.62`, then `roborev wait <sha>`.

### Task 4: Version 0.6.63 and its changelog

**Files:**
- Modify: `Cargo.toml` (workspace version), `Cargo.lock` (via `cargo update -w`)
- Modify: `CHANGELOG.md` (`## [Unreleased]` → `## [0.6.63] - <date of the PR>` with a fresh empty `## [Unreleased]` above it)
- Modify: `README.md` / `CLAUDE.md` "Version" lines only where they state the current version (`rg -n "0\.6\.62"` decides; the README table's measured 0.6.62 cells stay)

- [ ] **Step 1:** Add a `### Changed` entry: the DuckDB extension is built and tested for DuckDB 2.0.0. It names the toolchain pin and the CLI in CI, says the binary still uses the stable C API at the 1.2 floor, and says nothing changes for 1.5. Add a `### Fixed` (or `Changed`) entry: `ft_profile`/`ft_detail` under an aggregate `ORDER BY` work on DuckDB 2.0, are tested there, and still crash on 1.5. The existing MCP removal entry stays as written.
- [ ] **Step 2:** Bump to 0.6.63 and run `cargo update -w`.
- [ ] **Step 3:** Verify: `python3 scripts/check_release_tag.py --release-rehearsal` exits 0; `python3 scripts/check_release_tag.py --wiring` exits 0; `cargo clippy -p finetype-duckdb -- -D warnings` passes (the pre-push hook runs the full mirror).
- [ ] **Step 4:** Commit `Release 0.6.63: the extension is built and tested for DuckDB 2.0`, then `roborev wait <sha>`.

### Closing the branch

- [ ] Fold: the outcome lives in `docs/DEVELOPMENT.md` (Task 2). Delete this plan and the spec in a final commit.
- [ ] Push, open the pull request, run `roborev review --branch --wait`, fix findings at Medium and above, then close the review.
- [ ] `gh pr checks <pr> --watch`. The distribution build's log must show `v2.0.0`, five binaries, and `finetype_order_by.test` passing.
- [ ] Link the pull request to the Kata issue in Forge (sync, then kata-links).
- [ ] Draft for Hugh, unsent: the `duckdb/community-extensions` change to `extensions/finetype/description.yml` that moves `ref` (and `ref_next`) to the 0.6.63 tag commit and `version: 0.6.63`, leaving `variegata` alone. It is made after the tag exists.
