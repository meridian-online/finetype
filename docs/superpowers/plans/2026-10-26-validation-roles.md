# Validation roles Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Separate a type's validation rule into its four roles (model input, discrimination, published conformance, transform), then fix the first batch of 13 disputed rules to agree with their defining sources.

**Architecture:** The model reads a frozen snapshot of the rules it was trained on, so published-rule fixes cannot move it. Validation gains a grade per value (conforms / conforms with a note / fails) via optional per-type note rules. A per-type detector hook lets Sharpen and the veto keep discrimination stricter than the source where a source-conformant rule would be too broad. The 13 veto-allowlist types are then fixed into this structure, one task each (two pairs share a task).

**Tech Stack:** Rust workspace (finetype-core, finetype-model, finetype-cli, finetype-duckdb, finetype-train), taxonomy YAML under `labels/`, Python eval scripts under `scripts/` (run via `eval/gittables/.venv/bin/python`).

**Spec:** `docs/superpowers/specs/2026-10-26-validation-roles-design.md`

## Global Constraints

- One cargo build at a time on Hugh's Mac (16 GB). Do not run implementers in parallel.
- `infer`/`profile`/`taxonomy`/`validate`/`generate` stay stable for `arc mcp`. Every output change is additive.
- Rules stay value-based. No header-based discrimination is added.
- Public repo: no private planning references (tracker ids, private repo names, local paths) in tracked text, commit messages or PR text.
- Gold may fall by at most 2 points **cumulatively across the whole job**, from the baseline Task 0 measures. A batch that would exceed it stops for Hugh's ruling.
- The corpus-honest gate stays blocking for rule changes. A NO-GO is adjudicated per transition against the defining source, and only Hugh can override it.
- Gold re-labels go through panel-proposes / Hugh-ratifies. Until a re-label is ratified, it is reported as a known wrong label.
- Before each commit, run `cargo fmt --all` and `cargo clippy --all-targets -- -D warnings` plus the tests the change touches. CI runs the full gate.

## Review Focus

1. **Empty, whitespace and `null` cells.** Grading must treat them as validation does today (skipped, never "fails" or "noted"). Pinned in Task 5.
2. **A model directory with no snapshot** (older models, `FINETYPE_MODEL_DIR` pointing at a hand-built dir). It must load and behave exactly as today. Pinned in Task 2.
3. **A snapshot whose keys disagree with `config.type_index_keys`** (stale file copied beside a different model). This must fail loudly at load, not silently zero features. Pinned in Task 2.
4. **Historic values that the accepting rule does not accept** (a historic code missing from the enum). `taxonomy check` must reject the taxonomy. Pinned in Task 5.
5. **A one-distinct-value column of the fixed type** (one uppercase UUID ×40). It must keep its type through veto and Sharpen. Pinned in every Task 9–19.

## Rulings made while writing this plan

Hugh sees these when he reviews the plan. Each can be undone and leaves the design unchanged.

- **Batch 2 (the other 42 children) gets its own plan** once this branch lands, because its fixes depend on the structure built here. The spec stays in the repo until batch 2's branch folds it.
- **`file_size`:** B-suffixed units are decimal (SI: kB/KB = 1000, MB = 10^6…). `iB` units and bare GNU letters (K, M, G, T, P) are binary (1024). The transform changes accordingly, so `1 MB` casts to 1,000,000 bytes, not 1,048,576. This goes in the release notes.
- **Too-loose rules for real-world forms with no other home.** A form the source does not define stays accepted, with a `nonstandard` note, when it meets both conditions: it occurs in real data, and no other leaf accepts it. Examples are rfc_3339's `+0000` offset and iso_8601_offset's one-digit-hour offset. Otherwise a strict fix would turn real columns into `unknown`, against "best assessment of the type".
- **`currency_code` historic set:** ISO 4217 list-three codes withdrawn on or after 1999-01-01. That covers the euro changeover (DEM, FRF…) through HRK, BGN and ZWL.
- **`tsid` gets no Sharpen recovery in this work.** 13 Crockford base32 characters confirm too many alphanumeric IDs to recover on shape alone.
- **The training run writes the snapshot from `labels/` at train start.** The overnight scripts prepare features and train from one checkout, so the two cannot diverge within a run.

---

### Task 0: Baseline measurement

No code. This records the numbers every later task compares against.

- [ ] **Step 1:** On the branch base (before Task 1), build the release binary: `cargo build --release` (~10 min).
- [ ] **Step 2:** Run `scripts/eval_rule.sh --label baseline --out-dir output/validation-roles/baseline --default`. It runs gold, the corpus pass and the corpus-honest gate, and takes hours. Expected: it writes the gold report and the per-column predictions.
- [ ] **Step 3:** Score the representative band (`score_gold_anchor.py … --reframe` on `eval/repr/representative_corpus.tsv`) and the external band (`scripts/external_band.py`) into the same directory.
- [ ] **Step 4:** Record the three headlines in `output/validation-roles/baseline/README.md`, and commit them. `output/` is ignored if listed in `.gitignore`; check first, and if it is ignored, record the headlines in the Kata comment instead.

### Task 1: `ValidatorSnapshot` in core

**Files:**
- Create: `crates/finetype-core/src/validator_snapshot.rs`
- Modify: `crates/finetype-core/src/lib.rs` (add `pub mod validator_snapshot;`)
- Test: inline `#[cfg(test)]` in the new file

**Interfaces:**
- Produces:
  - `pub struct ValidatorSnapshot { pub format: u32, pub keys: Vec<String>, pub validations: BTreeMap<String, Validation> }` (Serialize/Deserialize; `format` = 1)
  - `pub const SNAPSHOT_FILE: &str = "validation_snapshot.json";`
  - `impl ValidatorSnapshot { pub fn from_taxonomy(tax: &Taxonomy, keys: &[String]) -> Self; pub fn from_json(bytes: &[u8]) -> Result<Self, ValidatorError>; pub fn to_json_pretty(&self) -> String; pub fn compile(&self) -> Result<HashMap<String, CompiledValidator>, ValidatorError>; pub fn keys(&self) -> impl Iterator<Item=&String> }`
  - `keys` holds every key passed to `from_taxonomy`, in order. A key whose type has no `validation` is present in `keys` but absent from `validations`. `from_json` refuses a wrong `format`, duplicate keys, and a `validations` label not in `keys`.

- [ ] **Step 1: Write the failing tests**
  - `snapshot_round_trips_and_compiles_like_live`: load `labels/`, build a snapshot over `tax.labels()`, then serialise and parse it back. For each of `uuid`, `currency_code` and `percentage`, `compile()[label].is_valid(v)` equals `tax.get_validator(label).unwrap().is_valid(v)` for the type's own `samples` and for `"not-a-value"`.
  - `snapshot_ignores_later_taxonomy_edits`: build the snapshot, then mutate the uuid pattern in a cloned taxonomy (via `Taxonomy::from_yaml` on an edited string). The snapshot still rejects `"550E8400-E29B-41D4-A716-446655440000"`.
- [ ] **Step 2:** Run `cargo test -p finetype-core validator_snapshot`. Expected: FAIL (module missing).
- [ ] **Step 3:** Implement. `compile` uses `CompiledValidator::new_for_label`.
- [ ] **Step 4:** Run the tests again. Expected: PASS.
- [ ] **Step 5:** Commit: `Add a serialisable snapshot of type validation rules`.

### Task 2: The model reads its snapshot

**Files:**
- Modify: `crates/finetype-model/src/validation_features.rs`
- Modify: `crates/finetype-model/src/multi_branch/mod.rs` (`load` near :109, `from_bytes` :141, extractor build :249, `resolve_validation_extractor` :597)
- Test: `crates/finetype-model/src/validation_features.rs` tests; `crates/finetype-model/src/multi_branch/tests.rs`

**Interfaces:**
- Consumes: `ValidatorSnapshot`, `SNAPSHOT_FILE` (Task 1)
- Produces:
  - `ValidationFeatureExtractor::with_snapshot(type_keys: Vec<String>, snapshot: &ValidatorSnapshot) -> Result<Self, InferenceError>`, which errors when `snapshot.keys()` ≠ `type_keys` as sets.
  - `extract(&self, values, taxonomy)` uses its pinned validators when present, and `taxonomy.get_validator` otherwise. When pinned, a key listed in the snapshot with no rule is a frozen "no rule" (feature 0.0), never a fall-back to the live taxonomy. The signature is unchanged.
  - `MultiBranchClassifier::from_bytes` gains a trailing `snapshot: Option<&[u8]>` parameter. `load` reads `dir.join(SNAPSHOT_FILE)` if it exists.

- [ ] **Step 1: Write the failing tests**
  - `pinned_extractor_ignores_live_rule_change`: a snapshot-backed extractor and a taxonomy whose uuid pattern was loosened, run on `["550E8400-E29B-41D4-A716-446655440000"; 5]`. The uuid feature is `0.0`.
  - `extractor_without_snapshot_matches_today`: the `from_type_keys` path gives byte-identical features to the current code on the existing test fixture.
  - `snapshot_key_mismatch_is_an_error`: `with_snapshot` with one key dropped from the snapshot returns `Err`.
- [ ] **Step 2:** Run `cargo test -p finetype-model validation_features`. Expected: FAIL.
- [ ] **Step 3:** Implement. Store `Option<HashMap<String, CompiledValidator>>` on the extractor. Update every `from_bytes` caller (CLI embedded loader, tests) to pass `None` for now.
- [ ] **Step 4:** Run `cargo test -p finetype-model`. Expected: PASS.
- [ ] **Step 5:** Commit: `Let the model read validation rules from its own snapshot`.

### Task 3: Ship the snapshot for the default model, and prove it changes nothing

**Files:**
- Create: `crates/finetype-build-tools/src/bin/write_validation_snapshot.rs` (`write_validation_snapshot <model_dir> [--labels labels]`, which writes `<model_dir>/validation_snapshot.json` over `config.type_index_keys`)
- Create: `models/m2v8m-s43/validation_snapshot.json` (generated, committed)
- Modify: `crates/finetype-cli/build.rs` (emit `MB_VALIDATION_SNAPSHOT: &[u8]` and `HAS_MB_VALIDATION_SNAPSHOT`, as with `MB_VALUE_*` at :350–384), and the CLI's embedded loader to pass it to `from_bytes`
- Modify: `crates/finetype-train/src/multi_branch.rs` (write the snapshot beside `config.json` at :2754, from `labels/`)
- Modify: `scripts/package.sh`, `.github/scripts/download-model.sh` (include the file when present)

**Interfaces:**
- Consumes: Tasks 1–2.

- [ ] **Step 1:** Run `cargo run -p finetype-build-tools --bin write_validation_snapshot -- models/m2v8m-s43`. Expected: the file is written, with 244 keys.
- [ ] **Step 2:** Add the test `default_model_ships_a_matching_snapshot` in `crates/finetype-model/src/multi_branch/tests.rs`. It asserts the snapshot's keys equal `config.type_index_keys`, and that the embedded and on-disk snapshots are byte-equal.
- [ ] **Step 3:** Build the release binary, then re-run Task 0's three measurements into `output/validation-roles/after-snapshot`. Expected: gold, repr and external predictions are byte-identical to the baseline (`diff` the predictions files).
- [ ] **Step 4:** Commit: `Freeze the default model's validation rules in a snapshot`.

### Task 4: The DuckDB extension uses the snapshot

**Files:**
- Modify: `crates/finetype-duckdb/src/lib.rs` (`MODEL_FILES` :69 plus the fetch loop :111; the snapshot is fetched as optional, so a 404 means no snapshot)
- Test: the extension's model-loading unit test in `crates/finetype-duckdb/src/lib.rs`

- [ ] **Step 1: Write the failing test** `extension_loads_snapshot_when_present`. A temp model dir with the snapshot gives a pinned extractor. Without it, the extractor is unpinned.
- [ ] **Step 2:** Run `cargo test -p finetype-duckdb`. Expected: FAIL.
- [ ] **Step 3:** Implement. A missing remote file must not fail the load.
- [ ] **Step 4:** Run the tests again. Expected: PASS.
- [ ] **Step 5:** Commit: `DuckDB extension: read the model's validation snapshot`.
- [ ] **Step 6 (Hugh's step, outward-facing):** Upload `validation_snapshot.json` to the Hugging Face model repo through the `/release model` flow. Then release the extension by hand. Record both in the PR description.

### Task 5: Note rules and the value grade

**Files:**
- Modify: `crates/finetype-core/src/taxonomy.rs` (`Definition` gains `validation_notes: Option<ValidationNotes>`; `validate()` at :339 checks it)
- Create: `crates/finetype-core/src/grade.rs`
- Test: inline tests in `grade.rs`; `crates/finetype-core/tests/` taxonomy-check test

**Interfaces:**
- Produces:
  - YAML: `validation_notes: { canonical: { kind: non_canonical, name: "lowercase", pattern: "…" }, historic: [ { value: "HRK", until: "2023-01" } ] }`. Both parts are optional. The published `validation` rule is always the full accepting set. `canonical` only classifies accepted values: an accepted value that fails `canonical.pattern` is `Noted` with `canonical.kind` (`non_canonical` for a form the source allows but does not prefer, `nonstandard` for a real-world form the source does not define).
  - `pub enum NoteKind { NonCanonical, Withdrawn, Nonstandard }` (serde: `non_canonical` | `withdrawn` | `nonstandard`)
  - `pub struct Note { pub kind: NoteKind, pub detail: String }` (detail: `"lowercase"`, `"withdrawn 2023-01"`…)
  - `pub enum Grade { Skipped, Conforms, Noted(Note), Fails { constraint: String, message: String } }`
  - `pub fn grade_value_for_label(value: &str, label: &str, tax: &Taxonomy) -> Grade`
  - `taxonomy check` enforces that every historic value and every type's own `samples` pass the published rule.

- [ ] **Step 1: Write the failing tests** (on a fixture taxonomy built with `Taxonomy::from_yaml`):
  - `grade_conforms`, `grade_noncanonical_upper_uuid` (→ `Noted{NonCanonical,"lowercase"}`), `grade_withdrawn_hrk` (→ `Noted{Withdrawn,"withdrawn 2023-01"}`), `grade_fails_keeps_constraint_token` (→ `Fails{constraint:"pattern",..}`), `grade_empty_and_null_are_skipped` (`""`, `"  "`, `"NULL"` → `Skipped`)
  - `check_rejects_historic_value_not_accepted`: a historic value absent from the enum makes `Taxonomy::validate()` return `Err`.
- [ ] **Step 2:** Run `cargo test -p finetype-core grade`. Expected: FAIL.
- [ ] **Step 3:** Implement. Checksum and membership failures surface as `Fails` with the tokens `checksum` and `membership`.
- [ ] **Step 4:** Run `cargo test -p finetype-core` and `cargo run -- check`. Expected: PASS, with taxonomy check clean.
- [ ] **Step 5:** Commit: `Grade values: conforms, conforms with a note, or fails`.

### Task 6: Notes in JSON Schema and `finetype validate`

**Files:**
- Modify: `crates/finetype-core/src/json_schema.rs` (emit `x-finetype-canonical` (`{kind, name, pattern}`) and `x-finetype-historic` (`[{value, until}]`) on a column property whose label has them; update the module doc's extension list)
- Modify: `crates/finetype-core/src/table_validator.rs` (`TableValidationResult` gains `pub column_notes: Vec<ColumnNotes>`; reject records and `valid_row_indices` are unchanged)
- Modify: `crates/finetype-cli/src/validate.rs` (print/emit `column_notes`)
- Test: `crates/finetype-cli/tests/cli_golden.rs`, `table_validator` unit tests

**Interfaces:**
- Consumes: Task 5.
- Produces: `pub struct ColumnNotes { pub column_name: String, pub counts: BTreeMap<NoteKind, usize>, pub examples: Vec<String> }` (at most 5 examples, in first-seen order).

- [ ] **Step 1: Write the failing tests**
  - `noted_values_are_not_rejects`: a 3-row uuid column (lowercase, uppercase, `bad`) gives `invalid_rows == 1`, one reject record, and `column_notes[0].counts[NonCanonical] == 1`.
  - Golden: `finetype validate` JSON output on that file contains `column_notes`, and the existing golden fields are unchanged.
- [ ] **Step 2:** Run `cargo test -p finetype-core table_validator` and `cargo test -p finetype-cli --test cli_golden`. Expected: FAIL.
- [ ] **Step 3:** Implement. `table_validator` reads the extensions from the schema property, so `validate` works on a stored schema without the taxonomy.
- [ ] **Step 4:** Run the tests again. Expected: PASS.
- [ ] **Step 5:** Commit: `Report noted values in validate and the JSON Schema, apart from rejects`.

### Task 7: `ft_validate_text` returns the grade

**Files:**
- Modify: `crates/finetype-duckdb/src/validate.rs` (`validate_value_detail` returns a struct with `valid`, `constraint`, `message`, `grade`, `note`)
- Modify: `crates/finetype-duckdb/src/lib.rs` (:413 STRUCT gains `grade VARCHAR` (`conforms`|`noted`|`fails`) and `note VARCHAR`, appended after the existing three fields)

- [ ] **Step 1: Write the failing tests** in `validate.rs`: an uppercase UUID against a fragment carrying `x-finetype-canonical` gives `valid=true, grade="noted", note="non_canonical: lowercase"`, and `bad` gives `valid=false, grade="fails"`. A null value gives `valid=true, grade=NULL`.
- [ ] **Step 2:** Run `cargo test -p finetype-duckdb validate`. Expected: FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Run the tests, then `make build-extension` and a SQL smoke `SELECT ft_validate_text('550E8400-E29B-41D4-A716-446655440000', '<fragment>')`. Expected: PASS, and the struct shows `noted`.
- [ ] **Step 5:** Commit: `ft_validate_text: add the value's grade and note`.

### Task 8: Per-type discrimination detectors

**Files:**
- Create: `crates/finetype-core/src/discrimination.rs`
- Modify: `crates/finetype-core/src/validation_veto.rs` (`evaluate_validation_veto`), `crates/finetype-model/src/column/value_sharpen.rs` (`schema_fail_demotion` :647), `crates/finetype-model/src/column/mod.rs` (`label_validates_sample` :461, used by `ceded_leaf_recovery`)
- Create: `crates/finetype-core/tests/random_confirm_rate.rs`

**Interfaces:**
- Produces:
  - `pub fn detector_for(label: &str) -> Option<fn(&str) -> bool>`, a match over a static table that starts empty (Task 18 adds npi).
  - `pub fn value_passes_for_discrimination(value: &str, label: &str, tax: &Taxonomy) -> bool`: the detector if one exists, else the published rule, resolved the way each call site resolves it today (the veto and `schema_fail_demotion` through their current validator lookup; `label_validates_sample` through `resolve_label_validator`, which compiles on a cache miss). Behaviour without a detector is unchanged at all three sites.
  - `random_confirm_rate.rs`: a test over two seeded populations (`rand_chacha`, seed 42): 10,000 printable-ASCII strings of lengths 1–40, and 10,000 digit-only strings of lengths 1–20. It writes `tests/fixtures/random_confirm_rate.tsv` (`label, published_ascii, discrimination_ascii, published_digits, discrimination_digits`). Labels with no validation are omitted. The gate is `discrimination_ascii > 0.5`, on the ASCII population only, since numeric types accept digit strings by design. Labels already above 0.5 when the test lands are listed in the test as baseline exceptions, with their measured rate; the test fails for any other label above 0.5, or for a listed label whose rate rises.

- [ ] **Step 1: Write the failing tests**
  - `no_detector_means_published_rule`: for uuid, the result equals the compiled rule on its samples.
  - `random_confirm_rate` as above.
- [ ] **Step 2:** Run `cargo test -p finetype-core discrimination` and `--test random_confirm_rate`. Expected: FAIL.
- [ ] **Step 3:** Implement and re-point the three call sites.
- [ ] **Step 4:** Run `cargo test --workspace`. Expected: PASS, with behaviour unchanged since there are no detectors yet. Commit the generated TSV as the baseline.
- [ ] **Step 5:** Commit: `Let a type discriminate by a detector stricter than its published rule`.

### Tasks 9–19: the 13 rule fixes

Every fix task follows the same steps. Only its **Rule** block differs.

**Files (each):** the type's entry in `labels/definitions_<domain>.yaml`; ACCEPT/REJECT cases appended to `crates/finetype-core/tests/precision_widenings.rs` or `precision_tightenings.rs`; a one-value test appended to `crates/finetype-model/src/column/tests/validation_gate.rs`; the generator arm in `crates/finetype-core/src/generator/` when samples change; `tests/fixtures/precise_audit.tsv` and `random_confirm_rate.tsv` regenerated.

- [ ] **Step 1: Write the failing tests:**
  - `<type>_accepts_source_forms` (ACCEPT list) and `<type>_rejects_undefined_forms` (REJECT list), through `validate_value_for_label`;
  - grade assertions for any note;
  - `<type>_constant_column_keeps_type`: 40 copies of the first ACCEPT value. `evaluate_validation_veto(label, …, &audited_safe_labels())` is not vetoed, and `schema_fail_demotion` returns `None`.
- [ ] **Step 2:** Run `cargo test -p finetype-core --test precision_widenings --test precision_tightenings` and `cargo test -p finetype-model validation_gate`. Expected: FAIL.
- [ ] **Step 3:** Edit the YAML rule (and any transform, samples or description the Rule block names).
- [ ] **Step 4:** Run the tests again, then `cargo run -- check` and `cargo test -p finetype-core --test random_confirm_rate`. Expected: PASS. No label's discrimination rate exceeds 0.5. If this fix's published rate rose above 0.5, the type needs a detector in this task.
- [ ] **Step 5:** Commit: `<type>: agree with <source>`, then close its tracker child with the commit as evidence.

**Task 9: uuid** (RFC 9562). This is the reproduction case.
Rule: hex digits are case-insensitive (`[0-9a-fA-F]`). `validation_notes.canonical`: `{kind: non_canonical, name: lowercase, pattern: <today's lowercase pattern>}`. Description: cite RFC 9562, which obsoletes RFC 4122, and mention versions 6–8.
ACCEPT: `550e8400-e29b-41d4-a716-446655440000`, `550E8400-E29B-41D4-A716-446655440000`, `550e8400-E29B-41d4-a716-446655440000`. REJECT: `550e8400e29b41d4a716446655440000`, `550e8400-e29b-41d4-a716-44665544000g`.
Extra: an end-to-end `profile` golden test on 40 uppercase UUIDs expects `representation.identifier.uuid`.

**Task 10: percentage** (SI Brochure; French, German and Turkish typography).
Rule: the sign may be preceded by U+0020, U+00A0 or U+202F, or may lead the number (`%50`). Keep the sign optional, as today. Write the two non-ASCII spaces as YAML ` ` / ` ` escapes so the literal characters reach the published pattern.
ACCEPT: `50%`, `50 %`, `50\u{00A0}%`, `50\u{202F}%`, `%50`, `-12.5%`, `12.5`. REJECT: `50%%`, `%50%`, `abc%`, `50 % %`.
Transform: extract the number in all ACCEPT forms.

**Task 11: file_size** (NIST/IEC; GNU coreutils).
Rule: units `B, kB, KB, MB, GB, TB, PB, KiB, MiB, GiB, TiB, PiB`, the bare letters `K, M, G, T, P`, and the GNU `iB` forms. Space optional.
Transform (per the ruling): B-suffixed units are decimal; `iB` units and bare letters are binary; a TB/TiB/T branch is added; and the `%B%` catch-all is removed, because it cast `2 TB` to 2 bytes.
ACCEPT: `512`, `1.5 kB`, `2 TB`, `4KiB`, `10M`, `3 GiB`. REJECT: `1.5 XB`, `MB`, `5 Mb`.
Test that the transform gives `2 TB` → 2e12 and `4KiB` → 4096 (the DuckDB SQL is checked through the existing transform test harness, `crates/finetype-cli/tests/` transform tests).

**Task 12: boolean.initials** (Frictionless Table Schema).
Rule: unchanged. The datapackage output for this type declares `trueValues: ["T","t","Y","y"]` and `falseValues: ["F","f","N","n"]` (match the type's enum).
Test: a `datapackage_conformance.rs` case asserts both lists.

**Task 13: locale_code** (RFC 5646).
Rule: `^[a-zA-Z]{2,3}(?:[-_][a-zA-Z]{4})?(?:[-_](?:[a-zA-Z]{2}|[0-9]{3}))?$`.
ACCEPT: `en`, `en-US`, `es-419`, `zh-Hant-TW`, `pt_BR`. REJECT: `english`, `en-1234`, `e`.
Check the existing structural locale guard still agrees (`crates/finetype-model/src/column/guards.rs`, search `locale_code`).

**Task 14: abbreviated_month** (CLDR en, en-GB, fr; US military format).
Rule: the universal pattern accepts an optional full stop after the month, `Sept`, and a two-digit year in the day-first form (`4 Feb 23`). French goes in `validation_by_locale: fr`, listing CLDR's abbreviations (`janv.`, `févr.`, `mars`, `avr.`, `mai`, `juin`, `juil.`, `août`, `sept.`, `oct.`, `nov.`, `déc.`).
Transform: add `'%d %b %y'` and `'%b. %d, %Y'`, and rewrite `Sept` to `Sep` before parsing. French values stay un-castable by the English-only `strptime`, which the type's `notes` say.
ACCEPT (universal): `Feb 4, 2023`, `4 Feb 2023`, `4 Feb 23`, `Sept 4, 2023`, `Feb. 4, 2023`. ACCEPT (fr): `4 janv. 2023`, `12 déc. 2023`. REJECT: `4 February 23x`, `Febr 4 2023`.

**Task 15: rfc_3339 and iso_8601_offset** (RFC 3339; ISO 8601). One task, because the two rules overlap through `datetime_format_refinement`.
- rfc_3339: published `^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}(?:\.\d+)? ?(?:[Zz]|[+-]\d{2}:?\d{2})$`. `canonical`: `{kind: nonstandard, name: "RFC 3339 offset", pattern: "^\\d{4}-\\d{2}-\\d{2} \\d{2}:\\d{2}:\\d{2}(?:\\.\\d+)?(?:[Zz]|[+-]\\d{2}:\\d{2})$"}`. The separator stays a space, since T-forms belong to other leaves. ACCEPT: `2024-01-01 10:00:00Z`, `2024-01-01 10:00:00.123+05:30`, `2024-01-01 10:00:00z`. Noted: `2024-01-01 10:00:00 +0000`. REJECT: `2024-01-01 10:00:00+5:30`.
- iso_8601_offset: published `^\d{4}-\d{2}-\d{2}[Tt]\d{2}:\d{2}:\d{2}[+-]\d{1,2}(?::?\d{2})?$`, with the length bounds dropped. `canonical`: `{kind: nonstandard, name: "two-digit offset hour", pattern: <the same with \d{2} for the hour>}`. ACCEPT: `2024-01-01T10:00:00+05:30`, `2024-01-01T10:00:00+0530`, `2024-01-01T10:00:00+05`, `2024-01-01t10:00:00+05:30`. Noted: `2024-01-01T10:00:00+5:30`.
- Extra: run `crates/finetype-model/src/column/tests/datetime.rs` and check `datetime_format_refinement` routes each ACCEPT value to the same leaf as before. Transforms must parse every ACCEPT value (`%z` accepts `+0530` and `+05:30`; normalise `z`/`t` case first).

**Task 16: currency_code** (SIX ISO 4217 lists one and three).
Rule: enum = list one ∪ historic set. `validation_notes.historic` = list-three codes withdrawn on or after 1999-01-01, with `until` = withdrawal month. Add a small `scripts/fetch_iso4217.py` that regenerates both from the two SIX XML files, run by hand (hermetic build).
ACCEPT: `USD`, `EUR`, `XCG` (if list one has it). Noted: `HRK` (`withdrawn 2023-01`), `BGN` (`withdrawn 2026-01`), `ZWL` (`withdrawn 2024-09`), `DEM`. REJECT: `ABC`, `US`.
currency_code is a ceded leaf, so confirm `ceded_leaf_recovery` still promotes a list-one column.

**Task 17: username and blood_type** (useradd(8); Rh nomenclature).
- username: the disputes are the trailing `$` and the 256-character maximum, both from useradd(8). The character set and the 3-character minimum stay as today, which covers web-service names such as GitHub's (useradd's own lowercase-only default is stricter than the names analysts meet). Rule: `^[a-zA-Z0-9_.\-]{2,255}[a-zA-Z0-9_.\-$]$`, which allows `$` only as the last character, with `minLength` 3 and `maxLength` 256. Description cites useradd(8) for `$` and length. ACCEPT: `alice`, `bob.smith`, `svc$`, a 39-char GitHub name, a 256-char name. REJECT: `ab`, `al ice`, `$alice`, `a$b`, a 257-char name.
- blood_type: in YAML, `"^(A|B|AB|O)[+\\-−]?$"` (the YAML `−` escape yields the literal minus sign, as elsewhere in the taxonomy). ACCEPT: `A+`, `O-`, `AB−` (U+2212). REJECT: `C+`, `A++`. Test that the emitted JSON Schema pattern contains the literal U+2212.
- Extra: check username's published rate in `random_confirm_rate.tsv`. It was already high. If it now exceeds 0.5, stop and report it as a finding rather than adding a detector here.

**Task 18: npi** (CMS NPI standard). This is the first detector.
Rule: published pattern `^\d{10}$`, checksum `npi` unchanged. `validation_notes.canonical`: `{kind: non_canonical, name: "issued range (first digit 1 or 2)", pattern: "^[12]\\d{9}$"}`.
Detector `npi`: `^[12]\d{9}$` and Luhn with the 80840 prefix (`finetype_core::checksum`). Register it in `discrimination::detector_for`.
ACCEPT: `1234567893` (CMS's worked example), and a Luhn-valid number starting with 3, computed in the test with `finetype_core::checksum` (→ noted). REJECT: `123456789`, `1234567890` (bad check digit).
Test: `npi_detector_keeps_attractor_demotion`. A column of 10-digit Luhn-valid numbers starting with 3 is still demoted by `schema_fail_demotion`.

**Task 19: tsid** (tsid-creator README).
Rule: `^[0-9A-Fa-f][0-9A-HJKMNP-TV-Za-hjkmnp-tv-z]{12}$` (13 Crockford base32 characters; the first is ≤ F because a TSID is 64 bits). Rewrite the description, the samples (from a generator arm producing real TSIDs: 42-bit ms since 2020-01-01, 10 node bits, 12 counter bits) and `notes` (also storable as a 64-bit integer, which finetype types as an integer). No recovery, per the ruling.
ACCEPT: three values from the new generator arm, and their lowercase forms. REJECT: `0190a1b2c3d4e5f6a7b8c9d0e1f2a3b4`, `GAWE5HZP3SKTK`, `0AWE5HZP3SKTU` (U is not in Crockford's set).
Test: a 32-hex column labelled `tsid` by the model is vetoed (tsid is on the allowlist) and falls back to `alphanumeric_id`.

### Task 20: Batch-1 gates, gold accounting and hand-off

No new code.

- [ ] **Step 1:** Build the release binary and run Task 0's three measurements plus the corpus-honest gate into `output/validation-roles/batch1`. This takes hours.
- [ ] **Step 2:** Write `output/validation-roles/batch1/accounting.md`:
  - each gold column that moved, with its type and the source that explains it;
  - the cumulative gold delta, which must be ≤ 2 points or the run stops for Hugh;
  - repr and external deltas;
  - the veto allowlist re-measured: re-run `scripts/false_veto_sweep.py`, regenerate `labels/veto_safe.txt` (commit it), and list every label that joins or leaves it;
  - each gate transition on a fixed type, with its adjudication;
  - gold labels proposed for re-label.
- [ ] **Step 3:** Run the panel on the proposed re-labels. Flag the tracker issue `needs-human` for Hugh to ratify them and to rule on any gate NO-GO.
- [ ] **Step 4:** Fold the structure into `docs/ARCHITECTURE.md`: the snapshot, the grades, detectors, and the four roles. Add a `CHANGELOG.md` entry that lists which types now accept or reject which forms (from the Rule blocks), and the file_size decimal change. Delete this plan, and keep the spec for batch 2.
- [ ] **Step 5:** Create the batch-2 tracker item, plus one for the downstream tool to take the release. Run `roborev review --branch --wait`, open the PR, wait for `gh pr checks <pr> --watch`, link the PR in Forge, and report it ready with the test totals and the rulings above.
