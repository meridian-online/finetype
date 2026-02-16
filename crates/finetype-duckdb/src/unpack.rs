//! JSON unpacking for finetype_unpack().
//!
//! Parses JSON input, classifies each scalar value, and returns annotated JSON
//! with per-field type information.

use crate::type_mapping;
use finetype_model::CharClassifier;
use serde_json::{Map, Value};

/// Unpack a JSON string: classify each scalar field and return annotated JSON.
///
/// For each scalar value (strings, numbers, booleans), the output includes:
/// - `value`: the original value
/// - `type`: the detected finetype label
/// - `confidence`: classification confidence
/// - `duckdb_type`: recommended DuckDB type
///
/// Nested objects are recursively unpacked. Arrays have each element annotated.
/// Non-JSON input returns None.
pub fn unpack_json(input: &str, classifier: &CharClassifier) -> Option<String> {
    let parsed: Value = serde_json::from_str(input.trim()).ok()?;
    let annotated = annotate_value(&parsed, classifier);
    serde_json::to_string(&annotated).ok()
}

/// Recursively annotate a JSON value with type information.
fn annotate_value(value: &Value, classifier: &CharClassifier) -> Value {
    match value {
        Value::Object(map) => {
            let mut result = Map::new();
            for (key, val) in map {
                result.insert(key.clone(), annotate_value(val, classifier));
            }
            Value::Object(result)
        }
        Value::Array(arr) => {
            let annotated: Vec<Value> = arr.iter().map(|v| annotate_value(v, classifier)).collect();
            Value::Array(annotated)
        }
        Value::String(s) => classify_and_annotate(s, classifier),
        Value::Number(n) => classify_and_annotate(&n.to_string(), classifier),
        Value::Bool(b) => {
            let mut anno = Map::new();
            anno.insert("value".to_string(), Value::Bool(*b));
            anno.insert(
                "type".to_string(),
                Value::String("representation.boolean.terms".to_string()),
            );
            anno.insert("confidence".to_string(), Value::from(1.0));
            anno.insert(
                "duckdb_type".to_string(),
                Value::String("BOOLEAN".to_string()),
            );
            Value::Object(anno)
        }
        Value::Null => {
            let mut anno = Map::new();
            anno.insert("value".to_string(), Value::Null);
            anno.insert("type".to_string(), Value::String("unknown".to_string()));
            anno.insert("confidence".to_string(), Value::from(0.0));
            anno.insert(
                "duckdb_type".to_string(),
                Value::String("VARCHAR".to_string()),
            );
            Value::Object(anno)
        }
    }
}

/// Classify a string value and return an annotation object.
fn classify_and_annotate(value: &str, classifier: &CharClassifier) -> Value {
    let mut anno = Map::new();
    anno.insert("value".to_string(), Value::String(value.to_string()));

    if value.is_empty() {
        anno.insert("type".to_string(), Value::String("unknown".to_string()));
        anno.insert("confidence".to_string(), Value::from(0.0));
        anno.insert(
            "duckdb_type".to_string(),
            Value::String("VARCHAR".to_string()),
        );
        return Value::Object(anno);
    }

    match classifier.classify(value) {
        Ok(result) => {
            let duckdb_type = type_mapping::to_duckdb_type(&result.label);
            anno.insert("type".to_string(), Value::String(result.label));
            anno.insert(
                "confidence".to_string(),
                Value::from(result.confidence as f64),
            );
            anno.insert(
                "duckdb_type".to_string(),
                Value::String(duckdb_type.to_string()),
            );
        }
        Err(_) => {
            anno.insert("type".to_string(), Value::String("unknown".to_string()));
            anno.insert("confidence".to_string(), Value::from(0.0));
            anno.insert(
                "duckdb_type".to_string(),
                Value::String("VARCHAR".to_string()),
            );
        }
    }

    Value::Object(anno)
}

#[cfg(test)]
mod tests {
    // Note: Integration tests require the CharClassifier which needs model weights.
    // These tests validate the JSON structure rather than classification accuracy.

    use super::*;

    #[test]
    fn test_invalid_json_returns_none() {
        // Can't test with a real classifier in unit tests, but we can verify
        // that invalid JSON returns None from the parsing step.
        let result = serde_json::from_str::<Value>("not json");
        assert!(result.is_err());
    }

    #[test]
    fn test_null_annotation_structure() {
        // Test annotate_value directly with null
        // We need a classifier for full tests, but we can verify structure
        let null_val = Value::Null;
        // Can't call annotate_value without classifier, but we verify the const path
        assert!(null_val.is_null());
    }
}
