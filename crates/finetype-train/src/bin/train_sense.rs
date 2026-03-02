//! CLI binary: train-sense-model
//!
//! Trains Sense Architecture A (cross-attention over Model2Vec) for
//! broad category + entity subtype classification.

use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;

use finetype_train::data::SenseDataset;
use finetype_train::sense_train::{train_sense, SenseTrainConfig};

#[derive(Parser, Debug)]
#[command(
    name = "train-sense-model",
    about = "Train Sense classifier (Architecture A)"
)]
struct Args {
    /// Path to training data directory (containing train.jsonl, val.jsonl)
    #[arg(long, default_value = "data/sense_prod")]
    data: PathBuf,

    /// Output directory for model artifacts
    #[arg(long, default_value = "models/sense_prod/arch_a")]
    output: PathBuf,

    /// Maximum training epochs
    #[arg(long, default_value = "50")]
    epochs: usize,

    /// Batch size
    #[arg(long, default_value = "64")]
    batch_size: usize,

    /// Learning rate (AdamW)
    #[arg(long, default_value = "5e-4")]
    lr: f64,

    /// Early stopping patience (epochs without improvement)
    #[arg(long, default_value = "10")]
    patience: usize,

    /// Maximum values per column
    #[arg(long, default_value = "50")]
    max_values: usize,

    /// Random seed for reproducibility
    #[arg(long, default_value = "42")]
    seed: u64,

    /// Header dropout rate during training
    #[arg(long, default_value = "0.5")]
    header_dropout: f64,

    /// AdamW weight decay
    #[arg(long, default_value = "0.01")]
    weight_decay: f64,

    /// Entity loss weight
    #[arg(long, default_value = "0.5")]
    entity_loss_weight: f64,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let args = Args::parse();

    // 1. Load training and validation data
    let train_path = args.data.join("train.jsonl");
    let val_path = args.data.join("val.jsonl");

    tracing::info!("Loading training data from {}", train_path.display());
    let train_data = SenseDataset::load(&train_path)
        .with_context(|| format!("Failed to load training data from {}", train_path.display()))?;

    tracing::info!("Loading validation data from {}", val_path.display());
    let val_data = SenseDataset::load(&val_path)
        .with_context(|| format!("Failed to load validation data from {}", val_path.display()))?;

    tracing::info!(
        "Data loaded: {} train samples, {} val samples",
        train_data.len(),
        val_data.len(),
    );

    // 2. Build config from CLI args
    let config = SenseTrainConfig {
        output_dir: args.output,
        epochs: args.epochs,
        batch_size: args.batch_size,
        lr: args.lr,
        weight_decay: args.weight_decay,
        min_lr: 1e-6,
        patience: args.patience,
        seed: args.seed,
        header_dropout: args.header_dropout,
        entity_loss_weight: args.entity_loss_weight,
    };

    // 3. Train
    let summary = train_sense(&config, &train_data, &val_data)?;

    // 4. Print summary
    println!();
    println!("=== Training Complete ===");
    println!("Best epoch:       {}", summary.best_epoch + 1);
    println!("Best val accuracy: {:.4}", summary.best_val_accuracy);
    println!("Total epochs:     {}", summary.total_epochs);
    println!("Total time:       {:.1}s", summary.total_time_secs);
    println!("Output:           {}", config.output_dir.display());

    Ok(())
}
