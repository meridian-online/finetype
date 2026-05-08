# Discovery: DuckDB extension ergonomics — validate as reject-pipeline

**Date:** 2026-04-22
**Interviewer:** Nightingale
**Cards:** .orbit/cards/0005-schema-driven-data-validation.yaml, .orbit/cards/0007-duckdb-sql-extension.yaml
**Mode:** discovery

---

## Context

Triggered by a hands-on ergonomics review of the DuckDB extension output after
decision 0063 consolidated the extension code in-tree. The review surfaced six
levers:

1. Confidence-gate `load`/`schema` commitments
2. Macro layer (e.g. `finetype.amount_to_decimal(x)`) so `load` stops inlining
   280-char regex walls
3. Validate as reject-pipeline (DuckDB `reject_scans`/`reject_errors` shape)
4. Pattern-library calibration audit
5. ENUM heuristic: floor on sample size before distinct-ratio triggers
6. `_t` → `_type` suffix

The ecommerce_orders evidence anchored the need: `finetype schema` misclassified
3/12 columns (order_id → loinc, order_date → year_month, phone → SSN) and emitted
an over-restrictive `total_price` pattern that rejects plain `1688.11`. `finetype
validate` then faithfully rejected all rows.

**Scope decision:** lever 3 only. Levers 1, 2, 4, 5, 6 parked as follow-up
discoveries.

## Q&A

### Q1: Confidence gating, expanded
**Q:** Explain confidence gating a bit further, and are there existing solutions
for the macro layer?
**A:** Confidence gating explained: confidence alone is a weak signal (ecommerce
phone was 0.99 and wrong; email 0.69 and right). The stronger gate is
self-consistency — does the top prediction's own validation pattern accept the
column's values? Macro layer: DuckDB's native `CREATE MACRO` primitive is ready;
other extensions (spatial, httpfs) ship helper SQL alongside C++. Today FineType
has zero macros — the 280-char amount regex is inlined verbatim in every load
statement.

### Q2: Scope
**Q:** Tightest scope for this discovery?
**A:** Validate as reject-pipeline only. Parks levers 1, 2, 4, 5, 6 as follow-up
discoveries.

### Q3: Primary flow
**Q:** CLI-first, DuckDB-first, or shared-core for the reject pipeline?
**A:** DuckDB-first, SQL-native. Validate lives as a DuckDB table function
returning a queryable reject relation. CLI becomes a thin wrapper.

### Q4: Reject row shape
**Q:** What columns does each reject row carry?
**A:** Match DuckDB `reject_errors` closely — scan_id, file_id, line, column_idx,
column_name, error_type, csv_line, byte_position, error_message. Users who know
DuckDB's native reject shape feel at home; parse and semantic rejects can be
UNIONed.

### Q5: Shape purity vs FineType-specific detail
**Q:** How do we reconcile DuckDB-shape compatibility with richer FineType
semantics (expected_type, confidence, constraint_failed)?
**A:** Extended shape, NULL-pad on UNION. Extra columns live alongside the
DuckDB-shape base; UNION with parse rejects requires explicit projection but
rich filtering (`WHERE type_confidence < 0.5`) stays first-class.

### Q6: Schema source
**Q:** Where does the schema come from?
**A (initial):** Registered schema as catalog object (`CALL
finetype_infer_schema('orders')`).
**A (revised by Q8 principle):** Schema is a JSON file, passed inline per call
(`finetype_validate('orders', 'schemas/orders.json')`). No hidden catalog state.
Reproducible, diff-able, check-in-able.

### Q7: Schema authoring surface
**Q:** What's the surface for helping analysts author and refine a schema?
**A:** CLI-only schema authoring. `finetype schema file.csv > schema.json`
emits a candidate; analyst reviews and edits in their editor; DuckDB extension
only consumes schemas, never emits them. Clean separation: CLI authors,
extension enforces.

### Q8: Schema sanity — circular-reasoning foot-gun
**Q:** How do we protect users from "schema is wrong → all rows reject → user
can't tell why"?
**A:** Principle established: validation is a pure function of (JSON Schema,
row). If the schema rejects it, the schema is right — by definition. No
heuristics that second-guess. The foot-gun is a schema-authoring problem,
fixed upstream, not at validation time. This dissolves the question and
revises Q6: schema is a JSON file, not catalog state.

### Q9: CLI fate
**Q:** What happens to the existing `finetype validate` CLI?
**A (initial):** Rewrite as thin DuckDB wrapper that writes Parquet files.
**A (revised by Q11 correction):** CLI writes directly into a DuckDB `.db`
file, not Parquet. The analyst's end state is "open the database and query
the tables." No intermediate format.

### Q10: First slice
**Q:** What's the minimum shippable slice that earns "validate is a
face-saver"?
**A:** `finetype_validate()` table function + CLI wrapper only. No separate
`finetype_valid()` or summary helpers — users compose the rest in plain SQL
(anti-join, GROUP BY). Unix discipline: ship the primitive.

### Q11: Final shape corrections
**Q:** Does the revised shape match your mental model?
**A:** Two corrections:

1. **Name stays `finetype_validate`** — no new `finetype_rejects` name. Follow
   DuckDB's `read_csv(store_rejects=true)` idiom: one table function that
   returns valid rows and populates a reject sidecar as a side effect.
2. **CLI writes `.db` file, not Parquet** — the database file is the artefact;
   no intermediate format.

### Q12: Input shape
**Q:** (implicit) What does `finetype_validate` accept as input?
**A:** A table name string, not an arbitrary relation. User loads raw data
into a table first, then validates. This keeps `row_id` stable across
re-validations and matches DuckDB's PRAGMA/DESCRIBE idiom.

---

## Summary

### Goal
Replace today's CSV-emitting `finetype validate` with a DuckDB-native
reject pipeline. Validation becomes a pure, deterministic function of
(table name, JSON Schema path) — returning valid rows as a relation and
populating a reject sidecar that mirrors DuckDB's own `reject_errors`
shape. The CLI becomes a thin wrapper that writes the result directly
into a `.db` file. Analysts open the database and query the clean table
plus the reject sidecar in-situ.

### Constraints
- **Validation is deterministic.** Given a JSON Schema and a row, pass/fail
  is a pure function. No heuristics that second-guess the schema.
- **Schema lives in JSON files.** No catalog state. Reviewed, edited,
  checked into git. CLI authors schemas; DuckDB extension only consumes.
- **One function name: `finetype_validate`.** No new `finetype_rejects`;
  follow DuckDB's `store_rejects` idiom.
- **Table name input, not relation.** `finetype_validate('table_name',
  'schema.json')` — user loads raw data first.
- **`.db` file is the CLI artefact.** No intermediate Parquet or CSV.
- **Reject shape mirrors DuckDB's `reject_errors` with extensions.** Base
  columns compatible for UNION; extra columns (type_confidence,
  expected_type, constraint_failed, constraint_value) for rich queries.
- **Existing scalar `finetype_validate(value, schema_json)`** coexists via
  DuckDB function overloading. Spec decision: overload or rename.

### Success Criteria
- `FROM finetype_validate('orders', 'schemas/orders.json')` returns only
  valid rows as a DuckDB relation.
- `SELECT * FROM finetype_reject_errors` returns rejects with both
  DuckDB-shape base columns and FineType-specific extensions.
- `finetype validate data.csv schema.json --db orders.db --table orders`
  writes a `.db` file containing the clean table and the reject sidecar;
  exits non-zero if any rejects exist (CI gate).
- Same validation engine powers both CLI and DuckDB extension surfaces.
- Rejects UNION cleanly (with NULL-pad projection) against DuckDB's
  native `reject_errors`.
- The ecommerce_orders failure is no longer a foot-gun: a wrong schema
  produces transparent rejects whose `type_confidence` / `expected_type`
  columns make the cause obvious.

### Decisions Surfaced
- **Validation is deterministic** — chosen over heuristic schema-sanity
  checks at validate time. Rationale: keeps the contract clean; schema
  problems are fixed upstream at authoring time, not at enforcement time.
  (→ candidate MADR)
- **Schema is JSON file, not catalog state** — chosen over registered
  schemas. Rationale: reproducible, diff-able, git-check-in-able; no
  hidden state; same artefact works from CLI and DuckDB. (→ candidate
  MADR — likely supersedes or refines decision 0031)
- **CLI writes `.db` files, not Parquet** — chosen over Parquet export.
  Rationale: analyst end-state is "open the database and query"; no
  intermediate format. (→ candidate MADR)
- **Table function takes a table name string, not a relation** — chosen
  for row_id stability and DuckDB-idiom alignment. (→ spec detail,
  probably not MADR-worthy)

### Open Questions (spec-level)
- How does `finetype_validate`'s table overload coexist with the existing
  scalar `finetype_validate(value, schema_json)`? DuckDB function
  overloading should allow both, but verify.
- Reject sidecar table name: `finetype_reject_errors` vs reusing DuckDB's
  `reject_errors` with a `scan_source` discriminator?
- How does the scan_id space interact with DuckDB's native scan_id when
  both parse rejects and semantic rejects exist?
- Does the CLI's intermediate staging table need to be visible to users
  (debugging), or always dropped?
- `--strict` / exit-code semantics for CI gate — document precisely.
- What does `finetype_validate` do when the JSON Schema itself is
  malformed? Hard fail vs emit a single "schema parse error" reject?
- Performance envelope: per-row validation at 100M rows — streaming
  required, or batch-per-DuckDB-chunk sufficient?

### Parked for Follow-Up Discoveries
- **Lever 1: Confidence-gate load/schema commitments** — separate
  discovery on schema-authoring ergonomics.
- **Lever 2: Macro layer** — separate discovery, likely combined with
  load ergonomics.
- **Lever 4: Pattern-library calibration audit** — standalone data-quality
  exercise; touches taxonomy generators.
- **Lever 5: ENUM heuristic floor** — small, mechanical; could ship as a
  spec against card 0013 directly.
- **Lever 6: `_t` → `_type` suffix** — trivial; ship with the next load-
  touching spec.
