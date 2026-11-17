//! Tests for the CLI binary.

use super::*;

// build_transform_projection + format_column_name unit tests live in
// crates/finetype-cli/tests/build_transform_projection.rs — they exercise
// the public surface via the lib.

// ── Shell-out ingestion (choice 0100) ──────────────────────────────────────
//    read_csv_input now invokes the external `duckdb` CLI. These tests prove
//    parity with the old csv-crate reader's contract (headers, per-column
//    Vec<String>, row_count; only NULL counts as missing). They skip
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
fn test_ingest_keeps_literal_null_like_tokens() {
    if !duckdb_available() {
        eprintln!("duckdb not on PATH — skipping ingestion test");
        return;
    }
    // A value is missing only when it is NULL (an empty cell here). Literal
    // tokens such as `NA` are data: `NA` is North America's continent code
    // and Namibia's country code.
    let f = write_tmp_csv("x\nreal\nNULL\nnull\nNA\nN/A\nnan\nNaN\nNone\n\nkept\n");
    let (_headers, columns, _row_count) = profile_io::read_csv_input(f.path(), None).unwrap();
    assert_eq!(
        columns[0],
        vec!["real", "NULL", "null", "NA", "N/A", "nan", "NaN", "None", "kept"],
        "literal null-like tokens are kept as values"
    );
}

#[test]
fn test_ingest_parquet_keeps_literal_na_and_drops_null() {
    if !duckdb_available() {
        eprintln!("duckdb not on PATH — skipping ingestion test");
        return;
    }
    // The OurAirports shape: a VARCHAR continent column where 'NA' is North
    // America, alongside a genuine NULL. Only the NULL is missing.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("continents.parquet");
    let sql = format!(
        "COPY (SELECT * FROM (VALUES ('EGLL', 'EU'), ('KJFK', 'NA'), ('XXXX', NULL), \
         ('CYYZ', 'NA'), ('RJTT', 'AS')) t(ident, continent)) \
         TO '{}' (FORMAT parquet);",
        path.display()
    );
    let status = std::process::Command::new("duckdb")
        .arg("-c")
        .arg(&sql)
        .status()
        .unwrap();
    assert!(status.success(), "duckdb wrote the parquet fixture");
    let (headers, columns, row_count) = profile_io::read_csv_input(&path, None).unwrap();
    assert_eq!(headers, vec!["ident", "continent"]);
    assert_eq!(row_count, 5);
    assert_eq!(columns[1], vec!["EU", "NA", "NA", "AS"]);
}

#[test]
fn test_ingest_one_column_null_row_is_counted() {
    if !duckdb_available() {
        eprintln!("duckdb not on PATH — skipping ingestion test");
        return;
    }
    // With a single column, a NULL row would re-emit as an empty line, which
    // the csv crate skips. The row still exists, so it must count: the null
    // rate and the enum domain's row count are measured over it.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("one_column.parquet");
    let sql = format!(
        "COPY (SELECT * FROM (VALUES ('EU'), ('NA'), (NULL)) t(continent)) \
         TO '{}' (FORMAT parquet);",
        path.display()
    );
    let status = std::process::Command::new("duckdb")
        .arg("-c")
        .arg(&sql)
        .status()
        .unwrap();
    assert!(status.success(), "duckdb wrote the parquet fixture");
    let (headers, columns, row_count) = profile_io::read_csv_input(&path, None).unwrap();
    assert_eq!(headers, vec!["continent"]);
    assert_eq!(row_count, 3, "the NULL row is counted");
    assert_eq!(columns[0], vec!["EU", "NA"]);
}

#[test]
fn test_ingest_one_column_blank_line_is_a_missing_row() {
    if !duckdb_available() {
        eprintln!("duckdb not on PATH — skipping ingestion test");
        return;
    }
    // In a one-column CSV a blank line is the only way to write an empty
    // cell, and duckdb reads it as a row whose value is NULL. The row counts;
    // its value is missing. (In a multi-column CSV duckdb skips a blank line
    // altogether, so it neither counts nor contributes a value.)
    let f = write_tmp_csv("x\nreal\n\nkept\n");
    let (_headers, columns, row_count) = profile_io::read_csv_input(f.path(), None).unwrap();
    assert_eq!(row_count, 3, "the blank line is a row with a missing value");
    assert_eq!(columns[0], vec!["real", "kept"]);
}

#[test]
fn test_ingest_multi_column_blank_line_skipped() {
    if !duckdb_available() {
        eprintln!("duckdb not on PATH — skipping ingestion test");
        return;
    }
    let f = write_tmp_csv("x,y\n1,2\n\n3,4\n");
    let (_headers, columns, row_count) = profile_io::read_csv_input(f.path(), None).unwrap();
    assert_eq!(
        row_count, 2,
        "duckdb skips a blank line in a multi-column file"
    );
    assert_eq!(columns[0], vec!["1", "3"]);
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
    // is then dropped as missing — matching the old reader (which simply had
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
    // and is dropped as missing.
    assert_eq!(columns[0], vec!["spaced"]);
}

// ── Default model's validation snapshot ────────────────────────────────────

#[cfg(feature = "embed-models")]
#[test]
fn default_model_ships_a_matching_snapshot() {
    // The snapshot is a tracked file of the default model: build.rs embeds it, and
    // the binary must apply exactly the rules that sit beside config.json.
    let on_disk = std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../models/default")
            .join(finetype_core::validator_snapshot::SNAPSHOT_FILE),
    )
    .expect("the default model ships a validation snapshot");
    assert_eq!(embedded::MB_VALIDATION_SNAPSHOT, on_disk.as_slice());

    let snapshot = finetype_core::validator_snapshot::ValidatorSnapshot::from_json(
        embedded::MB_VALIDATION_SNAPSHOT,
    )
    .unwrap();
    let config: serde_json::Value = serde_json::from_slice(embedded::MB_CONFIG).unwrap();
    let expected: Vec<String> = serde_json::from_value(config["type_index_keys"].clone()).unwrap();
    assert_eq!(snapshot.keys().cloned().collect::<Vec<_>>(), expected);
}
