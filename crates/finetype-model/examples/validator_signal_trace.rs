//! Discovery instrument for spec
//! `orbit/specs/2026-04-21-validator-signal-attribution/`.
//!
//! Loads `models/default` (sherlock-v16) + taxonomy, then for each of four
//! target eval columns reports:
//!
//! 1. The 240-dim validation pass-rate vector's top-10 types and the rank /
//!    value for the expected (ground-truth) type.
//! 2. The multi-branch classification with validation branch intact.
//! 3. The same classification with validation branch ablated (`taxonomy = None`,
//!    which triggers the silent-zeros path in `compute_validation_tensor`).
//! 4. An attribution verdict derived from the two predictions.
//!
//! Run with:
//!
//! ```text
//! cargo run -p finetype-model --example validator_signal_trace --release
//! ```
//!
//! The binary is intentionally scoped to this one investigation. It is checked
//! in for reproducibility, not shipped as a public API. If the pattern proves
//! reusable, a follow-up spec can promote it to `finetype profile --debug-validation`.

use anyhow::{Context, Result};
use finetype_core::Taxonomy;
use finetype_model::{MultiBranchClassifier, ValidationFeatureExtractor};
use std::path::PathBuf;

struct Target {
    dataset: &'static str,
    column: &'static str,
    expected: &'static str,
    values: Vec<String>,
}

fn main() -> Result<()> {
    let manifest_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // crates/finetype-model → project root is two levels up
    let root = manifest_root
        .parent()
        .and_then(|p| p.parent())
        .context("could not resolve project root from CARGO_MANIFEST_DIR")?;
    let model_dir = root.join("models/default");
    let labels_dir = root.join("labels");

    eprintln!("Loading model from {}", model_dir.display());
    let mb = MultiBranchClassifier::load(&model_dir)
        .with_context(|| format!("failed to load model from {}", model_dir.display()))?;

    eprintln!("Loading taxonomy from {}", labels_dir.display());
    let mut taxonomy = Taxonomy::from_directory(&labels_dir)
        .with_context(|| format!("failed to load taxonomy from {}", labels_dir.display()))?;
    taxonomy.compile_validators();

    let extractor = ValidationFeatureExtractor::new(&taxonomy);
    let keys = extractor.type_keys().to_vec();

    let targets = build_targets();

    for t in &targets {
        println!("============================================================");
        println!("  {}/{}", t.dataset, t.column);
        println!("  expected: {}", t.expected);
        let preview: Vec<&str> = t.values.iter().take(6).map(|s| s.as_str()).collect();
        println!("  values  : {} ({} items)", preview.join(", "), t.values.len());
        println!("------------------------------------------------------------");

        let value_refs: Vec<&str> = t.values.iter().map(|s| s.as_str()).collect();
        let features = extractor.extract(&value_refs, &taxonomy);

        let mut indexed: Vec<(usize, f32)> = features
            .iter()
            .enumerate()
            .map(|(i, &v)| (i, v))
            .collect();
        indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        println!("  top-10 validation pass rates:");
        for (idx, rate) in indexed.iter().take(10) {
            let marker = if keys[*idx] == t.expected {
                " <-- expected"
            } else {
                ""
            };
            println!("    {:>5.3}  {}{}", rate, keys[*idx], marker);
        }

        if let Some(expected_idx) = extractor.index_of(t.expected) {
            let rank = indexed
                .iter()
                .position(|(idx, _)| *idx == expected_idx)
                .map(|r| r + 1)
                .unwrap_or(0);
            println!(
                "  expected type pass rate: {:.3} (rank: {} of {})",
                features[expected_idx],
                rank,
                keys.len()
            );
        } else {
            println!(
                "  expected type '{}' NOT FOUND in taxonomy index",
                t.expected
            );
        }

        // Classify with and without the validation branch
        let (normal_label, normal_conf) =
            mb.classify_column(&t.values, t.column, Some(&taxonomy))?;
        let (ablated_label, ablated_conf) = mb.classify_column(&t.values, t.column, None)?;

        println!(
            "  prediction (normal)  : {} ({:.3})",
            normal_label, normal_conf
        );
        println!(
            "  prediction (ablated) : {} ({:.3})  [validation branch zeroed]",
            ablated_label, ablated_conf
        );

        let normal_correct = normal_label == t.expected;
        let ablated_correct = ablated_label == t.expected;
        let verdict = match (normal_correct, ablated_correct) {
            (true, true) => "BOTH CORRECT (validation branch not needed here)",
            (true, false) => "BRANCH RESCUES (validation is the deciding signal)",
            (false, true) => "BRANCH HARMS (zero beats signal — unexpected)",
            (false, false) if normal_label == ablated_label => {
                "BOTH WRONG, SAME LABEL (validation branch has no effect)"
            }
            (false, false) => "BOTH WRONG, DIFFERENT LABELS (branch shifts but doesn't fix)",
        };
        println!("  attribution          : {}", verdict);
        println!();
    }

    Ok(())
}

fn build_targets() -> Vec<Target> {
    vec![
        Target {
            dataset: "coverage_closure_phase_ab",
            column: "http_method",
            expected: "technology.internet.http_method",
            values: ["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
        },
        Target {
            dataset: "coverage_closure_phase_ab",
            column: "excel_format",
            expected: "representation.file.excel_format",
            values: ["xlsx", "xls", "csv", "ods", "xlsm", "xlsb"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
        },
        Target {
            dataset: "geography_data",
            column: "country_code",
            expected: "geography.location.country_code",
            values: ["DE", "IN", "CA", "DE", "AE", "IN", "AU", "GB", "JP", "US"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
        },
        Target {
            dataset: "people_directory",
            column: "email",
            expected: "identity.person.email",
            values: [
                "william.jones34@outlook.com",
                "barbara.jones54@company.com",
                "sarah.williams38@gmail.com",
                "susan.garcia48@company.com",
                "mary.williams18@gmail.com",
                "david.brown98@gmail.com",
                "mary.rodriguez26@outlook.com",
                "barbara.williams9@gmail.com",
                "richard.smith30@gmail.com",
                "mary.martinez7@outlook.com",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
        },
    ]
}
