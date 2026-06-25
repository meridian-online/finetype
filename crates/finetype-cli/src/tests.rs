//! Tests for the CLI binary.

use super::*;
use std::fs;

#[test]
fn test_snapshot_skips_empty_dir() {
    let tmp = tempfile::tempdir().unwrap();
    let output = tmp.path().join("new_model");
    // Directory doesn't exist yet — no snapshot
    let result = snapshot_model_dir(&output).unwrap();
    assert!(result.is_none());
}

#[test]
fn test_snapshot_skips_dir_without_model_files() {
    let tmp = tempfile::tempdir().unwrap();
    let output = tmp.path().join("empty_model");
    fs::create_dir_all(&output).unwrap();
    fs::write(output.join("readme.txt"), "not a model").unwrap();
    // No model.safetensors or tier_graph.json — no snapshot
    let result = snapshot_model_dir(&output).unwrap();
    assert!(result.is_none());
}

#[test]
fn test_snapshot_flat_model() {
    let tmp = tempfile::tempdir().unwrap();
    let output = tmp.path().join("char-cnn");
    fs::create_dir_all(&output).unwrap();
    fs::write(output.join("model.safetensors"), "fake-weights").unwrap();
    fs::write(output.join("config.yaml"), "n_classes: 10").unwrap();
    fs::write(output.join("labels.json"), "[]").unwrap();

    let snapshot = snapshot_model_dir(&output).unwrap();
    assert!(snapshot.is_some());
    let snapshot_path = snapshot.unwrap();

    // Snapshot should contain the same files
    assert!(snapshot_path.join("model.safetensors").exists());
    assert!(snapshot_path.join("config.yaml").exists());
    assert!(snapshot_path.join("labels.json").exists());
    // Verify content is preserved
    assert_eq!(
        fs::read_to_string(snapshot_path.join("model.safetensors")).unwrap(),
        "fake-weights"
    );
    // Original should still exist
    assert!(output.join("model.safetensors").exists());
    // Snapshot name should contain "snapshot"
    let name = snapshot_path.file_name().unwrap().to_string_lossy();
    assert!(name.contains("snapshot"));
}

#[test]
fn test_snapshot_tiered_model() {
    let tmp = tempfile::tempdir().unwrap();
    let output = tmp.path().join("tiered-v2");
    let tier0 = output.join("tier0");
    fs::create_dir_all(&tier0).unwrap();
    fs::write(tier0.join("model.safetensors"), "tier0-weights").unwrap();
    fs::write(output.join("tier_graph.json"), "{}").unwrap();

    let snapshot = snapshot_model_dir(&output).unwrap();
    assert!(snapshot.is_some());
    let snapshot_path = snapshot.unwrap();

    // Nested structure should be preserved
    assert!(snapshot_path
        .join("tier0")
        .join("model.safetensors")
        .exists());
    assert!(snapshot_path.join("tier_graph.json").exists());
    assert_eq!(
        fs::read_to_string(snapshot_path.join("tier0").join("model.safetensors")).unwrap(),
        "tier0-weights"
    );
}

#[test]
fn test_copy_dir_recursive() {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");

    // Create nested structure
    fs::create_dir_all(src.join("sub1").join("sub2")).unwrap();
    fs::write(src.join("a.txt"), "file-a").unwrap();
    fs::write(src.join("sub1").join("b.txt"), "file-b").unwrap();
    fs::write(src.join("sub1").join("sub2").join("c.txt"), "file-c").unwrap();

    copy_dir_recursive(&src, &dst).unwrap();

    assert_eq!(fs::read_to_string(dst.join("a.txt")).unwrap(), "file-a");
    assert_eq!(
        fs::read_to_string(dst.join("sub1").join("b.txt")).unwrap(),
        "file-b"
    );
    assert_eq!(
        fs::read_to_string(dst.join("sub1").join("sub2").join("c.txt")).unwrap(),
        "file-c"
    );
}

#[test]
fn test_training_manifest_write() {
    let tmp = tempfile::tempdir().unwrap();
    let output = tmp.path().join("model");
    fs::create_dir_all(&output).unwrap();

    let manifest = TrainingManifest {
        output: &output,
        data_file: Path::new("training.ndjson"),
        epochs: 5,
        batch_size: 32,
        seed: Some(42),
        model_type: &ModelType::Tiered,
        n_classes: 171,
        n_samples: 17100,
        snapshot_path: Some(Path::new("models/tiered-v2.snapshot.20260228T120000Z")),
    };

    manifest.write().unwrap();

    let manifest_path = output.join("manifest.json");
    assert!(manifest_path.exists());

    let content: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&manifest_path).unwrap()).unwrap();
    assert_eq!(content["epochs"], 5);
    assert_eq!(content["batch_size"], 32);
    assert_eq!(content["seed"], 42);
    assert_eq!(content["model_type"], "tiered");
    assert_eq!(content["n_classes"], 171);
    assert_eq!(content["n_samples"], 17100);
    assert_eq!(content["data_file"], "training.ndjson");
    assert!(content["timestamp"].is_string());
    assert!(content["parent_snapshot"].is_string());
}

#[test]
fn test_training_manifest_no_seed_no_snapshot() {
    let tmp = tempfile::tempdir().unwrap();
    let output = tmp.path().join("model");
    fs::create_dir_all(&output).unwrap();

    let manifest = TrainingManifest {
        output: &output,
        data_file: Path::new("data.ndjson"),
        epochs: 10,
        batch_size: 64,
        seed: None,
        model_type: &ModelType::CharCnn,
        n_classes: 169,
        n_samples: 16900,
        snapshot_path: None,
    };

    manifest.write().unwrap();

    let content: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(output.join("manifest.json")).unwrap()).unwrap();
    assert!(content["seed"].is_null());
    assert!(content["parent_snapshot"].is_null());
    assert_eq!(content["model_type"], "charcnn");
}

// build_transform_projection + format_column_name unit tests live in
// crates/finetype-cli/tests/build_transform_projection.rs — they exercise
// the public surface via the lib.

// ── Shell-out ingestion (choice 0100) ──────────────────────────────────────
//    read_csv_input now invokes the external `duckdb` CLI. These tests prove
//    parity with the old csv-crate reader's contract (headers, per-column
//    Vec<String>, row_count, and identical null-ish filtering). They skip
//    gracefully when duckdb is not on PATH (CI matrices without it).

/// True when the `duckdb` CLI is invokable. Tests below skip when it is not.
fn duckdb_available() -> bool {
    std::process::Command::new("duckdb")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn write_tmp_csv(contents: &str) -> tempfile::NamedTempFile {
    use std::io::Write as _;
    let mut f = tempfile::Builder::new().suffix(".csv").tempfile().unwrap();
    f.write_all(contents.as_bytes()).unwrap();
    f.flush().unwrap();
    f
}

#[test]
fn test_ingest_basic_headers_columns_rowcount() {
    if !duckdb_available() {
        eprintln!("duckdb not on PATH — skipping ingestion test");
        return;
    }
    let f = write_tmp_csv("a,b,c\n1,2,3\n4,5,6\n");
    let (headers, columns, row_count) = profile_io::read_csv_input(f.path(), None).unwrap();
    assert_eq!(headers, vec!["a", "b", "c"]);
    assert_eq!(row_count, 2);
    assert_eq!(columns[0], vec!["1", "4"]);
    assert_eq!(columns[1], vec!["2", "5"]);
    assert_eq!(columns[2], vec!["3", "6"]);
}

#[test]
fn test_ingest_nullish_filtering() {
    if !duckdb_available() {
        eprintln!("duckdb not on PATH — skipping ingestion test");
        return;
    }
    // Each null-ish token must be dropped from the column, exactly as the old
    // csv-crate reader did. (Token rows are still rows — only their *values*
    // are filtered; a fully-blank line is a separate case, see below.)
    let f = write_tmp_csv("x\nreal\nNULL\nnull\nNA\nN/A\nnan\nNaN\nNone\nkept\n");
    let (_headers, columns, row_count) = profile_io::read_csv_input(f.path(), None).unwrap();
    assert_eq!(row_count, 9, "nine data rows (8 null-ish tokens + 1 kept)");
    assert_eq!(
        columns[0],
        vec!["real", "kept"],
        "all null-ish tokens dropped from values"
    );
}

#[test]
fn test_ingest_blank_line_skipped() {
    if !duckdb_available() {
        eprintln!("duckdb not on PATH — skipping ingestion test");
        return;
    }
    // KNOWN BENIGN DIFFERENCE vs the old csv-crate reader: duckdb's CSV reader
    // skips a fully-blank line entirely (it is not a zero-field record), so it
    // does NOT count toward row_count. The old reader counted a blank line as
    // an empty row. This affects only the eprintln'd row_count diagnostic — a
    // blank line contributed no values either way, so per-column profiling and
    // emitted labels are unaffected.
    let f = write_tmp_csv("x\nreal\n\nkept\n");
    let (_headers, columns, row_count) = profile_io::read_csv_input(f.path(), None).unwrap();
    assert_eq!(row_count, 2, "blank line not counted (duckdb skips it)");
    assert_eq!(columns[0], vec!["real", "kept"]);
}

#[test]
fn test_ingest_quoted_fields_with_commas() {
    if !duckdb_available() {
        eprintln!("duckdb not on PATH — skipping ingestion test");
        return;
    }
    let f = write_tmp_csv("name,note\n\"Smith, John\",\"a, b, c\"\n\"plain\",\"x\"\n");
    let (headers, columns, _row_count) = profile_io::read_csv_input(f.path(), None).unwrap();
    assert_eq!(headers, vec!["name", "note"]);
    assert_eq!(columns[0], vec!["Smith, John", "plain"]);
    assert_eq!(columns[1], vec!["a, b, c", "x"]);
}

#[test]
fn test_ingest_ragged_rows_padded() {
    if !duckdb_available() {
        eprintln!("duckdb not on PATH — skipping ingestion test");
        return;
    }
    // A short row: duckdb null_padding pads the missing trailing field, which
    // is then dropped as null-ish — matching the old reader (which simply had
    // no field at that index to push).
    let f = write_tmp_csv("a,b,c\n1,2,3\n4,5\n");
    let (headers, columns, row_count) = profile_io::read_csv_input(f.path(), None).unwrap();
    assert_eq!(headers.len(), 3);
    assert_eq!(row_count, 2);
    assert_eq!(columns[0], vec!["1", "4"]);
    assert_eq!(columns[1], vec!["2", "5"]);
    assert_eq!(columns[2], vec!["3"], "missing trailing field dropped");
}

#[test]
fn test_ingest_explicit_delimiter() {
    if !duckdb_available() {
        eprintln!("duckdb not on PATH — skipping ingestion test");
        return;
    }
    let f = write_tmp_csv("a;b;c\n1;2;3\n4;5;6\n");
    let (headers, columns, row_count) = profile_io::read_csv_input(f.path(), Some(';')).unwrap();
    assert_eq!(headers, vec!["a", "b", "c"]);
    assert_eq!(row_count, 2);
    assert_eq!(columns[1], vec!["2", "5"]);
}

#[test]
fn test_ingest_values_are_trimmed() {
    if !duckdb_available() {
        eprintln!("duckdb not on PATH — skipping ingestion test");
        return;
    }
    // Leading/trailing whitespace inside quoted fields is trimmed, matching the
    // old reader's `field.trim()`.
    let f = write_tmp_csv("v\n\"  spaced  \"\n\"   \"\n");
    let (_headers, columns, _row_count) = profile_io::read_csv_input(f.path(), None).unwrap();
    // First value trims to "spaced"; the all-whitespace value trims to empty
    // and is dropped as null-ish.
    assert_eq!(columns[0], vec!["spaced"]);
}
