# Finding: Column-Level Value Embeddings for Entity Type Disambiguation

**Task:** NNFT-150
**Date:** 2026-02-27
**Author:** @nightingale
**Status:** Complete

## Question

Can Model2Vec value embeddings, aggregated per column, separate entity types (person vs place vs organization vs creative work) well enough for production disambiguation?

## Method

Tested on 2,911 SOTAB validation columns with Schema.org entity type labels, grouped into 4 categories:

| Category | SOTAB Labels | Columns |
|---|---|---|
| person | Person, Person/name, MusicArtistAT | 816 |
| place | Place, addressLocality | 719 |
| organization | Organization, LocalBusiness/name | 647 |
| creative_work | MusicAlbum, MusicRecording/name, Event/name, CreativeWork | 729 |

Each column has up to 20 sampled values (extracted by `prepare_values.py`).

**Embedding pipeline:** Embed all values with `minishlab/potion-base-4M` (128-dim, same model FineType uses for header hints) → mean-aggregate per column → one 128-dim vector per column.

**Statistical features:** 20 hand-crafted features per column (length stats, word counts, character patterns, org suffix detection, title case ratio, uniqueness, etc.).

## Results

### Accuracy (4-class, 70/30 stratified split, 5 runs)

| Features | kNN (k=11) | Random Forest | Logistic Regression |
|---|---|---|---|
| Embeddings only (128-dim) | **73.2%** ± 1.3 | 71.3% ± 1.2 | 69.1% ± 1.3 |
| Statistical only (20-dim) | 50.7% ± 1.8 | 63.8% ± 1.1 | 53.2% ± 1.6 |
| Combined (148-dim) | 49.4% ± 2.1 | **73.6%** ± 0.9 | 71.9% ± 1.3 |

### Clustering quality

| Experiment | Silhouette (cosine) | Interpretation |
|---|---|---|
| 5-class (with music_artist separate) | 0.032 | Very weak |
| 4-class (music_artist → person) | 0.037 | Very weak |
| Binary (person vs non-person) | 0.035 | Very weak |

### Per-class performance (best model: RF on combined features, 73.6%)

| Category | Precision | Recall | F1 |
|---|---|---|---|
| creative_work | 81.0% | 82.9% | 81.9% |
| place | 78.4% | 70.0% | 74.0% |
| person | 67.8% | 80.5% | 73.6% |
| organization | 68.2% | 58.4% | 62.9% |

### Binary: person vs non-person (kNN k=5)

| Metric | Value |
|---|---|
| Accuracy | 84.9% |
| Person precision | 78.2% |
| Person recall | 63.8% |
| Non-person FP rate | 21.1% |

### Key structural observations

**Within-class spread >> between-class distance:**

| Category | Spread | Nearest centroid distance |
|---|---|---|
| person | 0.739 | 0.214 (to place) |
| organization | 0.715 | 0.206 (to place) |
| place | 0.679 | 0.206 (to organization) |
| creative_work | 0.545 | 0.217 (to person) |

Classes massively overlap in embedding space. The spread within each class is 3–4× larger than the distance between class centroids.

### Most important statistical features (RF importance)

| Feature | Importance | Notes |
|---|---|---|
| uniqueness | 12.9% | Cardinality / n — places repeat more |
| word_density | 10.3% | Words per character — creative works are wordy |
| mean_length | 8.7% | Creative works longer, person names shorter |
| std_words | 7.4% | Creative works have variable word counts |
| mean_cap_words | 7.1% | Capitalization patterns differ |
| org_suffix | 4.0% | Inc/Ltd/Corp — specific but sparse signal |
| person_title | 0.6% | Mr/Mrs/Dr — almost useless (rarely present) |

## Interpretation

### The signal exists, but it's weak

73% accuracy on 4 classes (vs 25% random baseline) proves that column-level value distributions **do** carry entity-type signal. This is meaningful. But:

- **Silhouette scores of 0.03** indicate massive class overlap — the clusters are not cleanly separable.
- **Organization is the hardest** (63% F1) — org names and person names share length, capitalization, and word structure.
- **Creative works are the easiest** (82% F1) — distinctive patterns (longer titles, "The" prefix, numbers, mixed punctuation).
- Even the **binary person vs non-person** task only gets 85% with 21% false positives — not reliable enough for a disambiguation rule.

### Embeddings carry more signal than surface features

Embeddings alone (73.2% kNN) beat statistical features alone (63.8% RF). The semantic content of the values matters more than their surface structure. However, combining them only gives marginal improvement (73.6% RF) — the features are somewhat redundant.

### Why off-the-shelf embeddings fall short

`potion-base-4M` is a general-purpose word embedding model. It captures word-level semantics but wasn't trained to distinguish "is this word a person name or a place name." "London" and "Paris" may embed near each other (both proper nouns, both geographic) but the model has no incentive to separate them from "Johnson" and "Williams" (also proper nouns, also high-frequency).

A model **fine-tuned on entity type classification** would learn to extract the right features — e.g., that columns of geographic names have higher overlap with a known city lexicon, or that person name columns have specific first-name/last-name distribution patterns.

## Recommendation

### Option C (embedding similarity alone) is insufficient

73% accuracy with 0.03 silhouette is not production quality. An analyst seeing ~27% of entity columns misclassified would lose trust.

### Option A (post-vote column-level classifier) is the right path

The evidence supports building a **trained column-level model** that operates after the CharCNN vote:

1. **Signal exists** — 73% from off-the-shelf tools proves column distributions carry entity-type information. A purpose-trained model should do significantly better.
2. **Training data exists** — SOTAB provides ~3k labelled entity columns across 4 categories. GitTables adds more.
3. **The model should combine signals** — both semantic embeddings and distributional features matter. A small neural network or gradient-boosted model on combined features is the natural architecture.
4. **Surgical insertion** — only fires when the CharCNN vote is ambiguous (full_name/entity_name/last_name competing), so it adds minimal latency for the 90%+ of columns that aren't entity-name-ambiguous.

### Option B (replace T2 person node) is premature

The data doesn't justify changing the fundamental inference contract yet. Option A can be prototyped and evaluated independently, then promoted to Option B if the accuracy is strong enough.

### Suggested next steps

1. **Train a column-level entity classifier** on SOTAB entity columns, using both embeddings and statistical features as input
2. **Evaluate on held-out SOTAB test split** — target >85% on 4-class to be production-useful
3. **Wire as post-vote disambiguation** — only activates when vote distribution is ambiguous between person/entity name types
4. **Expand training data** — GitTables has additional labelled columns; synthetic generation could boost underrepresented categories

## Raw data

- Spike scripts: `discovery/entity-disambiguation/embedding_spike.py`, `embedding_spike_extended.py`
- SOTAB source: `~/datasets/sotab/cta/validation/column_values.parquet`
- Model: `minishlab/potion-base-4M` (128-dim, same as FineType's Model2Vec)
