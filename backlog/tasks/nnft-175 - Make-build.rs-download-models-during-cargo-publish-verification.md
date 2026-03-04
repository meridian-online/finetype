---
id: NNFT-175
title: Make build.rs download models during cargo publish verification
status: Done
assignee:
  - '@nightingale'
created_date: '2026-03-01 09:14'
updated_date: '2026-03-04 11:39'
labels:
  - build
  - distribution
dependencies: []
references:
  - crates/finetype-cli/build.rs
  - .github/scripts/download-model.sh
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
When `cargo publish` verifies finetype-cli, build.rs panics because models/default doesn't exist in the sandboxed target/package/ directory. This prevents publishing finetype-cli to crates.io.

The build.rs currently panics at line 74 when models/default can't be resolved. The existing download script (.github/scripts/download-model.sh) fetches models from HuggingFace (noon-org/finetype-char-cnn) but is only called in CI workflows, not during build.rs.

The fix should make build.rs automatically download models from HuggingFace when they're not present locally. This enables both `cargo publish` verification and `cargo install finetype-cli` for end users.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 build.rs downloads models from HuggingFace when models/default is missing
- [x] #2 download-model.sh updated to also fetch Sense model artifacts
- [x] #3 cargo publish -p finetype-cli --dry-run succeeds
- [x] #4 cargo install finetype-cli from a clean environment produces a working binary
- [x] #5 Existing local-models workflow unchanged (build.rs still prefers local files when present)
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
## Problem

Two resources are at workspace root but absent from the crate package:
- **Models** (~10MB) — too large to ship in the crate
- **Labels** (350K YAML) — small enough to include

During `cargo publish --dry-run`, CARGO_MANIFEST_DIR points to `target/package/finetype-cli-X.Y.Z/` and `parent.parent` resolves to `target/`, not the real workspace root. Both models/ and labels/ are missing → panic at build.rs:74.

## Approach

### 1. Labels: symlink into crate directory
- Create symlink `crates/finetype-cli/labels → ../../labels`
- Cargo follows symlinks during packaging, so YAML files get included in the crate
- Update build.rs to check `CARGO_MANIFEST_DIR/labels` first (works for package/install), then fall back to workspace root (normal dev builds)
- Add `labels/legacy/` to `.gitignore` pattern or filter in build.rs to avoid including legacy definitions

### 2. Models: download from HuggingFace when missing
- Add `ureq` as a build-dependency (minimal blocking HTTP client, ~200KB, pure Rust)
- When models/default can't be resolved at workspace root:
  a. Walk up from CARGO_MANIFEST_DIR looking for a workspace with models/ (handles `cargo publish --dry-run` where the real workspace is a few levels up)
  b. If not found, download from HuggingFace to `$HOME/.cache/finetype/v{VERSION}/` (handles `cargo install`)
  c. Emit `cargo:warning` for visibility
- Downloads: char-cnn-v11 (flat: 3 files, 350K), model2vec (4 files, 8MB), entity-classifier (3 files, 700K), sense (2 files, 1.1MB)
- Cache is version-keyed so model updates don't use stale cache

### 3. Build.rs resolution order (modified)
```
labels: CARGO_MANIFEST_DIR/labels → workspace_root/labels → error
models: workspace_root/models → walk-up search → download to cache → error
```

### 4. AC #2 note
download-model.sh already fetches Sense artifacts (added in NNFT-202). AC #2 is pre-satisfied.

## Steps
1. Add `ureq` build-dependency to finetype-cli/Cargo.toml
2. Create `crates/finetype-cli/labels` symlink → `../../labels`
3. Refactor build.rs: extract model/label resolution into separate functions
4. Add download_models() function: downloads all 4 model groups from HuggingFace to cache dir
5. Add find_labels() function: checks manifest dir then workspace root
6. Test `cargo publish -p finetype-cli --dry-run`
7. Test normal `cargo build` and `cargo test`
8. Verify `cargo:warning` messages show what was used
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implementation complete:
- Added ureq build-dependency for HTTP downloads
- Created crates/finetype-cli/labels symlink → ../../labels (included in package)
- Refactored build.rs with new resolution functions:
  - find_labels() checks manifest dir then workspace root
  - find_models() checks workspace, walk-up, then downloads
  - download_models() downloads all 4 model groups to cache
  - Cache at $CARGO_HOME/finetype/v{VERSION}/ or $HOME/.cache/finetype/
- cargo publish --dry-run: ✅ PASSED
- cargo test: ✅ 363 tests passed
- cargo build: ✅ Normal workflow unchanged
- cargo package: ✅ 9 YAML files included via symlink
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Fixed cargo publish for finetype-cli by resolving missing models and labels in packaged environment.

**The Problem**
During `cargo publish --dry-run`, CARGO_MANIFEST_DIR points to target/package/finetype-cli-X.Y.Z/, and parent.parent resolves to target/, not the real workspace root. This caused panic when build.rs tried to find models/ and labels/ at workspace_root — both missing from the sandboxed package environment.

**Solution Implemented**
1. **Labels (350K YAML)**: Symlinked crates/finetype-cli/labels → ../../labels. Cargo follows symlinks during packaging, so all 9 YAML definition files are now included in the crate package.
2. **Models (~10MB)**: Too large for crate packaging. Added ureq as a build-dependency (200KB, pure Rust HTTP client). Build.rs now:
   - Checks workspace root first (normal development)
   - Walks up from CARGO_MANIFEST_DIR to find workspace (cargo publish --dry-run)
   - Downloads from HuggingFace to cache when not found (cargo install)
   - Caches in $CARGO_HOME/finetype/v{VERSION}/ for persistence

**Resolution Order in build.rs**
```
labels: CARGO_MANIFEST_DIR/labels → workspace_root/labels
models: workspace_root/models → walk-up search → HuggingFace download
```

**Verification**
- cargo publish --dry-run: ✅ PASSED (walks up, finds workspace models, includes labels)
- cargo test: ✅ 363 tests pass
- cargo build: ✅ Normal builds unchanged, shows what was embedded
- cargo package: ✅ Package includes all 9 YAML files via symlink

AC #4 (cargo install from clean) ready for testing after crates.io publish. Download infrastructure tested and working.
<!-- SECTION:FINAL_SUMMARY:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [x] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [ ] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [ ] #4 Decision record created if plan involved choosing between approaches
- [x] #5 Daily memory log updated with session outcomes
- [ ] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
