# ac-00 — B07 consumer-graph audit of the embed branch

## Key finding: Model2Vec serves TWO branches, not one

| Branch | Input | Dim | Source |
|---|---|---|---|
| **embed** | aggregation over **values** | 512 | Model2Vec |
| **header** | the **column header** string | 128 | Model2Vec |

The bet is the **embed (value)** branch. Decision for the first build: **swap embed only**,
leave the header branch on Model2Vec. This isolates the bet (the header branch is small and
working) at the cost of both gte-tiny (embed) and Model2Vec (header) being present at
inference. Swapping the header branch to gte-tiny too (removing Model2Vec entirely) is a
follow-up once the embed swap proves out.

## The feature binary is format-versioned — this makes the data change clean

`prepare_multibranch_data.py`: `MAGIC=b"FTMB"`, `VERSION_V2/V3/V4`. Header packs
`<HHHH char_dim embed_dim stats_dim header_dim>`, then per-row feature blocks. Introduce
**VERSION_V5** with `embed_dim=384` (gte-tiny) so the Rust reader distinguishes it from the
512-dim Model2Vec formats — no ambiguity, old models still load.

## The embed branch can use FROZEN gte-tiny

gte-tiny is a feature extractor here; the multi-branch learns the embed MLP `[300,300]` on
top. So no separate gte-tiny fine-tune is needed — compute **frozen** gte-tiny value
embeddings (384-dim, mean-pooled over the sampled values) as the embed slot, and the
multi-branch training adapts the rest. (Fine-tuning gte-tiny end-to-end through the
multi-branch is a later option.)

## Edit set per surface

| Surface | File | Change |
|---|---|---|
| Data | `scripts/prepare_multibranch_data.py` | embed slot → frozen gte-tiny (384); `EMBED_DIM=384`; `VERSION_V5` |
| Config | `crates/finetype-model/src/multi_branch/config.rs` | `embed_dim` 384 for the new model |
| Train read | `crates/finetype-train/src/multi_branch.rs`, `data.rs` | read V5 (384 embed) |
| Aggregation | `crates/finetype-model/src/embedding_aggregation.rs` | inference-only; touched in ac-04 |
| Inference | `crates/finetype-model/src/multi_branch/mod.rs`, `column/mod.rs`, `cli/src/main.rs` | ac-04: run gte-tiny for the embed slot instead of `extract_embedding_aggregation` |

## De-risking sequence (load-bearing — H03)
ac-01 (data) → ac-02 (model dim + train) → **ac-03 offline accuracy gate (GO/NO-GO)** →
only if GO: ac-04 (candle inference) → ac-05 (corpus gate + swap). Training reads precomputed
features, so ac-01→ac-03 never touch inference; the expensive candle path is built only after
the model is proven to beat v19 offline.
