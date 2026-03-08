//! Inference utilities for text classification.

use crate::char_cnn::{CharCnn, CharCnnConfig, CharVocab};
use crate::model::{TextClassifier, TextClassifierConfig};
use candle_core::{DType, Device, Tensor};
use candle_nn::VarBuilder;
use finetype_core::{Taxonomy, Tokenizer};
use regex::Regex;
use std::collections::HashMap;
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum InferenceError {
    #[error("Model error: {0}")]
    ModelError(#[from] candle_core::Error),
    #[error("Tokenizer error: {0}")]
    TokenizerError(#[from] finetype_core::tokenizer::TokenizerError),
    #[error("Taxonomy error: {0}")]
    TaxonomyError(#[from] finetype_core::taxonomy::TaxonomyError),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Invalid model path: {0}")]
    InvalidPath(String),
}

/// Classification result.
#[derive(Debug, Clone)]
pub struct ClassificationResult {
    pub label: String,
    pub confidence: f32,
    pub all_scores: Vec<(String, f32)>,
}

/// Trait for any classifier that can classify text values.
///
/// Implemented by `CharClassifier`, `TieredClassifier`, and `Classifier`.
/// Used by `ColumnClassifier` to support both flat and tiered models.
pub trait ValueClassifier: Send + Sync {
    /// Classify a single text value.
    fn classify(&self, text: &str) -> Result<ClassificationResult, InferenceError>;

    /// Classify a batch of text values.
    fn classify_batch(&self, texts: &[String])
        -> Result<Vec<ClassificationResult>, InferenceError>;
}

/// Classifier for text classification inference.
pub struct Classifier {
    model: TextClassifier,
    tokenizer: Tokenizer,
    index_to_label: HashMap<usize, String>,
    device: Device,
    max_seq_length: usize,
}

impl Classifier {
    /// Load a classifier from a directory containing model weights and taxonomy.
    pub fn load<P: AsRef<Path>>(model_dir: P) -> Result<Self, InferenceError> {
        let model_dir = model_dir.as_ref();

        // Determine device
        let device = Self::get_device();

        // Load taxonomy
        let taxonomy_path = model_dir.join("taxonomy.yaml");
        let taxonomy = if taxonomy_path.exists() {
            Taxonomy::from_file(&taxonomy_path)?
        } else {
            // Try default labels path
            Taxonomy::from_file(model_dir.join("labels.yaml"))?
        };

        let n_classes = taxonomy.len();
        let index_to_label = taxonomy.index_to_label();

        // Load config
        // TODO: Load from config file if available
        let config = TextClassifierConfig {
            n_classes,
            max_seq_length: 128, // Must match training config
            ..Default::default()
        };

        // Load model weights
        let weights_path = model_dir.join("model.safetensors");
        let vb = if weights_path.exists() {
            unsafe { VarBuilder::from_mmaped_safetensors(&[weights_path], DType::F32, &device)? }
        } else {
            // Initialize with random weights for testing
            VarBuilder::zeros(DType::F32, &device)
        };

        let model = TextClassifier::new(config.clone(), vb)?;
        let tokenizer = Tokenizer::bert_cased()?;

        Ok(Self {
            model,
            tokenizer,
            index_to_label,
            device,
            max_seq_length: config.max_seq_length,
        })
    }

    /// Classify a single text input.
    pub fn classify(&self, text: &str) -> Result<ClassificationResult, InferenceError> {
        let results = self.classify_batch(&[text.to_string()])?;
        Ok(results.into_iter().next().unwrap())
    }

    /// Classify multiple text inputs.
    pub fn classify_batch(
        &self,
        texts: &[String],
    ) -> Result<Vec<ClassificationResult>, InferenceError> {
        let batch_size = texts.len();

        // Tokenize all inputs
        let mut all_ids = Vec::with_capacity(batch_size);
        let mut all_masks = Vec::with_capacity(batch_size);

        for text in texts {
            let (ids, mask) = self.tokenizer.encode_padded(text, self.max_seq_length)?;
            all_ids.push(ids);
            all_masks.push(mask);
        }

        // Create tensors
        let input_ids = Tensor::new(
            all_ids.into_iter().flatten().collect::<Vec<u32>>(),
            &self.device,
        )?
        .reshape((batch_size, self.max_seq_length))?;

        let attention_mask = Tensor::new(
            all_masks.into_iter().flatten().collect::<Vec<u32>>(),
            &self.device,
        )?
        .reshape((batch_size, self.max_seq_length))?
        .to_dtype(DType::F32)?;

        // Run inference
        let probs = self.model.infer(&input_ids, Some(&attention_mask))?;
        let probs = probs.to_vec2::<f32>()?;

        // Convert to results
        let mut results = Vec::with_capacity(batch_size);
        for prob_row in probs {
            let (max_idx, max_prob) = prob_row
                .iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
                .unwrap();

            let label = self
                .index_to_label
                .get(&max_idx)
                .cloned()
                .unwrap_or_else(|| "unknown".to_string());

            let all_scores: Vec<(String, f32)> = prob_row
                .iter()
                .enumerate()
                .map(|(i, &p)| {
                    let lbl = self
                        .index_to_label
                        .get(&i)
                        .cloned()
                        .unwrap_or_else(|| format!("class_{}", i));
                    (lbl, p)
                })
                .collect();

            results.push(ClassificationResult {
                label,
                confidence: *max_prob,
                all_scores,
            });
        }

        Ok(results)
    }

    /// Get the best device available.
    fn get_device() -> Device {
        #[cfg(feature = "cuda")]
        {
            if let Ok(device) = Device::new_cuda(0) {
                return device;
            }
        }

        #[cfg(feature = "metal")]
        {
            if let Ok(device) = Device::new_metal(0) {
                return device;
            }
        }

        Device::Cpu
    }

    /// Get the tokenizer.
    pub fn tokenizer(&self) -> &Tokenizer {
        &self.tokenizer
    }

    /// Get the device being used.
    pub fn device(&self) -> &Device {
        &self.device
    }
}

impl ValueClassifier for Classifier {
    fn classify(&self, text: &str) -> Result<ClassificationResult, InferenceError> {
        self.classify(text)
    }

    fn classify_batch(
        &self,
        texts: &[String],
    ) -> Result<Vec<ClassificationResult>, InferenceError> {
        self.classify_batch(texts)
    }
}

/// CharCNN-based classifier for text classification inference.
pub struct CharClassifier {
    model: CharCnn,
    vocab: CharVocab,
    index_to_label: HashMap<usize, String>,
    device: Device,
    max_seq_length: usize,
    /// Compiled validation patterns from taxonomy, keyed by type label.
    /// When set, predictions are validated against these patterns and
    /// fall back to next-best predictions on mismatch.
    validation_patterns: Option<HashMap<String, Regex>>,
}

impl CharClassifier {
    /// Load a CharCNN classifier from embedded byte slices.
    ///
    /// Used by the DuckDB extension where model files are compiled into the binary.
    pub fn from_bytes(
        weights: &[u8],
        labels_json: &[u8],
        config_yaml: &[u8],
    ) -> Result<Self, InferenceError> {
        let device = Self::get_device();

        // Parse labels
        let labels_str = std::str::from_utf8(labels_json).map_err(|e| {
            InferenceError::InvalidPath(format!("Invalid UTF-8 in labels.json: {}", e))
        })?;
        let labels: Vec<String> = serde_json::from_str(labels_str).map_err(|e| {
            InferenceError::InvalidPath(format!("Failed to parse labels.json: {}", e))
        })?;
        let n_classes = labels.len();
        let index_to_label: HashMap<usize, String> = labels.into_iter().enumerate().collect();

        // Parse config
        let config_str = std::str::from_utf8(config_yaml).unwrap_or("");
        let mut vocab_size = 97usize;
        let mut max_seq_length = 128usize;
        let mut embed_dim = 32usize;
        let mut num_filters = 64usize;
        let mut hidden_dim = 128usize;
        let mut feature_dim = 0usize;

        for line in config_str.lines() {
            if let Some((key, val)) = line.split_once(':') {
                let key = key.trim();
                let val = val.trim();
                match key {
                    "vocab_size" => vocab_size = val.parse().unwrap_or(97),
                    "max_seq_length" => max_seq_length = val.parse().unwrap_or(128),
                    "embed_dim" => embed_dim = val.parse().unwrap_or(32),
                    "num_filters" => num_filters = val.parse().unwrap_or(64),
                    "hidden_dim" => hidden_dim = val.parse().unwrap_or(128),
                    "feature_dim" => feature_dim = val.parse().unwrap_or(0),
                    _ => {}
                }
            }
        }

        let vocab = CharVocab::new();
        let config = CharCnnConfig {
            vocab_size,
            max_seq_length,
            embed_dim,
            num_filters,
            kernel_sizes: vec![2, 3, 4, 5],
            hidden_dim,
            n_classes,
            dropout: 0.0,
            feature_dim,
        };

        let vb = VarBuilder::from_buffered_safetensors(weights.to_vec(), DType::F32, &device)?;
        let model = CharCnn::new(config, vb)?;

        Ok(Self {
            model,
            vocab,
            index_to_label,
            device,
            max_seq_length,
            validation_patterns: None,
        })
    }

    /// Load a CharCNN classifier from a directory.
    pub fn load<P: AsRef<Path>>(model_dir: P) -> Result<Self, InferenceError> {
        let model_dir = model_dir.as_ref();
        let device = Self::get_device();

        // Load label mapping — try labels.json first (saved by trainer), then taxonomy.yaml
        let labels_json_path = model_dir.join("labels.json");
        let taxonomy_path = model_dir.join("taxonomy.yaml");
        let (n_classes, index_to_label) = if labels_json_path.exists() {
            let content = std::fs::read_to_string(&labels_json_path)?;
            let labels: Vec<String> = serde_json::from_str(&content).map_err(|e| {
                InferenceError::InvalidPath(format!("Failed to parse labels.json: {}", e))
            })?;
            let n = labels.len();
            let mapping: HashMap<usize, String> = labels.into_iter().enumerate().collect();
            (n, mapping)
        } else if taxonomy_path.exists() {
            let taxonomy = Taxonomy::from_file(&taxonomy_path)?;
            let n = taxonomy.len();
            (n, taxonomy.index_to_label())
        } else {
            let labels_yaml_path = model_dir.join("labels.yaml");
            let taxonomy = Taxonomy::from_file(&labels_yaml_path)?;
            let n = taxonomy.len();
            (n, taxonomy.index_to_label())
        };

        // Load config from config.yaml if available
        let config_path = model_dir.join("config.yaml");
        let (vocab_size, max_seq_length, embed_dim, num_filters, hidden_dim, feature_dim) =
            if config_path.exists() {
                let config_str = std::fs::read_to_string(&config_path)?;
                let mut vocab_size = 97usize;
                let mut max_seq_length = 128usize;
                let mut embed_dim = 32usize;
                let mut num_filters = 64usize;
                let mut hidden_dim = 128usize;
                let mut feature_dim = 0usize;

                for line in config_str.lines() {
                    if let Some((key, val)) = line.split_once(':') {
                        let key = key.trim();
                        let val = val.trim();
                        match key {
                            "vocab_size" => vocab_size = val.parse().unwrap_or(97),
                            "max_seq_length" => max_seq_length = val.parse().unwrap_or(128),
                            "embed_dim" => embed_dim = val.parse().unwrap_or(32),
                            "num_filters" => num_filters = val.parse().unwrap_or(64),
                            "hidden_dim" => hidden_dim = val.parse().unwrap_or(128),
                            "feature_dim" => feature_dim = val.parse().unwrap_or(0),
                            _ => {}
                        }
                    }
                }
                (
                    vocab_size,
                    max_seq_length,
                    embed_dim,
                    num_filters,
                    hidden_dim,
                    feature_dim,
                )
            } else {
                (97, 128, 32, 64, 128, 0)
            };

        let vocab = CharVocab::new();

        let config = CharCnnConfig {
            vocab_size,
            max_seq_length,
            embed_dim,
            num_filters,
            kernel_sizes: vec![2, 3, 4, 5],
            hidden_dim,
            n_classes,
            dropout: 0.0, // No dropout during inference
            feature_dim,
        };

        // Load model weights
        let weights_path = model_dir.join("model.safetensors");
        let vb =
            unsafe { VarBuilder::from_mmaped_safetensors(&[weights_path], DType::F32, &device)? };

        let model = CharCnn::new(config, vb)?;

        Ok(Self {
            model,
            vocab,
            index_to_label,
            device,
            max_seq_length,
            validation_patterns: None,
        })
    }

    /// Set validation patterns from taxonomy definitions.
    ///
    /// Compiles regex patterns from the taxonomy's validation fields. After this,
    /// `classify_batch()` will validate predictions against patterns and fall back
    /// to next-best predictions when the input doesn't match.
    pub fn set_validation_patterns(&mut self, patterns: HashMap<String, String>) {
        let compiled: HashMap<String, Regex> = patterns
            .into_iter()
            .filter_map(|(label, pattern)| Regex::new(&pattern).ok().map(|re| (label, re)))
            .collect();
        if !compiled.is_empty() {
            self.validation_patterns = Some(compiled);
        }
    }

    /// Get the validation patterns (if set).
    pub fn validation_patterns(&self) -> Option<&HashMap<String, Regex>> {
        self.validation_patterns.as_ref()
    }

    /// Classify a single text input.
    pub fn classify(&self, text: &str) -> Result<ClassificationResult, InferenceError> {
        let results = self.classify_batch(&[text.to_string()])?;
        Ok(results.into_iter().next().unwrap())
    }

    /// Classify multiple text inputs.
    pub fn classify_batch(
        &self,
        texts: &[String],
    ) -> Result<Vec<ClassificationResult>, InferenceError> {
        let batch_size = texts.len();

        // Encode all inputs
        let mut all_ids = Vec::with_capacity(batch_size * self.max_seq_length);
        for text in texts {
            let ids = self.vocab.encode(text, self.max_seq_length);
            all_ids.extend(ids);
        }

        // Create tensor
        let input_ids =
            Tensor::new(all_ids, &self.device)?.reshape((batch_size, self.max_seq_length))?;

        // Run inference
        let probs = self.model.infer(&input_ids)?;
        let probs = probs.to_vec2::<f32>()?;

        // Convert to results
        let mut results = Vec::with_capacity(batch_size);
        for prob_row in probs {
            let (max_idx, max_prob) = prob_row
                .iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
                .unwrap();

            let label = self
                .index_to_label
                .get(&max_idx)
                .cloned()
                .unwrap_or_else(|| "unknown".to_string());

            let all_scores: Vec<(String, f32)> = prob_row
                .iter()
                .enumerate()
                .map(|(i, &p)| {
                    let lbl = self
                        .index_to_label
                        .get(&i)
                        .cloned()
                        .unwrap_or_else(|| format!("class_{}", i));
                    (lbl, p)
                })
                .collect();

            results.push(ClassificationResult {
                label,
                confidence: *max_prob,
                all_scores,
            });
        }

        // Post-process: apply format-based corrections for known model confusions
        for (result, text) in results.iter_mut().zip(texts.iter()) {
            post_process(result, text);
        }

        // Pattern-validate: check predictions against taxonomy validation patterns.
        // If the input doesn't match the predicted type's pattern, fall back to
        // the next-best prediction that either has no pattern or matches.
        if let Some(ref patterns) = self.validation_patterns {
            for (result, text) in results.iter_mut().zip(texts.iter()) {
                pattern_validate(result, text.trim(), patterns);
            }
        }

        Ok(results)
    }

    /// Get the best device available.
    fn get_device() -> Device {
        #[cfg(feature = "cuda")]
        {
            if let Ok(device) = Device::new_cuda(0) {
                return device;
            }
        }

        #[cfg(feature = "metal")]
        {
            if let Ok(device) = Device::new_metal(0) {
                return device;
            }
        }

        Device::Cpu
    }
}

impl ValueClassifier for CharClassifier {
    fn classify(&self, text: &str) -> Result<ClassificationResult, InferenceError> {
        self.classify(text)
    }

    fn classify_batch(
        &self,
        texts: &[String],
    ) -> Result<Vec<ClassificationResult>, InferenceError> {
        self.classify_batch(texts)
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// POST-PROCESSING RULES
// ═══════════════════════════════════════════════════════════════════════════════

/// Apply format-based corrections for known model confusion pairs.
///
/// These rules check the actual input text to resolve confusions where the model
/// struggles but the format provides a definitive signal. Each rule is a simple
/// character/pattern check with no ambiguity.
fn post_process(result: &mut ClassificationResult, text: &str) {
    // Rule 1: rfc_3339 vs iso_8601_offset
    //
    // The only difference is T (ISO 8601) vs space (RFC 3339) between date and time.
    // The model confuses these 100% of the time. A simple character check resolves it.
    //
    // iso_8601_offset: "2024-01-15T10:30:00+05:00" (T separator)
    // rfc_3339:        "2024-01-15 10:30:00+05:00" (space separator)
    if result.label == "datetime.timestamp.iso_8601_offset"
        || result.label == "datetime.timestamp.rfc_3339"
    {
        let trimmed = text.trim();
        // Look for the separator at position 10 (after YYYY-MM-DD)
        if trimmed.len() >= 11 {
            let sep = trimmed.as_bytes()[10];
            if sep == b'T' {
                result.label = "datetime.timestamp.iso_8601_offset".to_string();
            } else if sep == b' ' {
                result.label = "datetime.timestamp.rfc_3339".to_string();
            }
        }
    }

    // Rule 2: hash vs token_hex
    //
    // Cryptographic hashes have fixed lengths: 32 (MD5), 40 (SHA-1), 64 (SHA-256), 128 (SHA-512).
    // Hex tokens have variable non-standard lengths. The model confuses these (58x token_hex→hash).
    // A simple length check on the trimmed hex string resolves this definitively.
    if result.label == "technology.cryptographic.hash"
        || result.label == "technology.cryptographic.token_hex"
    {
        let trimmed = text.trim();
        let is_hex = !trimmed.is_empty()
            && trimmed
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase());
        if is_hex {
            let len = trimmed.len();
            if len == 32 || len == 40 || len == 64 || len == 128 {
                result.label = "technology.cryptographic.hash".to_string();
            } else {
                result.label = "technology.cryptographic.token_hex".to_string();
            }
        }
    }

    // Rule 3: emoji vs gender_symbol
    //
    // Gender symbols are a specific set: ♂ (U+2642), ♀ (U+2640), ⚧ (U+26A7), ⚪ (U+26AA).
    // The model confuses emojis as gender symbols (96x). A character identity check resolves this.
    if result.label == "identity.person.gender_symbol"
        || result.label == "representation.text.emoji"
    {
        let trimmed = text.trim();
        let is_gender_symbol = trimmed.chars().count() == 1
            && matches!(trimmed.chars().next(), Some('♂' | '♀' | '⚧' | '⚪'));
        if is_gender_symbol {
            result.label = "identity.person.gender_symbol".to_string();
        } else if !trimmed.is_empty() {
            result.label = "representation.text.emoji".to_string();
        }
    }

    // Rule 4: ISSN vs postal_code
    //
    // ISSN has a distinctive format: XXXX-XXX[0-9X] (4 digits, hyphen, 3 digits, check char).
    // Postal codes are typically 5 digits (ZIP), 5-4 (ZIP+4), or other country formats.
    // The model confuses these bidirectionally (24x issn→postal, 23x postal→issn).
    // A regex-free pattern check on the hyphenated format resolves this.
    if result.label == "identity.commerce.issn" || result.label == "geography.address.postal_code" {
        let trimmed = text.trim();
        let bytes = trimmed.as_bytes();
        // ISSN pattern: exactly 9 chars, format DDDD-DDD[DX]
        let is_issn = bytes.len() == 9
            && bytes[4] == b'-'
            && bytes[..4].iter().all(|b| b.is_ascii_digit())
            && bytes[5..8].iter().all(|b| b.is_ascii_digit())
            && (bytes[8].is_ascii_digit() || bytes[8] == b'X');
        if is_issn {
            result.label = "identity.commerce.issn".to_string();
        } else {
            result.label = "geography.address.postal_code".to_string();
        }
    }

    // Rule 5: longitude vs latitude (partial)
    //
    // Latitude is bounded to -90..+90, longitude to -180..+180.
    // If a value's absolute magnitude exceeds 90, it's definitively longitude.
    // Values within ±90 remain as the model predicted (ambiguous at single-value level).
    // The model confuses these (30x lon→lat, 21x lat→lon).
    if result.label == "geography.coordinate.latitude"
        || result.label == "geography.coordinate.longitude"
    {
        let trimmed = text.trim();
        if let Ok(val) = trimmed.parse::<f64>() {
            if val.abs() > 90.0 {
                result.label = "geography.coordinate.longitude".to_string();
            }
        }
    }

    // Rule 6: email rescue
    //
    // The `@` sign is a definitive format signal for email addresses. The model sometimes
    // misclassifies emails with short/uncommon domains as hostname, username, or slug.
    // Only applies to specific confusion labels (not container types, CSV, JSON, etc.).
    // The text must look like a standalone email, not a substring within structured data.
    if result.label == "technology.internet.hostname"
        || result.label == "identity.person.username"
        || result.label == "technology.internet.slug"
    {
        let trimmed = text.trim();
        if let Some(at_pos) = trimmed.find('@') {
            let local = &trimmed[..at_pos];
            let domain = &trimmed[at_pos + 1..];
            // Strict email check: standalone email, not embedded in structured data
            let looks_like_email = !local.is_empty()
                && !domain.is_empty()
                && domain.contains('.')
                && trimmed.find('@') == trimmed.rfind('@') // exactly one @
                && !trimmed.contains(' ')
                && !trimmed.contains("://")
                && !trimmed.contains(',')
                && !trimmed.contains('=')
                && !trimmed.contains('&')
                && !trimmed.contains('{')
                && !trimmed.contains('|')
                && !trimmed.contains(';');
            if looks_like_email {
                result.label = "identity.person.email".to_string();
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// PATTERN VALIDATION
// ═══════════════════════════════════════════════════════════════════════════════

/// Validate a prediction against the taxonomy's validation pattern.
///
/// If the predicted type has a validation pattern and the input text doesn't
/// match it, fall back to the next-best prediction (by confidence) that either
/// has no pattern or whose pattern matches the input.
///
/// This runs AFTER post-processing rules, which handle known confusion pairs.
/// Pattern validation is a general-purpose safety net that catches mismatches
/// the model can't see (e.g., "C85" predicted as iata_code but failing ^[A-Z]{3}$).
///
/// Only falls back through the top 5 predictions to avoid expensive regex checks
/// on low-probability candidates.
fn pattern_validate(
    result: &mut ClassificationResult,
    text: &str,
    patterns: &HashMap<String, Regex>,
) {
    // If the current prediction has no pattern, nothing to validate
    let current_pattern = match patterns.get(&result.label) {
        Some(pat) => pat,
        None => return,
    };

    // If the input matches the current prediction's pattern, keep it
    if current_pattern.is_match(text) {
        return;
    }

    // Input doesn't match the predicted type's pattern — find a fallback.
    // Sort all_scores by confidence descending and try the top candidates.
    let mut candidates: Vec<(String, f32)> = result.all_scores.clone();
    candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    for (label, score) in candidates.iter().take(5) {
        // Skip the current (rejected) prediction
        if *label == result.label {
            continue;
        }
        // Accept if: no pattern for this type, OR input matches the pattern
        match patterns.get(label.as_str()) {
            None => {
                // No pattern constraint — accept this fallback
                result.label = label.clone();
                result.confidence = *score;
                return;
            }
            Some(pat) if pat.is_match(text) => {
                // Pattern matches — accept this fallback
                result.label = label.clone();
                result.confidence = *score;
                return;
            }
            _ => {
                // Pattern doesn't match — try next candidate
                continue;
            }
        }
    }

    // No valid fallback found — keep original prediction (better than nothing)
}

/// Extract validation patterns from a taxonomy as a label → pattern string map.
///
/// This is a convenience function for building the patterns map to pass to
/// `CharClassifier::set_validation_patterns()`.
pub fn extract_validation_patterns(taxonomy: &Taxonomy) -> HashMap<String, String> {
    taxonomy
        .definitions()
        .filter_map(|(label, def)| {
            def.validation
                .as_ref()
                .and_then(|v| v.pattern.as_ref())
                .map(|p| (label.clone(), p.clone()))
        })
        .collect()
}

/// A mock classifier for testing column-level inference.
///
/// Always returns the same label with 0.8 confidence, regardless of input.
/// Used in integration tests to verify that header hints and semantic hints
/// properly override the base classifier's output.
#[cfg(test)]
pub struct MockClassifier {
    label: String,
}

#[cfg(test)]
impl MockClassifier {
    pub fn new(label: &str) -> Self {
        Self {
            label: label.to_string(),
        }
    }
}

#[cfg(test)]
impl ValueClassifier for MockClassifier {
    fn classify(&self, _text: &str) -> Result<ClassificationResult, InferenceError> {
        Ok(ClassificationResult {
            label: self.label.clone(),
            confidence: 0.8,
            all_scores: vec![(self.label.clone(), 0.8)],
        })
    }

    fn classify_batch(
        &self,
        texts: &[String],
    ) -> Result<Vec<ClassificationResult>, InferenceError> {
        Ok(texts
            .iter()
            .map(|_| ClassificationResult {
                label: self.label.clone(),
                confidence: 0.8,
                all_scores: vec![(self.label.clone(), 0.8)],
            })
            .collect())
    }
}

#[cfg(test)]
mod post_process_tests {
    use super::*;

    fn make_result(label: &str) -> ClassificationResult {
        ClassificationResult {
            label: label.to_string(),
            confidence: 0.9,
            all_scores: vec![],
        }
    }

    #[test]
    fn test_iso_8601_offset_with_t_separator() {
        let mut result = make_result("datetime.timestamp.rfc_3339");
        post_process(&mut result, "2024-01-15T10:30:00+05:00");
        assert_eq!(result.label, "datetime.timestamp.iso_8601_offset");
    }

    #[test]
    fn test_rfc_3339_with_space_separator() {
        let mut result = make_result("datetime.timestamp.iso_8601_offset");
        post_process(&mut result, "2024-01-15 10:30:00+05:00");
        assert_eq!(result.label, "datetime.timestamp.rfc_3339");
    }

    #[test]
    fn test_correct_iso_8601_offset_unchanged() {
        let mut result = make_result("datetime.timestamp.iso_8601_offset");
        post_process(&mut result, "2024-01-15T10:30:00+05:00");
        assert_eq!(result.label, "datetime.timestamp.iso_8601_offset");
    }

    #[test]
    fn test_correct_rfc_3339_unchanged() {
        let mut result = make_result("datetime.timestamp.rfc_3339");
        post_process(&mut result, "2024-01-15 10:30:00+05:00");
        assert_eq!(result.label, "datetime.timestamp.rfc_3339");
    }

    #[test]
    fn test_unrelated_label_unchanged() {
        let mut result = make_result("technology.internet.ip_v4");
        post_process(&mut result, "192.168.1.1");
        assert_eq!(result.label, "technology.internet.ip_v4");
    }

    #[test]
    fn test_short_text_no_crash() {
        let mut result = make_result("datetime.timestamp.rfc_3339");
        post_process(&mut result, "short");
        // No crash, label unchanged (too short to check)
        assert_eq!(result.label, "datetime.timestamp.rfc_3339");
    }

    // ── Rule 2: hash vs token_hex ────────────────────────────────────────────

    #[test]
    fn test_hash_md5_length_32() {
        let mut result = make_result("technology.cryptographic.token_hex");
        post_process(&mut result, "5d41402abc4b2a76b9719d911017c592");
        assert_eq!(result.label, "technology.cryptographic.hash");
    }

    #[test]
    fn test_hash_sha1_length_40() {
        let mut result = make_result("technology.cryptographic.token_hex");
        post_process(&mut result, "aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d");
        assert_eq!(result.label, "technology.cryptographic.hash");
    }

    #[test]
    fn test_hash_sha256_length_64() {
        let mut result = make_result("technology.cryptographic.token_hex");
        post_process(
            &mut result,
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824",
        );
        assert_eq!(result.label, "technology.cryptographic.hash");
    }

    #[test]
    fn test_token_hex_non_standard_length() {
        let mut result = make_result("technology.cryptographic.hash");
        post_process(&mut result, "a417b553b18d13027c23e8016c3466b81e70832254");
        // 42 chars — not a standard hash length, should become token_hex
        assert_eq!(result.label, "technology.cryptographic.token_hex");
    }

    #[test]
    fn test_correct_hash_unchanged() {
        let mut result = make_result("technology.cryptographic.hash");
        post_process(&mut result, "5d41402abc4b2a76b9719d911017c592");
        assert_eq!(result.label, "technology.cryptographic.hash");
    }

    #[test]
    fn test_correct_token_hex_unchanged() {
        let mut result = make_result("technology.cryptographic.token_hex");
        post_process(&mut result, "deadbeefcafebabe00ff11");
        // 22 chars — non-standard, stays as token_hex
        assert_eq!(result.label, "technology.cryptographic.token_hex");
    }

    #[test]
    fn test_hash_with_uppercase_not_reclassified() {
        // Uppercase hex isn't lowercase-only, so rule doesn't fire
        let mut result = make_result("technology.cryptographic.hash");
        post_process(&mut result, "5D41402ABC4B2A76B9719D911017C592");
        // Label unchanged — uppercase hex doesn't match our hex check
        assert_eq!(result.label, "technology.cryptographic.hash");
    }

    // ── Rule 3: emoji vs gender_symbol ───────────────────────────────────────

    #[test]
    fn test_gender_symbol_male() {
        let mut result = make_result("representation.text.emoji");
        post_process(&mut result, "♂");
        assert_eq!(result.label, "identity.person.gender_symbol");
    }

    #[test]
    fn test_gender_symbol_female() {
        let mut result = make_result("representation.text.emoji");
        post_process(&mut result, "♀");
        assert_eq!(result.label, "identity.person.gender_symbol");
    }

    #[test]
    fn test_gender_symbol_transgender() {
        let mut result = make_result("representation.text.emoji");
        post_process(&mut result, "⚧");
        assert_eq!(result.label, "identity.person.gender_symbol");
    }

    #[test]
    fn test_emoji_not_gender_symbol() {
        let mut result = make_result("identity.person.gender_symbol");
        post_process(&mut result, "🎉");
        assert_eq!(result.label, "representation.text.emoji");
    }

    #[test]
    fn test_emoji_rocket_not_gender_symbol() {
        let mut result = make_result("identity.person.gender_symbol");
        post_process(&mut result, "🚀");
        assert_eq!(result.label, "representation.text.emoji");
    }

    #[test]
    fn test_correct_emoji_unchanged() {
        let mut result = make_result("representation.text.emoji");
        post_process(&mut result, "😀");
        assert_eq!(result.label, "representation.text.emoji");
    }

    #[test]
    fn test_correct_gender_symbol_unchanged() {
        let mut result = make_result("identity.person.gender_symbol");
        post_process(&mut result, "♂");
        assert_eq!(result.label, "identity.person.gender_symbol");
    }

    // ── Rule 4: ISSN vs postal_code ──────────────────────────────────────────

    #[test]
    fn test_issn_format_corrects_postal_code() {
        let mut result = make_result("geography.address.postal_code");
        post_process(&mut result, "0028-0836");
        assert_eq!(result.label, "identity.commerce.issn");
    }

    #[test]
    fn test_issn_with_x_check_digit() {
        let mut result = make_result("geography.address.postal_code");
        post_process(&mut result, "1234-567X");
        assert_eq!(result.label, "identity.commerce.issn");
    }

    #[test]
    fn test_postal_code_zip5_corrects_issn() {
        let mut result = make_result("identity.commerce.issn");
        post_process(&mut result, "58763");
        assert_eq!(result.label, "geography.address.postal_code");
    }

    #[test]
    fn test_postal_code_zip_plus_4() {
        let mut result = make_result("identity.commerce.issn");
        post_process(&mut result, "79262-7606");
        // 10 chars (5-4), not ISSN format (4-3+check = 9 chars)
        assert_eq!(result.label, "geography.address.postal_code");
    }

    #[test]
    fn test_correct_issn_unchanged() {
        let mut result = make_result("identity.commerce.issn");
        post_process(&mut result, "5019-8538");
        assert_eq!(result.label, "identity.commerce.issn");
    }

    #[test]
    fn test_correct_postal_code_unchanged() {
        let mut result = make_result("geography.address.postal_code");
        post_process(&mut result, "55502");
        assert_eq!(result.label, "geography.address.postal_code");
    }

    // ── Rule 5: longitude vs latitude ────────────────────────────────────────

    #[test]
    fn test_longitude_outside_latitude_range() {
        let mut result = make_result("geography.coordinate.latitude");
        post_process(&mut result, "170.0522");
        // >90 → definitively longitude
        assert_eq!(result.label, "geography.coordinate.longitude");
    }

    #[test]
    fn test_negative_longitude_outside_latitude_range() {
        let mut result = make_result("geography.coordinate.latitude");
        post_process(&mut result, "-138.9274");
        // abs > 90 → definitively longitude
        assert_eq!(result.label, "geography.coordinate.longitude");
    }

    #[test]
    fn test_value_within_latitude_range_unchanged() {
        // 40.7128 is within ±90, so prediction stays as-is
        let mut result = make_result("geography.coordinate.latitude");
        post_process(&mut result, "40.7128");
        assert_eq!(result.label, "geography.coordinate.latitude");
    }

    #[test]
    fn test_longitude_within_range_stays_longitude() {
        // -58.0804 is within ±90, but predicted as longitude — stays
        let mut result = make_result("geography.coordinate.longitude");
        post_process(&mut result, "-58.0804");
        assert_eq!(result.label, "geography.coordinate.longitude");
    }

    #[test]
    fn test_exactly_90_stays_as_predicted() {
        // Exactly 90.0 is valid latitude, should not change
        let mut result = make_result("geography.coordinate.latitude");
        post_process(&mut result, "90.0");
        assert_eq!(result.label, "geography.coordinate.latitude");
    }

    #[test]
    fn test_just_over_90_becomes_longitude() {
        let mut result = make_result("geography.coordinate.latitude");
        post_process(&mut result, "90.001");
        assert_eq!(result.label, "geography.coordinate.longitude");
    }

    // ── Rule 6: email rescue ─────────────────────────────────────────────────

    #[test]
    fn test_email_rescues_from_hostname() {
        let mut result = make_result("technology.internet.hostname");
        post_process(&mut result, "bob@demo.net");
        assert_eq!(result.label, "identity.person.email");
    }

    #[test]
    fn test_email_rescues_from_username() {
        let mut result = make_result("identity.person.username");
        post_process(&mut result, "info@startup.co");
        assert_eq!(result.label, "identity.person.email");
    }

    #[test]
    fn test_email_rescues_from_slug() {
        let mut result = make_result("technology.internet.slug");
        post_process(&mut result, "hello@world.org");
        assert_eq!(result.label, "identity.person.email");
    }

    #[test]
    fn test_correct_email_unchanged() {
        let mut result = make_result("identity.person.email");
        post_process(&mut result, "alice@gmail.com");
        assert_eq!(result.label, "identity.person.email");
    }

    #[test]
    fn test_paypal_email_not_overridden() {
        let mut result = make_result("finance.payment.paypal_email");
        post_process(&mut result, "user@paypal.com");
        assert_eq!(result.label, "finance.payment.paypal_email");
    }

    #[test]
    fn test_non_email_hostname_unchanged() {
        // A plain hostname without @ should not become email
        let mut result = make_result("technology.internet.hostname");
        post_process(&mut result, "www.example.com");
        assert_eq!(result.label, "technology.internet.hostname");
    }

    #[test]
    fn test_url_with_at_not_email() {
        // URL with @ (e.g., basic auth) has :// so should not become email
        let mut result = make_result("technology.internet.url");
        post_process(&mut result, "https://user@host.com/path");
        assert_eq!(result.label, "technology.internet.url");
    }

    #[test]
    fn test_at_without_dot_not_email() {
        // @ without a dot in domain is not a valid email
        let mut result = make_result("identity.person.username");
        post_process(&mut result, "user@localhost");
        assert_eq!(result.label, "identity.person.username");
    }
}

#[cfg(test)]
mod pattern_validate_tests {
    use super::*;

    /// Helper to build a ClassificationResult with all_scores for fallback testing.
    fn make_result_with_scores(
        label: &str,
        confidence: f32,
        scores: Vec<(&str, f32)>,
    ) -> ClassificationResult {
        ClassificationResult {
            label: label.to_string(),
            confidence,
            all_scores: scores
                .into_iter()
                .map(|(l, s)| (l.to_string(), s))
                .collect(),
        }
    }

    fn make_patterns(pairs: Vec<(&str, &str)>) -> HashMap<String, Regex> {
        pairs
            .into_iter()
            .map(|(label, pat)| (label.to_string(), Regex::new(pat).unwrap()))
            .collect()
    }

    #[test]
    fn test_pattern_match_keeps_prediction() {
        // Input "SFO" matches IATA pattern ^[A-Z]{3}$ → keep prediction
        let mut result = make_result_with_scores(
            "geography.transportation.iata_code",
            0.9,
            vec![
                ("geography.transportation.iata_code", 0.9),
                ("representation.text.word", 0.05),
            ],
        );
        let patterns = make_patterns(vec![("geography.transportation.iata_code", r"^[A-Z]{3}$")]);

        pattern_validate(&mut result, "SFO", &patterns);
        assert_eq!(result.label, "geography.transportation.iata_code");
        assert_eq!(result.confidence, 0.9);
    }

    #[test]
    fn test_pattern_mismatch_triggers_fallback() {
        // Input "C85" does NOT match IATA pattern ^[A-Z]{3}$ → fall back
        let mut result = make_result_with_scores(
            "geography.transportation.iata_code",
            0.7,
            vec![
                ("geography.transportation.iata_code", 0.7),
                ("representation.text.word", 0.15),
                ("technology.internet.hostname", 0.05),
            ],
        );
        let patterns = make_patterns(vec![("geography.transportation.iata_code", r"^[A-Z]{3}$")]);

        pattern_validate(&mut result, "C85", &patterns);
        // Should fall back to "representation.text.word" (no pattern → accepted)
        assert_eq!(result.label, "representation.text.word");
        assert_eq!(result.confidence, 0.15);
    }

    #[test]
    fn test_no_pattern_for_predicted_type_keeps_prediction() {
        // Predicted type has no validation pattern → nothing to validate, keep it
        let mut result = make_result_with_scores(
            "representation.text.word",
            0.8,
            vec![
                ("representation.text.word", 0.8),
                ("geography.transportation.iata_code", 0.1),
            ],
        );
        let patterns = make_patterns(vec![("geography.transportation.iata_code", r"^[A-Z]{3}$")]);

        pattern_validate(&mut result, "hello", &patterns);
        assert_eq!(result.label, "representation.text.word");
        assert_eq!(result.confidence, 0.8);
    }

    #[test]
    fn test_fallback_skips_candidates_that_also_fail_pattern() {
        // Input "12AB" fails predicted type AND the first fallback's pattern,
        // but matches a later fallback's pattern.
        let mut result = make_result_with_scores(
            "geography.transportation.iata_code",
            0.6,
            vec![
                ("geography.transportation.iata_code", 0.6),
                ("identity.commerce.issn", 0.2), // has pattern, will fail
                ("representation.text.word", 0.1), // no pattern → accepted
            ],
        );
        let patterns = make_patterns(vec![
            ("geography.transportation.iata_code", r"^[A-Z]{3}$"),
            ("identity.commerce.issn", r"^\d{4}-\d{3}[\dX]$"),
        ]);

        pattern_validate(&mut result, "12AB", &patterns);
        // IATA fails (not 3 uppercase letters), ISSN fails (not DDDD-DDDX),
        // word has no pattern → accepted
        assert_eq!(result.label, "representation.text.word");
        assert_eq!(result.confidence, 0.1);
    }

    #[test]
    fn test_fallback_to_candidate_whose_pattern_matches() {
        // Input "12345678" predicted as ISSN (fails DDDD-DDDX pattern).
        // Fallback 1 (hash) has a pattern that also fails (wrong length).
        // Fallback 2 (postal_code) has a pattern that matches.
        let mut result = make_result_with_scores(
            "identity.commerce.issn",
            0.5,
            vec![
                ("identity.commerce.issn", 0.5),
                ("technology.cryptographic.hash", 0.3),
                ("geography.address.postal_code", 0.15),
            ],
        );
        let patterns = make_patterns(vec![
            ("identity.commerce.issn", r"^\d{4}-\d{3}[\dX]$"),
            (
                "technology.cryptographic.hash",
                r"^[a-f0-9]{32}$|^[a-f0-9]{40}$|^[a-f0-9]{64}$",
            ),
            ("geography.address.postal_code", r"^\d{3,10}$"),
        ]);

        pattern_validate(&mut result, "12345678", &patterns);
        // ISSN fails (no hyphen), hash fails (8 chars), postal_code matches
        assert_eq!(result.label, "geography.address.postal_code");
        assert_eq!(result.confidence, 0.15);
    }

    #[test]
    fn test_no_valid_fallback_keeps_original() {
        // All candidates have patterns and none match → keep original
        let mut result = make_result_with_scores(
            "geography.transportation.iata_code",
            0.6,
            vec![
                ("geography.transportation.iata_code", 0.6),
                ("identity.commerce.issn", 0.3),
            ],
        );
        let patterns = make_patterns(vec![
            ("geography.transportation.iata_code", r"^[A-Z]{3}$"),
            ("identity.commerce.issn", r"^\d{4}-\d{3}[\dX]$"),
        ]);

        pattern_validate(&mut result, "ZZZZ", &patterns);
        // IATA fails (4 chars), ISSN fails → keep original
        assert_eq!(result.label, "geography.transportation.iata_code");
        assert_eq!(result.confidence, 0.6);
    }

    #[test]
    fn test_empty_patterns_map_keeps_prediction() {
        let mut result = make_result_with_scores(
            "geography.transportation.iata_code",
            0.9,
            vec![("geography.transportation.iata_code", 0.9)],
        );
        let patterns: HashMap<String, Regex> = HashMap::new();

        pattern_validate(&mut result, "whatever", &patterns);
        assert_eq!(result.label, "geography.transportation.iata_code");
    }

    #[test]
    fn test_extract_validation_patterns_from_taxonomy() {
        // Test the extract helper with a real taxonomy
        use finetype_core::Taxonomy;

        // Try loading from labels/ directory
        let labels_path = std::path::PathBuf::from("../../labels");
        if labels_path.exists() {
            let taxonomy = Taxonomy::from_directory(&labels_path).unwrap();
            let patterns = extract_validation_patterns(&taxonomy);

            // We know at least iata_code and ip_v4 have patterns
            assert!(
                patterns.len() > 50,
                "Expected >50 validation patterns, got {}",
                patterns.len()
            );

            // Spot-check a few known patterns
            assert!(
                patterns.contains_key("geography.transportation.iata_code"),
                "iata_code should have a validation pattern"
            );
            assert!(
                patterns.contains_key("technology.internet.ip_v4"),
                "ip_v4 should have a validation pattern"
            );
        }
    }

    #[test]
    fn test_cabin_value_rejected_as_iata() {
        // Real-world scenario: Titanic "Cabin" values like "C85" predicted as iata_code.
        // Pattern ^[A-Z]{3}$ rejects "C85" (only 3 chars but includes digit).
        let mut result = make_result_with_scores(
            "geography.transportation.iata_code",
            0.65,
            vec![
                ("geography.transportation.iata_code", 0.65),
                ("representation.text.word", 0.2),
                ("technology.internet.hostname", 0.05),
            ],
        );
        let patterns = make_patterns(vec![("geography.transportation.iata_code", r"^[A-Z]{3}$")]);

        pattern_validate(&mut result, "C85", &patterns);
        assert_ne!(result.label, "geography.transportation.iata_code");
        // Should fall back to word (no pattern constraint)
        assert_eq!(result.label, "representation.text.word");
    }
}
