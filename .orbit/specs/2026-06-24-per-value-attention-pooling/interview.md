# Design interview — per-value cross-value attention pooling (spec 2026-06-24-per-value-attention-pooling, choice 0106)

Date: 2026-06-24. Resolves ac-0. Driven as a design session; the author delegated
("run the design"). Recommendations made with rationale; the one genuine fork
(D1, pool query) is flagged for override.

## The shape we're building

Replace the value branch's fixed mean/var/min/max blender with:

```
per-value potion-8M embeddings [N=50, 256] + valid-mask
   │
   ├─▶ cross-value self-attention (1 layer, pre-norm) ─▶ [50, 256] contextualised
   │
   └─▶ PMA pool (learned seed query [k, 256]) ──────────▶ [k, 256] ─ flatten ─▶ [k·256]
                                                                                  │
   mean/var/min/max blender [1024] ───────────────────────────────────── concat ─┤
                                                                                  ▼
                                                    value-branch MLP (embed_hidden) ─▶ trunk
```

Strictly additive over today's branch: if attention learns nothing, the blender
half still carries the signal it carries now — so the change cannot regress below
the current value-branch baseline. That property is what makes the gold-parity
gate (ac-5) safe to bet a retrain on.

## Decisions

### D1 — Pool query: **learned seed (PMA), NOT header-derived** *(fork — author may override)*
The in-repo prior art (`sense.rs`) uses the header embedding as the attention
query. Rejected as the default here for two reasons: (1) value-based inference is
deliberately robust to missing/adversarial headers (decision 0048 deprecated
header hints), and a header-conditioned pool degrades exactly when the header is
junk — the case value inference exists to handle; (2) the header signal already
has its own dedicated branch in the trunk, so a header-query pool double-counts it.
A learned seed query (Pooling-by-Multihead-Attention, a small set of trainable
query slots) is header-independent and learns "which values are most
type-discriminative" from value content alone.
**Override note:** if the author wants value salience tied to the column name, flip
`pool_query_type` to `header`. Left as a cheap post-hoc A/B in ac-4.

### D2 — Attention shape: 1 self-attn layer + 1 PMA layer, 4 heads, 256-dim, pre-norm, GELU
Shallow on purpose. 50 tokens of 256-dim need no depth; one cross-value layer adds
the context the blender lacks, and deeper risks overfitting at our training scale.
4 heads (64-dim each, standard). 256-dim matches potion-8M's native width — no
projection, so no information loss before pooling. Pre-norm + GELU mirrors the
config's GELU win (+1.3pp, "autoresearch winner") and `sibling_context.rs`. FFN
hidden 512 (2×). Attention dropout 0.1 (lower than the 0.35 trunk dropout — the
trunk dropout still applies downstream).

### D3 — N values + masking: **N=50, pad-to-fixed + attention mask**
50 is the historical Sense sample width; halves the sequence cost vs 100 and a
column is well-summarised by 50 samples. Short columns pad to 50 with a boolean
mask that zeroes pad positions in both self-attention and PMA — single-value
columns must still classify (the blender handles this today via valid-row
filtering; the mask is the attention equivalent). Sampling matches the current
pipeline (no dedup in v1; first-50-distinct is a noted later A/B — the stats branch
already owns cardinality, so the value pool should see a representative sample, not
a deduped one).

### D4 — Keep the mean/var/min/max concat: **yes, as auxiliary**
Free signal, collapse-proof floor, and min/max capture per-dim extremes a soft
attention pool can wash out. Concatenate `[PMA output (k·256) ‖ blender (1024)]`
into the value-branch MLP. With k=4 → 1024 + 1024 = **2048-dim** value-branch
input. This is what makes the change strictly additive (D-shape note above).

### D5 — One shared Candle module (train == infer), NOT a dual implementation
`finetype-train` depends on `finetype-model`, so the attention-pool forward lives
**once** in `finetype-model` (new `value_attention.rs`) and is used by both the
training loop (VarMap-backed, trainable) and inference (safetensors-loaded,
frozen). Candle's `forward` is identical for both. This deliberately avoids
`sense.rs`'s Python-trained-vs-Rust-trained dual format and kills the parity-bug
class outright — the parity check in ac-3 becomes a numerical confirmation of one
codepath, not a reconciliation of two.

### D6 — Config + tag
`models/m2v8m-attn-244-config.json` (committed this session), based on
`m2v8m-244-config.json` with a `value_attention` block and `embed_dim` widened to
2048 (the concatenated value-branch input). potion-8M stays the value encoder
(`value_embed_model` unchanged). Tag `m2v8m-attn`; 3 seeds (42/43/44) per the
overnight recipe. potion-32M is a cheap A/B *after* the harness exists, not now.

## Open question deferred to implementation (not blocking ac-0)
- PMA seed slot count k: pinned to 4 for v1 (4·256=1024 matches the blender half,
  clean concat). Sweeping k∈{1,4,8} is an ac-4 sub-experiment if the v1 result is
  borderline.

## What this design does NOT touch (scope guard)
Encoder choice (potion-8M fixed), the char/stats/header/validation branches, the
trunk, the head, and the entire Sharpen/rule layer. Composed is rule-bound
(choice 0106 honest-scope); the rule-layer bet is separate and gated on ac-4's
pass-through reading. One variable: the value-branch pooling.
