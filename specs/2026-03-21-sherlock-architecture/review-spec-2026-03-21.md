# Spec Review

**Date:** 2026-03-21
**Reviewer:** Context-separated agent (fresh session, sonnet)
**Spec:** specs/2026-03-21-sherlock-architecture/spec.yaml
**Verdict:** REQUEST_CHANGES

---

## Findings

### [CRITICAL] Training data format transition is unspecified
**Category:** missing-requirement
**Description:** The existing distilled data stores raw column values as JSON in `sample_values`. The new pipeline needs pre-computed 960+512+36 dim feature vectors. AC-5's verification is "dry-run produces correct sample counts" — this is the single largest implementation task and has almost no specification detail. No on-disk schema, no scale test, no wall-clock time estimate.
**Evidence:** Current `finetype-train` `ColumnSample` struct stores per-value embeddings, not per-column feature aggregations. The new pipeline needs a fundamentally different data format.
**Recommendation:** Define the on-disk format for feature-vector training records. Add an AC that validates feature prep at full scale (119K rows) with measured wall-clock time.

### [CRITICAL] Model2Vec per-value embedding latency is unverified
**Category:** assumption
**Description:** AC-2 proposes embedding 100 values per column via Model2Vec (128d) at inference time. The existing codebase only embeds single header strings (sub-ms) or uses pre-computed embeddings. There is no precedent for 100-value embedding at inference. The 50ms/column latency budget (AC-10) must cover all three branches.
**Evidence:** Production target is Linux x86 CPU, not M1. No latency measurement exists for Model2Vec per-value embedding at 100 values.
**Recommendation:** Benchmark Model2Vec embedding for 100 values on x86 CPU before committing. If >20ms, use a smaller sample (10–20 values) for the embedding branch.

### [CRITICAL] Sibling-context attention is unresolved
**Category:** missing-requirement
**Description:** The 2-layer pre-norm transformer attention (shipped, contributing to 170/174 profile eval) is neither preserved nor dropped in the spec. The interview flags this as an open question but the spec doesn't resolve it. Profile eval floor (AC-8: 170/174) may depend on it.
**Evidence:** `classify_columns_with_context()` is the current multi-column entry point. The spec says "direct 250-type classification" with "no Sense→category routing" but doesn't address cross-column context.
**Recommendation:** Explicitly decide: preserve sibling-context before the new model, drop it (accepting risk to 170/174), or defer to follow-up. The spec cannot leave this ambiguous.

### [CRITICAL] 960-dim char feature extraction needs precise specification
**Category:** failure-mode
**Description:** The char distribution features compute 10 aggregations per 96 ASCII chars across sampled column values. This is architecturally distinct from the current per-value 36-dim extractor. AC-1's verification (assert dimensions + spot-check 5 values) is insufficient to catch errors in kurtosis/skewness. The spec doesn't clarify whether stats operate on per-value character frequency or binary presence.
**Evidence:** Current `features.rs` takes a single `&str` → `[f32; 36]`. Sherlock computes per-character, per-column aggregations — different computation axis entirely.
**Recommendation:** Add pseudocode or reference implementation. Define whether the 10 stats operate on per-value char frequency (fraction) or binary presence. Add edge case tests: all-identical values, empty values, non-ASCII.

### [WARN] 85% Tier 2 target has no empirical basis
**Category:** assumption
**Description:** The spike proved data blending helps CharCNN. The spec assumes Sherlock-style features add further gains, but Spike A was never executed. No evidence that 960-dim char features improve accuracy on FineType's 250-type taxonomy.
**Evidence:** Architecture review: "Spike A: never executed." Sherlock achieved 78% F1 on 78 types with known train-test overlap (inflated scores). FineType's problem is harder (250 types, fine-grained datetime variants).
**Recommendation:** Define a minimum acceptable outcome that triggers a pivot. Is 82% a success? Is 80.6% (same as CharCNN+blend) a failure requiring rollback?

### [WARN] Spike baseline models are ephemeral — regression check is unverifiable
**Category:** test-gap
**Description:** AC-7 compares per-type results against the spike baseline, but those models were "ephemeral (not persisted)." The only baseline is numbers in findings.md — not a reproducible artifact.
**Evidence:** Spike findings: "The spike models were ephemeral (not persisted after the experiment loop)."
**Recommendation:** Retrain blend-30-70 CharCNN as the official baseline artifact before starting implementation (~2 hours). This also fixes the "Tier 1 regression not captured" gap from the spike.

### [WARN] DuckDB extension compatibility unaddressed
**Category:** missing-requirement
**Description:** The DuckDB extension uses CharCNN directly for column classification. Replacing CharCNN with a multi-branch model that requires per-column feature extraction changes the DuckDB extension's code path entirely.
**Evidence:** CLAUDE.md: "DuckDB extension: uses flat CharCNN with chunk-aware column classification (~2048-row chunks)."
**Recommendation:** Explicitly defer DuckDB to follow-up sprint, or add an AC for DuckDB extension compatibility.

### [WARN] No rollback plan
**Category:** missing-requirement
**Description:** If the new model regresses on Tier 1 or fails to meet 85% Tier 2, there's no defined path back to production. The exit condition's "findings document" path doesn't address what ships.
**Recommendation:** Add rollback criterion: "If best multi-branch model scores <80% Tier 2 OR <170/174 Tier 1, production model remains char-cnn-v14-250."

### [WARN] Distilled data label noise (72% disagreement rows)
**Category:** failure-mode
**Description:** The `unix_seconds` regression (-10) from the spike was attributed to conflicting epoch labels in disagreement rows. The spec trains on the same noisy data without auditing it.
**Evidence:** Spike findings: "likely fixable — suggests conflicting labels between seconds/milliseconds/microseconds."
**Recommendation:** Either filter known-problematic types' disagreement rows, or acknowledge the regression may persist and is out of scope.

### [INFO] Stats branch passthrough may be numerically dominated
**Category:** assumption
**Description:** The stats branch (36-dim, passthrough) concatenated with two branch outputs of ~256-512 dims each will contribute little signal. Sherlock gave each branch independent hidden layers.
**Recommendation:** Give the stats branch at least one hidden layer (36→64 or 36→128).

### [INFO] AC-4 model compile check is weak
**Category:** test-gap
**Description:** "Train for 1 epoch on 1000 samples without error" won't catch zero-gradient branches or incorrect dimension handling.
**Recommendation:** Add gradient flow check: verify nonzero gradients for all branch inputs after one backward pass, or feature ablation test (zeroing a branch changes output).

---

## Honest Assessment

This spec proposes replacing a working 80.6% CharCNN with an untested multi-branch design on a two-sprint timeline. The ambition is justified, but four issues could individually derail the sprint: (1) the training data format transition is the biggest task and has no specification detail; (2) per-value Model2Vec embedding has never been benchmarked on x86 CPU; (3) sibling-context attention is unresolved, putting the 170/174 floor at risk; and (4) the regression baseline doesn't exist as a reproducible artifact. The 85% target is aggressive for an architecture that's never been run. Fix the four critical items and add a rollback criterion, and the spec is shippable.
