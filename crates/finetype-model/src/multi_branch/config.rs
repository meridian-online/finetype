//! Multi-branch model configuration types (deserialized from config.json).

use serde::{Deserialize, Serialize};

// ═══════════════════════════════════════════════════════════════════════════════
// Configuration (mirrors finetype-train MultiBranchConfig for deserialization)
// ═══════════════════════════════════════════════════════════════════════════════

/// Activation function for branch and merge layers.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub enum Activation {
    /// Rectified Linear Unit (default for backward compatibility).
    #[default]
    ReLU,
    /// Gaussian Error Linear Unit (autoresearch winner: +1.3pp).
    GELU,
}

/// Classification head type.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub enum HeadType {
    #[default]
    Flat,
    Hierarchical,
}

/// Configuration for the multi-branch model (deserialized from config.json).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiBranchConfig {
    pub char_dim: usize,
    pub embed_dim: usize,
    pub stats_dim: usize,
    /// Input dimension for header embedding features (Model2Vec, 128-dim).
    /// Defaults to 0 for backward compatibility with old configs.
    #[serde(default)]
    pub header_dim: usize,
    pub char_hidden: [usize; 2],
    pub embed_hidden: [usize; 2],
    pub stats_hidden: [usize; 2],
    /// Hidden layer sizes for the header branch (2 layers).
    /// Defaults to [0, 0] for old configs.
    #[serde(default)]
    pub header_hidden: [usize; 2],
    pub merge_hidden: [usize; 2],
    pub n_classes: usize,
    pub dropout: f32,
    pub head_type: HeadType,
    /// Activation function (default: ReLU for backward compat with old configs).
    #[serde(default)]
    pub activation: Activation,
    /// Whether to use LayerNorm instead of BatchNorm, and add input LayerNorm to all branches.
    #[serde(default)]
    pub use_layer_norm: bool,
    /// Input dimension for validation features (pass rates per type, e.g. 239).
    /// Defaults to 0 for backward compatibility with v11 and earlier models.
    #[serde(default)]
    pub valid_dim: usize,
    /// Hidden layer sizes for the validation branch (2 layers).
    /// Defaults to [0, 0] for old configs. Recommended: [128, 64].
    #[serde(default)]
    pub valid_hidden: [usize; 2],
    /// Ordered type keys used at training time for validation feature indexing.
    /// Stored to decouple inference from exact taxonomy version — types added
    /// after training get zero pass rates, types removed are ignored.
    #[serde(default)]
    pub type_index_keys: Vec<String>,
    /// Optional path to a SECOND Model2Vec encoder used ONLY for the value-
    /// aggregation branch (dual-encoder, e.g. potion-8M). Resolved relative to
    /// the model dir first, then as a workspace/absolute path. When absent, the
    /// value branch shares the header encoder (potion-4M) — backward compatible
    /// with v19 and every single-encoder model.
    #[serde(default)]
    pub value_embed_model: Option<String>,
    /// Cross-value attention pool for the value branch (choice 0106). When set, the
    /// embed branch input is the mean/var/min/max blender ‖ pool output, and the
    /// per-value embeds are encoded once from the column values. None (default) =
    /// the legacy fixed-pool value branch.
    #[serde(default)]
    pub value_attention: Option<crate::value_attention::ValueAttentionConfig>,
}

impl MultiBranchConfig {
    /// Whether the header branch is enabled (header_dim > 0 with valid hidden dims).
    pub fn has_header_branch(&self) -> bool {
        self.header_dim > 0 && self.header_hidden[0] > 0 && self.header_hidden[1] > 0
    }

    /// Whether the validation branch is enabled (valid_dim > 0 with valid hidden dims).
    pub fn has_validation_branch(&self) -> bool {
        self.valid_dim > 0 && self.valid_hidden[0] > 0 && self.valid_hidden[1] > 0
    }

    /// Actual input dimension of the embed branch. Without value attention this is
    /// `embed_dim` (the blender). With it, the blender (when `keep_blender_concat`)
    /// is concatenated with the pool output before the branch.
    pub fn embed_branch_input_dim(&self) -> usize {
        match &self.value_attention {
            Some(va) => {
                let blender = if va.keep_blender_concat {
                    self.embed_dim
                } else {
                    0
                };
                blender + va.output_dim()
            }
            None => self.embed_dim,
        }
    }

    /// Total dimension of the merged trunk input (sum of all branch hidden[1] dims).
    pub fn merged_dim(&self) -> usize {
        let mut dim = self.char_hidden[1] + self.embed_hidden[1] + self.stats_hidden[1];
        if self.has_header_branch() {
            dim += self.header_hidden[1];
        }
        if self.has_validation_branch() {
            dim += self.valid_hidden[1];
        }
        dim
    }
}
