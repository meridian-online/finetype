# Spec Review

**Date:** 2026-04-24
**Reviewer:** Context-separated agent (fresh session)
**Spec:** `.orbit/specs/2026-04-22-duckdb-extension-ergonomics/spec.yaml` (v1.1)
**Verdict:** APPROVE

---

## Review Depth

| Pass | Triggered by | Findings |
|------|-------------|----------|
| 1 — Structural scan | always | 4 (1 LOW, 3 INFO) |
| 2 — Assumption & failure | content signals (cross-system boundaries: core <-> duckdb-rs upstream, CLI writes `.db` files, schema-as-input-contract) | 3 (all LOW / advisory) |
| 3 — Adversarial | not triggered — Pass 2 surfaced no structural contradictions, cascading failure modes, untestable ACs, or unclear downstream impact | — |

---

## Change-delta vs v1.0 review (`review-spec-2026-04-24.md`)

The prior review (v1.0 → REQUEST_CHANGES) raised three blocking items and six non-blockers. v1.1's changelog claims all nine are addressed. Cold-read verification:

- **Blocker 1 (ac-01 shape clarification).** v1.1 now names both new fields verbatim (`valid_row_indices: Vec<usize>`, `rejects: Vec<RejectRecord>`), enumerates every `RejectRecord` field with types, enumerates the `constraint_failed` token set, lists every preserved legacy field by name, and adds a workspace-scoped additivity constraint. ac-01 verification names the test, the fixture shape (3 rows, 1 valid, 2 failing), and the four specific assertions. Cross-checked against `crates/finetype-core/src/table_validator.rs:61-70` — the preserved-field list in ac-01 matches the code exactly (`total_rows`, `valid_rows`, `invalid_rows`, `columns`, `grade`, `row_errors`, `missing_columns`). Resolved.
- **Blocker 2 (ac-04/ac-05 spike).** v1.1 introduces ac-04 as a pre-gate spike AC with three explicit feasibility questions (vtab in 1.4.4 under loadable-extension, same-name coexistence, runtime schema derivation) and a named written deliverable (`spike-duckdb-rs.md`). ac-05 is reshaped to return an annotated relation (no table-function side effects) — the CLI now owns sidecar materialisation (ac-06, ac-09). The "side effect during scan" design risk the v1.0 review flagged is eliminated, not merely documented. Resolved.
- **Blocker 3 (scan_id & sidecar lifecycle).** v1.1 adds a constraint pinning scan_id allocation (fresh `.db` → 1, `--append` → MAX+1), naming the CLI as sole writer, and declaring concurrent writers unsupported in v1. Sidecar is persistent (implied by CLI materialisation into `.db`). Resolved.
- **Non-blockers 4–9.** ac-09 now specifies staging-drop behaviour for both success and failure paths (`finally` block + transaction rollback). A new constraint declares `finetype schema`'s JSON output as an input contract (covers non-blocker 5). ac-12 wording now states explicitly that `type_confidence` / `expected_type` are authored-time values, not recomputed (covers 6). ac-13 now has an explicit scenario grid across the three crates totalling 15 named functions (covers 7). ac-14 names MADR 0064 and the supersession/refinement relationship to 0031/0032 (covers 8). A `rollback_plan` section with three named scenarios (A/B/C) covers 9.

All nine prior items are addressed at the level of written spec text. What remains is checking for new issues introduced by v1.1.

---

## Findings

### [LOW] ac-13 grid counts to 15, wording says "≥15"
**Category:** test-gap
**Pass:** 1
**Description:** ac-13 names the scenario grid exhaustively: 7 core + 4 DuckDB + 4 CLI = 15. Header says "at least 15"; `grep` verification line says "at least 15 matches". The grid is explicit and complete, but "at least" leaves room for the CLI-exit-code sub-cases (4 sub-cases under `cli_exit_code_grid`) to be counted as one function or four — grep will see whatever the author writes. Minor.
**Evidence:** spec.yaml ac-13 lines 300–319.
**Recommendation:** Either drop "at least" (the grid is the grid) or state explicitly that sub-cases within `constraint_grid` and `cli_exit_code_grid` count toward the 15. Not blocking — a follow-up reviewer can count functions against the grid during PR review.

### [LOW] Gate-AC verification rule 5 — all four gate ACs pass the deterministic check
**Category:** test-gap
**Pass:** 1
**Description:** Four ACs tagged `gate`: ac-01, ac-04, ac-05, ac-09, ac-13. Running the three deterministic rules over each `verification` field:
- ac-01: 534 chars, no placeholder token, non-empty. PASS.
- ac-04: 507 chars, no placeholder token, non-empty. PASS.
- ac-05: 499 chars, no placeholder token, non-empty. PASS.
- ac-09: 462 chars, no placeholder token, non-empty. PASS.
- ac-13: 213 chars, no placeholder token, non-empty. PASS.

All gates satisfy the deterministic check. Recording as a LOW note only because one AC tagged `gate` in v1.0 (ac-04) is new in v1.1 and its verification leans heavily on "spike file exists with three sections (a/b/c)" — content-quality of those sections is not machine-checkable. This is the spec's single biggest live uncertainty (see Pass 2).
**Evidence:** spec.yaml ac-01, ac-04, ac-05, ac-09, ac-13.
**Recommendation:** None for Pass 1. See Pass 2 note on ac-04 closure criteria.

### [INFO] ac-02 enumerates 6 constraint tokens; ac-01 enumerates 6; list consistency is fine
**Category:** missing-requirement
**Pass:** 1
**Description:** ac-01's `constraint_failed` token list: `pattern|min_length|max_length|enum|type|required`. ac-02 adds `other` as the fallthrough. ac-13's `constraint_grid` enumerates "one sub-test per pattern/min_length/max_length/enum/type/required" (6, not 7). No test is specified for the `other` fallthrough. This is consistent with "other is a catch-all for unforeseen schema keywords" but means the fallthrough branch is not in the test grid.
**Evidence:** spec.yaml ac-01 lines 38–40, ac-02 lines 60–66, ac-13 lines 304–306.
**Recommendation:** Optional — add a 7th sub-case testing that an unknown keyword (e.g. `format: email` or `multipleOf: 3`) produces `constraint_failed = "other"`. Not blocking.

### [INFO] ac-14 frontmatter date is a prediction
**Category:** test-gap
**Pass:** 1
**Description:** ac-14 verification asserts the MADR 0064 file's frontmatter lists `date-created: 2026-04-24`. The spec itself is dated 2026-04-24 (metadata.timestamp). If implementation lands on a different day, this assertion will fail cosmetically without indicating a real problem. Low severity — easy to fix at implementation time.
**Evidence:** spec.yaml ac-14 verification lines 340–342; metadata.timestamp line 426.
**Recommendation:** Optional — reword to "the frontmatter contains a valid `date-created` field" or accept the mild brittleness.

### [LOW] ac-04 spike closure is the live load-bearing gate
**Category:** assumption
**Pass:** 2
**Description:** The entire spec's structural design (ac-05 shape, ac-07 coexistence claim, ac-13 DuckDB test names) depends on spike outcomes. The spec handles this well: ac-04 is explicitly a pre-gate, its outcome parameterises downstream ACs ("ratified name" language in ac-05/ac-07), and `rollback_plan` Scenarios A/B/C each name a concrete fallback. The one residual soft spot: ac-04's verification accepts "a written finding … citing a duckdb-rs source reference or a minimal working extension snippet" — "citing a source reference" is weaker than "compiles a proof". The trivial smoke test (`finetype_spike(n BIGINT)`) anchors at least (a); (b) and (c) could in principle land with just a doc citation.
**Evidence:** spec.yaml ac-04 lines 90–118.
**Recommendation:** Optional — strengthen ac-04 verification to require a compiled smoke test for each of (a/b/c), not just (a). If (b) cannot be smoked, that pushes Scenario B. If (c) cannot be smoked, that pushes Scenario C. The `rollback_plan` already names both, so this is a verification-rigour ask rather than a design gap. Acceptable as-is; the spec's own rollback discipline is the structural safety net.

### [LOW] ac-07 test relies on reading the spike file at test time
**Category:** test-gap
**Pass:** 2
**Description:** ac-07 verification: "The test reads the ratified name from the spike file (ac-04) and parametrises accordingly." This couples test code to markdown file parsing of `spike-duckdb-rs.md`. If the spike file's "ratified name" section is informal prose, the test's parser is brittle.
**Evidence:** spec.yaml ac-07 verification lines 178–183.
**Recommendation:** Optional — at implementation time, the spike should emit the ratified name into a machine-readable location (e.g. a `const TABLE_FN_NAME: &str = "..."` in the extension source, or a JSON file in the spec dir). Keep markdown for humans, source-of-truth in code/JSON. Not a spec-freeze blocker; it's an implementation hint.

### [LOW] Concurrent-writer disclaimer punts, ergonomics question remains implicit
**Category:** failure-mode
**Pass:** 2
**Description:** The scan_id constraint states "Concurrent writers are NOT supported in v1; the CLI documents sequential use." Fine, but what happens if two CLI invocations race? DuckDB file locks + single-writer model typically yield "database is locked" errors. The spec doesn't say whether the CLI detects this and surfaces a clean message (exit 2 path under ac-10?) or leaks a raw DuckDB error. ac-10 covers "DuckDB error → exit 2" generically, so this is likely covered transitively, but the failure mode isn't named.
**Evidence:** spec.yaml constraint line 21; ac-10 lines 231–246.
**Recommendation:** Optional — add to ac-10 verification a case "concurrent invocation against locked `.db` → exit 2 with DuckDB-locked-database message surfaced". Low severity; analyst running validate manually won't hit this often.

---

## Honest Assessment

Spec v1.1 is ready. Every blocking and non-blocking item from the v1.0 review was addressed in writing, and the addresses are concrete rather than hand-waved (fields named, constraints pinned, a rollback_plan with three named scenarios, MADR number fixed, gate ordering made explicit). The restructure of ac-05 — moving sidecar materialisation from a table-function side effect to a CLI-owned projection — is the strongest single improvement: it eliminates the review's deepest structural concern (third-party extensions writing catalog state during scan execution) rather than merely documenting it. The remaining risk is load-bearing and known: ac-04's spike might return Scenario A/B/C outcomes that degrade ergonomics, but the spec's own rollback_plan names each outcome and the corresponding degradation, so "spike returns bad news" is a design-quality outcome rather than a re-plan trigger. No structural blockers, no contradictions, no untestable ACs. Pass 3 was not triggered and there is no reason to force it.
