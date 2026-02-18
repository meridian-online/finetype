//! Tiered inference engine for hierarchical text classification.
//!
//! Chains multiple CharCNN models in a hierarchy:
//! - Tier 0: Broad type classification (VARCHAR, DATE, TIMESTAMP, etc.)
//! - Tier 1: Category classification within a broad type
//! - Tier 2: Specific type classification within a category
//!
//! The engine loads models from a directory structure created by `TieredTrainer`.

use crate::char_cnn::{CharCnn, CharCnnConfig, CharVocab};
use crate::inference::{ClassificationResult, InferenceError, ValueClassifier};
use candle_core::{DType, Device, Tensor};
use candle_nn::VarBuilder;
use std::collections::HashMap;
use std::path::Path;

/// Metadata for the tier graph, loaded from tier_graph.json.
#[derive(Debug, Clone, serde::Deserialize)]
#[allow(dead_code)]
struct TierGraphMeta {
    tier0: Tier0Meta,
    tier1: HashMap<String, Tier1Meta>,
    tier2: HashMap<String, Tier2Meta>,
    #[serde(default = "default_tier2_min")]
    tier2_min_types: usize,
}

fn default_tier2_min() -> usize {
    1
}

#[derive(Debug, Clone, serde::Deserialize)]
#[allow(dead_code)]
struct Tier0Meta {
    dir: String,
    broad_types: Vec<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[allow(dead_code)]
struct Tier1Meta {
    #[serde(default)]
    dir: Option<String>,
    #[serde(default)]
    direct: Option<String>,
    #[serde(default)]
    categories: Option<Vec<String>>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[allow(dead_code)]
struct Tier2Meta {
    #[serde(default)]
    dir: Option<String>,
    #[serde(default)]
    direct: Option<String>,
    #[serde(default)]
    types: Option<Vec<String>>,
    count: usize,
}

/// A loaded CharCNN model with its label mapping.
#[allow(dead_code)]
struct LoadedModel {
    model: CharCnn,
    index_to_label: HashMap<usize, String>,
    label_to_index: HashMap<String, usize>,
}

/// Timing breakdown for tiered inference.
#[derive(Debug, Clone)]
pub struct TierTiming {
    /// Time spent encoding inputs (ms)
    pub encode_ms: f64,
    /// Time spent on Tier 0 — broad type classification (ms)
    pub tier0_ms: f64,
    /// Time spent on Tier 1 — category classification (ms)
    pub tier1_ms: f64,
    /// Number of Tier 1 models invoked
    pub tier1_models: usize,
    /// Time spent on Tier 2 — specific type classification (ms)
    pub tier2_ms: f64,
    /// Number of Tier 2 models invoked
    pub tier2_models: usize,
    /// Total wall time (ms)
    pub total_ms: f64,
}

/// Tiered classifier that chains multiple models.
pub struct TieredClassifier {
    /// Tier 0 broad type model
    tier0: LoadedModel,
    /// Tier 1 models: broad_type → model
    tier1: HashMap<String, LoadedModel>,
    /// Tier 1 direct resolutions: broad_type → single category name
    tier1_direct: HashMap<String, String>,
    /// Tier 2 models: "{broad_type}_{category}" → model
    tier2: HashMap<String, LoadedModel>,
    /// Tier 2 direct resolutions: "{broad_type}_{category}" → single full label
    tier2_direct: HashMap<String, String>,
    /// Character vocabulary (shared across all models)
    vocab: CharVocab,
    device: Device,
    max_seq_length: usize,
}

impl TieredClassifier {
    /// Load a tiered classifier from a directory.
    ///
    /// Expected structure:
    /// ```text
    /// model_dir/
    ///   tier_graph.json
    ///   tier0/                        # Broad type model
    ///   tier1_{broad_type}/           # Category models
    ///   tier2_{broad_type}_{category}/ # Type models
    /// ```
    pub fn load<P: AsRef<Path>>(model_dir: P) -> Result<Self, InferenceError> {
        let model_dir = model_dir.as_ref();
        let device = Self::get_device();

        // Load graph metadata
        let graph_path = model_dir.join("tier_graph.json");
        let graph_str = std::fs::read_to_string(&graph_path).map_err(|e| {
            InferenceError::InvalidPath(format!("Failed to read tier_graph.json: {}", e))
        })?;
        let graph_meta: TierGraphMeta = serde_json::from_str(&graph_str).map_err(|e| {
            InferenceError::InvalidPath(format!("Failed to parse tier_graph.json: {}", e))
        })?;

        eprintln!(
            "Loading tiered model: {} broad types",
            graph_meta.tier0.broad_types.len()
        );

        // Load Tier 0
        let tier0 = Self::load_model(&model_dir.join(&graph_meta.tier0.dir), &device)?;
        eprintln!("  Tier 0: {} classes loaded", tier0.index_to_label.len());

        // Load Tier 1 models
        let mut tier1 = HashMap::new();
        let mut tier1_direct = HashMap::new();

        for (broad_type, meta) in &graph_meta.tier1 {
            if let Some(dir) = &meta.dir {
                let model = Self::load_model(&model_dir.join(dir), &device)?;
                eprintln!(
                    "  Tier 1 [{}]: {} categories loaded",
                    broad_type,
                    model.index_to_label.len()
                );
                tier1.insert(broad_type.clone(), model);
            } else if let Some(direct) = &meta.direct {
                tier1_direct.insert(broad_type.clone(), direct.clone());
            }
        }

        // Load Tier 2 models
        let mut tier2 = HashMap::new();
        let mut tier2_direct = HashMap::new();

        for (key, meta) in &graph_meta.tier2 {
            if let Some(dir) = &meta.dir {
                let model = Self::load_model(&model_dir.join(dir), &device)?;
                eprintln!(
                    "  Tier 2 [{}]: {} types loaded",
                    key,
                    model.index_to_label.len()
                );
                tier2.insert(key.clone(), model);
            } else if let Some(direct) = &meta.direct {
                tier2_direct.insert(key.clone(), direct.clone());
            }
        }

        let vocab = CharVocab::new();
        let max_seq_length = 128; // Matches training default

        eprintln!(
            "Tiered model loaded: {} tier1 models, {} tier2 models",
            tier1.len(),
            tier2.len()
        );

        Ok(Self {
            tier0,
            tier1,
            tier1_direct,
            tier2,
            tier2_direct,
            vocab,
            device,
            max_seq_length,
        })
    }

    /// Classify a single text input through the tier chain.
    pub fn classify(&self, text: &str) -> Result<ClassificationResult, InferenceError> {
        let results = self.classify_batch(&[text.to_string()])?;
        Ok(results.into_iter().next().unwrap())
    }

    /// Classify multiple text inputs through the tier chain.
    ///
    /// Uses group-then-batch processing: after each tier, samples are grouped
    /// by their predicted class, then batch-forwarded through the next tier's
    /// model. This reduces forward passes from O(N) to O(num_models) per tier.
    pub fn classify_batch(
        &self,
        texts: &[String],
    ) -> Result<Vec<ClassificationResult>, InferenceError> {
        let batch_size = texts.len();

        // Encode all inputs once (shared across tiers)
        let input_ids = self.encode_batch(texts)?;

        // --- Tier 0: batch forward all inputs ---
        let tier0_results = self.run_model(&self.tier0, &input_ids)?;

        // Per-sample state: (broad_type, t0_conf, category, t1_conf, label, t2_conf)
        let mut broad_types: Vec<String> = Vec::with_capacity(batch_size);
        let mut t0_confs: Vec<f32> = Vec::with_capacity(batch_size);
        let mut categories: Vec<String> = vec![String::new(); batch_size];
        let mut t1_confs: Vec<f32> = vec![0.0; batch_size];
        let mut final_labels: Vec<String> = vec![String::new(); batch_size];
        let mut t2_confs: Vec<f32> = vec![0.0; batch_size];

        for (bt, conf) in &tier0_results {
            broad_types.push(bt.clone());
            t0_confs.push(*conf);
        }

        // --- Tier 1: group by broad_type, batch each group ---
        // Collect indices per T1 model key
        let mut t1_groups: HashMap<String, Vec<usize>> = HashMap::new();
        for (i, bt) in broad_types.iter().enumerate() {
            if self.tier1.contains_key(bt) {
                t1_groups.entry(bt.clone()).or_default().push(i);
            } else if let Some(direct) = self.tier1_direct.get(bt) {
                categories[i] = direct.clone();
                t1_confs[i] = 1.0;
            } else {
                categories[i] = "unknown".to_string();
                t1_confs[i] = 0.0;
            }
        }

        for (bt, indices) in &t1_groups {
            let model = &self.tier1[bt];
            let group_input = self.gather_rows(&input_ids, indices)?;
            let results = self.run_model(model, &group_input)?;
            for (j, idx) in indices.iter().enumerate() {
                categories[*idx] = results[j].0.clone();
                t1_confs[*idx] = results[j].1;
            }
        }

        // --- Tier 2: group by (broad_type, category), batch each group ---
        let mut t2_groups: HashMap<String, Vec<usize>> = HashMap::new();
        for i in 0..batch_size {
            let key = format!("{}_{}", broad_types[i], categories[i]);
            if self.tier2.contains_key(&key) {
                t2_groups.entry(key).or_default().push(i);
            } else if let Some(direct) = self.tier2_direct.get(&key) {
                final_labels[i] = direct.clone();
                t2_confs[i] = 1.0;
            } else {
                final_labels[i] = format!("{}.{}", broad_types[i], categories[i]);
                t2_confs[i] = 0.0;
            }
        }

        for (key, indices) in &t2_groups {
            let model = &self.tier2[key];
            let group_input = self.gather_rows(&input_ids, indices)?;
            let results = self.run_model(model, &group_input)?;
            for (j, idx) in indices.iter().enumerate() {
                final_labels[*idx] = results[j].0.clone();
                t2_confs[*idx] = results[j].1;
            }
        }

        // --- Assemble final results ---
        let mut final_results = Vec::with_capacity(batch_size);
        for i in 0..batch_size {
            let combined_confidence = t0_confs[i] * t1_confs[i] * t2_confs[i];
            final_results.push(ClassificationResult {
                label: final_labels[i].clone(),
                confidence: combined_confidence,
                all_scores: vec![],
            });
        }

        Ok(final_results)
    }

    /// Classify a batch of texts with per-tier timing breakdown.
    ///
    /// Same logic as `classify_batch` but returns timing alongside results.
    pub fn classify_batch_timed(
        &self,
        texts: &[String],
    ) -> Result<(Vec<ClassificationResult>, TierTiming), InferenceError> {
        use std::time::Instant;

        let total_start = Instant::now();
        let batch_size = texts.len();

        // Encode
        let t = Instant::now();
        let input_ids = self.encode_batch(texts)?;
        let encode_ms = t.elapsed().as_secs_f64() * 1000.0;

        // Tier 0
        let t = Instant::now();
        let tier0_results = self.run_model(&self.tier0, &input_ids)?;
        let tier0_ms = t.elapsed().as_secs_f64() * 1000.0;

        let mut broad_types: Vec<String> = Vec::with_capacity(batch_size);
        let mut t0_confs: Vec<f32> = Vec::with_capacity(batch_size);
        let mut categories: Vec<String> = vec![String::new(); batch_size];
        let mut t1_confs: Vec<f32> = vec![0.0; batch_size];
        let mut final_labels: Vec<String> = vec![String::new(); batch_size];
        let mut t2_confs: Vec<f32> = vec![0.0; batch_size];

        for (bt, conf) in &tier0_results {
            broad_types.push(bt.clone());
            t0_confs.push(*conf);
        }

        // Tier 1
        let t = Instant::now();
        let mut t1_groups: HashMap<String, Vec<usize>> = HashMap::new();
        for (i, bt) in broad_types.iter().enumerate() {
            if self.tier1.contains_key(bt) {
                t1_groups.entry(bt.clone()).or_default().push(i);
            } else if let Some(direct) = self.tier1_direct.get(bt) {
                categories[i] = direct.clone();
                t1_confs[i] = 1.0;
            } else {
                categories[i] = "unknown".to_string();
                t1_confs[i] = 0.0;
            }
        }
        let tier1_models = t1_groups.len();
        for (bt, indices) in &t1_groups {
            let model = &self.tier1[bt];
            let group_input = self.gather_rows(&input_ids, indices)?;
            let results = self.run_model(model, &group_input)?;
            for (j, idx) in indices.iter().enumerate() {
                categories[*idx] = results[j].0.clone();
                t1_confs[*idx] = results[j].1;
            }
        }
        let tier1_ms = t.elapsed().as_secs_f64() * 1000.0;

        // Tier 2
        let t = Instant::now();
        let mut t2_groups: HashMap<String, Vec<usize>> = HashMap::new();
        for i in 0..batch_size {
            let key = format!("{}_{}", broad_types[i], categories[i]);
            if self.tier2.contains_key(&key) {
                t2_groups.entry(key).or_default().push(i);
            } else if let Some(direct) = self.tier2_direct.get(&key) {
                final_labels[i] = direct.clone();
                t2_confs[i] = 1.0;
            } else {
                final_labels[i] = format!("{}.{}", broad_types[i], categories[i]);
                t2_confs[i] = 0.0;
            }
        }
        let tier2_models = t2_groups.len();
        for (key, indices) in &t2_groups {
            let model = &self.tier2[key];
            let group_input = self.gather_rows(&input_ids, indices)?;
            let results = self.run_model(model, &group_input)?;
            for (j, idx) in indices.iter().enumerate() {
                final_labels[*idx] = results[j].0.clone();
                t2_confs[*idx] = results[j].1;
            }
        }
        let tier2_ms = t.elapsed().as_secs_f64() * 1000.0;

        // Assemble results
        let mut final_results = Vec::with_capacity(batch_size);
        for i in 0..batch_size {
            let combined_confidence = t0_confs[i] * t1_confs[i] * t2_confs[i];
            final_results.push(ClassificationResult {
                label: final_labels[i].clone(),
                confidence: combined_confidence,
                all_scores: vec![],
            });
        }

        let total_ms = total_start.elapsed().as_secs_f64() * 1000.0;
        let timing = TierTiming {
            encode_ms,
            tier0_ms,
            tier1_ms,
            tier1_models,
            tier2_ms,
            tier2_models,
            total_ms,
        };

        Ok((final_results, timing))
    }

    /// Gather specific rows from a 2D tensor by indices.
    fn gather_rows(&self, tensor: &Tensor, indices: &[usize]) -> Result<Tensor, InferenceError> {
        let index_tensor = Tensor::new(
            indices.iter().map(|&i| i as u32).collect::<Vec<u32>>(),
            tensor.device(),
        )?;
        Ok(tensor.index_select(&index_tensor, 0)?)
    }

    /// Encode a batch of texts to tensor.
    fn encode_batch(&self, texts: &[String]) -> Result<Tensor, InferenceError> {
        let batch_size = texts.len();
        let mut all_ids = Vec::with_capacity(batch_size * self.max_seq_length);
        for text in texts {
            let ids = self.vocab.encode(text, self.max_seq_length);
            all_ids.extend(ids);
        }
        let input_ids =
            Tensor::new(all_ids, &self.device)?.reshape((batch_size, self.max_seq_length))?;
        Ok(input_ids)
    }

    /// Run a model on input and return (label, confidence) pairs.
    fn run_model(
        &self,
        loaded: &LoadedModel,
        input_ids: &Tensor,
    ) -> Result<Vec<(String, f32)>, InferenceError> {
        let probs = loaded.model.infer(input_ids)?;
        let probs = probs.to_vec2::<f32>()?;

        let mut results = Vec::with_capacity(probs.len());
        for prob_row in probs {
            let (max_idx, max_prob) = prob_row
                .iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
                .unwrap();

            let label = loaded
                .index_to_label
                .get(&max_idx)
                .cloned()
                .unwrap_or_else(|| "unknown".to_string());

            results.push((label, *max_prob));
        }

        Ok(results)
    }

    /// Load a tiered classifier from embedded byte slices.
    ///
    /// This is used by the DuckDB extension where models are compiled into the binary
    /// via `include_bytes!`. The `get_data` function maps directory names to byte slices
    /// for (weights, labels_json, config_yaml).
    /// Type alias for the embedded model data lookup function.
    #[allow(clippy::type_complexity)]
    pub fn from_embedded(
        graph_json: &[u8],
        get_data: fn(&str) -> Option<(&'static [u8], &'static [u8], &'static [u8])>,
    ) -> Result<Self, InferenceError> {
        let device = Self::get_device();

        let graph_str = std::str::from_utf8(graph_json).map_err(|e| {
            InferenceError::InvalidPath(format!("Invalid UTF-8 in tier_graph.json: {}", e))
        })?;
        let graph_meta: TierGraphMeta = serde_json::from_str(graph_str).map_err(|e| {
            InferenceError::InvalidPath(format!("Failed to parse tier_graph.json: {}", e))
        })?;

        // Load Tier 0
        let tier0_dir = &graph_meta.tier0.dir;
        let (w, l, c) = get_data(tier0_dir).ok_or_else(|| {
            InferenceError::InvalidPath(format!("Missing embedded data for {}", tier0_dir))
        })?;
        let tier0 = Self::load_model_from_bytes(w, l, c, &device)?;

        // Load Tier 1 models
        let mut tier1 = HashMap::new();
        let mut tier1_direct = HashMap::new();

        for (broad_type, meta) in &graph_meta.tier1 {
            if let Some(dir) = &meta.dir {
                let (w, l, c) = get_data(dir).ok_or_else(|| {
                    InferenceError::InvalidPath(format!("Missing embedded data for {}", dir))
                })?;
                let model = Self::load_model_from_bytes(w, l, c, &device)?;
                tier1.insert(broad_type.clone(), model);
            } else if let Some(direct) = &meta.direct {
                tier1_direct.insert(broad_type.clone(), direct.clone());
            }
        }

        // Load Tier 2 models
        let mut tier2 = HashMap::new();
        let mut tier2_direct = HashMap::new();

        for (key, meta) in &graph_meta.tier2 {
            if let Some(dir) = &meta.dir {
                let (w, l, c) = get_data(dir).ok_or_else(|| {
                    InferenceError::InvalidPath(format!("Missing embedded data for {}", dir))
                })?;
                let model = Self::load_model_from_bytes(w, l, c, &device)?;
                tier2.insert(key.clone(), model);
            } else if let Some(direct) = &meta.direct {
                tier2_direct.insert(key.clone(), direct.clone());
            }
        }

        let vocab = CharVocab::new();
        let max_seq_length = 128;

        Ok(Self {
            tier0,
            tier1,
            tier1_direct,
            tier2,
            tier2_direct,
            vocab,
            device,
            max_seq_length,
        })
    }

    /// Load a CharCNN model from a directory.
    fn load_model(model_dir: &Path, device: &Device) -> Result<LoadedModel, InferenceError> {
        // Load labels
        let labels_path = model_dir.join("labels.json");
        let content = std::fs::read_to_string(&labels_path).map_err(|e| {
            InferenceError::InvalidPath(format!(
                "Failed to read labels.json in {:?}: {}",
                model_dir, e
            ))
        })?;
        let labels: Vec<String> = serde_json::from_str(&content).map_err(|e| {
            InferenceError::InvalidPath(format!("Failed to parse labels.json: {}", e))
        })?;

        // Load config
        let config_path = model_dir.join("config.yaml");
        let config_str = if config_path.exists() {
            std::fs::read_to_string(&config_path)?
        } else {
            String::new()
        };

        // Load weights
        let weights_path = model_dir.join("model.safetensors");
        let weights_bytes = std::fs::read(&weights_path).map_err(|e| {
            InferenceError::InvalidPath(format!(
                "Failed to read model.safetensors in {:?}: {}",
                model_dir, e
            ))
        })?;

        Self::build_model(&labels, &config_str, &weights_bytes, device)
    }

    /// Load a CharCNN model from embedded byte slices.
    fn load_model_from_bytes(
        weights: &[u8],
        labels_json: &[u8],
        config_yaml: &[u8],
        device: &Device,
    ) -> Result<LoadedModel, InferenceError> {
        let labels_str = std::str::from_utf8(labels_json).map_err(|e| {
            InferenceError::InvalidPath(format!("Invalid UTF-8 in labels.json: {}", e))
        })?;
        let labels: Vec<String> = serde_json::from_str(labels_str).map_err(|e| {
            InferenceError::InvalidPath(format!("Failed to parse labels.json: {}", e))
        })?;

        let config_str = std::str::from_utf8(config_yaml).unwrap_or("");

        Self::build_model(&labels, config_str, weights, device)
    }

    /// Build a LoadedModel from parsed labels, config string, and weight bytes.
    fn build_model(
        labels: &[String],
        config_str: &str,
        weights: &[u8],
        device: &Device,
    ) -> Result<LoadedModel, InferenceError> {
        let n_classes = labels.len();

        let index_to_label: HashMap<usize, String> = labels.iter().cloned().enumerate().collect();
        let label_to_index: HashMap<String, usize> = labels
            .iter()
            .cloned()
            .enumerate()
            .map(|(i, l)| (l, i))
            .collect();

        // Parse config
        let (vocab_size, max_seq_length, embed_dim, num_filters, hidden_dim) =
            parse_config_yaml(config_str);

        let config = CharCnnConfig {
            vocab_size,
            max_seq_length,
            embed_dim,
            num_filters,
            kernel_sizes: vec![2, 3, 4, 5],
            hidden_dim,
            n_classes,
            dropout: 0.0,
        };

        let vb = VarBuilder::from_buffered_safetensors(weights.to_vec(), DType::F32, device)?;
        let model = CharCnn::new(config, vb)?;

        Ok(LoadedModel {
            model,
            index_to_label,
            label_to_index,
        })
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

impl ValueClassifier for TieredClassifier {
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

/// Parse a config.yaml string into model hyperparameters.
fn parse_config_yaml(config_str: &str) -> (usize, usize, usize, usize, usize) {
    let mut vocab_size = 97usize;
    let mut max_seq_length = 128usize;
    let mut embed_dim = 32usize;
    let mut num_filters = 64usize;
    let mut hidden_dim = 128usize;

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
    )
}
