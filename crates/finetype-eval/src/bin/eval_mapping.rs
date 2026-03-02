//! YAML → CSV schema mapping converter (NNFT-184)
//!
//! Replaces the Python one-liner in the Makefile eval-mapping target.
//! Reads eval/schema_mapping.yaml and writes eval/schema_mapping.csv.

use anyhow::{Context, Result};
use clap::Parser;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "eval-mapping", about = "Convert schema_mapping.yaml to CSV")]
struct Args {
    #[arg(long, default_value = "eval/schema_mapping.yaml")]
    input: PathBuf,

    #[arg(long, short, default_value = "eval/schema_mapping.csv")]
    output: PathBuf,
}

#[derive(Deserialize)]
struct SchemaMapping {
    mappings: Vec<MappingEntry>,
}

#[derive(Deserialize)]
struct MappingEntry {
    gt_label: String,
    source: String,
    finetype_label: Option<String>,
    #[serde(default)]
    finetype_domain: String,
    match_quality: String,
    #[serde(default)]
    expand: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let text = std::fs::read_to_string(&args.input)
        .with_context(|| format!("Failed to read {}", args.input.display()))?;
    let mapping: SchemaMapping = serde_yaml::from_str(&text)
        .with_context(|| format!("Failed to parse {}", args.input.display()))?;

    let mut wtr = csv::Writer::from_path(&args.output)
        .with_context(|| format!("Failed to create {}", args.output.display()))?;

    wtr.write_record([
        "gt_label",
        "source",
        "finetype_label",
        "finetype_domain",
        "match_quality",
        "expand",
    ])?;

    for m in &mapping.mappings {
        let expand_str = if m.expand { "true" } else { "false" };
        let finetype_label = m.finetype_label.as_deref().unwrap_or("");
        wtr.write_record([
            m.gt_label.as_str(),
            m.source.as_str(),
            finetype_label,
            m.finetype_domain.as_str(),
            m.match_quality.as_str(),
            expand_str,
        ])?;
    }

    wtr.flush()?;
    println!("✓ {} generated", args.output.display());

    Ok(())
}
