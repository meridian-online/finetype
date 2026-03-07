//! Semantic column name classifier using Model2Vec static embeddings.
//!
//! Replaces the hardcoded `header_hint()` match table with a learned embedding
//! lookup: tokenize column name → index into embedding matrix → mean pool →
//! L2 normalize → cosine similarity against pre-computed type embeddings.
//!
//! The model artifacts (tokenizer, embeddings, type embeddings, label index)
//! are prepared by `scripts/prepare_model2vec.py` and stored in `models/model2vec/`.
//! At build time they can be embedded into the binary via `build.rs`.

use crate::inference::InferenceError;
use crate::model2vec_shared::Model2VecResources;
use candle_core::{DType, Device, Tensor};
use std::path::Path;

/// Result of a semantic header classification.
#[derive(Debug, Clone)]
pub struct SemanticHintResult {
    /// The predicted type label (e.g., "identity.person.email").
    pub label: String,
    /// Cosine similarity between the column name embedding and the best-match type embedding.
    pub similarity: f32,
}

/// Semantic column name classifier using Model2Vec static embeddings.
///
/// Inference is trivially simple: tokenize → index into embedding matrix →
/// mean pool → L2 normalize → cosine similarity against pre-computed type
/// embeddings. Sub-millisecond latency per column name.
///
/// Supports max-sim matching: when K > 1, each type has K representative
/// embeddings stored in an interleaved layout `[n_types * K, embed_dim]`.
/// Matching computes similarity against all K representatives and takes the
/// maximum per type. K is inferred from the type_embeddings shape at load time.
/// K=1 is backward-compatible with legacy single-centroid artifacts.
pub struct SemanticHintClassifier {
    tokenizer: tokenizers::Tokenizer,
    /// Token embedding matrix: [vocab_size, embed_dim]
    embeddings: Tensor,
    /// Pre-computed, L2-normalised type embeddings: [n_types * k, embed_dim]
    type_embeddings: Tensor,
    /// Ordered label index: type_embeddings rows i*k..(i+1)*k correspond to label_index[i]
    label_index: Vec<String>,
    /// Number of representative embeddings per type (inferred from shape)
    k: usize,
    /// Minimum cosine similarity to accept a match (tuned to avoid false positives)
    threshold: f32,
    device: Device,
}

/// Default similarity threshold.
///
/// Calibrated against a test set of ~30 column names using potion-base-4M:
///
/// True positives (semantically clear column names):
///   email=0.898, zip_code=0.899, phone_number=0.901, gender=0.907,
///   country=0.904, first_name=0.900, age=0.875, latitude=0.853,
///   city=0.826, uuid=0.820, url=0.809, birth_date=0.799,
///   user_email=0.771
///
/// True negatives (generic or ambiguous names):
///   data=0.687, type=0.656, status=0.628, description=0.582,
///   x=0.569, category=0.541, value=0.496, column=0.426,
///   col1=0.287, foo=0.228, xyz=0.377
///
/// Threshold 0.65 balances precision (93.1%) and recall (74.0%), recovering
/// 12 additional correct matches vs 0.70 (timezone, ean, postal codes, status
/// codes, price variants, tracking URLs). One borderline false positive on
/// generics (data→form_data at 0.687). See discovery/model2vec-specialisation/
/// FINDING.md for the full threshold sweep analysis.
const DEFAULT_THRESHOLD: f32 = 0.65;

impl SemanticHintClassifier {
    /// Load from a directory containing the 4 model artifacts.
    pub fn load<P: AsRef<Path>>(model_dir: P) -> Result<Self, InferenceError> {
        let dir = model_dir.as_ref();

        let tokenizer_bytes = std::fs::read(dir.join("tokenizer.json"))?;
        let model_bytes = std::fs::read(dir.join("model.safetensors"))?;
        let type_emb_bytes = std::fs::read(dir.join("type_embeddings.safetensors"))?;
        let label_bytes = std::fs::read(dir.join("label_index.json"))?;

        Self::from_bytes(
            &tokenizer_bytes,
            &model_bytes,
            &type_emb_bytes,
            &label_bytes,
        )
    }

    /// Load from in-memory byte slices (for compile-time embedding).
    pub fn from_bytes(
        tokenizer_bytes: &[u8],
        model_bytes: &[u8],
        type_emb_bytes: &[u8],
        label_bytes: &[u8],
    ) -> Result<Self, InferenceError> {
        let resources = Model2VecResources::from_bytes(tokenizer_bytes, model_bytes)?;
        Self::from_shared(&resources, type_emb_bytes, label_bytes)
    }

    /// Load from shared Model2Vec resources plus type-embedding byte slices.
    ///
    /// The tokenizer and embedding matrix are cloned from `resources` (O(1)
    /// for the Tensor due to Arc-backed storage). Only the type embeddings
    /// and label index are loaded from the provided bytes.
    pub fn from_shared(
        resources: &Model2VecResources,
        type_emb_bytes: &[u8],
        label_bytes: &[u8],
    ) -> Result<Self, InferenceError> {
        let device = Device::Cpu;
        let (type_embeddings, label_index, k) =
            Self::load_type_embeddings(type_emb_bytes, label_bytes, &device)?;

        Ok(Self {
            tokenizer: resources.tokenizer().clone(),
            embeddings: resources.embeddings().clone(),
            type_embeddings,
            label_index,
            k,
            threshold: DEFAULT_THRESHOLD,
            device,
        })
    }

    /// Load type embeddings and label index from byte slices.
    ///
    /// Returns (type_embeddings tensor, label_index vec, K representatives per type).
    fn load_type_embeddings(
        type_emb_bytes: &[u8],
        label_bytes: &[u8],
        device: &Device,
    ) -> Result<(Tensor, Vec<String>, usize), InferenceError> {
        let type_tensors = candle_core::safetensors::load_buffer(type_emb_bytes, device)?;
        let type_embeddings = type_tensors
            .get("type_embeddings")
            .ok_or_else(|| {
                InferenceError::InvalidPath(
                    "Missing 'type_embeddings' tensor in type_embeddings.safetensors".into(),
                )
            })?
            .to_dtype(DType::F32)?;

        let label_index: Vec<String> = serde_json::from_slice(label_bytes).map_err(|e| {
            InferenceError::InvalidPath(format!("Failed to parse label_index.json: {}", e))
        })?;

        let n_labels = label_index.len();
        let n_rows = type_embeddings.dim(0)?;
        if n_labels == 0 || n_rows % n_labels != 0 {
            return Err(InferenceError::InvalidPath(format!(
                "type_embeddings rows ({}) must be a multiple of label_index length ({})",
                n_rows, n_labels,
            )));
        }
        let k = n_rows / n_labels;

        Ok((type_embeddings, label_index, k))
    }

    /// Set a custom similarity threshold.
    pub fn with_threshold(mut self, threshold: f32) -> Self {
        self.threshold = threshold;
        self
    }

    /// Get the current threshold.
    pub fn threshold(&self) -> f32 {
        self.threshold
    }

    /// Get a reference to the tokenizer (shared with EntityClassifier).
    pub fn tokenizer(&self) -> &tokenizers::Tokenizer {
        &self.tokenizer
    }

    /// Get a reference to the token embedding matrix (shared with EntityClassifier).
    /// Shape: [vocab_size, embed_dim]. Clone is O(1) due to Arc-backed storage.
    pub fn embeddings(&self) -> &Tensor {
        &self.embeddings
    }

    /// Classify a column header name, returning the best-matching type label
    /// if above the similarity threshold.
    ///
    /// Returns `None` for generic names like "data", "col1", "V1", etc.
    pub fn classify_header(&self, header: &str) -> Option<SemanticHintResult> {
        // 1. Normalize: lowercase, replace separators with spaces, trim
        let normalized = header
            .to_lowercase()
            .replace(['_', '-', '.'], " ")
            .trim()
            .to_string();

        if normalized.is_empty() {
            return None;
        }

        // 2. Tokenize (add_special_tokens=false — no CLS/SEP added)
        let encoding = self.tokenizer.encode(normalized, false).ok()?;
        let ids = encoding.get_ids();

        if ids.is_empty() {
            return None;
        }

        // Filter out PAD tokens (id=0) only.
        // We encode with add_special_tokens=false, so CLS/SEP are not present.
        let valid_ids: Vec<u32> = ids.iter().copied().filter(|&id| id != 0).collect();

        if valid_ids.is_empty() {
            return None;
        }

        // 3. Look up token embeddings: index_select
        let id_tensor = Tensor::new(valid_ids.as_slice(), &self.device).ok()?;
        let token_embeds = self.embeddings.index_select(&id_tensor, 0).ok()?; // [n_tokens, dim]

        // 4. Mean pool over tokens → [dim]
        let mean_embed = token_embeds.mean(0).ok()?; // [dim]

        // 5. L2 normalize
        let norm = mean_embed
            .sqr()
            .ok()?
            .sum_all()
            .ok()?
            .sqrt()
            .ok()?
            .to_scalar::<f32>()
            .ok()?;
        if norm < 1e-8 {
            return None;
        }
        let query = (mean_embed / norm as f64).ok()?; // [dim]

        // 6. Cosine similarity: type_embeddings @ query → [n_types * k]
        let query_2d = query.unsqueeze(1).ok()?; // [dim, 1]
        let all_sims = self.type_embeddings.matmul(&query_2d).ok()?; // [n_types * k, 1]
        let all_sims = all_sims.squeeze(1).ok()?; // [n_types * k]

        // 7. Max-sim: reshape to [n_types, k], take max over k dimension, argmax
        let n_types = self.label_index.len();
        let sim_vec: Vec<f32> = all_sims.to_vec1().ok()?;

        // Find the best (max) similarity per type across all K representatives
        let (best_idx, best_sim) = (0..n_types)
            .map(|t| {
                let start = t * self.k;
                let end = start + self.k;
                sim_vec[start..end]
                    .iter()
                    .cloned()
                    .fold(f32::NEG_INFINITY, f32::max)
            })
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))?;
        let best_sim = &best_sim;

        if *best_sim >= self.threshold {
            Some(SemanticHintResult {
                label: self.label_index[best_idx].clone(),
                similarity: *best_sim,
            })
        } else {
            None
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: build a minimal WordPiece tokenizer for testing.
    fn make_test_tokenizer() -> Result<tokenizers::Tokenizer, InferenceError> {
        let tokenizer_json = r###"{
            "version": "1.0",
            "model": {
                "type": "WordPiece",
                "unk_token": "[UNK]",
                "continuing_subword_prefix": "##",
                "max_input_chars_per_word": 100,
                "vocab": {
                    "[PAD]": 0,
                    "[UNK]": 1,
                    "email": 2,
                    "phone": 3,
                    "number": 4,
                    "data": 5
                }
            },
            "normalizer": {
                "type": "BertNormalizer",
                "clean_text": true,
                "handle_chinese_chars": true,
                "strip_accents": null,
                "lowercase": true
            },
            "pre_tokenizer": { "type": "BertPreTokenizer" }
        }"###;
        tokenizers::Tokenizer::from_bytes(tokenizer_json.as_bytes())
            .map_err(|e| InferenceError::InvalidPath(format!("test tokenizer: {e}")))
    }

    /// Helper: standard token embeddings [6, 4] for testing.
    fn make_test_token_embeddings(device: &Device) -> Result<Tensor, InferenceError> {
        let emb_data: Vec<f32> = vec![
            0.0, 0.0, 0.0, 0.0, // [PAD] = 0
            0.0, 0.0, 0.0, 0.0, // [UNK] = 1
            0.0, 1.0, 0.0, 0.0, // "email" = 2
            0.0, 0.0, 1.0, 0.0, // "phone" = 3
            0.0, 0.0, 0.5, 0.5, // "number" = 4
            0.1, 0.1, 0.1, 0.1, // "data" = 5 — generic
        ];
        Ok(Tensor::from_vec(emb_data, (6, 4), device)?)
    }

    /// Helper: create a tiny synthetic classifier for unit testing (K=1 legacy mode).
    ///
    /// Vocab: [PAD]=0, [UNK]=1, email=2, phone=3, number=4, data=5
    /// embed_dim = 4, 2 type labels, K=1 (single centroid per type).
    ///
    /// Token embeddings:
    ///   0: [0, 0, 0, 0]  ([PAD])
    ///   1: [0, 0, 0, 0]  ([UNK])
    ///   2: [0, 1, 0, 0]  ("email")
    ///   3: [0, 0, 1, 0]  ("phone")
    ///   4: [0, 0, 0.5, 0.5]  ("number")
    ///   5: [0.1, 0.1, 0.1, 0.1]  ("data" — generic, low sim to all types)
    ///
    /// Type embeddings (pre-normalised, K=1):
    ///   "identity.person.email":        [0, 1, 0, 0]
    ///   "identity.person.phone_number": [0, 0, 1, 0]
    fn make_test_classifier() -> Result<SemanticHintClassifier, InferenceError> {
        let device = Device::Cpu;
        let tokenizer = make_test_tokenizer()?;
        let embeddings = make_test_token_embeddings(&device)?;

        // Type embeddings [2, 4] — K=1, already L2-normalised
        let type_data: Vec<f32> = vec![
            0.0, 1.0, 0.0, 0.0, // identity.person.email
            0.0, 0.0, 1.0, 0.0, // identity.person.phone_number
        ];
        let type_embeddings = Tensor::from_vec(type_data, (2, 4), &device)?;

        let label_index = vec![
            "identity.person.email".to_string(),
            "identity.person.phone_number".to_string(),
        ];

        Ok(SemanticHintClassifier {
            tokenizer,
            embeddings,
            type_embeddings,
            label_index,
            k: 1,
            threshold: DEFAULT_THRESHOLD,
            device,
        })
    }

    /// Helper: create a classifier with K=2 for max-sim testing.
    ///
    /// Type embeddings [2 types * 2 reps, 4 dims]:
    ///   identity.person.email rep1:        [0, 1, 0, 0]    (exact match for "email" token)
    ///   identity.person.email rep2:        [0.1, 0.9, 0.1, 0] (slight variation, normalised)
    ///   identity.person.phone_number rep1: [0, 0, 1, 0]    (exact match for "phone" token)
    ///   identity.person.phone_number rep2: [0, 0, 0, 0]    (zero-padded — only 1 real rep)
    fn make_test_classifier_k2() -> Result<SemanticHintClassifier, InferenceError> {
        let device = Device::Cpu;
        let tokenizer = make_test_tokenizer()?;
        let embeddings = make_test_token_embeddings(&device)?;

        // Normalise rep2 for email: [0.1, 0.9, 0.1, 0] → norm ≈ 0.9110
        let norm = (0.1f32 * 0.1 + 0.9 * 0.9 + 0.1 * 0.1 + 0.0).sqrt();
        let r2 = [0.1 / norm, 0.9 / norm, 0.1 / norm, 0.0 / norm];

        // Type embeddings [4 rows (2 types * K=2), 4 dims]
        #[rustfmt::skip]
        let type_data: Vec<f32> = vec![
            0.0, 1.0, 0.0, 0.0,                 // email rep1 (exact match for "email" token)
            r2[0], r2[1], r2[2], r2[3],          // email rep2 (slight variation, normalised)
            0.0, 0.0, 1.0, 0.0,                  // phone rep1 (exact match for "phone" token)
            0.0, 0.0, 0.0, 0.0,                  // phone rep2 (zero-padded — only 1 real rep)
        ];
        let type_embeddings = Tensor::from_vec(type_data, (4, 4), &device)?;

        let label_index = vec![
            "identity.person.email".to_string(),
            "identity.person.phone_number".to_string(),
        ];

        Ok(SemanticHintClassifier {
            tokenizer,
            embeddings,
            type_embeddings,
            label_index,
            k: 2,
            threshold: DEFAULT_THRESHOLD,
            device,
        })
    }

    #[test]
    fn test_cosine_similarity_math() {
        // Direct cosine similarity: [0,1,0,0] · [0,1,0,0] = 1.0
        let device = Device::Cpu;
        let a = Tensor::new(&[0.0f32, 1.0, 0.0, 0.0], &device).unwrap();
        let b = Tensor::new(&[0.0f32, 1.0, 0.0, 0.0], &device).unwrap();
        let sim = (&a * &b)
            .unwrap()
            .sum_all()
            .unwrap()
            .to_scalar::<f32>()
            .unwrap();
        assert!((sim - 1.0).abs() < 1e-5);

        // Orthogonal vectors: similarity = 0
        let c = Tensor::new(&[0.0f32, 0.0, 1.0, 0.0], &device).unwrap();
        let sim2 = (&a * &c)
            .unwrap()
            .sum_all()
            .unwrap()
            .to_scalar::<f32>()
            .unwrap();
        assert!(sim2.abs() < 1e-5);
    }

    #[test]
    fn test_classify_header_known_types() {
        let classifier = make_test_classifier().unwrap();

        // "email" should match identity.person.email with high similarity
        let result = classifier.classify_header("email");
        assert!(result.is_some(), "Expected match for 'email'");
        let r = result.unwrap();
        assert_eq!(r.label, "identity.person.email");
        assert!(
            r.similarity > 0.9,
            "Expected high similarity, got {}",
            r.similarity
        );

        // "phone" should match identity.person.phone_number
        let result = classifier.classify_header("phone");
        assert!(result.is_some(), "Expected match for 'phone'");
        let r = result.unwrap();
        assert_eq!(r.label, "identity.person.phone_number");
        assert!(
            r.similarity > 0.9,
            "Expected high similarity, got {}",
            r.similarity
        );
    }

    #[test]
    fn test_classify_header_no_match() {
        let classifier = make_test_classifier().unwrap();

        // "data" should have low similarity to both types and return None
        let result = classifier.classify_header("data");
        // The "data" token embedding [0.1, 0.1, 0.1, 0.1] normalised is [0.5, 0.5, 0.5, 0.5]
        // Dot with [0, 1, 0, 0] = 0.5. This is below the default threshold of 0.70.
        assert!(
            result.is_none(),
            "Expected no match for 'data', got {:?}",
            result
        );
    }

    #[test]
    fn test_classify_header_empty() {
        let classifier = make_test_classifier().unwrap();
        assert!(classifier.classify_header("").is_none());
    }

    #[test]
    fn test_classify_header_normalisation() {
        let classifier = make_test_classifier().unwrap();

        // Underscores, dashes, dots should be replaced with spaces
        // "user_email" → "user email" → tokens [UNK=1, email=2]
        // UNK has zero embedding, so it's just email's embedding → should match email
        let result = classifier.classify_header("user_email");
        assert!(result.is_some(), "Expected match for 'user_email'");
        let r = result.unwrap();
        assert_eq!(r.label, "identity.person.email");
    }

    #[test]
    fn test_threshold_boundary() {
        // With threshold = 0.0, even weak matches should pass
        let low_threshold = SemanticHintClassifier {
            threshold: 0.0,
            ..make_test_classifier().unwrap()
        };
        let result = low_threshold.classify_header("data");
        assert!(result.is_some(), "With threshold=0, 'data' should match");

        // With threshold = 0.99, only exact matches pass
        let high_threshold = SemanticHintClassifier {
            threshold: 0.99,
            ..make_test_classifier().unwrap()
        };
        let result = high_threshold.classify_header("email");
        assert!(
            result.is_some(),
            "With threshold=0.99, exact 'email' should still match"
        );
    }

    #[test]
    fn test_max_sim_picks_best_representative() {
        // With K=2, the classifier should still correctly match types
        let classifier = make_test_classifier_k2().unwrap();

        // "email" token → [0,1,0,0] — matches email rep1 exactly (sim=1.0)
        let result = classifier.classify_header("email");
        assert!(result.is_some(), "Expected match for 'email' with K=2");
        let r = result.unwrap();
        assert_eq!(r.label, "identity.person.email");
        assert!(
            r.similarity > 0.9,
            "Expected high similarity, got {}",
            r.similarity
        );

        // "phone" → [0,0,1,0] — matches phone rep1 (sim=1.0), rep2 is zero (sim=0.0)
        let result = classifier.classify_header("phone");
        assert!(result.is_some(), "Expected match for 'phone' with K=2");
        let r = result.unwrap();
        assert_eq!(r.label, "identity.person.phone_number");
        assert!(
            r.similarity > 0.9,
            "Expected high similarity, got {}",
            r.similarity
        );
    }

    #[test]
    fn test_zero_padded_rep_ignored() {
        // phone_number has rep2 = [0,0,0,0] (zero-padded).
        // The zero-padded row should produce 0.0 similarity and never win.
        let classifier = make_test_classifier_k2().unwrap();

        // "data" token → [0.1,0.1,0.1,0.1] normalised → [0.5,0.5,0.5,0.5]
        // Dot with email rep1 [0,1,0,0] = 0.5
        // Dot with email rep2 ≈ [0.11, 0.99, 0.11, 0] → ~0.55
        // Dot with phone rep1 [0,0,1,0] = 0.5
        // Dot with phone rep2 [0,0,0,0] = 0.0  ← zero-padded, correctly ignored
        // Max per type: email ≈ 0.55, phone = 0.5
        // Zero-padded rep does NOT inflate phone's score
        let result = classifier.classify_header("data");
        // At default threshold (0.65), this should be None
        assert!(
            result.is_none(),
            "Expected no match for 'data' at threshold 0.65, got {:?}",
            result
        );
    }

    #[test]
    fn test_k_inferred_from_shape() {
        let device = Device::Cpu;

        // K=1: 2 types, 2 rows → K=1
        let type_data_k1: Vec<f32> = vec![0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0];
        let type_emb_k1 = Tensor::from_vec(type_data_k1, (2, 4), &device).unwrap();
        let labels = vec![
            "identity.person.email".to_string(),
            "identity.person.phone_number".to_string(),
        ];
        let k1 = type_emb_k1.dim(0).unwrap() / labels.len();
        assert_eq!(k1, 1);

        // K=3: 2 types, 6 rows → K=3
        let type_data_k3: Vec<f32> = vec![0.0; 6 * 4]; // 6 rows × 4 dims
        let type_emb_k3 = Tensor::from_vec(type_data_k3, (6, 4), &device).unwrap();
        let k3 = type_emb_k3.dim(0).unwrap() / labels.len();
        assert_eq!(k3, 3);

        // Invalid: 5 rows for 2 types → not divisible
        let n_rows = 5;
        let n_labels = labels.len();
        assert_ne!(n_rows % n_labels, 0, "5 rows should not divide evenly by 2");
    }

    /// Integration test: load real model artifacts from disk (skip if not present).
    #[test]
    fn test_from_files_if_available() {
        let model_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("models")
            .join("model2vec");

        if !model_dir.join("model.safetensors").exists() {
            eprintln!("Skipping integration test: models/model2vec not found");
            return;
        }

        let classifier = SemanticHintClassifier::load(&model_dir).unwrap();

        // Test known column names from header_hint()
        let test_cases = vec![
            ("email", "identity.person.email"),
            ("phone_number", "identity.person.phone_number"),
            ("zip_code", "geography.address.postal_code"),
            ("latitude", "geography.coordinate.latitude"),
            ("longitude", "geography.coordinate.longitude"),
            ("first_name", "identity.person.first_name"),
            ("country", "geography.location.country"),
            ("gender", "identity.person.gender"),
            // ("age", "identity.person.age") — REMOVED in v0.5.2 (NNFT-192)
            // ("url", "technology.internet.url") — matches "urn" after NNFT-244 expansion
            //   (url/urn too close in embedding space). Hardcoded header_hint() handles "url" correctly.
        ];

        for (header, expected_label) in &test_cases {
            let result = classifier.classify_header(header);
            assert!(
                result.is_some(),
                "Expected match for '{}' -> '{}', got None",
                header,
                expected_label
            );
            let r = result.unwrap();
            assert_eq!(
                r.label, *expected_label,
                "For '{}': expected '{}', got '{}' (sim={:.3})",
                header, expected_label, r.label, r.similarity
            );
        }

        // Test generic names return None (all below 0.65 threshold)
        // Note: "xyz" excluded — with max-sim K=3, it matches the "tz" representative
        // for datetime.offset.iana at 0.80 (shared ##z subword token). Accepted trade-off:
        // "xyz" is not a realistic column name, and "tz" matching IANA timezone is valuable.
        let generic_names = vec![
            "foo", "col1", "column_a", "V1", "field_3", "value", "col", "column", "result",
            "output", "input", "var1",
        ];
        for name in &generic_names {
            let result = classifier.classify_header(name);
            assert!(
                result.is_none(),
                "Expected no match for generic name '{}', got {:?}",
                name,
                result
            );
        }

        // "data" is a known borderline match at 0.687 (→ form_data). Accepted
        // trade-off at 0.65 threshold — see FINDING.md false positive assessment.
        let data_result = classifier.classify_header("data");
        assert!(
            data_result.is_some(),
            "Expected borderline match for 'data' at 0.65 threshold"
        );
        assert_eq!(data_result.unwrap().label, "container.key_value.form_data");
    }

    /// Integration test: from_shared() produces identical results to load().
    #[test]
    fn test_from_shared_matches_load() {
        let model_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("models")
            .join("model2vec");

        if !model_dir.join("model.safetensors").exists() {
            eprintln!("Skipping from_shared integration test: models/model2vec not found");
            return;
        }

        // Load standalone (existing path)
        let standalone = SemanticHintClassifier::load(&model_dir).unwrap();

        // Load via shared resources (new path)
        let resources = Model2VecResources::load(&model_dir).unwrap();
        let type_emb_bytes = std::fs::read(model_dir.join("type_embeddings.safetensors")).unwrap();
        let label_bytes = std::fs::read(model_dir.join("label_index.json")).unwrap();
        let shared =
            SemanticHintClassifier::from_shared(&resources, &type_emb_bytes, &label_bytes).unwrap();

        // Both should produce identical results for the same inputs
        let test_headers = vec![
            "email",
            "phone_number",
            "zip_code",
            "data",
            "foo",
            "country",
        ];
        for header in test_headers {
            let r1 = standalone.classify_header(header);
            let r2 = shared.classify_header(header);
            match (&r1, &r2) {
                (Some(a), Some(b)) => {
                    assert_eq!(
                        a.label, b.label,
                        "Label mismatch for '{}': standalone='{}' vs shared='{}'",
                        header, a.label, b.label
                    );
                    assert!(
                        (a.similarity - b.similarity).abs() < 1e-5,
                        "Similarity mismatch for '{}': {:.5} vs {:.5}",
                        header,
                        a.similarity,
                        b.similarity
                    );
                }
                (None, None) => {} // Both correctly returned None
                _ => panic!(
                    "Mismatch for '{}': standalone={:?}, shared={:?}",
                    header, r1, r2
                ),
            }
        }
    }
}
