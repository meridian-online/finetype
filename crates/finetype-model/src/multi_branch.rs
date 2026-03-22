//! Multi-branch model loader and inference for column-level classification.
//!
//! Loads multi-branch model artifacts (model.safetensors + config.json + label_map.json)
//! and provides column-level inference: Vec<String> → features → MLP forward → label.
//!
//! Architecture (from finetype-train):
//! ```text
//! Branch 1 (char):  [960] → Dense(300, ReLU) → Dense(300, ReLU) → [300]
//! Branch 2 (embed): [512] → Dense(200, ReLU) → Dense(200, ReLU) → [200]
//! Branch 3 (stats): [27]  → Dense(128, ReLU) → Dense(64, ReLU)  → [64]
//!                             ↓
//! Merge:             concat([300, 200, 64]) = [564]
//!                             ↓
//!                    BatchNorm → Dense(500, ReLU) → Dense(500, ReLU)
//!                             ↓
//! Head (flat):       Dense(n_classes, softmax)
//! ```

use crate::char_distribution::{extract_char_distribution, CHAR_DIST_DIM};
use crate::column_stats::{extract_column_stats, COLUMN_STATS_DIM};
use crate::embedding_aggregation::{extract_embedding_aggregation, EMBED_AGG_DIM};
use crate::inference::InferenceError;
use crate::model2vec_shared::Model2VecResources;
use candle_core::{DType, Device, Tensor};
use candle_nn::{batch_norm, linear, BatchNorm, BatchNormConfig, Linear, ModuleT, VarBuilder};
use serde::{Deserialize, Serialize};
use std::path::Path;

// ═══════════════════════════════════════════════════════════════════════════════
// Configuration (mirrors finetype-train MultiBranchConfig for deserialization)
// ═══════════════════════════════════════════════════════════════════════════════

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
    pub char_hidden: [usize; 2],
    pub embed_hidden: [usize; 2],
    pub stats_hidden: [usize; 2],
    pub merge_hidden: [usize; 2],
    pub n_classes: usize,
    pub dropout: f32,
    pub head_type: HeadType,
}

impl MultiBranchConfig {
    fn merged_dim(&self) -> usize {
        self.char_hidden[1] + self.embed_hidden[1] + self.stats_hidden[1]
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Branch weights (2-layer MLP, inference only — no dropout)
// ═══════════════════════════════════════════════════════════════════════════════

struct BranchWeights {
    linear1: Linear,
    linear2: Linear,
}

impl BranchWeights {
    fn new(input_dim: usize, hidden: [usize; 2], vb: VarBuilder) -> candle_core::Result<Self> {
        let linear1 = linear(input_dim, hidden[0], vb.pp("l1"))?;
        let linear2 = linear(hidden[0], hidden[1], vb.pp("l2"))?;
        Ok(Self { linear1, linear2 })
    }

    fn forward(&self, x: &Tensor) -> candle_core::Result<Tensor> {
        let h = self.linear1.forward_t(x, false)?;
        let h = h.relu()?;
        let h = self.linear2.forward_t(&h, false)?;
        h.relu()
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Model (inference only)
// ═══════════════════════════════════════════════════════════════════════════════

/// Multi-branch model for column-level type classification.
///
/// Loads from safetensors + config.json + label_map.json and provides
/// column-level inference without implementing ValueClassifier.
pub struct MultiBranchClassifier {
    char_branch: BranchWeights,
    embed_branch: BranchWeights,
    stats_branch: BranchWeights,
    merge_bn: BatchNorm,
    merge_linear1: Linear,
    merge_linear2: Linear,
    head: Linear,
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
    pub fn load<P: AsRef<Path>>(model_dir: P) -> Result<Self, InferenceError> {
        let dir = model_dir.as_ref();

        // Load config
        let config_bytes = std::fs::read(dir.join("config.json")).map_err(|e| {
            InferenceError::InvalidPath(format!("Failed to read config.json: {e}"))
        })?;
        let config: MultiBranchConfig = serde_json::from_slice(&config_bytes).map_err(|e| {
            InferenceError::InvalidPath(format!("Failed to parse config.json: {e}"))
        })?;

        if config.head_type != HeadType::Flat {
            return Err(InferenceError::InvalidPath(
                "Only flat head is supported for multi-branch inference (hierarchical not yet implemented)".into(),
            ));
        }

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

        // Build model
        let char_branch =
            BranchWeights::new(config.char_dim, config.char_hidden, vb.pp("char"))
                .map_err(|e| InferenceError::InvalidPath(format!("char branch: {e}")))?;
        let embed_branch =
            BranchWeights::new(config.embed_dim, config.embed_hidden, vb.pp("embed"))
                .map_err(|e| InferenceError::InvalidPath(format!("embed branch: {e}")))?;
        let stats_branch =
            BranchWeights::new(config.stats_dim, config.stats_hidden, vb.pp("stats"))
                .map_err(|e| InferenceError::InvalidPath(format!("stats branch: {e}")))?;

        let merged_dim = config.merged_dim();
        let merge_bn =
            batch_norm(merged_dim, BatchNormConfig::default(), vb.pp("merge_bn"))
                .map_err(|e| InferenceError::InvalidPath(format!("merge_bn: {e}")))?;
        let merge_linear1 =
            linear(merged_dim, config.merge_hidden[0], vb.pp("merge_l1"))
                .map_err(|e| InferenceError::InvalidPath(format!("merge_l1: {e}")))?;
        let merge_linear2 = linear(
            config.merge_hidden[0],
            config.merge_hidden[1],
            vb.pp("merge_l2"),
        )
        .map_err(|e| InferenceError::InvalidPath(format!("merge_l2: {e}")))?;

        let head = linear(config.merge_hidden[1], config.n_classes, vb.pp("head"))
            .map_err(|e| InferenceError::InvalidPath(format!("head: {e}")))?;

        // Load Model2Vec resources
        let m2v = Self::load_model2vec(dir)?;

        Ok(Self {
            char_branch,
            embed_branch,
            stats_branch,
            merge_bn,
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
    /// Extracts 3-branch features from the values, runs the MLP forward pass,
    /// and returns the predicted label with softmax confidence.
    pub fn classify_column(
        &self,
        values: &[String],
    ) -> Result<(String, f32), InferenceError> {
        if values.is_empty() {
            return Ok(("unknown".to_string(), 0.0));
        }

        let value_refs: Vec<&str> = values.iter().map(|s| s.as_str()).collect();

        // Extract features
        let char_feats =
            extract_char_distribution(&value_refs).unwrap_or([0.0f32; CHAR_DIST_DIM]);
        let embed_feats =
            extract_embedding_aggregation(&value_refs, &self.model2vec)
                .unwrap_or([0.0f32; EMBED_AGG_DIM]);
        let stats_feats =
            extract_column_stats(&value_refs).unwrap_or([0.0f32; COLUMN_STATS_DIM]);

        // Forward pass
        let device = Device::Cpu;
        let char_t = Tensor::from_slice(&char_feats, (1, CHAR_DIST_DIM), &device)
            .map_err(|e| InferenceError::InvalidPath(format!("char tensor: {e}")))?;
        let embed_t = Tensor::from_slice(&embed_feats, (1, EMBED_AGG_DIM), &device)
            .map_err(|e| InferenceError::InvalidPath(format!("embed tensor: {e}")))?;
        let stats_t = Tensor::from_slice(&stats_feats, (1, COLUMN_STATS_DIM), &device)
            .map_err(|e| InferenceError::InvalidPath(format!("stats tensor: {e}")))?;

        let logits = self.forward(&char_t, &embed_t, &stats_t)?;

        // Softmax + argmax
        let probs = candle_nn::ops::softmax(&logits, 1)
            .map_err(|e| InferenceError::InvalidPath(format!("softmax: {e}")))?;
        let probs_vec: Vec<f32> = probs
            .squeeze(0)
            .map_err(|e| InferenceError::InvalidPath(format!("squeeze: {e}")))?
            .to_vec1()
            .map_err(|e| InferenceError::InvalidPath(format!("to_vec1: {e}")))?;

        let (max_idx, max_prob) = probs_vec
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .unwrap();

        let label = self.labels.get(max_idx).cloned().unwrap_or_else(|| {
            format!("unknown_idx_{max_idx}")
        });

        Ok((label, *max_prob))
    }

    /// Forward pass through the model (inference mode, no dropout).
    fn forward(
        &self,
        char_feats: &Tensor,
        embed_feats: &Tensor,
        stats_feats: &Tensor,
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

        let merged = Tensor::cat(&[char_out, embed_out, stats_out], 1)
            .map_err(|e| InferenceError::InvalidPath(format!("concat: {e}")))?;

        // BatchNorm: [B, C] → [B, C, 1] → BN → [B, C]
        let merged_3d = merged
            .unsqueeze(2)
            .map_err(|e| InferenceError::InvalidPath(format!("unsqueeze: {e}")))?;
        let normed_3d = self
            .merge_bn
            .forward_t(&merged_3d, false)
            .map_err(|e| InferenceError::InvalidPath(format!("batch_norm: {e}")))?;
        let normed = normed_3d
            .squeeze(2)
            .map_err(|e| InferenceError::InvalidPath(format!("squeeze: {e}")))?;

        let h = self
            .merge_linear1
            .forward_t(&normed, false)
            .map_err(|e| InferenceError::InvalidPath(format!("merge_l1: {e}")))?;
        let h = h
            .relu()
            .map_err(|e| InferenceError::InvalidPath(format!("relu1: {e}")))?;
        let h = self
            .merge_linear2
            .forward_t(&h, false)
            .map_err(|e| InferenceError::InvalidPath(format!("merge_l2: {e}")))?;
        let h = h
            .relu()
            .map_err(|e| InferenceError::InvalidPath(format!("relu2: {e}")))?;

        self.head
            .forward_t(&h, false)
            .map_err(|e| InferenceError::InvalidPath(format!("head: {e}")))
    }

    /// Return the number of output classes.
    pub fn n_classes(&self) -> usize {
        self.config.n_classes
    }

    /// Return the label list (index → label mapping).
    pub fn labels(&self) -> &[String] {
        &self.labels
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
        assert_eq!(config.n_classes, 250);
        assert_eq!(config.merged_dim(), 564);
    }

    #[test]
    fn test_is_multi_branch_dir_missing_files() {
        // Use a path that definitely doesn't contain model files
        assert!(!MultiBranchClassifier::is_multi_branch_dir("/tmp/nonexistent-finetype-test"));
    }
}
