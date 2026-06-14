//! Profile I/O helpers: JSON/CSV input reading and JSON output reconstruction.

use super::*;

// ═══════════════════════════════════════════════════════════════════════════════
// PROFILE HELPERS — JSON/CSV input reading and JSON output reconstruction
// ═══════════════════════════════════════════════════════════════════════════════

/// Read JSON or NDJSON input into (headers, columns, row_count).
pub(crate) fn read_json_input(
    file: &std::path::Path,
    ext: &str,
) -> Result<(Vec<String>, Vec<Vec<String>>, usize)> {
    use finetype_core::json_reader;

    if ext == "json" {
        let content = std::fs::read_to_string(file)?;
        let value: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| anyhow::anyhow!("Malformed JSON in {:?}: {}", file, e))?;

        match &value {
            serde_json::Value::Array(arr) => {
                // Top-level array → treat as multi-row
                let mut all_paths: indexmap::IndexMap<String, Vec<Option<String>>> =
                    indexmap::IndexMap::new();
                let row_count = arr.len();
                for item in arr {
                    let item_map = json_reader::collect_json(item);
                    for (path, values) in item_map.all_paths() {
                        let entry = all_paths.entry(path.clone()).or_default();
                        entry.extend(values.iter().cloned());
                    }
                    // Fill missing paths with None
                    for (path, values) in &mut all_paths {
                        if !item_map.all_paths().contains_key(path) {
                            values.push(None);
                        }
                    }
                }
                let headers: Vec<String> = all_paths.keys().cloned().collect();
                let columns: Vec<Vec<String>> = all_paths
                    .values()
                    .map(|vals| {
                        vals.iter()
                            .filter_map(|v| v.clone())
                            .filter(|v| !v.is_empty())
                            .collect()
                    })
                    .collect();
                eprintln!(
                    "Found {} paths across {} array elements",
                    headers.len(),
                    row_count
                );
                Ok((headers, columns, row_count))
            }
            serde_json::Value::Object(_) => {
                let path_map = json_reader::collect_json(&value);
                let headers: Vec<String> = path_map.paths().cloned().collect();
                let columns: Vec<Vec<String>> = headers
                    .iter()
                    .map(|h| {
                        path_map
                            .get(h)
                            .map(|vals| {
                                vals.iter()
                                    .filter_map(|v| v.clone())
                                    .filter(|v| !v.is_empty())
                                    .collect()
                            })
                            .unwrap_or_default()
                    })
                    .collect();
                let row_count = path_map.row_count();
                eprintln!("Found {} paths in single JSON document", headers.len());
                Ok((headers, columns, row_count))
            }
            _ => {
                anyhow::bail!(
                    "JSON input must be an object or array of objects, got scalar value in {:?}",
                    file
                );
            }
        }
    } else {
        // NDJSON/JSONL: read line by line
        let reader = std::fs::File::open(file)?;
        let path_map = json_reader::collect_ndjson(reader)
            .map_err(|e| anyhow::anyhow!("Error reading NDJSON from {:?}: {}", file, e))?;

        let headers: Vec<String> = path_map.paths().cloned().collect();
        let columns: Vec<Vec<String>> = headers
            .iter()
            .map(|h| {
                path_map
                    .get(h)
                    .map(|vals| {
                        vals.iter()
                            .filter_map(|v| v.clone())
                            .filter(|v| !v.is_empty())
                            .collect()
                    })
                    .unwrap_or_default()
            })
            .collect();
        let row_count = path_map.row_count();
        eprintln!(
            "Found {} paths across {} NDJSON documents",
            headers.len(),
            row_count
        );
        Ok((headers, columns, row_count))
    }
}

/// Read CSV input into (headers, columns, row_count).
pub(crate) fn read_csv_input(
    file: &std::path::Path,
    delimiter: Option<char>,
) -> Result<(Vec<String>, Vec<Vec<String>>, usize)> {
    let mut reader_builder = csv::ReaderBuilder::new();
    reader_builder.flexible(true);
    if let Some(delim) = delimiter {
        reader_builder.delimiter(delim as u8);
    }
    let mut reader = reader_builder.from_path(file)?;

    let headers: Vec<String> = reader.headers()?.iter().map(|h| h.to_string()).collect();
    let n_cols = headers.len();
    eprintln!("Found {} columns: {:?}", n_cols, headers);

    let mut columns: Vec<Vec<String>> = vec![Vec::new(); n_cols];
    let mut row_count = 0;

    for result in reader.records() {
        let record = result?;
        row_count += 1;
        for (i, field) in record.iter().enumerate() {
            if i < n_cols {
                let trimmed = field.trim();
                if !trimmed.is_empty()
                    && trimmed != "NULL"
                    && trimmed != "null"
                    && trimmed != "NA"
                    && trimmed != "N/A"
                    && trimmed != "nan"
                    && trimmed != "NaN"
                    && trimmed != "None"
                {
                    columns[i].push(trimmed.to_string());
                }
            }
        }
    }

    Ok((headers, columns, row_count))
}

/// Extract the leaf component from a JSON path for use as header hint.
/// "users[].address.city" → "city"
/// "users[]" → "users"
/// "email" → "email"
pub(crate) fn path_leaf(path: &str) -> String {
    // Remove trailing [] brackets
    let clean = path.trim_end_matches("[]");
    // Take the last component after dot
    if let Some(pos) = clean.rfind('.') {
        clean[pos + 1..].to_string()
    } else {
        clean.to_string()
    }
}

/// Reconstruct a nested JSON schema from flat path profiles.
/// Converts flat paths like "users[].address.city" into nested structure.
pub(crate) fn reconstruct_json_schema(
    profiles: &[(String, String, Option<String>, f32)],
) -> serde_json::Value {
    let mut root = serde_json::Map::new();

    for (name, label, broad_type, confidence) in profiles {
        if label == "unknown" {
            continue;
        }

        let type_info = {
            let mut obj = serde_json::Map::new();
            obj.insert("type".to_string(), json!(label));
            if let Some(bt) = broad_type {
                obj.insert("broad_type".to_string(), json!(bt));
            }
            obj.insert(
                "confidence".to_string(),
                json!(format!("{:.1}%", confidence * 100.0)),
            );
            serde_json::Value::Object(obj)
        };

        insert_path(&mut root, name, type_info);
    }

    serde_json::Value::Object(root)
}

/// Insert a type_info value at a nested path within a JSON map.
/// Handles both dot notation (a.b) and array notation (a[]).
pub(crate) fn insert_path(
    root: &mut serde_json::Map<String, serde_json::Value>,
    path: &str,
    value: serde_json::Value,
) {
    let parts: Vec<&str> = path.split('.').collect();

    if parts.len() == 1 {
        let key = parts[0];
        if let Some(name) = key.strip_suffix("[]") {
            let entry = root
                .entry(name.to_string())
                .or_insert_with(|| json!({"_array": true}));
            if let serde_json::Value::Object(obj) = entry {
                obj.insert("_items".to_string(), value);
            }
        } else {
            root.insert(key.to_string(), value);
        }
        return;
    }

    let key = parts[0];
    let rest = parts[1..].join(".");

    if let Some(name) = key.strip_suffix("[]") {
        let entry = root
            .entry(name.to_string())
            .or_insert_with(|| json!({"_array": true}));
        if let serde_json::Value::Object(obj) = entry {
            obj.insert("_array".to_string(), json!(true));
            let items = obj
                .entry("_items".to_string())
                .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
            if let serde_json::Value::Object(items_map) = items {
                insert_path(items_map, &rest, value);
            }
        }
    } else {
        let entry = root
            .entry(key.to_string())
            .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
        if let serde_json::Value::Object(obj) = entry {
            insert_path(obj, &rest, value);
        }
    }
}
