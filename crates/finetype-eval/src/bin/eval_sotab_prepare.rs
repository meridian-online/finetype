//! SOTAB CTA value preparation (NNFT-184)
//!
//! Reads SOTAB table files (gzipped JSON) and ground truth CSV,
//! samples up to N non-null values per annotated column, and writes
//! column_values.parquet for DuckDB/CLI classification.
//!
//! Rust port of eval/sotab/prepare_values.py.

use anyhow::{Context, Result};
use arrow::array::{Int32Array, StringArray};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;
use clap::Parser;
use flate2::read::GzDecoder;
use parquet::arrow::ArrowWriter;
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::SeedableRng;
use std::collections::BTreeMap;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::sync::Arc;

const SAMPLE_VALUES_PER_COL: usize = 20;
const MAX_VALUE_LEN: usize = 500;

#[derive(Parser)]
#[command(name = "eval-sotab-prepare", about = "Extract SOTAB column values")]
struct Args {
    #[arg(long, env = "SOTAB_DIR")]
    sotab_dir: Option<PathBuf>,

    #[arg(long, default_value = "validation")]
    split: String,

    #[arg(long)]
    gt_file: Option<String>,

    #[arg(long, short)]
    output: Option<PathBuf>,
}

struct ValueRow {
    table_name: String,
    col_index: i32,
    gt_label: String,
    col_value: String,
}

fn load_ground_truth(gt_path: &std::path::Path) -> Result<BTreeMap<String, Vec<(i32, String)>>> {
    let mut gt: BTreeMap<String, Vec<(i32, String)>> = BTreeMap::new();
    let mut rdr = csv::Reader::from_path(gt_path)
        .with_context(|| format!("Failed to open {}", gt_path.display()))?;
    for result in rdr.records() {
        let record = result?;
        let table_name = record.get(0).unwrap_or("").to_string();
        let col_index: i32 = record.get(1).unwrap_or("0").parse().unwrap_or(0);
        let label = record.get(2).unwrap_or("").to_string();
        gt.entry(table_name).or_default().push((col_index, label));
    }
    Ok(gt)
}

fn extract_table_values(
    table_path: &std::path::Path,
    annotated_cols: &[(i32, String)],
    rng: &mut StdRng,
) -> Vec<ValueRow> {
    let annotated_indices: std::collections::HashSet<i32> =
        annotated_cols.iter().map(|(idx, _)| *idx).collect();
    let mut col_values: BTreeMap<i32, Vec<String>> = BTreeMap::new();

    let file = match std::fs::File::open(table_path) {
        Ok(f) => f,
        Err(_) => return Vec::new(),
    };
    let decoder = GzDecoder::new(file);
    let reader = BufReader::new(decoder);

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };
        let line = line.trim().to_string();
        if line.is_empty() {
            continue;
        }
        let row: serde_json::Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if let Some(obj) = row.as_object() {
            for (key, value) in obj {
                let col_idx: i32 = match key.parse() {
                    Ok(i) => i,
                    Err(_) => continue,
                };
                if !annotated_indices.contains(&col_idx) {
                    continue;
                }
                if !value.is_null() {
                    let s = match value {
                        serde_json::Value::String(s) => s.trim().to_string(),
                        _ => value.to_string().trim().to_string(),
                    };
                    if !s.is_empty() && s.len() < MAX_VALUE_LEN {
                        col_values.entry(col_idx).or_default().push(s);
                    }
                }
            }
        }
    }

    let table_name = table_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let mut rows = Vec::new();

    for (col_idx, label) in annotated_cols {
        let mut values = col_values.get(col_idx).cloned().unwrap_or_default();
        if values.len() > SAMPLE_VALUES_PER_COL {
            values.shuffle(rng);
            values.truncate(SAMPLE_VALUES_PER_COL);
        }
        for v in values {
            rows.push(ValueRow {
                table_name: table_name.clone(),
                col_index: *col_idx,
                gt_label: label.clone(),
                col_value: v,
            });
        }
    }

    rows
}

fn main() -> Result<()> {
    let args = Args::parse();

    let sotab_dir = args.sotab_dir.unwrap_or_else(|| {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/home".to_string());
        PathBuf::from(format!("{home}/datasets/sotab/cta"))
    });

    let (tables_dir, default_gt, gt_dir) = if args.split == "validation" {
        (
            sotab_dir.join("validation/Validation"),
            "CTA_validation_gt.csv",
            sotab_dir.join("validation"),
        )
    } else {
        (
            sotab_dir.join("test/Test"),
            "CTA_test_gt.csv",
            sotab_dir.join("test"),
        )
    };

    let gt_file = args.gt_file.as_deref().unwrap_or(default_gt);
    let gt_path = gt_dir.join(gt_file);

    if !tables_dir.exists() {
        eprintln!("Tables directory not found: {}", tables_dir.display());
        std::process::exit(1);
    }
    if !gt_path.exists() {
        eprintln!("Ground truth not found: {}", gt_path.display());
        std::process::exit(1);
    }

    let output_path = args
        .output
        .unwrap_or_else(|| gt_dir.join("column_values.parquet"));

    let gt = load_ground_truth(&gt_path)?;
    println!(
        "Ground truth: {}",
        gt_path.file_name().unwrap().to_string_lossy()
    );
    println!("  Tables with annotations: {}", gt.len());
    let total_cols: usize = gt.values().map(|v| v.len()).sum();
    println!("  Total annotated columns: {total_cols}");
    let unique_labels: std::collections::HashSet<&str> = gt
        .values()
        .flat_map(|v| v.iter().map(|(_, l)| l.as_str()))
        .collect();
    println!("  Unique labels: {}", unique_labels.len());

    let mut rng = StdRng::seed_from_u64(42);
    let mut all_rows: Vec<ValueRow> = Vec::new();
    let mut found = 0usize;
    let mut missing = 0usize;

    let sorted_gt: Vec<_> = gt.iter().collect();
    for (i, (table_name, cols)) in sorted_gt.iter().enumerate() {
        if i % 500 == 0 {
            println!(
                "  {}/{} tables processed, {} values collected",
                i,
                gt.len(),
                all_rows.len()
            );
        }

        let table_path = tables_dir.join(table_name);
        if !table_path.exists() {
            missing += 1;
            continue;
        }
        found += 1;

        let rows = extract_table_values(&table_path, cols, &mut rng);
        all_rows.extend(rows);
    }

    println!(
        "  {}/{} tables processed, {} values collected",
        gt.len(),
        gt.len(),
        all_rows.len()
    );
    println!("  Found: {found}, Missing: {missing}");

    if all_rows.is_empty() {
        eprintln!("No values extracted!");
        std::process::exit(1);
    }

    // Write parquet
    let schema = Arc::new(Schema::new(vec![
        Field::new("table_name", DataType::Utf8, false),
        Field::new("col_index", DataType::Int32, false),
        Field::new("gt_label", DataType::Utf8, false),
        Field::new("col_value", DataType::Utf8, false),
    ]));

    let table_names: Vec<&str> = all_rows.iter().map(|r| r.table_name.as_str()).collect();
    let col_indices: Vec<i32> = all_rows.iter().map(|r| r.col_index).collect();
    let gt_labels: Vec<&str> = all_rows.iter().map(|r| r.gt_label.as_str()).collect();
    let col_values: Vec<&str> = all_rows.iter().map(|r| r.col_value.as_str()).collect();

    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(StringArray::from(table_names)),
            Arc::new(Int32Array::from(col_indices)),
            Arc::new(StringArray::from(gt_labels)),
            Arc::new(StringArray::from(col_values)),
        ],
    )?;

    let file = std::fs::File::create(&output_path)
        .with_context(|| format!("Failed to create {}", output_path.display()))?;
    let mut writer = ArrowWriter::try_new(file, schema, None)?;
    writer.write(&batch)?;
    writer.close()?;

    let unique_cols: std::collections::HashSet<(String, i32)> = all_rows
        .iter()
        .map(|r| (r.table_name.clone(), r.col_index))
        .collect();
    let unique_labels: std::collections::HashSet<&str> =
        all_rows.iter().map(|r| r.gt_label.as_str()).collect();

    println!("\nOutput: {}", output_path.display());
    println!("  Rows: {}", all_rows.len());
    println!("  Columns: {}", unique_cols.len());
    println!("  Labels: {}", unique_labels.len());

    Ok(())
}
