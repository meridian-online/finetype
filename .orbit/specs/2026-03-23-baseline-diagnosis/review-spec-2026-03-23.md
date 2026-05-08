# Spec Review

**Date:** 2026-03-23
**Reviewer:** Context-separated agent (fresh session)
**Spec:** .orbit/specs/2026-03-23-baseline-diagnosis/spec.yaml
**Verdict:** REQUEST_CHANGES

---

## Findings

### [CRITICAL] BD-1: Fresh eval command may not work as specified
**Category:** assumption
**Description:** The methodology specifies `./scripts/eval.sh --model models/char-cnn-v14-250` but that directory may not exist as a standalone path — it may only be the symlink target of `models/default`. The BD-1 verification ("profile_results.json exists with 190 entries") will pass trivially because the file already exists from the prior run.
**Recommendation:** Verify `models/char-cnn-v14-250` exists, or change to `./scripts/eval.sh` (default model). Change BD-1 verification to check timestamp freshness, or acknowledge the existing file satisfies BD-1.

### [CRITICAL] BD-2: DuckDB query won't produce exactly 42 rows naively
**Category:** assumption
**Description:** A naive `WHERE predicted != expected` filter won't produce 42 rows due to multi-row gt_label entries in schema_mapping.csv, equivalence rules in eval_profile.sql, and match_quality scoping. The spec never defines the exact query.
**Recommendation:** Verify BD-2 by reading `misclassifications` from the existing `profile_results.json` directly rather than reconstructing from a CSV join. Or define the exact DuckDB query.

### [CRITICAL] BD-3: Missing bucket — `schema_mapping_error`
**Category:** missing-requirement
**Description:** ~19/42 failures are `"decimal number"` columns where schema_mapping maps the gt_label to `integer_number` instead of `decimal_number`. The pipeline predicts correctly; the mapping is wrong. This is distinct from `bad_gt` (where the data genuinely isn't the claimed type) and requires a different fix (change schema_mapping, not the manifest). The spec's five-bucket taxonomy doesn't capture this dominant failure mode.
**Recommendation:** Add `schema_mapping_error` as a sixth bucket: "The finetype_label in schema_mapping.csv is wrong — the GT category is correct but mapped to the wrong canonical type."

### [WARN] BD-4: Multi-row gt_label scoring may miscount improvements
**Category:** failure-mode
**Description:** `schema_mapping.csv` has multiple rows for some gt_labels (e.g., six rows for "name"). The eval SQL's best-match deduplication may pick the wrong candidate, causing correct predictions to score as failures. At least `titanic.Name` → `full_name` appears to be a true positive scored as a failure because the "name" gt_label's best match resolved to `geography.location.state`.
**Recommendation:** Before triaging "name" cluster failures, verify the eval SQL's multi-candidate deduplication logic. Some apparent failures may be eval scoring artefacts.

### [WARN] BD-5: Effort estimates for sense_routing and generator_gap will be speculative
**Category:** test-gap
**Description:** Assigning `sense_routing` and `generator_gap` buckets requires internal pipeline visibility (Sense category routing, per-value vote distributions) that isn't in the eval output. The spec says "inspect sample values" but this only supports heuristic assignment.
**Recommendation:** Note that `sense_routing`/`generator_gap` require `finetype profile --verbose` to assign confidently. Scope BD-5 effort estimates to exact numbers for `bad_gt`/`schema_mapping_error` and labelled bounds for other buckets.

### [WARN] BD-6: generator_gap action conflicts with "no retrain" constraint
**Category:** constraint-conflict
**Description:** The `generator_gap` bucket's action is "fix generator YAML or add synthetic examples" — but this is meaningless without a subsequent training run, which the constraint forbids.
**Recommendation:** Clarify that generator fixes are staged in this PR for a future training PR. Make this explicit in the bucket definition.

### [INFO] Missing bucket: eval_harness_bug
**Category:** missing-requirement
**Description:** Some failures (e.g., multi-row "name" scoring) may be bugs in the eval harness itself, not pipeline or GT issues. Without this bucket, they'll be miscategorised.
**Recommendation:** Add `eval_harness_bug` as a seventh bucket option.

### [INFO] BD-1 and BD-2 are already satisfied
**Category:** assumption
**Description:** The JSON with all 42 failures already exists from the prior run. Steps 1–2 of the methodology describe work that's already done. Actual work begins at step 3 (manual triage).
**Recommendation:** Acknowledge existing data satisfies BD-1/BD-2 or re-run for freshness confirmation.

---

## Honest Assessment

The spec is well-structured but has three friction points. First, the dominant failure cluster (~19/42) is schema_mapping errors where the pipeline is correct but the GT mapping is wrong — this needs its own bucket. Second, multi-row gt_label scoring in the eval SQL may be producing false failures (particularly for "name" columns). Third, `sense_routing` and `generator_gap` bucket assignments will be speculative without debug instrumentation. The changes are straightforward: add two buckets, acknowledge the eval SQL multi-match issue, and scope effort estimates appropriately.
