//! The `write_validation_snapshot` bin: write, `--check`, `--check-keys` and the
//! two key-file formats.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const KEYS: [&str; 2] = ["technology.identifier.tsid", "identity.medical.npi"];

fn labels_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../labels")
}

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_write_validation_snapshot"))
        .args(args)
        .arg("--labels")
        .arg(labels_dir())
        .output()
        .expect("run bin")
}

fn s(p: &Path) -> &str {
    p.to_str().unwrap()
}

fn write_keys(dir: &Path, name: &str, wrapped: bool) -> PathBuf {
    let path = dir.join(name);
    let keys = serde_json::json!(KEYS);
    let body = if wrapped {
        serde_json::json!({ "type_index_keys": keys })
    } else {
        keys
    };
    std::fs::write(&path, body.to_string()).unwrap();
    path
}

#[test]
fn write_check_and_check_keys() {
    let dir = tempfile::tempdir().unwrap();
    let bare = write_keys(dir.path(), "bare.json", false);
    let wrapped = write_keys(dir.path(), "config.json", true);
    let snap = dir.path().join("snap.json");

    // Both key-file formats write the same snapshot.
    assert!(run(&["--keys-from", s(&bare), "--out", s(&snap)])
        .status
        .success());
    let first = std::fs::read(&snap).unwrap();
    assert!(run(&["--keys-from", s(&wrapped), "--out", s(&snap)])
        .status
        .success());
    assert_eq!(first, std::fs::read(&snap).unwrap());

    // --check matches a fresh snapshot, and --check-keys matches the keys.
    assert!(run(&["--keys-from", s(&bare), "--check", s(&snap)])
        .status
        .success());
    assert!(run(&["--keys-from", s(&bare), "--check-keys", s(&snap)])
        .status
        .success());

    // A model directory supplies keys from its config.json.
    let model = dir.path().join("model");
    std::fs::create_dir(&model).unwrap();
    std::fs::copy(&wrapped, model.join("config.json")).unwrap();
    assert!(run(&[s(&model), "--check", s(&snap)]).status.success());
    assert!(run(&[s(&model), "--check-keys", s(&snap)]).status.success());

    // Editing one validation makes --check fail but leaves --check-keys passing.
    let mut value: serde_json::Value = serde_json::from_slice(&first).unwrap();
    let rule = value["validations"]["technology.identifier.tsid"]
        .as_object_mut()
        .expect("tsid rule");
    rule.insert("pattern".into(), serde_json::json!("^edited$"));
    let edited = dir.path().join("edited.json");
    std::fs::write(&edited, value.to_string()).unwrap();
    assert!(!run(&["--keys-from", s(&bare), "--check", s(&edited)])
        .status
        .success());
    assert!(run(&["--keys-from", s(&bare), "--check-keys", s(&edited)])
        .status
        .success());

    // Different keys fail --check-keys.
    let other = dir.path().join("other.json");
    std::fs::write(&other, r#"["technology.identifier.tsid"]"#).unwrap();
    assert!(!run(&["--keys-from", s(&other), "--check-keys", s(&snap)])
        .status
        .success());

    // A missing snapshot fails --check.
    let missing = dir.path().join("missing.json");
    assert!(!run(&["--keys-from", s(&bare), "--check", s(&missing)])
        .status
        .success());
}

#[test]
fn keys_from_without_out_fails() {
    let dir = tempfile::tempdir().unwrap();
    let bare = write_keys(dir.path(), "bare.json", false);
    assert!(!run(&["--keys-from", s(&bare)]).status.success());
}
