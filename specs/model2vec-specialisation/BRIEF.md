# Discovery: Model2Vec Specialisation for Column Name Classification

## Problem

FineType's Model2Vec semantic hint system uses `minishlab/potion-base-4M` — a general-purpose pre-distilled English embedding model — essentially off-the-shelf. We "plugged it in" with a synonym-based type embedding scheme and a 0.70 cosine similarity threshold. It works well for unambiguous column names (`email`, `phone_number`, `gender`, `country`) but misses domain-specific terms and falls below threshold for many real-world column names.

### What we're leaving on the table

The current system has three gaps:

**1. Vocabulary gap:** potion-base-4M was distilled from general English text. Analytics/database column naming conventions use specific vocabulary — `amt`, `qty`, `dt`, `ts`, `desc`, `cat`, `num`, `val`, `pct`, `yr`, `mo` — that may not be well-represented in the base model's token space.

**2. Synonym coverage gap:** Type embeddings are built from: taxonomy title, aliases, label components, and hardcoded header_hint entries. This is ~167 entries covering ~169 types — roughly 1 synonym per type on average. Real-world column names are vastly more diverse: `annual_pay`, `compensation`, `wages`, `remuneration` all mean salary. `zipcode`, `zip`, `postcode`, `plz` all mean postal_code.

**3. Threshold gap:** The 0.70 threshold was tuned to avoid false positives on a set of ~30 test names. This is conservative — it means many correct matches just below 0.70 are discarded. A more nuanced approach (per-domain thresholds, or confidence calibration) could recover these without introducing false positives.

### Driving example: salary

`people_directory.salary` is predicted as `postal_code@0.91` by the CharCNN. The Model2Vec semantic hint *should* catch this — "salary" is clearly a semantic match for a price/currency/number type. But currently:

- Does "salary" hit above 0.70 for any type? If yes, which type? If no, why not?
- Does any type have "salary" in its synonym list? (Likely not — it's not a header_hint entry, not a taxonomy title, not an alias.)
- Would distilling a custom vocabulary (with analytics terms like salary, wages, revenue, profit, cost, etc.) improve the match?

## Current Architecture

```
Column name → Tokenize (wordpiece) → Index embeddings → Mean pool → L2 normalize
                                                                        ↓
Pre-computed type embeddings ← Synonym texts → Model2Vec encode → Mean pool → L2 normalize
                                                                        ↓
Cosine similarity → Best match above 0.70 threshold → Override if prediction is generic
```

**Key files:**
- `scripts/prepare_model2vec.py` — Builds type embeddings from taxonomy + header hints
- `crates/finetype-model/src/semantic.rs` — Runtime inference (tokenize → embed → cosine)
- `models/model2vec/` — Artifacts (tokenizer, embeddings, type_embeddings, label_index)

**Current stats:**
- Model: potion-base-4M (7.4MB float16, 4M token vocabulary, 256-dim embeddings)
- Type embeddings: 169 types, each from mean of ~1 synonym embedding
- Threshold: 0.70 (zero false positives on 30-name test set)
- Calibrated true positive range: 0.771 (user_email) to 0.907 (gender)

## Proposed Investigation Areas

### 1. Synonym expansion

The lowest-cost, highest-impact improvement. Expand the synonym lists used to build type embeddings:

- **Mine real-world column names:** GitTables (2.7M columns), SOTAB (16k columns), Kaggle datasets, public schema registries
- **Domain-specific aliases:** Analytics terms (amt, qty, pct), database conventions (dt, ts, desc), business terms (salary, revenue, margin)
- **Abbreviation expansion:** Map common abbreviations to their full forms
- **Target:** 5-10 synonyms per type instead of ~1

### 2. Custom vocabulary distillation

Model2Vec supports distilling with a custom vocabulary. Instead of the general potion-base-4M vocab, distill from a sentence transformer (e.g., MiniLM-L6-v2) using a vocabulary curated for column name classification:

- Database column naming conventions (snake_case, camelCase, abbreviations)
- Type label vocabulary (all taxonomy labels, titles, aliases)
- Domain terms (financial, medical, geographic, technical)
- Common prefixes/suffixes (is_, has_, _id, _code, _date, _name, _type)

This produces a Model2Vec with the same architecture and speed but token embeddings optimised for our domain.

### 3. Threshold refinement

Move from a single 0.70 threshold to:
- **Per-domain thresholds:** Geography types might need 0.65 (more ambiguous names like "region"), identity types might be fine at 0.75
- **Confidence calibration:** Convert cosine similarity to a calibrated probability using the known TP/TN distribution
- **Relative ranking:** Instead of absolute threshold, compare top-1 vs top-2 similarity — a large gap indicates confidence

### 4. Training data for type embedding fine-tuning

Rather than mean-pooling synonym embeddings, train a lightweight mapping (linear projection or single-layer MLP) from raw Model2Vec column name embeddings to type labels. This requires labelled data:
- GitTables has schema annotations for some tables
- Profile eval has 209 annotated columns across 20 datasets
- Could synthetically generate column names per type (from taxonomy descriptions + LLM augmentation)

## Open Questions

1. **What's the current similarity distribution for all 209 profile eval column names?** This tells us exactly how many correct matches fall below 0.70 and could be recovered.

2. **Is synonym expansion sufficient, or do we need custom distillation?** If "salary" simply isn't in the vocabulary, no amount of synonym expansion will help — we need the token embeddings themselves to understand the word.

3. **What's the false positive risk of lowering the threshold?** The 0.70 was tuned on 30 names — is this enough? Need a larger test set of truly generic column names (col1, col2, x, data, value, field1, etc.).

4. **Does distilling from a multilingual base model (paraphrase-multilingual-MiniLM-L12-v2) improve non-English column name handling?** Current model is English-only.

5. **What's the right balance between synonym expansion and custom distillation?** Synonyms are fast to iterate on; distillation requires Python + a 30-second process + rebuilding type embeddings.

## Success Criteria

- Model2Vec correctly identifies "salary" as price/number-related (above threshold)
- Coverage of profile eval columns at threshold improves (measure: how many of 209 columns get a correct semantic hint?)
- No increase in false positives on generic column names
- Process for expanding synonyms is documented and repeatable (not one-time manual effort)
