//! Build script for finetype CLI.
//!
//! When the `embed-models` feature is enabled, embeds the active model
//! (flat CharCNN or tiered) and taxonomy YAML files at compile time so the
//! binary works standalone. Detects tiered models by the presence of
//! tier_graph.json in the default model directory.
//!
//! Model resolution strategy:
//! 1. Try workspace root models/ (normal development builds)
//! 2. Walk up from CARGO_MANIFEST_DIR to find workspace (cargo publish --dry-run)
//! 3. Download from HuggingFace to cache (cargo install from crates.io)

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const HF_REPO: &str = "https://huggingface.co/noon-org/finetype-char-cnn/resolve/main";
const CACHE_VERSION: &str = "0.5.3";

/// Convert a path to a string safe for use inside `include_bytes!()` / `include_str!()`.
/// On Windows, `canonicalize()` produces `\\?\D:\...` paths with backslashes that Rust
/// interprets as escape sequences. Forward slashes work on all platforms.
fn portable_path(p: &Path) -> String {
    p.canonicalize()
        .unwrap()
        .to_string_lossy()
        .replace('\\', "/")
}

/// Find labels directory: check manifest dir first (for packaged builds),
/// then workspace root (for normal development).
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
fn download_models() -> PathBuf {
    let cache_dir = get_cache_dir();
    let models_dir = cache_dir.join("models");

    // Create models directory
    fs::create_dir_all(&models_dir).expect("Failed to create models cache directory");

    // Download and setup models/default -> char-cnn-v11
    download_model_group(
        &models_dir,
        "char-cnn-v11",
        &["model.safetensors", "labels.json", "config.yaml"],
    );

    // Download optional model groups (graceful degradation if they fail)
    download_model_group_optional(
        &models_dir,
        "model2vec",
        &[
            "model.safetensors",
            "type_embeddings.safetensors",
            "tokenizer.json",
            "label_index.json",
        ],
    );

    download_model_group_optional(
        &models_dir,
        "entity-classifier",
        &["model.safetensors", "config.json", "label_index.json"],
    );

    download_model_group_optional(&models_dir, "sense", &["model.safetensors", "config.json"]);

    // Create models/default symlink
    let default_link = models_dir.join("default");
    let _ = fs::remove_file(&default_link);
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        symlink("char-cnn-v11", &default_link).expect("Failed to create models/default symlink");
    }
    #[cfg(windows)]
    {
        // On Windows, create a plain text file containing the target path
        fs::write(&default_link, "char-cnn-v11")
            .expect("Failed to create models/default link file");
    }

    println!(
        "cargo:warning=Downloaded models to cache: {}",
        models_dir.display()
    );
    models_dir
}

/// Get the cache directory for models. Uses CARGO_HOME or HOME/.cache/finetype.
fn get_cache_dir() -> PathBuf {
    // Prefer CARGO_HOME if set (more aligned with Rust tooling conventions)
    if let Ok(cargo_home) = env::var("CARGO_HOME") {
        return PathBuf::from(cargo_home)
            .join("finetype")
            .join(&format!("v{}", CACHE_VERSION));
    }

    // Fall back to HOME/.cache/finetype on Unix, %LOCALAPPDATA% on Windows
    #[cfg(target_os = "windows")]
    {
        if let Ok(appdata) = env::var("LOCALAPPDATA") {
            return PathBuf::from(appdata)
                .join("finetype")
                .join(&format!("v{}", CACHE_VERSION));
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        if let Ok(home) = env::var("HOME") {
            return PathBuf::from(home)
                .join(".cache")
                .join("finetype")
                .join(&format!("v{}", CACHE_VERSION));
        }
    }

    // Fallback to OUT_DIR
    let out_dir = env::var("OUT_DIR").unwrap_or_else(|_| "/tmp/finetype-models".to_string());
    PathBuf::from(out_dir)
}

/// Download a model group (e.g., char-cnn-v11, model2vec). Panics if any file is missing.
fn download_model_group(models_dir: &Path, group_name: &str, files: &[&str]) {
    let group_dir = models_dir.join(group_name);
    fs::create_dir_all(&group_dir).expect(&format!("Failed to create {} directory", group_name));

    for file in files {
        let file_path = group_dir.join(file);

        // Skip if already downloaded
        if file_path.exists() {
            continue;
        }

        let url = format!("{}/{}/{}", HF_REPO, group_name, file);
        download_file(&url, &file_path).expect(&format!(
            "Failed to download {}/{} from HuggingFace",
            group_name, file
        ));
    }

    println!(
        "cargo:warning=Downloaded {} ({} files)",
        group_name,
        files.len()
    );
}

/// Download a model group, but don't panic if it fails (optional models).
fn download_model_group_optional(models_dir: &Path, group_name: &str, files: &[&str]) {
    let group_dir = models_dir.join(group_name);
    fs::create_dir_all(&group_dir).ok();

    for file in files {
        let file_path = group_dir.join(file);

        // Skip if already downloaded
        if file_path.exists() {
            continue;
        }

        let url = format!("{}/{}/{}", HF_REPO, group_name, file);
        if download_file(&url, &file_path).is_err() {
            println!(
                "cargo:warning=Failed to download {}/{} — this component will be disabled",
                group_name, file
            );
            let _ = fs::remove_dir_all(&group_dir);
            return;
        }
    }

    println!(
        "cargo:warning=Downloaded {} ({} files) — optional component enabled",
        group_name,
        files.len()
    );
}

/// Download a single file from a URL using ureq.
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

    // Detect model type: tiered (has tier_graph.json) or flat (has model.safetensors)
    let is_tiered = model_dir.join("tier_graph.json").exists();

    if is_tiered {
        generate_tiered_embeds(&model_dir, &mut code);
        // Generate empty flat stubs so main.rs compiles regardless of model type
        code.push_str("\npub const FLAT_WEIGHTS: &[u8] = &[];\n");
        code.push_str("pub const FLAT_LABELS: &[u8] = &[];\n");
        code.push_str("pub const FLAT_CONFIG: &[u8] = &[];\n");
        code.push_str("\npub const EMBEDDED_MODEL_TYPE: &str = \"tiered\";\n");
    } else {
        generate_flat_embeds(&model_dir, &mut code);
        // Generate empty tiered stubs
        code.push_str("\npub const TIER_GRAPH: &[u8] = &[];\n");
        code.push_str("pub fn get_tiered_model_data(_: &str) -> Option<(&'static [u8], &'static [u8], &'static [u8])> { None }\n");
        code.push_str("\npub const EMBEDDED_MODEL_TYPE: &str = \"flat\";\n");
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
        let type_path = portable_path(&model2vec_dir.join("type_embeddings.safetensors"));
        let label_path = portable_path(&model2vec_dir.join("label_index.json"));

        code.push_str("\n// Model2Vec semantic hint classifier\n");
        code.push_str("pub const HAS_MODEL2VEC: bool = true;\n");
        code.push_str(&format!(
            "pub const M2V_TOKENIZER: &[u8] = include_bytes!(\"{tok_path}\");\n"
        ));
        code.push_str(&format!(
            "pub const M2V_MODEL: &[u8] = include_bytes!(\"{emb_path}\");\n"
        ));
        code.push_str(&format!(
            "pub const M2V_TYPE_EMBEDDINGS: &[u8] = include_bytes!(\"{type_path}\");\n"
        ));
        code.push_str(&format!(
            "pub const M2V_LABEL_INDEX: &[u8] = include_bytes!(\"{label_path}\");\n"
        ));

        println!(
            "cargo:warning=Embedding Model2Vec from {}",
            model2vec_dir.display()
        );
    } else {
        code.push_str("\n// Model2Vec not available — semantic hint classifier disabled\n");
        code.push_str("pub const HAS_MODEL2VEC: bool = false;\n");
        code.push_str("pub const M2V_TOKENIZER: &[u8] = &[];\n");
        code.push_str("pub const M2V_MODEL: &[u8] = &[];\n");
        code.push_str("pub const M2V_TYPE_EMBEDDINGS: &[u8] = &[];\n");
        code.push_str("pub const M2V_LABEL_INDEX: &[u8] = &[];\n");
    }

    // ── Entity classifier (full_name demotion gate) ────────────────────────
    // Embeds the entity classifier MLP weights and config for the binary
    // demotion gate. Optional — full_name demotion is disabled when unavailable.
    let entity_dir = models_base.join("entity-classifier");
    println!("cargo:rerun-if-changed={}", entity_dir.display());

    if entity_dir.join("model.safetensors").exists() && entity_dir.join("config.json").exists() {
        let model_path = portable_path(&entity_dir.join("model.safetensors"));
        let config_path = portable_path(&entity_dir.join("config.json"));

        code.push_str("\n// Entity classifier (full_name demotion gate, NNFT-152)\n");
        code.push_str("pub const HAS_ENTITY_CLASSIFIER: bool = true;\n");
        code.push_str(&format!(
            "pub const ENTITY_MODEL: &[u8] = include_bytes!(\"{model_path}\");\n"
        ));
        code.push_str(&format!(
            "pub const ENTITY_CONFIG: &[u8] = include_bytes!(\"{config_path}\");\n"
        ));

        println!(
            "cargo:warning=Embedding entity classifier from {}",
            entity_dir.display()
        );
    } else {
        code.push_str("\n// Entity classifier not available — full_name demotion disabled\n");
        code.push_str("pub const HAS_ENTITY_CLASSIFIER: bool = false;\n");
        code.push_str("pub const ENTITY_MODEL: &[u8] = &[];\n");
        code.push_str("pub const ENTITY_CONFIG: &[u8] = &[];\n");
    }

    // ── Sense classifier (broad category prediction, NNFT-171) ──────────
    // Embeds the Sense model (Architecture A cross-attention) for broad
    // semantic category prediction. Optional — when absent, the legacy
    // header-hint pipeline is used.
    let sense_dir = models_base.join("sense");
    println!("cargo:rerun-if-changed={}", sense_dir.display());

    if sense_dir.join("model.safetensors").exists() && sense_dir.join("config.json").exists() {
        let model_path = portable_path(&sense_dir.join("model.safetensors"));
        let config_path = portable_path(&sense_dir.join("config.json"));

        code.push_str("\n// Sense classifier (broad category prediction, NNFT-171)\n");
        code.push_str("pub const HAS_SENSE_CLASSIFIER: bool = true;\n");
        code.push_str(&format!(
            "pub const SENSE_MODEL: &[u8] = include_bytes!(\"{model_path}\");\n"
        ));
        code.push_str(&format!(
            "pub const SENSE_CONFIG: &[u8] = include_bytes!(\"{config_path}\");\n"
        ));

        println!(
            "cargo:warning=Embedding Sense classifier from {}",
            sense_dir.display()
        );
    } else {
        code.push_str("\n// Sense classifier not available — legacy pipeline used\n");
        code.push_str("pub const HAS_SENSE_CLASSIFIER: bool = false;\n");
        code.push_str("pub const SENSE_MODEL: &[u8] = &[];\n");
        code.push_str("pub const SENSE_CONFIG: &[u8] = &[];\n");
    }

    fs::write(&dest, code).unwrap_or_else(|e| panic!("Failed to write {}: {}", dest.display(), e));
}

/// Generate embeds for a flat CharCNN model (single model.safetensors + labels.json + config.yaml).
#[cfg(feature = "embed-models")]
fn generate_flat_embeds(model_dir: &Path, code: &mut String) {
    assert!(
        model_dir.join("model.safetensors").exists(),
        "Flat model not found at {:?}. Run from workspace root or disable embed-models feature.",
        model_dir
    );

    let weights_path = portable_path(&model_dir.join("model.safetensors"));
    let labels_path = portable_path(&model_dir.join("labels.json"));
    let config_path = portable_path(&model_dir.join("config.yaml"));

    code.push_str(&format!(
        "pub const FLAT_WEIGHTS: &[u8] = include_bytes!(\"{weights_path}\");\n"
    ));
    code.push_str(&format!(
        "pub const FLAT_LABELS: &[u8] = include_bytes!(\"{labels_path}\");\n"
    ));
    code.push_str(&format!(
        "pub const FLAT_CONFIG: &[u8] = include_bytes!(\"{config_path}\");\n"
    ));
}

/// Generate embeds for a tiered model (tier_graph.json + multiple tier subdirectories).
///
/// Produces:
/// - `TIER_GRAPH: &[u8]` — the tier_graph.json bytes
/// - `get_tiered_model_data(dir: &str) -> Option<(&[u8], &[u8], &[u8])>` — lookup function
#[cfg(feature = "embed-models")]
fn generate_tiered_embeds(model_dir: &Path, code: &mut String) {
    let graph_path = model_dir.join("tier_graph.json");
    assert!(
        graph_path.exists(),
        "Tiered model tier_graph.json not found at {:?}.",
        model_dir
    );

    // Embed tier_graph.json
    let graph_portable = portable_path(&graph_path);
    code.push_str(&format!(
        "pub const TIER_GRAPH: &[u8] = include_bytes!(\"{graph_portable}\");\n\n"
    ));

    // Find all tier subdirectories that contain model.safetensors
    let mut tier_dirs: Vec<(String, PathBuf)> = Vec::new();
    for entry in fs::read_dir(model_dir).expect("Failed to read model directory") {
        let entry = entry.expect("Failed to read directory entry");
        let path = entry.path();
        if path.is_dir() && path.join("model.safetensors").exists() {
            let dir_name = entry.file_name().to_string_lossy().to_string();
            tier_dirs.push((dir_name, path));
        }
    }
    tier_dirs.sort_by(|a, b| a.0.cmp(&b.0));

    // Generate the lookup function
    code.push_str(
        "pub fn get_tiered_model_data(dir_name: &str) -> Option<(&'static [u8], &'static [u8], &'static [u8])> {\n",
    );
    code.push_str("    match dir_name {\n");

    for (dir_name, dir_path) in &tier_dirs {
        let weights = portable_path(&dir_path.join("model.safetensors"));
        let labels = portable_path(&dir_path.join("labels.json"));
        let config = portable_path(&dir_path.join("config.yaml"));

        code.push_str(&format!(
            "        \"{dir_name}\" => Some((\n\
             \x20           include_bytes!(\"{weights}\"),\n\
             \x20           include_bytes!(\"{labels}\"),\n\
             \x20           include_bytes!(\"{config}\"),\n\
             \x20       )),\n"
        ));
    }

    code.push_str("        _ => None,\n");
    code.push_str("    }\n");
    code.push_str("}\n");
}
