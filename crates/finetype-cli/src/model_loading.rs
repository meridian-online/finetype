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

/// Wire the shared Model2Vec encoder into a multi-branch classifier.
///
/// The multi-branch header branch encodes the column header with this encoder,
/// so every `profile`/`infer` path needs it.
///
/// This used to also load `models/sibling-context/model.safetensors` and attach
/// it as a cross-column enrichment layer whenever that directory happened to be
/// present. `build.rs` never embedded that artifact, so a released binary never
/// had it: a developer running `finetype profile` from this checkout was
/// measuring a pipeline that no released binary can run. Wiring it here is gone;
/// the artifact is still trained (`train-sibling-context`) and still consumed by
/// the multi-branch trainer, which is where it earns its keep.
///
/// That leaves a REAL train/serve skew — the header branch was trained behind
/// that module — and putting it back has been built and measured rather than
/// argued: `docs/sibling-context-serving-measurement.md`. Reproducing training
/// conditions here moves the emitted record on 46.8% of columns of real files,
/// ties on the only ground-truthed instrument (720 vs 719 of 828), and loses
/// 8,113 columns to 2,611 against a fixed column-intrinsic oracle over 693,499
/// corpus columns. Do not restore it on the strength of the argument alone; the
/// argument is sound and the measurement disagrees with it.
pub(crate) fn wire_model2vec(cc: &mut finetype_model::ColumnClassifier) {
    if let Some(m2v) = load_model2vec_resources() {
        cc.set_model2vec(m2v);
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
