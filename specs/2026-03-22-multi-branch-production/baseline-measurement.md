# M-2: Multi-Branch Baseline Measurement

**Date:** 2026-03-22
**Model:** sherlock-v1-flat (28.5k training samples, 250 classes, 10 epochs, flat head)
**Eval:** Tier 1 profile eval (190 format-detectable columns across 30 datasets)
**Host:** Beelink (Linux x86_64, CPU inference)

---

## Headline Results

```
| Metric            | Multi-Branch (sherlock-v1-flat) | CharCNN v16-baseline | CharCNN v14 (production) |
|-------------------|-------------------------------|----------------------|--------------------------|
| Label accuracy    | 27/190 (14.2%)                | 178/190 (93.7%)      | 185/190 (97.4%)          |
| Domain accuracy   | 82/190 (43.2%)                | 182/190 (95.8%)      | 188/190 (98.9%)          |
| Val accuracy      | 60.3%                         | 84.0%                | —                        |
```

The gap between val accuracy (60.3%) and real-world label accuracy (14.2%) is dramatic but explainable — see analysis below.

---

## Why 60.3% Val → 14.2% Real-World

### 1. Eval harness uses exact label matching, not the model's label vocabulary

The eval scoring compares the model's predicted label against ground truth through a schema mapping layer (`eval/schema_mapping.yaml`). The multi-branch model outputs its 250 taxonomy labels, but the schema mapping was built for CharCNN predictions — it maps from CharCNN's output vocabulary (which goes through the Sense→Sharpen pipeline, applying disambiguation rules, header hints, and validation-based corrections).

Multi-branch skips all of that. It outputs raw MLP predictions like `decimal_number`, `numeric_code`, `isbn` — these are correct taxonomy labels but often the wrong *specificity level* for the ground truth annotation. For example:
- Model says `decimal_number` → GT says `integer_number` (correct domain, wrong leaf)
- Model says `isbn` → GT says `uuid` (both are identifier-shaped, model learned the wrong attractor)
- Model says `full_name` → GT says `first_name` (correct category, wrong specificity)

### 2. No Sharpen corrections

The CharCNN pipeline applies 6 disambiguation rules (F1–F6), validation-based elimination, entity demotion, header hints, and locale detection. These boost accuracy significantly — they're the "last mile" that turns 84% CharCNN val accuracy into 97.4% real-world accuracy.

Multi-branch in M-1 runs with **zero post-processing**. Every prediction is a raw argmax from the MLP.

### 3. Attractor collapse

The model collapses many types into a few dominant attractors:

```
| Attractor           | Predictions | Correct | Notes                                    |
|---------------------|-------------|---------|------------------------------------------|
| decimal_number      | 31          | 0       | Absorbs percentages, integers, IPs, etc. |
| numeric_code        | 17          | 0       | Absorbs ISSNs, NPIs, postal codes, etc.  |
| isbn                | 14          | 0       | Absorbs UUIDs, hashes, credit cards, etc. |
| alphanumeric_id     | 13          | 0       | Absorbs IATA, ICAO, SWIFT, VINs, etc.    |
| full_name           | 12          | 0       | Absorbs first/last names, job titles      |
```

These 5 attractors account for 87/190 predictions (46%) with 0% precision. The model has learned to bucket things into broad categories but can't distinguish within them.

### 4. Training data volume

```
| Metric                  | CharCNN v16-baseline    | Multi-Branch            |
|-------------------------|-------------------------|-------------------------|
| Training unit           | Individual values       | Column features         |
| Training samples        | 373,500                 | 28,506                  |
| Samples per class (avg) | 1,494                   | ~114                    |
| Input dimensions        | 97 (char vocab)         | 1,499 (3 streams)      |
| Epochs (trained)        | 10                      | 10                      |
```

13× fewer samples with 15× more input dimensions. Most types had only 15 synthetic samples.

---

## What Multi-Branch Gets Right

Despite 14.2% overall, some patterns emerge:

```
| Type              | Predictions | Correct | Precision | Why it works                        |
|-------------------|-------------|---------|-----------|-------------------------------------|
| gender            | 2           | 2       | 100%      | Distinctive value distribution      |
| color_hsl         | 1           | 1       | 100%      | Unique format → char distribution   |
| geojson           | 1           | 1       | 100%      | Distinctive structure               |
| initials          | 1           | 1       | 100%      | Short uppercase pattern             |
| dmy_hm            | 1           | 1       | 100%      | Timestamp pattern in char stats     |
| country           | 10          | 7       | 70%       | Strong embedding signal             |
| city              | 7           | 3       | 42.9%     | Embedding signal, some confusion    |
| year              | 4           | 2       | 50%       | Numeric pattern recognition         |
| continent         | 3           | 2       | 66.7%     | Embedding signal                    |
| hm_24h            | 3           | 2       | 66.7%     | Time format in char distribution    |
```

Types with **distinctive value distributions** (gender, color_hsl, geojson) or **strong embedding signal** (country, city, continent) work. Types that require **format-level discrimination** (uuid vs isbn vs hash — all hex-ish strings) fail.

---

## Confidence Analysis

The model's confidence is poorly calibrated:

```
| Confidence Range | Predictions | Correct | Accuracy |
|------------------|-------------|---------|----------|
| > 0.50           | 18          | 7       | 38.9%    |
| 0.10 – 0.50      | 50          | 12      | 24.0%    |
| 0.01 – 0.10      | 97          | 7       | 7.2%     |
| < 0.01           | 25          | 1       | 4.0%     |
```

Even high-confidence predictions (>50%) are wrong 61% of the time. The model knows it doesn't know (most predictions are low confidence) but can't distinguish what it does know from what it doesn't.

---

## Gap Analysis: What Needs to Change for 95%

### Problem 1: Data starvation (M-3, M-4, M-5)
Most types had 15 training samples. The model needs ≥1,000 per type to learn discriminative features. The data scaling pipeline (M-3 label remapping + M-4 synthetic generation + M-5 blending to 300k) is the primary lever.

### Problem 2: No post-processing (M-8)
CharCNN gets ~13pp boost from Sharpen rules. Multi-branch will benefit from the same corrections. But this is deferred to M-8 — raw model accuracy needs to be high enough first.

### Problem 3: Attractor collapse
The model needs to learn to distinguish within domains (uuid vs isbn vs hash), not just between domains. This may require:
- More training data per type (M-5)
- Hard negative mining (train on confusing pairs)
- Hierarchical head (M-7) which encodes domain structure

### Problem 4: Format-level types underserved by column features
Types distinguished by character-level format (dates, IPs, UUIDs) may not be well-served by column-level aggregation statistics. The 960-dim char distribution captures *statistical* properties of characters across a column, not *structural* patterns within individual values. This is a potential architectural limitation — worth monitoring as data scales.

---

## Conclusions

1. **14.2% label accuracy is the true baseline.** The 60.3% val accuracy was misleadingly high because the validation set had the same distribution as training data.

2. **The gap to 95% is 81pp.** This is large but the primary bottleneck is clear: data volume. The model saw 28.5k samples across 250 classes — most types had only 15 examples.

3. **The data scaling pipeline (M-3→M-5) is the highest-leverage work.** Label remapping recovers real-world signal, synthetic column generation provides volume, and blend-30-70 balances them.

4. **Sharpen integration (M-8) will provide an additional boost** but should wait until raw model accuracy is higher — applying rules to a 14% accurate model won't help much.

5. **The 95% target remains ambitious but not ruled out.** CharCNN achieved 84% val accuracy on 373k samples and 97.4% real-world with Sharpen. If multi-branch can reach similar val accuracy at scale, plus Sharpen corrections, 95% is plausible.
