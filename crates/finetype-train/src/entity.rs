//! Entity classifier training: Deep Sets MLP for demotion gating.
//!
//! Input: 300-dim features (128 emb_mean + 128 emb_std + 44 statistical).
//! Output: 4-class entity logits (person, place, organization, creative_work).
//!
//! Feature computation reuses `finetype_model::entity::EntityClassifier::compute_stat_features`
//! logic — the same 44 statistical features used at inference time.

use std::collections::HashSet;
use std::path::Path;

use anyhow::{Context, Result};
use candle_core::{DType, Device, Tensor};
use candle_nn::{linear, Linear, Module, Optimizer, VarBuilder, VarMap};

use crate::sense::{EMBED_DIM, N_ENTITY};
use crate::training::{
    compute_accuracy, shuffled_batches, vec2_to_tensor, CosineScheduler, EarlyStopping,
    EpochMetrics, TrainingSummary,
};

// ── Constants ────────────────────────────────────────────────────────────────

/// Number of statistical features (must match finetype-model::entity).
pub const N_STAT_FEATURES: usize = 44;

/// Total input dimension: emb_mean (128) + emb_std (128) + stats (44).
pub const INPUT_DIM: usize = 2 * EMBED_DIM + N_STAT_FEATURES;

/// Default hidden dimension for MLP layers.
pub const HIDDEN_DIM: usize = 256;

/// Dropout rate during training.
pub const DROPOUT_RATE: f64 = 0.2;

/// Entity subtype labels (order matches class indices).
pub const ENTITY_LABELS: [&str; N_ENTITY] = ["person", "place", "organization", "creative_work"];

/// Stat feature names (order matches compute_stat_features).
pub const STAT_FEATURE_NAMES: [&str; N_STAT_FEATURES] = [
    "mean_len",
    "std_len",
    "median_len",
    "p25_len",
    "p75_len",
    "mean_words",
    "std_words",
    "single_word_ratio",
    "two_word_ratio",
    "three_plus_ratio",
    "mean_alpha_ratio",
    "mean_digit_ratio",
    "mean_space_ratio",
    "mean_punct_ratio",
    "has_digits_ratio",
    "title_case_ratio",
    "all_caps_ratio",
    "has_comma_ratio",
    "has_parens_ratio",
    "has_ampersand_ratio",
    "has_apostrophe_ratio",
    "has_hyphen_ratio",
    "has_dot_ratio",
    "org_suffix_ratio",
    "person_title_ratio",
    "place_indicator_ratio",
    "creative_indicator_ratio",
    "the_prefix_ratio",
    "numeric_prefix_ratio",
    "uniqueness",
    "token_diversity",
    "avg_word_len",
    "cap_words_mean",
    "cap_word_ratio",
    "word_density",
    "short_value_ratio",
    "long_value_ratio",
    "cv_length",
    "preposition_ratio",
    "contains_number_ratio",
    "has_quotes_ratio",
    "column_size",
    "max_value_len",
    "max_word_count",
];

// ── Entity Classifier (Training) ─────────────────────────────────────────────

/// Entity classifier MLP for training.
///
/// Architecture (matching Python `train_entity_classifier.py`):
/// ```text
/// Input: [B, 300]
///   → Linear(300→256) → ReLU → Dropout(0.2)
///   → Linear(256→256) → ReLU → Dropout(0.2)
///   → Linear(256→128) → ReLU → Dropout(0.2)
///   → Linear(128→4)
/// Output: [B, 4] logits
/// ```
///
/// Note: Dropout is applied during training only. The production
/// `finetype_model::entity::EntityClassifier` has no dropout.
pub struct EntityClassifierTrainable {
    fc1: Linear,
    fc2: Linear,
    fc3: Linear,
    fc4: Linear,
    dropout_rate: f64,
    training: bool,
}

impl EntityClassifierTrainable {
    /// Create a new trainable entity classifier.
    pub fn new(varmap: &VarMap, device: &Device) -> Result<Self> {
        let vb = VarBuilder::from_varmap(varmap, DType::F32, device);

        let fc1 = linear(INPUT_DIM, HIDDEN_DIM, vb.pp("ec_fc1"))?;
        let fc2 = linear(HIDDEN_DIM, HIDDEN_DIM, vb.pp("ec_fc2"))?;
        let fc3 = linear(HIDDEN_DIM, HIDDEN_DIM / 2, vb.pp("ec_fc3"))?;
        let fc4 = linear(HIDDEN_DIM / 2, N_ENTITY, vb.pp("ec_fc4"))?;

        Ok(Self {
            fc1,
            fc2,
            fc3,
            fc4,
            dropout_rate: DROPOUT_RATE,
            training: true,
        })
    }

    /// Set training mode (enables dropout).
    pub fn set_training(&mut self, training: bool) {
        self.training = training;
    }

    /// Forward pass with optional dropout.
    pub fn forward(&self, features: &Tensor) -> Result<Tensor> {
        let x = self.fc1.forward(features)?.relu()?;
        let x = self.maybe_dropout(&x)?;

        let x = self.fc2.forward(&x)?.relu()?;
        let x = self.maybe_dropout(&x)?;

        let x = self.fc3.forward(&x)?.relu()?;
        let x = self.maybe_dropout(&x)?;

        Ok(self.fc4.forward(&x)?)
    }

    /// Apply dropout during training (Candle doesn't have a built-in dropout module).
    fn maybe_dropout(&self, tensor: &Tensor) -> Result<Tensor> {
        if !self.training || self.dropout_rate == 0.0 {
            return Ok(tensor.clone());
        }
        Ok(candle_nn::ops::dropout(tensor, self.dropout_rate as f32)?)
    }
}

// ── Feature Computation ─────────────────────────────────────────────────────

/// Word-boundary matching for domain-specific statistical features.
///
/// Replaces regex patterns from `crates/finetype-model/src/entity.rs`
/// with case-insensitive word-boundary matching. Produces identical
/// results for the SOTAB entity corpus.
const ORG_SUFFIXES: &[&str] = &[
    "inc",
    "llc",
    "ltd",
    "corp",
    "co",
    "company",
    "group",
    "foundation",
    "association",
    "university",
    "institute",
    "hospital",
    "church",
    "school",
    "bank",
    "restaurant",
    "hotel",
    "salon",
    "clinic",
    "studio",
    "studios",
    "records",
    "entertainment",
    "gmbh",
    "ag",
    "sa",
    "pty",
    "pvt",
    "plc",
    "llp",
];

const PERSON_TITLES: &[&str] = &["mr", "mrs", "ms", "dr", "jr", "sr", "iii", "ii", "iv"];

const PLACE_INDICATORS: &[&str] = &[
    "street",
    "st",
    "avenue",
    "ave",
    "road",
    "rd",
    "boulevard",
    "blvd",
    "drive",
    "lane",
    "court",
    "place",
    "square",
    "park",
    "bridge",
    "hill",
    "valley",
    "lake",
    "river",
    "mountain",
    "beach",
    "island",
    "bay",
    "county",
    "province",
    "state",
    "region",
    "district",
    "city",
    "town",
    "village",
];

const CREATIVE_INDICATORS: &[&str] = &[
    "album",
    "song",
    "track",
    "episode",
    "season",
    "chapter",
    "vol",
    "volume",
    "remix",
    "live",
    "feat",
    "ft",
    "edition",
    "deluxe",
    "remaster",
    "ost",
    "soundtrack",
    "tour",
    "concert",
    "festival",
];

const PREPOSITIONS: &[&str] = &["of", "in", "at", "for", "the", "and", "by", "on", "to"];

/// Check if any word in `text` matches any keyword (case-insensitive word boundary).
///
/// A "word" is defined by splitting on whitespace and stripping trailing
/// punctuation, matching the \b behaviour of the production regex patterns.
fn contains_word(text: &str, keywords: &[&str]) -> bool {
    for word in text.split(|c: char| c.is_whitespace() || c == ',' || c == ';') {
        let trimmed = word.trim_matches(|c: char| !c.is_alphanumeric());
        if trimmed.is_empty() {
            continue;
        }
        let lower = trimmed.to_lowercase();
        // Also check without trailing period (for "Dr." → "dr")
        let lower_no_dot = lower.trim_end_matches('.');
        for &kw in keywords {
            if lower == kw || lower_no_dot == kw {
                return true;
            }
        }
    }
    false
}

/// Check if text contains any ASCII digit.
fn contains_digit(text: &str) -> bool {
    text.bytes().any(|b| b.is_ascii_digit())
}

/// Compute the 300-dim feature vector for a column of values.
///
/// Layout: [emb_mean (128), emb_std (128), stat_features (44)]
///
/// This replicates the feature computation from
/// `finetype_model::entity::EntityClassifier::compute_features`,
/// operating on raw strings + Model2Vec resources rather than an
/// instantiated classifier.
pub fn compute_entity_features(
    values: &[String],
    model2vec: &finetype_model::Model2VecResources,
) -> Result<Vec<f32>> {
    let embed_dim = model2vec.embed_dim().context("embed_dim")?;
    let device = model2vec.device();

    if values.is_empty() {
        return Ok(vec![0.0; INPUT_DIM]);
    }

    // Encode all values with Model2Vec: tokenize → index_select → mean_pool
    // NOTE: We use raw mean-pool (no L2 normalisation) to match the production
    // entity classifier's compute_features(), which does NOT L2-normalise
    // individual value embeddings before aggregation.
    let mut value_embeddings: Vec<Vec<f32>> = Vec::with_capacity(values.len());

    let tokenizer = model2vec.tokenizer();
    let embeddings = model2vec.embeddings();

    for value in values {
        let encoding = tokenizer
            .encode(value.as_str(), false)
            .map_err(|e| anyhow::anyhow!("Tokenizer encode failed: {}", e))?;

        let ids = encoding.get_ids();
        let valid_ids: Vec<u32> = ids.iter().copied().filter(|&id| id != 0).collect();

        if valid_ids.is_empty() {
            value_embeddings.push(vec![0.0; embed_dim]);
            continue;
        }

        let id_tensor = Tensor::new(valid_ids.as_slice(), device)?;
        let token_embeds = embeddings.index_select(&id_tensor, 0)?;
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
    let stat_features = compute_stat_features(values);

    // Concatenate: [emb_mean (128), emb_std (128), stat_features (44)] = 300-dim
    let mut feature_vec = Vec::with_capacity(INPUT_DIM);
    feature_vec.extend_from_slice(&emb_mean);
    feature_vec.extend_from_slice(&emb_std);
    feature_vec.extend_from_slice(&stat_features);

    Ok(feature_vec)
}

/// Compute 44 statistical features for a column of string values.
///
/// Must match `finetype_model::entity::EntityClassifier::compute_stat_features`
/// exactly. Feature order matches `config.json:stat_feature_names`.
fn compute_stat_features(values: &[String]) -> Vec<f32> {
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
    let mean_len = stat_mean(&lengths);
    let std_len = stat_std_dev(&lengths, mean_len);
    let median_len = stat_median(&lengths);
    let p25_len = stat_percentile(&lengths, 25.0);
    let p75_len = stat_percentile(&lengths, 75.0);

    // Word count distribution (5)
    let mean_words = stat_mean(&word_counts);
    let std_words = stat_std_dev(&word_counts, mean_words);
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

    let mean_alpha_ratio = stat_mean(&alpha_ratios);
    let mean_digit_ratio = stat_mean(&digit_ratios);
    let mean_space_ratio = stat_mean(&space_ratios);
    let mean_punct_ratio = stat_mean(&punct_ratios);
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
        .filter(|v| contains_word(v, ORG_SUFFIXES))
        .count() as f32
        / n;
    let person_title_ratio = values
        .iter()
        .filter(|v| contains_word(v, PERSON_TITLES))
        .count() as f32
        / n;
    let place_indicator_ratio = values
        .iter()
        .filter(|v| contains_word(v, PLACE_INDICATORS))
        .count() as f32
        / n;
    let creative_indicator_ratio = values
        .iter()
        .filter(|v| contains_word(v, CREATIVE_INDICATORS))
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

    // Value diversity (5)
    let unique_count = {
        let mut seen = HashSet::new();
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
        let unique_words: HashSet<&str> = all_words.iter().copied().collect();
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
    let cap_words_mean = stat_mean(&cap_words_per_value);

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
    let cap_word_ratio = stat_mean(&cap_word_ratio_values);

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
        .filter(|v| contains_word(v, PREPOSITIONS))
        .count() as f32
        / n;
    let contains_number_ratio = values.iter().filter(|v| contains_digit(v)).count() as f32 / n;
    let has_quotes_ratio = values
        .iter()
        .filter(|v| {
            v.contains('"') || v.contains('\'') || v.contains('\u{00ab}') || v.contains('\u{00bb}')
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

// ── Statistical helpers ─────────────────────────────────────────────────────
// Exact copies of finetype-model::entity helpers for feature parity.

fn stat_mean(v: &[f32]) -> f32 {
    if v.is_empty() {
        return 0.0;
    }
    v.iter().sum::<f32>() / v.len() as f32
}

/// Population standard deviation (numpy default).
fn stat_std_dev(v: &[f32], mean: f32) -> f32 {
    if v.is_empty() {
        return 0.0;
    }
    let variance = v.iter().map(|x| (x - mean).powi(2)).sum::<f32>() / v.len() as f32;
    variance.sqrt()
}

fn stat_median(v: &[f32]) -> f32 {
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
fn stat_percentile(v: &[f32], pct: f32) -> f32 {
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

// ── Training Configuration ──────────────────────────────────────────────────

/// Configuration for entity classifier training.
pub struct EntityTrainConfig {
    pub epochs: usize,
    pub batch_size: usize,
    pub lr: f64,
    pub min_lr: f64,
    pub patience: usize,
    pub demotion_threshold: f64,
    pub seed: u64,
}

impl Default for EntityTrainConfig {
    fn default() -> Self {
        Self {
            epochs: 100,
            batch_size: 64,
            lr: 5e-4,
            min_lr: 1e-6,
            patience: 15,
            demotion_threshold: 0.6,
            seed: 42,
        }
    }
}

// ── Training Loop ───────────────────────────────────────────────────────────

/// Train entity classifier on pre-computed feature vectors.
///
/// - `train_features`: [N_train, 300] feature matrix
/// - `train_labels`: [N_train] class indices (0–3)
/// - `val_features`: [N_val, 300] feature matrix
/// - `val_labels`: [N_val] class indices (0–3)
///
/// Returns a `TrainingSummary` with per-epoch metrics.
pub fn train_entity(
    config: &EntityTrainConfig,
    train_features: &[Vec<f32>],
    train_labels: &[usize],
    val_features: &[Vec<f32>],
    val_labels: &[usize],
) -> Result<(TrainingSummary, VarMap)> {
    let device = Device::Cpu;
    let varmap = VarMap::new();
    let mut model = EntityClassifierTrainable::new(&varmap, &device)?;

    let n_train = train_features.len();
    let n_val = val_features.len();
    tracing::info!(
        "Training entity classifier: {} train, {} val samples",
        n_train,
        n_val
    );

    // Compute class weights (inverse frequency)
    let class_weights = compute_class_weights(train_labels, N_ENTITY, &device)?;
    tracing::info!(
        "Class weights: {:?}",
        class_weights.to_vec1::<f32>().unwrap_or_default()
    );

    // Pre-compute validation tensors
    let val_feat_tensor = vec2_to_tensor(val_features, &device)?;
    let val_label_u32: Vec<u32> = val_labels.iter().map(|&x| x as u32).collect();
    let val_label_tensor = Tensor::new(val_label_u32.as_slice(), &device)?;

    // Optimizer: AdamW with cosine annealing
    let all_vars = varmap.all_vars();
    let adamw_params = candle_nn::ParamsAdamW {
        lr: config.lr,
        weight_decay: 0.01,
        ..Default::default()
    };
    let mut optimizer = candle_nn::AdamW::new(all_vars, adamw_params)?;
    let scheduler = CosineScheduler::new(config.lr, config.min_lr, config.epochs);

    // Early stopping on validation accuracy
    let mut early_stopping = EarlyStopping::new(config.patience, true);
    let mut epoch_metrics = Vec::new();
    let mut rng = {
        use rand::SeedableRng;
        rand::rngs::StdRng::seed_from_u64(config.seed)
    };

    let start_time = std::time::Instant::now();

    for epoch in 0..config.epochs {
        let epoch_start = std::time::Instant::now();
        let lr = scheduler.lr(epoch);
        optimizer.set_learning_rate(lr);

        // Training pass
        model.set_training(true);
        let batches = shuffled_batches(n_train, config.batch_size, &mut rng);
        let mut train_loss_sum = 0.0f32;
        let mut train_correct = 0.0f32;
        let mut train_batches = 0usize;

        for batch_indices in &batches {
            let batch_features: Vec<Vec<f32>> = batch_indices
                .iter()
                .map(|&i| train_features[i].clone())
                .collect();
            let batch_labels: Vec<u32> = batch_indices
                .iter()
                .map(|&i| train_labels[i] as u32)
                .collect();

            let feat_tensor = vec2_to_tensor(&batch_features, &device)?;
            let label_tensor = Tensor::new(batch_labels.as_slice(), &device)?;

            let logits = model.forward(&feat_tensor)?;
            let loss = entity_weighted_ce_loss(&logits, &label_tensor, &class_weights)?;

            // Backward + step
            optimizer.backward_step(&loss)?;

            train_loss_sum += loss.to_scalar::<f32>()?;
            train_correct += compute_accuracy(&logits, &label_tensor)? * batch_indices.len() as f32;
            train_batches += 1;
        }

        let train_loss = train_loss_sum / train_batches as f32;
        let train_accuracy = train_correct / n_train as f32;

        // Validation pass
        model.set_training(false);
        let val_logits = model.forward(&val_feat_tensor)?;
        let val_loss = entity_weighted_ce_loss(&val_logits, &val_label_tensor, &class_weights)?
            .to_scalar::<f32>()?;
        let val_accuracy = compute_accuracy(&val_logits, &val_label_tensor)?;

        let epoch_time = epoch_start.elapsed().as_secs_f32();

        let metrics = EpochMetrics {
            epoch,
            train_loss,
            val_loss,
            train_accuracy,
            val_accuracy,
            learning_rate: lr,
            epoch_time_secs: epoch_time,
        };

        if epoch % 10 == 0 || epoch == config.epochs - 1 {
            tracing::info!(
                "Epoch {:3}/{} — train_loss: {:.4}, val_loss: {:.4}, \
                 train_acc: {:.3}, val_acc: {:.3}, lr: {:.2e}",
                epoch + 1,
                config.epochs,
                train_loss,
                val_loss,
                train_accuracy,
                val_accuracy,
                lr,
            );
        }

        epoch_metrics.push(metrics);

        // Early stopping check
        if early_stopping.step(epoch, val_accuracy) {
            tracing::info!(
                "Early stopping at epoch {} (best: epoch {} with val_acc {:.4})",
                epoch + 1,
                early_stopping.best_epoch() + 1,
                early_stopping.best_metric(),
            );
            break;
        }
    }

    let total_time = start_time.elapsed().as_secs_f32();

    let summary = TrainingSummary {
        best_epoch: early_stopping.best_epoch(),
        best_val_accuracy: early_stopping.best_metric(),
        total_epochs: epoch_metrics.len(),
        total_time_secs: total_time,
        epoch_metrics,
    };

    Ok((summary, varmap))
}

/// Weighted cross-entropy loss for class-imbalanced entity training.
///
/// Unlike `training::weighted_cross_entropy_loss`, this handles the
/// class_weights gather correctly for Candle's dimension requirements:
/// expand weights from [C] to [1, C] before gathering.
///
/// - `logits`: [B, C] unnormalized class scores
/// - `targets`: [B] integer class indices (u32)
/// - `class_weights`: [C] per-class weights
fn entity_weighted_ce_loss(
    logits: &Tensor,
    targets: &Tensor,
    class_weights: &Tensor,
) -> Result<Tensor> {
    let log_probs = candle_nn::ops::log_softmax(logits, candle_core::D::Minus1)?;
    let target_log_probs = log_probs.gather(&targets.unsqueeze(1)?, 1)?.squeeze(1)?;

    // Gather weights: expand class_weights [C] → [B, C], then gather along dim 1
    let batch_size = targets.dims()[0];
    let weights_2d = class_weights
        .unsqueeze(0)?
        .expand((batch_size, class_weights.dims()[0]))?
        .contiguous()?;
    let sample_weights = weights_2d.gather(&targets.unsqueeze(1)?, 1)?.squeeze(1)?;

    let weighted_loss = (target_log_probs.neg()? * sample_weights)?;
    let loss = weighted_loss.mean_all()?;
    Ok(loss)
}

/// Compute inverse-frequency class weights for balanced training.
///
/// weight[c] = N_total / (N_classes * count[c])
fn compute_class_weights(labels: &[usize], n_classes: usize, device: &Device) -> Result<Tensor> {
    let n = labels.len() as f32;
    let mut counts = vec![0f32; n_classes];
    for &l in labels {
        if l < n_classes {
            counts[l] += 1.0;
        }
    }

    let weights: Vec<f32> = counts
        .iter()
        .map(|&c| {
            if c > 0.0 {
                n / (n_classes as f32 * c)
            } else {
                1.0
            }
        })
        .collect();

    Ok(Tensor::new(weights.as_slice(), device)?)
}

// ── Model Saving ────────────────────────────────────────────────────────────

/// Save trained entity classifier model artifacts to a directory.
///
/// Writes:
/// - `model.safetensors` — model weights
/// - `config.json` — architecture configuration and training metadata
/// - `label_index.json` — class label mapping
pub fn save_entity_model(
    output_dir: &Path,
    varmap: &VarMap,
    summary: &TrainingSummary,
    demotion_threshold: f64,
    train_labels: &[usize],
    val_labels: &[usize],
) -> Result<()> {
    std::fs::create_dir_all(output_dir)?;

    // Save weights
    let model_path = output_dir.join("model.safetensors");
    varmap.save(&model_path)?;
    tracing::info!("Saved model weights to {}", model_path.display());

    // Demotion analysis at various thresholds
    let demotion_analysis = compute_demotion_analysis(val_labels, summary);

    // Compute class distribution
    let n_train = train_labels.len();
    let n_val = val_labels.len();

    // config.json
    // Convert arrays to Vec for serde (arrays > 32 don't implement Serialize)
    let stat_names_vec: Vec<&str> = STAT_FEATURE_NAMES.to_vec();
    let class_labels_vec: Vec<&str> = ENTITY_LABELS.to_vec();

    let config = serde_json::json!({
        "architecture": "deep_sets_mlp",
        "input_dim": INPUT_DIM,
        "hidden_dim": HIDDEN_DIM,
        "n_classes": N_ENTITY,
        "dropout": DROPOUT_RATE,
        "demotion_threshold": demotion_threshold,
        "test_accuracy": summary.best_val_accuracy,
        "class_labels": class_labels_vec,
        "stat_feature_names": stat_names_vec,
        "feature_layout": {
            "emb_mean": { "start": 0, "end": EMBED_DIM },
            "emb_std": { "start": EMBED_DIM, "end": 2 * EMBED_DIM },
            "stat_features": { "start": 2 * EMBED_DIM, "end": INPUT_DIM }
        },
        "train_size": n_train,
        "test_size": n_val,
        "best_epoch": summary.best_epoch,
        "total_epochs": summary.total_epochs,
        "training_time_secs": summary.total_time_secs,
        "demotion_analysis": demotion_analysis,
    });

    let config_path = output_dir.join("config.json");
    std::fs::write(&config_path, serde_json::to_string_pretty(&config)?)?;
    tracing::info!("Saved config to {}", config_path.display());

    // label_index.json
    let label_index: serde_json::Value = ENTITY_LABELS
        .iter()
        .enumerate()
        .map(|(i, label)| (label.to_string(), serde_json::json!(i)))
        .collect::<serde_json::Map<String, serde_json::Value>>()
        .into();

    let label_path = output_dir.join("label_index.json");
    std::fs::write(&label_path, serde_json::to_string_pretty(&label_index)?)?;
    tracing::info!("Saved label index to {}", label_path.display());

    Ok(())
}

/// Compute demotion precision/coverage at multiple thresholds.
///
/// The demotion analysis shows: at each threshold, how many non-person
/// columns would be correctly demoted (precision) and what fraction of
/// non-person columns are covered (coverage).
fn compute_demotion_analysis(
    val_labels: &[usize],
    _summary: &TrainingSummary,
) -> serde_json::Value {
    // For now, report the class distribution. Full analysis requires
    // running inference on the val set, which is done by the binary.
    let mut counts = [0usize; N_ENTITY];
    for &l in val_labels {
        if l < N_ENTITY {
            counts[l] += 1;
        }
    }

    let n_person = counts[0];
    let n_nonperson: usize = counts[1..].iter().sum();

    serde_json::json!({
        "val_person_count": n_person,
        "val_nonperson_count": n_nonperson,
        "val_class_distribution": {
            "person": counts[0],
            "place": counts[1],
            "organization": counts[2],
            "creative_work": counts[3],
        },
        "note": "Full threshold analysis requires inference pass (see binary output)"
    })
}

// ── SOTAB Label Mapping ─────────────────────────────────────────────────────

/// Map SOTAB Schema.org ground-truth labels to entity subtype indices (0–3).
///
/// Returns `None` for non-entity labels.
pub fn sotab_to_entity_class(gt_label: &str) -> Option<usize> {
    match gt_label {
        // Person (0)
        "Person" => Some(0),

        // Place (1) — not present in entity training from SOTAB column values
        // (place is Geographic broad category, but some overlap exists)
        "Country" | "City" | "State" | "AdministrativeArea" | "Place" | "Continent" => Some(1),

        // Organization (2)
        "Organization"
        | "MusicGroup"
        | "SportsClub"
        | "SportsTeam"
        | "LocalBus"
        | "Corporation"
        | "EducationalOrganization" => Some(2),

        // Creative work (3)
        "CreativeWork" | "Movie" | "MusicAlbum" | "MusicRecording" | "TVSeries" | "Book"
        | "Product" => Some(3),

        _ => None,
    }
}

/// All SOTAB labels that map to entity classes (for SQL IN clause).
pub const SOTAB_ENTITY_LABELS: &[&str] = &[
    "Person",
    "Organization",
    "MusicGroup",
    "SportsClub",
    "SportsTeam",
    "LocalBus",
    "Corporation",
    "EducationalOrganization",
    "CreativeWork",
    "Movie",
    "MusicAlbum",
    "MusicRecording",
    "TVSeries",
    "Book",
    "Product",
    "Country",
    "City",
    "State",
    "AdministrativeArea",
    "Place",
    "Continent",
];

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stat_helpers() {
        assert!((stat_mean(&[1.0, 2.0, 3.0]) - 2.0).abs() < 1e-5);
        assert!((stat_std_dev(&[1.0, 2.0, 3.0], 2.0) - 0.8165).abs() < 0.001);
        assert!((stat_median(&[1.0, 2.0, 3.0]) - 2.0).abs() < 1e-5);
        assert!((stat_median(&[1.0, 2.0, 3.0, 4.0]) - 2.5).abs() < 1e-5);
        assert!((stat_percentile(&[1.0, 2.0, 3.0, 4.0], 25.0) - 1.75).abs() < 1e-5);
        assert!((stat_percentile(&[1.0, 2.0, 3.0, 4.0], 75.0) - 3.25).abs() < 1e-5);
    }

    #[test]
    fn test_stat_feature_count() {
        let values = vec![
            "John Smith".to_string(),
            "Jane Doe".to_string(),
            "Robert Johnson".to_string(),
        ];
        let features = compute_stat_features(&values);
        assert_eq!(
            features.len(),
            N_STAT_FEATURES,
            "Expected {} stat features, got {}",
            N_STAT_FEATURES,
            features.len()
        );
    }

    #[test]
    fn test_contains_word_patterns() {
        // Org suffixes
        assert!(contains_word("Acme Inc", ORG_SUFFIXES));
        assert!(contains_word("MIT University", ORG_SUFFIXES));
        assert!(contains_word("Royal Hotel", ORG_SUFFIXES));
        assert!(contains_word("Smith & Co GmbH", ORG_SUFFIXES));
        assert!(!contains_word("John Smith", ORG_SUFFIXES));

        // Person titles
        assert!(contains_word("Dr. Smith", PERSON_TITLES));
        assert!(contains_word("John Jr.", PERSON_TITLES));
        assert!(!contains_word("John Smith", PERSON_TITLES));

        // Place indicators
        assert!(contains_word("123 Main Street", PLACE_INDICATORS));
        assert!(contains_word("Hyde Park", PLACE_INDICATORS));
        assert!(!contains_word("Acme Inc", PLACE_INDICATORS));

        // Creative indicators
        assert!(contains_word("Greatest Hits Album", CREATIVE_INDICATORS));
        assert!(contains_word("Live at the Apollo", CREATIVE_INDICATORS));
        assert!(!contains_word("John Smith", CREATIVE_INDICATORS));

        // Prepositions
        assert!(contains_word("King of the Hill", PREPOSITIONS));
    }

    #[test]
    fn test_contains_digit() {
        assert!(contains_digit("abc123"));
        assert!(contains_digit("42"));
        assert!(!contains_digit("hello world"));
        assert!(!contains_digit(""));
    }

    #[test]
    fn test_sotab_label_mapping() {
        assert_eq!(sotab_to_entity_class("Person"), Some(0));
        assert_eq!(sotab_to_entity_class("Organization"), Some(2));
        assert_eq!(sotab_to_entity_class("Movie"), Some(3));
        assert_eq!(sotab_to_entity_class("Country"), Some(1));
        assert_eq!(sotab_to_entity_class("SomeOtherType"), None);
    }

    #[test]
    fn test_class_weights_balanced() {
        let device = Device::Cpu;
        let labels = vec![0, 0, 0, 1, 1, 1, 2, 2, 2, 3, 3, 3];
        let weights = compute_class_weights(&labels, 4, &device).unwrap();
        let w: Vec<f32> = weights.to_vec1().unwrap();
        // Balanced: all weights should be ~1.0
        for &weight in &w {
            assert!(
                (weight - 1.0).abs() < 0.01,
                "Expected ~1.0 for balanced classes, got {}",
                weight
            );
        }
    }

    #[test]
    fn test_class_weights_imbalanced() {
        let device = Device::Cpu;
        // 6 person, 2 org = imbalanced
        let labels = vec![0, 0, 0, 0, 0, 0, 2, 2];
        let weights = compute_class_weights(&labels, 4, &device).unwrap();
        let w: Vec<f32> = weights.to_vec1().unwrap();
        // Person (6/8): weight = 8 / (4 * 6) = 0.333
        assert!(
            (w[0] - 8.0 / 24.0).abs() < 0.01,
            "Person weight: expected {}, got {}",
            8.0 / 24.0,
            w[0]
        );
        // Org (2/8): weight = 8 / (4 * 2) = 1.0
        assert!(
            (w[2] - 1.0).abs() < 0.01,
            "Org weight: expected 1.0, got {}",
            w[2]
        );
        // Empty classes: weight = 1.0 (fallback)
        assert!(
            (w[1] - 1.0).abs() < 0.01,
            "Empty class weight: expected 1.0, got {}",
            w[1]
        );
    }

    #[test]
    fn test_train_entity_synthetic() {
        // Create synthetic 300-dim features: 50 samples, 4 classes
        use rand::{Rng, SeedableRng};
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);

        let n_per_class = 13; // ~52 total
        let n_total = n_per_class * N_ENTITY;
        let mut features = Vec::with_capacity(n_total);
        let mut labels = Vec::with_capacity(n_total);

        for class_idx in 0..N_ENTITY {
            for _ in 0..n_per_class {
                let mut feat = vec![0.0f32; INPUT_DIM];
                // Create class-separable features: offset by class index
                for f in feat.iter_mut() {
                    *f = rng.gen::<f32>() * 0.5 + class_idx as f32;
                }
                features.push(feat);
                labels.push(class_idx);
            }
        }

        // 90/10 split
        let split = (n_total as f32 * 0.9) as usize;
        let train_features = &features[..split];
        let train_labels = &labels[..split];
        let val_features = &features[split..];
        let val_labels = &labels[split..];

        let config = EntityTrainConfig {
            epochs: 10,
            batch_size: 16,
            lr: 1e-3,
            min_lr: 1e-5,
            patience: 15,
            demotion_threshold: 0.6,
            seed: 42,
        };

        let (summary, _varmap) = train_entity(
            &config,
            train_features,
            train_labels,
            val_features,
            val_labels,
        )
        .expect("Training should succeed");

        // Assert loss decreases: first epoch loss > last epoch loss
        let first_loss = summary.epoch_metrics[0].train_loss;
        let last_loss = summary.epoch_metrics.last().unwrap().train_loss;
        assert!(
            last_loss < first_loss,
            "Loss should decrease: first={:.4}, last={:.4}",
            first_loss,
            last_loss,
        );

        // Assert we completed the expected number of epochs
        assert_eq!(summary.total_epochs, 10);
    }

    #[test]
    fn test_train_entity_save() {
        // Train on synthetic data and verify save works
        use rand::{Rng, SeedableRng};
        let mut rng = rand::rngs::StdRng::seed_from_u64(123);

        let n_per_class = 10;
        let n_total = n_per_class * N_ENTITY;
        let mut features = Vec::with_capacity(n_total);
        let mut labels = Vec::with_capacity(n_total);

        for class_idx in 0..N_ENTITY {
            for _ in 0..n_per_class {
                let mut feat = vec![0.0f32; INPUT_DIM];
                for f in feat.iter_mut() {
                    *f = rng.gen::<f32>() + class_idx as f32 * 2.0;
                }
                features.push(feat);
                labels.push(class_idx);
            }
        }

        let split = 32;
        let config = EntityTrainConfig {
            epochs: 5,
            batch_size: 8,
            lr: 1e-3,
            min_lr: 1e-5,
            patience: 15,
            demotion_threshold: 0.6,
            seed: 123,
        };

        let (summary, varmap) = train_entity(
            &config,
            &features[..split],
            &labels[..split],
            &features[split..],
            &labels[split..],
        )
        .expect("Training should succeed");

        // Save to temp directory
        let tmp_dir = tempfile::tempdir().expect("tempdir");
        save_entity_model(
            tmp_dir.path(),
            &varmap,
            &summary,
            config.demotion_threshold,
            &labels[..split],
            &labels[split..],
        )
        .expect("Save should succeed");

        // Verify files exist
        assert!(
            tmp_dir.path().join("model.safetensors").exists(),
            "model.safetensors should exist"
        );
        assert!(
            tmp_dir.path().join("config.json").exists(),
            "config.json should exist"
        );
        assert!(
            tmp_dir.path().join("label_index.json").exists(),
            "label_index.json should exist"
        );

        // Verify config.json contents
        let config_str = std::fs::read_to_string(tmp_dir.path().join("config.json")).unwrap();
        let config_json: serde_json::Value = serde_json::from_str(&config_str).unwrap();
        assert_eq!(config_json["architecture"], "deep_sets_mlp");
        assert_eq!(config_json["input_dim"], INPUT_DIM);
        assert_eq!(config_json["n_classes"], N_ENTITY);

        // Verify label_index.json
        let label_str = std::fs::read_to_string(tmp_dir.path().join("label_index.json")).unwrap();
        let label_json: serde_json::Value = serde_json::from_str(&label_str).unwrap();
        assert_eq!(label_json["person"], 0);
        assert_eq!(label_json["place"], 1);
        assert_eq!(label_json["organization"], 2);
        assert_eq!(label_json["creative_work"], 3);
    }
}
