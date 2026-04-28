# Review (cycle 2): spec for `validate` absorbs `load` (card 0005)

**Spec:** `orbit/specs/2026-04-28-validate-absorbs-load/spec.yaml` (v1.1, 2026-04-28T16:30:00Z)
**Card:** `orbit/cards/0005-schema-driven-data-validation.yaml`
**Rally:** `orbit/specs/2026-04-28-v0619-cli-consolidation-rally/` (autonomy: auto, phase: implementing)
**Sibling artefacts read:** `interview.md`, `decisions.md`, `orbit/memos/2026-04-27-load-folds-into-validate.md`
**Implementation parent:** PR #54 (rally/schema-verb-fold)
**Cycle 1 review:** `review-spec-2026-04-28.md` (REQUEST_CHANGES, 10 findings F1-F10)

---

**Verdict:** APPROVE

The 10 cycle-1 findings have been substantively addressed in v1.1. The
spec now (1) splits the 15 `vrp_*` test population correctly across
crates, (2) drops the fictional `vrp_ac0X_*` names, (3) routes the MCP
description-string edit to its actual home at
`crates/finetype-mcp/src/lib.rs:113`, (4) fixes the ac-11 grep target,
(5) adds the labelled-but-unknown-label projection branch to ac-01,
(6) surfaces ENUM-emission drop as both a binding constraint and an
ac-12 migration entry, (7) re-bases the ac-05 LOC delta against PR #54
head, (8) pins the pre-CTAS sweep as a binding constraint, (9) tightens
the ac-12 CHANGELOG verification to extract the relevant section first,
and (10) names the `error_message` semantic split in MADR 0071's
required content. The architectural shape was sound in v1.0 and remains
sound; v1.1 closes the precision gaps.

The spec is implementer-ready. Three minor polish items remain (Section
"Polish — non-blocking" below) — none rise to the bar for another revision
cycle. The implementer can absorb them or leave them.

---

## How v1.1 addressed the cycle-1 findings

```
| #   | Finding                                               | v1.1 resolution                                        | Status |
|-----|-------------------------------------------------------|--------------------------------------------------------|--------|
| F1  | ac-08 mislocates 15 vrp_*                             | ac-08 + constraint line 16: 8 CLI + 7 engine; verification splits the cargo test runs across crates | Fixed* |
| F2  | Fictional vrp_ac0X_* names                            | ac-08 enumerates the real 8 (3 named + 5 ac13_cli_*); fixture caveat (identity.code.id not in taxonomy → bare-passthrough) explicit | Fixed |
| F3  | MCP description edit at wrong path                    | ac-07 + constraint line 15 + impl note line 314: edit at `lib.rs:113`; tools/validate.rs limited to module doc comment | Fixed |
| F4  | ac-11 grep target wrong                               | Verification now `grep -c "only the 5 public commands" CLAUDE.md` returns ≥1 | Fixed |
| F5  | ac-01 silent on labelled-but-unknown-label            | ac-01 now enumerates branch (b) explicitly: label present, taxonomy.get() returns None → bare quoted identifier | Fixed |
| F6  | ENUM drop buried in impl notes                        | New constraint line 21 + ac-12 `### Changed` (b) call out ENUM-emission drop verbatim | Fixed |
| F7  | ac-05 LOC delta vs wrong baseline                     | ac-05 now: "Net delta ≤ -250 measured against PR #54's post-merge HEAD" — relative, sturdy | Fixed |
| F8  | Pre-CTAS vs post-CTAS sweep choice deferred           | New constraint line 22: pre-CTAS sweep is binding; impl note 308 reinforces; "post-CTAS DELETE alternative is forbidden" | Fixed |
| F9  | ac-12 CHANGELOG grep too loose                        | Verification now extracts `[Unreleased]/[0.6.19]` section first via awk before searching for tokens | Fixed |
| F10 | error_message semantic split undocumented in MADR     | ac-10 now requires MADR 0071 to carry "error_message semantic split" labelled section | Fixed |
```

\* F1 has a residual factual nit — see Polish item P1 below. Resolution
is sufficient for APPROVE; the nit doesn't block implementation.

---

## Stress-test on the focus areas requested

### Reject ontology extension (ac-03, ac-04)

**ac-03 — transform-failure emits a reject row.**

Verified the AC fully covers the case: predicate `staging IS NOT NULL
AND TRY(transform) IS NULL`, `error_type='TRANSFORM_FAILED'`,
`constraint_failed='transform'`, `error_message=<staging cell value>`,
`expected_type=<x-finetype-label>`, `constraint_value=<transform
expression>`, row excluded from user table. Test fixture is precise:
`2024-02-30` matches `^\d{4}-\d{2}-\d{2}$` but `strptime` returns NULL.
Three concrete assertions on the test side: user-table row count = 2,
reject sidecar row count = 1 with the named tokens, exit code 1.

The complementary constraint at line 20 ("NULL-in-NULL-out is NOT a
transform failure") names the predicate explicitly so the implementer
cannot conflate the two cases. Constraint line 22 binds the
implementation shape (pre-CTAS sweep) so the "row excluded from user
table" semantics fall out structurally rather than depending on a
post-CTAS DELETE pattern.

**ac-04 — NULL-in-NULL-out is NOT a reject.**

Test fixture is the contrasting case: row 2 has empty CSV cell (`,,`),
which the CSV reader at `main.rs:3804-3812` normalises to None, then
`TRY(transform)` returns NULL. Predicate `staging IS NOT NULL` is false,
so no reject row. Three concrete assertions: user-table row count = 3
(NOT 2), reject sidecar TRANSFORM_FAILED count = 0, row 2's date column
reads NULL, exit code 0.

ac-03 and ac-04 are mutually exclusive predicates with distinct test
functions and distinct expected outputs. The pair correctly partitions
the failure space. **Coverage of the ontology extension is complete.**

### Hard-removal posture for `cmd_load` (ac-05, ac-06)

**ac-05 — clean deletion of the implementation.**

Names all three deletion points: (a) `Commands::Load` variant from the
clap enum, (b) dispatch arm `Commands::Load { ... } => cmd_load(...)`
in `main()`, (c) `cmd_load` function body. Conditional deletion of
`build_load_expr` and `build_load_expr_enum` ("deleted IF unused by
`build_transform_projection`; otherwise renamed/inlined") is correct
— the lifted helper subsumes their logic, so they're either folded or
deleted. The verification grep `fn cmd_load|Commands::Load|fn
build_load_expr` returning zero matches catches all three. The clippy
guard `-D dead_code -D warnings` catches a half-deletion (e.g.,
function body removed but `Commands::Load` variant left).

LOC bound (`≤ -250 measured against PR #54's post-merge HEAD`) is
sturdy after the F7 fix. Spot-check against the codebase: today's
`cmd_load` is at `main.rs:2627-3168` (~542 lines including helpers),
plus `Commands::Load` variant (~40 lines) plus dispatch arm
(~20 lines) — comfortably exceeds 250 even after partial helper
retention.

**ac-06 — user-facing error path via clap unknown-subcommand handler.**

Verified the AC specifies exactly what's needed: exit code 2 (clap's
standard), stderr contains "unrecognized subcommand 'load'" or
locale-stable equivalent. No shim, no deprecation warning, no carve-out.
The integration test `vrp_load_subcommand_unknown` is one cargo run
invocation with two assertions. Matches the v0.6.19 hard-removal posture
PR #51 ratified.

**Coverage of hard-removal is complete.**

### ac-08 — 15 existing vrp_* tests stay green

The migration guarantee is the load-bearing claim that says "this fold
doesn't break what's already tested." v1.1 splits this correctly:

- **8 CLI-side tests** in `crates/finetype-cli/tests/validate_cli.rs` —
  these ARE on the materialise code path, but their schema fixtures
  (`SCHEMA_WITH_EXT`, `SCHEMA_NO_EXT`) use `x-finetype-label:
  identity.code.id`, which is NOT in the taxonomy. Per ac-01 branch (b),
  unknown labels fall through to bare-passthrough VARCHAR. So the
  existing CLI tests pass byte-identically without any column-type
  assertions tightening.

- **7 engine-side tests** in `crates/finetype-core/src/table_validator.rs`
  — these are pure-function tests of `validate_table` and cannot
  regress under this fold (no CLI code paths affect them).

The split is verified-correct on the codebase except for the engine
count — see Polish P1 below (the actual count is 14, not 7). The claim
that they "cannot regress under this fold" is correct regardless of
count; the literal "7" is the only nit.

Constraint line 16 captures the typed-CTAS-coverage gap correctly:
the new ac-02/ac-03/ac-04 tests use real taxonomy labels
(`datetime.date.iso_8601`, `finance.currency.amount`) and carry the
typed-CTAS coverage that the existing CLI tests don't exercise. So
the migration is "stay green" + "new tests cover the new behaviour" —
which is the correct partitioning.

**The migration guarantee holds.** Implementer can run the verification
commands literally as written and they'll pass.

### ac-13 — `make ci` baseline is PR #54

The cycle-1 finding F7 was about ac-05's LOC count baseline; v1.1
correctly re-bases it. ac-13's clippy baseline ("Zero new clippy
warnings vs PR #54 baseline") is the parallel statement and is also
correctly stated in v1.1. Since this card is the second half of the
v0.6.19 CLI consolidation rally and PR #54 is the parent, this is the
right baseline — `main` is one rally back.

**Baseline is correct.**

---

## Polish — non-blocking

These are precision items the implementer can absorb or leave. None of
them block implementation; none affect AC verifiability.

### P1 — Engine-side `vrp_*` count is 14, not 7

**Severity:** trivial (claim is robust to the count; only the literal
number is wrong).

`grep -c 'fn test_vrp_' crates/finetype-core/src/table_validator.rs`
returns 14 — comprised of `ac01_result_shape` (1) + `ac02_*` (6:
pattern/min_length/max_length/enum/type/required/+) +
`ac03_determinism` (1) + `ac13_*` (6: happy_all_valid/all_reject/
partial_reject_mixed/multi_reject_per_row/empty_input/single_row
_single_column).

The cycle-1 review (`review-spec-2026-04-28.md:42`) and the PR #46
review-pr (`2026-04-22-duckdb-extension-ergonomics/review-pr-2026-04-24
.md:41`) both recorded "core 7" — the original count, before subsequent
test additions. v1.1 carries "7 engine-side" forward from the cycle-1
review.

The substantive claim — "engine-side tests cannot regress under this
fold (no CLI code paths affect them)" — is correct regardless of count.
And the verification command (`cargo test -p finetype-core
table_validator`) runs whatever exists. So the wrong literal doesn't
break anything.

**Suggested:** in ac-08 and constraint line 16, replace "7 engine-side"
with "all engine-side `vrp_*` tests" (count-free) or update to "14
engine-side." Either works.

### P2 — `Commands::Load` LOC count in decisions.md disagrees with ac-05

**Severity:** trivial (decisions.md is reference; ac-05 is binding).

decisions.md:106 says "~270 LOC deleted from `main.rs:3061-3328`" but
the actual `cmd_load` function (verified) lives at `main.rs:2628-3168`
(~542 lines). decisions.md's line range is stale by one rally
(pre-PR-#54). ac-05's bound (`≤ -250`) is the binding statement and
remains correct.

**Suggested:** if a follow-up touches decisions.md, refresh the line
range. Not blocking.

### P3 — ac-09 conditional ("delete OR replace") leaves implementer
discretion

**Severity:** low-trivial (both choices are AC-compliant by design).

ac-09 allows the implementer to pick deletion OR replacement of
`golden_load_*` per the AC's own logic ("Choose deletion when
validate_cli.rs covers the same semantic; choose replacement only
when the golden test exercised a behaviour validate_cli.rs does
not"). This is honest implementer discretion at the right level — but
if Hugh wants spec-binding rather than implementer-judgement, the spec
could state the verdict directly (e.g., "delete both `golden_load_*` —
validate_cli.rs covers their surface").

The current language is fine. Just flagging it as a remaining
discretion point.

---

## What the spec gets right (recap from cycle 1, still true)

- **Both reject-ontology test cases covered** (ac-03 + ac-04). Predicates
  are mutually exclusive; test functions are distinct; assertions are
  concrete.
- **Hard-removal posture for `cmd_load` is clean** (ac-05 + ac-06). All
  three deletion points named; clippy guard catches half-deletions;
  user-facing failure routes through clap's standard handler.
- **Single source of validation truth preserved** (constraint 9 +
  constraint 17). MADR 0064 invariant respected.
- **MCP surface stable** (ac-07 + constraint 15). `ValidateRequest`
  shape unchanged; description-string edit is the only surface change,
  now correctly routed to `lib.rs:113`.
- **Decisions traceable** (ac-10). MADR 0071 brief includes D1, D2, D4,
  refines 0064, supersedes nothing — accurate. error_message semantic
  split now explicit.
- **Migration map verbatim in CHANGELOG** (ac-12). Both old → new
  invocations spelled out; ENUM-loss surfaced under `### Changed`.

---

## Suggested rev to land alongside implementation (optional)

```
| #  | What                                                | Where                  | Blocking? |
|----|-----------------------------------------------------|------------------------|-----------|
| P1 | Engine vrp_* count: 14 not 7 (or count-free phrasing)| ac-08 + constraint 16 | No        |
| P2 | Refresh decisions.md cmd_load line range            | decisions.md:106       | No        |
| P3 | Spec-bind ac-09 deletion vs replacement              | ac-09                  | No        |
```

None of these need a v1.2 spec revision. The implementer can absorb P1
in their PR (single-line edit), leave P2 as a doc cleanup follow-up,
and P3 is honestly discretionary.

---

**Verdict:** APPROVE
