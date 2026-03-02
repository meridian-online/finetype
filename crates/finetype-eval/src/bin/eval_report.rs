//! FineType Evaluation Report Generator (NNFT-184)
//!
//! Generates eval/eval_output/report.md — a unified markdown dashboard.
//! Rust port of eval/eval_report.py (NNFT-147).

use anyhow::{Context, Result};
use chrono::Local;
use clap::Parser;
use finetype_eval::csv_utils::load_csv;
use finetype_eval::matching::{is_domain_match, is_label_match};
use finetype_eval::taxonomy::load_taxonomy_stats;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "eval-report", about = "FineType Evaluation Report Generator")]
struct Args {
    #[arg(long, default_value = "eval/eval_output/profile_results.csv")]
    profile_results: PathBuf,
    #[arg(long, default_value = "eval/eval_output/ground_truth.csv")]
    ground_truth: PathBuf,
    #[arg(long, default_value = "eval/schema_mapping.csv")]
    schema_mapping: PathBuf,
    #[arg(long, default_value = "eval/eval_output/actionability_results.csv")]
    actionability_results: PathBuf,
    #[arg(long, default_value = "labels")]
    labels_dir: PathBuf,
    #[arg(long, short, default_value = "eval/eval_output/report.md")]
    output: PathBuf,
}

struct Miss {
    dataset: String,
    column: String,
    predicted: String,
    expected: String,
    gt_label: String,
    confidence: f64,
}

fn round1(v: f64) -> f64 {
    (v * 10.0).round() / 10.0
}

fn status_emoji(value: f64, green: f64, yellow: f64) -> &'static str {
    if value >= green {
        "🟢"
    } else if value >= yellow {
        "🟡"
    } else {
        "🔴"
    }
}

fn main() -> Result<()> {
    let args = Args::parse();

    let predictions = load_csv(&args.profile_results)?;
    let ground_truth = load_csv(&args.ground_truth)?;
    let schema_mapping = {
        let rows = load_csv(&args.schema_mapping)?;
        let mut m = HashMap::new();
        for row in rows {
            if let Some(k) = row.get("gt_label") {
                m.insert(k.clone(), row);
            }
        }
        m
    };
    let actionability = load_csv(&args.actionability_results)?;
    let taxonomy_stats = load_taxonomy_stats(&args.labels_dir)?;

    println!(
        "Loaded {} predictions, {} ground truth, {} mappings, {} actionability results",
        predictions.len(),
        ground_truth.len(),
        schema_mapping.len(),
        actionability.len()
    );

    // Build GT lookup
    let mut gt_lookup: HashMap<(String, String), String> = HashMap::new();
    for row in &ground_truth {
        let d = row.get("dataset").cloned().unwrap_or_default();
        let c = row.get("column_name").cloned().unwrap_or_default();
        let l = row.get("gt_label").cloned().unwrap_or_default();
        gt_lookup.insert((d, c), l);
    }

    // Compute profile accuracy
    let mut total = 0usize;
    let mut label_correct = 0usize;
    let mut domain_correct = 0usize;
    let mut datasets_seen = std::collections::HashSet::new();
    let mut misses: Vec<Miss> = Vec::new();
    let mut type_counts: HashMap<String, (usize, usize)> = HashMap::new();

    for pred in &predictions {
        let dataset = pred.get("dataset").cloned().unwrap_or_default();
        let column_name = pred.get("column_name").cloned().unwrap_or_default();
        let gt_label = match gt_lookup.get(&(dataset.clone(), column_name.clone())) {
            Some(l) => l.clone(),
            None => continue,
        };
        let mapping = match schema_mapping.get(&gt_label) {
            Some(m) => m,
            None => continue,
        };
        let mq = mapping
            .get("match_quality")
            .map(|s| s.as_str())
            .unwrap_or("");
        if mq != "direct" && mq != "close" {
            continue;
        }

        total += 1;
        datasets_seen.insert(dataset.clone());
        let predicted = pred.get("predicted_type").cloned().unwrap_or_default();
        let expected_label = mapping.get("finetype_label").cloned().unwrap_or_default();
        let expected_domain = mapping.get("finetype_domain").cloned().unwrap_or_default();

        let lok = is_label_match(&predicted, &expected_label);
        if lok {
            label_correct += 1;
        }
        if is_domain_match(&predicted, &expected_label, &expected_domain) {
            domain_correct += 1;
        }

        // Precision tracking
        let e = type_counts.entry(predicted.clone()).or_insert((0, 0));
        e.0 += 1;
        if lok {
            e.1 += 1;
        }

        if !lok {
            let conf: f64 = pred
                .get("confidence")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.0);
            misses.push(Miss {
                dataset,
                column: column_name,
                predicted,
                expected: expected_label,
                gt_label,
                confidence: conf,
            });
        }
    }

    misses.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());

    let label_acc = if total > 0 {
        round1(label_correct as f64 / total as f64 * 100.0)
    } else {
        0.0
    };
    let domain_acc = if total > 0 {
        round1(domain_correct as f64 / total as f64 * 100.0)
    } else {
        0.0
    };
    let n_datasets = datasets_seen.len();

    println!("Profile: {label_correct}/{total} ({label_acc}% label, {domain_acc}% domain)");

    // Precision by type (sorted by predicted count desc)
    let mut precision_list: Vec<_> = type_counts.into_iter().collect();
    precision_list.sort_by(|a, b| b.1 .0.cmp(&a.1 .0));

    // Generate report
    let now = Local::now().format("%Y-%m-%d %H:%M").to_string();
    let mut w: Vec<String> = Vec::new();
    let p = |w: &mut Vec<String>, s: &str| w.push(s.to_string());

    p(&mut w, "# FineType Evaluation Report");
    p(&mut w, "");
    w.push(format!("**Generated:** {now}"));
    p(&mut w, "");

    // Headline
    p(&mut w, "## Headline Metrics");
    p(&mut w, "");
    let ls = status_emoji(label_acc, 90.0, 80.0);
    let ds = status_emoji(domain_acc, 95.0, 90.0);
    p(&mut w, "| Metric | Value | Status |");
    p(&mut w, "|---|---|---|");
    w.push(format!(
        "| Profile label accuracy | {label_correct}/{total} ({label_acc}%) | {ls} |"
    ));
    w.push(format!(
        "| Profile domain accuracy | {domain_correct}/{total} ({domain_acc}%) | {ds} |"
    ));

    if !actionability.is_empty() {
        let tv: i64 = actionability
            .iter()
            .map(|r| {
                r.get("total_values")
                    .and_then(|s| s.parse::<i64>().ok())
                    .unwrap_or(0)
            })
            .sum();
        let sv: i64 = actionability
            .iter()
            .map(|r| {
                r.get("parse_success")
                    .and_then(|s| s.parse::<i64>().ok())
                    .unwrap_or(0)
            })
            .sum();
        let ar = if tv > 0 {
            round1(sv as f64 / tv as f64 * 100.0)
        } else {
            0.0
        };
        let as_ = status_emoji(ar, 95.0, 80.0);
        let cp = actionability
            .iter()
            .filter(|r| {
                r.get("success_rate")
                    .and_then(|s| s.parse::<f64>().ok())
                    .unwrap_or(0.0)
                    >= 95.0
            })
            .count();
        w.push(format!(
            "| Actionability (datetime) | {sv}/{tv} ({ar}%) | {as_} |"
        ));
        w.push(format!(
            "| Columns with >95% parse rate | {cp}/{} | |",
            actionability.len()
        ));
    }

    let ts = &taxonomy_stats;
    w.push(format!("| Taxonomy types | {} | |", ts.total_types));
    w.push(format!(
        "| Types with format_string | {} | |",
        ts.with_format_string
    ));
    w.push(format!(
        "| Types with validation | {} | |",
        ts.with_validation
    ));
    w.push(format!(
        "| Types with locale validation | {} | |",
        ts.with_locale_validation
    ));
    p(&mut w, "");

    // Taxonomy coverage
    p(&mut w, "## Taxonomy Coverage");
    p(&mut w, "");
    p(&mut w, "| Domain | Types |");
    p(&mut w, "|---|---|");
    for (domain, count) in &ts.domains {
        w.push(format!("| {domain} | {count} |"));
    }
    p(&mut w, "");

    // Profile eval detail
    p(&mut w, "## Profile Evaluation");
    p(&mut w, "");
    w.push(format!(
        "**Label accuracy:** {label_correct}/{total} ({label_acc}%)"
    ));
    w.push(format!(
        "**Domain accuracy:** {domain_correct}/{total} ({domain_acc}%)"
    ));
    p(&mut w, "");

    if !misses.is_empty() {
        p(&mut w, "### Misclassifications");
        p(&mut w, "");
        p(
            &mut w,
            "| Dataset | Column | Predicted | Expected | Confidence |",
        );
        p(&mut w, "|---|---|---|---|---|");
        for m in &misses {
            let ps = m.predicted.split('.').last().unwrap_or(&m.predicted);
            let es = if !m.expected.is_empty() {
                m.expected.split('.').last().unwrap_or(&m.expected)
            } else {
                &m.gt_label
            };
            w.push(format!(
                "| {} | {} | {} | {} | {:.2} |",
                m.dataset, m.column, ps, es, m.confidence
            ));
        }
        p(&mut w, "");
    }

    // Precision per type
    if !precision_list.is_empty() {
        p(&mut w, "## Precision Per Type (Profile Eval)");
        p(&mut w, "");
        p(
            &mut w,
            "| Predicted Type | Predicted | Correct | Precision | Status |",
        );
        p(&mut w, "|---|---|---|---|---|");
        for (ptype, (predicted, correct)) in &precision_list {
            let prec = if *predicted > 0 {
                round1(*correct as f64 / *predicted as f64 * 100.0)
            } else {
                0.0
            };
            let st = if *correct as f64 >= *predicted as f64 * 0.95 {
                "🟢"
            } else if *correct as f64 >= *predicted as f64 * 0.80 {
                "🟡"
            } else {
                "🔴"
            };
            let short = ptype.split('.').last().unwrap_or(ptype);
            w.push(format!(
                "| {short} | {predicted} | {correct} | {prec}% | {st} |"
            ));
        }
        p(&mut w, "");
    }

    // Actionability detail
    if !actionability.is_empty() {
        p(&mut w, "## Actionability Evaluation");
        p(&mut w, "");
        p(
            &mut w,
            "Can analysts safely TRY_CAST using FineType's format_string predictions?",
        );
        p(&mut w, "**Target:** >95% success rate for datetime types");
        p(&mut w, "");

        // Summary by type
        let mut type_stats: std::collections::BTreeMap<String, (i64, i64, usize)> =
            std::collections::BTreeMap::new();
        for r in &actionability {
            let t = r.get("predicted_type").cloned().unwrap_or_default();
            let e = type_stats.entry(t).or_insert((0, 0, 0));
            e.0 += r
                .get("total_values")
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(0);
            e.1 += r
                .get("parse_success")
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(0);
            e.2 += 1;
        }
        let mut tl: Vec<_> = type_stats.iter().collect();
        tl.sort_by(|a, b| b.1 .2.cmp(&a.1 .2));

        p(&mut w, "### By Type");
        p(&mut w, "");
        p(
            &mut w,
            "| Type | Columns | Values | Success Rate | Status |",
        );
        p(&mut w, "|---|---|---|---|---|");
        for (t, (tot, suc, cols)) in &tl {
            let rate = if *tot > 0 {
                round1(*suc as f64 / *tot as f64 * 100.0)
            } else {
                0.0
            };
            let st = status_emoji(rate, 95.0, 80.0);
            let short = t.split('.').last().unwrap_or(t);
            w.push(format!("| {short} | {cols} | {tot} | {rate}% | {st} |"));
        }
        p(&mut w, "");

        // Failures
        let failures: Vec<_> = actionability
            .iter()
            .filter(|r| {
                r.get("success_rate")
                    .and_then(|s| s.parse::<f64>().ok())
                    .unwrap_or(0.0)
                    < 95.0
            })
            .collect();
        if !failures.is_empty() {
            p(&mut w, "### Below Target (<95%)");
            p(&mut w, "");
            p(
                &mut w,
                "| Dataset | Column | Type | Format | Success Rate |",
            );
            p(&mut w, "|---|---|---|---|---|");
            let mut sf = failures;
            sf.sort_by(|a, b| {
                let ra: f64 = a
                    .get("success_rate")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0.0);
                let rb: f64 = b
                    .get("success_rate")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0.0);
                ra.partial_cmp(&rb).unwrap()
            });
            for r in &sf {
                let st = r
                    .get("predicted_type")
                    .map(|s| s.split('.').last().unwrap_or(s))
                    .unwrap_or("");
                let d = r.get("dataset").map(|s| s.as_str()).unwrap_or("");
                let c = r.get("column_name").map(|s| s.as_str()).unwrap_or("");
                let f = r.get("format_string").map(|s| s.as_str()).unwrap_or("");
                let sr = r.get("success_rate").map(|s| s.as_str()).unwrap_or("0");
                w.push(format!("| {d} | {c} | {st} | `{f}` | {sr}% 🔴 |"));
            }
            p(&mut w, "");
        }
    }

    // Eval components
    p(&mut w, "## Evaluation Components");
    p(&mut w, "");
    p(&mut w, "| Component | Scope | Target | Status |");
    p(&mut w, "|---|---|---|---|");
    w.push(format!(
        "| Profile regression | {total} columns, {n_datasets} datasets | No regressions | {ls} |"
    ));
    p(
        &mut w,
        "| Precision per type | SOTAB/GitTables | 🟢≥95% per type | Run `make eval-sotab-cli` |",
    );
    p(
        &mut w,
        "| Overcall analysis | SOTAB/GitTables | <5% FP rate | Run `make eval-sotab-cli` |",
    );
    let al = if !actionability.is_empty() {
        let tv: i64 = actionability
            .iter()
            .map(|r| {
                r.get("total_values")
                    .and_then(|s| s.parse::<i64>().ok())
                    .unwrap_or(0)
            })
            .sum();
        let sv: i64 = actionability
            .iter()
            .map(|r| {
                r.get("parse_success")
                    .and_then(|s| s.parse::<i64>().ok())
                    .unwrap_or(0)
            })
            .sum();
        let ar = if tv > 0 {
            round1(sv as f64 / tv as f64 * 100.0)
        } else {
            0.0
        };
        status_emoji(ar, 95.0, 80.0).to_string()
    } else {
        "Not run".to_string()
    };
    w.push(format!(
        "| Actionability | Profile eval datetime | >95% parse rate | {al} |"
    ));
    p(
        &mut w,
        "| Confidence calibration | SOTAB/GitTables | Gap <10pp | Run `make eval-sotab-cli` |",
    );
    p(
        &mut w,
        "| Domain accuracy | SOTAB format-detectable | >80% | Run `make eval-sotab-cli` |",
    );
    p(&mut w, "");
    p(&mut w, "---");
    p(
        &mut w,
        "*Generated by eval-report (NNFT-184, Rust port of eval_report.py)*",
    );
    p(&mut w, "");

    let report = w.join("\n");

    if let Some(parent) = args.output.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&args.output, &report)
        .with_context(|| format!("Failed to write report: {}", args.output.display()))?;
    println!("Report written to: {}", args.output.display());

    Ok(())
}
