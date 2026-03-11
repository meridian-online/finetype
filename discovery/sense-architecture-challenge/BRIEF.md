# Research Brief: FineType Column-Type Inference Architecture Challenge

**Date:** 2026-03-08
**Interview session:** interview_20260308_111418
**Status:** Ready for execution
**Scope:** Column-type detection literature only (not adjacent domains)

## Background

FineType is a Rust/Candle type inference engine with 250 types across 7 domains, deployed as a local CLI tool. The current pipeline — Sense (6-category cross-attention over Model2Vec) → CharCNN (250-class flat classifier) → masked vote aggregation → disambiguation rules → header hints — achieves 179/186 (96.2%) on profile eval but has quality issues on real-world data.

**Remaining errors (7):**
- 3× bare-name ambiguity (need sibling context: airports.name, world_cities.name, multilingual.name)
- 3× model confusions between visually similar types (git_sha/hash, hs_code/decimal_number, docker_ref/hostname)
- 1× GT edge case (response_time_ms integer vs decimal)

**Key constraint:** Rules currently outperform learned features (NNFT-253 showed feature_dim=32 CharCNN causes city attractor regression). Training data is 100% synthetic.

## Design Decisions from Interview

| Dimension | Decision |
|---|---|
| **Taxonomy** | 250 types, non-negotiable differentiator |
| **Model size** | 10–50 MB acceptable |
| **Priority** | Accuracy first, speed close second |
| **Deployment** | Local CLI, not cloud |
| **Context modes** | Full-table AND single-column, graceful degradation required |
| **Training toolchain** | Evaluate all frameworks, annotate Candle feasibility vs. PyTorch-export |
| **Embedding backbone** | Model2Vec replaceable — evaluate alternatives |
| **Rules** | Classify each as permanent domain knowledge vs. subsumable patch |
| **Architecture bias** | Ambitious rethinks first, incremental fallbacks last |
| **Comparison method** | Raw published numbers + qualitative architectural analysis, no taxonomy normalisation |
| **Success metric** | Eliminate failure categories (bare-name ambiguity, visually-similar-type confusion) |
| **Multilingual** | Must address 50+ postal locales, 45+ phone locales, 700+ CLDR date/time locales |
| **LLM scope** | Include as ceiling baseline AND explore local LLM paths (Phi-3, Llama-3-8B) |
| **Workflow** | Agents decide execution order, no forced sequencing |

---

## Section 1: Literature Deep-Dive Targets

### Extraction Template

For each paper, extract:

| Field | Description |
|---|---|
| `paper_title` | Full title |
| `authors` | First author + et al. |
| `year` | Publication year |
| `venue` | Conference/journal |
| `doi_or_url` | DOI or arXiv URL |
| `architecture` | Model architecture (e.g., multi-input DNN, BERT fine-tune, CNN, hybrid) |
| `type_count` | Number of types classified |
| `type_taxonomy` | Source of types (DBpedia, schema.org, custom) |
| `dataset` | Training/eval datasets used |
| `dataset_size` | Number of columns/tables |
| `reported_f1` | Best reported F1 (macro/weighted, specify which) |
| `context_mechanism` | How sibling/table context is used (none, LDA, serialised input, cross-attention) |
| `context_degradation` | Can it operate on single columns? How does performance change? |
| `model_size` | Parameters or disk size |
| `embedding_backbone` | What pretrained representations are used |
| `multilingual_support` | Handles non-English data? How? |
| `training_framework` | PyTorch, TensorFlow, etc. |
| `candle_feasibility` | Low / Medium / High — estimated effort to implement in Rust/Candle |
| `key_insight` | The transferable architectural idea |
| `limitations` | Known weaknesses or failure modes |

### Target Papers and Search Queries

| Paper | Search Query | Known Reference |
|---|---|---|
| Sherlock (2019) | `"Sherlock" "semantic type detection" Hulsebos site:arxiv.org` | arXiv:1905.10688 |
| Sato (2020) | `"Sato" "context-aware semantic type detection" Zhang site:arxiv.org` | arXiv:2005.11688 |
| DODUO (2022) | `"DODUO" "column type annotation" Suhara site:arxiv.org` | arXiv:2104.01785 |
| ColNet | `"ColNet" column type classification neural network` | Search required |
| SOTAB benchmark | `"SOTAB" "semantic column type" benchmark WebDataCommons` | Search required |
| Arctype / LLM-based | `"column type detection" LLM GPT "schema inference" site:arxiv.org` | Search required |
| TURL (2020) | `"TURL" "table understanding" "pre-training" Deng site:arxiv.org` | arXiv:2006.14806 |
| TaBERT (2020) | `"TaBERT" "pre-training" "tabular data" Yin site:arxiv.org` | arXiv:2005.08314 |
| DODUO follow-ups | `"column annotation" transformer 2023 2024 site:arxiv.org` | Search required |
| Byte-level models for tabular | `"byte-level" OR "character-level" "column type" OR "schema inference"` | Search required |
| Local LLM classification | `"Phi-3" OR "Llama" "type detection" OR "schema inference" quantized local` | Search required |
| Multilingual column typing | `"multilingual" "column type" OR "semantic type" detection` | Search required |

### Additional Search Queries

- `"column type prediction" survey 2023 2024`
- `"semantic type detection" benchmark comparison`
- `"knowledge distillation" "column type" OR "table understanding"`
- `"GitTables" column annotation`
- `site:huggingface.co column type detection model`

---

## Section 2: Transferable Architectural Patterns

Rank by ambition (most ambitious first).

### Tier 1 — End-to-End Replacements

1. **Serialised-table transformer** (DODUO-style): Serialise column as `[CLS] header [SEP] val1 [SEP] val2 ...`, fine-tune pretrained LM for 250-class classification. Evaluate: Can this work with byte-level tokenisation (ByT5/Charformer) to unify character and semantic understanding? How does it scale to 250 classes?

2. **Local LLM classification backend**: Phi-3-mini / Llama-3-8B-Q4 as classifier — prompt with column samples, constrained decoding to type taxonomy. Evaluate: latency per column, accuracy vs. dedicated models, distillation potential.

3. **Byte-level unified model**: Replace both Model2Vec and CharCNN with a single byte/character-level encoder (ByT5, Charformer, CANINE) that processes raw column bytes end-to-end. Evaluate: does unifying character-level and semantic-level eliminate the Sense→CharCNN split entirely?

### Tier 2 — Hybrid Enhancements

4. **Sibling-context attention**: Add cross-column attention layer that conditions type prediction on other columns in the table (Sato's LDA approach, or learned attention). Must degrade gracefully to single-column mode (attention over self only).

5. **Alternative embedding backbone**: Replace Model2Vec with tabular-pretrained embeddings (from TURL, TaBERT pretraining objectives) or multilingual byte-level models. Evaluate multilingual coverage.

6. **Multi-signal Sherlock-style features + learned fusion**: Expand deterministic features beyond current 32 to Sherlock's ~1588 (character distributions, word embeddings, paragraph vectors, global stats), fuse with learned representations rather than using them as CharCNN input.

### Tier 3 — Incremental Fallbacks

7. **Hierarchical classification**: Replace flat 250-class with learned hierarchy (7 domains → subcategories → types). Could subsume Sense's 6-category role with a learned tree.

8. **Contrastive pre-training on column data**: Self-supervised pretraining on unlabelled columns (columns from same table = positive pairs) to learn better representations before fine-tuning.

9. **Improved vote aggregation**: Replace majority vote with learned aggregation (attention-weighted votes, confidence-based weighting).

### Per-Pattern Extraction

For each pattern, agents should extract:
- Which failure categories it addresses (bare-name ambiguity, visually-similar-type confusion, or both)
- Whether it handles multilingual data natively or needs adaptation
- Candle feasibility (Low/Medium/High)
- Estimated model size
- Whether it subsumes existing rules or requires them

---

## Section 3: Dataset Recommendations

### Labeled Datasets (for fine-tuning and eval benchmarking)

| Dataset | Search Query / URL | Purpose | Expected Format |
|---|---|---|---|
| VizNet | `"VizNet" dataset Plotly site:github.com` | 31M+ real-world values; fine-tuning + eval | CSV columns with type labels |
| GitTables | `"GitTables" dataset site:zenodo.org OR site:github.com` | 1M+ CSV tables from GitHub; fine-tuning + eval | Parquet with schema annotations |
| SOTAB | `"SOTAB" WebDataCommons benchmark download` | Web table columns with schema.org types; secondary benchmark | CSV with semantic type labels |
| WikiTables | `"WikiTables" dataset "column type" download` | Wikipedia tables; eval diversity | Varies |
| Sherlock dataset | `"Sherlock" dataset "VizNet" download site:github.com` | 78-type labelled columns; baseline comparison | JSON/CSV |
| T2Dv2 | `"T2Dv2" "Web Data Commons" table dataset` | Web tables matched to DBpedia; entity type eval | CSV + annotations |

### Unlabeled Datasets (for representation pretraining)

| Dataset | Search Query / URL | Purpose |
|---|---|---|
| Kaggle Datasets | `site:kaggle.com/datasets CSV` | Diverse real-world CSVs for contrastive pretraining |
| data.gov | `site:data.gov CSV download` | Government open data; locale/format diversity |
| GitHub CSV corpus | `"GitHub" CSV corpus large-scale` | Raw CSVs for self-supervised learning |
| Common Crawl tables | `"Common Crawl" "web tables" extraction` | Massive unlabelled web table data |

### Multilingual-Specific Datasets

| Dataset | Search Query | Purpose |
|---|---|---|
| Multilingual web tables | `"multilingual" "web tables" dataset` | Non-English column data |
| CLDR test data | `site:unicode.org CLDR test data download` | Locale-specific date/number/phone formats |
| International address datasets | `"international address" dataset "postal code" multilingual` | Postal locale coverage |

### Per-Dataset Extraction

For each dataset, agents should extract:
- Download URL / access method
- Size (rows, columns, tables)
- Type taxonomy used (if labelled)
- Mappability to FineType's 250-type taxonomy (High/Medium/Low/None)
- Language/locale coverage
- Licence

---

## Section 4: Rules Classification Template

For each existing rule in FineType's pipeline, classify as `domain_knowledge` (facts about the world, better expressed declaratively — should survive any architecture change) or `model_patch` (compensates for model weakness — should be subsumed by a better model).

| Rule | Location | Description | Classification | Rationale |
|---|---|---|---|---|
| F1 (leading-zero → numeric_code) | column.rs | Upgrades postal_code/cpt to numeric_code when leading zeros present | `domain_knowledge` | Leading-zero preservation is a data engineering fact |
| F2 (slash-segments → docker_ref) | column.rs | Disambiguates by slash segment count | | |
| F3 (digit-ratio+dots → hs_code) | column.rs | Disambiguates hs_code from decimal_number | | |
| Rule 14 (Duration override) | column.rs | SEDOL + P-prefix → duration | | |
| Rule 15 (Attractor demotion) | column.rs | Three-signal demotion for attractor types | | |
| Rule 16 (Text length demotion) | column.rs | full_address + long median → sentence | | |
| Rule 17 (UTC offset override) | column.rs | `[+-]HH:MM` ≥80% → utc_offset | | |
| Rule 18 (Entity demotion) | column.rs | full_name + non-person entity → entity_name | | |
| Rule 19 (Percentage without %) | column.rs | percentage winner + no % sign → decimal_number | | |
| Validation-based elimination | column.rs | Eliminates candidates where >50% fail validation | | |
| Header hints (hardcoded) | column.rs | ~30+ exact-match + substring header rules | | |
| Header hints (Model2Vec) | semantic.rs | Semantic similarity matching at 0.65 threshold | | |
| Geography rescue | column.rs | Unmasked vote check for location types | | |
| Locale detection | column.rs | Post-hoc locale detection via validation_by_locale | | |
| Leading-zero pivot (proposed) | — | numeric_code → integer_number when no leading zeros | `domain_knowledge` | Per discovery/numeric-code-leading-zero |

**For each architectural candidate from Section 2, agents should note which `model_patch` rules it would eliminate.**

---

## Section 5: Evaluation Methodology

### 5a. Real-World Benchmark Construction

| Source | Search Query | Selection Criteria |
|---|---|---|
| Kaggle popular CSVs | `site:kaggle.com/datasets popular CSV` | Top-voted datasets with diverse column types |
| data.gov | `site:data.gov CSV` | Government data with standard formats |
| GitHub trending CSVs | `GitHub CSV dataset stars:>100` | Community-vetted real data |
| Awesome Public Datasets | `github.com/awesomedata/awesome-public-datasets` | Curated list |

**Target: 500+ columns** across diverse domains, manually labelled with FineType's 250-type taxonomy.

### 5b. Failure-Category Test Suites

| Suite | Description | Minimum Size |
|---|---|---|
| `bare_name_ambiguity` | Columns where header is ambiguous (e.g., "name") and only values + siblings disambiguate | 50 columns |
| `visually_similar_types` | Pairs of types with overlapping character distributions (git_sha/hash, hs_code/decimal_number, docker_ref/hostname) + new pairs | 30 column pairs |
| `multilingual` | Non-English columns across postal, phone, date/time, name types for multiple locales | 100 columns |
| `edge_cases` | GT disputes, multi-type columns, empty/null-heavy columns | 30 columns |

### 5c. Secondary Benchmarks

- Run FineType on SOTAB benchmark (after mapping types) for a published-comparable number
- Run on Sherlock's test set (after mapping types) for another reference point

### 5d. Metrics

- Per-failure-category elimination rate (primary)
- Macro F1 across all 250 types (secondary)
- Per-domain accuracy breakdown (7 domains)
- Confusion matrix focused on known confusable pairs
- Multilingual accuracy by locale group

---

## Section 6: Prioritised Roadmap

Ordered by expected payoff per failure category eliminated:

| Priority | Approach | Failure Categories Addressed | Expected Payoff | Effort | Candle Feasibility |
|---|---|---|---|---|---|
| 1 | **Byte-level unified model** (ByT5/Charformer replacing Model2Vec + CharCNN) | Both (character-level + semantic in one model; multilingual by default) | High — eliminates architectural split | High | Medium (ONNX export) |
| 2 | **Sibling-context attention** with graceful degradation | Bare-name ambiguity (primary) | High — directly addresses 3/7 errors | Medium | High (Candle attention) |
| 3 | **Local LLM distillation** (GPT-4/Claude as teacher → small student model) | Both (LLM world knowledge for disambiguation) | Medium-High — ceiling validation + practical path | Medium | High (student is small) |
| 4 | **Serialised-column transformer** (DODUO-style, fine-tuned on column sequences) | Visually-similar-type confusion | Medium — proven, scaling to 250 untested | High | Low (PyTorch + ONNX) |
| 5 | **Expanded Sherlock-style features** + learned fusion | Visually-similar-type confusion | Medium — augments existing pipeline | Low | High (Rust features) |
| 6 | **Contrastive pretraining** on unlabelled columns | Both (better representations overall) | Medium — indirect, improves foundation | Medium | Medium |
| 7 | **Hierarchical classification** (learned tree replacing Sense) | Both (finer intermediate categories) | Medium-Low — evolutionary improvement | Low-Medium | High |
| 8 | **Real-world data augmentation** (GitTables/VizNet fine-tuning) | Both (synthetic distribution mismatch) | Medium — prerequisite for other improvements | Low | N/A (data) |

**Cross-cutting:** Multilingual evaluation should be applied to every candidate. Byte-level models (P1) and LLM approaches (P3) have inherent multilingual advantages. Subword models (P4) depend on tokeniser language coverage.

---

## Execution Notes for Agents

- Execute Section 5 (eval methodology) early — better eval is prerequisite for measuring architectural improvements
- Section 1 (literature) and Section 3 (datasets) can execute in parallel
- Section 2 (architectural patterns) should be refined after Section 1 findings
- Section 4 (rules classification) requires codebase access — read `column.rs`, `features.rs`, `semantic.rs`
- Section 6 (roadmap) should be updated after all other sections complete
- All web searches should capture access dates; papers should be archived as PDFs where possible
- For each paper found, check for associated code repositories (GitHub links) and note implementation language
