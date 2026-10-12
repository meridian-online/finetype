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
- Produces: `check_readme_version_table.py --self-test` (offline); `check_readme_version_table.py --measure [--readme README.md] [--report report.md]` (network; exit 0 when nothing is stale or missing, 1 otherwise, 2 on a tool error); `parse_table(text) -> list[Row]`; `classify_cell(cell) -> Outcome`; `classify_probe(install_out, load_out, version_out) -> Outcome`; `missing_rows(rows, releases) -> list[str]`.

- [ ] **Step 1: Write the script with its self-test first**

```python
#!/usr/bin/env python3
"""Re-measure the README's DuckDB version table against the community registry.

README.md carries a table: per DuckDB version, what the community build of
finetype answered to SELECT ft_version() on osx_arm64, on a dated run. Nothing
re-measured it after the registry republished, so a cell could go stale with
no one noticing. This does the measuring, on a schedule, and reports.

WHAT IT DOES
    1. Parse the table: one row per DuckDB version, each cell read into an
       outcome: a version string, "not published" (INSTALL fails with HTTP
       404), or "refused" (INSTALL serves a file the version refuses to load).
    2. For each row, download that DuckDB CLI release for this platform and
       run the README's two lines, INSTALL finetype FROM community; and
       LOAD finetype; then SELECT ft_version(), falling back to
       SELECT finetype_version() when the first name does not exist (the
       build that predates the ft_ names). Classify the result the same way
       and compare.
    3. List every stable DuckDB release at or above the table's lowest
       version that has no row.

A stale cell or a missing row is reported and the exit is 1. The README is
never edited: a cell is a measurement with a date, and the fix is a new
measurement a person commits.

It is a signal, not a gate. The workflow that runs it is scheduled and never
triggered by a pull request.

USAGE
    scripts/check_readme_version_table.py --self-test   # offline: parse, classify, compare
    scripts/check_readme_version_table.py --measure     # network: the real thing
    scripts/check_readme_version_table.py --measure --report report.md

Stdlib only.
"""

from __future__ import annotations

import argparse
import json
import os
import platform
import re
import subprocess
import sys
import tempfile
import urllib.request
import zipfile
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
README = ROOT / "README.md"
RELEASES_URL = "https://api.github.com/repos/duckdb/duckdb/releases?per_page=100"

ROW_RE = re.compile(r"^> \| (\d+\.\d+\.\d+) \| (.*) \|\s*$")
VERSION_RE = re.compile(r"`finetype (\d+\.\d+\.\d+)`")


@dataclass(frozen=True)
class Outcome:
    kind: str          # "version" | "not_published" | "refused" | "unreadable" | "error"
    detail: str = ""   # the version for "version", the message otherwise

    def __str__(self) -> str:
        return f"{self.kind}:{self.detail}" if self.detail else self.kind


@dataclass(frozen=True)
class Row:
    duckdb: str
    cell: str
    outcome: Outcome


def version_key(v: str) -> tuple[int, ...]:
    return tuple(int(p) for p in v.split("."))


def classify_cell(cell: str) -> Outcome:
    m = VERSION_RE.search(cell)
    if m:
        return Outcome("version", m.group(1))
    if "HTTP 404" in cell:
        return Outcome("not_published")
    if "refuses to load" in cell:
        return Outcome("refused")
    return Outcome("unreadable", cell)


def parse_table(text: str) -> list[Row]:
    rows = []
    for line in text.splitlines():
        m = ROW_RE.match(line)
        if m:
            rows.append(Row(m.group(1), m.group(2), classify_cell(m.group(2))))
    if not rows:
        raise SystemExit("no version-table rows found; the README table shape changed")
    return rows


def classify_probe(install_out: str, load_out: str, version_out: str) -> Outcome:
    """Classify one DuckDB release's probe from the three statements' output.

    A refused build surfaces at INSTALL on some releases and at LOAD on others,
    so both outputs are read the same way: a 404 means no build is published,
    a version complaint means the served build is for another DuckDB.
    """
    for out in (install_out, load_out):
        if "Error" not in out:
            continue
        if "404" in out:
            return Outcome("not_published")
        if "version" in out.lower():
            return Outcome("refused")
        return Outcome("error", out.strip().splitlines()[-1])
    m = re.search(r"finetype (\d+\.\d+\.\d+)", version_out)
    if m:
        return Outcome("version", m.group(1))
    return Outcome("error", version_out.strip().splitlines()[-1] if version_out.strip() else "no output")


def missing_rows(rows: list[Row], releases: list[dict]) -> list[str]:
    """Stable releases at or above the table's lowest version with no row."""
    have = {r.duckdb for r in rows}
    floor = min(version_key(r.duckdb) for r in rows)
    out = []
    for rel in releases:
        if rel.get("prerelease") or rel.get("draft"):
            continue
        tag = rel.get("tag_name", "")
        if not re.fullmatch(r"v\d+\.\d+\.\d+", tag):
            continue
        v = tag[1:]
        if version_key(v) >= floor and v not in have:
            out.append(v)
    return sorted(out, key=version_key)


def compare(rows: list[Row], measured: dict[str, Outcome]) -> list[str]:
    problems = []
    for r in rows:
        got = measured[r.duckdb]
        if r.outcome.kind == "unreadable":
            problems.append(f"{r.duckdb}: cell is not readable as a version or a known failure: {r.cell!r}")
        elif got != r.outcome:
            problems.append(f"{r.duckdb}: README says {r.outcome}, measured {got}")
    return problems


# ── measurement ─────────────────────────────────────────────────────────────

def cli_asset() -> str:
    system, machine = platform.system(), platform.machine()
    if system == "Darwin":
        return "duckdb_cli-osx-universal.zip"
    if system == "Linux":
        return "duckdb_cli-linux-aarch64.zip" if machine in ("arm64", "aarch64") else "duckdb_cli-linux-amd64.zip"
    raise SystemExit(f"unsupported platform {system}/{machine}")


def fetch_cli(version: str, into: Path) -> Path:
    url = f"https://github.com/duckdb/duckdb/releases/download/v{version}/{cli_asset()}"
    zip_path = into / f"duckdb-{version}.zip"
    with urllib.request.urlopen(url, timeout=120) as resp, open(zip_path, "wb") as fh:
        fh.write(resp.read())
    with zipfile.ZipFile(zip_path) as zf:
        zf.extract("duckdb", into)
    exe = into / "duckdb"
    exe.chmod(0o755)
    return exe


def run_sql(exe: Path, ext_dir: Path, sql: str) -> str:
    proc = subprocess.run(
        [str(exe), "-noheader", "-csv", "-c", f"SET extension_directory='{ext_dir}'; {sql}"],
        capture_output=True, text=True, timeout=300,
    )
    return proc.stdout + proc.stderr


def probe(version: str, work: Path) -> Outcome:
    exe = fetch_cli(version, work / version)
    ext_dir = work / version / "ext"
    ext_dir.mkdir(parents=True, exist_ok=True)
    install_out = run_sql(exe, ext_dir, "INSTALL finetype FROM community;")
    if "Error" in install_out:
        return classify_probe(install_out, "", "")
    load_out = run_sql(exe, ext_dir, "LOAD finetype;")
    if "Error" in load_out:
        return classify_probe(install_out, load_out, "")
    version_out = run_sql(exe, ext_dir, "LOAD finetype; SELECT ft_version();")
    if "Catalog Error" in version_out:
        # The build predates the ft_ names (the 1.5.0 row); its old name answers.
        version_out = run_sql(exe, ext_dir, "LOAD finetype; SELECT finetype_version();")
    return classify_probe(install_out, load_out, version_out)


def fetch_releases() -> list[dict]:
    req = urllib.request.Request(RELEASES_URL, headers={"Accept": "application/vnd.github+json"})
    token = os.environ.get("GITHUB_TOKEN")
    if token:
        req.add_header("Authorization", f"Bearer {token}")
    with urllib.request.urlopen(req, timeout=60) as resp:
        return json.load(resp)


def measure(readme: Path, report: Path | None) -> int:
    rows = parse_table(readme.read_text(encoding="utf-8"))
    measured: dict[str, Outcome] = {}
    with tempfile.TemporaryDirectory() as tmp:
        work = Path(tmp)
        for r in rows:
            measured[r.duckdb] = probe(r.duckdb, work)
            print(f"  {r.duckdb}: README {r.outcome}, measured {measured[r.duckdb]}")
    problems = compare(rows, measured)
    missing = missing_rows(rows, fetch_releases())
    lines = [f"Re-measured {len(rows)} rows on {platform.system()}/{platform.machine()}.", ""]
    if problems:
        lines += ["Stale cells:"] + [f"- {p}" for p in problems] + [""]
    if missing:
        lines += ["DuckDB releases with no row:"] + [f"- {v}" for v in missing] + [""]
    if not problems and not missing:
        lines.append("Every cell matches and every release has a row.")
    text = "\n".join(lines)
    print(text)
    if report:
        report.write_text(text + "\n", encoding="utf-8")
    return 1 if problems or missing else 0


# ── self-test ───────────────────────────────────────────────────────────────

FIXTURE = """\
> | DuckDB | `SELECT ft_version()` |
> |---|---|
> | 1.5.6 | `finetype 0.6.57` |
> | 1.5.2 | no build published — `INSTALL` fails with HTTP 404 |
> | 1.5.1 | `INSTALL` fails — the file served is the 1.5.0 build, which 1.5.1 refuses to load |
> | 1.5.0 | `ft_version()` does not exist — the build is `finetype 0.2.0`; `SELECT finetype_version()` answers |
"""


def self_test() -> int:
    passed = failed = 0

    def case(name: str, ok: bool, detail: str = "") -> None:
        nonlocal passed, failed
        if ok:
            passed += 1
            print(f"  [PASS] {name}")
        else:
            failed += 1
            print(f"  [FAIL] {name} {detail}")

    rows = parse_table(FIXTURE)
    case("parse_four_rows", [r.duckdb for r in rows] == ["1.5.6", "1.5.2", "1.5.1", "1.5.0"], str(rows))
    case("cell_version", rows[0].outcome == Outcome("version", "0.6.57"))
    case("cell_not_published", rows[1].outcome == Outcome("not_published"))
    case("cell_refused", rows[2].outcome == Outcome("refused"))
    case("cell_old_build_reads_its_version", rows[3].outcome == Outcome("version", "0.2.0"))
    case("cell_unreadable", classify_cell("something else entirely").kind == "unreadable")

    case("probe_version", classify_probe("", "", "finetype 0.6.57\n") == Outcome("version", "0.6.57"))
    case("probe_404", classify_probe("IO Error: Failed to download extension ... HTTP 404", "", "").kind == "not_published")
    case("probe_refused_at_load", classify_probe("", "Invalid Input Error: ... built for DuckDB version 'v1.5.0' ...", "").kind == "refused")
    case("probe_refused_at_install", classify_probe("IO Error: ... built for DuckDB version 'v1.5.0' ...", "", "").kind == "refused")
    case("probe_other_error", classify_probe("", "", "Catalog Error: ft_version does not exist").kind == "error")

    measured = {
        "1.5.6": Outcome("version", "0.6.57"),
        "1.5.2": Outcome("not_published"),
        "1.5.1": Outcome("refused"),
        "1.5.0": Outcome("version", "0.2.0"),
    }
    case("compare_all_match", compare(rows, measured) == [])
    stale = dict(measured, **{"1.5.6": Outcome("version", "0.6.61")})
    case("compare_stale_version", any("1.5.6" in p and "0.6.61" in p for p in compare(rows, stale)))
    unreadable = parse_table(FIXTURE + "> | 1.4.4 | served, probably |\n")
    case("compare_reports_unreadable", any("1.4.4" in p and "not readable" in p for p in compare(unreadable, dict(measured, **{"1.4.4": Outcome("version", "0.6.0")}))))

    releases = [
        {"tag_name": "v1.5.7", "prerelease": False},
        {"tag_name": "v1.5.6", "prerelease": False},
        {"tag_name": "v2.0.0", "prerelease": False},
        {"tag_name": "v2.0.0-dev", "prerelease": True},
        {"tag_name": "v1.4.4", "prerelease": False},
    ]
    case("missing_rows_above_floor", missing_rows(rows, releases) == ["1.5.7", "2.0.0"])
    case("prerelease_is_not_missing", "2.0.0-dev" not in missing_rows(rows, releases))

    print(f"\nResults: {passed} passed, {failed} failed")
    return 0 if failed == 0 else 1


def main() -> int:
    parser = argparse.ArgumentParser(description=(__doc__ or "").split("\n")[0])
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("--measure", action="store_true")
    parser.add_argument("--readme", type=Path, default=README)
    parser.add_argument("--report", type=Path)
    args = parser.parse_args()
    if args.self_test:
        return self_test()
    if args.measure:
        return measure(args.readme, args.report)
    parser.print_help()
    return 2


if __name__ == "__main__":
    sys.exit(main())
```

- [ ] **Step 2: Run the self-test**

Run: `chmod +x scripts/check_readme_version_table.py && scripts/check_readme_version_table.py --self-test`
Expected: `Results: 16 passed, 0 failed`. If a case fails, fix the classifier, not the case: each case is the spec's outcome table.

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

```yaml
name: Weekly README version table

# Re-measures the per-DuckDB-version table in README.md against the community
# registry, on the platform the table names, and files an issue when a cell is
# stale or a DuckDB release has no row. A signal, never a gate: no pull-request
# or push trigger, because a registry outage must not redden a pull request its
# author cannot fix. The README is never edited by this workflow.

on:
  schedule:
    - cron: '0 7 * * 1'      # Mondays 07:00 UTC, after the compat build
  workflow_dispatch:

concurrency:
  group: weekly-readme-version-table
  cancel-in-progress: true

permissions:
  contents: read

jobs:
  measure:
    # macOS arm64: the table's rows were measured on osx_arm64.
    runs-on: macos-15
    timeout-minutes: 30
    outputs:
      status: ${{ steps.run.outcome }}
    steps:
      - uses: actions/checkout@v5
      - name: Re-measure every row
        id: run
        env:
          GITHUB_TOKEN: ${{ github.token }}
        run: scripts/check_readme_version_table.py --measure --report "$RUNNER_TEMP/report.md"
      - name: Keep the report
        if: always()
        uses: actions/upload-artifact@v4
        with:
          name: readme-version-table-report
          path: ${{ runner.temp }}/report.md
          if-no-files-found: ignore

  notify:
    needs: measure
    if: failure()
    runs-on: ubuntu-latest
    permissions:
      issues: write
    steps:
      - uses: actions/download-artifact@v4
        with:
          name: readme-version-table-report
          path: report
        continue-on-error: true
      - uses: actions/github-script@v7
        with:
          script: |
            const fs = require('fs');
            const label = 'readme-version-table';
            try {
              await github.rest.issues.getLabel({ owner: context.repo.owner, repo: context.repo.repo, name: label });
            } catch (e) {
              await github.rest.issues.createLabel({ owner: context.repo.owner, repo: context.repo.repo, name: label, color: 'fbca04' });
            }
            let report = '(no report file: the measurement itself failed; read the run log)';
            try { report = fs.readFileSync('report/report.md', 'utf8'); } catch (e) {}
            const run = `${context.serverUrl}/${context.repo.owner}/${context.repo.repo}/actions/runs/${context.runId}`;
            // A stale cell stays stale until a person commits a new measurement,
            // so the same finding lands on the open issue rather than a new one.
            const open = await github.rest.issues.listForRepo({
              owner: context.repo.owner, repo: context.repo.repo, labels: label, state: 'open', per_page: 1,
            });
            if (open.data.length > 0) {
              await github.rest.issues.createComment({
                owner: context.repo.owner, repo: context.repo.repo, issue_number: open.data[0].number,
                body: ['Re-measured again.', '', '```', report.trim(), '```', '', `Run: ${run}`].join('\n'),
              });
              return;
            }
            await github.rest.issues.create({
              owner: context.repo.owner,
              repo: context.repo.repo,
              title: `README version table is stale or incomplete (run ${context.runId})`,
              body: [
                'The weekly re-measurement of the README DuckDB version table found a cell that no longer matches the community registry, or a DuckDB release with no row.',
                '',
                'This is a signal, not a gate. Re-measure, update the table with the new date, and close this.',
                '',
                '```', report.trim(), '```',
                '',
                `Run: ${run}`,
              ].join('\n'),
              labels: [label],
            });
```

Check the `upload-artifact`/`download-artifact` major versions match the ones used in `MainDistributionPipeline.yml` (v4 today); use the same.

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
