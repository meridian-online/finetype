//! CLI binary: train-sense-model
//!
//! Trains Sense Architecture A (cross-attention over Model2Vec) for
//! broad category + entity subtype classification.

use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

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
}

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let _args = Args::parse();

    // TODO: Implement in Step 3
    // 1. Load train.jsonl and val.jsonl from args.data
    // 2. Create SenseModelA with VarMap
    // 3. Training loop with AdamW + cosine annealing
    // 4. Early stopping on val broad accuracy
    // 5. Save best model to args.output

    eprintln!("train-sense-model: not yet implemented (NNFT-185 Step 3)");
    std::process::exit(1);
}
