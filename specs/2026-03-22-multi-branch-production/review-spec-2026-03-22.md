# Spec Review

**Date:** 2026-03-22
**Reviewer:** Context-separated agent (fresh session)
**Spec:** specs/2026-03-22-multi-branch-production/spec.yaml
**Verdict:** REQUEST_CHANGES

---

## Findings

### [CRITICAL] M-1 gate is impossible without unspecified code changes
**Category:** failure-mode
**Description:** M-1's gate says "produces type predictions for all columns" but `ModelType` enum has no `MultiBranch` variant. Running `eval.sh --model models/sherlock-v1-flat` will hard-fail at model load time (CharCNN loader rejects `.safetensors` format). The ablation confirmed this exact failure. The spec knows about the gap but does not enumerate the Rust changes required.
**Evidence:** `main.rs` line 487–491: `enum ModelType { Transformer, CharCnn, Tiered }`. Ablation results: "all 31 datasets returned errors."
**Recommendation:** M-1 must explicitly enumerate: (a) add `MultiBranch` variant to `ModelType`, (b) implement `MultiBranchColumnClassifier` wrapper, (c) implement model loading from `.safetensors` + `config.json`, (d) wire into `cmd_profile`'s match arm.

### [CRITICAL] ValueClassifier trait boundary incompatible with column-level model
**Category:** assumption
**Description:** `cmd_profile` feeds values through `ColumnClassifier::with_semantic_hint(classifier, ...)` where `classifier: Box<dyn ValueClassifier>`. `ValueClassifier` takes a single string → label. Multi-branch takes a `Vec<String>` column + feature extraction. The spec says "replaces the Sense stage" but doesn't address how the trait boundary changes.
**Evidence:** `main.rs` line 3433: `ColumnClassifier::with_semantic_hint(classifier, config, semantic)`. Multi-branch does not implement `ValueClassifier`.
**Recommendation:** M-1 must specify the integration approach: (a) shim `ValueClassifier` that buffers and defers, (b) new `ColumnClassifier` constructor bypassing `ValueClassifier`, or (c) extract a `ColumnLevelClassifier` trait. This is a design decision, not implementation detail.

### [WARN] Data pipeline subprocess bottleneck at 300k scale
**Category:** test-gap
**Description:** `prepare_multibranch_data.py` calls `finetype extract-features` as a subprocess per column. At 33k columns: ~5 minutes. At 300k columns: ~30+ minutes for subprocess overhead alone. M-5 has no time budget.
**Evidence:** `prepare_multibranch_data.py` lines 235–241, 461–505: per-column subprocess calls. Ablation: "AC-5: FTMB data prep: ~5min" for 33,536 columns.
**Recommendation:** Add time budget to M-5. Consider Rust-native batch feature extraction.

### [WARN] Synthetic column volume feasibility unverified
**Category:** assumption
**Description:** Blend-30-70 at 300k needs ~840 synthetic columns per type (84,000 values per type). Current default generates 1,500 values per type (15 columns). 56× scale-up untested.
**Evidence:** `prepare_multibranch_data.py` lines 169–178. Ablation: 3,720 synthetic columns from default settings.
**Recommendation:** M-4 gate should verify generation at M-5 target volume, not just the 100-column-per-type minimum.

### [WARN] Tier 1 eval set doesn't cover all 250 types
**Category:** assumption
**Description:** The ≥95% gate is measured on 190 format-detectable columns across 30 datasets — a subset of the 250-type taxonomy. Multi-branch could hit 95% by excelling on represented types while failing on unrepresented ones.
**Evidence:** Ablation: 190 format-detectable columns. Taxonomy: 250 types.
**Recommendation:** Either expand eval set coverage or document that Tier 2 regression analysis covers the long-tail types.

### [WARN] No post-deployment rollback procedure for M-8
**Category:** missing-requirement
**Description:** Rollback section covers pre-deployment (below 90% → keep CharCNN). No rollback for partial M-8 failures (multi-branch wired up, golden tests pass, field bug surfaces).
**Recommendation:** Add M-8 rollback: repoint `models/default` symlink to CharCNN. Specify whether `--model-type` flag persists for per-invocation model switching.

### [WARN] Python dependency in training pipeline vs "pure Rust" constraint
**Category:** constraint-conflict
**Description:** Constraint says "zero Python dependencies at build or inference time" but the training pipeline (`prepare_multibranch_data.py`) is Python. Technically compliant (training ≠ build/inference) but ambiguous.
**Recommendation:** Clarify constraint as "zero Python at inference time (training pipeline may use Python)."

### [INFO] Training time estimate at 300k samples
**Category:** test-gap
**Description:** At 28.5k: flat took 169s for 10 epochs. At 300k (10×): ~28 min/epoch. With ≥20 epochs + patience 10, worst case 30 epochs = ~87 min (flat), ~87 min (hierarchical). Within 2-hour budget but marginal.
**Recommendation:** State the back-of-envelope estimate explicitly in M-6 so the constraint can be monitored.

---

## Honest Assessment

The spec is well-structured with clear milestone sequencing. Eval before retraining is correct. The known risks are honest.

The two CRITICAL findings describe fundamental interface mismatches that will block M-1 before any training work happens. `ModelType` has no `MultiBranch` variant, and the `ValueClassifier` trait boundary is structurally incompatible with a column-level model. The spec describes the outcome but not the design choice needed to get there. That design decision should be settled in the spec, not left to the implementer to discover.

The biggest risk after fixing the blockers is M-6 training time at 300k samples. The math works (marginal) but should be verified. If the data pipeline proves slow to generate and training runs hit the 2-hour wall, the milestone-driven plan could stall at the most critical gate. A lightweight dry-run estimate after M-5, before committing to M-6 hyperparameters, would mitigate this.
