# Release & Model Promotion

Reference for promoting a new model and cutting a release.

## Promotion flow (new model → release)

After the v0.6.17 release we decoupled CI from the `models/default` symlink (see `.orbit/specs/2026-04-20-ci-decouple-default-symlink/`). The 3-step flow:

1. **Publish to HuggingFace** — upload the trained model directory to `meridian-online/finetype-model` on HF.
2. **Bump `FINETYPE_CI_MODEL`** in `.github/workflows/ci.yml` and `.github/workflows/release.yml` (workflow-level `env:` blocks).
3. **Flip `models/default`** — `ln -sfn <new-model> models/default`.

Steps 2 and 3 may ship in the same PR. Step 1 must precede step 2 (or step 2 can be deferred if the promotion is purely a runtime change).

A non-blocking drift check (`.github/scripts/check-ci-model-drift.sh`) warns in CI when `FINETYPE_CI_MODEL` and `models/default` disagree — legitimate during promotion PRs, but visible so divergence isn't silent for weeks.

See also: `DEVELOPMENT.md` for the three model-name env vars (`FINETYPE_CI_MODEL`, `FINETYPE_MODEL`, `FINETYPE_MODEL_DIR`) — each read by exactly one consumer.
