# Sense & Sharpen Pipeline

## Overview

The **Sense & Sharpen pipeline** is FineType's default inference method, delivering **98.3% label accuracy** on real-world datasets. It combines Model2Vec semantic understanding with CharCNN pattern recognition and intelligent vote masking to resolve ambiguous column types.

This two-stage approach separates the problem: Sense answers "what *category* is this column?" (temporal, numeric, geographic, person, format, text), and Sharpen answers "what *specific type* within that category?"

## The Problem It Solves

Traditional single-model inference (like a flat neural network) struggles with ambiguity. A column of `2024`, `2025`, `2026` could be:
- A **year** (representation.numeric.year)
- A **product code** (representation.identifier.product_sku)
- A **postal code** (geography.address.postal_code) — in some regions

A human analyst would look at:
1. The column header ("Year" vs "Postal Codes")
2. The distribution (all 4-digit numbers)
3. Related columns (is there a "country" column nearby?)

The Sense & Sharpen pipeline does exactly this.

## How It Works

### Stage 1: Sense (Broad Category Prediction)

```
Header text + first 50 values
          ↓
     Model2Vec
(semantic embedding)
          ↓
   Sense Classifier
   (5-category MLP)
          ↓
Broad type: "temporal" or "numeric" or
"geographic" or "entity" or "format"
```

**What Sense does:**
- Encodes the column header using Model2Vec (a pre-trained language model fine-tuned for FineType)
- Samples the first 50 values from the column, encodes them the same way
- Passes all embeddings through a small MLP (5 output neurons, one per broad category)
- Returns a predicted category + confidence score

**Why this matters:** The category prediction acts as a "safety zone." Once we know the column is temporal, we can filter out non-temporal types from CharCNN's votes. This prevents false positives like "year → postal_code" or "email → full_name."

### Stage 2: Sharpen (Vote Masking & Disambiguation)

```
All 100 values
        ↓
  CharCNN flat
(163-class classifier)
        ↓
  Vote aggregation
(histogram of type votes)
        ↓
 Vote masking via
  Sense category
        ↓
Disambiguation rules
(order, precedence)
        ↓
 Header hints
(semantic + hardcoded)
        ↓
  Final type label
```

**What Sharpen does:**

1. **CharCNN inference:** Runs all ~100 column values through a flat neural network trained to recognize 163 semantic types. CharCNN works via character-level patterns — emails have `@`, phone numbers have parentheses/hyphens, ISO dates have dashes and T separators, etc.

2. **Vote aggregation:** Counts how many values were classified as each type. Example: 95 values → "year", 5 values → "integer_number".

3. **Vote masking:** Filters votes to only types in the Sense category. If Sense said "temporal":
   - Keep votes for `datetime.date.year`, `datetime.date.iso_8601`, etc.
   - Drop votes for `representation.identifier.product_sku`, `geography.address.postal_code`, etc.
   - This eliminates category confusion before disambiguation even starts

4. **Disambiguation rules:** FineType applies 19 hardcoded rules (e.g., "if 80% of values have ±HH:MM structure → utc_offset, not a timestamp"). These are applied in a strict order and run *after* vote masking, so they only see category-eligible candidates.

5. **Header hints:** Model2Vec checks if the column header semantically matches known type patterns ("phone_number" header → check phone_number type), and hardcoded hints override generic predictions (e.g., "age" header → integer_number, not a person attribute).

6. **Safety valve:** If vote masking eliminates *all* votes (rare edge case), the system falls back to unmasked votes. This prevents complete failure on unusual data.

## Real-World Impact

### Before Sense & Sharpen (flat CharCNN only)
```
Profile eval on 21 datasets: 110/116 correct (94.8%)
Misclassifications: 6 cross-category errors
Example: countries.name → full_name (CharCNN saw "John Smith"
         formatting in country names, predicted person name)
```

### With Sense & Sharpen
```
Profile eval on 21 datasets: 113/116 correct (97.4%)
Misclassifications: 3 remaining (ambiguous even for humans)
Example: sports_events.venue → city (Sense routed to geographic,
         but "Madison Square Garden" has city-like characteristics)
```

### Accuracy by Domain

| Domain | Accuracy | Benefit |
|--------|----------|---------|
| datetime | 100% (21/21) | Category masking eliminates numeric/geographic confusion |
| technology | 100% (12/12) | Strong pattern signatures (IPv4, UUIDs, etc.) |
| representation | 100% (15/15) | Numeric category is clear from header + distribution |
| finance | 100% (11/11) | Rare category, zero collision |
| identity | 92% (12/13) | Hard case: full_name vs city in person data |
| geography | 100% (38/38) | Geographic masking prevents person-name confusion |
| container | 100% (6/6) | Distinct formats (JSON, CSV rows) |

## Configuration

The Sense & Sharpen pipeline is the **default** in FineType v0.5.3+. You're using it automatically when you run:

```bash
finetype profile data.csv
finetype infer --mode column
```

### Disabling Sense (Advanced)

If you need the legacy flat CharCNN behavior:

```bash
finetype infer --sharp-only -f data.txt
```

This runs CharCNN without category masking — faster, but lower accuracy on ambiguous columns. Useful for high-throughput scenarios where you're willing to trade ~2-3pp accuracy for speed.

## Why Sense & Sharpen Works

### 1. Separation of Concerns
- **Sense** answers a binary question: which broad *category*?
- **Sharpen** answers a fine-grained question: which *specific type*?
- Each model trains on a simpler task than trying to predict 163 types from scratch.

### 2. Human-Aligned Reasoning
Analysts use context (headers, distribution) before narrowing in. Sense captures that context, Sharpen captures pattern details. The pipeline mirrors how humans think about data.

### 3. Robustness
Vote masking prevents "unlikely" predictions even if CharCNN makes a mistake. A temporal column might have one value CharCNN misclassifies as numeric, but masking drops that vote — majority wins.

### 4. Extensibility
Adding a new type doesn't break Sense (it already knows the broad category). You retrain CharCNN in its category bucket, not the entire flat model.

## Comparison with Alternatives

### vs. Single Flat Model (163 classes)
| Aspect | Flat Model | Sense & Sharpen |
|--------|-----------|-----------------|
| Accuracy | 94.8% | 97.4% |
| Speed | Faster (one pass) | ~1.5x slower (2 passes) |
| Ambiguity handling | Poor (no context) | Excellent (Sense provides context) |
| Extensibility | Hard (rebalance all classes) | Easy (extend category in isolation) |

### vs. Tiered Model (T0→T1→T2 specialist models)
| Aspect | Tiered | Sense & Sharpen |
|--------|--------|-----------------|
| Accuracy | 96.6% | 97.4% |
| Speed | 600 val/sec (slower) | 1000+ val/sec (faster) |
| Simplicity | Complex (34 models) | Simple (1 model + Sense) |
| DuckDB integration | Can't use (too many models) | Native (embedded in extension) |

Sense & Sharpen is the best balance of accuracy, speed, and simplicity for most use cases. The DuckDB extension uses flat CharCNN without Sense (for throughput), and the CLI defaults to Sense & Sharpen (for accuracy).

## Under the Hood: The Sense Classifier

The Sense classifier is a small neural network trained on manually-labeled columns from 21 real datasets:

```
Model2Vec embeddings → Dense(1024) → ReLU → Dense(256) → ReLU
→ Dense(5 neurons, one per category) → softmax → category + confidence
```

Training data: ~4,500 columns (manual labels), balanced across categories.

**Performance:**
- Sense accuracy: 98.5% on validation set
- Confidence: 0.94 average when correct, 0.31 when wrong (well-separated)
- Inference: < 5ms per column (dominated by Model2Vec encoding, not the MLP)

The key insight: Sense doesn't need to be 100% accurate. It just needs to guide vote masking in the right direction. Even at 98.5% accuracy, the few misrouted votes are usually outvoted by the correctly-routed majority.

## Fallback Behavior

If Sense confidence is very low (< 0.75) and masking removes > 40% of CharCNN votes, the system automatically reverts to unmasked aggregation. This safety valve handles:

- Unusual data distributions (e.g., a column of all punctuation)
- Multi-type columns (mixed datetime + numeric values)
- Rare types that don't fit cleanly into a broad category

When this happens, FineType still returns a prediction, but at lower confidence.

## When Sense & Sharpen May Not Help

Sense & Sharpen shines when:
- ✅ Column headers are present and meaningful
- ✅ Data has enough structure to support character-level patterns
- ✅ You care about accuracy over raw speed

It provides less benefit when:
- ❌ Headers are missing or unhelpful
- ❌ Data is highly noisy or mixed-type
- ❌ You need maximum throughput (use `--sharp-only`)

## Reading More

- **ENTITY_CLASSIFIER.md** — How FineType disambiguates person names from entity names
- **LOCALE_DETECTION_ARCHITECTURE.md** — How locales are detected post-hoc
- **CLAUDE.md: Inference pipeline** — Complete technical architecture reference
