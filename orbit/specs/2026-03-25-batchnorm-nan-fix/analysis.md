# BatchNorm NaN from Batch Size 1 — Root Cause Analysis

**Date:** 2026-03-25
**Analyst:** Nightingale
**Symptom:** Ablation baseline model (sherlock-v4-baseline) produces `val_loss: null` every epoch, train accuracy collapses from 52.7% to 0.82% after epoch 0.
**Affected code:** `crates/finetype-train/src/training.rs` (`shuffled_batches`)

---

## Symptom

The overnight v4 pipeline trains two models on identical FTMB v3 data (193,695 records):

1. **Sibling model** (group batching): Healthy training, 90% val accuracy, 121/190 eval.
2. **Baseline model** (flat batching): `val_loss: null` from epoch 0, accuracy stuck at 0.82%.

```
| Metric          | Sibling (epoch 19) | Baseline (epoch 0) | Baseline (epoch 1+) |
|-----------------|--------------------|--------------------|----------------------|
| train_loss      | 0.3197             | 3.2995             | 1.01 → 0.28         |
| val_loss        | 0.2892             | null               | null                 |
| train_accuracy  | 91.6%              | 52.7%              | 0.82%                |
| val_accuracy    | 89.9%              | 0.8%               | 0.8%                 |
```

Train loss decreases but accuracy is stuck — the model minimises loss in train mode (batch statistics) but eval mode (running statistics) is broken.

## Investigation

### Step 1: Model weight inspection

Saved model weights inspected via `safetensors`:

```
Baseline merge_bn.running_var: shape=[628], range=[nan, nan]  -- ALL 628 dims NaN
Baseline merge_bn.running_mean: shape=[628], range=[0.0, 85.8]  -- OK
Sibling merge_bn.running_var: shape=[628], range=[0.000012, 33692.0]  -- OK
```

**All 628 dimensions** of `running_var` are NaN. Not branch-specific — a systematic corruption.

### Step 2: Candle BatchNorm source

Candle 0.8.4 (`candle-nn/src/batch_norm.rs`) updates running variance with Bessel's correction:

```rust
let norm_x_weight = self.momentum * batch_size / (batch_size - 1.0);
```

For `batch_size = 1`: `1.0 / (1.0 - 1.0)` = `1.0 / 0.0` = **Inf**.

The update becomes `(0.9 * running_var) + (norm_x * Inf)` = **NaN**.

### Step 3: Last batch size

```
Training records: 164,641
Batch size:       32
Full batches:     5,145  (164,641 / 32 = 5145.03125)
Last batch:       1 record  (164,641 % 32 = 1)
```

The last batch of every epoch has exactly **1 record**.

### Step 4: Why sibling model is unaffected

The sibling model uses **group batching** (`batch_groups`): table groups are packed into batches until `batch_size` is reached. The last batch contains the remaining records from the final groups — typically multiple records (group sizes range 3–15). It never creates a batch of 1.

The baseline model uses **flat batching** (`shuffled_batches`): indices are shuffled and chunked, creating a trailing batch of size `n_train % batch_size`.

## Root Cause

`shuffled_batches()` in `training.rs` can produce a trailing batch of size 1:

```rust
// Before fix:
indices.chunks(batch_size).map(|c| c.to_vec()).collect()
```

Candle's `BatchNorm::forward_t` uses Bessel's correction `N/(N-1)` which divides by zero for N=1. Once NaN enters `running_var`, it never recovers (EMA: `(1-m)*NaN + m*x = NaN`). All subsequent eval-mode forward passes produce garbage.

## The NaN Cascade

```
Epoch 0, batch 5145 (last batch, size=1)
  └─ BatchNorm forward_t: norm_x_weight = momentum * 1.0 / (1.0 - 1.0) = Inf
  └─ running_var update: 0.9 * running_var + norm_x * Inf = NaN
  └─ running_var is now NaN for ALL 628 dimensions

Epoch 0, validation (eval mode)
  └─ BatchNorm forward_eval: uses running_var → NaN → val_loss = NaN → serialised as null

Epoch 1+ training
  └─ Train forward (train=true): uses batch stats → OK → loss decreases
  └─ Accuracy forward (train=false): uses running_var → NaN → accuracy = 0.8%
  └─ Pattern: loss decreases but accuracy is frozen
```

## Fix

Drop the trailing batch if it has fewer than 2 records:

```rust
// After fix (training.rs):
let mut batches: Vec<Vec<usize>> = indices.chunks(batch_size).map(|c| c.to_vec()).collect();
if let Some(last) = batches.last() {
    if last.len() < 2 {
        batches.pop();
    }
}
batches
```

**Trade-off:** Drops at most 1 training sample per epoch (0.0006% of data). Acceptable.

**Scope:** Affects all training paths that use `shuffled_batches` — multi-branch flat, sense, entity. Group batching (`batch_groups`) is unaffected.

## Verification

Three new tests confirm the fix:

```
| Test                                    | Records | Batch | Expected Batches | Last Batch |
|-----------------------------------------|---------|-------|------------------|------------|
| test_shuffled_batches                   | 10      | 3     | 3 (not 4)       | Dropped (was 1) |
| test_shuffled_batches_no_drop_when_even | 9       | 3     | 3                | N/A (exact) |
| test_shuffled_batches_keeps_remainder_2 | 11      | 3     | 4                | Kept (size 2) |
```

## Retrain Required

The baseline model must be retrained:

```bash
rm -rf models/sherlock-v4-baseline
./scripts/overnight_sherlock.sh --skip-data --skip-ablation  # sibling already trained
# Then run baseline separately, or full pipeline without --skip-ablation
```

The sibling model does not need retraining (uses group batching, unaffected).

---

## Lessons

1. **BatchNorm + variable batch sizes = landmine.** Always guard against batch size 1 when using BatchNorm. This is a known issue in PyTorch too (documented in `torch.nn.BatchNorm1d`), but Candle doesn't guard against it.
2. **Train-mode loss hiding eval-mode bugs.** The decreasing train_loss masked the NaN running statistics because training uses batch stats. The `val_loss: null` was the visible symptom.
3. **Modular group vs flat batching paths.** The two batching strategies have different failure modes. Group batching's natural minimum batch size (group_size >= 3) accidentally protected the sibling model.
