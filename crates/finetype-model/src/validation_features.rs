//! Validation feature extraction for the multi-branch model (v12+).
//!
//! Computes a pass-rate vector by validating column values against every type's
//! JSON Schema. The resulting N-dim vector (one float per taxonomy type) becomes
//! the input to the validation branch of the multi-branch model.
//!
//! Uses [`CompiledValidator::is_valid()`] which handles both string patterns
//! (regex, length, enum) and numeric range checks (min/max) — no additional
//! range-checking code is needed.

use finetype_core::Taxonomy;
use std::collections::BTreeMap;

/// Pre-compiled validators for all taxonomy types, with a stable type-to-index mapping.
///
/// Validators are compiled once from the taxonomy and held in memory for reuse
/// across all columns. The type-to-index mapping ensures consistent ordering
/// between training and inference.
pub struct ValidationFeatureExtractor {
    /// Sorted type keys (defines the index → type mapping).
    type_keys: Vec<String>,
    /// Reverse mapping: type key → index in the feature vector.
    type_index: BTreeMap<String, usize>,
}

impl ValidationFeatureExtractor {
    /// Build a new extractor from a taxonomy.
    ///
    /// The taxonomy must have validators compiled (`taxonomy.compile_validators()`).
    /// Types without validation schemas are included in the index (with pass_rate = 0.0
    /// for all values, since there's no validator to check against).
    pub fn new(taxonomy: &Taxonomy) -> Self {
        let type_keys: Vec<String> = taxonomy.labels().to_vec();
        let type_index: BTreeMap<String, usize> = type_keys
            .iter()
            .enumerate()
            .map(|(i, k)| (k.clone(), i))
            .collect();
        Self {
            type_keys,
            type_index,
        }
    }

    /// Build from a saved type-to-index mapping (loaded from model config).
    ///
    /// Used at inference time when the model was trained on a potentially
    /// different taxonomy snapshot. Types in the current taxonomy that weren't
    /// in the training-time mapping get zero pass rates.
    pub fn from_type_keys(type_keys: Vec<String>) -> Self {
        let type_index: BTreeMap<String, usize> = type_keys
            .iter()
            .enumerate()
            .map(|(i, k)| (k.clone(), i))
            .collect();
        Self {
            type_keys,
            type_index,
        }
    }

    /// Number of dimensions in the validation feature vector.
    pub fn dim(&self) -> usize {
        self.type_keys.len()
    }

    /// The ordered type keys (for serialisation into model config).
    pub fn type_keys(&self) -> &[String] {
        &self.type_keys
    }

    /// Look up the index for a type key.
    pub fn index_of(&self, type_key: &str) -> Option<usize> {
        self.type_index.get(type_key).copied()
    }

    /// Extract validation features for a column of string values.
    ///
    /// For each type in the taxonomy (in index order), computes the fraction
    /// of `values` that pass that type's [`CompiledValidator::is_valid()`] check.
    /// Types without a compiled validator get 0.0.
    ///
    /// # Returns
    /// A `Vec<f32>` of length `self.dim()`, where each element is in [0.0, 1.0].
    pub fn extract(&self, values: &[&str], taxonomy: &Taxonomy) -> Vec<f32> {
        let n = values.len();
        if n == 0 {
            return vec![0.0; self.dim()];
        }

        let n_f32 = n as f32;
        self.type_keys
            .iter()
            .map(|type_key| {
                match taxonomy.get_validator(type_key) {
                    Some(validator) => {
                        let pass_count = values
                            .iter()
                            .filter(|v| !v.trim().is_empty() && validator.is_valid(v))
                            .count();
                        pass_count as f32 / n_f32
                    }
                    None => 0.0, // No validator for this type
                }
            })
            .collect()
    }

    /// Extract validation features, using a training-time mapping to align
    /// indices with a potentially different current taxonomy.
    ///
    /// Types that exist in the training mapping but not in the current taxonomy
    /// get 0.0. Types in the current taxonomy but not in the training mapping
    /// are ignored (they have no index in the feature vector).
    pub fn extract_aligned(
        &self,
        values: &[&str],
        taxonomy: &Taxonomy,
    ) -> Vec<f32> {
        // When the extractor was built from the taxonomy itself, alignment is identity.
        // When built from a saved mapping, only types in both the mapping and taxonomy
        // get non-zero pass rates.
        self.extract(values, taxonomy)
    }
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

    #[test]
    fn ac02_country_codes_discriminate_country_vs_country_code() {
        let taxonomy = load_taxonomy();
        let extractor = ValidationFeatureExtractor::new(&taxonomy);

        // Country code values: "US", "FR", "JP"
        let values: Vec<&str> = vec!["US", "FR", "JP"];
        let features = extractor.extract(&values, &taxonomy);

        assert_eq!(features.len(), extractor.dim());
        assert_eq!(features.len(), taxonomy.labels().len());

        // country_code should have high pass rate (all 3 are valid ISO codes)
        let cc_idx = extractor
            .index_of("geography.location.country_code")
            .unwrap();
        assert!(
            features[cc_idx] > 0.99,
            "country_code pass rate should be ~1.0, got {}",
            features[cc_idx]
        );

        // country has no pattern constraint — should also pass (loose validation)
        let country_idx = extractor
            .index_of("geography.location.country")
            .unwrap();
        // country validation has minLength:2, maxLength:100 — "US" passes
        // But the key discriminator is: country_code has ENUM + pattern,
        // so random 2-letter strings would fail country_code but pass country.

        // email should have ~0.0 pass rate
        let email_idx = extractor.index_of("identity.person.email").unwrap();
        assert!(
            features[email_idx] < 0.01,
            "email pass rate should be ~0.0, got {}",
            features[email_idx]
        );

        // url should have ~0.0 pass rate
        let url_idx = extractor.index_of("technology.internet.url").unwrap();
        assert!(
            features[url_idx] < 0.01,
            "url pass rate should be ~0.0, got {}",
            features[url_idx]
        );
    }

    #[test]
    fn ac02_latitude_range_discriminates_from_decimal_number() {
        let taxonomy = load_taxonomy();
        let extractor = ValidationFeatureExtractor::new(&taxonomy);

        // Mix of in-range and out-of-range values for latitude
        let values: Vec<&str> = vec!["40.7", "-33.8", "91.5"];
        let features = extractor.extract(&values, &taxonomy);

        let lat_idx = extractor
            .index_of("geography.coordinate.latitude")
            .unwrap();
        let dec_idx = extractor
            .index_of("representation.numeric.decimal_number")
            .unwrap();

        // latitude: 2/3 pass (91.5 out of range)
        let expected_lat = 2.0 / 3.0;
        assert!(
            (features[lat_idx] - expected_lat).abs() < 0.01,
            "latitude pass rate should be ~{:.3}, got {}",
            expected_lat,
            features[lat_idx]
        );

        // decimal_number: 3/3 pass (no range constraint)
        assert!(
            features[dec_idx] > 0.99,
            "decimal_number pass rate should be ~1.0, got {}",
            features[dec_idx]
        );
    }

    #[test]
    fn ac02_empty_values_returns_zeros() {
        let taxonomy = load_taxonomy();
        let extractor = ValidationFeatureExtractor::new(&taxonomy);

        let features = extractor.extract(&[], &taxonomy);
        assert_eq!(features.len(), extractor.dim());
        assert!(features.iter().all(|&f| f == 0.0));
    }

    #[test]
    fn ac02_email_display_discriminates_from_email() {
        let taxonomy = load_taxonomy();
        let extractor = ValidationFeatureExtractor::new(&taxonomy);

        // email_display values with angle brackets
        let values: Vec<&str> = vec![
            "John Doe <john@example.com>",
            "Jane Smith <jane@corp.com>",
        ];
        let features = extractor.extract(&values, &taxonomy);

        let email_idx = extractor.index_of("identity.person.email").unwrap();
        let display_idx = extractor
            .index_of("identity.person.email_display")
            .unwrap();

        // email_display should pass (angle bracket pattern matches)
        assert!(
            features[display_idx] > 0.5,
            "email_display pass rate should be high, got {}",
            features[display_idx]
        );

        // email should fail (bare email pattern rejects angle brackets)
        assert!(
            features[email_idx] < 0.5,
            "email pass rate should be low for display-format values, got {}",
            features[email_idx]
        );
    }

    #[test]
    fn ac02_type_index_mapping_consistency() {
        let taxonomy = load_taxonomy();
        let extractor = ValidationFeatureExtractor::new(&taxonomy);

        // Type keys should be sorted
        let keys = extractor.type_keys();
        for i in 1..keys.len() {
            assert!(
                keys[i] > keys[i - 1],
                "type_keys should be sorted: {} > {} failed",
                keys[i],
                keys[i - 1]
            );
        }

        // Round-trip: from_type_keys produces same mapping
        let restored = ValidationFeatureExtractor::from_type_keys(keys.to_vec());
        assert_eq!(restored.dim(), extractor.dim());
        for key in keys {
            assert_eq!(
                restored.index_of(key),
                extractor.index_of(key),
                "index mismatch for {}",
                key
            );
        }
    }

    #[test]
    fn ac02_benchmark_100_values_239_types() {
        let taxonomy = load_taxonomy();
        let extractor = ValidationFeatureExtractor::new(&taxonomy);

        // Generate 100 realistic values (mix of types)
        let values: Vec<String> = (0..100)
            .map(|i| match i % 5 {
                0 => format!("192.168.1.{}", i),
                1 => format!("user{}@example.com", i),
                2 => format!("{}.{}", 40 + (i % 50), i * 7 % 1000),
                3 => format!("2024-01-{:02}", (i % 28) + 1),
                _ => format!("value_{}", i),
            })
            .collect();
        let value_refs: Vec<&str> = values.iter().map(|s| s.as_str()).collect();

        let start = std::time::Instant::now();
        let features = extractor.extract(&value_refs, &taxonomy);
        let elapsed = start.elapsed();

        assert_eq!(features.len(), extractor.dim());
        // Budget: <50ms in release mode. Debug builds are 5-10x slower.
        let budget_ms = if cfg!(debug_assertions) { 500 } else { 50 };
        assert!(
            elapsed.as_millis() < budget_ms,
            "100 values x {} types took {}ms (budget: <{}ms)",
            extractor.dim(),
            elapsed.as_millis(),
            budget_ms,
        );
    }
}
