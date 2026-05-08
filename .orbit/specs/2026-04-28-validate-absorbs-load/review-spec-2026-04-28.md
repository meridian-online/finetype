# Review: spec for `validate` absorbs `load` (card 0005)

**Spec:** `.orbit/specs/2026-04-28-validate-absorbs-load/spec.yaml` (v1.0, 2026-04-28T15:30:00Z)
**Card:** `.orbit/cards/0005-schema-driven-data-validation.yaml`
**Rally:** `.orbit/specs/2026-04-28-v0619-cli-consolidation-rally/` (autonomy: auto, phase: implementing)
**Sibling artefacts read:** `interview.md`, `decisions.md`, `orbit/memos/2026-04-27-load-folds-into-validate.md`
**Implementation parent:** PR #54 (rally/schema-verb-fold, OPEN, MERGEABLE)

---

**Verdict:** REQUEST_CHANGES

The architectural shape is sound — the design pack and memo have done the
hard work, and the spec inherits clean decisions. The 13 ACs cover the
right surface (projection builder, typed CTAS, both ontology test cases,
hard-removal, MCP stability, docs, CI gate). But several verification
clauses are factually wrong against the codebase, and one branch of the
projection logic is implicit rather than specified. These need to be
nailed down before implementation starts; otherwise the spec ships with
verifications that can't be run as written and an implementer-discretion
gap on a behaviourally important code path.

The findings below are concrete and small — none of them is an
architectural rethink. After rev1 lands the corrections, this spec is
APPROVE-ready.

---

## Findings

### F1 — ac-08 mislocates the existing 15 vrp_* tests (factually wrong)

**Severity:** medium (load-bearing migration claim hangs on this number).

ac-08 says "All 15 existing `vrp_*` tests in
`crates/finetype-cli/tests/validate_cli.rs` continue to pass after the
fold." This is wrong. The 15 vrp_* tests are split across two crates:

```
| Crate                                   | vrp_* tests | Source             |
|-----------------------------------------|------------:|--------------------|
| finetype-core/src/table_validator.rs    |           7 | engine-side tests  |
| finetype-cli/tests/validate_cli.rs      |           8 | CLI integration    |
| Total                                   |          15 |                    |
```

(Verified by `grep 'fn test_vrp_' crates/`.) The PR #46 review-pr at
`.orbit/specs/2026-04-22-duckdb-extension-ergonomics/review-pr-2026-04-24.md:41`
documents this split explicitly: "core 7 + CLI 8."

**Why this matters:**
- The CLI-side tests are the ones that call into the materialise path
  and would actually exercise the fold's behavioural change. Of the 8
  CLI vrp_* tests, exactly 5 (`test_vrp_ac13_cli_*`) plus 2 ac-11 tests
  plus 1 ac-12 test would be re-run.
- The 7 engine-side core tests don't touch `cmd_validate_table` at all
  — they're pure-function unit tests for `validate_table`. They cannot
  regress under this fold (no CLI code changes affect them).
- The spec's verification command `cargo test -p finetype-cli --test
  validate_cli` runs only the 8 CLI tests, not 19 ("original 15 + 4
  new"). Hugh's checklist will fail at the count.

**Required:** Rev ac-08 to correctly state 8 existing CLI vrp_* tests
+ 4 new = 12 total in `validate_cli.rs`; the 7 engine-side tests stay
green by construction. Update the verification command's expected
count.

---

### F2 — ac-08 names a "VARCHAR-asserting" tightening that doesn't exist (implicit assertion concern)

**Severity:** medium (this is the load-bearing concern Hugh flagged).

ac-08 says "Tests that previously asserted user-table columns are
VARCHAR (the materialise-path tests) are updated to assert the typed
column type the schema's `x-finetype-label` implies." Implementation
note line 273 names `vrp_ac01_*`, `vrp_ac02_constraint_grid`, the
`vrp_ac06_*` family.

**The problem:**
1. None of those test names exist in `validate_cli.rs`. The 8 CLI
   vrp_* tests are named `vrp_ac11_xft_extensions_surface`,
   `vrp_ac11_null_on_absence`, `vrp_ac12_ecommerce_end_to_end`, and
   five `vrp_ac13_cli_*` tests. The names in the implementation note
   reference the spec's *own AC numbering* from
   `2026-04-22-duckdb-extension-ergonomics/spec.yaml`, not actual
   function identifiers.
2. None of the 8 actual CLI tests asserts that user-table columns
   are VARCHAR. The closest is
   `test_vrp_ac13_cli_writes_db_with_sidecar` which asserts
   `SELECT COUNT(*) FROM orders == 2` — a row count, not a column
   type. So there's nothing to tighten.
3. The schema fixtures (`SCHEMA_WITH_EXT`, `SCHEMA_NO_EXT`) use
   `x-finetype-label: "identity.code.id"` — and `identity.code.id`
   is *not* in the taxonomy (verified: zero matches in
   `labels/definitions_*.yaml`). Per Decision 5 graceful-degradation,
   an unknown label resolves to bare-passthrough VARCHAR. So the
   typed CTAS for these tests is going to emit VARCHAR-only columns
   regardless — which makes the existing tests pass byte-identically,
   but also means the fold's typed-CTAS behaviour is not actually
   exercised by any of the 8 retained tests.

**Implications for the implicit-assertions concern Hugh raised:**
The "tests stay green" guarantee is *trivially* true under the
current schema fixtures because they use a label the taxonomy doesn't
know about. The fold could silently regress (e.g., a bug that emits
the wrong projection) and these tests wouldn't catch it. The new
ac-02 / ac-03 / ac-04 tests are doing all the real work.

**Required:**
1. Rewrite ac-08 to drop the names that don't exist and explicitly
   acknowledge the schema-fixture corner: `identity.code.id` is not
   in the taxonomy, so the existing CLI tests fall through to
   VARCHAR pass-through. The migration is "verify they stay green
   against the existing fixtures," not "tighten VARCHAR
   assertions."
2. Either (a) accept that ac-08 is a no-op test-migration AC and
   the four new tests in ac-02..ac-04 + ac-06 carry the real
   coverage, OR (b) update one of the existing fixtures to use a
   real taxonomy label (e.g., `identity.code.uuid` or
   `representation.discrete.categorical`) so at least one existing
   test exercises the typed projection.
3. Drop the "vrp_ac01_*, vrp_ac02_constraint_grid, vrp_ac06_*"
   string from implementation_notes line 273.

---

### F3 — ac-07 wrong file path for the MCP description string (factually wrong)

**Severity:** medium (verification cannot run as written).

ac-07 description says the MCP `validate` tool "description gains
one sentence" and says "the MCP `validate` tool description string
lives at `crates/finetype-mcp/src/tools/validate.rs` near the
`#[tool(...)]` macro / `description = \"...\"` literal."

This is not where it lives. The `#[tool(description = "...")]` literal
for `validate` lives at `crates/finetype-mcp/src/lib.rs:113`:

```rust
    #[tool(
        description = "Validate CSV data against a JSON Schema. Returns per-row and per-column validation results with a quality grade."
    )]
```

`crates/finetype-mcp/src/tools/validate.rs:9-22` contains only the
`ValidateRequest` struct and `#[schemars(description = ...)]` attrs
on its fields. The constraint at spec line 15 ("MCP `validate`
tool's `ValidateRequest` shape unchanged...
`crates/finetype-mcp/src/tools/validate.rs:9-22` not edited") is
correct as written. But the ac-07 description ALSO requires editing
that same file's "tool-description string" — which doesn't live
there. Verification fails on contradiction:

> `git diff main -- crates/finetype-mcp/src/tools/validate.rs`
> shows changes ONLY in (a) tool-description string and (b) module
> doc comment

If the implementer correctly edits `lib.rs:113` for the description
addendum, this diff will be empty (only module doc comment edits
remain in `tools/validate.rs`), and the verification fails. If the
implementer takes the spec literally and tries to edit
`tools/validate.rs`, there is no description string there to edit.

**Required:** Rev ac-07 to (a) put the description-string edit in
`crates/finetype-mcp/src/lib.rs:113` (the actual `#[tool(description
= "...")]` literal), (b) keep the module doc comment edit in
`tools/validate.rs`, (c) update the `git diff` verification to
inspect both files. Implementation note line 276 needs the same
correction.

---

### F4 — ac-11 verification grep is wrong (factually wrong)

**Severity:** low (cosmetic but it can't run as written).

ac-11 verification says:
> `head -3 CLAUDE.md | grep -c "5 public commands"` returns 1

The string "X public commands" lives at line 177 of CLAUDE.md
(currently "**only the 7 public commands**"; PR #54 turns this into
"**only the 6 public commands**"; this card needs to turn it into
"**only the 5 public commands**"). It is nowhere near `head -3`.

**Required:** Replace with `grep -c "only the 5 public commands"
CLAUDE.md` returns ≥ 1.

---

### F5 — ac-01 silent on the labelled-but-unknown-label branch (specification gap)

**Severity:** medium (concrete behavioural gap, not just wording).

ac-01 specifies four projection branches:
1. unlabelled OR `duckdb_type == VARCHAR` → bare quoted identifier
2. transform present, `try_wrap=true` → `TRY(transform) AS "col"`
3. transform present, `try_wrap=false` → `transform AS "col"`
4. no transform, non-VARCHAR `duckdb_type` → `TRY_CAST(...)` /
   `CAST(...)`

None of these covers the case where `x-finetype-label` IS present
but the taxonomy doesn't have an entry for that label
(`taxonomy.ddl_info(&label)` returns `None`). The existing
`cmd_load` handles this at `main.rs:2758-2760` by falling through
to `("VARCHAR", None, "")` → the bare-passthrough branch.

The test fixture `SCHEMA_WITH_EXT` exercises exactly this branch
(label `identity.code.id` is not in the taxonomy). Without
specifying it, the implementer might:
- crash on `ddl_info(...).unwrap()` (silent panic),
- treat it as `duckdb_type=VARCHAR` (matches today's behaviour),
- or refuse the validate run with exit 2 (over-strict).

**Required:** Add a fifth branch (or fold into branch 1) explicitly:
"label present but `taxonomy.get(label)` is `None` → fall through to
bare-passthrough VARCHAR (preserves Decision 5 graceful
degradation)."

This is the same posture as today's `cmd_load:2758-2760`, but it
needs to be stated so it doesn't depend on implementer reading
between lines.

---

### F6 — ENUM behaviour change is buried in implementation_notes, not surfaced to ACs (silent regression risk)

**Severity:** medium (user-visible behavioural delta).

Implementation note line 271:
> ENUM emission is dropped from the validate fold; the
> `build_load_expr_enum` function is deleted alongside `cmd_load`.

This is a quiet behavioural regression: today, `finetype load -f
file.csv` emits `CREATE TYPE x_t AS ENUM (...)` for low-cardinality
columns when `--enum-threshold > 0`. After the fold, `validate --db
--table` produces no ENUM types — only VARCHAR or the column's
`duckdb_type`. The migration map in ac-12 (`finetype load → finetype
validate ...`) does NOT mention ENUM loss.

**Why this matters:**
- A user migrating from `finetype load -f orders.csv -t orders` to
  `finetype validate orders.csv schema.json --db out.db --table
  orders` will silently lose ENUM columns they previously had. No
  warning, no migration note.
- No AC verifies "ENUM is dropped" — so an implementer could
  partially implement ENUM on the fold path and the spec wouldn't
  catch it.
- decisions.md does not mention ENUM at all (Decision 4 only talks
  about LOC counts).

**Required:**
1. Add a constraint: "ENUM type emission is NOT carried over from
   `cmd_load` — `validate --db --table` produces no `CREATE TYPE
   ... AS ENUM`. Low-cardinality columns retain their schema's
   `duckdb_type` (typically VARCHAR)."
2. Add a verifying AC or extend ac-08 to assert the absence of
   `CREATE TYPE` in any test CTAS.
3. Add an ENUM-loss line to ac-12's migration map under `### Changed`.

---

### F7 — ac-05 LOC delta is measured against the wrong baseline (CI gate ambiguity)

**Severity:** low (gate is achievable but the bar is wobbly).

ac-05: "Net main.rs LOC delta: ≤ -250 (from ~5143 today; target ≤
4900 after fold)." ac-13: "Zero new clippy warnings vs PR #54
baseline (the schema-verb-fold parent PR)."

Today's main.rs is 5142 lines (before PR #54). PR #54 mutates
main.rs (the schema-fold removes the `Schema` variant and dispatch
arm — net negative on LOC, but the verb fold also adds ~30 LOC of
glue in `Taxonomy`). After PR #54 lands, the baseline for THIS
card's LOC delta is no longer "5143." Yet ac-05 hard-codes "5143
today" and "≤ 4900 after fold."

**Required:** Either (a) state the baseline relative to PR #54's
post-merge HEAD (e.g., "main.rs at PR #54 head is N lines; after
fold ≤ N-250"), or (b) drop the absolute line counts and express
the delta only ("net delta ≤ -250 LOC vs PR #54 head"). Option (b)
is sturdier — line counts will drift through implementation review.

---

### F8 — Pre-CTAS vs post-CTAS sweep choice deferred to implementer (specification deferral)

**Severity:** low (both shapes work; the deferral is honest but the
spec should pick).

Implementation note line 269 says:
> Spec author picks the simpler shape — recommendation: pre-CTAS
> sweep (single-pass user table). OR run the CTAS first, sweep
> transform failures from the user table after, then DELETE failed
> rows.

A spec is the contract; leaving "pick the simpler shape" inside
the spec itself is implementer discretion at the wrong level. Both
shapes are functionally equivalent w.r.t. the ACs (the user table
ends up with the same rows and the reject sidecar ends up with the
same TRANSFORM_FAILED entries), so the ACs don't constrain. This
means the implementer is genuinely free to choose. Acceptable, but
should be explicit.

**Required:** Either (a) pin the recommendation as binding (move it
from "recommended" to "constraint: pre-CTAS sweep"), or (b) leave
it explicitly as implementer discretion in a constraint ("either
pre-CTAS sweep OR post-CTAS DELETE — implementer picks; both
satisfy the ACs"). Don't leave it as a recommendation buried in
notes.

---

### F9 — ac-12 CHANGELOG section assumption (minor)

**Severity:** low (depends on PR #54 ordering).

ac-12 says "CHANGELOG.md `[Unreleased]` (or `[0.6.19]` if the version
line is already cut) gains a `### Removed` entry." PR #54 is the
parent and almost certainly modifies CHANGELOG. The implementer needs
to land their entry in whatever section PR #54 leaves at the top of
CHANGELOG. The "or" disjunction is fine, but worth flagging that the
spec's CHANGELOG verification (`grep -A 8 "Removed"`) needs to find
the migration map ANYWHERE in CHANGELOG, not just the first match.

**Required:** Tighten verification to "the most-recent unreleased or
v0.6.19 entry" — since `grep -A 8` matches the first occurrence, an
older `### Removed` (e.g., v0.6.19's PR #51 removed `--model`,
`--sharp-only`, `eval-gittables`) will swallow the grep before the
new entries surface.

---

### F10 — `error_message` semantic shift for TRANSFORM_FAILED (worth surfacing)

**Severity:** low (decision is correct; clarify before ship).

ac-03 mandates `error_message=<staging cell value>` for
TRANSFORM_FAILED reject rows. Today's SEMANTIC_TYPE rejects use
`error_message` for the validation engine's diagnostic string
(e.g., "did not match pattern"). So `error_message` now means two
different things depending on `error_type`:

```
| error_type        | error_message contents                  |
|-------------------|-----------------------------------------|
| SEMANTIC_TYPE     | engine diagnostic ("did not match …")   |
| TRANSFORM_FAILED  | raw staging cell value                  |
```

This is a defensible choice (the cell value IS the diagnostic for
transform failures), but it's a non-obvious semantic split. The
ontology_schema block notes it but doesn't loudly flag it as a
behavioural ontology shift. Worth mentioning in MADR 0071 (ac-10)
so future maintainers don't get caught by it.

**Required:** Add a sentence to the ac-10 MADR brief: "MADR 0071
documents the `error_message` semantic split — for
SEMANTIC_TYPE rejects it carries the engine diagnostic, for
TRANSFORM_FAILED rejects it carries the failing input cell value."

---

## What the spec gets right

- **Both reject-ontology test cases are covered.** ac-03 (transform
  failure emits a TRANSFORM_FAILED reject row, the row is excluded
  from the user table) and ac-04 (NULL-in-NULL-out is NOT a
  transform failure) are two distinct test cases with two distinct
  test functions and clear, mutually-exclusive predicates. The
  failure predicate `staging IS NOT NULL AND TRY(transform) IS
  NULL` is correctly stated and constraint line 20 reinforces it.
- **Hard-removal posture for `cmd_load` is clean.** ac-05 names all
  three call-site deletions (variant + dispatch arm + fn body) and
  guards against dead-code (`clippy -D dead_code`). ac-06 routes
  user-facing failure through clap's unknown-subcommand handler with
  exit code 2 and a fixture-stable stderr substring. No shim, no
  warning, no carve-out — matches the v0.6.19 hard-removal posture
  ratified in PR #51.
- **Single source of validation truth preserved.** Constraint line 9
  ("TRY-wrap each transform — single-CTAS shape preserved, no second
  validation pass") and constraint line 17 ("Existing check-only
  mode preserved verbatim") together prevent the spec from sliding
  into a second validator pass. This is the MADR 0064 invariant
  ("validation is a pure function over (schema, rows)") and the
  spec respects it.
- **MCP surface stable.** ac-07 + constraint line 15 keep the MCP
  `ValidateRequest` shape unchanged; the description-string edit is
  the only surface change. (Modulo F3's wrong file path.)
- **Decisions traceable.** Decision pack at decisions.md is solid;
  the spec inherits D1-D6 cleanly. MADR 0071 is the right shape
  (refines 0064, supersedes nothing — accurate).

---

## Suggested rev1 changes (precise)

```
| #  | What                                              | Where                            |
|----|---------------------------------------------------|----------------------------------|
| F1 | Correct vrp_* count (8 CLI + 7 core = 15)         | ac-08 description + verification |
| F2 | Drop fictional vrp_ac0X_* names; fixture caveat   | ac-08 + impl note 273            |
| F3 | Move description string edit to lib.rs:113        | ac-07 + impl note 276            |
| F4 | Fix grep target line (drop `head -3`)             | ac-11 verification               |
| F5 | Add labelled-but-unknown-label projection branch  | ac-01 description                |
| F6 | Surface ENUM-emission drop as constraint+AC       | constraints + ac-12 migration    |
| F7 | Express LOC delta vs PR #54 head, not absolute    | ac-05                            |
| F8 | Pin pre-CTAS-vs-post sweep choice as constraint   | constraints                      |
| F9 | Tighten CHANGELOG grep target (most-recent)       | ac-12 verification               |
| F10| Document error_message semantic split in MADR     | ac-10 description                |
```

After these land, the spec is ready to drive. The architectural
shape and the AC structure are sound; the corrections are precision
work, not redesign.

---

**Verdict:** REQUEST_CHANGES
