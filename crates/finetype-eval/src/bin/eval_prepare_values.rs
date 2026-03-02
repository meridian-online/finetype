//! GitTables value preparation (NNFT-184)
//!
//! Read sampled parquet files, unpivot columns, sample values per column.
//! Outputs column_values.parquet for DuckDB classification.
//!
//! Rust port of eval/gittables/prepare_1m_values.py.

use anyhow::{Context, Result};
use arrow::array::{Array, StringArray};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;
use clap::Parser;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use parquet::arrow::ArrowWriter;
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::SeedableRng;
use std::path::PathBuf;
use std::sync::Arc;

const SAMPLE_VALUES_PER_COL: usize = 20;
const MAX_VALUE_LEN: usize = 500;

#[derive(Parser)]
#[command(
    name = "eval-prepare-values",
    about = "Extract column values from sampled parquet files"
)]
struct Args {
    #[arg(long, env = "EVAL_OUTPUT")]
    output_dir: Option<PathBuf>,

    #[arg(long, env = "GITTABLES_DIR")]
    gittables_dir: Option<PathBuf>,
}

struct ValueRow {
    topic: String,
    table_name: String,
    col_name: String,
    col_value: String,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let gittables_dir = args.gittables_dir.unwrap_or_else(|| {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/home".to_string());
        PathBuf::from(format!("{home}/datasets/gittables"))
    });
    let output_dir = args
        .output_dir
        .unwrap_or_else(|| gittables_dir.join("eval_output"));

    // Read metadata CSV
    let metadata_path = output_dir.join("metadata.csv");
    let mut metadata = Vec::new();
    {
        let mut rdr = csv::Reader::from_path(&metadata_path)
            .with_context(|| format!("Failed to open {}", metadata_path.display()))?;
        let headers: Vec<String> = rdr.headers()?.iter().map(|s| s.to_string()).collect();
        for result in rdr.records() {
            let record = result?;
            let mut row = std::collections::HashMap::new();
            for (i, header) in headers.iter().enumerate() {
                row.insert(header.clone(), record.get(i).unwrap_or("").to_string());
            }
            metadata.push(row);
        }
    }

    println!("Processing {} parquet files...", metadata.len());

    let mut rng = StdRng::seed_from_u64(42);
    let mut rows: Vec<ValueRow> = Vec::new();
    let mut errors = 0usize;

    for (i, meta) in metadata.iter().enumerate() {
        if i % 500 == 0 {
            println!(
                "  {}/{} files processed, {} values collected",
                i,
                metadata.len(),
                rows.len()
            );
        }

        let file_path = meta.get("file_path").cloned().unwrap_or_default();
        let topic = meta.get("topic").cloned().unwrap_or_default();
        let table_name = meta.get("table_name").cloned().unwrap_or_default();

        let file = match std::fs::File::open(&file_path) {
            Ok(f) => f,
            Err(_) => {
                errors += 1;
                continue;
            }
        };

        let builder = match ParquetRecordBatchReaderBuilder::try_new(file) {
            Ok(b) => b,
            Err(_) => {
                errors += 1;
                continue;
            }
        };

        let reader = match builder.build() {
            Ok(r) => r,
            Err(_) => {
                errors += 1;
                continue;
            }
        };

        for batch_result in reader {
            let batch = match batch_result {
                Ok(b) => b,
                Err(_) => {
                    errors += 1;
                    continue;
                }
            };

            for col_idx in 0..batch.num_columns() {
                let col_name = batch.schema().field(col_idx).name().clone();
                let col = batch.column(col_idx);

                // Convert column to string values
                let mut values: Vec<String> = Vec::new();
                // Try to cast to string array
                let string_col = arrow::compute::cast(col, &DataType::Utf8).ok();
                if let Some(ref arr) = string_col {
                    let str_arr = arr.as_any().downcast_ref::<StringArray>();
                    if let Some(str_arr) = str_arr {
                        for i in 0..str_arr.len() {
                            if !str_arr.is_null(i) {
                                let s = str_arr.value(i).trim().to_string();
                                if !s.is_empty() && s.len() < MAX_VALUE_LEN {
                                    values.push(s);
                                }
                            }
                        }
                    }
                }

                // Sample
                if values.len() > SAMPLE_VALUES_PER_COL {
                    values.shuffle(&mut rng);
                    values.truncate(SAMPLE_VALUES_PER_COL);
                }

                for v in values {
                    rows.push(ValueRow {
                        topic: topic.clone(),
                        table_name: table_name.clone(),
                        col_name: col_name.clone(),
                        col_value: v,
                    });
                }
            }
        }
    }

    println!(
        "  {}/{} files processed, {} values collected",
        metadata.len(),
        metadata.len(),
        rows.len()
    );
    println!("  Errors: {errors}");

    // Write as parquet
    let schema = Arc::new(Schema::new(vec![
        Field::new("topic", DataType::Utf8, false),
        Field::new("table_name", DataType::Utf8, false),
        Field::new("col_name", DataType::Utf8, false),
        Field::new("col_value", DataType::Utf8, false),
    ]));

    let topics: Vec<&str> = rows.iter().map(|r| r.topic.as_str()).collect();
    let table_names: Vec<&str> = rows.iter().map(|r| r.table_name.as_str()).collect();
    let col_names: Vec<&str> = rows.iter().map(|r| r.col_name.as_str()).collect();
    let col_values: Vec<&str> = rows.iter().map(|r| r.col_value.as_str()).collect();

    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(StringArray::from(topics)),
            Arc::new(StringArray::from(table_names)),
            Arc::new(StringArray::from(col_names)),
            Arc::new(StringArray::from(col_values)),
        ],
    )?;

    let out_path = output_dir.join("column_values.parquet");
    let file = std::fs::File::create(&out_path)
        .with_context(|| format!("Failed to create {}", out_path.display()))?;
    let mut writer = ArrowWriter::try_new(file, schema, None)?;
    writer.write(&batch)?;
    writer.close()?;

    // Count unique tables and columns
    let unique_tables: std::collections::HashSet<String> = rows
        .iter()
        .map(|r| format!("{}/{}", r.topic, r.table_name))
        .collect();
    let unique_cols: std::collections::HashSet<String> = rows
        .iter()
        .map(|r| format!("{}/{}/{}", r.topic, r.table_name, r.col_name))
        .collect();

    println!("\nOutput: {}", out_path.display());
    println!("  Rows: {}", rows.len());
    println!("  Tables: {}", unique_tables.len());
    println!("  Columns: {}", unique_cols.len());

    Ok(())
}
