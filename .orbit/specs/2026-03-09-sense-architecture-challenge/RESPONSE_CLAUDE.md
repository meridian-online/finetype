# FineType column-type inference architecture research brief

**FineType's 96.2% profile eval accuracy masks real-world weaknesses that demand architectural intervention.** The 7 remaining errors — 3 bare-name ambiguity cases requiring sibling context, 3 visually-similar-type confusions (git_sha/hash, hs_code/decimal_number, docker_ref/hostname), and 1 ground-truth edge case — align precisely with failure modes documented across a decade of column-type annotation literature. This brief synthesizes findings from 15+ systems, 13 datasets, and 9 architectural patterns to chart a path toward eliminating these failures while maintaining a **10–50 MB local CLI** footprint across 250 types, 50+ postal locales, 45+ phone locales, and 700+ CLDR date/time locales.

---

## Section 1: Literature deep-dive

### Sherlock through DODUO — the feature-engineering to transformer arc

**Sherlock (KDD 2019)** established the field with a multi-input deep neural network consuming **1,588 hand-crafted features** across four subnetworks: global statistics (27 features), character distributions (960 features), word embeddings (200-dim GloVe), and paragraph vectors (400-dim PV-DBOW). It classifies **78 DBpedia-derived types** on the VizNet corpus (686,765 columns), achieving **support-weighted F1 of 0.89**. Crucially, Sherlock operates on single columns with zero table context — it cannot distinguish "birthplace" from "city" when values overlap. The architecture is a straightforward feedforward MLP, making Candle reimplementation **High feasibility**. The model is small (estimated <2M parameters). Authors: Hulsebos, Hu, Bakker, Zgraggen, Satyanarayan, Kraska, Demiralp, Hidalgo. Published at ACM SIGKDD; arXiv:1905.10688. GitHub: `mitmedialab/sherlock-project`.

**Sato (VLDB 2020)** extended Sherlock with two-level context: an LDA topic model capturing "table intent" as a distribution over latent topics (global context), and a Linear-Chain CRF performing joint multi-column prediction (local context). On the same 78-type VizNet benchmark, Sato achieved **support-weighted F1 of 0.925** and **macro F1 of 0.735** — improvements of +5.3% weighted and +14.4% macro over Sherlock. The macro improvement signals dramatically better performance on underrepresented types. Context degradation is graceful: the LDA topic vector becomes uninformative for single-column tables, and the CRF falls back to independent prediction. Training framework is PyTorch + Gensim. Candle feasibility is **Medium** due to LDA integration complexity. Authors: Zhang, Suhara, Li, Hulsebos, Demiralp, Tan. arXiv:1911.06311. GitHub: `megagonlabs/sato`.

**DODUO (SIGMOD 2022)** replaced hand-crafted features entirely with a pre-trained Transformer (BERT-base, ~110M parameters). Its key innovation is **table-wise serialization**: an entire table is serialized as `[CLS] val1 val2 … [CLS] val3 val4 … [SEP]`, where each `[CLS]` marks a column boundary. Full self-attention across the sequence provides both intra- and inter-column context implicitly. Multi-task learning jointly predicts column types and column relations. On VizNet (78 types), DODUO achieves **micro F1 ≈ 92.5%**; on WikiTables (255 types), **micro F1 = 92.36%**, beating TURL's 88.86% by +4 points. Remarkably, **only 8 tokens per column suffice to outperform Sato's full model**, demonstrating that pre-trained LMs carry substantial world knowledge about entity types. The single-column variant (DosoloSCol) still outperforms Sato but shows significant degradation versus multi-column mode. DODUO's ~440 MB model size and 512-token sequence limit are practical constraints. On SOTAB-91 (real web data), DODUO achieves **84.8% micro F1** — a significant drop from VizNet, indicating distribution shift vulnerability. On GitTables, it achieves only **0.623 weighted F1** due to wide tables exceeding the 512-token limit. Candle feasibility is **Medium-Low** (BERT inference is feasible; training requires significant effort). Authors: Suhara, Li, Li, Zhang, Demiralp, Chen, Tan. arXiv:2104.01785. GitHub: `megagonlabs/doduo`.

**ColNet (AAAI 2019)** took a parallel approach: a CNN operating on word2vec embeddings of cell values, combined with DBpedia entity lookup voting. The CNN learns locality features across cells within a column through convolutional filters of varying heights. An ensemble of CNN prediction and KB entity lookup achieves strong results on T2Dv2 (237 tables) and Limaye (295 tables). No inter-column context is used. Candle feasibility is **High** for the neural component (simple CNN + FC), though the KB lookup pipeline adds complexity. Authors: Chen, Jiménez-Ruiz, Horrocks, Sutton. arXiv:1811.01304. No public code released.

### TURL and TaBERT — structure-aware pre-training

**TURL (VLDB 2021)** introduced a structure-aware Transformer with a **visibility matrix** that constrains self-attention to structurally related cells: entities attend only to elements in the same row or column, plus table caption and topic entity. Pre-training uses dual objectives — masked language modeling and **Masked Entity Recovery (MER)**, which learns factual knowledge by recovering masked entity cells from context. For CTA on WikiTables (255 Freebase types), TURL achieves **micro F1 = 88.86%**. Performance degrades significantly without table metadata (headers/captions). The entity embedding table for ~2M entities adds substantial parameters beyond the BERT base (~130–200M total). Candle feasibility is **Medium** — the visibility matrix is just an attention mask, but the large entity embedding table is a constraint. Authors: Deng, Sun, Lees, Wu, Yu. arXiv:2006.14806. GitHub: `sunlab-osu/TURL`.

**TaBERT (ACL 2020)** addresses joint understanding of textual and tabular data via **content snapshots** — selecting the K rows most relevant to an input NL utterance — and **vertical self-attention** for cross-row information flow. Pre-trained on **26.6 million table-sentence pairs** from Wikipedia and WebTables. TaBERT achieves **51.4% execution accuracy** on WikiTableQuestions (semantic parsing, not CTA). While not directly applicable to column type detection (it uses only 2 column types: text/real), TaBERT's vertical self-attention mechanism is architecturally relevant for encoding cross-row patterns within a column. The core architecture is BERT + custom attention layers (~110–350M parameters). Candle feasibility is **Medium-High** since it's fundamentally BERT with additional attention layers. Authors: Yin, Neubig, Yih, Riedel. arXiv:2005.08314. GitHub: `facebookresearch/TaBERT` (archived Oct 2023).

### SOTAB benchmark and LLM-based approaches

**SOTAB (SemTab@ISWC 2022)** is the most challenging CTA benchmark, drawn from real heterogeneous web data across **74,215 websites** with Schema.org annotations. V1 provides **91 CTA types** across **162,351 columns from 59,548 tables**; V2 offers 82 CTA types across 120,507 columns from 45,834 tables with manually verified validation/test sets. Tables explicitly exclude metadata (no headers or captions), making the benchmark exceptionally challenging. DODUO achieves 84.8% micro F1 as the strongest baseline. English-only (FastText language filtering). Authors: Korini, Peeters, Bizer (University of Mannheim). GitHub: `wbsg-uni-mannheim/wdc-sotab`.

**ArcheType (VLDB 2024)** pioneered LLM-based open-set CTA with a four-stage pipeline: context sampling, prompt serialization, model querying, and label remapping. Using LLAMA-7B fine-tuned via LoRA, ArcheType matches DODUO on SOTAB-91 while enabling **zero-shot CTA** where label sets are defined at inference time. The CONTAINS+RESAMPLE remapping algorithm outperforms embedding-based anchoring. Most critically, ArcheType exposes DODUO's distribution-shift vulnerability: **DODUO drops from 84.8% to 23.8%** when evaluated on out-of-domain data. ArcheType requires 7B+ parameter models (~13 GB), making Candle feasibility **Low** for the full system. Authors: Feuer, Liu, Hegde, Freire. arXiv:2310.18208. GitHub: `penfever/ArcheType`.

**Korini & Bizer (2023)** evaluated ChatGPT for CTA on a 32-type SOTAB subset: **85.25% zero-shot micro F1** with table+instructions+roles prompting, versus **89.73%** for fine-tuned RoBERTa and **88.44%** for DODUO. A 2025 follow-up (Brinkmann & Bizer) found that using training data to generate label definitions outperforms using the same data as in-context demonstrations.

### Recent advances (2023–2025)

**Pythagoras (EDBT 2024)** specifically addresses numerical column typing with context injection, surpassing DODUO by **+17.92% weighted F1 and +21.53% macro F1** on SportsTables. This is directly relevant to FineType's hs_code/decimal_number confusion.

**Watchog (SIGMOD 2024)** from Megagon Labs applies contrastive learning with data augmentation at token/cell/column levels, achieving **up to +26% micro F1 and +41% macro F1** over prior SOTA in semi-supervised settings. GitHub: `megagonlabs/watchog`.

**Jellyfish (EMNLP 2024)** instruction-tunes Mistral-7B and LLaMA-3-8B for data preprocessing tasks including CTA. Uses **reasoning data distillation from Mixtral**. Inference latency: **0.07–0.15 seconds per instance** versus GPT-4's 1–8 seconds. Competitive with GPT-3.5. Models on HuggingFace: `NECOUDBFM/Jellyfish`. Candle feasibility is **High** (Mistral-7B fully supported in Candle with GGUF quantization).

**TableLlama (NAACL 2024)** fine-tunes LLaMA-2-7B on TableInstruct (2.6M instances across 8 table tasks). Achieves 5–44 point gains on out-of-domain datasets versus the base model. GitHub: `OSU-NLP-Group/TableLlama`.

**AdaTyper (2023)** by Hulsebos et al. addresses deployment adaptation using hybrid rule-based estimators + ML, achieving ~0.6 average precision after only 5 examples of a new type. GitHub: `madelonhulsebos/AdaTyper`.

**ICE-T (AAAI 2025)** introduces interactions-aware cross-column contrastive embedding, treating each column as a distinct modality and contrasting it against aggregate embeddings of complementary columns. Scales linearly with column count.

### Byte-level and character-level models

**No papers apply byte-level models to column type detection** — this is a clear research gap. However, the relevant architectures are well-characterized:

**ByT5** (Google, 2021) operates directly on UTF-8 bytes with a 259-token vocabulary. Uses an asymmetric encoder-decoder (3:1 encoder-to-decoder depth). ByT5-Small has ~300M parameters; ByT5-Base ~580M. It is **4× more robust to noise** than subword models and outperforms mT5 at Small/Base sizes on GLUE/SuperGLUE. Inference is **1–10× slower** than token-level models due to ~4× longer byte sequences. Pre-trained on mC4 (multilingual). Candle: T5 architecture is supported but ByT5's asymmetric design requires custom implementation (**Medium** feasibility).

**CANINE** (Google, TACL 2022) uses hash-based character embeddings (K=8 hashes → 768-dim) with 4× downsampling via strided convolution, then a 12-layer Transformer on the downsampled sequence. **121M parameters** (~500 MB). Outperforms mBERT by **2.8 F1 on TyDi QA** with **28% fewer parameters**. Pre-trained on 104 languages. Candle: **Not implemented** — requires porting hash embeddings, local attention, strided convolutions (**Low-Medium** feasibility).

**Charformer** (Google, ICLR 2022) introduces Gradient-Based Subword Tokenization (GBST), which learns latent subword segmentation end-to-end. **28–100% faster** than both byte-level and subword models. Competitive with T5-Base on GLUE. No official weights released; no HuggingFace integration. Candle: **Low** feasibility (would require ground-up GBST implementation).

### Model2Vec — the lightweight embedding option

**Model2Vec** (MinishLab, 2024) distills sentence transformers into static embeddings by forward-passing the vocabulary through the teacher model, then applying PCA and SIF weighting. The result: **potion-base-8M** (~8 MB) and **potion-base-32M** (~30 MB) — up to **50× smaller** and **500× faster** than the original sentence transformer, with strong performance on MTEB. Critically, **an official Rust implementation exists** (`MinishLab/model2vec-rs`) that is **1.7× faster** than the Python version, uses safetensors, and supports f32/f16/i8 weight types. This is currently in FineType's pipeline and represents the most production-ready embedding option for Rust deployment.

### Summary comparison table

| System | Year | Venue | Types | Best F1 | Context | Size | Candle |
|--------|------|-------|-------|---------|---------|------|--------|
| Sherlock | 2019 | KDD | 78 | 0.89 (weighted) | None | ~2M params | High |
| Sato | 2020 | VLDB | 78 | 0.925 (weighted) | LDA + CRF | ~5M params | Medium |
| ColNet | 2019 | AAAI | Variable | ~0.91–0.93 | KB lookup | ~2M params | High |
| TURL | 2021 | VLDB | 255 | 88.86 (micro) | Visibility matrix | ~130–200M | Medium |
| TaBERT | 2020 | ACL | 2 | N/A (parsing) | Vertical attn | ~110–350M | Medium-High |
| DODUO | 2022 | SIGMOD | 78/255 | 92.36 (micro) | Full self-attn | ~110M | Medium-Low |
| SOTAB | 2022 | SemTab | 91 CTA | Benchmark | N/A | N/A | N/A |
| ArcheType | 2024 | VLDB | Open-set | ~85 (SOTAB-91) | Column-only | 7B+ | Low |
| Jellyfish | 2024 | EMNLP | Open-set | Competitive w/ GPT-3.5 | Column-only | 7–13B | High (quantized) |
| Watchog | 2024 | SIGMOD | 78 | +26% micro over SOTA | Contrastive | ~110M | Medium |

---

## Section 2: Transferable architectural patterns

### Tier 1 — End-to-end replacements

**Pattern 1: Serialised-table transformer (DODUO-style)**

Serialize a column as `[CLS] header [SEP] val1 [SEP] val2 …`, fine-tune a pre-trained LM for 250-class classification. DODUO proves this works for 255 types on WikiTables (92.36% micro F1). The open question is whether **byte-level tokenization** (ByT5/Charformer) can replace subword tokenization. ByT5 would eliminate tokenization artifacts on alphanumeric patterns like git SHAs, HS codes, and Docker references — precisely the visually-similar types causing FineType's confusions. ByT5-Small at 300M parameters is too large for a 10–50 MB CLI; however, a distilled byte-level encoder trained specifically for column typing could potentially be compressed to 20–50 MB. Charformer's GBST is theoretically ideal (learned subwords, faster than byte-level) but has no released weights or HuggingFace integration.

- **Failure categories addressed**: Both bare-name ambiguity (via multi-column serialization) and visually-similar-type confusion (byte-level tokenization captures character patterns)
- **Multilingual**: Native with byte-level models (ByT5 pre-trained on mC4)
- **Candle feasibility**: Medium (T5 supported; ByT5 asymmetric design needs custom work)
- **Model size**: 50–300M parameters (distilled: potentially 10–50M)
- **Subsumes rules**: Would subsume leading-zero→numeric_code, slash-segments→docker_ref, digit-ratio+dots→hs_code, header hints

**Pattern 2: Local LLM classification backend**

Use Phi-3-mini (3.8B) or LLaMA-3-8B-Q4 as a classifier with constrained decoding. Candle fully supports both with GGUF quantization. **llguidance** (Microsoft, native Rust) enables constrained decoding at **~50μs per token**. For 250 classes, the FSM state space is trivially small. ArcheType demonstrates that fine-tuned LLAMA-7B matches DODUO on SOTAB-91, and Jellyfish shows 0.07–0.15s per-instance latency on A100. However, **quantized 4-bit LLaMA-7B requires ~4 GB model storage** — far exceeding the 10–50 MB target. Distillation from a large LLM teacher to a small student (DistilBERT-scale, ~66M params) retaining ~97% accuracy is the practical path.

- **Failure categories addressed**: Both (LLMs have broad world knowledge for disambiguation)
- **Multilingual**: Inherent (LLMs are multilingual)
- **Candle feasibility**: High for inference; Low for fitting in 10–50 MB without distillation
- **Model size**: 2–8 GB quantized (distilled student: 50–250 MB)
- **Subsumes rules**: Would subsume nearly all model_patch rules; domain_knowledge rules may still be needed for edge cases

**Pattern 3: Byte-level unified model**

Replace both Model2Vec and CharCNN with a single byte/character-level encoder. The candidates are:

| Model | Params | Candle | Multilingual | Speed | FineType fit |
|-------|--------|--------|-------------|-------|-------------|
| ByT5-Small | ~300M | Medium | Yes (mC4) | 1–10× slower | Too large without distillation |
| CANINE | ~121M | Low | Yes (104 langs) | Comparable to BERT | Needs porting |
| Charformer | ~220M | Low | Yes | 28–100% faster | No weights released |
| Custom byte CNN | ~5–20M | High | Yes (byte-level) | Very fast | Best size fit |

A **custom byte-level CNN** (inspired by ColNet's cell-level CNN but operating on raw bytes) is the most feasible option for the 10–50 MB constraint. This would process column values as raw byte sequences through convolutional filters, capturing the character-level patterns that distinguish git_sha from hash, hs_code from decimal_number, and docker_ref from hostname.

- **Failure categories addressed**: Primarily visually-similar-type confusion; byte patterns directly encode the distinguishing features (hex characters in SHA, dot-separated digits in HS codes, slash segments in Docker refs)
- **Multilingual**: Native (byte-level is language-agnostic)
- **Candle feasibility**: High for custom CNN; Medium for ByT5; Low for CANINE/Charformer
- **Model size**: 5–20M (custom CNN) to 121–300M (pre-trained)
- **Subsumes rules**: leading-zero→numeric_code, slash-segments→docker_ref, digit-ratio+dots→hs_code, duration override, validation-based elimination

### Tier 2 — Hybrid enhancements

**Pattern 4: Sibling-context attention**

Cross-column attention conditioned on other columns in the same table, with graceful degradation to single-column mode. The literature offers strong precedents: DODUO's full self-attention, TURL's visibility matrix, TabTransformer's categorical column attention, and TableFormer's order-invariant attention biases. For FineType, this directly addresses **bare-name ambiguity** — a column named "code" is git_sha when siblings include "commit_message" and "author", but hs_code when siblings include "country_of_origin" and "tariff_rate". Implementation: represent each column as an embedding vector (from Model2Vec on header + sampled values), apply 2–4 self-attention layers across column embeddings, then classify each column's attended representation. When only one column is available, the attention reduces to self-attention (identity). **TabTransformer demonstrates graceful degradation up to 30% blanked features.** ICE-T's approach of contrasting each column against aggregate sibling embeddings is particularly relevant.

- **Failure categories addressed**: Bare-name ambiguity (primary); also helps visually-similar types when sibling columns provide domain context
- **Multilingual**: Depends on embedding backbone
- **Candle feasibility**: High (standard attention primitives available in Candle)
- **Model size**: +1–5M parameters on top of existing backbone
- **Subsumes rules**: header hints (both hardcoded and Model2Vec), geography rescue, entity demotion, attractor demotion

**Pattern 5: Alternative embedding backbone**

Replace Model2Vec with tabular-pretrained embeddings (TURL, TaBERT) or multilingual byte-level models. TURL's entity embeddings capture factual knowledge but require a massive entity table. TaBERT's vertical self-attention learns cross-row patterns. For FineType's constraints, the most practical alternative is **multilingual Model2Vec** — distilling a multilingual sentence transformer (e.g., multilingual-e5-base) into a static embedding model using Model2Vec's 30-second CPU distillation process. This would produce ~8–30 MB multilingual embeddings compatible with the existing `model2vec-rs` Rust crate.

- **Failure categories addressed**: Primarily bare-name ambiguity (better header understanding); multilingual variants address locale coverage
- **Multilingual**: Yes with multilingual backbone
- **Candle feasibility**: High (Model2Vec-rs already exists)
- **Model size**: 8–30 MB
- **Subsumes rules**: header hints, locale detection (partially)

**Pattern 6: Multi-signal Sherlock-style features + learned fusion**

Expand to Sherlock's ~1,588 features (character distributions, statistical properties, word embeddings, paragraph vectors) and fuse them with learned representations from Model2Vec or a byte-level encoder. Sherlock's character distribution features (960 features) alone carry strong signal for distinguishing visually-similar types. A learned fusion layer (MLP or attention) combines hand-crafted features with neural embeddings. This is the most conservative enhancement with proven effectiveness (Sherlock achieves 0.89 F1 with features alone).

- **Failure categories addressed**: Both (character distributions distinguish visual patterns; statistical features provide disambiguation signal)
- **Multilingual**: Partial (statistical features are language-agnostic; word embeddings are not)
- **Candle feasibility**: High (MLP fusion is trivial; feature extraction is string processing in Rust)
- **Model size**: +2–5 MB for feature extraction + fusion
- **Subsumes rules**: leading-zero→numeric_code, digit-ratio+dots→hs_code, text length demotion, percentage without %, validation-based elimination

### Tier 3 — Incremental fallbacks

**Pattern 7: Hierarchical classification**

Implement a tree-structured softmax following FineType's natural hierarchy: 7 domains → subcategories → 250 types. Each internal node has a local softmax over its children, and the final probability is the product of path probabilities from root to leaf. Schuurmans & Murre (2023) show this improves macro-F1 and macro-recall on all 4 tested datasets versus flat softmax. Computational benefit: **O(log 250) ≈ O(8)** versus O(250) per sample. When fine-grained classification fails, the coarse-level prediction remains valid — this is particularly useful for the git_sha/hash and docker_ref/hostname confusions, where at least the correct domain (identifiers vs. network) would be predicted.

- **Failure categories addressed**: Both (reduces search space at each level; domain prediction disambiguates visually-similar types)
- **Multilingual**: Architecture-agnostic
- **Candle feasibility**: High (tree of linear layers)
- **Model size**: Minimal additional (~100K parameters)
- **Subsumes rules**: None directly, but improves overall accuracy

**Pattern 8: Contrastive pre-training on column data**

Pre-train column embeddings via contrastive learning: positive pairs are columns with the same semantic type, negatives are different types. Watchog demonstrates **+26% micro F1** in semi-supervised settings. Self-supervised pre-training on unlabeled tables is feasible using augmentations: permuting values, masking column names, corrupting cell values. ICE-T's cross-column contrastive approach is particularly relevant, producing column-specific embeddings that capture both content and context.

- **Failure categories addressed**: Both (contrastive learning sharpens boundaries between similar types)
- **Multilingual**: Depends on backbone
- **Candle feasibility**: Low for training (training in Candle is immature); High for inference with pre-trained weights
- **Model size**: Same as backbone
- **Subsumes rules**: Attractor demotion (learned rather than hand-coded)

**Pattern 9: Improved vote aggregation**

Replace simple majority voting with attention-weighted or confidence-based aggregation across sampled column values. Current per-value predictions are aggregated with equal weight; instead, learn an attention function over value predictions that weights high-confidence predictions more heavily. This is the simplest possible enhancement.

- **Failure categories addressed**: Primarily visually-similar-type confusion (high-confidence predictions on distinctive values can override low-confidence predictions on ambiguous values)
- **Multilingual**: Architecture-agnostic
- **Candle feasibility**: High (attention is a basic primitive)
- **Model size**: Minimal (<100K parameters)
- **Subsumes rules**: Attractor demotion, text length demotion

---

## Section 3: Dataset recommendations

### Labelled datasets

| Dataset | Size | Types | Mappability | Language | License | Access |
|---------|------|-------|-------------|----------|---------|--------|
| **GitTables** | 1.7M tables | 2K+ (Schema.org + DBpedia) | **High** | English | CC BY 4.0 | zenodo.org/records/6517052 |
| **WikiTables-TURL** | 570K tables, 628K cols | 255 (Freebase) | **High** | English | CC BY-SA | ⚠️ OneDrive link broken; HuggingFace: `ibm/turl_table_col_type` |
| **SOTAB V2** | 45,834 tables, 120K cols | 82 CTA + 108 CPA | **Medium-High** | English | Public | webdatacommons.org/structureddata/sotab/v2/ |
| **VizNet/Sherlock** | 78,733 tables, 119K cols | 78 (DBpedia) | **Medium** | English | Source-dependent | Via `megagonlabs/doduo` download script |
| **Sherlock raw** | 686,765 columns | 78 (DBpedia) | **Medium** | English | Source-dependent | github.com/mitmedialab/sherlock-project |
| **T2Dv2** | 779 tables | ~91 DBpedia classes | **Medium** | English | CC BY-SA | webdatacommons.org/webtables/goldstandardV2.html |

**GitTables is the single most valuable labelled resource** for FineType. Its 1.7M tables from GitHub CSVs resemble real-world data far more than web tables, with tables averaging 25 columns and 209 rows (versus web tables' typical 3–5 columns). The 2K+ Schema.org and DBpedia type annotations provide the richest type coverage, though annotations are automatically generated (distant supervision) and therefore noisy. The manually curated 1,101-table benchmark subset has 122 DBpedia types and 59 Schema.org types.

**SOTAB V2 is the gold-standard evaluation benchmark.** Its 82 CTA types from real web data across 55,511 websites, with manually verified validation and test sets, represent the most rigorous available test of column type detection. English only. The deliberate absence of headers and captions makes it exceptionally challenging.

**WikiTables-TURL offers the closest match to FineType's 250-type count** (255 Freebase types) but faces a critical data access issue: the original OneDrive hosting was revoked when the lead author graduated from OSU. A mirror exists at `ibm/turl_table_col_type` on HuggingFace and `sefeoglu/TURL-dataset-re` on GitHub.

**Known dataset quality issue**: Recent analysis (Babamahmoudi, 2025) identified significant **train-test overlap** in the VizNet/Sherlock dataset, meaning reported F1 scores may be inflated.

### Unlabelled datasets for pre-training

| Dataset | Size | Multilingual | Access |
|---------|------|-------------|--------|
| **Kaggle Datasets** | 100K+ datasets | Mostly English | kaggle.com/docs/api |
| **data.gov** | 100K+ federal datasets | English | catalog.data.gov/api/3/ |
| **GitHub CSV (via GitTables)** | 1.7M tables extracted | Mostly English | GitTables pipeline |
| **WDC Web Tables** | 233M HTML tables (90M relational) | Multilingual | webdatacommons.org/webtables/ |
| **WDC Schema.org Tables** | ~5M tables (42 classes) | Mostly English | webdatacommons.org/structureddata/schemaorgtables/2023/ |

### Multilingual-specific resources

| Resource | Coverage | Type | Access |
|----------|----------|------|--------|
| **CLDR** | 400+ locales | Format patterns (date, number, currency) | github.com/unicode-org/cldr |
| **OpenAddresses** | 578M addresses, 40+ countries | Address components | openaddresses.io |
| **WDC non-English tables** | ~40M+ relational tables | Unlabelled web tables | webdatacommons.org/webtables/ |

**No dedicated multilingual column type annotation dataset exists** — this is a significant gap in the literature. For FineType's multilingual requirements (50+ postal locales, 45+ phone locales, 700+ CLDR date/time locales), the path forward is: (1) use CLDR as the definitive source for locale-specific format patterns, (2) leverage OpenAddresses for real-world multilingual address data, and (3) construct a custom multilingual evaluation set from WDC non-English web tables with manual annotation.

---

## Section 4: Rules classification

Each of FineType's existing disambiguation rules falls into one of two categories: **domain_knowledge** (facts about the world; permanent) or **model_patch** (compensates for model weakness; should be subsumed by a better model).

### Domain knowledge rules (permanent)

| Rule | Rationale |
|------|-----------|
| **Leading-zero → numeric_code** | Factual: numeric codes (ZIP, NAICS, HS) preserve leading zeros; pure numbers do not. This is a data representation fact independent of model quality. |
| **UTC offset override** | Factual: UTC offset formats (±HH:MM) are definitional. No model should need to "learn" this. |
| **Validation-based elimination** | Factual: if values fail format validation for a candidate type (e.g., invalid Luhn check for credit card), that type is eliminated. This is logical constraint, not model compensation. |
| **Locale detection** | Factual: locale-specific patterns (decimal separators, date formats) are defined by CLDR specifications. A rule encoding CLDR facts is permanent knowledge. |

### Model patch rules (should be subsumed)

| Rule | What it compensates for | Eliminated by pattern(s) |
|------|------------------------|-------------------------|
| **Slash-segments → docker_ref** | Model cannot distinguish docker_ref from hostname based on "/" character patterns | Byte-level unified model (P3), Serialised-table transformer (P1) |
| **Digit-ratio + dots → hs_code** | Model confuses HS codes (NN.NN.NN.NN) with decimal numbers and IP addresses | Byte-level unified model (P3), Sherlock-style features (P6) |
| **Duration override** | Model misclassifies ISO 8601 duration strings | Byte-level unified model (P3), Serialised-table transformer (P1) |
| **Attractor demotion** | Model over-predicts high-frequency "attractor" types | Contrastive pre-training (P8), improved vote aggregation (P9) |
| **Text length demotion** | Model misclassifies long text as structured types | Sherlock-style features (P6), improved vote aggregation (P9) |
| **Entity demotion** | Model over-predicts entity types for non-entity columns | Sibling-context attention (P4), hierarchical classification (P7) |
| **Percentage without %** | Model fails to detect percentage values lacking the % symbol | Byte-level unified model (P3), Sherlock-style features (P6) |
| **Header hints (hardcoded)** | Hardcoded header→type mappings compensate for model's inability to use header semantics | Sibling-context attention (P4), alternative embedding backbone (P5) |
| **Header hints (Model2Vec)** | Soft header→type mapping via embeddings, still a patch for weak value-based classification | Serialised-table transformer (P1), alternative embedding backbone (P5) |
| **Geography rescue** | Model fails to identify geographic columns without explicit geo-indicators | Sibling-context attention (P4), contrastive pre-training (P8) |

### Architectural subsumption matrix

| Pattern | Model patches eliminated | Domain rules kept |
|---------|------------------------|-------------------|
| P1: Serialised-table transformer | slash-segments, duration, header hints (both), hs_code | All 4 domain rules |
| P2: Local LLM backend | Nearly all model patches | All 4 domain rules |
| P3: Byte-level unified model | slash-segments, digit-ratio+dots, duration, percentage, leading-zero* | UTC offset, validation, locale |
| P4: Sibling-context attention | header hints (both), entity demotion, attractor demotion, geography rescue | All 4 domain rules |
| P6: Sherlock-style features | digit-ratio+dots, text length, percentage, leading-zero* | UTC offset, validation, locale |
| P7: Hierarchical classification | entity demotion (partial) | All 4 domain rules |

*Note: leading-zero is classified as domain_knowledge, but a sufficiently powerful byte-level model may learn this pattern from data, making the explicit rule redundant even though it encodes factual knowledge.

---

## Section 5: Evaluation methodology

### Real-world benchmark construction

Build a **500+ column benchmark** from diverse real-world sources, stratified by domain and difficulty:

- **Kaggle** (~150 columns): Use the Kaggle API to sample popular datasets across domains. Prioritize datasets with >100 downloads and diverse column types. Target 20+ columns from each of: finance, healthcare, geography, technology, e-commerce, government, science.
- **data.gov** (~100 columns): Sample from the CKAN API across agencies (Census, BLS, EPA, NOAA, SEC). Focus on columns with ambiguous types (codes, identifiers, mixed formats).
- **GitHub** (~150 columns): Use GitTables' pipeline to sample CSV files from diverse repositories. Prioritize repositories with >10 stars to filter toy data.
- **Awesome Public Datasets** (~100 columns): Curate from the GitHub awesome-public-datasets list, sampling datasets not already in Kaggle/data.gov.

Each column must be annotated by **2 independent annotators** using FineType's 250-type taxonomy, with adjudication by a third annotator for disagreements. Record inter-annotator agreement (Cohen's κ) as a quality metric.

### Failure-category test suites

**Bare-name ambiguity suite (50 columns)**: Columns where the header alone is ambiguous (e.g., "code" could be postal_code, country_code, hs_code, git_sha; "name" could be person_name, company_name, product_name, city_name). Each column must include 2–3 sibling columns that disambiguate. Test both with and without sibling context to measure context-dependent accuracy.

**Visually-similar-types suite (30 pairs)**: Construct 30 column pairs where values are superficially similar but semantically different. Priority confusions from FineType's current errors:

- git_sha vs hash (both 40-char hex strings; distinguish by column context and value distribution)
- hs_code vs decimal_number (both dot-separated digit groups; distinguish by segment count and value ranges)
- docker_ref vs hostname (both contain dots and slashes; distinguish by structure patterns)
- Additional pairs: IP address vs version number, phone number vs numeric ID, date vs numeric code, URL vs file path, email vs username

**Multilingual suite (100 columns)**: 2 columns per locale for the top 50 locales, covering: dates (locale-specific formats), numbers (decimal/thousands separators), addresses (country-specific formats), currency (symbols and placement), phone numbers (country codes and formats).

**Edge cases suite (30 columns)**: Ground-truth ambiguous columns, empty columns, single-value columns, mixed-type columns, columns with >50% null values, columns with encoding issues, columns with header-value mismatches.

### Secondary benchmarks

**SOTAB alignment**: Map FineType's 250 types to SOTAB's 82 CTA types. For types without direct mapping, use the closest Schema.org parent. Report micro F1 and macro F1 on SOTAB V2 test set (15,040 columns). This enables direct comparison with DODUO (84.8%), ArcheType (~85%), and other published baselines.

**Sherlock test set alignment**: Map FineType's 250 types to Sherlock's 78 DBpedia types. Report on the standard 5-fold cross-validation splits. This enables comparison with Sherlock (0.89), Sato (0.925), and DODUO (~0.925).

### Metrics framework

| Metric | Scope | Purpose |
|--------|-------|---------|
| **Per-failure-category elimination rate** | Each of the 4 test suites | Primary: measures whether architectural changes fix targeted failures |
| **Macro F1** | Full 250-type taxonomy | Overall quality across all types, including rare ones |
| **Support-weighted F1** | Full taxonomy | Quality weighted by type frequency (production relevance) |
| **Per-domain accuracy** | Each of the 7 domains | Identifies domain-specific weaknesses |
| **Confusion matrices** | Visually-similar pairs | Detailed error analysis for known confusions |
| **Multilingual accuracy by locale** | Per-locale | Identifies locale-specific gaps |
| **Context lift** | With vs. without sibling columns | Measures value of context mechanism |
| **Latency (p50, p95, p99)** | Per-column classification | Ensures CLI responsiveness (<100ms p95 target) |
| **Model size (MB)** | Total on-disk footprint | Enforces 10–50 MB constraint |

---

## Section 6: Prioritised roadmap

### Validated priority ordering

Based on literature findings, the original priority ordering requires significant revision. The key insight is that **byte-level models, while theoretically ideal, face practical Candle feasibility challenges**, whereas **sibling-context attention and distillation-based approaches have clearer implementation paths**.

**Revised priority ordering:**

**Priority 1: Sibling-context attention with graceful degradation** — This should move to the top position. The literature consistently shows that context is the single most impactful factor: Sato's CRF improves macro F1 by +14.4% over Sherlock; DODUO's multi-column mode significantly outperforms its single-column variant; Pythagoras shows +17.92% improvement for numerical columns specifically. Implementation in Candle is **High feasibility** using standard attention primitives. This directly addresses bare-name ambiguity (3 of 7 remaining errors) and costs minimal model size (+1–5 MB). The TabTransformer precedent demonstrates graceful degradation up to 30% blanked features. Self-attention over Model2Vec column embeddings is the simplest possible design.

**Priority 2: Expanded Sherlock-style features + learned fusion** — Move up from position 5. Sherlock's 960 character distribution features directly capture the byte-level patterns that distinguish visually-similar types — without requiring a byte-level Transformer. Computing character trigram distributions, digit-to-alpha ratios, positional patterns of dots/slashes/colons, and value-length statistics in Rust is trivially efficient. A learned fusion MLP (~2 MB) combining these features with Model2Vec embeddings addresses 3 of 7 errors (visually-similar confusions) with **High Candle feasibility** and near-zero latency impact. This is the lowest-risk, highest-certainty improvement available.

**Priority 3: Local LLM distillation (GPT-4/Claude teacher → small student)** — Validated as high-value. Use GPT-4 or Claude to classify ~50K columns across FineType's 250 types (estimated cost: $50–200). Train a student model (Model2Vec + hierarchical MLP, ~30 MB) on these labels. Jellyfish demonstrates that distillation from large models produces competitive classifiers. ArcheType confirms LLMs achieve ~85% on SOTAB-91 zero-shot. This serves dual purpose: ceiling validation (how good can a 250-type classifier get?) and practical training data generation. The student model runs within the 10–50 MB budget.

**Priority 4: Byte-level unified model (custom byte CNN)** — Revised from ByT5/Charformer to a **custom byte-level CNN**, as neither ByT5 nor Charformer has Candle implementations or released weights suitable for the 10–50 MB constraint. A custom CNN processing raw UTF-8 bytes through convolutional filters (inspired by ColNet's cell-level CNN and CANINE's downsampling) is the practical byte-level option. This directly addresses visually-similar-type confusion and is inherently multilingual. Estimated size: 5–20 MB. Candle feasibility: **High** for a custom CNN.

**Priority 5: Hierarchical classification** — Move up from position 7. The 7-domain → subcategory → 250-type hierarchy is a natural fit for FineType's taxonomy. Hierarchical softmax reduces the classification problem to a series of 5–15 class decisions, improving both accuracy (Schuurmans & Murre 2023 show consistent improvement) and computational efficiency (O(log N) vs O(N)). Implementation is trivial in Candle. This should be implemented alongside Priorities 1–2 as an incremental improvement.

**Priority 6: Serialised-column transformer (DODUO-style)** — Demoted from position 4. While DODUO is proven for 255 types, its ~440 MB BERT backbone far exceeds FineType's 10–50 MB budget. A DistilBERT backbone (~66M params, ~250 MB) is still too large. This becomes viable only if combined with aggressive quantization or if the CLI size constraint is relaxed. More importantly, Priorities 1–3 are likely to resolve the 7 remaining errors without requiring an end-to-end replacement.

**Priority 7: Contrastive pretraining on unlabelled columns** — Watchog's +26% micro F1 in semi-supervised settings is compelling, but the approach requires PyTorch training infrastructure and a large unlabeled corpus. The practical value is in **pre-training column embeddings that transfer to FineType's 250-type taxonomy**. This is a medium-term investment that pays off when scaling to new types or domains.

**Priority 8: Real-world data augmentation (GitTables/VizNet)** — GitTables is the most immediately useful augmentation source. Its 1.7M tables from GitHub CSVs cover diverse real-world column types. However, its automatic annotations are noisy and would require significant mapping effort to FineType's 250-type taxonomy. Best used after establishing the real-world benchmark (Section 5) to identify specific type coverage gaps.

### Cross-cutting concerns

**Multilingual evaluation must accompany every candidate.** Byte-level models (Priority 4) and LLM approaches (Priority 3) have inherent multilingual advantages. Sibling-context attention (Priority 1) is multilingual-agnostic. Sherlock-style features (Priority 2) are partially multilingual (statistical features are language-agnostic; character distributions work across scripts). The CLDR test data (400+ locales) should be used to construct the multilingual evaluation suite for every candidate.

**The 10–50 MB constraint is the binding practical constraint** that eliminates many theoretically superior approaches. The most productive path is: Model2Vec (8–30 MB) + character-level feature extraction (2–5 MB) + sibling-context attention (1–5 MB) + hierarchical classification head (<1 MB) + domain knowledge rules (code, <1 MB) = **12–42 MB total**, well within budget and addressing all 7 failure categories.

### Implementation timeline estimate

| Priority | Effort | Expected impact | Errors addressed |
|----------|--------|----------------|-----------------|
| P1: Sibling-context attention | 2–3 weeks | +3–5% macro F1 | 3 bare-name ambiguity |
| P2: Sherlock features + fusion | 1–2 weeks | +2–4% macro F1 | 3 visually-similar |
| P5: Hierarchical classification | 1 week | +1–2% macro F1 | Improves all categories |
| P3: LLM distillation | 2–3 weeks | Training data + ceiling | All categories |
| P4: Custom byte CNN | 3–4 weeks | +3–5% macro F1 | 3 visually-similar + multilingual |
| P6: DODUO-style transformer | 4–6 weeks | Major overhaul | All categories |

The first three priorities (P1 + P2 + P5) can be implemented in approximately **4–6 weeks** and are expected to address all 7 remaining errors while staying within the 10–50 MB CLI budget. The LLM distillation (P3) runs in parallel as a data-generation effort and provides the training signal for the other components.
