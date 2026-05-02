# Spec Review

**Date:** 2026-05-01
**Reviewer:** Context-separated agent (fresh session)
**Bead:** finetype-u6a
**Verdict:** REQUEST_CHANGES

---

## Review Depth

| Pass | Triggered by | Findings |
|------|-------------|----------|
| 1 — Structural scan | always | 4 |
| 2 — Assumption & failure | Pass 1 found a HIGH spec-vs-reality conflict + content signal (cross-system boundary with agent runtimes) | 3 |
| 3 — Adversarial | Pass 2 found cascading drift risk and an AC term contradicted by the codebase | 2 |

## Findings

### [HIGH] AC-01 names a `load` step that no longer exists in the pipeline
**Category:** constraint-conflict
**Pass:** 1
**Description:** AC-01 specifies the pipeline as `profile, schema, validate, load`, but the `finetype load` verb was removed in v0.6.19 (MADR 0070/0071). The actual modern pipeline is `profile → schema → validate (--db --table)`, where materialisation is folded into `validate`. The implementation deliverable already in `.claude/skills/finetype-pipeline/SKILL.md` correctly omits `load` and explicitly documents that `finetype load …` errors out via clap with exit 2 — so the existing implementation contradicts the AC text it is supposed to satisfy.
**Evidence:**
- Bead AC-01 description: "guides it through all steps: profile, schema, validate, load — not just profile and stop"
- `.claude/skills/finetype-pipeline/SKILL.md` line 18: `profile → schema → validate (--db --table)` (3-step pipeline)
- `.claude/skills/finetype-pipeline/SKILL.md` line 159: "`finetype load …` was removed in v0.6.19 (MADR 0070)"
- `CLAUDE.md`: "Version: 0.6.19" — `load` removal is in scope of the current release line
**Recommendation:** Rewrite AC-01 to reflect the post-MADR-0070/0071 surface. Suggested wording: "the skill guides it through all steps: profile, schema, validate (with `--db --table` for typed materialisation) — not just profile and stop." If the spec author genuinely wants a separate `load` step restored, that is an architecture change requiring a new MADR, not a skill spec.

### [HIGH] AC-02 has no mechanism to detect skill-vs-binary drift
**Category:** test-gap
**Pass:** 2
**Description:** AC-02 requires "every command, flag, and output format" to be inline in the CLI skill, but provides no procedure for verifying completeness or for keeping the skill in sync with future CLI changes. The current `.claude/skills/finetype-cli/SKILL.md` already advertises "FineType v0.6.12" while the project is at v0.6.19 — three minor versions of drift have already accumulated. Without an explicit drift-detection rule, this AC will pass at implementation time and silently rot.
**Evidence:**
- `.claude/skills/finetype-cli/SKILL.md` line 11: "FineType v0.6.12 — Precision format detection for text data."
- `CLAUDE.md`: "Version: 0.6.19"
- AC-02: no test, no comparison procedure, no diff harness mentioned
**Recommendation:** Either (a) add a verification AC that diffs `finetype --help` (and per-subcommand `--help`) against the skill content, failing on missing flags, or (b) reframe AC-02 as a snapshot taken at a named version (e.g. "complete for v0.6.19 surface as of release X") and add a follow-up bead for drift checking. Option (a) is more durable.

### [MEDIUM] No verification method for AC-01's behavioural claim
**Category:** test-gap
**Pass:** 2
**Description:** AC-01 is a behavioural assertion ("agents complete the full pipeline, not just profile") but the spec gives no way to test this. Reading the markdown is not the same as observing an agent change behaviour. Two failure modes are possible: (1) the skill is well-written but never invoked by an agent because trigger language is wrong; (2) the skill is invoked but the agent still stops after profile because the "do not stop here" prompt is too weak.
**Evidence:**
- AC-01: verification is implicit — there is no `then: agent observed to run all four commands` clause
- The current `finetype-pipeline/SKILL.md` says "Do not stop here" once at line 48 and again as principle 1 at line 196 — no evidence this is sufficient prompting
**Recommendation:** Add a verification AC that runs a scripted agent scenario (e.g. via a transcript fixture or a manual test plan) and asserts the agent invoked `profile`, `profile -o json-schema`, and `validate --db --table` in sequence on a representative CSV. If a behavioural test is too heavy, at minimum document a manual checklist in the spec.

### [MEDIUM] Scope vs. goal — "ship two skills" without delta from current state
**Category:** missing-requirement
**Pass:** 1
**Description:** The goal says "Two skills shipped" and both skills already exist on disk in `.claude/skills/finetype-pipeline/SKILL.md` and `.claude/skills/finetype-cli/SKILL.md`. The bead is `in_progress` but the spec does not describe what work remains: are the skills being created from scratch (no — they exist), edited to fix gaps (which gaps?), promoted from project-local to user-global (where?), or distributed via Homebrew/release artefact (no mention)? An implementer reading this spec cold cannot tell what to do.
**Evidence:**
- Bead status: `in_progress`
- `.claude/skills/finetype-pipeline/SKILL.md` and `.claude/skills/finetype-cli/SKILL.md` both exist (8.4K and 7.5K respectively)
- Card 0011 references `web: docs/projects/finetype/agent-ready.mdx` — likely the marketing page that prompted the card — but the spec doesn't say whether that page is the source of truth for the deliverable
**Recommendation:** Add a "current state" stanza to the bead description explaining what exists, what is missing, and what "shipped" means in this context (committed to repo? included in release artefact? published in the FineType plugin?). Either tighten the goal to a verifiable delta or split into sub-beads for distribution.

### [MEDIUM] AC-03 misses the cross-runtime portability question
**Category:** missing-requirement
**Pass:** 2
**Description:** AC-03 asserts "no MCP server, no API keys, no configuration — just markdown that teaches the agent what to do." This is true for Claude Code (which reads `.claude/skills/`), but other agent runtimes (Cursor, Codex, Aider, Cline) do not auto-discover these files. The spec implicitly equates "agent" with "Claude Code agent" — but the goal language is generic. If portability is in scope, the AC fails for non-Claude runtimes; if it is out of scope, the spec should say so.
**Evidence:**
- AC-03: "no MCP server… just markdown"
- `.claude/skills/` is a Claude Code convention; other agent runtimes have their own conventions (`.cursorrules`, etc.)
- Card `i_want`: "without configuring an MCP server" — the framing is anti-MCP, not pro-Claude-Code-specifically
**Recommendation:** Add an explicit constraint: "Target runtime: Claude Code skill loader (`.claude/skills/<name>/SKILL.md`)." If multi-runtime portability is wanted, file a separate bead — it is a meaningfully larger scope than two SKILL.md files.

### [MEDIUM] CLI skill is stale — references v0.6.12 in v0.6.19 codebase
**Category:** assumption
**Pass:** 1
**Description:** The existing `finetype-cli` skill banner and several command examples are pinned to v0.6.12. The codebase is at v0.6.19 and has shipped surface changes (MADR 0070/0071 retire `schema` verb and `load` verb; flag inventories may differ). A spec to "ship" this skill must specify the version pin and bring the content current — otherwise AC-02 ("complete CLI reference") is false on day one.
**Evidence:**
- `.claude/skills/finetype-cli/SKILL.md` line 11: "FineType v0.6.12"
- CLAUDE.md: "Version: 0.6.19"
- Same skill correctly mentions v0.6.19 changes later (line 41: "v0.6.19 surface change") — the file has been partially updated, suggesting incomplete maintenance
**Recommendation:** Add an AC that names the version target ("CLI reference accurate against v0.6.19 binary") and add a verification step that diffs against `finetype --help` output. Update the banner.

### [LOW] No rollback / regression-protection plan
**Category:** missing-requirement
**Pass:** 3
**Description:** The spec has no statement about what happens if a future CLI change breaks the skill, or how to roll back if the skill causes agents to fail loudly. Skills are markdown so technical rollback is trivial (revert the file), but a regression-protection plan would prevent the drift problem identified in [HIGH] AC-02 from re-occurring.
**Evidence:**
- No `regression`, `rollback`, `drift`, or `version` keyword in the spec
- The drift between v0.6.12 banner and v0.6.19 codebase is direct evidence the gap is real
**Recommendation:** If an AC for drift detection is added (per HIGH AC-02), this finding is absorbed. Otherwise add a single sentence committing to a release-time check.

### [LOW] AC-03 verification method is implicit
**Category:** test-gap
**Pass:** 3
**Description:** AC-03 is structurally testable ("does the skill require any setup?") but has no explicit verification — `grep -r MCP\|api.key\|config .claude/skills/finetype-{pipeline,cli}/` would suffice. Naming the test makes the AC unambiguous.
**Evidence:** AC-03 prose only.
**Recommendation:** Add a one-line verification: "Verified by inspection — neither SKILL.md mentions MCP, API keys, or external configuration."

---

## Honest Assessment

This spec is not ready to implement. Two HIGH issues block: AC-01 names a `load` step that the codebase removed in v0.6.19 (so the spec literally contradicts the architecture), and AC-02 has no drift-detection mechanism while the existing CLI skill is already three minor versions stale. The biggest structural risk is that both deliverables already exist on disk in a partially-stale state, and the spec does not describe the delta between current and "shipped" — an implementer cannot tell whether the work is "create", "update", "verify", or "distribute". Tighten the goal to a verifiable delta, reconcile AC-01 with the post-0070/0071 pipeline surface, and add a drift-detection AC for the CLI reference. After those changes the spec becomes implementable in a small, focused PR.
