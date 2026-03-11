//! Trainable sibling-context attention module.
//!
//! Mirrors the inference `SiblingContextAttention` from `finetype-model` but uses
//! `candle_nn::VarMap` for gradient-tracked parameters. Key names exactly match
//! the inference model's `from_tensors` convention, so `varmap.save()` produces
//! artifacts directly loadable by `SiblingContextAttention::load()`.
//!
//! Architecture: 2-layer pre-norm transformer self-attention over [N, 128] embeddings.
//! Parameters: 396,800 (1.51 MB as f32) with default config.

use anyhow::Result;
use candle_core::{DType, Device, Tensor};
use candle_nn::{Init, VarMap};
use finetype_model::sibling_context::SiblingContextConfig;

/// Trainable sibling-context attention module.
///
/// All weights are variable-backed tensors from VarMap, enabling gradient computation.
/// The forward pass is identical to the inference `SiblingContextAttention`.
pub struct SiblingContextTrainable {
    blocks: Vec<TrainableTransformerBlock>,
    final_norm: TrainableLayerNorm,
    embed_dim: usize,
}

struct TrainableTransformerBlock {
    norm1: TrainableLayerNorm,
    attn: TrainableMultiHeadAttention,
    norm2: TrainableLayerNorm,
    ffn: TrainableFFN,
}

struct TrainableLayerNorm {
    weight: Tensor, // [D]
    bias: Tensor,   // [D]
}

struct TrainableMultiHeadAttention {
    wq: Tensor,         // [D, D]
    bq: Tensor,         // [D]
    wk: Tensor,         // [D, D]
    bk: Tensor,         // [D]
    wv: Tensor,         // [D, D]
    bv: Tensor,         // [D]
    out_weight: Tensor, // [D, D]
    out_bias: Tensor,   // [D]
    n_heads: usize,
    head_dim: usize,
}

struct TrainableFFN {
    w1: Tensor, // [4D, D]
    b1: Tensor, // [4D]
    w2: Tensor, // [D, 4D]
    b2: Tensor, // [D]
}

impl SiblingContextTrainable {
    /// Create a new trainable model, registering all parameters in the VarMap.
    ///
    /// Key names match the inference model exactly (e.g., `blocks.0.attn.wq`),
    /// so `varmap.save()` produces artifacts loadable by `SiblingContextAttention::load()`.
    pub fn new(varmap: &VarMap, config: &SiblingContextConfig, device: &Device) -> Result<Self> {
        let d = config.embed_dim;
        let ff = d * 4;
        let n_heads = config.n_heads;
        let head_dim = d / n_heads;
        let scale = (1.0f64 / d as f64).sqrt();

        let mut blocks = Vec::with_capacity(config.n_layers);

        for i in 0..config.n_layers {
            let prefix = format!("blocks.{}", i);

            let norm1 = TrainableLayerNorm {
                weight: varmap.get(
                    (d,),
                    &format!("{prefix}.norm1.weight"),
                    Init::Const(1.0),
                    DType::F32,
                    device,
                )?,
                bias: varmap.get(
                    (d,),
                    &format!("{prefix}.norm1.bias"),
                    Init::Const(0.0),
                    DType::F32,
                    device,
                )?,
            };

            let attn = TrainableMultiHeadAttention {
                wq: varmap.get(
                    (d, d),
                    &format!("{prefix}.attn.wq"),
                    Init::Randn {
                        mean: 0.0,
                        stdev: scale,
                    },
                    DType::F32,
                    device,
                )?,
                bq: varmap.get(
                    (d,),
                    &format!("{prefix}.attn.bq"),
                    Init::Const(0.0),
                    DType::F32,
                    device,
                )?,
                wk: varmap.get(
                    (d, d),
                    &format!("{prefix}.attn.wk"),
                    Init::Randn {
                        mean: 0.0,
                        stdev: scale,
                    },
                    DType::F32,
                    device,
                )?,
                bk: varmap.get(
                    (d,),
                    &format!("{prefix}.attn.bk"),
                    Init::Const(0.0),
                    DType::F32,
                    device,
                )?,
                wv: varmap.get(
                    (d, d),
                    &format!("{prefix}.attn.wv"),
                    Init::Randn {
                        mean: 0.0,
                        stdev: scale,
                    },
                    DType::F32,
                    device,
                )?,
                bv: varmap.get(
                    (d,),
                    &format!("{prefix}.attn.bv"),
                    Init::Const(0.0),
                    DType::F32,
                    device,
                )?,
                out_weight: varmap.get(
                    (d, d),
                    &format!("{prefix}.attn.out_weight"),
                    Init::Randn {
                        mean: 0.0,
                        stdev: scale,
                    },
                    DType::F32,
                    device,
                )?,
                out_bias: varmap.get(
                    (d,),
                    &format!("{prefix}.attn.out_bias"),
                    Init::Const(0.0),
                    DType::F32,
                    device,
                )?,
                n_heads,
                head_dim,
            };

            let norm2 = TrainableLayerNorm {
                weight: varmap.get(
                    (d,),
                    &format!("{prefix}.norm2.weight"),
                    Init::Const(1.0),
                    DType::F32,
                    device,
                )?,
                bias: varmap.get(
                    (d,),
                    &format!("{prefix}.norm2.bias"),
                    Init::Const(0.0),
                    DType::F32,
                    device,
                )?,
            };

            let ffn = TrainableFFN {
                w1: varmap.get(
                    (ff, d),
                    &format!("{prefix}.ffn.w1"),
                    Init::Randn {
                        mean: 0.0,
                        stdev: scale,
                    },
                    DType::F32,
                    device,
                )?,
                b1: varmap.get(
                    (ff,),
                    &format!("{prefix}.ffn.b1"),
                    Init::Const(0.0),
                    DType::F32,
                    device,
                )?,
                w2: varmap.get(
                    (d, ff),
                    &format!("{prefix}.ffn.w2"),
                    Init::Randn {
                        mean: 0.0,
                        stdev: scale,
                    },
                    DType::F32,
                    device,
                )?,
                b2: varmap.get(
                    (d,),
                    &format!("{prefix}.ffn.b2"),
                    Init::Const(0.0),
                    DType::F32,
                    device,
                )?,
            };

            blocks.push(TrainableTransformerBlock {
                norm1,
                attn,
                norm2,
                ffn,
            });
        }

        let final_norm = TrainableLayerNorm {
            weight: varmap.get(
                (d,),
                "final_norm.weight",
                Init::Const(1.0),
                DType::F32,
                device,
            )?,
            bias: varmap.get(
                (d,),
                "final_norm.bias",
                Init::Const(0.0),
                DType::F32,
                device,
            )?,
        };

        Ok(Self {
            blocks,
            final_norm,
            embed_dim: d,
        })
    }

    /// Forward pass: [N, D] → [N, D].
    ///
    /// Identical to the inference model's forward pass. Tensors are variable-backed
    /// from VarMap, so gradients flow through for training.
    pub fn forward(&self, x: &Tensor) -> Result<Tensor> {
        let mut out = x.clone();
        for block in &self.blocks {
            out = block.forward(&out)?;
        }
        self.final_norm.forward(&out)
    }

    /// Count total trainable parameters.
    pub fn param_count(&self) -> usize {
        let d = self.embed_dim;
        let ff = d * 4;
        let per_block = 4 * (d * d + d) +  // MHA: Q, K, V, Out (weight + bias)
            2 * (d + d) +      // LayerNorm x2
            (ff * d + ff) + (d * ff + d); // FFN
        let final_norm = d + d;
        per_block * self.blocks.len() + final_norm
    }
}

impl TrainableTransformerBlock {
    fn forward(&self, x: &Tensor) -> Result<Tensor> {
        let normed = self.norm1.forward(x)?;
        let attn_out = self.attn.forward(&normed)?;
        let x = (x + &attn_out)?;

        let normed = self.norm2.forward(&x)?;
        let ffn_out = self.ffn.forward(&normed)?;
        Ok((&x + &ffn_out)?)
    }
}

impl TrainableLayerNorm {
    fn forward(&self, x: &Tensor) -> Result<Tensor> {
        let eps = 1e-5_f64;
        let d = x.dim(1)?;
        let mean = (x.sum(1)? / d as f64)?;
        let mean = mean.unsqueeze(1)?;
        let diff = x.broadcast_sub(&mean)?;
        let var = ((&diff * &diff)?.sum(1)? / d as f64)?;
        let std = (var + eps)?.sqrt()?.unsqueeze(1)?;
        let normed = diff.broadcast_div(&std)?;
        Ok(normed
            .broadcast_mul(&self.weight)?
            .broadcast_add(&self.bias)?)
    }
}

impl TrainableMultiHeadAttention {
    fn forward(&self, x: &Tensor) -> Result<Tensor> {
        let n = x.dim(0)?;
        let d = x.dim(1)?;
        let h = self.n_heads;
        let hd = self.head_dim;

        let q = x.matmul(&self.wq.t()?)?.broadcast_add(&self.bq)?;
        let k = x.matmul(&self.wk.t()?)?.broadcast_add(&self.bk)?;
        let v = x.matmul(&self.wv.t()?)?.broadcast_add(&self.bv)?;

        let q = q.reshape((n, h, hd))?.transpose(0, 1)?;
        let k = k.reshape((n, h, hd))?.transpose(0, 1)?;
        let v = v.reshape((n, h, hd))?.transpose(0, 1)?;

        let scale = (hd as f64).sqrt();
        let attn_weights = (q.matmul(&k.transpose(1, 2)?)? / scale)?;

        let attn_max = attn_weights.max(2)?.unsqueeze(2)?;
        let shifted = attn_weights.broadcast_sub(&attn_max)?;
        let exp = shifted.exp()?;
        let sum_exp = exp.sum(2)?.unsqueeze(2)?;
        let attn_probs = exp.broadcast_div(&sum_exp)?;

        let attn_out = attn_probs.matmul(&v)?;
        let attn_out = attn_out.transpose(0, 1)?.reshape((n, d))?;

        Ok(attn_out
            .matmul(&self.out_weight.t()?)?
            .broadcast_add(&self.out_bias)?)
    }
}

impl TrainableFFN {
    fn forward(&self, x: &Tensor) -> Result<Tensor> {
        let h = x.matmul(&self.w1.t()?)?.broadcast_add(&self.b1)?;
        let h = h.gelu_erf()?;
        Ok(h.matmul(&self.w2.t()?)?.broadcast_add(&self.b2)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use candle_nn::{AdamW, Optimizer, ParamsAdamW};

    #[test]
    fn test_trainable_param_count() {
        let varmap = VarMap::new();
        let config = SiblingContextConfig::default();
        let device = Device::Cpu;
        let model = SiblingContextTrainable::new(&varmap, &config, &device).unwrap();
        assert_eq!(model.param_count(), 396800);

        let varmap_params: usize = varmap
            .all_vars()
            .iter()
            .map(|v| v.as_tensor().elem_count())
            .sum();
        assert_eq!(varmap_params, 396800);
    }

    #[test]
    fn test_trainable_forward_shape() {
        let varmap = VarMap::new();
        let config = SiblingContextConfig::default();
        let device = Device::Cpu;
        let model = SiblingContextTrainable::new(&varmap, &config, &device).unwrap();

        for n in [1, 5, 10, 20] {
            let input = Tensor::randn(0.0f32, 1.0, (n, 128), &device).unwrap();
            let output = model.forward(&input).unwrap();
            assert_eq!(output.dims(), &[n, 128], "Shape mismatch for N={}", n);
        }
    }

    #[test]
    fn test_trainable_save_load_round_trip() {
        let varmap = VarMap::new();
        let config = SiblingContextConfig::default();
        let device = Device::Cpu;
        let model = SiblingContextTrainable::new(&varmap, &config, &device).unwrap();

        let tmp_dir = std::env::temp_dir().join("finetype_sibling_train_test");
        let _ = std::fs::remove_dir_all(&tmp_dir);
        std::fs::create_dir_all(&tmp_dir).unwrap();

        let model_path = tmp_dir.join("model.safetensors");
        varmap.save(&model_path).unwrap();

        let config_json = serde_json::to_string_pretty(&config).unwrap();
        std::fs::write(tmp_dir.join("config.json"), &config_json).unwrap();

        let loaded = finetype_model::SiblingContextAttention::load(&tmp_dir).unwrap();
        assert_eq!(loaded.param_count(), 396800);

        let input = Tensor::randn(0.0f32, 1.0, (5, 128), &device).unwrap();
        let out_train: Vec<f32> = model
            .forward(&input)
            .unwrap()
            .flatten_all()
            .unwrap()
            .to_vec1()
            .unwrap();
        let out_infer: Vec<f32> = loaded
            .forward(&input)
            .unwrap()
            .flatten_all()
            .unwrap()
            .to_vec1()
            .unwrap();

        for (a, b) in out_train.iter().zip(out_infer.iter()) {
            assert!((a - b).abs() < 1e-5, "Round-trip mismatch: {} vs {}", a, b);
        }

        let _ = std::fs::remove_dir_all(&tmp_dir);
    }

    /// Verify gradient flow: attention weights change after backward_step.
    ///
    /// Uses a simple MSE loss directly on attention output (bypassing Sense)
    /// to isolate the gradient flow test from frozen downstream models.
    #[test]
    fn test_gradient_flow() {
        let varmap = VarMap::new();
        let config = SiblingContextConfig::default();
        let device = Device::Cpu;
        let model = SiblingContextTrainable::new(&varmap, &config, &device).unwrap();

        // Snapshot initial weight
        let initial: Vec<f32> = varmap.all_vars()[0]
            .as_tensor()
            .flatten_all()
            .unwrap()
            .to_vec1()
            .unwrap();

        // Simple loss: MSE between attention output and a target
        let input = Tensor::randn(0.0f32, 1.0, (3, 128), &device).unwrap();
        let target = Tensor::randn(0.0f32, 1.0, (3, 128), &device).unwrap();
        let output = model.forward(&input).unwrap();
        let loss = (&output - &target)
            .unwrap()
            .sqr()
            .unwrap()
            .mean_all()
            .unwrap();

        let adamw_params = ParamsAdamW {
            lr: 1e-2,
            weight_decay: 0.0,
            ..Default::default()
        };
        let mut optimizer = AdamW::new(varmap.all_vars(), adamw_params).unwrap();
        optimizer.backward_step(&loss).unwrap();

        // Verify weights changed
        let updated: Vec<f32> = varmap.all_vars()[0]
            .as_tensor()
            .flatten_all()
            .unwrap()
            .to_vec1()
            .unwrap();

        let max_diff: f32 = initial
            .iter()
            .zip(updated.iter())
            .map(|(a, b)| (a - b).abs())
            .fold(0.0f32, f32::max);

        assert!(
            max_diff > 1e-8,
            "Weights should change after backward_step, max_diff={}",
            max_diff
        );
    }
}
