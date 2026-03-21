# Sherlock Training Runbook — M1 Pro

**Date:** 2026-03-21
**Spec:** `specs/2026-03-21-sherlock-architecture/spec.yaml`
**Context:** AC-1 through AC-5 are merged (PR #19). Everything below requires Metal acceleration.

---

## Pre-flight

```bash
cd ~/github/meridian-online/finetype
git pull origin main

# Verify the merged code compiles with Metal
cargo build --bin finetype --no-default-features --features metal --release

# Verify feature extractors work
echo -e "John Smith\nJane Doe\n2024-01-15\nhello@example.com" | \
  ./target/release/finetype extract-features

# Verify distilled data exists (~9MB gzipped, 119K rows)
ls -lh output/distillation-v3/sherlock_distilled.csv.gz

# Run tests
cargo test -p finetype-model
cargo test -p finetype-train
```

---

## Step 1: Implement Hierarchical Head (~1 hour)

The multi-branch model currently rejects `HeadType::Hierarchical` — it needs a tree softmax implementation (7 domains → 43 categories → 250 types) to enable ablation experiments AC-6c and AC-6d.

**What to implement** in `crates/finetype-train/src/multi_branch.rs`:

1. Add a hierarchical head variant to `MultiBranchModel::new()` that creates three classification layers:
   - Domain head: shared_output_dim (500) → 7
   - Category head: shared_output_dim (500) → 43
   - Type head: shared_output_dim (500) → 250

2. Forward pass: compute all three logits, return the type-level logits (250-dim) as the primary output.

3. Loss: weighted sum of cross-entropy at each level (e.g., 0.5 × type_loss + 0.3 × category_loss + 0.2 × domain_loss). This requires a label → domain/category mapping, which already exists in `LabelCategoryMap` from `finetype-model`.

4. Remove the test `test_head_type_hierarchical_rejected` and add a new test that verifies the hierarchical model compiles and trains for 1 epoch.

**Ask Nightingale to implement this:**
```
Implement HeadType::Hierarchical in crates/finetype-train/src/multi_branch.rs.
See specs/2026-03-21-sherlock-architecture/spec.yaml AC-6 and this runbook.
```

---

## Step 2: PRE-1 — Retrain Baseline CharCNN (~2 hours)

Train a blend-30-70 CharCNN as the comparison baseline. This is the existing pipeline, just with the distilled data mixed in.

```bash
# Generate blended training data (30% distilled, 70% synthetic)
# The existing train.sh handles CharCNN — we need blended NDJSON first.
# Use the spike's approach: prepare_spike_data.py generates the blend.
python3 scripts/prepare_spike_data.py \
  --distilled output/distillation-v3/sherlock_distilled.csv.gz \
  --ratio-distilled 0.3 \
  --samples-per-type 1500 \
  --seed 42 \
  --output output/baseline-training/blend-30-70.ndjson

# Train CharCNN on blended data (large preset, 10 epochs)
./scripts/train.sh \
  --data output/baseline-training/blend-30-70.ndjson \
  --size large \
  --epochs 10 \
  --seed 42 \
  --model-name char-cnn-v16-baseline

# Evaluate — Tier 2
./scripts/eval.sh --model models/char-cnn-v16-baseline

# Evaluate — Tier 1 profile
eval/profile_eval.sh
```

**Expected:** ~80.6% Tier 2 overall (matching spike), ≥170/174 Tier 1.
Save the eval results — these are the comparison point for everything that follows.

---

## Step 3: AC-5 — Prepare Multi-Branch Training Data (~30-60 min)

```bash
# Dry run first — verify counts and type coverage
python3 scripts/prepare_multibranch_data.py --dry-run

# Full extraction (uses finetype extract-features subprocess)
python3 scripts/prepare_multibranch_data.py \
  --distilled output/distillation-v3/sherlock_distilled.csv.gz \
  --finetype ./target/release/finetype \
  --output output/multibranch-training/blend-30-70.ftmb \
  --samples-per-type 1500 \
  --ratio-distilled 0.3 \
  --seed 42 \
  --workers 8

# Verify the output
python3 scripts/read_ftmb.py output/multibranch-training/blend-30-70.ftmb --stats --verify
```

**Expected:** ~65K qualifying records, 180+ types, 1499 dims per record (960+512+27).

---

## Step 4: AC-6 — Ablation Study (4 experiments, ~8 hours total)

Each experiment trains the multi-branch model with different configuration. You'll need to add a `train-multi-branch` subcommand to the CLI, or ask Nightingale to wire it up.

The training function is `train_multi_branch()` in `crates/finetype-train/src/multi_branch.rs`. It reads `.ftmb` files and trains using Candle with Metal.

### Experiment Matrix

```
| Experiment | Head          | Sibling Context | Model Directory                    |
|------------|---------------|------------------|------------------------------------|
| AC-6a      | Flat          | Off              | models/sherlock-v1-flat             |
| AC-6b      | Flat          | On               | models/sherlock-v1-flat-sibling     |
| AC-6c      | Hierarchical  | Off              | models/sherlock-v1-hier             |
| AC-6d      | Hierarchical  | On               | models/sherlock-v1-hier-sibling     |
```

**Shared hyperparameters** (from spec):
- Seed: 42
- Batch size: 32
- Epochs: 10
- L2 regularisation: 0.0001
- Dropout: 0.35
- Learning rate: 0.0001 (Adam/AdamW)
- Cosine LR schedule with early stopping

**For sibling-context experiments (AC-6b, AC-6d):** The embedding branch features need to be extracted with sibling-context-enriched headers. This means either:
- (a) Prepare a second .ftmb file with sibling-context preprocessing, or
- (b) Add a `--sibling-context` flag to the data prep pipeline

This is an open implementation question — **ask Nightingale how to handle this** before running AC-6b/d.

---

## Step 5: AC-7 & AC-8 — Evaluation

```bash
# Tier 2 eval for all 4 experiments
for model in sherlock-v1-flat sherlock-v1-flat-sibling sherlock-v1-hier sherlock-v1-hier-sibling; do
  echo "=== $model ==="
  ./scripts/eval.sh --model models/$model
done

# Compare against baseline
# Look for:
#   - Overall accuracy ≥ 85% (target)
#   - No type regression > 3 columns vs v16-baseline
#   - If best < 82%: document pivot decision
#   - If best < 80%: rollback, don't ship

# Tier 1 profile eval on the best model
# (Update models/default symlink to point to best model first)
ln -sfn <best-model-dir> models/default
eval/profile_eval.sh
# Target: ≥ 170/174
```

---

## Step 6: AC-9 & AC-10 — Pipeline Integration & Latency (if results warrant)

Only proceed here if the best model hits ≥82% Tier 2 and ≥170/174 Tier 1.

- **AC-9:** Wire the multi-branch model into `column.rs` as the new Sense classifier. Sharpen stays deterministic (F1–F6, locale detection, etc.)
- **AC-10:** Benchmark 100 columns, report p50/p95/p99. Target: p95 ≤ 50ms on M1.

These are significant code changes — start a new session with Nightingale for implementation.

---

## Decision Points

After Step 4 (training), you'll have results to inform the decision:

```
| Result                              | Action                                    |
|-------------------------------------|-------------------------------------------|
| Best model ≥ 85% Tier 2            | Proceed to pipeline integration (AC-9/10) |
| Best model 82–85% Tier 2           | Iterate — adjust hyperparams, retrain     |
| Best model < 82% Tier 2            | Document pivot, consider hybrid approach  |
| Best model < 80% OR < 170/174 T1   | Rollback — keep char-cnn-v14-250          |
```

---

## Time Estimates

```
| Step                        | Estimated Time | Can Parallelise? |
|-----------------------------|----------------|------------------|
| Hierarchical head impl      | 1 hour         | No (blocks 6c/d) |
| PRE-1 baseline training     | 2 hours        | Yes (with Step 3) |
| AC-5 data preparation       | 30–60 min      | Yes (with Step 2) |
| AC-6a flat, no sibling      | 2 hours        | Sequential        |
| AC-6b flat, with sibling    | 2 hours        | Sequential        |
| AC-6c hier, no sibling      | 2 hours        | Sequential        |
| AC-6d hier, with sibling    | 2 hours        | Sequential        |
| AC-7/8 evaluation           | 30 min         | —                 |
| Total                       | ~12 hours      | ~10 hours wall    |
```

Steps 2 and 3 can run in parallel (different terminals). The four training experiments (Step 4) are sequential since they all need Metal. The hierarchical head implementation blocks AC-6c/d but not AC-6a/b — you can start flat experiments while implementing it.

---

## Quick Start (TL;DR)

```bash
# On the Mac:
cd ~/github/meridian-online/finetype && git pull
cargo build --bin finetype --no-default-features --features metal --release
cargo test

# Then open Claude Code and say:
# "Implement HeadType::Hierarchical — see specs/2026-03-21-sherlock-architecture/mac-runbook.md"
```
