# Spec Review

**Date:** 2026-04-15
**Reviewer:** Context-separated agent (fresh session)
**Spec:** .orbit/specs/2026-04-15-validation-branch-v12/spec.yaml
**Verdict:** REQUEST_CHANGES

---

## Findings

### [HIGH] ac-02 claims to add range constraints that already exist in the taxonomy
**Category:** assumption
**Description:** ac-02 says "Add numeric range constraints to latitude (minimum: -90, maximum: 90) and longitude (minimum: -180, maximum: 180) in definitions_geography.yaml." However, `definitions_geography.yaml` already contains these exact constraints at lines 791-793 (latitude) and 823-825 (longitude). Furthermore, the `CompiledValidator` already handles `minimum`/`maximum` via manual string-to-f64 parsing (validator.rs lines 100-116). The AC as written would be a no-op.
**Evidence:** `labels/definitions_geography.yaml` lines 790-793: `validation: { type: number, minimum: -90, maximum: 90 }`. `crates/finetype-core/src/validator.rs` lines 95-116: `CompiledValidator::is_valid()` already parses to f64 and checks bounds.
**Recommendation:** Rewrite ac-02 to reflect reality. The actual work is ensuring `extract_validation_features()` correctly invokes `CompiledValidator::is_valid()` which already handles numeric range checks. If the intent was to add *pattern* validation (e.g., a regex for decimal number format) alongside the range, say so explicitly. Otherwise, delete ac-02 or merge its verification into ac-04.

---

### [HIGH] ac-04's verification assumes behaviour that already works in CompiledValidator
**Category:** test-gap
**Description:** ac-04 says "Extend validation feature extraction to support numeric range checks" and its verification tests ["40.7", "-33.8", "91.5"] against latitude yielding pass_rate = 2/3. But `CompiledValidator::is_valid()` already handles this. The real work in ac-04 is ensuring `extract_validation_features()` calls `CompiledValidator::is_valid()` (which already handles numeric bounds) rather than only the JSON Schema validator (which uses `"type": "string"` and would silently pass on numeric range). The AC doesn't distinguish these two paths clearly, which could lead an implementer to duplicate the range-checking logic.
**Evidence:** `taxonomy.rs` `to_json_schema()` always emits `"type": "string"` regardless of the definition's `schema_type`. Numeric bounds are deliberately excluded from the JSON Schema and handled manually in `CompiledValidator`. If `extract_validation_features()` uses `CompiledValidator::is_valid()` (the obvious choice), ac-04 is satisfied with zero additional code.
**Recommendation:** Clarify: is the intent to use `CompiledValidator::is_valid()` for validation feature extraction (in which case range checking is already covered), or to use raw JSON Schema validation (in which case range extension is genuinely needed)? The answer affects implementation scope significantly.

---

### [HIGH] 239-dim vector is coupled to taxonomy size at training time
**Category:** failure-mode
**Description:** The validation branch uses a 239-dim vector (one element per taxonomy type) stored in FTMB at training time and recomputed at inference time. The type ordering is defined by `BTreeMap<String, usize>` built from taxonomy keys at startup. If the taxonomy size changes between training and inference (types added or removed), the dimension mismatch will cause a tensor shape error during the forward pass. The spec states "No taxonomy types are added or removed" as a constraint, but this is only a constraint for *this* spec — it doesn't address what happens when types *are* added in future versions.
**Evidence:** Constraint: "No taxonomy types are added or removed — only validation fields updated." But CLAUDE.md describes an active taxonomy at 239 types. Any future taxonomy expansion (the card's scenario 1 mentions 7 domains, CLAUDE.md says "accommodate new data types") would break v12 models.
**Recommendation:** Add a forward-compatibility story. Options: (a) store the type-to-index mapping in the model directory (like `label_map.json`) so the inference code can map current taxonomy keys to training-time indices, padding with zeros for new types; (b) store `valid_dim` + `type_keys` in config.json; (c) document explicitly that taxonomy changes require retraining (which is probably fine, but should be stated). This is not blocking for v12 but needs a documented answer.

---

### [HIGH] 215/227 target may be unrealistic for what validation features can discriminate
**Category:** assumption
**Description:** The spec targets 215/227 (14 more correct out of 26 misclassifications). But examining the actual misclassification table, many are NOT same-category sibling confusions that validation can help with. For example: `compact_ym` vs `year` (both numeric strings, both would pass similar validators), `iso` vs `mdy_dash`/`dmy_dash`/`iso_week` (date format ambiguity where values match multiple date patterns), `alphanumeric_id` vs `isbn`/`geohash`/`order_id` (generic alphanumeric confusion), `integer_number` vs `postal_code`/`status_code`/`port` (all are integers), `plain_text` vs `user_agent`/`scientific_notation`. These pairs will have nearly identical validation pass rates because the values genuinely pass both validators.
**Evidence:** From the eval report: only 6-7 of 26 misclassifications are cases where validation patterns differ meaningfully (country/country_code, email/email_display, phone/phone_e164, url/data_uri, jwt/user_agent, decimal_number/latitude). The remaining 19-20 involve types where values legitimately pass multiple validators (date formats, numeric types, alphanumeric IDs).
**Recommendation:** Set a more conservative primary target (e.g., 208-210/227) with 215 as stretch. The exit conditions partially address this with the "plateaus below 215 — ship if >= 210" clause, but the headline target shapes expectations. Consider explicitly listing which misclassifications the validation branch is expected to fix vs. which remain for future work.

---

### [MEDIUM] Performance budget for inference is underspecified
**Category:** missing-requirement
**Description:** ac-03 specifies "100 values x 239 types completes in <50ms" but doesn't specify the environment. On a cold start with 239 schema compilations, this could be much slower. The spec mentions `CompiledValidator` but doesn't address when compilation happens. At inference time, the `Taxonomy` struct's `compile_validators()` method pre-compiles all 239 validators. If validation features are computed per-column, the 239 compiled validators need to be held in memory.
**Evidence:** The DuckDB extension processes ~2048-row chunks. If validation features are computed per chunk (100 sampled values x 239 validators = 23,900 validations), and each profile processes multiple columns, the cumulative cost could be significant. The spec doesn't address caching or amortization strategy.
**Recommendation:** Specify: (a) are validators compiled once at model load time and reused across all columns? (b) What's the memory impact of holding 239 `CompiledValidator` instances? (c) What's the per-column latency budget, not just per-extraction? The existing `SCHEMA_CACHE` mechanism in the DuckDB extension suggests this has been solved before — reference it.

---

### [MEDIUM] FTMB v4 header layout is unspecified
**Category:** missing-requirement
**Description:** ac-05 says "Header extends v3 with valid_dim (u16) after reserved bytes" but the interview lists this as an open question ("Exact FTMB v4 header layout — how many bytes for valid_dim and range features"). The v3 header is 28 bytes (4 magic + 4 version + 8 n_records + 2+2+2+2 dims + 2 n_groups + 2 reserved). The spec says valid_dim goes "after reserved bytes" — does it replace the reserved bytes or extend the header? This ambiguity could cause byte-alignment issues.
**Evidence:** `FTMB_HEADER_SIZE_V3 = 28` in `finetype-train/src/multi_branch.rs`. The current v3 header has 2 reserved bytes at offset 26-27. If valid_dim replaces the reserved bytes, header stays 28 bytes but there's no room for the table group count. If it extends, the header grows to 30+ bytes.
**Recommendation:** Pin the exact byte layout in the spec. Suggested: keep v3's 28-byte layout and add valid_dim (u16) at offset 28, making v4 header 30 bytes. Include the full byte map.

---

### [MEDIUM] No mention of DuckDB extension impact
**Category:** missing-requirement
**Description:** The DuckDB extension downloads models at runtime via `hf_hub`. The spec adds validation features that require access to the full taxonomy at inference time (to compile 239 validators and compute pass rates). Currently the DuckDB extension loads only the model weights. It's unclear whether the DuckDB extension has access to taxonomy definitions for computing validation features.
**Evidence:** `crates/finetype-duckdb/src/lib.rs` loads the multi-branch model. The taxonomy is loaded from `labels/` directory or embedded. If validation features require taxonomy definitions at runtime, the DuckDB extension needs them bundled or downloaded alongside the model.
**Recommendation:** Explicitly address: will taxonomy YAML definitions be bundled into the model directory (e.g., `models/sherlock-v12/compiled_validators.bin`) or will the DuckDB extension need a new data dependency? This is a distribution concern that should be resolved before implementation.

---

### [MEDIUM] ac-09 references prepare_multibranch_data.py — Python dependency contradiction
**Category:** constraint-conflict
**Description:** ac-09 says "Feature extraction pipeline (extract-features CLI command or prepare_multibranch_data.py) computes validation features." CLAUDE.md states "Zero Python dependencies (build + runtime)." If validation feature extraction requires changes to the Python script, this contradicts the zero-Python principle. The `or` phrasing is ambiguous — is this a choice or are both paths required?
**Evidence:** `prepare_multibranch_data.py` is a Python script that shells out to `finetype extract-features`. It writes FTMB files. If v4 format requires validation features, either the Python script needs updating (Python in the build pipeline) or `finetype extract-features` needs to compute validation features and output FTMB v4 natively.
**Recommendation:** Clarify: does `finetype extract-features` gain the ability to compute validation features and write FTMB v4 directly? If so, the Python script just needs to pass a `--format v4` flag. If the Python script must compute validation features itself, that's a bigger change. Pick one path and make it explicit.

---

### [MEDIUM] Backward compatibility test for v4 reader on v3 is necessary but insufficient
**Category:** test-gap
**Description:** ac-06 tests reading v3 files with the v4 reader, defaulting validation features to zeros. But it doesn't test: (a) the v3 reader encountering a v4 file (should it fail gracefully with a clear version error, or silently ignore validation features?); (b) v1/v2 files through the v4 reader; (c) the training loop's behaviour when mixing v3 and v4 FTMB files in the same training run.
**Evidence:** Current `read_training_header()` bails on unknown versions: `if version != 1 && version != 2 && version != 3 { bail!(...) }`. A v4 file encountered by old code will give "Unsupported FTMB version: 4" — this is fine but not tested or documented.
**Recommendation:** Add test cases: (a) v3 reader (old code) rejects v4 with a clear error; (b) v4 reader handles v1 and v2 files correctly (not just v3); (c) document that FTMB v4 files are forward-incompatible with v3 readers (expected, but state it).

---

### [MEDIUM] No rollback plan if v12 training fails or regresses
**Category:** missing-requirement
**Description:** The exit conditions cover three outcomes (ship v12, ship if >= 210, abandon). But there's no rollback plan. What happens if v12 training completes but introduces regressions on previously-correct columns? What if the validation branch improves the 6 targeted pairs but regresses 8 others? The spec doesn't define "regression" or set a bound on acceptable accuracy loss on any individual type.
**Evidence:** Exit condition: "Validation branch shows no improvement over v11 — abandon branch." But "no improvement" is vague. Net improvement with per-type regressions could still be a net negative for users.
**Recommendation:** Add a regression constraint: "No type that was correctly classified by v11 may regress to incorrect in v12." Or at minimum, define acceptable regression tolerance (e.g., "net +10 with at most 2 regressions").

---

### [LOW] Spec says stats_dim is 36 via CLAUDE.md but code shows 27
**Category:** assumption
**Description:** CLAUDE.md says "36-dim deterministic feature extractor" while the code defines `COLUMN_STATS_DIM = 27`. The spec's merge layer math uses `[300+200+64+64+64] = [692]` which doesn't match either. The current 4-branch merge is `[300+200+64+64] = 628` (from the model architecture comments). Adding a 5th branch with output 64 would make it 692, matching the spec.
**Evidence:** `crates/finetype-model/src/column_stats.rs` line 17: `pub const COLUMN_STATS_DIM: usize = 27`. The "36-dim" in CLAUDE.md appears to refer to the combined ColumnFeatures (mean, variance, min, max of 9 dimensions = 36), which is the Sharpen-layer feature set, not the model input.
**Recommendation:** No spec change needed — the merge math is correct (adds 64 for validation branch output). But the CLAUDE.md discrepancy is worth noting to avoid confusion during implementation.

---

### [LOW] Missing consideration: validation branch input normalization
**Category:** missing-requirement
**Description:** ac-07 describes the validation branch architecture as `validation(239) -> Dense(128, Act) -> Dense(64, Act) -> [64]` but doesn't specify whether the input gets LayerNorm. The header branch uses `new_with_input_norm()` because "raw embeddings need stabilisation." Validation pass rates are bounded [0, 1] so they're already well-conditioned, but the spec should be explicit about this choice.
**Evidence:** The header branch always gets input LayerNorm. The char/embed/stats branches get it only when `use_layer_norm=true`. Validation pass rates are naturally in [0, 1] with clear semantics, so LayerNorm may not help — but the spec should state the design choice.
**Recommendation:** Add a note: "Validation branch does NOT use input LayerNorm (pass rates are already [0, 1])." Or if it should use it for consistency, say so.

---

### [LOW] country_code enum: no automated source or update mechanism
**Category:** missing-requirement
**Description:** ac-01 adds ~249 ISO 3166-1 alpha-2 codes as an enum. The spec doesn't address how this list stays current. ISO 3166-1 changes periodically (e.g., CS removed 2006, SS added 2011). A stale enum could cause false negatives.
**Evidence:** ac-01 verification tests "XX" and "ZZ" as invalid, but these are explicitly assigned (XX for no nationality, ZZ for unknown). Some edge cases in the ISO standard are ambiguous.
**Recommendation:** Document the source of the enum list and add a note about update frequency. This is low severity because the list changes rarely, but should be acknowledged.

---

## Honest Assessment

This is a well-motivated spec that targets a real limitation in the current pipeline, and the overall approach is sound — feeding validation pass rates as model features is a strong architectural choice that leverages existing infrastructure. However, the spec has two significant problems. First, ac-02 and ac-04 describe work that is already done in the codebase (latitude/longitude range constraints exist, CompiledValidator already handles them), which suggests the spec was written without reading the current taxonomy definitions and validator code — this is concerning because it means other assumptions may also be stale. Second, the 215/227 headline target is optimistic given that only 6-7 of 26 misclassifications involve types with meaningfully different validation signatures; the exit conditions wisely include a fallback, but the primary target should be calibrated against which specific misclassifications the validation branch can plausibly fix. The FTMB v4 format, DuckDB distribution story, and taxonomy-size coupling need more concrete answers before implementation begins — these are the kind of details that become expensive to fix mid-implementation. I recommend one revision pass to reconcile ACs with existing code, sharpen the target analysis, and pin the FTMB byte layout.
