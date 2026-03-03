//! Shared training infrastructure: early stopping, schedulers, loss functions, metrics.

use anyhow::Result;
use candle_core::{DType, Device, Tensor, D};
use serde::{Deserialize, Serialize};

// ── Early Stopping ───────────────────────────────────────────────────────────

/// Tracks validation metric and stops training when no improvement for `patience` epochs.
pub struct EarlyStopping {
    patience: usize,
    best_metric: f32,
    best_epoch: usize,
    epochs_without_improvement: usize,
    higher_is_better: bool,
}

impl EarlyStopping {
    /// Create early stopping tracker.
    ///
    /// - `patience`: number of epochs without improvement before stopping
    /// - `higher_is_better`: true for accuracy, false for loss
    pub fn new(patience: usize, higher_is_better: bool) -> Self {
        Self {
            patience,
            best_metric: if higher_is_better {
                f32::NEG_INFINITY
            } else {
                f32::INFINITY
            },
            best_epoch: 0,
            epochs_without_improvement: 0,
            higher_is_better,
        }
    }

    /// Record a metric value. Returns `true` if training should stop.
    pub fn step(&mut self, epoch: usize, metric: f32) -> bool {
        let improved = if self.higher_is_better {
            metric > self.best_metric
        } else {
            metric < self.best_metric
        };

        if improved {
            self.best_metric = metric;
            self.best_epoch = epoch;
            self.epochs_without_improvement = 0;
        } else {
            self.epochs_without_improvement += 1;
        }

        self.epochs_without_improvement >= self.patience
    }

    /// Best metric value seen so far.
    pub fn best_metric(&self) -> f32 {
        self.best_metric
    }

    /// Epoch at which the best metric was observed.
    pub fn best_epoch(&self) -> usize {
        self.best_epoch
    }
}

// ── Cosine Annealing Scheduler ───────────────────────────────────────────────

/// Cosine annealing learning rate schedule with optional minimum LR floor.
pub struct CosineScheduler {
    base_lr: f64,
    min_lr: f64,
    total_epochs: usize,
}

impl CosineScheduler {
    pub fn new(base_lr: f64, min_lr: f64, total_epochs: usize) -> Self {
        Self {
            base_lr,
            min_lr,
            total_epochs,
        }
    }

    /// Compute learning rate for a given epoch.
    pub fn lr(&self, epoch: usize) -> f64 {
        if epoch >= self.total_epochs {
            return self.min_lr;
        }
        let progress = epoch as f64 / self.total_epochs as f64;
        let cosine = (1.0 + (std::f64::consts::PI * progress).cos()) / 2.0;
        self.min_lr + (self.base_lr - self.min_lr) * cosine
    }
}

// ── Loss Functions ───────────────────────────────────────────────────────────

/// Cross-entropy loss: -mean(log_softmax(logits)[target]).
///
/// Validated in Candle spike (test_cross_entropy_loss).
/// Kept as reference implementation; production training uses `candle_nn::loss::cross_entropy`.
///
/// - `logits`: [B, C] unnormalized class scores
/// - `targets`: [B] integer class indices (u32)
#[allow(dead_code)]
pub fn cross_entropy_loss(logits: &Tensor, targets: &Tensor) -> Result<Tensor> {
    let log_probs = candle_nn::ops::log_softmax(logits, D::Minus1)?;
    let target_log_probs = log_probs.gather(&targets.unsqueeze(1)?, 1)?.squeeze(1)?;
    let loss = target_log_probs.neg()?.mean_all()?;
    Ok(loss)
}

/// Weighted cross-entropy loss for class-imbalanced training.
///
/// - `logits`: [B, C] unnormalized class scores
/// - `targets`: [B] integer class indices (u32)
/// - `class_weights`: [C] per-class weights
pub fn weighted_cross_entropy_loss(
    logits: &Tensor,
    targets: &Tensor,
    class_weights: &Tensor,
) -> Result<Tensor> {
    let log_probs = candle_nn::ops::log_softmax(logits, D::Minus1)?;
    let target_log_probs = log_probs.gather(&targets.unsqueeze(1)?, 1)?.squeeze(1)?;

    // Gather weights for each sample's target class
    let sample_weights = class_weights
        .gather(&targets.unsqueeze(1)?, 0)?
        .squeeze(1)?;

    let weighted_loss = (target_log_probs.neg()? * sample_weights)?;
    let loss = weighted_loss.mean_all()?;
    Ok(loss)
}

// ── Accuracy ─────────────────────────────────────────────────────────────────

/// Compute classification accuracy: fraction of argmax(logits) == targets.
///
/// - `logits`: [B, C]
/// - `targets`: [B] (u32)
pub fn compute_accuracy(logits: &Tensor, targets: &Tensor) -> Result<f32> {
    let preds = logits.argmax(D::Minus1)?; // [B]
    let targets_u32 = targets.to_dtype(DType::U32)?;
    let correct = preds
        .eq(&targets_u32)?
        .to_dtype(DType::F32)?
        .mean_all()?
        .to_scalar::<f32>()?;
    Ok(correct)
}

// ── Metrics ──────────────────────────────────────────────────────────────────

/// Per-epoch training metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpochMetrics {
    pub epoch: usize,
    pub train_loss: f32,
    pub val_loss: f32,
    pub train_accuracy: f32,
    pub val_accuracy: f32,
    pub learning_rate: f64,
    pub epoch_time_secs: f32,
}

/// Summary of a complete training run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingSummary {
    pub best_epoch: usize,
    pub best_val_accuracy: f32,
    pub total_epochs: usize,
    pub total_time_secs: f32,
    pub epoch_metrics: Vec<EpochMetrics>,
}

// ── Batch Shuffling ──────────────────────────────────────────────────────────

/// Generate shuffled batch indices for an epoch.
pub fn shuffled_batches(
    n_samples: usize,
    batch_size: usize,
    rng: &mut impl rand::Rng,
) -> Vec<Vec<usize>> {
    use rand::seq::SliceRandom;
    let mut indices: Vec<usize> = (0..n_samples).collect();
    indices.shuffle(rng);
    indices.chunks(batch_size).map(|c| c.to_vec()).collect()
}

// ── Tensor Conversion Helpers ────────────────────────────────────────────────

/// Flatten nested 3D Vec → Tensor [d0, d1, d2].
pub fn vec3_to_tensor(data: &[Vec<Vec<f32>>], device: &Device) -> Result<Tensor> {
    let d0 = data.len();
    let d1 = data[0].len();
    let d2 = data[0][0].len();
    let mut flat = Vec::with_capacity(d0 * d1 * d2);
    for batch in data {
        for row in batch {
            flat.extend_from_slice(row);
        }
    }
    Ok(Tensor::new(flat.as_slice(), device)?.reshape((d0, d1, d2))?)
}

/// Flatten 2D Vec → Tensor [d0, d1].
pub fn vec2_to_tensor(data: &[Vec<f32>], device: &Device) -> Result<Tensor> {
    let d0 = data.len();
    let d1 = data[0].len();
    let mut flat = Vec::with_capacity(d0 * d1);
    for row in data {
        flat.extend_from_slice(row);
    }
    Ok(Tensor::new(flat.as_slice(), device)?.reshape((d0, d1))?)
}

/// Convert usize slice to u32 Tensor [N].
pub fn usize_to_tensor(data: &[usize], device: &Device) -> Result<Tensor> {
    let data_u32: Vec<u32> = data.iter().map(|&x| x as u32).collect();
    Ok(Tensor::new(data_u32.as_slice(), device)?)
}

/// Convert bool 2D Vec → f32 Tensor [d0, d1] (1.0 for true, 0.0 for false).
pub fn bool2d_to_tensor(data: &[Vec<bool>], device: &Device) -> Result<Tensor> {
    let d0 = data.len();
    let d1 = data[0].len();
    let mut flat = Vec::with_capacity(d0 * d1);
    for row in data {
        for &b in row {
            flat.push(if b { 1.0f32 } else { 0.0 });
        }
    }
    Ok(Tensor::new(flat.as_slice(), device)?.reshape((d0, d1))?)
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_early_stopping_improves() {
        let mut es = EarlyStopping::new(3, true);
        assert!(!es.step(0, 0.5));
        assert!(!es.step(1, 0.6));
        assert!(!es.step(2, 0.7));
        assert_eq!(es.best_epoch(), 2);
        assert!((es.best_metric() - 0.7).abs() < 1e-6);
    }

    #[test]
    fn test_early_stopping_triggers() {
        let mut es = EarlyStopping::new(2, true);
        assert!(!es.step(0, 0.9));
        assert!(!es.step(1, 0.8)); // 1 without improvement
        assert!(es.step(2, 0.7)); // 2 without improvement → stop
        assert_eq!(es.best_epoch(), 0);
    }

    #[test]
    fn test_early_stopping_loss_mode() {
        let mut es = EarlyStopping::new(2, false); // lower is better
        assert!(!es.step(0, 1.0));
        assert!(!es.step(1, 0.8)); // improved
        assert!(!es.step(2, 0.9)); // worse
        assert!(es.step(3, 0.85)); // worse again → stop
        assert_eq!(es.best_epoch(), 1);
    }

    #[test]
    fn test_cosine_scheduler() {
        let sched = CosineScheduler::new(1e-3, 1e-5, 100);
        let lr_0 = sched.lr(0);
        let lr_50 = sched.lr(50);
        let lr_100 = sched.lr(100);

        assert!((lr_0 - 1e-3).abs() < 1e-8, "epoch 0 should be base_lr");
        assert!(
            (lr_50 - (1e-5 + (1e-3 - 1e-5) * 0.5)).abs() < 1e-8,
            "epoch 50 should be midpoint"
        );
        assert!((lr_100 - 1e-5).abs() < 1e-8, "epoch 100 should be min_lr");
    }

    #[test]
    fn test_cross_entropy_loss() {
        let device = Device::Cpu;
        // logits: [[2.0, 1.0, 0.1], [0.5, 2.0, 0.3]]
        let logits = Tensor::new(&[[2.0f32, 1.0, 0.1], [0.5, 2.0, 0.3]], &device).unwrap();
        let targets = Tensor::new(&[0u32, 1], &device).unwrap();

        let loss = cross_entropy_loss(&logits, &targets).unwrap();
        let loss_val = loss.to_scalar::<f32>().unwrap();

        // Should be positive and finite
        assert!(loss_val > 0.0);
        assert!(loss_val.is_finite());
        // Correct predictions → loss should be relatively low
        assert!(loss_val < 2.0);
    }

    #[test]
    fn test_compute_accuracy() {
        let device = Device::Cpu;
        // logits: predictions match targets for 2/3
        let logits = Tensor::new(&[[2.0f32, 0.1], [0.1, 2.0], [2.0, 0.1]], &device).unwrap();
        let targets = Tensor::new(&[0u32, 1, 1], &device).unwrap(); // 2 correct, 1 wrong

        let acc = compute_accuracy(&logits, &targets).unwrap();
        assert!((acc - 2.0 / 3.0).abs() < 1e-4);
    }

    #[test]
    fn test_shuffled_batches() {
        let mut rng = rand::thread_rng();
        let batches = shuffled_batches(10, 3, &mut rng);
        assert_eq!(batches.len(), 4); // ceil(10/3)
        assert_eq!(batches[0].len(), 3);
        assert_eq!(batches[3].len(), 1); // remainder

        // All indices present exactly once
        let mut all: Vec<usize> = batches.iter().flatten().copied().collect();
        all.sort();
        assert_eq!(all, (0..10).collect::<Vec<_>>());
    }
}
