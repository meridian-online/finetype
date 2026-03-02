//! Sense model Architecture A: Cross-attention over Model2Vec embeddings.
//!
//! Multi-task: broad category (6 classes) + entity subtype (4 classes).
//! Cross-attention: header embedding queries value embeddings.
//! Validated in Candle spike (NNFT-182): all 10 tests pass.

use anyhow::Result;
use candle_core::{DType, Device, Tensor, D};
use candle_nn::{layer_norm, linear, LayerNorm, Linear, Module, VarBuilder, VarMap};

// ── Constants ────────────────────────────────────────────────────────────────

pub const EMBED_DIM: usize = 128;
pub const N_BROAD: usize = 6;
pub const N_ENTITY: usize = 4;
pub const HIDDEN_DIM: usize = 256;
pub const MAX_VALUES: usize = 50;

/// Broad category labels (order matches training indices).
pub const BROAD_CATEGORIES: [&str; N_BROAD] = [
    "entity",
    "format",
    "geographic",
    "numeric",
    "temporal",
    "text",
];

/// Entity subtype labels (order matches training indices).
pub const ENTITY_SUBTYPES: [&str; N_ENTITY] = ["person", "place", "organization", "creative_work"];

// ── Sense Model A ────────────────────────────────────────────────────────────

/// Sense Architecture A: Lightweight cross-attention over Model2Vec embeddings.
///
/// Input: pre-computed Model2Vec embeddings for column values + optional header.
/// Output: dual-head logits for broad category (6) and entity subtype (4).
///
/// Architecture:
/// 1. Project header → query (or use learnable default if no header)
/// 2. Cross-attention: query attends to value embeddings
/// 3. Concat [attn_out, value_mean, value_std] → 3×128 = 384 features
/// 4. Two independent 3-layer MLPs: broad (384→256→128→6) and entity (384→256→128→4)
pub struct SenseModelA {
    header_proj: Linear,
    default_query: Tensor,
    norm: LayerNorm,
    broad_fc1: Linear,
    broad_fc2: Linear,
    broad_fc3: Linear,
    entity_fc1: Linear,
    entity_fc2: Linear,
    entity_fc3: Linear,
}

impl SenseModelA {
    /// Create a new model, registering all parameters in the VarMap.
    pub fn new(varmap: &VarMap, device: &Device) -> Result<Self> {
        let vb = VarBuilder::from_varmap(varmap, DType::F32, device);
        let feature_dim = 3 * EMBED_DIM;

        let header_proj = linear(EMBED_DIM, EMBED_DIM, vb.pp("header_proj"))?;
        let norm = layer_norm(
            EMBED_DIM,
            candle_nn::LayerNormConfig::default(),
            vb.pp("norm"),
        )?;

        let broad_fc1 = linear(feature_dim, HIDDEN_DIM, vb.pp("broad_fc1"))?;
        let broad_fc2 = linear(HIDDEN_DIM, HIDDEN_DIM / 2, vb.pp("broad_fc2"))?;
        let broad_fc3 = linear(HIDDEN_DIM / 2, N_BROAD, vb.pp("broad_fc3"))?;

        let entity_fc1 = linear(feature_dim, HIDDEN_DIM, vb.pp("entity_fc1"))?;
        let entity_fc2 = linear(HIDDEN_DIM, HIDDEN_DIM / 2, vb.pp("entity_fc2"))?;
        let entity_fc3 = linear(HIDDEN_DIM / 2, N_ENTITY, vb.pp("entity_fc3"))?;

        let default_query = varmap.get(
            (1, 1, EMBED_DIM),
            "default_query",
            candle_nn::Init::Randn {
                mean: 0.0,
                stdev: 0.02,
            },
            DType::F32,
            device,
        )?;

        Ok(Self {
            header_proj,
            default_query,
            norm,
            broad_fc1,
            broad_fc2,
            broad_fc3,
            entity_fc1,
            entity_fc2,
            entity_fc3,
        })
    }

    /// Forward pass.
    ///
    /// - `value_embeds`: [B, N, 128] — Model2Vec embeddings of column values
    /// - `mask`: [B, N] — 1.0 for real values, 0.0 for padding (currently unused)
    /// - `header_embed`: [B, 128] — Model2Vec embedding of column header
    /// - `has_header`: [B] — 1.0 if header present, 0.0 otherwise
    ///
    /// Returns `(broad_logits [B, 6], entity_logits [B, 4])`.
    pub fn forward(
        &self,
        value_embeds: &Tensor,
        _mask: &Tensor,
        header_embed: &Tensor,
        has_header: &Tensor,
    ) -> Result<(Tensor, Tensor)> {
        let batch_size = value_embeds.dim(0)?;

        // Query: project header or use learnable default
        let header_proj = self.header_proj.forward(header_embed)?;
        let query = header_proj.unsqueeze(1)?; // [B, 1, D]

        let has_h = has_header.unsqueeze(1)?.unsqueeze(2)?; // [B, 1, 1]
        let default_q = self
            .default_query
            .broadcast_as((batch_size, 1, EMBED_DIM))?;
        let one_minus_h = has_h.affine(-1.0, 1.0)?;
        let query = (query.broadcast_mul(&has_h)? + default_q.broadcast_mul(&one_minus_h)?)?;

        // Cross-attention: softmax(Q @ K^T / √d) @ V
        let scale = (EMBED_DIM as f64).sqrt();
        let scores = query.matmul(&value_embeds.transpose(1, 2)?)?;
        let scores = (scores / scale)?;
        let attn_weights = candle_nn::ops::softmax(&scores, D::Minus1)?;
        let attn_out = attn_weights.matmul(value_embeds)?.squeeze(1)?;
        let attn_out = self.norm.forward(&attn_out)?;

        // Statistics
        let value_mean = value_embeds.mean(1)?;
        let centered = value_embeds.broadcast_sub(&value_mean.unsqueeze(1)?)?;
        let value_std = centered.sqr()?.mean(1)?.sqrt()?;

        // Concat → [B, 384]
        let features = Tensor::cat(&[&attn_out, &value_mean, &value_std], 1)?;

        // Broad category head
        let b = self.broad_fc1.forward(&features)?.relu()?;
        let b = self.broad_fc2.forward(&b)?.relu()?;
        let broad_logits = self.broad_fc3.forward(&b)?;

        // Entity subtype head
        let e = self.entity_fc1.forward(&features)?.relu()?;
        let e = self.entity_fc2.forward(&e)?.relu()?;
        let entity_logits = self.entity_fc3.forward(&e)?;

        Ok((broad_logits, entity_logits))
    }
}
