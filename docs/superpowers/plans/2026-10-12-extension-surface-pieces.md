# Extension surface pieces 3, 2 and 4: implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship the three pieces of the extension-surface spec that need nothing from DuckDB 2.0, one pull request each, in the order piece 3, piece 2, piece 4.

**Architecture:** Piece 3 widens an existing shell gate from one pin pair to five pins and extends its scratch-tree self-test. Piece 2 deletes a field from the taxonomy's YAML, struct, CLI and outputs, and replaces its one live job with a per-leaf `generate: on_request` field read by the generator. Piece 4 adds a stdlib Python script that parses the README version table, probes each DuckDB release's community build, compares, and a weekly macOS workflow that runs it and files an issue.

**Tech Stack:** bash (gates under `.github/scripts/`), Rust (`finetype-core`, `finetype-cli`), Python 3 stdlib (`scripts/`), GitHub Actions.

**Spec:** `docs/superpowers/specs/2026-10-12-extension-surface-pieces-design.md`

## Global Constraints

- This repository is public: plain words in code, comments, commits, branches and pull requests; cite nothing outside this repository and DuckDB's public sources. `scripts/check-public-hygiene.sh` and the pull-request text check run on every pull request.
- Before each push: `scripts/check_sql_examples.py`, `scripts/check_duckdb_catalog.py`, and `cargo test` as CI runs it (`cargo test --workspace`); quote the totals in the pull request.
- Pre-push hook runs `cargo fmt --check` and `cargo clippy -D warnings`; never bypass it.
- One pull request per piece. Piece 3 ships on branch `model-pin-check` (this branch, which carries the spec and this plan); piece 2 on `no-priority-dimension` from `main` after piece 3 merges; piece 4 on `readme-version-table-check` after piece 2 merges.
- A compatibility check is a signal, never a gate: the piece 4 workflow has no `pull_request` or `push` trigger.
- The README version table's existing cells stay as measured; piece 4 corrects only the 1.5.0 cell's advice.
- Delete rather than restate: a removed field leaves no comment saying it was removed.
- No merging: say the pull request is ready, with its link, and stop.

## Review Focus

1. Piece 3: a Windows checkout writes `models/default` as a text file with CRLF; the script must still read it (`check-ci-model-drift.sh --self-test` cases `drift_plain_text_symlink` and `aligned_plain_text_symlink`).
2. Piece 3: `build.rs` may be reformatted by `cargo fmt` with different spacing around `=`; the constant must still be read (`check-ci-model-drift.sh --self-test` case `aligned_build_rs_spacing`).
3. Piece 2: a `generate:` value other than `on_request` must fail the taxonomy load, so the field cannot become a second scale (Task B1 test `generate_policy_rejects_unknown_value`).
4. Piece 4: a README table row whose cell is prose with no version string and no known failure phrase must be reported as unreadable rather than silently matched (Task C1 case `cell_unreadable`).
5. Piece 4: a DuckDB release the GitHub API lists as a prerelease must not count as a missing row (Task C1 case `prerelease_is_not_missing`).

---

# Part A: piece 3, every model pin compared on every pull request

**Done on branch `model-pin-check` (pull request 154).** The implementation is the committed file, not a copy here: `.github/scripts/check-ci-model-drift.sh` reads the five pins and carries its own self-test of 25 scratch-tree cases (`--self-test`), the `Model pins agree` job in `.github/workflows/ci.yml` runs both, and `docs/RELEASE.md` names the five pins in the promotion flow. The review findings that shaped it are on the pull request's commits: a literal (not regex) gitignore match, CRLF tolerance on the gitignore, every `FINETYPE_CI_MODEL:` line in a workflow counted, a trailing YAML comment stripped, an unterminated last line kept, and the whole gitignore read so `pipefail` cannot misreport. Any later change to the script starts from the file and its self-test, under the same test-first rule as the parts below.

---

# Part B: piece 2, the taxonomy has no priority dimension

Branch: `no-priority-dimension` from `main` after Part A merges. Pull request title: `Taxonomy: delete release_priority; three leaves generate on request`.

### Task B1: the core crate drops the field and gains `generate: on_request`

**Files:**
- Modify: `crates/finetype-core/src/taxonomy.rs:424-427` (field), `:700-706` (`at_priority`), `:1394-1398` (test), and the `release_priority:` lines in its test YAML
- Modify: `crates/finetype-core/src/generator/mod.rs:65-80`, `:108-120`
- Modify: `crates/finetype-core/src/checker.rs:33-34`, `:174-180`, `:239-247`, `:313`, `:471-474`, `:519-522`, and its test YAML
- Modify: `crates/finetype-core/src/generator/tests.rs:13`, `:442`
- Modify: `crates/finetype-core/src/validation_veto.rs`, `crates/finetype-core/src/validator.rs`, `crates/finetype-core/tests/precise_audit.rs`, `crates/finetype-model/src/column/tests/{attractor,datetime,geo,identity,sharpen,substance}.rs` (test YAML lines only)
- Modify: `crates/finetype-core/README.md:34`

**Interfaces:**
- Produces: `Definition.generate: Option<GeneratePolicy>`; `GeneratePolicy::OnRequest` (serde value `on_request`); `Taxonomy::generation_set(&self, all: bool) -> Vec<(&String, &Definition)>`; `Generator::generate_all(&mut self, all: bool, samples_per_label: usize)`; `Generator::generate_all_localized(&mut self, all: bool, samples_per_label: usize)`. `DefinitionCheckResult` loses `release_priority`; `CheckReport::at_priority` is gone.

- [ ] **Step 1: Write the failing tests in `taxonomy.rs`**

Replace `test_at_priority` with:

```rust
    const GENERATE_YAML: &str = r#"
a.b.default_leaf:
  title: "Default"
  tier: [VARCHAR]
a.b.on_request_leaf:
  title: "On request"
  tier: [VARCHAR]
  generate: on_request
"#;

    #[test]
    fn generation_set_skips_on_request_leaves_unless_asked() {
        let taxonomy = Taxonomy::from_yaml(GENERATE_YAML).unwrap();
        let default_keys: Vec<&String> = taxonomy
            .generation_set(false)
            .into_iter()
            .map(|(k, _)| k)
            .collect();
        assert_eq!(default_keys, vec!["a.b.default_leaf"]);
        assert_eq!(taxonomy.generation_set(true).len(), 2);
    }

    #[test]
    fn generate_policy_rejects_unknown_value() {
        let yaml = "a.b.c:\n  title: \"X\"\n  tier: [VARCHAR]\n  generate: sometimes\n";
        let err = Taxonomy::from_yaml(yaml).unwrap_err().to_string();
        assert!(err.contains("sometimes"), "error names the bad value: {err}");
    }
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test -p finetype-core generation_set generate_policy 2>&1 | tail -20`
Expected: compile error, `generation_set` not found.

- [ ] **Step 3: Implement in `taxonomy.rs`**

Replace the field at lines 424 to 427 with:

```rust
    /// When the generator emits this leaf. Absent for almost every leaf, which
    /// `finetype generate` always emits. `on_request` marks a leaf the default
    /// output leaves out and `--all` puts back: a catch-all whose synthetic
    /// values have no shape, or a leaf a rule recovers rather than the model
    /// predicts. The reason is in the leaf's `notes`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generate: Option<GeneratePolicy>,
```

Add the enum next to `Designation`:

```rust
/// Whether `finetype generate` emits a leaf by default. See `Definition::generate`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GeneratePolicy {
    /// Emitted only when the caller asks for every leaf.
    OnRequest,
}
```

Replace `at_priority` (lines 700 to 706) with:

```rust
    /// The leaves the generator emits: every leaf when `all`, otherwise the
    /// leaves not marked `generate: on_request`.
    pub fn generation_set(&self, all: bool) -> Vec<(&String, &Definition)> {
        self.definitions
            .iter()
            .filter(|(_, d)| all || d.generate.is_none())
            .collect()
    }
```

Export `GeneratePolicy` wherever `Definition` is re-exported (`crates/finetype-core/src/lib.rs`: add it to the `pub use taxonomy::{…}` line).

Delete every `  release_priority: N` line in the test YAML of `taxonomy.rs`, `checker.rs`, `generator/tests.rs`, `validation_veto.rs`, `validator.rs`, `tests/precise_audit.rs`, and the `assert_eq!(def.release_priority, 5);` at `taxonomy.rs:1378`:

```bash
sed -i '' '/^[[:space:]]*release_priority: [0-9]*[[:space:]]*$/d' \
  crates/finetype-core/src/taxonomy.rs crates/finetype-core/src/checker.rs \
  crates/finetype-core/src/generator/tests.rs crates/finetype-core/src/validation_veto.rs \
  crates/finetype-core/src/validator.rs crates/finetype-core/tests/precise_audit.rs \
  crates/finetype-core/README.md crates/finetype-model/src/column/tests/*.rs
grep -rn "release_priority" crates/ && echo "still referenced" || echo "clean"
```

Then remove the remaining references the grep lists (the `assert_eq!` line and the generator and checker uses below).

- [ ] **Step 4: Generator and checker**

In `generator/mod.rs`, change both signatures and their one call each:

```rust
    /// Generate samples for every leaf in the generation set (`all` puts the
    /// `generate: on_request` leaves back).
    pub fn generate_all(&mut self, all: bool, samples_per_label: usize) -> Vec<Sample> {
        let entries: Vec<(String, Designation, Vec<String>)> = self
            .taxonomy
            .generation_set(all)
```

and the same for `generate_all_localized`. In `generator/tests.rs:442`, `gen.generate_all_localized(5, 2)` becomes `gen.generate_all_localized(false, 2)`.

In `checker.rs`: delete the `release_priority` field and its doc comment from `DefinitionCheckResult` (lines 33 to 34); delete `at_priority` (lines 174 to 180); at lines 239 to 247 the tuple becomes a single `has_pattern` value:

```rust
        let has_pattern = definition
            .map(|def| {
                def.validation
                    .as_ref()
                    .and_then(|v| v.pattern.as_ref())
                    .is_some()
            })
            .unwrap_or(false);
```

delete `release_priority,` from the struct literal at line 313; and the two report lines `"  - {} (priority: {})\n", r.key, r.release_priority` become `"  - {}\n", r.key`.

- [ ] **Step 5: Run the crate's tests**

Run: `cargo test -p finetype-core 2>&1 | tail -15`
Expected: all pass, including the two new tests. Then `cargo clippy -p finetype-core -- -D warnings`.

- [ ] **Step 6: Commit**

```bash
git add crates/finetype-core
git commit -m "Core: delete release_priority; a leaf is generated by default or on request

The field ranked released leaves against each other, which distinguishes
nothing. Its one job, keeping three leaves out of the generator's default
output, is now a per-leaf field, generate: on_request, that the generation
set reads. Any other value fails the taxonomy load.

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

### Task B2: the definitions and the CLI

**Files:**
- Modify: `labels/definitions_*.yaml` (every `release_priority:` line; `generate: on_request` on three leaves; their `notes`)
- Modify: `crates/finetype-cli/src/main.rs:133-136`, `:157-160`, `:173-176`, `:201-204`, `:552-578`
- Modify: `crates/finetype-cli/src/cmd_taxonomy.rs:9-41`, `:64-137`, `:181-205`, `:272`, `:343-440`
- Create: `crates/finetype-cli/tests/generate_scope.rs`

**Interfaces:**
- Consumes: `Taxonomy::generation_set`, `Generator::generate_all(all, n)`, `Generator::generate_all_localized(all, n)`.
- Produces: `finetype generate [--all]`; `finetype taxonomy` without `--priority`; `finetype check` without `--priority`; JSON and CSV outputs without the priority fields.

- [ ] **Step 1: Write the failing CLI test**

Create `crates/finetype-cli/tests/generate_scope.rs`:

```rust
//! `finetype generate` leaves the `generate: on_request` leaves out of its
//! default output and puts them back under `--all`.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Command;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn labels_of(args: &[&str]) -> BTreeSet<String> {
    let out = tempfile::NamedTempFile::new().unwrap();
    let status = Command::new(env!("CARGO_BIN_EXE_finetype"))
        .current_dir(workspace_root())
        .args(["generate", "--samples", "1", "--output"])
        .arg(out.path())
        .args(args)
        .status()
        .unwrap();
    assert!(status.success());
    std::fs::read_to_string(out.path())
        .unwrap()
        .lines()
        .map(|l| {
            let v: serde_json::Value = serde_json::from_str(l).unwrap();
            v["classification"].as_str().unwrap().to_string()
        })
        .collect()
}

const ON_REQUEST: [&str; 3] = [
    "container.object.s_expression",
    "identity.person.password",
    "representation.text.plain_text",
];

#[test]
fn default_output_skips_on_request_leaves_and_all_puts_them_back() {
    let default = labels_of(&[]);
    let all = labels_of(&["--all"]);
    for key in ON_REQUEST {
        assert!(!default.contains(key), "{key} in default output");
        assert!(all.contains(key), "{key} missing under --all");
    }
    assert_eq!(all.len(), default.len() + ON_REQUEST.len());
}

#[test]
fn priority_flag_is_gone() {
    let status = Command::new(env!("CARGO_BIN_EXE_finetype"))
        .args(["generate", "--priority", "0"])
        .status()
        .unwrap();
    assert!(!status.success());
}
```

Check `tempfile` and `serde_json` are dev-dependencies of `finetype-cli` (`grep -n "tempfile\|serde_json" crates/finetype-cli/Cargo.toml`); add `tempfile` under `[dev-dependencies]` with the workspace version if it is missing.

- [ ] **Step 2: Run it to see it fail**

Run: `cargo test -p finetype-cli --test generate_scope 2>&1 | tail -10`
Expected: the first test fails because `--all` is an unknown flag; the second fails because `--priority 0` still succeeds.

- [ ] **Step 3: The definitions**

```bash
sed -i '' '/^  release_priority: [0-9]*$/d' labels/definitions_*.yaml
grep -n "release_priority" labels/definitions_*.yaml   # one hit left: a notes sentence in container.object.json
```

Delete that sentence, `Set release_priority to 3 (medium) due to complexity of recursive inference.`, from the `notes` of `container.object.json` in `labels/definitions_container.yaml`; the grep then prints nothing.

Then add the field and the reason to the three leaves. In `labels/definitions_container.yaml`, on `container.object.s_expression`, after the `tier:` line add `  generate: on_request` and append to its `notes` block: `Generated on request only: the model does not predict this leaf, a rule recovers it, so its synthetic values exist for the training set's label coverage and not for the default output.` In `labels/definitions_identity.yaml`, on `identity.person.password`, after `tier:` add `  generate: on_request` and append to `notes`: `Generated on request only: a catch-all leaf whose synthetic values have no shape, kept for the training set's label coverage.` In `labels/definitions_representation.yaml`, on `representation.text.plain_text`, the same line and the same note.

Run: `cargo run -q -- check 2>&1 | tail -3`
Expected: `ALL CHECKS PASSED` (the field is read; the loader accepts it).

- [ ] **Step 4: The CLI**

In `main.rs`: on `Generate`, replace the `priority` flag with

```rust
        /// Emit every leaf, including the three marked `generate: on_request`
        /// in the taxonomy (the default output leaves them out)
        #[arg(long)]
        all: bool,
```

On `Taxonomy`, delete the `priority` flag and change the positional's doc to `When supplied, --domain / --category are ignored.` On `Check`, delete the `priority` flag. In the dispatch (lines 552 to 578), replace `priority` with `all` for `Generate` and drop it for the other two, matching the new signatures below.

In `cmd_taxonomy.rs`: `cmd_generate(samples: usize, all: bool, output, taxonomy_path, seed, localized)`; the progress line becomes `"Generating {} samples per label ({}, mode: {})", samples, if all { "every leaf" } else { "default leaves" }, mode`; the two generator calls take `all`. `cmd_taxonomy` loses its `priority` parameter, the `else if let Some(prio) = priority` branch, the retain block (lines 131 to 137), and the `"priority": d.release_priority,` JSON key; the CSV header becomes `key,broad_type,designation,title` and its row loses the priority column. `definition_to_full_json` drops `release_priority` and adds `obj.insert("generate".into(), json!(d.generate));` (serialises as `"on_request"` or `null`). `cmd_check` loses its `priority` parameter, both `.filter(|r| priority…)` lines, the `release_priority` JSON key and the `priority` CSV column (header `key,domain,generator_exists,samples_generated,samples_passed,samples_failed,pass_rate,has_pattern,passed`).

- [ ] **Step 5: Run the tests**

Run: `cargo test -p finetype-cli --test generate_scope 2>&1 | tail -5`
Expected: both pass.

Run: `cargo test --workspace 2>&1 | tail -5 && cargo clippy --workspace --all-targets -- -D warnings`
Expected: green. `grep -rn "release_priority\|at_priority" crates/ labels/` prints nothing.

- [ ] **Step 6: Commit**

```bash
git add labels crates/finetype-cli
git commit -m "Taxonomy and CLI: drop the priority flags; generate --all puts the three leaves back

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

### Task B3: the training script, the skills and the changelog

**Files:**
- Modify: `scripts/prepare_multibranch_data.py:1622-1626`
- Modify: `.claude/skills/add-label/SKILL.md:27`
- Modify: `.claude/skills/finetype-cli/SKILL.md:187`, `:194`, `:213`, `:234`
- Modify: `CHANGELOG.md` (Unreleased)

- [ ] **Step 1: The training script**

Replace the two list items `"--priority", "0",  # Include all types (password is priority 0)` with `"--all",  # the three generate-on-request leaves too`.

Run: `python3 -I -c "import ast,sys; ast.parse(open('scripts/prepare_multibranch_data.py').read())"` and `grep -n '"--priority"' scripts/*.py` (expect nothing).

- [ ] **Step 2: The skills**

In `add-label/SKILL.md:27` remove `release_priority` from the required-field list. In `finetype-cli/SKILL.md` delete the three `--priority` table rows and the `/ `--priority`` mention at line 187.

- [ ] **Step 3: The changelog**

Under `## [Unreleased]` add a `### Removed` section (or extend it):

```
### Removed

- **`release_priority` leaves the taxonomy.** Every definition carried a rank from 0 to 5; the leaves are released, so the rank distinguished nothing. The field is gone from `labels/definitions_*.yaml`, from `finetype taxonomy`'s JSON (`release_priority`, `priority`) and CSV (`priority`) output, from `finetype check`'s output, and the `--priority` flag is gone from `generate`, `taxonomy` and `check`. Its one job survives as a per-leaf `generate: on_request` field on `container.object.s_expression`, `identity.person.password` and `representation.text.plain_text`: `finetype generate` leaves those three out unless passed `--all`, which is what the training data script passes.
```

- [ ] **Step 4: Gates, commit, push, pull request**

Run: `make build-extension && scripts/check_sql_examples.py && scripts/check_duckdb_catalog.py && scripts/check_doc_taxonomy_counts.py && cargo test --workspace 2>&1 | tail -3`
Expected: green; quote the totals.

```bash
git add scripts/prepare_multibranch_data.py .claude/skills CHANGELOG.md
git commit -m "Training script, skills and changelog follow the priority removal

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
git push -u origin no-priority-dimension
gh pr create --title "Taxonomy: delete release_priority; three leaves generate on request" --body-file -
```

Body: the field's one live job, the replacement field and the reason per leaf, what the site's build script will see at the next release, the gate totals. Attribution line. Wait for review, close findings, report the link.

---

# Part C: piece 4, a scheduled check re-measures the README version table

Branch: `readme-version-table-check` from `main` after Part B merges. Pull request title: `README version table: a weekly check re-measures it`.

### Task C1: the script, parse and compare, with a self-test

**Files:**
- Create: `scripts/check_readme_version_table.py`
- Modify: `.github/gate-self-tests.tsv` (add a row)
- Modify: `.github/workflows/ci.yml` (`doc-surface` job, a guarded step after the `registry_expressions` self-test)

**Interfaces:**
- Produces: `check_readme_version_table.py --self-test` (offline); `check_readme_version_table.py --measure [--readme README.md] [--report report.md]` (network; exit 0 when nothing is stale or missing, 1 otherwise, 3 when a row could not be measured or the release list could not be fetched); `parse_table(text) -> list[Row]`; `classify_cell(cell) -> Outcome`; `classify_probe(install_out, load_out, version_out) -> Outcome`; `missing_rows(rows, releases) -> list[str]`.

- [ ] **Step 1: Write the script with its self-test first**

The script is the committed file `scripts/check_readme_version_table.py`, not a copy here: its module docstring says what it does and how it is run, and its `--self-test` carries the case list (one per cell class, per probe outcome, a stale cell, an unreadable cell whose report carries the measured value, a release list with a prerelease that must not count as missing, and a probe that raises for one row without losing the others).

- [ ] **Step 2: Run the self-test**

Run: `chmod +x scripts/check_readme_version_table.py && scripts/check_readme_version_table.py --self-test`
Expected: `Results: 27 passed, 0 failed`. If a case fails, fix the classifier, not the case: each case is the spec's outcome table.

- [ ] **Step 3: Register the self-test and route it**

Append to `.github/gate-self-tests.tsv`:

```
readme_version_table	scripts/check_readme_version_table.py --self-test	scripts/check_readme_version_table.py
```

In `.github/workflows/ci.yml`, in the `doc-surface` job, after the step that runs `scripts/check_registry_expressions.py --self-test`, add:

```yaml
      - name: ...and the README version-table check reads cells and probes as the spec says
        # Offline: fixture rows for each cell class, canned probe output for
        # each outcome, a stale cell, an unreadable cell, and a release list
        # with a prerelease that must not count as a missing row. The
        # measurement itself runs on the weekly schedule, never here.
        if: steps.route.outputs.readme_version_table == 'true'
        run: scripts/check_readme_version_table.py --self-test
```

Run: `.github/scripts/gate-self-tests.py audit`
Expected: exit 0.

- [ ] **Step 4: Commit**

```bash
git add scripts/check_readme_version_table.py .github/gate-self-tests.tsv .github/workflows/ci.yml
git commit -m "README version table: a script re-measures each row against the registry

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

### Task C2: the weekly workflow and the 1.5.0 cell

**Files:**
- Create: `.github/workflows/weekly-readme-version-table.yml`
- Modify: `README.md:126-130` (the 1.5.0 row, the 1.4 sentence under the table)

- [ ] **Step 1: The workflow**

The workflow is the committed file `.github/workflows/weekly-readme-version-table.yml`, not a copy here: a Monday 07:00 UTC schedule plus `workflow_dispatch`, a `measure` job on `macos-15` that runs the script and keeps the report as an artifact, and a `notify` job that on any failure comments on the open `readme-version-table` issue or opens one whose body defers to the report. Its action versions match `MainDistributionPipeline.yml`.

- [ ] **Step 2: The 1.5.0 cell**

In `README.md`, replace the 1.5.0 row:

```
> | 1.5.0 | `ft_version()` does not exist — the build is `finetype 0.2.0`, from before the `ft_` verbs |
```

with:

```
> | 1.5.0 | `ft_version()` does not exist — the build is `finetype 0.2.0`, from before the `ft_` verbs; `SELECT finetype_version()` answers |
```

This was measured on 2026-10-12 on `osx_arm64`: the 1.5.0 build lists `finetype`, `finetype_cast`, `finetype_detail`, `finetype_unpack` and `finetype_version`, and `SELECT finetype_version()` returns `finetype 0.2.0`.

Then move the 1.4 facts from prose into rows. Replace the three `> `-prefixed lines under the table:

```
> Rows are the 1.5 line. The 1.4 maintenance line has the same gaps — 1.4.4 is served; 1.4.0 to
> 1.4.3 and 1.4.5 are not — so always check what you actually got with `SELECT ft_version();`
> rather than assuming.
```

with:

```
> Always check what you actually got with `SELECT ft_version();` (or `SELECT finetype_version();`
> on a build that predates the `ft_` names) rather than assuming.
```

and add one row per 1.4 release (1.4.5 down to 1.4.0) below the 1.5.0 row, each cell written from Step 3's measurement in the same form as the 1.5 cells: a `finetype X.Y.Z` string, `no build published — `INSTALL` fails with HTTP 404`, or the refused-load form. Change the note's date line `Measured 2026-10-11 on `osx_arm64`` to the date of Step 3's run. The 1.5 cells stay as they were measured.

- [ ] **Step 3: Run the real measurement once, locally**

Run it twice. First with placeholder 1.4 rows (cell text `to measure`), which the parser reports as unreadable while printing each measured outcome: `scripts/check_readme_version_table.py --measure --report /tmp/readme-table.md; echo "exit $?"`. Write each 1.4 cell from the printed outcome. Then run it again.
Expected on the second run: every row matches and exit 0, unless the registry has republished since 2026-10-11, in which case the report names the 1.5 cell; either way, paste the report into the pull request. It downloads thirteen DuckDB CLIs, so allow several minutes.

- [ ] **Step 4: Gates, commit, push, pull request**

Run: `scripts/check_sql_examples.py && scripts/check_duckdb_catalog.py && cargo test --workspace 2>&1 | tail -3 && ./scripts/check-public-hygiene.sh`

```bash
git add .github/workflows/weekly-readme-version-table.yml README.md
git commit -m "README version table: weekly re-measurement on macOS arm64, and the 1.5.0 cell says which call answers

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
git push -u origin readme-version-table-check
gh pr create --title "README version table: a weekly check re-measures it" --body-file -
```

Body: what goes stale and why nothing caught it, the four outcome classes, the schedule and the no-gate rule, the local measurement's report, the gate totals. Attribution line. After merge, trigger the workflow once by hand (`gh workflow run weekly-readme-version-table.yml`) and report its run link.
