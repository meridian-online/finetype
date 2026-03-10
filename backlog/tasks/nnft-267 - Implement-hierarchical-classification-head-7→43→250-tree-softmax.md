---
id: NNFT-267
title: Implement hierarchical classification head (7→43→250 tree softmax)
status: In Progress
assignee:
  - '@nightingale'
created_date: '2026-03-09 00:56'
updated_date: '2026-03-10 01:25'
labels:
  - architecture
  - model
  - training
milestone: m-13
dependencies: []
references:
  - discovery/sense-architecture-challenge/SPIKE_C_HIERARCHICAL.md
  - discovery/sense-architecture-challenge/ARCHITECTURE_EVOLUTION.md
  - crates/finetype-model/src/charcnn.rs
  - crates/finetype-train/src/training.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Phase 2 of the architecture evolution (Spike C findings). Requires model retrain.

Replace the flat 250-class softmax output layer with a hierarchical tree following FineType's natural taxonomy: 7 domains → 43 categories → 250 leaf types. Spike C confirmed:

- +6.1% parameter overhead (97K → 103K), negligible
- All 5 known confusion pairs resolved structurally (3 at domain level, 2 at category level)
- Largest leaf softmax reduced from 250 to 40 (datetime.date)
- 4 degenerate categories (1 type each) need no leaf classifier
- Multi-level cross-entropy loss with λ = (0.2, 0.3, 0.5)
- Greedy top-down inference (beam-2 as fallback if domain accuracy proves unreliable)

Key struct: HierarchicalHead with domain_head, category_heads (7), leaf_heads (39 non-degenerate).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Implement HierarchicalHead struct in charcnn.rs with per-node linear layers
- [x] #2 Implement multi-level cross-entropy loss in training.rs (domain + category + type)
- [x] #3 Add hierarchy mapping derivable from dotted label names (no manual config needed)
- [x] #4 Train char-cnn-v15 with hierarchical head on same data as v14 (250 types, 1500/type)
- [x] #5 Report per-level accuracy: domain acc, category acc, type acc
- [x] #6 Profile eval maintains or improves over flat baseline (179/186)
- [x] #7 Model serialization/deserialization works with safetensors (named tensors per head)
- [x] #8 Greedy top-down inference produces correct flat label output
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
## Implementation Plan

### Step 1: HierarchyMap struct (char_cnn.rs)
Pure data structure derived from sorted label list. Parses domain.category.type labels, groups/sorts, builds bidirectional flat↔hier index mappings. Used at train and inference time.

### Step 2: HierarchicalHead struct (char_cnn.rs)
domain_head (Linear hidden→7), category_heads (Vec of 7 Linears), leaf_heads (Vec<Vec<Option<Linear>>>). Degenerate categories (1 type) get None. VarBuilder prefixed paths for named tensors.

### Step 3: Extend CharCnn for dual-mode (char_cnn.rs)
HeadType enum (Flat/Hierarchical). CharCnn gains optional fc2 + optional HierarchicalHead. New constructor new_hierarchical(). Existing new() unchanged.

### Step 4: Hierarchical forward/infer (char_cnn.rs)
backbone_forward() returns hidden vector after fc1+ReLU. Hierarchical mode: domain logits → per-domain category logits → per-category leaf logits. Product probabilities scattered to (batch, 250) via hier_to_flat. infer() returns same shape regardless of mode.

### Step 5: Multi-level loss (char_training.rs)
use_hierarchical config flag. Training uses backbone_forward() + per-level CE losses: 0.2*domain + 0.3*category + 0.5*leaf. Per-level accuracy reported each epoch.

### Step 6: Config + serialization (char_training.rs)
config.yaml gains head_type: hierarchical. Safetensors uses VarBuilder prefix paths.

### Step 7: CharClassifier load (inference.rs)
Parse head_type from config.yaml (default flat). When hierarchical, reconstruct labels and call new_hierarchical().

### Step 8: CLI flag (main.rs)
--hierarchical flag on Train subcommand, passed through to CharTrainingConfig.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
## Implementation Complete

### What was built
- **HierarchyMap**: Pure data structure derived from sorted label list, parses domain.category.type labels, builds bidirectional flat↔hier index mappings
- **HierarchicalHead**: Domain head (hidden→7), per-domain category heads (7), per-(d,c) leaf heads (39 non-degenerate, 4 degenerate skipped). VarBuilder prefixed paths for named tensors
- **Dual-mode CharCnn**: HeadType enum (Flat/Hierarchical). `new()` unchanged for flat, `new_hierarchical()` for tree softmax. `backbone_forward()` exposed for training loop
- **Product probability inference**: domain_softmax × category_softmax × leaf_softmax scattered to flat (batch, 250) tensor
- **Multi-level training loss**: λ=(0.2, 0.3, 0.5) weighted domain/category/leaf CE. Per-level accuracy reported each epoch
- **Config serialization**: `head_type: hierarchical` in config.yaml, backward compatible (absent = flat)
- **Inference loading**: Both `load()` and `from_bytes()` parse head_type and reconstruct hierarchy from labels
- **CLI flag**: `--hierarchical` on Train subcommand

### Verification
- All 450 tests pass (cargo test)
- Taxonomy check passes (cargo run -- check)
- Zero clippy warnings
- Trained tiny hierarchical model (1 epoch, 10 samples/type) — trains, saves, loads, infers
- Flat model (v14) unchanged behavior verified
- Per-level accuracy reporting works: domain_acc, cat_acc, type_acc logged each epoch

### Remaining for AC #4 and #6
- AC4: Full v15 training (1500 samples/type, 10 epochs) needs train.sh script on Metal
- AC6: Profile eval needs trained v15 model

### Training deferred to Metal hardware
Intel N150 (this machine) is too slow for full training (375K samples × 10 epochs). AC #4 and #6 require running on Hugh's M1 Mac:
```bash
./scripts/train.sh --samples 1500 --epochs 10 --hierarchical --model-name char-cnn-v15-250
./scripts/eval.sh --model models/char-cnn-v15-250
make eval-report
```

### Training results (Metal, 155 min)
- **Type accuracy: 84.2%** (domain: 90.9%, category: 96.5%) — 10 epochs, 1500/type
- **Profile eval: 180/186** (96.8% label, 98.4% domain) — matches v14 flat baseline
- Model size: 456K safetensors
- SQL warnings in actionability eval are pre-existing (JSON column quoting, spatial extension) — not related to this change
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Implemented hierarchical classification head (7→43→250 tree softmax) as Phase 2 of the architecture evolution, replacing the flat 250-class output layer with a tree that follows FineType's natural taxonomy.

## Changes

**`crates/finetype-model/src/char_cnn.rs`** — Core architecture:
- `HierarchyMap`: Pure data struct derived from sorted label strings, builds bidirectional flat↔hierarchical index mappings (domain, category, type)
- `HierarchicalHead`: Domain head (hidden→7), 7 per-domain category heads, 39 non-degenerate leaf heads (4 degenerate categories skipped). VarBuilder-prefixed paths for named safetensors
- `HeadType` enum (Flat/Hierarchical) added to `CharCnnConfig`
- `CharCnn` made dual-mode: `fc2` and `hierarchical` are both `Option`. `new()` unchanged (flat), `new_hierarchical()` builds tree head
- `backbone_forward()` exposed for training loop to compute per-level losses
- Product probability inference: p(type) = softmax(domain) × softmax(cat) × softmax(leaf), scattered to flat (batch, 250) tensor

**`crates/finetype-model/src/char_training.rs`** — Training:
- `use_hierarchical` config flag
- Multi-level cross-entropy loss: λ = (0.2 domain, 0.3 category, 0.5 leaf)
- Per-level accuracy reported each epoch (domain_acc, cat_acc, type_acc)
- `head_type: hierarchical` written to config.yaml (absent = flat for backward compat)

**`crates/finetype-model/src/inference.rs`** — Loading:
- Both `load()` and `from_bytes()` parse `head_type` from config.yaml
- Hierarchical models reconstruct hierarchy from sorted labels at load time

**`crates/finetype-cli/src/main.rs`** — CLI:
- `--hierarchical` flag on Train subcommand

**`scripts/train.sh`** — Script:
- `--hierarchical` flag support

## Training Results (char-cnn-v15-250)
- 250 types, 1500 samples/type, 10 epochs, seed 42
- Type accuracy: 84.2%, Domain: 90.9%, Category: 96.5%
- Model size: 456K safetensors
- Profile eval: **180/186** (96.8% label, 98.4% domain) — matches v14 flat baseline

## Backward Compatibility
- `CharCnn::new()` unchanged, creates flat mode
- `head_type` absent from config.yaml defaults to flat
- All existing flat models (v14, tiered-v2) load and work identically
- `classify_batch()` returns identical `ClassificationResult` format

## Tests
- `cargo test`: 450 tests pass (0 failures, 13 ignored golden tests)
- `cargo run -- check`: All 250 types aligned
- `cargo clippy`: Zero warnings
- Tiny model train→save→load→infer verified end-to-end
- Flat model (v14) regression-free"}
<parameter name="definitionOfDoneCheck">[2, 3]
<!-- SECTION:FINAL_SUMMARY:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [x] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [x] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [ ] #4 Decision record created if plan involved choosing between approaches
- [ ] #5 Daily memory log updated with session outcomes
- [ ] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
