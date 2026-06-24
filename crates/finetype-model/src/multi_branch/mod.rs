//! Multi-branch model loader and inference for column-level classification.
//!
//! Loads multi-branch model artifacts (model.safetensors + config.json + label_map.json)
//! and provides column-level inference: Vec<String> -> features -> MLP forward -> label.
//!
//! Architecture (from finetype-train, configurable activation + normalization):
//! ```text
//! Branch 1 (char):   [960] -> [LN ->] Dense(300, Act) -> Dense(300, Act) -> [300]
//! Branch 2 (embed):  [512] -> [LN ->] Dense(200, Act) -> Dense(200, Act) -> [200]
//! Branch 3 (stats):  [27]  -> [LN ->] Dense(128, Act) -> Dense(64, Act)  -> [64]
//! Branch 4 (header): [128] -> LN -> Dense(128, Act) -> Dense(64, Act)    -> [64]  (optional)
//!                              |
//! Merge:              concat([300, 200, 64, 64]) = [628]  (or [564] without header)
//!                              |
//!                     Norm -> Dense(500, Act) -> Dense(500, Act)
//!                              |
//! Head (flat):        Dense(n_classes, softmax)
//! Head (hier):        TreeSoftmax(7 domains -> 43 categories -> 250 types)
//! ```
//!
//! Act = ReLU (default) or GELU (config.activation).
//! LN = LayerNorm on branch inputs (when config.use_layer_norm=true).
//! Norm = BatchNorm (default) or LayerNorm (when config.use_layer_norm=true).

use crate::char_cnn::HierarchicalHead;
use crate::char_distribution::{extract_char_distribution, CHAR_DIST_DIM};
use crate::column_stats::{extract_column_stats, COLUMN_STATS_DIM};
use crate::embedding_aggregation::{
    extract_embedding_aggregation_dyn, extract_embedding_aggregation_from_batch,
};
use crate::inference::InferenceError;
use crate::model2vec_shared::Model2VecResources;
use crate::value_attention::ValueAttentionPool;
use candle_core::{DType, Device, Module, Tensor};
use candle_nn::{batch_norm, linear, BatchNorm, BatchNormConfig, Linear, ModuleT, VarBuilder};
use finetype_core::Taxonomy;
use std::path::Path;

mod branch;
mod config;

use branch::{BranchWeights, ClassificationHead};
pub use config::{Activation, HeadType, MultiBranchConfig};

// ═══════════════════════════════════════════════════════════════════════════════
// Model (inference only)
// ═══════════════════════════════════════════════════════════════════════════════

/// Multi-branch model for column-level type classification.
///
/// Loads from safetensors + config.json + label_map.json and provides
/// column-level inference without implementing ValueClassifier.
///
/// Supports both flat and hierarchical classification heads.
/// Merge normalization: either BatchNorm (legacy) or LayerNorm (GELU+LN models).
enum MergeNorm {
    Batch(BatchNorm),
    Layer(candle_nn::LayerNorm),
}

pub struct MultiBranchClassifier {
    char_branch: BranchWeights,
    embed_branch: BranchWeights,
    stats_branch: BranchWeights,
    header_branch: Option<BranchWeights>,
    validation_branch: Option<BranchWeights>,
    /// Cross-value attention pool feeding the embed branch (choice 0106). None for
    /// the legacy fixed-pool value branch. Same module + weight names as training.
    value_attention: Option<ValueAttentionPool>,
    merge_norm: MergeNorm,
    merge_linear1: Linear,
    merge_linear2: Linear,
    head: ClassificationHead,
    config: MultiBranchConfig,
    /// Index → label mapping (sorted by index).
    labels: Vec<String>,
    /// Model2Vec resources for header encoding (and the value branch when no
    /// separate value encoder is configured).
    model2vec: Model2VecResources,
    /// Optional SECOND Model2Vec encoder for the value-aggregation branch only
    /// (dual-encoder, e.g. potion-8M). `None` for single-encoder models — the
    /// value branch then shares `model2vec`. Selected by `config.value_embed_model`.
    value_model2vec: Option<Model2VecResources>,
    /// Validation feature extractor (built from config.type_index_keys).
    /// None when the model has no validation branch (v11 and earlier).
    validation_extractor: Option<crate::validation_features::ValidationFeatureExtractor>,
    /// Fallback validation extractor derived from the live taxonomy at first use,
    /// for models whose config is missing `type_index_keys` but DO have a trained
    /// validation branch. Without this, the branch is silently fed zeros — a model
    /// trained on real validation pass-rates then mis-predicts (see the potion-8M
    /// config bug). Lazily built because the taxonomy is only available at classify
    /// time. `Some(None)` means derivation was attempted but the taxonomy dim did
    /// not match `valid_dim` (branch correctly left zeroed).
    derived_validation_extractor:
        std::sync::OnceLock<Option<crate::validation_features::ValidationFeatureExtractor>>,
}

impl MultiBranchClassifier {
    /// Load a multi-branch model from a directory containing:
    /// - model.safetensors (weights)
    /// - config.json (architecture config)
    /// - label_map.json (index → label mapping)
    ///
    /// Also loads Model2Vec resources from models/model2vec/ (required for
    /// embedding feature extraction).
    ///
    /// Supports both flat and hierarchical head types. The hierarchical head
    /// builds its hierarchy map from the label list (domain.category.type format).
    pub fn load<P: AsRef<Path>>(model_dir: P) -> Result<Self, InferenceError> {
        let dir = model_dir.as_ref();

        // Load config
        let config_bytes = std::fs::read(dir.join("config.json"))
            .map_err(|e| InferenceError::InvalidPath(format!("Failed to read config.json: {e}")))?;

        // Load label map
        let label_bytes = std::fs::read(dir.join("label_map.json")).map_err(|e| {
            InferenceError::InvalidPath(format!("Failed to read label_map.json: {e}"))
        })?;

        // Load weights
        let model_bytes = std::fs::read(dir.join("model.safetensors")).map_err(|e| {
            InferenceError::InvalidPath(format!("Failed to read model.safetensors: {e}"))
        })?;

        // Load Model2Vec resources (header + classifiers; potion-4M)
        let m2v = Self::load_model2vec(dir)?;

        // Optional second encoder for the value branch (dual-encoder, e.g. potion-8M).
        // Read the field straight from config.json so resolution can be relative to
        // the model dir before from_bytes (which has no dir).
        let value_m2v = Self::load_value_model2vec(dir, &config_bytes)?;

        Self::from_bytes(&config_bytes, &label_bytes, &model_bytes, m2v, value_m2v)
    }

    /// Construct a MultiBranchClassifier from raw byte slices.
    ///
    /// Used by the CLI to load from embedded model data (release binaries)
    /// when the model directory doesn't exist on disk.
    pub fn from_bytes(
        config_bytes: &[u8],
        label_bytes: &[u8],
        model_bytes: &[u8],
        model2vec: Model2VecResources,
        value_model2vec: Option<Model2VecResources>,
    ) -> Result<Self, InferenceError> {
        let config: MultiBranchConfig = serde_json::from_slice(config_bytes).map_err(|e| {
            InferenceError::InvalidPath(format!("Failed to parse config.json: {e}"))
        })?;

        let labels: Vec<String> = serde_json::from_slice(label_bytes).map_err(|e| {
            InferenceError::InvalidPath(format!("Failed to parse label_map.json: {e}"))
        })?;

        if labels.len() != config.n_classes {
            return Err(InferenceError::InvalidPath(format!(
                "label_map.json has {} labels but config.json specifies n_classes={}",
                labels.len(),
                config.n_classes,
            )));
        }

        let device = Device::Cpu;
        let tensors = candle_core::safetensors::load_buffer(model_bytes, &device)
            .map_err(|e| InferenceError::InvalidPath(format!("Failed to load safetensors: {e}")))?;

        let vb = VarBuilder::from_tensors(tensors, DType::F32, &device);

        // Build branches + merge trunk
        // When use_layer_norm=true, all branches get input LayerNorm.
        // When false, only header gets it (backward compatible).
        let build_branch =
            |input_dim, hidden, name: &str| -> Result<BranchWeights, InferenceError> {
                if config.use_layer_norm {
                    BranchWeights::new_with_input_norm(
                        input_dim,
                        hidden,
                        &config.activation,
                        vb.pp(name),
                    )
                } else {
                    BranchWeights::new(input_dim, hidden, &config.activation, vb.pp(name))
                }
                .map_err(|e| InferenceError::InvalidPath(format!("{name} branch: {e}")))
            };

        let char_branch = build_branch(config.char_dim, config.char_hidden, "char")?;
        // Embed branch input widens to blender ‖ pool when value attention is on.
        let embed_branch = build_branch(
            config.embed_branch_input_dim(),
            config.embed_hidden,
            "embed",
        )?;
        let stats_branch = build_branch(config.stats_dim, config.stats_hidden, "stats")?;

        // Cross-value attention pool (choice 0106). Same module + weight prefix
        // ("value_attn") training saved — loads straight from the safetensors.
        let value_attention = match &config.value_attention {
            Some(va) => Some(
                ValueAttentionPool::new(va, vb.pp("value_attn"))
                    .map_err(|e| InferenceError::InvalidPath(format!("value_attn: {e}")))?,
            ),
            None => None,
        };

        // Load header branch if config enables it and weights exist in safetensors
        // Header always gets input LayerNorm (stabilises raw Model2Vec embeddings).
        let header_branch = if config.has_header_branch() {
            match BranchWeights::new_with_input_norm(
                config.header_dim,
                config.header_hidden,
                &config.activation,
                vb.pp("header"),
            ) {
                Ok(branch) => Some(branch),
                Err(e) => {
                    // Graceful fallback: old model without header weights
                    tracing::warn!("Header branch configured but weights missing, disabling: {e}");
                    None
                }
            }
        } else {
            None
        };

        // Load validation branch if config enables it and weights exist
        // No input LayerNorm — pass rates are already bounded [0, 1].
        let validation_branch = if config.has_validation_branch() {
            match BranchWeights::new(
                config.valid_dim,
                config.valid_hidden,
                &config.activation,
                vb.pp("valid"),
            ) {
                Ok(branch) => Some(branch),
                Err(e) => {
                    tracing::warn!(
                        "Validation branch configured but weights missing, disabling: {e}"
                    );
                    None
                }
            }
        } else {
            None
        };

        // Build validation feature extractor from saved type_index_keys
        let validation_extractor =
            if validation_branch.is_some() && !config.type_index_keys.is_empty() {
                Some(
                    crate::validation_features::ValidationFeatureExtractor::from_type_keys(
                        config.type_index_keys.clone(),
                    ),
                )
            } else {
                None
            };

        let merged_dim = {
            let mut dim = config.char_hidden[1] + config.embed_hidden[1] + config.stats_hidden[1];
            if header_branch.is_some() {
                dim += config.header_hidden[1];
            }
            if validation_branch.is_some() {
                dim += config.valid_hidden[1];
            }
            dim
        };

        // Merge normalization: LayerNorm (new) or BatchNorm (legacy)
        let merge_norm = if config.use_layer_norm {
            MergeNorm::Layer(
                candle_nn::layer_norm(
                    merged_dim,
                    candle_nn::LayerNormConfig::default(),
                    vb.pp("merge_ln"),
                )
                .map_err(|e| InferenceError::InvalidPath(format!("merge_ln: {e}")))?,
            )
        } else {
            MergeNorm::Batch(
                batch_norm(merged_dim, BatchNormConfig::default(), vb.pp("merge_bn"))
                    .map_err(|e| InferenceError::InvalidPath(format!("merge_bn: {e}")))?,
            )
        };

        let merge_linear1 = linear(merged_dim, config.merge_hidden[0], vb.pp("merge_l1"))
            .map_err(|e| InferenceError::InvalidPath(format!("merge_l1: {e}")))?;
        let merge_linear2 = linear(
            config.merge_hidden[0],
            config.merge_hidden[1],
            vb.pp("merge_l2"),
        )
        .map_err(|e| InferenceError::InvalidPath(format!("merge_l2: {e}")))?;

        // Build classification head based on head_type
        let head = match config.head_type {
            HeadType::Flat => {
                let flat_head = linear(config.merge_hidden[1], config.n_classes, vb.pp("head"))
                    .map_err(|e| InferenceError::InvalidPath(format!("head: {e}")))?;
                ClassificationHead::Flat(flat_head)
            }
            HeadType::Hierarchical => {
                let hier_head = HierarchicalHead::new(
                    config.merge_hidden[1],
                    &labels,
                    vb.pp(HierarchicalHead::VARBUILDER_PREFIX),
                )
                .map_err(|e| InferenceError::InvalidPath(format!("hierarchical head: {e}")))?;
                ClassificationHead::Hierarchical(Box::new(hier_head))
            }
        };

        Ok(Self {
            char_branch,
            embed_branch,
            stats_branch,
            header_branch,
            validation_branch,
            value_attention,
            merge_norm,
            merge_linear1,
            merge_linear2,
            head,
            config,
            labels,
            model2vec,
            value_model2vec,
            validation_extractor,
            derived_validation_extractor: std::sync::OnceLock::new(),
        })
    }

    /// Load Model2Vec resources. Tries model_dir/model2vec/ first, then
    /// falls back to models/model2vec/ (shared location).
    fn load_model2vec(model_dir: &Path) -> Result<Model2VecResources, InferenceError> {
        // Try model-local first
        let local_m2v = model_dir.join("model2vec");
        if local_m2v.join("model.safetensors").exists() {
            return Model2VecResources::load(&local_m2v);
        }

        // Try shared location
        let shared_m2v = std::path::PathBuf::from("models/model2vec");
        if shared_m2v.join("model.safetensors").exists() {
            return Model2VecResources::load(&shared_m2v);
        }

        Err(InferenceError::InvalidPath(
            "Model2Vec resources not found. Checked: model_dir/model2vec/, models/model2vec/"
                .into(),
        ))
    }

    /// Load the optional value-branch Model2Vec encoder named by
    /// `config.value_embed_model`. Resolves the path relative to `model_dir`
    /// first, then as a workspace/absolute path. Returns `None` when the field
    /// is absent (single-encoder models). Errors only when the field is set but
    /// the artifact cannot be found/loaded.
    fn load_value_model2vec(
        model_dir: &Path,
        config_bytes: &[u8],
    ) -> Result<Option<Model2VecResources>, InferenceError> {
        let config: MultiBranchConfig = serde_json::from_slice(config_bytes).map_err(|e| {
            InferenceError::InvalidPath(format!("Failed to parse config.json: {e}"))
        })?;
        let Some(rel) = config.value_embed_model.as_deref() else {
            return Ok(None);
        };

        let local = model_dir.join(rel);
        let dir = if local.join("model.safetensors").exists() {
            local
        } else {
            std::path::PathBuf::from(rel)
        };
        if !dir.join("model.safetensors").exists() {
            return Err(InferenceError::InvalidPath(format!(
                "value_embed_model '{rel}' not found (checked {}/{rel} and {rel})",
                model_dir.display()
            )));
        }
        Ok(Some(Model2VecResources::load(&dir)?))
    }

    /// The encoder used for the value-aggregation branch: the dedicated value
    /// encoder when configured (dual-encoder), otherwise the shared `model2vec`.
    fn value_resources(&self) -> &Model2VecResources {
        self.value_model2vec.as_ref().unwrap_or(&self.model2vec)
    }

    /// Build the embed-branch input tensor `[1, embed_branch_input_dim]` for a column.
    ///
    /// Without value attention: the mean/var/min/max blender `[1, embed_dim]`
    /// (unchanged behaviour). With it: encodes the column's values with the value
    /// encoder ONCE and derives BOTH the blender and the per-value attention pool
    /// input from that single encode — the attention path never re-encodes (the
    /// inference-latency rule, choice 0106). Values are filtered to non-empty and
    /// capped at `n_values`, matching the FTMB v6 write-time filter so the pool sees
    /// the same rows it was trained on.
    fn embed_input_tensor(
        &self,
        value_refs: &[&str],
        device: &Device,
    ) -> Result<Tensor, InferenceError> {
        let resources = self.value_resources();
        let (Some(pool), Some(va)) = (&self.value_attention, self.config.value_attention.as_ref())
        else {
            // Legacy fixed-pool path — single encode inside `_dyn`, unchanged.
            let blender = extract_embedding_aggregation_dyn(value_refs, resources)
                .unwrap_or_else(|| vec![0.0f32; self.config.embed_dim]);
            return Tensor::from_slice(&blender, (1, self.config.embed_dim), device)
                .map_err(|e| InferenceError::InvalidPath(format!("embed tensor: {e}")));
        };

        let d = va.value_embed_dim;
        let n_values = va.n_values;
        let width = self.config.embed_branch_input_dim();

        let filtered: Vec<&str> = value_refs
            .iter()
            .copied()
            .filter(|s| !s.trim().is_empty())
            .take(n_values)
            .collect();
        if filtered.is_empty() {
            return Tensor::zeros((1, width), DType::F32, device)
                .map_err(|e| InferenceError::InvalidPath(format!("embed zeros: {e}")));
        }

        // One encode; the pool input AND the blender both derive from `batch`.
        let batch = resources
            .encode_batch(&filtered)
            .map_err(|e| InferenceError::InvalidPath(format!("value encode: {e}")))?;

        let mut embeds = vec![0.0f32; n_values * d];
        let mut mask = vec![0.0f32; n_values];
        for j in 0..filtered.len() {
            let row: Vec<f32> = batch
                .get(j)
                .and_then(|t| t.to_vec1())
                .map_err(|e| InferenceError::InvalidPath(format!("value row: {e}")))?;
            embeds[j * d..(j + 1) * d].copy_from_slice(&row[..d]);
            mask[j] = 1.0;
        }
        let embeds_t = Tensor::from_slice(&embeds, (1, n_values, d), device)
            .map_err(|e| InferenceError::InvalidPath(format!("value embeds: {e}")))?;
        let mask_t = Tensor::from_slice(&mask, (1, n_values), device)
            .map_err(|e| InferenceError::InvalidPath(format!("value mask: {e}")))?;
        let pooled = pool
            .forward(&embeds_t, &mask_t, false)
            .map_err(|e| InferenceError::InvalidPath(format!("value pool: {e}")))?;

        if va.keep_blender_concat {
            let blender = extract_embedding_aggregation_from_batch(&batch, filtered.len(), d)
                .unwrap_or_else(|| vec![0.0f32; self.config.embed_dim]);
            let blender_t = Tensor::from_slice(&blender, (1, self.config.embed_dim), device)
                .map_err(|e| InferenceError::InvalidPath(format!("blender tensor: {e}")))?;
            Tensor::cat(&[&blender_t, &pooled], 1)
                .map_err(|e| InferenceError::InvalidPath(format!("embed cat: {e}")))
        } else {
            Ok(pooled)
        }
    }

    /// Classify a column of values, returning (label, confidence).
    ///
    /// Extracts branch features from the values, runs the MLP forward pass,
    /// and returns the predicted label with confidence score.
    ///
    /// For flat heads: confidence is the softmax probability of the top prediction.
    /// For hierarchical heads: confidence is the product probability (domain × category × type).
    ///
    /// `header` is the column name. When non-empty and the model has a header branch,
    /// it is embedded via Model2Vec and fed through the 4th branch.
    ///
    /// `taxonomy` enables the validation branch (v12+). When provided and the model
    /// has a validation extractor, validation pass rates are computed and fed through
    /// the 5th branch. Pass `None` for v11 and earlier models.
    pub fn classify_column(
        &self,
        values: &[String],
        header: &str,
        taxonomy: Option<&Taxonomy>,
    ) -> Result<(String, f32), InferenceError> {
        if values.is_empty() {
            return Ok(("unknown".to_string(), 0.0));
        }

        let value_refs: Vec<&str> = values.iter().map(|s| s.as_str()).collect();

        // Extract features
        let char_feats = extract_char_distribution(&value_refs).unwrap_or([0.0f32; CHAR_DIST_DIM]);
        // embed branch input (blender ‖ pool) is built below from a single encode.
        let stats_feats = extract_column_stats(&value_refs).unwrap_or([0.0f32; COLUMN_STATS_DIM]);

        // Forward pass through trunk
        let device = Device::Cpu;
        let char_t = Tensor::from_slice(&char_feats, (1, CHAR_DIST_DIM), &device)
            .map_err(|e| InferenceError::InvalidPath(format!("char tensor: {e}")))?;
        let embed_t = self.embed_input_tensor(&value_refs, &device)?;
        let stats_t = Tensor::from_slice(
            &stats_feats[..self.config.stats_dim],
            (1, self.config.stats_dim),
            &device,
        )
        .map_err(|e| InferenceError::InvalidPath(format!("stats tensor: {e}")))?;

        // Extract header embedding if the model has a header branch
        let header_t = if self.header_branch.is_some() {
            let header_embed = if !header.is_empty() {
                self.model2vec
                    .encode_one(header)
                    .and_then(|t| t.to_vec1::<f32>().ok())
                    .unwrap_or_else(|| vec![0.0f32; self.config.header_dim])
            } else {
                vec![0.0f32; self.config.header_dim]
            };
            Some(
                Tensor::from_slice(&header_embed, (1, self.config.header_dim), &device)
                    .map_err(|e| InferenceError::InvalidPath(format!("header tensor: {e}")))?,
            )
        } else {
            None
        };

        // Extract validation features if the model has a validation branch
        let valid_t = self.compute_validation_tensor(&value_refs, taxonomy, &device)?;

        let hidden = self.forward_trunk(
            &char_t,
            &embed_t,
            &stats_t,
            header_t.as_ref(),
            valid_t.as_ref(),
        )?;

        // Head-specific forward pass + probability extraction
        let probs_vec = match &self.head {
            ClassificationHead::Flat(head) => {
                // Flat: hidden → logits → softmax → probabilities
                let logits = head
                    .forward_t(&hidden, false)
                    .map_err(|e| InferenceError::InvalidPath(format!("head: {e}")))?;
                let probs = candle_nn::ops::softmax(&logits, 1)
                    .map_err(|e| InferenceError::InvalidPath(format!("softmax: {e}")))?;
                probs
                    .squeeze(0)
                    .map_err(|e| InferenceError::InvalidPath(format!("squeeze: {e}")))?
                    .to_vec1::<f32>()
                    .map_err(|e| InferenceError::InvalidPath(format!("to_vec1: {e}")))?
            }
            ClassificationHead::Hierarchical(hier_head) => {
                // Hierarchical: hidden → tree softmax → product probabilities
                // forward() already returns probabilities (not logits)
                let probs = hier_head
                    .forward(&hidden, self.config.n_classes)
                    .map_err(|e| {
                        InferenceError::InvalidPath(format!("hierarchical forward: {e}"))
                    })?;
                probs
                    .squeeze(0)
                    .map_err(|e| InferenceError::InvalidPath(format!("squeeze: {e}")))?
                    .to_vec1::<f32>()
                    .map_err(|e| InferenceError::InvalidPath(format!("to_vec1: {e}")))?
            }
        };

        let (max_idx, max_prob) = probs_vec
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap();

        let label = self
            .labels
            .get(max_idx)
            .cloned()
            .unwrap_or_else(|| format!("unknown_idx_{max_idx}"));

        Ok((label, *max_prob))
    }

    /// Classify a column using a pre-enriched header embedding.
    ///
    /// Like `classify_column()`, but accepts a pre-computed header tensor
    /// (e.g., from sibling-context attention) instead of embedding the raw
    /// header string via Model2Vec. Used when sibling context is available
    /// to pass enriched header embeddings into the multi-branch model.
    ///
    /// `taxonomy` enables the validation branch (v12+). See `classify_column()`.
    pub fn classify_column_with_enriched_header(
        &self,
        values: &[String],
        enriched_header: &Tensor, // [D] — already enriched by sibling attention
        taxonomy: Option<&Taxonomy>,
    ) -> Result<(String, f32), InferenceError> {
        if values.is_empty() {
            return Ok(("unknown".to_string(), 0.0));
        }

        let value_refs: Vec<&str> = values.iter().map(|s| s.as_str()).collect();

        // Extract features (same as classify_column)
        let char_feats = extract_char_distribution(&value_refs).unwrap_or([0.0f32; CHAR_DIST_DIM]);
        // embed branch input (blender ‖ pool) is built below from a single encode.
        let stats_feats = extract_column_stats(&value_refs).unwrap_or([0.0f32; COLUMN_STATS_DIM]);

        let device = Device::Cpu;
        let char_t = Tensor::from_slice(&char_feats, (1, CHAR_DIST_DIM), &device)
            .map_err(|e| InferenceError::InvalidPath(format!("char tensor: {e}")))?;
        let embed_t = self.embed_input_tensor(&value_refs, &device)?;
        let stats_t = Tensor::from_slice(
            &stats_feats[..self.config.stats_dim],
            (1, self.config.stats_dim),
            &device,
        )
        .map_err(|e| InferenceError::InvalidPath(format!("stats tensor: {e}")))?;

        // Use the pre-enriched header embedding (reshaped to [1, D])
        let header_t = if self.header_branch.is_some() {
            let header_embed = enriched_header
                .to_vec1::<f32>()
                .unwrap_or_else(|_| vec![0.0f32; self.config.header_dim]);
            Some(
                Tensor::from_slice(&header_embed, (1, self.config.header_dim), &device)
                    .map_err(|e| InferenceError::InvalidPath(format!("header tensor: {e}")))?,
            )
        } else {
            None
        };

        // Extract validation features if the model has a validation branch
        let valid_t = self.compute_validation_tensor(&value_refs, taxonomy, &device)?;

        let hidden = self.forward_trunk(
            &char_t,
            &embed_t,
            &stats_t,
            header_t.as_ref(),
            valid_t.as_ref(),
        )?;

        // Head-specific forward pass (same as classify_column)
        let probs_vec = match &self.head {
            ClassificationHead::Flat(head) => {
                let logits = head
                    .forward_t(&hidden, false)
                    .map_err(|e| InferenceError::InvalidPath(format!("head: {e}")))?;
                let probs = candle_nn::ops::softmax(&logits, 1)
                    .map_err(|e| InferenceError::InvalidPath(format!("softmax: {e}")))?;
                probs
                    .squeeze(0)
                    .map_err(|e| InferenceError::InvalidPath(format!("squeeze: {e}")))?
                    .to_vec1::<f32>()
                    .map_err(|e| InferenceError::InvalidPath(format!("to_vec1: {e}")))?
            }
            ClassificationHead::Hierarchical(hier_head) => {
                let probs = hier_head
                    .forward(&hidden, self.config.n_classes)
                    .map_err(|e| {
                        InferenceError::InvalidPath(format!("hierarchical forward: {e}"))
                    })?;
                probs
                    .squeeze(0)
                    .map_err(|e| InferenceError::InvalidPath(format!("squeeze: {e}")))?
                    .to_vec1::<f32>()
                    .map_err(|e| InferenceError::InvalidPath(format!("to_vec1: {e}")))?
            }
        };

        let (max_idx, max_prob) = probs_vec
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap();

        let label = self
            .labels
            .get(max_idx)
            .cloned()
            .unwrap_or_else(|| format!("unknown_idx_{max_idx}"));

        Ok((label, *max_prob))
    }

    /// Resolve the validation extractor for this column.
    ///
    /// Precedence: the config-pinned extractor (`type_index_keys`) if present —
    /// that decouples inference from taxonomy version and is the intended path.
    /// Otherwise, if the model HAS a trained validation branch but the config is
    /// missing `type_index_keys`, derive the extractor from the live taxonomy
    /// (cached on first use). This prevents the branch from being silently fed
    /// zeros — a model trained on real validation pass-rates then mis-predicts.
    /// The derived order matches training, which built its features the same way
    /// (`ValidationFeatureExtractor::new`), as long as the taxonomy ordering is
    /// unchanged; guarded by a `valid_dim` match so a mismatched taxonomy leaves
    /// the branch zeroed rather than scrambled.
    fn resolve_validation_extractor(
        &self,
        taxonomy: Option<&Taxonomy>,
    ) -> Option<&crate::validation_features::ValidationFeatureExtractor> {
        if let Some(ext) = &self.validation_extractor {
            return Some(ext);
        }
        // No pinned keys. Only derive when a trained validation branch exists.
        self.validation_branch.as_ref()?;
        let tax = taxonomy?;
        self.derived_validation_extractor
            .get_or_init(|| {
                let ext = crate::validation_features::ValidationFeatureExtractor::new(tax);
                if ext.dim() == self.config.valid_dim {
                    tracing::warn!(
                        "config.type_index_keys missing; deriving validation order from live \
                         taxonomy ({} types). Persist type_index_keys in the model config to pin it.",
                        ext.dim()
                    );
                    Some(ext)
                } else {
                    tracing::warn!(
                        "config.type_index_keys missing and live taxonomy dim {} != valid_dim {}; \
                         validation branch left zeroed.",
                        ext.dim(),
                        self.config.valid_dim
                    );
                    None
                }
            })
            .as_ref()
    }

    /// Compute validation features as a tensor, if the model has a validation branch.
    ///
    /// Returns `Some(Tensor)` of shape `[1, valid_dim]` when a validation extractor
    /// resolves (config-pinned or taxonomy-derived) and a taxonomy is provided.
    /// Returns `None` otherwise (v11 models or when taxonomy is unavailable —
    /// forward_trunk fills zeros).
    fn compute_validation_tensor(
        &self,
        value_refs: &[&str],
        taxonomy: Option<&Taxonomy>,
        device: &Device,
    ) -> Result<Option<Tensor>, InferenceError> {
        let (extractor, tax) = match (self.resolve_validation_extractor(taxonomy), taxonomy) {
            (Some(ext), Some(tax)) => (ext, tax),
            _ => return Ok(None),
        };

        let feats = extractor.extract(value_refs, tax);
        let t = Tensor::from_slice(&feats, (1, extractor.dim()), device)
            .map_err(|e| InferenceError::InvalidPath(format!("validation tensor: {e}")))?;
        Ok(Some(t))
    }

    /// Forward pass through the trunk (branches + merge), returning the hidden
    /// representation before the classification head.
    fn forward_trunk(
        &self,
        char_feats: &Tensor,
        embed_feats: &Tensor,
        stats_feats: &Tensor,
        header_feats: Option<&Tensor>,
        validation_feats: Option<&Tensor>,
    ) -> Result<Tensor, InferenceError> {
        let char_out = self
            .char_branch
            .forward(char_feats)
            .map_err(|e| InferenceError::InvalidPath(format!("char forward: {e}")))?;
        let embed_out = self
            .embed_branch
            .forward(embed_feats)
            .map_err(|e| InferenceError::InvalidPath(format!("embed forward: {e}")))?;
        let stats_out = self
            .stats_branch
            .forward(stats_feats)
            .map_err(|e| InferenceError::InvalidPath(format!("stats forward: {e}")))?;

        // Collect branch outputs for concatenation
        let mut branches = vec![char_out, embed_out, stats_out];

        if let Some(ref hb) = self.header_branch {
            let batch_size = branches[0]
                .dim(0)
                .map_err(|e| InferenceError::InvalidPath(format!("batch dim: {e}")))?;
            let header_input = match header_feats {
                Some(hf) => hf.clone(),
                None => Tensor::zeros(
                    (batch_size, self.config.header_dim),
                    candle_core::DType::F32,
                    char_feats.device(),
                )
                .map_err(|e| InferenceError::InvalidPath(format!("header zeros: {e}")))?,
            };
            let header_out = hb
                .forward(&header_input)
                .map_err(|e| InferenceError::InvalidPath(format!("header forward: {e}")))?;
            branches.push(header_out);
        }

        if let Some(ref vb) = self.validation_branch {
            let batch_size = branches[0]
                .dim(0)
                .map_err(|e| InferenceError::InvalidPath(format!("batch dim: {e}")))?;
            let valid_input = match validation_feats {
                Some(vf) => vf.clone(),
                None => Tensor::zeros(
                    (batch_size, self.config.valid_dim),
                    candle_core::DType::F32,
                    char_feats.device(),
                )
                .map_err(|e| InferenceError::InvalidPath(format!("valid zeros: {e}")))?,
            };
            let valid_out = vb
                .forward(&valid_input)
                .map_err(|e| InferenceError::InvalidPath(format!("validation forward: {e}")))?;
            branches.push(valid_out);
        }

        let branch_refs: Vec<Tensor> = branches;
        let merged = Tensor::cat(&branch_refs, 1)
            .map_err(|e| InferenceError::InvalidPath(format!("concat: {e}")))?;

        // Merge normalization
        let normed = match &self.merge_norm {
            MergeNorm::Batch(bn) => {
                // BatchNorm: [B, C] -> [B, C, 1] -> BN -> [B, C]
                let merged_3d = merged
                    .unsqueeze(2)
                    .map_err(|e| InferenceError::InvalidPath(format!("unsqueeze: {e}")))?;
                let normed_3d = bn
                    .forward_t(&merged_3d, false)
                    .map_err(|e| InferenceError::InvalidPath(format!("batch_norm: {e}")))?;
                normed_3d
                    .squeeze(2)
                    .map_err(|e| InferenceError::InvalidPath(format!("squeeze: {e}")))?
            }
            MergeNorm::Layer(ln) => {
                // LayerNorm works directly on [B, C]
                ln.forward(&merged)
                    .map_err(|e| InferenceError::InvalidPath(format!("layer_norm: {e}")))?
            }
        };

        let activate = |x: &Tensor, label: &str| -> Result<Tensor, InferenceError> {
            match self.config.activation {
                Activation::ReLU => x
                    .relu()
                    .map_err(|e| InferenceError::InvalidPath(format!("{label}: {e}"))),
                Activation::GELU => x
                    .gelu_erf()
                    .map_err(|e| InferenceError::InvalidPath(format!("{label}: {e}"))),
            }
        };

        let h = self
            .merge_linear1
            .forward_t(&normed, false)
            .map_err(|e| InferenceError::InvalidPath(format!("merge_l1: {e}")))?;
        let h = activate(&h, "act1")?;
        let h = self
            .merge_linear2
            .forward_t(&h, false)
            .map_err(|e| InferenceError::InvalidPath(format!("merge_l2: {e}")))?;
        activate(&h, "act2")
    }

    /// Top-k classification surface — same input contract as `classify_column`
    /// but returns the top-k `(label, probability)` pairs in descending order
    /// for diagnostic use (ac-04 confidence distribution in the
    /// amount-variant-generators spec). Internal state is identical to
    /// `classify_column`; the only difference is we keep the full softmax
    /// instead of discarding it after taking argmax.
    pub fn classify_column_topk(
        &self,
        values: &[String],
        header: &str,
        taxonomy: Option<&Taxonomy>,
        k: usize,
    ) -> Result<Vec<(String, f32)>, InferenceError> {
        if values.is_empty() || k == 0 {
            return Ok(Vec::new());
        }

        let value_refs: Vec<&str> = values.iter().map(|s| s.as_str()).collect();

        let char_feats = extract_char_distribution(&value_refs).unwrap_or([0.0f32; CHAR_DIST_DIM]);
        // embed branch input (blender ‖ pool) is built below from a single encode.
        let stats_feats = extract_column_stats(&value_refs).unwrap_or([0.0f32; COLUMN_STATS_DIM]);

        let device = Device::Cpu;
        let char_t = Tensor::from_slice(&char_feats, (1, CHAR_DIST_DIM), &device)
            .map_err(|e| InferenceError::InvalidPath(format!("char tensor: {e}")))?;
        let embed_t = self.embed_input_tensor(&value_refs, &device)?;
        let stats_t = Tensor::from_slice(
            &stats_feats[..self.config.stats_dim],
            (1, self.config.stats_dim),
            &device,
        )
        .map_err(|e| InferenceError::InvalidPath(format!("stats tensor: {e}")))?;

        let header_t = if self.header_branch.is_some() {
            let header_embed = if !header.is_empty() {
                self.model2vec
                    .encode_one(header)
                    .and_then(|t| t.to_vec1::<f32>().ok())
                    .unwrap_or_else(|| vec![0.0f32; self.config.header_dim])
            } else {
                vec![0.0f32; self.config.header_dim]
            };
            Some(
                Tensor::from_slice(&header_embed, (1, self.config.header_dim), &device)
                    .map_err(|e| InferenceError::InvalidPath(format!("header tensor: {e}")))?,
            )
        } else {
            None
        };

        let valid_t = self.compute_validation_tensor(&value_refs, taxonomy, &device)?;

        let hidden = self.forward_trunk(
            &char_t,
            &embed_t,
            &stats_t,
            header_t.as_ref(),
            valid_t.as_ref(),
        )?;

        let probs_vec = match &self.head {
            ClassificationHead::Flat(head) => {
                let logits = head
                    .forward_t(&hidden, false)
                    .map_err(|e| InferenceError::InvalidPath(format!("head: {e}")))?;
                let probs = candle_nn::ops::softmax(&logits, 1)
                    .map_err(|e| InferenceError::InvalidPath(format!("softmax: {e}")))?;
                probs
                    .squeeze(0)
                    .map_err(|e| InferenceError::InvalidPath(format!("squeeze: {e}")))?
                    .to_vec1::<f32>()
                    .map_err(|e| InferenceError::InvalidPath(format!("to_vec1: {e}")))?
            }
            ClassificationHead::Hierarchical(hier_head) => {
                let probs = hier_head
                    .forward(&hidden, self.config.n_classes)
                    .map_err(|e| {
                        InferenceError::InvalidPath(format!("hierarchical forward: {e}"))
                    })?;
                probs
                    .squeeze(0)
                    .map_err(|e| InferenceError::InvalidPath(format!("squeeze: {e}")))?
                    .to_vec1::<f32>()
                    .map_err(|e| InferenceError::InvalidPath(format!("to_vec1: {e}")))?
            }
        };

        let mut indexed: Vec<(usize, f32)> =
            probs_vec.iter().enumerate().map(|(i, &p)| (i, p)).collect();
        indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let k = k.min(indexed.len());
        Ok(indexed
            .into_iter()
            .take(k)
            .map(|(i, p)| {
                let label = self
                    .labels
                    .get(i)
                    .cloned()
                    .unwrap_or_else(|| format!("unknown_idx_{i}"));
                (label, p)
            })
            .collect())
    }

    /// Return the number of output classes.
    pub fn n_classes(&self) -> usize {
        self.config.n_classes
    }

    /// Raw pre-softmax logits over all `n_classes`, in `labels()` order.
    ///
    /// This is View2 for the late-fusion classifier: the column-level signal the
    /// fusion head fuses with the value-level CharCNN view. Mirrors
    /// `classify_column`'s feature extraction and trunk, but returns the head's
    /// raw logit vector instead of an argmax label. A Flat head emits logits
    /// directly; a Hierarchical head returns probabilities, so we return their
    /// log (a monotone logit surrogate) to keep one contract.
    pub fn column_logits(
        &self,
        values: &[String],
        header: &str,
        taxonomy: Option<&Taxonomy>,
    ) -> Result<Vec<f32>, InferenceError> {
        if values.is_empty() {
            return Ok(vec![0.0f32; self.config.n_classes]);
        }

        let value_refs: Vec<&str> = values.iter().map(|s| s.as_str()).collect();

        let char_feats = extract_char_distribution(&value_refs).unwrap_or([0.0f32; CHAR_DIST_DIM]);
        // embed branch input (blender ‖ pool) is built below from a single encode.
        let stats_feats = extract_column_stats(&value_refs).unwrap_or([0.0f32; COLUMN_STATS_DIM]);

        let device = Device::Cpu;
        let char_t = Tensor::from_slice(&char_feats, (1, CHAR_DIST_DIM), &device)
            .map_err(|e| InferenceError::InvalidPath(format!("char tensor: {e}")))?;
        let embed_t = self.embed_input_tensor(&value_refs, &device)?;
        let stats_t = Tensor::from_slice(
            &stats_feats[..self.config.stats_dim],
            (1, self.config.stats_dim),
            &device,
        )
        .map_err(|e| InferenceError::InvalidPath(format!("stats tensor: {e}")))?;

        let header_t = if self.header_branch.is_some() {
            let header_embed = if !header.is_empty() {
                self.model2vec
                    .encode_one(header)
                    .and_then(|t| t.to_vec1::<f32>().ok())
                    .unwrap_or_else(|| vec![0.0f32; self.config.header_dim])
            } else {
                vec![0.0f32; self.config.header_dim]
            };
            Some(
                Tensor::from_slice(&header_embed, (1, self.config.header_dim), &device)
                    .map_err(|e| InferenceError::InvalidPath(format!("header tensor: {e}")))?,
            )
        } else {
            None
        };

        let valid_t = self.compute_validation_tensor(&value_refs, taxonomy, &device)?;

        let hidden = self.forward_trunk(
            &char_t,
            &embed_t,
            &stats_t,
            header_t.as_ref(),
            valid_t.as_ref(),
        )?;

        let logits = match &self.head {
            ClassificationHead::Flat(head) => head
                .forward_t(&hidden, false)
                .map_err(|e| InferenceError::InvalidPath(format!("head: {e}")))?
                .squeeze(0)
                .map_err(|e| InferenceError::InvalidPath(format!("squeeze: {e}")))?
                .to_vec1::<f32>()
                .map_err(|e| InferenceError::InvalidPath(format!("to_vec1: {e}")))?,
            ClassificationHead::Hierarchical(hier_head) => {
                let probs = hier_head
                    .forward(&hidden, self.config.n_classes)
                    .map_err(|e| InferenceError::InvalidPath(format!("hierarchical forward: {e}")))?
                    .squeeze(0)
                    .map_err(|e| InferenceError::InvalidPath(format!("squeeze: {e}")))?
                    .to_vec1::<f32>()
                    .map_err(|e| InferenceError::InvalidPath(format!("to_vec1: {e}")))?;
                probs.iter().map(|p| (p.max(1e-9)).ln()).collect()
            }
        };

        Ok(logits)
    }

    /// Return the label list (index → label mapping).
    pub fn labels(&self) -> &[String] {
        &self.labels
    }

    /// Return the head type of this model.
    pub fn head_type(&self) -> &HeadType {
        &self.config.head_type
    }

    /// Check if a model directory contains a multi-branch model.
    ///
    /// Looks for model.safetensors + config.json where config contains
    /// multi-branch fields (char_dim, embed_dim, stats_dim).
    pub fn is_multi_branch_dir<P: AsRef<Path>>(dir: P) -> bool {
        let dir = dir.as_ref();
        let config_path = dir.join("config.json");
        let model_path = dir.join("model.safetensors");
        let label_path = dir.join("label_map.json");

        if !config_path.exists() || !model_path.exists() || !label_path.exists() {
            return false;
        }

        // Check config has multi-branch fields
        if let Ok(bytes) = std::fs::read(&config_path) {
            if let Ok(config) = serde_json::from_slice::<MultiBranchConfig>(&bytes) {
                // Multi-branch models have char_dim, embed_dim, stats_dim
                return config.char_dim > 0 && config.embed_dim > 0 && config.stats_dim > 0;
            }
        }

        false
    }

    /// Get a reference to the Model2Vec resources (for external use).
    pub fn model2vec(&self) -> &Model2VecResources {
        &self.model2vec
    }
}

#[cfg(test)]
mod tests;
