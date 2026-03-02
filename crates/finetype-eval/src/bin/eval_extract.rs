//! GitTables 1M metadata extraction (NNFT-184)
//!
//! Samples tables per topic from the GitTables corpus, extracts parquet metadata
//! (schema.org / dbpedia column type annotations), writes catalog.csv, metadata.csv,
//! and sampled_files.txt.
//!
//! Rust port of eval/gittables/extract_metadata_1m.py.

use anyhow::{Context, Result};
use clap::Parser;
use parquet::file::reader::{FileReader, SerializedFileReader};
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::SeedableRng;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "eval-extract", about = "Extract GitTables 1M metadata")]
struct Args {
    #[arg(long, env = "GITTABLES_DIR")]
    gittables_dir: Option<PathBuf>,

    #[arg(long, default_value = "50")]
    samples_per_topic: usize,

    #[arg(long, env = "EVAL_OUTPUT")]
    output_dir: Option<PathBuf>,
}

fn extract_parquet_metadata(filepath: &Path) -> Option<Value> {
    let file = std::fs::File::open(filepath).ok()?;
    let reader = SerializedFileReader::new(file).ok()?;
    let metadata = reader.metadata();
    let file_meta = metadata.file_metadata().key_value_metadata()?;
    for kv in file_meta {
        if kv.key == "gittables" {
            if let Some(ref val) = kv.value {
                return serde_json::from_str(val).ok();
            }
        }
    }
    None
}

fn main() -> Result<()> {
    let args = Args::parse();

    let gittables_dir = args.gittables_dir.unwrap_or_else(|| {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/home".to_string());
        PathBuf::from(format!("{home}/datasets/gittables"))
    });
    let topics_dir = gittables_dir.join("topics");
    let output_dir = args
        .output_dir
        .unwrap_or_else(|| gittables_dir.join("eval_output"));

    std::fs::create_dir_all(&output_dir)
        .with_context(|| format!("Failed to create output dir: {}", output_dir.display()))?;

    // Catalog all topics
    let mut topics: Vec<PathBuf> = std::fs::read_dir(&topics_dir)
        .with_context(|| format!("Failed to read topics dir: {}", topics_dir.display()))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    topics.sort();

    println!(
        "Found {} topics (sampling {}/topic)",
        topics.len(),
        args.samples_per_topic
    );

    let mut rng = StdRng::seed_from_u64(42);
    let mut catalog_rows: Vec<BTreeMap<String, String>> = Vec::new();
    let mut metadata_rows: Vec<BTreeMap<String, String>> = Vec::new();
    let mut total_files = 0usize;
    let mut total_sampled = 0usize;
    let mut total_annotated = 0usize;

    for topic_dir in &topics {
        let topic = topic_dir.file_name().unwrap().to_string_lossy().to_string();

        let mut parquet_files: Vec<PathBuf> = std::fs::read_dir(topic_dir)?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "parquet"))
            .collect();
        parquet_files.sort();
        total_files += parquet_files.len();

        // Sample
        let sample_size = args.samples_per_topic.min(parquet_files.len());
        let sample: Vec<PathBuf> = if sample_size < parquet_files.len() {
            let mut shuffled = parquet_files.clone();
            shuffled.shuffle(&mut rng);
            shuffled.into_iter().take(sample_size).collect()
        } else {
            parquet_files.clone()
        };
        total_sampled += sample.len();

        catalog_rows.push(BTreeMap::from([
            ("topic".to_string(), topic.clone()),
            ("total_tables".to_string(), parquet_files.len().to_string()),
            ("sampled_tables".to_string(), sample.len().to_string()),
        ]));

        let mut annotated_count = 0usize;
        for fp in &sample {
            let meta = match extract_parquet_metadata(fp) {
                Some(m) => m,
                None => continue,
            };

            let has_schema = meta
                .get("schema_semantic_column_types")
                .map(|v| v.is_object() && !v.as_object().unwrap().is_empty())
                .unwrap_or(false);
            let has_dbpedia = meta
                .get("dbpedia_semantic_column_types")
                .map(|v| v.is_object() && !v.as_object().unwrap().is_empty())
                .unwrap_or(false);
            let nrows = meta
                .get("number_rows")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            let ncols = meta
                .get("number_columns")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);

            let schema_types = meta
                .get("schema_semantic_column_types")
                .and_then(|v| v.as_object())
                .cloned()
                .unwrap_or_default();
            let dbpedia_types = meta
                .get("dbpedia_semantic_column_types")
                .and_then(|v| v.as_object())
                .cloned()
                .unwrap_or_default();

            // Prefer schema.org, fall back to dbpedia
            let mut annotations: BTreeMap<String, String> = BTreeMap::new();
            for (col, info) in &dbpedia_types {
                let label = if let Some(obj) = info.as_object() {
                    obj.get("cleaned_label")
                        .or_else(|| obj.get("id"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string()
                } else {
                    info.to_string()
                };
                annotations.insert(col.clone(), label);
            }
            for (col, info) in &schema_types {
                let label = if let Some(obj) = info.as_object() {
                    obj.get("cleaned_label")
                        .or_else(|| obj.get("id"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string()
                } else {
                    info.to_string()
                };
                annotations.insert(col.clone(), label);
            }

            if !annotations.is_empty() {
                annotated_count += 1;
            }

            let annotations_json = if annotations.is_empty() {
                String::new()
            } else {
                serde_json::to_string(&annotations).unwrap_or_default()
            };

            metadata_rows.push(BTreeMap::from([
                ("topic".to_string(), topic.clone()),
                (
                    "table_name".to_string(),
                    fp.file_stem()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string(),
                ),
                ("file_path".to_string(), fp.to_string_lossy().to_string()),
                ("nrows".to_string(), nrows.to_string()),
                ("ncols".to_string(), ncols.to_string()),
                ("has_schema".to_string(), has_schema.to_string()),
                ("has_dbpedia".to_string(), has_dbpedia.to_string()),
                (
                    "n_annotated_cols".to_string(),
                    annotations.len().to_string(),
                ),
                ("annotations_json".to_string(), annotations_json),
            ]));
        }

        total_annotated += annotated_count;
        let ann_pct = if !sample.is_empty() {
            annotated_count as f64 / sample.len() as f64 * 100.0
        } else {
            0.0
        };
        println!(
            "  {}: {} tables, sampled {}, {} annotated ({:.0}%)",
            topic,
            parquet_files.len(),
            sample.len(),
            annotated_count,
            ann_pct
        );
    }

    // Write catalog
    {
        let mut wtr = csv::Writer::from_path(output_dir.join("catalog.csv"))?;
        wtr.write_record(["topic", "total_tables", "sampled_tables"])?;
        for row in &catalog_rows {
            wtr.write_record([
                row.get("topic").unwrap().as_str(),
                row.get("total_tables").unwrap().as_str(),
                row.get("sampled_tables").unwrap().as_str(),
            ])?;
        }
        wtr.flush()?;
    }

    // Write metadata
    {
        let fields = [
            "topic",
            "table_name",
            "file_path",
            "nrows",
            "ncols",
            "has_schema",
            "has_dbpedia",
            "n_annotated_cols",
            "annotations_json",
        ];
        let mut wtr = csv::Writer::from_path(output_dir.join("metadata.csv"))?;
        wtr.write_record(&fields)?;
        for row in &metadata_rows {
            let values: Vec<&str> = fields
                .iter()
                .map(|f| row.get(*f).map(|s| s.as_str()).unwrap_or(""))
                .collect();
            wtr.write_record(&values)?;
        }
        wtr.flush()?;
    }

    // Write file list
    {
        let file_list: Vec<String> = metadata_rows
            .iter()
            .map(|r| r.get("file_path").cloned().unwrap_or_default())
            .collect();
        std::fs::write(
            output_dir.join("sampled_files.txt"),
            file_list.join("\n") + "\n",
        )?;
    }

    let ann_pct = if total_sampled > 0 {
        total_annotated as f64 / total_sampled as f64 * 100.0
    } else {
        0.0
    };
    println!("\n=== Summary ===");
    println!("Topics: {}", topics.len());
    println!("Total tables: {total_files}");
    println!("Sampled: {total_sampled}");
    println!("With annotations: {total_annotated} ({ann_pct:.1}%)");
    println!("\nOutput: {}/", output_dir.display());
    println!("  catalog.csv: {} rows", catalog_rows.len());
    println!("  metadata.csv: {} rows", metadata_rows.len());
    println!("  sampled_files.txt: {} paths", metadata_rows.len());

    Ok(())
}
