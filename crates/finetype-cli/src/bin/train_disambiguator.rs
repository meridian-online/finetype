//! Disambiguator spike: train logistic regression and MLP models (AC-4/AC-5/AC-6).
//!
//! Reads the features.csv extracted by extract-features, trains two models
//! using Candle, and reports accuracy comparisons and feature importance.
//!
//! Usage: cargo run --release --bin train-disambiguator [features.csv]

use anyhow::Result;
use candle_core::{DType, Device, Tensor};
use candle_nn::{linear, Linear, Module, Optimizer, VarBuilder, VarMap};
use csv::ReaderBuilder;
use std::collections::HashMap;

/// Number of aggregated feature columns (36 features × 4 aggregations).
const N_FEATURES: usize = 144;
/// Number of cross-validation folds.
const N_FOLDS: usize = 5;

fn main() -> Result<()> {
    let features_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "specs/2026-03-disambiguator-spike/features.csv".to_string());

    eprintln!("Reading features from: {}", features_path);
    let (samples, label_index, _index_to_label) = load_features(&features_path)?;
    let n_classes = label_index.len();
    eprintln!(
        "Loaded {} samples, {} unique predicted labels",
        samples.len(),
        n_classes
    );

    // ── Experiment 1: Logistic Regression (AC-4) ────────────────────────
    eprintln!("\n═══ Experiment 1: Logistic Regression ═══");
    let logreg_results = cross_validate(&samples, n_classes, &label_index, N_FOLDS, false)?;
    print_results("Logistic Regression", &logreg_results, &samples);

    // ── Experiment 2: MLP (AC-5) ────────────────────────────────────────
    eprintln!("\n═══ Experiment 2: MLP (1 hidden layer, 64 units) ═══");
    let mlp_results = cross_validate(&samples, n_classes, &label_index, N_FOLDS, true)?;
    print_results("MLP", &mlp_results, &samples);

    // ── Experiment 3: Feature Importance via Logistic Regression (AC-6) ──
    eprintln!("\n═══ Feature Importance (Logistic Regression weights) ═══");
    let importance = compute_feature_importance(&samples, n_classes, &label_index)?;
    print_feature_importance(&importance);

    // ── Comparison summary ──────────────────────────────────────────────
    eprintln!("\n═══ Summary ═══");
    let logreg_acc = logreg_results.iter().map(|r| r.accuracy).sum::<f32>() / N_FOLDS as f32;
    let mlp_acc = mlp_results.iter().map(|r| r.accuracy).sum::<f32>() / N_FOLDS as f32;
    eprintln!(
        "Logistic Regression: {:.1}% mean accuracy",
        logreg_acc * 100.0
    );
    eprintln!("MLP (64 hidden):     {:.1}% mean accuracy", mlp_acc * 100.0);

    // Count how many the current rules get right (predicted_label matches gt)
    let rule_correct = samples
        .iter()
        .filter(|s| labels_match(&s.predicted_label, &s.gt_label))
        .count();
    eprintln!(
        "Current rules:       {:.1}% ({}/{})",
        rule_correct as f32 / samples.len() as f32 * 100.0,
        rule_correct,
        samples.len()
    );

    Ok(())
}

#[derive(Debug, Clone)]
#[allow(dead_code)] // Spike binary — fields retained for debugging
struct Sample {
    dataset: String,
    column_name: String,
    gt_label: String,
    predicted_label: String,
    confidence: f32,
    disambiguation_rule: String,
    is_generic: bool,
    features: Vec<f32>,       // 144 aggregated features
    vote_labels: Vec<String>, // top 5 vote labels
    vote_fracs: Vec<f32>,     // top 5 vote fractions
}

#[derive(Debug)]
struct FoldResult {
    accuracy: f32,
    correct: usize,
    total: usize,
    per_sample: Vec<(usize, bool)>, // (sample_index, correct)
}

type FeatureData = (Vec<Sample>, HashMap<String, usize>, Vec<String>);

/// Load features.csv and parse into samples.
fn load_features(path: &str) -> Result<FeatureData> {
    let mut rdr = ReaderBuilder::new().from_path(path)?;
    let headers: Vec<String> = rdr.headers()?.iter().map(|h| h.to_string()).collect();

    // Find column indices
    let feat_start = headers
        .iter()
        .position(|h| h == "feat_mean_is_numeric")
        .unwrap();
    let vote_start = headers.iter().position(|h| h == "vote1_label").unwrap();

    let mut samples = Vec::new();
    let mut label_set: HashMap<String, usize> = HashMap::new();

    for record in rdr.records() {
        let record = record?;
        let gt_label = record.get(3).unwrap_or("").to_string();
        let predicted_label = record.get(4).unwrap_or("").to_string();
        let confidence: f32 = record.get(5).unwrap_or("0").parse().unwrap_or(0.0);
        let disambiguation_rule = record.get(7).unwrap_or("").to_string();
        let is_generic: bool = record.get(8).unwrap_or("false") == "true";

        // Parse features (144 values)
        let mut features = Vec::with_capacity(N_FEATURES);
        for i in feat_start..(feat_start + N_FEATURES) {
            let v: f32 = record.get(i).unwrap_or("0").parse().unwrap_or(0.0);
            features.push(v);
        }

        // Parse vote distribution
        let mut vote_labels = Vec::new();
        let mut vote_fracs = Vec::new();
        for i in 0..5 {
            let label = record.get(vote_start + i * 2).unwrap_or("").to_string();
            let frac: f32 = record
                .get(vote_start + i * 2 + 1)
                .unwrap_or("0")
                .parse()
                .unwrap_or(0.0);
            vote_labels.push(label);
            vote_fracs.push(frac);
        }

        // Build label index using predicted labels (what the model needs to output)
        if !predicted_label.is_empty() && !label_set.contains_key(&predicted_label) {
            let idx = label_set.len();
            label_set.insert(predicted_label.clone(), idx);
        }

        samples.push(Sample {
            dataset: record.get(0).unwrap_or("").to_string(),
            column_name: record.get(2).unwrap_or("").to_string(),
            gt_label,
            predicted_label,
            confidence,
            disambiguation_rule,
            is_generic,
            features,
            vote_labels,
            vote_fracs,
        });
    }

    // Also add gt_labels that aren't in predicted labels
    for s in &samples {
        if !s.gt_label.is_empty() && !label_set.contains_key(&s.gt_label) {
            let idx = label_set.len();
            label_set.insert(s.gt_label.clone(), idx);
        }
    }

    let mut index_to_label = vec![String::new(); label_set.len()];
    for (label, idx) in &label_set {
        index_to_label[*idx] = label.clone();
    }

    Ok((samples, label_set, index_to_label))
}

/// Build feature tensor and target tensor from samples.
/// Target is the gt_label mapped to index.
fn build_tensors(
    samples: &[&Sample],
    label_index: &HashMap<String, usize>,
    device: &Device,
) -> Result<(Tensor, Tensor)> {
    let n = samples.len();
    let mut features = vec![0.0f32; n * N_FEATURES];
    let mut targets = vec![0u32; n];

    for (i, s) in samples.iter().enumerate() {
        for (j, &f) in s.features.iter().enumerate() {
            features[i * N_FEATURES + j] = f;
        }
        // Map gt_label to class index
        targets[i] = *label_index.get(&s.gt_label).unwrap_or(&0) as u32;
    }

    let x = Tensor::from_vec(features, (n, N_FEATURES), device)?.to_dtype(DType::F32)?;
    let y = Tensor::from_vec(targets, n, device)?.to_dtype(DType::U32)?;

    Ok((x, y))
}

/// Train a logistic regression or MLP model and evaluate.
fn train_and_eval(
    train_samples: &[&Sample],
    test_samples: &[&Sample],
    test_indices: &[usize],
    n_classes: usize,
    label_index: &HashMap<String, usize>,
    use_mlp: bool,
) -> Result<FoldResult> {
    let device = Device::Cpu;
    let (x_train, y_train) = build_tensors(train_samples, label_index, &device)?;
    let (x_test, _y_test) = build_tensors(test_samples, label_index, &device)?;

    let varmap = VarMap::new();
    let vb = VarBuilder::from_varmap(&varmap, DType::F32, &device);

    // Build model
    let (layer1, layer2): (Linear, Option<Linear>) = if use_mlp {
        let l1 = linear(N_FEATURES, 64, vb.pp("l1"))?;
        let l2 = linear(64, n_classes, vb.pp("l2"))?;
        (l1, Some(l2))
    } else {
        let l1 = linear(N_FEATURES, n_classes, vb.pp("l1"))?;
        (l1, None)
    };

    // Training loop
    let lr = 0.01;
    let epochs = 200;
    let mut opt = candle_nn::SGD::new(varmap.all_vars(), lr)?;

    for epoch in 0..epochs {
        let logits = if let Some(ref l2) = layer2 {
            let h = layer1.forward(&x_train)?.relu()?;
            l2.forward(&h)?
        } else {
            layer1.forward(&x_train)?
        };

        let loss = candle_nn::loss::cross_entropy(&logits, &y_train)?;
        opt.backward_step(&loss)?;

        if epoch % 50 == 0 {
            let loss_val: f32 = loss.to_scalar()?;
            eprint!("  epoch {}: loss={:.4}  \r", epoch, loss_val);
        }
    }
    eprintln!();

    // Evaluate
    let test_logits = if let Some(ref l2) = layer2 {
        let h = layer1.forward(&x_test)?.relu()?;
        l2.forward(&h)?
    } else {
        layer1.forward(&x_test)?
    };

    let test_preds = test_logits.argmax(1)?;
    let preds_vec: Vec<u32> = test_preds.to_vec1()?;

    let mut correct = 0;
    let mut per_sample = Vec::new();
    for (i, s) in test_samples.iter().enumerate() {
        let pred_idx = preds_vec[i] as usize;
        let gt_idx = *label_index.get(&s.gt_label).unwrap_or(&0);
        let is_correct = pred_idx == gt_idx;
        if is_correct {
            correct += 1;
        }
        per_sample.push((test_indices[i], is_correct));
    }

    Ok(FoldResult {
        accuracy: correct as f32 / test_samples.len() as f32,
        correct,
        total: test_samples.len(),
        per_sample,
    })
}

/// K-fold cross-validation.
fn cross_validate(
    samples: &[Sample],
    n_classes: usize,
    label_index: &HashMap<String, usize>,
    k: usize,
    use_mlp: bool,
) -> Result<Vec<FoldResult>> {
    let n = samples.len();
    let fold_size = n / k;
    let mut results = Vec::new();

    for fold in 0..k {
        let test_start = fold * fold_size;
        let test_end = if fold == k - 1 {
            n
        } else {
            test_start + fold_size
        };

        let test_indices: Vec<usize> = (test_start..test_end).collect();
        let train_samples: Vec<&Sample> = (0..n)
            .filter(|i| *i < test_start || *i >= test_end)
            .map(|i| &samples[i])
            .collect();
        let test_samples: Vec<&Sample> = test_indices.iter().map(|&i| &samples[i]).collect();

        eprint!(
            "  Fold {}/{}: train={}, test={}  ",
            fold + 1,
            k,
            train_samples.len(),
            test_samples.len()
        );
        let result = train_and_eval(
            &train_samples,
            &test_samples,
            &test_indices,
            n_classes,
            label_index,
            use_mlp,
        )?;
        eprintln!(
            "  accuracy: {:.1}% ({}/{})",
            result.accuracy * 100.0,
            result.correct,
            result.total
        );
        results.push(result);
    }

    Ok(results)
}

/// Train on all data and extract feature importance from logistic regression weights.
fn compute_feature_importance(
    samples: &[Sample],
    n_classes: usize,
    label_index: &HashMap<String, usize>,
) -> Result<Vec<(String, f32)>> {
    let device = Device::Cpu;
    let all_refs: Vec<&Sample> = samples.iter().collect();
    let (x, y) = build_tensors(&all_refs, label_index, &device)?;

    let varmap = VarMap::new();
    let vb = VarBuilder::from_varmap(&varmap, DType::F32, &device);
    let layer = linear(N_FEATURES, n_classes, vb.pp("l1"))?;

    let lr = 0.01;
    let mut opt = candle_nn::SGD::new(varmap.all_vars(), lr)?;

    for epoch in 0..300 {
        let logits = layer.forward(&x)?;
        let loss = candle_nn::loss::cross_entropy(&logits, &y)?;
        opt.backward_step(&loss)?;

        if epoch % 100 == 0 {
            let loss_val: f32 = loss.to_scalar()?;
            eprint!("  epoch {}: loss={:.4}  \r", epoch, loss_val);
        }
    }
    eprintln!();

    // Extract weight matrix and compute L2 norm per feature (across all classes)
    let weights = layer.weight(); // shape: [n_classes, N_FEATURES]
    let weight_abs = weights.abs()?;
    let importance_per_feature = weight_abs.mean(0)?; // mean across classes → [N_FEATURES]
    let importance_vec: Vec<f32> = importance_per_feature.to_vec1()?;

    // Feature names (36 features × 4 aggregations)
    let feature_names = finetype_model::FEATURE_NAMES;
    let prefixes = ["mean", "var", "min", "max"];

    let mut named_importance: Vec<(String, f32)> = Vec::new();
    for (p_idx, prefix) in prefixes.iter().enumerate() {
        for (f_idx, name) in feature_names.iter().enumerate() {
            let idx = p_idx * 36 + f_idx;
            named_importance.push((format!("{}_{}", prefix, name), importance_vec[idx]));
        }
    }

    // Sort by importance descending
    named_importance.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    Ok(named_importance)
}

fn print_results(model_name: &str, results: &[FoldResult], samples: &[Sample]) {
    let total_correct: usize = results.iter().map(|r| r.correct).sum();
    let total: usize = results.iter().map(|r| r.total).sum();
    eprintln!(
        "\n{}: {:.1}% overall ({}/{})",
        model_name,
        total_correct as f32 / total as f32 * 100.0,
        total_correct,
        total
    );

    // Collect all per-sample results
    let mut all_results: Vec<(usize, bool)> = Vec::new();
    for r in results {
        all_results.extend_from_slice(&r.per_sample);
    }

    // Per-dataset accuracy
    let mut dataset_stats: HashMap<String, (usize, usize)> = HashMap::new();
    for &(idx, correct) in &all_results {
        let entry = dataset_stats
            .entry(samples[idx].dataset.clone())
            .or_default();
        entry.1 += 1; // total
        if correct {
            entry.0 += 1; // correct
        }
    }

    eprintln!("\nPer-dataset accuracy:");
    let mut datasets: Vec<_> = dataset_stats.into_iter().collect();
    datasets.sort_by(|a, b| a.0.cmp(&b.0));
    for (dataset, (correct, total)) in &datasets {
        eprintln!(
            "  {:<30} {:.1}% ({}/{})",
            dataset,
            *correct as f32 / *total as f32 * 100.0,
            correct,
            total
        );
    }

    // Print misclassified columns
    let misclassified: Vec<_> = all_results
        .iter()
        .filter(|(_, correct)| !*correct)
        .map(|(idx, _)| &samples[*idx])
        .collect();

    if !misclassified.is_empty() {
        eprintln!("\nMisclassified ({}):", misclassified.len());
        for s in &misclassified {
            eprintln!(
                "  {}.{}: gt={}, pred={}, rule={}",
                s.dataset, s.column_name, s.gt_label, s.predicted_label, s.disambiguation_rule
            );
        }
    }
}

fn print_feature_importance(importance: &[(String, f32)]) {
    eprintln!("\nTop 20 most important features:");
    for (i, (name, score)) in importance.iter().take(20).enumerate() {
        eprintln!("  {:>2}. {:<35} {:.4}", i + 1, name, score);
    }
}

/// Check if gt_label and predicted_label match.
/// Uses simple heuristic: exact match on the type part (last component).
fn labels_match(predicted: &str, gt: &str) -> bool {
    // Direct substring match of gt_label in predicted label
    let pred_type = predicted.rsplit('.').next().unwrap_or(predicted);
    let gt_lower = gt.to_lowercase().replace(' ', "_");

    // Exact type match
    if pred_type == gt_lower {
        return true;
    }

    // Common gt_label → FineType type mappings
    match gt.to_lowercase().as_str() {
        "number" | "integer" => {
            pred_type == "integer_number"
                || pred_type == "decimal_number"
                || pred_type == "increment"
        }
        "decimal number" => pred_type == "decimal_number",
        "category" | "categorical" => {
            pred_type == "categorical" || pred_type == "ordinal" || pred_type == "extension"
        }
        "code" => {
            pred_type == "alphanumeric_id"
                || pred_type == "numeric_code"
                || pred_type == "categorical"
                || pred_type == "ordinal"
        }
        "id" => {
            pred_type == "increment"
                || pred_type == "alphanumeric_id"
                || pred_type == "integer_number"
        }
        "alphanumeric id" => pred_type == "alphanumeric_id" || pred_type == "categorical",
        "boolean" => pred_type == "binary" || pred_type == "initials",
        "name" | "entity name" => {
            pred_type == "entity_name" || pred_type == "full_name" || pred_type == "categorical"
        }
        "date" => pred_type.contains("date") || pred_type.contains("iso_8601"),
        "timestamp" | "iso timestamp" | "iso timestamp milliseconds" => {
            pred_type.contains("timestamp") || pred_type.contains("iso_8601")
        }
        "time" => pred_type.contains("epoch") || pred_type.contains("time"),
        "price" | "value" => pred_type == "amount" || pred_type == "decimal_number",
        "address" => pred_type == "full_address" || pred_type == "coordinates",
        "telephone" => pred_type == "phone_number" || pred_type == "phone_e164",
        "status" => pred_type == "ordinal" || pred_type == "categorical",
        "email" => pred_type == "email",
        "url" => pred_type == "url",
        "latitude" => pred_type == "latitude",
        "longitude" => pred_type == "longitude",
        "country" => pred_type == "country",
        "country code" => pred_type == "country_code",
        "city" => pred_type == "city",
        "state" | "region" => pred_type == "state" || pred_type == "country",
        "postal code" => pred_type == "postal_code",
        "uuid" => pred_type == "uuid",
        "ip_v4" | "ipv4" => pred_type == "ip_v4",
        "percentage" => pred_type == "percentage" || pred_type == "decimal_number",
        "currency" | "currency code" => pred_type == "currency_code",
        "gender" => pred_type == "gender",
        "year" => pred_type == "year",
        "occupation" => pred_type == "categorical" || pred_type == "ordinal",
        _ => {
            // Fallback: check if gt (spaces→underscores) matches pred type
            gt_lower == pred_type
        }
    }
}
