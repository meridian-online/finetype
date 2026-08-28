//! The constraint vocabulary in `finetype_core::frictionless_vocabulary` is the
//! vendored profile's, not a second copy of the standard.
//!
//! `CONSTRAINT_VOCABULARY` is a hand-written table because `finetype-core` is
//! published to crates.io without the sibling `vendor/` directory, so it cannot
//! `include_str!` the profile the way `embedded-taxonomy` includes `labels/`.
//! This test is what keeps the table honest: it reads
//! `vendor/frictionless/datapackage-profile.json`, pulls the `constraints`
//! keys out of each of the field object's fifteen `oneOf` branches, subtracts
//! the measured `REFERENCE_IMPLEMENTATION_NARROWING`, and requires the result
//! to equal the table keyword for keyword.
//!
//! A profile re-vendor, a keyword added to a branch, a type renamed, or an edit
//! to either constant reddens here.

use finetype_core::frictionless_vocabulary::{
    constraint_vocabulary, PROFILE_FIELD_TYPES, REFERENCE_IMPLEMENTATION_NARROWING,
};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("canonicalize repo root")
}

/// The field object's `oneOf` branches, as `type` → `constraints` keys.
///
/// Read out of the profile rather than described: `properties.resources.items
/// .properties.schema.properties.fields.items.oneOf`, each branch's
/// `properties.type.enum[0]` and `properties.constraints.properties`.
fn profile_branches() -> Vec<(String, BTreeSet<String>)> {
    let path = repo_root().join("vendor/frictionless/datapackage-profile.json");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read vendored profile {}: {e}", path.display()));
    let profile: serde_json::Value = serde_json::from_str(&text).expect("parse vendored profile");

    let branches = profile
        .pointer("/properties/resources/items/properties/schema/properties/fields/items/oneOf")
        .and_then(|v| v.as_array())
        .expect("field object is a oneOf in the vendored profile");

    branches
        .iter()
        .map(|branch| {
            let ftype = branch
                .pointer("/properties/type/enum/0")
                .and_then(|v| v.as_str())
                .expect("every field branch pins `type` to one value")
                .to_string();
            let keys = branch
                .pointer("/properties/constraints/properties")
                .and_then(|v| v.as_object())
                .map(|o| o.keys().cloned().collect())
                .unwrap_or_default();
            (ftype, keys)
        })
        .collect()
}

fn narrowing() -> BTreeMap<&'static str, BTreeSet<&'static str>> {
    REFERENCE_IMPLEMENTATION_NARROWING
        .iter()
        .map(|(t, kws)| (*t, kws.iter().copied().collect()))
        .collect()
}

#[test]
fn the_field_object_enum_pins_type_across_fifteen_branches() {
    // The claim `vendor/frictionless/README.md` used to make in reverse: the
    // profile does constrain `type`, so a type outside these fifteen fails
    // every branch and the whole descriptor with it.
    let branches = profile_branches();
    let types: Vec<&str> = branches.iter().map(|(t, _)| t.as_str()).collect();
    assert_eq!(
        types, PROFILE_FIELD_TYPES,
        "PROFILE_FIELD_TYPES must be the profile's branch types, in branch order"
    );
}

#[test]
fn the_vocabulary_table_is_the_profile_minus_the_measured_narrowing() {
    let narrowed = narrowing();
    for (ftype, profile_keys) in profile_branches() {
        let expected: BTreeSet<String> = profile_keys
            .iter()
            .filter(|k| {
                !narrowed
                    .get(ftype.as_str())
                    .is_some_and(|n| n.contains(k.as_str()))
            })
            .cloned()
            .collect();
        let actual: BTreeSet<String> = constraint_vocabulary(&ftype)
            .unwrap_or_else(|| panic!("no constraint vocabulary for profile type `{ftype}`"))
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(
            actual, expected,
            "constraint vocabulary for `{ftype}` has drifted from the vendored profile"
        );
    }
}

#[test]
fn every_narrowed_keyword_is_one_the_profile_actually_allows() {
    // A narrowing entry for a keyword the branch never had would be dead weight
    // that quietly hides a later profile change: the subtraction above would
    // remove a keyword the profile had just added.
    let branches: BTreeMap<String, BTreeSet<String>> = profile_branches().into_iter().collect();
    for (ftype, keywords) in REFERENCE_IMPLEMENTATION_NARROWING {
        let profile_keys = branches
            .get(*ftype)
            .unwrap_or_else(|| panic!("narrowing names `{ftype}`, which the profile does not"));
        for kw in *keywords {
            assert!(
                profile_keys.contains(*kw),
                "narrowing removes `{kw}` from `{ftype}`, but the profile branch never allowed it"
            );
        }
    }
}
