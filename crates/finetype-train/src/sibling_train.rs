//! Sibling-context attention training loop.
//!
//! Trains the attention module (396,800 params) to improve Sense classification
//! by enriching column embeddings with cross-column context. Model2Vec and Sense
//! weights are frozen — only attention parameters are updated.
//!
//! Training signal: cross-entropy loss between Sense predictions (using attended
//! embeddings) and silver Sense labels from FineType's own profiling.

use anyhow::{Context, Result};
use candle_core::{Device, Tensor};
use candle_nn::{AdamW, Optimizer, ParamsAdamW, VarMap};
use rand::rngs::StdRng;
use rand::SeedableRng;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::sense::{FrozenSense, EMBED_DIM, MAX_VALUES};
use crate::sibling_context::SiblingContextTrainable;
use crate::sibling_data::{SiblingDataset, TableSample};
use crate::training::{
    compute_accuracy, cross_entropy_loss, CosineScheduler, EarlyStopping, EpochMetrics,
    TrainingSummary,
};
use finetype_model::sibling_context::SiblingContextConfig;

// ── Configuration ────────────────────────────────────────────────────────────

/// Training hyperparameters for sibling-context attention.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiblingTrainConfig {
    /// Output directory for trained model artifacts.
    pub output_dir: PathBuf,
    /// Path to frozen Sense model (directory with model.safetensors + config.json).
    pub sense_model_dir: PathBuf,
    /// Maximum training epochs.
    pub epochs: usize,
    /// Initial learning rate (AdamW).
    pub lr: f64,
    /// AdamW weight decay.
    pub weight_decay: f64,
    /// Minimum learning rate floor for cosine scheduler.
    pub min_lr: f64,
    /// Early stopping patience (epochs without val accuracy improvement).
    pub patience: usize,
    /// Random seed for reproducibility.
    pub seed: u64,
    /// Gradient accumulation steps (number of tables before optimizer step).
    pub grad_accum_steps: usize,
}

impl Default for SiblingTrainConfig {
    fn default() -> Self {
        Self {
            output_dir: PathBuf::from("models/sibling-context"),
            sense_model_dir: PathBuf::from("models/sense"),
            epochs: 100,
            lr: 1e-4,
            weight_decay: 0.01,
            min_lr: 1e-6,
            patience: 15,
            seed: 42,
            grad_accum_steps: 4,
        }
    }
}

/// Model config saved alongside artifacts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiblingModelConfig {
    pub embed_dim: usize,
    pub n_heads: usize,
    pub n_layers: usize,
    pub n_params: usize,
    pub best_epoch: usize,
    pub val_accuracy: f32,
    pub training_config: SiblingTrainConfigSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiblingTrainConfigSnapshot {
    pub epochs: usize,
    pub lr: f64,
    pub patience: usize,
    pub grad_accum_steps: usize,
}

// ── Forward Pass Helpers ─────────────────────────────────────────────────────

/// Run one table through: attention → frozen Sense → logits.
///
/// Returns (broad_logits [N_cols, 6], broad_labels [N_cols]).
fn table_forward(
    table: &TableSample,
    attn_model: &SiblingContextTrainable,
    sense_model: &FrozenSense,
    device: &Device,
) -> Result<(Tensor, Tensor)> {
    let n_cols = table.columns.len();

    // 1. Collect header embeddings → [N_cols, 128]
    let header_flat: Vec<f32> = table
        .columns
        .iter()
        .flat_map(|c| c.header_embed.iter().copied())
        .collect();
    let header_embeds = Tensor::new(header_flat.as_slice(), device)?
        .reshape((n_cols, EMBED_DIM))?;

    // 2. Run attention: [N_cols, 128] → [N_cols, 128]
    let enriched_headers = attn_model.forward(&header_embeds)?;

    // 3. Prepare Sense batch: value_embeds [N_cols, MAX_VALUES, 128], mask, has_header
    let mut value_flat = Vec::with_capacity(n_cols * MAX_VALUES * EMBED_DIM);
    let mut mask_flat = Vec::with_capacity(n_cols * MAX_VALUES);
    let mut has_header = Vec::with_capacity(n_cols);

    for col in &table.columns {
        has_header.push(1.0f32);
        let n_vals = col.value_embeds.len().min(MAX_VALUES);

        // Copy actual value embeddings
        for vi in 0..n_vals {
            value_flat.extend_from_slice(&col.value_embeds[vi]);
            mask_flat.push(if col.value_mask.get(vi).copied().unwrap_or(false) {
                1.0f32
            } else {
                0.0
            });
        }
        // Pad to MAX_VALUES
        let pad_count = MAX_VALUES - n_vals;
        value_flat.extend(std::iter::repeat_n(0.0f32, pad_count * EMBED_DIM));
        mask_flat.extend(std::iter::repeat_n(0.0f32, pad_count));
    }

    let value_embeds =
        Tensor::new(value_flat.as_slice(), device)?.reshape((n_cols, MAX_VALUES, EMBED_DIM))?;
    let mask = Tensor::new(mask_flat.as_slice(), device)?.reshape((n_cols, MAX_VALUES))?;
    let has_header_tensor = Tensor::new(has_header.as_slice(), device)?;

    // 4. Frozen Sense forward: [N_cols, 6] logits
    let (broad_logits, _entity_logits) =
        sense_model.forward(&value_embeds, &mask, &enriched_headers, &has_header_tensor)?;

    // 5. Collect labels
    let labels: Vec<u32> = table
        .columns
        .iter()
        .map(|c| c.broad_category_idx as u32)
        .collect();
    let broad_labels = Tensor::new(labels.as_slice(), device)?;

    Ok((broad_logits, broad_labels))
}

// ── Validation ───────────────────────────────────────────────────────────────

/// Run validation over all tables, returning (accuracy, avg_loss).
fn validate(
    tables: &[TableSample],
    attn_model: &SiblingContextTrainable,
    sense_model: &FrozenSense,
    device: &Device,
) -> Result<(f32, f32)> {
    let mut total_correct = 0.0f64;
    let mut total_loss = 0.0f64;
    let mut total_cols = 0usize;

    for table in tables {
        if table.columns.is_empty() {
            continue;
        }
        let (logits, labels) = table_forward(table, attn_model, sense_model, device)?;
        let n = table.columns.len();

        let acc = compute_accuracy(&logits, &labels)?;
        total_correct += acc as f64 * n as f64;

        let loss: f32 = cross_entropy_loss(&logits, &labels)?.to_scalar()?;
        total_loss += loss as f64 * n as f64;

        total_cols += n;
    }

    if total_cols == 0 {
        return Ok((0.0, 0.0));
    }

    Ok((
        (total_correct / total_cols as f64) as f32,
        (total_loss / total_cols as f64) as f32,
    ))
}

// ── Training Loop ────────────────────────────────────────────────────────────

/// Train the sibling-context attention module.
///
/// Frozen Sense model loaded from `config.sense_model_dir`. Only attention
/// parameters (in `attn_varmap`) are updated by the optimizer.
pub fn train_sibling_context(
    config: &SiblingTrainConfig,
    train_data: &SiblingDataset,
    val_data: &SiblingDataset,
) -> Result<TrainingSummary> {
    let device = crate::get_device();

    tracing::info!(
        "Starting sibling-context training: {} train tables ({} cols), {} val tables ({} cols), {} epochs, lr={}",
        train_data.len(),
        train_data.total_columns(),
        val_data.len(),
        val_data.total_columns(),
        config.epochs,
        config.lr,
    );

    // 1. Create trainable attention model
    let attn_varmap = VarMap::new();
    let attn_config = SiblingContextConfig::default();
    let attn_model = SiblingContextTrainable::new(&attn_varmap, &attn_config, &device)?;
    let n_params = attn_model.param_count();
    tracing::info!("Attention parameters: {}", n_params);

    // 2. Load frozen Sense model as constant tensors (not Var-backed).
    // This is critical: VarMap-backed Vars act as leaf nodes in Candle's autograd,
    // blocking gradient flow to upstream attention variables. Loading weights as
    // constant tensors makes Sense gradient-transparent — gradients flow through
    // its operations back to the trainable attention parameters.
    let sense_path = config.sense_model_dir.join("model.safetensors");
    let sense_model = FrozenSense::load(&sense_path, &device).with_context(|| {
        format!(
            "Failed to load frozen Sense model from {}",
            sense_path.display()
        )
    })?;
    tracing::info!(
        "Loaded frozen Sense model (constant tensors) from {}",
        config.sense_model_dir.display()
    );

    // 3. Create optimizer — only attention vars, Sense is frozen
    let adamw_params = ParamsAdamW {
        lr: config.lr,
        weight_decay: config.weight_decay,
        ..Default::default()
    };
    let mut optimizer = AdamW::new(attn_varmap.all_vars(), adamw_params)?;

    // 4. Setup scheduler + early stopping
    let scheduler = CosineScheduler::new(config.lr, config.min_lr, config.epochs);
    let mut early_stopping = EarlyStopping::new(config.patience, true);

    // Create output directory
    std::fs::create_dir_all(&config.output_dir).with_context(|| {
        format!(
            "Failed to create output dir: {}",
            config.output_dir.display()
        )
    })?;

    let mut best_varmap_path: Option<PathBuf> = None;
    let mut epoch_metrics = Vec::new();
    let total_start = std::time::Instant::now();

    // Shuffled table indices
    let mut rng = StdRng::seed_from_u64(config.seed);

    for epoch in 0..config.epochs {
        let epoch_start = std::time::Instant::now();

        // Update learning rate
        let lr = scheduler.lr(epoch);
        optimizer.set_learning_rate(lr);

        // Shuffle table order
        use rand::seq::SliceRandom;
        let mut table_indices: Vec<usize> = (0..train_data.len()).collect();
        table_indices.shuffle(&mut rng);

        let mut train_loss_sum = 0.0f64;
        let mut train_correct = 0.0f64;
        let mut train_cols = 0usize;
        let mut accum_loss: Option<Tensor> = None;
        let mut accum_count = 0usize;

        for (step, &table_idx) in table_indices.iter().enumerate() {
            let table = &train_data.tables[table_idx];
            if table.columns.is_empty() {
                continue;
            }

            let (logits, labels) = table_forward(table, &attn_model, &sense_model, &device)?;
            let n = table.columns.len();

            let loss = cross_entropy_loss(&logits, &labels)?;
            let loss_val: f32 = loss.to_scalar()?;
            train_loss_sum += loss_val as f64 * n as f64;

            let acc = compute_accuracy(&logits, &labels)?;
            train_correct += acc as f64 * n as f64;
            train_cols += n;

            // Gradient accumulation
            accum_loss = Some(match accum_loss {
                Some(prev) => (prev + loss)?,
                None => loss,
            });
            accum_count += 1;

            // Step optimizer after accumulating `grad_accum_steps` tables
            if accum_count >= config.grad_accum_steps || step == table_indices.len() - 1 {
                if let Some(total_loss) = accum_loss.take() {
                    let avg_loss = (total_loss / accum_count as f64)?;
                    optimizer.backward_step(&avg_loss)?;
                }
                accum_count = 0;
            }
        }

        let train_loss = if train_cols > 0 {
            (train_loss_sum / train_cols as f64) as f32
        } else {
            0.0
        };
        let train_accuracy = if train_cols > 0 {
            (train_correct / train_cols as f64) as f32
        } else {
            0.0
        };

        // Validation
        let (val_accuracy, val_loss) =
            validate(&val_data.tables, &attn_model, &sense_model, &device)?;

        let epoch_time = epoch_start.elapsed().as_secs_f32();

        epoch_metrics.push(EpochMetrics {
            epoch,
            train_loss,
            val_loss,
            train_accuracy,
            val_accuracy,
            learning_rate: lr,
            epoch_time_secs: epoch_time,
        });

        tracing::info!(
            "Epoch {:>3}/{}: train_loss={:.4} val_loss={:.4} train_acc={:.3} val_acc={:.3} lr={:.2e} ({:.1}s)",
            epoch + 1,
            config.epochs,
            train_loss,
            val_loss,
            train_accuracy,
            val_accuracy,
            lr,
            epoch_time,
        );

        // Early stopping on val accuracy
        let should_stop = early_stopping.step(epoch, val_accuracy);

        // Save best checkpoint
        if early_stopping.best_epoch() == epoch {
            let checkpoint_path = config.output_dir.join("model_best.safetensors");
            attn_varmap.save(&checkpoint_path)?;
            best_varmap_path = Some(checkpoint_path);
            tracing::info!("  -> New best model saved (val_acc={:.3})", val_accuracy);
        }

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

    // Rename best checkpoint to final name
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

    // Save config.json (for inference SiblingContextAttention::load)
    let model_config = SiblingModelConfig {
        embed_dim: attn_config.embed_dim,
        n_heads: attn_config.n_heads,
        n_layers: attn_config.n_layers,
        n_params,
        best_epoch: early_stopping.best_epoch(),
        val_accuracy: early_stopping.best_metric(),
        training_config: SiblingTrainConfigSnapshot {
            epochs: config.epochs,
            lr: config.lr,
            patience: config.patience,
            grad_accum_steps: config.grad_accum_steps,
        },
    };

    let config_path = config.output_dir.join("config.json");
    let config_json = serde_json::to_string_pretty(&model_config)?;
    std::fs::write(&config_path, &config_json)
        .with_context(|| format!("Failed to write config.json to {}", config_path.display()))?;

    // Save results.json
    let results_path = config.output_dir.join("results.json");
    let results_json = serde_json::to_string_pretty(&epoch_metrics)?;
    std::fs::write(&results_path, &results_json)
        .with_context(|| format!("Failed to write results.json to {}", results_path.display()))?;

    let total_epochs = epoch_metrics.len();

    tracing::info!(
        "Training complete: best_epoch={}, val_acc={:.3}, {:.1}s total",
        early_stopping.best_epoch() + 1,
        early_stopping.best_metric(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sense::{SenseModelA, N_BROAD};
    use crate::sibling_data::SiblingColumn;

    /// Create synthetic table samples for testing.
    fn make_synthetic_tables(n_tables: usize, seed: u64) -> Vec<TableSample> {
        use rand::Rng;
        let mut rng = StdRng::seed_from_u64(seed);

        (0..n_tables)
            .map(|t| {
                let n_cols = rng.gen_range(2..=8);
                let columns: Vec<SiblingColumn> = (0..n_cols)
                    .map(|c| {
                        let broad_idx = c % N_BROAD;
                        // Create slightly class-biased embeddings
                        let header_embed: Vec<f32> = (0..EMBED_DIM)
                            .map(|d| {
                                let base: f32 = rng.gen::<f32>() * 0.3;
                                if d % N_BROAD == broad_idx {
                                    base + 1.0
                                } else {
                                    base
                                }
                            })
                            .collect();

                        let n_vals = 5;
                        let value_embeds: Vec<Vec<f32>> = (0..n_vals)
                            .map(|_| {
                                (0..EMBED_DIM)
                                    .map(|d| {
                                        let base: f32 = rng.gen::<f32>() * 0.3;
                                        if d % N_BROAD == broad_idx {
                                            base + 0.8
                                        } else {
                                            base
                                        }
                                    })
                                    .collect()
                            })
                            .collect();

                        SiblingColumn {
                            header: format!("col_{}_{}", t, c),
                            header_embed,
                            value_embeds,
                            value_mask: vec![true; n_vals],
                            broad_category_idx: broad_idx,
                            entity_subtype_idx: 0,
                        }
                    })
                    .collect();

                TableSample {
                    table_id: format!("table_{}.csv", t),
                    columns,
                }
            })
            .collect()
    }

    /// Helper: save random Sense weights to a temp file, return path.
    fn save_random_sense(device: &Device) -> (tempfile::TempDir, std::path::PathBuf) {
        let sense_varmap = VarMap::new();
        let _sense = SenseModelA::new(&sense_varmap, device).unwrap();
        let tmp_dir = tempfile::tempdir().unwrap();
        let path = tmp_dir.path().join("model.safetensors");
        sense_varmap.save(&path).unwrap();
        (tmp_dir, path)
    }

    #[test]
    fn test_table_forward_shapes() {
        let device = Device::Cpu;

        let attn_varmap = VarMap::new();
        let attn_config = SiblingContextConfig::default();
        let attn_model =
            SiblingContextTrainable::new(&attn_varmap, &attn_config, &device).unwrap();

        let (_tmp, sense_path) = save_random_sense(&device);
        let sense_model = FrozenSense::load(&sense_path, &device).unwrap();

        let tables = make_synthetic_tables(1, 42);
        let table = &tables[0];
        let n_cols = table.columns.len();

        let (logits, labels) =
            table_forward(table, &attn_model, &sense_model, &device).unwrap();
        assert_eq!(logits.dims(), &[n_cols, N_BROAD]);
        assert_eq!(labels.dims(), &[n_cols]);
    }

    #[test]
    fn test_end_to_end_pipeline() {
        let device = Device::Cpu;

        let train_tables = make_synthetic_tables(5, 42);
        let train_data = SiblingDataset::from_tables(train_tables);

        let attn_varmap = VarMap::new();
        let attn_config = SiblingContextConfig::default();
        let attn_model =
            SiblingContextTrainable::new(&attn_varmap, &attn_config, &device).unwrap();

        let (_tmp, sense_path) = save_random_sense(&device);
        let sense_model = FrozenSense::load(&sense_path, &device).unwrap();

        for table in &train_data.tables {
            if table.columns.is_empty() {
                continue;
            }
            let (logits, labels) =
                table_forward(table, &attn_model, &sense_model, &device).unwrap();
            let n = table.columns.len();
            assert_eq!(logits.dims(), &[n, N_BROAD]);
            assert_eq!(labels.dims(), &[n]);

            let loss = cross_entropy_loss(&logits, &labels).unwrap();
            let loss_val: f32 = loss.to_scalar().unwrap();
            assert!(loss_val.is_finite(), "Loss should be finite");
            assert!(loss_val > 0.0, "Loss should be positive");
        }
    }

    /// Verify gradients flow from loss through frozen Sense to attention vars.
    ///
    /// This is the critical test: FrozenSense uses constant tensors (not Vars),
    /// so Candle's autograd treats them as pass-through nodes. Gradients must
    /// flow back through Sense's computation to the attention variables.
    #[test]
    fn test_gradient_flow_through_frozen_sense() {
        let device = Device::Cpu;

        let attn_varmap = VarMap::new();
        let attn_config = SiblingContextConfig::default();
        let attn_model =
            SiblingContextTrainable::new(&attn_varmap, &attn_config, &device).unwrap();

        let (_tmp, sense_path) = save_random_sense(&device);
        let sense_model = FrozenSense::load(&sense_path, &device).unwrap();

        // Snapshot initial weight
        let initial: Vec<f32> = attn_varmap.all_vars()[0]
            .as_tensor()
            .flatten_all()
            .unwrap()
            .to_vec1()
            .unwrap();

        let tables = make_synthetic_tables(1, 99);
        let table = &tables[0];

        let (logits, labels) =
            table_forward(table, &attn_model, &sense_model, &device).unwrap();
        let loss = cross_entropy_loss(&logits, &labels).unwrap();

        // Check gradients exist for attention vars
        let grads = loss.backward().unwrap();
        let attn_vars = attn_varmap.all_vars();
        let mut has_grad_count = 0;
        for var in &attn_vars {
            if grads.get(var).is_some() {
                has_grad_count += 1;
            }
        }

        // Do optimizer step
        let adamw_params = ParamsAdamW {
            lr: 1e-2,
            weight_decay: 0.0,
            ..Default::default()
        };
        let mut optimizer = AdamW::new(attn_varmap.all_vars(), adamw_params).unwrap();
        optimizer.backward_step(&loss).unwrap();

        let updated: Vec<f32> = attn_varmap.all_vars()[0]
            .as_tensor()
            .flatten_all()
            .unwrap()
            .to_vec1()
            .unwrap();
        let max_diff: f32 = initial
            .iter()
            .zip(updated.iter())
            .map(|(a, b)| (a - b).abs())
            .fold(0.0f32, f32::max);

        assert!(
            has_grad_count > 0,
            "Attention vars should have gradients through FrozenSense (got 0/{})",
            attn_vars.len()
        );
        assert!(
            max_diff > 1e-10,
            "Attention weights should change after backward_step, max_diff={:.6e}",
            max_diff
        );
    }
}
