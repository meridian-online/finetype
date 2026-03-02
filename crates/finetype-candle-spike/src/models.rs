//! Core model architectures for Sense and Entity classification
//!
//! Uses Candle 0.8 VarBuilder API for idiomatic parameter management.

use anyhow::Result;
use candle_core::{DType, Device, Tensor, D};
use candle_nn::{layer_norm, linear, LayerNorm, Linear, Module, VarBuilder, VarMap};

// ── Configuration ────────────────────────────────────────

pub const EMBED_DIM: usize = 128;
pub const N_BROAD: usize = 6;
pub const N_ENTITY: usize = 4;
pub const HIDDEN_DIM: usize = 256;

// ── Sense Model A: Cross-Attention over Model2Vec ────────────

/// Sense Architecture A: Lightweight attention over Model2Vec embeddings.
///
/// Multi-task: broad category (6) + entity subtype (4).
/// Cross-attention: header embedding queries value embeddings.
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
    /// Create a new Sense model with VarMap for parameter tracking
    pub fn new(varmap: &VarMap, device: &Device) -> Result<Self> {
        let vb = VarBuilder::from_varmap(varmap, DType::F32, device);
        let feature_dim = 3 * EMBED_DIM;

        let header_proj = linear(EMBED_DIM, EMBED_DIM, vb.pp("header_proj"))?;
        let norm_config = candle_nn::LayerNormConfig::default();
        let norm = layer_norm(EMBED_DIM, norm_config, vb.pp("norm"))?;

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

        Ok(SenseModelA {
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

    /// Forward pass
    ///
    /// - value_embeds: [B, N, D]
    /// - mask: [B, N] (1.0 for real, 0.0 for padding)
    /// - header_embed: [B, D]
    /// - has_header: [B]
    pub fn forward(
        &self,
        value_embeds: &Tensor,
        _mask: &Tensor,
        header_embed: &Tensor,
        has_header: &Tensor,
    ) -> Result<(Tensor, Tensor)> {
        let batch_size = value_embeds.dim(0)?;

        // Query: project header embedding or use default
        let header_proj = self.header_proj.forward(header_embed)?;
        let query = header_proj.unsqueeze(1)?; // [B, 1, D]

        // Blend: has_header * projected_header + (1 - has_header) * default_query
        let has_h = has_header.unsqueeze(1)?.unsqueeze(2)?; // [B, 1, 1]
        let default_q = self
            .default_query
            .broadcast_as((batch_size, 1, EMBED_DIM))?;
        let one_minus_h = has_h.affine(-1.0, 1.0)?; // 1 - has_h
        let query = (query.broadcast_mul(&has_h)? + default_q.broadcast_mul(&one_minus_h)?)?;

        // Cross-attention: softmax(Q @ K^T / sqrt(d)) @ V
        let scale = (EMBED_DIM as f64).sqrt();
        let scores = query.matmul(&value_embeds.transpose(1, 2)?)?; // [B, 1, N]
        let scores = (scores / scale)?;
        let attn_weights = candle_nn::ops::softmax(&scores, D::Minus1)?;
        let attn_out = attn_weights.matmul(value_embeds)?.squeeze(1)?; // [B, D]
        let attn_out = self.norm.forward(&attn_out)?;

        // Statistics: mean and std of value embeddings
        let value_mean = value_embeds.mean(1)?; // [B, D]
        let centered = value_embeds.broadcast_sub(&value_mean.unsqueeze(1)?)?;
        let _n_vals = value_embeds.dim(1)? as f64;
        let value_std = centered.sqr()?.mean(1)?.sqrt()?; // [B, D] — std per dim

        // Concatenate features: [attn_out, mean, std] → [B, 3*D]
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

// ── Entity Classifier: Deep Sets MLP ─────────────────────

/// Entity Classifier: Deep Sets MLP for demotion gating.
///
/// Input: 44 statistical features + mean/std of embeddings (300 dims).
/// Output: 4-class entity logits (person, place, org, creative_work).
pub struct EntityClassifier {
    fc1: Linear,
    fc2: Linear,
    fc3: Linear,
    fc4: Linear,
}

impl EntityClassifier {
    pub fn new(varmap: &VarMap, device: &Device) -> Result<Self> {
        let vb = VarBuilder::from_varmap(varmap, DType::F32, device);
        let input_dim = 44 + 2 * EMBED_DIM; // 300

        let fc1 = linear(input_dim, 256, vb.pp("ec_fc1"))?;
        let fc2 = linear(256, 256, vb.pp("ec_fc2"))?;
        let fc3 = linear(256, 128, vb.pp("ec_fc3"))?;
        let fc4 = linear(128, N_ENTITY, vb.pp("ec_fc4"))?;

        Ok(EntityClassifier { fc1, fc2, fc3, fc4 })
    }

    pub fn forward(&self, features: &Tensor) -> Result<Tensor> {
        let x = self.fc1.forward(features)?.relu()?;
        let x = self.fc2.forward(&x)?.relu()?;
        let x = self.fc3.forward(&x)?.relu()?;
        Ok(self.fc4.forward(&x)?)
    }
}
