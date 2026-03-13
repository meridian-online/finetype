# Disambiguator Spike: Findings

**Date:** 2026-03-13
**Seed:** `specs/2026-03-disambiguator-spike/seed.yaml`
**Decision:** `decisions/0034-remove-id-increment-header-hint.md`

---

## Summary

**Recommendation: No-go on replacing the rule cascade with a learned disambiguator.**

A logistic regression and MLP trained on 278 eval columns achieved 19% and 18% accuracy respectively, versus 73% for the current rule cascade. The primary cause is not model weakness — it's a fundamental label granularity mismatch between coarse ground-truth labels and FineType's fine-grained 250-type taxonomy. The current rules work well *because* they operate at the type level the taxonomy defines, not the abstraction level humans naturally label at.

**Immediate fix shipped:** Removed the `id` → `increment` header hint (decision 0034). The earthquake dataset no longer produces a failing `CAST(id AS BIGINT)`.

---

## Experiment Results

### Setup

- **Dataset:** 278 columns from 27 CSV files (eval manifest + earthquake dataset)
- **Features:** 144-dim (36 Sherlock-style features × 4 aggregations: mean, variance, min, max)
- **Models:** Logistic regression (144 → n_classes), MLP (144 → 64 → n_classes, ReLU)
- **Training:** SGD, lr=0.01, 200 epochs, 5-fold cross-validation
- **Framework:** Candle (already a workspace dependency)

### Accuracy Comparison

| Model | Mean Accuracy (5-fold) | Correct/Total |
|-------|----------------------|---------------|
| Logistic Regression | 19.1% | 53/278 |
| MLP (64 hidden) | 18.0% | 50/278 |
| **Current rules** | **73.0%** | **203/278** |

### Per-Dataset Accuracy (Logistic Regression)

| Dataset | LR | MLP | Rules (approx) |
|---------|-----|------|------|
| scientific_measurements | 54.5% | 36.4% | ~100% |
| financial_data | 41.7% | 25.0% | ~100% |
| network_logs | 41.7% | 16.7% | ~90% |
| sports_events | 41.7% | 50.0% | ~80% |
| airports | 30.8% | 30.8% | ~90% |
| earthquakes_2024 | 13.6% | 31.8% | ~50% |
| codes_and_ids | 0.0% | 0.0% | ~100% |
| new_identity | 0.0% | 0.0% | ~100% |
| new_technology | 0.0% | 0.0% | ~100% |
| new_geography | 0.0% | 0.0% | ~100% |

### Top 20 Feature Importance (Logistic Regression Weights)

| Rank | Feature | Score |
|------|---------|-------|
| 1 | max_is_integer | 0.1038 |
| 2 | mean_has_iso_date_sep | 0.1029 |
| 3 | max_has_leading_zero | 0.1025 |
| 4 | var_segment_count_slash | 0.1018 |
| 5 | min_is_numeric | 0.1014 |
| 6 | max_ends_with_digit | 0.1007 |
| 7 | mean_has_protocol_prefix | 0.1006 |
| 8 | max_punctuation_density | 0.1004 |
| 9 | mean_segment_count_slash | 0.1004 |
| 10 | mean_lowercase_count | 0.0998 |
| 11 | var_unique_char_ratio | 0.0995 |
| 12 | mean_alpha_count | 0.0991 |
| 13 | mean_is_integer | 0.0991 |
| 14 | max_digit_count | 0.0990 |
| 15 | var_has_mixed_case | 0.0989 |
| 16 | max_is_numeric | 0.0987 |
| 17 | mean_alpha_ratio | 0.0981 |
| 18 | max_has_dash | 0.0979 |
| 19 | max_alpha_ratio | 0.0978 |
| 20 | var_max_alpha_run | 0.0977 |

Feature weights are nearly flat (range 0.098–0.104), suggesting no single feature is strongly predictive of fine-grained type — consistent with the problem being a classification task over too many classes with too few examples.

---

## Failure Mode Analysis (AC-7)

### Why learned models fail

The dominant failure mode is **label granularity mismatch**, not model inadequacy.

**Example "misclassifications" that are actually correct:**

| Column | gt_label | Prediction | Match? |
|--------|----------|------------|--------|
| iris.species | category | representation.discrete.categorical | ✓ semantically |
| earthquakes_2024.time | iso timestamp milliseconds | datetime.timestamp.iso_8601_milliseconds | ✓ semantically |
| people_directory.email | email | identity.person.email | ✓ semantically |
| titanic.Name | name | identity.person.full_name | ✓ semantically |
| books_catalog.year_published | year | datetime.component.year | ✓ semantically |
| geography_data.latitude | latitude | geography.coordinate.latitude | ✓ semantically |

The `labels_match()` heuristic catches many of these, bringing rule accuracy to 73%. But the learned models predict at the fine-grained level while being evaluated at the coarse level — a mismatch that makes cross-validation accuracy unreliable.

### Categories of current rule errors (75 columns where rules "fail")

1. **Coarse gt_label, correct fine prediction (≈50 columns):** FineType predicts the right fine-grained type but `labels_match()` doesn't map it. Example: `network_logs.method: gt=category, pred=technology.internet.http_method`. The prediction is *more* correct than the label.

2. **Genuinely ambiguous columns (≈10 columns):** Headers like `name`, `status`, `id` that could legitimately be multiple types depending on context.

3. **Actual misclassifications (≈15 columns):** Real errors in the pipeline.
   - `earthquakes_2024.magType/net/locationSource/magSource → file.extension` (2-letter codes misclassified)
   - `earthquakes_2024.id → geohash` (alphanumeric codes that look like geohashes)
   - `earthquakes_2024.place → coordinates` (place descriptions with numbers)
   - `books_catalog.isbn → cas_number` (format similarity)
   - `sports_events.status → geography.location.region` (short text strings)

### What rules get right that the model cannot learn

The rule cascade succeeds because it operates on **disambiguation signals** rather than raw features:

1. **Sense masking** (e.g., `sense_mask:format`): Filters the CharCNN vote distribution to relevant types. This is the pipeline's core strength — the model already has the right answer, rules just remove noise.
2. **Header hints** (e.g., `sense_header_hint_cross_domain:latitude`): Direct header-to-type mapping for unambiguous headers. Trivially correct but requires the lookup table.
3. **Feature rules F1–F5** (e.g., `feature_no_leading_zero`, `feature_git_sha`): Targeted checks for specific structural patterns. Only fire when features match precise criteria.

A learned model would need to replicate all three strategies from features alone — but features only capture strategy 3. Strategies 1 and 2 require the vote distribution and header string, which weren't included as model inputs.

---

## Data Sufficiency Assessment (AC-8)

### Is 278 columns enough?

**No, not for a 250-class problem.** Standard ML guidance suggests at least 10–50 examples per class. With 278 columns and ~120 unique types observed, most classes have 1–3 examples. This makes cross-validation unstable and prevents the model from learning type-specific patterns.

### Would more data help?

| Data Source | Size | Quality | Feasibility |
|-------------|------|---------|-------------|
| Current eval set | 278 columns | Gold labels (coarse) | Already used |
| LLM distillation (Qwen3 8B) | 5,359 columns | 20% agreement with FineType | Available but noisy |
| Synthetic generation | Unlimited | Perfect labels | Needs generator per type |
| Real-world CSVs (unlabelled) | ~500 datasets | No labels | Would need annotation |

Even with 5,359 columns from LLM distillation, label noise (80% disagreement) would likely degrade rather than improve a learned model. The path to a useful learned disambiguator requires **fine-grained ground truth labels** (FineType taxonomy keys, not coarse categories), which don't exist at scale.

### Learning curve projection

At 278 samples with 120+ classes, the model is firmly in the "not enough data" regime. Doubling the dataset would help but not solve the fundamental problem. An order-of-magnitude increase (2,500+ columns with fine-grained labels) would be the minimum viable training set.

---

## Earthquake Dataset Analysis

The earthquake dataset (USGS 2024, 22 columns, 14,132 rows) reveals several pipeline weaknesses:

| Column | gt_label | Predicted | Issue |
|--------|----------|-----------|-------|
| magType | category | file.extension | 2-letter codes (mb, ml) look like file extensions |
| net | category | file.extension | Same pattern (us, ci, nc) |
| locationSource | category | file.extension | Same pattern |
| magSource | category | file.extension | Same pattern |
| id | alphanumeric_id | geohash | Alphanumeric IDs (us6000pgkh) resemble geohashes |
| place | address | coordinates | Place descriptions with embedded numbers |
| gap | decimal_number | integer_number | Integers with no fractional part |
| horizontalError | decimal_number | integer_number | Same |
| status | status | ordinal | Close but imprecise |

**4× file.extension misclassification** is the most actionable finding. Short 2-letter alphabetic codes are a known weakness — the CharCNN sees them as file extensions. This could be addressed by:
- Adding a minimum-length threshold for file extension classification
- Using header context more aggressively (these headers don't suggest files)
- Adding a `short_code` / `abbreviation` type to the taxonomy

---

## Recommendation

### Decision: Keep rules, fix targeted gaps

The rule cascade at 73% (really ~90%+ accounting for label granularity) outperforms learned models by a wide margin. The path forward is:

1. **Fix specific misclassifications** (short-term):
   - Address 2-letter code → file.extension false positives (earthquake magType/net/locationSource/magSource)
   - Improve `id` column handling post-hint-removal (currently → geohash)
   - Consider a header-based override for "status" columns

2. **Expand eval ground truth** (medium-term):
   - Remap coarse gt_labels to fine-grained FineType keys in `eval/schema_mapping.yaml`
   - This alone would bring measured accuracy from 73% to ~90%+
   - Essential before any future learned-model experiments

3. **Revisit learned disambiguation later** (long-term prerequisites):
   - 2,500+ columns with fine-grained labels
   - Include vote distribution and header embedding as model inputs (not just features)
   - Train at the domain/category level (7 or 43 classes) rather than 250 types
   - Hybrid approach: learned model for ambiguous cases, rules for clear-cut types

### What NOT to do

- **Do not migrate hints to taxonomy YAML yet.** The hints work. Migration is a code organisation task, not an accuracy improvement. Deprioritise until after eval ground truth is fixed.
- **Do not train on LLM distillation data.** 80% disagreement rate means the labels would add noise.
- **Do not remove more hardcoded hints.** The `id` removal was justified because it was actively harmful. Other hints (latitude, email, etc.) are correct and useful.

---

## Acceptance Criteria Status

| AC | Description | Status |
|----|-------------|--------|
| AC-1 | Remove id → increment hint | ✅ Done (decision 0034) |
| AC-2 | Extract per-column features | ✅ 278 columns extracted |
| AC-3 | Earthquake dataset in eval | ✅ 22 columns added |
| AC-4 | Logistic regression experiment | ✅ 19.1% accuracy |
| AC-5 | MLP experiment | ✅ 18.0% accuracy |
| AC-6 | Feature importance | ✅ Top 20 features identified |
| AC-7 | Failure mode analysis | ✅ Above |
| AC-8 | Data sufficiency assessment | ✅ Above |
| AC-9 | Findings document | ✅ This document |
| AC-10 | Implementation plan | ✅ Not applicable (no-go) |

---

## Key Insight

The learned disambiguator failed not because the idea is wrong, but because it was tested at the wrong abstraction level. The 36-dim feature vector captures *structural* properties of values (numeric, has dashes, length, etc.) — useful for broad type discrimination but insufficient for distinguishing 250 fine-grained types. The current pipeline already solves this by using the CharCNN for fine-grained classification and features only for tie-breaking. The rules encode *when to trust the model vs. when to override* — a meta-decision that needs the model's vote distribution, not just features.

A future learned disambiguator should:
- Take the full vote distribution + confidence as input (not just features)
- Predict "trust model" vs. "override to X" (a much smaller decision space)
- Be trained on fine-grained labels to avoid the measurement problem we hit here
