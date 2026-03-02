//! CLI binary: prepare-model2vec
//!
//! Computes type label embeddings from the FineType taxonomy using
//! Farthest Point Sampling over Model2Vec-encoded synonyms.

use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;

use finetype_train::model2vec_prep::{compute_type_embeddings, write_type_embeddings};

#[derive(Parser, Debug)]
#[command(
    name = "prepare-model2vec",
    about = "Compute type label embeddings from taxonomy"
)]
struct Args {
    /// Output directory for type embeddings
    #[arg(long, default_value = "models/model2vec")]
    output: PathBuf,

    /// Number of representative embeddings per type (FPS)
    #[arg(long, default_value = "3")]
    max_k: usize,

    /// Path to Model2Vec model directory (tokenizer + token embeddings)
    #[arg(long, default_value = "models/model2vec")]
    model2vec_dir: PathBuf,

    /// Path to taxonomy label definitions directory
    #[arg(long, default_value = "labels")]
    labels_dir: PathBuf,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let args = Args::parse();

    if args.max_k < 1 {
        anyhow::bail!("--max-k must be >= 1");
    }

    eprintln!("=== prepare-model2vec ===");
    eprintln!("  Model2Vec dir: {}", args.model2vec_dir.display());
    eprintln!("  Labels dir:    {}", args.labels_dir.display());
    eprintln!("  Output:        {}", args.output.display());
    eprintln!("  K (reps/type): {}", args.max_k);
    eprintln!();

    // 1. Load taxonomy
    eprintln!("Loading taxonomy from {}...", args.labels_dir.display());
    let taxonomy = finetype_core::Taxonomy::from_directory(&args.labels_dir)
        .context("Failed to load taxonomy")?;
    let n_types = taxonomy.labels().len();
    eprintln!("  Found {} type definitions", n_types);

    // 2. Load Model2Vec
    eprintln!("Loading Model2Vec from {}...", args.model2vec_dir.display());
    let model2vec = finetype_model::Model2VecResources::load(&args.model2vec_dir)
        .context("Failed to load Model2Vec resources")?;
    let dim = model2vec.embed_dim().context("Failed to get embed dim")?;
    eprintln!("  Embedding dimension: {}", dim);

    // 3. Compute type embeddings
    eprintln!(
        "Computing type embeddings ({} types x {} reps)...",
        n_types, args.max_k
    );
    let (embeddings, labels) = compute_type_embeddings(&model2vec, &taxonomy, args.max_k)?;

    let total_rows = labels.len() * args.max_k;
    eprintln!(
        "  Shape: [{}, {}] ({} types x {} reps)",
        total_rows,
        dim,
        labels.len(),
        args.max_k,
    );

    // 4. Write output
    eprintln!("Writing to {}...", args.output.display());
    write_type_embeddings(
        &embeddings,
        labels.len(),
        args.max_k,
        dim,
        &labels,
        &args.output,
    )?;

    eprintln!();
    eprintln!("=== Done ===");
    eprintln!(
        "  type_embeddings.safetensors: {} rows x {} dim",
        total_rows, dim
    );
    eprintln!("  label_index.json: {} labels", labels.len());

    Ok(())
}
