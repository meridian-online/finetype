//! Entity classifier — binary demotion gate for full_name overcall (NNFT-152).
//!
//! When CharCNN votes `full_name` for a column, the entity classifier determines
//! whether the column actually contains person names or other entity types (places,
//! organizations, creative works). If the model confidently predicts "not person,"
//! the column label is demoted from `full_name` to `entity_name`.
//!
//! Architecture: Deep Sets MLP (Zaheer et al. 2017)
//!   Per-value: Model2Vec encoding (frozen, potion-base-4M, 128-dim)
//!   Column:    mean + std of value embeddings + 44 statistical features = 300-dim
//!   Classify:  MLP (BatchNorm → 3×Linear/ReLU → Linear) → 4 classes
//!   Decision:  if max(non-person probs) > threshold → demote to entity_name
//!
//! Model artifacts are prepared by `scripts/train_entity_classifier.py` and stored
//! in `models/entity-classifier/`. At build time they can be embedded into the binary.

use crate::inference::InferenceError;
use crate::model2vec_shared::Model2VecResources;
use candle_core::{DType, Device, Tensor};
use regex::Regex;
use std::path::Path;

/// Number of statistical features computed per column.
/// Must match `config.json:n_stat_features` and the Python training script.
const N_STAT_FEATURES: usize = 44;

/// Default demotion threshold — if max non-person probability exceeds this,
/// demote full_name to entity_name. Calibrated on SOTAB test set.
///
/// At 0.6 on balanced test data: 92.2% precision, 65.9% coverage.
/// At production base rates (~96% non-person): ~99% precision.
const DEFAULT_DEMOTION_THRESHOLD: f32 = 0.6;

/// Person class index in the output vector [person, place, organization, creative_work].
const PERSON_CLASS_INDEX: usize = 0;

/// Pre-compiled regex patterns for domain-specific statistical features.
struct DomainPatterns {
    org_suffixes: Regex,
    person_titles: Regex,
    place_indicators: Regex,
    creative_indicators: Regex,
    prepositions: Regex,
    digits: Regex,
}

impl DomainPatterns {
    fn new() -> Self {
        Self {
            org_suffixes: Regex::new(concat!(
                r"(?i)\b(Inc|LLC|Ltd|Corp|Co|Company|Group|Foundation|Association|",
                r"University|Institute|Hospital|Church|School|Bank|Restaurant|",
                r"Hotel|Salon|Clinic|Studios?|Records|Entertainment|GmbH|AG|SA|",
                r"Pty|Pvt|PLC|LLP)\b",
            ))
            .expect("org_suffixes regex"),
            person_titles: Regex::new(r"(?i)\b(Mr|Mrs|Ms|Dr|Jr|Sr|III|II|IV)\b\.?")
                .expect("person_titles regex"),
            place_indicators: Regex::new(concat!(
                r"(?i)\b(Street|St|Avenue|Ave|Road|Rd|Boulevard|Blvd|Drive|Lane|",
                r"Court|Place|Square|Park|Bridge|Hill|Valley|Lake|River|Mountain|",
                r"Beach|Island|Bay|County|Province|State|Region|District|City|Town|Village)\b",
            ))
            .expect("place_indicators regex"),
            creative_indicators: Regex::new(concat!(
                r"(?i)\b(Album|Song|Track|Episode|Season|Chapter|Vol|Volume|Remix|Live|",
                r"feat|ft|Edition|Deluxe|Remaster|OST|Soundtrack|Tour|Concert|Festival)\b",
            ))
            .expect("creative_indicators regex"),
            prepositions: Regex::new(r"(?i)\b(of|in|at|for|the|and|by|on|to)\b")
                .expect("prepositions regex"),
            digits: Regex::new(r"\d+").expect("digits regex"),
        }
    }
}

/// Entity classifier using a Deep Sets MLP.
///
/// Loads MLP weights from safetensors, reuses a shared Model2Vec tokenizer
/// and embedding matrix from `SemanticHintClassifier` for value encoding.
pub struct EntityClassifier {
    /// Token embedding matrix: [vocab_size, embed_dim] (shared from Model2Vec)
    embeddings: Tensor,
    /// Tokenizer (shared from Model2Vec)
    tokenizer: tokenizers::Tokenizer,
    /// MLP layer weights and biases
    mlp: MlpWeights,
    /// Demotion confidence threshold
    threshold: f32,
    /// Pre-compiled regex patterns
    patterns: DomainPatterns,
    device: Device,
}

/// MLP weights: BatchNorm1d → Linear(300,256) → Linear(256,256) → Linear(256,128) → Linear(128,4)
///
/// PyTorch Sequential naming convention:
///   net.0  = BatchNorm1d(300)
///   net.1  = Linear(300, 256)  (+ net.2 = ReLU, net.3 = Dropout)
///   net.4  = Linear(256, 256)  (+ net.5 = ReLU, net.6 = Dropout)
///   net.7  = Linear(256, 128)  (+ net.8 = ReLU, net.9 = Dropout)
///   net.10 = Linear(128, 4)
struct MlpWeights {
    // BatchNorm1d(300) in eval mode: y = (x - running_mean) / sqrt(running_var + eps) * weight + bias
    bn_weight: Tensor,       // [300]
    bn_bias: Tensor,         // [300]
    bn_running_mean: Tensor, // [300]
    bn_running_var: Tensor,  // [300]
    // Linear layers
    fc1_weight: Tensor, // [256, 300]
    fc1_bias: Tensor,   // [256]
    fc2_weight: Tensor, // [256, 256]
    fc2_bias: Tensor,   // [256]
    fc3_weight: Tensor, // [128, 256]
    fc3_bias: Tensor,   // [128]
    fc4_weight: Tensor, // [4, 128]
    fc4_bias: Tensor,   // [4]
}

impl MlpWeights {
    /// Load MLP weights from safetensors bytes.
    fn from_bytes(bytes: &[u8], device: &Device) -> Result<Self, InferenceError> {
        let tensors = candle_core::safetensors::load_buffer(bytes, device)?;

        let get = |name: &str| -> Result<Tensor, InferenceError> {
            tensors
                .get(name)
                .ok_or_else(|| {
                    InferenceError::InvalidPath(format!(
                        "Missing tensor '{}' in entity classifier safetensors",
                        name
                    ))
                })
                .and_then(|t| Ok(t.to_dtype(DType::F32)?))
        };

        Ok(Self {
            bn_weight: get("net.0.weight")?,
            bn_bias: get("net.0.bias")?,
            bn_running_mean: get("net.0.running_mean")?,
            bn_running_var: get("net.0.running_var")?,
            fc1_weight: get("net.1.weight")?,
            fc1_bias: get("net.1.bias")?,
            fc2_weight: get("net.4.weight")?,
            fc2_bias: get("net.4.bias")?,
            fc3_weight: get("net.7.weight")?,
            fc3_bias: get("net.7.bias")?,
            fc4_weight: get("net.10.weight")?,
            fc4_bias: get("net.10.bias")?,
        })
    }

    /// Forward pass: BatchNorm(eval) → Linear/ReLU × 3 → Linear → softmax.
    ///
    /// Input x: 1D tensor of shape [feature_dim] (single column).
    /// Output: 1D tensor of shape [n_classes] (class probabilities).
    fn forward(&self, x: &Tensor) -> Result<Tensor, InferenceError> {
        // Unsqueeze to [1, feature_dim] for matmul compatibility
        let x = x.unsqueeze(0)?;

        // BatchNorm1d in eval mode:
        //   y = (x - running_mean) / sqrt(running_var + eps) * weight + bias
        let eps = 1e-5_f64;
        let x_norm = x.broadcast_sub(&self.bn_running_mean)?;
        let var_eps = (&self.bn_running_var + eps)?;
        let std_inv = var_eps.sqrt()?.recip()?;
        let x_hat = x_norm.broadcast_mul(&std_inv)?;
        let x_bn = x_hat
            .broadcast_mul(&self.bn_weight)?
            .broadcast_add(&self.bn_bias)?;

        // FC1: Linear(300, 256) + ReLU
        let h1 = x_bn
            .matmul(&self.fc1_weight.t()?)?
            .broadcast_add(&self.fc1_bias)?
            .relu()?;

        // FC2: Linear(256, 256) + ReLU
        let h2 = h1
            .matmul(&self.fc2_weight.t()?)?
            .broadcast_add(&self.fc2_bias)?
            .relu()?;

        // FC3: Linear(256, 128) + ReLU
        let h3 = h2
            .matmul(&self.fc3_weight.t()?)?
            .broadcast_add(&self.fc3_bias)?
            .relu()?;

        // FC4: Linear(128, 4) — raw logits
        let logits = h3
            .matmul(&self.fc4_weight.t()?)?
            .broadcast_add(&self.fc4_bias)?;

        // Squeeze back to [n_classes]
        let logits = logits.squeeze(0)?;

        // Softmax over classes
        let max_val = logits.max(0)?;
        let shifted = logits.broadcast_sub(&max_val)?;
        let exp = shifted.exp()?;
        let sum_exp = exp.sum_all()?;
        let probs = exp.broadcast_div(&sum_exp)?;

        Ok(probs)
    }
}

impl EntityClassifier {
    /// Load from a directory containing model.safetensors and config.json.
    ///
    /// Requires a shared tokenizer and embedding matrix (typically from SemanticHintClassifier).
    pub fn load<P: AsRef<Path>>(
        model_dir: P,
        tokenizer: tokenizers::Tokenizer,
        embeddings: Tensor,
    ) -> Result<Self, InferenceError> {
        let dir = model_dir.as_ref();
        let model_bytes = std::fs::read(dir.join("model.safetensors"))?;
        let config_bytes = std::fs::read(dir.join("config.json"))?;

        Self::from_bytes(&model_bytes, &config_bytes, tokenizer, embeddings)
    }

    /// Load from in-memory byte slices (for compile-time embedding).
    pub fn from_bytes(
        model_bytes: &[u8],
        config_bytes: &[u8],
        tokenizer: tokenizers::Tokenizer,
        embeddings: Tensor,
    ) -> Result<Self, InferenceError> {
        let device = Device::Cpu;

        // Parse config for threshold
        let config: serde_json::Value = serde_json::from_slice(config_bytes).map_err(|e| {
            InferenceError::InvalidPath(format!("Failed to parse entity classifier config: {}", e))
        })?;

        let threshold = config
            .get("demotion_threshold")
            .and_then(|v| v.as_f64())
            .map(|v| v as f32)
            .unwrap_or(DEFAULT_DEMOTION_THRESHOLD);

        let mlp = MlpWeights::from_bytes(model_bytes, &device)?;

        Ok(Self {
            embeddings: embeddings.to_dtype(DType::F32)?,
            tokenizer,
            mlp,
            threshold,
            patterns: DomainPatterns::new(),
            device,
        })
    }

    /// Load from shared Model2Vec resources plus entity classifier byte slices.
    ///
    /// The tokenizer and embedding matrix are cloned from `resources` (O(1)
    /// for the Tensor due to Arc-backed storage). The MLP weights and config
    /// are loaded from the provided bytes.
    pub fn from_shared(
        model_bytes: &[u8],
        config_bytes: &[u8],
        resources: &Model2VecResources,
    ) -> Result<Self, InferenceError> {
        Self::from_bytes(
            model_bytes,
            config_bytes,
            resources.tokenizer().clone(),
            resources.embeddings().clone(),
        )
    }

    /// Load from a directory using shared Model2Vec resources.
    ///
    /// Like [`EntityClassifier::load`] but takes shared resources instead of
    /// requiring a separate tokenizer and embedding clone.
    pub fn load_shared<P: AsRef<Path>>(
        model_dir: P,
        resources: &Model2VecResources,
    ) -> Result<Self, InferenceError> {
        let dir = model_dir.as_ref();
        let model_bytes = std::fs::read(dir.join("model.safetensors"))?;
        let config_bytes = std::fs::read(dir.join("config.json"))?;

        Self::from_shared(&model_bytes, &config_bytes, resources)
    }

    /// Determine whether a column of values should be demoted from full_name to entity_name.
    ///
    /// Returns `true` if the entity classifier confidently predicts that the column
    /// contains non-person entities (places, organizations, creative works).
    pub fn should_demote(&self, values: &[String]) -> Result<bool, InferenceError> {
        if values.is_empty() {
            return Ok(false);
        }

        let features = self.compute_features(values)?;
        let probs = self.mlp.forward(&features)?;
        let probs_vec: Vec<f32> = probs.to_vec1()?;

        // Binary decision: if max non-person probability > threshold, demote
        let max_nonperson = probs_vec[PERSON_CLASS_INDEX + 1..]
            .iter()
            .cloned()
            .fold(f32::NEG_INFINITY, f32::max);

        Ok(max_nonperson > self.threshold)
    }

    /// Compute the 300-dim feature vector for a column of values.
    ///
    /// Layout: [emb_mean (128), emb_std (128), stat_features (44)]
    fn compute_features(&self, values: &[String]) -> Result<Tensor, InferenceError> {
        let embed_dim = self.embeddings.dim(1)?;

        // Encode all values with Model2Vec: tokenize → index_select → mean_pool
        let mut value_embeddings: Vec<Vec<f32>> = Vec::with_capacity(values.len());

        for value in values {
            let encoding = self.tokenizer.encode(value.as_str(), false).map_err(|e| {
                InferenceError::InvalidPath(format!("Tokenizer encode failed: {}", e))
            })?;

            let ids = encoding.get_ids();
            // Filter PAD tokens (id=0)
            let valid_ids: Vec<u32> = ids.iter().copied().filter(|&id| id != 0).collect();

            if valid_ids.is_empty() {
                // Zero embedding for empty/untokenizable values
                value_embeddings.push(vec![0.0; embed_dim]);
                continue;
            }

            let id_tensor = Tensor::new(valid_ids.as_slice(), &self.device)?;
            let token_embeds = self.embeddings.index_select(&id_tensor, 0)?;
            let mean_embed = token_embeds.mean(0)?;
            value_embeddings.push(mean_embed.to_vec1()?);
        }

        let n = value_embeddings.len() as f32;

        // Compute mean embedding across all values
        let mut emb_mean = vec![0.0f32; embed_dim];
        for emb in &value_embeddings {
            for (i, v) in emb.iter().enumerate() {
                emb_mean[i] += v;
            }
        }
        for v in &mut emb_mean {
            *v /= n;
        }

        // Compute std embedding (population std, matching Python numpy.std default)
        let mut emb_std = vec![0.0f32; embed_dim];
        if value_embeddings.len() > 1 {
            for emb in &value_embeddings {
                for (i, v) in emb.iter().enumerate() {
                    let diff = v - emb_mean[i];
                    emb_std[i] += diff * diff;
                }
            }
            for v in &mut emb_std {
                *v = (*v / n).sqrt();
            }
        }

        // Compute 44 statistical features
        let stat_features = self.compute_stat_features(values);

        // Concatenate: [emb_mean (128), emb_std (128), stat_features (44)] = 300-dim
        let mut feature_vec = Vec::with_capacity(embed_dim * 2 + N_STAT_FEATURES);
        feature_vec.extend_from_slice(&emb_mean);
        feature_vec.extend_from_slice(&emb_std);
        feature_vec.extend_from_slice(&stat_features);

        Tensor::from_vec(
            feature_vec,
            (embed_dim * 2 + N_STAT_FEATURES,),
            &self.device,
        )
        .map_err(Into::into)
    }

    /// Compute 44 statistical features for a column of string values.
    ///
    /// Must match `compute_column_features()` in `scripts/train_entity_classifier.py`.
    /// Feature order matches `config.json:stat_feature_names`.
    fn compute_stat_features(&self, values: &[String]) -> Vec<f32> {
        let n = values.len() as f32;
        if values.is_empty() {
            return vec![0.0; N_STAT_FEATURES];
        }

        let lengths: Vec<f32> = values.iter().map(|v| v.len() as f32).collect();
        let word_counts: Vec<f32> = values
            .iter()
            .map(|v| v.split_whitespace().count() as f32)
            .collect();

        // Length distribution (5)
        let mean_len = mean(&lengths);
        let std_len = std_dev(&lengths, mean_len);
        let median_len = median(&lengths);
        let p25_len = percentile(&lengths, 25.0);
        let p75_len = percentile(&lengths, 75.0);

        // Word count distribution (5)
        let mean_words = mean(&word_counts);
        let std_words = std_dev(&word_counts, mean_words);
        let single_word_ratio = word_counts.iter().filter(|&&w| w == 1.0).count() as f32 / n;
        let two_word_ratio = word_counts.iter().filter(|&&w| w == 2.0).count() as f32 / n;
        let three_plus_ratio = word_counts.iter().filter(|&&w| w >= 3.0).count() as f32 / n;

        // Character class ratios (5)
        let mut alpha_ratios = Vec::with_capacity(values.len());
        let mut digit_ratios = Vec::with_capacity(values.len());
        let mut space_ratios = Vec::with_capacity(values.len());
        let mut punct_ratios = Vec::with_capacity(values.len());
        let mut has_digits_count = 0usize;

        for v in values {
            let len = v.len() as f32;
            if len == 0.0 {
                alpha_ratios.push(0.0);
                digit_ratios.push(0.0);
                space_ratios.push(0.0);
                punct_ratios.push(0.0);
                continue;
            }
            let mut alpha = 0usize;
            let mut digit = 0usize;
            let mut space = 0usize;
            let mut has_digit = false;
            for c in v.chars() {
                if c.is_alphabetic() {
                    alpha += 1;
                } else if c.is_ascii_digit() {
                    digit += 1;
                    has_digit = true;
                } else if c.is_whitespace() {
                    space += 1;
                }
            }
            let a = alpha as f32 / len;
            let d = digit as f32 / len;
            let s = space as f32 / len;
            alpha_ratios.push(a);
            digit_ratios.push(d);
            space_ratios.push(s);
            punct_ratios.push(1.0 - a - d - s);
            if has_digit {
                has_digits_count += 1;
            }
        }

        let mean_alpha_ratio = mean(&alpha_ratios);
        let mean_digit_ratio = mean(&digit_ratios);
        let mean_space_ratio = mean(&space_ratios);
        let mean_punct_ratio = mean(&punct_ratios);
        let has_digits_ratio = has_digits_count as f32 / n;

        // Structural patterns (8)
        let title_case_ratio = values.iter().filter(|v| is_title_case(v)).count() as f32 / n;
        let all_caps_ratio = values
            .iter()
            .filter(|v| {
                let upper = v.to_uppercase();
                let lower = v.to_lowercase();
                v.as_str() == upper && v.as_str() != lower
            })
            .count() as f32
            / n;
        let has_comma_ratio = values.iter().filter(|v| v.contains(',')).count() as f32 / n;
        let has_parens_ratio = values
            .iter()
            .filter(|v| v.contains('(') || v.contains(')'))
            .count() as f32
            / n;
        let has_ampersand_ratio = values.iter().filter(|v| v.contains('&')).count() as f32 / n;
        let has_apostrophe_ratio = values
            .iter()
            .filter(|v| v.contains('\'') || v.contains('\u{2019}'))
            .count() as f32
            / n;
        let has_hyphen_ratio = values.iter().filter(|v| v.contains('-')).count() as f32 / n;
        let has_dot_ratio = values.iter().filter(|v| v.contains('.')).count() as f32 / n;

        // Domain patterns (6)
        let org_suffix_ratio = values
            .iter()
            .filter(|v| self.patterns.org_suffixes.is_match(v))
            .count() as f32
            / n;
        let person_title_ratio = values
            .iter()
            .filter(|v| self.patterns.person_titles.is_match(v))
            .count() as f32
            / n;
        let place_indicator_ratio = values
            .iter()
            .filter(|v| self.patterns.place_indicators.is_match(v))
            .count() as f32
            / n;
        let creative_indicator_ratio = values
            .iter()
            .filter(|v| self.patterns.creative_indicators.is_match(v))
            .count() as f32
            / n;
        let the_prefix_ratio = values
            .iter()
            .filter(|v| v.to_lowercase().starts_with("the "))
            .count() as f32
            / n;
        let numeric_prefix_ratio = values
            .iter()
            .filter(|v| v.starts_with(|c: char| c.is_ascii_digit()))
            .count() as f32
            / n;

        // Value diversity (5): uniqueness, token_diversity, avg_word_len, cap_words_mean, cap_word_ratio
        let unique_count = {
            let mut seen = std::collections::HashSet::new();
            for v in values {
                seen.insert(v.as_str());
            }
            seen.len()
        };
        let uniqueness = unique_count as f32 / n;

        let all_words: Vec<&str> = values.iter().flat_map(|v| v.split_whitespace()).collect();
        let token_diversity = if all_words.is_empty() {
            0.0
        } else {
            let unique_words: std::collections::HashSet<&str> = all_words.iter().copied().collect();
            unique_words.len() as f32 / all_words.len() as f32
        };

        let avg_word_len = if all_words.is_empty() {
            0.0
        } else {
            all_words.iter().map(|w| w.len() as f32).sum::<f32>() / all_words.len() as f32
        };

        let cap_words_per_value: Vec<f32> = values
            .iter()
            .map(|v| {
                v.split_whitespace()
                    .filter(|w| w.starts_with(|c: char| c.is_uppercase()))
                    .count() as f32
            })
            .collect();
        let cap_words_mean = mean(&cap_words_per_value);

        let cap_word_ratio_values: Vec<f32> = values
            .iter()
            .map(|v| {
                let words: Vec<&str> = v.split_whitespace().collect();
                if words.is_empty() {
                    return 0.0;
                }
                let cap = words
                    .iter()
                    .filter(|w| w.starts_with(|c: char| c.is_uppercase()))
                    .count() as f32;
                cap / words.len() as f32
            })
            .collect();
        let cap_word_ratio = mean(&cap_word_ratio_values);

        // Distributional shape (7)
        let word_density = if mean_len > 0.0 {
            mean_words / mean_len
        } else {
            0.0
        };
        let short_value_ratio = lengths.iter().filter(|&&l| l <= 3.0).count() as f32 / n;
        let long_value_ratio = lengths.iter().filter(|&&l| l > 50.0).count() as f32 / n;
        let cv_length = if mean_len > 0.0 {
            std_len / mean_len
        } else {
            0.0
        };
        let preposition_ratio = values
            .iter()
            .filter(|v| self.patterns.prepositions.is_match(v))
            .count() as f32
            / n;
        let contains_number_ratio = values
            .iter()
            .filter(|v| self.patterns.digits.is_match(v))
            .count() as f32
            / n;
        let has_quotes_ratio = values
            .iter()
            .filter(|v| {
                v.contains('"')
                    || v.contains('\'')
                    || v.contains('\u{00ab}')
                    || v.contains('\u{00bb}')
            })
            .count() as f32
            / n;

        // Column metadata (3)
        let column_size = n;
        let max_value_len = lengths.iter().cloned().fold(0.0f32, f32::max);
        let max_word_count = word_counts.iter().cloned().fold(0.0f32, f32::max);

        // Must match order in config.json:stat_feature_names
        vec![
            // Length distribution (5)
            mean_len,
            std_len,
            median_len,
            p25_len,
            p75_len,
            // Word count distribution (5)
            mean_words,
            std_words,
            single_word_ratio,
            two_word_ratio,
            three_plus_ratio,
            // Character class ratios (5)
            mean_alpha_ratio,
            mean_digit_ratio,
            mean_space_ratio,
            mean_punct_ratio,
            has_digits_ratio,
            // Structural patterns (8)
            title_case_ratio,
            all_caps_ratio,
            has_comma_ratio,
            has_parens_ratio,
            has_ampersand_ratio,
            has_apostrophe_ratio,
            has_hyphen_ratio,
            has_dot_ratio,
            // Domain patterns (6)
            org_suffix_ratio,
            person_title_ratio,
            place_indicator_ratio,
            creative_indicator_ratio,
            the_prefix_ratio,
            numeric_prefix_ratio,
            // Value diversity (5)
            uniqueness,
            token_diversity,
            avg_word_len,
            cap_words_mean,
            cap_word_ratio,
            // Distributional shape (7)
            word_density,
            short_value_ratio,
            long_value_ratio,
            cv_length,
            preposition_ratio,
            contains_number_ratio,
            has_quotes_ratio,
            // Column metadata (3)
            column_size,
            max_value_len,
            max_word_count,
        ]
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Statistical helpers
// ═══════════════════════════════════════════════════════════════════════════════

fn mean(v: &[f32]) -> f32 {
    if v.is_empty() {
        return 0.0;
    }
    v.iter().sum::<f32>() / v.len() as f32
}

/// Population standard deviation (numpy default).
fn std_dev(v: &[f32], mean: f32) -> f32 {
    if v.is_empty() {
        return 0.0;
    }
    let variance = v.iter().map(|x| (x - mean).powi(2)).sum::<f32>() / v.len() as f32;
    variance.sqrt()
}

fn median(v: &[f32]) -> f32 {
    if v.is_empty() {
        return 0.0;
    }
    let mut sorted = v.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = sorted.len();
    if n.is_multiple_of(2) {
        (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
    } else {
        sorted[n / 2]
    }
}

/// Linear interpolation percentile matching numpy.percentile default (linear method).
fn percentile(v: &[f32], pct: f32) -> f32 {
    if v.is_empty() {
        return 0.0;
    }
    let mut sorted = v.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = sorted.len();
    if n == 1 {
        return sorted[0];
    }
    let rank = pct / 100.0 * (n - 1) as f32;
    let lower = rank.floor() as usize;
    let upper = rank.ceil() as usize;
    if lower == upper {
        sorted[lower]
    } else {
        let frac = rank - lower as f32;
        sorted[lower] * (1.0 - frac) + sorted[upper] * frac
    }
}

/// Check if a string is title case (>70% of alphabetic words start with uppercase).
/// Matches the Python `is_title_case()` function.
fn is_title_case(s: &str) -> bool {
    let words: Vec<&str> = s
        .split_whitespace()
        .filter(|w| w.starts_with(|c: char| c.is_alphabetic()))
        .collect();
    if words.is_empty() {
        return false;
    }
    let upper_count = words
        .iter()
        .filter(|w| w.starts_with(|c: char| c.is_uppercase()))
        .count();
    upper_count as f32 / words.len() as f32 > 0.7
}

// ═══════════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stat_helpers() {
        assert!((mean(&[1.0, 2.0, 3.0]) - 2.0).abs() < 1e-5);
        assert!((std_dev(&[1.0, 2.0, 3.0], 2.0) - 0.8165).abs() < 0.001);
        assert!((median(&[1.0, 2.0, 3.0]) - 2.0).abs() < 1e-5);
        assert!((median(&[1.0, 2.0, 3.0, 4.0]) - 2.5).abs() < 1e-5);
        assert!((percentile(&[1.0, 2.0, 3.0, 4.0], 25.0) - 1.75).abs() < 1e-5);
        assert!((percentile(&[1.0, 2.0, 3.0, 4.0], 75.0) - 3.25).abs() < 1e-5);
    }

    #[test]
    fn test_is_title_case() {
        assert!(is_title_case("John Smith"));
        assert!(is_title_case("New York City"));
        assert!(!is_title_case("hello world"));
        // "HELLO WORLD" is title case by this definition: both words start with uppercase,
        // so 2/2 = 1.0 > 0.7. The all_caps_ratio feature captures this separately.
        assert!(is_title_case("HELLO WORLD"));
        assert!(!is_title_case("123 456")); // no alphabetic words
        assert!(!is_title_case("")); // empty
    }

    #[test]
    fn test_stat_feature_count() {
        let patterns = DomainPatterns::new();
        // Create a minimal EntityClassifier-like context to test features
        // We'll test the feature count directly
        let values = vec![
            "John Smith".to_string(),
            "Jane Doe".to_string(),
            "Robert Johnson".to_string(),
        ];

        // Create a dummy classifier just to test feature computation
        // We can't easily do this without a full model, so test the helper functions
        let lengths: Vec<f32> = values.iter().map(|v| v.len() as f32).collect();
        assert_eq!(lengths.len(), 3);

        // Test domain patterns — thorough coverage including multi-line regex fix
        assert!(patterns.person_titles.is_match("Dr. Smith"));
        assert!(patterns.person_titles.is_match("John Jr."));
        assert!(!patterns.person_titles.is_match("John Smith"));

        assert!(patterns.org_suffixes.is_match("Acme Inc"));
        assert!(patterns.org_suffixes.is_match("MIT University")); // second line of regex
        assert!(patterns.org_suffixes.is_match("Royal Hotel")); // third line
        assert!(patterns.org_suffixes.is_match("Smith & Co GmbH")); // fourth line
        assert!(!patterns.org_suffixes.is_match("John Smith"));

        assert!(patterns.place_indicators.is_match("123 Main Street"));
        assert!(patterns.place_indicators.is_match("Hyde Park")); // second line
        assert!(patterns.place_indicators.is_match("Pearl Bay")); // third line
        assert!(!patterns.place_indicators.is_match("Acme Inc"));

        assert!(patterns.creative_indicators.is_match("Greatest Hits Album"));
        assert!(patterns.creative_indicators.is_match("Live at the Apollo")); // second line
        assert!(patterns.creative_indicators.is_match("Summer Concert")); // second line
        assert!(!patterns.creative_indicators.is_match("John Smith"));

        assert!(patterns.prepositions.is_match("King of the Hill"));
    }

    /// Integration test: load real model artifacts from disk (skip if not present).
    #[test]
    fn test_load_entity_classifier_if_available() {
        let workspace_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf();

        let model_dir = workspace_root.join("models").join("entity-classifier");
        let m2v_dir = workspace_root.join("models").join("model2vec");

        if !model_dir.join("model.safetensors").exists()
            || !m2v_dir.join("model.safetensors").exists()
        {
            eprintln!("Skipping entity classifier integration test: model artifacts not found");
            return;
        }

        // Load Model2Vec tokenizer and embeddings
        let semantic = crate::semantic::SemanticHintClassifier::load(&m2v_dir).unwrap();

        let classifier = EntityClassifier::load(
            &model_dir,
            semantic.tokenizer().clone(),
            semantic.embeddings().clone(),
        )
        .unwrap();

        // Test with person-like column
        let person_values: Vec<String> = vec![
            "John Smith",
            "Jane Doe",
            "Robert Johnson",
            "Mary Williams",
            "James Brown",
            "Patricia Davis",
            "Michael Miller",
            "Jennifer Wilson",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        let demote_person = classifier.should_demote(&person_values).unwrap();
        // Person column should NOT be demoted
        assert!(
            !demote_person,
            "Person names should not be demoted to entity_name"
        );

        // Test with organization-like column
        let org_values: Vec<String> = vec![
            "Google Inc",
            "Microsoft Corp",
            "Amazon LLC",
            "Apple Inc",
            "Meta Platforms Inc",
            "Tesla Inc",
            "Netflix Inc",
            "Spotify Ltd",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        let demote_org = classifier.should_demote(&org_values).unwrap();
        // Organization column SHOULD be demoted
        assert!(
            demote_org,
            "Organization names should be demoted to entity_name"
        );
    }

    /// Integration test: from_shared() produces identical results to load().
    #[test]
    fn test_from_shared_matches_load() {
        let workspace_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf();

        let model_dir = workspace_root.join("models").join("entity-classifier");
        let m2v_dir = workspace_root.join("models").join("model2vec");

        if !model_dir.join("model.safetensors").exists()
            || !m2v_dir.join("model.safetensors").exists()
        {
            eprintln!("Skipping from_shared integration test: model artifacts not found");
            return;
        }

        // Load via existing path (SemanticHintClassifier → clone tok/emb)
        let semantic = crate::semantic::SemanticHintClassifier::load(&m2v_dir).unwrap();
        let standalone = EntityClassifier::load(
            &model_dir,
            semantic.tokenizer().clone(),
            semantic.embeddings().clone(),
        )
        .unwrap();

        // Load via shared resources (new path)
        let resources = Model2VecResources::load(&m2v_dir).unwrap();
        let shared = EntityClassifier::load_shared(&model_dir, &resources).unwrap();

        // Both should produce identical demotion decisions
        let person_values: Vec<String> = vec![
            "John Smith",
            "Jane Doe",
            "Robert Johnson",
            "Mary Williams",
            "James Brown",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        let org_values: Vec<String> = vec![
            "Google Inc",
            "Microsoft Corp",
            "Amazon LLC",
            "Apple Inc",
            "Tesla Inc",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        assert_eq!(
            standalone.should_demote(&person_values).unwrap(),
            shared.should_demote(&person_values).unwrap(),
            "Person demotion mismatch between standalone and shared"
        );
        assert_eq!(
            standalone.should_demote(&org_values).unwrap(),
            shared.should_demote(&org_values).unwrap(),
            "Org demotion mismatch between standalone and shared"
        );
    }
}
