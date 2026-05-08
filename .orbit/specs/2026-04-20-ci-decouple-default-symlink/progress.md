# Implementation Progress

**Spec:** `.orbit/specs/2026-04-20-ci-decouple-default-symlink/spec.yaml` (v1.1)
**Started:** 2026-04-20
**Completed:** 2026-04-20
**Branch:** `ci-decouple-default-symlink`

## Hard Constraints

- [x] Surgical changes only — download-model.sh resolution block replaced (~20 lines), rest untouched
- [x] Runtime behaviour of `models/default` UNCHANGED — symlink remains, readlink/cat fallback preserved
- [x] No new CI secrets introduced — workflow-level `env:` only
- [x] Local dev workflows unaffected on bootstrapped machines — tested: `unset FINETYPE_CI_MODEL` resolves via symlink
- [x] `${FINETYPE_CI_MODEL:-...}` (colon-dash) used — empty string treated as unset (verified by test case b + d)
- [x] Env var value is a model directory name, not a URL
- [x] Empty/whitespace resolution exits non-zero with clear error before any curl (verified by test cases c + d)
- [x] `FINETYPE_CI_MODEL` read ONLY by `download-model.sh` — no other consumer references it
- [x] Set at workflow-level `env:` — one key per workflow file (grep -c: ci.yml=1 env key, release.yml=1 env key)
- [x] Env propagates to fork PRs via workflow file (hardcoded, not secret)
- [x] Verification does NOT mutate `models/default` — test harness uses temp dirs, drift check is read-only
- [x] Changes confined to: `download-model.sh`, `ci.yml`, `release.yml`, `CLAUDE.md`, new test + drift scripts, handover back-ref

## Acceptance Criteria

- [x] **AC-1**: `download-model.sh` resolves with precedence + `${VAR:-...}` + empty/whitespace error. Test harness (`test-download-model.sh`) covers all 4 cases — 4/4 passing locally.
- [x] **AC-2**: `ci.yml` sets `FINETYPE_CI_MODEL` at workflow-level `env:` exactly once (line 14). All 4 jobs (Test, Clippy, CLI Smoke Tests, Taxonomy Check) inherit automatically.
- [x] **AC-3**: `release.yml` sets `FINETYPE_CI_MODEL` at workflow-level `env:` exactly once. All 5 matrix legs inherit. Windows leg no longer needs the symlink checkout.
- [x] **AC-4**: Ordering-independence verified via AC-1 harness (runs in CI as its own job `download-model-test`) + the `check-ci-model-drift.sh` drift warning. `models/default` untouched by verification.
- [x] **AC-5**: `CLAUDE.md` has a new `## Release & Model Promotion` section with the 3-env-var disambiguation table + 3-step promotion flow. `handover.md` back-reference added.
- [x] **AC-6**: Local dev unaffected — resolution test with env unset resolves to `models/default → sherlock-v16`. Fresh clone path unchanged (still runs download-model.sh with symlink fallback).
- [x] **AC-7**: `check-ci-model-drift.sh` emits `::warning::` when env var and symlink disagree. Wired as a new `drift-check` job in `ci.yml`. Smoke-tested locally: aligned=quiet, drift=warning, unset=quiet.

## Deliverables

```
| File                                               | Status                                    |
|----------------------------------------------------|-------------------------------------------|
| .github/scripts/download-model.sh                  | Modified: resolution block replaced       |
| .github/scripts/test-download-model.sh             | New: 4-case test harness (4/4 passing)    |
| .github/scripts/check-ci-model-drift.sh            | New: non-blocking drift warning           |
| .github/workflows/ci.yml                           | Added env: + 2 new jobs (test + drift)    |
| .github/workflows/release.yml                      | Added workflow-level env:                 |
| CLAUDE.md                                          | New "Release & Model Promotion" section   |
| .orbit/specs/2026-04-20-v16-release/handover.md           | Back-reference added                      |
```

## Test results

```
.github/scripts/test-download-model.sh:
  [PASS] a_env_var_wins
  [PASS] b_empty_env_falls_back
  [PASS] c_unresolvable_errors_loudly
  [PASS] d_whitespace_is_unresolvable
Results: 4 passed, 0 failed

.github/scripts/check-ci-model-drift.sh smoke tests:
  aligned (env=v16, symlink=v16) → silent success
  drift    (env=v99, symlink=v16) → ::warning:: annotation
  unset    (env unset)            → silent skip

Shell syntax (bash -n): all three scripts parse cleanly
YAML syntax: ci.yml + release.yml load cleanly via PyYAML
cargo fmt --all --check: pass (no Rust touched)
```

## Next step

`/orb:review-pr` to verify the implementation against the 7 ACs before merge.
