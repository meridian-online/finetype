# Interview: Sibling-Context Attention Training Pipeline

**Date:** 2026-03-11
**Interviewer:** Nightingale

---

## Context

The sibling-context attention module (NNFT-268) is architecturally complete and integrated into the Sense pipeline, but has no trained weights — it's currently inert. The module enriches column embeddings with cross-column context via 2-layer pre-norm transformer self-attention over Model2Vec embeddings (396,800 params, 1.51 MB). It addresses 3/7 remaining bare-name ambiguity errors in profile eval (180/186).

An earlier LLM distillation run (NNFT-269) produced 5,359 labelled columns from 508 real CSVs via Qwen3 8B, but only achieved 20% agreement with FineType. The key insight: the **table structure** (which columns appear together) is the valuable asset, not the LLM labels. FineType's own predictions at 96.8% accuracy are a cleaner supervision signal.

## Interview Q&A

### Q1: Training objective
**Q:** Should the module learn to improve Sense classification accuracy (supervised, end-to-end with Sense), or learn good column representations independently (self-supervised)?
**A:** Supervised with Sense. Train attention so that Sense classification error on real tables decreases. Directly optimises what we care about.

### Q2: Label source
**Q:** For supervised training we need (table_columns, correct_sense_labels) pairs. Do we use FineType's current Sense predictions as ground truth (silver labels), or manually verify a subset?
**A:** Silver labels from FineType. Run profile on all 508 CSVs, use FineType's Sense predictions as training targets. 96.8% profile accuracy means ~3-5% noise — acceptable.

### Q3: Weight freezing
**Q:** Should we freeze Model2Vec embeddings and Sense weights (only train the attention layers), or fine-tune Sense end-to-end alongside attention?
**A:** Freeze both, train attention only. Model2Vec + Sense stay fixed. Attention learns to rearrange embeddings so Sense's existing weights perform better. No risk of degrading Sense on single-column (N=1) inputs.

### Q4: Data scale
**Q:** We have 508 tables averaging ~10 columns each (5,359 total columns). Augment with more GitTables, or work with what we have?
**A:** Start with existing 508 CSVs. Build the pipeline end-to-end first. If results are weak, the bottleneck is data and we can scale up later.

### Q5: Code location
**Q:** Should the training pipeline live in the existing finetype-train crate or as a new standalone crate?
**A:** New binary in finetype-train (`train_sibling_context.rs`). Reuses existing Candle infrastructure, device detection, data pipeline patterns.

### Q6: Evaluation
**Q:** How should we evaluate whether the trained attention module actually helps?
**A:** Profile eval delta. Run profile eval before/after loading the attention model. Target: fix at least 1 of the 3 bare-name ambiguity cases (180→181+/186). Uses existing infrastructure.

### Q7: Batching strategy
**Q:** Should we batch over tables (1 table = 1 training example) or batch over columns across tables?
**A:** Batch over tables. Each training example = 1 table with N columns. Natural fit since attention operates on all columns of one table. Variable column count handled by padding/masking.

---

## Summary

### Goal
Train the sibling-context attention module (NNFT-268) to improve Sense classification accuracy by enriching column embeddings with cross-column context from real-world tables.

### Constraints
- Frozen Model2Vec embeddings and Sense classifier weights — only attention parameters are trainable
- Silver labels from FineType's own predictions (no manual labelling)
- Existing 508 CSVs (~5,359 columns) as training data — no new data collection
- New training binary in `finetype-train` crate, reusing existing infrastructure
- Table-level batching (1 table = 1 training example, variable column count)

### Success Criteria
- Profile eval improvement: 180/186 → 181+/186 (fix at least 1 bare-name ambiguity case)
- No regression on single-column (N=1) inputs — attention module must be a no-op for single columns
- Training completes on M1 MacBook in reasonable time (<1 hour)
- Trained model artifact saved to `models/sibling-context/` and loadable by the existing pipeline

### Open Questions
- Learning rate and epoch count — need to tune empirically
- Train/validation split strategy for 508 tables (random 80/20? stratified by domain coverage?)
- Whether padding/masking for variable column counts introduces training artifacts
- Whether 5,359 columns is sufficient for 396K parameters — may need data augmentation (column dropout, permutation) or more GitTables
