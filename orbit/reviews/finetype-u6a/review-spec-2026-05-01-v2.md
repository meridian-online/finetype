# Spec Review

**Date:** 2026-05-01
**Reviewer:** Context-separated agent (fresh session)
**Bead:** finetype-u6a
**Verdict:** REQUEST_CHANGES

---

## Review Depth

| Pass | Triggered by | Findings |
|------|-------------|----------|
| 1 — Structural scan | always | 3 |
| 2 — Assumption & failure | Pass 1 found a HIGH unsatisfiable AC + content signal (release-time CI gate, cross-system boundary) | 2 |
| 3 — Adversarial | Pass 2 surfaced a hidden-subcommand cascade affecting both AC-02 and the drift script's correctness | 1 |

## Cycle-1 → Cycle-2 progress

The cycle-1 review (`review-spec-2026-05-01.md`) raised 8 findings. The revised bead description adequately resolves the following:

- HIGH `load`-step contradiction → AC-01 now names the post-0070/0071 surface (`profile → profile -o json-schema → validate --db --table`) and explicitly forbids `load` references.
- HIGH drift-detection gap → AC-02 now names a "script or release-time check" diffing skill content against `--help`.
- MEDIUM AC-01 verification → AC-01 now names "transcript fixture or manual checklist."
- MEDIUM scope-vs-goal → "Current state" stanza explicitly enumerates the four-item delta.
- MEDIUM cross-runtime portability → "Scope constraints" explicitly pins target runtime to Claude Code's skill loader and defers other runtimes to a separate bead.
- MEDIUM CLI skill stale-banner → AC-02 explicitly requires "v0.6.19".
- LOW rollback/regression → absorbed by the new drift AC in AC-02.
- LOW AC-03 implicit verification → AC-03 now embeds a literal grep command.

The findings below are new defects in the cycle-2 spec (they did not exist in cycle 1 because the relevant AC text didn't exist).

---

## Findings

### [HIGH] AC-03's grep verification will fail on the current skill content
**Category:** test-gap
**Pass:** 1
**Description:** AC-03 specifies that verification is `grep -r 'MCP\|api[._]key\|config_required' .claude/skills/finetype-{pipeline,cli}/` returning no matches. Run against the current `.claude/skills/finetype-cli/SKILL.md`, that grep returns three matches — all benign references to the `mcp` *subcommand* and the MCP server's tool surface. Because `mcp` is a real, documented subcommand of the CLI, scrubbing the literal token to satisfy this grep would either remove load-bearing documentation (violating AC-02's "every command" requirement) or push the implementer into awkward camouflage (e.g., spelling it `M-C-P`).

The intent of AC-03 is clearly "does the skill require any external setup?" — but the chosen test conflates *mention* of MCP with *requirement* of MCP. That is a false positive trap that will either (a) block the AC indefinitely, or (b) drive an implementer to weaken the skill content to satisfy a literal grep.

**Evidence:**
- `grep -rn 'MCP\|api[._]key\|config_required' .claude/skills/finetype-pipeline/ .claude/skills/finetype-cli/` against the current tree returns:
  - `finetype-cli/SKILL.md:65` — "MCP `schema` tool's type-key branch is retained for v0.6.19"
  - `finetype-cli/SKILL.md:204` — "Start MCP server for AI agent integration (stdio transport)" (the `finetype mcp` subcommand description)
  - `finetype-cli/SKILL.md:210` — "Runs as a stdio MCP server exposing: `infer`, `profile`, `schema`, …"
- `finetype --help` confirms `mcp` is a top-level subcommand the CLI skill must document per AC-02.
- The bead description's intent line ("no MCP server, API key, or external configuration") describes *external dependency*, not *string mention*.

**Recommendation:** Replace the grep with a verification that targets *requirement* rather than *mention*. Two viable forms:
1. **Manual / inline assertion**: "Verified by inspection: neither SKILL.md instructs the agent to install or configure an MCP server, set an API key, or provide external configuration before the skill loads." This is a checklist item, not a script.
2. **Tightened grep targeting setup verbs**, e.g. `grep -rEn '(claude mcp add|ANTHROPIC_API_KEY=|export .*_API_KEY|mcp[ -]install|configure.*mcp)' .claude/skills/finetype-{pipeline,cli}/` returning zero matches. This catches setup instructions while leaving subcommand documentation alone.

Pick (1) if you want a low-ceremony checklist; pick (2) if you want a CI-runnable assertion.

### [MEDIUM] AC-02's "matches finetype --help" is ambiguous about hidden subcommands
**Category:** constraint-conflict
**Pass:** 2
**Description:** `finetype --help` (v0.6.19) lists exactly 5 top-level subcommands: `infer`, `taxonomy`, `validate`, `profile`, `mcp`. But `generate` and `check` are still functional subcommands — `finetype generate --help` and `finetype check --help` both succeed and emit complete help. They are *hidden* (likely `#[command(hide = true)]` or absent from the user-facing top-level listing) but not removed. The current CLI skill correctly documents both.

AC-02 says the skill "matches `finetype --help` and per-subcommand `--help` output." A literal reading creates a contradiction:
- If the diff iterates only over commands listed in top-level `--help`, `generate` and `check` are never checked — drift in their flags will not be caught.
- If the diff iterates over the union of top-level and per-subcommand help, the implementer first has to *enumerate* hidden subcommands somehow — `--help` won't reveal them.
- A naive interpretation ("anything in the skill that isn't in top-level `--help` is obsolete") would flag `generate`/`check` as drift to be removed, regressing skill coverage.

**Evidence:**
- `finetype --help` output shows 5 commands; `finetype generate --help` and `finetype check --help` both return rich help text.
- Current `.claude/skills/finetype-cli/SKILL.md` has dedicated sections for `generate` (lines 164-179) and `check` (lines 183-199).
- AC-02 says "command/flag inventory matches `finetype --help` and per-subcommand `--help`" — silent on whether `generate`/`check` are in scope.

**Recommendation:** Spell out the source-of-truth enumeration. Two options:
1. **Source-of-truth = clap command tree**: drift script enumerates subcommands from `clap`'s registered command list (cargo-built binary can dump this; or use `--bash` shell completion which exposes hidden commands). Documents *all* subcommands including hidden.
2. **Source-of-truth = curated allowlist**: drift script takes an explicit list of `{infer, taxonomy, validate, profile, mcp, generate, check}` and diffs each. Simpler; requires manual update when subcommands are added.

Either works; the AC needs to pick one so the implementer doesn't regress `generate`/`check` coverage.

### [MEDIUM] AC-02 conflates "refresh now" and "drift-check forever" in one criterion
**Category:** missing-requirement
**Pass:** 1
**Description:** AC-02 has three sub-requirements bundled into one bullet: (a) banner says v0.6.19, (b) inventory matches `--help` *today*, (c) a verification step runs *on every release*. (a) and (b) are one-shot edits; (c) is infrastructure (a script + a CI/release hook). Bundling them into a single AC makes it unclear whether passing the AC requires the release-time hook to be wired into the existing release workflow (`/.github/workflows/release.yml`) or just for the script to exist on disk. The implementer needs to know whether they are also editing the release workflow.

Concretely: today's CLI skill is missing AC-02-required content — `validate --db`, `validate --table`, `validate --append`, `validate --lenient` flags are not documented; `profile`'s `--enum-threshold` default is wrong (skill says 50, binary defaults to 32); `profile`'s `model-type` default is wrong (skill says `char-cnn`, binary defaults to `multi-branch`); `profile`'s `--sharp-only` flag in the skill is not in current `--help` output; `profile`'s `--stats` flag is missing from the skill. Refreshing these is a discrete content task; building the harness is a discrete infra task. They're often best as two ACs.

**Evidence:**
- `.claude/skills/finetype-cli/SKILL.md` flag tables vs `finetype profile --help` and `finetype validate --help` output (per above) — at least 5 concrete divergences.
- Existing release workflow `/.github/workflows/release.yml` has no skill-related step today.
- Existing pattern `/.github/scripts/check-ci-model-drift.sh` is precedent for a release-time check, suggesting where the new script belongs.

**Recommendation:** Split AC-02 into two:
- **AC-02a "CLI reference current at v0.6.19"**: banner names v0.6.19; the documented command/flag inventory equals the v0.6.19 binary's `--help` surface (enumerate the corrections explicitly: add `--db/--table/--append/--lenient` to validate; fix `--enum-threshold` default; fix `--model-type` default; remove `--sharp-only`; add `--stats`; etc.).
- **AC-02b "Drift-protected"**: a script (path: `.github/scripts/check-skill-drift.sh` or similar, mirroring `check-ci-model-drift.sh`) compares skill content against `finetype --help` + per-subcommand `--help`, fails on divergence, and is wired into the release workflow as a required job.

Naming the script path and the workflow integration point in AC-02b removes implementation guesswork.

### [MEDIUM] AC-01's "transcript fixture or manual checklist" leaves verification scope undefined
**Category:** test-gap
**Pass:** 2
**Description:** AC-01 says verification is "a transcript fixture or manual checklist showing the agent invokes all three commands in sequence on a representative CSV." The "or" makes this two very different artifacts: a transcript fixture is a captured agent session committed to the repo (likely under `.claude/skills/finetype-pipeline/` or `tests/`); a manual checklist is documentation a human follows at release time. The cost, durability, and regression-protection of these two options differ by an order of magnitude.

A transcript fixture also faces a model-stability question: agent transcripts captured against Sonnet today may not reproduce on a future model, and the fixture has no oracle distinguishing "skill content drift broke the agent" from "model behaviour changed." A manual checklist sidesteps this but provides no automated regression signal.

**Evidence:**
- AC-01 verification clause: "transcript fixture **or** manual checklist."
- No path or filename specified for either artefact.
- No source-of-truth named for "representative CSV."

**Recommendation:** Pick one and name the artefact. Suggested defaults:
- If checklist: store as `.claude/skills/finetype-pipeline/MANUAL-VERIFICATION.md` with the three commands listed and a checkbox for the verifier; reference it from the SKILL frontmatter or a release runbook.
- If transcript fixture: name the input CSV (e.g., `eval/datasets/contacts/contacts.csv`), commit the captured transcript under `.claude/skills/finetype-pipeline/transcripts/`, and add a CI job that grep-asserts the three commands appear in transcript order. Acknowledge the model-stability caveat.

### [LOW] Hidden cross-system boundary — release workflow modification not declared in scope
**Category:** content-signal
**Pass:** 2
**Description:** AC-02's "release-time check" implies a change to either `.github/workflows/release.yml`, `.github/workflows/ci.yml`, the `Makefile`'s release/CI target, or a new workflow file. The bead description's "Scope constraints" enumerate what is *out of scope* (multi-runtime portability, MADRs) but does not declare release-pipeline modification *in scope*. The cycle-1 review didn't catch this because cycle-1 AC-02 had no release-time clause.

**Evidence:**
- `.github/workflows/release.yml` contains no skill-related job today.
- AC-02 says "verification step (script or release-time check) … on every release" — implies workflow modification.
- Bead "Scope constraints" stanza doesn't declare CI/release file edits in scope.

**Recommendation:** Add a one-liner under "Scope constraints": "In scope: editing `.github/workflows/release.yml` (or equivalent) to wire the drift check into the release pipeline."

### [MEDIUM] Cascade — drift script gives false confidence if hidden-subcommand handling is wrong
**Category:** failure-mode
**Pass:** 3
**Description:** Combining the AC-02 ambiguity (above) with the AC-02-bundling issue produces a realistic failure mode: an implementer ships a drift script that iterates over top-level `finetype --help` output only. The script passes (no drift among the 5 visible commands), the release ships, and `generate`/`check` documentation silently rots. The next time someone runs `finetype check` from the skill's instructions and the flag has changed, they hit a confusing error. The drift script "passed" — providing false assurance — and the regression isn't detected until a user complains.

This is a case where a check that almost-works is worse than no check, because it discourages the manual cross-check that would otherwise happen.

**Evidence:**
- Hidden subcommands `generate`/`check` exist (verified above).
- AC-02 doesn't pin enumeration method.
- Existing pattern `check-ci-model-drift.sh` provides no template for subcommand enumeration — it checks model file hashes, not CLI surface.

**Recommendation:** Resolved by adopting the AC-02 split + enumeration choice in the [MEDIUM] AC-02 ambiguity finding. Specifically: pick the curated-allowlist approach if the team wants determinism, or the clap-tree-dump approach if the team wants automatic coverage. Either way, the AC must say so — not leave it to implementer judgement.

---

## Honest Assessment

The bead has improved substantially since cycle 1: the `load`-verb contradiction is gone, the cross-runtime ambiguity is resolved, the goal now describes a concrete delta, and AC-02 has acquired a drift-protection clause. Six of eight prior findings are cleanly addressed.

The remaining issues are concentrated in the new AC text. AC-03's grep is the only HIGH — the chosen verification command will return false positives against legitimate documentation of the `mcp` subcommand, making the AC unsatisfiable without weakening the skill. AC-02 has bundled three deliverables (banner refresh, content refresh, drift harness) into one criterion and is ambiguous about hidden subcommands; both are tractable splits. AC-01's verification artefact is under-specified — the implementer needs to know whether they're committing a transcript or writing a checklist.

The biggest risk is the AC-02 hidden-subcommand cascade: if the drift script is built without considering `generate` and `check`, the harness will appear to work while silently allowing rot in two real subcommands. That's worse than no harness.

After resolving the [HIGH] AC-03 grep and splitting AC-02 (with the enumeration method named), the spec is implementable in a focused PR — content refresh + small CI script + workflow wiring + a verification artefact for AC-01.
