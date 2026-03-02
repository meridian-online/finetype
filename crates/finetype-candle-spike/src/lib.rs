//! Candle feasibility spike for FineType training
//!
//! This crate evaluates whether HuggingFace Candle can handle FineType's ML requirements:
//! - Sense Architecture A (cross-attention over Model2Vec embeddings)
//! - Entity classifier MLP (Deep Sets architecture)
//! - Training with safetensors serialization

pub mod data;
pub mod models;
pub mod training;

pub use data::SenseDataset;
pub use models::{EntityClassifier, SenseModelA};
pub use training::TrainingConfig;

use anyhow::Result;
use candle_core::Device;
use candle_nn::VarMap;

/// Main spike entry point: train and validate Sense model
pub async fn run_spike(config: TrainingConfig) -> Result<()> {
    tracing::info!("Starting Candle feasibility spike...");
    tracing::info!("Config: {:?}", config);

    let device = Device::Cpu;

    // 1. Load training data
    tracing::info!("Loading training data from {:?}", config.data_path);
    let dataset = SenseDataset::load(&config.data_path).await?;
    tracing::info!("Loaded {} columns", dataset.len());

    // 2. Create model with VarMap for parameter tracking
    tracing::info!("Initializing Sense model (cross-attention)...");
    let varmap = VarMap::new();
    let model = SenseModelA::new(&varmap, &device)?;
    tracing::info!("Model created with {} parameters", varmap.all_vars().len());

    // 3. Run training
    tracing::info!("Starting training loop...");
    let metrics = training::train_sense(&model, &dataset, &config).await?;

    // 4. Validate accuracy
    tracing::info!("Training complete. Metrics: {:?}", metrics);

    if metrics.val_broad_accuracy < 0.90 {
        tracing::warn!("Validation accuracy below 90% threshold!");
        return Err(anyhow::anyhow!(
            "Accuracy too low: {:.2}%",
            metrics.val_broad_accuracy * 100.0
        ));
    }

    tracing::info!("Spike successful! Candle viability confirmed.");
    Ok(())
}
