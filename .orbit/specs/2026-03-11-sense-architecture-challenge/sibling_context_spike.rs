//! Spike B: Sibling-Context Self-Attention Prototype (NNFT-263)
//!
//! Validates feasibility of cross-column self-attention over Model2Vec column
//! embeddings in Candle 0.8. Measures latency for 1–20 columns, verifies
//! single-column graceful degradation (residual → near-identity).
//!
//! Architecture per transformer block:
//!   x → LayerNorm → MultiHeadSelfAttention → + residual → LayerNorm → FFN → + residual
//!
//! Run: cargo run --release -p sibling-context-spike

use candle_core::{DType, Device, Tensor};
use rand::Rng;
use std::time::Instant;

// ═══════════════════════════════════════════════════════════════════════════════
// Multi-head self-attention (manual implementation — Candle has no native MHA)
// ═══════════════════════════════════════════════════════════════════════════════

/// Weights for a single multi-head self-attention layer.
struct MultiHeadSelfAttention {
    wq: Tensor, // [D, D]
    bq: Tensor, // [D]
    wk: Tensor, // [D, D]
    bk: Tensor, // [D]
    wv: Tensor, // [D, D]
    bv: Tensor, // [D]
    out_weight: Tensor, // [D, D]
    out_bias: Tensor,   // [D]
    n_heads: usize,
    head_dim: usize,
}

impl MultiHeadSelfAttention {
    /// Initialise with small random weights (Xavier-like).
    fn new(embed_dim: usize, n_heads: usize, device: &Device) -> Self {
        assert!(embed_dim % n_heads == 0);
        let head_dim = embed_dim / n_heads;
        let scale = (1.0 / embed_dim as f64).sqrt();

        let rand_matrix = |rows: usize, cols: usize| -> Tensor {
            let mut rng = rand::thread_rng();
            let data: Vec<f32> = (0..rows * cols)
                .map(|_| rng.gen_range(-scale as f32..scale as f32))
                .collect();
            Tensor::from_vec(data, (rows, cols), device).unwrap()
        };
        let zeros = |n: usize| -> Tensor {
            Tensor::zeros(n, DType::F32, device).unwrap()
        };

        Self {
            wq: rand_matrix(embed_dim, embed_dim),
            bq: zeros(embed_dim),
            wk: rand_matrix(embed_dim, embed_dim),
            bk: zeros(embed_dim),
            wv: rand_matrix(embed_dim, embed_dim),
            bv: zeros(embed_dim),
            out_weight: rand_matrix(embed_dim, embed_dim),
            out_bias: zeros(embed_dim),
            n_heads,
            head_dim,
        }
    }

    /// Forward pass: [N, D] → [N, D]
    ///
    /// Self-attention: each column embedding attends to all column embeddings.
    /// No padding mask needed — all positions are real columns.
    fn forward(&self, x: &Tensor) -> candle_core::Result<Tensor> {
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
        attn_out
            .matmul(&self.out_weight.t()?)?
            .broadcast_add(&self.out_bias)
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// LayerNorm (manual — matches Sense implementation)
// ═══════════════════════════════════════════════════════════════════════════════

struct LayerNorm {
    weight: Tensor, // [D]
    bias: Tensor,   // [D]
}

impl LayerNorm {
    fn new(dim: usize, device: &Device) -> Self {
        Self {
            weight: Tensor::ones(dim, DType::F32, device).unwrap(),
            bias: Tensor::zeros(dim, DType::F32, device).unwrap(),
        }
    }

    /// Forward: per-row normalisation. Input [N, D] → [N, D]
    fn forward(&self, x: &Tensor) -> candle_core::Result<Tensor> {
        let eps = 1e-5_f64;
        let n = x.dim(0)?;
        let d = x.dim(1)?;

        // Compute per-row mean and variance
        let mean = (x.sum(1)? / d as f64)?; // [N]
        let mean = mean.unsqueeze(1)?; // [N, 1]
        let diff = x.broadcast_sub(&mean)?;
        let var = ((&diff * &diff)?.sum(1)? / d as f64)?; // [N]
        let std = (var + eps)?.sqrt()?.unsqueeze(1)?; // [N, 1]
        let normed = diff.broadcast_div(&std)?;

        // Scale and shift
        let _ = n; // suppress unused warning
        normed
            .broadcast_mul(&self.weight)?
            .broadcast_add(&self.bias)
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Feed-forward network
// ═══════════════════════════════════════════════════════════════════════════════

struct FeedForward {
    w1: Tensor, // [4D, D]
    b1: Tensor, // [4D]
    w2: Tensor, // [D, 4D]
    b2: Tensor, // [D]
}

impl FeedForward {
    fn new(dim: usize, device: &Device) -> Self {
        let ff_dim = dim * 4;
        let scale = (1.0 / dim as f64).sqrt();
        let mut rng = rand::thread_rng();

        let mut rand_matrix = |rows: usize, cols: usize| -> Tensor {
            let data: Vec<f32> = (0..rows * cols)
                .map(|_| rng.gen_range(-scale as f32..scale as f32))
                .collect();
            Tensor::from_vec(data, (rows, cols), device).unwrap()
        };
        let zeros = |n: usize| -> Tensor {
            Tensor::zeros(n, DType::F32, device).unwrap()
        };

        Self {
            w1: rand_matrix(ff_dim, dim),
            b1: zeros(ff_dim),
            w2: rand_matrix(dim, ff_dim),
            b2: zeros(dim),
        }
    }

    /// Forward: Linear(D, 4D) → GELU → Linear(4D, D). Input [N, D] → [N, D]
    fn forward(&self, x: &Tensor) -> candle_core::Result<Tensor> {
        let h = x.matmul(&self.w1.t()?)?.broadcast_add(&self.b1)?;
        let h = h.gelu_erf()?;
        h.matmul(&self.w2.t()?)?.broadcast_add(&self.b2)
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Transformer block: LN → MHA → residual → LN → FFN → residual
// ═══════════════════════════════════════════════════════════════════════════════

struct TransformerBlock {
    norm1: LayerNorm,
    attn: MultiHeadSelfAttention,
    norm2: LayerNorm,
    ffn: FeedForward,
}

impl TransformerBlock {
    fn new(embed_dim: usize, n_heads: usize, device: &Device) -> Self {
        Self {
            norm1: LayerNorm::new(embed_dim, device),
            attn: MultiHeadSelfAttention::new(embed_dim, n_heads, device),
            norm2: LayerNorm::new(embed_dim, device),
            ffn: FeedForward::new(embed_dim, device),
        }
    }

    /// Forward: Pre-norm transformer block. Input [N, D] → [N, D]
    fn forward(&self, x: &Tensor) -> candle_core::Result<Tensor> {
        // Self-attention with residual
        let normed = self.norm1.forward(x)?;
        let attn_out = self.attn.forward(&normed)?;
        let x = (x + &attn_out)?;

        // FFN with residual
        let normed = self.norm2.forward(&x)?;
        let ffn_out = self.ffn.forward(&normed)?;
        &x + &ffn_out
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Sibling Context Attention: stack of transformer blocks
// ═══════════════════════════════════════════════════════════════════════════════

struct SiblingContextAttention {
    blocks: Vec<TransformerBlock>,
    final_norm: LayerNorm,
}

impl SiblingContextAttention {
    fn new(embed_dim: usize, n_heads: usize, n_layers: usize, device: &Device) -> Self {
        let blocks = (0..n_layers)
            .map(|_| TransformerBlock::new(embed_dim, n_heads, device))
            .collect();
        Self {
            blocks,
            final_norm: LayerNorm::new(embed_dim, device),
        }
    }

    /// Forward: stack of transformer blocks + final norm. Input [N, D] → [N, D]
    fn forward(&self, x: &Tensor) -> candle_core::Result<Tensor> {
        let mut out = x.clone();
        for block in &self.blocks {
            out = block.forward(&out)?;
        }
        self.final_norm.forward(&out)
    }

    /// Count total parameters.
    fn param_count(&self, embed_dim: usize) -> usize {
        let d = embed_dim;
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
}

// ═══════════════════════════════════════════════════════════════════════════════
// Main: benchmarks and validation
// ═══════════════════════════════════════════════════════════════════════════════

fn make_random_embeddings(n_columns: usize, embed_dim: usize, device: &Device) -> Tensor {
    let mut rng = rand::thread_rng();
    let data: Vec<f32> = (0..n_columns * embed_dim)
        .map(|_| rng.gen_range(-1.0f32..1.0f32))
        .collect();
    Tensor::from_vec(data, (n_columns, embed_dim), device).unwrap()
}

/// Cosine similarity between two 1-D tensors.
fn cosine_similarity(a: &Tensor, b: &Tensor) -> f32 {
    let a_vec: Vec<f32> = a.to_vec1().unwrap();
    let b_vec: Vec<f32> = b.to_vec1().unwrap();
    let dot: f32 = a_vec.iter().zip(b_vec.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a_vec.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b_vec.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a < 1e-8 || norm_b < 1e-8 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}

fn main() {
    let device = Device::Cpu;
    let embed_dim = 128; // Model2Vec potion-base-4M dimension
    let n_heads = 4;     // 128 / 4 = 32 per head

    println!("═══════════════════════════════════════════════════════════");
    println!("Spike B: Sibling-Context Self-Attention Prototype");
    println!("═══════════════════════════════════════════════════════════");
    println!();

    // ── Test configurations: 2 and 4 layers ──
    for n_layers in [2, 4] {
        let model = SiblingContextAttention::new(embed_dim, n_heads, n_layers, &device);
        let param_count = model.param_count(embed_dim);
        let param_mb = param_count as f64 * 4.0 / 1024.0 / 1024.0; // f32 = 4 bytes

        println!("── Configuration: {n_layers} layers, {n_heads} heads, D={embed_dim} ──");
        println!("   Parameters: {param_count} ({param_mb:.2} MB as f32)");
        println!();

        // ── Latency benchmark ──
        println!("   Latency (100 iterations, median):");
        for n_cols in [1, 5, 10, 20] {
            let input = make_random_embeddings(n_cols, embed_dim, &device);

            // Warmup
            for _ in 0..5 {
                let _ = model.forward(&input).unwrap();
            }

            // Benchmark
            let mut times = Vec::with_capacity(100);
            for _ in 0..100 {
                let start = Instant::now();
                let _ = model.forward(&input).unwrap();
                times.push(start.elapsed());
            }
            times.sort();
            let median = times[50];
            let p99 = times[99];

            println!(
                "     N={n_cols:>2} columns: median={median:>8.1?}, p99={p99:>8.1?}"
            );
        }
        println!();

        // ── Single-column degradation test ──
        println!("   Single-column degradation (residual identity test):");
        let single = make_random_embeddings(1, embed_dim, &device);
        let output = model.forward(&single).unwrap();
        let input_row = single.get(0).unwrap();
        let output_row = output.get(0).unwrap();
        let cos_sim = cosine_similarity(&input_row, &output_row);

        // With random (untrained) weights: residual connection means output ≈ input + noise
        // Cosine similarity should be positive (residual dominates random noise for small init)
        println!("     Cosine similarity (input vs output): {cos_sim:.4}");
        println!(
            "     Assessment: {}",
            if cos_sim > 0.5 {
                "GOOD — residual preserves input direction"
            } else if cos_sim > 0.0 {
                "OK — residual present but attention adds signal"
            } else {
                "WARNING — residual may not dominate (expected with random weights)"
            }
        );
        println!();

        // ── Multi-column context test ──
        // The same column embedding should produce DIFFERENT outputs depending on siblings
        println!("   Context sensitivity test:");
        let base_col = make_random_embeddings(1, embed_dim, &device); // [1, D]

        // Context A: base_col with 4 random siblings
        let siblings_a = make_random_embeddings(4, embed_dim, &device);
        let input_a = Tensor::cat(&[&base_col, &siblings_a], 0).unwrap(); // [5, D]
        let output_a = model.forward(&input_a).unwrap();
        let out_a_row0 = output_a.get(0).unwrap();

        // Context B: same base_col with 4 DIFFERENT random siblings
        let siblings_b = make_random_embeddings(4, embed_dim, &device);
        let input_b = Tensor::cat(&[&base_col, &siblings_b], 0).unwrap(); // [5, D]
        let output_b = model.forward(&input_b).unwrap();
        let out_b_row0 = output_b.get(0).unwrap();

        let ctx_cos = cosine_similarity(&out_a_row0, &out_b_row0);
        println!("     Same column, different siblings — cosine: {ctx_cos:.4}");
        println!(
            "     Assessment: {}",
            if ctx_cos < 0.95 {
                "GOOD — sibling context changes output (attention working)"
            } else {
                "NEUTRAL — outputs very similar (attention may not be learning much with random weights)"
            }
        );
        println!();

        // ── Shape preservation test ──
        for n_cols in [1, 5, 10, 20] {
            let input = make_random_embeddings(n_cols, embed_dim, &device);
            let output = model.forward(&input).unwrap();
            assert_eq!(
                output.dims(),
                &[n_cols, embed_dim],
                "Shape mismatch for N={n_cols}"
            );
        }
        println!("   Shape preservation: PASS (all N produce [N, {embed_dim}])");
        println!();
    }

    // ── Candle API observations ──
    println!("═══════════════════════════════════════════════════════════");
    println!("Candle API Observations:");
    println!("═══════════════════════════════════════════════════════════");
    println!();
    println!("1. No native multi-head attention in candle-core 0.8");
    println!("   → Must implement manually (Q/K/V projections, reshape, softmax)");
    println!("   → Same pattern used by Sense classifier (cross-attention)");
    println!("   → candle-nn has Linear/LayerNorm but no MHA module");
    println!();
    println!("2. Dynamic batch sizes (variable N) work fine");
    println!("   → Tensor shapes are runtime-determined");
    println!("   → No padding needed when all positions are real columns");
    println!("   → For batch-of-tables (training), would need padding + mask");
    println!();
    println!("3. Attention masking:");
    println!("   → Not needed for inference (all columns are real)");
    println!("   → For training with batched tables of different widths:");
    println!("     pad to max_columns, apply mask tensor [B, 1, 1, N] of -inf");
    println!("   → Same pattern as Sense classifier padding mask");
    println!();
    println!("4. GELU activation available via Tensor::gelu_erf()");
    println!("   → No need for custom implementation");
    println!();
    println!("5. No blockers identified for full implementation.");
    println!("   → Architecture is a direct extension of existing Sense patterns");
    println!("   → All required ops (matmul, softmax, layernorm, residual) verified");
}
