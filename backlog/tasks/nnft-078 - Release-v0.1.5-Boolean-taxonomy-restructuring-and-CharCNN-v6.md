---
id: NNFT-078
title: 'Release v0.1.5: Boolean taxonomy restructuring and CharCNN v6'
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-16 07:37'
updated_date: '2026-02-16 07:52'
labels:
  - release
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Cut a v0.1.5 release including NNFT-075 (boolean taxonomy restructuring), NNFT-076 (small-integer disambiguation fix), CharCNN v6 model, and README updates.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Workspace version bumped to 0.1.5
- [x] #2 CHANGELOG.md updated with v0.1.5 section
- [x] #3 README type counts and model references updated to current values
- [x] #4 CharCNN v6 model uploaded to HuggingFace
- [x] #5 Git tag v0.1.5 created and pushed
- [x] #6 CI passes on tagged release
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Released v0.1.5 with boolean taxonomy restructuring and CharCNN v6.

Changes included:
- **NNFT-075**: Boolean taxonomy restructured from `technology.development.boolean` to `representation.boolean.{binary,initials,terms}`
- **NNFT-076**: Fixed boolean label mismatch bug, added small-integer ordinal disambiguation and 30+ header hints
- **NNFT-077**: Added early-development disclaimer to README
- **CharCNN v6**: Retrained with 169 classes, 89.15% accuracy
- **Smoke test fix**: URL classification accepts both `url` and `uri` labels

Release artifacts:
- Workspace version: 0.1.4 → 0.1.5
- CHANGELOG.md: full v0.1.5 section with breaking changes documented
- README.md: updated type counts (163→169), model references (v4→v6), test counts (155→213)
- HuggingFace: char-cnn-v6 model uploaded (model.safetensors, labels.json, config.yaml)
- GitHub Release: 4 platform binaries (linux x86_64/aarch64, macOS x86_64/aarch64)
- Homebrew formula auto-updated

CI: All 5 jobs pass (Format, Clippy, Test, Smoke Tests, Taxonomy Check)
Release: All 6 jobs pass (4 builds + Create Release + Update Homebrew)
<!-- SECTION:FINAL_SUMMARY:END -->
