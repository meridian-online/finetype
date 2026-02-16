---
id: NNFT-073
title: Release v0.1.4 with CharCNN v5 model and CI improvements
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-15 23:02'
updated_date: '2026-02-15 23:02'
labels:
  - release
dependencies: []
priority: high
---

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Version bumped to 0.1.4 in workspace Cargo.toml
- [x] #2 char-cnn-v5 model uploaded to HuggingFace
- [x] #3 CI/release workflows use dynamic model download via symlink
- [x] #4 Smoke test updated for v5 taxonomy (uri not url)
- [x] #5 CI pipeline passes 5/5 jobs
- [x] #6 Release pipeline builds 4 targets + creates GitHub release + updates Homebrew
- [x] #7 Pre-commit hook catches issues before push
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Released v0.1.4 — first release with CharCNN v5 model (168 types, 90% accuracy).

15 commits since v0.1.3, including:
- NNFT-055, 056, 059: Phone, address, excel format generators
- NNFT-063, 064, 066, 067: New taxonomy types, pattern-gated post-processing, header hints
- NNFT-071: CharCNN v5 model (159→168 classes)
- NNFT-072: CI fixes, pre-commit hook, `make ci` target

Release infrastructure improvements:
- Uploaded char-cnn-v5 to HuggingFace (noon-org/finetype-char-cnn)
- Extracted `.github/scripts/download-model.sh` — reads `models/default` symlink to auto-resolve model version
- CI and release workflows no longer hardcode model version
- Fixed smoke test assertion: `url` → `uri` to match v5 taxonomy
- Added pre-commit hook (.githooks/) running fmt + clippy + test

CI: 5/5 ✓ | Release: 4 platform builds ✓ | Homebrew tap updated ✓
https://github.com/noon-org/finetype/releases/tag/v0.1.4
<!-- SECTION:FINAL_SUMMARY:END -->
