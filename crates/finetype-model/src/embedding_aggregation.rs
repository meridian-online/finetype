//! Column-level embedding aggregation features.
//!
//! Computes a 512-dim feature vector for a column of string values by encoding
//! each value with Model2Vec and aggregating across the column. The four
//! statistics — mean, variance, min, max — are each 128-dim (matching the
//! Model2Vec embedding dimension), concatenated into a single descriptor.
//!
//! This module is intentionally standalone: it depends only on
//! [`Model2VecResources`] and performs no classification.

use crate::model2vec_shared::Model2VecResources;

/// Model2Vec embedding dimension (potion-base-4M).
pub const EMBED_DIM: usize = 128;

/// 128-dim embeddings x 4 statistics (mean, var, min, max) = 512 features.
pub const EMBED_AGG_DIM: usize = 512;

/// Extract 512-dim embedding aggregation features from a column of values.
///
/// Uses Model2Vec to encode each value, then computes per-dimension mean,
/// population variance, min, and max across all valid embeddings.
///
/// Returns `None` if no values produce valid embeddings (e.g. all empty strings).
pub fn extract_embedding_aggregation(
    values: &[&str],
    resources: &Model2VecResources,
) -> Option<[f32; EMBED_AGG_DIM]> {
    if values.is_empty() {
        return None;
    }

    // Encode all values in one batch for efficiency.
    // encode_batch returns [N, embed_dim]; rows for empty/untokenizable strings are zero vectors.
    let batch = resources.encode_batch(values).ok()?;
    let embed_dim = resources.embed_dim().ok()?;

    // Collect valid (non-zero) embeddings
    let mut valid_rows: Vec<Vec<f32>> = Vec::with_capacity(values.len());
    for i in 0..values.len() {
        let row: Vec<f32> = batch.get(i).ok()?.to_vec1().ok()?;
        let norm_sq: f32 = row.iter().map(|v| v * v).sum();
        if norm_sq > 1e-16 {
            valid_rows.push(row);
        }
    }

    if valid_rows.is_empty() {
        return None;
    }

    let n = valid_rows.len() as f32;

    // Compute per-dimension statistics in a single pass for mean, then derive var/min/max
    let mut mean = [0.0f32; EMBED_DIM];
    let mut min = [f32::INFINITY; EMBED_DIM];
    let mut max = [f32::NEG_INFINITY; EMBED_DIM];

    for row in &valid_rows {
        for d in 0..embed_dim.min(EMBED_DIM) {
            mean[d] += row[d];
            if row[d] < min[d] {
                min[d] = row[d];
            }
            if row[d] > max[d] {
                max[d] = row[d];
            }
        }
    }

    for v in mean.iter_mut().take(embed_dim.min(EMBED_DIM)) {
        *v /= n;
    }

    // Population variance: E[(X - mu)^2]
    let mut variance = [0.0f32; EMBED_DIM];
    for row in &valid_rows {
        for d in 0..embed_dim.min(EMBED_DIM) {
            let diff = row[d] - mean[d];
            variance[d] += diff * diff;
        }
    }
    for v in variance.iter_mut().take(embed_dim.min(EMBED_DIM)) {
        *v /= n;
    }

    // Concatenate: mean ++ variance ++ min ++ max → [512]
    let mut result = [0.0f32; EMBED_AGG_DIM];
    result[..EMBED_DIM].copy_from_slice(&mean);
    result[EMBED_DIM..2 * EMBED_DIM].copy_from_slice(&variance);
    result[2 * EMBED_DIM..3 * EMBED_DIM].copy_from_slice(&min);
    result[3 * EMBED_DIM..4 * EMBED_DIM].copy_from_slice(&max);

    Some(result)
}

// ═══════════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: resolve workspace root → models/model2vec.
    fn model_dir() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("models")
            .join("model2vec")
    }

    /// Load real Model2Vec resources, or skip the test if artifacts are absent.
    fn load_resources_or_skip() -> Option<Model2VecResources> {
        let dir = model_dir();
        if !dir.join("model.safetensors").exists() {
            eprintln!("Skipping: models/model2vec not found");
            return None;
        }
        Some(Model2VecResources::load(&dir).unwrap())
    }

    /// Cosine similarity between two equal-length slices.
    fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        assert_eq!(a.len(), b.len());
        let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm_a < 1e-8 || norm_b < 1e-8 {
            return 0.0;
        }
        dot / (norm_a * norm_b)
    }

    // ── Test 1: output dimensions ──────────────────────────────────────────

    #[test]
    fn test_output_dimensions_diverse_values() {
        let resources = match load_resources_or_skip() {
            Some(r) => r,
            None => return,
        };

        let values: Vec<&str> = vec![
            // Names
            "John Smith", "Jane Doe", "Robert Johnson", "Maria Garcia",
            "Ahmed Hassan", "Yuki Tanaka", "Priya Sharma", "Liam O'Brien",
            "Chen Wei", "Fatima Al-Rashid",
            // Numbers
            "42", "3.14159", "1,000,000", "-273.15", "0.001",
            "99.99", "12345", "7.5e6", "100%", "50/50",
            // Dates
            "2024-01-15", "March 3, 2023", "01/01/2000", "15-Dec-1999",
            "2023-12-31T23:59:59", "Jan 2024", "Q1 2023", "FY2024",
            "1999-12-31", "2000/01/01",
            // Emails
            "user@example.com", "admin@test.org", "hello@world.net",
            "first.last@company.co.uk", "info@domain.io",
            // URLs
            "https://example.com", "http://test.org/path",
            "www.google.com", "ftp://files.server.net",
            // Addresses
            "123 Main Street", "456 Oak Avenue, Suite 200",
            "London, UK", "Tokyo, Japan", "New York, NY 10001",
            // Codes
            "USD", "EUR", "GBP", "JPY", "AUD",
            "US", "GB", "JP", "DE", "FR",
            // IDs
            "550e8400-e29b-41d4-a716-446655440000",
            "ABC-123", "ID-00001", "REF/2024/001",
            // Misc
            "true", "false", "null", "N/A", "#REF!",
            // Phone-like
            "+1-555-0123", "(212) 555-1234", "+44 20 7946 0958",
            // IP addresses
            "192.168.1.1", "10.0.0.1", "172.16.0.0",
            // Long text
            "The quick brown fox jumps over the lazy dog",
            "Lorem ipsum dolor sit amet consectetur",
            "This is a moderately long sentence with several words in it",
            // Short
            "a", "b", "c", "AB", "XY",
            // More names for variety
            "Dr. Emily Chen", "Prof. Mark Davis", "Mrs. Sarah Williams",
            "Michael Brown Jr.", "Alexandra Petrova",
            // More numbers
            "1", "2", "3", "4", "5", "6", "7", "8", "9", "10",
            // Padding to reach 100
            "alpha", "beta", "gamma", "delta", "epsilon",
            "zeta", "eta", "theta", "iota", "kappa",
            "lambda", "mu", "nu", "xi", "omicron",
        ];

        assert!(values.len() >= 100, "Need at least 100 values for test, got {}", values.len());

        let result = extract_embedding_aggregation(&values, &resources);
        assert!(result.is_some(), "Should produce a result for diverse values");

        let features = result.unwrap();
        assert_eq!(features.len(), EMBED_AGG_DIM);
        assert_eq!(features.len(), 512);
    }

    // ── Test 2: semantic differentiation ───────────────────────────────────

    #[test]
    fn test_semantic_differentiation() {
        let resources = match load_resources_or_skip() {
            Some(r) => r,
            None => return,
        };

        let names: Vec<&str> = vec![
            "John Smith", "Jane Doe", "Robert Johnson", "Mary Williams",
            "James Brown", "Patricia Davis", "Michael Miller", "Jennifer Wilson",
            "David Taylor", "Linda Anderson", "William Thomas", "Barbara Jackson",
            "Richard White", "Susan Harris", "Joseph Martin", "Margaret Thompson",
            "Charles Garcia", "Dorothy Martinez", "Thomas Robinson", "Lisa Clark",
        ];

        let dates: Vec<&str> = vec![
            "2024-01-15", "2023-03-22", "2022-07-04", "2021-11-30",
            "2020-06-15", "2019-09-01", "2018-12-25", "2017-04-10",
            "2024-02-28", "2023-08-14", "2022-01-01", "2021-05-20",
            "2020-10-31", "2019-03-17", "2018-07-04", "2017-12-31",
            "2024-06-21", "2023-11-11", "2022-04-15", "2021-09-23",
        ];

        let names_features = extract_embedding_aggregation(&names, &resources)
            .expect("names should produce features");
        let dates_features = extract_embedding_aggregation(&dates, &resources)
            .expect("dates should produce features");

        let sim = cosine_similarity(&names_features, &dates_features);
        assert!(
            sim < 0.8,
            "Names and dates columns should be semantically different, cosine similarity = {sim}"
        );
    }

    // ── Test 3: single-value column ────────────────────────────────────────

    #[test]
    fn test_single_value_column() {
        let resources = match load_resources_or_skip() {
            Some(r) => r,
            None => return,
        };

        let values: Vec<&str> = vec!["hello world"];

        let result = extract_embedding_aggregation(&values, &resources);
        assert!(result.is_some(), "Single value should produce features");

        let features = result.unwrap();

        // Variance should be all zeros (only one observation)
        let variance_slice = &features[EMBED_DIM..2 * EMBED_DIM];
        for (i, &v) in variance_slice.iter().enumerate() {
            assert!(
                v.abs() < 1e-10,
                "Single-value variance should be zero at dim {i}, got {v}"
            );
        }

        // Min should equal max should equal mean
        let mean_slice = &features[..EMBED_DIM];
        let min_slice = &features[2 * EMBED_DIM..3 * EMBED_DIM];
        let max_slice = &features[3 * EMBED_DIM..4 * EMBED_DIM];

        for d in 0..EMBED_DIM {
            assert!(
                (mean_slice[d] - min_slice[d]).abs() < 1e-7,
                "Single-value: mean[{d}] != min[{d}]: {} vs {}",
                mean_slice[d],
                min_slice[d]
            );
            assert!(
                (mean_slice[d] - max_slice[d]).abs() < 1e-7,
                "Single-value: mean[{d}] != max[{d}]: {} vs {}",
                mean_slice[d],
                max_slice[d]
            );
        }
    }

    // ── Test 4: empty strings are skipped ──────────────────────────────────

    #[test]
    fn test_skip_empty_strings() {
        let resources = match load_resources_or_skip() {
            Some(r) => r,
            None => return,
        };

        // Column with empties mixed in
        let with_empties: Vec<&str> = vec!["", "John Smith", "", "Jane Doe", ""];
        let without_empties: Vec<&str> = vec!["John Smith", "Jane Doe"];

        let feat_with = extract_embedding_aggregation(&with_empties, &resources)
            .expect("Should produce features despite empties");
        let feat_without = extract_embedding_aggregation(&without_empties, &resources)
            .expect("Should produce features for clean column");

        // The features should be identical since empties are skipped
        for d in 0..EMBED_AGG_DIM {
            assert!(
                (feat_with[d] - feat_without[d]).abs() < 1e-6,
                "Feature dim {d} differs with/without empties: {} vs {}",
                feat_with[d],
                feat_without[d]
            );
        }
    }

    // ── Test 5: all-empty column returns None ──────────────────────────────

    #[test]
    fn test_all_empty_returns_none() {
        let resources = match load_resources_or_skip() {
            Some(r) => r,
            None => return,
        };

        let empties: Vec<&str> = vec!["", "", "", ""];
        let result = extract_embedding_aggregation(&empties, &resources);
        assert!(
            result.is_none(),
            "All-empty column should return None"
        );
    }
}
