# Spec Review — CI hygiene: decouple download-model.sh from `models/default`

**Date:** 2026-04-20
**Reviewer:** Context-separated agent (fresh session)
**Spec:** specs/2026-04-20-ci-decouple-default-symlink/spec.yaml
**Verdict:** REQUEST_CHANGES

---

## Summary of claim verification

Claims against code that I checked:

- AC-2 names four CI jobs that call `download-model.sh`: **Test, Clippy, CLI Smoke Tests, Taxonomy Check**. Verified against `.github/workflows/ci.yml` lines 22-65 — all four jobs exist and call the script at lines 30, 41, 52, 64. The `fmt` job does not. Accurate.
- AC-3 claims "five platform matrix jobs" in `release.yml`. Verified against `.github/workflows/release.yml` lines 20-36 — exactly five matrix entries (x86_64-linux, aarch64-linux, x86_64-darwin, aarch64-darwin, x86_64-windows). A single `build` job using a matrix, not five independent jobs, but each matrix leg invokes `download-model.sh` (line 50). Accurate in spirit.
- Interview quotes `MODEL_DIR=$(readlink models/default 2>/dev/null || cat models/default)` as "line 10". Verified — `download-model.sh` line 10. Accurate.
- Grep confirms the only five call sites of `download-model.sh` are the four ci.yml lines and the one release.yml line. No hidden invocations (e.g., no DuckDB-extension CI job that would also need the env var).

---

## Findings

### HIGH — AC-1's fallback semantics under `set -euo pipefail` are underspecified
**Category:** failure-mode
**Description:** `download-model.sh` runs under `set -euo pipefail` (line 5). The spec says the script "resolves the model name from `FINETYPE_CI_MODEL` first, falling back to `models/default` when the env var is unset or empty." The naive implementation `MODEL_DIR="${FINETYPE_CI_MODEL:-$(readlink models/default 2>/dev/null || cat models/default)}"` has two failure modes that are not called out:
  1. Under `set -u`, referencing an unset `FINETYPE_CI_MODEL` is safe only with `${VAR:-default}` form. `${VAR-default}` (no colon) treats empty string as "set" and would silently pass empty to the downstream `curl`, producing a request to `https://huggingface.co/meridian-online/finetype-model/resolve/main//model.safetensors` (double slash, HF 404). The spec must require the `:-` form so empty-string is treated as unset.
  2. If the env var is unset *and* `models/default` doesn't exist (neither as symlink nor file), the current script on line 10 emits nothing (both `readlink` and `cat` fail). Under `set -e`, `cat models/default` failing would terminate the script — but only the subshell. `MODEL_DIR` is set to the empty string, and line 13's `echo "Active model: "` prints a blank, line 14's `mkdir -p "models/"` creates a stray directory, and line 17's URL becomes malformed. This is pre-existing behaviour, but AC-1's phrasing ("fallback path is the exact current behaviour") implicitly preserves it. Either call this out as out-of-scope, or add an explicit empty-check with a loud error.
**Evidence:** `.github/scripts/download-model.sh` lines 5, 10-11, 13-17; spec AC-1 lines 19-29.
**Recommendation:** Add a constraint and AC-specific verification: "If both `FINETYPE_CI_MODEL` is unset/empty AND `models/default` is missing, script exits non-zero with a clear error message (not a malformed curl)." Add a shellcheck/lint step to verify `:-` is used. Include a unit test for the empty-env-var case.

### HIGH — AC-4's "throwaway test branch" verification is dangerous and under-defined
**Category:** test-gap
**Description:** AC-4 says to "flip `models/default` to `sherlock-v99`" to verify the env-var override works. This is a destructive test — if the author forgets to revert, or the PR is merged by accident, `main` has a dangling symlink that breaks local dev and prevents runtime model resolution for every user of the CLI/DuckDB/MCP. The "revert after verification" clause is verbal, not mechanical.
**Evidence:** Spec lines 54-64.
**Recommendation:** Replace with a safer, mechanical verification: (a) add a CI job (or script) that runs `download-model.sh` in a dry-run / check-only mode with `FINETYPE_CI_MODEL=sherlock-nonexistent-xxx` and asserts a clean 404; and with `FINETYPE_CI_MODEL=sherlock-v16` and asserts success. (b) Use a disposable fork or `workflow_dispatch` manual run, not `main`'s symlink. Specifically forbid mutating `models/default` for this verification.

### MEDIUM — Naming collision risk with existing `FINETYPE_MODEL` / `FINETYPE_MODEL_DIR`
**Category:** missing-requirement
**Description:** The codebase already uses `FINETYPE_MODEL` (CLI flag / env, per CLAUDE.md and `eval/profile_eval.sh`) and `FINETYPE_MODEL_DIR` (DuckDB extension). Introducing a third `FINETYPE_CI_MODEL` in the same namespace is fine but deserves a constraint: CI's env var must not accidentally shadow or leak into commands that read the other two. Today, `./scripts/eval.sh` respects `FINETYPE_MODEL` (passed as `--model`); if a future CI job sets both `FINETYPE_CI_MODEL` and `FINETYPE_MODEL`, the distinction must be clear. The spec mentions this indirectly in CLAUDE.md docs (AC-5) but doesn't enumerate the interaction.
**Evidence:** CLAUDE.md "Profile eval" paragraph; `eval/profile_eval.sh`; spec AC-5/AC-6.
**Recommendation:** Add a one-line constraint: "`FINETYPE_CI_MODEL` is read only by `.github/scripts/download-model.sh`. CLI binary, MCP server, DuckDB extension, and all eval scripts ignore it." AC-5 documentation should contain a small table distinguishing all three env vars.

### MEDIUM — AC-6 conflates cases; unclear whether constraints apply to fresh clones
**Category:** test-gap
**Description:** AC-6 claims "running `cargo test`, `make eval-report`, and `./scripts/eval.sh` without setting `FINETYPE_CI_MODEL` produces identical behaviour to today." Verification says "`unset FINETYPE_CI_MODEL; make ci` passes." But `make ci` on a fresh clone requires a model present at `models/<name>/` — it normally runs locally where the dev already has `models/default` populated. The spec does not address: in CI itself (fresh VM), the script *downloads* the model; locally (fresh clone), `make ci` would fail until the dev fetches the model via `download-model.sh` anyway. The "no change" claim is technically true but the verification doesn't cover the fresh-clone case.
**Evidence:** Spec AC-6 lines 78-86; `Makefile` `ci` target (not shown but referenced).
**Recommendation:** Rephrase AC-6's verification to "on an already-bootstrapped dev machine" and add a separate note that fresh-clone workflows are unchanged (still use `download-model.sh` with the symlink fallback path).

### MEDIUM — No observability / drift-detection for env-var vs symlink divergence
**Category:** missing-requirement
**Description:** Once CI is pinned to `FINETYPE_CI_MODEL=sherlock-v16` independent of `models/default`, there's nothing stopping the two from silently diverging for weeks. Someone could flip `models/default` to `sherlock-v17` without bumping the env var, CI passes green (testing against v16), local dev and production users are on v17. Golden tests, smoke tests, and profile-eval golden expectations are written against the symlinked model — they'd break or pass spuriously depending on which side drifted.
**Evidence:** Spec has no AC for "detect divergence." Only AC-5 mentions a 3-step promotion flow verbally.
**Recommendation:** Add an AC: "A lightweight CI check (job or script) warns when `FINETYPE_CI_MODEL` and the `models/default` readlink target disagree." This does not have to fail the build — a warning in job output is sufficient — but the divergence must be visible. Alternatively, require they match on `main` branch merges.

### LOW — AC-3 counts matrix legs as "jobs"
**Category:** assumption
**Description:** AC-3 says "all five platform matrix jobs (Linux x86/arm, macOS x86/arm, Windows) resolve the variable." Technically release.yml has *one* `build` job with a 5-entry matrix; setting the env var at `job.env` level automatically covers all legs, but setting it per-matrix-entry is different. The spec should be explicit about where the `env:` key lives (job-level vs matrix-include-level vs step-level). Setting it at the job or workflow level is simplest and most correct for this use case.
**Evidence:** `.github/workflows/release.yml` lines 14-50; spec AC-3 lines 48-51.
**Recommendation:** Clarify: "set `FINETYPE_CI_MODEL` at workflow `env:` level (covers both ci.yml and release.yml in one place)" — this is also fewer lines, satisfying the "Minimal surface" evaluation principle (weight 0.25).

### LOW — `readlink` vs `FINETYPE_CI_MODEL` on Windows
**Category:** failure-mode
**Description:** The existing comment at line 8-9 notes that Windows git checkouts may produce plain text instead of symlinks, which is why `cat` is the second-chance fallback. The env-var path sidesteps both `readlink` and `cat`, which is actually an improvement for Windows. But the spec should note this as an explicit benefit/test — the release.yml Windows build leg was previously dependent on git's symlink checkout behaviour and now will not be.
**Evidence:** `download-model.sh` lines 8-9; release.yml line 34-36 (windows-latest).
**Recommendation:** Either add a one-line note, or add to AC-3 verification: "Windows build leg fetches the env-var-pinned model even if `models/default` was checked out as plain text, corrupt, or absent."

### LOW — Interaction with `pull_request` events from forks
**Category:** assumption
**Description:** `ci.yml` triggers on `pull_request` from forks. GitHub propagates workflow-level `env:` to fork PRs (secrets are not propagated, but `env:` values hardcoded in the workflow file are fine). So the chosen design works for fork PRs — good. But the spec doesn't name this; a reviewer might worry. Worth one line to close the question.
**Evidence:** `ci.yml` lines 3-7.
**Recommendation:** Add a constraint or non-goal line: "No secrets are introduced; env var propagates to fork PRs via workflow file, confirmed."

---

## Honest Assessment

The plan is directionally correct and the design choice (option A, env-var pin) is reasonable. The spec is clear about goals and non-goals. But two things would bite during implementation if not fixed: (1) the shell semantics around `set -euo pipefail` + empty-string env var are not specified precisely enough to prevent a silent failure — the naive `${VAR-default}` vs `${VAR:-default}` distinction matters here and is exactly the kind of thing that passes "unit tests" and fails in production with a confusing 404; (2) AC-4's verification proposes mutating `models/default` on a branch, which is the sort of test that one day gets merged by mistake and breaks `main` for every user. Those two are concrete, fixable, and I'd want them addressed before implementation starts. The other findings (naming collision docs, divergence detection, workflow-level `env:` placement) are lower-priority polish. Overall: **REQUEST_CHANGES**, scope is maybe 30 minutes of spec tightening before the 1-2 hour implementation.
