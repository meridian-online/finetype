//! Build script for finetype CLI.
//!
//! When the `embed-models` feature is enabled, embeds the active multi-branch
//! model (model.safetensors + config.json + label_map.json), the Model2Vec
//! header encoder, the optional dual-encoder value model, and taxonomy YAML
//! files at compile time so the binary works standalone. (The legacy flat
//! CharCNN / tiered embeds were removed in choice 0107.)
//!
//! Model resolution strategy:
//! 1. Try workspace root models/ (normal development builds)
//! 2. Walk up from CARGO_MANIFEST_DIR to find workspace (cargo publish --dry-run)
//! 3. Download from HuggingFace to cache (cargo install from crates.io)

use std::env;
#[cfg(feature = "embed-models")]
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(feature = "embed-models")]
const HF_REPO: &str = "https://huggingface.co/meridian-online/finetype-model/resolve/main";
#[cfg(feature = "embed-models")]
const CACHE_VERSION: &str = "0.6.38";
/// The active default model — MUST match the `models/default` symlink target.
/// The crates.io source tarball ships no `models/` symlink, so this fallback
/// hardcodes the current default; bump it on every default-model ship (see
/// docs/RELEASE.md). The retired char-cnn-v11 fallback shipped here until
/// v0.6.38 and made a clean `cargo install` from crates.io panic, because
/// `generate_embedded_models` rejects any non-multi-branch default.
#[cfg(feature = "embed-models")]
const DEFAULT_MODEL: &str = "m2v8m-s43";

/// Convert a path to a string safe for use inside `include_bytes!()` / `include_str!()`.
/// On Windows, `canonicalize()` produces `\\?\D:\...` paths with backslashes that Rust
/// interprets as escape sequences. Forward slashes work on all platforms.
#[cfg(feature = "embed-models")]
fn portable_path(p: &Path) -> String {
    p.canonicalize()
        .unwrap()
        .to_string_lossy()
        .replace('\\', "/")
}

/// Find labels directory: check manifest dir first (for packaged builds),
/// then workspace root (for normal development).
#[cfg(feature = "embed-models")]
fn find_labels(manifest_dir: &Path, workspace_root: &Path) -> PathBuf {
    // Check CARGO_MANIFEST_DIR/labels first (works for packaged crates)
    let manifest_labels = manifest_dir.join("labels");
    if manifest_labels.exists() && fs::read_dir(&manifest_labels).is_ok() {
        return manifest_labels;
    }

    // Fall back to workspace root labels
    let workspace_labels = workspace_root.join("labels");
    if workspace_labels.exists() && fs::read_dir(&workspace_labels).is_ok() {
        return workspace_labels;
    }

    panic!(
        "Cannot find labels directory. Checked:\n  {}\n  {}",
        manifest_labels.display(),
        workspace_labels.display()
    );
}

/// Walk up from start_dir looking for a models/default symlink or directory.
/// Returns the parent directory containing models/, or None if not found.
fn find_workspace_with_models(start_dir: &Path) -> Option<PathBuf> {
    let mut current = start_dir.to_path_buf();
    // Limit to 10 levels to avoid infinite loops
    for _ in 0..10 {
        let models_default = current.join("models").join("default");
        // Check for symlink or directory
        if models_default.exists()
            || std::fs::read_link(&models_default).is_ok()
            || std::fs::read_to_string(&models_default).is_ok()
        {
            return Some(current);
        }
        if !current.pop() {
            break;
        }
    }
    None
}

/// Find models directory: try workspace root, walk-up search, or download.
#[cfg(feature = "embed-models")]
fn find_models(manifest_dir: &Path, workspace_root: &Path) -> PathBuf {
    // Try workspace root first (normal development builds)
    let workspace_models = workspace_root.join("models");
    if workspace_models.join("default").exists() {
        println!(
            "cargo:warning=Using models from workspace: {}",
            workspace_models.display()
        );
        return workspace_models;
    }

    // Try to walk up from manifest dir to find real workspace (cargo publish --dry-run)
    if let Some(found_root) = find_workspace_with_models(manifest_dir) {
        let found_models = found_root.join("models");
        println!(
            "cargo:warning=Found workspace models via walk-up: {}",
            found_models.display()
        );
        return found_models;
    }

    // No local models found — download from HuggingFace to cache
    println!("cargo:warning=Models not found locally, downloading from HuggingFace...");
    download_models()
}

/// Download all model groups from HuggingFace to a cache directory.
/// Returns the path to the models directory.
#[cfg(feature = "embed-models")]
fn download_models() -> PathBuf {
    let cache_dir = get_cache_dir();
    let models_dir = cache_dir.join("models");

    // Create models directory
    fs::create_dir_all(&models_dir).expect("Failed to create models cache directory");

    // Download the active multi-branch default model. These are the three files
    // generate_embedded_models embeds (model.safetensors + config.json +
    // label_map.json); the legacy char-cnn shape (labels.json / config.yaml) was
    // removed in choice 0107, so fetching it here made the embed step panic.
    download_model_group(
        &models_dir,
        DEFAULT_MODEL,
        &["model.safetensors", "label_map.json", "config.json"],
    );

    // Dual-encoder: the value-aggregation branch needs a SECOND Model2Vec encoder
    // (e.g. potion-8M) co-located at <model>/value_model2vec/, declared by
    // config.value_embed_model. Required for m2v8m-s43 — the model fails to load
    // without it. Mirrors download-model.sh's value-encoder fetch.
    let value_subdir = format!("{DEFAULT_MODEL}/value_model2vec");
    download_model_group(
        &models_dir,
        &value_subdir,
        &["model.safetensors", "tokenizer.json"],
    );

    // Header branch: the shared Model2Vec (potion-4M) encoder. Required — the
    // multi-branch header branch depends on it. `type_embeddings.safetensors`
    // and `label_index.json` used to be fetched here for the semantic header
    // classifier; that classifier had no reachable construction site outside
    // tests and is gone, and neither file was ever `include_bytes!`-embedded,
    // so fetching them only slowed the build.
    download_model_group(
        &models_dir,
        "model2vec",
        &["model.safetensors", "tokenizer.json"],
    );

    // Create models/default symlink -> the multi-branch default.
    let default_link = models_dir.join("default");
    let _ = fs::remove_file(&default_link);
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        symlink(DEFAULT_MODEL, &default_link).expect("Failed to create models/default symlink");
    }
    #[cfg(windows)]
    {
        // On Windows, create a plain text file containing the target path
        fs::write(&default_link, DEFAULT_MODEL).expect("Failed to create models/default link file");
    }

    println!(
        "cargo:warning=Downloaded models to cache: {}",
        models_dir.display()
    );
    models_dir
}

/// Get the cache directory for models. Uses CARGO_HOME or HOME/.cache/finetype.
#[cfg(feature = "embed-models")]
fn get_cache_dir() -> PathBuf {
    // Prefer CARGO_HOME if set (more aligned with Rust tooling conventions)
    if let Ok(cargo_home) = env::var("CARGO_HOME") {
        return PathBuf::from(cargo_home)
            .join("finetype")
            .join(format!("v{}", CACHE_VERSION));
    }

    // Fall back to HOME/.cache/finetype on Unix, %LOCALAPPDATA% on Windows
    #[cfg(target_os = "windows")]
    {
        if let Ok(appdata) = env::var("LOCALAPPDATA") {
            return PathBuf::from(appdata)
                .join("finetype")
                .join(format!("v{}", CACHE_VERSION));
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        if let Ok(home) = env::var("HOME") {
            return PathBuf::from(home)
                .join(".cache")
                .join("finetype")
                .join(format!("v{}", CACHE_VERSION));
        }
    }

    // Fallback to OUT_DIR
    let out_dir = env::var("OUT_DIR").unwrap_or_else(|_| "/tmp/finetype-models".to_string());
    PathBuf::from(out_dir)
}

/// Download a model group (e.g., char-cnn-v11, model2vec). Panics if any file is missing.
#[cfg(feature = "embed-models")]
fn download_model_group(models_dir: &Path, group_name: &str, files: &[&str]) {
    let group_dir = models_dir.join(group_name);
    fs::create_dir_all(&group_dir)
        .unwrap_or_else(|_| panic!("Failed to create {} directory", group_name));

    for file in files {
        let file_path = group_dir.join(file);

        // Skip if already downloaded
        if file_path.exists() {
            continue;
        }

        let url = format!("{}/{}/{}", HF_REPO, group_name, file);
        download_file(&url, &file_path).unwrap_or_else(|_| {
            panic!(
                "Failed to download {}/{} from HuggingFace",
                group_name, file
            )
        });
    }

    println!(
        "cargo:warning=Downloaded {} ({} files)",
        group_name,
        files.len()
    );
}

/// Download a single file from a URL using ureq.
#[cfg(feature = "embed-models")]
fn download_file(url: &str, dest: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let response = ureq::get(url).call()?;
    let mut reader = response.into_reader();
    let mut file = fs::File::create(dest)?;
    std::io::copy(&mut reader, &mut file)?;
    Ok(())
}

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let manifest_path = PathBuf::from(&manifest_dir);

    // Try to find workspace root: start from manifest, go up 2 levels (normal case),
    // but be prepared to use walk-up search if needed
    let mut workspace_root = manifest_path
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.to_path_buf());

    // If the above doesn't give us a valid workspace root, try walking up
    if let Some(root) = &workspace_root {
        if !root.join("Cargo.toml").exists() {
            workspace_root = find_workspace_with_models(&manifest_path);
        }
    } else {
        workspace_root = find_workspace_with_models(&manifest_path);
    }

    let workspace_root = workspace_root.unwrap_or_else(|| {
        // Last resort: use manifest dir as base
        PathBuf::from(&manifest_dir)
    });

    #[cfg(feature = "embed-models")]
    {
        let labels_dir = find_labels(&manifest_path, &workspace_root);
        let models_dir = find_models(&manifest_path, &workspace_root);

        println!("cargo:rerun-if-changed={}", models_dir.display());
        println!("cargo:rerun-if-changed={}", labels_dir.display());

        generate_embedded_models(&models_dir, &labels_dir);
    }

    #[cfg(not(feature = "embed-models"))]
    {
        let _ = workspace_root;
    }
}

#[cfg(feature = "embed-models")]
fn generate_embedded_models(models_base: &Path, labels_base: &Path) {
    // Follow the models/default symlink to find the active model.
    //
    // Resolution order (handles all platforms):
    //  1. read_link — works on Linux/macOS and Windows with real symlinks
    //  2. read_to_string — Windows fallback where git checks out symlinks as
    //     plain text files containing the target path
    //
    // We skip the exists() check because on Windows a file-type symlink
    // pointing to a directory returns false for exists() even when the
    // target directory is present.
    let default_link = models_base.join("default");
    let model_dir = std::fs::read_link(&default_link)
        .map(|target| {
            if target.is_relative() {
                models_base.join(target)
            } else {
                target
            }
        })
        .or_else(|_| {
            // Windows fallback: read as plain text file (git symlink compat)
            std::fs::read_to_string(&default_link).map(|content| models_base.join(content.trim()))
        })
        .unwrap_or_else(|e| {
            panic!(
                "Cannot resolve models/default at {:?}: {e}. \
                 This should not happen — models were either local or downloaded. Please report this issue.",
                default_link
            )
        });

    let out_dir = env::var("OUT_DIR").unwrap();
    let dest = PathBuf::from(&out_dir).join("embedded_models.rs");

    let mut code = String::new();
    code.push_str("// Auto-generated by build.rs — do not edit\n\n");

    // The shipped model is multi-branch (label_map.json + config.json); the
    // legacy flat-CharCNN and tiered embeds were removed (choice 0107 stage 2b).
    if !(model_dir.join("label_map.json").exists() && model_dir.join("config.json").exists()) {
        panic!(
            "models/default ({:?}) is not a multi-branch model (missing label_map.json / config.json). \
             The flat-CharCNN and tiered model types were removed in choice 0107.",
            model_dir
        );
    }
    // Embed multi-branch model files (model.safetensors, config.json, label_map.json)
    let weights_path = portable_path(&model_dir.join("model.safetensors"));
    let config_path = portable_path(&model_dir.join("config.json"));
    let labels_path = portable_path(&model_dir.join("label_map.json"));
    code.push_str(&format!(
        "\npub const MB_WEIGHTS: &[u8] = include_bytes!(\"{weights_path}\");\n"
    ));
    code.push_str(&format!(
        "pub const MB_CONFIG: &[u8] = include_bytes!(\"{config_path}\");\n"
    ));
    code.push_str(&format!(
        "pub const MB_LABELS: &[u8] = include_bytes!(\"{labels_path}\");\n"
    ));
    code.push_str("\npub const EMBEDDED_MODEL_TYPE: &str = \"multi-branch\";\n");

    // Dual-encoder: embed the value-branch encoder (e.g. potion-8M) when the default
    // model co-locates one at <model_dir>/value_model2vec/ (config.value_embed_model).
    // The header branch + semantic/entity/sense classifiers keep the shared model2vec
    // (M2V_*); this is the SECOND encoder the value-aggregation branch needs. Absent
    // for single-encoder models (v19, m2v-244) → false stubs, from_bytes passes None.
    let value_m2v = model_dir.join("value_model2vec");
    if value_m2v.join("model.safetensors").exists() {
        println!("cargo:rerun-if-changed={}", value_m2v.display());
        let vtok = portable_path(&value_m2v.join("tokenizer.json"));
        let vmodel = portable_path(&value_m2v.join("model.safetensors"));
        code.push_str("\npub const HAS_MB_VALUE_M2V: bool = true;\n");
        code.push_str(&format!(
            "pub const MB_VALUE_TOKENIZER: &[u8] = include_bytes!(\"{vtok}\");\n"
        ));
        code.push_str(&format!(
            "pub const MB_VALUE_MODEL: &[u8] = include_bytes!(\"{vmodel}\");\n"
        ));
        println!(
            "cargo:warning=Embedding dual-encoder value model2vec from {}",
            value_m2v.display()
        );
    } else {
        code.push_str("\npub const HAS_MB_VALUE_M2V: bool = false;\n");
        code.push_str("pub const MB_VALUE_TOKENIZER: &[u8] = &[];\n");
        code.push_str("pub const MB_VALUE_MODEL: &[u8] = &[];\n");
    }

    // Embed taxonomy YAML files
    let mut yaml_paths: Vec<_> = fs::read_dir(labels_base)
        .expect("Failed to read labels directory")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with("definitions_") && n.ends_with(".yaml"))
                .unwrap_or(false)
        })
        .collect();
    yaml_paths.sort();

    code.push_str("\npub const TAXONOMY_YAMLS: &[&str] = &[\n");
    for path in &yaml_paths {
        let canonical = portable_path(path);
        code.push_str(&format!("    include_str!(\"{canonical}\"),\n"));
    }
    code.push_str("];\n");

    // ── Model2Vec semantic hint classifier ──────────────────────────────────
    // Embeds the Model2Vec artifacts (tokenizer, embeddings, type embeddings,
    // label index) for semantic column name classification. Optional — the
    // classifier falls back to the hardcoded header_hint() when unavailable.
    let model2vec_dir = models_base.join("model2vec");
    println!("cargo:rerun-if-changed={}", model2vec_dir.display());

    if model2vec_dir.join("model.safetensors").exists() {
        let tok_path = portable_path(&model2vec_dir.join("tokenizer.json"));
        let emb_path = portable_path(&model2vec_dir.join("model.safetensors"));

        code.push_str("\n// Model2Vec header encoder (multi-branch header branch)\n");
        code.push_str("pub const HAS_MODEL2VEC: bool = true;\n");
        code.push_str(&format!(
            "pub const M2V_TOKENIZER: &[u8] = include_bytes!(\"{tok_path}\");\n"
        ));
        code.push_str(&format!(
            "pub const M2V_MODEL: &[u8] = include_bytes!(\"{emb_path}\");\n"
        ));

        println!(
            "cargo:warning=Embedding Model2Vec from {}",
            model2vec_dir.display()
        );
    } else {
        code.push_str("\n// Model2Vec not available — header encoder disabled\n");
        code.push_str("pub const HAS_MODEL2VEC: bool = false;\n");
        code.push_str("pub const M2V_TOKENIZER: &[u8] = &[];\n");
        code.push_str("pub const M2V_MODEL: &[u8] = &[];\n");
    }

    fs::write(&dest, code).unwrap_or_else(|e| panic!("Failed to write {}: {}", dest.display(), e));
}
