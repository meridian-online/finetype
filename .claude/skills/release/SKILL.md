---
description: >-
  Release FineType — `model <dir>` publishes trained weights to HuggingFace and swaps the default symlink; `binary` bumps the workspace version, tags, and triggers the GitHub release workflow that produces cross-platform binaries + Homebrew tap update; no-arg form does both. Network + git push, so run pre-flight checks first and stop on any failure.
when_to_use: User says "release", "ship", "cut a release", "publish the model", "tag a version", or names a model dir and asks to upload it. Treat as a deliberate, reviewed action — never auto-fire mid-task.
argument-hint: "[model <model-dir> | binary]"
arguments: mode target
allowed-tools: Bash, Read, Edit
---

# /release

Two release types: **model** (publish trained weights to HuggingFace) and **binary** (cut a GitHub release with cross-platform binaries + Homebrew update).

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
   - `config.json` (required — architecture config)
   - `label_map.json` (required — class index mapping)
   - `results.json` (optional — training metrics)

2. **Tests pass** — `cargo test -p finetype-model` must pass with zero failures.

3. **Zero warnings** — `cargo build --workspace 2>&1 | grep "^warning\[" | head -1` must be empty (build.rs info lines are fine).

4. **Config sanity** — read `config.json` and print key fields: `n_classes`, `activation`, `head_type`. Confirm `n_classes` matches the taxonomy count (`239` as of current taxonomy).

5. **Current default** — read `models/default` symlink and show what's being replaced.

Present the pre-flight summary and wait for confirmation before proceeding.

### 2. Upload to HuggingFace

Upload the three required files to `meridian-online/finetype-model`:

```python
from huggingface_hub import HfApi
api = HfApi()

for file in ["model.safetensors", "config.json", "label_map.json"]:
    api.upload_file(
        path_or_fileobj=f"{model_dir}/{file}",
        path_in_repo=f"{model_name}/{file}",
        repo_id="meridian-online/finetype-model",
        repo_type="model",
        commit_message=f"Add {model_name}/{file}"
    )
```

Where `model_name` is the directory basename (e.g. `sherlock-v11`).

After upload, verify the files are accessible:
```bash
curl -sfI "https://huggingface.co/meridian-online/finetype-model/resolve/main/{model_name}/model.safetensors"
```

### 3. Update Default Symlink

```bash
ln -sfn {model_name} models/default
```

### 4. Update Card & CLAUDE.md

If eval results exist (check `<model_dir>/eval/` or `eval/eval_output/profile_results.json`):

1. **Card 0002** (`.orbit/cards/0002-semantic-type-detection.yaml`) — update the `goal:` field with the new eval baseline
2. **CLAUDE.md** — update the `Default model:` line and `Profile eval:` numbers in both Current State and Evaluation infrastructure sections

If no eval results exist, note this and skip — the eval must run before CLAUDE.md can be updated.

### 5. Commit & Push

Stage and commit:
```bash
git add models/default .orbit/cards/0002-semantic-type-detection.yaml CLAUDE.md
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

1. **CI is green** — all jobs on the current main branch must pass.
2. **No uncommitted changes** — `git status` must be clean.
3. **Tests pass locally** — `cargo test -p finetype-model` must pass.
4. **Zero warnings** — `cargo build --workspace 2>&1 | grep "^warning\[" | head -1` must be empty.

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

### Rollback

**Model rollback:**
1. Revert the `models/default` symlink to the previous model
2. Commit and push — CI and DuckDB extension will revert on next download
3. HuggingFace files can stay (old versions are still available by path)

**Binary rollback:**
1. Delete the GitHub release and tag if builds failed
2. If already published: cut a new patch release with the fix
3. Homebrew users get the fix on next `brew upgrade`
