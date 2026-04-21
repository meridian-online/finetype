# Phase 2: Sense → Sharpen Integration Design

**Task:** NNFT-164
**Date:** 2026-03-01
**Author:** @nightingale
**Depends on:** Phase 0 (NNFT-162), Phase 1 (NNFT-163), decision-004, decision-005

## Overview

This document specifies how the Sense model (Architecture A, NNFT-163) integrates into
FineType's column classification pipeline. It makes Phase 3 (Rust implementation)
mechanical — every interface, data flow, and design choice is recorded here.

**Key design principle:** The Sense model is an *additive* pre-routing stage. When absent,
the pipeline falls back to the exact current behaviour. No existing tests should break.

---

## A. New Pipeline Flow

### Current Pipeline

```
sample(100) → CharCNN batch → remap → vote → 18 rules → entity demotion → locale → header hints
```

### New Pipeline (Sense + Sharpen)

```
sample(100) → M2V encode header + first 50 values
           → Sense classify(header_emb, value_embs) → broad_category, entity_subtype
           → flat CharCNN batch(100) → remap → vote (masked to category) → ~12 rules → locale
```

### Fallback (Sense model absent)

```
sample(100) → CharCNN batch → remap → vote → 18 rules → entity demotion → locale → header hints
```

Identical to current pipeline. Ensured by `sense: Option<SenseClassifier>` — when `None`,
`classify_column` and `classify_column_with_header` behave exactly as today.

### Step-by-Step Detail

1. **Sample** 100 values (unchanged from current `ColumnConfig::sample_size`).

2. **Model2Vec encode** the column header (if present) and the first 50 values.
   Uses the shared `Model2VecResources` (tokenizer + embedding matrix). Produces:
   - `header_emb: Option<[128]>` — L2-normalised header embedding
   - `value_embs: [[128]; N]` — per-value embeddings (N ≤ 50)
   - `value_mask: [bool; N]` — true for real values

3. **Sense classify** with `SenseClassifier::classify()`:
   - Input: header_emb (or learned default query), value_embs, value_mask
   - Output: `SenseResult { broad_category: BroadCategory, entity_subtype: Option<EntitySubtype>, broad_confidence: f32, entity_confidence: f32 }`
   - `BroadCategory` enum: `Entity`, `Format`, `Temporal`, `Numeric`, `Geographic`, `Text`
   - `EntitySubtype` enum: `Person`, `Place`, `Organization`, `CreativeWork`

4. **CharCNN batch inference** on all 100 sampled values (unchanged).

5. **Remap** collapsed labels via `remap_collapsed_label()` (unchanged).

6. **Masked vote aggregation**: Only count votes for types eligible under the Sense
   category (see Section B). Types outside the Sense category are zeroed. If all votes
   are masked out (Sense routed to the wrong category), fall back to unmasked vote
   aggregation — this is the safety valve.

7. **Disambiguation rules** (~12 rules, reduced from 17+; see Section C).

8. **Post-hoc locale detection** (unchanged).

9. **Entity handling**: When Sense predicts `Entity` with subtype:
   - `Person` + majority vote `full_name` → keep as `full_name` (no demotion needed)
   - `Person` + majority vote not `full_name` → keep majority vote within entity-eligible types
   - Non-person subtype + majority vote `full_name` → demote to `entity_name` (replaces Rule 18 + EntityClassifier)
   - Non-person subtype + other entity-eligible type → keep majority vote

10. **Header hints**: Eliminated as a separate system. The column header is already a
    Sense input (step 2). The semantic hint classifier, hardcoded `header_hint()` function,
    geography protection, measurement disambiguation, and entity demotion guard are all
    subsumed. The `classify_column_with_header()` method becomes a thin wrapper that passes
    the header to Sense, with no separate header-hint logic when Sense is active.

### Performance Budget

| Stage | Time | Notes |
|-------|------|-------|
| M2V encode (50 values + header) | ~2.5ms | Shared tokenizer, batch index_select |
| Sense forward pass | ~1.5ms | Cross-attention, constant time |
| Flat CharCNN (100 values) | ~20ms | Single model, no tiered routing |
| Vote + rules + locale | <1ms | Unchanged |
| **Total** | **~25ms** | Well under 50ms budget |

Current pipeline: ~73ms (tiered CharCNN). **Net speedup: ~2.9x.**

The flat CharCNN replaces the tiered 34-model cascade. Sense handles routing;
the flat model handles fine-grained type discrimination within the routed category.

---

## B. Sense Category → FineType Type Mapping

Every type is assigned to exactly one primary Sense category. The `LabelCategoryMap`
provides `category_for(label) -> BroadCategory` and `eligible_labels(category) -> &[&str]`.

### temporal (46 types)

All `datetime.*` types:

| Types | Count |
|-------|-------|
| `datetime.component.*` (century, day_of_month, day_of_week, month_name, periodicity, year) | 6 |
| `datetime.date.*` (abbreviated_month, compact_dmy/mdy/ymd, eu_dot, eu_slash, iso, iso_week, julian, long_full_month, ordinal, short_dmy/mdy/ymd, us_slash, weekday_abbreviated_month, weekday_full_month) | 17 |
| `datetime.duration.iso_8601` | 1 |
| `datetime.epoch.*` (unix_microseconds, unix_milliseconds, unix_seconds) | 3 |
| `datetime.offset.*` (iana, utc) | 2 |
| `datetime.time.*` (hm_12h, hm_24h, hms_12h, hms_24h, iso) | 5 |
| `datetime.timestamp.*` (american, american_24h, european, iso_8601, iso_8601_compact, iso_8601_microseconds, iso_8601_offset, iso_microseconds, rfc_2822, rfc_2822_ordinal, rfc_3339, sql_standard) | 12 |
| **Total** | **46** |

### numeric (14 types)

Numeric values, measurements, quantities:

| Type | Notes |
|------|-------|
| `representation.numeric.decimal_number` | |
| `representation.numeric.increment` | |
| `representation.numeric.integer_number` | |
| `representation.numeric.percentage` | |
| `representation.numeric.scientific_notation` | |
| `representation.numeric.si_number` | |
| `representation.file.file_size` | Byte counts — numeric |
| `identity.person.age` | Numeric measurement |
| `identity.person.height` | Numeric measurement |
| `identity.person.weight` | Numeric measurement |
| `technology.internet.port` | Small integer |
| `technology.internet.http_status_code` | Small integer |
| `technology.hardware.ram_size` | Numeric quantity |
| `technology.hardware.screen_size` | Numeric quantity |
| **Total** | **14** |

### geographic (16 types)

All `geography.*` types:

| Types | Count |
|-------|-------|
| `geography.address.*` (full_address, postal_code, street_name, street_number, street_suffix) | 5 |
| `geography.contact.calling_code` | 1 |
| `geography.coordinate.*` (coordinates, latitude, longitude) | 3 |
| `geography.location.*` (city, continent, country, country_code, region) | 5 |
| `geography.transportation.*` (iata_code, icao_code) | 2 |
| **Total** | **16** |

### entity (9 types)

Person and named-entity types:

| Type | Notes |
|------|-------|
| `identity.person.full_name` | Person names |
| `identity.person.first_name` | |
| `identity.person.last_name` | |
| `identity.person.username` | Person identifier |
| `identity.person.gender` | Person attribute |
| `identity.person.gender_code` | Person attribute |
| `identity.person.gender_symbol` | Person attribute |
| `identity.person.blood_type` | Person attribute |
| `representation.text.entity_name` | Generic named entity |
| **Total** | **9** |

### format (48 types)

Structured identifiers, codes, URLs, containers, technical formats:

| Types | Count |
|-------|-------|
| `container.array.*` (comma_separated, pipe_separated, semicolon_separated, whitespace_separated) | 4 |
| `container.key_value.*` (form_data, query_string) | 2 |
| `container.object.*` (csv, json, json_array, xml, yaml) | 5 |
| `identity.medical.*` (dea_number, ndc, npi) | 3 |
| `identity.payment.*` (bitcoin_address, credit_card_expiration_date, credit_card_network, credit_card_number, currency_code, currency_symbol, cusip, cvv, ethereum_address, isin, lei, paypal_email, sedol, swift_bic) | 14 |
| `identity.person.email` | Structured format |
| `identity.person.phone_number` | Structured format |
| `identity.person.password` | Structured format |
| `representation.code.alphanumeric_id` | |
| `representation.file.*` (excel_format, extension, mime_type) | 3 |
| `representation.scientific.*` (dna_sequence, measurement_unit, metric_prefix, protein_sequence, rna_sequence) | 5 |
| `representation.text.color_hex` | Structured format |
| `representation.text.color_rgb` | Structured format |
| `technology.code.*` (doi, ean, imei, isbn, issn, locale_code, pin) | 7 |
| `technology.cryptographic.*` (hash, token_hex, token_urlsafe, uuid) | 4 |
| `technology.development.*` (calver, os, programming_language, software_license, stage, version) | 6 |
| `technology.internet.*` (hostname, http_method, ip_v4, ip_v4_with_port, ip_v6, mac_address, top_level_domain, url, user_agent) | 9 |
| **Total** | **48** (see note) |

**Note:** `identity.person.email`, `identity.person.phone_number`, and
`identity.person.password` are in `format` rather than `entity` because they
are structurally detectable formats. The Sense model was trained with this mapping
(see `prepare_sense_data.py` where `telephone` and `email` map to `format`).

### text (30 types)

Free text, categories, enums, booleans, discrete values:

| Types | Count |
|-------|-------|
| `representation.boolean.*` (binary, initials, terms) | 3 |
| `representation.discrete.*` (categorical, ordinal) | 2 |
| `representation.text.*` (emoji, paragraph, plain_text, sentence, word) | 5 |
| `identity.payment.credit_card_network` | Low-cardinality enum |
| `identity.payment.currency_symbol` | (overlap: also in format; primary = format) |
| `technology.development.os` | Low-cardinality enum |
| `technology.development.programming_language` | Low-cardinality enum |
| `technology.development.software_license` | Low-cardinality enum |
| `technology.development.stage` | Low-cardinality enum |
| `technology.internet.http_method` | Low-cardinality enum |
| **Total** | **~30** (see reconciliation below) |

### Category Total Reconciliation

| Category | Count |
|----------|-------|
| temporal | 46 |
| numeric | 14 |
| geographic | 16 |
| entity | 9 |
| format | 48 |
| text | 30 |
| **Total** | **163** |

### Overlap Resolution

A few types could plausibly belong to multiple categories. Resolution:

| Type | Primary | Alternative | Resolution |
|------|---------|-------------|------------|
| `geography.address.postal_code` | geographic | format | geographic — analyst thinks "where" |
| `geography.contact.calling_code` | geographic | format | geographic — country indicator |
| `geography.coordinate.*` | geographic | numeric | geographic — lat/lng are locations |
| `identity.person.email` | format | entity | format — structurally detectable |
| `identity.person.phone_number` | format | entity | format — structurally detectable |
| `identity.payment.credit_card_network` | format (in taxonomy) | text | text — low-cardinality enum, but included in format for vote masking since CharCNN can detect it |

For overlapping types, the masking is permissive: if Sense predicts either the primary
or alternative category, the type passes the mask. This is implemented via an
`also_eligible_in` secondary mapping for the ~6 overlap types.

---

## C. Rule Survival Analysis

The current pipeline has 17 numbered disambiguation rules plus 6 header-hint-related
behaviours (semantic hint, hardcoded hint, geography protection, measurement disambiguation,
entity demotion guard, is_generic gating). Total: ~23 distinct behaviours.

### Absorbed by Sense (6 behaviours eliminated)

| Behaviour | Why absorbed |
|-----------|-------------|
| **Rule 18: Entity demotion** (EntityClassifier) | Sense entity subtyping replaces the Deep Sets MLP. When Sense predicts non-person entity subtype, column is entity_name. |
| **Header hint system** (semantic + hardcoded) | Column header is a direct Sense input. Sense sees the header embedding with 50% dropout training — it already learned header→type associations. |
| **Geography protection** | Sense classifies geographic directly. No need to guard person-name hints from overriding geography. |
| **Entity demotion guard** | Eliminated together with header hints. |
| **`is_generic` + header override logic** | Sense sees the header from the start. The entire generic-detection → header-override cascade is unnecessary. |
| **Measurement disambiguation** (age/height/weight) | Sense routes numeric correctly. Retained as a lightweight safety net only within the numeric category (see below). |

### Retained (12 rules)

These rules operate on value-level patterns that Sense (column-level, 128-dim embeddings)
cannot resolve. They run *after* the masked vote, within the Sense-predicted category.

| Rule | Name | Category scope | Why retained |
|------|------|---------------|-------------|
| 1 | Date slash disambiguation (us_slash vs eu_slash) | temporal | Value-level DD/MM vs MM/DD parsing |
| 2 | Short date disambiguation (short_mdy vs short_dmy) | temporal | Same as Rule 1 |
| 3 | Coordinate disambiguation (lat vs lng) | geographic | Value range -90..90 vs -180..180 |
| 4 | IPv4 detection | format | Dotted-quad regex pattern |
| 5 | Day-of-week name detection | temporal | 80% value matching |
| 6 | Month name detection | temporal | 80% value matching |
| 7 | Boolean sub-type normalization | text | Value-level 0/1 vs T/F vs true/false |
| 8 | Gender detection | entity | ALL values match gender set |
| 9 | Boolean override (small integer spread) | text | Prevents false boolean on skewed integers |
| 10 | Small-integer ordinal detection | numeric/text | day_of_month → ordinal for 1,2,3 columns |
| 11 | Categorical detection (low cardinality) | text | ≤20 unique values pattern |
| 12 | Numeric type disambiguation | numeric | Port/year/postal/increment heuristics |

### Partially Absorbed → Safety Nets (4 rules)

These rules have reduced scope — they only fire within the Sense-predicted category.

| Rule | Name | Change |
|------|------|--------|
| 13 | SI number override | Retained within numeric only |
| 14 | Duration override (SEDOL → duration) | Safety net within temporal only. Sense should route temporal correctly, but SEDOL/duration confusion persists at CharCNN level. |
| 15 | Attractor demotion | **Reduced scope**: only demotes attractors within the Sense category. The 3-signal system (validation failure, confidence, cardinality) still applies, but `is_generic` handling changes — Sense replaces the "fall through to header hint" path. Demoted types fall to the category's generic fallback. |
| 16 | Text length demotion (full_address → sentence) | Retained. Still needed for very long text misclassified as addresses. |

### Rule 17 (UTC offset override)

Retained within temporal. Standalone `+05:30` values confused with `hm_24h` at CharCNN level.

### Summary

| Status | Count | Rules |
|--------|-------|-------|
| Retained unchanged | 12 | 1-12, 17 |
| Reduced scope (safety net) | 4 | 13, 14, 15, 16 |
| Absorbed by Sense | 1 | 18 (entity demotion) |
| Absorbed behaviour | 5 | header hints, geography protection, entity guard, is_generic, measurement |
| **Total rules post-Sense** | **~16** | (12 retained + 4 reduced) |

The disambiguate() function retains most rules but each fires only within its applicable
Sense category. Rules for temporal only fire when Sense predicts temporal, etc.

---

## D. Rust Interfaces

### New Module: `sense.rs`

```rust
//! Sense classifier — column-level semantic routing via cross-attention (NNFT-163/164).

use candle_core::{DType, Device, Tensor};
use crate::inference::InferenceError;

/// Broad semantic categories predicted by the Sense model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BroadCategory {
    Entity = 0,
    Format = 1,
    Temporal = 2,
    Numeric = 3,
    Geographic = 4,
    Text = 5,
}

/// Entity subtypes predicted by the Sense model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntitySubtype {
    Person = 0,
    Place = 1,
    Organization = 2,
    CreativeWork = 3,
}

/// Result of Sense classification.
#[derive(Debug, Clone)]
pub struct SenseResult {
    pub broad_category: BroadCategory,
    pub broad_confidence: f32,
    pub entity_subtype: Option<EntitySubtype>,
    pub entity_confidence: f32,
}

/// Sense classifier — Architecture A (cross-attention over Model2Vec).
///
/// Weights: ~1.4MB safetensors.
/// Input: header embedding (optional) + value embeddings (up to 50).
/// Output: 6-class broad category + 4-class entity subtype.
pub struct SenseClassifier {
    // Cross-attention components
    header_proj: Linear,          // Linear(128, 128)
    cross_attn_q: Linear,        // Linear(128, 128)  \
    cross_attn_k: Linear,        // Linear(128, 128)   > MHA decomposed
    cross_attn_v: Linear,        // Linear(128, 128)  /
    cross_attn_out: Linear,      // Linear(128, 128)
    norm_weight: Tensor,          // [128]
    norm_bias: Tensor,            // [128]
    default_query: Tensor,        // [1, 1, 128]
    // Classification heads (3-layer MLP each)
    broad_head: MlpHead,         // 384 → 256 → 128 → 6
    entity_head: MlpHead,        // 384 → 256 → 128 → 4
    n_heads: usize,              // 4
    embed_dim: usize,            // 128
    device: Device,
}

struct Linear { weight: Tensor, bias: Tensor }

struct MlpHead {
    fc1: Linear,  // 384 → 256
    fc2: Linear,  // 256 → 128
    fc3: Linear,  // 128 → n_classes
}
```

**Key methods:**

```rust
impl SenseClassifier {
    /// Load from safetensors bytes + config JSON.
    pub fn from_bytes(model_bytes: &[u8], config_bytes: &[u8]) -> Result<Self, InferenceError>;

    /// Load from directory.
    pub fn load<P: AsRef<Path>>(model_dir: P) -> Result<Self, InferenceError>;

    /// Classify a column given pre-computed embeddings.
    ///
    /// - header_emb: Optional [embed_dim] header embedding
    /// - value_embs: [N, embed_dim] value embeddings (N ≤ 50)
    /// - mask: [N] true for real values
    pub fn classify(
        &self,
        header_emb: Option<&Tensor>,
        value_embs: &Tensor,
        mask: &[bool],
    ) -> Result<SenseResult, InferenceError>;
}
```

**Forward pass (matching PyTorch `SenseModelA.forward`):**

1. Build query: `has_header * header_proj(header_emb) + (1 - has_header) * default_query`
2. Multi-head cross-attention (4 heads, 32-dim each):
   - Q = query @ Wq, K = values @ Wk, V = values @ Wv
   - Per-head: attn = softmax(Q @ K^T / sqrt(32), mask) @ V
   - Concat heads → out_proj → LayerNorm
3. Masked mean and std of value embeddings
4. Feature vector: concat(attn_output[128], val_mean[128], val_std[128]) = 384-dim
5. broad_logits = broad_head(features), entity_logits = entity_head(features)
6. Softmax both → BroadCategory from argmax, EntitySubtype from argmax (only when broad = Entity)

### New Module: `model2vec_shared.rs`

```rust
//! Shared Model2Vec resources — tokenizer + embedding matrix loaded once.

/// Shared Model2Vec tokenizer and embedding matrix.
///
/// Loaded once, borrowed by SemanticHintClassifier, EntityClassifier,
/// and SenseClassifier. Avoids loading the ~7.4MB embedding matrix
/// three times.
pub struct Model2VecResources {
    tokenizer: tokenizers::Tokenizer,
    embeddings: Tensor,  // [vocab_size, 128], F32
    device: Device,
}

impl Model2VecResources {
    pub fn from_bytes(
        tokenizer_bytes: &[u8],
        model_bytes: &[u8],
    ) -> Result<Self, InferenceError>;

    pub fn load<P: AsRef<Path>>(model_dir: P) -> Result<Self, InferenceError>;

    /// Tokenizer reference (cheap clone via Arc).
    pub fn tokenizer(&self) -> &tokenizers::Tokenizer;

    /// Embedding matrix reference (cheap clone via Arc-backed Tensor).
    pub fn embeddings(&self) -> &Tensor;

    /// Encode a single string → mean-pooled, L2-normalised [embed_dim] vector.
    pub fn encode_one(&self, text: &str) -> Result<Tensor, InferenceError>;

    /// Encode multiple strings → [N, embed_dim] matrix (not normalised).
    /// Used by SenseClassifier for value embeddings.
    pub fn encode_batch(&self, texts: &[&str]) -> Result<Tensor, InferenceError>;
}
```

### New Module: `label_category_map.rs`

```rust
//! Maps FineType labels to Sense broad categories for vote masking.

/// Static mapping from FineType type labels to Sense broad categories.
pub struct LabelCategoryMap {
    primary: HashMap<String, BroadCategory>,
    also_eligible: HashMap<String, Vec<BroadCategory>>,
}

impl LabelCategoryMap {
    /// Build from hardcoded mapping (all 163 types).
    pub fn new() -> Self;

    /// Get the primary Sense category for a type label.
    pub fn category_for(&self, label: &str) -> Option<BroadCategory>;

    /// Check if a type label is eligible under a given Sense category.
    /// Returns true if the category is the primary OR in also_eligible.
    pub fn is_eligible(&self, label: &str, category: BroadCategory) -> bool;

    /// Get all labels eligible under a Sense category.
    pub fn eligible_labels(&self, category: BroadCategory) -> Vec<&str>;
}
```

### Modified Module: `column.rs`

```rust
pub struct ColumnClassifier {
    classifier: Box<dyn ValueClassifier>,
    config: ColumnConfig,
    // Existing (unchanged)
    taxonomy: Option<Taxonomy>,
    // New: shared Model2Vec resources
    model2vec: Option<Model2VecResources>,
    // Modified: SemanticHintClassifier now borrows from model2vec
    semantic_hint: Option<SemanticHintClassifier>,
    // Modified: EntityClassifier now borrows from model2vec
    entity_classifier: Option<EntityClassifier>,
    // New: Sense classifier
    sense: Option<SenseClassifier>,
    // New: label → category mapping
    label_map: Option<LabelCategoryMap>,
}
```

New method:

```rust
/// Sense + Sharpen pipeline.
/// Called when self.sense is Some, otherwise falls back to current pipeline.
fn classify_sense_sharpen(
    &self,
    values: &[String],
    header: Option<&str>,
    sense: &SenseClassifier,
    m2v: &Model2VecResources,
    label_map: &LabelCategoryMap,
) -> Result<ColumnResult, InferenceError>;
```

The existing `classify_column` and `classify_column_with_header` methods dispatch to
`classify_sense_sharpen` when Sense is loaded, else follow the current path unchanged.

### Modified Module: `semantic.rs`

Refactored to accept shared resources:

```rust
impl SemanticHintClassifier {
    // Existing load methods remain for backward compatibility
    pub fn load<P: AsRef<Path>>(model_dir: P) -> Result<Self, InferenceError>;

    // New: construct from shared resources + type-specific artifacts
    pub fn from_shared(
        model2vec: &Model2VecResources,  // borrows tokenizer + embeddings
        type_emb_bytes: &[u8],
        label_bytes: &[u8],
    ) -> Result<Self, InferenceError>;
}
```

Internally, `tokenizer` and `embeddings` are `Clone`d from the shared resources
(O(1) for `Tensor` due to Arc-backed storage; `Tokenizer::clone()` is also Arc-based).

### Modified Module: `entity.rs`

Same pattern:

```rust
impl EntityClassifier {
    // Existing: takes owned tokenizer + embeddings
    pub fn from_bytes(..., tokenizer: Tokenizer, embeddings: Tensor) -> Result<Self, ...>;

    // New: construct from shared resources
    pub fn from_shared(
        model2vec: &Model2VecResources,
        model_bytes: &[u8],
        config_bytes: &[u8],
    ) -> Result<Self, InferenceError>;
}
```

### Modified Module: `lib.rs`

Add public exports:

```rust
pub mod label_category_map;
pub mod model2vec_shared;
pub mod sense;

pub use label_category_map::LabelCategoryMap;
pub use model2vec_shared::Model2VecResources;
pub use sense::{BroadCategory, EntitySubtype, SenseClassifier, SenseResult};
```

---

## E. Architecture A Candle Port

The PyTorch `SenseModelA` maps to these Candle operations:

### Weight Mapping (safetensors key → Rust field)

```
header_proj.weight           → header_proj.weight     [128, 128]
header_proj.bias             → header_proj.bias       [128]
cross_attention.in_proj_weight → split into Q/K/V     [384, 128] → 3x [128, 128]
cross_attention.in_proj_bias   → split into Q/K/V     [384] → 3x [128]
cross_attention.out_proj.weight → cross_attn_out.weight [128, 128]
cross_attention.out_proj.bias   → cross_attn_out.bias   [128]
norm.weight                  → norm_weight             [128]
norm.bias                    → norm_bias               [128]
default_query                → default_query            [1, 1, 128]
broad_head.0.weight          → broad_head.fc1.weight   [256, 384]
broad_head.0.bias            → broad_head.fc1.bias     [256]
broad_head.3.weight          → broad_head.fc2.weight   [128, 256]
broad_head.3.bias            → broad_head.fc2.bias     [128]
broad_head.6.weight          → broad_head.fc3.weight   [6, 128]
broad_head.6.bias            → broad_head.fc3.bias     [6]
entity_head.0.weight         → entity_head.fc1.weight  [256, 384]
entity_head.0.bias           → entity_head.fc1.bias    [256]
entity_head.3.weight         → entity_head.fc2.weight  [128, 256]
entity_head.3.bias           → entity_head.fc2.bias    [128]
entity_head.6.weight         → entity_head.fc3.weight  [4, 128]
entity_head.6.bias           → entity_head.fc3.bias    [4]
```

### Multi-Head Attention Decomposition

PyTorch's `nn.MultiheadAttention` stores Q, K, V projections concatenated in
`in_proj_weight` [3*embed_dim, embed_dim]. We split during loading:

```rust
let qkv_weight = tensors.get("cross_attention.in_proj_weight")?; // [384, 128]
let q_weight = qkv_weight.narrow(0, 0, 128)?;      // [128, 128]
let k_weight = qkv_weight.narrow(0, 128, 128)?;     // [128, 128]
let v_weight = qkv_weight.narrow(0, 256, 128)?;      // [128, 128]
```

Same for `in_proj_bias` [384] → 3x [128].

### Forward Pass (Pseudocode)

```rust
fn classify(&self, header_emb: Option<&Tensor>, value_embs: &Tensor, mask: &[bool])
    -> Result<SenseResult, InferenceError>
{
    // 1. Build query [1, 1, D]
    let query = match header_emb {
        Some(h) => self.header_proj.forward(h)?.unsqueeze(0)?.unsqueeze(0)?,
        None => self.default_query.clone(),
    };

    // 2. Multi-head cross-attention
    let n = value_embs.dim(0)?;   // N values
    let d = self.embed_dim;        // 128
    let head_d = d / self.n_heads; // 32

    let q = query.matmul(&self.cross_attn_q.weight.t()?)? + &self.cross_attn_q.bias;
    let k = value_embs.matmul(&self.cross_attn_k.weight.t()?)? + &self.cross_attn_k.bias;
    let v = value_embs.matmul(&self.cross_attn_v.weight.t()?)? + &self.cross_attn_v.bias;

    // Reshape to [n_heads, seq_len, head_dim] and compute attention per head
    // ... (standard scaled dot-product with mask)
    // concat heads → out_proj → LayerNorm

    // 3. Masked mean + std of value embeddings
    // ... (same as EntityClassifier pattern)

    // 4. Concatenate: [attn_out (128), val_mean (128), val_std (128)] = 384
    // 5. MLP heads → softmax → result
}
```

**Estimated size:** ~250 lines of Rust (including weight loading, forward pass,
and the Linear/MLP helper structs). Follows the same pattern as `entity.rs`.

---

## F. Shared Model2Vec Architecture

```
Model2VecResources (loaded once: ~7.4MB tokenizer + embeddings)
  ├── SemanticHintClassifier (borrows tok+emb, owns type_embeddings + label_index)
  ├── EntityClassifier (borrows tok+emb, owns MLP weights + patterns)
  └── SenseClassifier (borrows tok+emb, owns attention + MLP weights)
```

### Memory Impact

| Component | Current | With Sense | Delta |
|-----------|---------|------------|-------|
| Model2Vec tokenizer | 3.1MB | 3.1MB (shared) | 0 |
| Model2Vec embeddings | 4.3MB | 4.3MB (shared) | 0 |
| Semantic type embeddings | 0.3MB | 0.3MB | 0 |
| Entity classifier MLP | 0.4MB | 0.4MB | 0 |
| Sense model weights | — | **1.4MB** | **+1.4MB** |
| Sense config | — | <1KB | ~0 |
| LabelCategoryMap | — | ~8KB | ~0 |
| **Total** | **8.1MB** | **9.5MB** | **+1.4MB** |

No extra copies of the tokenizer or embedding matrix.

### Loading Sequence (CLI)

```rust
// 1. Load shared resources first
let model2vec = Model2VecResources::from_bytes(M2V_TOKENIZER, M2V_MODEL)?;

// 2. Construct consumers from shared resources
let semantic = SemanticHintClassifier::from_shared(
    &model2vec, M2V_TYPE_EMBEDDINGS, M2V_LABEL_INDEX
)?;
let entity = EntityClassifier::from_shared(
    &model2vec, ENTITY_MODEL, ENTITY_CONFIG
)?;
let sense = SenseClassifier::from_bytes(SENSE_MODEL, SENSE_CONFIG)?;

// 3. Wire into ColumnClassifier
let mut col = ColumnClassifier::with_defaults(Box::new(classifier));
col.set_model2vec(model2vec);
col.set_semantic_hint(semantic);
col.set_entity_classifier(entity);
col.set_sense(sense);
col.set_label_category_map(LabelCategoryMap::new());
```

---

## G. Build System Changes

### New Model Directory

```
models/
  sense/
    model.safetensors    (~1.4MB — copied from models/sense_spike/arch_a/)
    config.json          (architecture metadata)
```

### CLI `build.rs` Additions

```rust
// ── Sense classifier ──────────────────────────────────────────────
let sense_dir = models_base.join("sense");
println!("cargo:rerun-if-changed={}", sense_dir.display());

if sense_dir.join("model.safetensors").exists() {
    let model_path = portable_path(&sense_dir.join("model.safetensors"));
    let config_path = portable_path(&sense_dir.join("config.json"));

    code.push_str("\n// Sense classifier (column-level routing, NNFT-164)\n");
    code.push_str("pub const HAS_SENSE_CLASSIFIER: bool = true;\n");
    code.push_str(&format!(
        "pub const SENSE_MODEL: &[u8] = include_bytes!(\"{model_path}\");\n"
    ));
    code.push_str(&format!(
        "pub const SENSE_CONFIG: &[u8] = include_bytes!(\"{config_path}\");\n"
    ));
} else {
    code.push_str("pub const HAS_SENSE_CLASSIFIER: bool = false;\n");
    code.push_str("pub const SENSE_MODEL: &[u8] = &[];\n");
    code.push_str("pub const SENSE_CONFIG: &[u8] = &[];\n");
}
```

### DuckDB Extension

**No changes for Phase 3.** The DuckDB extension continues using the flat CharCNN
without Sense. Adding Sense to the extension is a future workstream — it requires
the extension to ship Model2Vec embeddings (~4.3MB increase) and handle the
additional inference step within DuckDB's chunk-based processing model.

---

## H. Verification Plan

### Regression Testing

| Benchmark | Baseline | Acceptance | Notes |
|-----------|----------|------------|-------|
| Profile eval (label) | 116/120 (96.7%) | ≥ 116/120 | Must not regress |
| Profile eval (domain) | 118/120 (98.3%) | ≥ 118/120 | Must not regress |
| Actionability (datetime) | 98.7% | ≥ 98.5% | Format strings parse correctly |
| SOTAB CTA (label) | 43.6% | ≥ 43.6% | Expect improvement |
| SOTAB CTA (domain) | 68.6% | ≥ 68.6% | Expect improvement |
| `cargo test` | Pass | Pass | All existing tests |
| `make ci` | Pass | Pass | fmt + clippy + test + check |

### A/B Comparison

Run both pipelines on all eval data, log every prediction difference:

```bash
# New: with Sense
finetype profile <file> --model sense  # or auto-detected from embedded model

# Old: without Sense (fallback path)
finetype profile <file> --no-sense     # force legacy pipeline
```

Output a diff report showing where predictions changed and whether each change
is an improvement (matches GT) or regression.

### Speed Verification

Benchmark `finetype profile` on the 21-dataset eval corpus:

| Metric | Current | Target |
|--------|---------|--------|
| Mean column inference | 73ms | <50ms |
| P99 column inference | TBD | <100ms |
| Total profile (21 datasets) | TBD | Faster than current |

### Unit Tests

- `SenseClassifier::from_bytes` loads spike model artifacts
- `SenseClassifier::classify` produces valid categories on synthetic embeddings
- `LabelCategoryMap::new` covers all 163 types
- `Model2VecResources` shares correctly (same pointer for embeddings)
- Masked vote aggregation correctness
- Fallback path (sense=None) unchanged behaviour
- Entity subtype demotion via Sense replaces EntityClassifier demotion

---

## I. Migration Path

### Phase 3 Implementation Order

1. **Model2VecResources** — new module, no breaking changes
2. **Refactor SemanticHintClassifier** — add `from_shared()`, keep `load()`
3. **Refactor EntityClassifier** — add `from_shared()`, keep existing constructors
4. **Port SenseClassifier** — new module, safetensors loading, forward pass
5. **LabelCategoryMap** — new module, static mapping
6. **Integrate into ColumnClassifier** — new `classify_sense_sharpen()` method
7. **Build system** — embed Sense model, CLI loading sequence
8. **Eval: A/B comparison + regression** — verify all benchmarks pass

Each step is a single PR that passes all existing tests. Steps 1-3 are pure refactors
with zero behavioural changes. Step 4 is a new module with its own tests. Steps 5-7
wire everything together. Step 8 validates.

### Backward Compatibility

- `ColumnClassifier::classify_column()` and `classify_column_with_header()` signatures unchanged
- When `sense` is `None`, behaviour is identical to current
- `SemanticHintClassifier::load()` and `EntityClassifier::load()` / `from_bytes()` still work
- DuckDB extension completely unaffected
- CLI `--no-sense` flag forces legacy pipeline for debugging

---

## J. Open Questions (for Phase 3)

1. **Flat CharCNN vs tiered for Sharpen?** This design specifies flat CharCNN + output
   masking. The flat model (char-cnn-v7, 169 classes) already exists. If accuracy within
   Sense categories is insufficient, a future Phase 4 could retrain category-specific
   CharCNN models. But start with flat + masking — simpler and proven.

2. **Sense model improvement path**: After Phase 3 ships, improving Sense accuracy
   (currently 88.5% → target 92%+) is a separate workstream: category boundary cleanup,
   more training data, character distribution features.

3. **DuckDB extension Sense integration**: Deferred. Requires shipping M2V embeddings
   in the extension binary (+4.3MB) and adapting chunk-based inference.
