//! Column ORDER is part of the contract, not a presentation detail.
//!
//! `classify_columns_with_context` returns one result per input column and every
//! caller zips those results back against its own column list BY POSITION — the
//! CLI's profile writer, the eval fixtures that read the emitted columns
//! positionally, the DuckDB extension. The loop is parallel, and a parallel
//! iterator that collects into an unordered container passes a set comparison
//! while silently permuting the sequence, so these tests compare SEQUENCES.

use super::super::*;
use std::collections::HashSet;

fn v(xs: &[&str]) -> Vec<String> {
    xs.iter().map(|s| s.to_string()).collect()
}

/// Columns chosen so each lands on a DIFFERENT label. A fixture whose columns
/// all classify the same way would satisfy any permutation, so the test would
/// pass on reordered output — `labels_are_distinguishable` below refuses that.
fn distinguishable_columns() -> Vec<(Vec<String>, String)> {
    vec![
        (
            v(&[
                "ada@example.com",
                "grace@example.org",
                "alan@example.net",
                "edsger@example.com",
            ]),
            "email".to_string(),
        ),
        (
            v(&[
                "2024-01-05T09:30:00Z",
                "2024-02-11T18:04:22Z",
                "2024-03-30T00:00:01Z",
                "2024-04-01T12:12:12Z",
            ]),
            "created_at".to_string(),
        ),
        (
            v(&["192.168.0.1", "10.0.0.255", "172.16.4.9", "8.8.8.8"]),
            "ip_address".to_string(),
        ),
        (
            v(&[
                "550e8400-e29b-41d4-a716-446655440000",
                "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
                "6ba7b811-9dad-11d1-80b4-00c04fd430c8",
                "6ba7b812-9dad-11d1-80b4-00c04fd430c8",
            ]),
            "uuid".to_string(),
        ),
        (
            v(&[
                "+14155552671",
                "+442071838750",
                "+61285993000",
                "+81312345678",
            ]),
            "phone_number".to_string(),
        ),
        (
            v(&["#ff0000", "#00ff00", "#0000ff", "#123abc"]),
            "colour".to_string(),
        ),
        (
            v(&[
                "https://a.example.com",
                "https://b.example.org",
                "https://c.example.net",
                "https://d.example.io",
            ]),
            "homepage".to_string(),
        ),
        (v(&["12.5", "88.25", "3.75", "0.5"]), "amount".to_string()),
    ]
}

fn classifier() -> ColumnClassifier {
    ColumnClassifier::with_defaults(Box::new(crate::inference::MockClassifier::new("unknown")))
}

fn batch_labels(cc: &ColumnClassifier, columns: &[(Vec<String>, String)]) -> Vec<String> {
    cc.classify_columns_with_context(columns)
        .unwrap()
        .into_iter()
        .map(|r| r.label)
        .collect()
}

fn sequential_labels(cc: &ColumnClassifier, columns: &[(Vec<String>, String)]) -> Vec<String> {
    columns
        .iter()
        .map(|(values, header)| {
            cc.classify_column_with_header(values, header)
                .unwrap()
                .label
        })
        .collect()
}

#[test]
fn labels_are_distinguishable() {
    // Guards every other test in this file: if the fixture stopped producing
    // distinct labels, the sequence assertions would hold under any permutation
    // and would no longer be testing order at all.
    let cc = classifier();
    let columns = distinguishable_columns();
    let labels = sequential_labels(&cc, &columns);
    let distinct: HashSet<&String> = labels.iter().collect();
    assert!(
        distinct.len() >= 4,
        "fixture must classify to several different labels or the order tests are vacuous; got {labels:?}"
    );
}

#[test]
fn classify_columns_with_context_returns_results_in_input_order() {
    let cc = classifier();
    let columns = distinguishable_columns();

    // SEQUENCE equality, position for position. A permutation of the same
    // labels — what an unordered parallel collect produces — fails here and
    // would pass a set comparison.
    assert_eq!(
        batch_labels(&cc, &columns),
        sequential_labels(&cc, &columns),
        "batch classification must return each column's result at that column's index"
    );
}

#[test]
fn column_order_survives_a_reordering_of_the_input() {
    // Same columns, reversed. If the results were being emitted in some
    // schedule-determined order rather than input order, one of the two
    // directions would disagree with its own input.
    let cc = classifier();
    let mut columns = distinguishable_columns();
    columns.reverse();

    assert_eq!(
        batch_labels(&cc, &columns),
        sequential_labels(&cc, &columns),
        "reversed input must produce reversed output"
    );
}

#[test]
fn column_order_is_stable_across_repeated_runs() {
    // Thread scheduling varies run to run; the answer must not.
    let cc = classifier();
    let columns = distinguishable_columns();
    let first = batch_labels(&cc, &columns);
    for _ in 0..8 {
        assert_eq!(
            batch_labels(&cc, &columns),
            first,
            "repeated batch classification produced a different sequence"
        );
    }
}
