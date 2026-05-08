# Spec Review

**Date:** 2026-05-01
**Reviewer:** Context-separated agent (fresh session)
**Bead:** finetype-u6a
**Cycle:** 3 (drive_review_spec_cycle=2 → this review fires the third pass)
**Verdict:** APPROVE | REQUEST_CHANGES | BLOCK

**Verdict:** APPROVE

---

## Review Depth

| Pass | Triggered by | Findings |
|------|-------------|----------|
| 1 — Structural scan | always | 1 (LOW) |
| 2 — Assumption & failure | Pass 1 surfaced one minor under-enumeration in AC-02's divergence list | 1 (LOW) |
| 3 — Adversarial | not triggered — Pass 2 found only completeness nits, no unsatisfiable ACs or cross-system breakage | 0 |

Pass 3 was not triggered: the cycle-2 review's HIGH (AC-03 grep false positive) and three MEDIUMs (AC-02 bundling, AC-02 hidden-subcommand ambiguity, AC-01 verification artefact) are all cleanly resolved in cycle-3, and the residual concerns are non-blocking enumeration polish rather than design defects.

---

## Cycle-2 → Cycle-3 progress

The cycle-2 review (`review-spec-2026-05-01-v2.md`) raised 6 findings (1 HIGH, 4 MEDIUM, 1 LOW). The revised bead description resolves all six:

- **HIGH AC-03 grep false-positive trap** → Resolved. The grep is gone; AC-04 (renumbered from AC-03) is now inspection-only and explicitly carves out the `mcp` *subcommand* mention as non-violating: "Documentation references to FineType's own `mcp` subcommand are not setup requirements and do not violate this AC." The verification targets *requirement* rather than *mention*, exactly as recommended.
- **MEDIUM AC-02 hidden-subcommand ambiguity** → Resolved. AC-02 explicitly names `generate` and `check`. AC-03 says the drift script enumerates them "via a curated allowlist baked into the script, not from `--help` parsing alone" — matches the cycle-2 recommendation's option (2). The bead also adds a "Hidden-subcommand handling" stanza explaining the rationale ("Treating them as obsolete because they're not in `--help` would silently delete real documentation").
- **MEDIUM AC-02 bundling** → Resolved. AC-02 (content refresh) and AC-03 (drift harness) are now separate ACs. CI wiring is explicitly deferred — "Wiring into release CI is out of scope for this bead and tracked as a follow-up; ship the invokable script" — and the script path is named: `scripts/verify-cli-skill-coverage.sh`.
- **MEDIUM AC-01 verification artefact under-specified** → Resolved. The "transcript fixture or" branch is dropped; the artefact is unambiguously a manual checklist at a named path (`.claude/skills/finetype-pipeline/CHECKLIST.md`).
- **MEDIUM hidden-subcommand cascade** → Absorbed by the AC-02/AC-03 split with curated-allowlist enumeration.
- **LOW release-workflow scope** → Resolved. "Scope constraints" now states "CI integration: out of scope for this bead… tracked as a follow-up bead." No release-workflow modification is in scope; the cycle-2 LOW about an undeclared cross-system boundary is moot.

The cycle-3 spec also adds two structural improvements not requested in cycle-2:
- A "Current state (cycle 3)" stanza that states what each skill looks like *today* and what the iteration's delta is (4 enumerated items).
- A "No new MADR" line clarifying the work is purely conformance to 0070/0071 — no architecture change.

---

## Findings

### [LOW] AC-02's divergence list applies "wrong `--model-type` default" and "stale `--sharp-only` flag" to `profile` but the same divergences exist for `infer`
**Category:** missing-requirement (completeness)
**Pass:** 2
**Description:** AC-02 lists "wrong `--model-type` default (binary=multi-branch)" and "stale `--sharp-only` flag" as divergences to close. Both items also apply to `finetype infer` — its `--model-type` default in the binary is `multi-branch` (the skill currently says `char-cnn`), and `--sharp-only` is documented in the skill's `infer` flag table but is not present in the v0.6.19 binary's `infer --help`. The phrase "at least these divergences" provides a soft escape hatch, but the implementer who scans the AC's enumerated list and stops there will fix `profile` only and leave `infer` stale, partially defeating AC-02.

**Evidence:**
- `finetype infer --help` (v0.6.19): `--model-type` default is `multi-branch`; no `--sharp-only` flag.
- `.claude/skills/finetype-cli/SKILL.md` line 134: `--model-type <TYPE>` default `char-cnn` listed under `infer` flag table.
- `.claude/skills/finetype-cli/SKILL.md` line 135: `--sharp-only` listed under `infer` flag table.
- AC-02 enumerated list does not name `infer` as a target.

**Recommendation:** Either (a) name both subcommands in the enumeration ("wrong `--model-type` default on `profile` and `infer`; stale `--sharp-only` flag on `profile` and `infer`"), or (b) lean harder on the existing "at least these" hedge by adding a sentence: "The enumerated list is illustrative; AC-02 passes only when the entire flag inventory is current — implementer must walk the full subcommand set, not just the named items." Either tightens the AC's behavioural guarantee. Option (a) is cheaper and more deterministic.

This is non-blocking because AC-03's drift script — once written — will catch any residual divergences on its first run, including these two on `infer`. So the safety net exists; the AC text just doesn't lead the implementer there.

### [LOW] AC-01's "manual checklist" verification — does the AC pass on checklist *existence and content* or on a *recorded run-through*?
**Category:** test-gap (mild)
**Pass:** 1
**Description:** AC-01 says verification is "by `.claude/skills/finetype-pipeline/CHECKLIST.md` — a manual checklist that enumerates the three commands the agent must invoke in sequence on a representative CSV." The intent reads clearly as inspection-of-checklist-content (does the checklist exist, and does it list the three commands in the correct order?), but the AC text leaves a small ambiguity: a strict reader could interpret "Verified by … a manual checklist" as "verified by *running* the checklist against a CSV and capturing the result." The cycle-2 review explicitly flagged this kind of ambiguity in its AC-01 finding.

**Evidence:**
- AC-01 verifier clause: "Verified by `.claude/skills/finetype-pipeline/CHECKLIST.md` — a manual checklist that enumerates the three commands the agent must invoke in sequence on a representative CSV."
- No "the checklist must be checked off and committed" or "the checklist's existence with the three commands listed in order is sufficient" disambiguator.

**Recommendation:** Add four words. Either:
- "Verified by **inspection of** `.claude/skills/finetype-pipeline/CHECKLIST.md` — a manual checklist that enumerates the three commands…" (passes on existence + content)
- "Verified by **a run-through recorded in** `.claude/skills/finetype-pipeline/CHECKLIST.md`…" (requires actual execution + committed checked boxes)

Pick whichever matches Hugh's intent; the spec is implementable either way but the implementer would benefit from knowing which artefact shape to commit.

This is non-blocking because the inspection interpretation is the natural reading and the existing pipeline SKILL.md already documents the three commands in the correct sequence — so even the strictest interpretation of AC-01 is satisfiable with ≤30 minutes of additional work.

---

## Honest Assessment

This is a clean cycle-3 spec. The cycle-2 HIGH (AC-03 grep) is gone, AC-02 has been split correctly with named artefact paths, hidden-subcommand handling is now explicit and well-justified, and the scope envelope is honest about what ships now (skill content + drift script) vs. what ships later (CI wiring as a follow-up bead). The "Current state" and "Hidden-subcommand handling" stanzas are the kind of context that makes a spec implementable cold.

The two residual LOWs are completeness polish, not design defects:
- The `--model-type` default and `--sharp-only` divergences also exist on `infer`, but the "at least these" hedge plus AC-03's drift script will catch them on first run.
- AC-01's "manual checklist" verifier could be 4 words clearer about whether existence-with-correct-content is enough or a run-through is required, but both interpretations are cheaply satisfiable.

The biggest residual risk has shifted from "spec is wrong" to "the enumerated divergence list is illustrative rather than exhaustive." That is the correct kind of risk to carry into implementation, because AC-03's drift script is the canonical safety net — by design, it catches anything the implementer's manual sweep misses.

Recommendation to the implementer: when you do the AC-02 content refresh, walk *every* row of *every* flag table against `finetype <subcommand> --help`, not just the items named in the divergence list. Then run the AC-03 drift script as the final cross-check before you call AC-02 done.

**Verdict:** APPROVE — the spec is implementable in a focused PR (CLI skill content refresh + new drift script + new pipeline checklist + zero-setup inspection note). The two LOWs are advisory and can be addressed during implementation without re-running review-spec.
