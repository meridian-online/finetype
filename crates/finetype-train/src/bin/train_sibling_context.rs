//! CLI binary: train-sibling-context
//!
//! Trains the sibling-context attention module to improve Sense classification
//! using cross-column context from real-world CSV tables.
//!
//! Data pipeline:
//! 1. Read CSVs from --csv-dir (default: data/csvs/)
//! 2. Profile each column with Model2Vec + Sense to get silver labels
//! 3. Cache prepared data to --cache-dir for fast reloading
//! 4. Train attention module with frozen Sense
//! 5. Save to --output (default: models/sibling-context/)

use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;

use finetype_train::sibling_data::{
    load_csv_tables, prepare_table_samples, SiblingDataset,
};
use finetype_train::sibling_train::{train_sibling_context, SiblingTrainConfig};

#[derive(Parser, Debug)]
#[command(
    name = "train-sibling-context",
    about = "Train sibling-context attention module for cross-column disambiguation"
)]
struct Args {
    /// Directory containing CSV files for training
    #[arg(long, default_value = "data/csvs")]
    csv_dir: PathBuf,

    /// Directory for cached prepared data (JSONL)
    #[arg(long, default_value = "data/sibling_context_cache")]
    cache_dir: PathBuf,

    /// Frozen Sense model directory
    #[arg(long, default_value = "models/sense")]
    sense_model: PathBuf,

    /// Model2Vec resources directory
    #[arg(long, default_value = "models/model2vec")]
    model2vec: PathBuf,

    /// Output directory for trained model
    #[arg(long, default_value = "models/sibling-context")]
    output: PathBuf,

    /// Maximum training epochs
    #[arg(long, default_value = "100")]
    epochs: usize,

    /// Learning rate (AdamW)
    #[arg(long, default_value = "1e-4")]
    lr: f64,

    /// AdamW weight decay
    #[arg(long, default_value = "0.01")]
    weight_decay: f64,

    /// Early stopping patience (epochs without improvement)
    #[arg(long, default_value = "15")]
    patience: usize,

    /// Random seed for reproducibility
    #[arg(long, default_value = "42")]
    seed: u64,

    /// Gradient accumulation steps (tables per optimizer step)
    #[arg(long, default_value = "4")]
    grad_accum: usize,

    /// Validation split fraction (by table count)
    #[arg(long, default_value = "0.2")]
    val_fraction: f64,

    /// Maximum values to sample per column
    #[arg(long, default_value = "50")]
    max_values: usize,

    /// Force re-preparation of data (ignore cache)
    #[arg(long)]
    no_cache: bool,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let args = Args::parse();

    // 1. Load or prepare data
    let cache_path = args.cache_dir.join("tables.jsonl");
    let dataset = if cache_path.exists() && !args.no_cache {
        tracing::info!("Loading cached data from {}", cache_path.display());
        SiblingDataset::load(&cache_path)?
    } else {
        tracing::info!("Preparing data from CSVs in {}", args.csv_dir.display());

        // Load Model2Vec resources
        tracing::info!("Loading Model2Vec from {}", args.model2vec.display());
        let m2v = finetype_model::Model2VecResources::load(&args.model2vec)
            .with_context(|| format!("Failed to load Model2Vec from {}", args.model2vec.display()))?;

        // Load Sense classifier (inference version for silver labels)
        tracing::info!("Loading Sense classifier from {}", args.sense_model.display());
        let sense = finetype_model::SenseClassifier::load(&args.sense_model)
            .with_context(|| format!("Failed to load Sense from {}", args.sense_model.display()))?;

        // Read CSVs
        let raw_tables = load_csv_tables(&args.csv_dir, args.max_values)?;
        tracing::info!("Read {} CSV files", raw_tables.len());

        // Prepare: encode + classify
        let tables = prepare_table_samples(&raw_tables, &m2v, &sense, args.max_values)?;
        let dataset = SiblingDataset::from_tables(tables);

        // Cache for next run
        std::fs::create_dir_all(&args.cache_dir)?;
        dataset.save(&cache_path)?;
        tracing::info!("Cached prepared data to {}", cache_path.display());

        dataset
    };

    tracing::info!(
        "Dataset: {} tables, {} columns",
        dataset.len(),
        dataset.total_columns(),
    );

    // 2. Train/val split
    let (train_data, val_data) = dataset.train_val_split(args.val_fraction, args.seed);
    tracing::info!(
        "Split: {} train tables ({} cols), {} val tables ({} cols)",
        train_data.len(),
        train_data.total_columns(),
        val_data.len(),
        val_data.total_columns(),
    );

    // 3. Build training config
    let config = SiblingTrainConfig {
        output_dir: args.output.clone(),
        sense_model_dir: args.sense_model,
        epochs: args.epochs,
        lr: args.lr,
        weight_decay: args.weight_decay,
        min_lr: 1e-6,
        patience: args.patience,
        seed: args.seed,
        grad_accum_steps: args.grad_accum,
    };

    // 4. Train
    let summary = train_sibling_context(&config, &train_data, &val_data)?;

    // 5. Print summary
    println!();
    println!("=== Sibling-Context Training Complete ===");
    println!("Best epoch:       {}", summary.best_epoch + 1);
    println!("Best val accuracy: {:.4}", summary.best_val_accuracy);
    println!("Total epochs:     {}", summary.total_epochs);
    println!("Total time:       {:.1}s", summary.total_time_secs);
    println!("Output:           {}", args.output.display());
    println!();
    println!("Next steps:");
    println!("  1. Run profile eval:  make eval-report");
    println!("  2. Compare baseline:  180/186 → ?/186");

    Ok(())
}
