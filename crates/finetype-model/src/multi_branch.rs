//! Multi-branch model loader and inference for column-level classification.
//!
//! Loads multi-branch model artifacts (model.safetensors + config.json + label_map.json)
//! and provides column-level inference: Vec<String> -> features -> MLP forward -> label.
//!
//! Architecture (from finetype-train, configurable activation + normalization):
//! ```text
//! Branch 1 (char):   [960] -> [LN ->] Dense(300, Act) -> Dense(300, Act) -> [300]
//! Branch 2 (embed):  [512] -> [LN ->] Dense(200, Act) -> Dense(200, Act) -> [200]
//! Branch 3 (stats):  [27]  -> [LN ->] Dense(128, Act) -> Dense(64, Act)  -> [64]
//! Branch 4 (header): [128] -> LN -> Dense(128, Act) -> Dense(64, Act)    -> [64]  (optional)
//!                              |
//! Merge:              concat([300, 200, 64, 64]) = [628]  (or [564] without header)
//!                              |
//!                     Norm -> Dense(500, Act) -> Dense(500, Act)
//!                              |
//! Head (flat):        Dense(n_classes, softmax)
//! Head (hier):        TreeSoftmax(7 domains -> 43 categories -> 250 types)
//! ```
//!
//! Act = ReLU (default) or GELU (config.activation).
//! LN = LayerNorm on branch inputs (when config.use_layer_norm=true).
//! Norm = BatchNorm (default) or LayerNorm (when config.use_layer_norm=true).

use crate::char_cnn::HierarchicalHead;
use crate::char_distribution::{extract_char_distribution, CHAR_DIST_DIM};
use crate::column_stats::{extract_column_stats, COLUMN_STATS_DIM};
use crate::embedding_aggregation::{extract_embedding_aggregation, EMBED_AGG_DIM};
use crate::inference::InferenceError;
use crate::model2vec_shared::Model2VecResources;
use candle_core::{DType, Device, Module, Tensor};
use candle_nn::{batch_norm, linear, BatchNorm, BatchNormConfig, Linear, ModuleT, VarBuilder};
use serde::{Deserialize, Serialize};
use std::path::Path;

// ═══════════════════════════════════════════════════════════════════════════════
// Configuration (mirrors finetype-train MultiBranchConfig for deserialization)
// ═══════════════════════════════════════════════════════════════════════════════

/// Activation function for branch and merge layers.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub enum Activation {
    /// Rectified Linear Unit (default for backward compatibility).
    #[default]
    ReLU,
    /// Gaussian Error Linear Unit (autoresearch winner: +1.3pp).
    GELU,
}

/// Classification head type.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub enum HeadType {
    #[default]
    Flat,
    Hierarchical,
}

/// Configuration for the multi-branch model (deserialized from config.json).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiBranchConfig {
    pub char_dim: usize,
    pub embed_dim: usize,
    pub stats_dim: usize,
    /// Input dimension for header embedding features (Model2Vec, 128-dim).
    /// Defaults to 0 for backward compatibility with old configs.
    #[serde(default)]
    pub header_dim: usize,
    pub char_hidden: [usize; 2],
    pub embed_hidden: [usize; 2],
    pub stats_hidden: [usize; 2],
    /// Hidden layer sizes for the header branch (2 layers).
    /// Defaults to [0, 0] for old configs.
    #[serde(default)]
    pub header_hidden: [usize; 2],
    pub merge_hidden: [usize; 2],
    pub n_classes: usize,
    pub dropout: f32,
    pub head_type: HeadType,
    /// Activation function (default: ReLU for backward compat with old configs).
    #[serde(default)]
    pub activation: Activation,
    /// Whether to use LayerNorm instead of BatchNorm, and add input LayerNorm to all branches.
    #[serde(default)]
    pub use_layer_norm: bool,
}

impl MultiBranchConfig {
    /// Whether the header branch is enabled (header_dim > 0 with valid hidden dims).
    pub fn has_header_branch(&self) -> bool {
        self.header_dim > 0 && self.header_hidden[0] > 0 && self.header_hidden[1] > 0
    }

    fn merged_dim(&self) -> usize {
        let base = self.char_hidden[1] + self.embed_hidden[1] + self.stats_hidden[1];
        if self.has_header_branch() {
            base + self.header_hidden[1]
        } else {
            base
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Branch weights (2-layer MLP, inference only — no dropout)
// ═══════════════════════════════════════════════════════════════════════════════

struct BranchWeights {
    input_norm: Option<candle_nn::LayerNorm>,
    linear1: Linear,
    linear2: Linear,
    activation: Activation,
}

impl BranchWeights {
    fn new(
        input_dim: usize,
        hidden: [usize; 2],
        activation: &Activation,
        vb: VarBuilder,
    ) -> candle_core::Result<Self> {
        Self::new_inner(input_dim, hidden, false, activation, vb)
    }

    fn new_with_input_norm(
        input_dim: usize,
        hidden: [usize; 2],
        activation: &Activation,
        vb: VarBuilder,
    ) -> candle_core::Result<Self> {
        Self::new_inner(input_dim, hidden, true, activation, vb)
    }

    fn new_inner(
        input_dim: usize,
        hidden: [usize; 2],
        normalize_input: bool,
        activation: &Activation,
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
            activation: activation.clone(),
        })
    }

    fn activate(&self, x: &Tensor) -> candle_core::Result<Tensor> {
        match self.activation {
            Activation::ReLU => x.relu(),
            Activation::GELU => x.gelu_erf(),
        }
    }

    fn forward(&self, x: &Tensor) -> candle_core::Result<Tensor> {
        let x = match &self.input_norm {
            Some(ln) => ln.forward(x)?,
            None => x.clone(),
        };
        let h = self.linear1.forward_t(&x, false)?;
        let h = self.activate(&h)?;
        let h = self.linear2.forward_t(&h, false)?;
        self.activate(&h)
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Classification head (flat or hierarchical)
// ═══════════════════════════════════════════════════════════════════════════════

/// Classification head: either a flat linear layer or a hierarchical tree softmax.
enum ClassificationHead {
    /// Flat: Dense(hidden_dim → n_classes) producing logits.
    Flat(Linear),
    /// Hierarchical: 3-level tree softmax (domain → category → type)
    /// producing product probabilities.
    Hierarchical(Box<HierarchicalHead>),
}

// ═══════════════════════════════════════════════════════════════════════════════
// Model (inference only)
// ═══════════════════════════════════════════════════════════════════════════════

/// Multi-branch model for column-level type classification.
///
/// Loads from safetensors + config.json + label_map.json and provides
/// column-level inference without implementing ValueClassifier.
///
/// Supports both flat and hierarchical classification heads.
/// Merge normalization: either BatchNorm (legacy) or LayerNorm (GELU+LN models).
enum MergeNorm {
    Batch(BatchNorm),
    Layer(candle_nn::LayerNorm),
}

pub struct MultiBranchClassifier {
    char_branch: BranchWeights,
    embed_branch: BranchWeights,
    stats_branch: BranchWeights,
    header_branch: Option<BranchWeights>,
    merge_norm: MergeNorm,
    merge_linear1: Linear,
    merge_linear2: Linear,
    head: ClassificationHead,
    config: MultiBranchConfig,
    /// Index → label mapping (sorted by index).
    labels: Vec<String>,
    /// Model2Vec resources for embedding extraction.
    model2vec: Model2VecResources,
}

impl MultiBranchClassifier {
    /// Load a multi-branch model from a directory containing:
    /// - model.safetensors (weights)
    /// - config.json (architecture config)
    /// - label_map.json (index → label mapping)
    ///
    /// Also loads Model2Vec resources from models/model2vec/ (required for
    /// embedding feature extraction).
    ///
    /// Supports both flat and hierarchical head types. The hierarchical head
    /// builds its hierarchy map from the label list (domain.category.type format).
    pub fn load<P: AsRef<Path>>(model_dir: P) -> Result<Self, InferenceError> {
        let dir = model_dir.as_ref();

        // Load config
        let config_bytes = std::fs::read(dir.join("config.json"))
            .map_err(|e| InferenceError::InvalidPath(format!("Failed to read config.json: {e}")))?;
        let config: MultiBranchConfig = serde_json::from_slice(&config_bytes).map_err(|e| {
            InferenceError::InvalidPath(format!("Failed to parse config.json: {e}"))
        })?;

        // Load label map
        let label_bytes = std::fs::read(dir.join("label_map.json")).map_err(|e| {
            InferenceError::InvalidPath(format!("Failed to read label_map.json: {e}"))
        })?;
        let labels: Vec<String> = serde_json::from_slice(&label_bytes).map_err(|e| {
            InferenceError::InvalidPath(format!("Failed to parse label_map.json: {e}"))
        })?;

        if labels.len() != config.n_classes {
            return Err(InferenceError::InvalidPath(format!(
                "label_map.json has {} labels but config.json specifies n_classes={}",
                labels.len(),
                config.n_classes,
            )));
        }

        // Load weights
        let device = Device::Cpu;
        let model_bytes = std::fs::read(dir.join("model.safetensors")).map_err(|e| {
            InferenceError::InvalidPath(format!("Failed to read model.safetensors: {e}"))
        })?;
        let tensors = candle_core::safetensors::load_buffer(&model_bytes, &device)
            .map_err(|e| InferenceError::InvalidPath(format!("Failed to load safetensors: {e}")))?;

        let vb = VarBuilder::from_tensors(tensors, DType::F32, &device);

        // Build branches + merge trunk
        // When use_layer_norm=true, all branches get input LayerNorm.
        // When false, only header gets it (backward compatible).
        let build_branch =
            |input_dim, hidden, name: &str| -> Result<BranchWeights, InferenceError> {
                if config.use_layer_norm {
                    BranchWeights::new_with_input_norm(
                        input_dim,
                        hidden,
                        &config.activation,
                        vb.pp(name),
                    )
                } else {
                    BranchWeights::new(input_dim, hidden, &config.activation, vb.pp(name))
                }
                .map_err(|e| InferenceError::InvalidPath(format!("{name} branch: {e}")))
            };

        let char_branch = build_branch(config.char_dim, config.char_hidden, "char")?;
        let embed_branch = build_branch(config.embed_dim, config.embed_hidden, "embed")?;
        let stats_branch = build_branch(config.stats_dim, config.stats_hidden, "stats")?;

        // Load header branch if config enables it and weights exist in safetensors
        // Header always gets input LayerNorm (stabilises raw Model2Vec embeddings).
        let header_branch = if config.has_header_branch() {
            match BranchWeights::new_with_input_norm(
                config.header_dim,
                config.header_hidden,
                &config.activation,
                vb.pp("header"),
            ) {
                Ok(branch) => Some(branch),
                Err(e) => {
                    // Graceful fallback: old model without header weights
                    tracing::warn!("Header branch configured but weights missing, disabling: {e}");
                    None
                }
            }
        } else {
            None
        };

        let merged_dim = if header_branch.is_some() {
            config.merged_dim()
        } else {
            // Fall back to 3-branch merged dim if header branch not loaded
            config.char_hidden[1] + config.embed_hidden[1] + config.stats_hidden[1]
        };

        // Merge normalization: LayerNorm (new) or BatchNorm (legacy)
        let merge_norm = if config.use_layer_norm {
            MergeNorm::Layer(
                candle_nn::layer_norm(
                    merged_dim,
                    candle_nn::LayerNormConfig::default(),
                    vb.pp("merge_ln"),
                )
                .map_err(|e| InferenceError::InvalidPath(format!("merge_ln: {e}")))?,
            )
        } else {
            MergeNorm::Batch(
                batch_norm(merged_dim, BatchNormConfig::default(), vb.pp("merge_bn"))
                    .map_err(|e| InferenceError::InvalidPath(format!("merge_bn: {e}")))?,
            )
        };

        let merge_linear1 = linear(merged_dim, config.merge_hidden[0], vb.pp("merge_l1"))
            .map_err(|e| InferenceError::InvalidPath(format!("merge_l1: {e}")))?;
        let merge_linear2 = linear(
            config.merge_hidden[0],
            config.merge_hidden[1],
            vb.pp("merge_l2"),
        )
        .map_err(|e| InferenceError::InvalidPath(format!("merge_l2: {e}")))?;

        // Build classification head based on head_type
        let head = match config.head_type {
            HeadType::Flat => {
                let flat_head = linear(config.merge_hidden[1], config.n_classes, vb.pp("head"))
                    .map_err(|e| InferenceError::InvalidPath(format!("head: {e}")))?;
                ClassificationHead::Flat(flat_head)
            }
            HeadType::Hierarchical => {
                let hier_head = HierarchicalHead::new(
                    config.merge_hidden[1],
                    &labels,
                    vb.pp(HierarchicalHead::VARBUILDER_PREFIX),
                )
                .map_err(|e| InferenceError::InvalidPath(format!("hierarchical head: {e}")))?;
                ClassificationHead::Hierarchical(Box::new(hier_head))
            }
        };

        // Load Model2Vec resources
        let m2v = Self::load_model2vec(dir)?;

        Ok(Self {
            char_branch,
            embed_branch,
            stats_branch,
            header_branch,
            merge_norm,
            merge_linear1,
            merge_linear2,
            head,
            config,
            labels,
            model2vec: m2v,
        })
    }

    /// Load Model2Vec resources. Tries model_dir/model2vec/ first, then
    /// falls back to models/model2vec/ (shared location).
    fn load_model2vec(model_dir: &Path) -> Result<Model2VecResources, InferenceError> {
        // Try model-local first
        let local_m2v = model_dir.join("model2vec");
        if local_m2v.join("model.safetensors").exists() {
            return Model2VecResources::load(&local_m2v);
        }

        // Try shared location
        let shared_m2v = std::path::PathBuf::from("models/model2vec");
        if shared_m2v.join("model.safetensors").exists() {
            return Model2VecResources::load(&shared_m2v);
        }

        Err(InferenceError::InvalidPath(
            "Model2Vec resources not found. Checked: model_dir/model2vec/, models/model2vec/"
                .into(),
        ))
    }

    /// Classify a column of values, returning (label, confidence).
    ///
    /// Extracts branch features from the values, runs the MLP forward pass,
    /// and returns the predicted label with confidence score.
    ///
    /// For flat heads: confidence is the softmax probability of the top prediction.
    /// For hierarchical heads: confidence is the product probability (domain × category × type).
    ///
    /// `header` is the column name. When non-empty and the model has a header branch,
    /// it is embedded via Model2Vec and fed through the 4th branch.
    pub fn classify_column(
        &self,
        values: &[String],
        header: &str,
    ) -> Result<(String, f32), InferenceError> {
        if values.is_empty() {
            return Ok(("unknown".to_string(), 0.0));
        }

        let value_refs: Vec<&str> = values.iter().map(|s| s.as_str()).collect();

        // Extract features
        let char_feats = extract_char_distribution(&value_refs).unwrap_or([0.0f32; CHAR_DIST_DIM]);
        let embed_feats = extract_embedding_aggregation(&value_refs, &self.model2vec)
            .unwrap_or([0.0f32; EMBED_AGG_DIM]);
        let stats_feats = extract_column_stats(&value_refs).unwrap_or([0.0f32; COLUMN_STATS_DIM]);

        // Forward pass through trunk
        let device = Device::Cpu;
        let char_t = Tensor::from_slice(&char_feats, (1, CHAR_DIST_DIM), &device)
            .map_err(|e| InferenceError::InvalidPath(format!("char tensor: {e}")))?;
        let embed_t = Tensor::from_slice(&embed_feats, (1, EMBED_AGG_DIM), &device)
            .map_err(|e| InferenceError::InvalidPath(format!("embed tensor: {e}")))?;
        let stats_t = Tensor::from_slice(&stats_feats, (1, COLUMN_STATS_DIM), &device)
            .map_err(|e| InferenceError::InvalidPath(format!("stats tensor: {e}")))?;

        // Extract header embedding if the model has a header branch
        let header_t = if self.header_branch.is_some() {
            let header_embed = if !header.is_empty() {
                self.model2vec
                    .encode_one(header)
                    .and_then(|t| t.to_vec1::<f32>().ok())
                    .unwrap_or_else(|| vec![0.0f32; self.config.header_dim])
            } else {
                vec![0.0f32; self.config.header_dim]
            };
            Some(
                Tensor::from_slice(&header_embed, (1, self.config.header_dim), &device)
                    .map_err(|e| InferenceError::InvalidPath(format!("header tensor: {e}")))?,
            )
        } else {
            None
        };

        let hidden = self.forward_trunk(&char_t, &embed_t, &stats_t, header_t.as_ref())?;

        // Head-specific forward pass + probability extraction
        let probs_vec = match &self.head {
            ClassificationHead::Flat(head) => {
                // Flat: hidden → logits → softmax → probabilities
                let logits = head
                    .forward_t(&hidden, false)
                    .map_err(|e| InferenceError::InvalidPath(format!("head: {e}")))?;
                let probs = candle_nn::ops::softmax(&logits, 1)
                    .map_err(|e| InferenceError::InvalidPath(format!("softmax: {e}")))?;
                probs
                    .squeeze(0)
                    .map_err(|e| InferenceError::InvalidPath(format!("squeeze: {e}")))?
                    .to_vec1::<f32>()
                    .map_err(|e| InferenceError::InvalidPath(format!("to_vec1: {e}")))?
            }
            ClassificationHead::Hierarchical(hier_head) => {
                // Hierarchical: hidden → tree softmax → product probabilities
                // forward() already returns probabilities (not logits)
                let probs = hier_head
                    .forward(&hidden, self.config.n_classes)
                    .map_err(|e| {
                        InferenceError::InvalidPath(format!("hierarchical forward: {e}"))
                    })?;
                probs
                    .squeeze(0)
                    .map_err(|e| InferenceError::InvalidPath(format!("squeeze: {e}")))?
                    .to_vec1::<f32>()
                    .map_err(|e| InferenceError::InvalidPath(format!("to_vec1: {e}")))?
            }
        };

        let (max_idx, max_prob) = probs_vec
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap();

        let label = self
            .labels
            .get(max_idx)
            .cloned()
            .unwrap_or_else(|| format!("unknown_idx_{max_idx}"));

        Ok((label, *max_prob))
    }

    /// Classify a column using a pre-enriched header embedding.
    ///
    /// Like `classify_column()`, but accepts a pre-computed header tensor
    /// (e.g., from sibling-context attention) instead of embedding the raw
    /// header string via Model2Vec. Used when sibling context is available
    /// to pass enriched header embeddings into the multi-branch model.
    pub fn classify_column_with_enriched_header(
        &self,
        values: &[String],
        enriched_header: &Tensor, // [D] — already enriched by sibling attention
    ) -> Result<(String, f32), InferenceError> {
        if values.is_empty() {
            return Ok(("unknown".to_string(), 0.0));
        }

        let value_refs: Vec<&str> = values.iter().map(|s| s.as_str()).collect();

        // Extract features (same as classify_column)
        let char_feats = extract_char_distribution(&value_refs).unwrap_or([0.0f32; CHAR_DIST_DIM]);
        let embed_feats = extract_embedding_aggregation(&value_refs, &self.model2vec)
            .unwrap_or([0.0f32; EMBED_AGG_DIM]);
        let stats_feats = extract_column_stats(&value_refs).unwrap_or([0.0f32; COLUMN_STATS_DIM]);

        let device = Device::Cpu;
        let char_t = Tensor::from_slice(&char_feats, (1, CHAR_DIST_DIM), &device)
            .map_err(|e| InferenceError::InvalidPath(format!("char tensor: {e}")))?;
        let embed_t = Tensor::from_slice(&embed_feats, (1, EMBED_AGG_DIM), &device)
            .map_err(|e| InferenceError::InvalidPath(format!("embed tensor: {e}")))?;
        let stats_t = Tensor::from_slice(&stats_feats, (1, COLUMN_STATS_DIM), &device)
            .map_err(|e| InferenceError::InvalidPath(format!("stats tensor: {e}")))?;

        // Use the pre-enriched header embedding (reshaped to [1, D])
        let header_t = if self.header_branch.is_some() {
            let header_embed = enriched_header
                .to_vec1::<f32>()
                .unwrap_or_else(|_| vec![0.0f32; self.config.header_dim]);
            Some(
                Tensor::from_slice(&header_embed, (1, self.config.header_dim), &device)
                    .map_err(|e| InferenceError::InvalidPath(format!("header tensor: {e}")))?,
            )
        } else {
            None
        };

        let hidden = self.forward_trunk(&char_t, &embed_t, &stats_t, header_t.as_ref())?;

        // Head-specific forward pass (same as classify_column)
        let probs_vec = match &self.head {
            ClassificationHead::Flat(head) => {
                let logits = head
                    .forward_t(&hidden, false)
                    .map_err(|e| InferenceError::InvalidPath(format!("head: {e}")))?;
                let probs = candle_nn::ops::softmax(&logits, 1)
                    .map_err(|e| InferenceError::InvalidPath(format!("softmax: {e}")))?;
                probs
                    .squeeze(0)
                    .map_err(|e| InferenceError::InvalidPath(format!("squeeze: {e}")))?
                    .to_vec1::<f32>()
                    .map_err(|e| InferenceError::InvalidPath(format!("to_vec1: {e}")))?
            }
            ClassificationHead::Hierarchical(hier_head) => {
                let probs = hier_head
                    .forward(&hidden, self.config.n_classes)
                    .map_err(|e| {
                        InferenceError::InvalidPath(format!("hierarchical forward: {e}"))
                    })?;
                probs
                    .squeeze(0)
                    .map_err(|e| InferenceError::InvalidPath(format!("squeeze: {e}")))?
                    .to_vec1::<f32>()
                    .map_err(|e| InferenceError::InvalidPath(format!("to_vec1: {e}")))?
            }
        };

        let (max_idx, max_prob) = probs_vec
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap();

        let label = self
            .labels
            .get(max_idx)
            .cloned()
            .unwrap_or_else(|| format!("unknown_idx_{max_idx}"));

        Ok((label, *max_prob))
    }

    /// Forward pass through the trunk (branches + merge), returning the hidden
    /// representation before the classification head.
    fn forward_trunk(
        &self,
        char_feats: &Tensor,
        embed_feats: &Tensor,
        stats_feats: &Tensor,
        header_feats: Option<&Tensor>,
    ) -> Result<Tensor, InferenceError> {
        let char_out = self
            .char_branch
            .forward(char_feats)
            .map_err(|e| InferenceError::InvalidPath(format!("char forward: {e}")))?;
        let embed_out = self
            .embed_branch
            .forward(embed_feats)
            .map_err(|e| InferenceError::InvalidPath(format!("embed forward: {e}")))?;
        let stats_out = self
            .stats_branch
            .forward(stats_feats)
            .map_err(|e| InferenceError::InvalidPath(format!("stats forward: {e}")))?;

        let merged = if let Some(ref hb) = self.header_branch {
            let batch_size = char_out
                .dim(0)
                .map_err(|e| InferenceError::InvalidPath(format!("batch dim: {e}")))?;
            let header_input = match header_feats {
                Some(hf) => hf.clone(),
                None => Tensor::zeros(
                    (batch_size, self.config.header_dim),
                    candle_core::DType::F32,
                    char_feats.device(),
                )
                .map_err(|e| InferenceError::InvalidPath(format!("header zeros: {e}")))?,
            };
            let header_out = hb
                .forward(&header_input)
                .map_err(|e| InferenceError::InvalidPath(format!("header forward: {e}")))?;
            Tensor::cat(&[char_out, embed_out, stats_out, header_out], 1)
                .map_err(|e| InferenceError::InvalidPath(format!("concat: {e}")))?
        } else {
            Tensor::cat(&[char_out, embed_out, stats_out], 1)
                .map_err(|e| InferenceError::InvalidPath(format!("concat: {e}")))?
        };

        // Merge normalization
        let normed = match &self.merge_norm {
            MergeNorm::Batch(bn) => {
                // BatchNorm: [B, C] -> [B, C, 1] -> BN -> [B, C]
                let merged_3d = merged
                    .unsqueeze(2)
                    .map_err(|e| InferenceError::InvalidPath(format!("unsqueeze: {e}")))?;
                let normed_3d = bn
                    .forward_t(&merged_3d, false)
                    .map_err(|e| InferenceError::InvalidPath(format!("batch_norm: {e}")))?;
                normed_3d
                    .squeeze(2)
                    .map_err(|e| InferenceError::InvalidPath(format!("squeeze: {e}")))?
            }
            MergeNorm::Layer(ln) => {
                // LayerNorm works directly on [B, C]
                ln.forward(&merged)
                    .map_err(|e| InferenceError::InvalidPath(format!("layer_norm: {e}")))?
            }
        };

        let activate = |x: &Tensor, label: &str| -> Result<Tensor, InferenceError> {
            match self.config.activation {
                Activation::ReLU => x
                    .relu()
                    .map_err(|e| InferenceError::InvalidPath(format!("{label}: {e}"))),
                Activation::GELU => x
                    .gelu_erf()
                    .map_err(|e| InferenceError::InvalidPath(format!("{label}: {e}"))),
            }
        };

        let h = self
            .merge_linear1
            .forward_t(&normed, false)
            .map_err(|e| InferenceError::InvalidPath(format!("merge_l1: {e}")))?;
        let h = activate(&h, "act1")?;
        let h = self
            .merge_linear2
            .forward_t(&h, false)
            .map_err(|e| InferenceError::InvalidPath(format!("merge_l2: {e}")))?;
        activate(&h, "act2")
    }

    /// Return the number of output classes.
    pub fn n_classes(&self) -> usize {
        self.config.n_classes
    }

    /// Return the label list (index → label mapping).
    pub fn labels(&self) -> &[String] {
        &self.labels
    }

    /// Return the head type of this model.
    pub fn head_type(&self) -> &HeadType {
        &self.config.head_type
    }

    /// Check if a model directory contains a multi-branch model.
    ///
    /// Looks for model.safetensors + config.json where config contains
    /// multi-branch fields (char_dim, embed_dim, stats_dim).
    pub fn is_multi_branch_dir<P: AsRef<Path>>(dir: P) -> bool {
        let dir = dir.as_ref();
        let config_path = dir.join("config.json");
        let model_path = dir.join("model.safetensors");
        let label_path = dir.join("label_map.json");

        if !config_path.exists() || !model_path.exists() || !label_path.exists() {
            return false;
        }

        // Check config has multi-branch fields
        if let Ok(bytes) = std::fs::read(&config_path) {
            if let Ok(config) = serde_json::from_slice::<MultiBranchConfig>(&bytes) {
                // Multi-branch models have char_dim, embed_dim, stats_dim
                return config.char_dim > 0 && config.embed_dim > 0 && config.stats_dim > 0;
            }
        }

        false
    }

    /// Get a reference to the Model2Vec resources (for external use).
    pub fn model2vec(&self) -> &Model2VecResources {
        &self.model2vec
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_deserialization() {
        // Old-style config without header/activation/layer_norm fields (backward compat)
        let json = r#"{
            "char_dim": 960,
            "embed_dim": 512,
            "stats_dim": 27,
            "char_hidden": [300, 300],
            "embed_hidden": [200, 200],
            "stats_hidden": [128, 64],
            "merge_hidden": [500, 500],
            "n_classes": 250,
            "dropout": 0.35,
            "head_type": "Flat"
        }"#;
        let config: MultiBranchConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.char_dim, 960);
        assert_eq!(config.embed_dim, 512);
        assert_eq!(config.stats_dim, 27);
        assert_eq!(config.header_dim, 0); // default
        assert_eq!(config.header_hidden, [0, 0]); // default
        assert!(!config.has_header_branch());
        assert_eq!(config.n_classes, 250);
        assert_eq!(config.merged_dim(), 564); // 3-branch only
        // New fields default to backward-compatible values
        assert_eq!(config.activation, Activation::ReLU);
        assert!(!config.use_layer_norm);
    }

    #[test]
    fn test_config_deserialization_with_header() {
        let json = r#"{
            "char_dim": 960,
            "embed_dim": 512,
            "stats_dim": 27,
            "header_dim": 128,
            "char_hidden": [300, 300],
            "embed_hidden": [200, 200],
            "stats_hidden": [128, 64],
            "header_hidden": [128, 64],
            "merge_hidden": [500, 500],
            "n_classes": 250,
            "dropout": 0.35,
            "head_type": "Flat"
        }"#;
        let config: MultiBranchConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.header_dim, 128);
        assert_eq!(config.header_hidden, [128, 64]);
        assert!(config.has_header_branch());
        assert_eq!(config.merged_dim(), 628); // 300+200+64+64
    }

    #[test]
    fn test_config_deserialization_gelu_layer_norm() {
        // New-style config with GELU + LayerNorm (autoresearch winner)
        let json = r#"{
            "char_dim": 960,
            "embed_dim": 512,
            "stats_dim": 27,
            "header_dim": 128,
            "char_hidden": [450, 450],
            "embed_hidden": [300, 300],
            "stats_hidden": [192, 96],
            "header_hidden": [750, 750],
            "merge_hidden": [500, 500],
            "n_classes": 250,
            "dropout": 0.35,
            "head_type": "Flat",
            "activation": "GELU",
            "use_layer_norm": true
        }"#;
        let config: MultiBranchConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.activation, Activation::GELU);
        assert!(config.use_layer_norm);
        assert!(config.has_header_branch());
    }

    #[test]
    fn test_config_deserialization_hierarchical() {
        let json = r#"{
            "char_dim": 960,
            "embed_dim": 512,
            "stats_dim": 27,
            "char_hidden": [300, 300],
            "embed_hidden": [200, 200],
            "stats_hidden": [128, 64],
            "merge_hidden": [500, 500],
            "n_classes": 250,
            "dropout": 0.35,
            "head_type": "Hierarchical"
        }"#;
        let config: MultiBranchConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.head_type, HeadType::Hierarchical);
        assert_eq!(config.n_classes, 250);
    }

    #[test]
    fn test_is_multi_branch_dir_missing_files() {
        // Use a path that definitely doesn't contain model files
        assert!(!MultiBranchClassifier::is_multi_branch_dir(
            "/tmp/nonexistent-finetype-test"
        ));
    }
}
