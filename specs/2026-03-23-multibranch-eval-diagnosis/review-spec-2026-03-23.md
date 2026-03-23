# Spec Review

**Date:** 2026-03-23
**Reviewer:** Context-separated agent (fresh session)
**Spec:** specs/2026-03-23-multibranch-eval-diagnosis/spec.yaml
**Verdict:** REQUEST_CHANGES

---

## Findings

### [CRITICAL] D-1 misdiagnoses root cause of hier eval 0/0
**Category:** assumption
**Description:** The spec says "Either the detection fails, the model loading fails, or the inference path for hierarchical multi-branch is missing." The actual root cause is definitive: `MultiBranchClassifier::load()` at line 128 has `if config.head_type != HeadType::Flat { return Err(...) }` — it's a hard rejection of non-flat heads, not a detection issue.
**Evidence:** `crates/finetype-model/src/multi_branch.rs:128` — explicit guard that rejects Hierarchical head type with error "Only flat head is supported for multi-branch inference (hierarchical not yet implemented)".
**Recommendation:** Update D-1 description to state the known root cause: hierarchical inference path is unimplemented. The fix requires: (1) loading hierarchy_map from label_map.json at runtime, (2) implementing hierarchical forward pass (tree softmax: domain→category→type), (3) mapping leaf predictions back to flat label space. This is not a detection fix — it's new inference code.

### [CRITICAL] Shared staging directory may overwrite flat results
**Category:** failure-mode
**Description:** If eval.sh uses a shared eval_output/ staging directory, running eval for both flat and hier models sequentially will overwrite the flat model's results with the hier model's.
**Evidence:** Spec says "Both flat and hier eval results are preserved in their respective model directories" but doesn't specify the isolation mechanism.
**Recommendation:** AC-1 should specify that eval output lands in `models/<model-name>/eval/` directories, not a shared staging path. Eval script needs `--output-dir` or model-directory-based output.

### [WARN] "37-point gap" framing is imprecise
**Category:** assumption
**Description:** The spec frames the gap as "94% training → 57% profile eval" (37pp). But the meaningful comparison is against the production CharCNN pipeline which scores 97.7% (170/174) on the same profile eval. The actual gap to close is 41pp (97.7% → 56.8%). Additionally, the 94% is validation accuracy on a different data distribution (synthetic+distilled), so it's a distribution mismatch, not a generalization gap.
**Evidence:** CLAUDE.md states profile eval is 97.7% (170/174) for the current Sense→Sharpen pipeline.
**Recommendation:** Reframe D-2/D-4 to compare against production baseline (97.7%), not training val accuracy. Clarify that this is distribution shift, not overfitting.

### [WARN] Val accuracy comparison is misleading
**Category:** assumption
**Description:** Comparing 94% val accuracy (synthetic+distilled data) with 57% profile eval (real-world CSVs) conflates two different distributions. The val set is a held-out split of the same synthetic data, so high val accuracy is expected and doesn't predict real-world performance.
**Evidence:** Training data is from `prepare_multibranch_data.py` (synthetic generators + distilled labels). Profile eval uses real-world datasets from `eval/datasets/manifest.csv`.
**Recommendation:** D-4 findings should explicitly note this is distribution mismatch. The model learned synthetic patterns, not real-world data patterns.

### [WARN] DuckDB CLI dependency needs preflight check
**Category:** missing-requirement
**Description:** D-3 introduces a hard dependency on `duckdb` CLI being installed and on PATH. The overnight script should fail early if it's missing, not silently produce empty results.
**Evidence:** Spec says "Overnight script reads them with duckdb -json" but doesn't require a preflight check.
**Recommendation:** Add preflight check: `command -v duckdb >/dev/null 2>&1 || { echo "duckdb CLI required"; exit 1; }` at top of overnight script.

### [WARN] AC-1 should require correctness, not just non-zero
**Category:** test-gap
**Description:** AC-1 gate says "produces non-zero label and domain accuracy numbers." A model that classifies everything as "text" would produce non-zero numbers. The gate should verify the results are plausible.
**Evidence:** Spec line: "eval.sh --model models/sherlock-v2-hier produces non-zero label and domain accuracy numbers."
**Recommendation:** Tighten AC-1: "produces label and domain accuracy ≥ 10% (sanity floor) and results are preserved in model-specific eval directory."

### [INFO] Sense is bypassed for multi-branch — D-2 trace framing needs clarification
**Category:** assumption
**Description:** The multi-branch model bypasses Sense classification entirely (it's a direct column→type classifier). The D-2 trace asks "What Sense category would have been assigned" — this is asking what WOULD happen if the prediction were fed through the existing pipeline's post-processing, not what DID happen.
**Evidence:** Multi-branch operates at column level with its own feature extraction, not through the Sense→Sharpen pipeline.
**Recommendation:** Clarify in D-2 that the trace is a counterfactual analysis: "If this prediction were processed through Sense→Sharpen post-processing, would the error be caught?" This is the correct framing for deciding whether pipeline integration closes the gap.

### [INFO] models/default symlink risk during eval
**Category:** failure-mode
**Description:** If the system has a `models/default` symlink, running eval could inadvertently use the wrong model if the eval harness falls back to default model loading.
**Evidence:** Standard FineType convention uses `models/default` symlink for the active model.
**Recommendation:** Eval scripts should always require explicit `--model` path, never fall back to default.

---

## Honest Assessment

The spec correctly identifies the four workstreams needed (fix hier eval, trace misclassifications, structured eval output, findings document). The interview was thorough and the constraints are sensible. However, D-1 underestimates the work required — implementing hierarchical inference is writing new code, not fixing a detection bug. The "37-point gap" framing also needs sharpening: the meaningful baseline is production (97.7%), not training val accuracy, and the gap is distribution mismatch not generalization failure. These aren't blockers to the work — they're corrections that will lead to better-targeted implementation and more accurate findings.
