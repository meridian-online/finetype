---
id: NNFT-175
title: Make build.rs download models during cargo publish verification
status: To Do
assignee: []
created_date: '2026-03-01 09:14'
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
- [ ] #1 build.rs downloads models from HuggingFace when models/default is missing
- [ ] #2 download-model.sh updated to also fetch Sense model artifacts
- [ ] #3 cargo publish -p finetype-cli --dry-run succeeds
- [ ] #4 cargo install finetype-cli from a clean environment produces a working binary
- [ ] #5 Existing local-models workflow unchanged (build.rs still prefers local files when present)
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [ ] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [ ] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [ ] #4 Decision record created if plan involved choosing between approaches
- [ ] #5 Daily memory log updated with session outcomes
- [ ] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
