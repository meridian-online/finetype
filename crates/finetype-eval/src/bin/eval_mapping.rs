//! YAML → CSV schema mapping converter
//!
//! Replaces the Python one-liner in the Makefile eval-mapping target.
//! Reads eval/schema_mapping.yaml and writes eval/schema_mapping.csv.

use anyhow::{Context, Result};
use clap::Parser;
use serde::Deserialize;
use std::collections::BTreeSet;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "eval-mapping", about = "Convert schema_mapping.yaml to CSV")]
struct Args {
    #[arg(long, default_value = "eval/schema_mapping.yaml")]
    input: PathBuf,

    #[arg(long, short, default_value = "eval/schema_mapping.csv")]
    output: PathBuf,

    /// Path to labels/ directory for taxonomy validation
    #[arg(long, default_value = "labels")]
    labels_dir: PathBuf,

    /// Path to eval manifest CSV for coverage validation
    #[arg(long, default_value = "eval/datasets/manifest.csv")]
    manifest: PathBuf,

    /// Validate mapping labels against taxonomy and manifest (fails on mismatch)
    #[arg(long)]
    validate: bool,
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
    /// Optional list of acceptable FineType labels for coarse GT labels.
    /// When present, each label generates a separate CSV row so the eval SQL
    /// can match any of them.
    #[serde(default)]
    finetype_labels: Vec<String>,
    #[serde(default)]
    finetype_domain: Option<String>,
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
        let domain = m.finetype_domain.as_deref().unwrap_or("");

        if m.finetype_labels.is_empty() {
            // Single label (or null): one row
            let finetype_label = m.finetype_label.as_deref().unwrap_or("");
            wtr.write_record([
                m.gt_label.as_str(),
                m.source.as_str(),
                finetype_label,
                domain,
                m.match_quality.as_str(),
                expand_str,
            ])?;
        } else {
            // Multiple acceptable labels: one row per label
            for label in &m.finetype_labels {
                wtr.write_record([
                    m.gt_label.as_str(),
                    m.source.as_str(),
                    label.as_str(),
                    domain,
                    m.match_quality.as_str(),
                    expand_str,
                ])?;
            }
        }
    }

    wtr.flush()?;
    println!("✓ {} generated", args.output.display());

    if args.validate {
        let errors = validate_mapping(&mapping, &args.labels_dir, &args.manifest)?;
        if errors.is_empty() {
            println!("✓ All mapping labels exist in taxonomy");
            println!("✓ All manifest gt_labels have a mapping entry");
        } else {
            for e in &errors {
                eprintln!("  ✗ {e}");
            }
            anyhow::bail!("{} validation error(s) found", errors.len());
        }
    }

    Ok(())
}

/// Validate that every finetype_label in the mapping exists in the taxonomy,
/// and every gt_label in the eval manifest has at least one mapping entry.
fn validate_mapping(
    mapping: &SchemaMapping,
    labels_dir: &PathBuf,
    manifest_path: &PathBuf,
) -> Result<Vec<String>> {
    let mut errors = Vec::new();

    // 1. Load taxonomy labels from YAML definition files
    let taxonomy_labels = load_taxonomy_labels(labels_dir)?;
    println!(
        "  Taxonomy: {} types from {}",
        taxonomy_labels.len(),
        labels_dir.display()
    );

    // 2. Check every finetype_label against taxonomy
    let mut mapping_labels = BTreeSet::new();
    for m in &mapping.mappings {
        if let Some(ref label) = m.finetype_label {
            if !label.is_empty() {
                mapping_labels.insert(label.clone());
            }
        }
        for label in &m.finetype_labels {
            mapping_labels.insert(label.clone());
        }
    }

    for label in &mapping_labels {
        if !taxonomy_labels.contains(label.as_str()) {
            errors.push(format!(
                "Mapping references '{label}' but it does not exist in taxonomy"
            ));
        }
    }
    println!(
        "  Mapping: {} unique finetype_labels checked",
        mapping_labels.len()
    );

    // 3. Check every manifest gt_label has a mapping entry
    if manifest_path.exists() {
        let mapping_gt_labels: BTreeSet<String> = mapping
            .mappings
            .iter()
            .map(|m| m.gt_label.clone())
            .collect();

        let mut rdr = csv::Reader::from_path(manifest_path)
            .with_context(|| format!("Failed to read {}", manifest_path.display()))?;

        let mut manifest_gt_labels = BTreeSet::new();
        for record in rdr.records() {
            let record = record?;
            if let Some(gt_label) = record.get(3) {
                manifest_gt_labels.insert(gt_label.to_string());
            }
        }

        for gt_label in &manifest_gt_labels {
            if !mapping_gt_labels.contains(gt_label) {
                errors.push(format!(
                    "Manifest gt_label '{gt_label}' has no entry in schema_mapping"
                ));
            }
        }
        println!(
            "  Manifest: {} unique gt_labels checked",
            manifest_gt_labels.len()
        );
    } else {
        println!(
            "  WARN: Manifest not found at {}, skipping coverage check",
            manifest_path.display()
        );
    }

    Ok(errors)
}

/// Parse taxonomy labels from YAML definition files.
/// Each top-level key (e.g., `identity.person.email:`) is a type label.
fn load_taxonomy_labels(labels_dir: &PathBuf) -> Result<BTreeSet<String>> {
    let mut labels = BTreeSet::new();
    let pattern = labels_dir.join("definitions_*.yaml");
    let pattern_str = pattern.to_string_lossy();

    for entry in
        glob::glob(&pattern_str).with_context(|| format!("Invalid glob pattern: {pattern_str}"))?
    {
        let path = entry?;
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read {}", path.display()))?;

        for line in text.lines() {
            // Top-level keys start at column 0, contain dots, and end with ':'
            if !line.starts_with(' ')
                && !line.starts_with('#')
                && line.contains('.')
                && line.ends_with(':')
            {
                labels.insert(line.trim_end_matches(':').to_string());
            }
        }
    }

    Ok(labels)
}
