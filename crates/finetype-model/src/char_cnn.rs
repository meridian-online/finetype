//! Character-level CNN for text classification.
//!
//! Architecture:
//! - Character embedding
//! - Multiple parallel 1D convolutions (kernel sizes 2,3,4,5)
//! - Max pooling over sequence
//! - Optional parallel feature vector fusion at classifier head (NNFT-248)
//! - Fully connected layers
//! - Softmax classifier

use candle_core::{DType, Device, Module, Result, Tensor};
use candle_nn::{conv1d, embedding, linear, Conv1d, Conv1dConfig, Embedding, Linear, VarBuilder};
use std::collections::HashMap;

/// Character vocabulary for the model.
pub struct CharVocab {
    char_to_idx: std::collections::HashMap<char, u32>,
    vocab_size: usize,
}

impl CharVocab {
    /// Create a new character vocabulary.
    pub fn new() -> Self {
        let mut char_to_idx = std::collections::HashMap::new();
        let mut idx = 1u32; // 0 reserved for padding

        // Lowercase letters
        for c in 'a'..='z' {
            char_to_idx.insert(c, idx);
            idx += 1;
        }
        // Uppercase letters
        for c in 'A'..='Z' {
            char_to_idx.insert(c, idx);
            idx += 1;
        }
        // Digits
        for c in '0'..='9' {
            char_to_idx.insert(c, idx);
            idx += 1;
        }
        // Common punctuation and special characters
        for c in " .-_@:/\\#$%&*+='\"<>()[]{}|~`!?,;^".chars() {
            char_to_idx.insert(c, idx);
            idx += 1;
        }

        let vocab_size = idx as usize + 1; // +1 for unknown

        Self {
            char_to_idx,
            vocab_size,
        }
    }

    /// Encode a string to character indices.
    pub fn encode(&self, text: &str, max_len: usize) -> Vec<u32> {
        let mut ids = Vec::with_capacity(max_len);
        for c in text.chars().take(max_len) {
            let idx = self
                .char_to_idx
                .get(&c)
                .copied()
                .unwrap_or(self.vocab_size as u32 - 1);
            ids.push(idx);
        }
        // Pad to max_len
        while ids.len() < max_len {
            ids.push(0); // padding
        }
        ids
    }

    /// Get vocabulary size.
    pub fn vocab_size(&self) -> usize {
        self.vocab_size
    }
}

impl Default for CharVocab {
    fn default() -> Self {
        Self::new()
    }
}

/// Classification head type for the CharCNN (NNFT-267).
#[derive(Debug, Clone, Default, PartialEq)]
pub enum HeadType {
    /// Standard flat softmax over all classes.
    #[default]
    Flat,
    /// Hierarchical tree softmax: domain → category → leaf type.
    Hierarchical,
}

/// Configuration for the character-level CNN.
#[derive(Debug, Clone)]
pub struct CharCnnConfig {
    pub vocab_size: usize,
    pub max_seq_length: usize,
    pub embed_dim: usize,
    pub num_filters: usize,
    pub kernel_sizes: Vec<usize>,
    pub hidden_dim: usize,
    pub n_classes: usize,
    pub dropout: f64,
    /// Dimension of the parallel feature vector (0 = no features, backward compatible).
    /// When > 0, fc1 input becomes `total_filters + feature_dim` to accommodate
    /// the concatenated feature vector at the classifier head. (NNFT-248)
    pub feature_dim: usize,
    /// Classification head type (NNFT-267). Default: Flat.
    pub head_type: HeadType,
}

impl Default for CharCnnConfig {
    fn default() -> Self {
        Self {
            vocab_size: 100,
            max_seq_length: 128,
            embed_dim: 32,
            num_filters: 64,
            kernel_sizes: vec![2, 3, 4, 5],
            hidden_dim: 128,
            n_classes: 100,
            dropout: 0.3,
            feature_dim: 0,
            head_type: HeadType::Flat,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// HIERARCHY MAP (NNFT-267)
// ═══════════════════════════════════════════════════════════════════════════════

/// Maps between flat label indices and the hierarchical (domain, category, type) tree.
///
/// Derived purely from sorted label strings of the form `domain.category.type`.
/// Used at both train time (to compute per-level targets) and inference time
/// (to scatter product probabilities back to the flat label space).
pub struct HierarchyMap {
    /// Sorted domain names (e.g., ["container", "datetime", ...]).
    domain_names: Vec<String>,
    /// Domain name → domain index.
    domain_to_idx: HashMap<String, usize>,
    /// `[domain_idx]` → sorted category names within that domain.
    categories_per_domain: Vec<Vec<String>>,
    /// `(domain_idx, category_name)` → local category index within the domain.
    category_to_local_idx: HashMap<(usize, String), usize>,
    /// `[domain_idx][cat_idx]` → sorted type names within that category.
    types_per_category: Vec<Vec<Vec<String>>>,
    /// `(domain_idx, cat_idx, type_name)` → local type index within the category.
    type_to_local_idx: HashMap<(usize, usize, String), usize>,
    /// `[domain_idx][cat_idx]` → true when category has exactly 1 type (degenerate).
    degenerate: Vec<Vec<bool>>,
    /// `flat_idx` → `(domain_idx, cat_local_idx, type_local_idx)`.
    flat_to_hier: Vec<(usize, usize, usize)>,
    /// `(domain_idx, cat_local_idx, type_local_idx)` → flat_idx.
    hier_to_flat: HashMap<(usize, usize, usize), usize>,
    /// Number of domains (7).
    num_domains: usize,
    /// Total number of categories across all domains (43).
    total_categories: usize,
}

impl HierarchyMap {
    /// Build a hierarchy map from sorted label strings.
    ///
    /// Labels must be in `domain.category.type` format. The labels slice must be
    /// sorted (as produced by `Taxonomy::labels()`).
    pub fn from_labels(labels: &[String]) -> Self {
        // Group labels by domain → category → type
        let mut domain_map: std::collections::BTreeMap<String, std::collections::BTreeMap<String, Vec<String>>> =
            std::collections::BTreeMap::new();

        for label in labels {
            let parts: Vec<&str> = label.splitn(3, '.').collect();
            if parts.len() == 3 {
                domain_map
                    .entry(parts[0].to_string())
                    .or_default()
                    .entry(parts[1].to_string())
                    .or_default()
                    .push(parts[2].to_string());
            }
        }

        let domain_names: Vec<String> = domain_map.keys().cloned().collect();
        let domain_to_idx: HashMap<String, usize> = domain_names
            .iter()
            .enumerate()
            .map(|(i, name)| (name.clone(), i))
            .collect();
        let num_domains = domain_names.len();

        let mut categories_per_domain = Vec::with_capacity(num_domains);
        let mut types_per_category = Vec::with_capacity(num_domains);
        let mut degenerate = Vec::with_capacity(num_domains);
        let mut category_to_local_idx = HashMap::new();
        let mut type_to_local_idx = HashMap::new();
        let mut total_categories = 0;

        for (d_idx, domain) in domain_names.iter().enumerate() {
            let cat_map = &domain_map[domain];
            let cat_names: Vec<String> = cat_map.keys().cloned().collect();
            let mut types_in_domain = Vec::with_capacity(cat_names.len());
            let mut degen_in_domain = Vec::with_capacity(cat_names.len());

            for (c_idx, cat) in cat_names.iter().enumerate() {
                category_to_local_idx.insert((d_idx, cat.clone()), c_idx);
                let mut type_names: Vec<String> = cat_map[cat].clone();
                type_names.sort();
                degen_in_domain.push(type_names.len() == 1);
                for (t_idx, tname) in type_names.iter().enumerate() {
                    type_to_local_idx.insert((d_idx, c_idx, tname.clone()), t_idx);
                }
                types_in_domain.push(type_names);
            }

            total_categories += cat_names.len();
            categories_per_domain.push(cat_names);
            types_per_category.push(types_in_domain);
            degenerate.push(degen_in_domain);
        }

        // Build flat ↔ hier index mappings
        // Labels are sorted, so we iterate in sorted order to match flat indices
        let mut flat_to_hier = vec![(0, 0, 0); labels.len()];
        let mut hier_to_flat = HashMap::new();

        for (flat_idx, label) in labels.iter().enumerate() {
            let parts: Vec<&str> = label.splitn(3, '.').collect();
            if parts.len() == 3 {
                let d_idx = domain_to_idx[parts[0]];
                let c_idx = category_to_local_idx[&(d_idx, parts[1].to_string())];
                let t_idx = type_to_local_idx[&(d_idx, c_idx, parts[2].to_string())];
                flat_to_hier[flat_idx] = (d_idx, c_idx, t_idx);
                hier_to_flat.insert((d_idx, c_idx, t_idx), flat_idx);
            }
        }

        Self {
            domain_names,
            domain_to_idx,
            categories_per_domain,
            category_to_local_idx,
            types_per_category,
            type_to_local_idx,
            degenerate,
            flat_to_hier,
            hier_to_flat,
            num_domains,
            total_categories,
        }
    }

    /// Number of domains.
    pub fn num_domains(&self) -> usize {
        self.num_domains
    }

    /// Total categories across all domains.
    pub fn total_categories(&self) -> usize {
        self.total_categories
    }

    /// Number of categories in a domain.
    pub fn num_categories(&self, domain_idx: usize) -> usize {
        self.categories_per_domain[domain_idx].len()
    }

    /// Number of types in a category.
    pub fn num_types(&self, domain_idx: usize, cat_idx: usize) -> usize {
        self.types_per_category[domain_idx][cat_idx].len()
    }

    /// Whether a category is degenerate (single type).
    pub fn is_degenerate(&self, domain_idx: usize, cat_idx: usize) -> bool {
        self.degenerate[domain_idx][cat_idx]
    }

    /// Convert flat index to hierarchical (domain, category, type) indices.
    pub fn flat_to_hier(&self, flat_idx: usize) -> (usize, usize, usize) {
        self.flat_to_hier[flat_idx]
    }

    /// Convert hierarchical indices to flat index.
    pub fn hier_to_flat(&self, d: usize, c: usize, t: usize) -> usize {
        self.hier_to_flat[&(d, c, t)]
    }

    /// Resolve a label string to hierarchical indices.
    pub fn label_to_hier(&self, label: &str) -> Option<(usize, usize, usize)> {
        let parts: Vec<&str> = label.splitn(3, '.').collect();
        if parts.len() != 3 {
            return None;
        }
        let d_idx = self.domain_to_idx.get(parts[0]).copied()?;
        let c_idx = self
            .category_to_local_idx
            .get(&(d_idx, parts[1].to_string()))
            .copied()?;
        let t_idx = self
            .type_to_local_idx
            .get(&(d_idx, c_idx, parts[2].to_string()))
            .copied()?;
        Some((d_idx, c_idx, t_idx))
    }

    /// Domain names (sorted).
    pub fn domain_names(&self) -> &[String] {
        &self.domain_names
    }

    /// Category names within a domain.
    pub fn category_names(&self, domain_idx: usize) -> &[String] {
        &self.categories_per_domain[domain_idx]
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// HIERARCHICAL HEAD (NNFT-267)
// ═══════════════════════════════════════════════════════════════════════════════

/// Hierarchical classification head: domain → category → leaf type.
///
/// Product of per-level softmax probabilities produces the final flat distribution.
/// Degenerate categories (1 type) skip the leaf classifier (probability = 1.0).
pub struct HierarchicalHead {
    /// Domain classifier: hidden_dim → num_domains.
    domain_head: Linear,
    /// Per-domain category classifiers: hidden_dim → num_categories_in_domain.
    category_heads: Vec<Linear>,
    /// Per-(domain, category) leaf classifiers. None for degenerate categories.
    leaf_heads: Vec<Vec<Option<Linear>>>,
    /// The hierarchy mapping.
    hierarchy: HierarchyMap,
}

impl HierarchicalHead {
    /// Create a new hierarchical head.
    pub fn new(
        hidden_dim: usize,
        labels: &[String],
        vb: VarBuilder,
    ) -> Result<Self> {
        let hierarchy = HierarchyMap::from_labels(labels);

        let domain_head = linear(
            hidden_dim,
            hierarchy.num_domains(),
            vb.pp("hier.domain"),
        )?;

        let mut category_heads = Vec::with_capacity(hierarchy.num_domains());
        for d in 0..hierarchy.num_domains() {
            let cat_head = linear(
                hidden_dim,
                hierarchy.num_categories(d),
                vb.pp(format!("hier.cat_{}", d)),
            )?;
            category_heads.push(cat_head);
        }

        let mut leaf_heads = Vec::with_capacity(hierarchy.num_domains());
        for d in 0..hierarchy.num_domains() {
            let mut cat_leaves = Vec::with_capacity(hierarchy.num_categories(d));
            for c in 0..hierarchy.num_categories(d) {
                if hierarchy.is_degenerate(d, c) {
                    cat_leaves.push(None);
                } else {
                    let leaf = linear(
                        hidden_dim,
                        hierarchy.num_types(d, c),
                        vb.pp(format!("hier.leaf_{}_{}", d, c)),
                    )?;
                    cat_leaves.push(Some(leaf));
                }
            }
            leaf_heads.push(cat_leaves);
        }

        Ok(Self {
            domain_head,
            category_heads,
            leaf_heads,
            hierarchy,
        })
    }

    /// Access the hierarchy map.
    pub fn hierarchy(&self) -> &HierarchyMap {
        &self.hierarchy
    }

    /// Forward pass: produce flat (batch, n_classes) log-probabilities.
    ///
    /// Computes product probabilities: p(type) = softmax(domain)[d] * softmax(cat)[c] * softmax(leaf)[t]
    /// and scatters them into a flat tensor matching the label index ordering.
    #[allow(clippy::needless_range_loop)]
    pub fn forward(&self, hidden: &Tensor, n_classes: usize) -> Result<Tensor> {
        let batch_size = hidden.dims()[0];
        let device = hidden.device();

        // Domain probabilities: (batch, num_domains)
        let domain_logits = self.domain_head.forward(hidden)?;
        let domain_probs = candle_nn::ops::softmax(&domain_logits, 1)?;
        let domain_probs_vec = domain_probs.to_vec2::<f32>()?;

        // Accumulate flat probabilities
        let mut flat_probs = vec![vec![0.0f32; n_classes]; batch_size];

        for d in 0..self.hierarchy.num_domains() {
            // Category probabilities for this domain: (batch, num_cats_in_domain)
            let cat_logits = self.category_heads[d].forward(hidden)?;
            let cat_probs = candle_nn::ops::softmax(&cat_logits, 1)?;
            let cat_probs_vec = cat_probs.to_vec2::<f32>()?;

            for c in 0..self.hierarchy.num_categories(d) {
                if self.hierarchy.is_degenerate(d, c) {
                    // Single type → probability = domain_prob * cat_prob * 1.0
                    let flat_idx = self.hierarchy.hier_to_flat(d, c, 0);
                    for b in 0..batch_size {
                        flat_probs[b][flat_idx] =
                            domain_probs_vec[b][d] * cat_probs_vec[b][c];
                    }
                } else {
                    // Leaf probabilities: (batch, num_types_in_cat)
                    let leaf = self.leaf_heads[d][c].as_ref().unwrap();
                    let leaf_logits = leaf.forward(hidden)?;
                    let leaf_probs = candle_nn::ops::softmax(&leaf_logits, 1)?;
                    let leaf_probs_vec = leaf_probs.to_vec2::<f32>()?;

                    for t in 0..self.hierarchy.num_types(d, c) {
                        let flat_idx = self.hierarchy.hier_to_flat(d, c, t);
                        for b in 0..batch_size {
                            flat_probs[b][flat_idx] = domain_probs_vec[b][d]
                                * cat_probs_vec[b][c]
                                * leaf_probs_vec[b][t];
                        }
                    }
                }
            }
        }

        // Convert back to tensor
        let flat_data: Vec<f32> = flat_probs.into_iter().flatten().collect();
        Tensor::new(flat_data, device)?.reshape((batch_size, n_classes))
    }

    /// Compute per-level logits for training loss.
    ///
    /// Returns `(domain_logits, category_logits_per_domain, leaf_logits_per_category)`.
    /// Used by the training loop to compute multi-level cross-entropy.
    #[allow(clippy::type_complexity)]
    pub fn forward_levels(
        &self,
        hidden: &Tensor,
    ) -> Result<(Tensor, Vec<Tensor>, Vec<Vec<Option<Tensor>>>)> {
        let domain_logits = self.domain_head.forward(hidden)?;

        let mut cat_logits = Vec::with_capacity(self.hierarchy.num_domains());
        let mut leaf_logits = Vec::with_capacity(self.hierarchy.num_domains());

        for d in 0..self.hierarchy.num_domains() {
            cat_logits.push(self.category_heads[d].forward(hidden)?);

            let mut leaves = Vec::with_capacity(self.hierarchy.num_categories(d));
            for c in 0..self.hierarchy.num_categories(d) {
                match &self.leaf_heads[d][c] {
                    Some(leaf) => leaves.push(Some(leaf.forward(hidden)?)),
                    None => leaves.push(None),
                }
            }
            leaf_logits.push(leaves);
        }

        Ok((domain_logits, cat_logits, leaf_logits))
    }
}

/// Character-level CNN classifier.
///
/// Supports two head types (NNFT-267):
/// - **Flat**: Standard fc2 → n_classes softmax (existing, default)
/// - **Hierarchical**: Tree softmax via `HierarchicalHead` (domain → category → leaf)
pub struct CharCnn {
    char_embedding: Embedding,
    convs: Vec<Conv1d>,
    fc1: Linear,
    /// Flat classification head (present when head_type == Flat).
    fc2: Option<Linear>,
    /// Hierarchical classification head (present when head_type == Hierarchical).
    hierarchical: Option<HierarchicalHead>,
    config: CharCnnConfig,
}

impl CharCnn {
    /// Create a new character-level CNN with flat head (backward compatible).
    pub fn new(config: CharCnnConfig, vb: VarBuilder) -> Result<Self> {
        let (char_embedding, convs, fc1) = Self::build_backbone(&config, &vb)?;
        let fc2 = linear(config.hidden_dim, config.n_classes, vb.pp("fc2"))?;

        Ok(Self {
            char_embedding,
            convs,
            fc1,
            fc2: Some(fc2),
            hierarchical: None,
            config,
        })
    }

    /// Create a new character-level CNN with hierarchical head (NNFT-267).
    ///
    /// The hierarchy is derived from the sorted label strings (domain.category.type).
    pub fn new_hierarchical(
        config: CharCnnConfig,
        labels: &[String],
        vb: VarBuilder,
    ) -> Result<Self> {
        let (char_embedding, convs, fc1) = Self::build_backbone(&config, &vb)?;
        let hierarchical = HierarchicalHead::new(config.hidden_dim, labels, vb)?;

        Ok(Self {
            char_embedding,
            convs,
            fc1,
            fc2: None,
            hierarchical: Some(hierarchical),
            config,
        })
    }

    /// Build the shared backbone (embedding, convolutions, fc1).
    fn build_backbone(
        config: &CharCnnConfig,
        vb: &VarBuilder,
    ) -> Result<(Embedding, Vec<Conv1d>, Linear)> {
        let char_embedding = embedding(config.vocab_size, config.embed_dim, vb.pp("char_emb"))?;

        let mut convs = Vec::with_capacity(config.kernel_sizes.len());
        for (i, &kernel_size) in config.kernel_sizes.iter().enumerate() {
            let conv_config = Conv1dConfig {
                padding: 0,
                stride: 1,
                dilation: 1,
                groups: 1,
            };
            let conv = conv1d(
                config.embed_dim,
                config.num_filters,
                kernel_size,
                conv_config,
                vb.pp(format!("conv_{}", i)),
            )?;
            convs.push(conv);
        }

        let total_filters = config.num_filters * config.kernel_sizes.len();
        let fc1_input = total_filters + config.feature_dim;
        let fc1 = linear(fc1_input, config.hidden_dim, vb.pp("fc1"))?;

        Ok((char_embedding, convs, fc1))
    }

    /// Backbone forward pass: returns hidden vector (batch, hidden_dim) after fc1 + ReLU.
    ///
    /// Shared between flat and hierarchical heads. Also used by the training loop
    /// to compute per-level losses separately from the head.
    pub fn backbone_forward(
        &self,
        input_ids: &Tensor,
        features: Option<&Tensor>,
    ) -> Result<Tensor> {
        let (batch_size, _seq_len) = input_ids.dims2()?;

        // Character embeddings: (batch, seq_len) -> (batch, seq_len, embed_dim)
        let emb = self.char_embedding.forward(input_ids)?;

        // Conv1d expects (batch, channels, seq_len), so transpose
        let emb = emb.transpose(1, 2)?;

        // Apply each convolution and max pool
        let mut pooled_outputs = Vec::with_capacity(self.convs.len());
        for conv in &self.convs {
            let conv_out = conv.forward(&emb)?;
            let conv_out = conv_out.relu()?;
            let pooled = conv_out.max(2)?;
            pooled_outputs.push(pooled);
        }

        // Concatenate all pooled conv outputs: (batch, total_filters)
        let conv_out = Tensor::cat(&pooled_outputs, 1)?;

        // Fuse with feature vector if feature_dim > 0 (NNFT-248)
        let fused = if self.config.feature_dim > 0 {
            let feat = match features {
                Some(f) => f.clone(),
                None => Tensor::zeros(
                    (batch_size, self.config.feature_dim),
                    DType::F32,
                    input_ids.device(),
                )?,
            };
            Tensor::cat(&[conv_out, feat], 1)?
        } else {
            conv_out
        };

        // fc1 + ReLU
        let hidden = self.fc1.forward(&fused)?;
        hidden.relu()
    }

    /// Forward pass without features (backward compatible).
    pub fn forward(&self, input_ids: &Tensor) -> Result<Tensor> {
        self.forward_with_features(input_ids, None)
    }

    /// Forward pass with optional parallel feature vector (NNFT-248).
    ///
    /// Returns logits for flat mode, or product probabilities for hierarchical mode.
    /// Shape is always (batch, n_classes).
    pub fn forward_with_features(
        &self,
        input_ids: &Tensor,
        features: Option<&Tensor>,
    ) -> Result<Tensor> {
        let hidden = self.backbone_forward(input_ids, features)?;

        if let Some(ref fc2) = self.fc2 {
            // Flat mode: fc2 produces logits
            fc2.forward(&hidden)
        } else if let Some(ref hier) = self.hierarchical {
            // Hierarchical mode: product probabilities (already normalized)
            hier.forward(&hidden, self.config.n_classes)
        } else {
            unreachable!("CharCnn must have either fc2 or hierarchical head")
        }
    }

    /// Inference with softmax probabilities (no features).
    pub fn infer(&self, input_ids: &Tensor) -> Result<Tensor> {
        self.infer_with_features(input_ids, None)
    }

    /// Inference with softmax probabilities and optional features (NNFT-248).
    ///
    /// For flat mode, applies softmax to logits.
    /// For hierarchical mode, forward already returns probabilities.
    pub fn infer_with_features(
        &self,
        input_ids: &Tensor,
        features: Option<&Tensor>,
    ) -> Result<Tensor> {
        if self.hierarchical.is_some() {
            // Hierarchical forward already produces probabilities
            self.forward_with_features(input_ids, features)
        } else {
            // Flat mode: apply softmax to logits
            let logits = self.forward_with_features(input_ids, features)?;
            candle_nn::ops::softmax(&logits, 1)
        }
    }

    /// Whether this model uses a hierarchical head.
    pub fn is_hierarchical(&self) -> bool {
        self.hierarchical.is_some()
    }

    /// Access the hierarchical head (if present).
    pub fn hierarchical_head(&self) -> Option<&HierarchicalHead> {
        self.hierarchical.as_ref()
    }

    /// Get config.
    pub fn config(&self) -> &CharCnnConfig {
        &self.config
    }

    /// Get the device from the model's embedding layer.
    pub fn device(&self) -> Device {
        // Use embedding weight tensor to determine device
        Device::Cpu // Safe default; actual device comes from VarBuilder at construction
    }
}
