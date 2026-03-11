//! Data preparation for sibling-context attention training.
//!
//! Reads CSVs from a directory, profiles each with FineType's Sense classifier
//! to get silver labels, and pre-computes Model2Vec embeddings. Outputs are
//! cached as JSONL for fast reloading.

use anyhow::{bail, Context, Result};
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Model2Vec embedding dimension (used in tests and data structures).
#[cfg(test)]
const EMBED_DIM: usize = 128;

// ── Data Structures ──────────────────────────────────────────────────────────

/// A single column within a table, with pre-computed embeddings and silver label.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiblingColumn {
    /// Column header text.
    pub header: String,
    /// Pre-computed Model2Vec embedding of header [128].
    pub header_embed: Vec<f32>,
    /// Pre-computed Model2Vec embeddings of sampled values [n_values, 128].
    pub value_embeds: Vec<Vec<f32>>,
    /// Value mask: true for real (non-zero-embedding) values.
    pub value_mask: Vec<bool>,
    /// Silver Sense broad category index (0–5).
    pub broad_category_idx: usize,
    /// Silver Sense entity subtype index (0–3), meaningful only when broad=0 (entity).
    pub entity_subtype_idx: usize,
}

/// A table: a collection of columns from one CSV file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableSample {
    /// Source filename (for debugging).
    pub table_id: String,
    /// Columns in this table.
    pub columns: Vec<SiblingColumn>,
}

/// Dataset of tables for sibling-context training.
pub struct SiblingDataset {
    pub tables: Vec<TableSample>,
}

impl SiblingDataset {
    /// Load from JSONL file (one TableSample per line).
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read sibling dataset: {}", path.display()))?;

        let mut tables = Vec::new();
        for (i, line) in content.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let table: TableSample = serde_json::from_str(line)
                .with_context(|| format!("Failed to parse table on line {}", i + 1))?;
            tables.push(table);
        }

        tracing::info!(
            "Loaded {} tables ({} total columns) from {}",
            tables.len(),
            tables.iter().map(|t| t.columns.len()).sum::<usize>(),
            path.display(),
        );
        Ok(Self { tables })
    }

    /// Save to JSONL file.
    pub fn save(&self, path: &Path) -> Result<()> {
        use std::io::Write;
        let file = std::fs::File::create(path)
            .with_context(|| format!("Failed to create {}", path.display()))?;
        let mut writer = std::io::BufWriter::new(file);
        for table in &self.tables {
            serde_json::to_writer(&mut writer, table)?;
            writeln!(writer)?;
        }
        tracing::info!("Saved {} tables to {}", self.tables.len(), path.display());
        Ok(())
    }

    /// Create from pre-built tables.
    pub fn from_tables(tables: Vec<TableSample>) -> Self {
        Self { tables }
    }

    /// Number of tables.
    pub fn len(&self) -> usize {
        self.tables.len()
    }

    /// Whether the dataset is empty.
    pub fn is_empty(&self) -> bool {
        self.tables.is_empty()
    }

    /// Total number of columns across all tables.
    pub fn total_columns(&self) -> usize {
        self.tables.iter().map(|t| t.columns.len()).sum()
    }

    /// Split into train/val by table index.
    pub fn train_val_split(self, val_fraction: f64, seed: u64) -> (SiblingDataset, SiblingDataset) {
        use rand::SeedableRng;
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        let mut indices: Vec<usize> = (0..self.tables.len()).collect();
        indices.shuffle(&mut rng);

        let val_count = (self.tables.len() as f64 * val_fraction).round() as usize;
        let val_count = val_count.max(1).min(self.tables.len() - 1);

        let val_indices: std::collections::HashSet<usize> =
            indices[..val_count].iter().copied().collect();

        let mut train_tables = Vec::new();
        let mut val_tables = Vec::new();

        for (i, table) in self.tables.into_iter().enumerate() {
            if val_indices.contains(&i) {
                val_tables.push(table);
            } else {
                train_tables.push(table);
            }
        }

        (
            SiblingDataset::from_tables(train_tables),
            SiblingDataset::from_tables(val_tables),
        )
    }
}

// ── Data Preparation ─────────────────────────────────────────────────────────

/// Read all CSVs from a directory, returning (filename, headers, columns).
///
/// Each column is a Vec of sampled string values (up to `max_values`).
/// A raw table: (filename, headers, column_values).
pub type RawTable = (String, Vec<String>, Vec<Vec<String>>);

pub fn load_csv_tables(csv_dir: &Path, max_values: usize) -> Result<Vec<RawTable>> {
    let mut entries: Vec<_> = std::fs::read_dir(csv_dir)
        .with_context(|| format!("Failed to read CSV directory: {}", csv_dir.display()))?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("csv"))
        })
        .collect();

    entries.sort_by_key(|e| e.path());

    let mut tables = Vec::new();

    for entry in &entries {
        let path = entry.path();
        let filename = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        match read_csv_columns(&path, max_values) {
            Ok((headers, columns)) => {
                if !headers.is_empty() {
                    tables.push((filename, headers, columns));
                }
            }
            Err(e) => {
                tracing::warn!("Skipping {}: {}", filename, e);
            }
        }
    }

    tracing::info!(
        "Loaded {} CSV files from {}",
        tables.len(),
        csv_dir.display(),
    );
    Ok(tables)
}

/// Read a single CSV file, returning headers and column values.
fn read_csv_columns(path: &Path, max_values: usize) -> Result<(Vec<String>, Vec<Vec<String>>)> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_path(path)?;

    let headers: Vec<String> = rdr.headers()?.iter().map(|h| h.to_string()).collect();

    if headers.is_empty() {
        bail!("No headers found");
    }

    let n_cols = headers.len();
    let mut columns: Vec<Vec<String>> = vec![Vec::new(); n_cols];

    for result in rdr.records() {
        let record = match result {
            Ok(r) => r,
            Err(_) => continue,
        };
        for (i, field) in record.iter().enumerate() {
            if i >= n_cols {
                break;
            }
            let val = field.trim();
            if !val.is_empty() && columns[i].len() < max_values {
                columns[i].push(val.to_string());
            }
        }
        // Stop if all columns have enough values
        if columns.iter().all(|c| c.len() >= max_values) {
            break;
        }
    }

    Ok((headers, columns))
}

/// Prepare table samples by encoding with Model2Vec and classifying with Sense.
///
/// For each CSV table:
/// 1. Encode each column header with Model2Vec → [128] embedding
/// 2. Encode column values with Model2Vec → [N, 128] embeddings
/// 3. Classify with Sense → silver broad category label
///
/// Returns prepared TableSample entries ready for training.
pub fn prepare_table_samples(
    raw_tables: &[RawTable],
    m2v: &finetype_model::Model2VecResources,
    sense: &finetype_model::SenseClassifier,
    max_values: usize,
) -> Result<Vec<TableSample>> {
    let mut tables = Vec::new();

    for (i, (filename, headers, columns)) in raw_tables.iter().enumerate() {
        if i > 0 && i % 100 == 0 {
            tracing::info!("  Prepared {}/{} tables...", i, raw_tables.len());
        }

        let mut table_columns = Vec::new();

        for (j, header) in headers.iter().enumerate() {
            let col_values = &columns[j];

            // Skip empty columns
            if col_values.is_empty() {
                continue;
            }

            // Encode header
            let header_batch = m2v.encode_batch(&[header.as_str()])?;
            let header_embed_tensor = header_batch.squeeze(0)?;
            let header_embed: Vec<f32> = header_embed_tensor.to_vec1()?;

            // Check header embedding is non-zero
            let header_norm: f32 = header_embed.iter().map(|x| x * x).sum::<f32>().sqrt();
            if header_norm < 1e-8 {
                continue; // Skip columns with empty/unparseable headers
            }

            // Encode values (up to max_values)
            let n_values = col_values.len().min(max_values);
            let value_strs: Vec<&str> = col_values
                .iter()
                .take(n_values)
                .map(|s| s.as_str())
                .collect();
            let value_embs_tensor = m2v.encode_batch(&value_strs)?; // [N, 128]

            // Build value mask (non-zero embeddings)
            let mut value_embeds = Vec::with_capacity(n_values);
            let mut value_mask = Vec::with_capacity(n_values);
            for vi in 0..n_values {
                let row: Vec<f32> = value_embs_tensor.get(vi)?.to_vec1()?;
                let is_nonzero = row.iter().any(|&v| v.abs() > 1e-8);
                value_mask.push(is_nonzero);
                value_embeds.push(row);
            }

            // Classify with Sense to get silver label
            let sense_result = sense.classify(m2v, Some(header.as_str()), &value_strs)?;
            let broad_idx = sense_result.broad_category as usize;
            let entity_idx = sense_result.entity_subtype.map(|e| e as usize).unwrap_or(0);

            table_columns.push(SiblingColumn {
                header: header.clone(),
                header_embed,
                value_embeds,
                value_mask,
                broad_category_idx: broad_idx,
                entity_subtype_idx: entity_idx,
            });
        }

        if !table_columns.is_empty() {
            tables.push(TableSample {
                table_id: filename.clone(),
                columns: table_columns,
            });
        }
    }

    tracing::info!(
        "Prepared {} tables ({} columns)",
        tables.len(),
        tables.iter().map(|t| t.columns.len()).sum::<usize>(),
    );
    Ok(tables)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_table_sample_serde_round_trip() {
        let table = TableSample {
            table_id: "test.csv".to_string(),
            columns: vec![
                SiblingColumn {
                    header: "name".to_string(),
                    header_embed: vec![0.1; EMBED_DIM],
                    value_embeds: vec![vec![0.2; EMBED_DIM]; 3],
                    value_mask: vec![true, true, false],
                    broad_category_idx: 0,
                    entity_subtype_idx: 0,
                },
                SiblingColumn {
                    header: "age".to_string(),
                    header_embed: vec![0.3; EMBED_DIM],
                    value_embeds: vec![vec![0.4; EMBED_DIM]; 2],
                    value_mask: vec![true, true],
                    broad_category_idx: 3,
                    entity_subtype_idx: 0,
                },
            ],
        };

        let json = serde_json::to_string(&table).unwrap();
        let parsed: TableSample = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.table_id, "test.csv");
        assert_eq!(parsed.columns.len(), 2);
        assert_eq!(parsed.columns[0].header, "name");
        assert_eq!(parsed.columns[0].broad_category_idx, 0);
        assert_eq!(parsed.columns[1].broad_category_idx, 3);
    }

    #[test]
    fn test_train_val_split() {
        let tables: Vec<TableSample> = (0..20)
            .map(|i| TableSample {
                table_id: format!("table_{}.csv", i),
                columns: vec![SiblingColumn {
                    header: "col".to_string(),
                    header_embed: vec![0.0; EMBED_DIM],
                    value_embeds: vec![],
                    value_mask: vec![],
                    broad_category_idx: 0,
                    entity_subtype_idx: 0,
                }],
            })
            .collect();

        let dataset = SiblingDataset::from_tables(tables);
        let (train, val) = dataset.train_val_split(0.2, 42);

        assert_eq!(train.len() + val.len(), 20);
        assert_eq!(val.len(), 4); // 20 * 0.2 = 4
        assert_eq!(train.len(), 16);
    }
}
