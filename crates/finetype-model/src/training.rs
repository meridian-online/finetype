//! Training utilities for text classification.

use crate::model::{TextClassifier, TextClassifierConfig};
use candle_core::{DType, Device, Tensor};
use candle_nn::{AdamW, Optimizer, ParamsAdamW, VarBuilder, VarMap};
use finetype_core::{Sample, Taxonomy, Tokenizer};
use std::path::Path;
use thiserror::Error;
#[derive(Error, Debug)]
pub enum TrainingError {
    #[error("Model error: {0}")]
    ModelError(#[from] candle_core::Error),
    #[error("Tokenizer error: {0}")]
    TokenizerError(#[from] finetype_core::tokenizer::TokenizerError),
    #[error("Generator error: {0}")]
    GeneratorError(#[from] finetype_core::generator::GeneratorError),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Training configuration.
#[derive(Debug, Clone)]
pub struct TrainingConfig {
    pub batch_size: usize,
    pub epochs: usize,
    pub learning_rate: f64,
    pub max_seq_length: usize,
    pub warmup_steps: usize,
    pub weight_decay: f64,
}

impl Default for TrainingConfig {
    fn default() -> Self {
        Self {
            batch_size: 32,
            epochs: 5,
            learning_rate: 1e-4,
            max_seq_length: 256,
            warmup_steps: 1000,
            weight_decay: 0.01,
        }
    }
}

/// Trainer for the text classifier.
pub struct Trainer {
    config: TrainingConfig,
    device: Device,
}

impl Trainer {
    /// Create a new trainer.
    pub fn new(config: TrainingConfig) -> Self {
        let device = crate::device::get_device();
        Self { config, device }
    }

    /// Train the model.
    pub fn train(
        &self,
        taxonomy: &Taxonomy,
        samples: &[Sample],
        output_dir: &Path,
    ) -> Result<(), TrainingError> {
        eprintln!("Starting training with {} samples", samples.len());
        eprintln!("Device: {:?}", self.device);

        // Create label mapping
        eprintln!("Creating label mapping...");
        let label_to_index = taxonomy.label_to_index();
        let n_classes = taxonomy.len();
        eprintln!("Number of classes: {}", n_classes);

        // Initialize model
        eprintln!("Initializing model...");
        let varmap = VarMap::new();
        let vb = VarBuilder::from_varmap(&varmap, DType::F32, &self.device);

        let model_config = TextClassifierConfig {
            n_classes,
            max_seq_length: self.config.max_seq_length,
            ..Default::default()
        };

        let model = TextClassifier::new(model_config, vb)?;
        eprintln!("Model initialized");

        eprintln!("Loading tokenizer...");
        let tokenizer = Tokenizer::bert_cased()?;
        eprintln!("Tokenizer loaded");

        // Create optimizer
        let params = ParamsAdamW {
            lr: self.config.learning_rate,
            weight_decay: self.config.weight_decay,
            ..Default::default()
        };
        let mut optimizer = AdamW::new(varmap.all_vars(), params)?;

        // Training loop
        let num_batches = samples.len().div_ceil(self.config.batch_size);
        eprintln!("Training: {} batches per epoch", num_batches);

        for epoch in 0..self.config.epochs {
            eprintln!("Starting epoch {}/{}", epoch + 1, self.config.epochs);
            let mut total_loss = 0.0;
            let mut num_correct = 0;
            let mut num_total = 0;

            for batch_idx in 0..num_batches {
                let start = batch_idx * self.config.batch_size;
                let end = (start + self.config.batch_size).min(samples.len());
                let batch = &samples[start..end];

                // Prepare batch
                let (input_ids, attention_mask, labels) =
                    self.prepare_batch(batch, &tokenizer, &label_to_index)?;

                // Forward pass
                let logits = model.forward(&input_ids, Some(&attention_mask))?;
                let logits = logits.contiguous()?;

                // Compute loss (cross-entropy)
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

                // Print progress every 10 batches
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

        eprintln!("Model saved to {:?}", output_dir);

        Ok(())
    }

    /// Prepare a batch for training.
    fn prepare_batch(
        &self,
        samples: &[Sample],
        tokenizer: &Tokenizer,
        label_to_index: &std::collections::HashMap<String, usize>,
    ) -> Result<(Tensor, Tensor, Tensor), TrainingError> {
        let batch_size = samples.len();
        let max_len = self.config.max_seq_length;

        let mut all_ids = Vec::with_capacity(batch_size * max_len);
        let mut all_masks = Vec::with_capacity(batch_size * max_len);
        let mut all_labels = Vec::with_capacity(batch_size);

        for sample in samples {
            let (ids, mask) = tokenizer.encode_padded(&sample.text, max_len)?;
            all_ids.extend(ids);
            all_masks.extend(mask);

            let label_idx = label_to_index.get(&sample.label).copied().unwrap_or(0) as u32;
            all_labels.push(label_idx);
        }

        let input_ids = Tensor::new(all_ids, &self.device)?.reshape((batch_size, max_len))?;
        let attention_mask = Tensor::new(all_masks, &self.device)?
            .reshape((batch_size, max_len))?
            .to_dtype(DType::F32)?;
        let labels = Tensor::new(all_labels, &self.device)?;

        Ok((input_ids, attention_mask, labels))
    }
}
