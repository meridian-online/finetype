//! ac-04 — an emitted Data Package descriptor validates against the vendored
//! Frictionless v2.0 profile (custom `x-finetype-*` extensions included), and
//! every field's type/format round-trips back to the authoritative map.
//!
//! Mirrors dovetail-core/tests/conformance.rs. Uses the embedded taxonomy
//! (feature `embedded-taxonomy`) so type/format come from the real map.

use finetype_core::frictionless_vocabulary::{constraint_vocabulary, is_profile_field_type};
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

// ── The type→constraint vocabulary gate ────────────────────────────────────
//
// Added 2026-08-28 after `frictionless==5.19.0` refused three of the four
// descriptors published from this engine. The two tests above cannot see it:
// the vendored profile sets `additionalProperties` nowhere, so a `pattern`
// beside `"type": "integer"` still matches the profile's `integer` branch —
// it is the reference implementation's per-type `supported_constraints` that
// refuses it. The gate therefore has to ask the vocabulary directly.

/// Taxonomy labels whose declared Frictionless type is outside the fifteen the
/// vendored profile's field `oneOf` pins, and what each declares.
///
/// `list` is a Data Package v2 **specification** type that the published v2.0
/// profile does not carry and `frictionless==5.19.0` does not implement — it
/// answers a `list` field with `field type "list" is not supported`, so a
/// descriptor containing one is refused whole. Which of `string` or `array`
/// describes a comma-separated list is a taxonomy decision rather than an
/// emitter one, so the gap is pinned here in executable form instead of being
/// filtered away silently.
const TYPES_OUTSIDE_THE_PROFILE: &[(&str, &str)] = &[("container.array.comma_separated", "list")];

/// One column per taxonomy label, with no observed values — the worst case for
/// this gate, because an unobserved column publishes the type's canonical
/// `pattern` verbatim rather than a fitted or omitted one.
fn every_label_as_a_column(taxonomy: &Taxonomy) -> Vec<(String, String)> {
    taxonomy
        .labels()
        .iter()
        .enumerate()
        .map(|(i, label)| (format!("c{i}"), label.clone()))
        .collect()
}

fn descriptor_over(labels: &[(String, String)], taxonomy: &Taxonomy) -> serde_json::Value {
    let empty: Vec<String> = Vec::new();
    let cols: Vec<DatapackageColumn<'_>> = labels
        .iter()
        .map(|(name, label)| DatapackageColumn {
            name,
            label,
            values: &empty,
            confidence: Some(0.9),
            locale: None,
        })
        .collect();
    emit_datapackage(&cols, &meta(), taxonomy, 32)
}

#[test]
fn no_emitted_field_carries_a_constraint_outside_its_declared_types_vocabulary() {
    let taxonomy = Taxonomy::embedded().expect("embedded taxonomy");

    // Every label the taxonomy holds, plus the fixture set the two tests above
    // use, so this gate covers every descriptor this repository can produce.
    let mut columns = every_label_as_a_column(&taxonomy);
    columns.extend(
        fixture_columns()
            .iter()
            .map(|(n, l, _)| ((*n).to_string(), (*l).to_string())),
    );
    assert!(
        columns.len() > 200,
        "expected the whole taxonomy under this gate, got {} columns",
        columns.len()
    );

    let descriptor = descriptor_over(&columns, &taxonomy);
    let fields = descriptor["resources"][0]["schema"]["fields"]
        .as_array()
        .expect("fields array");

    let mut offences: Vec<String> = Vec::new();
    for field in fields {
        let ftype = field["type"].as_str().expect("every field declares a type");
        let label = field["x-finetype-label"].as_str().unwrap_or("<none>");
        let Some(vocabulary) = constraint_vocabulary(ftype) else {
            // A type outside the profile's fifteen; pinned separately below.
            continue;
        };
        let Some(constraints) = field.get("constraints").and_then(|c| c.as_object()) else {
            continue;
        };
        for keyword in constraints.keys() {
            if !vocabulary.contains(&keyword.as_str()) {
                offences.push(format!(
                    "{label}: constraint `{keyword}` is not supported by type `{ftype}`"
                ));
            }
        }
    }
    assert!(
        offences.is_empty(),
        "{} field(s) carry a constraint their declared type has no place for — \
         `frictionless` refuses the whole package for each one:\n  {}",
        offences.len(),
        offences.join("\n  ")
    );
}

#[test]
fn a_filtered_constraint_is_carried_beside_the_field_not_dropped() {
    let taxonomy = Taxonomy::embedded().expect("embedded taxonomy");
    let columns = every_label_as_a_column(&taxonomy);
    let descriptor = descriptor_over(&columns, &taxonomy);
    let fields = descriptor["resources"][0]["schema"]["fields"]
        .as_array()
        .expect("fields array");

    let mut carried = 0usize;
    for field in fields {
        let label = field["x-finetype-label"].as_str().expect("label");
        let Some(def) = taxonomy.get(label) else {
            continue;
        };
        let Some(v) = def.validation.as_ref() else {
            continue;
        };
        let ftype = field["type"].as_str().expect("type");
        if constraint_vocabulary(ftype).is_none() {
            continue;
        }

        // What the emitter computed before routing, from the label alone.
        let mut expected: Vec<&str> = Vec::new();
        if v.pattern.is_some() {
            expected.push("pattern");
        }
        if v.min_length.is_some() {
            expected.push("minLength");
        }
        if v.max_length.is_some() {
            expected.push("maxLength");
        }
        if v.minimum.is_some() {
            expected.push("minimum");
        }
        if v.maximum.is_some() {
            expected.push("maximum");
        }

        for keyword in expected {
            let in_constraints = field
                .pointer(&format!("/constraints/{keyword}"))
                .is_some_and(|v| !v.is_null());
            let in_carrier = field
                .pointer(&format!("/x-finetype-unsupported-constraints/{keyword}"))
                .is_some_and(|v| !v.is_null());
            assert!(
                in_constraints ^ in_carrier,
                "{label} ({ftype}): `{keyword}` is in {} of the two places it may be — \
                 the routing must move a keyword, never drop or duplicate it",
                if in_constraints { "both" } else { "neither" }
            );
            if in_carrier {
                carried += 1;
            }
        }
    }
    // A gate that would pass on a taxonomy where nothing needed carrying is a
    // gate that proves nothing about the carrier.
    assert!(
        carried >= 100,
        "expected the carrier to be exercised across the taxonomy, saw {carried} keyword(s)"
    );
}

#[test]
fn the_declared_types_outside_the_profile_are_exactly_the_ones_pinned() {
    let taxonomy = Taxonomy::embedded().expect("embedded taxonomy");
    let mut found: Vec<(String, String)> = taxonomy
        .definitions()
        .filter_map(|(label, def)| {
            let ftype = def.frictionless.as_ref()?.ftype.clone();
            (!is_profile_field_type(&ftype)).then_some((label.clone(), ftype))
        })
        .collect();
    found.sort();

    let expected: Vec<(String, String)> = TYPES_OUTSIDE_THE_PROFILE
        .iter()
        .map(|(l, t)| ((*l).to_string(), (*t).to_string()))
        .collect();
    assert_eq!(
        found, expected,
        "the set of labels declaring a Frictionless type the vendored profile \
         does not pin has changed. A new one makes every descriptor containing \
         that column unreadable by `frictionless`; a removed one means the pin \
         and the comment above it are stale."
    );
}

#[test]
fn a_descriptor_over_every_profile_typed_label_validates_against_the_vendored_profile() {
    let taxonomy = Taxonomy::embedded().expect("embedded taxonomy");
    let outside: Vec<&str> = TYPES_OUTSIDE_THE_PROFILE.iter().map(|(l, _)| *l).collect();
    let columns: Vec<(String, String)> = every_label_as_a_column(&taxonomy)
        .into_iter()
        .filter(|(_, label)| !outside.contains(&label.as_str()))
        .collect();

    let descriptor = descriptor_over(&columns, &taxonomy);
    let schema = vendored_profile();
    let validator = jsonschema::validator_for(&schema).expect("compile vendored profile");
    let errors: Vec<String> = validator
        .iter_errors(&descriptor)
        .map(|e| format!("{} at {}", e, e.instance_path()))
        .take(20)
        .collect();
    assert!(
        errors.is_empty(),
        "a descriptor over the whole taxonomy failed v2.0 profile conformance:\n  {}",
        errors.join("\n  ")
    );
}
