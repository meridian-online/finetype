//! Binary to train Sense model for feasibility spike

use anyhow::Result;
use clap::Parser;
use finetype_candle_spike::TrainingConfig;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "train-sense")]
#[command(about = "Train Sense model (Candle spike proof-of-concept)")]
struct Args {
    #[arg(short, long, default_value = "training_data.jsonl")]
    data_path: PathBuf,

    #[arg(short, long, default_value = "models/sense-candle/")]
    output_dir: PathBuf,

    #[arg(short, long, default_value = "30")]
    epochs: usize,

    #[arg(short, long, default_value = "32")]
    batch_size: usize,

    #[arg(long, default_value = "0.001")]
    lr: f32,

    #[arg(long, default_value = "0.2")]
    val_split: f32,

    #[arg(long, default_value = "42")]
    seed: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let args = Args::parse();

    let config = TrainingConfig {
        data_path: args.data_path,
        output_dir: args.output_dir,
        epochs: args.epochs,
        batch_size: args.batch_size,
        learning_rate: args.lr,
        val_split: args.val_split,
        seed: args.seed,
    };

    tracing::info!("Training Sense model with config: {:?}", config);

    // Run the spike
    finetype_candle_spike::run_spike(config).await?;

    tracing::info!("✅ Spike training complete!");
    Ok(())
}
