# Spec Review

**Date:** 2026-03-18
**Reviewer:** Context-separated agent (fresh session)
**Spec:** specs/2026-03-18-distillation-v2-fixes/spec.yaml
**Verdict:** REQUEST_CHANGES

---

## Findings

### [CRITICAL] Taxonomy YAML migration is architecturally underspecified
**Category:** missing-requirement
**Description:** The spec says to move hints into `labels/definitions_*.yaml` with "contextual constraints," but (a) the `Definition` struct has no `header_hints` field, (b) the semantic (model2vec) classifier reads from `models/model2vec/label_index.json` and pre-computed embeddings — it has no connection to YAML fields, and (c) the hardcoded `header_hint()` function reads from the Rust source, not YAML. There is no existing mechanism by which a new YAML field would change what the hint system does.

Critically: fix-3 ("points"), fix-6 ("created"), and fix-9 ("yield/pct/rate") may come from the **model2vec semantic classifier**, not the hardcoded `header_hint()` table — there may be no entries for these in the Rust match arms. Migrating them to YAML requires either (a) a new post-hint value-pattern guard reading YAML fields, or (b) re-training or adjusting the model2vec threshold per type. These are very different scopes.

**Evidence:** `crates/finetype-core/src/taxonomy.rs` shows the `Definition` struct — no `header_hints` field. `crates/finetype-model/src/semantic.rs` shows the semantic classifier reads only from safetensors + JSON artifacts. The interview records this as an open question.
**Recommendation:** Before writing any PR-1 code, design and record the taxonomy YAML schema, the Rust parser changes in `finetype-core`, and the wiring into `column.rs`. Determine whether the target hints come from `header_hint()` or model2vec — this fundamentally changes the approach.

### [CRITICAL] Offline re-score script doesn't exist and join methodology is undefined
**Category:** assumption
**Description:** AC-1 and AC-2 depend on a re-score script that doesn't exist. The join key is `(source_file, column_name)`, but `source_file` in `merged_labels.csv` contains bare filenames while `finetype profile` uses whatever path was passed. ~55 rows may map to missing CSV files.
**Evidence:** `output/distillation-v2/merged_labels.csv` uses bare filenames. No re-score script exists in `scripts/`. The interview flags this as an open question.
**Recommendation:** Design the re-scoring script before PR-1 is merged, since PR-1 AC-1 requires running it. Define: the exact command, output format, join key, and handling of rows with missing source files.

### [WARN] Fix-1 (boolean binary) is misplaced in PR-1
**Category:** assumption
**Description:** Fix-1 is placed in PR-1 ("Header hint fixes") but has no connection to taxonomy YAML migration. The implementation lives entirely in `disambiguate_boolean_override()` in `column.rs` — the same file as all PR-2 fixes.
**Evidence:** `column.rs` shows `disambiguate_boolean_override` and `disambiguate_boolean_subtype` — no taxonomy YAML involvement. The interview explicitly flags this placement as an open question.
**Recommendation:** Move fix-1 to PR-2. It belongs with column heuristic fixes.

### [WARN] Fix-7 (sequential ID) already partially exists
**Category:** assumption
**Description:** `disambiguate_numeric()` already has sequential increment detection, but `amount_minor_int` is not in its trigger list. The fix needs to extend the trigger list, not build from scratch.
**Evidence:** `column.rs` shows `disambiguate_numeric` handles `[increment, integer_number, decimal_number, postal_code, year]` only. `amount_minor_int` validation `^-?[0-9]+$` matches any integer.
**Recommendation:** Clarify that fix-7 extends `disambiguate_numeric` to trigger when `amount_minor_int` is the top vote and sequential properties hold.

### [WARN] Fix-2 (categorical vs ordinal) impact may be overstated
**Category:** assumption
**Description:** `ordinal` is a first-class CharCNN output label, not just a fallback. `disambiguate_categorical()` only fires when `top_is_generic` — it does not override strong CharCNN ordinal votes. The 123-case estimate includes all ordinal→categorical disagreements, not just fallback-driven ones.
**Evidence:** `column.rs` shows `disambiguate_categorical` only fires when `top_is_generic`. `label_category_map.rs` shows `representation.discrete.ordinal` is a first-class CharCNN label.
**Recommendation:** Sample ~20 of the 123 cases from `merged_labels.csv` and check whether ordinal comes from CharCNN votes or from fallback disambiguation. This determines whether the fix is a disambiguation change or requires model retraining.

### [WARN] No minimum delta thresholds on AC-1 and AC-2
**Category:** test-gap
**Description:** "Agreement increases" could be satisfied by +1. If PR-1's measured delta is +30 instead of +434, that's a signal the approach isn't working, but it technically passes the AC.
**Evidence:** Estimated impact ~880 total, but these are estimates. No minimum stated.
**Recommendation:** Add minimum thresholds (e.g., "AC-1: agreement improves by at least 200").

### [INFO] No rollback plan for unjustifiable regressions
**Category:** missing-requirement
**Description:** If a PR regresses profile eval on a column not in the distillation corpus, D-0037 doesn't apply and the path forward is undefined.
**Recommendation:** Add: "If a PR group regresses profile eval on a column not in the distillation corpus, the regression is reverted or the fix scoped to exclude that case."

### [INFO] No-feature-flags constraint acknowledged at pre-1.0 stage
**Category:** constraint-conflict
**Description:** "No feature flags" combined with D-0037 means regressions are permanent. Acceptable at pre-1.0, but should be explicitly acknowledged.
**Recommendation:** State explicitly in spec that this is acceptable given the project stage.

---

## Honest Assessment

The plan is directionally correct and well-evidenced from real data, but it has a load-bearing hole in the center: the "taxonomy YAML migration" approach for PR-1 is a design intention, not a design. The three highest-impact fixes in PR-1 (fix-3, fix-6, fix-9) may route through the model2vec semantic classifier, not the hardcoded `header_hint()` Rust table — meaning there may be no direct path from adding YAML fields to changing runtime behavior without new Rust plumbing. That plumbing design needs to happen before implementation begins. The offline re-score methodology also doesn't exist as a script yet. Starting PR-1 without those two things designed means implementing against an unclear target. Everything else (fix-1 placement, the ordinal question, missing thresholds) is fixable in a day. The YAML migration architecture and the rescore script are the blockers.
