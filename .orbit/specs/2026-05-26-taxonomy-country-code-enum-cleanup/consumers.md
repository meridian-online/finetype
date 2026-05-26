# Consumers of `geography.location.country_code.validation.enum`

Per spec `2026-05-26-taxonomy-country-code-enum-cleanup` ac-01.

## Method

`grep -rn` across `crates/`, `scripts/` for: `enum_values`, `enum_set`,
`validation.enum`, `country_code`, `to_json_schema`. Then inspect
each hit and trace the dispatch path.

## Headline finding

**The enum is already canonical.** `labels/definitions_geography.yaml`
lines 75–323 contain exactly 249 alphabetically-sorted ISO 3166-1
alpha-2 codes (AD through ZW). Zero duplicates. Zero US state codes.
Zero CA province codes.

The memory `taxonomy-country-code-enum-contamination` (written
2026-05-26 during v23 development) described contamination in this
enum that does not exist. The state/province codes the memory
observed live in `geography.location.state_code.validation_by_locale`
(yaml lines 358–441), which is the correct location for them
(EN_US: 54 codes, EN_CA: 13 codes, EN_AU: 8 codes).

That collapses several of the spec's downstream ACs into verification
work rather than cleanup work. See the AC notes at the end.

## Consumer map

### 1. `crates/finetype-core/src/taxonomy.rs` — load + emit

- `Validation::enum_values: Option<Vec<String>>` (line 65) deserialises
  the yaml `enum:` block.
- `Validation::to_json_schema()` (line 76–102) emits the enum into a
  JSON Schema object alongside `pattern`/`minLength`/`maxLength`.
  **Both** pattern AND enum land in the schema — they apply jointly.
- `Validation::is_precise()` (line 120–138) returns true when an
  enum is present with ≥1 entry. Used by demotion guards.

Expected post-cleanup delta: **none** (yaml already canonical).

### 2. `crates/finetype-core/src/validator.rs` — compile + match

- The `CompiledValidator` consumes a `Validation` and applies pattern
  AND enum (both must pass). See test `ac01_country_code_enum_validates_official_codes`
  (line 1252) which loads the real taxonomy and asserts:
  - `US`, `GB`, `JP`, `DE`, `ZW` pass.
  - `QQ`, `XX`, `ZZ` (user-assigned ranges) fail enum membership.
  - `A1`, `usa`, `USA` fail pattern.

Expected delta: **none**. The existing test is the regression guard
the spec's ac-06 was going to ask for — see ac-06 notes.

### 3. `crates/finetype-core/src/infer.rs` — enum-aware cascade

- `validator_is_enum()` (line 392) checks whether a label's validation
  has a non-empty `enum_values`. Wired into the Rule 3 trigger in
  the inference cascade. Pure boolean — the cleaned enum changes the
  values it dispatches on, not whether it dispatches.

Expected delta: **none** for country_code (still enum-bearing).

### 4. `crates/finetype-core/src/generator.rs` — sample synthesis

- `gen_geography("location", "country_code")` (line 2279) returns
  values from a hardcoded 20-entry array: `US, GB, CA, AU, DE, FR,
  JP, CN, IN, BR, MX, IT, ES, KR, RU, NL, CH, SE, NO, DK`. All are
  canonical alpha-2; outputs are subset of the enum.
- `gen_geography("transportation", "unlocode")` (line 2533) prefixes
  unlocodes with a 20-entry country list (also canonical).

Expected delta: **none**. The generator does not read the yaml enum;
it has its own list. Worth a follow-up if we want a single source of
truth (file as `taxonomy-country-code-followups` memory if needed).

### 5. `crates/finetype-mcp/src/tools/schema.rs` — JSON Schema export

- Calls `def.validation.to_json_schema()` and emits the schema as a
  user-facing artefact through the `schema` MCP tool (line 167–169).
- The cleaned enum **already** flows through this path. The 249-entry
  schema is what the MCP tool currently emits.

Expected delta: **none** (enum already clean).

### 6. `crates/finetype-mcp/src/json_schema.rs` — profile schema

- `attach_stats()` (line 141) emits a `enum` keyword **derived from
  observed data**, gated by `enum_threshold` (default 50 unique
  values). Independent mechanism from the taxonomy enum.

Expected delta: **none** (this enum source is unrelated).

### 7. `crates/finetype-duckdb/src/validate.rs` — `finetype_validate()`

- A scalar UDF that takes a user-provided JSON Schema string and
  validates a value against it. The schema string is opaque to
  DuckDB — whatever schema callers pass in (typically generated via
  MCP `schema` or written by hand) is what is enforced.

Expected delta: **none**. Downstream users who paste in the cleaned
schema get strict validation; those with stale schemas continue to
use them until they re-export.

### 8. `crates/finetype-eval/src/bin/validate_corpus.rs` — eval

- `CODE_TYPED_LABELS` (line 337) is an allowlist of "code-shaped
  types" used for eval-time rejection rules. Includes
  `geography.location.country_code` as a *label*, not as a value
  set. Doesn't consume the enum.

Expected delta: **none**.

### 9. `scripts/apply_ydf_validation_gate.py` — YDF validation gate

- `ENUM_SKIP_LABELS = frozenset(["geography.location.country_code"])`
  (line 72) — workaround that forces the gate to use the alpha-2
  pattern instead of the enum for country_code. The comment claims
  the yaml enum is contaminated; **that claim is false**, the enum
  is canonical.
- `_compile_spec()` (line 142–163) selects ONE validation kind per
  label in priority order: `pattern > enum > locale > range`. Since
  country_code has both a pattern AND an enum, the pattern wins
  today — independent of `ENUM_SKIP_LABELS`. The skip is dead code.

**Important consumer behaviour delta.** The gate today validates
country_code with the alpha-2 pattern `^[A-Z]{2}$`, which accepts
non-country alpha-2 shapes (`UT`, `OK`, `OR`, `OH`, `IA`, `NY`,
`TX`, `WA`, `WV`, `WI`, `WY`...). The taxonomy's
`to_json_schema()` emits BOTH the pattern AND the enum — they apply
jointly in `jsonschema` validators. The gate's single-kind
short-circuit diverges from the rest of the codebase.

Fixing this for ac-05 requires two changes — see ac-05 notes.

## AC-by-AC implication

| AC | Original plan | Implication of finding |
|---|---|---|
| ac-01 | Catalogue consumers | This document. |
| ac-02 | Broad audit of all enums | Still useful — run as drafted; reports overall enum health. |
| ac-03 | Clean the country_code enum to ~249 ISO codes | **Already done** in commit `08c0d2b`. Verify, do not re-clean. |
| ac-04 | Regenerate goldens that depend on the enum | **No yaml change** so nothing to regenerate. `cargo test --workspace` still runs as a smoke. |
| ac-05 | Remove `ENUM_SKIP_LABELS`, re-run gate, capture delta | Two changes needed: (i) remove the skip frozenset (it's dead code), (ii) fix `_compile_spec` to apply pattern AND enum jointly when both exist, matching the rest of the codebase. THEN re-run the gate to capture the cell-2 delta. |
| ac-06 | Regression test asserting enum rejects state codes | Existing `ac01_country_code_enum_validates_official_codes` already covers the membership semantics. Extend with explicitly-named state codes that are also non-country (e.g. `AK`, `NY`, `TX`, `FL`, `OK`, `OH`, `OR`, `WI`) — **not** with codes that ARE countries (AL=Albania, CA=Canada, etc.). The spec's listed assertions for `AL`, `CA` would FAIL because those are valid ISO 3166-1 alpha-2 codes. |
| ac-07 | Triage ac-02 findings | Depends on ac-02 output. |

## Memory follow-up

`taxonomy-country-code-enum-contamination` needs correction or
deletion. The contamination it described does not exist in the
committed yaml; the state codes it observed are in
`geography.location.state_code.validation_by_locale.EN_US.enum`,
which is the correct location for US state codes.
