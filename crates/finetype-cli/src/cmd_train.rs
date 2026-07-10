//! `cmd_train` — extracted from main.rs (mechanical split, no behaviour change).

use super::*;

/// Train a multi-branch Sherlock-style model from FTMB feature-vector data.
#[cfg(feature = "train")]
#[allow(clippy::too_many_arguments)]
pub(crate) fn cmd_train_multi_branch(
    data: PathBuf,
    output: PathBuf,
    epochs: usize,
    batch_size: usize,
    lr: f64,
    weight_decay: f64,
    dropout: f32,
    seed: u64,
    head: String,
    patience: usize,
    logit_adjust_tau: f64,
    taxonomy: PathBuf,
    val_split: f32,
    no_tui: bool,
    model_config: Option<PathBuf>,
    value_encoder: Option<PathBuf>,
    cede_labels: Option<PathBuf>,
) -> Result<()> {
    use finetype_model::model2vec_shared::Model2VecResources;
    use finetype_train::multi_branch::{
        read_training_data, train_multi_branch, HeadType, MultiBranchConfig, MultiBranchDataset,
        MultiBranchTrainConfig,
    };
    use finetype_train::tui::{LogRenderer, TrainingRenderer};
    use rand::rngs::StdRng;
    use rand::seq::SliceRandom;
    use rand::SeedableRng;

    let head_type = match head.as_str() {
        "flat" => HeadType::Flat,
        "hierarchical" => HeadType::Hierarchical,
        _ => anyhow::bail!(
            "Unknown head type '{}'. Use 'flat' or 'hierarchical'.",
            head
        ),
    };

    // Load taxonomy to get sorted labels
    let taxonomy = Taxonomy::from_directory(&taxonomy)?;

    // Reshape cede-list (spec 2026-06-27-model-label-space-reshape ac-1): leaves to
    // DENY from the output label space. They are removed from labels_list/label_to_idx
    // here, so the softmax head shrinks and the existing record-filter below drops
    // their training rows for free (their label is no longer in label_to_idx). The
    // validation branch (valid_dim, one feature per taxonomy type) is unaffected.
    let cede_set: std::collections::HashSet<String> = match &cede_labels {
        Some(path) => {
            let txt = std::fs::read_to_string(path)?;
            txt.lines()
                .map(|l| l.split('#').next().unwrap_or("").trim())
                .filter(|l| !l.is_empty())
                .map(|l| l.to_string())
                .collect()
        }
        None => std::collections::HashSet::new(),
    };

    let labels_list: Vec<String> = taxonomy
        .labels()
        .iter()
        .filter(|l| !cede_set.contains(*l))
        .cloned()
        .collect();
    // Rebuild label_to_idx from the (possibly filtered) list so class indices are
    // contiguous 0..n_classes and stay consistent with labels_list / label_map.json.
    let label_to_idx: std::collections::HashMap<String, u32> = labels_list
        .iter()
        .enumerate()
        .map(|(i, l)| (l.clone(), i as u32))
        .collect();
    let n_classes = labels_list.len();
    if !cede_set.is_empty() {
        let matched = cede_set
            .iter()
            .filter(|l| taxonomy.label_to_index().contains_key(*l))
            .count();
        eprintln!(
            "Reshape cede-list: {} leaves denied ({} matched taxonomy); n_classes {} -> {}",
            cede_set.len(),
            matched,
            taxonomy.len(),
            n_classes,
        );
    }

    eprintln!("Loading training data from {}...", data.display());
    let (header, records, table_groups) = read_training_data(&data)?;
    eprintln!(
        "Loaded {} records ({} char, {} embed, {} stats dims, {} table groups)",
        records.len(),
        header.char_dim,
        header.embed_dim,
        header.stats_dim,
        table_groups.len(),
    );

    // Filter records to only include labels present in taxonomy.
    // Build old→new index mapping for remapping table group indices.
    let mut valid_records = Vec::new();
    let mut old_to_new: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
    for (old_idx, record) in records.into_iter().enumerate() {
        if label_to_idx.contains_key(&record.label) {
            let new_idx = valid_records.len();
            old_to_new.insert(old_idx, new_idx);
            valid_records.push(record);
        }
    }

    // Remap table group indices, dropping records that were filtered out
    let remapped_groups: Vec<_> = table_groups
        .into_iter()
        .filter_map(|g| {
            let new_indices: Vec<usize> = g
                .record_indices
                .iter()
                .filter_map(|old| old_to_new.get(old).copied())
                .collect();
            if new_indices.is_empty() {
                None
            } else {
                Some(finetype_train::multi_branch::TableGroup {
                    record_indices: new_indices,
                    sibling_headers: g.sibling_headers,
                })
            }
        })
        .collect();

    eprintln!(
        "{} records match taxonomy ({} classes, {} groups retained)",
        valid_records.len(),
        n_classes,
        remapped_groups.len(),
    );

    // Split into train/val
    let mut indices: Vec<usize> = (0..valid_records.len()).collect();
    let mut rng = StdRng::seed_from_u64(seed);
    indices.shuffle(&mut rng);
    let val_size = (valid_records.len() as f32 * val_split) as usize;
    let (val_indices, train_indices) = indices.split_at(val_size);

    let train_records: Vec<_> = train_indices
        .iter()
        .map(|&i| valid_records[i].clone())
        .collect();
    let val_records: Vec<_> = val_indices
        .iter()
        .map(|&i| valid_records[i].clone())
        .collect();

    // Remap table groups for train/val splits — each group's record_indices
    // need to be re-indexed into the split-local arrays
    let train_idx_map: std::collections::HashMap<usize, usize> = train_indices
        .iter()
        .enumerate()
        .map(|(new, &old)| (old, new))
        .collect();
    let val_idx_map: std::collections::HashMap<usize, usize> = val_indices
        .iter()
        .enumerate()
        .map(|(new, &old)| (old, new))
        .collect();

    let mut train_groups = Vec::new();
    let mut val_groups = Vec::new();
    for group in &remapped_groups {
        // Count how many records from this group land in train vs val
        let train_remap: Vec<usize> = group
            .record_indices
            .iter()
            .filter_map(|idx| train_idx_map.get(idx).copied())
            .collect();
        let val_remap: Vec<usize> = group
            .record_indices
            .iter()
            .filter_map(|idx| val_idx_map.get(idx).copied())
            .collect();
        if !train_remap.is_empty() {
            train_groups.push(finetype_train::multi_branch::TableGroup {
                record_indices: train_remap,
                sibling_headers: group.sibling_headers.clone(),
            });
        }
        if !val_remap.is_empty() {
            val_groups.push(finetype_train::multi_branch::TableGroup {
                record_indices: val_remap,
                sibling_headers: group.sibling_headers.clone(),
            });
        }
    }

    eprintln!(
        "Train: {} ({} groups) | Val: {} ({} groups)",
        train_records.len(),
        train_groups.len(),
        val_records.len(),
        val_groups.len(),
    );

    let char_dim = header.char_dim as usize;
    let embed_dim = header.embed_dim as usize;
    let stats_dim = header.stats_dim as usize;
    let header_dim = header.header_dim as usize;
    let valid_dim = header.valid_dim as usize;

    let train_data = MultiBranchDataset::from_records_with_groups(
        &train_records,
        &label_to_idx,
        char_dim,
        embed_dim,
        stats_dim,
        header_dim,
        valid_dim,
        Some(train_groups),
    )?;
    let val_data = MultiBranchDataset::from_records_with_groups(
        &val_records,
        &label_to_idx,
        char_dim,
        embed_dim,
        stats_dim,
        header_dim,
        valid_dim,
        Some(val_groups),
    )?;

    let model_config =
        if let Some(config_path) = &model_config {
            // Load architecture from JSON config file
            let config_str = std::fs::read_to_string(config_path).map_err(|e| {
                anyhow::anyhow!(
                    "Failed to read model config {}: {}",
                    config_path.display(),
                    e
                )
            })?;
            let mut cfg: MultiBranchConfig = serde_json::from_str(&config_str).map_err(|e| {
                anyhow::anyhow!(
                    "Failed to parse model config {}: {}",
                    config_path.display(),
                    e
                )
            })?;
            // Override n_classes and dropout from CLI args (these are training params, not architecture)
            cfg.n_classes = n_classes;
            cfg.dropout = dropout;
            cfg.head_type = head_type.clone();
            eprintln!(
            "Loaded model config from {}: char_hidden={:?}, embed_hidden={:?}, merge_hidden={:?}",
            config_path.display(), cfg.char_hidden, cfg.embed_hidden, cfg.merge_hidden,
        );
            cfg
        } else {
            MultiBranchConfig {
                char_dim,
                embed_dim,
                stats_dim,
                header_dim,
                header_hidden: if header_dim > 0 { [128, 64] } else { [0, 0] },
                n_classes,
                dropout,
                head_type: head_type.clone(),
                ..Default::default()
            }
        };

    // Cross-value attention (choice 0106): encode the FTMB v6 value strings into
    // per-value embeddings, once, with the value encoder. Done after model_config so
    // we know whether attention is enabled.
    let (train_data, val_data) = if let Some(va) = model_config.value_attention.clone() {
        let enc_dir = value_encoder.as_ref().ok_or_else(|| {
            anyhow::anyhow!(
                "model config has a `value_attention` block but --value-encoder was not given"
            )
        })?;
        let enc = Model2VecResources::load(enc_dir).map_err(|e| {
            anyhow::anyhow!("failed to load value encoder {}: {e}", enc_dir.display())
        })?;
        eprintln!(
            "Value attention: encoding up to {} values/col with {} ({}d) for {} train + {} val records",
            va.n_values,
            enc_dir.display(),
            va.value_embed_dim,
            train_records.len(),
            val_records.len(),
        );
        (
            train_data.with_value_attention(&train_records, &va, &enc)?,
            val_data.with_value_attention(&val_records, &va, &enc)?,
        )
    } else {
        (train_data, val_data)
    };

    let train_config = MultiBranchTrainConfig {
        output_dir: output.clone(),
        epochs,
        batch_size,
        lr,
        weight_decay,
        patience,
        seed,
        logit_adjust_tau,
        ..Default::default()
    };

    let labels_opt = if head_type == HeadType::Hierarchical {
        Some(labels_list.as_slice())
    } else {
        None
    };

    // Create renderer: TUI dashboard by default, log-only with --no-tui
    let renderer: Option<Box<dyn TrainingRenderer>> = if no_tui {
        Some(Box::new(LogRenderer::new()))
    } else {
        let head_label = match &model_config.head_type {
            HeadType::Flat => "Flat",
            HeadType::Hierarchical => "Hierarchical",
        };
        let title = format!(
            "Multi-Branch {} ({} classes, {} epochs)",
            head_label, model_config.n_classes, train_config.epochs
        );
        match finetype_train::tui::TuiRenderer::new(title) {
            Ok(tui) => Some(Box::new(tui)),
            Err(e) => {
                eprintln!("TUI init failed ({e}), falling back to log output");
                Some(Box::new(LogRenderer::new()))
            }
        }
    };

    // Pass sibling-context model path if available — loaded inside
    // train_multi_branch on the same device as the training model to
    // avoid Metal device handle mismatch.
    let sibling_ctx_dir = std::path::PathBuf::from("models/sibling-context");
    let sibling_ctx_path = if sibling_ctx_dir.join("model.safetensors").exists() {
        eprintln!(
            "Sibling-context model found at {}",
            sibling_ctx_dir.display()
        );
        Some(sibling_ctx_dir)
    } else {
        None
    };

    let summary = train_multi_branch(
        &train_config,
        &model_config,
        &train_data,
        &val_data,
        labels_opt,
        sibling_ctx_path.as_deref(),
        renderer,
    )?;

    // Save label_map.json (index → label mapping, required for inference)
    let label_map_path = output.join("label_map.json");
    let label_map_json = serde_json::to_string_pretty(&labels_list)?;
    std::fs::write(&label_map_path, label_map_json)?;
    eprintln!(
        "Saved label map ({} labels) to {}",
        labels_list.len(),
        label_map_path.display()
    );

    eprintln!();
    eprintln!("Training complete:");
    eprintln!("  Best epoch: {}", summary.best_epoch + 1);
    eprintln!(
        "  Best val accuracy: {:.2}%",
        summary.best_val_accuracy * 100.0
    );
    eprintln!("  Total epochs: {}", summary.total_epochs);
    eprintln!("  Total time: {:.1}s", summary.total_time_secs);
    eprintln!("  Model saved to: {}", output.display());

    Ok(())
}

/// Extract multi-branch feature vectors from a column of values read from stdin.
///
/// Reads values (one per line, or JSON array with --json), then extracts:
/// - char: 960-dim character distribution features
/// - embed: 512-dim Model2Vec embedding aggregation features
/// - stats: 27-dim column-level statistics
///
/// Outputs JSON to stdout.
pub(crate) fn cmd_extract_features(
    header: Option<String>,
    json_input: bool,
    include_validation: bool,
) -> Result<()> {
    use finetype_model::{
        extract_char_distribution, extract_column_stats, extract_embedding_aggregation,
        ValidationFeatureExtractor, CHAR_DIST_DIM, COLUMN_STATS_DIM, EMBED_AGG_DIM,
    };

    // Read values from stdin
    let stdin = io::stdin();
    let values: Vec<String> = if json_input {
        let mut buf = String::new();
        stdin.lock().read_to_string(&mut buf)?;
        let parsed: Vec<String> = serde_json::from_str(&buf)
            .map_err(|e| anyhow::anyhow!("Failed to parse JSON array from stdin: {}", e))?;
        parsed
    } else {
        stdin.lock().lines().collect::<Result<Vec<_>, _>>()?
    };

    if values.is_empty() {
        anyhow::bail!("No values provided on stdin");
    }

    let value_refs: Vec<&str> = values.iter().map(|s| s.as_str()).collect();

    // Load Model2Vec resources (shared across embed + header features)
    let m2v = load_model2vec_resources();

    // 1. Character distribution (960-dim, deterministic, no model needed)
    let char_features = extract_char_distribution(&value_refs).unwrap_or([0.0f32; CHAR_DIST_DIM]);

    // 2. Embedding aggregation (512-dim, requires Model2Vec)
    let embed_features = match &m2v {
        Some(m2v) => {
            extract_embedding_aggregation(&value_refs, m2v).unwrap_or([0.0f32; EMBED_AGG_DIM])
        }
        None => {
            eprintln!("Warning: Model2Vec not available, embedding features will be zeros");
            [0.0f32; EMBED_AGG_DIM]
        }
    };

    // 3. Column statistics (27-dim, deterministic)
    let stats_features = extract_column_stats(&value_refs).unwrap_or([0.0f32; COLUMN_STATS_DIM]);

    // 4. Header embedding (128-dim, requires Model2Vec + header string)
    let header_features: Vec<f32> = match (&m2v, &header) {
        (Some(m2v), Some(h)) if !h.is_empty() => {
            let embed_dim = m2v.embed_dim().unwrap_or(128);
            match m2v.encode_one(h) {
                Some(tensor) => tensor.to_vec1::<f32>().unwrap_or(vec![0.0f32; embed_dim]),
                None => vec![0.0f32; embed_dim],
            }
        }
        (Some(m2v), _) => {
            // No header provided — zero vector
            let embed_dim = m2v.embed_dim().unwrap_or(128);
            vec![0.0f32; embed_dim]
        }
        (None, _) => {
            eprintln!("Warning: Model2Vec not available, header features will be zeros");
            vec![0.0f32; 128]
        }
    };

    // 5. Validation pass-rate features (239-dim, requires taxonomy with compiled validators)
    let (validation_features, type_index_keys) = if include_validation {
        let taxonomy_path = PathBuf::from("labels");
        let mut taxonomy = load_taxonomy(&taxonomy_path)?;
        taxonomy.compile_validators();
        let extractor = ValidationFeatureExtractor::new(&taxonomy);
        let feats = extractor.extract(&value_refs, &taxonomy);
        let keys: Vec<String> = extractor.type_keys().to_vec();
        (feats, keys)
    } else {
        (Vec::new(), Vec::new())
    };

    // Output as JSON
    let mut output = json!({
        "char": char_features.to_vec(),
        "embed": embed_features.to_vec(),
        "stats": stats_features.to_vec(),
        "header_features": header_features,
        "header": header,
        "n_values": values.len(),
    });

    if include_validation {
        output["validation"] = json!(validation_features);
        output["type_index_keys"] = json!(type_index_keys);
    }

    let stdout = io::stdout();
    serde_json::to_writer(stdout.lock(), &output)?;
    println!();

    Ok(())
}
