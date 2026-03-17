# Architecture Challenge: Findings & Spike Plan

**Date:** 2026-03-09
**Based on:** BRIEF.md responses from Claude and Gemini
**Status:** Findings synthesised, spikes defined

## Summary

Two independent literature reviews covering 15+ systems, 13 datasets, and 9 architectural patterns converged on the same core conclusions. High-confidence consensus exists on what to build; the remaining uncertainty is *how* it performs in FineType's specific context (250 types, Rust/Candle, 10–50 MB constraint).

## Consensus Findings (High Confidence)

### 1. Sibling-context attention is the highest-value enhancement

Every system that adds inter-column context shows dramatic improvement:
- Sato: +14.4% macro F1 over Sherlock (same 78 types, same data)
- DODUO multi-column: significant gains over single-column variant
- Pythagoras: +17.92% weighted F1 for numerical columns specifically

This directly addresses 3/7 remaining errors (bare-name ambiguity: airports.name, world_cities.name, multilingual.name). Candle feasibility: High.

### 2. Expanded deterministic features + learned fusion is lowest-risk

Sherlock's 960 character distribution features carry strong signal for visually-similar types. Computing character trigram distributions, digit-to-alpha ratios, positional patterns of dots/slashes/colons in Rust is trivially efficient. A learned fusion MLP (~2 MB) addresses 3/7 errors (git_sha/hash, hs_code/decimal_number, docker_ref/hostname). Candle feasibility: High.

We already have 32 features (NNFT-250). The question is which *additional* features discriminate between our specific confusable pairs.

### 3. Hierarchical classification is low-hanging fruit

Replace flat 250-class softmax with learned tree following FineType's natural hierarchy (7 domains → subcategories → 250 types). Adds <100K parameters. Computational benefit: O(log 250) vs O(250). When fine-grained fails, coarse domain prediction remains valid. Candle feasibility: High.

### 4. Rules classification is settled

| Classification | Rules |
|---|---|
| **Domain knowledge** (permanent) | F1 (leading-zero), Rule 14 (duration), Rule 17 (UTC offset), Validation-based elimination, Locale detection |
| **Model patch** (subsumable) | F2 (slash-segments), F3 (digit-ratio), Rule 15 (attractor demotion), Rule 16 (text length), Rule 18 (entity demotion), Rule 19 (percentage), Header hints (both), Geography rescue |

### 5. The 10–50 MB composite stack fits

Model2Vec (8–30 MB) + feature extraction (2–5 MB) + sibling-context attention (1–5 MB) + hierarchical head (<1 MB) + domain knowledge rules (code) = **12–42 MB total**.

### 6. GitTables is the priority real-world dataset

1.7M tables from GitHub CSVs, CC BY 4.0. Real-world header naming conventions that synthetic data lacks. SOTAB V2 is the gold-standard evaluation benchmark (82 CTA types, manually verified).

### 7. ByT5/Charformer/CANINE are impractical at current constraints

All exceed 10–50 MB even with aggressive quantization. A *custom* byte CNN (5–20 MB) is the practical byte-level option, but only after simpler approaches prove insufficient.

## Adopted Priority Ordering

Based on Claude's response (validated by literature consensus):

| Priority | Approach | Addresses | Effort | Risk |
|---|---|---|---|---|
| **P1** | Sibling-context attention | 3/7 bare-name ambiguity | 2–3 weeks | Low (proven in literature) |
| **P2** | Expanded features + fusion | 3/7 visually-similar types | 1–2 weeks | Very low (deterministic) |
| **P3** | LLM distillation (Qwen3 32B, local) | Training data + ceiling | 2–3 weeks | Medium (novel for 250 types) |
| **P4** | Custom byte CNN | Visually-similar + multilingual | 3–4 weeks | Medium-high |
| **P5** | Hierarchical classification | All categories (incremental) | 1 week | Very low |

P5 can be interleaved with P1–P2 as it's largely independent.

## Deferred Approaches

| Approach | Reason |
|---|---|
| ByT5/CANINE/Charformer | Exceeds 10–50 MB; no Candle implementations |
| DODUO-style serialised transformer | ~440 MB BERT backbone; DistilBERT still too large |
| Full LLM deployment (Phi-3/Llama-3) | 2–8 GB; incompatible with CLI constraint |
| Contrastive pretraining | Medium-term investment; needs unlabeled corpus |

## LLM Distillation Approach

**Teacher:** Qwen3 32B via Ollama (local, no API cost)
**Method:** Classify columns from GitTables/real CSVs into FineType's 250-type taxonomy with constrained output
**Student:** Small model (Model2Vec + hierarchical MLP, ~30 MB) trained on teacher labels
**Purpose:** Ceiling validation + high-quality training data from real-world columns

## Discovery Spikes (Pre-Implementation)

Three targeted spikes to validate feasibility before committing to implementation:

### Spike A: Sherlock Features for 250 Types (~4 hours)
**Question:** Which of Sherlock's 1,588 features actually discriminate between our confusable type pairs?
**Method:** Compute expanded features on existing eval data. Measure class separability for git_sha/hash, hs_code/decimal_number, docker_ref/hostname pairs.
**Output:** Ranked list of discriminative features with separability scores.

### Spike B: Sibling-Context Attention in Candle (~4 hours)
**Question:** What does cross-column attention look like in our Candle codebase?
**Method:** Prototype minimal self-attention layer over Model2Vec column embeddings. Test: compiles, inference cost, graceful single-column degradation.
**Output:** Working prototype or identified blockers. Latency measurement.

### Spike C: Hierarchical Classification Head (~4 hours)
**Question:** Does hierarchical prediction improve domain accuracy over flat 250-class?
**Method:** Map existing 250 types to tree structure (7 domains → subcategories → types). Prototype hierarchical softmax in Candle. Compare domain-level accuracy vs flat model.
**Output:** Accuracy comparison table. Training pipeline changes identified.

## Key Literature References

| Paper | Year | Key Insight for FineType |
|---|---|---|
| Sherlock (Hulsebos et al.) | 2019 | 1,588 features, 0.89 F1, Candle High |
| Sato (Zhang et al.) | 2020 | LDA + CRF context: +14.4% macro F1 |
| DODUO (Suhara et al.) | 2022 | Serialised-table transformer, 92.36% on 255 types |
| Pythagoras | 2024 | Numerical column typing: +17.92% weighted F1 |
| Watchog (Megagon Labs) | 2024 | Contrastive learning: +26% micro F1 semi-supervised |
| ArcheType (Feuer et al.) | 2024 | DODUO drops 84.8%→23.8% out-of-domain (!!) |
| Jellyfish | 2024 | Distilled Mistral-7B competitive with GPT-3.5 |
| ICE-T | 2025 | Cross-column contrastive embedding, scales linearly |

## Caution

- VizNet/Sherlock dataset has known train-test overlap (Babamahmoudi, 2025) — published F1 scores may be inflated
- DODUO's 84.8%→23.8% out-of-domain drop validates FineType's rules-based robustness
- No papers apply byte-level models to column type detection — this is a research gap, not a proven path
