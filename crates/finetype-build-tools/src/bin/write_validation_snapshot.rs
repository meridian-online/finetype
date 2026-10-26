//! Freeze a model's validation rules into `<model_dir>/validation_snapshot.json`.
//!
//! The snapshot records the validation block of every key in the model's
//! `config.json` `type_index_keys`, as the taxonomy has them now, so the model
//! keeps reading the rules it was shipped with when `labels/` later changes.
//! A rule that does not compile is an error here, at write time, rather than a
//! load failure for whoever receives the model.
//!
//! Usage:
//!
//! ```text
//! write_validation_snapshot <model_dir> [--labels labels]
//! write_validation_snapshot --keys-from <file> --out <path> [--labels labels]
//! write_validation_snapshot (<model_dir> | --keys-from <file>) --check <path> [--labels labels]
//! write_validation_snapshot (<model_dir> | --keys-from <file>) --check-keys <path>
//! ```
//!
//! `--keys-from` reads the keys from a file instead of a model's `config.json`:
//! a JSON array of keys, or an object with a `type_index_keys` array. `--out`
//! writes there instead of into the model directory. `--check` writes nothing; it
//! exits non-zero unless the snapshot at that path equals a fresh snapshot of the
//! live labels for the same keys. A training script uses these to freeze the
//! rules when it builds its training data and to refuse data built under other
//! rules. `--check-keys` compares only the snapshot's `keys` with the given keys,
//! reading no labels, for a model trained under rules that may since have changed.

use finetype_core::validator_snapshot::{ValidatorSnapshot, SNAPSHOT_FILE};
use finetype_core::Taxonomy;
use std::path::{Path, PathBuf};
use std::process;

fn fail(message: String) -> ! {
    eprintln!("write_validation_snapshot: {message}");
    process::exit(1);
}

fn read_keys(path: &Path) -> Vec<String> {
    let value: serde_json::Value = std::fs::read(path)
        .map_err(|e| e.to_string())
        .and_then(|b| serde_json::from_slice(&b).map_err(|e| e.to_string()))
        .unwrap_or_else(|e| fail(format!("cannot read {}: {e}", path.display())));
    let keys = value.get("type_index_keys").unwrap_or(&value);
    let keys: Vec<String> = serde_json::from_value(keys.clone()).unwrap_or_default();
    if keys.is_empty() {
        fail(format!(
            "{} has no type_index_keys, so there is nothing to freeze",
            path.display()
        ));
    }
    keys
}

fn main() {
    const USAGE: &str = "usage: write_validation_snapshot (<model_dir> | --keys-from <file> --out <path>) [--labels labels] [--check <path>]";
    let mut model_dir: Option<PathBuf> = None;
    let mut labels = PathBuf::from("labels");
    let mut keys_from: Option<PathBuf> = None;
    let mut out: Option<PathBuf> = None;
    let mut check: Option<PathBuf> = None;
    let mut check_keys: Option<PathBuf> = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let mut path_arg = |flag: &str| {
            args.next()
                .map(PathBuf::from)
                .unwrap_or_else(|| fail(format!("{flag} needs a path")))
        };
        match arg.as_str() {
            "--labels" => labels = path_arg("--labels"),
            "--keys-from" => keys_from = Some(path_arg("--keys-from")),
            "--out" => out = Some(path_arg("--out")),
            "--check" => check = Some(path_arg("--check")),
            "--check-keys" => check_keys = Some(path_arg("--check-keys")),
            _ if model_dir.is_none() && !arg.starts_with("--") => {
                model_dir = Some(PathBuf::from(arg));
            }
            _ => fail(format!("{USAGE} (got {arg})")),
        }
    }

    let keys = match (&keys_from, &model_dir) {
        (Some(file), None) => read_keys(file),
        (None, Some(dir)) => read_keys(&dir.join("config.json")),
        _ => fail(format!("give a model directory or --keys-from. {USAGE}")),
    };

    if let Some(path) = check_keys {
        let committed: serde_json::Value = std::fs::read(&path)
            .map_err(|e| e.to_string())
            .and_then(|b| serde_json::from_slice(&b).map_err(|e| e.to_string()))
            .unwrap_or_else(|e| fail(format!("cannot read {}: {e}", path.display())));
        if committed.get("keys") != Some(&serde_json::json!(keys)) {
            fail(format!(
                "the keys in {} differ from the model's type_index_keys",
                path.display()
            ));
        }
        println!("{} holds the model's keys", path.display());
        return;
    }

    let taxonomy = Taxonomy::from_directory(&labels)
        .unwrap_or_else(|e| fail(format!("cannot load {}: {e}", labels.display())));
    let snapshot = ValidatorSnapshot::from_taxonomy(&taxonomy, &keys);
    if let Err(e) = snapshot.compile() {
        fail(format!("a rule does not compile: {e}"));
    }

    if let Some(path) = check {
        let read = |text: &str| serde_json::from_str::<serde_json::Value>(text).ok();
        let committed = std::fs::read_to_string(&path)
            .ok()
            .and_then(|t| read(&t))
            .unwrap_or_else(|| fail(format!("cannot read a snapshot at {}", path.display())));
        if Some(committed) != read(&snapshot.to_json_pretty()) {
            fail(format!(
                "{} differs from a fresh snapshot of {} for the same keys",
                path.display(),
                labels.display()
            ));
        }
        println!("{} matches {}", path.display(), labels.display());
        return;
    }

    let out = match (out, &model_dir) {
        (Some(path), _) => path,
        (None, Some(dir)) => dir.join(SNAPSHOT_FILE),
        (None, None) => fail(format!("--keys-from needs --out. {USAGE}")),
    };
    std::fs::write(&out, snapshot.to_json_pretty() + "\n")
        .unwrap_or_else(|e| fail(format!("cannot write {}: {e}", out.display())));
    println!(
        "wrote {} ({} keys, {} rules)",
        out.display(),
        snapshot.keys.len(),
        snapshot.validations.len()
    );
}
