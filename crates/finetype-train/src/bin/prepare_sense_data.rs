//! CLI binary: prepare-sense-data
//!
//! Loads SOTAB parquet + profile eval CSV columns, encodes with Model2Vec,
//! and writes JSONL training data with pre-computed embeddings.

use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

use finetype_train::data::{prepare_and_write, PrepareConfig};

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
    let args = Args::parse();

    // Expand tilde in sotab_dir
    let sotab_dir = expand_tilde(&args.sotab_dir);

    let config = PrepareConfig {
        sotab_dir,
        output_dir: args.output.clone(),
        max_values: args.max_values,
        val_fraction: args.val_fraction,
        seed: args.seed,
        include_profile: args.include_profile,
        profile_repeat: args.profile_repeat,
        synthetic_headers: args.synthetic_headers,
        header_fraction: args.header_fraction,
        manifest_path: args.manifest,
        schema_mapping_path: args.schema_mapping,
        model2vec_dir: args.model2vec_dir,
    };

    eprintln!("=== prepare-sense-data ===");
    eprintln!("  SOTAB dir:     {}", config.sotab_dir.display());
    eprintln!("  Output:        {}", config.output_dir.display());
    eprintln!("  Max values:    {}", config.max_values);
    eprintln!("  Val fraction:  {}", config.val_fraction);
    eprintln!("  Seed:          {}", config.seed);
    eprintln!("  Profile:       {}", config.include_profile);
    eprintln!("  Syn. headers:  {}", config.synthetic_headers);
    eprintln!();

    let stats = prepare_and_write(&config)?;

    eprintln!();
    eprintln!("=== Results ===");
    eprintln!("  Train samples:     {}", stats.n_train);
    eprintln!("  Val samples:       {}", stats.n_val);
    eprintln!("  SOTAB columns:     {}", stats.n_sotab_columns);
    eprintln!("  Profile columns:   {}", stats.n_profile_columns);
    eprintln!("  Synthetic headers: {}", stats.n_synthetic_headers);
    eprintln!("  Category distribution:");
    let mut cats: Vec<_> = stats.category_distribution.iter().collect();
    cats.sort_by_key(|(k, _)| (*k).clone());
    for (cat, count) in cats {
        eprintln!("    {:12}: {:>6}", cat, count);
    }

    Ok(())
}

/// Expand leading ~ to home directory.
fn expand_tilde(path: &std::path::Path) -> PathBuf {
    let s = path.to_string_lossy();
    if let Some(stripped) = s.strip_prefix("~/") {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home).join(stripped);
        }
    }
    path.to_path_buf()
}
