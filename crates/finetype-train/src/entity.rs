//! Entity classifier training: Deep Sets MLP for demotion gating.
//!
//! Input: 300-dim features (128 emb_mean + 128 emb_std + 44 statistical).
//! Output: 4-class entity logits (person, place, organization, creative_work).
//!
//! Feature computation reuses `finetype_model::entity::EntityClassifier::compute_stat_features`
//! logic — the same 44 statistical features used at inference time.

use anyhow::Result;
use candle_core::{DType, Device, Tensor};
use candle_nn::{linear, Linear, Module, VarBuilder, VarMap};

use crate::sense::{EMBED_DIM, N_ENTITY};

// ── Constants ────────────────────────────────────────────────────────────────

/// Number of statistical features (must match finetype-model::entity).
pub const N_STAT_FEATURES: usize = 44;

/// Total input dimension: emb_mean (128) + emb_std (128) + stats (44).
pub const INPUT_DIM: usize = 2 * EMBED_DIM + N_STAT_FEATURES;

/// Default hidden dimension for MLP layers.
pub const HIDDEN_DIM: usize = 256;

/// Dropout rate during training.
pub const DROPOUT_RATE: f64 = 0.2;

// ── Entity Classifier (Training) ─────────────────────────────────────────────

/// Entity classifier MLP for training.
///
/// Architecture (matching Python `train_entity_classifier.py`):
/// ```text
/// Input: [B, 300]
///   → Linear(300→256) → ReLU → Dropout(0.2)
///   → Linear(256→256) → ReLU → Dropout(0.2)
///   → Linear(256→128) → ReLU → Dropout(0.2)
///   → Linear(128→4)
/// Output: [B, 4] logits
/// ```
///
/// Note: Dropout is applied during training only. The production
/// `finetype_model::entity::EntityClassifier` has no dropout.
pub struct EntityClassifierTrainable {
    fc1: Linear,
    fc2: Linear,
    fc3: Linear,
    fc4: Linear,
    dropout_rate: f64,
    training: bool,
}

impl EntityClassifierTrainable {
    /// Create a new trainable entity classifier.
    pub fn new(varmap: &VarMap, device: &Device) -> Result<Self> {
        let vb = VarBuilder::from_varmap(varmap, DType::F32, device);

        let fc1 = linear(INPUT_DIM, HIDDEN_DIM, vb.pp("ec_fc1"))?;
        let fc2 = linear(HIDDEN_DIM, HIDDEN_DIM, vb.pp("ec_fc2"))?;
        let fc3 = linear(HIDDEN_DIM, HIDDEN_DIM / 2, vb.pp("ec_fc3"))?;
        let fc4 = linear(HIDDEN_DIM / 2, N_ENTITY, vb.pp("ec_fc4"))?;

        Ok(Self {
            fc1,
            fc2,
            fc3,
            fc4,
            dropout_rate: DROPOUT_RATE,
            training: true,
        })
    }

    /// Set training mode (enables dropout).
    pub fn set_training(&mut self, training: bool) {
        self.training = training;
    }

    /// Forward pass with optional dropout.
    pub fn forward(&self, features: &Tensor) -> Result<Tensor> {
        let x = self.fc1.forward(features)?.relu()?;
        let x = self.maybe_dropout(&x)?;

        let x = self.fc2.forward(&x)?.relu()?;
        let x = self.maybe_dropout(&x)?;

        let x = self.fc3.forward(&x)?.relu()?;
        let x = self.maybe_dropout(&x)?;

        Ok(self.fc4.forward(&x)?)
    }

    /// Apply dropout during training (Candle doesn't have a built-in dropout module).
    fn maybe_dropout(&self, tensor: &Tensor) -> Result<Tensor> {
        if !self.training || self.dropout_rate == 0.0 {
            return Ok(tensor.clone());
        }
        Ok(candle_nn::ops::dropout(tensor, self.dropout_rate as f32)?)
    }
}
