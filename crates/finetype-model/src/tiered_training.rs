//! Training utilities for tiered inference models.
//!
//! Trains a hierarchy of CharCNN models:
//! - Tier 0: Broad type classifier (e.g., VARCHAR, DATE, TIMESTAMP)
//! - Tier 1: Per-broad-type category models (e.g., VARCHAR → internet/person/code)
//! - Tier 2: Per-category type models for categories with many types

use crate::char_cnn::{CharCnn, CharCnnConfig, CharVocab};
use candle_core::{DType, Device, Tensor};
use candle_nn::{AdamW, Optimizer, ParamsAdamW, VarBuilder, VarMap};
use finetype_core::{Sample, Taxonomy, TierGraph};
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::SeedableRng;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TieredTrainingError {
    #[error("Model error: {0}")]
    ModelError(#[from] candle_core::Error),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("No samples for tier {tier} group {group}")]
    EmptyGroup { tier: String, group: String },
}

/// Configuration for tiered training.
#[derive(Debug, Clone)]
pub struct TieredTrainingConfig {
    pub batch_size: usize,
    pub epochs: usize,
    pub learning_rate: f64,
    pub max_seq_length: usize,
    pub embed_dim: usize,
    pub num_filters: usize,
    pub hidden_dim: usize,
    pub weight_decay: f64,
    /// Minimum number of types in a (broad_type, category) group to train a Tier 2 model.
    /// Groups with fewer types are resolved by Tier 1 directly.
    pub tier2_min_types: usize,
    /// Optional seed for deterministic training. When set, uses a seeded RNG
    /// instead of `thread_rng()` for reproducible shuffle order.
    pub seed: Option<u64>,
}

impl Default for TieredTrainingConfig {
    fn default() -> Self {
        Self {
            batch_size: 64,
            epochs: 10,
            learning_rate: 1e-3,
            max_seq_length: 128,
            embed_dim: 32,
            num_filters: 64,
            hidden_dim: 128,
            weight_decay: 1e-4,
            tier2_min_types: 1,
            seed: None,
        }
    }
}

/// Trainer for the full tiered model graph.
pub struct TieredTrainer {
    config: TieredTrainingConfig,
    device: Device,
    vocab: CharVocab,
}

impl TieredTrainer {
    /// Create a new tiered trainer.
    pub fn new(config: TieredTrainingConfig) -> Self {
        let device = Self::get_device();
        let vocab = CharVocab::new();
        Self {
            config,
            device,
            vocab,
        }
    }

    /// Train all tier models from a single dataset.
    ///
    /// Saves models to `output_dir` with the following structure:
    /// ```text
    /// output_dir/
    ///   tier0/                       # Broad type classifier
    ///   tier1_{broad_type}/          # Category classifier per broad type
    ///   tier2_{broad_type}_{category}/  # Type classifier per category
    ///   tier_graph.json              # Graph metadata
    /// ```
    pub fn train_all(
        &self,
        taxonomy: &Taxonomy,
        samples: &[Sample],
        output_dir: &Path,
    ) -> Result<TieredTrainingReport, TieredTrainingError> {
        let graph = taxonomy.tier_graph();
        let mut report = TieredTrainingReport::default();

        // Helper: resolve any label (3-level or 4-level) to its 3-level taxonomy key.
        // For 4-level labels (domain.category.type.LOCALE), strips the locale suffix.
        // Returns None if the label can't be resolved to a known taxonomy key.
        let resolve_3level = |label: &str| -> Option<String> {
            if graph.tier_path(label).is_some() {
                Some(label.to_string())
            } else if let Some((prefix, _)) = label.rsplit_once('.') {
                if graph.tier_path(prefix).is_some() {
                    Some(prefix.to_string())
                } else {
                    None
                }
            } else {
                None
            }
        };

        // Normalize sample labels to 3-level for T0 and T1 training.
        // T0 and T1 don't need locale info — they route by broad_type and category.
        let samples_3level: Vec<Sample> = samples
            .iter()
            .filter_map(|s| {
                resolve_3level(&s.label).map(|label| Sample {
                    text: s.text.clone(),
                    label,
                })
            })
            .collect();

        // Detect whether samples contain 4-level (localized) labels
        let has_4level = samples
            .iter()
            .any(|s| resolve_3level(&s.label).as_deref() != Some(&s.label));
        if has_4level {
            let n_4level = samples
                .iter()
                .filter(|s| resolve_3level(&s.label).as_deref() != Some(&s.label))
                .count();
            eprintln!(
                "Localized training: {} of {} samples have 4-level locale labels",
                n_4level,
                samples.len()
            );
        }

        eprintln!("=== Tiered Training ===");
        eprintln!("{}", graph.summary());

        // --- Tier 0: Broad type classification (uses 3-level labels) ---
        eprintln!("\n--- Training Tier 0 (broad type) ---");
        let tier0_dir = output_dir.join("tier0");
        let tier0_accuracy = self.train_tier0(&graph, &samples_3level, &tier0_dir)?;
        report.tier0_accuracy = tier0_accuracy;
        report.tier0_classes = graph.num_broad_types();
        eprintln!(
            "Tier 0: {:.2}% accuracy ({} classes)",
            tier0_accuracy * 100.0,
            graph.num_broad_types()
        );

        // --- Tier 1: Per-broad-type category models (uses 3-level labels) ---
        for broad_type in graph.broad_types() {
            let categories = graph.categories_for(broad_type);
            if categories.len() <= 1 {
                // Only one category — no Tier 1 model needed, resolved by lookup
                eprintln!(
                    "\n--- Tier 1 [{}]: skipped (single category: {}) ---",
                    broad_type,
                    categories.first().map(|s| s.as_str()).unwrap_or("none")
                );
                report.tier1_skipped.push(broad_type.clone());
                continue;
            }

            eprintln!(
                "\n--- Training Tier 1 [{}] ({} categories) ---",
                broad_type,
                categories.len()
            );
            let tier1_dir = output_dir.join(format!("tier1_{}", broad_type));
            match self.train_tier1(&graph, broad_type, &samples_3level, &tier1_dir) {
                Ok(tier1_accuracy) => {
                    report.tier1_results.push(TierModelResult {
                        name: broad_type.clone(),
                        classes: categories.len(),
                        accuracy: tier1_accuracy,
                    });
                    eprintln!(
                        "Tier 1 [{}]: {:.2}% accuracy ({} categories)",
                        broad_type,
                        tier1_accuracy * 100.0,
                        categories.len()
                    );
                }
                Err(TieredTrainingError::EmptyGroup { .. }) => {
                    eprintln!("Tier 1 [{}]: skipped (no training samples)", broad_type);
                    report.tier1_skipped.push(broad_type.clone());
                }
                Err(e) => return Err(e),
            }
        }

        // --- Tier 2: Per-category type models ---
        // Uses ORIGINAL samples to preserve 4-level locale labels at the leaf tier.
        // Each sample's label is resolved to 3-level for tier path routing, but the
        // original label (possibly 4-level) is kept as the T2 class name.
        let mut t2_label_map: HashMap<String, Vec<String>> = HashMap::new();
        for broad_type in graph.broad_types() {
            for category in graph.categories_for(broad_type) {
                let key = format!("{}_{}", broad_type, category);

                // Collect samples for this T2 group, keeping original (possibly 4-level) labels.
                // Route via 3-level prefix: resolve_3level → tier_path → (broad_type, category).
                let filtered: Vec<Sample> = samples
                    .iter()
                    .filter(|s| {
                        resolve_3level(&s.label)
                            .and_then(|r| graph.tier_path(&r))
                            .map(|(bt, cat)| bt == broad_type && cat == category)
                            .unwrap_or(false)
                    })
                    .cloned()
                    .collect();

                if filtered.is_empty() {
                    continue;
                }

                // Build label set from unique labels in this group (may be 4-level)
                let mut labels: Vec<String> = filtered
                    .iter()
                    .map(|s| s.label.clone())
                    .collect::<HashSet<_>>()
                    .into_iter()
                    .collect();
                labels.sort();

                t2_label_map.insert(key.clone(), labels.clone());

                if labels.len() <= self.config.tier2_min_types {
                    // Too few labels — resolved directly by Tier 1
                    continue;
                }

                eprintln!(
                    "\n--- Training Tier 2 [{}/{}] ({} labels) ---",
                    broad_type,
                    category,
                    labels.len()
                );
                let tier2_dir = output_dir.join(format!("tier2_{}_{}", broad_type, category));
                match self.train_model(&labels, &filtered, &tier2_dir) {
                    Ok(tier2_accuracy) => {
                        report.tier2_results.push(TierModelResult {
                            name: format!("{}/{}", broad_type, category),
                            classes: labels.len(),
                            accuracy: tier2_accuracy,
                        });
                        eprintln!(
                            "Tier 2 [{}/{}]: {:.2}% accuracy ({} labels)",
                            broad_type,
                            category,
                            tier2_accuracy * 100.0,
                            labels.len()
                        );
                    }
                    Err(TieredTrainingError::EmptyGroup { .. }) => {
                        eprintln!(
                            "Tier 2 [{}/{}]: skipped (no training samples)",
                            broad_type, category
                        );
                        report
                            .tier2_skipped
                            .push(format!("{}/{}", broad_type, category));
                    }
                    Err(e) => return Err(e),
                }
            }
        }

        // Save tier graph metadata with 4-level labels at T2
        let graph_meta = self.build_graph_metadata(&graph, &report, output_dir, &t2_label_map);
        let graph_json =
            serde_json::to_string_pretty(&graph_meta).unwrap_or_else(|_| "{}".to_string());
        std::fs::create_dir_all(output_dir)?;
        std::fs::write(output_dir.join("tier_graph.json"), graph_json)?;

        eprintln!("\n=== Tiered Training Complete ===");
        eprintln!("{}", report);

        Ok(report)
    }

    /// Train Tier 0: broad type classification.
    fn train_tier0(
        &self,
        graph: &TierGraph,
        samples: &[Sample],
        output_dir: &Path,
    ) -> Result<f64, TieredTrainingError> {
        // Relabel samples: full label → broad_type
        let relabeled: Vec<Sample> = samples
            .iter()
            .filter_map(|s| {
                graph.broad_type_for(&s.label).map(|bt| Sample {
                    text: s.text.clone(),
                    label: bt.to_string(),
                })
            })
            .collect();

        if relabeled.is_empty() {
            return Err(TieredTrainingError::EmptyGroup {
                tier: "0".into(),
                group: "all".into(),
            });
        }

        // Build label set
        let mut labels: Vec<String> = graph.broad_types().to_vec();
        labels.sort();

        self.train_model(&labels, &relabeled, output_dir)
    }

    /// Train Tier 1: category classification within a broad type.
    fn train_tier1(
        &self,
        graph: &TierGraph,
        broad_type: &str,
        samples: &[Sample],
        output_dir: &Path,
    ) -> Result<f64, TieredTrainingError> {
        // Filter samples for this broad type, relabel to category
        let relabeled: Vec<Sample> = samples
            .iter()
            .filter_map(|s| {
                let (bt, cat) = graph.tier_path(&s.label)?;
                if bt == broad_type {
                    Some(Sample {
                        text: s.text.clone(),
                        label: cat.clone(),
                    })
                } else {
                    None
                }
            })
            .collect();

        if relabeled.is_empty() {
            return Err(TieredTrainingError::EmptyGroup {
                tier: "1".into(),
                group: broad_type.into(),
            });
        }

        let mut labels: Vec<String> = graph.categories_for(broad_type).to_vec();
        labels.sort();

        self.train_model(&labels, &relabeled, output_dir)
    }

    // Note: Tier 2 training is now done inline in train_all() to support
    // 4-level locale labels. The original train_tier2() was removed because
    // T2 needs access to the original (possibly 4-level) samples, not the
    // 3-level normalized versions used by T0/T1.

    /// Train a single CharCNN model with the given labels and samples.
    /// Returns the final epoch accuracy.
    fn train_model(
        &self,
        labels: &[String],
        samples: &[Sample],
        output_dir: &Path,
    ) -> Result<f64, TieredTrainingError> {
        let n_classes = labels.len();
        let label_to_index: HashMap<String, usize> = labels
            .iter()
            .enumerate()
            .map(|(i, l)| (l.clone(), i))
            .collect();

        // Shuffle samples
        let mut samples_vec: Vec<&Sample> = samples.iter().collect();
        let mut rng: Box<dyn rand::RngCore> = match self.config.seed {
            Some(seed) => Box::new(StdRng::seed_from_u64(seed)),
            None => Box::new(rand::thread_rng()),
        };
        samples_vec.shuffle(&mut *rng);

        // Initialize model
        let varmap = VarMap::new();
        let vb = VarBuilder::from_varmap(&varmap, DType::F32, &self.device);

        let model_config = CharCnnConfig {
            vocab_size: self.vocab.vocab_size(),
            max_seq_length: self.config.max_seq_length,
            embed_dim: self.config.embed_dim,
            num_filters: self.config.num_filters,
            hidden_dim: self.config.hidden_dim,
            n_classes,
            ..Default::default()
        };

        let model = CharCnn::new(model_config, vb)?;

        // Create optimizer
        let params = ParamsAdamW {
            lr: self.config.learning_rate,
            weight_decay: self.config.weight_decay,
            ..Default::default()
        };
        let mut optimizer = AdamW::new(varmap.all_vars(), params)?;

        // Training loop
        let num_batches = samples_vec.len().div_ceil(self.config.batch_size);
        let mut final_accuracy = 0.0;

        for epoch in 0..self.config.epochs {
            if epoch > 0 {
                samples_vec.shuffle(&mut *rng);
            }

            let mut total_loss = 0.0;
            let mut num_correct = 0usize;
            let mut num_total = 0usize;

            for batch_idx in 0..num_batches {
                let start = batch_idx * self.config.batch_size;
                let end = (start + self.config.batch_size).min(samples_vec.len());
                let batch: Vec<&Sample> = samples_vec[start..end].to_vec();

                let (input_ids, batch_labels) = self.prepare_batch(&batch, &label_to_index)?;

                let logits = model.forward(&input_ids)?;
                let logits = logits.contiguous()?;
                let loss = candle_nn::loss::cross_entropy(&logits, &batch_labels)?;
                optimizer.backward_step(&loss)?;

                let loss_val = loss.to_scalar::<f32>()?;
                total_loss += loss_val;

                let predictions = logits.argmax(1)?;
                let correct = predictions
                    .eq(&batch_labels)?
                    .to_dtype(DType::F32)?
                    .sum_all()?
                    .to_scalar::<f32>()?;
                num_correct += correct as usize;
                num_total += batch.len();
            }

            let avg_loss = total_loss / num_batches as f32;
            final_accuracy = num_correct as f64 / num_total as f64;

            eprintln!(
                "  Epoch {}/{}: loss={:.4}, accuracy={:.2}%",
                epoch + 1,
                self.config.epochs,
                avg_loss,
                final_accuracy * 100.0
            );
        }

        // Save model
        std::fs::create_dir_all(output_dir)?;
        varmap.save(output_dir.join("model.safetensors"))?;

        // Save config
        let config_str = format!(
            "vocab_size: {}\nmax_seq_length: {}\nembed_dim: {}\nnum_filters: {}\nhidden_dim: {}\nn_classes: {}\nmodel_type: char_cnn\n",
            self.vocab.vocab_size(),
            self.config.max_seq_length,
            self.config.embed_dim,
            self.config.num_filters,
            self.config.hidden_dim,
            n_classes
        );
        std::fs::write(output_dir.join("config.yaml"), config_str)?;

        // Save label mapping
        let labels_json = serde_json::to_string_pretty(labels).unwrap_or_else(|_| "[]".to_string());
        std::fs::write(output_dir.join("labels.json"), labels_json)?;

        Ok(final_accuracy)
    }

    /// Prepare a batch for training.
    fn prepare_batch(
        &self,
        samples: &[&Sample],
        label_to_index: &HashMap<String, usize>,
    ) -> Result<(Tensor, Tensor), TieredTrainingError> {
        let batch_size = samples.len();
        let max_len = self.config.max_seq_length;

        let mut all_ids = Vec::with_capacity(batch_size * max_len);
        let mut all_labels = Vec::with_capacity(batch_size);

        for sample in samples {
            let ids = self.vocab.encode(&sample.text, max_len);
            all_ids.extend(ids);
            // Try exact match first, then strip locale/UNIVERSAL suffix from generated labels
            let label_idx = label_to_index
                .get(&sample.label)
                .or_else(|| {
                    sample
                        .label
                        .rsplit_once('.')
                        .and_then(|(prefix, _)| label_to_index.get(prefix))
                })
                .copied()
                .unwrap_or(0) as u32;
            all_labels.push(label_idx);
        }

        let input_ids = Tensor::new(all_ids, &self.device)?.reshape((batch_size, max_len))?;
        let labels = Tensor::new(all_labels, &self.device)?;

        Ok((input_ids, labels))
    }

    /// Build graph metadata JSON for the inference engine.
    ///
    /// Only references models that were actually trained (have directories on disk).
    /// Skipped groups use "direct" resolution to first type.
    ///
    /// `t2_label_map` provides the actual labels used at T2 (may be 4-level locale labels).
    /// If empty, falls back to the 3-level taxonomy labels.
    fn build_graph_metadata(
        &self,
        graph: &TierGraph,
        _report: &TieredTrainingReport,
        output_dir: &Path,
        t2_label_map: &HashMap<String, Vec<String>>,
    ) -> serde_json::Value {
        use serde_json::json;

        let mut tier1_models = serde_json::Map::new();
        for broad_type in graph.broad_types() {
            let categories = graph.categories_for(broad_type);
            let tier1_dir = format!("tier1_{}", broad_type);
            if categories.len() > 1 && output_dir.join(&tier1_dir).exists() {
                tier1_models.insert(
                    broad_type.clone(),
                    json!({
                        "dir": tier1_dir,
                        "categories": categories,
                    }),
                );
            } else {
                tier1_models.insert(
                    broad_type.clone(),
                    json!({
                        "direct": categories.first(),
                    }),
                );
            }
        }

        let mut tier2_models = serde_json::Map::new();
        for broad_type in graph.broad_types() {
            for category in graph.categories_for(broad_type) {
                let key = format!("{}_{}", broad_type, category);
                let tier2_dir = format!("tier2_{}_{}", broad_type, category);

                // Use actual labels from training (may be 4-level) or fall back to 3-level
                let labels = t2_label_map
                    .get(&key)
                    .cloned()
                    .unwrap_or_else(|| graph.types_for(broad_type, category).to_vec());
                let n_labels = labels.len();

                if n_labels > self.config.tier2_min_types && output_dir.join(&tier2_dir).exists() {
                    // Model was trained — reference its directory
                    tier2_models.insert(
                        key,
                        json!({
                            "dir": tier2_dir,
                            "types": labels,
                            "count": n_labels,
                        }),
                    );
                } else {
                    // Single label or skipped training — resolve directly to first label
                    tier2_models.insert(
                        key,
                        json!({
                            "direct": labels.first(),
                            "count": n_labels,
                        }),
                    );
                }
            }
        }

        json!({
            "version": 1,
            "tier0": {
                "dir": "tier0",
                "broad_types": graph.broad_types(),
            },
            "tier1": tier1_models,
            "tier2": tier2_models,
            "tier2_min_types": self.config.tier2_min_types,
        })
    }

    /// Get the best available device.
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

/// Result for a single tier model training.
#[derive(Debug, Clone)]
pub struct TierModelResult {
    pub name: String,
    pub classes: usize,
    pub accuracy: f64,
}

/// Report from tiered training.
#[derive(Debug, Clone, Default)]
pub struct TieredTrainingReport {
    pub tier0_accuracy: f64,
    pub tier0_classes: usize,
    pub tier1_results: Vec<TierModelResult>,
    pub tier1_skipped: Vec<String>,
    pub tier2_results: Vec<TierModelResult>,
    pub tier2_skipped: Vec<String>,
}

impl std::fmt::Display for TieredTrainingReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Tiered Training Report:")?;
        writeln!(
            f,
            "  Tier 0: {:.2}% accuracy ({} classes)",
            self.tier0_accuracy * 100.0,
            self.tier0_classes
        )?;
        writeln!(
            f,
            "  Tier 1: {} models trained, {} skipped (single category)",
            self.tier1_results.len(),
            self.tier1_skipped.len()
        )?;
        for r in &self.tier1_results {
            writeln!(
                f,
                "    [{}]: {:.2}% ({} categories)",
                r.name,
                r.accuracy * 100.0,
                r.classes
            )?;
        }
        writeln!(
            f,
            "  Tier 2: {} models trained, {} skipped (no samples)",
            self.tier2_results.len(),
            self.tier2_skipped.len()
        )?;
        for r in &self.tier2_results {
            writeln!(
                f,
                "    [{}]: {:.2}% ({} types)",
                r.name,
                r.accuracy * 100.0,
                r.classes
            )?;
        }
        Ok(())
    }
}
