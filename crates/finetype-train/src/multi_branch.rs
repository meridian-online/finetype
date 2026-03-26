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
//! Head (hier):       Tree softmax (7 domains → 43 categories → 250 types)
//! ```
//!
//! Training data is stored in a custom binary format (FTMB) with per-record
//! feature vectors from three extractors: char_distribution, embedding_aggregation,
//! and column_stats.

use anyhow::{bail, Context, Result};
use candle_core::{DType, Device, Module, Tensor};
use candle_nn::{
    batch_norm, linear, BatchNorm, BatchNormConfig, Linear, ModuleT, VarBuilder, VarMap,
};
use finetype_model::char_cnn::{HierarchicalHead, HierarchyMap};
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
    /// Input dimension for header embedding features (Model2Vec, 128-dim).
    /// Defaults to 0 for backward compatibility with old configs.
    #[serde(default)]
    pub header_dim: usize,
    /// Hidden layer sizes for the character branch (2 layers).
    pub char_hidden: [usize; 2],
    /// Hidden layer sizes for the embedding branch (2 layers).
    pub embed_hidden: [usize; 2],
    /// Hidden layer sizes for the statistics branch (2 layers).
    pub stats_hidden: [usize; 2],
    /// Hidden layer sizes for the header branch (2 layers).
    /// Defaults to [128, 64] for new models, [0, 0] for old configs.
    #[serde(default)]
    pub header_hidden: [usize; 2],
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
            header_dim: 128,
            char_hidden: [300, 300],
            embed_hidden: [200, 200],
            stats_hidden: [128, 64],
            header_hidden: [128, 64],
            merge_hidden: [500, 500],
            n_classes: 250,
            dropout: 0.35,
            head_type: HeadType::Flat,
        }
    }
}

impl MultiBranchConfig {
    /// Whether the header branch is enabled (header_dim > 0 with valid hidden dims).
    pub fn has_header_branch(&self) -> bool {
        self.header_dim > 0 && self.header_hidden[0] > 0 && self.header_hidden[1] > 0
    }

    /// Compute the merged dimension (sum of final branch hidden sizes).
    pub fn merged_dim(&self) -> usize {
        let base = self.char_hidden[1] + self.embed_hidden[1] + self.stats_hidden[1];
        if self.has_header_branch() {
            base + self.header_hidden[1]
        } else {
            base
        }
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
    input_norm: Option<candle_nn::LayerNorm>,
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
        Self::new_inner(input_dim, hidden, dropout, false, vb)
    }

    /// Create a branch with LayerNorm on the input (stabilises raw embeddings).
    fn new_with_input_norm(
        input_dim: usize,
        hidden: [usize; 2],
        dropout: f32,
        vb: VarBuilder,
    ) -> candle_core::Result<Self> {
        Self::new_inner(input_dim, hidden, dropout, true, vb)
    }

    fn new_inner(
        input_dim: usize,
        hidden: [usize; 2],
        dropout: f32,
        normalize_input: bool,
        vb: VarBuilder,
    ) -> candle_core::Result<Self> {
        let input_norm = if normalize_input {
            Some(candle_nn::layer_norm(
                input_dim,
                candle_nn::LayerNormConfig::default(),
                vb.pp("input_ln"),
            )?)
        } else {
            None
        };
        let linear1 = linear(input_dim, hidden[0], vb.pp("l1"))?;
        let linear2 = linear(hidden[0], hidden[1], vb.pp("l2"))?;
        Ok(Self {
            input_norm,
            linear1,
            linear2,
            dropout,
        })
    }

    /// Forward pass: [LayerNorm →] Linear → ReLU → Dropout → Linear → ReLU → Dropout.
    fn forward(&self, x: &Tensor, train: bool) -> candle_core::Result<Tensor> {
        let x = match &self.input_norm {
            Some(ln) => ln.forward(x)?,
            None => x.clone(),
        };
        let h = self.linear1.forward_t(&x, false)?;
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
/// by a classification head (flat softmax or hierarchical tree softmax).
pub struct MultiBranchModel {
    char_branch: BranchWeights,
    embed_branch: BranchWeights,
    stats_branch: BranchWeights,
    header_branch: Option<BranchWeights>,
    merge_bn: BatchNorm,
    merge_linear1: Linear,
    merge_linear2: Linear,
    /// Flat classification head (250-class softmax). None when hierarchical.
    head: Option<Linear>,
    /// Hierarchical tree softmax head (7→43→250). None when flat.
    hierarchical: Option<HierarchicalHead>,
    config: MultiBranchConfig,
}

impl MultiBranchModel {
    /// Create a new multi-branch model with a flat classification head.
    pub fn new(config: &MultiBranchConfig, vb: VarBuilder) -> candle_core::Result<Self> {
        let (
            char_branch,
            embed_branch,
            stats_branch,
            header_branch,
            merge_bn,
            merge_linear1,
            merge_linear2,
        ) = Self::build_trunk(config, &vb)?;

        let head = linear(config.merge_hidden[1], config.n_classes, vb.pp("head"))?;

        Ok(Self {
            char_branch,
            embed_branch,
            stats_branch,
            header_branch,
            merge_bn,
            merge_linear1,
            merge_linear2,
            head: Some(head),
            hierarchical: None,
            config: config.clone(),
        })
    }

    /// Create a new multi-branch model with a hierarchical tree softmax head.
    ///
    /// `labels` must be the sorted list of all type labels in `domain.category.type` format.
    pub fn new_hierarchical(
        config: &MultiBranchConfig,
        labels: &[String],
        vb: VarBuilder,
    ) -> candle_core::Result<Self> {
        let (
            char_branch,
            embed_branch,
            stats_branch,
            header_branch,
            merge_bn,
            merge_linear1,
            merge_linear2,
        ) = Self::build_trunk(config, &vb)?;

        let hier_head = HierarchicalHead::new(
            config.merge_hidden[1],
            labels,
            vb.pp(HierarchicalHead::VARBUILDER_PREFIX),
        )?;

        Ok(Self {
            char_branch,
            embed_branch,
            stats_branch,
            header_branch,
            merge_bn,
            merge_linear1,
            merge_linear2,
            head: None,
            hierarchical: Some(hier_head),
            config: config.clone(),
        })
    }

    /// Build the shared trunk (branches + merge layers). Used by both constructors.
    #[allow(clippy::type_complexity)]
    fn build_trunk(
        config: &MultiBranchConfig,
        vb: &VarBuilder,
    ) -> candle_core::Result<(
        BranchWeights,
        BranchWeights,
        BranchWeights,
        Option<BranchWeights>,
        BatchNorm,
        Linear,
        Linear,
    )> {
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

        let header_branch = if config.has_header_branch() {
            // Input LayerNorm stabilises raw Model2Vec embeddings which can have
            // large magnitudes. Without this, gradient explosion collapses training
            // when sibling-context enrichment (which includes its own LayerNorm)
            // is not active.
            Some(BranchWeights::new_with_input_norm(
                config.header_dim,
                config.header_hidden,
                config.dropout,
                vb.pp("header"),
            )?)
        } else {
            None
        };

        let merged_dim = config.merged_dim();
        let merge_bn = batch_norm(merged_dim, BatchNormConfig::default(), vb.pp("merge_bn"))?;
        let merge_linear1 = linear(merged_dim, config.merge_hidden[0], vb.pp("merge_l1"))?;
        let merge_linear2 = linear(
            config.merge_hidden[0],
            config.merge_hidden[1],
            vb.pp("merge_l2"),
        )?;

        Ok((
            char_branch,
            embed_branch,
            stats_branch,
            header_branch,
            merge_bn,
            merge_linear1,
            merge_linear2,
        ))
    }

    /// Forward pass through the trunk only (branches → merge → hidden).
    ///
    /// Returns the hidden representation `[B, merge_hidden[1]]` before the classification head.
    /// `header_feats` is optional — pass `None` for old 3-branch models.
    pub fn forward_trunk(
        &self,
        char_feats: &Tensor,
        embed_feats: &Tensor,
        stats_feats: &Tensor,
        header_feats: Option<&Tensor>,
        train: bool,
    ) -> candle_core::Result<Tensor> {
        // Process each branch independently
        let char_out = self.char_branch.forward(char_feats, train)?;
        let embed_out = self.embed_branch.forward(embed_feats, train)?;
        let stats_out = self.stats_branch.forward(stats_feats, train)?;

        // Concatenate branch outputs: [B, merged_dim]
        let merged = if let Some(ref hb) = &self.header_branch {
            let batch_size = char_out.dim(0)?;
            let header_input = match header_feats {
                Some(hf) => hf.clone(),
                None => Tensor::zeros(
                    (batch_size, self.config.header_dim),
                    DType::F32,
                    char_feats.device(),
                )?,
            };
            let header_out = hb.forward(&header_input, train)?;
            Tensor::cat(&[char_out, embed_out, stats_out, header_out], 1)?
        } else {
            Tensor::cat(&[char_out, embed_out, stats_out], 1)?
        };

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
        if train {
            candle_nn::ops::dropout(&h, self.config.dropout)
        } else {
            Ok(h)
        }
    }

    /// Forward pass. Returns output of shape `[B, n_classes]`.
    ///
    /// - For flat head: returns logits (pre-softmax).
    /// - For hierarchical head: returns product probabilities.
    ///
    /// In both cases, `argmax(dim=1)` gives predicted class indices.
    pub fn forward(
        &self,
        char_feats: &Tensor,
        embed_feats: &Tensor,
        stats_feats: &Tensor,
        header_feats: Option<&Tensor>,
        train: bool,
    ) -> candle_core::Result<Tensor> {
        let hidden =
            self.forward_trunk(char_feats, embed_feats, stats_feats, header_feats, train)?;

        if let Some(ref head) = self.head {
            // Flat head: logits
            head.forward_t(&hidden, false)
        } else if let Some(ref hier) = self.hierarchical {
            // Hierarchical head: product probabilities
            hier.forward(&hidden, self.config.n_classes)
        } else {
            candle_core::bail!("No classification head configured");
        }
    }

    /// Forward pass returning per-level logits for hierarchical training loss.
    ///
    /// Returns `(domain_logits, category_logits_per_domain, leaf_logits_per_category)`.
    /// Only available when the model has a hierarchical head.
    #[allow(clippy::type_complexity)]
    pub fn forward_levels(
        &self,
        char_feats: &Tensor,
        embed_feats: &Tensor,
        stats_feats: &Tensor,
        header_feats: Option<&Tensor>,
        train: bool,
    ) -> candle_core::Result<(Tensor, Vec<Tensor>, Vec<Vec<Option<Tensor>>>)> {
        let hidden =
            self.forward_trunk(char_feats, embed_feats, stats_feats, header_feats, train)?;

        match &self.hierarchical {
            Some(ref hier) => hier.forward_levels(&hidden),
            None => candle_core::bail!("forward_levels() requires a hierarchical head"),
        }
    }

    /// Access the hierarchical head (if present).
    pub fn hierarchical_head(&self) -> Option<&HierarchicalHead> {
        self.hierarchical.as_ref()
    }

    /// Whether this model uses a hierarchical classification head.
    pub fn is_hierarchical(&self) -> bool {
        self.hierarchical.is_some()
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

/// Current format version for flat writes (v2 adds header features).
const FTMB_VERSION_V2: u32 = 2;

/// Format version for table-grouped writes (v3 adds sibling headers).
const FTMB_VERSION_V3: u32 = 3;

/// Header size in bytes for v1/v2 (24 bytes).
/// v1: 4 magic + 4 version + 8 n_records + 2+2+2 dims + 2 padding = 24.
/// v2: 4 magic + 4 version + 8 n_records + 2+2+2+2 dims = 24 (header_dim replaces padding).
const FTMB_HEADER_SIZE_V2: usize = 24;

/// Header size in bytes for v3 (28 bytes).
/// v3: 4 magic + 4 version + 8 n_records + 2+2+2+2 dims + 2 n_groups + 2 reserved = 28.
const FTMB_HEADER_SIZE_V3: usize = 28;

/// A single training record with label and feature vectors.
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
    /// Header embedding features (Model2Vec, 128-dim). Empty for v1 data.
    pub header_features: Vec<f32>,
}

/// A group of training records from the same table, with sibling header names.
///
/// Used by FTMB v3 to enable frozen sibling-context attention during training.
/// For v1/v2 data, records form a single group with empty sibling headers.
#[derive(Debug, Clone)]
pub struct TableGroup {
    /// Indices into the flat record list for columns belonging to this table.
    pub record_indices: Vec<usize>,
    /// Header names for all columns in this table (used by sibling-context attention).
    /// Order matches record_indices.
    pub sibling_headers: Vec<String>,
}

/// Write training records to an FTMB v2 binary file.
pub fn write_training_data(
    path: &Path,
    records: &[TrainingRecord],
    char_dim: u16,
    embed_dim: u16,
    stats_dim: u16,
    header_dim: u16,
) -> Result<()> {
    let mut file = std::fs::File::create(path)
        .with_context(|| format!("Failed to create training data file: {}", path.display()))?;

    // Write v2 header (24 bytes): magic + version + n_records + char_dim + embed_dim + stats_dim + header_dim
    file.write_all(FTMB_MAGIC)?;
    file.write_all(&FTMB_VERSION_V2.to_le_bytes())?;
    file.write_all(&(records.len() as u64).to_le_bytes())?;
    file.write_all(&char_dim.to_le_bytes())?;
    file.write_all(&embed_dim.to_le_bytes())?;
    file.write_all(&stats_dim.to_le_bytes())?;
    file.write_all(&header_dim.to_le_bytes())?;

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
        if record.header_features.len() != header_dim as usize {
            bail!(
                "header_features length {} != expected {}",
                record.header_features.len(),
                header_dim
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
        for &v in &record.header_features {
            file.write_all(&v.to_le_bytes())?;
        }
    }

    Ok(())
}

/// FTMB file header.
#[derive(Debug)]
pub struct FtmbHeader {
    pub version: u32,
    pub n_records: u64,
    pub char_dim: u16,
    pub embed_dim: u16,
    pub stats_dim: u16,
    /// Header embedding dimension. 0 for v1 files.
    pub header_dim: u16,
    /// Number of table groups. 0 for v1/v2 files.
    pub n_groups: u16,
}

/// Read the header from an FTMB binary file. Supports v1, v2, and v3.
pub fn read_training_header(path: &Path) -> Result<FtmbHeader> {
    let mut file = std::fs::File::open(path)
        .with_context(|| format!("Failed to open training data file: {}", path.display()))?;

    // Read the common prefix first (24 bytes covers v1/v2 fully)
    let mut header = [0u8; FTMB_HEADER_SIZE_V3];
    // Read at least the v2 header size to determine version
    file.read_exact(&mut header[..FTMB_HEADER_SIZE_V2])
        .context("Failed to read FTMB header")?;

    if &header[0..4] != FTMB_MAGIC {
        bail!(
            "Invalid FTMB magic: expected {:?}, got {:?}",
            FTMB_MAGIC,
            &header[0..4]
        );
    }

    let version = u32::from_le_bytes(header[4..8].try_into().unwrap());
    if version != 1 && version != 2 && version != 3 {
        bail!(
            "Unsupported FTMB version: {} (expected 1, 2, or 3)",
            version
        );
    }

    let n_records = u64::from_le_bytes(header[8..16].try_into().unwrap());
    let char_dim = u16::from_le_bytes(header[16..18].try_into().unwrap());
    let embed_dim = u16::from_le_bytes(header[18..20].try_into().unwrap());
    let stats_dim = u16::from_le_bytes(header[20..22].try_into().unwrap());

    // v2+: the 2 bytes at offset 22 are header_dim instead of padding
    let header_dim = if version >= 2 {
        u16::from_le_bytes(header[22..24].try_into().unwrap())
    } else {
        0
    };

    // v3: read the additional 4 bytes (n_groups + reserved)
    let n_groups = if version >= 3 {
        file.read_exact(&mut header[24..28])
            .context("Failed to read v3 header extension")?;
        u16::from_le_bytes(header[24..26].try_into().unwrap())
    } else {
        0
    };

    Ok(FtmbHeader {
        version,
        n_records,
        char_dim,
        embed_dim,
        stats_dim,
        header_dim,
        n_groups,
    })
}

/// Read all training records from an FTMB binary file. Supports v1, v2, and v3.
///
/// v1 files are loaded with zero-vector header_features (graceful degradation).
/// v1/v2 files return a single table group containing all records with empty sibling headers.
/// v3 files return proper table groups with sibling header metadata.
pub fn read_training_data(
    path: &Path,
) -> Result<(FtmbHeader, Vec<TrainingRecord>, Vec<TableGroup>)> {
    let mut file = std::fs::File::open(path)
        .with_context(|| format!("Failed to open training data file: {}", path.display()))?;

    // Read common header prefix (24 bytes for v1/v2)
    let mut header_buf = [0u8; FTMB_HEADER_SIZE_V3];
    file.read_exact(&mut header_buf[..FTMB_HEADER_SIZE_V2])
        .context("Failed to read FTMB header")?;

    if &header_buf[0..4] != FTMB_MAGIC {
        bail!("Invalid FTMB magic");
    }
    let version = u32::from_le_bytes(header_buf[4..8].try_into().unwrap());
    if version != 1 && version != 2 && version != 3 {
        bail!(
            "Unsupported FTMB version: {} (expected 1, 2, or 3)",
            version
        );
    }

    let n_records = u64::from_le_bytes(header_buf[8..16].try_into().unwrap());
    let char_dim = u16::from_le_bytes(header_buf[16..18].try_into().unwrap()) as usize;
    let embed_dim = u16::from_le_bytes(header_buf[18..20].try_into().unwrap()) as usize;
    let stats_dim = u16::from_le_bytes(header_buf[20..22].try_into().unwrap()) as usize;
    let header_dim = if version >= 2 {
        u16::from_le_bytes(header_buf[22..24].try_into().unwrap()) as usize
    } else {
        0
    };

    // v3: read additional 4 bytes
    let n_groups = if version >= 3 {
        file.read_exact(&mut header_buf[24..28])
            .context("Failed to read v3 header extension")?;
        u16::from_le_bytes(header_buf[24..26].try_into().unwrap()) as usize
    } else {
        0
    };

    let header = FtmbHeader {
        version,
        n_records,
        char_dim: char_dim as u16,
        embed_dim: embed_dim as u16,
        stats_dim: stats_dim as u16,
        header_dim: header_dim as u16,
        n_groups: n_groups as u16,
    };

    let mut records = Vec::with_capacity(n_records as usize);
    let mut table_groups = Vec::new();
    let mut label_len_buf = [0u8; 2];
    let mut f32_buf = [0u8; 4];

    if version >= 3 {
        // v3: read table groups sequentially
        let mut record_offset = 0usize;
        for _ in 0..n_groups {
            // Read group header: n_columns (u16) + n_sibling_headers (u16)
            let mut group_header = [0u8; 4];
            file.read_exact(&mut group_header)?;
            let n_columns = u16::from_le_bytes(group_header[0..2].try_into().unwrap()) as usize;
            let n_sibling_headers =
                u16::from_le_bytes(group_header[2..4].try_into().unwrap()) as usize;

            // Read sibling headers
            let mut sibling_headers = Vec::with_capacity(n_sibling_headers);
            for _ in 0..n_sibling_headers {
                file.read_exact(&mut label_len_buf)?;
                let header_len = u16::from_le_bytes(label_len_buf) as usize;
                let mut header_bytes = vec![0u8; header_len];
                file.read_exact(&mut header_bytes)?;
                sibling_headers
                    .push(String::from_utf8(header_bytes).context("Invalid UTF-8 in header")?);
            }

            let mut group_indices = Vec::with_capacity(n_columns);

            // Read records for this group
            for _ in 0..n_columns {
                // Read label
                file.read_exact(&mut label_len_buf)?;
                let label_len = u16::from_le_bytes(label_len_buf) as usize;
                let mut label_buf = vec![0u8; label_len];
                file.read_exact(&mut label_buf)?;
                let label = String::from_utf8(label_buf).context("Invalid UTF-8 in label")?;

                // Read column_index (u16) — index into sibling_headers
                let mut col_idx_buf = [0u8; 2];
                file.read_exact(&mut col_idx_buf)?;
                // column_index is stored for reference but we use positional order
                let _column_index = u16::from_le_bytes(col_idx_buf);

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
                let mut header_features = Vec::with_capacity(header_dim);
                for _ in 0..header_dim {
                    file.read_exact(&mut f32_buf)?;
                    header_features.push(f32::from_le_bytes(f32_buf));
                }

                group_indices.push(record_offset);
                records.push(TrainingRecord {
                    label,
                    char_features,
                    embed_features,
                    stats_features,
                    header_features,
                });
                record_offset += 1;
            }

            table_groups.push(TableGroup {
                record_indices: group_indices,
                sibling_headers,
            });
        }
    } else {
        // v1/v2: read flat records
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
            // v2: read header features; v1: zero-vector
            let header_features = if version >= 2 && header_dim > 0 {
                let mut hf = Vec::with_capacity(header_dim);
                for _ in 0..header_dim {
                    file.read_exact(&mut f32_buf)?;
                    hf.push(f32::from_le_bytes(f32_buf));
                }
                hf
            } else {
                vec![0.0f32; header_dim]
            };

            records.push(TrainingRecord {
                label,
                char_features,
                embed_features,
                stats_features,
                header_features,
            });
        }

        // v1/v2: single group with all records, empty sibling headers
        if !records.is_empty() {
            table_groups.push(TableGroup {
                record_indices: (0..records.len()).collect(),
                sibling_headers: Vec::new(),
            });
        }
    }

    Ok((header, records, table_groups))
}

/// Write training records to an FTMB v3 binary file with table group metadata.
///
/// Each table group contains records from the same source table, along with
/// sibling header names that enable frozen sibling-context attention during training.
pub fn write_training_data_v3(
    path: &Path,
    records: &[TrainingRecord],
    table_groups: &[TableGroup],
    char_dim: u16,
    embed_dim: u16,
    stats_dim: u16,
    header_dim: u16,
) -> Result<()> {
    let mut file = std::fs::File::create(path)
        .with_context(|| format!("Failed to create training data file: {}", path.display()))?;

    // Count total records across all groups
    let total_records: usize = table_groups.iter().map(|g| g.record_indices.len()).sum();
    if total_records != records.len() {
        bail!(
            "Table groups reference {} records but {} were provided",
            total_records,
            records.len()
        );
    }

    // Write v3 header (28 bytes)
    file.write_all(FTMB_MAGIC)?;
    file.write_all(&FTMB_VERSION_V3.to_le_bytes())?;
    file.write_all(&(records.len() as u64).to_le_bytes())?;
    file.write_all(&char_dim.to_le_bytes())?;
    file.write_all(&embed_dim.to_le_bytes())?;
    file.write_all(&stats_dim.to_le_bytes())?;
    file.write_all(&header_dim.to_le_bytes())?;
    file.write_all(&(table_groups.len() as u16).to_le_bytes())?;
    file.write_all(&[0u8; 2])?; // reserved

    // Write each table group
    for group in table_groups {
        // Group header: n_columns + n_sibling_headers
        file.write_all(&(group.record_indices.len() as u16).to_le_bytes())?;
        file.write_all(&(group.sibling_headers.len() as u16).to_le_bytes())?;

        // Write sibling headers
        for header_name in &group.sibling_headers {
            let header_bytes = header_name.as_bytes();
            if header_bytes.len() > u16::MAX as usize {
                bail!(
                    "Sibling header too long ({} bytes): {}",
                    header_bytes.len(),
                    header_name
                );
            }
            file.write_all(&(header_bytes.len() as u16).to_le_bytes())?;
            file.write_all(header_bytes)?;
        }

        // Write records for this group
        for (col_idx, &record_idx) in group.record_indices.iter().enumerate() {
            let record = &records[record_idx];

            // Label
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

            // Column index into sibling_headers
            file.write_all(&(col_idx as u16).to_le_bytes())?;

            // Validate and write features
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
            if record.header_features.len() != header_dim as usize {
                bail!(
                    "header_features length {} != expected {}",
                    record.header_features.len(),
                    header_dim
                );
            }

            for &v in &record.char_features {
                file.write_all(&v.to_le_bytes())?;
            }
            for &v in &record.embed_features {
                file.write_all(&v.to_le_bytes())?;
            }
            for &v in &record.stats_features {
                file.write_all(&v.to_le_bytes())?;
            }
            for &v in &record.header_features {
                file.write_all(&v.to_le_bytes())?;
            }
        }
    }

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// FrozenSiblingContext — constant tensor version for multi-branch training
// ═══════════════════════════════════════════════════════════════════════════════

/// Frozen sibling-context attention for multi-branch training.
///
/// Loads weights as constant tensors (not Var-backed) so gradients pass through
/// but don't update attention weights. This follows the `FrozenSense` pattern
/// from `crates/finetype-train/src/sense.rs`.
///
/// Architecture: 2-layer pre-norm transformer, 4 heads, 128-dim (matching
/// `SiblingContextAttention` from `finetype-model`).
pub struct FrozenSiblingContext {
    blocks: Vec<FrozenTransformerBlock>,
    final_norm_weight: Tensor,
    final_norm_bias: Tensor,
    embed_dim: usize,
}

struct FrozenTransformerBlock {
    // norm1
    norm1_weight: Tensor,
    norm1_bias: Tensor,
    // attn
    wq: Tensor,
    bq: Tensor,
    wk: Tensor,
    bk: Tensor,
    wv: Tensor,
    bv: Tensor,
    out_weight: Tensor,
    out_bias: Tensor,
    n_heads: usize,
    head_dim: usize,
    // norm2
    norm2_weight: Tensor,
    norm2_bias: Tensor,
    // ffn
    ffn_w1: Tensor,
    ffn_b1: Tensor,
    ffn_w2: Tensor,
    ffn_b2: Tensor,
}

impl FrozenSiblingContext {
    /// Load frozen sibling-context weights from a model directory.
    ///
    /// Expects `model.safetensors` and `config.json` in the directory.
    /// All tensors are loaded as constants (not variables), allowing gradients
    /// to flow through this model's forward pass to upstream trainable parameters.
    pub fn load(model_dir: &Path, device: &Device) -> Result<Self> {
        use finetype_model::sibling_context::SiblingContextConfig;

        let config_bytes =
            std::fs::read(model_dir.join("config.json")).context("Failed to read config.json")?;
        let config: SiblingContextConfig =
            serde_json::from_slice(&config_bytes).context("Failed to parse config.json")?;

        let tensors = candle_core::safetensors::load(model_dir.join("model.safetensors"), device)?;

        let get = |name: &str| -> Result<Tensor> {
            tensors
                .get(name)
                .cloned()
                .ok_or_else(|| {
                    anyhow::anyhow!("Missing tensor '{}' in sibling-context model", name)
                })
                .and_then(|t| Ok(t.to_dtype(DType::F32)?))
        };

        let d = config.embed_dim;
        let n_heads = config.n_heads;
        let head_dim = d / n_heads;

        let mut blocks = Vec::with_capacity(config.n_layers);
        for i in 0..config.n_layers {
            let p = format!("blocks.{}", i);
            blocks.push(FrozenTransformerBlock {
                norm1_weight: get(&format!("{p}.norm1.weight"))?,
                norm1_bias: get(&format!("{p}.norm1.bias"))?,
                wq: get(&format!("{p}.attn.wq"))?,
                bq: get(&format!("{p}.attn.bq"))?,
                wk: get(&format!("{p}.attn.wk"))?,
                bk: get(&format!("{p}.attn.bk"))?,
                wv: get(&format!("{p}.attn.wv"))?,
                bv: get(&format!("{p}.attn.bv"))?,
                out_weight: get(&format!("{p}.attn.out_weight"))?,
                out_bias: get(&format!("{p}.attn.out_bias"))?,
                n_heads,
                head_dim,
                norm2_weight: get(&format!("{p}.norm2.weight"))?,
                norm2_bias: get(&format!("{p}.norm2.bias"))?,
                ffn_w1: get(&format!("{p}.ffn.w1"))?,
                ffn_b1: get(&format!("{p}.ffn.b1"))?,
                ffn_w2: get(&format!("{p}.ffn.w2"))?,
                ffn_b2: get(&format!("{p}.ffn.b2"))?,
            });
        }

        Ok(Self {
            blocks,
            final_norm_weight: get("final_norm.weight")?,
            final_norm_bias: get("final_norm.bias")?,
            embed_dim: d,
        })
    }

    /// Run frozen attention: `[N_cols, 128] -> [N_cols, 128]`.
    ///
    /// Gradients flow transparently through all operations to upstream
    /// trainable parameters (e.g., multi-branch model weights).
    pub fn forward(&self, header_embeds: &Tensor) -> Result<Tensor> {
        let mut out = header_embeds.clone();
        for block in &self.blocks {
            out = block.forward(&out)?;
        }
        // Final layer norm
        frozen_layer_norm(&out, &self.final_norm_weight, &self.final_norm_bias)
    }

    /// Get the embedding dimension.
    pub fn embed_dim(&self) -> usize {
        self.embed_dim
    }
}

impl FrozenTransformerBlock {
    fn forward(&self, x: &Tensor) -> Result<Tensor> {
        // Pre-norm → self-attention → residual
        let normed = frozen_layer_norm(x, &self.norm1_weight, &self.norm1_bias)?;
        let attn_out = self.forward_attn(&normed)?;
        let x = (x + &attn_out)?;

        // Pre-norm → FFN → residual
        let normed = frozen_layer_norm(&x, &self.norm2_weight, &self.norm2_bias)?;
        let ffn_out = self.forward_ffn(&normed)?;
        Ok((&x + &ffn_out)?)
    }

    fn forward_attn(&self, x: &Tensor) -> Result<Tensor> {
        let n = x.dim(0)?;
        let d = x.dim(1)?;
        let h = self.n_heads;
        let hd = self.head_dim;

        let q = x.matmul(&self.wq.t()?)?.broadcast_add(&self.bq)?;
        let k = x.matmul(&self.wk.t()?)?.broadcast_add(&self.bk)?;
        let v = x.matmul(&self.wv.t()?)?.broadcast_add(&self.bv)?;

        // .contiguous() required after transpose — Metal's matmul kernel
        // cannot operate on non-contiguous (strided) tensors. Without this,
        // Metal sees the stride array as the shape and rejects the matmul.
        let q = q.reshape((n, h, hd))?.transpose(0, 1)?.contiguous()?;
        let k = k.reshape((n, h, hd))?.transpose(0, 1)?.contiguous()?;
        let v = v.reshape((n, h, hd))?.transpose(0, 1)?.contiguous()?;

        let scale = (hd as f64).sqrt();
        let attn_weights = (q.matmul(&k.transpose(1, 2)?.contiguous()?)? / scale)?;

        let attn_max = attn_weights.max(2)?.unsqueeze(2)?;
        let shifted = attn_weights.broadcast_sub(&attn_max)?;
        let exp = shifted.exp()?;
        let sum_exp = exp.sum(2)?.unsqueeze(2)?;
        let attn_probs = exp.broadcast_div(&sum_exp)?;

        let attn_out = attn_probs.matmul(&v)?;
        let attn_out = attn_out.transpose(0, 1)?.contiguous()?.reshape((n, d))?;

        Ok(attn_out
            .matmul(&self.out_weight.t()?)?
            .broadcast_add(&self.out_bias)?)
    }

    fn forward_ffn(&self, x: &Tensor) -> Result<Tensor> {
        let h = x.matmul(&self.ffn_w1.t()?)?.broadcast_add(&self.ffn_b1)?;
        let h = h.gelu_erf()?;
        Ok(h.matmul(&self.ffn_w2.t()?)?.broadcast_add(&self.ffn_b2)?)
    }
}

/// Layer normalisation using constant tensors (for frozen models).
fn frozen_layer_norm(x: &Tensor, weight: &Tensor, bias: &Tensor) -> Result<Tensor> {
    let eps = 1e-5_f64;
    let d = x.dim(1)?;
    let mean = (x.sum(1)? / d as f64)?;
    let mean = mean.unsqueeze(1)?;
    let diff = x.broadcast_sub(&mean)?;
    let var = ((&diff * &diff)?.sum(1)? / d as f64)?;
    let std = (var + eps)?.sqrt()?.unsqueeze(1)?;
    let normed = diff.broadcast_div(&std)?;
    Ok(normed.broadcast_mul(weight)?.broadcast_add(bias)?)
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
    /// Header embedding features, flat [N * header_dim].
    pub header_feats: Vec<f32>,
    /// Label indices [N].
    pub labels: Vec<u32>,
    /// Number of samples.
    pub n_samples: usize,
    /// Feature dimensions.
    pub char_dim: usize,
    pub embed_dim: usize,
    pub stats_dim: usize,
    pub header_dim: usize,
    /// Table groups from v3 data. Empty for v1/v2 data (single group with all records).
    pub table_groups: Vec<TableGroup>,
}

impl MultiBranchDataset {
    /// Build a dataset from training records, a label-to-index mapping, and optional table groups.
    ///
    /// When `table_groups` is `None`, a single group spanning all records is created
    /// (backward compatible with v1/v2 data).
    pub fn from_records(
        records: &[TrainingRecord],
        label_to_idx: &std::collections::HashMap<String, u32>,
        char_dim: usize,
        embed_dim: usize,
        stats_dim: usize,
        header_dim: usize,
    ) -> Result<Self> {
        Self::from_records_with_groups(
            records,
            label_to_idx,
            char_dim,
            embed_dim,
            stats_dim,
            header_dim,
            None,
        )
    }

    /// Build a dataset from training records with explicit table group metadata.
    pub fn from_records_with_groups(
        records: &[TrainingRecord],
        label_to_idx: &std::collections::HashMap<String, u32>,
        char_dim: usize,
        embed_dim: usize,
        stats_dim: usize,
        header_dim: usize,
        table_groups: Option<Vec<TableGroup>>,
    ) -> Result<Self> {
        let n = records.len();
        let mut char_feats = Vec::with_capacity(n * char_dim);
        let mut embed_feats_flat = Vec::with_capacity(n * embed_dim);
        let mut stats_feats = Vec::with_capacity(n * stats_dim);
        let mut header_feats = Vec::with_capacity(n * header_dim);
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
            // Pad or truncate header features to expected dim
            if record.header_features.len() >= header_dim {
                header_feats.extend_from_slice(&record.header_features[..header_dim]);
            } else {
                header_feats.extend_from_slice(&record.header_features);
                header_feats.extend(std::iter::repeat_n(
                    0.0f32,
                    header_dim - record.header_features.len(),
                ));
            }
        }

        // Use provided groups or create a single default group
        let groups = table_groups.unwrap_or_else(|| {
            if n > 0 {
                vec![TableGroup {
                    record_indices: (0..n).collect(),
                    sibling_headers: Vec::new(),
                }]
            } else {
                Vec::new()
            }
        });

        Ok(Self {
            char_feats,
            embed_feats: embed_feats_flat,
            stats_feats,
            header_feats,
            labels,
            n_samples: n,
            char_dim,
            embed_dim,
            stats_dim,
            header_dim,
            table_groups: groups,
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
    ///
    /// Returns `(char, embed, stats, header, labels)`. When `header_dim == 0`,
    /// the header tensor is a zero-width `[B, 0]` tensor.
    pub fn batch(
        &self,
        indices: &[usize],
        device: &Device,
    ) -> candle_core::Result<(Tensor, Tensor, Tensor, Option<Tensor>, Tensor)> {
        let bs = indices.len();

        let mut char_batch = Vec::with_capacity(bs * self.char_dim);
        let mut embed_batch = Vec::with_capacity(bs * self.embed_dim);
        let mut stats_batch = Vec::with_capacity(bs * self.stats_dim);
        let mut header_batch = Vec::with_capacity(bs * self.header_dim);
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
            if self.header_dim > 0 {
                let header_start = i * self.header_dim;
                header_batch.extend_from_slice(
                    &self.header_feats[header_start..header_start + self.header_dim],
                );
            }
            label_batch.push(self.labels[i]);
        }

        let char_t = Tensor::new(char_batch.as_slice(), device)?.reshape((bs, self.char_dim))?;
        let embed_t = Tensor::new(embed_batch.as_slice(), device)?.reshape((bs, self.embed_dim))?;
        let stats_t = Tensor::new(stats_batch.as_slice(), device)?.reshape((bs, self.stats_dim))?;
        let header_t = if self.header_dim > 0 {
            Some(Tensor::new(header_batch.as_slice(), device)?.reshape((bs, self.header_dim))?)
        } else {
            None
        };
        let labels_t = Tensor::new(label_batch.as_slice(), device)?;

        Ok((char_t, embed_t, stats_t, header_t, labels_t))
    }

    /// Extract a batch from whole table groups, optionally enriching headers via
    /// frozen sibling-context attention.
    ///
    /// `group_indices` selects which table groups to include. All records from each
    /// selected group are concatenated into a single batch. When `frozen_ctx` is
    /// `Some`, header features for each group are passed through the frozen attention
    /// module before being placed into the batch.
    ///
    /// For groups with empty sibling_headers (v1/v2 data), header features pass through
    /// unchanged regardless of `frozen_ctx`.
    pub fn batch_groups(
        &self,
        group_indices: &[usize],
        frozen_ctx: Option<&FrozenSiblingContext>,
        device: &Device,
    ) -> Result<(Tensor, Tensor, Tensor, Option<Tensor>, Tensor)> {
        // Collect all record indices from the selected groups
        let mut all_indices = Vec::new();
        // Track group boundaries for per-group header enrichment
        let mut group_boundaries: Vec<(usize, usize, usize)> = Vec::new(); // (start, end, group_idx)

        for &gi in group_indices {
            let group = &self.table_groups[gi];
            let start = all_indices.len();
            all_indices.extend_from_slice(&group.record_indices);
            let end = all_indices.len();
            group_boundaries.push((start, end, gi));
        }

        let bs = all_indices.len();
        if bs == 0 {
            // Return empty tensors
            let char_t = Tensor::zeros((0, self.char_dim), DType::F32, device)?;
            let embed_t = Tensor::zeros((0, self.embed_dim), DType::F32, device)?;
            let stats_t = Tensor::zeros((0, self.stats_dim), DType::F32, device)?;
            let header_t = if self.header_dim > 0 {
                Some(Tensor::zeros((0, self.header_dim), DType::F32, device)?)
            } else {
                None
            };
            let labels_t = Tensor::zeros(0, DType::U32, device)?;
            return Ok((char_t, embed_t, stats_t, header_t, labels_t));
        }

        // Build base batch from flat indices
        let mut char_batch = Vec::with_capacity(bs * self.char_dim);
        let mut embed_batch = Vec::with_capacity(bs * self.embed_dim);
        let mut stats_batch = Vec::with_capacity(bs * self.stats_dim);
        let mut header_batch = Vec::with_capacity(bs * self.header_dim);
        let mut label_batch = Vec::with_capacity(bs);

        for &i in &all_indices {
            let char_start = i * self.char_dim;
            char_batch.extend_from_slice(&self.char_feats[char_start..char_start + self.char_dim]);
            let embed_start = i * self.embed_dim;
            embed_batch
                .extend_from_slice(&self.embed_feats[embed_start..embed_start + self.embed_dim]);
            let stats_start = i * self.stats_dim;
            stats_batch
                .extend_from_slice(&self.stats_feats[stats_start..stats_start + self.stats_dim]);
            if self.header_dim > 0 {
                let header_start = i * self.header_dim;
                header_batch.extend_from_slice(
                    &self.header_feats[header_start..header_start + self.header_dim],
                );
            }
            label_batch.push(self.labels[i]);
        }

        let char_t = Tensor::new(char_batch.as_slice(), device)?.reshape((bs, self.char_dim))?;
        let embed_t = Tensor::new(embed_batch.as_slice(), device)?.reshape((bs, self.embed_dim))?;
        let stats_t = Tensor::new(stats_batch.as_slice(), device)?.reshape((bs, self.stats_dim))?;
        let labels_t = Tensor::new(label_batch.as_slice(), device)?;

        let header_t = if self.header_dim > 0 {
            let mut header_t =
                Tensor::new(header_batch.as_slice(), device)?.reshape((bs, self.header_dim))?;

            // Enrich headers per-group via frozen sibling-context attention
            if let Some(ctx) = frozen_ctx {
                for &(start, end, gi) in &group_boundaries {
                    let group = &self.table_groups[gi];
                    let n_cols = end - start;
                    // Only enrich groups with sibling headers (v3 data)
                    if n_cols > 1 && !group.sibling_headers.is_empty() {
                        // Extract this group's header features: [n_cols, header_dim]
                        let group_headers = header_t.narrow(0, start, n_cols)?;
                        // Run frozen attention
                        let enriched = ctx.forward(&group_headers)?;
                        // Splice enriched headers back into the batch tensor
                        // We rebuild by concatenating: [0..start] + enriched + [end..bs]
                        let mut parts: Vec<Tensor> = Vec::new();
                        if start > 0 {
                            parts.push(header_t.narrow(0, 0, start)?);
                        }
                        parts.push(enriched);
                        if end < bs {
                            parts.push(header_t.narrow(0, end, bs - end)?);
                        }
                        header_t = Tensor::cat(&parts, 0)?;
                    }
                }
            }

            Some(header_t)
        } else {
            None
        };

        Ok((char_t, embed_t, stats_t, header_t, labels_t))
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

/// Compute hierarchical multi-level cross-entropy loss.
///
/// Decomposes flat label indices into domain/category/type targets using the
/// hierarchy map, computes per-level cross-entropy, and returns a weighted sum:
/// `0.2 * domain_loss + 0.3 * category_loss + 0.5 * leaf_loss`.
///
/// Also returns the flat-space accuracy for metric tracking.
fn compute_hierarchical_loss(
    domain_logits: &Tensor,
    cat_logits_all: &[Tensor],
    leaf_logits_all: &[Vec<Option<Tensor>>],
    flat_labels: &Tensor,
    hierarchy: &HierarchyMap,
    device: &Device,
) -> candle_core::Result<Tensor> {
    let flat_labels_vec: Vec<u32> = flat_labels.to_vec1()?;
    let batch_len = flat_labels_vec.len();

    // Decompose flat labels into per-level targets
    let mut domain_targets = Vec::with_capacity(batch_len);
    let mut cat_targets_by_domain: Vec<Vec<u32>> = vec![Vec::new(); hierarchy.num_domains()];
    let mut cat_sample_indices_by_domain: Vec<Vec<usize>> =
        vec![Vec::new(); hierarchy.num_domains()];
    let mut leaf_targets_by_cat: Vec<Vec<Vec<u32>>> = Vec::new();
    let mut leaf_sample_indices_by_cat: Vec<Vec<Vec<usize>>> = Vec::new();

    for d in 0..hierarchy.num_domains() {
        leaf_targets_by_cat.push(vec![Vec::new(); hierarchy.num_categories(d)]);
        leaf_sample_indices_by_cat.push(vec![Vec::new(); hierarchy.num_categories(d)]);
    }

    for (i, &flat_idx) in flat_labels_vec.iter().enumerate() {
        let (d, c, t) = hierarchy.flat_to_hier(flat_idx as usize);
        domain_targets.push(d as u32);
        cat_targets_by_domain[d].push(c as u32);
        cat_sample_indices_by_domain[d].push(i);
        leaf_targets_by_cat[d][c].push(t as u32);
        leaf_sample_indices_by_cat[d][c].push(i);
    }

    // Domain loss (all samples)
    let domain_target_tensor = Tensor::new(domain_targets, device)?;
    let domain_loss = candle_nn::loss::cross_entropy(domain_logits, &domain_target_tensor)?;

    // Category loss (grouped by ground-truth domain)
    let mut cat_loss_sum = Tensor::new(0.0f32, device)?;
    let mut cat_count = 0usize;

    for d in 0..hierarchy.num_domains() {
        if cat_targets_by_domain[d].is_empty() {
            continue;
        }
        let indices: Vec<u32> = cat_sample_indices_by_domain[d]
            .iter()
            .map(|&i| i as u32)
            .collect();
        let idx_tensor = Tensor::new(indices, device)?;
        let cat_logits_subset = cat_logits_all[d].index_select(&idx_tensor, 0)?;
        let cat_target_tensor = Tensor::new(cat_targets_by_domain[d].clone(), device)?;
        let cl = candle_nn::loss::cross_entropy(&cat_logits_subset, &cat_target_tensor)?;
        let n = cat_targets_by_domain[d].len() as f32;
        cat_loss_sum = (cat_loss_sum + cl.broadcast_mul(&Tensor::new(n, device)?))?;
        cat_count += cat_targets_by_domain[d].len();
    }

    let cat_loss = if cat_count > 0 {
        cat_loss_sum.broadcast_div(&Tensor::new(cat_count as f32, device)?)?
    } else {
        Tensor::new(0.0f32, device)?
    };

    // Leaf loss (grouped by ground-truth domain + category, skip degenerate)
    let mut leaf_loss_sum = Tensor::new(0.0f32, device)?;
    let mut leaf_count = 0usize;

    for d in 0..hierarchy.num_domains() {
        for c in 0..hierarchy.num_categories(d) {
            if hierarchy.is_degenerate(d, c) || leaf_targets_by_cat[d][c].is_empty() {
                continue;
            }
            if let Some(ref leaf_logits) = leaf_logits_all[d][c] {
                let indices: Vec<u32> = leaf_sample_indices_by_cat[d][c]
                    .iter()
                    .map(|&i| i as u32)
                    .collect();
                let idx_tensor = Tensor::new(indices, device)?;
                let leaf_logits_subset = leaf_logits.index_select(&idx_tensor, 0)?;
                let leaf_target_tensor = Tensor::new(leaf_targets_by_cat[d][c].clone(), device)?;
                let ll = candle_nn::loss::cross_entropy(&leaf_logits_subset, &leaf_target_tensor)?;
                let n = leaf_targets_by_cat[d][c].len() as f32;
                leaf_loss_sum = (leaf_loss_sum + ll.broadcast_mul(&Tensor::new(n, device)?))?;
                leaf_count += leaf_targets_by_cat[d][c].len();
            }
        }
    }

    let leaf_loss = if leaf_count > 0 {
        leaf_loss_sum.broadcast_div(&Tensor::new(leaf_count as f32, device)?)?
    } else {
        Tensor::new(0.0f32, device)?
    };

    // Weighted combination: 0.2 * domain + 0.3 * category + 0.5 * leaf
    let total = (domain_loss.broadcast_mul(&Tensor::new(0.2f32, device)?)?
        + cat_loss.broadcast_mul(&Tensor::new(0.3f32, device)?)?
        + leaf_loss.broadcast_mul(&Tensor::new(0.5f32, device)?)?)?;

    Ok(total)
}

/// Train the multi-branch model.
///
/// Loads feature-vector training data, runs forward/backward passes with Adam,
/// logs loss per epoch, and saves the best model weights in safetensors format.
///
/// When `model_config.head_type == HeadType::Hierarchical`, `labels` must be
/// provided (sorted list of all type labels). For flat head, `labels` is ignored.
pub fn train_multi_branch(
    config: &MultiBranchTrainConfig,
    model_config: &MultiBranchConfig,
    train_data: &MultiBranchDataset,
    val_data: &MultiBranchDataset,
    labels: Option<&[String]>,
    sibling_ctx_dir: Option<&Path>,
    renderer: Option<Box<dyn crate::tui::TrainingRenderer>>,
) -> Result<crate::training::TrainingSummary> {
    use crate::training::{
        compute_accuracy, shuffled_batches, CosineScheduler, EarlyStopping, EpochMetrics,
    };
    use candle_nn::{AdamW, Optimizer, ParamsAdamW};
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    let (device, device_name) = crate::get_device();
    eprintln!("Using {device_name} device");
    let mut rng = StdRng::seed_from_u64(config.seed);

    // Load frozen sibling-context on the SAME device as the training model.
    // Loading externally creates a separate Metal device handle, causing
    // "device mismatch" errors during matmul.
    let frozen_ctx = match sibling_ctx_dir {
        Some(dir) => match FrozenSiblingContext::load(dir, &device) {
            Ok(ctx) => {
                tracing::info!(
                    "Loaded frozen sibling-context attention ({}d) on training device",
                    ctx.embed_dim()
                );
                Some(ctx)
            }
            Err(e) => {
                tracing::warn!("Failed to load sibling-context model: {e}");
                None
            }
        },
        None => None,
    };
    let frozen_ref = frozen_ctx.as_ref();

    let is_hierarchical = model_config.head_type == HeadType::Hierarchical;
    let use_group_batching = frozen_ref.is_some() && !train_data.table_groups.is_empty();

    if frozen_ref.is_some() {
        tracing::info!("Sibling-context enrichment enabled (frozen attention)");
    }

    tracing::info!(
        "Starting multi-branch training: {} train, {} val, {} epochs, batch_size={}, lr={}, head={}",
        train_data.len(),
        val_data.len(),
        config.epochs,
        config.batch_size,
        config.lr,
        if is_hierarchical { "hierarchical" } else { "flat" },
    );

    // Create model
    let varmap = VarMap::new();
    let vb = VarBuilder::from_varmap(&varmap, DType::F32, &device);
    let model = if is_hierarchical {
        let labels = labels
            .ok_or_else(|| anyhow::anyhow!("Hierarchical head requires sorted labels list"))?;
        MultiBranchModel::new_hierarchical(model_config, labels, vb)?
    } else {
        MultiBranchModel::new(model_config, vb)?
    };

    let n_params = count_parameters(&varmap);
    tracing::info!("Model parameters: {}", n_params);
    if model_config.has_header_branch() {
        tracing::info!(
            "Architecture: char [{} → {} → {}] | embed [{} → {} → {}] | stats [{} → {} → {}] | header [{} → {} → {}] | merge [{} → {} → {}] | head → {}",
            model_config.char_dim, model_config.char_hidden[0], model_config.char_hidden[1],
            model_config.embed_dim, model_config.embed_hidden[0], model_config.embed_hidden[1],
            model_config.stats_dim, model_config.stats_hidden[0], model_config.stats_hidden[1],
            model_config.header_dim, model_config.header_hidden[0], model_config.header_hidden[1],
            model_config.merged_dim(),
            model_config.merge_hidden[0], model_config.merge_hidden[1],
            model_config.n_classes,
        );
    } else {
        tracing::info!(
            "Architecture: char [{} → {} → {}] | embed [{} → {} → {}] | stats [{} → {} → {}] | merge [{} → {} → {}] | head → {}",
            model_config.char_dim, model_config.char_hidden[0], model_config.char_hidden[1],
            model_config.embed_dim, model_config.embed_hidden[0], model_config.embed_hidden[1],
            model_config.stats_dim, model_config.stats_hidden[0], model_config.stats_hidden[1],
            model_config.merged_dim(),
            model_config.merge_hidden[0], model_config.merge_hidden[1],
            model_config.n_classes,
        );
    }

    if is_hierarchical {
        let hier = model.hierarchical_head().unwrap();
        let h = hier.hierarchy();
        tracing::info!(
            "Hierarchical: {} domains, {} categories, {} degenerate",
            h.num_domains(),
            h.total_categories(),
            (0..h.num_domains())
                .flat_map(|d| (0..h.num_categories(d)).map(move |c| (d, c)))
                .filter(|&(d, c)| h.is_degenerate(d, c))
                .count()
        );
    }

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

    // Initialize renderer (fall back to LogRenderer if None)
    let mut renderer: Box<dyn crate::tui::TrainingRenderer> =
        renderer.unwrap_or_else(|| Box::new(crate::tui::LogRenderer::new()));

    // Compute initial batches count for renderer (will be recomputed each epoch due to shuffling)
    let initial_batch_count = train_data.len().div_ceil(config.batch_size);
    renderer.on_train_start(config.epochs, initial_batch_count);

    for epoch in 0..config.epochs {
        let epoch_start = std::time::Instant::now();

        // Update learning rate
        let lr = scheduler.lr(epoch);
        optimizer.set_learning_rate(lr);

        let mut train_loss_sum = 0.0f64;
        let mut train_correct_sum = 0.0f64;
        let mut train_samples = 0usize;

        // Build batches: group-based (with frozen enrichment) or flat (traditional)
        let group_batches: Vec<Vec<usize>>;
        let flat_batches: Vec<Vec<usize>>;
        let total_batches;

        if use_group_batching {
            // Shuffle at table-group level, sample whole groups until batch is full
            use rand::seq::SliceRandom;
            let n_groups = train_data.table_groups.len();
            let mut group_order: Vec<usize> = (0..n_groups).collect();
            group_order.shuffle(&mut rng);

            // Pack groups into batches (may overshoot batch_size per group)
            let mut batches = Vec::new();
            let mut current_batch = Vec::new();
            let mut current_size = 0usize;
            for &gi in &group_order {
                let group_size = train_data.table_groups[gi].record_indices.len();
                current_batch.push(gi);
                current_size += group_size;
                if current_size >= config.batch_size {
                    batches.push(current_batch);
                    current_batch = Vec::new();
                    current_size = 0;
                }
            }
            if !current_batch.is_empty() {
                batches.push(current_batch);
            }
            group_batches = batches;
            flat_batches = Vec::new();
            total_batches = group_batches.len();
        } else {
            group_batches = Vec::new();
            flat_batches = shuffled_batches(train_data.len(), config.batch_size, &mut rng);
            total_batches = flat_batches.len();
        }

        // Training loop
        for batch_num in 0..total_batches {
            let (char_t, embed_t, stats_t, header_t, labels_t) = if use_group_batching {
                train_data.batch_groups(&group_batches[batch_num], frozen_ref, &device)?
            } else {
                train_data.batch(&flat_batches[batch_num], &device)?
            };
            let bs = labels_t.dim(0)?;

            let loss = if is_hierarchical {
                let hier = model.hierarchical_head().unwrap();
                let hierarchy = hier.hierarchy();
                let (domain_logits, cat_logits, leaf_logits) =
                    model.forward_levels(&char_t, &embed_t, &stats_t, header_t.as_ref(), true)?;
                compute_hierarchical_loss(
                    &domain_logits,
                    &cat_logits,
                    &leaf_logits,
                    &labels_t,
                    hierarchy,
                    &device,
                )?
            } else {
                let logits = model.forward(&char_t, &embed_t, &stats_t, header_t.as_ref(), true)?;
                candle_nn::loss::cross_entropy(&logits, &labels_t)?
            };

            optimizer.backward_step(&loss)?;

            let loss_val: f32 = loss.to_scalar()?;
            train_loss_sum += loss_val as f64 * bs as f64;

            // Accuracy via forward (product probabilities for hier, logits for flat)
            let output = model.forward(&char_t, &embed_t, &stats_t, header_t.as_ref(), false)?;
            let acc = compute_accuracy(&output, &labels_t)?;
            train_correct_sum += acc as f64 * bs as f64;
            train_samples += bs;

            renderer.on_batch_end(epoch, batch_num + 1, total_batches, loss_val);
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
                let (char_t, embed_t, stats_t, header_t, labels_t) =
                    val_data.batch(batch_idx, &device)?;
                let bs = batch_idx.len();

                let loss_val = if is_hierarchical {
                    let hier = model.hierarchical_head().unwrap();
                    let hierarchy = hier.hierarchy();
                    let (domain_logits, cat_logits, leaf_logits) = model.forward_levels(
                        &char_t,
                        &embed_t,
                        &stats_t,
                        header_t.as_ref(),
                        false,
                    )?;
                    let loss = compute_hierarchical_loss(
                        &domain_logits,
                        &cat_logits,
                        &leaf_logits,
                        &labels_t,
                        hierarchy,
                        &device,
                    )?;
                    loss.to_scalar::<f32>()?
                } else {
                    let logits =
                        model.forward(&char_t, &embed_t, &stats_t, header_t.as_ref(), false)?;
                    let loss = candle_nn::loss::cross_entropy(&logits, &labels_t)?;
                    loss.to_scalar::<f32>()?
                };

                val_loss_sum += loss_val as f64 * bs as f64;

                let output =
                    model.forward(&char_t, &embed_t, &stats_t, header_t.as_ref(), false)?;
                let acc = compute_accuracy(&output, &labels_t)?;
                val_correct_sum += acc as f64 * bs as f64;
                val_samples += bs;
            }

            let val_acc = (val_correct_sum / val_samples as f64) as f32;
            let val_loss = (val_loss_sum / val_samples as f64) as f32;
            (val_acc, val_loss)
        };

        let epoch_time = epoch_start.elapsed().as_secs_f32();

        let metrics = EpochMetrics {
            epoch,
            train_loss,
            val_loss,
            train_accuracy,
            val_accuracy,
            learning_rate: lr,
            epoch_time_secs: epoch_time,
        };
        renderer.on_epoch_end(&metrics);
        epoch_metrics.push(metrics);

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

    renderer.on_train_end();

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
            .forward(&char_feats, &embed_feats, &stats_feats, None, true)
            .unwrap();
        assert_eq!(logits.dims(), &[batch_size, config.n_classes]);

        // Eval mode
        let logits = model
            .forward(&char_feats, &embed_feats, &stats_feats, None, false)
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
            .forward(&char_feats, &embed_feats, &stats_feats, None, true)
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
            header_dim: 128,
            char_hidden: [300, 300],
            embed_hidden: [200, 200],
            stats_hidden: [128, 64],
            header_hidden: [128, 64],
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
        assert_eq!(config.header_dim, loaded.header_dim);
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
                header_features: (0..128).map(|j| (i * 128 + j) as f32 * 0.003).collect(),
            })
            .collect();

        let tmp = tempfile::NamedTempFile::new().unwrap();
        write_training_data(tmp.path(), &records, 960, 512, 27, 128).unwrap();

        let (header, loaded, _groups) = read_training_data(tmp.path()).unwrap();
        assert_eq!(header.n_records, 10);
        assert_eq!(header.version, 2);
        assert_eq!(header.char_dim, 960);
        assert_eq!(header.embed_dim, 512);
        assert_eq!(header.stats_dim, 27);
        assert_eq!(header.header_dim, 128);
        assert_eq!(loaded.len(), 10);

        for (orig, read) in records.iter().zip(loaded.iter()) {
            assert_eq!(orig.label, read.label);
            assert_eq!(orig.char_features.len(), read.char_features.len());
            assert_eq!(orig.embed_features.len(), read.embed_features.len());
            assert_eq!(orig.stats_features.len(), read.stats_features.len());
            assert_eq!(orig.header_features.len(), read.header_features.len());

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
            for (a, b) in orig.header_features.iter().zip(read.header_features.iter()) {
                assert_eq!(a.to_bits(), b.to_bits(), "header feature mismatch");
            }
        }
    }

    #[test]
    fn test_ftmb_v1_read_compat() {
        // Write a v1-format file manually and verify read_training_data loads it
        // with zero header_features (graceful degradation).
        use std::io::Write;

        let tmp = tempfile::NamedTempFile::new().unwrap();
        let mut file = std::fs::File::create(tmp.path()).unwrap();

        let char_dim: u16 = 4;
        let embed_dim: u16 = 3;
        let stats_dim: u16 = 2;
        let n_records: u64 = 2;

        // Write v1 header (version=1, padding=0)
        file.write_all(b"FTMB").unwrap();
        file.write_all(&1u32.to_le_bytes()).unwrap(); // version 1
        file.write_all(&n_records.to_le_bytes()).unwrap();
        file.write_all(&char_dim.to_le_bytes()).unwrap();
        file.write_all(&embed_dim.to_le_bytes()).unwrap();
        file.write_all(&stats_dim.to_le_bytes()).unwrap();
        file.write_all(&[0u8; 2]).unwrap(); // padding (v1)

        // Write 2 records
        for i in 0..2 {
            let label = format!("type_{i}");
            let label_bytes = label.as_bytes();
            file.write_all(&(label_bytes.len() as u16).to_le_bytes())
                .unwrap();
            file.write_all(label_bytes).unwrap();

            for j in 0..char_dim {
                file.write_all(&((i * 10 + j) as f32).to_le_bytes())
                    .unwrap();
            }
            for j in 0..embed_dim {
                file.write_all(&((i * 10 + j) as f32 * 0.5).to_le_bytes())
                    .unwrap();
            }
            for j in 0..stats_dim {
                file.write_all(&((i * 10 + j) as f32 * 0.1).to_le_bytes())
                    .unwrap();
            }
        }
        drop(file);

        let (header, records, _groups) = read_training_data(tmp.path()).unwrap();
        assert_eq!(header.version, 1);
        assert_eq!(header.n_records, 2);
        assert_eq!(header.char_dim, 4);
        assert_eq!(header.embed_dim, 3);
        assert_eq!(header.stats_dim, 2);
        assert_eq!(header.header_dim, 0);
        assert_eq!(records.len(), 2);

        // v1 records should have empty header_features (header_dim=0 -> zero-length vec)
        for record in &records {
            assert!(
                record.header_features.is_empty(),
                "v1 should have empty header_features"
            );
            assert_eq!(record.char_features.len(), 4);
            assert_eq!(record.embed_features.len(), 3);
            assert_eq!(record.stats_features.len(), 2);
        }
        assert_eq!(records[0].label, "type_0");
        assert_eq!(records[1].label, "type_1");
    }

    #[test]
    fn test_ftmb_v2_roundtrip_4_vectors() {
        // Write v2 with 4 feature vectors, read back, assert all 4 match.
        let records: Vec<TrainingRecord> = (0..3)
            .map(|i| TrainingRecord {
                label: format!("test.type.t_{i}"),
                char_features: vec![i as f32 * 1.0; 8],
                embed_features: vec![i as f32 * 2.0; 6],
                stats_features: vec![i as f32 * 3.0; 4],
                header_features: vec![i as f32 * 4.0; 5],
            })
            .collect();

        let tmp = tempfile::NamedTempFile::new().unwrap();
        write_training_data(tmp.path(), &records, 8, 6, 4, 5).unwrap();

        let (header, loaded, _groups) = read_training_data(tmp.path()).unwrap();
        assert_eq!(header.version, 2);
        assert_eq!(header.n_records, 3);
        assert_eq!(header.char_dim, 8);
        assert_eq!(header.embed_dim, 6);
        assert_eq!(header.stats_dim, 4);
        assert_eq!(header.header_dim, 5);
        assert_eq!(loaded.len(), 3);

        for (orig, read) in records.iter().zip(loaded.iter()) {
            assert_eq!(orig.label, read.label);
            assert_eq!(orig.char_features, read.char_features);
            assert_eq!(orig.embed_features, read.embed_features);
            assert_eq!(orig.stats_features, read.stats_features);
            assert_eq!(orig.header_features, read.header_features);
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
                header_features: vec![i as f32 * 4.0; 128],
            })
            .collect();

        let mut label_to_idx = std::collections::HashMap::new();
        label_to_idx.insert("identity.person.type_0".to_string(), 0u32);
        label_to_idx.insert("identity.person.type_1".to_string(), 1u32);
        label_to_idx.insert("identity.person.type_2".to_string(), 2u32);

        let dataset =
            MultiBranchDataset::from_records(&records, &label_to_idx, 960, 512, 27, 128).unwrap();
        assert_eq!(dataset.len(), 5);

        let device = Device::Cpu;
        let (char_t, embed_t, stats_t, header_t, labels_t) =
            dataset.batch(&[0, 2, 4], &device).unwrap();
        assert_eq!(char_t.dims(), &[3, 960]);
        assert_eq!(embed_t.dims(), &[3, 512]);
        assert_eq!(stats_t.dims(), &[3, 27]);
        let header_t = header_t.expect("header tensor should be Some for header_dim=128");
        assert_eq!(header_t.dims(), &[3, 128]);
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
                header_features: (0..128)
                    .map(|j| if j % 3 == class_idx { 0.5 + bias } else { 0.05 })
                    .collect(),
            });
        }

        let mut label_to_idx = std::collections::HashMap::new();
        label_to_idx.insert("type_a".to_string(), 0u32);
        label_to_idx.insert("type_b".to_string(), 1u32);
        label_to_idx.insert("type_c".to_string(), 2u32);

        let train_data =
            MultiBranchDataset::from_records(&records[..20], &label_to_idx, 960, 512, 27, 128)
                .unwrap();
        let val_data =
            MultiBranchDataset::from_records(&records[20..], &label_to_idx, 960, 512, 27, 128)
                .unwrap();

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

        let summary = train_multi_branch(
            &train_config,
            &config,
            &train_data,
            &val_data,
            None,
            None,
            None,
        )
        .unwrap();

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
        assert_eq!(config.merged_dim(), 300 + 200 + 64 + 64); // 628 (with header branch)

        // Old-style config without header branch
        let old_config = MultiBranchConfig {
            header_dim: 0,
            header_hidden: [0, 0],
            ..Default::default()
        };
        assert_eq!(old_config.merged_dim(), 300 + 200 + 64); // 564
    }

    // ── Hierarchical head tests ──────────────────────────────────────

    /// Generate a minimal set of sorted labels spanning multiple domains/categories.
    fn make_hier_labels() -> Vec<String> {
        vec![
            "container.array.comma_separated".to_string(),
            "container.array.pipe_separated".to_string(),
            "container.object.json".to_string(),
            "datetime.date.iso".to_string(),
            "datetime.date.ymd_slash".to_string(),
            "datetime.time.hms_24h".to_string(),
            "geography.location.city".to_string(),
            "geography.location.country".to_string(),
            "identity.person.email".to_string(),
            "identity.person.full_name".to_string(),
        ]
    }

    #[test]
    fn test_hierarchical_forward_pass_shape() {
        let labels = make_hier_labels();
        let config = MultiBranchConfig {
            n_classes: labels.len(),
            ..Default::default()
        };
        let device = Device::Cpu;
        let varmap = VarMap::new();
        let vb = VarBuilder::from_varmap(&varmap, DType::F32, &device);
        let model = MultiBranchModel::new_hierarchical(&config, &labels, vb).unwrap();

        assert!(model.is_hierarchical());

        let batch_size = 5;
        let char_feats =
            Tensor::randn(0.0f32, 1.0, (batch_size, config.char_dim), &device).unwrap();
        let embed_feats =
            Tensor::randn(0.0f32, 1.0, (batch_size, config.embed_dim), &device).unwrap();
        let stats_feats =
            Tensor::randn(0.0f32, 1.0, (batch_size, config.stats_dim), &device).unwrap();

        // forward() returns product probabilities [B, n_classes]
        let output = model
            .forward(&char_feats, &embed_feats, &stats_feats, None, false)
            .unwrap();
        assert_eq!(output.dims(), &[batch_size, labels.len()]);

        // forward_levels() returns per-level logits
        let (domain_logits, cat_logits, leaf_logits) = model
            .forward_levels(&char_feats, &embed_feats, &stats_feats, None, false)
            .unwrap();

        // 4 domains: container, datetime, geography, identity
        assert_eq!(domain_logits.dims()[0], batch_size);
        assert_eq!(domain_logits.dims()[1], 4);
        assert_eq!(cat_logits.len(), 4);
        assert_eq!(leaf_logits.len(), 4);
    }

    #[test]
    fn test_hierarchical_gradient_flow() {
        let labels = make_hier_labels();
        let config = MultiBranchConfig {
            n_classes: labels.len(),
            ..Default::default()
        };
        let device = Device::Cpu;
        let varmap = VarMap::new();
        let vb = VarBuilder::from_varmap(&varmap, DType::F32, &device);
        let model = MultiBranchModel::new_hierarchical(&config, &labels, vb).unwrap();

        let batch_size = 4;
        let char_feats =
            Tensor::randn(0.0f32, 1.0, (batch_size, config.char_dim), &device).unwrap();
        let embed_feats =
            Tensor::randn(0.0f32, 1.0, (batch_size, config.embed_dim), &device).unwrap();
        let stats_feats =
            Tensor::randn(0.0f32, 1.0, (batch_size, config.stats_dim), &device).unwrap();
        // Target labels: one per sample, spread across domains
        let targets = Tensor::new(&[0u32, 3, 6, 8], &device).unwrap();

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

        // Compute hierarchical loss
        let hier = model.hierarchical_head().unwrap();
        let hierarchy = hier.hierarchy();
        let (domain_logits, cat_logits, leaf_logits) = model
            .forward_levels(&char_feats, &embed_feats, &stats_feats, None, true)
            .unwrap();
        let loss = compute_hierarchical_loss(
            &domain_logits,
            &cat_logits,
            &leaf_logits,
            &targets,
            hierarchy,
            &device,
        )
        .unwrap();

        let adamw_params = candle_nn::ParamsAdamW {
            lr: 0.01,
            ..Default::default()
        };
        let mut optimizer = candle_nn::AdamW::new(varmap.all_vars(), adamw_params).unwrap();
        optimizer.backward_step(&loss).unwrap();

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

        let n_changed: usize = initial_values
            .iter()
            .zip(updated_values.iter())
            .filter(|(a, b)| a != b)
            .count();
        assert!(
            n_changed > 5,
            "Expected many parameters to change after hierarchical backward, only {} changed out of {}",
            n_changed,
            vars.len()
        );
    }

    #[test]
    fn test_hierarchical_training_loop_small() {
        let labels = make_hier_labels();
        let n_classes = labels.len();
        let config = MultiBranchConfig {
            n_classes,
            head_type: HeadType::Hierarchical,
            ..Default::default()
        };

        // Create synthetic data with class-dependent features
        let mut records = Vec::new();
        for i in 0..30 {
            let class_idx = i % n_classes;
            let bias = class_idx as f32;
            records.push(TrainingRecord {
                label: labels[class_idx].clone(),
                char_features: (0..960)
                    .map(|j| {
                        if j % n_classes == class_idx {
                            1.0 + bias
                        } else {
                            0.1
                        }
                    })
                    .collect(),
                embed_features: (0..512)
                    .map(|j| {
                        if j % n_classes == class_idx {
                            1.0 + bias
                        } else {
                            0.1
                        }
                    })
                    .collect(),
                stats_features: (0..27)
                    .map(|j| {
                        if j % n_classes == class_idx {
                            1.0 + bias
                        } else {
                            0.1
                        }
                    })
                    .collect(),
                header_features: (0..128)
                    .map(|j| {
                        if j % n_classes == class_idx {
                            0.5 + bias
                        } else {
                            0.05
                        }
                    })
                    .collect(),
            });
        }

        let mut label_to_idx = std::collections::HashMap::new();
        for (i, label) in labels.iter().enumerate() {
            label_to_idx.insert(label.clone(), i as u32);
        }

        let train_data =
            MultiBranchDataset::from_records(&records[..20], &label_to_idx, 960, 512, 27, 128)
                .unwrap();
        let val_data =
            MultiBranchDataset::from_records(&records[20..], &label_to_idx, 960, 512, 27, 128)
                .unwrap();

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

        let summary = train_multi_branch(
            &train_config,
            &config,
            &train_data,
            &val_data,
            Some(&labels),
            None,
            None,
        )
        .unwrap();

        assert_eq!(summary.total_epochs, 5);

        // Loss should decrease
        let loss_0 = summary.epoch_metrics[0].train_loss;
        let loss_4 = summary.epoch_metrics[4].train_loss;
        assert!(
            loss_4 < loss_0,
            "Hierarchical training loss should decrease: epoch 0 = {}, epoch 4 = {}",
            loss_0,
            loss_4,
        );

        // Model artifacts should exist
        assert!(tmp_dir.path().join("model.safetensors").exists());
        assert!(tmp_dir.path().join("config.json").exists());
    }

    // ── FTMB v3 tests ──────────────────────────────────────────────

    #[test]
    fn test_ftmb_v3_roundtrip() {
        // Create records belonging to 2 table groups
        let records: Vec<TrainingRecord> = (0..5)
            .map(|i| TrainingRecord {
                label: format!("test.type.t_{i}"),
                char_features: vec![i as f32 * 1.0; 8],
                embed_features: vec![i as f32 * 2.0; 6],
                stats_features: vec![i as f32 * 3.0; 4],
                header_features: vec![i as f32 * 4.0; 5],
            })
            .collect();

        let groups = vec![
            TableGroup {
                record_indices: vec![0, 1, 2],
                sibling_headers: vec!["city".to_string(), "name".to_string(), "email".to_string()],
            },
            TableGroup {
                record_indices: vec![3, 4],
                sibling_headers: vec!["amount".to_string(), "currency".to_string()],
            },
        ];

        let tmp = tempfile::NamedTempFile::new().unwrap();
        write_training_data_v3(tmp.path(), &records, &groups, 8, 6, 4, 5).unwrap();

        let (header, loaded, loaded_groups) = read_training_data(tmp.path()).unwrap();
        assert_eq!(header.version, 3);
        assert_eq!(header.n_records, 5);
        assert_eq!(header.n_groups, 2);
        assert_eq!(header.char_dim, 8);
        assert_eq!(header.embed_dim, 6);
        assert_eq!(header.stats_dim, 4);
        assert_eq!(header.header_dim, 5);
        assert_eq!(loaded.len(), 5);
        assert_eq!(loaded_groups.len(), 2);

        // Verify group metadata
        assert_eq!(loaded_groups[0].record_indices, vec![0, 1, 2]);
        assert_eq!(loaded_groups[0].sibling_headers.len(), 3);
        assert_eq!(loaded_groups[0].sibling_headers[0], "city");
        assert_eq!(loaded_groups[0].sibling_headers[1], "name");
        assert_eq!(loaded_groups[0].sibling_headers[2], "email");

        assert_eq!(loaded_groups[1].record_indices, vec![3, 4]);
        assert_eq!(loaded_groups[1].sibling_headers.len(), 2);
        assert_eq!(loaded_groups[1].sibling_headers[0], "amount");
        assert_eq!(loaded_groups[1].sibling_headers[1], "currency");

        // Verify record data roundtrip
        for (orig, read) in records.iter().zip(loaded.iter()) {
            assert_eq!(orig.label, read.label);
            assert_eq!(orig.char_features, read.char_features);
            assert_eq!(orig.embed_features, read.embed_features);
            assert_eq!(orig.stats_features, read.stats_features);
            assert_eq!(orig.header_features, read.header_features);
        }
    }

    #[test]
    fn test_ftmb_v1_read_as_single_group() {
        // Write a v1 file and verify it loads as single group with empty siblings
        use std::io::Write;

        let tmp = tempfile::NamedTempFile::new().unwrap();
        let mut file = std::fs::File::create(tmp.path()).unwrap();

        let char_dim: u16 = 4;
        let embed_dim: u16 = 3;
        let stats_dim: u16 = 2;
        let n_records: u64 = 2;

        file.write_all(b"FTMB").unwrap();
        file.write_all(&1u32.to_le_bytes()).unwrap();
        file.write_all(&n_records.to_le_bytes()).unwrap();
        file.write_all(&char_dim.to_le_bytes()).unwrap();
        file.write_all(&embed_dim.to_le_bytes()).unwrap();
        file.write_all(&stats_dim.to_le_bytes()).unwrap();
        file.write_all(&[0u8; 2]).unwrap(); // padding

        for i in 0..2 {
            let label = format!("type_{i}");
            let label_bytes = label.as_bytes();
            file.write_all(&(label_bytes.len() as u16).to_le_bytes())
                .unwrap();
            file.write_all(label_bytes).unwrap();
            for j in 0..char_dim {
                file.write_all(&((i * 10 + j) as f32).to_le_bytes())
                    .unwrap();
            }
            for j in 0..embed_dim {
                file.write_all(&((i * 10 + j) as f32 * 0.5).to_le_bytes())
                    .unwrap();
            }
            for j in 0..stats_dim {
                file.write_all(&((i * 10 + j) as f32 * 0.1).to_le_bytes())
                    .unwrap();
            }
        }
        drop(file);

        let (_header, records, groups) = read_training_data(tmp.path()).unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(groups.len(), 1, "v1 should have single group");
        assert_eq!(groups[0].record_indices, vec![0, 1]);
        assert!(
            groups[0].sibling_headers.is_empty(),
            "v1 group should have empty sibling headers"
        );
    }

    #[test]
    fn test_ftmb_v2_read_as_single_group() {
        // Write v2, read back, verify single group wrapper
        let records: Vec<TrainingRecord> = (0..3)
            .map(|i| TrainingRecord {
                label: format!("test.type.t_{i}"),
                char_features: vec![i as f32; 8],
                embed_features: vec![i as f32 * 2.0; 6],
                stats_features: vec![i as f32 * 3.0; 4],
                header_features: vec![i as f32 * 4.0; 5],
            })
            .collect();

        let tmp = tempfile::NamedTempFile::new().unwrap();
        write_training_data(tmp.path(), &records, 8, 6, 4, 5).unwrap();

        let (_header, loaded, groups) = read_training_data(tmp.path()).unwrap();
        assert_eq!(loaded.len(), 3);
        assert_eq!(groups.len(), 1, "v2 should have single group");
        assert_eq!(groups[0].record_indices, vec![0, 1, 2]);
        assert!(
            groups[0].sibling_headers.is_empty(),
            "v2 group should have empty sibling headers"
        );
    }

    #[test]
    fn test_frozen_sibling_context_forward() {
        // Create a real sibling-context model, save it, then load as frozen
        use crate::sibling_context::SiblingContextTrainable;
        use finetype_model::sibling_context::SiblingContextConfig;

        let varmap = VarMap::new();
        let config = SiblingContextConfig::default();
        let device = Device::Cpu;
        let trainable = SiblingContextTrainable::new(&varmap, &config, &device).unwrap();

        // Save to temp dir
        let tmp_dir = std::env::temp_dir().join("finetype_frozen_sibling_test");
        let _ = std::fs::remove_dir_all(&tmp_dir);
        std::fs::create_dir_all(&tmp_dir).unwrap();
        varmap.save(tmp_dir.join("model.safetensors")).unwrap();
        let config_json = serde_json::to_string_pretty(&config).unwrap();
        std::fs::write(tmp_dir.join("config.json"), &config_json).unwrap();

        // Load as frozen
        let frozen = FrozenSiblingContext::load(&tmp_dir, &device).unwrap();
        assert_eq!(frozen.embed_dim(), 128);

        // Verify shape preservation for various column counts
        for n_cols in [1, 5, 10, 20] {
            let input = Tensor::randn(0.0f32, 1.0, (n_cols, 128), &device).unwrap();
            let output = frozen.forward(&input).unwrap();
            assert_eq!(
                output.dims(),
                &[n_cols, 128],
                "Shape mismatch for N={}",
                n_cols
            );
        }

        // Verify output matches trainable model
        let input = Tensor::randn(0.0f32, 1.0, (5, 128), &device).unwrap();
        let out_trainable: Vec<f32> = trainable
            .forward(&input)
            .unwrap()
            .flatten_all()
            .unwrap()
            .to_vec1()
            .unwrap();
        let out_frozen: Vec<f32> = frozen
            .forward(&input)
            .unwrap()
            .flatten_all()
            .unwrap()
            .to_vec1()
            .unwrap();

        for (a, b) in out_trainable.iter().zip(out_frozen.iter()) {
            assert!(
                (a - b).abs() < 1e-5,
                "Frozen output should match trainable: {} vs {}",
                a,
                b
            );
        }

        let _ = std::fs::remove_dir_all(&tmp_dir);
    }

    #[test]
    fn test_dataset_batch_groups_without_enrichment() {
        // Test batch_groups without frozen context (passthrough)
        let records: Vec<TrainingRecord> = (0..6)
            .map(|i| TrainingRecord {
                label: format!("identity.person.type_{}", i % 3),
                char_features: vec![i as f32; 8],
                embed_features: vec![i as f32 * 2.0; 6],
                stats_features: vec![i as f32 * 3.0; 4],
                header_features: vec![i as f32 * 4.0; 5],
            })
            .collect();

        let mut label_to_idx = std::collections::HashMap::new();
        label_to_idx.insert("identity.person.type_0".to_string(), 0u32);
        label_to_idx.insert("identity.person.type_1".to_string(), 1u32);
        label_to_idx.insert("identity.person.type_2".to_string(), 2u32);

        let groups = vec![
            TableGroup {
                record_indices: vec![0, 1, 2],
                sibling_headers: vec!["city".to_string(), "name".to_string(), "email".to_string()],
            },
            TableGroup {
                record_indices: vec![3, 4, 5],
                sibling_headers: vec![
                    "amount".to_string(),
                    "currency".to_string(),
                    "date".to_string(),
                ],
            },
        ];

        let dataset = MultiBranchDataset::from_records_with_groups(
            &records,
            &label_to_idx,
            8,
            6,
            4,
            5,
            Some(groups),
        )
        .unwrap();

        // Batch first group only
        let device = Device::Cpu;
        let (char_t, embed_t, stats_t, header_t, labels_t) =
            dataset.batch_groups(&[0], None, &device).unwrap();
        assert_eq!(char_t.dims(), &[3, 8]);
        assert_eq!(embed_t.dims(), &[3, 6]);
        assert_eq!(stats_t.dims(), &[3, 4]);
        let header_t = header_t.expect("header should be Some");
        assert_eq!(header_t.dims(), &[3, 5]);
        assert_eq!(labels_t.dims(), &[3]);

        // Batch both groups
        let (char_t, _, _, _, labels_t) = dataset.batch_groups(&[0, 1], None, &device).unwrap();
        assert_eq!(char_t.dims(), &[6, 8]);
        assert_eq!(labels_t.dims(), &[6]);
    }
}
