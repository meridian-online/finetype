# ac-00 — Dual-encoder native inference: B07 audit + design

**Date:** 2026-06-23
**Spec:** 2026-06-23-dual-encoder-native-ship-potion8m
**Status:** design complete; ready for ac-01 implementation

## The headline

potion-8M needs the binary to encode column *values* with a 256-dim potion-8M
model while everything else (header branch, Sense/entity/semantic/sibling
classifiers) keeps the 128-dim potion-4M. The whole dual-encoder change is
**localised to one struct** — `MultiBranchClassifier`. It already owns its own
`model2vec` resource and is the only thing that calls `extract_embedding_aggregation`
(the value branch). Nothing else in the codebase mixes the two encoders, so the
blast radius is small: a second optional resource on that struct, a dynamic
aggregation function, and one new config field. v19 and m2v-244 load unchanged
because the config field defaults to absent.

## What the value encoder is, concretely

- **potion-8M** (staged `models/m2v8m/`): `embeddings [29528, 256]` F32 + its own
  `tokenizer.json`. Value-agg = mean‖var‖min‖max over 256 dims = **1024**.
- **potion-4M** (`models/model2vec/`): `embeddings [29528, 128]` F16. Value-agg = 512.
- The trained m2v8m configs already declare `embed_dim: 1024` (the embed-branch
  input). So **`config.embed_dim == 4 × value-encoder-dim`** holds for both
  families (v19: 4×128=512; m2v8m: 4×256=1024). This is the invariant the
  dim-handling rests on.

## Consumer graph — every reader of `models/model2vec` and `EMBED_DIM`/`EMBED_AGG_DIM`

Codegraph + ripgrep enumeration. Split by **runtime-inference (load-bearing)** vs
**train/dev tooling (not shipped, out of scope)**.

### A. Runtime value-aggregation branch — THE dual-encoder surface

`MultiBranchClassifier` (`crates/finetype-model/src/multi_branch/mod.rs`) holds one
`model2vec: Model2VecResources` (field, line 72) used for **two different things**:

1. **Value aggregation** — `extract_embedding_aggregation(&value_refs, &self.model2vec)`
   at four call sites: `classify_column` (340), `classify_column_with_enriched_header`
   (454), `classify_column_topk` (691), `column_logits` (805). Each then builds the
   embed tensor `(1, EMBED_AGG_DIM)` and fallback `[0.0f32; EMBED_AGG_DIM]`.
   **→ this is what must switch to potion-8M.**
2. **Header encoding** — `self.model2vec.encode_one(header)` → 128-dim, at
   `classify_column` (360), `classify_column_topk` (709), `column_logits` (823).
   **→ this must STAY potion-4M.**

Live dispatch into these (the real `finetype profile` path):
`ColumnClassifier` (`column/mod.rs`) → `with_multi_branch` (712) → `mb.classify_column`
(2402, no-sibling) and `mb.classify_column_with_enriched_header` (2517, sibling
context). `fusion.rs:131` calls `mb.column_logits` (B3 fusion, not shipped default
but compiled). `classify_column_topk` is the amvg diagnostic example.

`MultiBranchClassifier::load` (89) → `load_model2vec(dir)` (293): tries
`<model_dir>/model2vec/` then `models/model2vec/`. `from_bytes` (116) takes one
`model2vec` (release-embedded path, `load_multi_branch_classifier` main.rs:2143).

### B. Header / Sense / entity / semantic / sibling — STAY potion-4M (untouched)

A separate resource, `ColumnClassifier.model2vec` (`column/mod.rs:557`), set via
`set_sense` (664) / `set_model2vec` (698), wired in the CLI by
`wire_model2vec_and_siblings` (main.rs:2442) and `wire_sense` (2435) — both load
`models/model2vec` (potion-4M). Consumers: `SenseClassifier` (sense.rs),
`EntityClassifier` (entity.rs), `SemanticHintClassifier` (semantic.rs),
sibling-context header enrichment (column/mod.rs:1115, 1510). **None of these
touch value aggregation; all keep potion-4M. No change.**

### C. Dev / train tooling — out of scope (not the shipped binary)

- `cmd_extract_features` (main.rs:1456) — `extract-features` debug subcommand,
  dumps 512-dim features to stdout. Keep on the const-array path; not the live
  pipeline.
- `finetype-train/*` (data.rs, entity.rs, sense.rs, sibling_*, prepare_model2vec,
  predict_multibranch) — training-time encoding, 128-dim potion-4M, separate
  `EMBED_DIM` consts. Untouched.
- `finetype-candle-spike`, `sense_train`, examples — not shipped.

### D. Distribution surfaces — note for ac-04 (not ac-01)

- `crates/finetype-duckdb/src/lib.rs:48-112` fetches `model2vec/*` from HF and calls
  `MultiBranchClassifier::load`. For potion-8M via DuckDB the value encoder must
  also ship in the HF repo (a second subdir). ac-01 only needs the dev/disk path.
- `crates/finetype-cli/build.rs:430` embeds `models/model2vec` for release binaries
  (`embed-models`). Shipping potion-8M as the release default (ac-04) needs the
  value encoder embedded too + `from_bytes` extended. **Deferred to ac-04**; the
  gating path (ac-01–03) runs `FINETYPE_MODEL=models/m2v8m-s44` from disk.

## The design

### D1. Second resource on `MultiBranchClassifier`

Add `value_model2vec: Option<Model2VecResources>`.

- `None` (v19, m2v-244, every existing model) → value-agg uses the shared
  `model2vec` (potion-4M). Bit-identical to today.
- `Some(potion-8M)` → value-agg uses it; header still uses `model2vec`.

Accessor: `fn value_resources(&self) -> &Model2VecResources { self.value_model2vec.as_ref().unwrap_or(&self.model2vec) }`.

### D2. Config-schema change

In `multi_branch/config.rs`, add to `MultiBranchConfig`:

```rust
/// Optional path to a SECOND Model2Vec encoder used ONLY for the value-
/// aggregation branch (dual-encoder, e.g. potion-8M). Resolved relative to the
/// model dir first, then as a workspace/absolute path. When absent, the value
/// branch shares the header encoder (potion-4M) — backward compatible.
#[serde(default)]
pub value_embed_model: Option<String>,
```

`#[serde(default)]` → every existing config.json deserialises with `None`. The
m2v8m configs gain `"value_embed_model": "models/m2v8m"`.

### D3. Loading

In `MultiBranchClassifier::load` / `from_bytes`, after loading `model2vec`:

```rust
let value_model2vec = match &config.value_embed_model {
    Some(rel) => {
        let local = model_dir.join(rel);          // <model_dir>/<rel>
        let dir = if local.join("model.safetensors").exists() { local }
                  else { PathBuf::from(rel) };      // workspace/absolute
        Some(Model2VecResources::load(&dir)?)
    }
    None => None,
};
```

`load` has `model_dir`; `from_bytes` (release) does not — for ac-01 the disk
`load` path is sufficient (gating runs from disk). `from_bytes` passes `None` for
now; ac-04 extends it to accept embedded value-encoder bytes.

### D4. Dim handling — config-driven, NOT a new const

The current `extract_embedding_aggregation` is hardwired to 128/512 via
`const EMBED_DIM = 128` / `EMBED_AGG_DIM = 512` and fixed-size arrays; worse, it
loops `embed_dim.min(EMBED_DIM)`, so a 256-dim encoder would silently truncate to
128 and emit a 512-vec. So a const bump is wrong — it must be **dynamic**.

Add a sibling function (keep the const one for the dev `extract-features` tool and
existing tests — zero churn there):

```rust
/// Dynamic value-aggregation: returns 4 × embed_dim floats (mean‖var‖min‖max).
/// Handles any encoder width (128→512, 256→1024).
pub fn extract_embedding_aggregation_dyn(
    values: &[&str], resources: &Model2VecResources,
) -> Option<Vec<f32>>
```

Same maths as the const version, sized to `resources.embed_dim()`. In the four
`MultiBranchClassifier` methods:

- `extract_embedding_aggregation(&value_refs, &self.model2vec).unwrap_or([0.0f32; EMBED_AGG_DIM])`
  → `extract_embedding_aggregation_dyn(&value_refs, self.value_resources()).unwrap_or_else(|| vec![0.0f32; self.config.embed_dim])`
- tensor shape `(1, EMBED_AGG_DIM)` → `(1, self.config.embed_dim)`.

`config.embed_dim` is the single source of truth for the embed-branch input width
and already equals `4 × value-encoder-dim` for both families (verified). The embed
branch itself is already built from `config.embed_dim` (mod.rs:163), so the branch
weights and the feature vector agree by construction.

### Edit set (ac-01)

1. `multi_branch/config.rs` — add `value_embed_model: Option<String>` field.
2. `embedding_aggregation.rs` — add `extract_embedding_aggregation_dyn` (+ unit
   test for a 256-dim synthetic resource → 1024-len output); export in `lib.rs`.
3. `multi_branch/mod.rs` — add `value_model2vec` field; load it in `load`/`from_bytes`;
   add `value_resources()`; switch the 4 value-agg call sites to the dyn fn +
   `config.embed_dim` tensor shape + `vec!` fallback. Header encoding unchanged.
4. `models/m2v8m-s{42,43,44}/config.json` — add `"value_embed_model": "models/m2v8m"`
   (or co-locate potion-8M at `<model_dir>/value_model2vec/`).
5. Tests: a load+classify smoke on `models/m2v8m-s44` (256-dim value agg path) and
   a regression assert that v19 still produces a 512 embed tensor (None branch).

### Why this is safe (B07)

- One struct, one new optional field, one new function. Existing models hit the
  `None` branch → behaviour is bit-identical (the dyn fn over potion-4M yields the
  same 512 vector as the const fn).
- Header / Sense / entity / semantic / sibling encoders are a *different* resource
  and are not touched — potion-8M never reaches them.
- `config.embed_dim` already gates the embed branch, so feature width and weight
  width cannot silently diverge — a wrong value encoder fails loudly at the tensor
  matmul, not silently.
- Load-bearing path → H02 verify (native gold composed must reproduce offline
  0.794 / Sense ~0.522, the ac-01 halt) + `clippy -D warnings` + `cargo test`.

## Halts carried into ac-01

- Native potion-8M gold composed must reproduce offline **0.794** (Sense ~**0.522**).
  Divergence → STOP and diagnose (native path differs from offline).
- v19 must profile identically (None branch regression check).
