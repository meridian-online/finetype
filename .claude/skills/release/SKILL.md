---
description: >-
  Release FineType — `model <dir>` publishes trained weights to HuggingFace and swaps the default symlink; `binary` bumps the workspace version, tags, and triggers the GitHub release workflow that produces cross-platform binaries + Homebrew tap update; no-arg form does both. Network + git push, so run pre-flight checks first and stop on any failure.
when_to_use: User says "release", "ship", "cut a release", "publish the model", "tag a version", or names a model dir and asks to upload it. Treat as a deliberate, reviewed action — never auto-fire mid-task.
argument-hint: "[model <model-dir> | binary]"
arguments: mode target
allowed-tools: Bash, Read, Edit
---

# /release

Two release types: **model** (publish trained weights to HuggingFace) and **binary** (cut a GitHub release with cross-platform binaries + Homebrew update). A binary release also **surfaces the separate DuckDB community-channel refresh** — that channel is not part of `release.yml`, so the skill prompts for it rather than letting it drift silently.

## Versioning Policy

**Prefer patch releases** (0.6.x) over minor bumps. Reserve minor (0.x.0) for breaking changes to the CLI interface, DuckDB extension API, or taxonomy structure.

## Usage

```
/release model <model_dir>    # Publish model to HuggingFace
/release binary               # Cut a GitHub release (bump version, tag, push)
/release                      # Both: publish model + cut binary release
```

### Model Release

Where `<model_dir>` is a path under `models/` (e.g. `models/sherlock-v11`).

## Instructions

### 1. Pre-flight Checks

Run these checks and **stop on any failure**:

1. **Model artifacts exist** — verify `<model_dir>` contains:
   - `model.safetensors` (required — the weights)
   - `config.json` (required — architecture config; for dual-encoder models it must
     carry `value_embed_model` and `type_index_keys`)
   - `label_map.json` (required — class index mapping)
   - `value_model2vec/` (required for dual-encoder models — the co-located value
     encoder; profile-time loading fails without it, an empty snapshot masquerades as
     a model failure)
   - `results.json` (optional — training metrics)

2. **Tests pass** — `cargo test -p finetype-model` must pass with zero failures.

3. **Zero warnings** — `cargo build --workspace 2>&1 | grep "^warning\[" | head -1` must be empty (build.rs info lines are fine).

4. **Config sanity** — read `config.json` and print key fields: `n_classes`, `activation`, `head_type`. Confirm `n_classes` matches the live taxonomy count from `./target/release/finetype taxonomy | grep -cE "^[a-z]"` — never a hardcoded number (a stale `239` here once outlived three taxonomy expansions). Recovery-only leaves are model-invisible by design, so an intentional gap must be stated in the release notes, not silently accepted.

5. **Current default** — read `models/default` symlink and show what's being replaced.

Present the pre-flight summary and wait for confirmation before proceeding.

### 2. Upload to HuggingFace

Upload the three required files to `meridian-online/finetype-model`:

```python
from huggingface_hub import HfApi
api = HfApi()

files = ["model.safetensors", "config.json", "label_map.json"]
# Dual-encoder models additionally ship the co-located value encoder:
from pathlib import Path
ve = Path(model_dir) / "value_model2vec"
if ve.is_dir():
    files += [f"value_model2vec/{p.name}" for p in ve.iterdir() if p.is_file()]

for file in files:
    api.upload_file(
        path_or_fileobj=f"{model_dir}/{file}",
        path_in_repo=f"{model_name}/{file}",
        repo_id="meridian-online/finetype-model",
        repo_type="model",
        commit_message=f"Add {model_name}/{file}"
    )
```

Where `model_name` is the directory basename (e.g. `sherlock-v11`).

After upload, verify the files are accessible (include one value_model2vec file for
dual-encoder models — `download-model.sh` fetches them and a partial upload 404s at
install time, not at release time):
```bash
curl -sfI "https://huggingface.co/meridian-online/finetype-model/resolve/main/{model_name}/model.safetensors"
curl -sfI "https://huggingface.co/meridian-online/finetype-model/resolve/main/{model_name}/value_model2vec/model.safetensors"  # dual-encoder only
```

### 3. Update Default Symlink

```bash
ln -sfn {model_name} models/default
```

### 4. Update Card & CLAUDE.md

If eval results exist (check `<model_dir>/eval/` or `eval/eval_output/profile_results.json`):

1. **CLAUDE.md** — update the `Default model:` line and `Profile eval:` numbers in both Current State and Evaluation infrastructure sections
2. **Eval baseline** — record the new baseline wherever eval goals are tracked.

If no eval results exist, note this and skip — the eval must run before CLAUDE.md can be updated.

### 5. Commit & Push

Stage and commit:
```bash
git add models/default CLAUDE.md
git commit -m "Publish {model_name} to HuggingFace, update default model"
git push
```

### 6. Verify CI Download

The CI download script (`.github/scripts/download-model.sh`) reads `models/default` to determine which model to fetch. After pushing, CI will automatically download the new model on next run.

Optionally, dry-run the download script locally:
```bash
.github/scripts/download-model.sh
```

### 7. Summary

Report:
```
Released {model_name} to HuggingFace.

  HF repo:  meridian-online/finetype-model
  Files:    {model_name}/model.safetensors, config.json, label_map.json
  Default:  models/default → {model_name}
  Eval:     {score}/{total} ({pct}% label)

  DuckDB extension and CI will download automatically.
```

### Binary Release

Cut a GitHub release with cross-platform binaries.

#### 1. Pre-flight Checks

Run these and **stop on any failure** — a binary release that fails build ships nothing (publish jobs skip when builds fail), so every gap here is caught the hard way in CI.

1. **CI is green** — all jobs on the current main branch must pass.
2. **No uncommitted changes** — `git status` must be clean.
3. **Tests pass locally** — `cargo test -p finetype-model` must pass.
4. **Zero warnings** — `cargo build --workspace 2>&1 | grep "^warning\[" | head -1` must be empty.
5. **Model symlink ↔ CI env agree** — `embed-models` (default feature) bakes the `models/default` target into the binary via `build.rs`, and CI only downloads the model named by `FINETYPE_CI_MODEL`. If they diverge, every platform build fails with `Flat model not found at "models/<name>"`. Assert all three match:

   ```bash
   DEFAULT=$(basename "$(readlink models/default)")
   CI_MODEL=$(grep -h 'FINETYPE_CI_MODEL:' .github/workflows/ci.yml .github/workflows/release.yml | awk '{print $2}' | sort -u)
   echo "models/default → $DEFAULT"
   echo "FINETYPE_CI_MODEL → $CI_MODEL"
   # All three values MUST be identical. If not, fix the drift before tagging
   # (bump FINETYPE_CI_MODEL in both workflows, or revert the symlink).
   ```

6. **Default model is published on HuggingFace** — bumping `FINETYPE_CI_MODEL` is useless if the weights aren't uploaded; CI's download 404s. Confirm the `models/default` target is fetchable before tagging:

   ```bash
   curl -sfI "https://huggingface.co/meridian-online/finetype-model/resolve/main/$DEFAULT/model.safetensors" >/dev/null \
     && echo "$DEFAULT published" || echo "MISSING — run /release model first"
   # Dual-encoder defaults: the value encoder must be fetchable too, or
   # download-model.sh installs a model that cannot load at profile time.
   [ -d "models/$DEFAULT/value_model2vec" ] && \
     { curl -sfI "https://huggingface.co/meridian-online/finetype-model/resolve/main/$DEFAULT/value_model2vec/model.safetensors" >/dev/null \
       && echo "$DEFAULT value encoder published" || echo "VALUE ENCODER MISSING — rerun /release model"; }
   ```

   If it 404s, the model has not been released — run the **Model Release** flow first.

7. **Windows bundled-DuckDB risk (only if the duckdb pin changed)** — `finetype-cli` pulls `libduckdb-sys` transitively (finetype-cli → finetype-train → duckdb), so a workspace `duckdb` pin bump recompiles bundled DuckDB C++ on every platform, including Windows MSVC. New DuckDB releases have broken MSVC builds (e.g. 1.5.3 fmt `checked_array_iterator` C2061). If `git diff v{prev}..HEAD -- Cargo.toml` touches the `duckdb`/`libduckdb-sys` pins, confirm a Windows build has gone green on a PR before tagging — the local toolchain can't reproduce the MSVC failure.

#### 2. Bump Version

Bump the **patch** version in the workspace `Cargo.toml` (e.g. `0.6.12` → `0.6.13`). All crates use `version.workspace = true` so this is the only file to change.

```bash
# Update version in Cargo.toml
# Then verify it compiles:
cargo check --workspace
```

#### 3. Update CHANGELOG.md

Move the `[Unreleased]` section contents into a new version heading. Follow [Keep a Changelog](https://keepachangelog.com/) format:

```markdown
## [0.6.X] - YYYY-MM-DD

### Changed
- ...

### Added
- ...

### Fixed
- ...
```

Categories: `Changed`, `Added`, `Fixed`, `Removed`, `Discovery` (for research findings).

Compile the changelog from:
1. `git log v{prev}..HEAD --oneline` — all commits since last release
2. Specs completed since last release (check `specs/` dates)
3. Decisions made since last release (check `decisions/` dates)

Focus on user-visible changes: model accuracy, new commands/flags, bug fixes, performance. Skip internal refactors unless architecturally significant.

If prior releases have placeholder entries (`_Changelog not maintained_`), leave them — don't backfill.

#### 4. Update CLAUDE.md

Update the `**Version:**` line in CLAUDE.md to match the new version.

#### 5. Commit, Tag, Push

```bash
git add Cargo.toml CLAUDE.md CHANGELOG.md
git commit -m "Release v0.6.X"
git tag v0.6.X
git push && git push --tags
```

The `v*` tag triggers `.github/workflows/release.yml` which:
- Builds cross-platform binaries (Linux x86/arm, macOS x86/arm, Windows)
- Creates a GitHub release with auto-generated release notes
- Updates the Homebrew tap formula (`meridian-online/homebrew-tap`)
- Dispatches install script update to `install.meridian.online`

#### 6. Verify

1. Check the [release page](https://github.com/meridian-online/finetype/releases) for the new version
2. Verify all 5 platform builds completed
3. Test: `brew upgrade finetype` (after Homebrew tap updates)

### DuckDB community extension (separate channel — surface this every binary release)

`release.yml` does **not** touch the DuckDB community channel — the extension is a distinct
artifact, published by a manual PR to `duckdb/community-extensions`, which DuckDB's own CI
then rebuilds (including for each new DuckDB release, at the pinned `ref`). Because it sits
outside this automation it drifts silently, so **after every binary release, surface it and
ask the user** — don't skip quietly:

> The DuckDB community extension is on a separate channel, currently **{community_version}**
> (this release is **{new_version}**). Refresh it now? [y/N]

Read `{community_version}` from the descriptor (or the last merged upstream PR):

```bash
grep -m1 '^  version:' ../community-extensions/extensions/finetype/description.yml
gh pr list -R duckdb/community-extensions --search finetype --state merged -L 1
```

**Resubmit only when it earns an external PR — batch, don't PR per patch:**
- extension users should get materially newer finetype (new/renamed types or `ft_` functions), or
- a DuckDB bump broke the pinned build and needs a new `ref` (see `weekly-duckdb-compat.yml`).

If **yes** — in the `community-extensions` repo (a fork of `duckdb/community-extensions`):
1. Edit `extensions/finetype/description.yml`: bump `version` → `{new_version}`, repoint
   `repo.ref` to the release commit/tag, and refresh the type count + `ft_` docs if they changed.
2. If the duckdb pin moved, confirm the `ref` builds on DuckDB's CI first (Windows MSVC is the
   usual break — see Binary pre-flight #7).
3. PR upstream to `duckdb/community-extensions` (branch `finetype-v{new_version}`); their CI
   rebuilds + redistributes on merge. This is an external, reviewed, outward-facing PR.

If **no** — note it and move on: CLI / Homebrew / install users are already current; extension
users stay on {community_version} until a deliberate resubmission.

### crates.io (library crates — separate, deliberate step)

The binary release above does **not** publish to crates.io. The five library crates
(`finetype-core`, `finetype-model`, `finetype-mcp`, `finetype-train`, `finetype-cli`) are a
distinct publish. Do it when a release changes library-visible behaviour a downstream crate
consumer should get — e.g. taxonomy/validator changes, which are **embedded into
`finetype-core`** at build time, so they reach consumers only via a crates.io bump (a
`labels/*.yaml` change like the 0.6.41 checksum guards qualifies). Skip it for pure
binary/CI/tooling changes.

**Publish order — dependency order; each must be on crates.io before the crate that depends
on it:**

```
core → model → mcp → train → cli
```

`finetype-cli` depends on `finetype-mcp` + `finetype-train`, so those publish first even
though their READMEs mark them internal/no-stability.

```bash
for c in finetype-core finetype-model finetype-mcp finetype-train finetype-cli; do
  cargo publish -p "$c" --dry-run   # then drop --dry-run; let the index settle between crates
done
```

**Gotcha — `include_str!` of a `labels/` file breaks packaging.**
`include_str!("../../../labels/…")` escapes the package root, so `cargo package`/`publish`
fails. Any `labels/` file a crate embeds MUST be reached through an in-crate `data/` symlink
(e.g. `crates/finetype-core/data/… → ../../../labels/…`); cargo dereferences it at packaging
while `labels/` stays canonical. Adding a **new** `include_str!` of a `labels/` file without
the matching `data/` symlink will break the next publish (this silently broke packaging once
already — see memory `company-reference-audit-2026-07`). Verify with `cargo package -p
finetype-core` before a real publish.

### Rollback

**Model rollback:**
1. Revert the `models/default` symlink to the previous model
2. Commit and push — CI and DuckDB extension will revert on next download
3. HuggingFace files can stay (old versions are still available by path)

**Binary rollback:**
1. Delete the GitHub release and tag if builds failed
2. If already published: cut a new patch release with the fix
3. Homebrew users get the fix on next `brew upgrade`
