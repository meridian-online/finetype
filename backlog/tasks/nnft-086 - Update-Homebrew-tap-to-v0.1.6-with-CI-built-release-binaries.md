---
id: NNFT-086
title: Update Homebrew tap to v0.1.6 with CI-built release binaries
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-17 09:31'
updated_date: '2026-02-17 11:20'
labels:
  - release
  - infrastructure
  - homebrew
dependencies: []
references:
  - ~/github/noon-org/homebrew-tap/Formula/finetype.rb
  - 'https://github.com/noon-org/finetype/releases/tag/v0.1.6'
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The Homebrew formula (noon-org/homebrew-tap) is stuck at v0.1.1. The v0.1.6 GitHub Release was created from a git tag only — no pre-built binary tarballs are attached.

To fix Homebrew properly:
1. Build release binaries for all target platforms
2. Upload them to the v0.1.6 GitHub Release
3. Update the Homebrew formula with new URLs and SHA256 hashes

Target platforms:
- aarch64-apple-darwin (macOS ARM)
- x86_64-apple-darwin (macOS Intel)
- aarch64-unknown-linux-gnu (Linux ARM)
- x86_64-unknown-linux-gnu (Linux x86)

The long-term solution is a GitHub Actions release workflow that automatically builds binaries and updates the formula on new tags. The short-term fix is to build binaries manually or via a one-off CI run.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Pre-built binary tarballs for all 4 target platforms uploaded to v0.1.6 GitHub Release
- [x] #2 Homebrew formula updated with v0.1.6 URLs and correct SHA256 hashes
- [x] #3 brew upgrade finetype installs v0.1.6 successfully
- [x] #4 GitHub Actions workflow created for automated release binary builds on future tags
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Root cause: v0.1.6 tag has models/default → char-cnn-v7, but HuggingFace only has up to v6. The download-model.sh script gets 404, causing all CI/release builds to fail.

Fix plan:
1. Upload char-cnn-v7 model to HuggingFace (noon-org/finetype-char-cnn repo)
2. Verify download-model.sh can fetch char-cnn-v7
3. Re-trigger the release pipeline (re-run or re-tag)
4. Verify Homebrew formula auto-updates via the Update Homebrew Formula job
5. Test `brew upgrade finetype` installs v0.1.6

Note: AC#4 (GitHub Actions workflow) is already met — release.yml exists and works (v0.1.3–v0.1.5 succeeded). The issue is purely a missing model upload.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Root cause: v0.1.6 tag had models/default → char-cnn-v7 but HuggingFace only had up to char-cnn-v6. The download-model.sh script got HTTP 404 (exit code 22/56), failing all CI and release builds.

Fix:
1. Uploaded char-cnn-v7 (model.safetensors, labels.json, config.yaml) to HuggingFace noon-org/finetype-char-cnn repo
2. Re-ran the v0.1.6 release workflow (run 22088944604) — all 6 jobs passed
3. Verified 8 binary assets on GitHub Release (4 platforms × tar.gz + sha256)
4. Homebrew formula auto-updated to v0.1.6 by the Update Homebrew Formula job
5. brew upgrade finetype → 0.1.6, finetype --version confirms 0.1.6

AC#4 was already met — release.yml existed and worked for v0.1.3–v0.1.5. The issue was purely a missing model upload to HuggingFace.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Fixed the v0.1.6 release pipeline and Homebrew formula update.

Root cause: The `models/default` symlink pointed to `char-cnn-v7` which hadn't been uploaded to HuggingFace. The CI/release `download-model.sh` script got HTTP 404, failing all builds since v0.1.6 was tagged.

Fix:
- Uploaded char-cnn-v7 model files to HuggingFace (noon-org/finetype-char-cnn)
- Re-ran the release workflow — all 4 platform builds + release creation + Homebrew formula update succeeded
- Homebrew formula now at v0.1.6 with correct URLs and SHA256 hashes for all 4 platforms

Verified:
- `brew update && brew upgrade finetype` → 0.1.6 installed successfully
- `finetype --version` → 0.1.6
- Inference smoke test passes
- GitHub Release has 8 assets (4 tarballs + 4 SHA256 files)
- Also re-ran CI on main to clear the same download failure
<!-- SECTION:FINAL_SUMMARY:END -->
