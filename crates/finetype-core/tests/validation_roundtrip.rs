//! The emit -> validate round trip on the live taxonomy, with no model.
//!
//! `profile -o json-schema` emits a column schema from a type's published rule and
//! its `validation_notes`; `finetype validate` reads that schema back. This pins
//! the user-facing promise across the two: an upper-case UUID and a withdrawn
//! currency code are valid values that carry a note, and a value outside the rule
//! is a reject.

use finetype_core::grade::NoteKind;
use finetype_core::json_schema::{emit_table_schema, TableSchemaColumn};
use finetype_core::table_validator::validate_table;
use finetype_core::taxonomy::Taxonomy;
use std::path::PathBuf;

const LOWER_UUID: &str = "550e8400-e29b-41d4-a716-446655440000";
const UPPER_UUID: &str = "550E8400-E29B-41D4-A716-446655440000";

fn live_taxonomy() -> Taxonomy {
    let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    root.pop();
    root.pop();
    let mut tax = Taxonomy::from_directory(root.join("labels")).expect("load labels");
    tax.compile_validators();
    tax
}

fn column<'a>(name: &'a str, label: &'a str) -> TableSchemaColumn<'a> {
    TableSchemaColumn {
        name,
        label,
        values: &[],
        null_count: 0,
        unknown_reason: None,
        nominated: false,
    }
}

fn cells(values: &[&str]) -> Vec<Vec<Option<String>>> {
    values.iter().map(|v| vec![Some(v.to_string())]).collect()
}

#[test]
fn emitted_schema_grades_uuid_case_and_withdrawn_currency_as_notes() {
    let tax = live_taxonomy();

    // uuid: lower case is canonical, upper case is noted, a bad value is rejected.
    let schema = emit_table_schema(
        &[column("id", "representation.identifier.uuid")],
        "t",
        "t",
        &tax,
        false,
        0,
    );
    let result = validate_table(
        &["id".to_string()],
        &cells(&[LOWER_UUID, UPPER_UUID, "not-a-uuid"]),
        &schema,
    )
    .expect("validate uuid column");
    assert_eq!(result.rejects.len(), 1, "{:?}", result.rejects);
    assert_eq!(result.rejects[0].row_index, 2);
    assert_eq!(result.valid_row_indices, vec![0, 1]);
    assert_eq!(result.column_notes.len(), 1);
    let notes = &result.column_notes[0];
    assert_eq!(notes.column_name, "id");
    assert_eq!(notes.counts[&NoteKind::NonCanonical], 1);
    assert_eq!(notes.examples, vec![UPPER_UUID.to_string()]);

    // currency_code: USD conforms, the withdrawn HRK is noted, ABC is rejected.
    let schema = emit_table_schema(
        &[column("ccy", "finance.currency.currency_code")],
        "t",
        "t",
        &tax,
        false,
        0,
    );
    let result = validate_table(
        &["ccy".to_string()],
        &cells(&["USD", "HRK", "ABC"]),
        &schema,
    )
    .expect("validate currency column");
    assert_eq!(result.rejects.len(), 1, "{:?}", result.rejects);
    assert_eq!(result.rejects[0].row_index, 2);
    assert_eq!(result.valid_row_indices, vec![0, 1]);
    assert_eq!(result.column_notes.len(), 1);
    let notes = &result.column_notes[0];
    assert_eq!(notes.column_name, "ccy");
    assert_eq!(notes.counts[&NoteKind::Withdrawn], 1);
    assert_eq!(notes.examples, vec!["HRK".to_string()]);
}
