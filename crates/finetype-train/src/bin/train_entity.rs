//! CLI binary: train-entity-classifier
//!
//! Trains Deep Sets MLP for entity type demotion gating.

use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "train-entity-classifier",
    about = "Train entity classifier (Deep Sets MLP)"
)]
struct Args {
    /// SOTAB data directory (containing column_values.parquet)
    #[arg(long, default_value = "~/datasets/sotab/cta")]
    sotab_dir: PathBuf,

    /// Output directory for model artifacts
    #[arg(long, default_value = "models/entity-classifier")]
    output: PathBuf,

    /// MLP hidden dimension
    #[arg(long, default_value = "256")]
    hidden_dim: usize,

    /// Maximum training epochs
    #[arg(long, default_value = "100")]
    epochs: usize,

    /// Learning rate (AdamW)
    #[arg(long, default_value = "5e-4")]
    lr: f64,

    /// Batch size
    #[arg(long, default_value = "64")]
    batch_size: usize,

    /// Dropout rate
    #[arg(long, default_value = "0.2")]
    dropout: f64,

    /// Demotion confidence threshold
    #[arg(long, default_value = "0.6")]
    demotion_threshold: f64,

    /// Random seed
    #[arg(long, default_value = "42")]
    seed: u64,

    /// Path to Model2Vec model directory
    #[arg(long, default_value = "models/model2vec")]
    model2vec_dir: PathBuf,

    /// Skip cross-validation
    #[arg(long)]
    skip_cv: bool,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let _args = Args::parse();

    // TODO: Implement in Step 4
    // 1. Load SOTAB entity columns via DuckDB
    // 2. Compute 300-dim features (Model2Vec embeddings + stats)
    // 3. Training loop with class-weighted CE, AdamW
    // 4. Optional K-fold CV
    // 5. Save best model to args.output

    eprintln!("train-entity-classifier: not yet implemented (NNFT-185 Step 4)");
    std::process::exit(1);
}
