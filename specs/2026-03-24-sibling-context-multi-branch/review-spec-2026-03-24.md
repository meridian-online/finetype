# Spec Review

**Date:** 2026-03-24
**Reviewer:** Context-separated agent (fresh session)
**Spec:** specs/2026-03-24-sibling-context-multi-branch/spec.yaml
**Verdict:** REQUEST_CHANGES

---

## Findings

### [CRITICAL] Training loop modification unspecified
**Category:** missing-requirement
**Description:** AC-3 requires "multi-branch training loads sibling-context model, runs frozen attention" but the current training loop has no concept of a sibling-context model. No code path exists to load SiblingContextAttention, run it over header inputs, and pass enriched embeddings into the header branch MLP. The current training loop reads flat FTMB v2 records with pre-computed header embeddings. The spec does not describe the required code changes, how frozen weights are loaded alongside VarMap, or how batching changes with table-level grouping.
**Evidence:** `crates/finetype-train/src/multi_branch.rs:459` shows `FTMB_VERSION: u32 = 2` — no v3 reader. Training loop at ~line 700 uses flat `TrainingRecord` structs. The FrozenSense pattern from the previous sibling-context training spike (constant tensors, not Var-backed) is the proven mechanism but is not referenced.
**Recommendation:** Add explicit AC describing training binary changes: how sibling-context model is loaded (frozen via constant tensors), how enriched header embedding is computed per batch, and training loop signature. Reference the FrozenSense pattern.

### [CRITICAL] FTMB v3 binary format undefined
**Category:** missing-requirement
**Description:** The spec says "records grouped by source table with sibling header list" but gives no byte layout — how sibling header count is stored, whether headers are strings or pre-computed embeddings, whether records within a group are contiguous, whether there's an index block. This directly affects whether v1/v2 backward compatibility (AC-1) is achievable.
**Evidence:** v2 header is 24 bytes with no reserved space for table-group count or sibling list length. A table-grouped format requires fundamentally different file layout. The Python data prep script hardcodes `VERSION = 2` with no table grouping structure.
**Recommendation:** Define FTMB v3 layout in the spec: binary structure (magic, version, index block, group header, record layout), sibling header storage format, and backward-compatibility strategy.

### [CRITICAL] Frozen attention trained for Sense (6-class), not fine-grained (250-class)
**Category:** assumption
**Description:** The sibling-context attention model was trained to improve Sense classification (6 broad categories). Multi-branch doesn't use Sense — its header branch operates in a 250-class regime. No evidence that attention optimised for 6-class generalises to 250-class. AC-7 (ablation) should catch this, but there's no fallback plan if siblings prove neutral or harmful.
**Evidence:** Previous sibling-context evaluation reports 78% val accuracy against Sense labels. Multi-branch header branch learns 250 fine-grained classes. Transfer across this semantic gap is untested.
**Recommendation:** Add fallback plan: what happens if AC-7 shows sibling context provides no signal? Ship format machinery anyway, or investigate further?

### [WARN] Single-value inference degradation path unclear
**Category:** failure-mode
**Description:** AC-4 removes the multi-branch bypass in `classify_columns_with_context()`. AC-5 says single-value inference degrades gracefully. But `finetype infer -i "value"` goes through `classify_column_with_header`, not `classify_columns_with_context`. The N=1 attention behavior is tested with random weights, not trained weights.
**Evidence:** `column.rs:822` bypass. MCP `infer` tool and CLI `finetype infer` both use `classify_column_with_header`. The single-column attention test uses random weights.
**Recommendation:** Add explicit test case: `finetype infer -i "alice@example.com"` produces valid label, no panic, no NaN. Clarify whether single-value uses N=1 attention pass or bypasses entirely.

### [WARN] Ablation baseline ambiguous
**Category:** test-gap
**Description:** AC-7 says "with-siblings vs without-siblings on same training data" but doesn't specify the baseline: re-train on v3 data with attention disabled, or use existing sherlock-v3-flat trained on v2 data? These produce different comparisons. If both use v3 data, the comparison is cleaner but requires two training runs. If baseline is existing v2-trained model, format differences confound the comparison.
**Evidence:** sherlock-v3-flat already trained on v2 data (298,800 records, 98.31% val accuracy). The spec doesn't clarify which baseline.
**Recommendation:** Specify ablation baseline explicitly: "Re-train on FTMB v3 data with attention disabled" or "Use existing sherlock-v3-flat."

### [WARN] Synthetic table assembly strategy undefined
**Category:** assumption
**Description:** "5-15 related types grouped by domain knowledge" is not a specification. No definition of "related", no domain knowledge source, no grouping algorithm. Incoherent synthetic tables may teach attention to ignore sibling signal.
**Evidence:** `prepare_multibranch_data.py` has no table grouping logic — generates individual columns per type. The HEADER_VARIATIONS dict covers 65 types but with no co-occurrence information.
**Recommendation:** Define concrete grouping strategy: grouped by domain, by co-occurrence in Sherlock corpus, or by curated table templates.

### [WARN] Multi-branch + regex hints interaction
**Category:** constraint-conflict
**Description:** When multi-branch is active, regex header hints are inert (early return at line 868 bypasses the hint path). This means post-processed eval scores in AC-6 do NOT include regex hint contributions. The spec should state this explicitly to avoid confusion.
**Recommendation:** Clarify that multi-branch active = regex hints inert in the post-processing pipeline.

### [INFO] Memory constraints for training
**Category:** missing-requirement
**Description:** Hierarchical model training was OOM killed (Signal 9) on M1 in the previous run. Adding frozen SiblingContextAttention (396K params) increases memory pressure.
**Evidence:** `results/overnight-v3.log` shows hierarchical model killed. Flat model completed. Adding attention tensors to training increases peak memory.
**Recommendation:** Add memory constraint: flat model training must complete without OOM on M1 16GB. Consider flat-only for this sprint.

---

## Honest Assessment

The spec is architecturally coherent but critically underspecified in three areas: (1) FTMB v3 binary format is named but not defined, (2) the training loop modification is the hardest engineering task but has no implementation detail, and (3) the assumption that attention trained for 6-class Sense generalises to 250-class multi-branch is untested. The ablation will catch problem (3) but without a fallback plan. The synthetic table assembly is also undefined but tractable once a strategy is chosen. With these gaps filled — binary layout, training loop changes, ablation baseline, fallback plan — this is a feasible sprint.
