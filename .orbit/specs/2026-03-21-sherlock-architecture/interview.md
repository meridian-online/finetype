# Interview: Sherlock-Style Architecture for FineType

**Date:** 2026-03-21
**Interviewer:** Nightingale

---

## Context

With the retraining spike proving that blended distilled+synthetic data improves Tier 2 accuracy from 75.0% to 80.6%, and 119K distilled rows now accumulated from Sherlock, the question is: what is the most ambitious training we can run to improve FineType?

The prior architecture review (2026-03-11) identified Sherlock-style expanded features as the "lowest-risk path for visually-similar types" — but Spike A was never executed. The hierarchical head (v15) was trained on synthetic only. Both represent untapped potential.

## Interview Q&A

### Q1: Ambition Dimension
**Q:** What's the primary dimension you want to push on? The spike held architecture constant and only varied data mix. We could push data volume, architecture scale, training methodology, or all three.
**A:** Full rethink with literature review. Consider what the Sherlock research found. Revisit the architecture review from 2026-03-11 — it's worth checking again now that we have distilled data.

### Q2: Scope
**Q:** The architecture review's Spike A (Sherlock-style features) was never executed. What's your appetite for architectural change vs training-data-only improvement?
**A:** Full Sherlock replication. Replace CharCNN entirely with multi-branch architecture adapted for 250 types. Highest ambition, highest risk.

### Q3: Embeddings
**Q:** Sherlock's features require GloVe (200d) and Doc2Vec (400d). FineType is pure Rust with Model2Vec already integrated. How do you handle the embedding dependency?
**A:** Adapt with Model2Vec (pure Rust). Research model2vec-rs to confirm no limitations. Pure Rust is the right decision.

**Research finding:** Model2Vec-potion-base-4M (128d, ~8MB) is already integrated in three FineType components (semantic hints, entity classifier, Sense). It handles short cell values well, has phrase-level semantics (unlike GloVe), and stays pure Rust. The 960-dim char distribution features — which need no embeddings — port to Rust trivially and are arguably Sherlock's most valuable feature set.

### Q4: Classification Head
**Q:** Flat (v14-style, 250-way softmax) or hierarchical (v15-style, tree softmax 7→43→250)?
**A:** Ablation study — train both. M1 is fast, data prep is the bottleneck not training. Get both answers at once.

**Recommendation context:** Research suggested flat-first for clean attribution, but ablation is justified since M1 training is cheap and the spike methodology (vary one thing) is preserved — the architecture is held constant while the head varies.

### Q5: Pipeline Architecture
**Q:** Keep Sense→Sharpen routing or go direct like Sherlock (single pass)?
**A:** Redefine the stages. Sense becomes the most sophisticated model we can build (the Sherlock-style multi-branch, 250-type classification). Sharpen becomes a container for deterministic post-inference corrections: leading zeros, pandas `.0` artifacts, column cardinality for categorical, locale detection. The model IS Sense. The rules stay as Sharpen.

### Q6: Training Data
**Q:** Train on existing 119K distilled rows (87% of Sherlock) or finish the remaining 180 batches first?
**A:** Train now with 119K. The spike proved the concept with 58K. Doubling should amplify gains without waiting.

### Q7: Success Criteria
**Q:** What does success look like? The spike's best was 80.6% Tier 2 overall.
**A:** Beat 85% Tier 2 overall. ~5pp above the spike's best, justifying the architectural investment. No regression worse than -3 on any individual type.

### Q8: Timeline
**Q:** Sherlock-style multi-branch needs: char feature extraction in Rust, Model2Vec branch adaptation, multi-branch training pipeline, feature vector data prep, ablation study, evaluation. Time budget?
**A:** Two sprints (2 weeks). Comfortable timeline for full implementation + thorough evaluation.

---

## Summary

### Goal
Build a Sherlock-style multi-branch neural network architecture for FineType that replaces CharCNN as the primary classification model. The new model combines 960-dim character distribution features, Model2Vec (128d) value embeddings, and statistical features into a multi-branch architecture targeting 250 types. Train with blend-30-70 (30% distilled, 70% synthetic) data. Ablation study: flat head vs hierarchical tree softmax.

### Constraints
- Pure Rust implementation (zero Python dependencies at inference)
- Model2Vec-potion-base-4M for embeddings (no GloVe/Doc2Vec)
- 10–50 MB total model budget
- Train on existing 119K distilled rows + synthetic generators
- M1 Pro hardware for training
- Two-sprint timeline (2 weeks)

### Success Criteria
- Beat 85% Tier 2 overall accuracy (current best: 80.6% from spike)
- No individual type regression worse than -3
- Profile eval (Tier 1): no regression from 170/174

### Architecture Vision
- **Sense** = Sherlock-style multi-branch model (the best classifier we can build)
- **Sharpen** = deterministic post-inference corrections (leading zeros, pandas artifacts, cardinality, locale detection)
- Single-pass classification, no routing — direct 250-type prediction with post-hoc cleanup

### Open Questions
- Exact feature dimensionality for the Model2Vec branch (per-value embedding + column aggregation strategy)
- Whether to include sibling-context attention in the new architecture or treat it as a separate enhancement layer
- Handling of the 72% disagreement rate in distilled data — filter to agreement-only, or use all rows?
- How the existing 6 feature-based disambiguation rules (F1–F6) map to the new Sharpen stage
