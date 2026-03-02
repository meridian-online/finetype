//! CLI binary: train-entity-classifier
//!
//! Trains Deep Sets MLP for entity type demotion gating.
//!
//! Pipeline:
//! 1. Load SOTAB entity columns from parquet via DuckDB
//! 2. Compute 300-dim features (Model2Vec embeddings + 44 statistical)
//! 3. Split train/val (90/10)
//! 4. Train with class-weighted CE loss, AdamW + cosine annealing
//! 5. Save model.safetensors + config.json + label_index.json

use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;
use duckdb::Connection;

use finetype_model::Model2VecResources;
use finetype_train::entity::{
    compute_entity_features, save_entity_model, sotab_to_entity_class, train_entity,
    EntityTrainConfig, ENTITY_LABELS, SOTAB_ENTITY_LABELS,
};

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

    /// Early stopping patience (epochs without improvement)
    #[arg(long, default_value = "15")]
    patience: usize,

    /// Validation split ratio (0.0–1.0)
    #[arg(long, default_value = "0.1")]
    val_split: f64,

    /// Skip cross-validation
    #[arg(long)]
    skip_cv: bool,
}

/// A column of values with its entity label.
struct EntityColumn {
    values: Vec<String>,
    class_idx: usize,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let args = Args::parse();

    // Expand home directory
    let sotab_dir = expand_home(&args.sotab_dir);
    let model2vec_dir = expand_home(&args.model2vec_dir);
    let output_dir = expand_home(&args.output);

    // 1. Load SOTAB entity columns via DuckDB
    tracing::info!("Loading SOTAB entity columns from {}", sotab_dir.display());
    let columns = load_sotab_entity_columns(&sotab_dir)?;
    tracing::info!("Loaded {} entity columns", columns.len());

    // Report class distribution
    let mut class_counts = [0usize; 4];
    for col in &columns {
        class_counts[col.class_idx] += 1;
    }
    for (i, label) in ENTITY_LABELS.iter().enumerate() {
        tracing::info!("  {}: {} columns", label, class_counts[i]);
    }

    // 2. Load Model2Vec resources for embedding computation
    tracing::info!("Loading Model2Vec from {}", model2vec_dir.display());
    let model2vec =
        Model2VecResources::load(&model2vec_dir).context("Failed to load Model2Vec resources")?;
    tracing::info!(
        "Model2Vec loaded: embed_dim={}",
        model2vec.embed_dim().unwrap_or(0)
    );

    // 3. Compute features for all columns
    tracing::info!(
        "Computing 300-dim features for {} columns...",
        columns.len()
    );
    let mut all_features = Vec::with_capacity(columns.len());
    let mut all_labels = Vec::with_capacity(columns.len());

    for (i, col) in columns.iter().enumerate() {
        let features = compute_entity_features(&col.values, &model2vec)
            .with_context(|| format!("Feature computation failed for column {}", i))?;
        all_features.push(features);
        all_labels.push(col.class_idx);

        if (i + 1) % 500 == 0 {
            tracing::info!(
                "  Computed features for {}/{} columns",
                i + 1,
                columns.len()
            );
        }
    }
    tracing::info!("Feature computation complete");

    // 4. Split train/val (stratified by class)
    let (train_features, train_labels, val_features, val_labels) =
        stratified_split(&all_features, &all_labels, args.val_split, args.seed);

    tracing::info!(
        "Split: {} train, {} val (ratio={:.2})",
        train_features.len(),
        val_features.len(),
        args.val_split,
    );

    // 5. Train
    let config = EntityTrainConfig {
        epochs: args.epochs,
        batch_size: args.batch_size,
        lr: args.lr,
        min_lr: 1e-6,
        patience: args.patience,
        demotion_threshold: args.demotion_threshold,
        seed: args.seed,
    };

    let (summary, varmap) = train_entity(
        &config,
        &train_features,
        &train_labels,
        &val_features,
        &val_labels,
    )?;

    tracing::info!(
        "Training complete: best_epoch={}, best_val_acc={:.4}, total_time={:.1}s",
        summary.best_epoch + 1,
        summary.best_val_accuracy,
        summary.total_time_secs,
    );

    // 6. Save model artifacts
    save_entity_model(
        &output_dir,
        &varmap,
        &summary,
        args.demotion_threshold,
        &train_labels,
        &val_labels,
    )?;

    tracing::info!("Model saved to {}", output_dir.display());

    Ok(())
}

/// Load SOTAB entity columns from parquet via DuckDB.
///
/// Reads `column_values.parquet` from the SOTAB directory, filtering to
/// entity-relevant Schema.org types and grouping values by (table, column).
fn load_sotab_entity_columns(sotab_dir: &std::path::Path) -> Result<Vec<EntityColumn>> {
    let parquet_path = sotab_dir.join("column_values.parquet");
    if !parquet_path.exists() {
        anyhow::bail!(
            "SOTAB parquet not found at {}. Expected column_values.parquet in --sotab-dir.",
            parquet_path.display()
        );
    }

    let conn = Connection::open_in_memory()?;

    // Build IN clause for entity labels
    let label_list: String = SOTAB_ENTITY_LABELS
        .iter()
        .map(|l| format!("'{}'", l))
        .collect::<Vec<_>>()
        .join(", ");

    let query = format!(
        "SELECT table_name, col_index, gt_label, col_value \
         FROM read_parquet('{}') \
         WHERE gt_label IN ({}) \
         ORDER BY table_name, col_index",
        parquet_path.display(),
        label_list,
    );

    let mut stmt = conn.prepare(&query)?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
        ))
    })?;

    // Group rows by (table_name, col_index) → column
    let mut columns_map: HashMap<(String, i64), (String, Vec<String>)> = HashMap::new();
    let mut row_count = 0usize;

    for row in rows {
        let (table_name, col_index, gt_label, col_value) = row?;
        let key = (table_name, col_index);
        columns_map
            .entry(key)
            .or_insert_with(|| (gt_label, Vec::new()))
            .1
            .push(col_value);
        row_count += 1;
    }

    tracing::info!(
        "Read {} rows, {} unique columns",
        row_count,
        columns_map.len()
    );

    // Convert to EntityColumn structs, filtering to columns with valid entity mapping
    let mut entity_columns = Vec::new();
    let mut skipped = 0usize;

    for ((_table, _col_idx), (gt_label, values)) in columns_map {
        if let Some(class_idx) = sotab_to_entity_class(&gt_label) {
            entity_columns.push(EntityColumn { values, class_idx });
        } else {
            skipped += 1;
        }
    }

    if skipped > 0 {
        tracing::warn!("Skipped {} columns with unmappable labels", skipped);
    }

    Ok(entity_columns)
}

/// Result of a stratified train/val split.
type SplitResult = (Vec<Vec<f32>>, Vec<usize>, Vec<Vec<f32>>, Vec<usize>);

/// Stratified train/val split preserving class proportions.
fn stratified_split(
    features: &[Vec<f32>],
    labels: &[usize],
    val_ratio: f64,
    seed: u64,
) -> SplitResult {
    use rand::seq::SliceRandom;
    use rand::SeedableRng;

    let mut rng = rand::rngs::StdRng::seed_from_u64(seed);

    // Group indices by class
    let n_classes = 4;
    let mut class_indices: Vec<Vec<usize>> = vec![Vec::new(); n_classes];
    for (i, &label) in labels.iter().enumerate() {
        if label < n_classes {
            class_indices[label].push(i);
        }
    }

    let mut train_features = Vec::new();
    let mut train_labels = Vec::new();
    let mut val_features = Vec::new();
    let mut val_labels = Vec::new();

    for class_idx in &mut class_indices {
        class_idx.shuffle(&mut rng);
        let n_val = ((class_idx.len() as f64) * val_ratio).ceil() as usize;
        let n_val = n_val.max(1).min(class_idx.len());

        for (i, &idx) in class_idx.iter().enumerate() {
            if i < n_val {
                val_features.push(features[idx].clone());
                val_labels.push(labels[idx]);
            } else {
                train_features.push(features[idx].clone());
                train_labels.push(labels[idx]);
            }
        }
    }

    (train_features, train_labels, val_features, val_labels)
}

/// Expand `~` in path to home directory.
fn expand_home(path: &std::path::Path) -> PathBuf {
    let s = path.to_string_lossy();
    if s.starts_with("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(format!("{}{}", home, &s[1..]));
        }
    }
    path.to_path_buf()
}
