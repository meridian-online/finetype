//! `finetype generate` leaves the `generate: on_request` leaves out of its
//! default output and puts them back under `--all`.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Command;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn labels_of(args: &[&str]) -> BTreeSet<String> {
    let out = tempfile::NamedTempFile::new().unwrap();
    let status = Command::new(env!("CARGO_BIN_EXE_finetype"))
        .current_dir(workspace_root())
        .args(["generate", "--samples", "1", "--output"])
        .arg(out.path())
        .args(args)
        .status()
        .unwrap();
    assert!(status.success());
    std::fs::read_to_string(out.path())
        .unwrap()
        .lines()
        .map(|l| {
            let v: serde_json::Value = serde_json::from_str(l).unwrap();
            v["classification"].as_str().unwrap().to_string()
        })
        .collect()
}

const ON_REQUEST: [&str; 3] = [
    "container.object.s_expression",
    "identity.person.password",
    "representation.text.plain_text",
];

#[test]
fn default_output_skips_on_request_leaves_and_all_puts_them_back() {
    let default = labels_of(&[]);
    let all = labels_of(&["--all"]);
    for key in ON_REQUEST {
        assert!(!default.contains(key), "{key} in default output");
        assert!(all.contains(key), "{key} missing under --all");
    }
    assert_eq!(all.len(), default.len() + ON_REQUEST.len());
}

#[test]
fn priority_flag_is_gone() {
    let status = Command::new(env!("CARGO_BIN_EXE_finetype"))
        .args(["generate", "--priority", "0"])
        .status()
        .unwrap();
    assert!(!status.success());
}
