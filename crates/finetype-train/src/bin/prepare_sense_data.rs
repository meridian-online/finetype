//! CLI binary: prepare-sense-data
//!
//! Loads SOTAB parquet + profile eval CSV columns, encodes with Model2Vec,
//! and writes JSONL training data with pre-computed embeddings.

use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "prepare-sense-data", about = "Prepare Sense training data")]
struct Args {
    /// SOTAB data directory
    #[arg(long, default_value = "~/datasets/sotab/cta")]
    sotab_dir: PathBuf,

    /// Output directory for JSONL files
    #[arg(long, default_value = "data/sense_prod")]
    output: PathBuf,

    /// Maximum values per column
    #[arg(long, default_value = "50")]
    max_values: usize,

    /// Validation split fraction
    #[arg(long, default_value = "0.2")]
    val_fraction: f64,

    /// Random seed
    #[arg(long, default_value = "42")]
    seed: u64,

    /// Include profile eval columns from manifest
    #[arg(long)]
    include_profile: bool,

    /// Repeat profile columns N times (upsampling)
    #[arg(long, default_value = "50")]
    profile_repeat: usize,

    /// Generate synthetic headers for SOTAB columns
    #[arg(long)]
    synthetic_headers: bool,

    /// Fraction of SOTAB columns to give synthetic headers
    #[arg(long, default_value = "0.5")]
    header_fraction: f64,

    /// Path to evaluation manifest CSV
    #[arg(long, default_value = "eval/datasets/manifest.csv")]
    manifest: PathBuf,

    /// Path to schema mapping YAML
    #[arg(long, default_value = "eval/schema_mapping.yaml")]
    schema_mapping: PathBuf,

    /// Path to Model2Vec model directory
    #[arg(long, default_value = "models/model2vec")]
    model2vec_dir: PathBuf,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let _args = Args::parse();

    // TODO: Implement in Step 5
    // 1. Load SOTAB parquet via DuckDB
    // 2. Map SOTAB labels → broad categories + entity subtypes
    // 3. Sample values (top-K by frequency + random fill)
    // 4. Optionally load profile eval columns
    // 5. Optionally generate synthetic headers
    // 6. Encode values + headers with Model2VecResources
    // 7. Write train.jsonl + val.jsonl
    // 8. Write meta.json

    eprintln!("prepare-sense-data: not yet implemented (NNFT-185 Step 5)");
    std::process::exit(1);
}
