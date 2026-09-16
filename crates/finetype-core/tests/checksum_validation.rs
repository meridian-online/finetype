//! `finetype validate` verifies check digits, not only shape.
//!
//! Before this suite existed, the `validate` verb confirmed a column's shape and
//! called it validation: a 20-character alphanumeric string that is not an LEI,
//! or a 13-digit number that is not an ISBN, cleared the gate and the analyst
//! stopped double-checking. The taxonomy had declared the arithmetic all along —
//! a `checksum:` directive on nineteen leaves — and `finetype_core::checksum`
//! had held it all along; nothing on the validate path read either.
//!
//! What is pinned here:
//!
//! * **Every** leaf carrying the directive is check-digit verified by
//!   `validate_table`, not a hand-picked four. The cases are derived from
//!   `CHECKSUM_LABELS`, so a leaf added to the taxonomy without a case here
//!   reddens rather than going unverified.
//! * The verification is switched on by the column schema's `x-finetype-label`
//!   extension and by nothing else — the same value under a schema without the
//!   extension still validates clean. That is the seam the model path is kept on
//!   the other side of.
//! * A check-digit failure is reported as `constraint_failed = "checksum"`,
//!   distinct from `pattern`, because the value HAS the leaf's shape. If the two
//!   tokens were the same, a consumer counting rejects by constraint could not
//!   tell "wrong format" from "not actually this identifier".
//!
//! The per-leaf values are not hand-written. For each leaf the suite searches
//! single-character mutations of the taxonomy's own samples for two values the
//! engine itself agrees are shape-valid: one whose check digit verifies and one
//! whose check digit does not. A leaf for which the search cannot produce BOTH
//! fails the test by name — a skip would read exactly like a pass.

use finetype_core::checksum::{self, CHECKSUM_LABELS};
use finetype_core::table_validator::{validate_table, RejectRecord};
use finetype_core::taxonomy::Taxonomy;
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::path::PathBuf;

/// Workspace root, from this crate's manifest dir (`crates/finetype-core`).
fn workspace_root() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop(); // -> crates/
    p.pop(); // -> workspace root
    p
}

fn taxonomy() -> Taxonomy {
    Taxonomy::from_directory(&workspace_root().join("labels"))
        .expect("load taxonomy from labels/ — is labels/ present in the workspace?")
}

/// A one-column table schema carrying `label`'s validation fragment.
///
/// `with_label` decides whether the `x-finetype-label` extension is written. It
/// is the only difference between the two schemas, and therefore the only thing
/// that can account for a difference in the rejects.
fn schema_for(taxonomy: &Taxonomy, label: &str, with_label: bool) -> Value {
    let validation = taxonomy
        .get(label)
        .unwrap_or_else(|| panic!("taxonomy has no definition for {label}"))
        .validation
        .as_ref()
        .unwrap_or_else(|| panic!("{label} carries a checksum: directive but no validation"));
    let mut prop = validation.to_json_schema();
    if with_label {
        prop.as_object_mut()
            .expect("validation JSON Schema is an object")
            .insert("x-finetype-label".into(), json!(label));
    }
    json!({ "type": "object", "properties": { "id": prop } })
}

/// Validate `values` as a one-column table and return the reject sidecar rows.
fn rejects_for(schema: &Value, values: &[&str]) -> Vec<RejectRecord> {
    let headers = vec!["id".to_string()];
    let rows: Vec<Vec<Option<String>>> = values
        .iter()
        .map(|v| vec![Some((*v).to_string())])
        .collect();
    validate_table(&headers, &rows, schema)
        .expect("validate_table accepts a one-column object schema")
        .rejects
}

/// Single-character mutations of `seed`, drawn from the alphabet the label's own
/// samples use plus the decimal digits.
///
/// Deliberately blunt: a check digit is a function of every other character, so
/// changing one character is enough to break it, and restricting the alphabet to
/// characters the samples already use keeps the mutants inside the leaf's shape
/// often enough for the search to land.
fn mutants(seed: &str, alphabet: &[char]) -> Vec<String> {
    let chars: Vec<char> = seed.chars().collect();
    let mut out = Vec::new();
    for i in 0..chars.len() {
        for c in alphabet {
            if chars[i] == *c {
                continue;
            }
            let mut m = chars.clone();
            m[i] = *c;
            out.push(m.into_iter().collect::<String>());
        }
    }
    out
}

/// The characters the label's samples are written from, plus `0`–`9`.
fn alphabet_from(samples: &[String]) -> Vec<char> {
    let mut set: BTreeSet<char> = samples.iter().flat_map(|s| s.chars()).collect();
    set.extend('0'..='9');
    set.into_iter().collect()
}

fn samples_of(taxonomy: &Taxonomy, label: &str) -> Vec<String> {
    taxonomy
        .get(label)
        .unwrap_or_else(|| panic!("taxonomy has no definition for {label}"))
        .samples
        .iter()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect()
}

/// A value the engine agrees has the leaf's shape whose check digit verifies,
/// and one whose check digit does not.
///
/// "The engine agrees" is load-bearing: shape is decided by running the value
/// through `validate_table` under the label-free schema and requiring zero
/// rejects, so a value that would have failed for `pattern` can never be
/// mistaken here for one that failed for `checksum`.
fn shaped_pair(
    taxonomy: &Taxonomy,
    label: &str,
    verify: fn(&str) -> bool,
) -> (Option<String>, Option<String>) {
    let shape_only = schema_for(taxonomy, label, false);
    let samples = samples_of(taxonomy, label);
    let alphabet = alphabet_from(&samples);
    let mut good = None;
    let mut bad = None;

    let consider = |v: &str, good: &mut Option<String>, bad: &mut Option<String>| {
        if good.is_some() && bad.is_some() {
            return;
        }
        if !rejects_for(&shape_only, &[v]).is_empty() {
            return; // not this leaf's shape — nothing to say about its check digit
        }
        if verify(v) {
            if good.is_none() {
                *good = Some(v.to_string());
            }
        } else if bad.is_none() {
            *bad = Some(v.to_string());
        }
    };

    for s in &samples {
        consider(s, &mut good, &mut bad);
    }
    for s in &samples {
        if good.is_some() && bad.is_some() {
            break;
        }
        for m in mutants(s, &alphabet) {
            consider(&m, &mut good, &mut bad);
            if good.is_some() && bad.is_some() {
                break;
            }
        }
    }
    (good, bad)
}

// ═══════════════════════════════════════════════════════════════════════════
// AC1 — every checksum-bearing leaf is verified, not four of them
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn every_checksum_leaf_rejects_a_shape_valid_value_with_a_bad_check_digit() {
    let taxonomy = taxonomy();
    assert!(
        !CHECKSUM_LABELS.is_empty(),
        "CHECKSUM_LABELS is empty — this test would pass over nothing"
    );

    for (label, scheme) in CHECKSUM_LABELS {
        let verify = checksum::resolve(scheme)
            .unwrap_or_else(|| panic!("{label}: scheme {scheme:?} does not resolve"));
        let (good, bad) = shaped_pair(&taxonomy, label, verify);

        let good = good.unwrap_or_else(|| {
            panic!("{label}: found no shape-valid value whose {scheme} check digit verifies")
        });
        let bad = bad.unwrap_or_else(|| {
            panic!("{label}: found no shape-valid value whose {scheme} check digit fails")
        });

        let labelled = schema_for(&taxonomy, label, true);

        // The genuine identifier still validates clean.
        assert!(
            rejects_for(&labelled, &[&good]).is_empty(),
            "{label}: {good:?} carries a valid {scheme} check digit and must validate clean"
        );

        // The lookalike is rejected, and rejected AS a checksum failure.
        let rejects = rejects_for(&labelled, &[&bad]);
        assert_eq!(
            rejects.len(),
            1,
            "{label}: {bad:?} should produce exactly one reject, got {rejects:?}"
        );
        assert_eq!(
            rejects[0].constraint_failed, "checksum",
            "{label}: {bad:?} has the leaf's shape, so the reject must name the check digit, \
             not {:?}",
            rejects[0].constraint_failed
        );
        assert_eq!(
            rejects[0].constraint_value.as_deref(),
            Some(*scheme),
            "{label}: the reject should name the scheme that refused it"
        );
        assert_eq!(rejects[0].value.as_deref(), Some(bad.as_str()));

        // Without the label extension the same value is clean: the taxonomy
        // label is the whole switch, which is what keeps the model path out.
        assert!(
            rejects_for(&schema_for(&taxonomy, label, false), &[&bad]).is_empty(),
            "{label}: {bad:?} must stay clean under a schema with no x-finetype-label"
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// AC2 — the reject token is distinct from `pattern`
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn a_wrong_shape_and_a_wrong_check_digit_get_different_tokens() {
    let taxonomy = taxonomy();
    let schema = schema_for(&taxonomy, "finance.securities.lei", true);

    // Right shape (20 chars, [A-Z0-9]{18}[0-9]{2}), wrong check digit.
    let wrong_digit = rejects_for(&schema, &["529900T8BM49AURSDO56"]);
    assert_eq!(wrong_digit.len(), 1);
    assert_eq!(wrong_digit[0].constraint_failed, "checksum");

    // Wrong shape entirely.
    let wrong_shape = rejects_for(&schema, &["not-an-lei"]);
    assert!(!wrong_shape.is_empty());
    assert!(
        wrong_shape
            .iter()
            .all(|r| r.constraint_failed != "checksum"),
        "a value that is not the leaf's shape is a shape reject, not a checksum reject: \
         {wrong_shape:?}"
    );
}

#[test]
fn a_checksum_reject_carries_its_row_and_column_position() {
    let taxonomy = taxonomy();
    let schema = schema_for(&taxonomy, "finance.securities.lei", true);
    let rejects = rejects_for(
        &schema,
        &[
            "529900T8BM49AURSDO55", // genuine
            "529900T8BM49AURSDO56", // check digit altered
        ],
    );
    assert_eq!(rejects.len(), 1);
    assert_eq!(rejects[0].row_index, 1);
    assert_eq!(rejects[0].column_index, 0);
    assert_eq!(rejects[0].column_name, "id");
}

// ═══════════════════════════════════════════════════════════════════════════
// The two columns the card names: an LEI column and an ISBN column
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn a_well_shaped_non_lei_column_no_longer_passes() {
    let taxonomy = taxonomy();
    let schema = schema_for(&taxonomy, "finance.securities.lei", true);
    // Twenty characters, right alphabet, right trailing digits — and not LEIs.
    let values = [
        "AAAAAAAAAAAAAAAAAA11",
        "ZZZZZZZZZZZZZZZZZZ99",
        "529900T8BM49AURSDO54",
    ];
    let headers = vec!["lei".to_string()];
    let rows: Vec<Vec<Option<String>>> = values
        .iter()
        .map(|v| vec![Some((*v).to_string())])
        .collect();
    let schema = json!({
        "type": "object",
        "properties": { "lei": schema["properties"]["id"].clone() }
    });
    let result = validate_table(&headers, &rows, &schema).expect("validates");
    assert_eq!(
        result.invalid_rows,
        values.len(),
        "every one of these is shaped like an LEI and is not one"
    );
    assert!(result
        .rejects
        .iter()
        .all(|r| r.constraint_failed == "checksum"));
}

#[test]
fn a_thirteen_digit_non_isbn_column_no_longer_passes() {
    let taxonomy = taxonomy();
    let schema = schema_for(&taxonomy, "identity.commerce.isbn", true);
    let rejects = rejects_for(&schema, &["9780306406158", "9780306406157"]);
    assert_eq!(
        rejects.len(),
        1,
        "the genuine ISBN-13 passes and the altered one does not: {rejects:?}"
    );
    assert_eq!(rejects[0].constraint_failed, "checksum");
    assert_eq!(rejects[0].value.as_deref(), Some("9780306406158"));
}

// ═══════════════════════════════════════════════════════════════════════════
// Nothing outside the directive changed
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn a_label_without_the_directive_is_untouched() {
    let taxonomy = taxonomy();
    let label = "representation.numeric.integer_number";
    assert!(
        checksum::scheme_for_label(label).is_none(),
        "{label} must not carry a checksum directive for this test to mean anything"
    );
    let schema = schema_for(&taxonomy, label, true);
    assert!(rejects_for(&schema, &["42", "-7", "1000000"]).is_empty());
}

#[test]
fn an_unknown_label_on_a_schema_is_not_an_error() {
    // A consumer's hand-written schema can carry any x-finetype-label string.
    // An unrecognised one means "no checksum", never a hard failure.
    let schema = json!({
        "type": "object",
        "properties": {
            "id": { "type": "string", "x-finetype-label": "no.such.label" }
        }
    });
    assert!(rejects_for(&schema, &["anything at all"]).is_empty());
}

#[test]
fn a_null_cell_in_a_checksum_column_still_passes() {
    let taxonomy = taxonomy();
    let schema = schema_for(&taxonomy, "finance.securities.lei", true);
    let headers = vec!["id".to_string()];
    let rows = vec![vec![None], vec![Some("".to_string())]];
    let result = validate_table(&headers, &rows, &schema).expect("validates");
    assert_eq!(
        result.invalid_rows, 0,
        "nulls pass validation as they always did"
    );
    assert!(result.rejects.is_empty());
}
