# The extension's public surface: four pieces

Date: 2026-10-12. Status: design, awaiting review.

## Outcome

A stranger installs finetype from the DuckDB community registry and each example on its published page and in its docs runs as written. The surface is already true on `main`: the install line is `INSTALL finetype FROM community;`, the docs create the tables their examples read, each registry expression returns what it names, `ft_profile` and `ft_detail` are aggregates so they work on a column, and two gates (`scripts/check_sql_examples.py`, `scripts/check_duckdb_catalog.py`) hold it there. This spec covers what is left to make that surface discoverable, honest and self-checking.

Four pieces are ready. A fifth, removing the two agent skills from `.claude/skills/`, waits on another repository carrying the agent-facing reference and is out of scope here.

## Where DuckDB 2.0 stands, measured 2026-10-12

The rename in piece 1 is timed to DuckDB 2.0, so the timing was established first.

- DuckDB's release calendar lists 2.0.0 for 2026-10-21 and 2.0.1 for 2026-11-18, both marked tentative. The `v2.0-cyanoptera` branch is cut and in test; the development post of 2026-09-02 projected the second half of October.
- The v2 C API on that branch carries function documentation as of 2026-10-11 (upstream pull request 26820, merged that day): `duckdb_v2_function_docs_set_description`, `duckdb_v2_function_docs_add_example`, `duckdb_v2_function_docs_add_category`, each marked stable for 2.0.0, plus parameter names through `duckdb_v2_function_signature_add_parameter`. Scalar, aggregate and macro functions are created, documented and registered through that API.
- A v2 extension is a `C_STRUCT` extension whose stamped C API version has major 2; DuckDB then calls `<name>_init_c_api_v2` with a `duckdb_v2_extension_input`. An extension stamped with a v1 C API version keeps loading through the v1 entry point, so the build finetype publishes today loads on 2.0 unchanged.
- The Rust bindings finetype builds with (`duckdb`, `libduckdb-sys`) have a `release-version-main-v2.0` branch at version 1.20000.0, pinned to an alpha from 2026-10-09, before the documentation calls landed. Nothing from it is on crates.io. Its entry-point macro still emits only the v1 entry point.
- The extension build tooling (`duckdb/extension-ci-tools`) has no 2.0 tag yet. Its metadata stamper accepts any ABI type and version string, so stamping a v2 build needs no change there.
- The community registry publishes a 2.0 build from a `repo.ref_next` commit in the extension's descriptor; nine extensions use it today.

Consequence, ruled by Hugh on 2026-10-12: pieces 3, 2 and 4 go now, in that order; piece 1 is specified here and built once 2.0.0 is tagged and the build tooling has a matching tag. After piece 1, `main` builds the extension for DuckDB 2.0 only. The 1.5 builds the registry already serves stay as they are and the README table records them.

## Rulings carried in

- The functions are renamed `finetype_*` for 2.0, for discoverability in `duckdb_functions()`; `ft_*` stays as an alias so an upgrade cannot break a query.
- The move to the v2 C API happens in the same pass as the names, descriptions, examples and categories, as said publicly on issue 141. On 1.5.x the v1 C API has no setter for any of them.
- The taxonomy has no priority dimension: the leaves are released, so a rank distinguishes nothing. The three exceptions get an exclusion that says what it is.
- The site's copy of the taxonomy is another repository's work and follows at the next finetype release.
- "Runs as written" means with no setup the reader does not see.
- A compatibility check is a signal, never a gate.
- Delete rather than restate: where a claim is false, remove it.
- This repository is public: plain words everywhere.

## Piece 3: every model pin is compared on every pull request

### Problem

Five tracked places name the default model, and they have to agree: the CI workflow's `FINETYPE_CI_MODEL`, the release workflow's `FINETYPE_CI_MODEL`, the `DEFAULT_MODEL` constant in the CLI crate's build script, the `models/default` symlink target, and the gitignore line for that model's weights directory. The build-script constant is the pin a `cargo install finetype-cli` binary embeds, and it drifted for a week in June 2026 with CI green, because `.github/scripts/check-ci-model-drift.sh` compares the CI variable against the symlink and nothing else. A step in the release workflow is the wrong place: that workflow fires on the tag, which is too late.

### Design

`check-ci-model-drift.sh` reads all five pins from the tree and requires one name:

| Pin | Where | How it is read |
|---|---|---|
| CI variable | `.github/workflows/ci.yml` | every `FINETYPE_CI_MODEL:` line in the file, whichever `env:` block it sits in; a job that re-pins the model lower down downloads that model |
| release variable | `.github/workflows/release.yml` | the same |
| embedded default | `crates/finetype-cli/build.rs` | the `const DEFAULT_MODEL: &str = "…";` line |
| symlink | `models/default` | `readlink`, then `cat` for the plain-text form a Windows checkout leaves |
| ignored weights | `.gitignore` | a line equal to `models/<name>/` |

The symlink target is the reference the other four are compared against, as today. When `FINETYPE_CI_MODEL` is set in the environment it must also equal the target, as today. A mismatch fails with one message that names every pin that disagrees and its value, so one run shows the whole drift. A pin the script cannot find is a failure, not a skip: a missing constant means the crate embeds nothing, a missing gitignore line means the tree no longer records which model directory CI fills (the weight files themselves are ignored by the `models/**` globs above it, so the per-model line is a record, not the guard), and silence on either is the June failure again. The gitignore line is matched as a literal, not a pattern.

The job that runs the script, `CI model vs symlink drift`, already runs on every pull request and every push to `main`; it is renamed to say what it now checks. The release workflow keeps its variable and gains no check.

The self-test keeps its shape, scratch trees held to an exit status and a message, and materialises the four files it now reads. New cases: a drift planted in each of the four new pins with the rest aligned; a missing gitignore line; a missing constant; a constant whose spacing differs from the canonical form, which must still be read (the constant is read in its double-quoted `&str` form only; any other shape fails closed as a missing pin). The aligned case materialises all five. `.github/gate-self-tests.tsv` lists the script as before; the four read files are not gate code, so they do not route the self-test.

### Out of scope

`.github/scripts/assemble-release-assets.sh` writes the model name into a fixture in its own self-test; that is test data, not a pin.

## Piece 2: the taxonomy has no priority dimension

### Problem

At `main` 392142cb the 251 definitions carry `release_priority` as 144 at 3, 44 at 4, 60 at 5, 2 at 0 and 1 at 2. It does one live job: the three leaves below 3 (`container.object.s_expression` at 2, `identity.person.password` and `representation.text.plain_text` at 0) stay out of `finetype generate`'s default output, because the command's `--priority` defaults to 3, and `scripts/prepare_multibranch_data.py` passes `--priority 0` to put them back for training. Everything else that reads the field (`finetype taxonomy --priority`, `finetype check --priority`, the checker's filter and its `(priority: N)` lines, the `release_priority` key in the taxonomy JSON and the `priority` column in its CSV) ranks released leaves against each other, which distinguishes nothing.

### Design

The field is deleted: from every definition in `labels/definitions_*.yaml` and the one `notes` sentence that mentions it, from the `Definition` struct and `Taxonomy::at_priority`, from the checker's result struct and `at_priority`, from the three CLI flags and the `--priority` text in their help, from the JSON and CSV output of `finetype taxonomy` and `finetype check`, and from every inline YAML test fixture that carries it (in `finetype-core` and in `finetype-model`'s column tests), with the two unit tests that read the field directly. It leaves the tree entirely rather than surviving as inert fixture text. The crate README example and the maintainer skill `add-label`, which lists the field as required, drop it. The agent skill `finetype-cli` drops its three `--priority` rows so piece 5 does not inherit a fresh drift.

In its place, a per-leaf field that says what it does:

```yaml
generate: on_request
```

It appears on the three leaves only. `finetype generate` skips a leaf carrying it unless passed `--all`, and `prepare_multibranch_data.py` passes `--all` where it passed `--priority 0`. Every other consumer of the generator (`finetype check`, the mining scripts, the YDF trainers) is unchanged, because none of them asked for a priority below 3. The reason sits in each leaf's `notes`: plain text and password are catch-all leaves whose synthetic values have no shape, so the default output does not pretend otherwise; the S-expression leaf is recovered at profile time by a rule, not predicted by the model, and its synthetic values exist for the training set's label coverage.

The field is `#[serde(default)]` so the 248 other leaves need nothing. `finetype check` fails a value other than `on_request`, so the field cannot drift into a second priority scale.

The site's build script reads `release_priority` from the taxonomy JSON today. It adapts at the next finetype release, as ruled; this piece changes nothing there.

### Ordering

After piece 3 and before piece 4. It touches the catalog gate's prose counts only if a count names the field, which none does; it touches the docs where the flags are documented.

## Piece 4: a scheduled check re-measures the README version table

### Problem

The table at `README.md` lines 115 to 129 reports, per DuckDB version, what `SELECT ft_version()` returned from the community build on `osx_arm64` on 2026-10-11. Nothing re-measures it after the registry republishes, so each cell goes stale silently, and the 1.5.0 row's advice to run `SELECT ft_version()` raises a Catalog Error on that build, whose functions predate the `ft_` names.

### Design

A workflow, `.github/workflows/weekly-readme-version-table.yml`, scheduled weekly with a manual trigger, on a macOS arm64 runner because that is the platform the table names. It is never triggered by a pull request or a push: a registry outage must not redden a pull request its author cannot fix.

A script, `scripts/check_readme_version_table.py`, does the work so it runs the same locally:

1. Parse the table: one row per DuckDB version, with the cell's outcome class and version string.
2. For each row, download that DuckDB CLI release for the runner's platform, then run exactly the README's two lines, `INSTALL finetype FROM community;` and `LOAD finetype;`, followed by `SELECT ft_version()` and, if that name does not exist, `SELECT finetype_version()`. Every build published so far registers `ft_version`, and the one that predates it answers the second call; after piece 1 both names answer.
3. Classify the outcome into one of four classes and compare it to the cell: a version string (which must match), `INSTALL` failed with an HTTP 404, `INSTALL` served a file the version refused to load, or the function does not exist.
4. List every stable DuckDB release at or above the table's lowest version that has no row. The 1.4 line's facts, which the README asserts in prose under the table today (1.4.4 served, 1.4.0 to 1.4.3 and 1.4.5 not), become rows of the same table, measured before they are committed, so the probe covers them and the prose sentence is deleted rather than left unmeasured.

A stale cell or a missing row writes a report and files it under the label `readme-version-table`: as a comment on the open issue carrying that label when there is one, since a stale cell stays stale until a person commits a new measurement and a fresh issue every Monday would say the same thing, and as a new issue otherwise. A run with nothing to report files nothing. The script never edits the README: a cell is a measurement with a date, and the fix is a new measurement a person commits.

The parser and classifier have a self-test over fixture tables and canned probe outputs, registered in `.github/gate-self-tests.tsv` so a change to the script re-proves it on the pull request that makes it. The measurement itself runs only on the schedule.

The 1.5.0 row's cell is corrected in the same pull request to say what the probe will find: the build predates the `ft_` names, and `SELECT finetype_version()` is the call that answers. The 1.4 rows are added, measured, in the same pull request.

### Ordering

After piece 2, since both edit `README.md`.

## Piece 1: the DuckDB 2.0 pass, specified now, built on the tag

### Problem

`ft_version`, `ft_infer`, `ft_validate_text`, `ft_cast`, `ft_unpack`, `ft_profile`, `ft_detail` and the `ft_validate` table macro answer to a prefix a stranger reading `duckdb_functions()` cannot connect to the extension, and none of them carries a description, an example, a category or a real parameter name, because the v1 C API has nowhere to put one. Issue 141 commits finetype to fixing all of it in one pass on DuckDB 2.0.

### Design

Names. Every function is registered under its `finetype_*` name, and again under its `ft_*` name as an alias: the same callbacks and signature, a description that says `Alias of finetype_x` and nothing more. The two table macros are created twice the same way. The surface after the pass:

| Function | Kind | Parameters | Alias |
|---|---|---|---|
| `finetype_version()` | scalar | none | `ft_version` |
| `finetype_infer(value)` | scalar | `value` | `ft_infer` |
| `finetype_validate_text(value, schema)` | scalar | `value`, `schema` | `ft_validate_text` |
| `finetype_cast(value)` | scalar | `value` | `ft_cast` |
| `finetype_unpack(json)` | scalar | `json` | `ft_unpack` |
| `finetype_profile(value)`, `finetype_profile(value, header)` | aggregate | `value`, `header` | `ft_profile` |
| `finetype_detail(value)`, `finetype_detail(value, header)` | aggregate | `value`, `header` | `ft_detail` |
| `finetype_profile(tbl)` | table macro | `tbl` | `ft_profile` |
| `finetype_validate(tbl, schema)` | table macro | `tbl`, `schema` | `ft_validate` |

Each primary registration carries a one-sentence description taken from the existing docs, one runnable example (a bare expression for scalars and aggregates, a full `SELECT * FROM …` statement for the macros), and the category `finetype`.

Three of the new names were the pre-0.6.23 scalar names, with different shapes: `finetype_validate` was a scalar returning VARCHAR and is now a table macro, `finetype_detail` was a scalar and is now an aggregate, and the bare `finetype` is not reintroduced. A call written for the old scalars fails at bind time now rather than with a Catalog Error. The changelog says so, in the migration table that already carries the 0.6.23 renames.

Registration. The extension crate stops depending on the `duckdb` and `libduckdb-sys` crates and registers through the v2 C API directly: an entry point `finetype_init_c_api_v2` taking `duckdb_v2_extension_input`, a vendored bindings module generated from the 2.0.0 release's `duckdb_v2.h` with the generator command recorded beside it, and a thin safe layer over the handful of calls the extension uses (factory, type, signature, docs, scalar, aggregate, macro, data chunk and vector access). The scalars move off the bindings crate's scalar wrapper onto the v2 scalar function; the two aggregates move from their raw v1 calls to the v2 equivalents with the same shared state; the two macros are created through `duckdb_v2_macro_function_create` with their SQL bodies unchanged, so they carry documentation like the rest. Waiting for a released Rust bindings crate for 2.0 was rejected: it is unreleased, pinned before the documentation calls, and its entry-point macro is v1-shaped. The `spike` cargo feature, `src/spike.rs` and the CI step that compiles it go in the same pass: the spike was kept as compile-time evidence about the bindings crate's table-function support, and the crate no longer uses that crate.

Build and gates. The in-tree Makefile stamps `--abi-type C_STRUCT` with a 2.x C API version; `TARGET_DUCKDB_VERSION` moves to `v2.0.0`. `MainDistributionPipeline.yml` and `weekly-duckdb-compat.yml` move to the 2.0.0 tags of DuckDB and the build tooling. The CI jobs that install a DuckDB CLI install 2.0.0, so `check_duckdb_catalog.py` and `check_sql_examples.py` run on the DuckDB the build targets. The catalog gate's function table in `docs/ARCHITECTURE.md` gains the `finetype_*` rows and an alias column, and its prose counts follow; `check_registry_expressions.py` learns the `finetype_` prefix beside `ft_`.

Docs. `README.md`, `docs/DEVELOPMENT.md`, `docs/ARCHITECTURE.md`, `docs/LOCALE_GUIDE.md`, the extension's module doc and the eval SQL switch to the new names. The README version table is the exception: its rows report builds published before the rename, measured with `SELECT ft_version()`, and stay as measured; the 2.0 build gets a row once it is published.

### What waits on Hugh

The pull request opens once 2.0.0 is tagged and the build tooling has a matching tag. The registry descriptor (its `ref_next`, then its `ref`, and the `extended_description` that teaches the names) is another repository's change, made at release time. A release of this crate against a new DuckDB follows `docs/RELEASE.md` and is not cut without asking.

## Verification, every piece

Before a push: `scripts/check_sql_examples.py`, `scripts/check_duckdb_catalog.py` and `cargo test` as CI runs it, with the totals quoted in the pull request. The public-hygiene check runs on each pull request; the spec, the plan, the branches, the commits and the pull requests use plain words and cite nothing outside this repository and DuckDB's public sources.

## Not in this spec

- Piece 5, removing `.claude/skills/finetype-cli` and `.claude/skills/finetype-pipeline` with `scripts/verify-cli-skill-coverage.sh` and their rows in `scripts/public-hygiene-accepted-tree.txt`. It waits on the other repository's skill.
- The registry descriptor, the site's taxonomy copy, and any release.
