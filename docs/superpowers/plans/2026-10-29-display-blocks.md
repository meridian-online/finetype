# Display blocks Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Every definition whose cast yields a number or a date carries a validated `display` block (a d3-format or d3-time-format specifier plus money, currency, fraction and bytes flags), exported through `taxonomy -o json` and datapackage output.

**Architecture:** A `Display` struct on `Definition` with its own validator in a new `finetype-core` module, mirroring the `frictionless` block. A kind classifier decides which definitions need one; `finetype check` and a `cargo test` mirror enforce it. The blocks are authored in `labels/definitions_*.yaml`.

**Tech Stack:** Rust (serde, serde_yaml, regex, chrono already in `finetype-core`), YAML taxonomy.

**Spec:** `docs/superpowers/specs/2026-10-29-display-blocks-design.md`

## Global Constraints

- Block keys, snake_case everywhere: `number`, `time`, `money`, `currency`, `fraction`, `bytes`. Booleans are written only when true.
- No specifier contains `$`.
- No `transform` or `broad_type` changes in this plan.
- No new CLI verb or output format; JSON Schema output and the DuckDB extension are unchanged.
- One cargo build at a time on the owner's Mac; never run two implementers in parallel.
- Public repo: no tracker refs, private repo names or local paths in commits, PR text or files.

## Review Focus

1. A YAML `display` block with an unknown key (`numbr:`) must fail to load or fail `check`, not be silently dropped. Task 1 adds `#[serde(deny_unknown_fields)]` and a test.
2. A specifier with a stray `$` hidden after a fill character (`$>10,.2f` uses `$` as fill) is still refused; the rule is "no `$` anywhere". Task 1 test.
3. `fraction` on a label whose transform divides by 1000 is refused (only 100 and 10,000 count). Task 2 test.
4. A datapackage field whose label has no block gets no `x-finetype-display` key at all (not `null`, not `{}`). Task 6 test.
5. A `time` specifier with a padding modifier on a non-directive (`%-x` is valid, `%-k` is not) is refused. Task 1 test.

## File map

- Create `crates/finetype-core/src/display.rs`: `Display`, `DisplayKind`, `Display::validate`, `display_kind_for`, `check_display`, `DISPLAY_PENDING`, `KIND_OVERRIDES`.
- Modify `crates/finetype-core/src/taxonomy.rs`: `Definition.display` field; gate test mirror beside `test_every_definition_has_valid_frictionless`.
- Modify `crates/finetype-core/src/lib.rs`: `pub mod display;` and re-exports.
- Modify `crates/finetype-cli/src/cmd_taxonomy.rs`: display gate in `cmd_check` after the frictionless gate; `display` in both JSON forms of `cmd_taxonomy`.
- Modify `crates/finetype-core/src/datapackage.rs`: `x-finetype-display` on fields.
- Modify `labels/definitions_{finance,representation,geography,identity,datetime,technology}.yaml`.
- Create `crates/finetype-core/tests/display_blocks.rs`: the table test and the time round-trip test over real `labels/`.
- Modify `docs/ARCHITECTURE.md`, `CHANGELOG.md` (task 8).

---

### Task 1: The `Display` block and its validator

**Files:**
- Create: `crates/finetype-core/src/display.rs`
- Modify: `crates/finetype-core/src/taxonomy.rs` (add field to `Definition`, after `frictionless`)
- Modify: `crates/finetype-core/src/lib.rs`

**Interfaces:**
- Produces:
  ```rust
  #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
  #[serde(deny_unknown_fields)]
  pub struct Display {
      #[serde(default, skip_serializing_if = "Option::is_none")] pub number: Option<String>,
      #[serde(default, skip_serializing_if = "Option::is_none")] pub time: Option<String>,
      #[serde(default, skip_serializing_if = "is_false")] pub money: bool,
      #[serde(default, skip_serializing_if = "Option::is_none")] pub currency: Option<String>,
      #[serde(default, skip_serializing_if = "is_false")] pub fraction: bool,
      #[serde(default, skip_serializing_if = "is_false")] pub bytes: bool,
  }
  impl Display { pub fn validate(&self) -> Result<(), String>; }
  ```
  and `Definition.display: Option<Display>` (`#[serde(default, skip_serializing_if = "Option::is_none")]`).

- [ ] **Step 1: Write the failing tests** in a `#[cfg(test)] mod tests` in `display.rs`:
  - `accepts_spec_examples`: `(,.2f`+money, `~%`+fraction, `,d`+bytes, `,.2f`+money+`INR`, `d`, `~e`, `~s`, `,`, `,.8~f`; time `%d/%m/%Y`, `%Y-%m-%dT%H:%M:%S.%L%Z`, `Q%q %Y`, `%Y年%-m月%-d日`, `%a %b %-d %H:%M:%S %Y` all `Ok`.
  - `refuses`: each of these is `Err`: both `number` and `time`; neither; `$,.2f`; `$>10,.2f`; `,.2q` (bad type); `%Y-%k` and `%-k` (unknown directive); trailing lone `%`; `fraction` with `,.2f`; `currency` without `money`; `currency: "inr"`; `money` with `time`; `bytes` with `money`; `fraction` with `money`.
  - `unknown_key_refused`: `serde_yaml::from_str::<Display>("numbr: d")` is `Err`.
  - `round_trips_snake_case`: `serde_json::to_value(Display{number:Some(",d".into()), bytes:true, ..Default::default()})` equals `json!({"number": ",d", "bytes": true})`.
- [ ] **Step 2: Run** `cargo test -p finetype-core display::` — expect compile failure (module missing).
- [ ] **Step 3: Implement** `Display` and `validate` in `display.rs`. Number grammar as one anchored regex: `^(?:(.)?[<>=^])?[-+( ]?[#]?0?\d*,?(?:\.\d+)?~?[efgrs%pboxXcd]?$` after first refusing any `$`. Time: scan `%`, optional modifier in `0_-`, then a directive in `aAbBcdefgGHIjLmMpqQsSuUVwWxXyYZ%`; anything else after `%` is an error. Flag rules exactly as spec section 1. Add the field to `Definition`; `pub mod display;` and `pub use display::Display;` in `lib.rs`.
- [ ] **Step 4: Run** `cargo test -p finetype-core display::` — expect PASS. Also `cargo test -p finetype-core taxonomy::` still passes (the field is optional, so labels load unchanged).
- [ ] **Step 5: Commit** `Display block on a definition, with its validator`.

### Task 2: Which definitions take a block, and the per-definition check

**Files:**
- Modify: `crates/finetype-core/src/display.rs`
- Modify: `crates/finetype-cli/src/cmd_taxonomy.rs` (`cmd_check`, after the frictionless gate)

**Interfaces:**
- Consumes: `Display`, `Definition` (task 1).
- Produces:
  ```rust
  #[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum DisplayKind { Number, Time }
  pub const DISPLAY_PENDING: &[&str] = &["representation.numeric.percentage"];
  pub const KIND_OVERRIDES: &[(&str, DisplayKind)] = &[("technology.identifier.snowflake_id", DisplayKind::Time)];
  pub fn display_kind_for(key: &str, def: &Definition) -> Option<DisplayKind>;
  /// `require`: whether a missing block is an error (false until task 5).
  pub fn check_display(key: &str, def: &Definition, require: bool) -> Result<(), String>;
  ```

- [ ] **Step 1: Write the failing tests** (`display.rs` tests, building `Definition`s from small YAML strings):
  - `kind_from_broad_type`: `SMALLINT`/`BIGINT`/`DECIMAL`/`DOUBLE` → `Number`; `DATE`/`TIME`/`TIMESTAMP`/`TIMESTAMPTZ` → `Time`; `INTERVAL`, `BOOLEAN`, `UUID`, `JSON`, `POINT` → `None`.
  - `kind_from_varchar_transform`: `VARCHAR` with transform `CAST(REGEXP_REPLACE({col}, '%$', '') AS DECIMAL(10,4)) / 100` → `Number`; with `... AS INTEGER)` → `Number`; with `CAST({col} AS VARCHAR)` → `None`.
  - `snowflake_override`: key `technology.identifier.snowflake_id`, `broad_type: BIGINT` → `Time`.
  - `check_display` cases: block of wrong kind → `Err`; block on a `None`-kind definition → `Err`; missing block with `require=true` → `Err`, with `require=false` → `Ok`; missing block on a pending key with `require=true` → `Ok`; `fraction` with transform ending `/ 100` → `Ok`, `/ 10000` → `Ok`, `/ 1000` → `Err`, no division → `Err`; an invalid block → the validator's `Err`.
- [ ] **Step 2: Run** `cargo test -p finetype-core display::` — expect FAIL.
- [ ] **Step 3: Implement.** Numeric-transform detection: regex over the trimmed transform, `(?i)AS\s+(DECIMAL|DOUBLE|INTEGER|BIGINT|SMALLINT)(\(\s*\d+\s*,\s*\d+\s*\))?\s*\)\s*(/\s*\d+)?$`. Fraction check: transform ends `/\s*(100|10000)\s*$`. `KIND_OVERRIDES` is consulted before `broad_type`. Wire `cmd_check`: loop `taxonomy.definitions()`, collect `check_display(key, def, false)` errors, print `Display block check FAILED (N definition(s)):` with one `  - ` line each and `exit(1)`, same shape as the frictionless gate.
- [ ] **Step 4: Run** `cargo test -p finetype-core display::` — PASS; `cargo run -- check` — exits 0 (no blocks exist yet, and none are required).
- [ ] **Step 5: Commit** `check refuses a malformed or misplaced display block`.

### Task 3: Number blocks

**Files:**
- Modify: `labels/definitions_finance.yaml`, `definitions_representation.yaml`, `definitions_geography.yaml`, `definitions_identity.yaml`, `definitions_datetime.yaml` (year only)
- Create: `crates/finetype-core/tests/display_blocks.rs`

**Interfaces:**
- Consumes: `Display`, `check_display` (tasks 1–2).

- [ ] **Step 1: Write the failing test** `number_blocks_match_the_design` in `display_blocks.rs`: load `Taxonomy::from_directory` from `../../labels`, and for every row of spec section 5's table except `representation.numeric.percentage` assert the definition's `display` equals the row exactly (specifier and flags; absent flags false). The table in code, as `&[(&str, &str, &[&str], Option<&str>)]` (key, number, flags, currency), 26 rows.
- [ ] **Step 2: Run** `cargo test -p finetype-core --test display_blocks number_blocks` — expect FAIL.
- [ ] **Step 3: Add the blocks** to the YAML, placed directly after each definition's `frictionless` block. Quote every specifier (`number: "(,.2f"`).
- [ ] **Step 4: Run** the test — PASS; `cargo run -- check` — exits 0.
- [ ] **Step 5: Commit** `Number types say how their values are shown`.

### Task 4: Time blocks

**Files:**
- Modify: `labels/definitions_datetime.yaml` (79 temporal leaves), `labels/definitions_technology.yaml` (`snowflake_id`)
- Modify: `crates/finetype-core/tests/display_blocks.rs`

- [ ] **Step 1: Write the failing tests** in `display_blocks.rs`:
  - `every_temporal_definition_has_a_time_block`: every definition with `display_kind_for == Some(Time)` has `display.time`.
  - `time_blocks_follow_the_source_form`: for each temporal definition with a `format_string`, outside the exception set, the specifier equals the mechanical translation (`%g`→`%L`, `%z`→`%Z`, literal trailing `Z`→`%Z`, everything else unchanged). Exception set and their exact expected specifiers, from spec section 6: `compact_ymd`/`compact_dmy`/`compact_mdy` `%Y-%m-%d`; `compact_ym` `%Y-%m`; `iso_8601_compact` `%Y-%m-%dT%H:%M:%S`; the four epochs `%Y-%m-%d %H:%M:%S`; `rfc_2822_ordinal` `%d %b %Y %H:%M:%S %Z`; `jp_era_short`/`jp_era_long` `%Y-%m-%d`; `julian` `%y-%j`; `chinese_ymd` `%Y年%-m月%-d日`; `korean_ymd` `%Y년 %-m월 %-d일`; `period.quarter` `Q%q %Y`; `snowflake_id` `%Y-%m-%d %H:%M:%S.%L`.
  - `time_specifiers_round_trip`: for each mirrored (non-exception) definition, parse its first sample with chrono using its `format_string`, format it with the d3 specifier translated to chrono (`%L`→`%3f`, `%f`→`%6f`, `%Z`→`%:z`, `%q` unused here), and assert it parses back with the original `format_string` to the same value. Skip labels whose `format_string` chrono cannot parse, listing them in the assertion message so the skip is visible.
- [ ] **Step 2: Run** `cargo test -p finetype-core --test display_blocks time_` — expect FAIL.
- [ ] **Step 3: Add the blocks.** Generate the mirrored specifiers with a throwaway script in the session scratchpad (not committed), apply them to the YAML after each `frictionless` block, then hand-write the exception set.
- [ ] **Step 4: Run** `cargo test -p finetype-core --test display_blocks` — PASS; `cargo run -- check` — exits 0.
- [ ] **Step 5: Commit** `Date and time types say how their values are shown`.

### Task 5: The completeness gate

**Files:**
- Modify: `crates/finetype-cli/src/cmd_taxonomy.rs` (`check_display(…, true)`)
- Modify: `crates/finetype-core/src/taxonomy.rs` (test `test_every_definition_has_valid_display` beside the frictionless mirror)

- [ ] **Step 1: Write the test** `test_every_definition_has_valid_display`: same loader as the frictionless mirror; collect `check_display(key, def, true)` errors over every definition; assert empty, listing failures.
- [ ] **Step 2: Run** `cargo test -p finetype-core test_every_definition_has_valid_display` — PASS (tasks 3–4 supplied every block; percentage is pending). Then delete one block locally, rerun, confirm FAIL naming that key, and restore it.
- [ ] **Step 3: Switch** `cmd_check` to `require = true`.
- [ ] **Step 4: Run** `cargo run -- check` — exits 0.
- [ ] **Step 5: Commit** `check requires a display block on every number and date type`.

### Task 6: Export in taxonomy JSON and datapackage output

**Files:**
- Modify: `crates/finetype-cli/src/cmd_taxonomy.rs` (`cmd_taxonomy` short JSON, `definition_to_full_json`)
- Modify: `crates/finetype-core/src/datapackage.rs` (field emission, after `x-finetype-label`)
- Test: `crates/finetype-core/src/datapackage.rs` tests; `crates/finetype-cli/tests/cli_golden.rs`

- [ ] **Step 1: Write the failing tests:**
  - datapackage unit test `field_carries_display_block`: extend `test_taxonomy()` with a definition that has `display: {number: "d"}` and one without; emit; assert the first field's `x-finetype-display == json!({"number": "d"})` and the second field has no `x-finetype-display` key.
  - `cli_golden.rs` `taxonomy_json_includes_display`: `finetype taxonomy finance.currency.amount_accounting -o json` → `[0]["display"] == json!({"number": "(,.2f", "money": true})`; with `--full` the same; `finetype taxonomy representation.identifier.uuid -o json` → `[0]` has no `display` key.
- [ ] **Step 2: Run** `cargo test -p finetype-core datapackage::` and `cargo test -p finetype-cli --test cli_golden taxonomy_json_includes_display` — expect FAIL.
- [ ] **Step 3: Implement.** Datapackage: `if let Some(d) = def.and_then(|d| d.display.as_ref()) { field.insert("x-finetype-display".into(), serde_json::to_value(d)?) }` placed after `x-finetype-label`. Taxonomy JSON: insert `display` only when `Some`, in both forms.
- [ ] **Step 4: Run** both tests — PASS; then the full `cargo test -p finetype-cli --test cli_golden` and `--test datapackage_conformance` to confirm no golden moved unexpectedly (a field gaining `x-finetype-display` is expected; update a golden only for that).
- [ ] **Step 5: Commit** `Datapackage fields and taxonomy JSON carry the display block`.

### Task 7: The percentage block (waits on the percentage cast storing a fraction)

Start only after the change that makes `representation.numeric.percentage`'s transform divide by 100 is on `main`; rebase onto it first.

**Files:**
- Modify: `labels/definitions_representation.yaml`, `crates/finetype-core/src/display.rs` (`DISPLAY_PENDING` becomes `&[]`), `crates/finetype-core/tests/display_blocks.rs` (add the percentage row)

- [ ] **Step 1:** Add the percentage row (`~%`, fraction) to `number_blocks_match_the_design`; run it — FAIL.
- [ ] **Step 2:** Add `display: {number: "~%", fraction: true}`; empty `DISPLAY_PENDING`.
- [ ] **Step 3: Run** `cargo test -p finetype-core display` and `cargo run -- check` — PASS and exit 0. If the transform does not yet end in `/ 100`, `check` fails on the fraction rule: stop, the dependency has not landed.
- [ ] **Step 4: Commit** `A percentage is shown as one`.

### Task 8: Fold, review, pull request

- [ ] **Step 1:** Add a "How a type's values are shown" subsection to `docs/ARCHITECTURE.md` under Taxonomy Definitions: the block's fields, which definitions take one and why (`broad_type` plus the transform's final cast, the `snowflake_id` override), the export surfaces, and each ruling from the spec's "Calls" list as approved, with its reason. Add a `CHANGELOG.md` Unreleased entry.
- [ ] **Step 2:** Delete `docs/superpowers/specs/2026-10-29-display-blocks-design.md` and this plan; commit `Fold the display-block design into the architecture doc`.
- [ ] **Step 3:** `make` pre-push checks pass (`cargo fmt --check`, `cargo clippy -- -D warnings`); push; open the pull request.
- [ ] **Step 4:** `roborev review --branch --wait`; fix or answer each finding at Medium and above; close the review. `gh pr checks <pr> --watch`. The pull request description carries CI's test totals and the rulings.

## Verification costs

Not measured this session (no build was run). Expected order: `cargo test -p finetype-core display::` and the `display_blocks` integration test are seconds after the first build; `cargo run -- check` runs every generator and takes on the order of a minute; `cli_golden` builds the CLI with the embedded model and is the slowest local step. CI is the full gate.
