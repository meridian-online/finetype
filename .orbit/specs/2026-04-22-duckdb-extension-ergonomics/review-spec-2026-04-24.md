# Spec Review — 2026-04-22-duckdb-extension-ergonomics

**Reviewer:** Nightingale (orb:review-spec)
**Date:** 2026-04-24
**Spec:** `.orbit/specs/2026-04-22-duckdb-extension-ergonomics/spec.yaml` (v1.0)
**Interview:** `.orbit/specs/2026-04-22-duckdb-extension-ergonomics/interview.md`

**Verdict:** REQUEST_CHANGES

---

## Pass 1 — Cold read

### Shape
- 14 ACs (ac-01 … ac-14), four tagged `gate` (ac-01, ac-04, ac-09, ac-13), rest `code`/`doc`. Ambiguity score 0.065.
- Goal and constraints are clean and internally consistent with the interview. The interview's seven open questions are largely resolved in the spec (schema malformation → ac-08; scan_id → ac-05; overload vs rename → ac-07; exit semantics → ac-10; staging table visibility → ac-09 "dropped on success"). Good traceability.
- Ontology (`RejectEntry`) is fully specified with types and UNION rationale.
- Evaluation principles (0.3 / 0.3 / 0.2 / 0.2) are explicit and weighted.

### Initial findings (surfaced from cold read; drove a Pass 2)

1. **Signature mismatch with the existing code.** ac-01 names the function `table_validator::validate_table(columns, schema) -> TableValidationResult`. The current implementation at `crates/finetype-core/src/table_validator.rs:97` is `pub fn validate_table(headers: &[String], rows: &[Vec<Option<String>>], schema: &Value) -> Result<TableValidationResult, TableValidatorError>`. The spec's parenthetical "or equivalent signature" gives wiggle room, but ac-01 also asserts the result "contains the expected valid_row_indices vector" — and the present `TableValidationResult` has no such field. It carries `row_errors`, `columns` (stats), `missing_columns`, `grade`. ac-01 therefore implicitly proposes a breaking shape change to an existing public type without naming it as such.
2. **Scan_id lifecycle is underspecified.** ac-05 says "monotonic per-session counter" and the ontology repeats "Monotonic per-session". Neither defines: what "session" means (DuckDB Connection? DuckDB Database? process?), whether it persists across `.db` reopens, what happens on rollback, or where the counter lives (static, thread_local, per-connection state, a real table). The CLI in ac-09 opens a fresh `.db` per invocation — does scan_id always start at 1? Is that documented? This is the review task's first focus area and it genuinely is underspecified.
3. **Table-function side-effect contract vs DuckDB's purity assumption.** ac-05 mandates "side effect populates a sidecar table" as part of the table function's execution. DuckDB table functions are modelled as streaming scans (bind → init → execute chunks), they don't own transactions, and they don't create catalog objects as part of normal execution. Sidecar population for native `reject_errors` is implemented inside DuckDB core with privileged access to the parser state and internal tables — it is not a public extension-API primitive. The spec treats "follow DuckDB's `read_csv(store_rejects=true)` pattern" as if that pattern is a framework hook third-party extensions can reach; today it is not. This needs either (a) a spike confirming the duckdb-rs `vtab` API exposes what's needed, (b) a fallback design if it doesn't, or (c) an explicit note that this is a research spike.
4. **ac-07 overload claim needs verification against the Rust binding.** `crates/finetype-duckdb/src/lib.rs:554` registers `FineTypeValidate` as a scalar via `register_scalar_function::<FineTypeValidate>("finetype_validate")`. Table functions in duckdb-rs are registered via a different path (`register_table_function` / `VTab`). Whether the DuckDB catalog permits two different function *kinds* under the same name with disjoint arity is a claim — not a given. Spatial and httpfs don't obviously demonstrate this pattern. The spec asserts this works "via DuckDB's syntactic disambiguation" without naming the catalog entry type (e.g. `CatalogType::SCALAR_FUNCTION_ENTRY` vs `TABLE_FUNCTION_ENTRY`) or citing precedent.
5. **ac-05 verification assumes a stable sidecar across calls.** "Multiple validate calls in the same session append to the sidecar" — but if the sidecar is a real DuckDB TABLE, who creates it, when, and with what ownership (temp vs persistent)? If it's a view over per-call state, how does `COUNT(DISTINCT scan_id)` work? The verification step relies on behaviour that the design doesn't pin down.
6. **RejectEntry ontology vs the interview's "circular-reasoning foot-gun" (Q8).** The interview resolved the foot-gun by declaring validation deterministic. ac-12 re-surfaces it as a test: asserts `type_confidence` and `expected_type` populate the sidecar so the analyst can spot a wrong schema from SQL. This is internally consistent, but `type_confidence` and `expected_type` are extracted from `x-finetype-confidence` / `x-finetype-label` in the schema JSON (ac-02). If the schema was authored by the CLI at profile time, those carry the classifier's belief *at authoring time* — they are not recomputed during validation. That's fine for the "schema confidently wrong" story, but the ontology description ("Classifier's confidence for the column's inferred type") reads as if it's live. Minor wording risk but worth nailing down; also relates to the `precise_validation` work already shipped (decision 0059, `Validation::is_precise()`), which ac-02 doesn't mention.
7. **ac-09 staging-drop-on-success contract.** "The staging table is dropped on success" — what about failure? Left around for debugging, dropped anyway, or explicit `--keep-staging` flag? Unspecified. The interview (Q? missing) flagged this as an open question.
8. **ac-11 assumes `finetype schema <file>` output shape is stable.** That command's JSON Schema output includes `x-finetype-*` extensions today, but there's no constraint pinning the CLI to keep emitting them. ac-11 tests interop but doesn't declare the JSON Schema format as an input contract the spec depends on.
9. **Test count floor in ac-13 (≥15).** ACs ac-01 through ac-12 each have one or more verifications, so ≥12 tests are implied already. Requiring 15 is reasonable but arbitrary; a scenario grid (happy / all-reject / partial / multi-per-row / empty / single / each constraint = 6 constraints) gives 12 scenarios — the "across" language is loose. Fine, but name the grid explicitly.
10. **Exit conditions vs AC list.** Exit conditions mention "14 acceptance criteria marked [x] in progress.md" — progress.md isn't in the repo for this spec yet (expected, pre-implementation). Fine. Also mentions "a new MADR file in .orbit/choices/" — ac-14 verification names it more precisely but doesn't number it; next free is 0064.

These findings are enough to warrant Pass 2 on the concrete DuckDB/core surface.

---

## Pass 2 — Verification against existing code and DuckDB framework

### Core-crate reality check (finetype-core)

- `TableValidationResult` today (lines 61–70):
  ```
  pub struct TableValidationResult {
      pub total_rows: usize,
      pub valid_rows: usize,
      pub invalid_rows: usize,
      pub columns: Vec<ColumnValidationStats>,
      pub grade: String,
      pub row_errors: Vec<RowErrors>,
      pub missing_columns: Vec<String>,
  }
  ```
  The spec's ac-01 wants `valid_row_indices: Vec<usize>` and `rejects: Vec<…>` (with `constraint_failed`, `constraint_value`). Neither field exists. `RowErrors.row_index` + `CellError { column, value, error, schema_path }` is close but not isomorphic — `constraint_failed` / `constraint_value` would need to be carried separately, and `schema_path` (currently a JSON Pointer) is a different notion.
  **Ask:** ac-01 should explicitly say "add new fields" or "replace the shape", name the MSRV / SemVer impact on downstream callers (`finetype-cli`, `finetype-mcp`), and either reuse `schema_path` for `constraint_failed` or admit it's a new field. The `validate <file> <schema>` CLI (`crates/finetype-cli/src/…`) consumes the existing shape today.
- `split_rows` at line 257 already separates valid/invalid — reuse or supersede isn't declared.
- Existing tests (lines 333–559) are indexed off row_errors, not valid_row_indices — adding the new field is non-breaking but the tests should be migrated or documented as retained.

### DuckDB extension reality check (finetype-duckdb)

- Extension registers five scalar functions only: `finetype_version`, `finetype_validate` (scalar form — `lib.rs:554`), `finetype`, `finetype_detail`, `finetype_cast`, `finetype_unpack` (five `register_scalar_function` calls). Zero table functions registered today.
- The duckdb-rs crate's `vtab` / `table_function` API is the integration point the spec silently depends on. Whether it exposes:
  - (a) registering a TABLE function with the same catalog name as an existing SCALAR function;
  - (b) writing to arbitrary sidecar tables during scan execution;
  - (c) a monotonic scan_id hook shared with `read_csv`;
  … is **not established** in this codebase and not cited in the spec. ac-04 and ac-05 are the two most structurally risky ACs. They should be gated behind a named spike (e.g. "spike-0 — duckdb-rs table-fn + same-name scalar coexistence"), with an explicit fallback if the binding doesn't support same-name overload (e.g., provide `finetype_validate` scalar + `finetype_validate_rows` table function — would break the interview's explicit Q11 correction, which is fine if stated).
- `store_rejects=true` in native DuckDB works because `read_csv` is a first-party table function and reject tables (`reject_scans`, `reject_errors`) are hard-coded temp tables in `src/include/duckdb/main/relation` with privileged CSV-reader context. A third-party extension cannot emulate this without either writing into the catalog via a side-effecting CALL (not a table function) or creating normal temp tables with DDL side effects during bind/init. The latter is unusual and likely brittle. **The spec should acknowledge this gap** and either (a) define the sidecar as a normal TEMP TABLE created by a companion function (`CALL finetype_init_rejects()`), (b) redefine the contract so the table function *returns* (valid, rejects) as two relations via overload, or (c) prove the side-effect model works via a spike before ac-04/ac-05 are marked achievable.

### Scan_id semantics — concrete questions left unanswered

The review asked about multi-session / connection boundaries / WAL. Concretely:
- If the sidecar is a **persistent** table in the `.db` file, scan_id must survive DuckDB close/reopen, which means either monotonic-from-max-existing or UUID. Neither is stated.
- If the sidecar is a **TEMP** table, it doesn't persist past the DuckDB Database shutdown — which contradicts ac-09's expectation that the CLI writes `.db` with the sidecar materialised for the analyst to open later.
- If the sidecar is persistent, concurrent connections (e.g. two CLI invocations against the same `.db`) must not race. WAL + explicit INSERT ordering is fine in principle but the spec doesn't say INSERT-from-bind or append-on-close.
- Rollback: if the validate CALL errors mid-chunk, are partial rejects committed or rolled back? ac-08 says "no partial reject population with a new scan_id" for malformed-schema failure, but says nothing about mid-scan failures.

**Ask:** add a "scan_id & sidecar lifecycle" subsection (in constraints or as an ADR that the spec references) specifying:
1. Sidecar is persistent vs temp (recommend persistent for CLI use case).
2. scan_id = `MAX(scan_id) + 1` at bind time (or UUID-v7 for parallel safety).
3. INSERT semantics on error (mid-scan rollback behaviour).
4. Multi-connection rules (sequential only, or concurrent-safe via DuckDB's default transaction isolation).

### ac-07 same-name overload — evidence check

The `read_csv` example in the interview is *not* a same-name scalar+table coexistence; `read_csv` is table-only. A better precedent to cite would be `json_each` / `json_tree` (table functions with distinct names) or the JSON extension's macro-to-scalar pattern. I am not aware of an existing DuckDB extension where the same identifier binds both a scalar and a table function. This doesn't mean it's impossible — DuckDB's catalog does key by `(schema, type, name)` with type being a distinct enum — but **the spec cites no precedent and has no spike.** If the duckdb-rs binding forbids it, ac-07's "no rename, no deprecation" constraint is unsatisfiable. This should be investigated before freezing the spec.

### Gate-ordering check (ac-01 / ac-04 / ac-09 / ac-13)

Reviewer prompt flagged these as the bottlenecks. Assessment:
- **ac-01 first: correct.** The shared core type must land before either CLI or DuckDB can consume it. But ac-01 as written is two tasks: "add valid_row_indices + structured rejects" and "one signature consumed by both". Splitting into ac-01a (shape) and ac-01b (both call sites) would help sequencing.
- **ac-04 second: structurally right but assumes the spike.** Before ac-04 can turn green, the "can duckdb-rs register a table fn?" spike has to close. Promote that to a named gate (ac-01.5?) or make ac-04 explicitly depend on a spike deliverable.
- **ac-09 third: correct.** CLI wraps the extension; order is sound.
- **ac-13 last: correct.** Test-coverage rollup at the end.
- **Missing gate:** ac-05 (sidecar mechanism) is tagged `code` but is a bigger design risk than ac-04. Consider promoting to `gate` or merging the sidecar-design AC into ac-04.

### Scope sizing

14 ACs across three crates (core, duckdb, cli) plus docs is at the upper bound of "one spec". Achievable if the duckdb-rs spike is de-risked; otherwise, consider a two-spec split:
- **Spec A (this one, scoped down):** core `validate_table` shape change + scalar overload unchanged + CLI writes `.db` via *two* function surfaces (`finetype_validate_rows` table + existing scalar). Defers same-name overload.
- **Spec B (follow-up):** same-name catalog overload once the duckdb-rs upstream supports it (or after a precedent is found).

This is a reasonable fallback to keep in the spec's "rollback plan" section (currently absent).

---

## Pass 3 — not run

Pass 2 surfaced enough concrete issues to act on. A third pass (e.g. cross-spec redundancy check, decision-register audit for superseding 0031/0032) can be folded into the response to these findings rather than drawn out separately.

---

## Summary of required changes

**Blocking:**
1. **ac-01 shape clarification.** Either name the new fields (`valid_row_indices: Vec<usize>`, `rejects: Vec<RejectRecord>` with explicit field list mirroring the RejectEntry ontology) and call out this is a breaking change to `TableValidationResult`, or change the verification to match the existing shape. Currently ac-01 can't be implemented against the codebase without a design decision it doesn't capture.
2. **ac-04 / ac-05 spike.** Add a pre-gate AC (or a dedicated "Design Risks" section) that lands the duckdb-rs binding investigation: can a third-party extension (a) register a table function, (b) with the same name as an existing scalar, (c) that writes side-effect rows into a sidecar during scan. Name the fallback if any of (a/b/c) fails. This is the spec's biggest open risk and it's currently silent.
3. **Scan_id & sidecar lifecycle.** Add a constraints-level spec of sidecar persistence (temp vs persistent), scan_id allocation (monotonic-from-max vs UUID), failure behaviour (mid-scan rollback), and multi-connection rules. Referenced by ac-05 and ac-08.

**Non-blocking (nice to have):**
4. ac-09 failure-path behaviour for the staging table (drop on failure or keep?).
5. ac-11 declare the `finetype schema` JSON output format as an input contract for this spec.
6. ac-12 clarify that `type_confidence` / `expected_type` are authored-time values from `x-finetype-*`, not recomputed at validation (wording fix only).
7. ac-13 replace "minimum 15 new test functions" with an explicit scenario grid (happy, all-reject, partial, multi-per-row, empty, single, 6 constraints = 12 named; add 3 named integration scenarios if desired).
8. ac-14 name the MADR number (next free = 0064) and whether this supersedes 0031 / 0032 (interview says "likely supersedes or refines 0031").
9. Add a `rollback_plan` to the spec: if the same-name overload is blocked upstream, accept `finetype_validate_rows` as the table-function name and keep scalar `finetype_validate` unchanged.

**Strengths worth preserving:**
- Goal, constraints, and ontology are crisp and well-traced to the interview.
- Evaluation principles with weights — unusual and good.
- ac-12's "ecommerce_orders as the face-saver test" is a strong end-to-end anchor.
- Determinism (ac-03) and backward-compatibility (ac-07) are both first-class, which is the right defensive posture.

Once blockers (1, 2, 3) are addressed, this is APPROVE territory — the design is sound in spirit; it just under-specifies its two structural dependencies on DuckDB's extension framework.
