//! Multi-branch neural network for Sherlock-style column classification.
//!
//! Architecture (Sherlock-inspired):
//! ```text
//! Branch 1 (char):  [960] → Dense(300, ReLU) → Dropout → Dense(300, ReLU) → Dropout → [300]
//! Branch 2 (embed): [512] → Dense(200, ReLU) → Dropout → Dense(200, ReLU) → Dropout → [200]
//! Branch 3 (stats): [27]  → Dense(128, ReLU) → Dropout → Dense(64, ReLU)  → Dropout → [64]
//!                             ↓
//! Merge:             concat([300, 200, 64]) = [564]
//!                             ↓
//!                    BatchNorm → Dense(500, ReLU) → Dropout → Dense(500, ReLU) → Dropout
//!                             ↓
//! Head (flat):       Dense(250, softmax)
//! ```
//!
//! Training data is stored in a custom binary format (FTMB) with per-record
//! feature vectors from three extractors: char_distribution, embedding_aggregation,
//! and column_stats.

use anyhow::{bail, Context, Result};
use candle_core::{DType, Device, Tensor};
use candle_nn::{
    batch_norm, linear, BatchNorm, BatchNormConfig, Linear, ModuleT, VarBuilder, VarMap,
};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::path::Path;

// ═══════════════════════════════════════════════════════════════════════════════
// Configuration
// ═══════════════════════════════════════════════════════════════════════════════

/// Classification head type for the multi-branch model.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub enum HeadType {
    /// Standard flat softmax over all classes.
    #[default]
    Flat,
    /// Hierarchical tree softmax (domain → category → leaf type). Not yet implemented.
    Hierarchical,
}

/// Configuration for the multi-branch Sherlock-style model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiBranchConfig {
    /// Input dimension for character distribution features.
    pub char_dim: usize,
    /// Input dimension for embedding aggregation features.
    pub embed_dim: usize,
    /// Input dimension for column statistics features.
    pub stats_dim: usize,
    /// Hidden layer sizes for the character branch (2 layers).
    pub char_hidden: [usize; 2],
    /// Hidden layer sizes for the embedding branch (2 layers).
    pub embed_hidden: [usize; 2],
    /// Hidden layer sizes for the statistics branch (2 layers).
    pub stats_hidden: [usize; 2],
    /// Hidden layer sizes for the merge trunk (2 layers).
    pub merge_hidden: [usize; 2],
    /// Number of output classes.
    pub n_classes: usize,
    /// Dropout probability (applied during training only).
    pub dropout: f32,
    /// Classification head type.
    pub head_type: HeadType,
}

impl Default for MultiBranchConfig {
    fn default() -> Self {
        Self {
            char_dim: 960,
            embed_dim: 512,
            stats_dim: 27,
            char_hidden: [300, 300],
            embed_hidden: [200, 200],
            stats_hidden: [128, 64],
            merge_hidden: [500, 500],
            n_classes: 250,
            dropout: 0.35,
            head_type: HeadType::Flat,
        }
    }
}

impl MultiBranchConfig {
    /// Compute the merged dimension (sum of final branch hidden sizes).
    pub fn merged_dim(&self) -> usize {
        self.char_hidden[1] + self.embed_hidden[1] + self.stats_hidden[1]
    }

    /// Save config to JSON file.
    pub fn save(&self, path: &Path) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Load config from JSON file.
    pub fn load(path: &Path) -> Result<Self> {
        let json = std::fs::read_to_string(path)?;
        let config: Self = serde_json::from_str(&json)?;
        Ok(config)
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Branch weights (2-layer MLP with dropout)
// ═══════════════════════════════════════════════════════════════════════════════

/// A single feature-processing branch: two linear layers with ReLU and dropout.
struct BranchWeights {
    linear1: Linear,
    linear2: Linear,
    dropout: f32,
}

impl BranchWeights {
    fn new(
        input_dim: usize,
        hidden: [usize; 2],
        dropout: f32,
        vb: VarBuilder,
    ) -> candle_core::Result<Self> {
        let linear1 = linear(input_dim, hidden[0], vb.pp("l1"))?;
        let linear2 = linear(hidden[0], hidden[1], vb.pp("l2"))?;
        Ok(Self {
            linear1,
            linear2,
            dropout,
        })
    }

    /// Forward pass: Linear → ReLU → Dropout → Linear → ReLU → Dropout.
    fn forward(&self, x: &Tensor, train: bool) -> candle_core::Result<Tensor> {
        let h = self.linear1.forward_t(x, false)?;
        let h = h.relu()?;
        let h = if train {
            candle_nn::ops::dropout(&h, self.dropout)?
        } else {
            h
        };
        let h = self.linear2.forward_t(&h, false)?;
        let h = h.relu()?;
        if train {
            candle_nn::ops::dropout(&h, self.dropout)
        } else {
            Ok(h)
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Multi-branch model
// ═══════════════════════════════════════════════════════════════════════════════

/// Multi-branch neural network combining character, embedding, and statistics features.
///
/// Three independent branches process their respective feature vectors, then outputs
/// are concatenated and passed through a shared merge trunk with BatchNorm, followed
/// by a classification head.
pub struct MultiBranchModel {
    char_branch: BranchWeights,
    embed_branch: BranchWeights,
    stats_branch: BranchWeights,
    merge_bn: BatchNorm,
    merge_linear1: Linear,
    merge_linear2: Linear,
    head: Linear,
    config: MultiBranchConfig,
}

impl MultiBranchModel {
    /// Create a new multi-branch model with randomly initialized weights.
    pub fn new(config: &MultiBranchConfig, vb: VarBuilder) -> candle_core::Result<Self> {
        if config.head_type != HeadType::Flat {
            candle_core::bail!("Only HeadType::Flat is currently implemented");
        }

        let char_branch = BranchWeights::new(
            config.char_dim,
            config.char_hidden,
            config.dropout,
            vb.pp("char"),
        )?;
        let embed_branch = BranchWeights::new(
            config.embed_dim,
            config.embed_hidden,
            config.dropout,
            vb.pp("embed"),
        )?;
        let stats_branch = BranchWeights::new(
            config.stats_dim,
            config.stats_hidden,
            config.dropout,
            vb.pp("stats"),
        )?;

        let merged_dim = config.merged_dim();
        let merge_bn = batch_norm(merged_dim, BatchNormConfig::default(), vb.pp("merge_bn"))?;
        let merge_linear1 = linear(merged_dim, config.merge_hidden[0], vb.pp("merge_l1"))?;
        let merge_linear2 = linear(
            config.merge_hidden[0],
            config.merge_hidden[1],
            vb.pp("merge_l2"),
        )?;

        let head = linear(config.merge_hidden[1], config.n_classes, vb.pp("head"))?;

        Ok(Self {
            char_branch,
            embed_branch,
            stats_branch,
            merge_bn,
            merge_linear1,
            merge_linear2,
            head,
            config: config.clone(),
        })
    }

    /// Forward pass. Returns logits (pre-softmax) of shape `[B, n_classes]`.
    ///
    /// - `char_feats`: `[B, char_dim]` (960)
    /// - `embed_feats`: `[B, embed_dim]` (512)
    /// - `stats_feats`: `[B, stats_dim]` (27)
    /// - `train`: when true, enables dropout and training-mode BatchNorm
    pub fn forward(
        &self,
        char_feats: &Tensor,
        embed_feats: &Tensor,
        stats_feats: &Tensor,
        train: bool,
    ) -> candle_core::Result<Tensor> {
        // Process each branch independently
        let char_out = self.char_branch.forward(char_feats, train)?;
        let embed_out = self.embed_branch.forward(embed_feats, train)?;
        let stats_out = self.stats_branch.forward(stats_feats, train)?;

        // Concatenate branch outputs: [B, merged_dim]
        let merged = Tensor::cat(&[char_out, embed_out, stats_out], 1)?;

        // BatchNorm expects [B, C, ...] — for 2D input [B, C] we add a dummy spatial dim
        let merged_3d = merged.unsqueeze(2)?; // [B, C, 1]
        let normed_3d = self.merge_bn.forward_t(&merged_3d, train)?;
        let normed = normed_3d.squeeze(2)?; // [B, C]

        // Merge trunk: Dense → ReLU → Dropout → Dense → ReLU → Dropout
        let h = self.merge_linear1.forward_t(&normed, false)?;
        let h = h.relu()?;
        let h = if train {
            candle_nn::ops::dropout(&h, self.config.dropout)?
        } else {
            h
        };
        let h = self.merge_linear2.forward_t(&h, false)?;
        let h = h.relu()?;
        let h = if train {
            candle_nn::ops::dropout(&h, self.config.dropout)?
        } else {
            h
        };

        // Classification head: logits
        self.head.forward_t(&h, false)
    }

    /// Get the model config.
    pub fn config(&self) -> &MultiBranchConfig {
        &self.config
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Training data format (FTMB binary)
// ═══════════════════════════════════════════════════════════════════════════════

/// Magic bytes for the FTMB binary format.
const FTMB_MAGIC: &[u8; 4] = b"FTMB";

/// Current format version.
const FTMB_VERSION: u32 = 1;

/// Header size in bytes (4 magic + 4 version + 8 n_records + 2+2+2 dims + 2 padding).
const FTMB_HEADER_SIZE: usize = 24;

/// A single training record with label and three feature vectors.
#[derive(Debug, Clone)]
pub struct TrainingRecord {
    /// Type label (e.g., "identity.person.email").
    pub label: String,
    /// Character distribution features.
    pub char_features: Vec<f32>,
    /// Embedding aggregation features.
    pub embed_features: Vec<f32>,
    /// Column statistics features.
    pub stats_features: Vec<f32>,
}

/// Write training records to an FTMB binary file.
pub fn write_training_data(
    path: &Path,
    records: &[TrainingRecord],
    char_dim: u16,
    embed_dim: u16,
    stats_dim: u16,
) -> Result<()> {
    let mut file = std::fs::File::create(path)
        .with_context(|| format!("Failed to create training data file: {}", path.display()))?;

    // Write header (24 bytes)
    file.write_all(FTMB_MAGIC)?;
    file.write_all(&FTMB_VERSION.to_le_bytes())?;
    file.write_all(&(records.len() as u64).to_le_bytes())?;
    file.write_all(&char_dim.to_le_bytes())?;
    file.write_all(&embed_dim.to_le_bytes())?;
    file.write_all(&stats_dim.to_le_bytes())?;
    file.write_all(&[0u8; 2])?; // padding

    // Write records
    for record in records {
        let label_bytes = record.label.as_bytes();
        if label_bytes.len() > u16::MAX as usize {
            bail!(
                "Label too long ({} bytes): {}",
                label_bytes.len(),
                record.label
            );
        }
        file.write_all(&(label_bytes.len() as u16).to_le_bytes())?;
        file.write_all(label_bytes)?;

        // Validate feature dimensions
        if record.char_features.len() != char_dim as usize {
            bail!(
                "char_features length {} != expected {}",
                record.char_features.len(),
                char_dim
            );
        }
        if record.embed_features.len() != embed_dim as usize {
            bail!(
                "embed_features length {} != expected {}",
                record.embed_features.len(),
                embed_dim
            );
        }
        if record.stats_features.len() != stats_dim as usize {
            bail!(
                "stats_features length {} != expected {}",
                record.stats_features.len(),
                stats_dim
            );
        }

        // Write features as raw f32 bytes (little-endian)
        for &v in &record.char_features {
            file.write_all(&v.to_le_bytes())?;
        }
        for &v in &record.embed_features {
            file.write_all(&v.to_le_bytes())?;
        }
        for &v in &record.stats_features {
            file.write_all(&v.to_le_bytes())?;
        }
    }

    Ok(())
}

/// FTMB file header.
#[derive(Debug)]
pub struct FtmbHeader {
    pub n_records: u64,
    pub char_dim: u16,
    pub embed_dim: u16,
    pub stats_dim: u16,
}

/// Read the header from an FTMB binary file.
pub fn read_training_header(path: &Path) -> Result<FtmbHeader> {
    let mut file = std::fs::File::open(path)
        .with_context(|| format!("Failed to open training data file: {}", path.display()))?;

    let mut header = [0u8; FTMB_HEADER_SIZE];
    file.read_exact(&mut header)
        .context("Failed to read FTMB header")?;

    if &header[0..4] != FTMB_MAGIC {
        bail!(
            "Invalid FTMB magic: expected {:?}, got {:?}",
            FTMB_MAGIC,
            &header[0..4]
        );
    }

    let version = u32::from_le_bytes(header[4..8].try_into().unwrap());
    if version != FTMB_VERSION {
        bail!("Unsupported FTMB version: {}", version);
    }

    let n_records = u64::from_le_bytes(header[8..16].try_into().unwrap());
    let char_dim = u16::from_le_bytes(header[16..18].try_into().unwrap());
    let embed_dim = u16::from_le_bytes(header[18..20].try_into().unwrap());
    let stats_dim = u16::from_le_bytes(header[20..22].try_into().unwrap());

    Ok(FtmbHeader {
        n_records,
        char_dim,
        embed_dim,
        stats_dim,
    })
}

/// Read all training records from an FTMB binary file.
pub fn read_training_data(path: &Path) -> Result<(FtmbHeader, Vec<TrainingRecord>)> {
    let mut file = std::fs::File::open(path)
        .with_context(|| format!("Failed to open training data file: {}", path.display()))?;

    let mut header_buf = [0u8; FTMB_HEADER_SIZE];
    file.read_exact(&mut header_buf)
        .context("Failed to read FTMB header")?;

    if &header_buf[0..4] != FTMB_MAGIC {
        bail!("Invalid FTMB magic");
    }
    let version = u32::from_le_bytes(header_buf[4..8].try_into().unwrap());
    if version != FTMB_VERSION {
        bail!("Unsupported FTMB version: {}", version);
    }

    let n_records = u64::from_le_bytes(header_buf[8..16].try_into().unwrap());
    let char_dim = u16::from_le_bytes(header_buf[16..18].try_into().unwrap()) as usize;
    let embed_dim = u16::from_le_bytes(header_buf[18..20].try_into().unwrap()) as usize;
    let stats_dim = u16::from_le_bytes(header_buf[20..22].try_into().unwrap()) as usize;

    let header = FtmbHeader {
        n_records,
        char_dim: char_dim as u16,
        embed_dim: embed_dim as u16,
        stats_dim: stats_dim as u16,
    };

    let mut records = Vec::with_capacity(n_records as usize);
    let mut label_len_buf = [0u8; 2];
    let mut f32_buf = [0u8; 4];

    for _ in 0..n_records {
        // Read label
        file.read_exact(&mut label_len_buf)?;
        let label_len = u16::from_le_bytes(label_len_buf) as usize;
        let mut label_buf = vec![0u8; label_len];
        file.read_exact(&mut label_buf)?;
        let label = String::from_utf8(label_buf).context("Invalid UTF-8 in label")?;

        // Read features
        let mut char_features = Vec::with_capacity(char_dim);
        for _ in 0..char_dim {
            file.read_exact(&mut f32_buf)?;
            char_features.push(f32::from_le_bytes(f32_buf));
        }

        let mut embed_features = Vec::with_capacity(embed_dim);
        for _ in 0..embed_dim {
            file.read_exact(&mut f32_buf)?;
            embed_features.push(f32::from_le_bytes(f32_buf));
        }

        let mut stats_features = Vec::with_capacity(stats_dim);
        for _ in 0..stats_dim {
            file.read_exact(&mut f32_buf)?;
            stats_features.push(f32::from_le_bytes(f32_buf));
        }

        records.push(TrainingRecord {
            label,
            char_features,
            embed_features,
            stats_features,
        });
    }

    Ok((header, records))
}

// ═══════════════════════════════════════════════════════════════════════════════
// Trainer
// ═══════════════════════════════════════════════════════════════════════════════

/// Training configuration for the multi-branch model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiBranchTrainConfig {
    /// Output directory for saved model artifacts.
    pub output_dir: std::path::PathBuf,
    /// Maximum training epochs.
    pub epochs: usize,
    /// Batch size.
    pub batch_size: usize,
    /// Initial learning rate (Adam).
    pub lr: f64,
    /// L2 regularization weight (Adam weight_decay).
    pub weight_decay: f64,
    /// Early stopping patience.
    pub patience: usize,
    /// Random seed.
    pub seed: u64,
    /// Minimum learning rate floor for cosine scheduler.
    pub min_lr: f64,
}

impl Default for MultiBranchTrainConfig {
    fn default() -> Self {
        Self {
            output_dir: std::path::PathBuf::from("models/multi-branch-v1"),
            epochs: 50,
            batch_size: 64,
            lr: 1e-4,
            weight_decay: 1e-4,
            patience: 10,
            seed: 42,
            min_lr: 1e-6,
        }
    }
}

/// In-memory dataset for multi-branch training.
pub struct MultiBranchDataset {
    /// Character distribution features, flat [N * char_dim].
    pub char_feats: Vec<f32>,
    /// Embedding aggregation features, flat [N * embed_dim].
    pub embed_feats: Vec<f32>,
    /// Column statistics features, flat [N * stats_dim].
    pub stats_feats: Vec<f32>,
    /// Label indices [N].
    pub labels: Vec<u32>,
    /// Number of samples.
    pub n_samples: usize,
    /// Feature dimensions.
    pub char_dim: usize,
    pub embed_dim: usize,
    pub stats_dim: usize,
}

impl MultiBranchDataset {
    /// Build a dataset from training records and a label-to-index mapping.
    pub fn from_records(
        records: &[TrainingRecord],
        label_to_idx: &std::collections::HashMap<String, u32>,
        char_dim: usize,
        embed_dim: usize,
        stats_dim: usize,
    ) -> Result<Self> {
        let n = records.len();
        let mut char_feats = Vec::with_capacity(n * char_dim);
        let mut embed_feats_flat = Vec::with_capacity(n * embed_dim);
        let mut stats_feats = Vec::with_capacity(n * stats_dim);
        let mut labels = Vec::with_capacity(n);

        for record in records {
            let idx = label_to_idx
                .get(&record.label)
                .copied()
                .with_context(|| format!("Unknown label: {}", record.label))?;
            labels.push(idx);
            char_feats.extend_from_slice(&record.char_features);
            embed_feats_flat.extend_from_slice(&record.embed_features);
            stats_feats.extend_from_slice(&record.stats_features);
        }

        Ok(Self {
            char_feats,
            embed_feats: embed_feats_flat,
            stats_feats,
            labels,
            n_samples: n,
            char_dim,
            embed_dim,
            stats_dim,
        })
    }

    /// Number of samples.
    pub fn len(&self) -> usize {
        self.n_samples
    }

    /// Whether the dataset is empty.
    pub fn is_empty(&self) -> bool {
        self.n_samples == 0
    }

    /// Extract a batch of tensors for the given sample indices.
    pub fn batch(
        &self,
        indices: &[usize],
        device: &Device,
    ) -> candle_core::Result<(Tensor, Tensor, Tensor, Tensor)> {
        let bs = indices.len();

        let mut char_batch = Vec::with_capacity(bs * self.char_dim);
        let mut embed_batch = Vec::with_capacity(bs * self.embed_dim);
        let mut stats_batch = Vec::with_capacity(bs * self.stats_dim);
        let mut label_batch = Vec::with_capacity(bs);

        for &i in indices {
            let char_start = i * self.char_dim;
            char_batch.extend_from_slice(&self.char_feats[char_start..char_start + self.char_dim]);
            let embed_start = i * self.embed_dim;
            embed_batch
                .extend_from_slice(&self.embed_feats[embed_start..embed_start + self.embed_dim]);
            let stats_start = i * self.stats_dim;
            stats_batch
                .extend_from_slice(&self.stats_feats[stats_start..stats_start + self.stats_dim]);
            label_batch.push(self.labels[i]);
        }

        let char_t = Tensor::new(char_batch.as_slice(), device)?.reshape((bs, self.char_dim))?;
        let embed_t = Tensor::new(embed_batch.as_slice(), device)?.reshape((bs, self.embed_dim))?;
        let stats_t = Tensor::new(stats_batch.as_slice(), device)?.reshape((bs, self.stats_dim))?;
        let labels_t = Tensor::new(label_batch.as_slice(), device)?;

        Ok((char_t, embed_t, stats_t, labels_t))
    }
}

/// Count total trainable parameters in a VarMap.
fn count_parameters(varmap: &VarMap) -> usize {
    varmap
        .all_vars()
        .iter()
        .map(|v| v.as_tensor().elem_count())
        .sum()
}

/// Train the multi-branch model.
///
/// Loads feature-vector training data, runs forward/backward passes with Adam,
/// logs loss per epoch, and saves the best model weights in safetensors format.
pub fn train_multi_branch(
    config: &MultiBranchTrainConfig,
    model_config: &MultiBranchConfig,
    train_data: &MultiBranchDataset,
    val_data: &MultiBranchDataset,
) -> Result<crate::training::TrainingSummary> {
    use crate::training::{
        compute_accuracy, shuffled_batches, CosineScheduler, EarlyStopping, EpochMetrics,
    };
    use candle_nn::{AdamW, Optimizer, ParamsAdamW};
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    let device = crate::get_device();
    let mut rng = StdRng::seed_from_u64(config.seed);

    tracing::info!(
        "Starting multi-branch training: {} train, {} val, {} epochs, batch_size={}, lr={}",
        train_data.len(),
        val_data.len(),
        config.epochs,
        config.batch_size,
        config.lr,
    );

    // Create model
    let varmap = VarMap::new();
    let vb = VarBuilder::from_varmap(&varmap, DType::F32, &device);
    let model = MultiBranchModel::new(model_config, vb)?;
    let n_params = count_parameters(&varmap);
    tracing::info!("Model parameters: {}", n_params);
    tracing::info!(
        "Architecture: char [{} → {} → {}] | embed [{} → {} → {}] | stats [{} → {} → {}] | merge [{} → {} → {}] | head → {}",
        model_config.char_dim, model_config.char_hidden[0], model_config.char_hidden[1],
        model_config.embed_dim, model_config.embed_hidden[0], model_config.embed_hidden[1],
        model_config.stats_dim, model_config.stats_hidden[0], model_config.stats_hidden[1],
        model_config.char_hidden[1] + model_config.embed_hidden[1] + model_config.stats_hidden[1],
        model_config.merge_hidden[0], model_config.merge_hidden[1],
        model_config.n_classes,
    );

    // Create optimizer (AdamW with weight_decay for L2 regularization)
    let adamw_params = ParamsAdamW {
        lr: config.lr,
        weight_decay: config.weight_decay,
        ..Default::default()
    };
    let mut optimizer = AdamW::new(varmap.all_vars(), adamw_params)?;

    // Setup scheduler + early stopping
    let scheduler = CosineScheduler::new(config.lr, config.min_lr, config.epochs);
    let mut early_stopping = EarlyStopping::new(config.patience, true);

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

        // Update learning rate
        let lr = scheduler.lr(epoch);
        optimizer.set_learning_rate(lr);

        // Shuffle into batches
        let batches = shuffled_batches(train_data.len(), config.batch_size, &mut rng);

        let mut train_loss_sum = 0.0f64;
        let mut train_correct_sum = 0.0f64;
        let mut train_samples = 0usize;

        // Training loop
        for batch_idx in &batches {
            let (char_t, embed_t, stats_t, labels_t) = train_data.batch(batch_idx, &device)?;

            let logits = model.forward(&char_t, &embed_t, &stats_t, true)?;
            let loss = candle_nn::loss::cross_entropy(&logits, &labels_t)?;

            optimizer.backward_step(&loss)?;

            let bs = batch_idx.len();
            let loss_val: f32 = loss.to_scalar()?;
            train_loss_sum += loss_val as f64 * bs as f64;
            let acc = compute_accuracy(&logits, &labels_t)?;
            train_correct_sum += acc as f64 * bs as f64;
            train_samples += bs;
        }

        let train_loss = (train_loss_sum / train_samples as f64) as f32;
        let train_accuracy = (train_correct_sum / train_samples as f64) as f32;

        // Validation
        let (val_accuracy, val_loss) = {
            let val_indices: Vec<usize> = (0..val_data.len()).collect();
            let val_batches: Vec<Vec<usize>> = val_indices
                .chunks(config.batch_size)
                .map(|c| c.to_vec())
                .collect();

            let mut val_loss_sum = 0.0f64;
            let mut val_correct_sum = 0.0f64;
            let mut val_samples = 0usize;

            for batch_idx in &val_batches {
                let (char_t, embed_t, stats_t, labels_t) = val_data.batch(batch_idx, &device)?;
                let logits = model.forward(&char_t, &embed_t, &stats_t, false)?;
                let loss = candle_nn::loss::cross_entropy(&logits, &labels_t)?;
                let bs = batch_idx.len();
                let loss_val: f32 = loss.to_scalar()?;
                val_loss_sum += loss_val as f64 * bs as f64;
                let acc = compute_accuracy(&logits, &labels_t)?;
                val_correct_sum += acc as f64 * bs as f64;
                val_samples += bs;
            }

            let val_acc = (val_correct_sum / val_samples as f64) as f32;
            let val_loss = (val_loss_sum / val_samples as f64) as f32;
            (val_acc, val_loss)
        };

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

        // Save best model checkpoint
        if early_stopping.best_epoch() == epoch {
            let checkpoint_path = config.output_dir.join("model_best.safetensors");
            varmap.save(&checkpoint_path)?;
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
    let best_path = config.output_dir.join("model_best.safetensors");
    let final_path = config.output_dir.join("model.safetensors");
    if best_path.exists() && best_path != final_path {
        std::fs::rename(&best_path, &final_path)?;
    }

    // Save model config
    model_config.save(&config.output_dir.join("config.json"))?;

    // Save training results
    let results_json = serde_json::to_string_pretty(&epoch_metrics)?;
    std::fs::write(config.output_dir.join("results.json"), &results_json)?;

    let total_epochs = epoch_metrics.len();

    tracing::info!(
        "Training complete: best_epoch={}, val_acc={:.3}, {:.1}s total",
        early_stopping.best_epoch() + 1,
        early_stopping.best_metric(),
        total_time,
    );

    Ok(crate::training::TrainingSummary {
        best_epoch: early_stopping.best_epoch(),
        best_val_accuracy: early_stopping.best_metric(),
        total_epochs,
        total_time_secs: total_time,
        epoch_metrics,
    })
}

// ═══════════════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use candle_core::{DType, Device};
    use candle_nn::{Optimizer, VarMap};

    fn make_config() -> MultiBranchConfig {
        MultiBranchConfig::default()
    }

    #[test]
    fn test_forward_pass_shape() {
        let config = make_config();
        let device = Device::Cpu;
        let varmap = VarMap::new();
        let vb = VarBuilder::from_varmap(&varmap, DType::F32, &device);
        let model = MultiBranchModel::new(&config, vb).unwrap();

        let batch_size = 10;
        let char_feats =
            Tensor::randn(0.0f32, 1.0, (batch_size, config.char_dim), &device).unwrap();
        let embed_feats =
            Tensor::randn(0.0f32, 1.0, (batch_size, config.embed_dim), &device).unwrap();
        let stats_feats =
            Tensor::randn(0.0f32, 1.0, (batch_size, config.stats_dim), &device).unwrap();

        // Training mode
        let logits = model
            .forward(&char_feats, &embed_feats, &stats_feats, true)
            .unwrap();
        assert_eq!(logits.dims(), &[batch_size, config.n_classes]);

        // Eval mode
        let logits = model
            .forward(&char_feats, &embed_feats, &stats_feats, false)
            .unwrap();
        assert_eq!(logits.dims(), &[batch_size, config.n_classes]);
    }

    #[test]
    fn test_gradient_flow() {
        let config = make_config();
        let device = Device::Cpu;
        let varmap = VarMap::new();
        let vb = VarBuilder::from_varmap(&varmap, DType::F32, &device);
        let model = MultiBranchModel::new(&config, vb).unwrap();

        let batch_size = 4;
        let char_feats =
            Tensor::randn(0.0f32, 1.0, (batch_size, config.char_dim), &device).unwrap();
        let embed_feats =
            Tensor::randn(0.0f32, 1.0, (batch_size, config.embed_dim), &device).unwrap();
        let stats_feats =
            Tensor::randn(0.0f32, 1.0, (batch_size, config.stats_dim), &device).unwrap();
        let targets = Tensor::new(&[0u32, 1, 2, 3], &device).unwrap();

        // Capture initial parameter values
        let vars = varmap.all_vars();
        let initial_values: Vec<Vec<f32>> = vars
            .iter()
            .map(|v| {
                v.as_tensor()
                    .flatten_all()
                    .unwrap()
                    .to_vec1::<f32>()
                    .unwrap()
            })
            .collect();

        // Forward + backward
        let logits = model
            .forward(&char_feats, &embed_feats, &stats_feats, true)
            .unwrap();
        let loss = candle_nn::loss::cross_entropy(&logits, &targets).unwrap();

        // Use AdamW to apply gradients
        let adamw_params = candle_nn::ParamsAdamW {
            lr: 0.01,
            ..Default::default()
        };
        let mut optimizer = candle_nn::AdamW::new(varmap.all_vars(), adamw_params).unwrap();
        optimizer.backward_step(&loss).unwrap();

        // Verify parameters changed (gradient flow through all branches)
        let updated_values: Vec<Vec<f32>> = vars
            .iter()
            .map(|v| {
                v.as_tensor()
                    .flatten_all()
                    .unwrap()
                    .to_vec1::<f32>()
                    .unwrap()
            })
            .collect();

        let mut any_changed = false;
        for (initial, updated) in initial_values.iter().zip(updated_values.iter()) {
            if initial != updated {
                any_changed = true;
                break;
            }
        }
        assert!(
            any_changed,
            "At least some parameters should have changed after backward pass"
        );

        // More specifically, check that parameters in each branch changed
        // The VarMap stores vars in insertion order; we check that not all are identical
        let n_changed: usize = initial_values
            .iter()
            .zip(updated_values.iter())
            .filter(|(a, b)| a != b)
            .count();
        assert!(
            n_changed > 5,
            "Expected many parameters to change, only {} changed out of {}",
            n_changed,
            vars.len()
        );
    }

    #[test]
    fn test_config_serialization() {
        let config = MultiBranchConfig {
            char_dim: 960,
            embed_dim: 512,
            stats_dim: 27,
            char_hidden: [300, 300],
            embed_hidden: [200, 200],
            stats_hidden: [128, 64],
            merge_hidden: [500, 500],
            n_classes: 250,
            dropout: 0.35,
            head_type: HeadType::Flat,
        };

        let tmp = tempfile::NamedTempFile::new().unwrap();
        config.save(tmp.path()).unwrap();
        let loaded = MultiBranchConfig::load(tmp.path()).unwrap();

        assert_eq!(config.char_dim, loaded.char_dim);
        assert_eq!(config.embed_dim, loaded.embed_dim);
        assert_eq!(config.stats_dim, loaded.stats_dim);
        assert_eq!(config.char_hidden, loaded.char_hidden);
        assert_eq!(config.embed_hidden, loaded.embed_hidden);
        assert_eq!(config.stats_hidden, loaded.stats_hidden);
        assert_eq!(config.merge_hidden, loaded.merge_hidden);
        assert_eq!(config.n_classes, loaded.n_classes);
        assert!((config.dropout - loaded.dropout).abs() < 1e-6);
        assert_eq!(config.head_type, loaded.head_type);
    }

    #[test]
    fn test_training_data_roundtrip() {
        let records: Vec<TrainingRecord> = (0..10)
            .map(|i| TrainingRecord {
                label: format!("identity.person.type_{}", i),
                char_features: (0..960).map(|j| (i * 960 + j) as f32 * 0.001).collect(),
                embed_features: (0..512).map(|j| (i * 512 + j) as f32 * 0.002).collect(),
                stats_features: (0..27).map(|j| (i * 27 + j) as f32 * 0.1).collect(),
            })
            .collect();

        let tmp = tempfile::NamedTempFile::new().unwrap();
        write_training_data(tmp.path(), &records, 960, 512, 27).unwrap();

        let (header, loaded) = read_training_data(tmp.path()).unwrap();
        assert_eq!(header.n_records, 10);
        assert_eq!(header.char_dim, 960);
        assert_eq!(header.embed_dim, 512);
        assert_eq!(header.stats_dim, 27);
        assert_eq!(loaded.len(), 10);

        for (orig, read) in records.iter().zip(loaded.iter()) {
            assert_eq!(orig.label, read.label);
            assert_eq!(orig.char_features.len(), read.char_features.len());
            assert_eq!(orig.embed_features.len(), read.embed_features.len());
            assert_eq!(orig.stats_features.len(), read.stats_features.len());

            // Verify exact float roundtrip
            for (a, b) in orig.char_features.iter().zip(read.char_features.iter()) {
                assert_eq!(a.to_bits(), b.to_bits(), "char feature mismatch");
            }
            for (a, b) in orig.embed_features.iter().zip(read.embed_features.iter()) {
                assert_eq!(a.to_bits(), b.to_bits(), "embed feature mismatch");
            }
            for (a, b) in orig.stats_features.iter().zip(read.stats_features.iter()) {
                assert_eq!(a.to_bits(), b.to_bits(), "stats feature mismatch");
            }
        }
    }

    #[test]
    fn test_dataset_batch() {
        let records: Vec<TrainingRecord> = (0..5)
            .map(|i| TrainingRecord {
                label: format!("identity.person.type_{}", i % 3),
                char_features: vec![i as f32; 960],
                embed_features: vec![i as f32 * 2.0; 512],
                stats_features: vec![i as f32 * 3.0; 27],
            })
            .collect();

        let mut label_to_idx = std::collections::HashMap::new();
        label_to_idx.insert("identity.person.type_0".to_string(), 0u32);
        label_to_idx.insert("identity.person.type_1".to_string(), 1u32);
        label_to_idx.insert("identity.person.type_2".to_string(), 2u32);

        let dataset =
            MultiBranchDataset::from_records(&records, &label_to_idx, 960, 512, 27).unwrap();
        assert_eq!(dataset.len(), 5);

        let device = Device::Cpu;
        let (char_t, embed_t, stats_t, labels_t) = dataset.batch(&[0, 2, 4], &device).unwrap();
        assert_eq!(char_t.dims(), &[3, 960]);
        assert_eq!(embed_t.dims(), &[3, 512]);
        assert_eq!(stats_t.dims(), &[3, 27]);
        assert_eq!(labels_t.dims(), &[3]);

        let labels: Vec<u32> = labels_t.to_vec1().unwrap();
        assert_eq!(labels, vec![0, 2, 1]); // type_0, type_2, type_1
    }

    #[test]
    fn test_training_loop_small() {
        // Minimal training test: verify loss decreases over a few epochs
        // with a tiny synthetic dataset
        let config = MultiBranchConfig {
            n_classes: 3,
            ..Default::default()
        };

        // Create synthetic data with slightly separable features
        let mut records = Vec::new();
        let labels = ["type_a", "type_b", "type_c"];
        for i in 0..30 {
            let class_idx = i % 3;
            let bias = class_idx as f32;
            records.push(TrainingRecord {
                label: labels[class_idx].to_string(),
                char_features: (0..960)
                    .map(|j| if j % 3 == class_idx { 1.0 + bias } else { 0.1 })
                    .collect(),
                embed_features: (0..512)
                    .map(|j| if j % 3 == class_idx { 1.0 + bias } else { 0.1 })
                    .collect(),
                stats_features: (0..27)
                    .map(|j| if j % 3 == class_idx { 1.0 + bias } else { 0.1 })
                    .collect(),
            });
        }

        let mut label_to_idx = std::collections::HashMap::new();
        label_to_idx.insert("type_a".to_string(), 0u32);
        label_to_idx.insert("type_b".to_string(), 1u32);
        label_to_idx.insert("type_c".to_string(), 2u32);

        let train_data =
            MultiBranchDataset::from_records(&records[..20], &label_to_idx, 960, 512, 27).unwrap();
        let val_data =
            MultiBranchDataset::from_records(&records[20..], &label_to_idx, 960, 512, 27).unwrap();

        let tmp_dir = tempfile::tempdir().unwrap();
        let train_config = MultiBranchTrainConfig {
            output_dir: tmp_dir.path().to_path_buf(),
            epochs: 5,
            batch_size: 10,
            lr: 1e-3,
            weight_decay: 1e-4,
            patience: 10,
            seed: 42,
            min_lr: 1e-6,
        };

        let summary = train_multi_branch(&train_config, &config, &train_data, &val_data).unwrap();

        assert_eq!(summary.total_epochs, 5);
        assert_eq!(summary.epoch_metrics.len(), 5);

        // Loss should decrease
        let loss_0 = summary.epoch_metrics[0].train_loss;
        let loss_4 = summary.epoch_metrics[4].train_loss;
        assert!(
            loss_4 < loss_0,
            "Training loss should decrease: epoch 0 = {}, epoch 4 = {}",
            loss_0,
            loss_4,
        );

        // Model artifacts should exist
        assert!(tmp_dir.path().join("model.safetensors").exists());
        assert!(tmp_dir.path().join("config.json").exists());
        assert!(tmp_dir.path().join("results.json").exists());
    }

    #[test]
    fn test_merged_dim() {
        let config = MultiBranchConfig::default();
        assert_eq!(config.merged_dim(), 300 + 200 + 64); // 564
    }

    #[test]
    fn test_head_type_hierarchical_rejected() {
        let config = MultiBranchConfig {
            head_type: HeadType::Hierarchical,
            ..Default::default()
        };
        let device = Device::Cpu;
        let varmap = VarMap::new();
        let vb = VarBuilder::from_varmap(&varmap, DType::F32, &device);
        let result = MultiBranchModel::new(&config, vb);
        assert!(
            result.is_err(),
            "Hierarchical head should be rejected for now"
        );
    }
}
