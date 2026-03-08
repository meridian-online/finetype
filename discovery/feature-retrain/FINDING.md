# Feature-Augmented CharCNN Retrain Spike (NNFT-253)

**Date:** 2026-03-08
**Question:** Does training CharCNN with 32 deterministic features fused at fc1 improve accuracy over feature_dim=0 + post-vote rules?

## Setup

| Parameter | v14 (baseline) | v15 (feature-augmented) |
|---|---|---|
| feature_dim | 0 | 32 |
| Architecture | small (32/64/128) | small (32/64/128) |
| Training data | 372k (1500/type x 250) | 372k (1500/type x 250) |
| Epochs | 10 | 10 |
| Seed | 42 | 42 |
| Model size | 390 KB | 406 KB (+16 KB) |
| Hardware | M1 Mac (Metal) | M1 Mac (Metal) |

## Results

### Training Metrics

| Metric | v14 | v15 | Delta |
|---|---|---|---|
| Final loss | 0.3488 | 0.1980 | -0.1508 (better) |
| Final accuracy | 86.62% | 91.61% | **+4.99pp** |

### Eval Metrics

| Metric | v14 + rules | v15 + rules | Delta |
|---|---|---|---|
| Profile label | 178/186 (95.7%) | 175/186 (94.1%) | **-1.6pp** |
| Profile domain | 181/186 (97.3%) | 178/186 (95.7%) | **-1.6pp** |
| Actionability | 232321/232541 (99.9%) | 231948/232317 (99.8%) | -0.1pp |

### Misclassification Comparison

**v14 misclassifications (8):**
- medical_records.height_in: numeric_code vs height
- new_technology.git_sha: hash vs git_sha
- airports.name: region vs full_name (bare "name" ambiguity)
- countries.name: region vs country (bare "name" ambiguity)
- multilingual.name: country vs full_name (bare "name" ambiguity)
- ecommerce_orders_json.total: hs_code vs decimal_number
- tech_systems.server_hostname: docker_ref vs hostname
- (1 additional model-level)

**v15 misclassifications (11):**
- medical_records.height_in: numeric_code vs height (same)
- new_technology.git_sha: hash vs git_sha (same)
- tech_systems.os: **city** vs os (NEW regression)
- api_users_json.name: **city** vs full_name (worse — was region)
- codes_and_ids.credit_card: unix_microseconds vs credit_card_number (NEW)
- ecommerce_orders_json.total: hs_code vs decimal_number (same)
- airports.name: **city** vs full_name (worse — was region)
- countries.name: **city** vs country (worse — was region)
- books_catalog.author: **city** vs full_name (NEW regression)
- weather_stations_json.station_name: **city** vs entity_name (NEW)
- tech_systems.server_hostname: docker_ref vs hostname (same)

### Key Finding: City Attractor

The v15 model develops a **city attractor** — 6 columns are incorrectly predicted as `city` (vs 0 in v14). The 32 features include character statistics (letter ratios, case patterns) that are shared between city names, person names, and other short text strings. The model appears to overfit on these surface-level features, routing ambiguous text toward `city` instead of maintaining the broader distribution across geography/identity types.

The v14 model's "bare name" ambiguity errors (→ region) were at least in the correct domain. The v15 errors are worse — they cross domain boundaries (person names → geography).

## Analysis

### Why Training Accuracy Rose But Eval Fell

The +5pp training accuracy improvement is real but misleading. The features help the model memorise training data patterns more effectively (lower loss, higher accuracy on synthetic data). However, on real-world data:

1. **Synthetic vs real distribution mismatch**: Features like `has_leading_zero`, `digit_ratio`, and `alpha_ratio` are highly discriminative on clean synthetic data but less so on messy real-world values.
2. **Feature correlation with wrong types**: Character-level statistics (the majority of the 32 features) create spurious correlations. Short capitalised strings → city, regardless of semantic content.
3. **Overfitting to feature shortcuts**: The model learns to rely on features as shortcuts instead of learning character-level patterns from the CNN. This makes it less robust to out-of-distribution inputs.

### What About Disabling F1-F3 Rules?

The F1-F3 post-vote rules (leading-zero, slash-segments, digit-ratio) are orthogonal to the city attractor problem. They operate on specific type pairs (postal_code/cpt, docker_ref/hostname, hs_code/decimal_number) and wouldn't fix or worsen the city confusion. Testing v15 without F1-F3 would show the same city problem plus additional regressions on those specific pairs.

## Recommendation

**Do NOT adopt v15 as default.** Revert to v14 (feature_dim=0) + post-vote rules F1-F3.

The feature fusion architecture (NNFT-248) is sound — the regression is a training data problem, not an architecture problem. Future paths to explore:

1. **Feature selection**: Use only the ~10 parse-test features (is_valid_date, is_valid_email, etc.) instead of all 32. Character statistics are the likely culprit.
2. **Feature weighting**: Add a learnable gate or attention over features so the model can down-weight unhelpful features.
3. **More training data**: 1500 samples/type may be insufficient for the model to learn when to trust features vs CNN output. Scaling to 5000+ could help.
4. **Curriculum training**: Start with feature_dim=0 for N epochs, then enable features for fine-tuning.

For now, the post-vote rule approach (feature_dim=0 + F1-F3) remains the better trade-off: simpler, more predictable, and higher eval accuracy.
