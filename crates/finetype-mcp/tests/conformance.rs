//! ac-04 — an emitted Data Package descriptor validates against the vendored
//! Frictionless v2.0 profile (custom `x-finetype-*` extensions included), and
//! every field's type/format round-trips back to the authoritative map.
//!
//! Mirrors dovetail-core/tests/conformance.rs. Uses the embedded taxonomy
//! (feature `embedded-taxonomy`) so type/format come from the real map.

use finetype_core::{frictionless_for, Taxonomy};
use finetype_mcp::datapackage::{emit_datapackage, DatapackageColumn, ResourceMeta};
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn vendored_profile() -> serde_json::Value {
    let path = repo_root().join("vendor/frictionless/datapackage-profile.json");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read vendored profile {}: {e}", path.display()));
    serde_json::from_str(&text).expect("parse vendored profile")
}

fn meta() -> ResourceMeta {
    ResourceMeta {
        name: "fixture".into(),
        path: "fixture.csv".into(),
        format: "csv".into(),
        mediatype: "text/csv".into(),
        encoding: Some("utf-8".into()),
        bytes: 128,
        hash: "sha256:0000".into(),
        created: "2026-06-24T00:00:00Z".into(),
    }
}

/// One fixture column per type family the spec calls out: string-format,
/// datetime, numeric, boolean, and categorical-enum (a string-typed
/// enum-keyword-eligible label with small distinct values emits a closed
/// `enum`; the boolean column exercises the boolean type without an enum).
///
/// Plus both outcomes of fitting a `pattern` to the values observed in a column,
/// because each one changes what the descriptor carries: a widened pattern and a
/// dropped one, each accompanied by an `x-finetype-pattern-fit` extension the
/// profile has to accept.
fn fixture_columns() -> Vec<(&'static str, &'static str, Vec<String>)> {
    vec![
        ("email", "identity.person.email", vec![]),
        ("d", "datetime.date.dmy_slash", vec![]),
        ("lat", "geography.coordinate.latitude", vec![]),
        (
            "jurisdiction",
            "geography.location.country_code",
            vec!["US".into(), "FR".into(), "US-DE".into(), "CA-ON".into()],
        ),
        (
            "registry",
            "geography.location.country_code",
            vec![
                "United States".into(),
                "France".into(),
                "Japan".into(),
                "The Netherlands".into(),
                "Cote d'Ivoire".into(),
            ],
        ),
        (
            "flag",
            "representation.boolean.binary",
            vec!["0".into(), "1".into(), "1".into(), "0".into()],
        ),
        (
            "category",
            "representation.discrete.categorical",
            vec!["A".into(), "B".into(), "A".into()],
        ),
        ("mystery", "unknown", vec![]),
    ]
}

#[test]
fn emitted_descriptor_validates_against_vendored_profile() {
    let taxonomy = Taxonomy::embedded().expect("embedded taxonomy");
    let fixtures = fixture_columns();
    let cols: Vec<DatapackageColumn<'_>> = fixtures
        .iter()
        .map(|(name, label, values)| DatapackageColumn {
            name,
            label,
            values,
            confidence: Some(0.9),
            locale: None,
        })
        .collect();

    let descriptor = emit_datapackage(&cols, &meta(), &taxonomy, 32);

    // An x-finetype-* extension is present (so we are actually testing that
    // extensions do not break conformance).
    let fields = descriptor["resources"][0]["schema"]["fields"]
        .as_array()
        .expect("fields array");
    assert!(
        fields[0].get("x-finetype-label").is_some(),
        "expected x-finetype-label on emitted fields"
    );
    // …including the one this fixture set exists to put in front of the
    // profile: both outcomes of fitting a pattern to the observed values.
    let outcomes: Vec<&str> = fields
        .iter()
        .filter_map(|f| f.pointer("/x-finetype-pattern-fit/outcome"))
        .filter_map(|o| o.as_str())
        .collect();
    assert_eq!(
        outcomes,
        vec!["widened", "omitted"],
        "expected the descriptor under test to carry both fit outcomes"
    );

    let schema = vendored_profile();
    let validator = jsonschema::validator_for(&schema).expect("compile vendored profile");
    let errors: Vec<String> = validator
        .iter_errors(&descriptor)
        .map(|e| format!("{} at {}", e, e.instance_path()))
        .collect();
    assert!(
        errors.is_empty(),
        "descriptor failed v2.0 profile conformance:\n  {}",
        errors.join("\n  ")
    );
}

#[test]
fn every_field_type_format_round_trips_to_the_map() {
    let taxonomy = Taxonomy::embedded().expect("embedded taxonomy");
    let fixtures = fixture_columns();
    let cols: Vec<DatapackageColumn<'_>> = fixtures
        .iter()
        .map(|(name, label, values)| DatapackageColumn {
            name,
            label,
            values,
            confidence: None,
            locale: None,
        })
        .collect();

    let descriptor = emit_datapackage(&cols, &meta(), &taxonomy, 32);
    let fields = descriptor["resources"][0]["schema"]["fields"]
        .as_array()
        .unwrap();

    for ((_, label, _), field) in fixtures.iter().zip(fields) {
        let emitted_type = field["type"].as_str().unwrap();
        let emitted_format = field.get("format").and_then(|f| f.as_str());

        match frictionless_for(label) {
            Some(fx) => {
                assert_eq!(emitted_type, fx.ftype, "type mismatch for {label}");
                assert_eq!(
                    emitted_format,
                    fx.format.as_deref(),
                    "format mismatch for {label}"
                );
            }
            None => {
                // Unknown label → string fallback, no format.
                assert_eq!(emitted_type, "string", "unknown {label} should be string");
                assert!(
                    emitted_format.is_none(),
                    "unknown {label} should have no format"
                );
            }
        }
    }
}
