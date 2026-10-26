//! Freeze a model's validation rules into `<model_dir>/validation_snapshot.json`.
//!
//! The snapshot records the validation block of every key in the model's
//! `config.json` `type_index_keys`, as the taxonomy has them now, so the model
//! keeps reading the rules it was shipped with when `labels/` later changes.
//! A rule that does not compile is an error here, at write time, rather than a
//! load failure for whoever receives the model.
//!
//! Usage: `write_validation_snapshot <model_dir> [--labels labels]`

use finetype_core::validator_snapshot::{ValidatorSnapshot, SNAPSHOT_FILE};
use finetype_core::Taxonomy;
use std::path::PathBuf;
use std::process;

fn fail(message: String) -> ! {
    eprintln!("write_validation_snapshot: {message}");
    process::exit(1);
}

fn main() {
    let mut model_dir: Option<PathBuf> = None;
    let mut labels = PathBuf::from("labels");
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--labels" => {
                labels = args
                    .next()
                    .map(PathBuf::from)
                    .unwrap_or_else(|| fail("--labels needs a directory".to_string()));
            }
            _ if model_dir.is_none() && !arg.starts_with("--") => {
                model_dir = Some(PathBuf::from(arg));
            }
            _ => fail(format!(
                "usage: write_validation_snapshot <model_dir> [--labels labels] (got {arg})"
            )),
        }
    }
    let model_dir = model_dir.unwrap_or_else(|| fail("a model directory is required".to_string()));

    let config_path = model_dir.join("config.json");
    let config: serde_json::Value = std::fs::read(&config_path)
        .map_err(|e| e.to_string())
        .and_then(|b| serde_json::from_slice(&b).map_err(|e| e.to_string()))
        .unwrap_or_else(|e| fail(format!("cannot read {}: {e}", config_path.display())));
    let keys: Vec<String> = config
        .get("type_index_keys")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();
    if keys.is_empty() {
        fail(format!(
            "{} has no type_index_keys, so there is nothing to freeze",
            config_path.display()
        ));
    }

    let taxonomy = Taxonomy::from_directory(&labels)
        .unwrap_or_else(|e| fail(format!("cannot load {}: {e}", labels.display())));
    let snapshot = ValidatorSnapshot::from_taxonomy(&taxonomy, &keys);
    if let Err(e) = snapshot.compile() {
        fail(format!("a rule does not compile: {e}"));
    }

    let out = model_dir.join(SNAPSHOT_FILE);
    std::fs::write(&out, snapshot.to_json_pretty() + "\n")
        .unwrap_or_else(|e| fail(format!("cannot write {}: {e}", out.display())));
    println!(
        "wrote {} ({} keys, {} rules)",
        out.display(),
        snapshot.keys.len(),
        snapshot.validations.len()
    );
}
