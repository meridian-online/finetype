//! Which constraint keywords a Frictionless v2 field may carry, given the type
//! it declares.
//!
//! Data Package v2's Table Schema field object is not one schema with a free
//! `type` string: it is a `oneOf` across fifteen branches, one per type, and
//! each branch gives `constraints` its own `properties` set. A `pattern` beside
//! `"type": "integer"` is therefore not a redundant keyword a reader may ignore
//! — `frictionless==5.19.0` refuses the whole package with
//! `constraint "pattern" is not supported by type "integer"`.
//!
//! Both Data Package emitters in the family ask this question, so it is
//! answered here rather than in either of them:
//! `finetype-mcp::datapackage` and dovetail's `dovetail-core::datapackage`.
//!
//! [`CONSTRAINT_VOCABULARY`] is pinned against
//! `vendor/frictionless/datapackage-profile.json` by
//! `crates/finetype-core/tests/frictionless_vocabulary.rs`, which reads the
//! branches out of the profile and reddens if this table and the profile
//! disagree by one keyword.

/// The fifteen field types the vendored v2.0 profile's field `oneOf` pins, in
/// the profile's own branch order.
///
/// This is **not** [`crate::FRICTIONLESS_TYPES`], which is the taxonomy's
/// declaration vocabulary and carries a sixteenth entry, `list`. `list` is a
/// Data Package v2 *specification* type that neither the vendored profile nor
/// `frictionless==5.19.0` implements — the reference implementation answers a
/// `list` field with `field type "list" is not supported`. See the README in
/// `vendor/frictionless/`.
pub const PROFILE_FIELD_TYPES: &[&str] = &[
    "string",
    "number",
    "integer",
    "date",
    "time",
    "datetime",
    "year",
    "yearmonth",
    "boolean",
    "object",
    "geopoint",
    "geojson",
    "array",
    "duration",
    "any",
];

/// Declared type → the constraint keywords a field of that type may carry.
///
/// Each entry is the vendored profile branch's `constraints.properties` keys
/// **minus** [`REFERENCE_IMPLEMENTATION_NARROWING`], because a descriptor that
/// clears the profile and is still refused by the reference implementation is
/// the defect this table exists to prevent, not a pass.
///
/// Keywords are sorted within a type; types are in profile branch order.
//
// Kept as an aligned table rather than reflowed: this is data read against the
// profile keyword by keyword, and one row per type is how it is checked by eye.
#[rustfmt::skip]
const CONSTRAINT_VOCABULARY: &[(&str, &[&str])] = &[
    ("string",    &["enum", "maxLength", "minLength", "pattern", "required", "unique"]),
    ("number",    &["enum", "maximum", "minimum", "required", "unique"]),
    ("integer",   &["enum", "maximum", "minimum", "required", "unique"]),
    ("date",      &["enum", "maximum", "minimum", "required", "unique"]),
    ("time",      &["enum", "maximum", "minimum", "required", "unique"]),
    ("datetime",  &["enum", "maximum", "minimum", "required", "unique"]),
    ("year",      &["enum", "maximum", "minimum", "required", "unique"]),
    ("yearmonth", &["enum", "maximum", "minimum", "required", "unique"]),
    ("boolean",   &["enum", "required"]),
    ("object",    &["enum", "maxLength", "minLength", "required", "unique"]),
    ("geopoint",  &["enum", "required", "unique"]),
    ("geojson",   &["enum", "required", "unique"]),
    ("array",     &["enum", "maxLength", "minLength", "required", "unique"]),
    ("duration",  &["enum", "required", "unique"]),
    ("any",       &["enum", "required", "unique"]),
];

/// Where `frictionless==5.19.0` refuses a keyword the vendored profile's branch
/// allows. Subtracted from the profile to give [`CONSTRAINT_VOCABULARY`].
///
/// Measured, not transcribed: `scripts/measure_frictionless_constraint_matrix.py`
/// puts one field carrying one keyword through `frictionless.Package` for every
/// (type, keyword) pair the profile mentions — 15 types × 11 keywords — and
/// prints this table. Re-run it when the pin moves.
///
/// The reference implementation lags the profile here rather than disagreeing
/// with the standard: its per-type `supported_constraints` lists omit
/// `exclusiveMinimum`/`exclusiveMaximum` everywhere, `jsonSchema` entirely,
/// `minLength`/`maxLength` on `geojson`, and `minimum`/`maximum` on `duration`.
#[rustfmt::skip]
pub const REFERENCE_IMPLEMENTATION_NARROWING: &[(&str, &[&str])] = &[
    ("number",    &["exclusiveMaximum", "exclusiveMinimum"]),
    ("integer",   &["exclusiveMaximum", "exclusiveMinimum"]),
    ("date",      &["exclusiveMaximum", "exclusiveMinimum"]),
    ("time",      &["exclusiveMaximum", "exclusiveMinimum"]),
    ("datetime",  &["exclusiveMaximum", "exclusiveMinimum"]),
    ("year",      &["exclusiveMaximum", "exclusiveMinimum"]),
    ("yearmonth", &["exclusiveMaximum", "exclusiveMinimum"]),
    ("object",    &["jsonSchema"]),
    ("geojson",   &["maxLength", "minLength"]),
    ("array",     &["jsonSchema"]),
    ("duration",  &["exclusiveMaximum", "exclusiveMinimum", "maximum", "minimum"]),
];

/// The constraint keywords legal beside `"type": {ftype}`, or `None` when
/// `ftype` is not one of [`PROFILE_FIELD_TYPES`].
///
/// `None` is a different answer from `Some(&[])`: the first means *the profile
/// says nothing about this type*, which is where a caller has to decide for
/// itself, and the second would mean *this type accepts no constraints*.
pub fn constraint_vocabulary(ftype: &str) -> Option<&'static [&'static str]> {
    CONSTRAINT_VOCABULARY
        .iter()
        .find(|(t, _)| *t == ftype)
        .map(|(_, kws)| *kws)
}

/// Whether `keyword` may sit in the `constraints` of a field declaring `ftype`.
///
/// A type outside [`PROFILE_FIELD_TYPES`] answers `false` for every keyword —
/// the profile gives no vocabulary to filter against. Callers that would rather
/// leave such a field alone should ask [`constraint_vocabulary`] and match on
/// `None`.
pub fn constraint_allowed(ftype: &str, keyword: &str) -> bool {
    constraint_vocabulary(ftype).is_some_and(|kws| kws.contains(&keyword))
}

/// Whether `ftype` is one of the fifteen types the vendored profile's field
/// `oneOf` pins.
pub fn is_profile_field_type(ftype: &str) -> bool {
    PROFILE_FIELD_TYPES.contains(&ftype)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_three_keywords_that_rejected_a_published_descriptor_are_refused() {
        // The exact (type, keyword) pairs `frictionless==5.19.0` named when it
        // refused three of the four descriptors published on 2026-08-28.
        assert!(!constraint_allowed("integer", "pattern"));
        assert!(!constraint_allowed("number", "pattern"));
        assert!(!constraint_allowed("date", "maxLength"));
        assert!(!constraint_allowed("date", "minLength"));
        assert!(!constraint_allowed("date", "pattern"));
        // …and the one type that does carry a pattern still does.
        assert!(constraint_allowed("string", "pattern"));
    }

    #[test]
    fn a_type_outside_the_profile_has_no_vocabulary() {
        assert_eq!(constraint_vocabulary("list"), None);
        assert!(!is_profile_field_type("list"));
        assert!(!constraint_allowed("list", "pattern"));
    }

    #[test]
    fn every_profile_type_has_a_vocabulary_and_vice_versa() {
        for t in PROFILE_FIELD_TYPES {
            assert!(
                constraint_vocabulary(t).is_some(),
                "{t} is a profile field type with no constraint vocabulary"
            );
        }
        for (t, _) in CONSTRAINT_VOCABULARY {
            assert!(
                is_profile_field_type(t),
                "{t} has a constraint vocabulary but is not a profile field type"
            );
        }
        assert_eq!(PROFILE_FIELD_TYPES.len(), CONSTRAINT_VOCABULARY.len());
    }
}
