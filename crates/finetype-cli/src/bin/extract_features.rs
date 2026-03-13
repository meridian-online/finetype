//! Feature extraction for the disambiguator spike (AC-2/AC-3).
//!
//! Reads the eval manifest, profiles each column, and writes a CSV with:
//! - Column metadata (dataset, header, ground truth label)
//! - Classification result (predicted label, confidence, disambiguation rule)
//! - Aggregated column features (36 × 4 = 144 floats: mean/var/min/max)
//! - Vote distribution (top 5 types with fractions)

use anyhow::{Context, Result};
use csv::{ReaderBuilder, WriterBuilder};
use finetype_core::Taxonomy;
use finetype_model::{
    CharClassifier, ColumnClassifier, ColumnConfig, EntityClassifier, LabelCategoryMap,
    Model2VecResources, SemanticHintClassifier, SenseClassifier, ValueClassifier, FEATURE_DIM,
    FEATURE_NAMES,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

fn main() -> Result<()> {
    let manifest_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "eval/datasets/manifest.csv".to_string());
    let output_path = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "specs/2026-03-disambiguator-spike/features.csv".to_string());

    eprintln!("Loading models...");
    let cc = setup_classifier()?;
    eprintln!("Models loaded.");

    // Read manifest
    eprintln!("Reading manifest: {}", manifest_path);
    let mut rdr = ReaderBuilder::new().from_path(&manifest_path)?;

    // Group by file_path to avoid re-reading CSVs
    let mut file_columns: HashMap<String, Vec<(String, String, String)>> = HashMap::new(); // file → [(dataset, column, gt_label)]
    for record in rdr.records() {
        let record = record?;
        let dataset = record.get(0).unwrap_or("").to_string();
        let file_path = record.get(1).unwrap_or("").to_string();
        let column_name = record.get(2).unwrap_or("").to_string();
        let gt_label = record.get(3).unwrap_or("").to_string();

        // Skip non-CSV files (JSON datasets need different handling)
        if !file_path.ends_with(".csv") {
            eprintln!("  Skipping non-CSV: {}", file_path);
            continue;
        }

        file_columns
            .entry(file_path)
            .or_default()
            .push((dataset, column_name, gt_label));
    }

    // Build CSV header
    let mut wtr = WriterBuilder::new().from_path(&output_path)?;

    let mut header: Vec<String> = vec![
        "dataset".into(),
        "file_path".into(),
        "column_name".into(),
        "gt_label".into(),
        "predicted_label".into(),
        "confidence".into(),
        "disambiguation_applied".into(),
        "disambiguation_rule".into(),
        "is_generic".into(),
        "samples_used".into(),
    ];

    // Feature columns: feat_mean_<name>, feat_var_<name>, feat_min_<name>, feat_max_<name>
    for prefix in &["mean", "var", "min", "max"] {
        for name in FEATURE_NAMES.iter() {
            header.push(format!("feat_{}_{}", prefix, name));
        }
    }

    // Vote distribution columns (top 5)
    for i in 1..=5 {
        header.push(format!("vote{}_label", i));
        header.push(format!("vote{}_fraction", i));
    }

    wtr.write_record(&header)?;

    let total_columns: usize = file_columns.values().map(|c| c.len()).sum();
    let mut processed = 0;
    eprintln!(
        "Processing {} columns across {} files",
        total_columns,
        file_columns.len()
    );

    for (file_path, columns) in &file_columns {
        // Read the CSV file
        let full_path = PathBuf::from(file_path);
        if !full_path.exists() {
            eprintln!("  WARNING: File not found: {}", file_path);
            continue;
        }

        let (headers, column_data, _row_count) = match read_csv_columns(&full_path) {
            Ok(data) => data,
            Err(e) => {
                eprintln!("  ERROR reading {}: {}", file_path, e);
                continue;
            }
        };

        // Build header→index map
        let header_map: HashMap<&str, usize> = headers
            .iter()
            .enumerate()
            .map(|(i, h)| (h.as_str(), i))
            .collect();

        for (dataset, column_name, gt_label) in columns {
            let col_idx = match header_map.get(column_name.as_str()) {
                Some(&idx) => idx,
                None => {
                    eprintln!(
                        "  WARNING: Column '{}' not found in {}",
                        column_name, file_path
                    );
                    continue;
                }
            };

            let values = &column_data[col_idx];
            if values.is_empty() {
                eprintln!("  WARNING: Empty column '{}' in {}", column_name, file_path);
                continue;
            }

            // Classify with header hint (the standard pipeline)
            let result = cc
                .classify_column_with_header(values, column_name)
                .context(format!("classifying {}.{}", file_path, column_name))?;

            // Build output row
            let mut row: Vec<String> = vec![
                dataset.clone(),
                file_path.clone(),
                column_name.clone(),
                gt_label.clone(),
                result.label.clone(),
                format!("{:.4}", result.confidence),
                result.disambiguation_applied.to_string(),
                result.disambiguation_rule.clone().unwrap_or_default(),
                result.is_generic.to_string(),
                result.samples_used.to_string(),
            ];

            // Features (144 values)
            if let Some(ref features) = result.column_features {
                for val in features.mean.iter() {
                    row.push(format!("{:.6}", val));
                }
                for val in features.variance.iter() {
                    row.push(format!("{:.6}", val));
                }
                for val in features.min.iter() {
                    row.push(format!("{:.6}", val));
                }
                for val in features.max.iter() {
                    row.push(format!("{:.6}", val));
                }
            } else {
                // No features (legacy path) — fill with empty
                for _ in 0..(FEATURE_DIM * 4) {
                    row.push(String::new());
                }
            }

            // Vote distribution (top 5, padded)
            for i in 0..5 {
                if i < result.vote_distribution.len() {
                    row.push(result.vote_distribution[i].0.clone());
                    row.push(format!("{:.4}", result.vote_distribution[i].1));
                } else {
                    row.push(String::new());
                    row.push(String::new());
                }
            }

            wtr.write_record(&row)?;
            processed += 1;

            if processed % 20 == 0 {
                eprintln!("  Processed {}/{} columns", processed, total_columns);
            }
        }
    }

    wtr.flush()?;
    eprintln!("Done. Wrote {} columns to {}", processed, output_path);

    Ok(())
}

/// Read a CSV file and return (headers, columns, row_count).
/// Each column is a Vec<String> of all values.
fn read_csv_columns(path: &Path) -> Result<(Vec<String>, Vec<Vec<String>>, usize)> {
    let mut rdr = ReaderBuilder::new().from_path(path)?;
    let headers: Vec<String> = rdr.headers()?.iter().map(|h| h.to_string()).collect();
    let n_cols = headers.len();

    let mut columns: Vec<Vec<String>> = vec![Vec::new(); n_cols];
    let mut row_count = 0;

    for record in rdr.records() {
        let record = record?;
        for (i, field) in record.iter().enumerate() {
            if i < n_cols {
                columns[i].push(field.to_string());
            }
        }
        row_count += 1;
    }

    Ok((headers, columns, row_count))
}

/// Set up the full column classifier with all models.
fn setup_classifier() -> Result<ColumnClassifier> {
    let model_path = find_model_path()?;

    eprintln!("  Loading CharCNN from {:?}", model_path);
    let classifier = CharClassifier::load(&model_path)?;
    let boxed: Box<dyn ValueClassifier> = Box::new(classifier);

    let config = ColumnConfig {
        sample_size: 100,
        ..Default::default()
    };

    let mut cc = if let Some(semantic) = load_semantic_hint() {
        eprintln!("  Loaded semantic hint classifier (Model2Vec)");
        let entity = load_entity_classifier(&semantic);
        let mut cc = ColumnClassifier::with_semantic_hint(boxed, config, semantic);
        if let Some(entity) = entity {
            eprintln!("  Loaded entity classifier");
            cc.set_entity_classifier(entity);
        }
        cc
    } else {
        ColumnClassifier::new(boxed, config)
    };

    // Load taxonomy
    let taxonomy_path = PathBuf::from("labels");
    if let Ok(mut taxonomy) = load_taxonomy(&taxonomy_path) {
        taxonomy.compile_validators();
        taxonomy.compile_locale_validators();
        eprintln!(
            "  Loaded taxonomy ({} types, {} validators)",
            taxonomy.labels().len(),
            taxonomy.validator_count()
        );
        cc.set_taxonomy(taxonomy);
    }

    // Wire up Sense classifier
    wire_sense(&mut cc);
    // Note: sibling context not wired — would need all column headers at once

    Ok(cc)
}

fn find_model_path() -> Result<PathBuf> {
    // Try workspace-relative paths
    for candidate in &["models/default", "models/char-cnn-v14-250"] {
        let p = PathBuf::from(candidate);
        if p.exists() {
            return Ok(p);
        }
    }
    anyhow::bail!("Could not find model directory. Run from the finetype workspace root.")
}

fn load_semantic_hint() -> Option<SemanticHintClassifier> {
    let model_dir = PathBuf::from("models/model2vec");
    if model_dir.join("model.safetensors").exists() {
        return SemanticHintClassifier::load(&model_dir)
            .map_err(|e| eprintln!("  WARNING: Failed to load Model2Vec: {e}"))
            .ok();
    }
    None
}

fn load_entity_classifier(semantic: &SemanticHintClassifier) -> Option<EntityClassifier> {
    let model_dir = PathBuf::from("models/entity-classifier");
    if model_dir.join("model.safetensors").exists() {
        return EntityClassifier::load(
            &model_dir,
            semantic.tokenizer().clone(),
            semantic.embeddings().clone(),
        )
        .map_err(|e| eprintln!("  WARNING: Failed to load entity classifier: {e}"))
        .ok();
    }
    None
}

fn wire_sense(cc: &mut ColumnClassifier) {
    let sense_dir = PathBuf::from("models/sense");
    if !sense_dir.join("model.safetensors").exists() {
        return;
    }
    let sense = match SenseClassifier::load(&sense_dir) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("  WARNING: Could not load Sense classifier: {}", e);
            return;
        }
    };
    let m2v_dir = PathBuf::from("models/model2vec");
    let m2v = match Model2VecResources::load(&m2v_dir) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("  WARNING: Could not load Model2Vec resources: {}", e);
            return;
        }
    };
    let label_map = LabelCategoryMap::new();
    cc.set_sense(sense, m2v, label_map);
    eprintln!("  Loaded Sense classifier");
}

fn load_taxonomy(path: &Path) -> Result<Taxonomy> {
    let taxonomy = Taxonomy::from_directory(path)?;
    Ok(taxonomy)
}
