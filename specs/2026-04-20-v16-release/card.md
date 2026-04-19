# v0.6.17 — Release sherlock-v16 as default model

**Status:** open
**Created:** 2026-04-20
**Depends on:** PR merging `feat/v16-retrain` (training-pipeline fixes for m-18)
**Blocks:** v16 accuracy gains (235/242) reaching users

## Context

sherlock-v16 is trained, scored, and sitting in `models/sherlock-v16/`
(seed 43, 235/242 = 97.1% on the corrected profile eval, +2 over v14).

The m-18 spec explicitly scoped itself as **train + evaluate**, not
**release**. When we attempted to promote v16 as part of that PR, CI
broke because:

- `.github/scripts/download-model.sh` reads `models/default` at CI time
  and `curl`s the pointed-to model from HuggingFace
- sherlock-v16 isn't on HuggingFace yet (publishing is a release task)

So the m-18 PR ships only the training-pipeline fixes with
`models/default → sherlock-v14`. This card covers the actual promotion.

## Expected behaviours

1. `models/default` symlink points to `sherlock-v16`
2. sherlock-v16 is downloadable from HuggingFace
   (`https://huggingface.co/meridian-online/finetype-model/resolve/main/sherlock-v16/...`)
3. `eval/eval_output/report.md` reflects v16 numbers (235/242, 97.1% label)
4. Golden tests pass with v16 predictions (e.g., `ecommerce_orders/phone →
   identity.person.phone_number` instead of v14's `→ identity.government.ssn`)
5. `finetype --version` prints `0.6.17`
6. GitHub release published with cross-platform binaries (Linux x86/arm,
   macOS x86/arm, Windows)
7. Homebrew tap bumped to 0.6.17

## Work items

Run the `/release` skill, plus these pre-release tweaks:

- [ ] `ln -sfn sherlock-v16 models/default` — commit the flip
- [ ] `cargo test -p finetype-cli --test cli_golden -- --ignored` — verify
      all goldens pass; update assertions that encoded v14 bugs
      (known: `ecommerce_orders/phone` expects `phone_number` not `ssn`)
- [ ] `make eval-report` — regenerate `eval/eval_output/report.md` from v16
- [ ] Package `models/sherlock-v16/` as tar.gz + SHA256 manifest
- [ ] Upload model artefacts to HuggingFace
      (`meridian-online/finetype-model`, path `sherlock-v16/`)
- [ ] Bump `Cargo.toml` workspace version 0.6.16 → 0.6.17
- [ ] Update `CLAUDE.md` **Default model** block back to v16
- [ ] Tag release `v0.6.17`, push — trigger GitHub Actions release pipeline
- [ ] Update Homebrew formula in `meridian-online/homebrew-tap`
- [ ] Announce (crates.io publish happens automatically via CI)

## Notes

- v16 has 7 known errors on the corrected eval (listed in CLAUDE.md
  "What's next"). These are acceptable; follow-up data-quality card
  will tackle them post-release.
- Decision 0049 documents why synthetic data is retained for the 7
  bad-distilled types (SSN, HTTP method, SWIFT BIC, etc.).
- CI hygiene: after this card ships, consider decoupling CI from
  `models/default` (fetch a specific pinned model name at CI time
  instead) so future promotion PRs don't have the same deadlock.
