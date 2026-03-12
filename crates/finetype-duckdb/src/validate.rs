//! `finetype_validate(value, schema_json)` -- Validate a value against a JSON Schema fragment.
//!
//! Returns 'valid' if the value passes validation, or the first error message if it fails.
//! The schema is compiled once per unique schema string and cached for performance.

use std::collections::HashMap;
use std::sync::Mutex;

use serde_json::Value as JsonValue;

/// Cached result of schema compilation: either a parsed JSON value (we re-compile
/// cheaply per chunk) or an error message for permanently invalid schemas.
enum CachedSchema {
    /// Schema JSON parsed successfully and compiles as a valid JSON Schema.
    Valid(JsonValue),
    /// Schema is permanently invalid (parse error or schema compilation error).
    Invalid(String),
}

/// Thread-safe cache of parsed JSON Schema values, keyed by schema string.
///
/// DuckDB calls the scalar function once per chunk (~2048 rows). The schema argument
/// is typically constant across all rows (from a literal in the SQL query), so caching
/// avoids re-parsing the schema JSON for every row.
static SCHEMA_CACHE: std::sync::LazyLock<Mutex<HashMap<String, CachedSchema>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

/// Validate a value string against a JSON Schema fragment string.
///
/// Returns `"valid"` on success, or the first validation error message on failure.
pub fn validate_value(value: &str, schema_str: &str) -> String {
    // Fast path: check cache
    {
        let cache = SCHEMA_CACHE.lock().unwrap();
        if let Some(cached) = cache.get(schema_str) {
            return match cached {
                CachedSchema::Invalid(err) => format!("schema error: {}", err),
                CachedSchema::Valid(schema_json) => run_validation(value, schema_json),
            };
        }
    }

    // Slow path: parse and validate the schema, then cache
    let schema_json = match serde_json::from_str::<JsonValue>(schema_str) {
        Ok(v) => v,
        Err(e) => {
            let err_msg = e.to_string();
            let mut cache = SCHEMA_CACHE.lock().unwrap();
            cache.insert(
                schema_str.to_string(),
                CachedSchema::Invalid(err_msg.clone()),
            );
            return format!("schema error: {}", err_msg);
        }
    };

    // Verify the schema compiles before caching as valid
    if let Err(e) = jsonschema::validator_for(&schema_json) {
        let err_msg = e.to_string();
        let mut cache = SCHEMA_CACHE.lock().unwrap();
        cache.insert(
            schema_str.to_string(),
            CachedSchema::Invalid(err_msg.clone()),
        );
        return format!("schema error: {}", err_msg);
    }

    let result = run_validation(value, &schema_json);

    let mut cache = SCHEMA_CACHE.lock().unwrap();
    cache.insert(schema_str.to_string(), CachedSchema::Valid(schema_json));

    result
}

/// Run validation of a value against a pre-parsed JSON Schema.
///
/// Compiles the validator from the cached JSON value. The `jsonschema` crate's
/// compilation is fast (~microseconds for simple schemas) so this is acceptable
/// overhead compared to the cost of re-parsing the JSON string.
fn run_validation(value: &str, schema_json: &JsonValue) -> String {
    let validator = match jsonschema::validator_for(schema_json) {
        Ok(v) => v,
        Err(e) => return format!("schema error: {}", e),
    };

    let json_value = JsonValue::String(value.to_string());

    let result = match validator.iter_errors(&json_value).next() {
        None => "valid".to_string(),
        Some(error) => error.to_string(),
    };
    result
}
