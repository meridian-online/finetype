# Release & Model Promotion

Reference for promoting a new model and cutting a release.

## Runtime dependency: the `duckdb` CLI (choice 0100, v0.6.32)

`profile` and `validate` shell out to the external `duckdb` CLI for all
CSV/Parquet ingestion, so it is a **hard runtime dependency** (on PATH). The
Homebrew formula template (`.github/workflows/release.yml`) declares
`depends_on "duckdb"`. This is a **shell-out, not a link** — the cross-platform
release build is unchanged (no `libduckdb` compile), so the Windows/MSVC
amalgamation risk that applies to a `duckdb` *pin bump* (see the binary-release
pre-flight) does **not** apply to ingestion. Any CI job that runs `profile`/
`validate` end-to-end must install the `duckdb` CLI (the smoke job does).

## Promotion flow (new model → release)

After the v0.6.17 release we decoupled CI from the `models/default` symlink (see `.orbit/specs/2026-04-20-ci-decouple-default-symlink/`). The 3-step flow:

1. **Publish to HuggingFace** — upload the trained model directory to `meridian-online/finetype-model` on HF.
2. **Bump `FINETYPE_CI_MODEL`** in `.github/workflows/ci.yml` and `.github/workflows/release.yml` (workflow-level `env:` blocks).
3. **Flip `models/default`** — `ln -sfn <new-model> models/default`.

Steps 2 and 3 may ship in the same PR. Step 1 must precede step 2 (or step 2 can be deferred if the promotion is purely a runtime change).

**Quality gates before any of this.** A candidate clears the promotion-order scoreboard *before* the flip (CLAUDE.md "Promotion order"): gold-anchor → drift proxy → gold + rare-type scoreboard → **representative accuracy (advisory)** → corpus-honest gate (**blocking**). The representative band (`eval/repr/representative_corpus.tsv`, scored `score_gold_anchor.py … --reframe`) is reported alongside gold and flags an advisory drop on the candidate-vs-v19 delta; it never blocks on its own. Only gold + the corpus-honest relocation gate block. See the spec `2026-06-18-representative-accuracy-gate`.

A non-blocking drift check (`.github/scripts/check-ci-model-drift.sh`) warns in CI when `FINETYPE_CI_MODEL` and `models/default` disagree — legitimate during promotion PRs, but visible so divergence isn't silent for weeks.

See also: `DEVELOPMENT.md` for the three model-name env vars (`FINETYPE_CI_MODEL`, `FINETYPE_MODEL`, `FINETYPE_MODEL_DIR`) — each read by exactly one consumer.
