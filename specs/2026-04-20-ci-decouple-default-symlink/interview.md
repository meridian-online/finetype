# CI hygiene — decouple download-model.sh from `models/default` (interview)

**Status:** interview
**Created:** 2026-04-20
**Priority:** medium (papers over friction, not blocking users)
**Estimate:** 1–2 hours
**Paired spec:** `spec.yaml` — option A chosen (env-var pin, smallest surface area)

## Problem

CI's release and build pipelines read `models/default` at workflow
time and fetch the pointed-to model from HuggingFace:

```bash
# .github/scripts/download-model.sh  (line 10)
MODEL_DIR=$(readlink models/default 2>/dev/null || cat models/default)
curl -sfL "https://huggingface.co/meridian-online/finetype-model/resolve/main/${MODEL_DIR}/..."
```

This creates a **circular dependency between model promotion and
model publication**:

1. To promote a new model, flip `models/default → sherlock-vNN`
2. For CI to pass, `sherlock-vNN/` must already exist on HuggingFace
3. But we usually only publish to HuggingFace as part of the release

In the v0.6.17 release we worked around this with a "HuggingFace
first, then PR" dance (see `specs/2026-04-20-v16-release/card.md`).
That works but is fragile — it's easy to forget, the intermediate
state of the PR is inconsistent (symlink flipped but CI failing),
and any future rollback would hit the same problem in reverse.

## Expected behaviours

1. A promotion PR can land **before** the new model is on HuggingFace
   (or the two can ship atomically via a merge queue) without CI
   being broken by the intermediate state.
2. `models/default` remains the *runtime* default (used by the CLI
   and DuckDB extension). CI should not hinge on it.
3. Release workflow continues to produce correctly-bundled binaries.
4. Local dev (`cargo test`, `./scripts/eval.sh`) is unaffected.

## Options to consider (interview/design will pick)

- **A. Pin CI to a specific model name via workflow env var.**
  e.g. `FINETYPE_CI_MODEL=sherlock-v16` set in `ci.yml` and
  `release.yml`. `download-model.sh` reads that env var first, falls
  back to `models/default`. Promotion = two PRs (bump env var +
  publish) or one PR where the env var bump lands with the HF
  publish.
- **B. Publish-aware fetch.** `download-model.sh` probes HF for the
  `models/default` target; if 404, falls back to a pinned "last known
  good" (itself another env var). Keeps `models/default` as the
  source of truth but tolerates unpublished intermediate states.
- **C. Pinned model file.** Dedicated `models/ci-pinned.txt`
  containing just a model name, read by CI only. `models/default`
  handles runtime. Promotion flow still needs HF publish first, but
  now the split of "live" vs "CI baseline" is explicit in the
  filesystem.
- **D. HuggingFace publish as a CI step, not a manual step.**
  Release workflow publishes the model to HF before it runs
  download-model.sh on the tag. Requires HF write token in CI
  secrets. Arguably the cleanest — makes the release pipeline
  truly self-contained.

## Non-goals

- NOT rewriting `download-model.sh` from scratch. Surgical changes.
- NOT changing the runtime behaviour of `models/default`.
- NOT automating HuggingFace publish for every commit (D is only
  for tagged releases).

## Entry points

- `.github/scripts/download-model.sh` — the file that reads the symlink
- `.github/workflows/ci.yml` — calls download-model.sh for Test / Clippy / CLI Smoke Tests / Taxonomy Check
- `.github/workflows/release.yml` — calls download-model.sh per platform build

## Resolution

After weighing options on simplicity, ordering-independence, secret-
management surface area, and future flexibility:

- **Chosen: A (env-var pin).** Smallest surgical change, zero new
  secrets, keeps `models/default` as runtime source of truth.
  Promotion flow becomes: (1) publish to HF, (2) bump `FINETYPE_CI_MODEL`
  in the workflow file, (3) flip `models/default` — or steps 2 and 3
  can ship in the same PR. CI no longer needs a specific ordering.
- **Deferred: D (HF publish as CI step).** Cleaner long-term but needs
  a HuggingFace write token in CI secrets and changes the release
  workflow's trust boundary. Revisit after A proves insufficient.
- **Rejected: B (publish-aware fetch with fallback).** Silent fallback
  hides promotion bugs — a failed HF publish should fail CI loudly,
  not quietly roll back to the "last known good" model.
- **Rejected: C (separate pinned file).** Adds a second source of
  truth for "what model is current" without clear benefit over A.

See `spec.yaml` for the implementation contract.
