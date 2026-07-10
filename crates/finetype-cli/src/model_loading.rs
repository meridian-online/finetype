//! `model_loading` — extracted from main.rs (mechanical split, no behaviour change).

use super::*;

/// Load a MultiBranchClassifier: try the model directory first, then fall back to
/// the embedded model if the path doesn't exist (release binaries).
pub(crate) fn load_multi_branch_classifier(
    model: &PathBuf,
) -> Result<finetype_model::MultiBranchClassifier> {
    if model.exists() && model.join("config.json").exists() {
        finetype_model::MultiBranchClassifier::load(model).map_err(Into::into)
    } else {
        #[cfg(feature = "embed-models")]
        {
            if embedded::EMBEDDED_MODEL_TYPE == "multi-branch" && !embedded::MB_WEIGHTS.is_empty() {
                // Load Model2Vec resources (disk or embedded)
                let m2v = load_model2vec_resources().ok_or_else(|| {
                    anyhow::anyhow!(
                        "Multi-branch model requires Model2Vec resources but none found"
                    )
                })?;
                // Dual-encoder: load the embedded value-branch encoder (potion-8M)
                // when present, so a released binary with no disk models drives the
                // value-aggregation branch correctly. Single-encoder models embed
                // HAS_MB_VALUE_M2V=false → None (value branch shares m2v).
                let value_m2v = if embedded::HAS_MB_VALUE_M2V {
                    Some(
                        finetype_model::Model2VecResources::from_bytes(
                            embedded::MB_VALUE_TOKENIZER,
                            embedded::MB_VALUE_MODEL,
                        )
                        .map_err(|e| {
                            anyhow::anyhow!("Failed to load embedded value encoder: {e}")
                        })?,
                    )
                } else {
                    None
                };
                return finetype_model::MultiBranchClassifier::from_bytes(
                    embedded::MB_CONFIG,
                    embedded::MB_LABELS,
                    embedded::MB_WEIGHTS,
                    m2v,
                    value_m2v,
                )
                .map_err(Into::into);
            }
        }
        anyhow::bail!(
            "Model directory {:?} not found and no embedded multi-branch model available. \
             Set FINETYPE_MODEL_DIR or build with `embed-models` feature.",
            model
        )
    }
}

/// Load shared Model2Vec resources (tokenizer + embeddings).
///
/// Resolution order:
///  1. models/model2vec directory on disk (development)
///  2. Embedded Model2Vec bytes (release binaries)
///  3. None — no shared resources available
pub(crate) fn load_model2vec_resources() -> Option<finetype_model::Model2VecResources> {
    // Try disk-based model first (development workflow)
    let model_dir = std::path::PathBuf::from("models/model2vec");
    if model_dir.join("model.safetensors").exists() {
        return finetype_model::Model2VecResources::load(&model_dir)
            .map_err(|e| eprintln!("Warning: Failed to load Model2Vec resources from disk: {e}"))
            .ok();
    }

    // Try embedded model bytes (release binary)
    #[cfg(feature = "embed-models")]
    {
        if embedded::HAS_MODEL2VEC {
            return finetype_model::Model2VecResources::from_bytes(
                embedded::M2V_TOKENIZER,
                embedded::M2V_MODEL,
            )
            .map_err(|e| eprintln!("Warning: Failed to load embedded Model2Vec resources: {e}"))
            .ok();
        }
    }

    None
}

/// Wire Model2Vec + sibling context for multi-branch classifiers.
///
/// When multi-branch is active, Sense is not used — but sibling-context attention
/// still needs Model2Vec to encode headers. This wires both independently of Sense.
pub(crate) fn wire_model2vec_and_siblings(cc: &mut finetype_model::ColumnClassifier) {
    if let Some(m2v) = load_model2vec_resources() {
        eprintln!("Loaded Model2Vec for multi-branch sibling context");
        cc.set_model2vec(m2v);
        wire_sibling_context(cc);
    }
}

/// Wire Model2Vec for a multi-branch classifier WITHOUT sibling context.
///
/// `infer` classifies one column at a time and never calls the cross-column
/// `classify_columns_with_context` path, so the sibling-context attention model
/// (396,800 params) would be loaded from disk only to never be invoked. Skipping
/// that load is the single-value/single-column infer fast path (card 0006).
/// `profile`, which has real siblings, still wires both via
/// `wire_model2vec_and_siblings`.
pub(crate) fn wire_model2vec_only(cc: &mut finetype_model::ColumnClassifier) {
    if let Some(m2v) = load_model2vec_resources() {
        cc.set_model2vec(m2v);
    }
}

/// Load and wire the sibling-context attention module.
///
/// Looks for `models/sibling-context/model.safetensors`. When found,
/// attaches to the column classifier. When absent, the pipeline is unchanged.
pub(crate) fn wire_sibling_context(cc: &mut finetype_model::ColumnClassifier) {
    let model_dir = std::path::PathBuf::from("models/sibling-context");
    if !model_dir.join("model.safetensors").exists() {
        return; // Silent — model is optional
    }
    match finetype_model::SiblingContextAttention::load(&model_dir) {
        Ok(sibling) => {
            eprintln!(
                "Loaded sibling-context attention ({} params)",
                sibling.param_count()
            );
            cc.set_sibling_context(sibling);
        }
        Err(e) => {
            eprintln!("Warning: Failed to load sibling-context model: {e}");
        }
    }
}

/// Load taxonomy from a file or directory.
pub(crate) fn load_taxonomy(path: &PathBuf) -> Result<Taxonomy> {
    if path.exists() {
        if path.is_dir() {
            Ok(Taxonomy::from_directory(path)?)
        } else {
            Ok(Taxonomy::from_file(path)?)
        }
    } else {
        // Fall back to embedded taxonomy (release binaries)
        #[cfg(feature = "embed-models")]
        {
            Ok(Taxonomy::from_yamls(embedded::TAXONOMY_YAMLS)?)
        }
        #[cfg(not(feature = "embed-models"))]
        {
            anyhow::bail!(
                "Taxonomy path {:?} not found. Build with `embed-models` feature for standalone use.",
                path
            )
        }
    }
}
