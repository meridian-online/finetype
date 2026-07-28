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

// ── Sibling-context serving ────────────────────────────────────────────────
//    The shipped model's header branch was trained behind frozen sibling-context
//    attention (finetype-train enriches each table group's headers before they
//    reach the branch, guarded by n_cols > 1). These tests hold serving to the
//    same condition. They deliberately assert on OUTPUT, not on wiring: the
//    failure they exist to catch is silently classifying un-enriched while every
//    `has_sibling_context()` check still says yes.
//
//    They run against the EMBEDDED model — `cargo test` runs with the crate dir
//    as cwd, so the relative `models/` lookups miss and the embedded bytes are
//    used. That is the path a released binary takes, and the path that used to
//    have no sibling-context module in it at all.

#[cfg(test)]
fn sibling_test_classifier() -> Option<finetype_model::ColumnClassifier> {
    let model = std::path::PathBuf::from("models/default");
    let cc_config = finetype_model::ColumnConfig {
        sample_size: 100,
        ..Default::default()
    };
    let mb = load_multi_branch_classifier(&model).ok()?;
    let mut cc = finetype_model::ColumnClassifier::with_multi_branch(mb, cc_config);
    wire_model2vec_and_siblings(&mut cc);
    Some(cc)
}

/// A four-column table whose columns have DIFFERENT lengths, so a result's
/// position can be checked against the column it belongs to.
#[cfg(test)]
fn sibling_test_table() -> Vec<(Vec<String>, &'static str)> {
    let v = |xs: &[&str]| -> Vec<String> { xs.iter().map(|s| s.to_string()).collect() };
    vec![
        (v(&["Paris", "London", "Berlin", "Madrid", "Rome"]), "city"),
        (v(&["Smith", "Jones", "Brown", "Davis"]), "name"),
        (v(&["a@b.com", "c@d.example", "e@f.org"]), "email"),
        (v(&["12.5", "88.25"]), "amount"),
    ]
}

#[test]
fn sibling_context_changes_the_emitted_record_on_a_multi_column_table() {
    let Some(cc) = sibling_test_classifier() else {
        eprintln!("no model available — skipping");
        return;
    };
    assert!(
        cc.has_sibling_context(),
        "the sibling-context module must be attached: neither models/sibling-context nor the \
         embedded copy resolved, so serving would silently fall back to raw headers"
    );

    let table = sibling_test_table();
    let inputs: Vec<(&[String], &str)> = table.iter().map(|(v, h)| (v.as_slice(), *h)).collect();
    let enriched = cc.classify_columns_with_context(&inputs).unwrap();
    let raw: Vec<_> = table
        .iter()
        .map(|(v, h)| cc.classify_column_with_header(v, h).unwrap())
        .collect();

    // At least one column's emitted record must move. If nothing moves, the
    // enriched header never reached the header branch and the whole path is
    // decorative — which is exactly the state this change was made to leave.
    let moved: Vec<&str> = table
        .iter()
        .enumerate()
        .filter(|(i, _)| {
            enriched[*i].label != raw[*i].label
                || (enriched[*i].confidence - raw[*i].confidence).abs() > 1e-3
        })
        .map(|(_, (_, h))| *h)
        .collect();
    assert!(
        !moved.is_empty(),
        "sibling enrichment changed nothing on a 4-column table; enriched={:?} raw={:?}",
        enriched
            .iter()
            .map(|r| (&r.label, r.confidence))
            .collect::<Vec<_>>(),
        raw.iter()
            .map(|r| (&r.label, r.confidence))
            .collect::<Vec<_>>()
    );
}

#[test]
fn a_one_column_table_is_served_un_enriched_exactly_as_training_saw_it() {
    let Some(cc) = sibling_test_classifier() else {
        eprintln!("no model available — skipping");
        return;
    };
    assert!(
        cc.has_sibling_context(),
        "sibling-context module not attached"
    );

    // The trainer's guard is `n_cols > 1`: a table group of one column had its
    // header features passed through unchanged. Enriching a lone column at serve
    // time would re-open the training/serving skew in the other direction.
    let (values, header) = sibling_test_table().remove(1); // "name" — the column that moves
    let one: Vec<(&[String], &str)> = vec![(values.as_slice(), header)];

    let via_context = cc.classify_columns_with_context(&one).unwrap();
    let direct = cc.classify_column_with_header(&values, header).unwrap();

    assert_eq!(via_context.len(), 1);
    assert_eq!(via_context[0].label, direct.label);
    assert_eq!(
        via_context[0].confidence.to_bits(),
        direct.confidence.to_bits(),
        "a single column must be classified bit-identically to the per-column path"
    );
    assert_eq!(
        via_context[0].disambiguation_rule, direct.disambiguation_rule,
        "the single-column path must emit the same record, rule included"
    );
}

#[test]
fn enriched_results_come_back_in_column_order() {
    let Some(cc) = sibling_test_classifier() else {
        eprintln!("no model available — skipping");
        return;
    };
    let table = sibling_test_table();
    let inputs: Vec<(&[String], &str)> = table.iter().map(|(v, h)| (v.as_slice(), *h)).collect();
    let results = cc.classify_columns_with_context(&inputs).unwrap();

    // samples_used is position-linked: each column here has a distinct length, so
    // a permuted collect cannot satisfy this. Label equality could not be used —
    // enrichment is allowed to move labels, which is the point of the path.
    let lengths: Vec<usize> = results.iter().map(|r| r.samples_used).collect();
    let expected: Vec<usize> = table.iter().map(|(v, _)| v.len()).collect();
    assert_eq!(
        lengths, expected,
        "each result must sit at its own column's index"
    );
}

#[test]
fn enrichment_responds_to_the_siblings_and_not_only_to_the_column() {
    let Some(cc) = sibling_test_classifier() else {
        eprintln!("no model available — skipping");
        return;
    };
    assert!(
        cc.has_sibling_context(),
        "sibling-context module not attached"
    );

    // Same column, same values, same header — two different tables around it.
    // Cross-column context is the entire claim of this path, so the record must
    // be ABLE to move when the siblings move. A header vector that ignores its
    // siblings (raw re-encode, or a zeroed stub) produces two identical records
    // here and fails, while still passing a "something changed vs raw" check.
    let v = |xs: &[&str]| -> Vec<String> { xs.iter().map(|s| s.to_string()).collect() };
    let subject = v(&["Smith", "Jones", "Brown", "Davis"]);

    let geo_ctx = v(&["Paris", "London", "Berlin", "Madrid"]);
    let geo_ctx2 = v(&["FR", "GB", "DE", "ES"]);
    let org_ctx = v(&["Acme Ltd", "Globex Inc", "Initech", "Umbrella"]);
    let org_ctx2 = v(&["94103", "10001", "60601", "33101"]);

    let in_geo: Vec<(&[String], &str)> = vec![
        (geo_ctx.as_slice(), "city"),
        (subject.as_slice(), "name"),
        (geo_ctx2.as_slice(), "country"),
    ];
    let in_org: Vec<(&[String], &str)> = vec![
        (org_ctx.as_slice(), "company"),
        (subject.as_slice(), "name"),
        (org_ctx2.as_slice(), "postcode"),
    ];

    let a = &cc.classify_columns_with_context(&in_geo).unwrap()[1];
    let b = &cc.classify_columns_with_context(&in_org).unwrap()[1];

    assert!(
        a.label != b.label || (a.confidence - b.confidence).abs() > 1e-6,
        "the same column emitted an identical record in two different tables — the header \
         branch is not seeing its siblings: geo={:?}/{} org={:?}/{}",
        a.label,
        a.confidence,
        b.label,
        b.confidence
    );
}

#[test]
fn a_columns_record_does_not_depend_on_where_it_sits_in_the_table() {
    let Some(cc) = sibling_test_classifier() else {
        eprintln!("no model available — skipping");
        return;
    };
    assert!(
        cc.has_sibling_context(),
        "sibling-context module not attached"
    );

    // The attention module carries no positional encoding, so an enriched row is
    // a function of its own header and the SET of sibling headers — not of the
    // column's index. Rotating the table must therefore leave every column's
    // record where it was.
    //
    // This is what pins enriched-row i to column i. `samples_used` order alone
    // does not: a mis-indexed enrichment still returns one result per column, at
    // the right index, carrying the wrong column's context.
    let v = |xs: &[&str]| -> Vec<String> { xs.iter().map(|s| s.to_string()).collect() };
    let a = v(&["Paris", "London", "Berlin", "Madrid"]);
    let b = v(&["Smith", "Jones", "Brown", "Davis"]);
    let c = v(&["a@b.com", "c@d.example", "e@f.org", "g@h.net"]);

    let straight: Vec<(&[String], &str)> = vec![
        (a.as_slice(), "city"),
        (b.as_slice(), "name"),
        (c.as_slice(), "email"),
    ];
    // Swap the FIRST TWO columns only. The permutation has to be non-cyclic: a
    // reversal is its own inverse and a rotation commutes with an index shift, so
    // either of those would let a mis-indexing bug satisfy this assertion. A
    // single transposition commutes with neither.
    let swapped: Vec<(&[String], &str)> = vec![
        (b.as_slice(), "name"),
        (a.as_slice(), "city"),
        (c.as_slice(), "email"),
    ];

    let r1 = cc.classify_columns_with_context(&straight).unwrap();
    let r2 = cc.classify_columns_with_context(&swapped).unwrap();

    // straight[i] is the same column as swapped[PERM[i]].
    const PERM: [usize; 3] = [1, 0, 2];
    for (i, header) in ["city", "name", "email"].iter().enumerate() {
        let x = &r1[i];
        let y = &r2[PERM[i]];
        assert_eq!(
            x.label, y.label,
            "`{header}` changed label when the columns were reordered: {} vs {}",
            x.label, y.label
        );
        assert!(
            (x.confidence - y.confidence).abs() < 1e-4,
            "`{header}` changed confidence when the columns were reordered: {} vs {}",
            x.confidence,
            y.confidence
        );
    }
}
