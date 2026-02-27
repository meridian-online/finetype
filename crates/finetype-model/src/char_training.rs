//! Training utilities for character-level CNN classifier.

use crate::char_cnn::{CharCnn, CharCnnConfig, CharVocab};
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
        let n_classes = taxonomy.len();
        eprintln!("Number of classes: {}", n_classes);

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
        eprintln!(
            "Model initialized (vocab_size={}, embed_dim={}, filters={})",
            self.vocab.vocab_size(),
            self.config.embed_dim,
            self.config.num_filters
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

            for batch_idx in 0..num_batches {
                let start = batch_idx * self.config.batch_size;
                let end = (start + self.config.batch_size).min(samples_vec.len());
                let batch: Vec<&Sample> = samples_vec[start..end].to_vec();

                // Prepare batch
                let (input_ids, labels) = self.prepare_batch(&batch, &label_to_index)?;

                // Forward pass
                let logits = model.forward(&input_ids)?;
                let logits = logits.contiguous()?;

                // Compute loss
                let loss = candle_nn::loss::cross_entropy(&logits, &labels)?;

                // Backward pass
                optimizer.backward_step(&loss)?;

                // Track metrics
                let loss_val = loss.to_scalar::<f32>()?;
                total_loss += loss_val;

                // Compute accuracy
                let predictions = logits.argmax(1)?;
                let correct = predictions
                    .eq(&labels)?
                    .to_dtype(DType::F32)?
                    .sum_all()?
                    .to_scalar::<f32>()?;
                num_correct += correct as usize;
                num_total += batch.len();

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

            eprintln!(
                "Epoch {}/{}: loss={:.4}, accuracy={:.2}%",
                epoch + 1,
                self.config.epochs,
                avg_loss,
                accuracy * 100.0
            );
        }

        // Save model
        eprintln!("Saving model to {:?}", output_dir);
        std::fs::create_dir_all(output_dir)?;
        varmap.save(output_dir.join("model.safetensors"))?;

        // Save config for inference
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

        // Save label mapping for inference (sorted, matching label_to_index order)
        let labels: Vec<&str> = taxonomy.labels().iter().map(|s| s.as_str()).collect();
        let labels_json =
            serde_json::to_string_pretty(&labels).unwrap_or_else(|_| "[]".to_string());
        std::fs::write(output_dir.join("labels.json"), labels_json)?;

        eprintln!("Model saved to {:?}", output_dir);

        Ok(())
    }

    /// Prepare a batch for training.
    fn prepare_batch(
        &self,
        samples: &[&Sample],
        label_to_index: &std::collections::HashMap<String, usize>,
    ) -> Result<(Tensor, Tensor), CharTrainingError> {
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
