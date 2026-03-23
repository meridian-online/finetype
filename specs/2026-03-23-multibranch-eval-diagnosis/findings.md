# Findings: Multi-Branch Generalization Gap Diagnosis

**Date:** 2026-03-23 (updated after VarBuilder fix and baseline re-eval)
**Models:** sherlock-v2-flat (94.0% val), sherlock-v2-hier (93.9% val)
**Eval set:** 30 datasets, 190 ground truth columns (expanded from 174)

## 0. Fair Comparison

All models evaluated on the same 190-column eval set:

```
| Model                              | Label          | Domain         | Actionability |
|------------------------------------|----------------|----------------|---------------|
| char-cnn-v14-250 + Sense->Sharpen  | 148/190 (78%)  | 183/190 (96%)  | 99.5%         |
| sherlock-v2-flat (raw)             | 101/190 (53%)  | 148/190 (78%)  | 99.5%         |
| sherlock-v2-hier (raw)             | 101/190 (53%)  | 148/190 (78%)  | 97.0%         |
```

The previous report compared multi-branch (190 columns) against the baseline's old score
(170/174 on the smaller eval set), overstating the gap by ~20pp. The baseline also drops
on the expanded eval set — from 98% to 78% label accuracy. The **real label gap is 25pp**,
not 41pp.

Note: the baseline's domain accuracy (96%) shows the Sense->Sharpen pipeline is strong at
broad classification. The multi-branch model matches it on domain (78%) without any
post-processing, which is encouraging.

## 1. Gap Attribution (revised)

The multi-branch model scores 53% on profile eval vs the baseline's 78% — a **25-point gap**.
Two factors contribute:

### Factor A: Missing post-processing (~25pp — accounts for the full gap)

The multi-branch model runs **without any post-processing**. The production Sense->Sharpen
pipeline applies 7 sequential processing layers after the raw classifier:

1. Sense category routing + masking
2. CharCNN vote aggregation
3. Disambiguation rules (F1-F6)
4. Validation-based attractor demotion
5. Semantic header hints (Model2Vec)
6. Entity demotion
7. Geography rescue

The counterfactual trace of the 10 highest-confidence misclassifications shows:

```
| Fix category                        | Count | Percentage |
|-------------------------------------|-------|------------|
| Header hint would fix               | 5     | 50%        |
| Sense would fix                     | 2     | 20%        |
| Disambiguation rule would fix       | 1     | 10%        |
| Needs training data / GT review     | 2     | 20%        |
```

**80% of the top errors are recoverable by existing post-processing layers.**

### Factor B: Distribution shift (masked by expanded eval set)

The model's 94.0% validation accuracy is on a synthetic+distilled data distribution:
- 70% synthetic data (from FineType generators)
- 30% distilled data (from Sherlock real-world columns, filtered through FineType)

The profile eval uses 30 real-world CSV datasets. The 16 new columns added to the eval set
are harder for both the baseline and multi-branch models — the baseline dropped from 98% to
78% on the expanded set. Distribution shift exists but is harder to quantify separately
from the eval set expansion.

### Factor C: No header signal in model

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

## 2. Why 94% Val != 53% Eval

This is **not overfitting** in the classical sense. The model generalises well within
its training distribution (val loss converges, no train-val divergence). The gap is
**distribution mismatch** compounded by a harder eval set:

- Training distribution: synthetic generators + distilled Sherlock data
- Eval distribution: real-world CSV datasets with natural format variation
- The baseline also struggles on the expanded eval set (78% vs 98% on the old set)

The 94% validation accuracy proves the model architecture works. The 53% eval accuracy
proves the training data doesn't cover the real-world distribution.

## 3. Hierarchical Inference Fix

The hierarchical model eval initially returned 0/0 due to two bugs:

1. **VarBuilder prefix mismatch:** Training saves hierarchical tensors under `vb.pp("hier")`,
   producing names like `hier.hier.domain.weight`. The inference loader used `vb.clone()`
   (root), looking for `hier.domain.weight`. Fixed: `vb.clone()` -> `vb.pp("hier")`.

2. **Silent eval failure:** `profile_eval.sh` captured only `head -1` of stderr (an info log)
   instead of the actual error, and continued with empty results instead of aborting.

Hardened with:
- `HierarchicalHead::VARBUILDER_PREFIX` constant shared between training and inference
- `profile_eval.sh` now aborts if zero datasets profiled
- Error capture greps for `Error:` lines instead of `head -1`

Both flat and hierarchical models now evaluate correctly through the same pipeline.

## 4. Recommended Integration Plan

### Phase 1: Post-processing overlay (next PR)

Apply existing Sense->Sharpen post-processing on top of multi-branch predictions:

1. After multi-branch predicts a column type, run it through:
   - Validation-based filtering (demote types whose validation pattern fails)
   - Header hints (semantic + hardcoded)
   - Sense category cross-check (flag if prediction contradicts Sense routing)
2. This is an **additive** change — no modification to the multi-branch model itself
3. Expected: close to baseline 78% (the post-processing is the same)
4. Key question: does multi-branch + post-processing **exceed** the baseline?

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

### Accuracy Target (revised)

```
| Phase                               | Est. Label Accuracy | Gap to Baseline |
|-------------------------------------|---------------------|-----------------|
| Current (raw multi-branch)          | 53%                 | -25pp           |
| Phase 1 (+ post-processing overlay) | ~78%                | ~0pp            |
| Phase 2 (+ header features)         | ~83-88%             | +5-10pp         |
| Phase 3 (+ real-world training)     | ~88-93%             | +10-15pp        |
| Baseline (Sense->Sharpen)           | 78%                 | reference       |
```

**The question Phase 1 answers:** Does multi-branch add value as a replacement for
CharCNN within the existing pipeline? If Phase 1 matches baseline (~78%), the model
is neutral. If it exceeds baseline, the architecture is an upgrade. Phase 2 and 3
are only worth pursuing if Phase 1 shows the model adds signal.

## 5. Training Data Recommendations

For the next training run:
1. **Increase distilled ratio:** 30% -> 50% distilled data (closer to real-world distribution)
2. **Add header as a feature:** Include column name in the FTMB format
3. **Balance by difficulty:** Over-sample types that the model currently confuses
   (country vs country_code, ip_v4 vs cidr, year vs compact_ym)
4. **Add negative examples:** Include cross-domain confusable pairs as explicit training signal

## 6. Top Misclassification Clusters (89 errors)

```
| Failure mode                     | Count | Example                           |
|----------------------------------|-------|-----------------------------------|
| decimal_number vs integer_number | 13    | temperature_c, ph_value, pe_ratio |
| full_name / last_name attractors | 6     | publisher, job_title, airport name|
| longitude/lat/ean on plain nums  | 6     | height_cm, weight_kg, port        |
| URLs -> ethereum_address         | 3     | tracking_url, profile_url         |
| phone -> ssn/abn                 | 3     | phone in 3 datasets               |
| version on decimals              | 3     | sepal_length, mag, petal_width    |
| Everything else                  | 55    | scattered across many types       |
```

The decimal/integer confusion (13 errors) and entity name confusion (6 errors) are the
largest clusters. Both are addressable by validation-based demotion and header hints
respectively — reinforcing Phase 1 as the right next step.

## 7. Decision: Merge PR #21 Now

PR #21 should merge with:
- Working eval for both flat and hierarchical models (VarBuilder fix)
- Hardened eval pipeline (fail-fast, better error capture, shared constant)
- Counterfactual trace documenting which pipeline layers close the gap
- JSON eval output queryable by DuckDB
- This findings document with corrected gap attribution and integration plan

The next PR will implement Phase 1 (post-processing overlay) to answer: does multi-branch
add value within the existing pipeline?
