//! Shared Model2Vec resources for tokenizer and embedding matrix.
//!
//! `Model2VecResources` loads the tokenizer and token embedding matrix once,
//! then provides encoding methods used by multiple consumers:
//! - `SemanticHintClassifier` (header → type matching)
//! - `EntityClassifier` (value → entity subtype demotion)
//! - `SenseClassifier` (column → broad category routing)
//!
//! This avoids loading the ~7.4MB embedding matrix multiple times.
//! Artifacts are prepared by `scripts/prepare_model2vec.py` and stored
//! in `models/model2vec/`.

use crate::inference::InferenceError;
use candle_core::{DType, Device, Tensor};
use std::path::Path;

/// Shared Model2Vec tokenizer and embedding matrix.
///
/// Load once via [`Model2VecResources::load`] or [`Model2VecResources::from_bytes`],
/// then pass (via `Arc<Model2VecResources>`) to classifiers that need value encoding.
pub struct Model2VecResources {
    tokenizer: tokenizers::Tokenizer,
    /// Token embedding matrix: [vocab_size, embed_dim]
    embeddings: Tensor,
    device: Device,
}

impl Model2VecResources {
    /// Load from a directory containing `tokenizer.json` and `model.safetensors`.
    pub fn load<P: AsRef<Path>>(model_dir: P) -> Result<Self, InferenceError> {
        let dir = model_dir.as_ref();

        let tokenizer_bytes = std::fs::read(dir.join("tokenizer.json"))?;
        let model_bytes = std::fs::read(dir.join("model.safetensors"))?;

        Self::from_bytes(&tokenizer_bytes, &model_bytes)
    }

    /// Load from in-memory byte slices (for compile-time embedding via `build.rs`).
    pub fn from_bytes(tokenizer_bytes: &[u8], model_bytes: &[u8]) -> Result<Self, InferenceError> {
        let device = Device::Cpu;

        let tokenizer = tokenizers::Tokenizer::from_bytes(tokenizer_bytes).map_err(|e| {
            InferenceError::InvalidPath(format!("Failed to load Model2Vec tokenizer: {}", e))
        })?;

        // Load token embeddings — stored as float16, convert to float32 for computation
        let model_tensors = candle_core::safetensors::load_buffer(model_bytes, &device)?;
        let embeddings = model_tensors
            .get("embeddings")
            .ok_or_else(|| {
                InferenceError::InvalidPath(
                    "Missing 'embeddings' tensor in model.safetensors".into(),
                )
            })?
            .to_dtype(DType::F32)?;

        Ok(Self {
            tokenizer,
            embeddings,
            device,
        })
    }

    /// Embedding dimension (e.g. 128 for potion-base-4M).
    pub fn embed_dim(&self) -> Result<usize, InferenceError> {
        Ok(self.embeddings.dim(1)?)
    }

    /// Reference to the tokenizer (for consumers that need custom tokenization).
    pub fn tokenizer(&self) -> &tokenizers::Tokenizer {
        &self.tokenizer
    }

    /// Reference to the raw embedding matrix `[vocab_size, embed_dim]`.
    ///
    /// Clone is O(1) due to Arc-backed Tensor storage.
    pub fn embeddings(&self) -> &Tensor {
        &self.embeddings
    }

    /// Reference to the device.
    pub fn device(&self) -> &Device {
        &self.device
    }

    /// Encode a single string → L2-normalised embedding `[embed_dim]`.
    ///
    /// Pipeline: tokenize → filter PAD (id=0) → index_select → mean pool → L2 normalize.
    /// Returns `None` for empty/untokenizable input or zero-norm embeddings.
    pub fn encode_one(&self, text: &str) -> Option<Tensor> {
        if text.is_empty() {
            return None;
        }

        let encoding = self.tokenizer.encode(text, false).ok()?;
        let ids = encoding.get_ids();

        // Filter PAD tokens (id=0). We encode with add_special_tokens=false,
        // so CLS/SEP are not present.
        let valid_ids: Vec<u32> = ids.iter().copied().filter(|&id| id != 0).collect();
        if valid_ids.is_empty() {
            return None;
        }

        let id_tensor = Tensor::new(valid_ids.as_slice(), &self.device).ok()?;
        let token_embeds = self.embeddings.index_select(&id_tensor, 0).ok()?; // [n_tokens, dim]
        let mean_embed = token_embeds.mean(0).ok()?; // [dim]

        // L2 normalize
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

        (mean_embed / norm as f64).ok()
    }

    /// Encode multiple strings → unnormalised mean-pooled embeddings `[N, embed_dim]`.
    ///
    /// Each row is the mean of token embeddings for the corresponding input string.
    /// Rows for empty/untokenizable strings are zero vectors.
    ///
    /// The output is **not** L2-normalised — consumers that need normalisation
    /// (e.g. cosine similarity) should normalise per-row afterwards.
    /// This matches the entity classifier's existing encoding pattern.
    pub fn encode_batch(&self, texts: &[&str]) -> Result<Tensor, InferenceError> {
        let embed_dim = self.embeddings.dim(1)?;

        if texts.is_empty() {
            return Ok(Tensor::zeros((0, embed_dim), DType::F32, &self.device)?);
        }

        let mut all_embeddings: Vec<f32> = Vec::with_capacity(texts.len() * embed_dim);

        for text in texts {
            let encoding = self.tokenizer.encode(*text, false).map_err(|e| {
                InferenceError::InvalidPath(format!("Tokenizer encode failed: {}", e))
            })?;

            let ids = encoding.get_ids();
            let valid_ids: Vec<u32> = ids.iter().copied().filter(|&id| id != 0).collect();

            if valid_ids.is_empty() {
                // Zero embedding for empty/untokenizable values
                all_embeddings.extend(std::iter::repeat_n(0.0f32, embed_dim));
                continue;
            }

            let id_tensor = Tensor::new(valid_ids.as_slice(), &self.device)?;
            let token_embeds = self.embeddings.index_select(&id_tensor, 0)?;
            let mean_embed = token_embeds.mean(0)?;
            let row: Vec<f32> = mean_embed.to_vec1()?;
            all_embeddings.extend_from_slice(&row);
        }

        Ok(Tensor::from_vec(
            all_embeddings,
            (texts.len(), embed_dim),
            &self.device,
        )?)
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a minimal WordPiece tokenizer for testing.
    ///
    /// Vocab: [PAD]=0, [UNK]=1, email=2, phone=3, number=4, data=5
    fn make_test_tokenizer() -> tokenizers::Tokenizer {
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
            .expect("test tokenizer should parse")
    }

    /// Standard token embeddings [6, 4] for testing.
    fn make_test_embeddings(device: &Device) -> Tensor {
        #[rustfmt::skip]
        let emb_data: Vec<f32> = vec![
            0.0, 0.0, 0.0, 0.0, // [PAD] = 0
            0.0, 0.0, 0.0, 0.0, // [UNK] = 1
            0.0, 1.0, 0.0, 0.0, // "email" = 2
            0.0, 0.0, 1.0, 0.0, // "phone" = 3
            0.0, 0.0, 0.5, 0.5, // "number" = 4
            0.1, 0.1, 0.1, 0.1, // "data" = 5
        ];
        Tensor::from_vec(emb_data, (6, 4), device).expect("test embeddings")
    }

    /// Build test Model2VecResources with known embeddings.
    fn make_test_resources() -> Model2VecResources {
        let device = Device::Cpu;
        Model2VecResources {
            tokenizer: make_test_tokenizer(),
            embeddings: make_test_embeddings(&device),
            device,
        }
    }

    #[test]
    fn test_embed_dim() {
        let res = make_test_resources();
        assert_eq!(res.embed_dim().unwrap(), 4);
    }

    #[test]
    fn test_encode_one_known_token() {
        let res = make_test_resources();

        // "email" → token 2 → embedding [0, 1, 0, 0] → already unit norm
        let emb = res.encode_one("email").expect("should encode 'email'");
        let v: Vec<f32> = emb.to_vec1().unwrap();
        assert_eq!(v.len(), 4);
        // After L2 normalisation: [0, 1, 0, 0] (already unit)
        assert!(
            (v[1] - 1.0).abs() < 1e-5,
            "expected v[1] ≈ 1.0, got {}",
            v[1]
        );
        assert!(v[0].abs() < 1e-5);
        assert!(v[2].abs() < 1e-5);
        assert!(v[3].abs() < 1e-5);
    }

    #[test]
    fn test_encode_one_multi_token() {
        let res = make_test_resources();

        // "phone number" → tokens [3, 4] → embeddings [0,0,1,0] + [0,0,0.5,0.5]
        // Mean: [0, 0, 0.75, 0.25] → norm = sqrt(0.75² + 0.25²) = sqrt(0.625) ≈ 0.7906
        // Normalised: [0, 0, 0.9487, 0.3162]
        let emb = res
            .encode_one("phone number")
            .expect("should encode 'phone number'");
        let v: Vec<f32> = emb.to_vec1().unwrap();
        assert_eq!(v.len(), 4);

        // Verify L2 norm ≈ 1.0
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!(
            (norm - 1.0).abs() < 1e-4,
            "expected unit norm, got {}",
            norm
        );

        // Third component should be largest (phone=1.0 + number=0.5 → mean 0.75)
        assert!(v[2] > v[3], "expected v[2] > v[3]");
    }

    #[test]
    fn test_encode_one_unk_token() {
        let res = make_test_resources();

        // "xyz" → token [UNK]=1 → embedding [0,0,0,0] → zero norm → None
        let result = res.encode_one("xyz");
        assert!(result.is_none(), "UNK-only input should return None");
    }

    #[test]
    fn test_encode_one_empty() {
        let res = make_test_resources();
        assert!(res.encode_one("").is_none());
    }

    #[test]
    fn test_encode_one_pad_filtering() {
        let res = make_test_resources();

        // Tokens that resolve to PAD (id=0) should be filtered out.
        // With our test tokenizer, [PAD] is a known token but gets filtered.
        // "email" has a real token, so it should still work.
        let emb = res
            .encode_one("email")
            .expect("should encode despite PAD filtering");
        let v: Vec<f32> = emb.to_vec1().unwrap();
        assert!((v[1] - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_encode_batch_shapes() {
        let res = make_test_resources();

        let texts = &["email", "phone", "data"];
        let batch = res.encode_batch(texts).unwrap();

        // Shape should be [3, 4]
        assert_eq!(batch.dims(), &[3, 4]);
    }

    #[test]
    fn test_encode_batch_empty() {
        let res = make_test_resources();

        let batch = res.encode_batch(&[]).unwrap();
        assert_eq!(batch.dims(), &[0, 4]);
    }

    #[test]
    fn test_encode_batch_values_match_individual() {
        let res = make_test_resources();

        // encode_batch should produce the same (unnormalised) mean-pool as
        // manual tokenize → index_select → mean for each input.
        let texts = &["email", "phone number"];
        let batch = res.encode_batch(texts).unwrap();

        // Row 0: "email" → token 2 → [0, 1, 0, 0]
        let row0: Vec<f32> = batch.get(0).unwrap().to_vec1().unwrap();
        assert!((row0[1] - 1.0).abs() < 1e-5);

        // Row 1: "phone number" → tokens [3, 4] → mean([0,0,1,0], [0,0,0.5,0.5]) = [0, 0, 0.75, 0.25]
        let row1: Vec<f32> = batch.get(1).unwrap().to_vec1().unwrap();
        assert!(
            (row1[2] - 0.75).abs() < 1e-5,
            "expected 0.75, got {}",
            row1[2]
        );
        assert!(
            (row1[3] - 0.25).abs() < 1e-5,
            "expected 0.25, got {}",
            row1[3]
        );
    }

    #[test]
    fn test_encode_batch_unk_produces_zeros() {
        let res = make_test_resources();

        // "xyz" → [UNK] → zero embedding → zero row in batch
        let batch = res.encode_batch(&["xyz"]).unwrap();
        let row: Vec<f32> = batch.get(0).unwrap().to_vec1().unwrap();
        assert!(
            row.iter().all(|&v| v.abs() < 1e-8),
            "UNK input should produce zero row, got {:?}",
            row
        );
    }

    #[test]
    fn test_encode_batch_not_normalised() {
        let res = make_test_resources();

        // "data" → token 5 → [0.1, 0.1, 0.1, 0.1]
        // encode_batch should return this unnormalised
        let batch = res.encode_batch(&["data"]).unwrap();
        let row: Vec<f32> = batch.get(0).unwrap().to_vec1().unwrap();
        let norm: f32 = row.iter().map(|x| x * x).sum::<f32>().sqrt();

        // [0.1, 0.1, 0.1, 0.1] has norm ≈ 0.2, not 1.0
        assert!(
            (norm - 0.2).abs() < 1e-4,
            "encode_batch should NOT normalise; norm = {}",
            norm
        );
    }

    /// Integration test: load real model artifacts from disk (skip if not present).
    #[test]
    fn test_load_from_disk_if_available() {
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

        let resources = Model2VecResources::load(&model_dir).unwrap();

        // potion-base-4M has 128-dim embeddings
        assert_eq!(resources.embed_dim().unwrap(), 128);

        // encode_one should produce a unit-norm vector
        let emb = resources
            .encode_one("email address")
            .expect("should encode 'email address'");
        let v: Vec<f32> = emb.to_vec1().unwrap();
        assert_eq!(v.len(), 128);
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!(
            (norm - 1.0).abs() < 1e-3,
            "encode_one should return unit norm, got {}",
            norm
        );

        // encode_batch should produce [3, 128]
        let batch = resources.encode_batch(&["hello", "world", "test"]).unwrap();
        assert_eq!(batch.dims(), &[3, 128]);

        // encode_one("email") and encode_batch(["email"])[0] should be related
        // (encode_one normalises, encode_batch doesn't, but direction should match)
        let one = resources.encode_one("email").unwrap();
        let batch_one = resources.encode_batch(&["email"]).unwrap();
        let row0: Vec<f32> = batch_one.get(0).unwrap().to_vec1().unwrap();
        let one_v: Vec<f32> = one.to_vec1().unwrap();

        // Cosine similarity between normalised and unnormalised should be ~1.0
        let batch_norm: f32 = row0.iter().map(|x| x * x).sum::<f32>().sqrt();
        let dot: f32 = one_v.iter().zip(row0.iter()).map(|(a, b)| a * b).sum();
        let cos_sim = dot / batch_norm;
        assert!(
            cos_sim > 0.999,
            "encode_one and encode_batch should agree on direction, cos_sim = {}",
            cos_sim
        );
    }
}
