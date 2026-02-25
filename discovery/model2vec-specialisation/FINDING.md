# Finding: Model2Vec Specialisation for Column Name Classification

**Task:** NNFT-119
**Date:** 2026-02-25
**Time budget:** 4 hours
**Status:** Complete

## Executive Summary

The primary lever for improving Model2Vec is **threshold adjustment** (0.70 → 0.65), not synonym expansion or custom distillation. Lowering the threshold recovers 12 correct matches with minimal false positive risk. Synonym expansion suffers from centroid dilution and is net-negative at lower thresholds. Custom distillation is promising but untested (requires torch).

**Recommended action:** Lower threshold from 0.70 to 0.65 and add targeted synonyms for ~5 specific types. Expected gain: +12 correct semantic overrides with ≤2 additional false positives.

## Baseline Measurement (AC #1)

Tested all 206 column names from the profile evaluation against the current Model2Vec semantic hint system (potion-base-4M, 0.70 threshold).

| Category | Count | Percentage |
|---|---|---|
| Correct match above 0.70 (active TPs) | 96 | 46.6% |
| Correct match below 0.70 (lost opportunities) | 50 | 24.3% |
| Wrong match above 0.70 (false positives) | 6 | 2.9% |
| Correctly rejected | 54 | 26.2% |

**Key finding:** 50 columns have the correct type as their top-1 match but fall below the 0.70 threshold. Of these:
- 12 are in [0.65, 0.70) — recoverable by threshold adjustment alone
- 7 are in [0.60, 0.65) — recoverable with threshold + targeted synonyms
- 5 are in [0.50, 0.60) — need vocabulary improvement
- 26 are below 0.50 — need fundamentally better embeddings

**Driving example:** `salary` → `representation.numeric.decimal_number` at 0.477. Despite "salary" being a synonym in the header hint list, the general-purpose embedding doesn't place it close enough to the type centroid.

### Threshold Sweep

| Threshold | TP | FP | Precision | Recall |
|---|---|---|---|---|
| 0.60 | 115 | 12 | 0.906 | 0.788 |
| **0.65** | **108** | **8** | **0.931** | **0.740** |
| 0.68 | 101 | 7 | 0.935 | 0.692 |
| **0.70 (current)** | **96** | **6** | **0.941** | **0.658** |
| 0.75 | 82 | 5 | 0.943 | 0.562 |

## Synonym Expansion (AC #2)

Added 244 synonyms across 19 types (709 → 953 total synonym texts). Real-world column naming conventions from Kaggle, GitTables, and database naming patterns.

### Results at 0.70 threshold

| Metric | Baseline | Expanded | Delta |
|---|---|---|---|
| True Positives | 96 | 102 | +6 |
| False Positives | 6 | 8 | +2 |

9 columns were newly recovered above 0.70, including `timezone` (0.669 → 0.893), `status_code` (0.683 → 0.813), and `shipping_postal_code` (0.685 → 0.795).

### The centroid dilution problem

Adding many synonyms to mean-pooled type embeddings **dilutes the centroid**. This caused 29 regressions:

| Column | Baseline | Expanded | Delta |
|---|---|---|---|
| `mime_type` | 0.781 | 0.519 | -0.262 |
| `full_name` | 0.911 | 0.707 | -0.203 |
| `percentage` | 0.926 | 0.754 | -0.172 |
| `timestamp` | 0.732 | 0.582 | -0.149 |

**Root cause:** Mean-pooling "salary", "price", "cost", "amount", "revenue", "profit", "margin", "balance", "total", "subtotal", "tax", "rate", "measurement", "metric" produces a generic "numbers/quantities" centroid that's far from any specific term. The more diverse the synonyms, the more generic the centroid.

**At lower thresholds, expansion is net-negative:**

| Threshold | Baseline TP | Expanded TP | Delta |
|---|---|---|---|
| 0.70 | 96 | 102 | +6 |
| 0.65 | 108 | 106 | **-2** |
| 0.60 | 115 | 108 | **-7** |

### Conclusion

Naive synonym expansion via mean-pooling is counterproductive when combined with threshold lowering. The gains at 0.70 are entirely offset by dilution losses at lower thresholds.

**Better approaches for future work:**
1. **Max-sim matching** — For each type, store top-K synonym embeddings separately. Match query against each, take the max. Avoids centroid dilution entirely.
2. **Weighted mean-pool** — Weight title/leaf-name embeddings 3x higher than header hint entries.
3. **Selective expansion** — Only add synonyms whose embedding is within 0.5 cosine similarity of the existing centroid. This prevents adding semantically distant terms that pull the centroid off-target.

## Custom Distillation (AC #3)

**Not tested** — requires `torch` (not installed). The vocabulary list (213 analytics/database terms) is prepared in `evaluate_distillation.py`.

**Hypothesis:** Distilling from all-MiniLM-L6-v2 with analytics-domain vocabulary would improve token embeddings for database column names. This addresses the fundamental issue that potion-base-4M was trained on general English text, not column naming conventions.

**Effort estimate:** ~30 minutes to install torch, distill, regenerate type embeddings, and test. No Rust code changes needed — just swap the model artifacts.

**Recommendation:** Defer to implementation phase. Threshold adjustment is the higher-value, lower-risk change.

## False Positive Assessment (AC #4)

Tested 163 generic/ambiguous column names against the baseline model to assess the risk of lowering the threshold.

### Generic names above threshold

| Name | Match | Similarity | Genuinely wrong? |
|---|---|---|---|
| `xml` | container.object.xml | 0.936 | No — correct |
| `csv` | container.object.csv | 0.895 | No — correct |
| `json` | container.object.json | 0.861 | No — correct |
| `text` | representation.text.plain_text | 0.857 | No — correct |
| `query` | container.key_value.query_string | 0.724 | Borderline |
| `string` | container.key_value.query_string | 0.722 | Borderline |
| `offset` | datetime.offset.utc | 0.696 | Borderline |
| `data` | container.key_value.form_data | 0.687 | Yes — false positive |

**Key finding:** Of 163 generic names, only **1** (`data` at 0.687) is a genuine false positive below 0.70 but above 0.65. The others that breach 0.65 are either correct matches (`xml`, `csv`, `json`, `text`) or borderline-acceptable (`query`, `string`, `offset`).

### Risk assessment by threshold

| Threshold | Genuine FPs from generic set |
|---|---|
| 0.70 | 0 (query/string are borderline, not harmful) |
| 0.68 | 0 |
| 0.65 | 1 (data → form_data) |
| 0.60 | 1 (same) |

**Conclusion:** Lowering threshold to 0.65 carries minimal false positive risk. The `data` case (0.687) is the only concern, and even that is borderline — `data` could plausibly refer to form data in some contexts.

## Recommendation

### Immediate action (low effort, high impact)

**Lower threshold from 0.70 to 0.65.**

- Gains: +12 true positives (timezone, ean, shipping_postal_code, status_code, content_type, price variants, tracking_url, alpha-2/3, rating, unix_ms)
- Cost: +2 false positives on eval columns (borderline cases), +1 on generic names (data)
- Precision: 93.1% (vs current 94.1%)
- Recall: 74.0% (vs current 65.8%)
- Code change: single constant in `semantic.rs`

### Short-term (implementation task)

**Add targeted synonyms for ~5 specific types** where the [0.65, 0.70) bucket shows the biggest gains:

| Type | Current sim | Needs synonyms for |
|---|---|---|
| `datetime.offset.iana` | 0.669 | timezone, tz, zone |
| `geography.address.postal_code` | 0.685 | shipping postal code, billing postal code |
| `technology.internet.url` | 0.638 | tracking url, callback url, redirect url |
| `technology.internet.http_status_code` | 0.683 | status code, response code |
| `representation.file.mime_type` | 0.682 | content type, media type |

Keep expansion minimal (3-5 per type) to avoid centroid dilution.

### Medium-term (follow-up task)

**Implement max-sim matching** instead of mean-pooled centroids:
- Store top-3 synonym embeddings per type (not mean-pooled)
- Match query against each, take the maximum similarity
- Eliminates the centroid dilution problem entirely
- Allows aggressive synonym expansion without regressions

### Long-term (when infrastructure supports it)

**Custom distillation** with analytics-domain vocabulary:
- Install torch, distill from all-MiniLM-L6-v2 with 213-term curated vocab
- Regenerate type embeddings
- Test against full eval suite
- May recover the 26 columns below 0.50 where vocabulary is the bottleneck

## Data Files

| File | Purpose |
|---|---|
| `analyse_similarity.py` | AC #1: baseline similarity measurement for all 206 columns |
| `evaluate_synonym_expansion.py` | AC #2: synonym expansion impact with 244 added synonyms |
| `evaluate_distillation.py` | AC #3/4: distillation test + false positive assessment |
| `BRIEF.md` | Original discovery brief |
| `FINDING.md` | This document |
