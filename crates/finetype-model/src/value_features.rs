//! Per-value feature contract for the value-level YDF labelling pipeline
//! (spec 2026-06-04-value-level-ydf-labelling, ac-00).
//!
//! A single value is mapped to a deterministic row of:
//!   - 37 `extract_features` dims (incl. `decimal_places`), then
//!   - one JSON-Schema pass flag per taxonomy type (0.0 / 1.0 for a single value).
//!
//! The column order is FIXED — `FEATURE_NAMES` followed by the extractor's
//! `type_keys` (which are `taxonomy.labels()`, sorted). The synthetic-prior path
//! (ac-01) and the gittables-scoring path (ac-02) both build their rows through
//! this one function, so the feature contract cannot drift between training and
//! scoring. None of these features is a Sense branch — they are deterministic, so
//! the YDF lens stays independent of the CharCNN it cleans data for.

use crate::features::{extract_features, FEATURE_DIM, FEATURE_NAMES};
use crate::validation_features::ValidationFeatureExtractor;
use finetype_core::Taxonomy;

/// Total per-value vector width: 37 deterministic features + one schema flag per
/// taxonomy type.
pub fn value_feature_dim(extractor: &ValidationFeatureExtractor) -> usize {
    FEATURE_DIM + extractor.dim()
}

/// Build the fixed-order per-value feature row for a single value.
///
/// `[ extract_features(value) (37) | schema_pass per type (extractor.dim()) ]`.
/// For one value each schema flag is 0.0 or 1.0.
pub fn value_feature_row(
    value: &str,
    extractor: &ValidationFeatureExtractor,
    taxonomy: &Taxonomy,
) -> Vec<f32> {
    let mut row = Vec::with_capacity(value_feature_dim(extractor));
    row.extend_from_slice(&extract_features(value));
    row.extend(extractor.extract(&[value], taxonomy));
    row
}

/// The ordered column names matching [`value_feature_row`]. Deterministic
/// features keep their `FEATURE_NAMES`; schema flags are prefixed `schema_pass:`
/// so a downstream reader can tell the two blocks apart.
pub fn value_feature_names(extractor: &ValidationFeatureExtractor) -> Vec<String> {
    let mut names: Vec<String> = FEATURE_NAMES.iter().map(|s| s.to_string()).collect();
    for key in extractor.type_keys() {
        names.push(format!("schema_pass:{key}"));
    }
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load_taxonomy() -> Taxonomy {
        let labels_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("labels");
        let mut taxonomy = Taxonomy::from_directory(&labels_dir).unwrap();
        taxonomy.compile_validators();
        taxonomy
    }

    fn idx_of(extractor: &ValidationFeatureExtractor, key: &str) -> usize {
        FEATURE_DIM
            + extractor
                .index_of(key)
                .unwrap_or_else(|| panic!("type key not found: {key}"))
    }

    const DECIMAL_PLACES: usize = 36; // FEATURE_NAMES[36]

    #[test]
    fn ac00_golden_vector_pins_the_feature_contract() {
        let taxonomy = load_taxonomy();
        let extractor = ValidationFeatureExtractor::new(&taxonomy);
        let names = value_feature_names(&extractor);

        // Contract width is fixed: 37 deterministic + one flag per type.
        assert_eq!(FEATURE_NAMES[DECIMAL_PLACES], "decimal_places");
        assert_eq!(names.len(), value_feature_dim(&extractor));
        assert_eq!(names.len(), FEATURE_DIM + taxonomy.labels().len());

        let lat_key = "geography.coordinate.latitude";
        let cc_key = "geography.location.country_code";
        let lat_i = idx_of(&extractor, lat_key);
        let cc_i = idx_of(&extractor, cc_key);

        // A 4dp latitude: precision feature = 4, passes the latitude schema.
        let lat = value_feature_row("37.7749", &extractor, &taxonomy);
        assert_eq!(lat.len(), value_feature_dim(&extractor));
        assert_eq!(lat[DECIMAL_PLACES], 4.0, "4dp latitude decimal_places");
        assert_eq!(lat[lat_i], 1.0, "37.7749 must pass the latitude schema");

        // A 1dp magnitude: precision feature = 1. (It sits in coordinate range,
        // so it also passes the latitude schema — precision is the separator,
        // which is exactly why decimal_places is in the contract.)
        let mag = value_feature_row("5.8", &extractor, &taxonomy);
        assert_eq!(mag[DECIMAL_PLACES], 1.0, "1dp magnitude decimal_places");

        // A 2-letter code: passes country_code, is not a number so fails latitude.
        let code = value_feature_row("US", &extractor, &taxonomy);
        assert_eq!(
            code[DECIMAL_PLACES], 0.0,
            "non-numeric has 0 decimal places"
        );
        assert_eq!(code[cc_i], 1.0, "US must pass the country_code schema");
        assert_eq!(code[lat_i], 0.0, "US must fail the latitude schema");
    }

    #[test]
    fn ac00_column_order_is_deterministic_features_then_schema() {
        let taxonomy = load_taxonomy();
        let extractor = ValidationFeatureExtractor::new(&taxonomy);
        let names = value_feature_names(&extractor);

        for (i, want) in FEATURE_NAMES.iter().enumerate() {
            assert_eq!(&names[i], want, "deterministic block out of order at {i}");
        }
        assert!(
            names[FEATURE_DIM].starts_with("schema_pass:"),
            "schema block must follow the deterministic block"
        );
    }
}
