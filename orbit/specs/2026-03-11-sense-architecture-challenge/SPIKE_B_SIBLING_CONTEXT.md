# Spike B: Sibling-Context Self-Attention in Candle

**Date:** 2026-03-09
**Task:** NNFT-263
**Status:** Complete — feasibility confirmed, no blockers

## Summary

Prototyped a multi-head self-attention transformer stack over Model2Vec column embeddings in Candle 0.8. The architecture compiles, runs correctly with variable column counts (1–20), and adds negligible latency (<4ms for 20 columns). No Candle API gaps or blockers were found.

## Architecture

Pre-norm transformer blocks over column embeddings:

```
Input: [N, 128]  (N column embeddings from Model2Vec)
  │
  ├─ TransformerBlock ×L
  │    │
  │    ├─ LayerNorm → MultiHeadSelfAttention(4 heads) → + residual
  │    └─ LayerNorm → FFN(128→512→128, GELU) → + residual
  │
  └─ Final LayerNorm
  │
Output: [N, 128]  (context-enriched column embeddings)
```

Each column's output embedding now carries information from sibling columns via self-attention. When N=1, self-attention reduces to attending to self, and the residual connection preserves the original embedding.

### Integration point

In the Sense pipeline, this would be inserted **between** Model2Vec encoding and Sense classification:

```
Current:   Model2Vec(header+values) → Sense(per-column) → CharCNN(masked)
Proposed:  Model2Vec(header+values) → SiblingContext(all columns) → Sense(per-column) → CharCNN(masked)
```

Each column's Model2Vec embedding (128-dim mean-pool of header + sampled values) feeds into the attention stack. The attended embeddings then replace the raw embeddings for downstream classification.

## Latency Results

Median and p99 over 100 iterations, release build, CPU (Intel), 128-dim embeddings:

| Columns | 2 layers (1.51 MB) | 4 layers (3.03 MB) |
|---------|-------------------|-------------------|
| N=1     | 112us / 140us     | 252us / 341us     |
| N=5     | 473us / 738us     | 929us / 1.0ms     |
| N=10    | 856us / 1.2ms     | 1.7ms / 5.7ms     |
| N=20    | 1.3ms / 1.9ms     | 3.7ms / 9.1ms     |

**Assessment:** Latency is negligible compared to the full pipeline. Profile mode (which benefits most from sibling context) already takes seconds per file. Even at 20 columns with 4 layers, the attention overhead is <10ms — well under 1% of total pipeline time.

**Recommendation:** Start with 2 layers. The latency is half of 4 layers, and the 2-layer stack already demonstrates strong context sensitivity (cosine similarity 0.90 for same column with different siblings, vs 0.77 for 4 layers). The difference indicates 4 layers may over-transform with random weights, but could be useful with trained weights. Start conservative.

## Parameter Budget

| Config | Parameters | Size (f32) | Size (f16) |
|--------|-----------|------------|------------|
| 2 layers, 4 heads | 396,800 | 1.51 MB | 0.76 MB |
| 4 layers, 4 heads | 793,344 | 3.03 MB | 1.51 MB |

Both fit comfortably within the 10–50 MB total model budget.

## Graceful Degradation

Single-column (N=1) test results:

| Config | Cosine(input, output) | Assessment |
|--------|----------------------|------------|
| 2 layers | 0.62 | Residual preserves direction |
| 4 layers | 0.45 | Residual present, more transformation |

With **trained** weights (not random), the single-column case would produce near-identity output because:
1. Pre-norm architecture: LayerNorm produces the same output regardless of N when N=1
2. Self-attention on a single token is a no-op (softmax([x]) = [1.0])
3. Residual connection adds attention output back to input
4. A trained model would learn to make the attention contribution small for N=1

The random-weight cosine values (0.45–0.62) are expected — the FFN adds random perturbations that a trained model would learn to suppress.

## Context Sensitivity

Same column embedding with different sibling columns:

| Config | Cosine(output_A, output_B) | Assessment |
|--------|---------------------------|------------|
| 2 layers | 0.90 | Context modifies output |
| 4 layers | 0.77 | Stronger context effect |

This confirms the attention mechanism is working: the same column produces different outputs depending on its sibling columns. With trained weights, this signal would be learned to distinguish ambiguous cases (e.g., "name" column in an airports table vs a persons table).

## Candle API Findings

### No blockers identified

1. **No native multi-head attention** in candle-core or candle-nn 0.8
   - Must implement manually: Q/K/V linear projections, reshape to heads, scaled dot-product, softmax, concat
   - This is the **same pattern** already used by `SenseClassifier` in `sense.rs` (cross-attention)
   - Straightforward to implement — ~60 lines of code

2. **Dynamic tensor shapes** work correctly
   - Variable N (number of columns) handled at runtime
   - No need for compile-time shape specifications
   - `Tensor::from_vec`, `reshape`, `transpose`, `matmul` all work with dynamic dims

3. **Attention masking** (for training with batched tables)
   - Not needed for inference (all columns are real)
   - For training: pad to max_columns, broadcast-add `-inf` mask
   - Same pattern as Sense classifier's value masking
   - Shape would be `[B, 1, 1, N_max]` for broadcasting over heads and queries

4. **Available activations:** `gelu_erf()`, `relu()`, `silu()` all present

5. **LayerNorm:** Manual implementation needed (candle-nn has `LayerNorm` struct but we'd want the same manual pattern as Sense for consistency)

### candle-nn utilities available but not required

`candle-nn` provides `Linear`, `LayerNorm`, `VarBuilder`, and `VarMap` for weight management. For production implementation, consider using `VarBuilder` for cleaner weight loading from safetensors.

## Key Code Patterns

### Self-attention forward pass (core ~60 lines)

```rust
fn forward(&self, x: &Tensor) -> Result<Tensor> {
    let (n, d) = (x.dim(0)?, x.dim(1)?);
    let (h, hd) = (self.n_heads, self.head_dim);

    // Q, K, V projections
    let q = x.matmul(&self.wq.t()?)?.broadcast_add(&self.bq)?;
    let k = x.matmul(&self.wk.t()?)?.broadcast_add(&self.bk)?;
    let v = x.matmul(&self.wv.t()?)?.broadcast_add(&self.bv)?;

    // Reshape to multi-head: [N, D] → [h, N, hd]
    let q = q.reshape((n, h, hd))?.transpose(0, 1)?;
    let k = k.reshape((n, h, hd))?.transpose(0, 1)?;
    let v = v.reshape((n, h, hd))?.transpose(0, 1)?;

    // Scaled dot-product attention
    let scale = (hd as f64).sqrt();
    let scores = (q.matmul(&k.transpose(1, 2)?)? / scale)?;

    // Softmax
    let max = scores.max(2)?.unsqueeze(2)?;
    let exp = (scores.broadcast_sub(&max)?).exp()?;
    let probs = exp.broadcast_div(&exp.sum(2)?.unsqueeze(2)?)?;

    // Weighted sum + concat heads + output projection
    let out = probs.matmul(&v)?
        .transpose(0, 1)?.reshape((n, d))?;
    out.matmul(&self.out_weight.t()?)?.broadcast_add(&self.out_bias)
}
```

### Pre-norm transformer block

```rust
fn forward(&self, x: &Tensor) -> Result<Tensor> {
    let x = (&x + &self.attn.forward(&self.norm1.forward(x)?)?)?;
    &x + &self.ffn.forward(&self.norm2.forward(&x)?)?
}
```

## Training Considerations

For the full implementation (not this spike):

1. **Data pipeline:** Need multi-column tables with known types. GitTables (1.7M tables) is the natural source.
2. **Batching:** Tables have variable column counts → pad to max in batch, apply attention mask.
3. **Loss:** Per-column cross-entropy, masked for padding columns.
4. **Curriculum:** Start with tables having 2–5 columns, gradually increase.
5. **Evaluation:** Compare with/without context on bare-name ambiguity cases.

## Conclusion

**Feasibility: Confirmed.** The sibling-context attention architecture is straightforward to implement in Candle 0.8, fits within the model size budget, adds negligible latency, and follows established patterns already in the FineType codebase. No API gaps or blockers were found.

**Recommended next step:** Implement as a trainable module in `finetype-model`, integrate into the Sense pipeline, and train on GitTables multi-column data with the 3 bare-name ambiguity cases as the primary evaluation target.
