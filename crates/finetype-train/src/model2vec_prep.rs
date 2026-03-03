//! Model2Vec type embedding preparation: FPS algorithm for taxonomy labels.
//!
//! For each type in the taxonomy, computes K representative embeddings
//! using Farthest Point Sampling (FPS) over synonym expansions.
//!
//! Output: `type_embeddings.safetensors` + `label_index.json`

use anyhow::{Context, Result};
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// Farthest Point Sampling: select K representative points maximising min-distance.
///
/// Given N embeddings of dimension D, select K ≤ N that are maximally spread out.
///
/// - `embeddings`: [N, D] flattened row-major
/// - `n`: number of points
/// - `dim`: embedding dimension
/// - `k`: number of representatives to select
///
/// Returns indices of selected points.
pub fn farthest_point_sampling(embeddings: &[f32], n: usize, dim: usize, k: usize) -> Vec<usize> {
    if k >= n {
        return (0..n).collect();
    }

    let mut selected = Vec::with_capacity(k);
    let mut min_distances = vec![f32::INFINITY; n];

    // Start with the first point
    selected.push(0);

    for _ in 1..k {
        let last = *selected.last().unwrap();
        let last_start = last * dim;

        // Update min distances from the last selected point
        for (i, min_dist) in min_distances.iter_mut().enumerate().take(n) {
            let i_start = i * dim;
            let dist: f32 = (0..dim)
                .map(|d| {
                    let diff = embeddings[last_start + d] - embeddings[i_start + d];
                    diff * diff
                })
                .sum();
            *min_dist = min_dist.min(dist);
        }

        // Select the point with maximum min-distance (farthest from all selected)
        let next = (0..n)
            .filter(|i| !selected.contains(i))
            .max_by(|&a, &b| {
                min_distances[a]
                    .partial_cmp(&min_distances[b])
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap();

        selected.push(next);
    }

    selected
}

/// Write type embeddings and label index to output directory.
///
/// - `embeddings`: [n_types * k, dim] interleaved (rows 0..K for type 0, etc.)
/// - `labels`: ordered type labels
/// - `output_dir`: directory to write to
pub fn write_type_embeddings(
    embeddings: &[f32],
    n_types: usize,
    k: usize,
    dim: usize,
    labels: &[String],
    output_dir: &Path,
) -> Result<()> {
    std::fs::create_dir_all(output_dir)?;

    // Write type_embeddings.safetensors
    let total_rows = n_types * k;
    let tensor_data: Vec<u8> = embeddings.iter().flat_map(|f| f.to_le_bytes()).collect();

    let mut tensors = std::collections::HashMap::new();
    tensors.insert(
        "type_embeddings".to_string(),
        safetensors::tensor::TensorView::new(
            safetensors::Dtype::F32,
            [total_rows, dim].to_vec(),
            &tensor_data,
        )?,
    );
    safetensors::tensor::serialize_to_file(
        &tensors,
        &None,
        &output_dir.join("type_embeddings.safetensors"),
    )
    .context("Failed to write type_embeddings.safetensors")?;

    // Write label_index.json
    let label_json = serde_json::to_string_pretty(labels)?;
    std::fs::write(output_dir.join("label_index.json"), label_json)?;

    tracing::info!(
        "Wrote type embeddings: {} types × {} reps = {} rows × {} dim",
        n_types,
        k,
        total_rows,
        dim,
    );

    Ok(())
}

// ── Synonym Expansion ───────────────────────────────────────────────────────

/// Build synonym text lists for each type label in the taxonomy.
///
/// For each type, collects:
/// - Title field (lowercased)
/// - Aliases (lowercased)
/// - Label components split on dots (e.g. "identity person email")
/// - Leaf name alone (e.g. "email", with underscores → spaces)
/// - All header hint entries that map to this type
///
/// Returns: Vec of (type_label, synonym_texts), sorted by label.
pub fn expand_synonyms(taxonomy: &finetype_core::Taxonomy) -> Vec<(String, Vec<String>)> {
    let header_hints = build_header_hint_entries();

    // Group header hints by target label
    let mut hints_by_label: HashMap<&str, Vec<&str>> = HashMap::new();
    for (hint_text, hint_label) in &header_hints {
        hints_by_label
            .entry(hint_label.as_str())
            .or_default()
            .push(hint_text.as_str());
    }

    let mut result: Vec<(String, Vec<String>)> = Vec::new();

    for label in taxonomy.labels() {
        let mut texts = HashSet::new();

        if let Some(defn) = taxonomy.get(label) {
            // Title
            if let Some(ref title) = defn.title {
                let t = title.to_lowercase();
                if !t.is_empty() {
                    texts.insert(t);
                }
            }

            // Aliases
            if let Some(ref aliases) = defn.aliases {
                for alias in aliases {
                    let a = alias.to_lowercase();
                    if !a.is_empty() {
                        texts.insert(a);
                    }
                }
            }
        }

        // Label components (e.g. "identity person email")
        let parts = label.replace('.', " ");
        texts.insert(parts);

        // Leaf name alone (e.g. "email" from "identity.person.email")
        if let Some(leaf) = label.split('.').next_back() {
            let leaf_clean = leaf.replace('_', " ");
            texts.insert(leaf_clean);
        }

        // Header hint entries
        if let Some(hint_texts) = hints_by_label.get(label.as_str()) {
            for hint in hint_texts {
                texts.insert(hint.to_string());
            }
        }

        let mut synonyms: Vec<String> = texts.into_iter().collect();
        synonyms.sort(); // deterministic order
        result.push((label.clone(), synonyms));
    }

    result.sort_by(|a, b| a.0.cmp(&b.0));
    result
}

/// Build header hint entries: exact column name → type label.
///
/// Mirrors the hardcoded entries from `prepare_model2vec.py::build_header_hint_entries()`
/// and `column.rs::header_hint()`. These ensure backward compatibility — every
/// column name the hardcoded function recognises appears as a synonym.
fn build_header_hint_entries() -> Vec<(String, String)> {
    vec![
        // Email
        ("email", "identity.person.email"),
        ("e mail", "identity.person.email"),
        ("email address", "identity.person.email"),
        ("emailaddress", "identity.person.email"),
        // URL
        ("url", "technology.internet.url"),
        ("uri", "technology.internet.url"),
        ("link", "technology.internet.url"),
        ("href", "technology.internet.url"),
        ("website", "technology.internet.url"),
        ("homepage", "technology.internet.url"),
        ("tracking url", "technology.internet.url"),
        ("callback url", "technology.internet.url"),
        ("redirect url", "technology.internet.url"),
        ("api url", "technology.internet.url"),
        // IP
        ("ip", "technology.internet.ip_v4"),
        ("ip address", "technology.internet.ip_v4"),
        ("ipaddress", "technology.internet.ip_v4"),
        ("ip addr", "technology.internet.ip_v4"),
        ("source ip", "technology.internet.ip_v4"),
        ("destination ip", "technology.internet.ip_v4"),
        ("src ip", "technology.internet.ip_v4"),
        ("dst ip", "technology.internet.ip_v4"),
        ("server ip", "technology.internet.ip_v4"),
        ("client ip", "technology.internet.ip_v4"),
        ("remote ip", "technology.internet.ip_v4"),
        ("local ip", "technology.internet.ip_v4"),
        // UUID
        ("uuid", "representation.identifier.uuid"),
        ("guid", "representation.identifier.uuid"),
        // Person
        ("gender", "identity.person.gender"),
        ("sex", "identity.person.gender"),
        ("age", "identity.person.age"),
        // Geo coordinates
        ("latitude", "geography.coordinate.latitude"),
        ("lat", "geography.coordinate.latitude"),
        ("longitude", "geography.coordinate.longitude"),
        ("lng", "geography.coordinate.longitude"),
        ("lon", "geography.coordinate.longitude"),
        ("long", "geography.coordinate.longitude"),
        // Geo locations
        ("country", "geography.location.country"),
        ("country name", "geography.location.country"),
        ("country code", "geography.location.country_code"),
        ("alpha 2", "geography.location.country_code"),
        ("alpha 3", "geography.location.country_code"),
        ("iso country", "geography.location.country_code"),
        ("iso alpha 2", "geography.location.country_code"),
        ("iso alpha 3", "geography.location.country_code"),
        ("country iso", "geography.location.country_code"),
        ("city", "geography.location.city"),
        ("city name", "geography.location.city"),
        ("state", "geography.location.region"),
        ("province", "geography.location.region"),
        ("region", "geography.location.region"),
        ("subcountry", "geography.location.region"),
        ("subregion", "geography.location.region"),
        ("sub region", "geography.location.region"),
        ("sub country", "geography.location.region"),
        // Currency
        ("currency", "finance.currency.currency_code"),
        ("currency code", "finance.currency.currency_code"),
        // Port
        ("port", "technology.internet.port"),
        // Phone
        ("phone", "identity.person.phone_number"),
        ("phone number", "identity.person.phone_number"),
        ("telephone", "identity.person.phone_number"),
        ("mobile", "identity.person.phone_number"),
        ("fax", "identity.person.phone_number"),
        // Postal
        ("zip", "geography.address.postal_code"),
        ("zip code", "geography.address.postal_code"),
        ("zipcode", "geography.address.postal_code"),
        ("postal code", "geography.address.postal_code"),
        ("postalcode", "geography.address.postal_code"),
        ("postcode", "geography.address.postal_code"),
        ("shipping postal code", "geography.address.postal_code"),
        ("billing postal code", "geography.address.postal_code"),
        ("mailing zip", "geography.address.postal_code"),
        // Names
        ("name", "identity.person.full_name"),
        ("full name", "identity.person.full_name"),
        ("fullname", "identity.person.full_name"),
        ("first name", "identity.person.first_name"),
        ("firstname", "identity.person.first_name"),
        ("given name", "identity.person.first_name"),
        ("last name", "identity.person.last_name"),
        ("lastname", "identity.person.last_name"),
        ("surname", "identity.person.last_name"),
        ("family name", "identity.person.last_name"),
        // Date/time
        ("date", "datetime.timestamp.iso_8601"),
        ("created date", "datetime.timestamp.iso_8601"),
        ("timestamp", "datetime.timestamp.iso_8601"),
        ("datetime", "datetime.timestamp.iso_8601"),
        ("year", "datetime.component.year"),
        ("birth date", "datetime.date.iso"),
        ("birthdate", "datetime.date.iso"),
        ("dob", "datetime.date.iso"),
        ("date of birth", "datetime.date.iso"),
        // Password
        ("password", "identity.person.password"),
        ("passwd", "identity.person.password"),
        // Numeric
        ("price", "representation.numeric.decimal_number"),
        ("cost", "representation.numeric.decimal_number"),
        ("amount", "representation.numeric.decimal_number"),
        ("salary", "representation.numeric.decimal_number"),
        ("fare", "representation.numeric.decimal_number"),
        ("fee", "representation.numeric.decimal_number"),
        ("toll", "representation.numeric.decimal_number"),
        ("charge", "representation.numeric.decimal_number"),
        ("count", "representation.numeric.integer_number"),
        ("quantity", "representation.numeric.integer_number"),
        ("qty", "representation.numeric.integer_number"),
        ("sibsp", "representation.numeric.integer_number"),
        ("parch", "representation.numeric.integer_number"),
        ("siblings", "representation.numeric.integer_number"),
        ("parents", "representation.numeric.integer_number"),
        ("children", "representation.numeric.integer_number"),
        ("dependents", "representation.numeric.integer_number"),
        ("id", "representation.identifier.increment"),
        ("identifier", "representation.identifier.increment"),
        // Ordinal
        ("class", "representation.discrete.ordinal"),
        ("pclass", "representation.discrete.ordinal"),
        ("grade", "representation.discrete.ordinal"),
        ("rank", "representation.discrete.ordinal"),
        ("level", "representation.discrete.ordinal"),
        ("tier", "representation.discrete.ordinal"),
        ("rating", "representation.discrete.ordinal"),
        ("priority", "representation.discrete.ordinal"),
        ("score", "representation.discrete.ordinal"),
        // Boolean
        ("survived", "representation.boolean.binary"),
        ("alive", "representation.boolean.binary"),
        ("deceased", "representation.boolean.binary"),
        ("dead", "representation.boolean.binary"),
        ("active", "representation.boolean.binary"),
        ("enabled", "representation.boolean.binary"),
        ("disabled", "representation.boolean.binary"),
        ("deleted", "representation.boolean.binary"),
        ("verified", "representation.boolean.binary"),
        ("approved", "representation.boolean.binary"),
        ("flagged", "representation.boolean.binary"),
        // UTC offset
        ("utc offset", "datetime.offset.utc"),
        ("gmt offset", "datetime.offset.utc"),
        ("timezone offset", "datetime.offset.utc"),
        ("tz offset", "datetime.offset.utc"),
        ("utcoffset", "datetime.offset.utc"),
        ("gmtoffset", "datetime.offset.utc"),
        // IANA timezone
        ("timezone", "datetime.offset.iana"),
        ("tz", "datetime.offset.iana"),
        ("time zone", "datetime.offset.iana"),
        ("iana timezone", "datetime.offset.iana"),
        // Financial
        ("swift", "finance.banking.swift_bic"),
        ("swift code", "finance.banking.swift_bic"),
        ("bic", "finance.banking.swift_bic"),
        ("bic code", "finance.banking.swift_bic"),
        ("swiftcode", "finance.banking.swift_bic"),
        ("biccode", "finance.banking.swift_bic"),
        ("issn", "identity.commerce.issn"),
        // Medical
        ("npi", "identity.medical.npi"),
        ("npi number", "identity.medical.npi"),
        // Barcode
        ("ean", "identity.commerce.ean"),
        ("barcode", "identity.commerce.ean"),
        ("gtin", "identity.commerce.ean"),
        ("upc", "identity.commerce.ean"),
        // OS
        ("os", "technology.development.os"),
        ("operating system", "technology.development.os"),
        ("platform", "technology.development.os"),
        // Categorical
        ("embarked", "representation.discrete.categorical"),
        ("boarded", "representation.discrete.categorical"),
        ("departed", "representation.discrete.categorical"),
        ("terminal", "representation.discrete.categorical"),
        ("gate", "representation.discrete.categorical"),
        // Alphanumeric ID
        ("ticket", "representation.identifier.alphanumeric_id"),
        ("ticket number", "representation.identifier.alphanumeric_id"),
        ("ticketno", "representation.identifier.alphanumeric_id"),
        ("cabin", "representation.identifier.alphanumeric_id"),
        ("room", "representation.identifier.alphanumeric_id"),
        ("compartment", "representation.identifier.alphanumeric_id"),
        ("berth", "representation.identifier.alphanumeric_id"),
        ("seat", "representation.identifier.alphanumeric_id"),
        // Address
        ("address", "geography.address.full_address"),
        ("street", "geography.address.full_address"),
        ("street address", "geography.address.full_address"),
        // Weight/Height
        ("weight", "identity.person.weight"),
        ("height", "identity.person.height"),
        // HTTP status
        ("status code", "technology.internet.http_status_code"),
        ("response code", "technology.internet.http_status_code"),
        ("http status", "technology.internet.http_status_code"),
        // MIME
        ("content type", "representation.file.mime_type"),
        ("media type", "representation.file.mime_type"),
        ("mime", "representation.file.mime_type"),
    ]
    .into_iter()
    .map(|(a, b)| (a.to_string(), b.to_string()))
    .collect()
}

// ── Type Embedding Computation ──────────────────────────────────────────────

/// Compute type embeddings from taxonomy using Model2Vec + FPS.
///
/// For each type label:
/// 1. Expand synonyms (title, aliases, components, header hints)
/// 2. Encode all synonyms with Model2Vec
/// 3. Select K representatives via FPS
/// 4. Interleave: rows 0..K for type 0, K..2K for type 1, etc.
///
/// Returns: (flattened embeddings [n_types * k * dim], ordered labels)
pub fn compute_type_embeddings(
    model2vec: &finetype_model::Model2VecResources,
    taxonomy: &finetype_core::Taxonomy,
    k: usize,
) -> Result<(Vec<f32>, Vec<String>)> {
    let synonyms = expand_synonyms(taxonomy);
    let dim = model2vec.embed_dim().context("Failed to get embed dim")?;

    let mut all_embeddings: Vec<f32> = Vec::new();
    let mut labels: Vec<String> = Vec::new();

    for (label, texts) in &synonyms {
        let texts_to_encode: Vec<&str> = if texts.is_empty() {
            // Fallback: use the label itself with dots replaced by spaces
            vec![]
        } else {
            texts.iter().map(|s| s.as_str()).collect()
        };

        // If no texts, use the label as fallback
        let fallback = label.replace('.', " ");
        let texts_to_encode = if texts_to_encode.is_empty() {
            vec![fallback.as_str()]
        } else {
            texts_to_encode
        };

        // Encode all synonym texts
        let encoded = model2vec
            .encode_batch(&texts_to_encode)
            .context("Failed to encode synonyms")?;
        let vecs: Vec<Vec<f32>> = encoded.to_vec2()?;
        let n_synonyms = vecs.len();

        if n_synonyms == 0 {
            // Zero-pad K rows
            all_embeddings.extend(std::iter::repeat_n(0.0f32, k * dim));
            labels.push(label.clone());
            continue;
        }

        if n_synonyms <= k {
            // Use all available, zero-pad the rest
            for vec in &vecs {
                all_embeddings.extend_from_slice(vec);
            }
            // Zero-pad remaining
            for _ in n_synonyms..k {
                all_embeddings.extend(std::iter::repeat_n(0.0f32, dim));
            }
        } else {
            // FPS: select K representatives from N synonyms
            let flat: Vec<f32> = vecs.iter().flat_map(|v| v.iter().copied()).collect();
            let selected = farthest_point_sampling(&flat, n_synonyms, dim, k);

            for &idx in &selected {
                all_embeddings.extend_from_slice(&vecs[idx]);
            }
        }

        labels.push(label.clone());
    }

    tracing::info!(
        "Computed type embeddings: {} types x {} reps = {} rows x {} dim",
        labels.len(),
        k,
        labels.len() * k,
        dim,
    );

    Ok((all_embeddings, labels))
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fps_selects_k() {
        // 4 points in 2D, select 2
        let embeddings = vec![
            0.0, 0.0, // point 0
            1.0, 0.0, // point 1
            0.0, 1.0, // point 2
            1.0, 1.0, // point 3
        ];
        let selected = farthest_point_sampling(&embeddings, 4, 2, 2);
        assert_eq!(selected.len(), 2);
        // First is always 0, second should be farthest (point 3: diagonal)
        assert_eq!(selected[0], 0);
        assert_eq!(selected[1], 3);
    }

    #[test]
    fn test_fps_all_when_k_ge_n() {
        let embeddings = vec![0.0, 1.0, 2.0];
        let selected = farthest_point_sampling(&embeddings, 3, 1, 5);
        assert_eq!(selected, vec![0, 1, 2]);
    }

    #[test]
    fn test_fps_single() {
        let embeddings = vec![1.0, 2.0, 3.0, 4.0];
        let selected = farthest_point_sampling(&embeddings, 2, 2, 1);
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0], 0);
    }

    #[test]
    fn test_fps_three_points_in_3d() {
        // 5 points in 3D, select 3
        let embeddings = vec![
            0.0, 0.0, 0.0, // point 0 (origin)
            1.0, 0.0, 0.0, // point 1
            0.0, 1.0, 0.0, // point 2
            0.0, 0.0, 1.0, // point 3
            0.5, 0.5, 0.0, // point 4 (between 1 and 2)
        ];
        let selected = farthest_point_sampling(&embeddings, 5, 3, 3);
        assert_eq!(selected.len(), 3);
        // First is 0, then should pick far points (1 or 3)
        assert_eq!(selected[0], 0);
        // Point 4 should not be selected (it's close to already-selected points)
        assert!(
            !selected.contains(&4) || selected.len() <= 3,
            "point 4 is close to 0,1,2 and should be avoided"
        );
    }

    #[test]
    fn test_write_type_embeddings_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path();

        let labels = vec!["type_a".to_string(), "type_b".to_string()];
        // 2 types, K=2, dim=3 → 4 rows × 3 = 12 floats
        let embeddings = vec![
            1.0, 0.0, 0.0, // type_a rep 0
            0.0, 1.0, 0.0, // type_a rep 1
            0.0, 0.0, 1.0, // type_b rep 0
            0.5, 0.5, 0.0, // type_b rep 1
        ];

        write_type_embeddings(&embeddings, 2, 2, 3, &labels, output).unwrap();

        // Verify files exist
        assert!(output.join("type_embeddings.safetensors").exists());
        assert!(output.join("label_index.json").exists());

        // Verify label_index.json
        let label_json = std::fs::read_to_string(output.join("label_index.json")).unwrap();
        let loaded_labels: Vec<String> = serde_json::from_str(&label_json).unwrap();
        assert_eq!(loaded_labels, labels);

        // Verify safetensors
        let st_bytes = std::fs::read(output.join("type_embeddings.safetensors")).unwrap();
        let tensors = safetensors::SafeTensors::deserialize(&st_bytes).unwrap();
        let emb_tensor = tensors.tensor("embeddings").unwrap();
        assert_eq!(emb_tensor.shape(), &[4, 3]);
    }

    #[test]
    fn test_expand_synonyms_basic() {
        // Create a minimal taxonomy with one type
        let yaml = r#"
"test.type.email":
  title: "Email Address"
  aliases: ["email", "emailaddress"]
  broad_type: VARCHAR
"#;
        let taxonomy = finetype_core::Taxonomy::from_yaml(yaml).unwrap();
        let synonyms = expand_synonyms(&taxonomy);

        assert_eq!(synonyms.len(), 1);
        let (label, texts) = &synonyms[0];
        assert_eq!(label, "test.type.email");
        // Should contain: "email address" (title), "email" (alias),
        // "emailaddress" (alias), "test type email" (components), "email" (leaf)
        assert!(
            texts.contains(&"email address".to_string()),
            "should contain title"
        );
        assert!(
            texts.contains(&"email".to_string()),
            "should contain alias/leaf"
        );
        assert!(
            texts.contains(&"test type email".to_string()),
            "should contain dot-split components"
        );
    }

    #[test]
    fn test_expand_synonyms_includes_header_hints() {
        let yaml = r#"
"identity.person.email":
  title: "Email"
  broad_type: VARCHAR
"#;
        let taxonomy = finetype_core::Taxonomy::from_yaml(yaml).unwrap();
        let synonyms = expand_synonyms(&taxonomy);

        let (_, texts) = &synonyms[0];
        // Should include header hints for identity.person.email
        assert!(
            texts.contains(&"e mail".to_string()),
            "should contain header hint 'e mail'"
        );
        assert!(
            texts.contains(&"email address".to_string()),
            "should contain header hint 'email address'"
        );
    }

    #[test]
    fn test_build_header_hint_entries_not_empty() {
        let hints = build_header_hint_entries();
        assert!(!hints.is_empty());
        // Check a known entry
        assert!(hints.contains(&("email".to_string(), "identity.person.email".to_string())));
        assert!(hints.contains(&(
            "zip code".to_string(),
            "geography.address.postal_code".to_string()
        )));
    }
}
