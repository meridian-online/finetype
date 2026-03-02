//! FineType Actionability Evaluation (NNFT-184)
//!
//! Tests whether FineType's format_string predictions work on real data.
//! Rust port of eval/eval_actionability.py (NNFT-147).

use anyhow::{Context, Result};
use clap::Parser;
use finetype_eval::csv_utils::load_csv;
use finetype_eval::taxonomy::load_format_strings;
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "eval-actionability",
    about = "FineType Actionability Evaluation"
)]
struct Args {
    #[arg(long, default_value = "eval/datasets/manifest.csv")]
    manifest: PathBuf,
    #[arg(long, default_value = "eval/eval_output/profile_results.csv")]
    predictions: PathBuf,
    #[arg(long, default_value = "labels")]
    labels_dir: PathBuf,
    #[arg(
        long,
        short,
        default_value = "eval/eval_output/actionability_results.csv"
    )]
    output: PathBuf,
}

#[derive(Clone)]
struct ActionResult {
    dataset: String,
    column_name: String,
    predicted_type: String,
    format_string: String,
    confidence: f64,
    total_values: i64,
    parse_success: i64,
    parse_fail: i64,
    success_rate: f64,
}

fn round1(v: f64) -> f64 {
    (v * 10.0).round() / 10.0
}

fn main() -> Result<()> {
    let args = Args::parse();

    let format_strings = load_format_strings(&args.labels_dir)?;
    println!("Loaded {} types with format_strings", format_strings.len());

    let predictions = load_csv(&args.predictions)?;
    println!("Loaded {} profile predictions", predictions.len());

    let manifest = {
        let rows = load_csv(&args.manifest)?;
        let mut m = std::collections::HashMap::new();
        for row in rows {
            let d = row.get("dataset").cloned().unwrap_or_default();
            let c = row.get("column_name").cloned().unwrap_or_default();
            let f = row.get("file_path").cloned().unwrap_or_default();
            m.insert((d, c), f);
        }
        m
    };
    println!("Loaded {} manifest entries", manifest.len());

    let testable = predictions
        .iter()
        .filter(|p| {
            p.get("predicted_type")
                .is_some_and(|t| format_strings.contains_key(t))
        })
        .count();
    println!("Testable predictions (have format_string): {testable}");

    // Run actionability tests via DuckDB
    let conn = duckdb::Connection::open_in_memory().context("Failed to open DuckDB")?;
    let mut results: Vec<ActionResult> = Vec::new();

    for pred in &predictions {
        let dataset = pred.get("dataset").cloned().unwrap_or_default();
        let column_name = pred.get("column_name").cloned().unwrap_or_default();
        let predicted_type = pred.get("predicted_type").cloned().unwrap_or_default();
        let confidence: f64 = pred
            .get("confidence")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.0);

        let fmt = match format_strings.get(&predicted_type) {
            Some(f) => f.clone(),
            None => continue,
        };
        let file_path = match manifest.get(&(dataset.clone(), column_name.clone())) {
            Some(p) => p.clone(),
            None => continue,
        };
        if !std::path::Path::new(&file_path).exists() {
            continue;
        }

        let fmt_escaped = fmt.replace('\'', "''");
        let col_escaped = column_name.replace('"', "\"\"");
        let file_escaped = file_path.replace('\'', "''");

        let query = format!(
            r#"SELECT count(*), count(TRY_STRPTIME("{col}", '{fmt}')), count(*) - count(TRY_STRPTIME("{col}", '{fmt}')) FROM read_csv('{file}', auto_detect=true, all_varchar=true) WHERE "{col}" IS NOT NULL AND TRIM("{col}") != ''"#,
            col = col_escaped,
            fmt = fmt_escaped,
            file = file_escaped,
        );

        match conn.query_row(&query, [], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
            ))
        }) {
            Ok((total, success, fail)) => {
                if total > 0 {
                    results.push(ActionResult {
                        dataset,
                        column_name,
                        predicted_type,
                        format_string: fmt,
                        confidence,
                        total_values: total,
                        parse_success: success,
                        parse_fail: fail,
                        success_rate: round1(success as f64 / total as f64 * 100.0),
                    });
                }
            }
            Err(e) => {
                eprintln!("Warning: DuckDB query failed: {e}");
                results.push(ActionResult {
                    dataset,
                    column_name,
                    predicted_type,
                    format_string: fmt,
                    confidence,
                    total_values: 0,
                    parse_success: 0,
                    parse_fail: 0,
                    success_rate: 0.0,
                });
            }
        }
    }

    // Print report
    println!(
        "\n{}\n          ACTIONABILITY EVALUATION (NNFT-147)\n{}\n",
        "═".repeat(70),
        "═".repeat(70)
    );
    println!("Can analysts safely TRY_CAST using FineType's format_string?");
    println!("Target: >95% success rate for datetime types\n");

    if results.is_empty() {
        println!("No datetime predictions with format_strings found.");
        return Ok(());
    }

    println!("{}", "─".repeat(70));
    println!(
        "{:<20} {:<20} {:<35} {:>8}",
        "Dataset", "Column", "Type", "Success"
    );
    println!("{}", "─".repeat(70));
    let mut sorted = results.clone();
    sorted.sort_by(|a, b| a.success_rate.partial_cmp(&b.success_rate).unwrap());
    for r in &sorted {
        let st = if r.success_rate >= 95.0 {
            "🟢"
        } else if r.success_rate >= 80.0 {
            "🟡"
        } else {
            "🔴"
        };
        let short = r
            .predicted_type
            .split('.')
            .last()
            .unwrap_or(&r.predicted_type);
        println!(
            "{:<20} {:<20} {:<35} {:>5.1}% {}",
            r.dataset, r.column_name, short, r.success_rate, st
        );
    }

    // Summary
    println!(
        "\n{}\nSummary by predicted type:\n{}",
        "─".repeat(70),
        "─".repeat(70)
    );
    let mut type_stats: BTreeMap<String, (i64, i64, usize)> = BTreeMap::new();
    for r in &results {
        let e = type_stats
            .entry(r.predicted_type.clone())
            .or_insert((0, 0, 0));
        e.0 += r.total_values;
        e.1 += r.parse_success;
        e.2 += 1;
    }
    println!(
        "{:<45} {:>5} {:>8} {:>8}",
        "Predicted Type", "Cols", "Values", "Success"
    );
    println!("{}", "─".repeat(70));
    let mut tl: Vec<_> = type_stats.iter().collect();
    tl.sort_by(|a, b| b.1 .2.cmp(&a.1 .2));
    for (t, (tot, suc, cols)) in &tl {
        let rate = if *tot > 0 {
            round1(*suc as f64 / *tot as f64 * 100.0)
        } else {
            0.0
        };
        let st = if rate >= 95.0 {
            "🟢"
        } else if rate >= 80.0 {
            "🟡"
        } else {
            "🔴"
        };
        println!("{:<45} {:>5} {:>8} {:>5.1}% {}", t, cols, tot, rate, st);
    }

    let tv: i64 = results.iter().map(|r| r.total_values).sum();
    let ts: i64 = results.iter().map(|r| r.parse_success).sum();
    let or = if tv > 0 {
        round1(ts as f64 / tv as f64 * 100.0)
    } else {
        0.0
    };
    let os = if or >= 95.0 {
        "🟢"
    } else if or >= 80.0 {
        "🟡"
    } else {
        "🔴"
    };
    println!("\nOverall: {ts}/{tv} values parsed successfully ({or}%) {os}");
    println!("Columns tested: {}", results.len());
    println!("Types tested: {}", type_stats.len());

    // Write CSV
    if !results.is_empty() {
        let mut wtr = csv::Writer::from_path(&args.output)
            .with_context(|| format!("Failed to create {}", args.output.display()))?;
        wtr.write_record([
            "dataset",
            "column_name",
            "predicted_type",
            "format_string",
            "confidence",
            "total_values",
            "parse_success",
            "parse_fail",
            "success_rate",
        ])?;
        for r in &results {
            wtr.write_record([
                &r.dataset,
                &r.column_name,
                &r.predicted_type,
                &r.format_string,
                &r.confidence.to_string(),
                &r.total_values.to_string(),
                &r.parse_success.to_string(),
                &r.parse_fail.to_string(),
                &r.success_rate.to_string(),
            ])?;
        }
        wtr.flush()?;
        println!("\nResults written to: {}", args.output.display());
    }

    Ok(())
}
