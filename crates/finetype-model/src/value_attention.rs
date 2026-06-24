//! Cross-value attention pooling for the value branch (choice 0106).
//!
//! Replaces the value branch's fixed mean/var/min/max pool with a *learned* pool
//! over the column's per-value embeddings:
//!
//! ```text
//!   value embeds [B, N, D]  ──▶ cross-value self-attention (pre-norm, masked)
//!                            ──▶ PMA pool (learned seed query, masked)
//!                            ──▶ [B, pool_slots · D]
//! ```
//!
//! Each value's representation is informed by the rest of the column (cross-value
//! context the blender throws away), then a small set of learned query "slots"
//! attends over the contextualised values to produce a fixed-size descriptor.
//! Cost is one pass over ~50 precomputed static vectors per column — near-free vs
//! a per-value transformer forward (the latency budget that ruled out gte-tiny).
//!
//! This is a SINGLE Candle module shared by training (`finetype-train`, VarMap-
//! backed) and inference (`finetype-model`, safetensors-backed): both construct it
//! from a [`VarBuilder`], so there is no Python-vs-Rust dual format and the
//! train/infer parity check is a numerical confirmation of one codepath.

use candle_core::{Result, Tensor, D};
use candle_nn::{layer_norm, linear, LayerNorm, Linear, Module, VarBuilder};
use serde::{Deserialize, Serialize};

/// Configuration for the value attention pool (mirrors the `value_attention`
/// block of the model config.json).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValueAttentionConfig {
    /// Per-value embedding width (the value encoder's dim, e.g. potion-8M = 256).
    pub value_embed_dim: usize,
    /// Number of values attended over per column (sequence length; pad/masked).
    pub n_values: usize,
    /// Attention heads (must divide `value_embed_dim`).
    pub n_heads: usize,
    /// Cross-value self-attention layers before pooling.
    pub self_attn_layers: usize,
    /// FFN hidden width inside each self-attention block.
    pub ffn_hidden: usize,
    /// Number of learned PMA query slots; output width is `pool_slots · value_embed_dim`.
    pub pool_slots: usize,
    /// Dropout applied to attention/FFN outputs during training.
    pub dropout: f32,
    /// Keep the mean/var/min/max blender as an auxiliary concat into the value
    /// branch (D4: strictly additive — the branch can fall back to the blender, so
    /// the attention pool cannot regress below today's value-branch baseline).
    #[serde(default = "default_true")]
    pub keep_blender_concat: bool,
}

fn default_true() -> bool {
    true
}

impl Default for ValueAttentionConfig {
    fn default() -> Self {
        Self {
            value_embed_dim: 256,
            n_values: 50,
            n_heads: 4,
            self_attn_layers: 1,
            ffn_hidden: 512,
            pool_slots: 4,
            dropout: 0.1,
            keep_blender_concat: true,
        }
    }
}

impl ValueAttentionConfig {
    /// Output feature width fed into the value branch (concatenated after the
    /// mean/var/min/max blender when `keep_blender_concat` is set).
    pub fn output_dim(&self) -> usize {
        self.pool_slots * self.value_embed_dim
    }
}

/// Batched multi-head attention with an additive key-padding mask.
///
/// `q`: `[B, Lq, D]`, `k`/`v`: `[B, Lk, D]`, `key_mask`: `[B, Lk]` (1.0 valid,
/// 0.0 pad). Returns `[B, Lq, D]`. Padded keys are excluded from the softmax.
struct MultiHeadAttention {
    q_proj: Linear,
    k_proj: Linear,
    v_proj: Linear,
    out_proj: Linear,
    n_heads: usize,
    head_dim: usize,
}

impl MultiHeadAttention {
    fn new(dim: usize, n_heads: usize, vb: VarBuilder) -> Result<Self> {
        assert!(
            dim.is_multiple_of(n_heads),
            "value_embed_dim {dim} not divisible by n_heads {n_heads}"
        );
        Ok(Self {
            q_proj: linear(dim, dim, vb.pp("q_proj"))?,
            k_proj: linear(dim, dim, vb.pp("k_proj"))?,
            v_proj: linear(dim, dim, vb.pp("v_proj"))?,
            out_proj: linear(dim, dim, vb.pp("out_proj"))?,
            n_heads,
            head_dim: dim / n_heads,
        })
    }

    /// Split `[B, L, D]` into heads `[B, h, L, hd]`.
    fn split_heads(&self, x: &Tensor) -> Result<Tensor> {
        let (b, l, _d) = x.dims3()?;
        x.reshape((b, l, self.n_heads, self.head_dim))?
            .transpose(1, 2)?
            .contiguous()
    }

    fn forward(&self, q_in: &Tensor, kv_in: &Tensor, key_mask: &Tensor) -> Result<Tensor> {
        let (b, lq, d) = q_in.dims3()?;
        let lk = kv_in.dim(1)?;

        let q = self.split_heads(&self.q_proj.forward(q_in)?)?; // [B, h, Lq, hd]
        let k = self.split_heads(&self.k_proj.forward(kv_in)?)?; // [B, h, Lk, hd]
        let v = self.split_heads(&self.v_proj.forward(kv_in)?)?; // [B, h, Lk, hd]

        let scale = (self.head_dim as f64).sqrt();
        let scores = (q.matmul(&k.transpose(2, 3)?.contiguous()?)? / scale)?; // [B, h, Lq, Lk]

        // Additive mask: pad keys → large negative so they vanish in softmax.
        // key_mask [B, Lk] → [B, 1, 1, Lk]; additive = (mask - 1) * 1e9.
        let add_mask = ((key_mask.reshape((b, 1, 1, lk))? - 1.0)? * 1e9_f64)?;
        let scores = scores.broadcast_add(&add_mask)?;

        let probs = candle_nn::ops::softmax(&scores, D::Minus1)?; // [B, h, Lq, Lk]
        let ctx = probs.matmul(&v)?; // [B, h, Lq, hd]

        let ctx = ctx
            .transpose(1, 2)? // [B, Lq, h, hd]
            .contiguous()?
            .reshape((b, lq, d))?;
        self.out_proj.forward(&ctx)
    }
}

/// One pre-norm transformer block: x + MHSA(LN(x)); x + FFN(LN(x)).
struct SelfAttnBlock {
    ln1: LayerNorm,
    attn: MultiHeadAttention,
    ln2: LayerNorm,
    fc1: Linear,
    fc2: Linear,
    dropout: f32,
}

impl SelfAttnBlock {
    fn new(cfg: &ValueAttentionConfig, vb: VarBuilder) -> Result<Self> {
        let d = cfg.value_embed_dim;
        Ok(Self {
            ln1: layer_norm(d, 1e-5, vb.pp("ln1"))?,
            attn: MultiHeadAttention::new(d, cfg.n_heads, vb.pp("attn"))?,
            ln2: layer_norm(d, 1e-5, vb.pp("ln2"))?,
            fc1: linear(d, cfg.ffn_hidden, vb.pp("fc1"))?,
            fc2: linear(cfg.ffn_hidden, d, vb.pp("fc2"))?,
            dropout: cfg.dropout,
        })
    }

    fn maybe_dropout(&self, x: &Tensor, train: bool) -> Result<Tensor> {
        if train && self.dropout > 0.0 {
            candle_nn::ops::dropout(x, self.dropout)
        } else {
            Ok(x.clone())
        }
    }

    fn forward(&self, x: &Tensor, key_mask: &Tensor, train: bool) -> Result<Tensor> {
        let h = self.ln1.forward(x)?;
        let attn = self.attn.forward(&h, &h, key_mask)?;
        let x = (x + self.maybe_dropout(&attn, train)?)?;

        let h = self.ln2.forward(&x)?;
        let ff = self.fc2.forward(&self.fc1.forward(&h)?.gelu_erf()?)?;
        x + self.maybe_dropout(&ff, train)?
    }
}

/// Cross-value attention pool: self-attention stack + PMA pooling by a learned seed.
pub struct ValueAttentionPool {
    blocks: Vec<SelfAttnBlock>,
    /// Learned PMA query seed: `[pool_slots, D]`.
    seed: Tensor,
    pool_attn: MultiHeadAttention,
    cfg: ValueAttentionConfig,
}

impl ValueAttentionPool {
    pub fn new(cfg: &ValueAttentionConfig, vb: VarBuilder) -> Result<Self> {
        let d = cfg.value_embed_dim;
        let mut blocks = Vec::with_capacity(cfg.self_attn_layers);
        for i in 0..cfg.self_attn_layers {
            blocks.push(SelfAttnBlock::new(cfg, vb.pp(format!("block{i}")))?);
        }
        // Learned seed, small init. `get_with_hints` gives a trainable Var under a
        // VarMap, or loads the persisted tensor at inference.
        let seed = vb.get_with_hints(
            (cfg.pool_slots, d),
            "pool_seed",
            candle_nn::Init::Randn {
                mean: 0.0,
                stdev: 0.02,
            },
        )?;
        let pool_attn = MultiHeadAttention::new(d, cfg.n_heads, vb.pp("pool_attn"))?;
        Ok(Self {
            blocks,
            seed,
            pool_attn,
            cfg: cfg.clone(),
        })
    }

    pub fn output_dim(&self) -> usize {
        self.cfg.output_dim()
    }

    /// `value_embeds`: `[B, N, D]`, `mask`: `[B, N]` (1.0 valid, 0.0 pad).
    /// Returns the pooled descriptor `[B, pool_slots · D]`.
    pub fn forward(&self, value_embeds: &Tensor, mask: &Tensor, train: bool) -> Result<Tensor> {
        let (b, _n, d) = value_embeds.dims3()?;

        // Cross-value self-attention (each value sees the rest of the column).
        let mut x = value_embeds.clone();
        for block in &self.blocks {
            x = block.forward(&x, mask, train)?;
        }

        // PMA: learned seed queries attend over the contextualised values.
        let seed = self
            .seed
            .unsqueeze(0)?
            .broadcast_as((b, self.cfg.pool_slots, d))?
            .contiguous()?;
        let pooled = self.pool_attn.forward(&seed, &x, mask)?; // [B, slots, D]
        pooled.reshape((b, self.cfg.pool_slots * d))
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use candle_core::{DType, Device};
    use candle_nn::VarMap;

    fn build(cfg: &ValueAttentionConfig) -> (ValueAttentionPool, Device) {
        let dev = Device::Cpu;
        let varmap = VarMap::new();
        let vb = VarBuilder::from_varmap(&varmap, DType::F32, &dev);
        (ValueAttentionPool::new(cfg, vb).unwrap(), dev)
    }

    #[test]
    fn test_output_shape() {
        let cfg = ValueAttentionConfig::default();
        let (pool, dev) = build(&cfg);
        let (b, n, d) = (3usize, cfg.n_values, cfg.value_embed_dim);
        let x = Tensor::randn(0f32, 1.0, (b, n, d), &dev).unwrap();
        let mask = Tensor::ones((b, n), DType::F32, &dev).unwrap();
        let out = pool.forward(&x, &mask, false).unwrap();
        assert_eq!(out.dims(), &[b, cfg.output_dim()]);
        assert_eq!(cfg.output_dim(), cfg.pool_slots * cfg.value_embed_dim);
    }

    #[test]
    fn test_padding_is_ignored() {
        // Two columns identical in their first 3 values; one is padded with junk in
        // the masked-out tail. With the tail masked, the pooled output must match
        // the version where the tail is zeros — i.e. masked positions don't leak.
        let cfg = ValueAttentionConfig {
            n_values: 6,
            ..Default::default()
        };
        let (pool, dev) = build(&cfg);
        let d = cfg.value_embed_dim;

        let real = Tensor::randn(0f32, 1.0, (1, 3, d), &dev).unwrap();
        let junk = Tensor::randn(5f32, 1.0, (1, 3, d), &dev).unwrap();
        let zeros = Tensor::zeros((1, 3, d), DType::F32, &dev).unwrap();

        // [real(3) ++ junk(3)] vs [real(3) ++ zeros(3)], both masking the last 3.
        let a = Tensor::cat(&[&real, &junk], 1).unwrap();
        let b = Tensor::cat(&[&real, &zeros], 1).unwrap();
        let mask = Tensor::from_vec(vec![1f32, 1., 1., 0., 0., 0.], (1, 6), &dev).unwrap();

        let oa = pool.forward(&a, &mask, false).unwrap();
        let ob = pool.forward(&b, &mask, false).unwrap();
        let diff = (oa - ob).unwrap().abs().unwrap().max(D::Minus1).unwrap();
        let maxd = diff.to_vec1::<f32>().unwrap()[0];
        assert!(maxd < 1e-4, "masked tail leaked into pool: max diff {maxd}");
    }

    #[test]
    fn test_save_load_roundtrip() {
        // The pool's weights must serialise on the training side and deserialise on
        // the inference side under the SAME VarBuilder prefix (choice 0106 D5). Build
        // with random weights, save, reload from safetensors, and confirm identical
        // output — the train-save → infer-load contract ac-3 depends on.
        let cfg = ValueAttentionConfig {
            n_values: 6,
            ..Default::default()
        };
        let dev = Device::Cpu;
        let varmap = VarMap::new();
        let vb = VarBuilder::from_varmap(&varmap, DType::F32, &dev);
        let pool1 = ValueAttentionPool::new(&cfg, vb.pp("value_attn")).unwrap();

        let path = std::env::temp_dir().join(format!("va_pool_{}.safetensors", std::process::id()));
        varmap.save(&path).unwrap();

        let vb2 =
            unsafe { VarBuilder::from_mmaped_safetensors(&[&path], DType::F32, &dev).unwrap() };
        let pool2 = ValueAttentionPool::new(&cfg, vb2.pp("value_attn")).unwrap();

        let (b, n, d) = (2usize, cfg.n_values, cfg.value_embed_dim);
        let x = Tensor::randn(0f32, 1.0, (b, n, d), &dev).unwrap();
        let mask = Tensor::ones((b, n), DType::F32, &dev).unwrap();
        let o1 = pool1.forward(&x, &mask, false).unwrap();
        let o2 = pool2.forward(&x, &mask, false).unwrap();
        let maxd = (o1 - o2)
            .unwrap()
            .abs()
            .unwrap()
            .max(D::Minus1)
            .unwrap()
            .max(D::Minus1)
            .unwrap()
            .to_scalar::<f32>()
            .unwrap();
        std::fs::remove_file(&path).ok();
        assert!(maxd < 1e-6, "reloaded pool diverged: max diff {maxd}");
    }

    #[test]
    fn test_single_valid_value() {
        // A column with one real value (rest padded) must produce a finite pool.
        let cfg = ValueAttentionConfig {
            n_values: 4,
            ..Default::default()
        };
        let (pool, dev) = build(&cfg);
        let d = cfg.value_embed_dim;
        let x = Tensor::randn(0f32, 1.0, (1, 4, d), &dev).unwrap();
        let mask = Tensor::from_vec(vec![1f32, 0., 0., 0.], (1, 4), &dev).unwrap();
        let out = pool.forward(&x, &mask, false).unwrap();
        let vals = out.flatten_all().unwrap().to_vec1::<f32>().unwrap();
        assert!(
            vals.iter().all(|v| v.is_finite()),
            "pool produced non-finite output"
        );
    }
}
