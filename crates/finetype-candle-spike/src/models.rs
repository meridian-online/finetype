//! Core model architectures for Sense and Entity classification

use anyhow::Result;
use candle_core::{DType, Device, Tensor, D};
use candle_nn::{LayerNorm, Linear, Module};

// ── Configuration ────────────────────────────────────────

pub const EMBED_DIM: usize = 128; // Model2Vec embedding dimension (potion-base-4M)
pub const N_BROAD: usize = 6; // Broad categories (temporal, numeric, etc.)
pub const N_ENTITY: usize = 4; // Entity subtypes (person, place, org, creative_work)
pub const N_HEADS: usize = 4; // Multi-head attention heads
pub const HIDDEN_DIM: usize = 256; // Hidden layer dimension

// ── Sense Model A: Cross-Attention over Model2Vec ────────────

/// Sense Architecture A: Lightweight attention over Model2Vec embeddings
///
/// Multi-task architecture for semantic routing:
/// - Broad category classification (6 classes): temporal, numeric, geographic, entity, format, text
/// - Entity subtype classification (4 classes): person, place, organization, creative_work
///
/// Architecture:
/// 1. Cross-attention: header embedding (query) attends to value embeddings (key/value)
/// 2. Feature aggregation: attention output + mean/std of value embeddings
/// 3. Dual classification heads for broad category and entity subtype
#[derive(Debug)]
pub struct SenseModelA {
    device: Device,

    // Query projection: header embedding → attention query space
    header_proj: Linear,

    // Learnable default query (used when header is missing)
    default_query: Tensor,

    // Layer normalization after attention
    norm: LayerNorm,

    // Broad category classifier head
    broad_fc1: Linear,
    broad_fc2: Linear,
    broad_fc3: Linear,

    // Entity subtype classifier head
    entity_fc1: Linear,
    entity_fc2: Linear,
    entity_fc3: Linear,
}

impl SenseModelA {
    /// Create a new Sense model with random initialization
    pub fn new() -> Result<Self> {
        let device = Device::Cpu; // Use CPU for spike; can enable CUDA later

        // Feature dimension after attention aggregation: attention_out + mean + std
        let feature_dim = 3 * EMBED_DIM;

        // Initialize linear layers
        let header_proj = Linear::new(EMBED_DIM, EMBED_DIM, &device)?;
        let norm = LayerNorm::new(EMBED_DIM, 1e-5, &device)?;

        // Broad category head: 384 → 256 → 128 → 6
        let broad_fc1 = Linear::new(feature_dim, HIDDEN_DIM, &device)?;
        let broad_fc2 = Linear::new(HIDDEN_DIM, HIDDEN_DIM / 2, &device)?;
        let broad_fc3 = Linear::new(HIDDEN_DIM / 2, N_BROAD, &device)?;

        // Entity subtype head: 384 → 256 → 128 → 4
        let entity_fc1 = Linear::new(feature_dim, HIDDEN_DIM, &device)?;
        let entity_fc2 = Linear::new(HIDDEN_DIM, HIDDEN_DIM / 2, &device)?;
        let entity_fc3 = Linear::new(HIDDEN_DIM / 2, N_ENTITY, &device)?;

        // Learnable default query: [1, 1, EMBED_DIM]
        let default_query = Tensor::randn((1, 1, EMBED_DIM), DType::F32, &device)? * 0.02;

        Ok(SenseModelA {
            device,
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

    /// Forward pass through Sense model
    ///
    /// Args:
    /// - value_embeds: [batch_size, n_values, EMBED_DIM] - Model2Vec embeddings
    /// - mask: [batch_size, n_values] - True for real values, False for padding
    /// - header_embed: [batch_size, EMBED_DIM] - Column header embedding
    /// - has_header: [batch_size] - 1.0 if header exists, 0.0 otherwise
    ///
    /// Returns:
    /// - broad_logits: [batch_size, N_BROAD]
    /// - entity_logits: [batch_size, N_ENTITY]
    pub fn forward(
        &self,
        value_embeds: &Tensor, // [B, N, D]
        mask: &Tensor,         // [B, N]
        header_embed: &Tensor, // [B, D]
        has_header: &Tensor,   // [B]
    ) -> Result<(Tensor, Tensor)> {
        let batch_size = value_embeds.dim(0)?;

        // Build query: use header when available, default query otherwise
        // Header projection: [B, D] → [B, D]
        let header_proj = self.header_proj.forward(header_embed)?;

        // Convert to [B, 1, D] for attention
        let query = header_proj.unsqueeze(1)?; // [B, 1, D]

        // Apply header availability mask: when has_header = 0, use default query
        let has_header_expanded = has_header.unsqueeze(1)?.unsqueeze(2)?; // [B, 1, 1]
        let query = query * has_header_expanded?;

        // Add default query component (scaled by presence of header)
        let default_scaled = self
            .default_query
            .broadcast_as((batch_size, 1, EMBED_DIM))?;
        let query = (query + &default_scaled)?;

        // Cross-attention: query [B, 1, D], values [B, N, D]
        // Using attention pattern: softmax((query @ values.T) / sqrt(D)) @ values

        // Simplified attention (not multi-head, but captures the pattern):
        // Compute attention weights: [B, 1, N]
        let scale = (EMBED_DIM as f32).sqrt();
        let scores = query.matmul(&value_embeds.transpose(1, 2)?)?; // [B, 1, N]
        let scores = (scores / scale)?;

        // Apply mask (set invalid positions to -inf before softmax)
        // This is simplified for spike; full implementation would need masked softmax
        let attention_weights = candle_nn::ops::softmax(&scores, D::Minus1)?; // [B, 1, N]

        // Apply attention to values: [B, 1, N] @ [B, N, D] → [B, 1, D]
        let attention_out = attention_weights.matmul(value_embeds)?; // [B, 1, D]
        let attention_out = attention_out.squeeze(1)?; // [B, D]

        // Apply layer norm
        let attention_out = self.norm.forward(&attention_out)?;

        // Compute statistics of value embeddings
        // Mean: [B, D]
        let value_mean = value_embeds.mean(1)?;

        // Std: simplified as using variance
        let value_centered = (value_embeds - value_mean.unsqueeze(1)?)?;
        let value_var = (value_centered.sqr()? / (value_embeds.dim(1)? as f32))?;
        let value_std = value_var.sqrt()?;

        // Concatenate features: [attention_out, mean, std] → [B, 3*D]
        let features = Tensor::cat(&[&attention_out, &value_mean, &value_std], 1)?;

        // Broad category head
        let broad_hidden = self.broad_fc1.forward(&features)?;
        let broad_hidden = broad_hidden.relu()?;
        let broad_hidden = self.broad_fc2.forward(&broad_hidden)?;
        let broad_hidden = broad_hidden.relu()?;
        let broad_logits = self.broad_fc3.forward(&broad_hidden)?;

        // Entity subtype head
        let entity_hidden = self.entity_fc1.forward(&features)?;
        let entity_hidden = entity_hidden.relu()?;
        let entity_hidden = self.entity_fc2.forward(&entity_hidden)?;
        let entity_hidden = entity_hidden.relu()?;
        let entity_logits = self.entity_fc3.forward(&entity_hidden)?;

        Ok((broad_logits, entity_logits))
    }
}

// ── Entity Classifier: Deep Sets MLP ─────────────────────

/// Entity Classifier: Deep Sets architecture for demotion gating
///
/// Input: Column-level statistics (44 features) + embeddings (mean/std)
/// Output: Entity class probabilities (4 classes: person, place, org, creative_work)
#[derive(Debug)]
pub struct EntityClassifier {
    device: Device,
    fc1: Linear,
    fc2: Linear,
    fc3: Linear,
    fc4: Linear,
}

impl EntityClassifier {
    /// Create a new entity classifier
    pub fn new() -> Result<Self> {
        let device = Device::Cpu;

        // Input: 44 statistical features + 2*128 (mean/std) = 300 dims
        let input_dim = 44 + 2 * EMBED_DIM;

        // MLP: 300 → 256 → 256 → 128 → 4
        let fc1 = Linear::new(input_dim, 256, &device)?;
        let fc2 = Linear::new(256, 256, &device)?;
        let fc3 = Linear::new(256, 128, &device)?;
        let fc4 = Linear::new(128, N_ENTITY, &device)?;

        Ok(EntityClassifier {
            device,
            fc1,
            fc2,
            fc3,
            fc4,
        })
    }

    /// Forward pass through entity classifier
    ///
    /// Args:
    /// - features: [batch_size, 300] - concatenated statistical features + embeddings
    ///
    /// Returns:
    /// - logits: [batch_size, N_ENTITY]
    pub fn forward(&self, features: &Tensor) -> Result<Tensor> {
        let x = self.fc1.forward(features)?;
        let x = x.relu()?;

        let x = self.fc2.forward(&x)?;
        let x = x.relu()?;

        let x = self.fc3.forward(&x)?;
        let x = x.relu()?;

        self.fc4.forward(&x)
    }
}
