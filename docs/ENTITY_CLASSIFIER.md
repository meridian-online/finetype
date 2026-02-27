# Entity Classifier — Integration Specification

## Overview

The entity classifier is a post-vote column-level model that determines whether a
column of values contains person names or other entity types (places, organizations,
creative works). It fires as a **binary demotion gate** in FineType's column inference
pipeline: when CharCNN votes `full_name` but the entity classifier confidently says
"not person," the prediction is demoted to `entity_name`.

## Architecture

**Deep Sets MLP** (Zaheer et al. 2017):

```
Column values (N strings)
    ↓
Model2Vec encode each value (frozen, potion-base-4M, 128-dim)
    ↓
Aggregate: mean embedding (128) + std embedding (128) + statistical features (44) = 300-dim
    ↓
MLP: BatchNorm → 256 → ReLU/Drop → 256 → ReLU/Drop → 128 → ReLU/Drop → 4
    ↓
Softmax → probabilities per class (person, place, organization, creative_work)
    ↓
Binary decision: if max(non-person probs) > threshold → demote to entity_name
```

## Model Artifacts

```
models/entity-classifier/
  model.safetensors    # MLP weights (694 KB)
  config.json          # Architecture, feature spec, threshold, evaluation results
  label_index.json     # Class names: ["person", "place", "organization", "creative_work"]
```

## Feature Specification

The model takes a 300-dimensional feature vector per column:

| Range | Dimensions | Description |
|-------|-----------|-------------|
| 0–127 | 128 | Mean of Model2Vec value embeddings |
| 128–255 | 128 | Std of Model2Vec value embeddings |
| 256–299 | 44 | Statistical features (see below) |

### Statistical Features (44)

These must be reimplemented in Rust for inference. See `config.json:stat_feature_names`
for the ordered list, and `scripts/train_entity_classifier.py:compute_column_features()`
for the reference implementation.

**Length distribution (5):** mean_len, std_len, median_len, p25_len, p75_len

**Word count distribution (5):** mean_words, std_words, single_word_ratio, two_word_ratio, three_plus_ratio

**Character class ratios (5):** mean_alpha_ratio, mean_digit_ratio, mean_space_ratio, mean_punct_ratio, has_digits_ratio

**Structural patterns (8):** title_case_ratio, all_caps_ratio, has_comma_ratio, has_parens_ratio, has_ampersand_ratio, has_apostrophe_ratio, has_hyphen_ratio, has_dot_ratio

**Domain patterns (6):** org_suffix_ratio, person_title_ratio, place_indicator_ratio, creative_indicator_ratio, the_prefix_ratio, numeric_prefix_ratio. Regex patterns defined in training script — must be ported to Rust.

**Value diversity (5):** uniqueness, token_diversity, avg_word_len, cap_words_mean, cap_word_ratio

**Distributional shape (7):** word_density, short_value_ratio, long_value_ratio, cv_length, preposition_ratio, contains_number_ratio, has_quotes_ratio

**Column metadata (3):** column_size, max_value_len, max_word_count

## Integration Point

In `crates/finetype-model/src/column.rs`, the entity classifier fires in `classify_column()`
**after** vote aggregation and disambiguation rules, **before** header hint application.

### Trigger Condition

The classifier fires when:
1. The majority vote label is `identity.person.full_name`, AND
2. The entity classifier model is loaded (optional component, like semantic hints)

### Demotion Logic

```rust
// Pseudocode for integration
if majority_label == "identity.person.full_name" {
    if let Some(entity_model) = &self.entity_classifier {
        let features = compute_entity_features(values, &self.model2vec);
        let probs = entity_model.forward(features);  // [person, place, org, creative_work]
        let max_nonperson_prob = probs[1..].iter().max();
        if max_nonperson_prob > DEMOTION_THRESHOLD {
            result.label = "representation.text.entity_name".to_string();
            result.disambiguation_rule = Some("entity_demotion:nonperson".to_string());
        }
    }
}
```

### Threshold

Default: **0.6** (configurable via `config.json:demotion_threshold`)

At 0.6 on balanced SOTAB test data:
- 92.2% demotion precision (7.8% of demotions are wrong — actual person columns)
- 65.9% coverage (65.9% of non-person columns get correctly demoted)

At production base rates (~96% of full_name predictions are non-person):
- **~99% demotion precision** (Bayesian adjustment for skewed base rate)

## Evaluation Results

**4-class accuracy:** 75.8% on held-out SOTAB test (2,117 columns)

| Class | Precision | Recall | F1 |
|-------|-----------|--------|-----|
| person | 74.3% | 76.3% | 75.3% |
| place | 75.4% | 76.8% | 76.1% |
| organization | 66.7% | 68.9% | 67.8% |
| creative_work | 85.6% | 79.8% | 82.6% |

**Training data:** 2,911 SOTAB validation columns (person 816, place 719, org 647, creative_work 729)
**Test data:** 2,117 SOTAB test columns (person 595, place 462, org 466, creative_work 594)

## Rust Implementation Checklist

For the follow-up integration task:

1. [ ] Load `model.safetensors` via Candle (Linear + BatchNorm1d layers)
2. [ ] Reuse existing Model2Vec from `SemanticHintClassifier` for value encoding
3. [ ] Implement `compute_entity_features()` in Rust (44 statistical features)
4. [ ] Port regex patterns (ORG_SUFFIXES, PERSON_PATTERNS, PLACE_PATTERNS, CREATIVE_PATTERNS)
5. [ ] Add `EntityClassifier` field to `ColumnClassifier` struct
6. [ ] Wire into `classify_column()` after disambiguation, before header hints
7. [ ] Add `--entity-model` CLI flag or auto-detect from model directory
8. [ ] Test with profile eval — verify full_name overcall reduction
