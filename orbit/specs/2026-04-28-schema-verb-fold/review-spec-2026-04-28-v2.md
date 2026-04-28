# Spec Review

**Date:** 2026-04-28
**Reviewer:** Context-separated agent (fresh session)
**Spec:** `/Users/hugh/github/meridian-online/finetype/orbit/specs/2026-04-28-schema-verb-fold/spec.yaml` (v1.1, cycle-2)
**Verdict:** APPROVE

---

## Review Depth

```
| Pass                       | Triggered by                                                                                  | Findings |
|----------------------------|-----------------------------------------------------------------------------------------------|----------|
| 1 — Structural scan        | always                                                                                        | 2 (LOW)  |
| 2 — Assumption & failure   | content signals (cross-system CLI/MCP carve-out, doc migration, decision register flip)       | 0        |
| 3 — Adversarial            | not triggered (Pass 2 surfaced no structural concerns; cycle-1 HIGH/MEDIUMs all addressed)    | —        |
```

## Cycle-1 Finding Disposition

This is a cycle-2 review of a v1.1 revision. Cycle-1 raised 10 findings
(2 HIGH, 6 MEDIUM, 2 LOW). Verifying each against the v1.1 text:

```
| Cycle-1 finding                                                            | Severity | Status   | Where addressed                                                                                       |
|----------------------------------------------------------------------------|----------|----------|-------------------------------------------------------------------------------------------------------|
| #1 — Taxonomy clap variant has no positional KEY                           | HIGH     | RESOLVED | ac-02 explicitly adds `type_key: Option<String>`, glob predicate, edit-distance suggestions          |
| #2 — `cmd_taxonomy` has no `--pretty` flag, README target broken           | HIGH     | RESOLVED | ac-06 drops `--pretty` from README target; impl notes line 287-292 commits to unconditional pretty   |
| #3 — Helper module location stated three different ways                    | MEDIUM   | RESOLVED | Implementation_notes lines 254-271 explicitly name the drift and pin finetype-mcp as authoritative   |
| #4 — `cmd_schema_table` is ~200 LOC, not a thin caller                     | MEDIUM   | RESOLVED | ac-04 describes ~200 LOC of model load + sibling-context + classification; clippy `-D dead_code` gate |
| #5 — Unknown-key contract under JsonSchema mode unspecified                | MEDIUM   | RESOLVED | ac-02 + ac-03 pin: exit 1, suggestions on stderr, empty stdout (NOT `[]`); verification covers it    |
| #6 — Migration grep verification has false-positive/negative risk          | MEDIUM   | RESOLVED | ac-06 verification has tightened greps (zero hits, MCP row exactly one hit, ≥2 taxonomy/profile)     |
| #7 — Silent `x-finetype-label` addition framed as "fix-in-passing"         | MEDIUM   | RESOLVED | Impl notes 272-283 reframe as "behaviour change"; ac-07 adds Changed sub-bullet; ac-05 asserts label |
| #8 — Public-surface count off-by-one (7 → 6 public commands)               | MEDIUM   | RESOLVED | ac-06 explicitly updates CLAUDE.md:177 ("7 public commands" → "6") + line 186 table row              |
| #9 — CLAUDE.md historical entries — confirm none are broken invocations    | LOW      | RESOLVED | ac-06 grep gates this; only hit was line 208 (CLI command-table row), which ac-06 deletes            |
| #10 — Clippy baseline unpinned                                             | LOW      | RESOLVED | ac-11 pins "PR #53 baseline" + "touched-crates-only baseline (-p finetype-cli -p finetype-mcp)"      |
```

All 10 cycle-1 findings substantively addressed. The revision history
block (`metadata.revision_history`) records the cycle-2 changes
truthfully and matches what's in the body.

I verified the central claims of v1.1 against the actual code state on
`main`:

- `Commands::Taxonomy` at `crates/finetype-cli/src/main.rs:172-197` has
  no positional argument and no `--pretty` flag — confirms ac-02 / ac-06
  are necessary additions, not duplicates of existing behaviour.
- `OutputFormat::JsonSchema` already exists at `main.rs:495`; `cmd_taxonomy`
  has a catch-all arm at line 2466 that degrades JsonSchema to Plain.
  ac-03's "replace catch-all with dedicated arm" is correctly described.
- `crates/finetype-mcp/src/json_schema.rs` exists (per `ls`); the
  helper-module-home drift narrative in implementation_notes is true to
  the as-shipped state.
- `cmd_schema` at line 2601 contains the exact-match-or-glob predicate
  + edit-distance fallback that ac-02 says will be ported into
  `cmd_taxonomy`. Source code reads as described.
- `cmd_schema_table` at line 2735 spans ~200 LOC of model loading,
  taxonomy compilation, CSV reading, and per-column classification
  before calling `emit_table_schema`. ac-04's size claim is accurate.

## Findings

### [LOW] ac-04 line-number references drift from actual file state
**Category:** test-gap
**Pass:** 2
**Description:** ac-04 description cites `cmd_schema` at `main.rs:2565+`,
`cmd_schema_table` at `main.rs:2699+`, `build_json_schema` at
`main.rs:2642`, and the path-sniffer dispatch arm at `main.rs:595+`.
Actual line numbers on `main` at HEAD are 2601, 2735, 2678, and 599
respectively. The deltas are small (~40-60 lines) and the function
names + grep verification both pin the targets unambiguously, so this
is not load-bearing. But the cycle-1 review flagged the same kind of
line-number drift for the helper-module location (finding #3) and the
v1.1 spec fixed it by adding an explicit drift-acknowledgement
paragraph; it's slightly inconsistent that ac-04's line refs weren't
re-grounded against current main during the same revision pass.
**Evidence:** `grep -n "fn cmd_taxonomy\|fn cmd_schema\b\|fn cmd_schema_table\|fn build_json_schema" crates/finetype-cli/src/main.rs` returns 2433 / 2601 / 2735 / 2678. Spec ac-04 cites 2565 / 2699 / 2642. Dispatch arm at line 599 (spec says 595+).
**Recommendation:** Either (a) update the line refs to current HEAD —
quick mechanical fix; or (b) drop the line numbers and rely on the
function names + the verification grep, which is what already happens
in ac-04's verification command. Implementation will not be misled
either way; this is a quality-of-evidence nit, not a blocker.

### [LOW] ac-06 doesn't grep for the new "MCP audit follow-up" CLAUDE.md note
**Category:** test-gap
**Pass:** 2
**Description:** ac-06 description (line 153-155) asks the implementer
to add an inline note to the retained CLAUDE.md MCP tools row reading
"MCP audit follow-up in v0.6.20 will mirror the CLI fold." The
verification block (line 170-176) has greps for "finetype schema"
zero-hits, MCP row exactly-one-hit, and "6 public commands" one-hit,
but does not grep for the new audit-follow-up note in CLAUDE.md.
ac-10 has a similar grep against `crates/finetype-mcp/src/` for the
same string, so the source-side comment is gated; the CLAUDE.md
addition is not. Minor — a missing note is a small docs miss, not a
behaviour break — but a one-line `grep -n "MCP audit follow-up" CLAUDE.md`
returns one hit verification would close the loop cheaply.
**Evidence:** Spec ac-06 description line 153-155 (note added to CLAUDE.md MCP tools row); ac-06 verification line 170-176 (no grep for the new note); ac-10 verification line 232-236 (grep for same string but only in `crates/finetype-mcp/src/`).
**Recommendation:** Add one line to ac-06 verification:
`grep -n "MCP audit follow-up" CLAUDE.md` returns at least one hit.
Optional, but symmetric with ac-10 and pre-empts a review-pr nit.

---

## Honest Assessment

The v1.1 revision is thorough. All 10 cycle-1 findings are
substantively addressed — not just acknowledged with a line of prose
but reflected in actual AC text changes (positional KEY arg in ac-02,
dropped --pretty in ac-06, dead-code clippy gate in ac-04, label-
presence assertion in ac-05, explicit Changed CHANGELOG bullet in
ac-07, public-surface count update in ac-06, baseline pin in ac-11).
The implementation_notes block grew the right paragraphs: helper-
module-home drift, in-passing label addition reframed as a behaviour
change, cmd_schema_table size correction.

The two LOW findings I surface are quality-of-evidence nits, not
content gaps. ac-04's line-number drift is consistent with the
pre-existing-line-ref pattern this spec is otherwise fastidious about
fixing; ac-06 missing a grep for its own new CLAUDE.md note is a
mirror-symmetry omission against ac-10. Both are one-line fixes that
could fold into the implementation PR without reopening design.

The biggest remaining risk for this card is execution: deleting
`cmd_schema_table` cleanly while ensuring the model-load /
sibling-context wiring stays reachable from `cmd_profile`. The spec
correctly gates this with `cargo clippy -D dead_code` (ac-04) — that
check, plus the existing golden tests for `profile -o json-schema`
from card 0003's PR #53, should catch any orphaned helpers.

I'd ship this. The two LOW findings can be folded in alongside the
implementation; they don't justify another review cycle.
