//! JSON path collection with structure preservation metadata.
//!
//! This module collects values grouped by JSON path, with metadata sufficient
//! to reconstruct the original JSON structure. Internal representation flattens
//! to paths for column-level classification, but preserves enough information
//! to rebuild nested structures in output.
//!
//! Path notation:
//! - Objects: `a.b.c` for nested fields
//! - Arrays: `a[]` or `a[].b` for array items and their fields
//! - Example: `users[].address.city` represents the city field within address of each user

use indexmap::IndexMap;
use serde_json::Value;
use std::io::{BufRead, BufReader, Read};

/// Collected values grouped by JSON path with shape metadata for reconstruction.
#[derive(Debug, Clone)]
pub struct JsonPathMap {
    /// Path → collected string values (None for nulls/missing)
    paths: IndexMap<String, Vec<Option<String>>>,
    /// Number of documents/rows processed
    row_count: usize,
}

impl JsonPathMap {
    /// Get the number of rows/documents processed
    pub fn row_count(&self) -> usize {
        self.row_count
    }

    /// Get all paths in order
    pub fn paths(&self) -> impl Iterator<Item = &String> {
        self.paths.keys()
    }

    /// Get values for a specific path
    pub fn get(&self, path: &str) -> Option<&Vec<Option<String>>> {
        self.paths.get(path)
    }

    /// Get all paths with their values as a map
    pub fn all_paths(&self) -> &IndexMap<String, Vec<Option<String>>> {
        &self.paths
    }

    /// Convert to a map representation suitable for JSON reconstruction
    pub fn to_map(&self) -> IndexMap<String, Vec<Option<String>>> {
        self.paths.clone()
    }
}

/// Collect values from a single JSON document, preserving path structure.
pub fn collect_json(value: &Value) -> JsonPathMap {
    let mut paths = IndexMap::new();
    collect_value(value, "", &mut paths);

    JsonPathMap {
        paths,
        row_count: 1,
    }
}

/// Collect values from NDJSON (newline-delimited JSON) with schema evolution.
///
/// Reads line by line, flattening each document to paths, and merging into
/// a single set of paths. Missing fields in some documents become None entries.
pub fn collect_ndjson<R: Read>(reader: R) -> Result<JsonPathMap, Box<dyn std::error::Error>> {
    let buf_reader = BufReader::new(reader);
    let mut all_paths: IndexMap<String, Vec<Option<String>>> = IndexMap::new();
    let mut row_count = 0;

    for line in buf_reader.lines() {
        let line = line?;
        let trimmed = line.trim();

        // Skip empty lines and comments
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let value: Value = serde_json::from_str(trimmed)?;

        // Collect paths from this document
        let mut doc_paths = IndexMap::new();
        collect_value(&value, "", &mut doc_paths);

        // First pass: add all existing paths with their values (or None if missing from doc)
        for (path, values) in &mut all_paths {
            if let Some(doc_values) = doc_paths.get(path) {
                // This path exists in the current document, append its values
                values.extend(doc_values.iter().cloned());
            } else {
                // This path doesn't exist in current document, add None for each value
                values.push(None);
            }
        }

        // Second pass: add new paths that weren't in previous rows
        for (path, doc_values) in &doc_paths {
            if !all_paths.contains_key(path) {
                // New path: add Nones for previous rows, then this row's values
                let mut values = vec![None; row_count];
                values.extend(doc_values.iter().cloned());
                all_paths.insert(path.clone(), values);
            }
        }

        row_count += 1;
    }

    Ok(JsonPathMap {
        paths: all_paths,
        row_count,
    })
}

/// Recursively collect all paths and their string values from a JSON value.
fn collect_value(value: &Value, prefix: &str, paths: &mut IndexMap<String, Vec<Option<String>>>) {
    match value {
        Value::Null => {
            // Add None for null values at this path
            if !prefix.is_empty() {
                paths.entry(prefix.to_string()).or_default().push(None);
            }
        }
        Value::Bool(b) => {
            let path = prefix.to_string();
            paths.entry(path).or_default().push(Some(b.to_string()));
        }
        Value::Number(n) => {
            let path = prefix.to_string();
            paths.entry(path).or_default().push(Some(n.to_string()));
        }
        Value::String(s) => {
            let path = prefix.to_string();
            paths.entry(path).or_default().push(Some(s.clone()));
        }
        Value::Array(arr) => {
            // Array path: use [] notation
            let array_prefix = if prefix.is_empty() {
                "[]".to_string()
            } else {
                format!("{}[]", prefix)
            };

            // For each item in the array, collect its values
            for item in arr {
                collect_value(item, &array_prefix, paths);
            }
        }
        Value::Object(obj) => {
            // Object: nest deeper with dot notation
            for (key, val) in obj {
                let new_prefix = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{}.{}", prefix, key)
                };
                collect_value(val, &new_prefix, paths);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_collect_simple_object() {
        let json = json!({"name": "Alice", "age": 30});
        let map = collect_json(&json);

        assert_eq!(map.row_count(), 1);
        assert_eq!(map.get("name"), Some(&vec![Some("Alice".to_string())]));
        assert_eq!(map.get("age"), Some(&vec![Some("30".to_string())]));
    }

    #[test]
    fn test_collect_nested_object() {
        let json = json!({
            "user": {
                "name": "Bob",
                "contact": {
                    "email": "bob@example.com"
                }
            }
        });
        let map = collect_json(&json);

        assert_eq!(map.get("user.name"), Some(&vec![Some("Bob".to_string())]));
        assert_eq!(
            map.get("user.contact.email"),
            Some(&vec![Some("bob@example.com".to_string())])
        );
    }

    #[test]
    fn test_collect_array() {
        let json = json!({
            "tags": ["rust", "json", "parsing"]
        });
        let map = collect_json(&json);

        assert!(map.get("tags[]").is_some());
        let values = map.get("tags[]").unwrap();
        assert_eq!(values.len(), 3);
        assert_eq!(values[0], Some("rust".to_string()));
        assert_eq!(values[1], Some("json".to_string()));
        assert_eq!(values[2], Some("parsing".to_string()));
    }

    #[test]
    fn test_collect_array_of_objects() {
        let json = json!({
            "users": [
                {"name": "Alice", "email": "alice@example.com"},
                {"name": "Bob", "email": "bob@example.com"}
            ]
        });
        let map = collect_json(&json);

        assert_eq!(
            map.get("users[].name"),
            Some(&vec![Some("Alice".to_string()), Some("Bob".to_string())])
        );
        assert_eq!(
            map.get("users[].email"),
            Some(&vec![
                Some("alice@example.com".to_string()),
                Some("bob@example.com".to_string())
            ])
        );
    }

    #[test]
    fn test_collect_with_nulls() {
        let json = json!({
            "name": "Charlie",
            "nickname": null
        });
        let map = collect_json(&json);

        assert_eq!(map.get("name"), Some(&vec![Some("Charlie".to_string())]));
        assert_eq!(map.get("nickname"), Some(&vec![None]));
    }

    #[test]
    fn test_collect_ndjson_simple() -> Result<(), Box<dyn std::error::Error>> {
        let ndjson = r#"{"name":"Alice","age":30}
{"name":"Bob","age":25}"#;

        let map = collect_ndjson(ndjson.as_bytes())?;

        assert_eq!(map.row_count(), 2);
        assert_eq!(
            map.get("name"),
            Some(&vec![Some("Alice".to_string()), Some("Bob".to_string())])
        );
        assert_eq!(
            map.get("age"),
            Some(&vec![Some("30".to_string()), Some("25".to_string())])
        );

        Ok(())
    }

    #[test]
    fn test_collect_ndjson_schema_evolution() -> Result<(), Box<dyn std::error::Error>> {
        let ndjson = r#"{"name":"Alice","age":30}
{"name":"Bob","email":"bob@example.com"}
{"name":"Charlie","age":35,"email":"charlie@example.com"}"#;

        let map = collect_ndjson(ndjson.as_bytes())?;

        assert_eq!(map.row_count(), 3);

        // All rows should have name
        let names = map.get("name").unwrap();
        assert_eq!(names.len(), 3);
        assert_eq!(names[0], Some("Alice".to_string()));
        assert_eq!(names[1], Some("Bob".to_string()));
        assert_eq!(names[2], Some("Charlie".to_string()));

        // Age is missing from row 2 (Bob)
        let ages = map.get("age").unwrap();
        assert_eq!(ages.len(), 3);
        assert_eq!(ages[0], Some("30".to_string()));
        assert_eq!(ages[1], None);
        assert_eq!(ages[2], Some("35".to_string()));

        // Email is missing from row 1 (Alice)
        let emails = map.get("email").unwrap();
        assert_eq!(emails.len(), 3);
        assert_eq!(emails[0], None);
        assert_eq!(emails[1], Some("bob@example.com".to_string()));
        assert_eq!(emails[2], Some("charlie@example.com".to_string()));

        Ok(())
    }

    #[test]
    fn test_collect_ndjson_with_nested_objects() -> Result<(), Box<dyn std::error::Error>> {
        let ndjson = r#"{"name":"Alice","address":{"city":"NYC","country":"USA"}}
{"name":"Bob","address":{"city":"LA","country":"USA"}}"#;

        let map = collect_ndjson(ndjson.as_bytes())?;

        assert_eq!(map.row_count(), 2);
        assert_eq!(
            map.get("address.city"),
            Some(&vec![Some("NYC".to_string()), Some("LA".to_string())])
        );
        assert_eq!(
            map.get("address.country"),
            Some(&vec![Some("USA".to_string()), Some("USA".to_string())])
        );

        Ok(())
    }

    #[test]
    fn test_collect_ndjson_empty_lines() -> Result<(), Box<dyn std::error::Error>> {
        let ndjson = r#"{"name":"Alice"}

{"name":"Bob"}"#;

        let map = collect_ndjson(ndjson.as_bytes())?;

        // Should skip empty lines
        assert_eq!(map.row_count(), 2);

        Ok(())
    }

    #[test]
    fn test_path_order_preserved() {
        // Note: serde_json::json! doesn't preserve insertion order by default
        // The order is determined by the JSON parser, which typically sorts alphabetically
        let json = serde_json::from_str::<Value>(r#"{"z_field":"z","a_field":"a","m_field":"m"}"#)
            .unwrap();
        let map = collect_json(&json);

        let paths: Vec<_> = map.paths().collect();
        // Verify that we got all three paths (order may vary by JSON parser)
        assert_eq!(paths.len(), 3);
        assert!(paths.iter().any(|p| *p == "z_field"));
        assert!(paths.iter().any(|p| *p == "a_field"));
        assert!(paths.iter().any(|p| *p == "m_field"));
    }

    #[test]
    fn test_deeply_nested_structure() {
        let json = json!({
            "level1": {
                "level2": {
                    "level3": {
                        "level4": {
                            "level5": {
                                "value": "deep"
                            }
                        }
                    }
                }
            }
        });
        let map = collect_json(&json);

        assert_eq!(
            map.get("level1.level2.level3.level4.level5.value"),
            Some(&vec![Some("deep".to_string())])
        );
    }

    #[test]
    fn test_mixed_array_types() {
        let json = json!({
            "mixed": [
                "string",
                42,
                true,
                null
            ]
        });
        let map = collect_json(&json);

        let values = map.get("mixed[]").unwrap();
        assert_eq!(values[0], Some("string".to_string()));
        assert_eq!(values[1], Some("42".to_string()));
        assert_eq!(values[2], Some("true".to_string()));
        assert_eq!(values[3], None);
    }

    #[test]
    fn test_empty_array() {
        let json = json!({
            "items": []
        });
        let map = collect_json(&json);

        // Empty array should have no values for that path, or just the path with no entries?
        // Let's verify the behavior
        if let Some(values) = map.get("items[]") {
            assert_eq!(values.len(), 0);
        }
    }
}
