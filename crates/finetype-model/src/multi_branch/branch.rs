//! Branch weight blocks and classification head (inference only).

use super::*;

// ═══════════════════════════════════════════════════════════════════════════════
// Branch weights (2-layer MLP, inference only — no dropout)
// ═══════════════════════════════════════════════════════════════════════════════

pub(crate) struct BranchWeights {
    input_norm: Option<candle_nn::LayerNorm>,
    linear1: Linear,
    linear2: Linear,
    activation: Activation,
}

impl BranchWeights {
    pub(crate) fn new(
        input_dim: usize,
        hidden: [usize; 2],
        activation: &Activation,
        vb: VarBuilder,
    ) -> candle_core::Result<Self> {
        Self::new_inner(input_dim, hidden, false, activation, vb)
    }

    pub(crate) fn new_with_input_norm(
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

    pub(crate) fn forward(&self, x: &Tensor) -> candle_core::Result<Tensor> {
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
pub(crate) enum ClassificationHead {
    /// Flat: Dense(hidden_dim → n_classes) producing logits.
    Flat(Linear),
    /// Hierarchical: 3-level tree softmax (domain → category → type)
    /// producing product probabilities.
    Hierarchical(Box<HierarchicalHead>),
}
