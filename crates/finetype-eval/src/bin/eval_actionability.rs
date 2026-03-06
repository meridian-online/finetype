//! FineType Actionability Evaluation (NNFT-184, NNFT-205)
//!
//! Tests whether FineType's type predictions produce working DuckDB transforms.
//!
//! **Tier A (strptime):** Types with `format_string` — tested via TRY_STRPTIME.
//! **Tier B (transform):** Types with `transform` but no `format_string` — tested
//! by executing the transform SQL (with CAST replaced by TRY_CAST).

use anyhow::{Context, Result};
use clap::Parser;
use finetype_eval::csv_utils::load_csv;
use finetype_eval::taxonomy::{load_format_strings, load_transforms};
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
    eval_method: String,
    format_or_transform: String,
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
    let transforms = load_transforms(&args.labels_dir)?;
    println!(
        "Loaded {} types with format_string (Tier A), {} with transform-only (Tier B)",
        format_strings.len(),
        transforms.len()
    );

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

    let testable_a = predictions
        .iter()
        .filter(|p| {
            p.get("predicted_type")
                .is_some_and(|t| format_strings.contains_key(t))
        })
        .count();
    let testable_b = predictions
        .iter()
        .filter(|p| {
            p.get("predicted_type")
                .is_some_and(|t| !format_strings.contains_key(t) && transforms.contains_key(t))
        })
        .count();
    println!("Testable: {testable_a} strptime (Tier A), {testable_b} transform (Tier B)");

    let conn = duckdb::Connection::open_in_memory().context("Failed to open DuckDB")?;
    let mut results: Vec<ActionResult> = Vec::new();

    // ── Tier A: strptime-based types ─────────────────────────────────────
    for pred in &predictions {
        let dataset = pred.get("dataset").cloned().unwrap_or_default();
        let column_name = pred.get("column_name").cloned().unwrap_or_default();
        let predicted_type = pred.get("predicted_type").cloned().unwrap_or_default();
        let confidence: f64 = pred
            .get("confidence")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.0);

        let fmts = match format_strings.get(&predicted_type) {
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

        let col_escaped = column_name.replace('"', "\"\"");
        let file_escaped = file_path.replace('\'', "''");

        let mut best: Option<ActionResult> = None;
        for fmt in &fmts {
            let fmt_escaped = fmt.replace('\'', "''");
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
                        let rate = round1(success as f64 / total as f64 * 100.0);
                        let candidate = ActionResult {
                            dataset: dataset.clone(),
                            column_name: column_name.clone(),
                            predicted_type: predicted_type.clone(),
                            eval_method: "strptime".to_string(),
                            format_or_transform: fmt.clone(),
                            confidence,
                            total_values: total,
                            parse_success: success,
                            parse_fail: fail,
                            success_rate: rate,
                        };
                        if best.as_ref().is_none_or(|b| rate > b.success_rate) {
                            best = Some(candidate);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Warning: strptime query failed for '{}': {}", fmt, e);
                }
            }
        }

        if let Some(result) = best {
            results.push(result);
        } else {
            results.push(ActionResult {
                dataset,
                column_name,
                predicted_type,
                eval_method: "strptime".to_string(),
                format_or_transform: fmts.first().cloned().unwrap_or_default(),
                confidence,
                total_values: 0,
                parse_success: 0,
                parse_fail: 0,
                success_rate: 0.0,
            });
        }
    }

    // ── Tier B: transform-based types ────────────────────────────────────
    for pred in &predictions {
        let dataset = pred.get("dataset").cloned().unwrap_or_default();
        let column_name = pred.get("column_name").cloned().unwrap_or_default();
        let predicted_type = pred.get("predicted_type").cloned().unwrap_or_default();
        let confidence: f64 = pred
            .get("confidence")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.0);

        // Skip types already handled by Tier A
        if format_strings.contains_key(&predicted_type) {
            continue;
        }
        let transform = match transforms.get(&predicted_type) {
            Some(t) => t.clone(),
            None => continue,
        };
        let file_path = match manifest.get(&(dataset.clone(), column_name.clone())) {
            Some(p) => p.clone(),
            None => continue,
        };
        if !std::path::Path::new(&file_path).exists() {
            continue;
        }

        let col_escaped = column_name.replace('"', "\"\"");
        let file_escaped = file_path.replace('\'', "''");

        // Substitute {col} with the actual column reference and replace CAST→TRY_CAST
        let col_ref = format!(r#""{col}""#, col = col_escaped);
        let transform_sql = transform.replace("{col}", &col_ref);
        let try_transform_sql = transform_sql.replace("CAST(", "TRY_CAST(");

        let query = format!(
            r#"SELECT count(*), count({expr}), count(*) - count({expr}) FROM read_csv('{file}', auto_detect=true, all_varchar=true) WHERE "{col}" IS NOT NULL AND TRIM("{col}") != ''"#,
            expr = try_transform_sql,
            file = file_escaped,
            col = col_escaped,
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
                    let rate = round1(success as f64 / total as f64 * 100.0);
                    results.push(ActionResult {
                        dataset,
                        column_name,
                        predicted_type,
                        eval_method: "transform".to_string(),
                        format_or_transform: transform,
                        confidence,
                        total_values: total,
                        parse_success: success,
                        parse_fail: fail,
                        success_rate: rate,
                    });
                }
            }
            Err(e) => {
                eprintln!(
                    "Warning: transform query failed for {}.{} ({}): {}",
                    dataset, column_name, predicted_type, e
                );
                results.push(ActionResult {
                    dataset,
                    column_name,
                    predicted_type,
                    eval_method: "transform".to_string(),
                    format_or_transform: transform,
                    confidence,
                    total_values: 0,
                    parse_success: 0,
                    parse_fail: 0,
                    success_rate: 0.0,
                });
            }
        }
    }

    // ── Report ───────────────────────────────────────────────────────────
    println!(
        "\n{}\n          ACTIONABILITY EVALUATION\n{}\n",
        "═".repeat(70),
        "═".repeat(70)
    );
    println!("Can analysts safely use FineType's transforms on real data?");
    println!("Target: >95% success rate\n");

    if results.is_empty() {
        println!("No testable predictions found.");
        return Ok(());
    }

    // Split results by method
    let tier_a: Vec<_> = results
        .iter()
        .filter(|r| r.eval_method == "strptime")
        .collect();
    let tier_b: Vec<_> = results
        .iter()
        .filter(|r| r.eval_method == "transform")
        .collect();

    // Print Tier A
    if !tier_a.is_empty() {
        println!(
            "{}\nTier A: strptime-based ({} columns)\n{}",
            "─".repeat(70),
            tier_a.len(),
            "─".repeat(70)
        );
        print_detail_table(&tier_a);
    }

    // Print Tier B
    if !tier_b.is_empty() {
        println!(
            "\n{}\nTier B: transform-based ({} columns)\n{}",
            "─".repeat(70),
            tier_b.len(),
            "─".repeat(70)
        );
        print_detail_table(&tier_b);
    }

    // Summary by predicted type (combined)
    println!(
        "\n{}\nSummary by predicted type:\n{}",
        "─".repeat(70),
        "─".repeat(70)
    );
    let mut type_stats: BTreeMap<String, (i64, i64, usize, String)> = BTreeMap::new();
    for r in &results {
        let e =
            type_stats
                .entry(r.predicted_type.clone())
                .or_insert((0, 0, 0, r.eval_method.clone()));
        e.0 += r.total_values;
        e.1 += r.parse_success;
        e.2 += 1;
    }
    println!(
        "{:<45} {:>5} {:>8} {:>8} {:>6}",
        "Predicted Type", "Cols", "Values", "Success", "Method"
    );
    println!("{}", "─".repeat(80));
    let mut tl: Vec<_> = type_stats.iter().collect();
    tl.sort_by(|a, b| b.1 .2.cmp(&a.1 .2));
    for (t, (tot, suc, cols, method)) in &tl {
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
        let m = if method == "strptime" { "A" } else { "B" };
        println!(
            "{:<45} {:>5} {:>8} {:>5.1}% {} {:>4}",
            t, cols, tot, rate, st, m
        );
    }

    // Tier-specific summaries
    print_tier_summary("Tier A (strptime)", &tier_a);
    print_tier_summary("Tier B (transform)", &tier_b);

    // Overall combined metric
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
    println!("\nOverall: {ts}/{tv} values transformed successfully ({or}%) {os}");
    println!(
        "Columns tested: {} ({} strptime + {} transform)",
        results.len(),
        tier_a.len(),
        tier_b.len()
    );
    println!("Types tested: {}", type_stats.len());

    // Write CSV
    if !results.is_empty() {
        let mut wtr = csv::Writer::from_path(&args.output)
            .with_context(|| format!("Failed to create {}", args.output.display()))?;
        wtr.write_record([
            "dataset",
            "column_name",
            "predicted_type",
            "eval_method",
            "format_or_transform",
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
                &r.eval_method,
                &r.format_or_transform,
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

fn print_detail_table(results: &[&ActionResult]) {
    println!(
        "{:<20} {:<20} {:<35} {:>8}",
        "Dataset", "Column", "Type", "Success"
    );
    println!("{}", "─".repeat(70));
    let mut sorted: Vec<_> = results.to_vec();
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
}

fn print_tier_summary(label: &str, results: &[&ActionResult]) {
    if results.is_empty() {
        return;
    }
    let tv: i64 = results.iter().map(|r| r.total_values).sum();
    let ts: i64 = results.iter().map(|r| r.parse_success).sum();
    let rate = if tv > 0 {
        round1(ts as f64 / tv as f64 * 100.0)
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
    println!(
        "\n{}: {}/{} ({:.1}%) {} [{} columns]",
        label,
        ts,
        tv,
        rate,
        st,
        results.len()
    );
}
