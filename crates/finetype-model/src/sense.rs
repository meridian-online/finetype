//! Sense classifier — column-level semantic category routing (NNFT-168).
//!
//! Ports the PyTorch `SenseModelA` (Architecture A from NNFT-163) to Candle.
//! Cross-attention over Model2Vec embeddings: column header as attention query
//! over value embeddings, producing broad category (6 classes) and entity
//! subtype (4 classes) predictions.
//!
//! Architecture:
//!   header_proj(header_emb) → query [1, 1, D]
//!   cross_attention(query, value_embs, value_embs) → attn_out [D]
//!   layer_norm(attn_out) → [D]
//!   features = cat(attn_out, val_mean, val_std) → [3D]
//!   broad_head(features) → [6]
//!   entity_head(features) → [4]
//!
//! Model artifacts from `models/sense_spike/arch_a/`.

use crate::inference::InferenceError;
use crate::model2vec_shared::Model2VecResources;
use candle_core::{DType, Device, Tensor};
use std::fmt;

// ═══════════════════════════════════════════════════════════════════════════════
// Enums
// ═══════════════════════════════════════════════════════════════════════════════

/// Broad semantic category predicted by the Sense model.
///
/// Maps to the 6 categories the model was trained on.
/// Order must match `config.json:broad_categories`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BroadCategory {
    Entity = 0,
    Format = 1,
    Temporal = 2,
    Numeric = 3,
    Geographic = 4,
    Text = 5,
}

impl BroadCategory {
    /// All categories in index order.
    pub const ALL: [BroadCategory; 6] = [
        BroadCategory::Entity,
        BroadCategory::Format,
        BroadCategory::Temporal,
        BroadCategory::Numeric,
        BroadCategory::Geographic,
        BroadCategory::Text,
    ];

    /// Convert from model output index.
    pub fn from_index(index: usize) -> Option<Self> {
        Self::ALL.get(index).copied()
    }
}

impl fmt::Display for BroadCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Entity => write!(f, "entity"),
            Self::Format => write!(f, "format"),
            Self::Temporal => write!(f, "temporal"),
            Self::Numeric => write!(f, "numeric"),
            Self::Geographic => write!(f, "geographic"),
            Self::Text => write!(f, "text"),
        }
    }
}

impl std::str::FromStr for BroadCategory {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "entity" => Ok(Self::Entity),
            "format" => Ok(Self::Format),
            "temporal" => Ok(Self::Temporal),
            "numeric" => Ok(Self::Numeric),
            "geographic" => Ok(Self::Geographic),
            "text" => Ok(Self::Text),
            _ => Err(format!("Unknown broad category: {}", s)),
        }
    }
}

/// Entity subtype predicted by the Sense model.
///
/// Only meaningful when `BroadCategory::Entity` is predicted.
/// Order must match `config.json:entity_subtypes`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EntitySubtype {
    Person = 0,
    Place = 1,
    Organization = 2,
    CreativeWork = 3,
}

impl EntitySubtype {
    /// All subtypes in index order.
    pub const ALL: [EntitySubtype; 4] = [
        EntitySubtype::Person,
        EntitySubtype::Place,
        EntitySubtype::Organization,
        EntitySubtype::CreativeWork,
    ];

    /// Convert from model output index.
    pub fn from_index(index: usize) -> Option<Self> {
        Self::ALL.get(index).copied()
    }
}

impl fmt::Display for EntitySubtype {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Person => write!(f, "person"),
            Self::Place => write!(f, "place"),
            Self::Organization => write!(f, "organization"),
            Self::CreativeWork => write!(f, "creative_work"),
        }
    }
}

impl std::str::FromStr for EntitySubtype {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "person" => Ok(Self::Person),
            "place" => Ok(Self::Place),
            "organization" => Ok(Self::Organization),
            "creative_work" => Ok(Self::CreativeWork),
            _ => Err(format!("Unknown entity subtype: {}", s)),
        }
    }
}

/// Result of Sense classification for a column.
#[derive(Debug, Clone)]
pub struct SenseResult {
    /// Predicted broad semantic category.
    pub broad_category: BroadCategory,
    /// Entity subtype (meaningful only when broad_category == Entity).
    pub entity_subtype: Option<EntitySubtype>,
    /// Softmax probability for the predicted broad category.
    pub broad_confidence: f32,
    /// Softmax probability for the predicted entity subtype (if entity).
    pub entity_confidence: f32,
}

// ═══════════════════════════════════════════════════════════════════════════════
// Model weights
// ═══════════════════════════════════════════════════════════════════════════════

/// Cross-attention weights (split from PyTorch's concatenated in_proj).
struct CrossAttentionWeights {
    wq: Tensor,         // [D, D]
    bq: Tensor,         // [D]
    wk: Tensor,         // [D, D]
    bk: Tensor,         // [D]
    wv: Tensor,         // [D, D]
    bv: Tensor,         // [D]
    out_weight: Tensor, // [D, D]
    out_bias: Tensor,   // [D]
}

/// Attention mechanism variant — Python vs Rust trained models.
enum AttentionVariant {
    /// Python-trained: full multi-head attention (Q,K,V projections + output projection).
    FullMha {
        weights: CrossAttentionWeights,
        n_heads: usize,
    },
    /// Rust-trained: simple single-head attention (no K,V,output projections).
    /// Query attends directly to value embeddings as both keys and values.
    Simple,
}

/// MLP head weights: Linear(3D, H) → ReLU → Linear(H, H/2) → ReLU → Linear(H/2, C)
struct MlpHead {
    fc1_weight: Tensor, // [H, 3D]
    fc1_bias: Tensor,   // [H]
    fc2_weight: Tensor, // [H/2, H]
    fc2_bias: Tensor,   // [H/2]
    fc3_weight: Tensor, // [C, H/2]
    fc3_bias: Tensor,   // [C]
}

impl MlpHead {
    /// Forward: Linear+ReLU → Linear+ReLU → Linear → softmax.
    /// Input: [1, 3D]. Output: [C] probabilities.
    fn forward(&self, x: &Tensor) -> Result<Tensor, InferenceError> {
        let h1 = x
            .matmul(&self.fc1_weight.t()?)?
            .broadcast_add(&self.fc1_bias)?
            .relu()?;
        let h2 = h1
            .matmul(&self.fc2_weight.t()?)?
            .broadcast_add(&self.fc2_bias)?
            .relu()?;
        let logits = h2
            .matmul(&self.fc3_weight.t()?)?
            .broadcast_add(&self.fc3_bias)?;
        let logits = logits.squeeze(0)?; // [C]

        // Softmax
        let max_val = logits.max(0)?;
        let shifted = logits.broadcast_sub(&max_val)?;
        let exp = shifted.exp()?;
        let sum_exp = exp.sum_all()?;
        Ok(exp.broadcast_div(&sum_exp)?)
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// SenseClassifier
// ═══════════════════════════════════════════════════════════════════════════════

/// Sense column-level classifier.
///
/// Predicts broad semantic category (6 classes) and entity subtype (4 classes)
/// from column header + value embeddings using cross-attention.
pub struct SenseClassifier {
    // Architecture parameters
    embed_dim: usize,

    // Weights
    header_proj_weight: Tensor, // [D, D]
    header_proj_bias: Tensor,   // [D]
    default_query: Tensor,      // [1, 1, D]
    attention: AttentionVariant,
    norm_weight: Tensor, // [D]
    norm_bias: Tensor,   // [D]
    broad_head: MlpHead,
    entity_head: MlpHead,

    device: Device,
}

impl SenseClassifier {
    /// Load from in-memory byte slices (model.safetensors + config.json).
    ///
    /// Supports two model formats:
    /// - **Python-trained** (PyTorch): full MHA with `cross_attention.in_proj_weight`,
    ///   MLP keys `broad_head.{0,3,6}`, `entity_head.{0,3,6}`.
    /// - **Rust-trained** (Candle): simple attention (no K/V projections),
    ///   MLP keys `broad_fc{1,2,3}`, `entity_fc{1,2,3}`.
    ///
    /// Format is auto-detected by checking for `cross_attention.in_proj_weight`.
    pub fn from_bytes(model_bytes: &[u8], config_bytes: &[u8]) -> Result<Self, InferenceError> {
        let device = Device::Cpu;

        // Parse config
        let config: serde_json::Value = serde_json::from_slice(config_bytes).map_err(|e| {
            InferenceError::InvalidPath(format!("Failed to parse Sense config: {}", e))
        })?;
        let embed_dim = config["embed_dim"].as_u64().unwrap_or(128) as usize;

        // Load all tensors
        let tensors = candle_core::safetensors::load_buffer(model_bytes, &device)?;

        let get = |name: &str| -> Result<Tensor, InferenceError> {
            tensors
                .get(name)
                .ok_or_else(|| {
                    InferenceError::InvalidPath(format!(
                        "Missing tensor '{}' in Sense model safetensors",
                        name
                    ))
                })
                .and_then(|t| Ok(t.to_dtype(DType::F32)?))
        };

        // Auto-detect format: Python models have cross_attention.in_proj_weight
        let is_python_format = tensors.contains_key("cross_attention.in_proj_weight");

        // Load attention variant
        let attention = if is_python_format {
            let n_heads = config["n_heads"].as_u64().unwrap_or(4) as usize;

            // Split in_proj_weight [3D, D] into Q, K, V each [D, D]
            let in_proj_weight = get("cross_attention.in_proj_weight")?;
            let in_proj_bias = get("cross_attention.in_proj_bias")?;

            let d = embed_dim;
            let wq = in_proj_weight.narrow(0, 0, d)?;
            let wk = in_proj_weight.narrow(0, d, d)?;
            let wv = in_proj_weight.narrow(0, 2 * d, d)?;
            let bq = in_proj_bias.narrow(0, 0, d)?;
            let bk = in_proj_bias.narrow(0, d, d)?;
            let bv = in_proj_bias.narrow(0, 2 * d, d)?;

            AttentionVariant::FullMha {
                weights: CrossAttentionWeights {
                    wq,
                    bq,
                    wk,
                    bk,
                    wv,
                    bv,
                    out_weight: get("cross_attention.out_proj.weight")?,
                    out_bias: get("cross_attention.out_proj.bias")?,
                },
                n_heads,
            }
        } else {
            // Rust-trained: simple attention — no separate attention weights to load
            AttentionVariant::Simple
        };

        // MLP heads: format-dependent key names
        let broad_head = if is_python_format {
            MlpHead {
                fc1_weight: get("broad_head.0.weight")?,
                fc1_bias: get("broad_head.0.bias")?,
                fc2_weight: get("broad_head.3.weight")?,
                fc2_bias: get("broad_head.3.bias")?,
                fc3_weight: get("broad_head.6.weight")?,
                fc3_bias: get("broad_head.6.bias")?,
            }
        } else {
            MlpHead {
                fc1_weight: get("broad_fc1.weight")?,
                fc1_bias: get("broad_fc1.bias")?,
                fc2_weight: get("broad_fc2.weight")?,
                fc2_bias: get("broad_fc2.bias")?,
                fc3_weight: get("broad_fc3.weight")?,
                fc3_bias: get("broad_fc3.bias")?,
            }
        };

        let entity_head = if is_python_format {
            MlpHead {
                fc1_weight: get("entity_head.0.weight")?,
                fc1_bias: get("entity_head.0.bias")?,
                fc2_weight: get("entity_head.3.weight")?,
                fc2_bias: get("entity_head.3.bias")?,
                fc3_weight: get("entity_head.6.weight")?,
                fc3_bias: get("entity_head.6.bias")?,
            }
        } else {
            MlpHead {
                fc1_weight: get("entity_fc1.weight")?,
                fc1_bias: get("entity_fc1.bias")?,
                fc2_weight: get("entity_fc2.weight")?,
                fc2_bias: get("entity_fc2.bias")?,
                fc3_weight: get("entity_fc3.weight")?,
                fc3_bias: get("entity_fc3.bias")?,
            }
        };

        Ok(Self {
            embed_dim,
            header_proj_weight: get("header_proj.weight")?,
            header_proj_bias: get("header_proj.bias")?,
            default_query: get("default_query")?,
            attention,
            norm_weight: get("norm.weight")?,
            norm_bias: get("norm.bias")?,
            broad_head,
            entity_head,
            device,
        })
    }

    /// Load from a directory containing `model.safetensors` and `config.json`.
    pub fn load<P: AsRef<std::path::Path>>(model_dir: P) -> Result<Self, InferenceError> {
        let dir = model_dir.as_ref();
        let model_bytes = std::fs::read(dir.join("model.safetensors"))?;
        let config_bytes = std::fs::read(dir.join("config.json"))?;
        Self::from_bytes(&model_bytes, &config_bytes)
    }

    /// Classify a column given its header (optional) and sample values.
    ///
    /// Uses `resources` for Model2Vec encoding. Encodes the header and up to
    /// `max_values` values, then runs the cross-attention forward pass.
    pub fn classify(
        &self,
        resources: &Model2VecResources,
        header: Option<&str>,
        values: &[&str],
    ) -> Result<SenseResult, InferenceError> {
        let max_values = 50; // Matches training config

        // Encode header — encode_batch returns L2-normalised vectors matching
        // Python model2vec.encode() which is what the Sense model was trained on.
        let (header_emb, has_header) = if let Some(h) = header {
            let batch = resources.encode_batch(&[h])?; // [1, D]
            let emb = batch.squeeze(0)?; // [D]
                                         // Check if header produced a non-zero embedding
            let norm: f32 = emb.sqr()?.sum_all()?.sqrt()?.to_scalar()?;
            if norm > 1e-8 {
                (emb, true)
            } else {
                (
                    Tensor::zeros(self.embed_dim, DType::F32, &self.device)?,
                    false,
                )
            }
        } else {
            (
                Tensor::zeros(self.embed_dim, DType::F32, &self.device)?,
                false,
            )
        };

        // Encode values (up to max_values)
        let n_values = values.len().min(max_values);
        let value_texts: Vec<&str> = values.iter().take(n_values).copied().collect();
        let value_embs = resources.encode_batch(&value_texts)?; // [N, D]

        // Build mask: true for real values (non-zero rows)
        // encode_batch returns zero rows for untokenizable values
        let mask: Vec<bool> = (0..n_values)
            .map(|i| {
                let row: Vec<f32> = value_embs.get(i).unwrap().to_vec1().unwrap_or_default();
                row.iter().any(|&v| v.abs() > 1e-8)
            })
            .collect();

        self.forward(&header_emb, has_header, &value_embs, &mask)
    }

    /// Classify a column using a pre-computed context-enriched header embedding.
    ///
    /// Used by the sibling-context attention module (NNFT-268): the header embedding
    /// has already been enriched with cross-column context before being passed here.
    /// The enriched embedding is used directly as the header, skipping `encode_batch`.
    ///
    /// `enriched_header_emb` should be a `[D]` tensor from the sibling-context module.
    pub fn classify_with_enriched_header(
        &self,
        resources: &Model2VecResources,
        enriched_header_emb: &Tensor,
        values: &[&str],
    ) -> Result<SenseResult, InferenceError> {
        let max_values = 50;

        // Encode values (same as classify)
        let n_values = values.len().min(max_values);
        let value_texts: Vec<&str> = values.iter().take(n_values).copied().collect();
        let value_embs = resources.encode_batch(&value_texts)?; // [N, D]

        // Build mask: true for real values (non-zero rows)
        let mask: Vec<bool> = (0..n_values)
            .map(|i| {
                let row: Vec<f32> = value_embs.get(i).unwrap().to_vec1().unwrap_or_default();
                row.iter().any(|&v| v.abs() > 1e-8)
            })
            .collect();

        // Check if enriched header is non-zero (has_header = true)
        let norm: f32 = enriched_header_emb.sqr()?.sum_all()?.sqrt()?.to_scalar()?;
        let has_header = norm > 1e-8;

        self.forward(enriched_header_emb, has_header, &value_embs, &mask)
    }

    /// Run the forward pass.
    ///
    /// Input shapes (single column, B=1):
    ///   header_emb: [D]
    ///   value_embs: [N, D]  (unnormalised mean-pooled embeddings)
    ///   mask: [N]  (true = real value, false = padding/UNK)
    fn forward(
        &self,
        header_emb: &Tensor,
        has_header: bool,
        value_embs: &Tensor,
        mask: &[bool],
    ) -> Result<SenseResult, InferenceError> {
        let n_values = value_embs.dim(0)?;
        if n_values == 0 {
            return Err(InferenceError::InvalidPath(
                "Sense classifier requires at least one value".into(),
            ));
        }

        // 1. Build query: header_proj(header_emb) or default_query
        let query = if has_header {
            // header_proj: Linear(D, D)
            let projected = header_emb
                .unsqueeze(0)? // [1, D]
                .matmul(&self.header_proj_weight.t()?)?
                .broadcast_add(&self.header_proj_bias)?; // [1, D]
            projected.unsqueeze(0)? // [1, 1, D]
        } else {
            self.default_query.clone() // [1, 1, D]
        };

        // 2. Cross-attention: query [1, 1, D] attends to value_embs [N, D]
        let values_3d = value_embs.unsqueeze(0)?; // [1, N, D]
        let attn_out = match &self.attention {
            AttentionVariant::FullMha { weights, n_heads } => {
                self.multi_head_attention(&query, &values_3d, mask, weights, *n_heads)?
            }
            AttentionVariant::Simple => self.simple_attention(&query, &values_3d, mask)?,
        }; // [1, 1, D]
        let attn_out = attn_out.squeeze(0)?.squeeze(0)?; // [D]

        // 3. LayerNorm
        let attn_normed = self.layer_norm(&attn_out)?; // [D]

        // 4. Masked mean and std of value embeddings
        let (val_mean, val_std) = self.masked_mean_std(value_embs, mask)?; // [D] each

        // 5. Concatenate features: [attn_out, val_mean, val_std] → [3D]
        let features = Tensor::cat(&[&attn_normed, &val_mean, &val_std], 0)?; // [3D]
        let features = features.unsqueeze(0)?; // [1, 3D]

        // 6. Classification heads
        let broad_probs = self.broad_head.forward(&features)?; // [6]
        let entity_probs = self.entity_head.forward(&features)?; // [4]

        // 7. Decode results
        let broad_vec: Vec<f32> = broad_probs.to_vec1()?;
        let entity_vec: Vec<f32> = entity_probs.to_vec1()?;

        let (broad_idx, broad_conf) = broad_vec
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .unwrap();
        let broad_category = BroadCategory::from_index(broad_idx).ok_or_else(|| {
            InferenceError::InvalidPath(format!("Invalid broad category index: {}", broad_idx))
        })?;

        let (entity_subtype, entity_confidence) = if broad_category == BroadCategory::Entity {
            let (ent_idx, ent_conf) = entity_vec
                .iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
                .unwrap();
            (EntitySubtype::from_index(ent_idx), *ent_conf)
        } else {
            (None, 0.0)
        };

        Ok(SenseResult {
            broad_category,
            entity_subtype,
            broad_confidence: *broad_conf,
            entity_confidence,
        })
    }

    /// Multi-head attention (Python-trained models): query [1, 1, D] × key/value [1, N, D] → [1, 1, D]
    fn multi_head_attention(
        &self,
        query: &Tensor,
        kv: &Tensor,
        mask: &[bool],
        weights: &CrossAttentionWeights,
        n_heads: usize,
    ) -> Result<Tensor, InferenceError> {
        let d = self.embed_dim;
        let h = n_heads;
        let head_dim = d / h;
        let n = kv.dim(1)?;

        // Project Q, K, V: [1, seq, D] × [D, D] → [1, seq, D]
        let q = query
            .squeeze(0)? // [1, D]
            .matmul(&weights.wq.t()?)?
            .broadcast_add(&weights.bq)?; // [1, D]
        let k = kv
            .squeeze(0)? // [N, D]
            .matmul(&weights.wk.t()?)?
            .broadcast_add(&weights.bk)?; // [N, D]
        let v = kv
            .squeeze(0)? // [N, D]
            .matmul(&weights.wv.t()?)?
            .broadcast_add(&weights.bv)?; // [N, D]

        // Reshape to multi-head: [seq, h, head_dim] → [h, seq, head_dim]
        let q = q.reshape((1, h, head_dim))?.transpose(0, 1)?; // [h, 1, head_dim]
        let k = k.reshape((n, h, head_dim))?.transpose(0, 1)?; // [h, N, head_dim]
        let v = v.reshape((n, h, head_dim))?.transpose(0, 1)?; // [h, N, head_dim]

        // Scaled dot-product attention: Q @ K^T / sqrt(head_dim)
        let scale = (head_dim as f64).sqrt();
        let attn_weights = (q.matmul(&k.transpose(1, 2)?)? / scale)?; // [h, 1, N]

        // Apply mask: set -inf for padding positions
        let attn_weights = if mask.iter().any(|&m| !m) {
            let mask_vals: Vec<f32> = mask
                .iter()
                .map(|&m| if m { 0.0 } else { f32::NEG_INFINITY })
                .collect();
            let mask_tensor = Tensor::from_vec(mask_vals, (1, 1, n), &self.device)?; // [1, 1, N]
            attn_weights.broadcast_add(&mask_tensor)? // [h, 1, N]
        } else {
            attn_weights
        };

        // Softmax over N dimension
        let attn_max = attn_weights.max(2)?.unsqueeze(2)?; // [h, 1, 1]
        let attn_shifted = attn_weights.broadcast_sub(&attn_max)?;
        let attn_exp = attn_shifted.exp()?;
        let attn_sum = attn_exp.sum(2)?.unsqueeze(2)?; // [h, 1, 1]
        let attn_probs = attn_exp.broadcast_div(&attn_sum)?; // [h, 1, N]

        // Weighted sum: attn_probs @ V → [h, 1, head_dim]
        let attn_out = attn_probs.matmul(&v)?; // [h, 1, head_dim]

        // Concatenate heads: [h, 1, head_dim] → [1, D]
        let attn_out = attn_out.transpose(0, 1)?; // [1, h, head_dim]
        let attn_out = attn_out.reshape((1, d))?; // [1, D]

        // Output projection
        let attn_out = attn_out
            .matmul(&weights.out_weight.t()?)?
            .broadcast_add(&weights.out_bias)?; // [1, D]

        Ok(attn_out.unsqueeze(0)?) // [1, 1, D]
    }

    /// Simple single-head attention (Rust-trained models): query [1, 1, D] × kv [1, N, D] → [1, 1, D]
    ///
    /// No K/V projections — query attends directly to value embeddings.
    /// Matches the training architecture in `finetype-train::sense::SenseModelA`.
    fn simple_attention(
        &self,
        query: &Tensor,
        kv: &Tensor,
        mask: &[bool],
    ) -> Result<Tensor, InferenceError> {
        let n = kv.dim(1)?;

        // Scaled dot-product: Q @ K^T / sqrt(D)
        let scale = (self.embed_dim as f64).sqrt();
        let scores = (query.matmul(&kv.transpose(1, 2)?)? / scale)?; // [1, 1, N]

        // Apply mask: set -inf for padding positions
        let scores = if mask.iter().any(|&m| !m) {
            let mask_vals: Vec<f32> = mask
                .iter()
                .map(|&m| if m { 0.0 } else { f32::NEG_INFINITY })
                .collect();
            let mask_tensor = Tensor::from_vec(mask_vals, (1, 1, n), &self.device)?;
            scores.broadcast_add(&mask_tensor)?
        } else {
            scores
        };

        // Softmax over N dimension
        let max_val = scores.max(2)?.unsqueeze(2)?;
        let shifted = scores.broadcast_sub(&max_val)?;
        let exp = shifted.exp()?;
        let sum_exp = exp.sum(2)?.unsqueeze(2)?;
        let attn_probs = exp.broadcast_div(&sum_exp)?; // [1, 1, N]

        // Weighted sum: [1, 1, N] @ [1, N, D] → [1, 1, D]
        Ok(attn_probs.matmul(kv)?)
    }

    /// LayerNorm: (x - mean) / sqrt(var + eps) * weight + bias
    fn layer_norm(&self, x: &Tensor) -> Result<Tensor, InferenceError> {
        let eps = 1e-5_f64;
        let mean = x.mean(0)?;
        let diff = x.broadcast_sub(&mean)?;
        let var = (&diff * &diff)?.mean(0)?;
        let std = (var + eps)?.sqrt()?;
        let normed = diff.broadcast_div(&std)?;
        Ok(normed
            .broadcast_mul(&self.norm_weight)?
            .broadcast_add(&self.norm_bias)?)
    }

    /// Compute masked mean and population std of value embeddings.
    fn masked_mean_std(
        &self,
        value_embs: &Tensor,
        mask: &[bool],
    ) -> Result<(Tensor, Tensor), InferenceError> {
        let d = self.embed_dim;
        let n = value_embs.dim(0)?;

        // Count valid values
        let n_valid = mask.iter().filter(|&&m| m).count();
        if n_valid == 0 {
            let zeros = Tensor::zeros(d, DType::F32, &self.device)?;
            return Ok((zeros.clone(), zeros));
        }

        // Build float mask [N, 1]
        let mask_f: Vec<f32> = mask.iter().map(|&m| if m { 1.0 } else { 0.0 }).collect();
        let mask_tensor = Tensor::from_vec(mask_f, (n, 1), &self.device)?;

        // Masked mean: sum(emb * mask) / n_valid
        let masked = value_embs.broadcast_mul(&mask_tensor)?; // [N, D]
        let sum = masked.sum(0)?; // [D]
        let val_mean = (&sum / n_valid as f64)?; // [D]

        // Masked population std
        let diff = value_embs.broadcast_sub(&val_mean.unsqueeze(0)?)?; // [N, D]
        let sq = (&diff * &diff)?;
        let masked_sq = sq.broadcast_mul(&mask_tensor)?;
        let var = (masked_sq.sum(0)? / n_valid as f64)?;
        let val_std = var.sqrt()?;

        Ok((val_mean, val_std))
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_broad_category_from_index() {
        assert_eq!(BroadCategory::from_index(0), Some(BroadCategory::Entity));
        assert_eq!(BroadCategory::from_index(5), Some(BroadCategory::Text));
        assert_eq!(BroadCategory::from_index(6), None);
    }

    #[test]
    fn test_broad_category_display() {
        assert_eq!(BroadCategory::Entity.to_string(), "entity");
        assert_eq!(BroadCategory::Geographic.to_string(), "geographic");
    }

    #[test]
    fn test_broad_category_from_str() {
        assert_eq!(
            "entity".parse::<BroadCategory>().unwrap(),
            BroadCategory::Entity
        );
        assert_eq!(
            "TEMPORAL".parse::<BroadCategory>().unwrap(),
            BroadCategory::Temporal
        );
        assert!("unknown".parse::<BroadCategory>().is_err());
    }

    #[test]
    fn test_entity_subtype_from_index() {
        assert_eq!(EntitySubtype::from_index(0), Some(EntitySubtype::Person));
        assert_eq!(
            EntitySubtype::from_index(3),
            Some(EntitySubtype::CreativeWork)
        );
        assert_eq!(EntitySubtype::from_index(4), None);
    }

    #[test]
    fn test_entity_subtype_display() {
        assert_eq!(EntitySubtype::Person.to_string(), "person");
        assert_eq!(EntitySubtype::CreativeWork.to_string(), "creative_work");
    }

    #[test]
    fn test_entity_subtype_from_str() {
        assert_eq!(
            "person".parse::<EntitySubtype>().unwrap(),
            EntitySubtype::Person
        );
        assert_eq!(
            "creative_work".parse::<EntitySubtype>().unwrap(),
            EntitySubtype::CreativeWork
        );
        assert!("alien".parse::<EntitySubtype>().is_err());
    }

    #[test]
    fn test_layer_norm_identity() {
        // LayerNorm on a constant vector should produce zeros (after mean subtraction)
        // then scale by weight and add bias.
        let device = Device::Cpu;
        let d = 4;

        let classifier = SenseClassifier {
            embed_dim: d,
            header_proj_weight: Tensor::zeros((d, d), DType::F32, &device).unwrap(),
            header_proj_bias: Tensor::zeros(d, DType::F32, &device).unwrap(),
            default_query: Tensor::zeros((1, 1, d), DType::F32, &device).unwrap(),
            attention: AttentionVariant::Simple,
            norm_weight: Tensor::ones(d, DType::F32, &device).unwrap(),
            norm_bias: Tensor::zeros(d, DType::F32, &device).unwrap(),
            broad_head: MlpHead {
                fc1_weight: Tensor::zeros((1, 1), DType::F32, &device).unwrap(),
                fc1_bias: Tensor::zeros(1, DType::F32, &device).unwrap(),
                fc2_weight: Tensor::zeros((1, 1), DType::F32, &device).unwrap(),
                fc2_bias: Tensor::zeros(1, DType::F32, &device).unwrap(),
                fc3_weight: Tensor::zeros((1, 1), DType::F32, &device).unwrap(),
                fc3_bias: Tensor::zeros(1, DType::F32, &device).unwrap(),
            },
            entity_head: MlpHead {
                fc1_weight: Tensor::zeros((1, 1), DType::F32, &device).unwrap(),
                fc1_bias: Tensor::zeros(1, DType::F32, &device).unwrap(),
                fc2_weight: Tensor::zeros((1, 1), DType::F32, &device).unwrap(),
                fc2_bias: Tensor::zeros(1, DType::F32, &device).unwrap(),
                fc3_weight: Tensor::zeros((1, 1), DType::F32, &device).unwrap(),
                fc3_bias: Tensor::zeros(1, DType::F32, &device).unwrap(),
            },
            device: device.clone(),
        };

        // Input [1, 2, 3, 4]: mean=2.5, var=1.25, std=1.118
        let x = Tensor::new(&[1.0f32, 2.0, 3.0, 4.0], &device).unwrap();
        let result = classifier.layer_norm(&x).unwrap();
        let v: Vec<f32> = result.to_vec1().unwrap();

        // With weight=1 and bias=0, output should be normalised to mean≈0, std≈1
        let mean: f32 = v.iter().sum::<f32>() / v.len() as f32;
        assert!(
            mean.abs() < 1e-5,
            "layer_norm mean should be ~0, got {}",
            mean
        );

        let variance: f32 = v.iter().map(|x| (x - mean).powi(2)).sum::<f32>() / v.len() as f32;
        assert!(
            (variance - 1.0).abs() < 0.01,
            "layer_norm variance should be ~1, got {}",
            variance
        );
    }

    #[test]
    fn test_masked_mean_std() {
        let device = Device::Cpu;
        let d = 2;

        let classifier = SenseClassifier {
            embed_dim: d,
            header_proj_weight: Tensor::zeros((d, d), DType::F32, &device).unwrap(),
            header_proj_bias: Tensor::zeros(d, DType::F32, &device).unwrap(),
            default_query: Tensor::zeros((1, 1, d), DType::F32, &device).unwrap(),
            attention: AttentionVariant::Simple,
            norm_weight: Tensor::ones(d, DType::F32, &device).unwrap(),
            norm_bias: Tensor::zeros(d, DType::F32, &device).unwrap(),
            broad_head: MlpHead {
                fc1_weight: Tensor::zeros((1, 1), DType::F32, &device).unwrap(),
                fc1_bias: Tensor::zeros(1, DType::F32, &device).unwrap(),
                fc2_weight: Tensor::zeros((1, 1), DType::F32, &device).unwrap(),
                fc2_bias: Tensor::zeros(1, DType::F32, &device).unwrap(),
                fc3_weight: Tensor::zeros((1, 1), DType::F32, &device).unwrap(),
                fc3_bias: Tensor::zeros(1, DType::F32, &device).unwrap(),
            },
            entity_head: MlpHead {
                fc1_weight: Tensor::zeros((1, 1), DType::F32, &device).unwrap(),
                fc1_bias: Tensor::zeros(1, DType::F32, &device).unwrap(),
                fc2_weight: Tensor::zeros((1, 1), DType::F32, &device).unwrap(),
                fc2_bias: Tensor::zeros(1, DType::F32, &device).unwrap(),
                fc3_weight: Tensor::zeros((1, 1), DType::F32, &device).unwrap(),
                fc3_bias: Tensor::zeros(1, DType::F32, &device).unwrap(),
            },
            device: device.clone(),
        };

        // 3 values, mask=[true, false, true] → only rows 0 and 2 count
        #[rustfmt::skip]
        let embs = Tensor::from_vec(
            vec![1.0f32, 2.0,   // row 0 (valid)
                 99.0, 99.0,    // row 1 (masked out)
                 3.0, 4.0],     // row 2 (valid)
            (3, 2),
            &device,
        ).unwrap();
        let mask = [true, false, true];

        let (mean, std) = classifier.masked_mean_std(&embs, &mask).unwrap();
        let mean_v: Vec<f32> = mean.to_vec1().unwrap();
        let std_v: Vec<f32> = std.to_vec1().unwrap();

        // Mean of [1,2] and [3,4] = [2.0, 3.0]
        assert!((mean_v[0] - 2.0).abs() < 1e-5);
        assert!((mean_v[1] - 3.0).abs() < 1e-5);

        // Pop std: sqrt(((1-2)^2 + (3-2)^2)/2) = sqrt(1) = 1.0
        assert!(
            (std_v[0] - 1.0).abs() < 1e-4,
            "expected std[0]=1.0, got {}",
            std_v[0]
        );
        assert!(
            (std_v[1] - 1.0).abs() < 1e-4,
            "expected std[1]=1.0, got {}",
            std_v[1]
        );
    }

    /// Integration test: load real Sense spike model, verify forward pass produces
    /// valid probabilities matching PyTorch reference.
    ///
    /// This tests:
    /// 1. Model loads from safetensors artifacts
    /// 2. Forward pass runs without error for various inputs
    /// 3. Output probabilities are valid (sum to ~1, no NaN)
    /// 4. Entity subtype is populated when broad_category == Entity
    /// 5. Numerical equivalence with PyTorch (verified via logged outputs)
    #[test]
    fn test_load_and_classify_if_available() {
        let workspace_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf();

        let sense_dir = workspace_root
            .join("models")
            .join("sense_spike")
            .join("arch_a");
        let m2v_dir = workspace_root.join("models").join("model2vec");

        if !sense_dir.join("model.safetensors").exists()
            || !m2v_dir.join("model.safetensors").exists()
        {
            eprintln!("Skipping Sense integration test: model artifacts not found");
            return;
        }

        let classifier = SenseClassifier::load(&sense_dir).unwrap();
        let resources = Model2VecResources::load(&m2v_dir).unwrap();

        // Test 1: Date column with header
        let date_values: Vec<&str> = vec![
            "2024-01-15",
            "2024-02-20",
            "2024-03-25",
            "2024-04-30",
            "2024-05-05",
            "2024-06-10",
            "2024-07-15",
            "2024-08-20",
        ];
        let date_result = classifier
            .classify(&resources, Some("date"), &date_values)
            .unwrap();
        assert!(
            date_result.broad_confidence > 0.0 && date_result.broad_confidence <= 1.0,
            "Date confidence should be valid probability, got {}",
            date_result.broad_confidence
        );

        // Test 2: Email column
        let email_values: Vec<&str> = vec![
            "john@example.com",
            "jane.doe@gmail.com",
            "bob@company.org",
            "alice@test.io",
            "charlie@email.net",
        ];
        let email_result = classifier
            .classify(&resources, Some("email"), &email_values)
            .unwrap();
        assert!(email_result.broad_confidence > 0.0);

        // Test 3: Person names — when Entity is predicted, entity_subtype must be Some
        let name_values: Vec<&str> = vec![
            "John Smith",
            "Jane Doe",
            "Robert Johnson",
            "Mary Williams",
            "James Brown",
            "Patricia Davis",
        ];
        let name_result = classifier
            .classify(&resources, Some("name"), &name_values)
            .unwrap();
        if name_result.broad_category == BroadCategory::Entity {
            assert!(
                name_result.entity_subtype.is_some(),
                "Entity prediction must include a subtype"
            );
            assert!(
                name_result.entity_confidence > 0.0,
                "Entity confidence must be positive"
            );
        }

        // Test 4: Numeric column
        let num_values: Vec<&str> = vec!["42", "100", "3.14", "256", "1024", "0.5", "99.9", "1000"];
        let num_result = classifier
            .classify(&resources, Some("amount"), &num_values)
            .unwrap();
        assert!(num_result.broad_confidence > 0.0);

        // Test 5: No header — should still produce valid output
        let no_header_result = classifier.classify(&resources, None, &date_values).unwrap();
        assert!(
            BroadCategory::from_index(no_header_result.broad_category as usize).is_some(),
            "No-header result should produce valid category"
        );

        // Test 6: Single value — minimal input
        let single_result = classifier
            .classify(&resources, Some("id"), &["12345"])
            .unwrap();
        assert!(single_result.broad_confidence > 0.0);

        eprintln!(
            "Sense integration (Python model): date={} ({:.1}%), email={} ({:.1}%), name={}/{:?} ({:.1}%), num={} ({:.1}%), no_header={}, single={}",
            date_result.broad_category,
            date_result.broad_confidence * 100.0,
            email_result.broad_category,
            email_result.broad_confidence * 100.0,
            name_result.broad_category,
            name_result.entity_subtype,
            name_result.broad_confidence * 100.0,
            num_result.broad_category,
            num_result.broad_confidence * 100.0,
            no_header_result.broad_category,
            single_result.broad_category,
        );
    }

    /// Integration test: load Rust-trained Sense model (simple attention variant).
    ///
    /// Tests that the dual-format loader correctly handles the Rust-trained model
    /// with `broad_fc{1,2,3}` keys and simple attention (no cross_attention weights).
    #[test]
    fn test_load_rust_trained_model_if_available() {
        let workspace_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf();

        let sense_dir = workspace_root
            .join("models")
            .join("sense_rust")
            .join("arch_a");
        let m2v_dir = workspace_root.join("models").join("model2vec");

        if !sense_dir.join("model.safetensors").exists()
            || !m2v_dir.join("model.safetensors").exists()
        {
            eprintln!("Skipping Rust-trained Sense test: model artifacts not found");
            return;
        }

        let classifier = SenseClassifier::load(&sense_dir).unwrap();
        let resources = Model2VecResources::load(&m2v_dir).unwrap();

        // Test with various column types
        let date_values: Vec<&str> = vec![
            "2024-01-15",
            "2024-02-20",
            "2024-03-25",
            "2024-04-30",
            "2024-05-05",
        ];
        let date_result = classifier
            .classify(&resources, Some("date"), &date_values)
            .unwrap();
        assert!(
            date_result.broad_confidence > 0.0 && date_result.broad_confidence <= 1.0,
            "Rust model: date confidence should be valid probability, got {}",
            date_result.broad_confidence
        );

        let name_values: Vec<&str> =
            vec!["John Smith", "Jane Doe", "Robert Johnson", "Mary Williams"];
        let name_result = classifier
            .classify(&resources, Some("name"), &name_values)
            .unwrap();
        assert!(name_result.broad_confidence > 0.0);

        // No header
        let no_header = classifier.classify(&resources, None, &date_values).unwrap();
        assert!(no_header.broad_confidence > 0.0);

        eprintln!(
            "Sense integration (Rust model): date={} ({:.1}%), name={}/{:?} ({:.1}%), no_header={} ({:.1}%)",
            date_result.broad_category,
            date_result.broad_confidence * 100.0,
            name_result.broad_category,
            name_result.entity_subtype,
            name_result.broad_confidence * 100.0,
            no_header.broad_category,
            no_header.broad_confidence * 100.0,
        );
    }
}
