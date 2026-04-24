//! amvg_topk — dump the v16 multi-branch model's top-k predictions for a
//! specified column of a CSV file. Used by scripts/amvg/ac04_confidence.py to
//! build the per-subtype confidence distribution (11 subtypes × top-5).
//!
//! Usage:
//!   cargo run --example amvg_topk -- <csv_path> <column_name> <model_dir> <k> [sample_size]
//!
//! Prints k lines of the form: `<rank>\t<label>\t<probability>` to stdout.
//! Exit 0 on success; prints errors to stderr.
//!
//! This binary hits MultiBranchClassifier::classify_column_topk directly —
//! bypassing the Sharpen post-processing layer — so the probabilities are raw
//! softmax from the model and ac-04's consistency checks measure model state,
//! not downstream rule state.

use std::env;
use std::path::PathBuf;

use finetype_core::taxonomy::Taxonomy;
use finetype_model::multi_branch::MultiBranchClassifier;

fn main() {
    let args: Vec<String> = env::args().collect();
    if !(5..=6).contains(&args.len()) {
        eprintln!("usage: amvg_topk <csv> <column> <model_dir> <k> [sample_size]");
        std::process::exit(2);
    }
    let csv = PathBuf::from(&args[1]);
    let column = &args[2];
    let model_dir = PathBuf::from(&args[3]);
    let k: usize = args[4].parse().expect("k must be a non-negative integer");
    let sample_size: usize = args
        .get(5)
        .map(|s| s.parse().expect("sample_size must be integer"))
        .unwrap_or(100);

    // Load CSV, extract the named column
    let mut reader = csv::Reader::from_path(&csv)
        .unwrap_or_else(|e| panic!("open csv {}: {e}", csv.display()));
    let headers = reader
        .headers()
        .expect("csv headers")
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();
    let col_idx = headers
        .iter()
        .position(|h| h == column)
        .unwrap_or_else(|| panic!("column {column} not found in csv"));
    let mut values: Vec<String> = Vec::new();
    for rec in reader.records() {
        let rec = rec.expect("csv record");
        if let Some(v) = rec.get(col_idx) {
            if !v.is_empty() {
                values.push(v.to_string());
            }
        }
        if values.len() >= sample_size {
            break;
        }
    }
    if values.is_empty() {
        eprintln!("no values in column {column}");
        std::process::exit(1);
    }

    // Load model + taxonomy (for validation branch)
    let classifier = MultiBranchClassifier::load(&model_dir)
        .unwrap_or_else(|e| panic!("load model {}: {e}", model_dir.display()));

    // labels/ lives at repo root; finetype-model manifest dir is crates/finetype-model
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let labels_dir = PathBuf::from(manifest_dir)
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.join("labels"))
        .expect("resolve labels dir");
    let taxonomy = Taxonomy::from_directory(&labels_dir)
        .unwrap_or_else(|e| panic!("load taxonomy: {e}"));

    // Run top-k with the column header (not empty) — matches production path
    let topk = classifier
        .classify_column_topk(&values, column, Some(&taxonomy), k)
        .unwrap_or_else(|e| panic!("topk inference: {e}"));

    for (rank, (label, prob)) in topk.iter().enumerate() {
        println!("{}\t{}\t{:.6}", rank + 1, label, prob);
    }
}
