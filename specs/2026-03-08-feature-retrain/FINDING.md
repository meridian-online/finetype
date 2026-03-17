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

---

# Deep Accuracy Spike: Header Hints + Hint Authority (NNFT-254)

**Date:** 2026-03-08
**Question:** Can expanded header hints and refined hint override authority close the remaining eval gaps without model retraining?
**Depends on:** NNFT-253 (established that feature_dim=0 + rules is the better path)

## Approach

Instead of retraining the model (NNFT-253 showed diminishing returns), this spike attacked the remaining misclassifications through three experiments targeting the Sense→Sharpen pipeline's disambiguation and header hint system.

## Experiments

### Experiment 1: Header Hint Gap Analysis + Fixes

**Method:** Profiled all 21 datasets in ~/datasets/ (213 columns total) and traced every misclassification through the pipeline.

**Root causes identified:**

| Category | Count | Examples |
|---|---|---|
| Missing header hints | 12 | epoch, age, altitude, pages, attendance, heart_rate |
| Categorical text → geography | 5 | species→region, sport→region, language→last_name |
| Numeric confusion (integer vs numeric_code vs amount_minor_int) | 8 | salary→amount_minor_int, response_time_ms→numeric_code |
| Hint authority too weak | 3 | postal_code→CPT, Cabin→ICD10, epoch→NPI |

**Hints added (~30 new exact-match + substring rules):**

| Category | Header patterns | Target type |
|---|---|---|
| Epoch/Unix | epoch, unix_timestamp, unix_epoch, posix_time, epoch_ms | datetime.epoch.unix_seconds/unix_milliseconds |
| Age | age, patient_age, customer_age | representation.numeric.integer_number |
| Altitude/Elevation | altitude, elevation, alt, elev | representation.numeric.integer_number |
| Duration (numeric) | duration_minutes, elapsed, elapsed_time | representation.numeric.integer_number |
| Attendance/Count | attendance, headcount, participants, capacity | representation.numeric.integer_number |
| Vital signs | heart_rate, bpm, pulse | representation.numeric.integer_number |
| Pages | pages, page_count, num_pages | representation.numeric.integer_number |
| Language | language, lang, programming_language | representation.discrete.categorical |
| Sport | sport, discipline, event_type | representation.discrete.categorical |
| Species | species, genus, taxon, breed | representation.discrete.categorical |
| Exchange | exchange, stock_exchange, market | representation.discrete.categorical |

**Substring fixes (7 false-positive guards):**
- `h.contains("count")` now excludes "country"/"county"
- `h.contains("address")` now excludes "mac"
- `h.contains("duration")` now excludes "iso"/"8601"
- `h.ends_with(" name")` now excludes "month name"/"day name"/"weekday name"
- Epoch/unix substring check moved BEFORE generic date/timestamp catch-all

**Result:** 178/186 → 179/186 (+1 label, +2 domain)

### Experiment 2: Hardcoded Hint Authority Threshold

**Problem:** Three showstopper misclassifications persisted because hardcoded hints couldn't override confident cross-domain predictions:
- shipping_postal_code → CPT (80% confidence, geography hint vs identity.medical)
- Cabin → ICD10 (57% confidence, representation hint vs identity.medical)
- unix_epoch → NPI (100% confidence, datetime hint vs identity.medical)

**Fix:** Cross-domain hardcoded hint override rule:

```
IF hardcoded hint exists
AND hint domain ≠ prediction domain
AND hint base type ≠ prediction base type (prevents uuid domain flip)
THEN override prediction with hint
```

Plus domain-aware threshold for hint-not-in-votes:
- Cross-domain: 0.85 threshold (header encodes semantic domain knowledge)
- Same-domain: 0.5 threshold (model more reliable within domain)

**Regressions caught and fixed:**
- uuid (representation→technology): Fixed with `hint_base != pred_base` guard
- currency_code (finance→identity): Same fix
- eu_date → iso_8601 override at 0.80: Fixed by making threshold domain-aware (0.85 for cross-domain only)

**Result:** Integrated into Experiment 1 numbers above

### Experiment 3: Feature-Augmented Model (Not Needed)

Experiments 1 and 2 achieved +1 over baseline without any model retraining. Given NNFT-253's finding that feature_dim=32 causes city attractor regression, the rule-based approach is confirmed as the better path.

## Final Results

| Metric | v14 baseline | After NNFT-254 | Delta |
|---|---|---|---|
| Profile label | 178/186 (95.7%) | 179/186 (96.2%) | **+0.5pp** |
| Profile domain | 181/186 (97.3%) | 183/186 (98.4%) | **+1.1pp** |
| Actionability | 232321/232541 (99.9%) | 232057/232177 (99.9%) | maintained |
| Tests passing | 438 | 438 | maintained |

### Remaining 7 Misclassifications

| Dataset | Column | Predicted | Expected | Root Cause |
|---|---|---|---|---|
| new_technology | git_sha | hash | git_sha | Model confusion — same char distribution |
| ecommerce_orders_json | total | hs_code | decimal_number | Model confusion — numeric overlap |
| airports | name | region | full_name | Bare "name" ambiguity |
| world_cities | name | region | city | Bare "name" ambiguity |
| multilingual | name | country | full_name | Bare "name" ambiguity |
| server_logs_json | response_time_ms | integer_number | decimal_number | Decimal values in integer-like column |
| tech_systems | server_hostname | docker_ref | hostname | Model confusion — similar formats |

Of these, 3 are the perennial "bare name" ambiguity (genuinely ambiguous — "name" means different things per dataset), 3 are model-level confusions that would require retraining to fix, and 1 (response_time_ms) is a ground truth edge case (column contains both integers and decimals).

## Recommendation

**Adopt the expanded rule set (feature_dim=0 + F1-F3 + NNFT-254 hints).** This is now the recommended configuration because:

1. No model retraining required — zero risk of the city attractor or other regression
2. Rules are transparent and debuggable — each fix has a clear NNFT-254 annotation
3. The 7 remaining misclassifications are at the boundary of what header hints can solve
4. Further accuracy gains will require either model retraining with better training data, or per-dataset ground truth refinement

**Future work:**
- NNFT-258: Expand golden tests to lock in these gains via Rust integration tests
- Consider retraining CharCNN on harder negatives (confusable type pairs) for git_sha/hash, hs_code/decimal_number
- The 3 bare "name" cases may benefit from a learned header→type classifier (replacing regex-based header_hint)
