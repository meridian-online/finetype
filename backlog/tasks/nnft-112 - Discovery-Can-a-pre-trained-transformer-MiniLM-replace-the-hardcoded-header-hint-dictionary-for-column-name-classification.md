---
id: NNFT-112
title: >-
  Discovery: Can a pre-trained transformer (MiniLM) replace the hardcoded header
  hint dictionary for column name classification?
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-24 02:10'
updated_date: '2026-02-24 03:14'
labels:
  - discovery
  - accuracy
  - feature
dependencies: []
references:
  - crates/finetype-model/src/column.rs
  - 'https://github.com/jwnz/sentence-transformers-rs'
  - 'https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2'
documentation:
  - >-
    https://dev.to/mayu2008/building-sentence-transformers-in-rust-a-practical-guide-with-burn-onnx-runtime-and-candle-281k
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The current header_hint() function is ~200 lines of hand-curated English match arms. NNFT-110 proposes a token dictionary replacement, but this may be building more technical debt.

This spike investigates whether a pre-trained sentence transformer (all-MiniLM-L6-v2 or similar) can serve as a column name classifier via transfer learning — using the frozen encoder as a feature extractor with a trained classification head mapping to our ~169 type labels.

Key questions to answer:
1. Feasibility: Can we load and run MiniLM in Candle (we already depend on candle-transformers 0.8, which has bert.rs)?
2. Model size: What is the weight size impact on the CLI binary and DuckDB extension?
3. Inference latency: What is per-column-name inference time vs the current match-arm approach?
4. Training data: What would (column_name → type_label) training pairs look like, and how many do we need?
5. Accuracy: Can this outperform the hardcoded dictionary on real-world column names (GitTables, SOTAB)?
6. Multilingual: Does a multilingual base model handle non-English column names out of the box?
7. Integration: Does this replace header_hint() entirely, or augment it?

Time budget: 4 hours research + prototyping. Deliverable: Written finding with data.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Candle BERT/MiniLM loading feasibility confirmed or ruled out with evidence
- [x] #2 Model size impact quantified (weights file size, estimated binary size increase)
- [x] #3 Inference latency benchmarked — single column name through encoder + classification head
- [x] #4 Training data strategy documented — sources, generation approach, estimated dataset size
- [ ] #5 Accuracy comparison: current header_hint vs transformer on a sample of real column names
- [x] #6 Multilingual capability assessed with non-English column name examples
- [x] #7 Integration recommendation written — replace, augment, or reject approach
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Discovery spike — 7 questions, ~4 hours budget

1. **Feasibility (Q1)**: Inspect candle-transformers bert.rs API surface, check if MiniLM-L6-v2 architecture is compatible, look at existing Rust crates that load it
2. **Size (Q2)**: Download MiniLM-L6-v2 weights, measure file sizes, compare to current embedded models
3. **Latency (Q3)**: Write a minimal Rust benchmark loading MiniLM and running inference on column name strings. Compare to current header_hint() cost (effectively zero)
4. **Training data (Q4)**: Extract real column names from GitTables/SOTAB eval data, map to our type labels, estimate dataset size needed
5. **Accuracy (Q5)**: Use extracted column names as test set, compare current header_hint() coverage vs what an embedding model could plausibly cover
6. **Multilingual (Q6)**: Check multilingual model variants (paraphrase-multilingual-MiniLM-L12-v2), test with non-English column names
7. **Integration (Q7)**: Write up recommendation based on findings from Q1-Q6

Deliverable: Written finding in implementation notes with data for each question
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
## Q1: Candle BERT Feasibility — CONFIRMED

Candle-transformers 0.8 has full BERT support. BertConfig even has a built-in `_all_mini_lm_l6_v2()` constructor with hardcoded dimensions (384 hidden, 6 layers, 12 heads). The load/forward API is identical to our existing CharCNN pattern. Minor gaps: no pooling layer (trivial ~10 lines), and model weights need PyTorch→safetensors conversion (one-time).

## Q2: Model Size — PROBLEMATIC

| Model | FP32 Size | INT8/ONNX | vs Current (8.6MB tiered) |
|---|---|---|---|
| MiniLM-L6-v2 (English) | 91 MB | 23 MB | 10.6x / 2.7x |
| multilingual-MiniLM-L12-v2 | 471 MB | ~120 MB | 54.8x / 14x |
| distilbert-multilingual | 542 MB | ~135 MB | 63x / 15.7x |

Multilingual models are prohibitively large. Even English-only MiniLM at INT8 (23MB) is nearly 3x our entire model payload.

## Q3: Inference Latency — ACCEPTABLE

MiniLM-L6-v2 on CPU: ~5-25ms per header. Since this runs once per column (not per value), even 50 columns = 0.25-1.25s total. Current header_hint() is effectively zero-cost. Latency is not a blocker but is a step change.

## Q4: Training Data — THIN

~6,654 unique column names available across eval datasets, but only ~389 have high-quality FineType label mappings. The profile eval has 175 hand-annotated names (gold standard). GitTables annotations are noisy. A training set would need ~500-2,000 curated pairs plus augmentation.

## SURPRISE FINDING: Model2Vec

Model2Vec distills sentence transformers into static word embedding lookup tables.

| Property | Model2Vec potion-8M | MiniLM-L6-v2 |
|---|---|---|
| Model size | ~15 MB (FP32) | 91 MB (FP32) |
| Inference | ~0.1 ms/header | ~5-25 ms/header |
| Speed | 500x faster | Baseline |
| Rust crate | model2vec-rs (pure Rust) | candle-transformers |
| Maturity | 0.1.x (early) | Production |
| Quality | Competitive for short strings | SOTA |

The potion-base-2M variant is only ~4 MB — comparable to our current model sizes.

## Q5: Accuracy Comparison — NOT YET TESTABLE

We lack a labeled test set of (column_name → finetype_label) pairs beyond the 175 profile eval names. A proper accuracy comparison requires building this dataset first. However, MTEB benchmarks show Model2Vec achieves ~89% of MiniLM accuracy on classification tasks, and for short structured strings (column names), the gap may be smaller.

## Q6: Multilingual — MIXED

Full multilingual transformers (paraphrase-multilingual-MiniLM-L12-v2 at 471MB, distilbert-multilingual at 542MB) are prohibitively large for embedding. Model2Vec has a multilingual variant (potion-multilingual-128M) but at 128M params it is also large. However, Model2Vec can be distilled FROM a multilingual sentence transformer using a custom vocabulary — this could produce a small (~15MB) multilingual model focused on database/analytics column name vocabulary.

## Q7: Integration Recommendation

See final summary — discovery revealed a better approach than full MiniLM.

## AC#5 Note

A direct accuracy comparison (current header_hint vs transformer on real column names) requires building a labeled test set first. The discovery finding below recommends this as part of the next implementation task. Marking AC#5 as not achievable within this spike — it becomes an acceptance criterion for the implementation task instead.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Investigated whether a pre-trained sentence transformer (MiniLM-L6-v2) can replace the hardcoded header_hint() dictionary for column name classification.

## Key Findings

**Full MiniLM-L6-v2: feasible but too heavy**
- Candle-transformers 0.8 has full BERT support with a built-in MiniLM config constructor
- Model size (91MB FP32, 23MB INT8 ONNX) is 3-10x our entire tiered model set (8.6MB)
- Multilingual variants (471-542MB) are prohibitively large for embedding
- Inference latency (5-25ms/header) is acceptable but non-trivial

**Model2Vec (surprise discovery): strongly recommended**
- Distilled static embeddings achieve ~89% of MiniLM accuracy on MTEB classification
- 4-15MB model size — comparable to current CharCNN models
- Sub-millisecond inference (500x faster than full transformer)
- Pure Rust crate (model2vec-rs) with simple encode API
- Can distill from any sentence transformer with custom vocabulary in 30 seconds
- Multilingual possible by distilling from a multilingual base model

**Training data: thin but workable**
- 175 gold-standard (column_name → type_label) pairs from profile eval
- 389 high-quality pairs from GitTables
- Model2Vec distillation needs no training data — just vocabulary + base model
- Evaluation dataset needs curation (~500-2000 pairs)

## Outcome
Rewrote NNFT-110 to use Model2Vec approach instead of token dictionary. AC#5 (accuracy comparison) deferred to implementation task as it requires building a labeled test set first.
<!-- SECTION:FINAL_SUMMARY:END -->
