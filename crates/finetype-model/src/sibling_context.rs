//! Sibling-context self-attention module (NNFT-268).
//!
//! Enriches column embeddings with cross-column context before Sense classification.
//! When a table has columns ["city", "name", "email"], the "name" column gets signal
//! from its siblings — resolving ambiguity between person name, city name, and
//! company name.
//!
//! Architecture per transformer block (pre-norm):
//!   x → LayerNorm → MultiHeadSelfAttention → + residual → LayerNorm → FFN → + residual
//!
//! Default config: 2 layers, 4 heads, 128-dim (matching Model2Vec potion-base-4M).
//! Parameters: 396,800 (1.51 MB as f32).
//!
//! When no trained model is available, the pipeline bypasses this module entirely
//! and falls back to per-column classification (identical to current behaviour).

use crate::inference::InferenceError;
use candle_core::{DType, Device, Tensor};
use std::collections::HashMap;
use std::path::Path;

// ═══════════════════════════════════════════════════════════════════════════════
// Configuration
// ═══════════════════════════════════════════════════════════════════════════════

/// Configuration for the sibling-context attention module.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct SiblingContextConfig {
    /// Embedding dimension (must match Model2Vec output dim).
    pub embed_dim: usize,
    /// Number of attention heads.
    pub n_heads: usize,
    /// Number of transformer layers.
    pub n_layers: usize,
}

impl Default for SiblingContextConfig {
    fn default() -> Self {
        Self {
            embed_dim: 128,
            n_heads: 4,
            n_layers: 2,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Multi-head self-attention
// ═══════════════════════════════════════════════════════════════════════════════

/// Weights for a single multi-head self-attention layer.
struct MultiHeadSelfAttention {
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

impl MultiHeadSelfAttention {
    /// Load from a tensor map with the given key prefix (e.g., "blocks.0.attn").
    fn from_tensors(
        tensors: &HashMap<String, Tensor>,
        prefix: &str,
        embed_dim: usize,
        n_heads: usize,
    ) -> Result<Self, InferenceError> {
        assert!(embed_dim.is_multiple_of(n_heads));
        let head_dim = embed_dim / n_heads;

        let get = |name: &str| -> Result<Tensor, InferenceError> {
            let key = format!("{}.{}", prefix, name);
            tensors
                .get(&key)
                .ok_or_else(|| {
                    InferenceError::InvalidPath(format!(
                        "Missing tensor '{}' in sibling-context model",
                        key
                    ))
                })
                .and_then(|t| Ok(t.to_dtype(DType::F32)?))
        };

        Ok(Self {
            wq: get("wq")?,
            bq: get("bq")?,
            wk: get("wk")?,
            bk: get("bk")?,
            wv: get("wv")?,
            bv: get("bv")?,
            out_weight: get("out_weight")?,
            out_bias: get("out_bias")?,
            n_heads,
            head_dim,
        })
    }

    /// Collect all tensors into a map for serialisation.
    fn to_tensors(&self, prefix: &str) -> Vec<(String, Tensor)> {
        vec![
            (format!("{}.wq", prefix), self.wq.clone()),
            (format!("{}.bq", prefix), self.bq.clone()),
            (format!("{}.wk", prefix), self.wk.clone()),
            (format!("{}.bk", prefix), self.bk.clone()),
            (format!("{}.wv", prefix), self.wv.clone()),
            (format!("{}.bv", prefix), self.bv.clone()),
            (format!("{}.out_weight", prefix), self.out_weight.clone()),
            (format!("{}.out_bias", prefix), self.out_bias.clone()),
        ]
    }

    /// Forward pass: [N, D] → [N, D]
    ///
    /// Self-attention: each column embedding attends to all column embeddings.
    fn forward(&self, x: &Tensor) -> Result<Tensor, InferenceError> {
        let n = x.dim(0)?;
        let d = x.dim(1)?;
        let h = self.n_heads;
        let hd = self.head_dim;

        // Project Q, K, V: [N, D] × [D, D]^T → [N, D]
        let q = x.matmul(&self.wq.t()?)?.broadcast_add(&self.bq)?;
        let k = x.matmul(&self.wk.t()?)?.broadcast_add(&self.bk)?;
        let v = x.matmul(&self.wv.t()?)?.broadcast_add(&self.bv)?;

        // Reshape to multi-head: [N, D] → [N, h, hd] → [h, N, hd]
        let q = q.reshape((n, h, hd))?.transpose(0, 1)?; // [h, N, hd]
        let k = k.reshape((n, h, hd))?.transpose(0, 1)?; // [h, N, hd]
        let v = v.reshape((n, h, hd))?.transpose(0, 1)?; // [h, N, hd]

        // Scaled dot-product attention: Q @ K^T / sqrt(hd)
        let scale = (hd as f64).sqrt();
        let attn_weights = (q.matmul(&k.transpose(1, 2)?)? / scale)?; // [h, N, N]

        // Softmax over last dimension (key dimension)
        let attn_max = attn_weights.max(2)?.unsqueeze(2)?; // [h, N, 1]
        let shifted = attn_weights.broadcast_sub(&attn_max)?;
        let exp = shifted.exp()?;
        let sum_exp = exp.sum(2)?.unsqueeze(2)?; // [h, N, 1]
        let attn_probs = exp.broadcast_div(&sum_exp)?; // [h, N, N]

        // Weighted sum: [h, N, N] @ [h, N, hd] → [h, N, hd]
        let attn_out = attn_probs.matmul(&v)?;

        // Concatenate heads: [h, N, hd] → [N, h, hd] → [N, D]
        let attn_out = attn_out.transpose(0, 1)?.reshape((n, d))?;

        // Output projection
        Ok(attn_out
            .matmul(&self.out_weight.t()?)?
            .broadcast_add(&self.out_bias)?)
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// LayerNorm
// ═══════════════════════════════════════════════════════════════════════════════

/// Layer normalisation weights.
struct LayerNorm {
    weight: Tensor, // [D]
    bias: Tensor,   // [D]
}

impl LayerNorm {
    /// Load from a tensor map with the given key prefix.
    fn from_tensors(
        tensors: &HashMap<String, Tensor>,
        prefix: &str,
    ) -> Result<Self, InferenceError> {
        let get = |name: &str| -> Result<Tensor, InferenceError> {
            let key = format!("{}.{}", prefix, name);
            tensors
                .get(&key)
                .ok_or_else(|| {
                    InferenceError::InvalidPath(format!(
                        "Missing tensor '{}' in sibling-context model",
                        key
                    ))
                })
                .and_then(|t| Ok(t.to_dtype(DType::F32)?))
        };

        Ok(Self {
            weight: get("weight")?,
            bias: get("bias")?,
        })
    }

    /// Collect tensors for serialisation.
    fn to_tensors(&self, prefix: &str) -> Vec<(String, Tensor)> {
        vec![
            (format!("{}.weight", prefix), self.weight.clone()),
            (format!("{}.bias", prefix), self.bias.clone()),
        ]
    }

    /// Forward: per-row normalisation. Input [N, D] → [N, D]
    fn forward(&self, x: &Tensor) -> Result<Tensor, InferenceError> {
        let eps = 1e-5_f64;
        let d = x.dim(1)?;

        // Compute per-row mean and variance
        let mean = (x.sum(1)? / d as f64)?; // [N]
        let mean = mean.unsqueeze(1)?; // [N, 1]
        let diff = x.broadcast_sub(&mean)?;
        let var = ((&diff * &diff)?.sum(1)? / d as f64)?; // [N]
        let std = (var + eps)?.sqrt()?.unsqueeze(1)?; // [N, 1]
        let normed = diff.broadcast_div(&std)?;

        // Scale and shift
        Ok(normed
            .broadcast_mul(&self.weight)?
            .broadcast_add(&self.bias)?)
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Feed-forward network
// ═══════════════════════════════════════════════════════════════════════════════

/// Two-layer feed-forward network with GELU activation.
struct FeedForward {
    w1: Tensor, // [4D, D]
    b1: Tensor, // [4D]
    w2: Tensor, // [D, 4D]
    b2: Tensor, // [D]
}

impl FeedForward {
    /// Load from a tensor map with the given key prefix.
    fn from_tensors(
        tensors: &HashMap<String, Tensor>,
        prefix: &str,
    ) -> Result<Self, InferenceError> {
        let get = |name: &str| -> Result<Tensor, InferenceError> {
            let key = format!("{}.{}", prefix, name);
            tensors
                .get(&key)
                .ok_or_else(|| {
                    InferenceError::InvalidPath(format!(
                        "Missing tensor '{}' in sibling-context model",
                        key
                    ))
                })
                .and_then(|t| Ok(t.to_dtype(DType::F32)?))
        };

        Ok(Self {
            w1: get("w1")?,
            b1: get("b1")?,
            w2: get("w2")?,
            b2: get("b2")?,
        })
    }

    /// Collect tensors for serialisation.
    fn to_tensors(&self, prefix: &str) -> Vec<(String, Tensor)> {
        vec![
            (format!("{}.w1", prefix), self.w1.clone()),
            (format!("{}.b1", prefix), self.b1.clone()),
            (format!("{}.w2", prefix), self.w2.clone()),
            (format!("{}.b2", prefix), self.b2.clone()),
        ]
    }

    /// Forward: Linear(D, 4D) → GELU → Linear(4D, D). Input [N, D] → [N, D]
    fn forward(&self, x: &Tensor) -> Result<Tensor, InferenceError> {
        let h = x.matmul(&self.w1.t()?)?.broadcast_add(&self.b1)?;
        let h = h.gelu_erf()?;
        Ok(h.matmul(&self.w2.t()?)?.broadcast_add(&self.b2)?)
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Transformer block: LN → MHA → residual → LN → FFN → residual
// ═══════════════════════════════════════════════════════════════════════════════

/// Pre-norm transformer block.
struct TransformerBlock {
    norm1: LayerNorm,
    attn: MultiHeadSelfAttention,
    norm2: LayerNorm,
    ffn: FeedForward,
}

impl TransformerBlock {
    /// Load from tensor map with block index prefix (e.g., "blocks.0").
    fn from_tensors(
        tensors: &HashMap<String, Tensor>,
        block_prefix: &str,
        embed_dim: usize,
        n_heads: usize,
    ) -> Result<Self, InferenceError> {
        Ok(Self {
            norm1: LayerNorm::from_tensors(tensors, &format!("{}.norm1", block_prefix))?,
            attn: MultiHeadSelfAttention::from_tensors(
                tensors,
                &format!("{}.attn", block_prefix),
                embed_dim,
                n_heads,
            )?,
            norm2: LayerNorm::from_tensors(tensors, &format!("{}.norm2", block_prefix))?,
            ffn: FeedForward::from_tensors(tensors, &format!("{}.ffn", block_prefix))?,
        })
    }

    /// Collect all tensors for serialisation.
    fn to_tensors(&self, block_prefix: &str) -> Vec<(String, Tensor)> {
        let mut tensors = Vec::new();
        tensors.extend(self.norm1.to_tensors(&format!("{}.norm1", block_prefix)));
        tensors.extend(self.attn.to_tensors(&format!("{}.attn", block_prefix)));
        tensors.extend(self.norm2.to_tensors(&format!("{}.norm2", block_prefix)));
        tensors.extend(self.ffn.to_tensors(&format!("{}.ffn", block_prefix)));
        tensors
    }

    /// Forward: Pre-norm transformer block. Input [N, D] → [N, D]
    fn forward(&self, x: &Tensor) -> Result<Tensor, InferenceError> {
        // Self-attention with residual
        let normed = self.norm1.forward(x)?;
        let attn_out = self.attn.forward(&normed)?;
        let x = (x + &attn_out)?;

        // FFN with residual
        let normed = self.norm2.forward(&x)?;
        let ffn_out = self.ffn.forward(&normed)?;
        Ok((&x + &ffn_out)?)
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// SiblingContextAttention
// ═══════════════════════════════════════════════════════════════════════════════

/// Sibling-context self-attention module.
///
/// Enriches per-column Model2Vec embeddings with cross-column context via a
/// stack of pre-norm transformer self-attention blocks. Output has the same
/// shape as input: `[N_cols, embed_dim]`.
///
/// When N=1, self-attention reduces to identity (residual connection preserves
/// the input), so single-column classification degrades gracefully.
pub struct SiblingContextAttention {
    blocks: Vec<TransformerBlock>,
    final_norm: LayerNorm,
    embed_dim: usize,
    #[allow(dead_code)]
    device: Device,
}

impl SiblingContextAttention {
    /// Load from in-memory byte slices (model.safetensors + config.json).
    pub fn from_bytes(model_bytes: &[u8], config_bytes: &[u8]) -> Result<Self, InferenceError> {
        let device = Device::Cpu;

        // Parse config
        let config: SiblingContextConfig = serde_json::from_slice(config_bytes).map_err(|e| {
            InferenceError::InvalidPath(format!("Failed to parse sibling-context config: {}", e))
        })?;

        // Load all tensors
        let tensors = candle_core::safetensors::load_buffer(model_bytes, &device)?;

        // Build blocks
        let mut blocks = Vec::with_capacity(config.n_layers);
        for i in 0..config.n_layers {
            let prefix = format!("blocks.{}", i);
            blocks.push(TransformerBlock::from_tensors(
                &tensors,
                &prefix,
                config.embed_dim,
                config.n_heads,
            )?);
        }

        let final_norm = LayerNorm::from_tensors(&tensors, "final_norm")?;

        Ok(Self {
            blocks,
            final_norm,
            embed_dim: config.embed_dim,
            device,
        })
    }

    /// Load from a directory containing `model.safetensors` and `config.json`.
    pub fn load<P: AsRef<Path>>(model_dir: P) -> Result<Self, InferenceError> {
        let dir = model_dir.as_ref();
        let model_bytes = std::fs::read(dir.join("model.safetensors"))?;
        let config_bytes = std::fs::read(dir.join("config.json"))?;
        Self::from_bytes(&model_bytes, &config_bytes)
    }

    /// Forward pass: enrich column embeddings with sibling context.
    ///
    /// Input: `[N_cols, embed_dim]` — raw Model2Vec column header embeddings.
    /// Output: `[N_cols, embed_dim]` — context-enriched embeddings.
    ///
    /// When N=1, self-attention attends only to itself and the residual
    /// connection dominates, preserving the original embedding.
    pub fn forward(&self, column_embeddings: &Tensor) -> Result<Tensor, InferenceError> {
        let mut out = column_embeddings.clone();
        for block in &self.blocks {
            out = block.forward(&out)?;
        }
        self.final_norm.forward(&out)
    }

    /// Save model weights and config to a directory.
    ///
    /// Writes `model.safetensors` and `config.json`.
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), InferenceError> {
        let dir = path.as_ref();
        std::fs::create_dir_all(dir)?;

        // Collect all tensors into a HashMap for safetensors
        let mut tensor_map: HashMap<String, Tensor> = HashMap::new();
        for (i, block) in self.blocks.iter().enumerate() {
            for (name, tensor) in block.to_tensors(&format!("blocks.{}", i)) {
                tensor_map.insert(name, tensor);
            }
        }
        for (name, tensor) in self.final_norm.to_tensors("final_norm") {
            tensor_map.insert(name, tensor);
        }

        candle_core::safetensors::save(&tensor_map, dir.join("model.safetensors"))?;

        // Write config
        let config = SiblingContextConfig {
            embed_dim: self.embed_dim,
            n_heads: if let Some(block) = self.blocks.first() {
                block.attn.n_heads
            } else {
                4
            },
            n_layers: self.blocks.len(),
        };
        let config_json = serde_json::to_string_pretty(&config).map_err(|e| {
            InferenceError::InvalidPath(format!("Failed to serialize config: {}", e))
        })?;
        std::fs::write(dir.join("config.json"), config_json)?;

        Ok(())
    }

    /// Count total trainable parameters.
    ///
    /// Expected: 396,800 for default config (2 layers, 4 heads, 128-dim).
    pub fn param_count(&self) -> usize {
        let d = self.embed_dim;
        let ff = d * 4;
        let per_block =
            // MHA: Q, K, V, Out projections (weight + bias each)
            4 * (d * d + d) +
            // LayerNorm x2
            2 * (d + d) +
            // FFN: two linear layers
            (ff * d + ff) + (d * ff + d);
        let final_norm = d + d;
        per_block * self.blocks.len() + final_norm
    }

    /// Get the embedding dimension.
    pub fn embed_dim(&self) -> usize {
        self.embed_dim
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    use rand::Rng;

    fn rand_matrix(rng: &mut impl Rng, rows: usize, cols: usize, scale: f32) -> Tensor {
        let device = Device::Cpu;
        let data: Vec<f32> = (0..rows * cols)
            .map(|_| rng.gen_range(-scale..scale))
            .collect();
        Tensor::from_vec(data, (rows, cols), &device).unwrap()
    }

    fn rand_vec(rng: &mut impl Rng, n: usize, scale: f32) -> Tensor {
        let device = Device::Cpu;
        let data: Vec<f32> = (0..n).map(|_| rng.gen_range(-scale..scale)).collect();
        Tensor::from_vec(data, n, &device).unwrap()
    }

    /// Helper: create a SiblingContextAttention with random weights for testing.
    fn create_random_model(
        embed_dim: usize,
        n_heads: usize,
        n_layers: usize,
    ) -> SiblingContextAttention {
        let device = Device::Cpu;
        let scale = (1.0 / embed_dim as f64).sqrt() as f32;
        let mut rng = rand::thread_rng();

        let ff_dim = embed_dim * 4;
        let head_dim = embed_dim / n_heads;

        let blocks: Vec<TransformerBlock> = (0..n_layers)
            .map(|_| TransformerBlock {
                norm1: LayerNorm {
                    weight: Tensor::ones(embed_dim, DType::F32, &device).unwrap(),
                    bias: Tensor::zeros(embed_dim, DType::F32, &device).unwrap(),
                },
                attn: MultiHeadSelfAttention {
                    wq: rand_matrix(&mut rng, embed_dim, embed_dim, scale),
                    bq: Tensor::zeros(embed_dim, DType::F32, &device).unwrap(),
                    wk: rand_matrix(&mut rng, embed_dim, embed_dim, scale),
                    bk: Tensor::zeros(embed_dim, DType::F32, &device).unwrap(),
                    wv: rand_matrix(&mut rng, embed_dim, embed_dim, scale),
                    bv: Tensor::zeros(embed_dim, DType::F32, &device).unwrap(),
                    out_weight: rand_matrix(&mut rng, embed_dim, embed_dim, scale),
                    out_bias: Tensor::zeros(embed_dim, DType::F32, &device).unwrap(),
                    n_heads,
                    head_dim,
                },
                norm2: LayerNorm {
                    weight: Tensor::ones(embed_dim, DType::F32, &device).unwrap(),
                    bias: Tensor::zeros(embed_dim, DType::F32, &device).unwrap(),
                },
                ffn: FeedForward {
                    w1: rand_matrix(&mut rng, ff_dim, embed_dim, scale),
                    b1: rand_vec(&mut rng, ff_dim, scale),
                    w2: rand_matrix(&mut rng, embed_dim, ff_dim, scale),
                    b2: rand_vec(&mut rng, embed_dim, scale),
                },
            })
            .collect();

        SiblingContextAttention {
            blocks,
            final_norm: LayerNorm {
                weight: Tensor::ones(embed_dim, DType::F32, &device).unwrap(),
                bias: Tensor::zeros(embed_dim, DType::F32, &device).unwrap(),
            },
            embed_dim,
            device,
        }
    }

    /// Helper: create random embeddings for N columns.
    fn random_embeddings(n_cols: usize, embed_dim: usize) -> Tensor {
        let device = Device::Cpu;
        let mut rng = rand::thread_rng();
        let data: Vec<f32> = (0..n_cols * embed_dim)
            .map(|_| rng.gen_range(-1.0f32..1.0f32))
            .collect();
        Tensor::from_vec(data, (n_cols, embed_dim), &device).unwrap()
    }

    #[test]
    fn test_shape_preservation() {
        let model = create_random_model(128, 4, 2);
        for n_cols in [1, 5, 10, 20] {
            let input = random_embeddings(n_cols, 128);
            let output = model.forward(&input).unwrap();
            assert_eq!(
                output.dims(),
                &[n_cols, 128],
                "Shape mismatch for N={}",
                n_cols
            );
        }
    }

    #[test]
    fn test_param_count() {
        let model = create_random_model(128, 4, 2);
        assert_eq!(
            model.param_count(),
            396800,
            "Expected 396,800 params for 2-layer, 4-head, 128-dim"
        );
    }

    #[test]
    fn test_safetensors_round_trip() {
        let model = create_random_model(128, 4, 2);

        // Create temp dir
        let tmp_dir = std::env::temp_dir().join("finetype_sibling_ctx_test");
        let _ = std::fs::remove_dir_all(&tmp_dir);
        std::fs::create_dir_all(&tmp_dir).unwrap();

        // Save
        model.save(&tmp_dir).unwrap();
        assert!(tmp_dir.join("model.safetensors").exists());
        assert!(tmp_dir.join("config.json").exists());

        // Load
        let loaded = SiblingContextAttention::load(&tmp_dir).unwrap();
        assert_eq!(loaded.embed_dim, 128);
        assert_eq!(loaded.blocks.len(), 2);
        assert_eq!(loaded.param_count(), 396800);

        // Verify identical output
        let input = random_embeddings(5, 128);
        let out_orig = model.forward(&input).unwrap();
        let out_loaded = loaded.forward(&input).unwrap();

        let orig_vec: Vec<f32> = out_orig.flatten_all().unwrap().to_vec1().unwrap();
        let loaded_vec: Vec<f32> = out_loaded.flatten_all().unwrap().to_vec1().unwrap();
        for (a, b) in orig_vec.iter().zip(loaded_vec.iter()) {
            assert!(
                (a - b).abs() < 1e-5,
                "Round-trip output mismatch: {} vs {}",
                a,
                b
            );
        }

        // Cleanup
        let _ = std::fs::remove_dir_all(&tmp_dir);
    }

    #[test]
    fn test_single_column_graceful_degradation() {
        let model = create_random_model(128, 4, 2);
        let input = random_embeddings(1, 128);
        let output = model.forward(&input).unwrap();

        // With random weights, residual connection should keep output correlated
        // with input (cosine similarity > 0)
        let in_vec: Vec<f32> = input.get(0).unwrap().to_vec1().unwrap();
        let out_vec: Vec<f32> = output.get(0).unwrap().to_vec1().unwrap();

        let dot: f32 = in_vec.iter().zip(out_vec.iter()).map(|(a, b)| a * b).sum();
        let norm_in: f32 = in_vec.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_out: f32 = out_vec.iter().map(|x| x * x).sum::<f32>().sqrt();
        let cos_sim = if norm_in > 1e-8 && norm_out > 1e-8 {
            dot / (norm_in * norm_out)
        } else {
            0.0
        };

        assert!(
            cos_sim > 0.0,
            "Single-column should preserve input direction via residual, got cos_sim={}",
            cos_sim
        );
    }

    #[test]
    fn test_config_default() {
        let config = SiblingContextConfig::default();
        assert_eq!(config.embed_dim, 128);
        assert_eq!(config.n_heads, 4);
        assert_eq!(config.n_layers, 2);
    }

    /// Latency benchmark: forward pass for 1, 5, 10, 20 columns.
    /// Asserts <5ms median for 20 columns on CPU (release mode).
    /// Marked `#[ignore]` because it requires `--release` for meaningful results.
    #[test]
    #[ignore]
    fn test_forward_latency() {
        use std::time::Instant;

        let model = create_random_model(128, 4, 2);

        for n_cols in [1, 5, 10, 20] {
            let input = random_embeddings(n_cols, 128);

            // Warmup
            for _ in 0..5 {
                let _ = model.forward(&input).unwrap();
            }

            // Benchmark
            let mut times = Vec::with_capacity(50);
            for _ in 0..50 {
                let start = Instant::now();
                let _ = model.forward(&input).unwrap();
                times.push(start.elapsed());
            }
            times.sort();
            let median = times[25];

            eprintln!("  N={:>2} columns: median={:>8.1?}", n_cols, median);

            // All sizes should be well under 5ms on modern CPU
            assert!(
                median.as_millis() < 5,
                "Forward pass for N={} took {:?} (>5ms)",
                n_cols,
                median
            );
        }
    }
}
