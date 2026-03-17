# Architecture Evolution Plan

**Date:** 2026-03-09
**Based on:** Spike findings A (NNFT-262), B (NNFT-263), C (NNFT-264)
**Status:** Approved — implementation phased

## Current Pipeline (v0.6.8)

```
Column input: values[] + header
    │
    ├─ 1.  Sample 100 values
    ├─ 1b. Extract 32 features (per-value), aggregate as MEAN only
    │
    ├─ 2.  Model2Vec encode (header + 50 values) → 128-dim embedding
    ├─ 3.  Sense classify → broad category (6 classes)
    │
    ├─ 4.  CharCNN batch (flat 250-class softmax) → per-value votes
    ├─ 5.  Masked vote aggregation (Sense category filter)
    │
    ├─ 5b. Feature rules (F1, F2, F3)          ← model patches
    ├─ 6.  Entity demotion (Rule 18)            ← model patch
    ├─ 7.  Header hints (hardcoded + Model2Vec) ← model patches
    ├─ 8.  Geography rescue                     ← model patch
    │
    └─ 9.  Post-hoc locale detection
```

**Problems:** 7 remaining errors. 10 model-patch rules compensating for 3 structural gaps:
- No inter-column context → bare-name ambiguity (3 errors)
- No column-level distributional features → visually-similar confusion (3 errors)
- Flat 250-class softmax → no structural separation of confusable types

## Proposed Pipeline (v0.7.x)

```
Table input: columns[] with values[] + headers[]
    │
    ├─ 1.  Sample 100 values per column
    ├─ 1b. Extract 32 features (per-value)
    ├─ 1c. ✨ Aggregate as MEAN + VAR + MIN + MAX         ← Spike A
    │      (+6-8 new column-level stats: length variance,
    │       char-presence flags for : and -, float-parseability)
    │
    ├─ 2.  Model2Vec encode per column → N × 128-dim
    ├─ 2b. ✨ Sibling-context attention (2-layer, 4-head)  ← Spike B
    │      Input:  N × 128 (raw column embeddings)
    │      Output: N × 128 (context-enriched embeddings)
    │      Single-column: degrades to self-attention (identity)
    │      Cost: 1.51 MB, 112μs–1.3ms
    │
    ├─ 3.  Sense classify (using attended embeddings)
    │
    ├─ 4.  CharCNN backbone → 128-dim hidden representation
    ├─ 4b. ✨ Hierarchical classification head              ← Spike C
    │      Level 1: Domain (7 classes)     → domain_pred
    │      Level 2: Category (43 classes)  → category_pred
    │      Level 3: Leaf type (max 40)     → type_pred
    │      Cost: +5,934 params (+6.1%), largest softmax 40 vs 250
    │
    ├─ 5.  Masked vote aggregation (Sense + hierarchy-aware masking)
    │
    ├─ 5b. Feature rules (F1 + enhanced F2/F3 with new agg stats)
    │      ✨ NEW: length-var=0 → git_sha (not hash)       ← Spike A
    │      ✨ ENHANCED: F3 + float-parseability fraction    ← Spike A
    │
    ├─ 6-8. Remaining rules (reduced set over time)
    │
    └─ 9.  Post-hoc locale detection
```

## What Changes Where

| Component | File(s) | Change | Spike |
|---|---|---|---|
| **Feature aggregation** | `column.rs`, `features.rs` | Add var/min/max to column agg; add `:` and `-` char flags | A |
| **New column-level rules** | `column.rs` | `length-agg-var == 0` → git_sha; `is_float fraction < 1.0` → hs_code | A |
| **Sibling-context module** | NEW: `sibling_context.rs` | 2-layer pre-norm transformer, ~60 lines MHA + block | B |
| **Pipeline integration** | `column.rs` | Insert attention between Model2Vec and Sense | B |
| **Hierarchical head** | `charcnn.rs` | Replace `fc2` (128→250) with `HierarchicalHead` (47 smaller heads) | C |
| **Training loop** | `training.rs` | Multi-level CE loss (λ=0.2/0.3/0.5), hierarchy labels from dotted names | C |
| **Model config** | `config.yaml` | Add tree structure mapping, attention layer count | B+C |

## Rule Elimination Timeline

As the new components prove themselves, model-patch rules can be removed:

| Rule | Eliminated by | Timeline |
|---|---|---|
| F2 (slash-segments → docker_ref) | Hierarchical head separates `technology.development` vs `technology.internet` | With hierarchical retrain |
| F3 (digit-ratio → hs_code) | Hierarchical head separates `geography.transportation` vs `representation.numeric` | With hierarchical retrain |
| Rule 15 (attractor demotion) | Better distributional features + hierarchical routing | Gradual |
| Rule 18 (entity demotion) | Sibling-context attention learns person vs entity from siblings | With context training |
| Header hints (hardcoded) | Sibling-context attention subsumes header semantics | Gradual |
| Geography rescue | Sibling-context provides domain signal | With context training |

## Permanent Rules (Domain Knowledge)

| Rule | Why it stays |
|---|---|
| F1 (leading-zero → numeric_code) | Data engineering fact about code preservation |
| Rule 14 (duration override) | ISO 8601 domain specification |
| Rule 17 (UTC offset override) | ISO 8601 domain specification |
| Validation-based elimination | Logical constraint — format failures are definitive |
| Locale detection | CLDR-defined locale patterns are factual |

## Size Budget

| Component | Current | Proposed | Delta |
|---|---|---|---|
| CharCNN backbone | 65 KB | 65 KB | — |
| Classification head | 32 KB (flat) | 38 KB (hierarchical) | +6 KB |
| Sibling-context attention | — | 1.51 MB | +1.51 MB |
| Model2Vec | 8–30 MB | 8–30 MB | — |
| Sense classifier | ~200 KB | ~200 KB | — |
| Feature extraction code | ~0 KB | ~0 KB | — |
| **Total** | **~9–31 MB** | **~10–32 MB** | **+1.5 MB** |

Well within the 10–50 MB constraint.

## Implementation Phases

### Phase 1: Expanded Features + Column Rules (no retrain)
- Expand feature aggregation (var/min/max) + new column rules
- Expected: git_sha/hash resolved, hs_code/decimal improved
- Effort: ~1 week

### Phase 2: Hierarchical Classification Head (requires retrain)
- Replace flat 250-class softmax with 7→43→250 tree
- Expected: structural separation of all 5 confusion pairs
- Effort: ~1-2 weeks

### Phase 3: Sibling-Context Attention (requires multi-column training data)
- 2-layer self-attention over Model2Vec column embeddings
- Expected: bare-name ambiguity resolved (3/7 errors)
- Effort: ~2-3 weeks

### Phase 4: LLM Distillation (parallel data effort)
- Qwen3 32B via Ollama on M1 MacBook
- Classify real-world columns from GitTables into 250-type taxonomy
- Train small student model on teacher labels
- Expected: ceiling validation + improved training distribution
- Effort: ~2-3 weeks
