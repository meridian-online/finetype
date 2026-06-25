//! Tests for the CLI binary.

use super::*;

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
