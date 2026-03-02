//! CLI binary: prepare-model2vec
//!
//! Computes type label embeddings from the FineType taxonomy using
//! Farthest Point Sampling over Model2Vec-encoded synonyms.

use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

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
    let _args = Args::parse();

    // TODO: Implement in Step 6
    // 1. Load taxonomy from YAML (via finetype_core::Taxonomy)
    // 2. For each type: collect synonyms (title + aliases + label components)
    // 3. Encode synonyms with Model2VecResources::encode_batch()
    // 4. FPS: select K representatives per type
    // 5. Write type_embeddings.safetensors + label_index.json

    eprintln!("prepare-model2vec: not yet implemented (NNFT-185 Step 6)");
    std::process::exit(1);
}
