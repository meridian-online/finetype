//! GitTables 1M CLI evaluation (NNFT-184)
//!
//! Reads pre-extracted column_values.parquet, groups by column, pipes through
//! `finetype infer --mode column --batch` with header hints, writes cli_predictions.csv.
//!
//! Rust port of eval/gittables/eval_cli.py (NNFT-130).

use anyhow::{Context, Result};
use clap::Parser;
use std::collections::BTreeMap;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Instant;

#[derive(Parser)]
#[command(
    name = "eval-gittables-cli",
    about = "GitTables 1M CLI batch evaluation"
)]
struct Args {
    #[arg(long, env = "GITTABLES_DIR")]
    gittables_dir: Option<PathBuf>,

    #[arg(long, env = "EVAL_OUTPUT")]
    output_dir: Option<PathBuf>,

    #[arg(long, env = "FINETYPE_BIN")]
    finetype_bin: Option<String>,
}

fn load_column_values(
    parquet_path: &std::path::Path,
) -> Result<BTreeMap<(String, String, String), Vec<String>>> {
    let conn = duckdb::Connection::open_in_memory()?;
    let mut stmt = conn.prepare(&format!(
        "SELECT topic, table_name, col_name, col_value FROM read_parquet('{}')",
        parquet_path.display()
    ))?;

    let mut columns: BTreeMap<(String, String, String), Vec<String>> = BTreeMap::new();
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
        ))
    })?;

    for row in rows {
        let (topic, table_name, col_name, col_value) = row?;
        columns
            .entry((topic, table_name, col_name))
            .or_default()
            .push(col_value);
    }

    Ok(columns)
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

    let parquet_path = output_dir.join("column_values.parquet");
    if !parquet_path.exists() {
        eprintln!(
            "column_values.parquet not found at {}",
            parquet_path.display()
        );
        eprintln!("Run: make eval-values");
        std::process::exit(1);
    }

    eprintln!("Loading column values from {}...", parquet_path.display());
    let columns = load_column_values(&parquet_path)?;
    let total_values: usize = columns.values().map(|v| v.len()).sum();
    eprintln!("  {} columns, {} values", columns.len(), total_values);

    // Build command
    let cmd_parts: Vec<String> = if let Some(ref bin) = args.finetype_bin {
        bin.split_whitespace().map(|s| s.to_string()).collect()
    } else {
        vec!["cargo".to_string(), "run".to_string(), "--".to_string()]
    };

    let mut cmd_args = cmd_parts.clone();
    cmd_args.extend(
        ["infer", "--mode", "column", "--batch"]
            .iter()
            .map(|s| s.to_string()),
    );

    eprintln!("Running: {}", cmd_args.join(" "));
    eprintln!("Classifying {} columns...", columns.len());

    // Build ordered key list
    let keys: Vec<_> = columns.keys().cloned().collect();

    // Generate JSONL input
    let mut jsonl_lines = Vec::new();
    for key in &keys {
        let values = &columns[key];
        let obj = serde_json::json!({
            "header": key.2,
            "values": values
        });
        jsonl_lines.push(serde_json::to_string(&obj)?);
    }
    let jsonl_input = jsonl_lines.join("\n") + "\n";

    let t_start = Instant::now();
    let mut child = Command::new(&cmd_args[0])
        .args(&cmd_args[1..])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("Failed to spawn: {}", cmd_args[0]))?;

    child
        .stdin
        .take()
        .unwrap()
        .write_all(jsonl_input.as_bytes())?;

    let output = child.wait_with_output()?;
    let elapsed = t_start.elapsed();
    eprintln!("Classification completed in {:.1}s", elapsed.as_secs_f64());

    if !output.status.success() {
        eprintln!(
            "finetype stderr:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        eprintln!("finetype exited with code {:?}", output.status.code());
        std::process::exit(1);
    }

    if !output.stderr.is_empty() {
        for line in String::from_utf8_lossy(&output.stderr).lines() {
            eprintln!("  [finetype] {}", line);
        }
    }

    // Parse output JSONL
    let stdout = String::from_utf8_lossy(&output.stdout);
    let output_lines: Vec<&str> = stdout.lines().filter(|l| !l.trim().is_empty()).collect();

    if output_lines.len() != keys.len() {
        eprintln!(
            "WARNING: Expected {} output lines, got {}",
            keys.len(),
            output_lines.len()
        );
    }

    let mut results = Vec::new();
    for (key, line) in keys.iter().zip(output_lines.iter()) {
        let pred: serde_json::Value = serde_json::from_str(line).unwrap_or_else(|_| {
            eprintln!("WARNING: Invalid JSON: {}", &line[..line.len().min(100)]);
            serde_json::json!({"label": "PARSE_ERROR", "confidence": 0.0})
        });

        results.push(BTreeMap::from([
            ("topic".to_string(), key.0.clone()),
            ("table_name".to_string(), key.1.clone()),
            ("col_name".to_string(), key.2.clone()),
            (
                "predicted_label".to_string(),
                pred["label"].as_str().unwrap_or("UNKNOWN").to_string(),
            ),
            (
                "confidence".to_string(),
                pred["confidence"].as_f64().unwrap_or(0.0).to_string(),
            ),
            (
                "samples_used".to_string(),
                pred["samples_used"].as_i64().unwrap_or(0).to_string(),
            ),
            (
                "disambiguation_rule".to_string(),
                pred["disambiguation_rule"]
                    .as_str()
                    .unwrap_or("")
                    .to_string(),
            ),
        ]));
    }

    // Write predictions CSV
    let output_path = output_dir.join("cli_predictions.csv");
    let mut wtr = csv::Writer::from_path(&output_path)?;
    wtr.write_record([
        "topic",
        "table_name",
        "col_name",
        "predicted_label",
        "confidence",
        "samples_used",
        "disambiguation_rule",
    ])?;
    for r in &results {
        wtr.write_record([
            r["topic"].as_str(),
            r["table_name"].as_str(),
            r["col_name"].as_str(),
            r["predicted_label"].as_str(),
            r["confidence"].as_str(),
            r["samples_used"].as_str(),
            r["disambiguation_rule"].as_str(),
        ])?;
    }
    wtr.flush()?;

    eprintln!("\nOutput: {}", output_path.display());
    eprintln!("  Predictions: {}", results.len());
    let unique_labels: std::collections::HashSet<&str> = results
        .iter()
        .map(|r| r["predicted_label"].as_str())
        .collect();
    eprintln!("  Unique labels: {}", unique_labels.len());

    let disambiguated = results
        .iter()
        .filter(|r| !r["disambiguation_rule"].is_empty())
        .count();
    eprintln!(
        "  Disambiguated: {} ({:.1}%)",
        disambiguated,
        disambiguated as f64 * 100.0 / results.len().max(1) as f64
    );

    Ok(())
}
