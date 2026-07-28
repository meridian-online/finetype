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

/// Storage dtypes accepted for the token embedding matrix.
///
/// The matrix is a lookup table, not a trained graph — every consumer pulls rows out
/// of it with `index_select` and does F32 arithmetic from there — so the dtype it is
/// *stored* in is packaging, not precision that inference depends on. F16 halves the
/// file, and halves what a release binary carries (build.rs pastes it in with
/// `include_bytes!`), for embeddings whose column-feature cosine against the F32
/// original is 1.0 to six decimal places.
///
/// The set is stated rather than inferred because `to_dtype` is not a validator: an
/// integer tensor would convert without complaint and give a matrix of
/// plausible-looking garbage. A dtype nobody meant to ship should fail at load with
/// its own name in the message.
const ACCEPTED_EMBEDDING_DTYPES: [DType; 3] = [DType::F16, DType::BF16, DType::F32];

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

        // Load token embeddings. Either F16 or F32 on disk (see
        // ACCEPTED_EMBEDDING_DTYPES); up-cast once, here, so everything downstream
        // sees one dtype and no caller has to know how the artifact was packed.
        let model_tensors = candle_core::safetensors::load_buffer(model_bytes, &device)?;
        let stored = model_tensors.get("embeddings").ok_or_else(|| {
            InferenceError::InvalidPath("Missing 'embeddings' tensor in model.safetensors".into())
        })?;
        if !ACCEPTED_EMBEDDING_DTYPES.contains(&stored.dtype()) {
            return Err(InferenceError::InvalidPath(format!(
                "'embeddings' is stored as {:?}, which is not a float dtype this loader \
                 will up-cast (accepted: {:?})",
                stored.dtype(),
                ACCEPTED_EMBEDDING_DTYPES,
            )));
        }
        let embeddings = stored.to_dtype(DType::F32)?;

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

    /// Encode multiple strings → L2-normalised mean-pooled embeddings `[N, embed_dim]`.
    ///
    /// Each row is the mean of token embeddings for the corresponding input string,
    /// then L2-normalised to unit length. This matches the output of Python's
    /// `model2vec.StaticModel.encode()` which always returns normalised vectors.
    ///
    /// Rows for empty/untokenizable strings are zero vectors (norm=0).
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
            let mut row: Vec<f32> = mean_embed.to_vec1()?;

            // L2-normalise to match Python model2vec.encode() output
            let norm: f32 = row.iter().map(|v| v * v).sum::<f32>().sqrt();
            if norm > 1e-8 {
                for v in &mut row {
                    *v /= norm;
                }
            }

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

    /// Minimal WordPiece tokenizer for testing.
    ///
    /// Vocab: [PAD]=0, [UNK]=1, email=2, phone=3, number=4, data=5
    const TEST_TOKENIZER_JSON: &str = r###"{
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

    fn make_test_tokenizer() -> tokenizers::Tokenizer {
        tokenizers::Tokenizer::from_bytes(TEST_TOKENIZER_JSON.as_bytes())
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

        // encode_batch should produce L2-normalised mean-pool for each input,
        // matching Python model2vec.encode() output.
        let texts = &["email", "phone number"];
        let batch = res.encode_batch(texts).unwrap();

        // Row 0: "email" → token 2 → [0, 1, 0, 0] → L2-norm → [0, 1, 0, 0]
        let row0: Vec<f32> = batch.get(0).unwrap().to_vec1().unwrap();
        assert!((row0[1] - 1.0).abs() < 1e-5);

        // Row 1: "phone number" → tokens [3, 4] → mean([0,0,1,0], [0,0,0.5,0.5]) = [0, 0, 0.75, 0.25]
        // L2-norm of [0, 0, 0.75, 0.25] = sqrt(0.75^2 + 0.25^2) = sqrt(0.625) ≈ 0.7906
        // Normalised: [0, 0, 0.75/0.7906, 0.25/0.7906] ≈ [0, 0, 0.9487, 0.3162]
        let row1: Vec<f32> = batch.get(1).unwrap().to_vec1().unwrap();
        let norm: f32 = (0.75f32.powi(2) + 0.25f32.powi(2)).sqrt();
        assert!(
            (row1[2] - 0.75 / norm).abs() < 1e-4,
            "expected {}, got {}",
            0.75 / norm,
            row1[2]
        );
        assert!(
            (row1[3] - 0.25 / norm).abs() < 1e-4,
            "expected {}, got {}",
            0.25 / norm,
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
    fn test_encode_batch_normalised() {
        let res = make_test_resources();

        // "data" → token 5 → [0.1, 0.1, 0.1, 0.1]
        // encode_batch returns L2-normalised vectors matching Python model2vec.encode()
        let batch = res.encode_batch(&["data"]).unwrap();
        let row: Vec<f32> = batch.get(0).unwrap().to_vec1().unwrap();
        let norm: f32 = row.iter().map(|x| x * x).sum::<f32>().sqrt();

        // [0.1, 0.1, 0.1, 0.1] → L2-norm → [0.5, 0.5, 0.5, 0.5], norm = 1.0
        assert!(
            (norm - 1.0).abs() < 1e-4,
            "encode_batch should L2-normalise; norm = {}",
            norm
        );
        // Each component should be 0.5 (= 0.1 / 0.2)
        assert!((row[0] - 0.5).abs() < 1e-4, "expected 0.5, got {}", row[0]);
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

    // ── stored dtype ────────────────────────────────────────────────────────────
    // The embedding matrix is a lookup table, so what it is stored as and what it is
    // computed in are separate decisions. These cases hold that separation: F16 on
    // disk must load, must arrive as F32, and must agree with the F32 file it was
    // packed from — and a dtype that is not a float at all must be refused rather
    // than converted into a matrix of plausible numbers.

    /// Write a loadable model dir (tokenizer + safetensors) under a unique temp path.
    fn write_model_dir(case: &str, embeddings: &Tensor) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("finetype-m2v-dtype-{}-{case}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp model dir");
        std::fs::write(dir.join("tokenizer.json"), TEST_TOKENIZER_JSON).expect("write tokenizer");
        let mut map = std::collections::HashMap::new();
        map.insert("embeddings".to_string(), embeddings.clone());
        candle_core::safetensors::save(&map, dir.join("model.safetensors")).expect("save weights");
        dir
    }

    /// A deterministic 6x8 matrix whose entries are NOT exactly representable in F16,
    /// so an F16 round trip genuinely perturbs it. Multiply-add only — no libm — so
    /// the same bytes come out on every platform.
    fn drifting_embeddings(device: &Device) -> Tensor {
        let data: Vec<f32> = (0..48).map(|i| (i as f32) * 0.017 + 0.003).collect();
        Tensor::from_vec(data, (6, 8), device).expect("test embeddings")
    }

    #[test]
    fn test_f16_stored_embeddings_load_and_upcast() {
        let device = Device::Cpu;
        let f16 = make_test_embeddings(&device)
            .to_dtype(DType::F16)
            .expect("cast to f16");
        let dir = write_model_dir("upcast", &f16);

        let res = Model2VecResources::load(&dir).expect("F16 embeddings should load");

        // Half precision is a storage decision only: everything downstream reads f32.
        assert_eq!(
            res.embeddings().dtype(),
            DType::F32,
            "embeddings must be up-cast to F32 at load"
        );
        assert_eq!(res.embed_dim().unwrap(), 4);

        // "email" = token 2 = [0, 1, 0, 0], all exactly representable in F16.
        let v: Vec<f32> = res
            .encode_one("email")
            .expect("F16-stored embeddings must still encode")
            .to_vec1()
            .unwrap();
        assert_eq!(v, vec![0.0, 1.0, 0.0, 0.0]);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_f16_and_f32_storage_agree_on_encodings() {
        let device = Device::Cpu;
        let f32_emb = drifting_embeddings(&device);
        let f16_emb = f32_emb.to_dtype(DType::F16).expect("cast to f16");

        let dir32 = write_model_dir("agree-f32", &f32_emb);
        let dir16 = write_model_dir("agree-f16", &f16_emb);
        let res32 = Model2VecResources::load(&dir32).expect("F32 dir loads");
        let res16 = Model2VecResources::load(&dir16).expect("F16 dir loads");

        let texts = &["email", "phone number", "data email phone"];
        let a = res32.encode_batch(texts).unwrap();
        let b = res16.encode_batch(texts).unwrap();
        assert_eq!(a.dims(), b.dims());

        let mut any_difference = false;
        for (i, text) in texts.iter().enumerate() {
            let ra: Vec<f32> = a.get(i).unwrap().to_vec1().unwrap();
            let rb: Vec<f32> = b.get(i).unwrap().to_vec1().unwrap();
            if ra != rb {
                any_difference = true;
            }
            let dot: f32 = ra.iter().zip(&rb).map(|(x, y)| x * y).sum();
            let na: f32 = ra.iter().map(|x| x * x).sum::<f32>().sqrt();
            let nb: f32 = rb.iter().map(|x| x * x).sum::<f32>().sqrt();
            let cos = dot / (na * nb);
            assert!(cos > 0.9999, "row {i} ({text}) diverged: cos_sim = {cos}");
            let max_abs = ra
                .iter()
                .zip(&rb)
                .map(|(x, y)| (x - y).abs())
                .fold(0.0f32, f32::max);
            assert!(max_abs < 1.0e-2, "row {i} ({text}) max |Δ| = {max_abs}");
        }

        // Without this the agreement above could be vacuous — two dirs holding the
        // same bytes agree trivially. The fixture is chosen so F16 really does move
        // the numbers; the point is that it does not move them enough to matter.
        assert!(
            any_difference,
            "fixture is not exercising the F16 round trip: every component came back bit-identical"
        );

        let _ = std::fs::remove_dir_all(&dir32);
        let _ = std::fs::remove_dir_all(&dir16);
    }

    #[test]
    fn test_non_float_stored_embeddings_are_rejected() {
        let device = Device::Cpu;
        let ints = Tensor::from_vec(vec![0i64, 1, 2, 3, 4, 5, 6, 7], (2, 4), &device)
            .expect("int embeddings");
        let dir = write_model_dir("reject-int", &ints);

        let err = Model2VecResources::load(&dir)
            .err()
            .expect("an integer embedding matrix must not load as if it were weights");
        let msg = err.to_string();
        assert!(
            msg.contains("I64"),
            "the rejection must name the dtype it found, got: {msg}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }
}
