# Findings: Multi-Branch Generalization Gap Diagnosis

**Date:** 2026-03-23
**Models:** sherlock-v2-flat (94.0% val), sherlock-v2-hier (93.9% val)
**Profile eval:** 108/190 (56.8% label), 144/190 (75.8% domain)
**Production baseline:** Sense→Sharpen pipeline: 170/174 (97.7% label, 98.9% domain)

## 1. Gap Attribution

The multi-branch model scores 56.8% on profile eval vs production's 97.7% — a **41-point gap**.
Three factors contribute:

### Factor A: Missing post-processing (estimated ~25pp recovery)

The multi-branch model runs **without any post-processing**. The production Sense→Sharpen
pipeline applies 7 sequential processing layers after the raw classifier:

1. Sense category routing + masking
2. CharCNN vote aggregation
3. Disambiguation rules (F1–F6)
4. Validation-based attractor demotion
5. Semantic header hints (Model2Vec)
6. Entity demotion
7. Geography rescue

The counterfactual trace of the 10 highest-confidence misclassifications shows:

```
| Fix category | Count | Percentage |
|--------------|-------|------------|
| Header hint would fix | 5 | 50% |
| Sense would fix | 2 | 20% |
| Disambiguation rule would fix | 1 | 10% |
| Needs training data / GT review | 2 | 20% |
```

**80% of the top errors are recoverable by existing post-processing layers.**

Extrapolating to the full 82 misclassifications (with diminishing returns at lower
confidence): estimated ~60-65% recovery rate → **~50 errors fixed → accuracy from
56.8% to ~83%**.

### Factor B: Distribution shift (estimated ~10pp gap)

The model's 94.0% validation accuracy is on a synthetic+distilled data distribution:
- 70% synthetic data (from FineType generators)
- 30% distilled data (from Sherlock real-world columns, filtered through FineType)

The profile eval uses 30 real-world CSV datasets with 190 directly-mappable columns.
These distributions differ in:

- **Format diversity:** Real-world data has more format variation than synthetic generators
- **Value distributions:** Synthetic generators produce "typical" values; real data has outliers
- **Column context:** Real datasets have multi-column context that synthetic data lacks
- **Label balance:** Synthetic data is balanced (1,200 samples/type); real data is skewed

This distribution mismatch means high val accuracy doesn't predict real-world performance.
The model learned synthetic patterns, not real-world data patterns.

### Factor C: No header signal in model (estimated ~5pp gap)

The multi-branch model has **no header information** in its feature vector:
- Branch 1: Character distribution (960d) — character-level patterns only
- Branch 2: Embedding aggregation (512d) — Model2Vec embeddings of values, not headers
- Branch 3: Column statistics (27d) — statistical features of values

The production pipeline integrates headers via:
- Semantic header hints (Model2Vec cosine similarity to type names)
- Hardcoded header patterns (financial column names)
- Sibling-context attention (cross-column header enrichment)

5 of 10 top misclassifications would be caught by header hints alone. The model is
architecturally blind to this signal.

## 2. Why 94% Val ≠ 57% Eval

This is **not overfitting** in the classical sense. The model generalises well within
its training distribution (val loss converges, no train-val divergence). The gap is
**distribution mismatch**:

- Training distribution: synthetic generators + distilled Sherlock data
- Eval distribution: real-world CSV datasets with natural format variation

The 94% validation accuracy proves the model architecture works. The 57% eval accuracy
proves the training data doesn't cover the real-world distribution.

## 3. Hierarchical Eval Fix

The hierarchical model eval returned 0/0 because `MultiBranchClassifier::load()` had
a hard guard rejecting non-flat heads. This has been fixed:

- Removed the `HeadType::Flat` guard
- Implemented hierarchical inference using the existing `HierarchicalHead` (tree softmax)
- The `ClassificationHead` enum now dispatches between flat (logits → softmax) and
  hierarchical (product probabilities from 3-level tree softmax)

Both flat and hierarchical models can now be evaluated through the same pipeline.

## 4. Recommended Integration Plan

### Phase 1: Post-processing overlay (next PR)

Apply existing Sense→Sharpen post-processing on top of multi-branch predictions:

1. After multi-branch predicts a column type, run it through:
   - Validation-based filtering (demote types whose validation pattern fails)
   - Header hints (semantic + hardcoded)
   - Sense category cross-check (flag if prediction contradicts Sense routing)
2. This is an **additive** change — no modification to the multi-branch model itself
3. Expected accuracy improvement: 56.8% → ~83% (based on trace analysis)

### Phase 2: Header features in model (future PR)

Add header information to the multi-branch feature vector:
- Branch 4: Header embedding (Model2Vec of column name) — 384d
- This gives the model direct access to header signal during training
- Expected additional improvement: ~5-10pp

### Phase 3: Real-world training data (future PR)

Expand training data to include real-world columns:
- Use profile eval datasets as additional training signal (leave-one-out or holdout split)
- Add more Sherlock distilled data with better extraction coverage
- Target: close the distribution shift gap
- Expected additional improvement: ~5-10pp

### Accuracy Target

```
| Phase | Estimated Accuracy | Gap to Production |
|-------|-------------------|-------------------|
| Current (no post-processing) | 56.8% | -41pp |
| Phase 1 (post-processing overlay) | ~83% | -15pp |
| Phase 2 (header features) | ~88-90% | -8pp |
| Phase 3 (real-world training data) | ~93-95% | -3pp |
| Production (Sense→Sharpen) | 97.7% | baseline |
```

**Realistic target after all three phases: 93-95%.** Closing the final 3-5pp to match
production would require either perfect training data coverage or model-specific
disambiguation rules.

## 5. Training Data Recommendations

For the next training run:
1. **Increase distilled ratio:** 30% → 50% distilled data (closer to real-world distribution)
2. **Add header as a feature:** Include column name in the FTMB format
3. **Balance by difficulty:** Over-sample types that the model currently confuses
   (country vs country_code, ip_v4 vs cidr, year vs compact_ym)
4. **Add negative examples:** Include cross-domain confusable pairs as explicit training signal

## 6. Decision: Merge PR #21 Now

PR #21 should merge with:
- ✅ Working eval for both flat and hierarchical models (D-1 fixed)
- ✅ Counterfactual trace documenting which pipeline layers close the gap (D-2 complete)
- ✅ JSON eval output queryable by DuckDB (D-3 implemented)
- ✅ This findings document with quantified gap attribution and integration plan (D-4)

The next PR will implement Phase 1 (post-processing overlay) and retrain with updated data.
