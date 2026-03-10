//! Training utilities for character-level CNN classifier.

use crate::char_cnn::{CharCnn, CharCnnConfig, CharVocab, HeadType, HierarchyMap};
use crate::features::{extract_features, FEATURE_DIM};
use candle_core::{DType, Device, Tensor};
use candle_nn::{AdamW, Optimizer, ParamsAdamW, VarBuilder, VarMap};
use finetype_core::{Sample, Taxonomy};
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::SeedableRng;
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CharTrainingError {
    #[error("Model error: {0}")]
    ModelError(#[from] candle_core::Error),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Training configuration for CharCNN.
#[derive(Debug, Clone)]
pub struct CharTrainingConfig {
    pub batch_size: usize,
    pub epochs: usize,
    pub learning_rate: f64,
    pub max_seq_length: usize,
    pub embed_dim: usize,
    pub num_filters: usize,
    pub hidden_dim: usize,
    pub weight_decay: f64,
    pub shuffle: bool,
    /// Optional seed for deterministic training. When set, uses a seeded RNG
    /// instead of `thread_rng()` for reproducible shuffle order.
    pub seed: Option<u64>,
    /// Enable feature-augmented training (NNFT-249). When true, deterministic
    /// features are extracted per sample and passed to the model alongside
    /// character encodings. The model's `feature_dim` is set to `FEATURE_DIM`.
    pub use_features: bool,
    /// Enable hierarchical classification head (NNFT-267). When true, the model
    /// uses a tree softmax (domain → category → leaf type) instead of a flat
    /// 250-class softmax. Training uses multi-level cross-entropy loss.
    pub use_hierarchical: bool,
}

impl Default for CharTrainingConfig {
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
            shuffle: true,
            seed: None,
            use_features: false,
            use_hierarchical: false,
        }
    }
}

/// Trainer for character-level CNN.
pub struct CharTrainer {
    config: CharTrainingConfig,
    device: Device,
    vocab: CharVocab,
}

impl CharTrainer {
    /// Create a new trainer.
    pub fn new(config: CharTrainingConfig) -> Self {
        let device = Self::get_device();
        let vocab = CharVocab::new();
        Self {
            config,
            device,
            vocab,
        }
    }

    /// Train the model.
    pub fn train(
        &self,
        taxonomy: &Taxonomy,
        samples: &[Sample],
        output_dir: &Path,
    ) -> Result<(), CharTrainingError> {
        eprintln!("Starting CharCNN training with {} samples", samples.len());
        eprintln!("Device: {:?}", self.device);

        // Create label mapping
        let label_to_index = taxonomy.label_to_index();
        let labels_list: Vec<String> = taxonomy.labels().to_vec();
        let n_classes = taxonomy.len();
        eprintln!("Number of classes: {}", n_classes);

        // Build hierarchy map if hierarchical mode (NNFT-267)
        let hierarchy = if self.config.use_hierarchical {
            let h = HierarchyMap::from_labels(&labels_list);
            eprintln!(
                "Hierarchical mode: {} domains, {} categories, {} leaf types ({} degenerate categories)",
                h.num_domains(),
                h.total_categories(),
                n_classes,
                (0..h.num_domains())
                    .flat_map(|d| (0..h.num_categories(d)).map(move |c| (d, c)))
                    .filter(|&(d, c)| h.is_degenerate(d, c))
                    .count()
            );
            Some(h)
        } else {
            None
        };

        // Shuffle samples if configured
        let mut samples_vec: Vec<&Sample> = samples.iter().collect();
        let mut rng: Box<dyn rand::RngCore> = match self.config.seed {
            Some(seed) => {
                eprintln!("Using deterministic seed: {}", seed);
                Box::new(StdRng::seed_from_u64(seed))
            }
            None => Box::new(rand::thread_rng()),
        };
        if self.config.shuffle {
            samples_vec.shuffle(&mut *rng);
            eprintln!("Shuffled training data");
        }

        // Initialize model
        eprintln!("Initializing CharCNN model...");
        let varmap = VarMap::new();
        let vb = VarBuilder::from_varmap(&varmap, DType::F32, &self.device);

        let feature_dim = if self.config.use_features {
            FEATURE_DIM
        } else {
            0
        };

        let head_type = if self.config.use_hierarchical {
            HeadType::Hierarchical
        } else {
            HeadType::Flat
        };

        let model_config = CharCnnConfig {
            vocab_size: self.vocab.vocab_size(),
            max_seq_length: self.config.max_seq_length,
            embed_dim: self.config.embed_dim,
            num_filters: self.config.num_filters,
            hidden_dim: self.config.hidden_dim,
            n_classes,
            feature_dim,
            head_type,
            ..Default::default()
        };

        let model = if self.config.use_hierarchical {
            CharCnn::new_hierarchical(model_config, &labels_list, vb)?
        } else {
            CharCnn::new(model_config, vb)?
        };
        eprintln!(
            "Model initialized (vocab_size={}, embed_dim={}, filters={}, feature_dim={}, head={})",
            self.vocab.vocab_size(),
            self.config.embed_dim,
            self.config.num_filters,
            feature_dim,
            if self.config.use_hierarchical {
                "hierarchical"
            } else {
                "flat"
            }
        );

        // Create optimizer
        let params = ParamsAdamW {
            lr: self.config.learning_rate,
            weight_decay: self.config.weight_decay,
            ..Default::default()
        };
        let mut optimizer = AdamW::new(varmap.all_vars(), params)?;

        // Training loop
        let num_batches = samples_vec.len().div_ceil(self.config.batch_size);
        eprintln!("Training: {} batches per epoch", num_batches);

        for epoch in 0..self.config.epochs {
            // Re-shuffle each epoch
            if self.config.shuffle && epoch > 0 {
                samples_vec.shuffle(&mut *rng);
            }

            eprintln!("Starting epoch {}/{}", epoch + 1, self.config.epochs);
            let mut total_loss = 0.0;
            let mut num_correct = 0usize;
            let mut num_total = 0usize;
            // Per-level accuracy tracking for hierarchical mode (NNFT-267)
            let mut domain_correct = 0usize;
            let mut cat_correct = 0usize;

            for batch_idx in 0..num_batches {
                let start = batch_idx * self.config.batch_size;
                let end = (start + self.config.batch_size).min(samples_vec.len());
                let batch: Vec<&Sample> = samples_vec[start..end].to_vec();

                // Prepare batch (includes features when use_features=true)
                let (input_ids, features, labels) =
                    self.prepare_batch(&batch, &label_to_index)?;

                let loss = if self.config.use_hierarchical {
                    // Hierarchical training: multi-level cross-entropy (NNFT-267)
                    let hier = hierarchy.as_ref().unwrap();
                    let hier_head = model.hierarchical_head().unwrap();

                    // Get backbone hidden representation
                    let hidden = model.backbone_forward(&input_ids, features.as_ref())?;

                    // Get per-level logits
                    let (domain_logits, cat_logits_all, leaf_logits_all) =
                        hier_head.forward_levels(&hidden)?;

                    // Prepare per-level targets from flat labels
                    let flat_labels: Vec<u32> = labels.to_vec1()?;
                    let batch_len = flat_labels.len();

                    let mut domain_targets = Vec::with_capacity(batch_len);
                    let mut cat_targets_by_domain: Vec<Vec<u32>> =
                        vec![Vec::new(); hier.num_domains()];
                    let mut cat_sample_indices_by_domain: Vec<Vec<usize>> =
                        vec![Vec::new(); hier.num_domains()];
                    let mut leaf_targets_by_cat: Vec<Vec<Vec<u32>>> = Vec::new();
                    let mut leaf_sample_indices_by_cat: Vec<Vec<Vec<usize>>> = Vec::new();

                    for d in 0..hier.num_domains() {
                        leaf_targets_by_cat
                            .push(vec![Vec::new(); hier.num_categories(d)]);
                        leaf_sample_indices_by_cat
                            .push(vec![Vec::new(); hier.num_categories(d)]);
                    }

                    for (i, &flat_idx) in flat_labels.iter().enumerate() {
                        let (d, c, t) = hier.flat_to_hier(flat_idx as usize);
                        domain_targets.push(d as u32);
                        cat_targets_by_domain[d].push(c as u32);
                        cat_sample_indices_by_domain[d].push(i);
                        leaf_targets_by_cat[d][c].push(t as u32);
                        leaf_sample_indices_by_cat[d][c].push(i);
                    }

                    // Domain loss (all samples)
                    let domain_target_tensor =
                        Tensor::new(domain_targets.clone(), &self.device)?;
                    let domain_loss = candle_nn::loss::cross_entropy(
                        &domain_logits,
                        &domain_target_tensor,
                    )?;

                    // Track domain accuracy
                    let domain_preds = domain_logits.argmax(1)?;
                    let d_correct = domain_preds
                        .eq(&domain_target_tensor)?
                        .to_dtype(DType::F32)?
                        .sum_all()?
                        .to_scalar::<f32>()?;
                    domain_correct += d_correct as usize;

                    // Category loss (grouped by GT domain)
                    let mut cat_loss_sum = Tensor::new(0.0f32, &self.device)?;
                    let mut cat_count = 0usize;

                    for d in 0..hier.num_domains() {
                        if cat_targets_by_domain[d].is_empty() {
                            continue;
                        }
                        let indices: Vec<u32> = cat_sample_indices_by_domain[d]
                            .iter()
                            .map(|&i| i as u32)
                            .collect();
                        let idx_tensor = Tensor::new(indices, &self.device)?;
                        let cat_logits_subset =
                            cat_logits_all[d].index_select(&idx_tensor, 0)?;
                        let cat_target_tensor =
                            Tensor::new(cat_targets_by_domain[d].clone(), &self.device)?;
                        let cl = candle_nn::loss::cross_entropy(
                            &cat_logits_subset,
                            &cat_target_tensor,
                        )?;
                        let n = cat_targets_by_domain[d].len() as f32;
                        cat_loss_sum =
                            (cat_loss_sum + cl.broadcast_mul(&Tensor::new(n, &self.device)?))?;
                        cat_count += cat_targets_by_domain[d].len();

                        // Track category accuracy
                        let cat_preds = cat_logits_subset.argmax(1)?;
                        let c_correct = cat_preds
                            .eq(&cat_target_tensor)?
                            .to_dtype(DType::F32)?
                            .sum_all()?
                            .to_scalar::<f32>()?;
                        cat_correct += c_correct as usize;
                    }

                    let cat_loss = if cat_count > 0 {
                        cat_loss_sum.broadcast_div(
                            &Tensor::new(cat_count as f32, &self.device)?,
                        )?
                    } else {
                        Tensor::new(0.0f32, &self.device)?
                    };

                    // Leaf loss (grouped by GT domain+category, skip degenerate)
                    let mut leaf_loss_sum = Tensor::new(0.0f32, &self.device)?;
                    let mut leaf_count = 0usize;

                    for d in 0..hier.num_domains() {
                        for c in 0..hier.num_categories(d) {
                            if hier.is_degenerate(d, c)
                                || leaf_targets_by_cat[d][c].is_empty()
                            {
                                continue;
                            }
                            let leaf_logits_opt = &leaf_logits_all[d][c];
                            if let Some(ref leaf_logits) = leaf_logits_opt {
                                let indices: Vec<u32> = leaf_sample_indices_by_cat[d][c]
                                    .iter()
                                    .map(|&i| i as u32)
                                    .collect();
                                let idx_tensor = Tensor::new(indices, &self.device)?;
                                let leaf_logits_subset =
                                    leaf_logits.index_select(&idx_tensor, 0)?;
                                let leaf_target_tensor = Tensor::new(
                                    leaf_targets_by_cat[d][c].clone(),
                                    &self.device,
                                )?;
                                let ll = candle_nn::loss::cross_entropy(
                                    &leaf_logits_subset,
                                    &leaf_target_tensor,
                                )?;
                                let n = leaf_targets_by_cat[d][c].len() as f32;
                                leaf_loss_sum = (leaf_loss_sum
                                    + ll.broadcast_mul(
                                        &Tensor::new(n, &self.device)?,
                                    ))?;
                                leaf_count += leaf_targets_by_cat[d][c].len();
                            }
                        }
                    }

                    let leaf_loss = if leaf_count > 0 {
                        leaf_loss_sum.broadcast_div(
                            &Tensor::new(leaf_count as f32, &self.device)?,
                        )?
                    } else {
                        Tensor::new(0.0f32, &self.device)?
                    };

                    // Weighted combination: λ = (0.2, 0.3, 0.5)
                    let total = (domain_loss
                        .broadcast_mul(&Tensor::new(0.2f32, &self.device)?)?
                        + cat_loss
                            .broadcast_mul(&Tensor::new(0.3f32, &self.device)?)?
                        + leaf_loss
                            .broadcast_mul(&Tensor::new(0.5f32, &self.device)?)?)?;

                    // Track flat accuracy via product probabilities
                    let probs =
                        model.forward_with_features(&input_ids, features.as_ref())?;
                    let predictions = probs.argmax(1)?;
                    let correct = predictions
                        .eq(&labels)?
                        .to_dtype(DType::F32)?
                        .sum_all()?
                        .to_scalar::<f32>()?;
                    num_correct += correct as usize;
                    num_total += batch.len();

                    total
                } else {
                    // Flat training: standard cross-entropy (existing path)
                    let logits =
                        model.forward_with_features(&input_ids, features.as_ref())?;
                    let logits = logits.contiguous()?;
                    let loss = candle_nn::loss::cross_entropy(&logits, &labels)?;

                    // Compute accuracy
                    let predictions = logits.argmax(1)?;
                    let correct = predictions
                        .eq(&labels)?
                        .to_dtype(DType::F32)?
                        .sum_all()?
                        .to_scalar::<f32>()?;
                    num_correct += correct as usize;
                    num_total += batch.len();

                    loss
                };

                // Backward pass
                optimizer.backward_step(&loss)?;

                // Track metrics
                let loss_val = loss.to_scalar::<f32>()?;
                total_loss += loss_val;

                // Print progress
                if (batch_idx + 1) % 10 == 0 || batch_idx == num_batches - 1 {
                    eprint!(
                        "\r  Batch {}/{}, loss={:.4}        ",
                        batch_idx + 1,
                        num_batches,
                        loss_val
                    );
                }
            }
            eprintln!();

            let avg_loss = total_loss / num_batches as f32;
            let accuracy = num_correct as f32 / num_total as f32;

            if self.config.use_hierarchical {
                let domain_acc = domain_correct as f32 / num_total as f32;
                let cat_acc = cat_correct as f32 / num_total as f32;
                eprintln!(
                    "Epoch {}/{}: loss={:.4}, type_acc={:.2}%, domain_acc={:.2}%, cat_acc={:.2}%",
                    epoch + 1,
                    self.config.epochs,
                    avg_loss,
                    accuracy * 100.0,
                    domain_acc * 100.0,
                    cat_acc * 100.0
                );
            } else {
                eprintln!(
                    "Epoch {}/{}: loss={:.4}, accuracy={:.2}%",
                    epoch + 1,
                    self.config.epochs,
                    avg_loss,
                    accuracy * 100.0
                );
            }
        }

        // Save model
        eprintln!("Saving model to {:?}", output_dir);
        std::fs::create_dir_all(output_dir)?;
        varmap.save(output_dir.join("model.safetensors"))?;

        // Save config for inference
        let head_type_str = if self.config.use_hierarchical {
            "hierarchical"
        } else {
            "flat"
        };
        let config_str = format!(
            "vocab_size: {}\nmax_seq_length: {}\nembed_dim: {}\nnum_filters: {}\nhidden_dim: {}\nn_classes: {}\nfeature_dim: {}\nhead_type: {}\nmodel_type: char_cnn\n",
            self.vocab.vocab_size(),
            self.config.max_seq_length,
            self.config.embed_dim,
            self.config.num_filters,
            self.config.hidden_dim,
            n_classes,
            feature_dim,
            head_type_str
        );
        std::fs::write(output_dir.join("config.yaml"), config_str)?;

        // Save label mapping for inference (sorted, matching label_to_index order)
        let labels: Vec<&str> = taxonomy.labels().iter().map(|s| s.as_str()).collect();
        let labels_json =
            serde_json::to_string_pretty(&labels).unwrap_or_else(|_| "[]".to_string());
        std::fs::write(output_dir.join("labels.json"), labels_json)?;

        eprintln!("Model saved to {:?}", output_dir);

        Ok(())
    }

    /// Prepare a batch for training.
    ///
    /// Returns `(input_ids, features, labels)`. `features` is `Some` when
    /// `use_features` is enabled, containing the feature tensor `(batch, FEATURE_DIM)`.
    fn prepare_batch(
        &self,
        samples: &[&Sample],
        label_to_index: &std::collections::HashMap<String, usize>,
    ) -> Result<(Tensor, Option<Tensor>, Tensor), CharTrainingError> {
        let batch_size = samples.len();
        let max_len = self.config.max_seq_length;

        let mut all_ids = Vec::with_capacity(batch_size * max_len);
        let mut all_features: Vec<f32> = if self.config.use_features {
            Vec::with_capacity(batch_size * FEATURE_DIM)
        } else {
            Vec::new()
        };
        let mut all_labels = Vec::with_capacity(batch_size);

        for sample in samples {
            let ids = self.vocab.encode(&sample.text, max_len);
            all_ids.extend(ids);

            // Extract deterministic features when enabled (NNFT-249)
            if self.config.use_features {
                let feats = extract_features(&sample.text);
                all_features.extend_from_slice(&feats);
            }

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

        let features = if self.config.use_features {
            Some(Tensor::new(all_features, &self.device)?.reshape((batch_size, FEATURE_DIM))?)
        } else {
            None
        };

        Ok((input_ids, features, labels))
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
