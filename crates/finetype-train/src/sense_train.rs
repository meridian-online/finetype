//! Sense model training loop: dual-head cross-attention over Model2Vec.
//!
//! Trains Architecture A for broad category (6 classes) + entity subtype (4 classes)
//! with AdamW, cosine annealing LR, early stopping, and header dropout.

use anyhow::{Context, Result};
use candle_core::Tensor;
use candle_nn::{AdamW, Optimizer, ParamsAdamW, VarMap};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::data::{BatchData, SenseDataset};
use crate::sense::{SenseModelA, EMBED_DIM, HIDDEN_DIM, N_BROAD, N_ENTITY};
use crate::training::{
    compute_accuracy, cross_entropy_loss, shuffled_batches, CosineScheduler, EarlyStopping,
    TrainingSummary,
};

// ── Configuration ────────────────────────────────────────────────────────────

/// Training hyperparameters for Sense model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SenseTrainConfig {
    /// Output directory for saved model artifacts.
    pub output_dir: PathBuf,

    /// Maximum training epochs.
    pub epochs: usize,

    /// Batch size.
    pub batch_size: usize,

    /// Initial learning rate (for AdamW).
    pub lr: f64,

    /// AdamW weight decay.
    pub weight_decay: f64,

    /// Early stopping patience (epochs without val accuracy improvement).
    pub patience: usize,

    /// Random seed for reproducibility.
    pub seed: u64,

    /// Header dropout rate during training (probability of zeroing has_header).
    pub header_dropout: f64,

    /// Minimum learning rate floor for cosine scheduler.
    pub min_lr: f64,

    /// Entity loss weight (multiplied by entity CE loss).
    pub entity_loss_weight: f64,
}

impl Default for SenseTrainConfig {
    fn default() -> Self {
        Self {
            output_dir: PathBuf::from("models/sense_prod/arch_a"),
            epochs: 50,
            batch_size: 64,
            lr: 5e-4,
            weight_decay: 0.01,
            min_lr: 1e-6,
            patience: 10,
            seed: 42,
            header_dropout: 0.5,
            entity_loss_weight: 0.5,
        }
    }
}

/// Saved alongside safetensors: model metadata + training results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SenseModelConfig {
    pub architecture: String,
    pub embed_dim: usize,
    pub hidden_dim: usize,
    pub n_broad: usize,
    pub n_entity: usize,
    pub n_params: usize,
    pub best_epoch: usize,
    pub val_broad_accuracy: f32,
    pub val_entity_accuracy: f32,
    pub training_config: SenseTrainConfigSnapshot,
}

/// Snapshot of training hyperparameters persisted in config.json.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SenseTrainConfigSnapshot {
    pub epochs: usize,
    pub batch_size: usize,
    pub lr: f64,
    pub patience: usize,
}

// ── Dual-head Loss ───────────────────────────────────────────────────────────

/// Compute dual-head loss: CE(broad) + entity_weight * CE(entity) for entity samples.
///
/// Entity loss only applies to samples where `broad_labels[i] == 0` (entity category).
/// If no entity samples are present in the batch, only broad loss is returned.
fn dual_head_loss(
    broad_logits: &Tensor,
    entity_logits: &Tensor,
    batch: &BatchData,
    entity_weight: f64,
) -> Result<Tensor> {
    let broad_loss = cross_entropy_loss(broad_logits, &batch.broad_labels)?;

    // Compute entity loss for entity-category samples only (broad_category_idx == 0).
    let broad_labels_vec: Vec<u32> = batch.broad_labels.to_vec1()?;
    let entity_indices: Vec<u32> = broad_labels_vec
        .iter()
        .enumerate()
        .filter(|(_, &label)| label == 0)
        .map(|(i, _)| i as u32)
        .collect();

    if entity_indices.is_empty() {
        return Ok(broad_loss);
    }

    let device = entity_logits.device();
    let idx_tensor = Tensor::new(entity_indices.as_slice(), device)?;

    let entity_logits_subset = entity_logits.index_select(&idx_tensor, 0)?;
    let entity_labels_subset = batch.entity_labels.index_select(&idx_tensor, 0)?;

    let entity_loss = cross_entropy_loss(&entity_logits_subset, &entity_labels_subset)?;
    let total = (broad_loss + (entity_loss * entity_weight)?)?;
    Ok(total)
}

// ── Header Dropout ───────────────────────────────────────────────────────────

/// Apply header dropout: randomly zero out has_header for a fraction of samples.
///
/// Returns a new `has_header` tensor with some entries set to 0.0.
fn apply_header_dropout(
    has_header: &Tensor,
    dropout_rate: f64,
    rng: &mut impl Rng,
) -> Result<Tensor> {
    let original: Vec<f32> = has_header.to_vec1()?;
    let dropped: Vec<f32> = original
        .iter()
        .map(|&v| {
            if rng.gen::<f64>() < dropout_rate {
                0.0
            } else {
                v
            }
        })
        .collect();
    Ok(Tensor::new(dropped.as_slice(), has_header.device())?)
}

// ── Parameter Counting ───────────────────────────────────────────────────────

/// Count total number of trainable parameters in a VarMap.
fn count_parameters(varmap: &VarMap) -> usize {
    varmap
        .all_vars()
        .iter()
        .map(|v| v.as_tensor().elem_count())
        .sum()
}

// ── Validation ───────────────────────────────────────────────────────────────

/// Run validation pass over dataset, returning (broad_accuracy, entity_accuracy, val_loss).
fn validate(
    model: &SenseModelA,
    dataset: &SenseDataset,
    batch_size: usize,
    entity_weight: f64,
) -> Result<(f32, f32, f32)> {
    let n = dataset.len();
    if n == 0 {
        return Ok((0.0, 0.0, 0.0));
    }

    let indices: Vec<Vec<usize>> = (0..n)
        .collect::<Vec<_>>()
        .chunks(batch_size)
        .map(|c| c.to_vec())
        .collect();

    let mut total_broad_correct = 0.0f64;
    let mut total_entity_correct = 0.0f64;
    let mut total_entity_count = 0usize;
    let mut total_loss = 0.0f64;
    let mut total_samples = 0usize;

    for batch_idx in &indices {
        let batch = dataset.batch(batch_idx)?;
        let (broad_logits, entity_logits) = model.forward(
            &batch.value_embeds,
            &batch.mask,
            &batch.header_embeds,
            &batch.has_header,
        )?;

        let bs = batch_idx.len();
        let broad_acc = compute_accuracy(&broad_logits, &batch.broad_labels)?;
        total_broad_correct += broad_acc as f64 * bs as f64;
        total_samples += bs;

        // Entity accuracy for entity-category samples only
        let broad_labels_vec: Vec<u32> = batch.broad_labels.to_vec1()?;
        let entity_indices: Vec<u32> = broad_labels_vec
            .iter()
            .enumerate()
            .filter(|(_, &label)| label == 0)
            .map(|(i, _)| i as u32)
            .collect();

        if !entity_indices.is_empty() {
            let device = entity_logits.device();
            let idx_tensor = Tensor::new(entity_indices.as_slice(), device)?;
            let entity_logits_sub = entity_logits.index_select(&idx_tensor, 0)?;
            let entity_labels_sub = batch.entity_labels.index_select(&idx_tensor, 0)?;
            let ent_acc = compute_accuracy(&entity_logits_sub, &entity_labels_sub)?;
            total_entity_correct += ent_acc as f64 * entity_indices.len() as f64;
            total_entity_count += entity_indices.len();
        }

        let loss = dual_head_loss(&broad_logits, &entity_logits, &batch, entity_weight)?;
        let loss_val: f32 = loss.to_scalar()?;
        total_loss += loss_val as f64 * bs as f64;
    }

    let broad_accuracy = (total_broad_correct / total_samples as f64) as f32;
    let entity_accuracy = if total_entity_count > 0 {
        (total_entity_correct / total_entity_count as f64) as f32
    } else {
        0.0
    };
    let avg_loss = (total_loss / total_samples as f64) as f32;

    Ok((broad_accuracy, entity_accuracy, avg_loss))
}

// ── Main Training Function ───────────────────────────────────────────────────

/// Train the Sense model (Architecture A) and save best checkpoint.
///
/// Steps:
/// 1. Create VarMap + SenseModelA
/// 2. Create AdamW optimizer
/// 3. For each epoch:
///    a. Shuffle training data into batches
///    b. For each batch: forward → dual-head loss → backward_step
///    c. Header dropout: randomly zero has_header for 50% of training samples
///    d. Compute validation broad accuracy + entity accuracy
///    e. Update cosine LR schedule
///    f. Check early stopping on val broad accuracy
/// 4. Save best model (varmap.save) + config.json + results.json
/// 5. Return TrainingSummary
pub fn train_sense(
    config: &SenseTrainConfig,
    train_data: &SenseDataset,
    val_data: &SenseDataset,
) -> Result<TrainingSummary> {
    let device = crate::get_device();
    let mut rng = StdRng::seed_from_u64(config.seed);

    tracing::info!(
        "Starting Sense training: {} train, {} val, {} epochs, batch_size={}, lr={}",
        train_data.len(),
        val_data.len(),
        config.epochs,
        config.batch_size,
        config.lr,
    );

    // 1. Create model
    let varmap = VarMap::new();
    let model = SenseModelA::new(&varmap, &device)?;
    let n_params = count_parameters(&varmap);
    tracing::info!("Model parameters: {}", n_params);

    // 2. Create optimizer
    let adamw_params = ParamsAdamW {
        lr: config.lr,
        weight_decay: config.weight_decay,
        ..Default::default()
    };
    let mut optimizer = AdamW::new(varmap.all_vars(), adamw_params)?;

    // 3. Setup scheduler + early stopping
    let scheduler = CosineScheduler::new(config.lr, config.min_lr, config.epochs);
    let mut early_stopping = EarlyStopping::new(config.patience, true);

    // Track best model state for saving
    let mut best_varmap_path: Option<PathBuf> = None;
    let mut best_val_entity_accuracy: f32 = 0.0;

    // Create output dir
    std::fs::create_dir_all(&config.output_dir).with_context(|| {
        format!(
            "Failed to create output dir: {}",
            config.output_dir.display()
        )
    })?;

    let mut epoch_metrics = Vec::new();
    let total_start = std::time::Instant::now();

    for epoch in 0..config.epochs {
        let epoch_start = std::time::Instant::now();

        // Update learning rate via cosine schedule
        let lr = scheduler.lr(epoch);
        optimizer.set_learning_rate(lr);

        // 3a. Shuffle into batches
        let batches = shuffled_batches(train_data.len(), config.batch_size, &mut rng);

        let mut train_loss_sum = 0.0f64;
        let mut train_broad_correct = 0.0f64;
        let mut train_samples = 0usize;

        // 3b. Training loop
        for batch_idx in &batches {
            let mut batch = train_data.batch(batch_idx)?;

            // 3c. Header dropout during training
            batch.has_header =
                apply_header_dropout(&batch.has_header, config.header_dropout, &mut rng)?;

            let (broad_logits, entity_logits) = model.forward(
                &batch.value_embeds,
                &batch.mask,
                &batch.header_embeds,
                &batch.has_header,
            )?;

            // 3d. Dual-head loss
            let loss = dual_head_loss(
                &broad_logits,
                &entity_logits,
                &batch,
                config.entity_loss_weight,
            )?;

            // Backward step
            optimizer.backward_step(&loss)?;

            let bs = batch_idx.len();
            let loss_val: f32 = loss.to_scalar()?;
            train_loss_sum += loss_val as f64 * bs as f64;
            let broad_acc = compute_accuracy(&broad_logits, &batch.broad_labels)?;
            train_broad_correct += broad_acc as f64 * bs as f64;
            train_samples += bs;
        }

        let train_loss = (train_loss_sum / train_samples as f64) as f32;
        let train_accuracy = (train_broad_correct / train_samples as f64) as f32;

        // 3e. Validation (no header dropout)
        let (val_broad_acc, val_entity_acc, val_loss) = validate(
            &model,
            val_data,
            config.batch_size,
            config.entity_loss_weight,
        )?;

        let epoch_time = epoch_start.elapsed().as_secs_f32();

        epoch_metrics.push(crate::training::EpochMetrics {
            epoch,
            train_loss,
            val_loss,
            train_accuracy,
            val_accuracy: val_broad_acc,
            learning_rate: lr,
            epoch_time_secs: epoch_time,
        });

        tracing::info!(
            "Epoch {:>3}/{}: train_loss={:.4} val_loss={:.4} train_acc={:.3} val_broad={:.3} val_entity={:.3} lr={:.2e} ({:.1}s)",
            epoch + 1,
            config.epochs,
            train_loss,
            val_loss,
            train_accuracy,
            val_broad_acc,
            val_entity_acc,
            lr,
            epoch_time,
        );

        // 3f. Early stopping on val broad accuracy
        let should_stop = early_stopping.step(epoch, val_broad_acc);

        // Save best model checkpoint
        if early_stopping.best_epoch() == epoch {
            let checkpoint_path = config.output_dir.join("model_best.safetensors");
            varmap.save(&checkpoint_path)?;
            best_varmap_path = Some(checkpoint_path);
            best_val_entity_accuracy = val_entity_acc;
            tracing::info!("  -> New best model saved (val_broad={:.3})", val_broad_acc);
        }

        // 3g. Check early stopping
        if should_stop {
            tracing::info!(
                "Early stopping at epoch {} (best epoch {})",
                epoch + 1,
                early_stopping.best_epoch() + 1,
            );
            break;
        }
    }

    let total_time = total_start.elapsed().as_secs_f32();

    // 4. Rename best checkpoint to final name
    let final_model_path = config.output_dir.join("model.safetensors");
    if let Some(best_path) = &best_varmap_path {
        if best_path != &final_model_path {
            std::fs::rename(best_path, &final_model_path).with_context(|| {
                format!(
                    "Failed to rename {} to {}",
                    best_path.display(),
                    final_model_path.display()
                )
            })?;
        }
    }

    // Save config.json (matching Python format)
    let model_config = SenseModelConfig {
        architecture: "A".to_string(),
        embed_dim: EMBED_DIM,
        hidden_dim: HIDDEN_DIM,
        n_broad: N_BROAD,
        n_entity: N_ENTITY,
        n_params,
        best_epoch: early_stopping.best_epoch(),
        val_broad_accuracy: early_stopping.best_metric(),
        val_entity_accuracy: best_val_entity_accuracy,
        training_config: SenseTrainConfigSnapshot {
            epochs: config.epochs,
            batch_size: config.batch_size,
            lr: config.lr,
            patience: config.patience,
        },
    };

    let config_path = config.output_dir.join("config.json");
    let config_json = serde_json::to_string_pretty(&model_config)?;
    std::fs::write(&config_path, &config_json)
        .with_context(|| format!("Failed to write config.json to {}", config_path.display()))?;

    // Save results.json (epoch-level metrics)
    let results_path = config.output_dir.join("results.json");
    let results_json = serde_json::to_string_pretty(&epoch_metrics)?;
    std::fs::write(&results_path, &results_json)
        .with_context(|| format!("Failed to write results.json to {}", results_path.display()))?;

    let total_epochs = epoch_metrics.len();

    tracing::info!(
        "Training complete: best_epoch={}, val_broad={:.3}, val_entity={:.3}, {:.1}s total",
        early_stopping.best_epoch() + 1,
        early_stopping.best_metric(),
        best_val_entity_accuracy,
        total_time,
    );

    Ok(TrainingSummary {
        best_epoch: early_stopping.best_epoch(),
        best_val_accuracy: early_stopping.best_metric(),
        total_epochs,
        total_time_secs: total_time,
        epoch_metrics,
    })
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::ColumnSample;
    use crate::sense::EMBED_DIM;
    use candle_core::{DType, Device};
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};

    /// Create synthetic ColumnSample entries for testing.
    fn make_synthetic_samples(n: usize, seed: u64) -> Vec<ColumnSample> {
        let mut rng = StdRng::seed_from_u64(seed);
        let mut samples = Vec::with_capacity(n);

        for i in 0..n {
            let broad_idx = i % N_BROAD;
            let entity_idx = i % N_ENTITY;

            // Create slightly separable embeddings per class (bias toward class index)
            let header_embed: Vec<f32> = (0..EMBED_DIM)
                .map(|d| {
                    let base: f32 = rng.gen::<f32>() * 0.5;
                    if d % N_BROAD == broad_idx {
                        base + 1.0
                    } else {
                        base
                    }
                })
                .collect();

            let n_values = 5;
            let value_embeds: Vec<Vec<f32>> = (0..n_values)
                .map(|_| {
                    (0..EMBED_DIM)
                        .map(|d| {
                            let base: f32 = rng.gen::<f32>() * 0.5;
                            if d % N_BROAD == broad_idx {
                                base + 0.8
                            } else {
                                base
                            }
                        })
                        .collect()
                })
                .collect();

            samples.push(ColumnSample {
                header: Some(format!("header_{}", i)),
                values: (0..n_values).map(|v| format!("val_{}_{}", i, v)).collect(),
                header_embed: Some(header_embed),
                value_embeds,
                broad_category_idx: broad_idx,
                entity_subtype_idx: entity_idx,
                broad_category: crate::sense::BROAD_CATEGORIES[broad_idx].to_string(),
                entity_subtype: crate::sense::ENTITY_SUBTYPES[entity_idx].to_string(),
            });
        }

        samples
    }

    #[test]
    fn test_train_sense_fixture_loss_decreases() {
        let train_samples = make_synthetic_samples(50, 42);
        let val_samples = make_synthetic_samples(20, 99);

        let train_data = SenseDataset::from_samples(train_samples);
        let val_data = SenseDataset::from_samples(val_samples);

        let tmp_dir = tempfile::tempdir().unwrap();

        let config = SenseTrainConfig {
            output_dir: tmp_dir.path().to_path_buf(),
            epochs: 5,
            batch_size: 16,
            lr: 1e-3,
            weight_decay: 0.01,
            min_lr: 1e-6,
            patience: 10, // no early stopping in 5 epochs
            seed: 42,
            header_dropout: 0.5,
            entity_loss_weight: 0.5,
        };

        let summary = train_sense(&config, &train_data, &val_data).unwrap();

        // Verify we trained the expected number of epochs
        assert_eq!(summary.total_epochs, 5);
        assert_eq!(summary.epoch_metrics.len(), 5);

        // Loss should decrease between epoch 0 and epoch 4
        let loss_0 = summary.epoch_metrics[0].train_loss;
        let loss_4 = summary.epoch_metrics[4].train_loss;
        assert!(
            loss_4 < loss_0,
            "Training loss should decrease: epoch 0 = {}, epoch 4 = {}",
            loss_0,
            loss_4,
        );

        // Verify model artifacts saved
        let model_path = tmp_dir.path().join("model.safetensors");
        assert!(model_path.exists(), "model.safetensors should exist");
        let metadata = std::fs::metadata(&model_path).unwrap();
        assert!(
            metadata.len() > 1000,
            "model file should be non-trivial size, got {} bytes",
            metadata.len()
        );

        let config_path = tmp_dir.path().join("config.json");
        assert!(config_path.exists(), "config.json should exist");

        // Verify config.json content
        let config_content = std::fs::read_to_string(&config_path).unwrap();
        let model_config: SenseModelConfig = serde_json::from_str(&config_content).unwrap();
        assert_eq!(model_config.architecture, "A");
        assert_eq!(model_config.embed_dim, EMBED_DIM);
        assert_eq!(model_config.n_broad, N_BROAD);
        assert_eq!(model_config.n_entity, N_ENTITY);
        assert!(model_config.n_params > 0);

        let results_path = tmp_dir.path().join("results.json");
        assert!(results_path.exists(), "results.json should exist");
    }

    #[test]
    fn test_dual_head_loss_entity_only_when_entity_category() {
        let device = Device::Cpu;

        // 4 samples: only index 0 and 2 are entity category (broad_label=0)
        let broad_logits = Tensor::randn(0.0f32, 1.0, (4, N_BROAD), &device).unwrap();
        let entity_logits = Tensor::randn(0.0f32, 1.0, (4, N_ENTITY), &device).unwrap();

        let batch = BatchData {
            value_embeds: Tensor::zeros((4, 5, EMBED_DIM), DType::F32, &device).unwrap(),
            mask: Tensor::ones((4, 5), DType::F32, &device).unwrap(),
            header_embeds: Tensor::zeros((4, EMBED_DIM), DType::F32, &device).unwrap(),
            has_header: Tensor::ones(4, DType::F32, &device).unwrap(),
            broad_labels: Tensor::new(&[0u32, 1, 0, 3], &device).unwrap(),
            entity_labels: Tensor::new(&[0u32, 0, 2, 0], &device).unwrap(),
        };

        let loss = dual_head_loss(&broad_logits, &entity_logits, &batch, 0.5).unwrap();
        let loss_val: f32 = loss.to_scalar().unwrap();
        assert!(loss_val.is_finite());
        assert!(loss_val > 0.0);
    }

    #[test]
    fn test_dual_head_loss_no_entity_samples() {
        let device = Device::Cpu;

        // No entity samples (all broad_labels > 0)
        let broad_logits = Tensor::randn(0.0f32, 1.0, (3, N_BROAD), &device).unwrap();
        let entity_logits = Tensor::randn(0.0f32, 1.0, (3, N_ENTITY), &device).unwrap();

        let batch = BatchData {
            value_embeds: Tensor::zeros((3, 5, EMBED_DIM), DType::F32, &device).unwrap(),
            mask: Tensor::ones((3, 5), DType::F32, &device).unwrap(),
            header_embeds: Tensor::zeros((3, EMBED_DIM), DType::F32, &device).unwrap(),
            has_header: Tensor::ones(3, DType::F32, &device).unwrap(),
            broad_labels: Tensor::new(&[1u32, 2, 3], &device).unwrap(),
            entity_labels: Tensor::new(&[0u32, 0, 0], &device).unwrap(),
        };

        let loss = dual_head_loss(&broad_logits, &entity_logits, &batch, 0.5).unwrap();
        let loss_val: f32 = loss.to_scalar().unwrap();
        assert!(loss_val.is_finite());
        assert!(loss_val > 0.0);
    }

    #[test]
    fn test_header_dropout() {
        let device = Device::Cpu;
        let has_header = Tensor::ones(100, DType::F32, &device).unwrap();
        let mut rng = StdRng::seed_from_u64(42);

        let dropped = apply_header_dropout(&has_header, 0.5, &mut rng).unwrap();
        let vals: Vec<f32> = dropped.to_vec1().unwrap();

        let n_zeros = vals.iter().filter(|&&v| v == 0.0).count();
        let n_ones = vals.iter().filter(|&&v| v == 1.0).count();

        // With 100 samples and 50% dropout, expect roughly 50 zeros
        assert!(
            n_zeros > 20,
            "Expected significant dropout, got {} zeros",
            n_zeros
        );
        assert!(n_ones > 20, "Expected some preserved, got {} ones", n_ones);
        assert_eq!(n_zeros + n_ones, 100);
    }

    #[test]
    fn test_count_parameters() {
        let varmap = VarMap::new();
        let device = Device::Cpu;
        let _model = SenseModelA::new(&varmap, &device).unwrap();
        let n_params = count_parameters(&varmap);

        // Expected: header_proj (128*128 + 128) + norm (128 + 128)
        //   + broad_fc1 (384*256 + 256) + broad_fc2 (256*128 + 128) + broad_fc3 (128*6 + 6)
        //   + entity_fc1 (384*256 + 256) + entity_fc2 (256*128 + 128) + entity_fc3 (128*4 + 4)
        //   + default_query (128)
        // Total should be in the 300k+ range
        assert!(
            n_params > 200_000,
            "Expected >200k params, got {}",
            n_params
        );
        assert!(
            n_params < 500_000,
            "Expected <500k params, got {}",
            n_params
        );
    }
}
