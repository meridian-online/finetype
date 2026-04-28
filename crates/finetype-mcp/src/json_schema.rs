//! Shared helper for emitting table-level JSON Schema documents.
//!
//! All three call sites route through `emit_table_schema()` so the output
//! stays byte-identical across surfaces:
//!
//! - CLI `profile -o json-schema` (card 0003)
//! - CLI `cmd_schema_table` (legacy `finetype schema <file.csv>`, retained
//!   through v0.6.19 only — card 0006 deletes it)
//! - MCP `profile` tool's `format: "json-schema"` branch (card 0003)
//!
//! The helper lives in `finetype-mcp` because both `finetype-cli` (which
//! depends on this crate) and the MCP tools need it, and the rally
//! deliberately keeps it out of `finetype-core` for v0.6.19. Card 0006
//! may revisit the home if type-mode export grows beyond CLI-only use.
//!
//! ## Verbosity contract (PR #51)
//!
//! Emitted JSON Schema documents carry exactly two `x-finetype-*`
//! extensions on each column property: `x-finetype-label` and
//! `x-finetype-pii`. The other extensions (broad-type, transform,
//! transform-ext, format-string, domain, confidence) are derivable from
//! the label + bundled taxonomy, and were dropped in PR #51 to keep the
//! schema export contract tight. Do not re-introduce them in this card.
//!
//! Under `--stats`, the helper additionally attaches observed-data
//! constraints — `minLength`, `maxLength` for strings; `minimum`,
//! `maximum` for numerics; `enum` (gated by `enum_threshold`); plus the
//! `x-finetype-null-rate` and `x-finetype-cardinality` diagnostic
//! extensions. Stats are observed-from-the-input numbers, not part of
//! the type contract, and live outside the verbosity contract above.

use finetype_core::Taxonomy;
use serde_json::{json, Value};
use std::collections::BTreeSet;

/// Per-column input to the schema emitter.
///
/// Borrowed slices keep the call sites zero-copy. Both `cmd_profile` and
/// `cmd_schema_table` already hold the underlying `Vec<String>` for each
/// column, so threading borrows is the natural shape.
pub struct TableSchemaColumn<'a> {
    /// Raw column header.
    pub name: &'a str,
    /// FineType label (e.g. `identity.person.email`) or `unknown`.
    pub label: &'a str,
    /// Non-null observed values from the column (used only when `stats`).
    pub values: &'a [String],
    /// Number of null/empty cells observed in the column.
    pub null_count: usize,
}

/// Emit a table-level JSON Schema document.
///
/// `file_stem` and `file_id` populate `title` and `$id`. The function
/// returns a `serde_json::Value`; callers are responsible for choosing
/// pretty-printing and the output destination (stdout / sidecar file).
///
/// `stats` toggles the observed-data constraints described in the
/// module-level docs. `enum_threshold` controls when an `enum` keyword is
/// emitted under `--stats` — set to 0 to suppress entirely.
pub fn emit_table_schema(
    cols: &[TableSchemaColumn<'_>],
    file_stem: &str,
    file_id: &str,
    taxonomy: &Taxonomy,
    stats: bool,
    enum_threshold: usize,
) -> Value {
    let mut properties = serde_json::Map::new();
    let mut required: Vec<String> = Vec::new();

    for col in cols {
        if col.label == "unknown" {
            // Unknown columns ship as plain string with the label extension.
            let mut prop = serde_json::Map::new();
            prop.insert("type".into(), json!("string"));
            prop.insert("x-finetype-label".into(), json!("unknown"));
            properties.insert(col.name.to_string(), Value::Object(prop));
            continue;
        }

        let mut prop = serde_json::Map::new();

        let pii = if let Some(def) = taxonomy.get(col.label) {
            // Merge validation keywords from the type definition.
            if let Some(validation) = &def.validation {
                let val_schema = validation.to_json_schema();
                if let Value::Object(val_obj) = val_schema {
                    for (k, v) in val_obj {
                        prop.insert(k, v);
                    }
                }
            } else {
                prop.insert("type".into(), json!("string"));
            }
            def.pii.unwrap_or(false)
        } else {
            // Label not found in taxonomy — basic string schema.
            prop.insert("type".into(), json!("string"));
            false
        };

        // Verbosity contract: only label + pii.
        prop.insert("x-finetype-label".into(), json!(col.label));
        prop.insert("x-finetype-pii".into(), json!(pii));

        if stats && !col.values.is_empty() {
            attach_stats(&mut prop, col, taxonomy, enum_threshold);
        }

        if col.null_count == 0 {
            required.push(col.name.to_string());
        }

        properties.insert(col.name.to_string(), Value::Object(prop));
    }

    let mut schema = serde_json::Map::new();
    schema.insert(
        "$schema".into(),
        json!("https://json-schema.org/draft/2020-12/schema"),
    );
    schema.insert("$id".into(), json!(format!("finetype://{}", file_id)));
    schema.insert("title".into(), json!(file_stem));
    schema.insert("type".into(), json!("object"));
    schema.insert("properties".into(), Value::Object(properties));
    if !required.is_empty() {
        required.sort();
        schema.insert("required".into(), json!(required));
    }

    Value::Object(schema)
}

/// Attach the `--stats` observed-data constraints + diagnostic extensions
/// to a per-column property map.
fn attach_stats(
    prop: &mut serde_json::Map<String, Value>,
    col: &TableSchemaColumn<'_>,
    taxonomy: &Taxonomy,
    enum_threshold: usize,
) {
    let total = col.values.len() + col.null_count;
    let null_rate = if total > 0 {
        col.null_count as f64 / total as f64
    } else {
        0.0
    };
    prop.insert(
        "x-finetype-null-rate".into(),
        json!((null_rate * 10000.0).round() / 10000.0),
    );

    let unique: BTreeSet<&str> = col.values.iter().map(|s| s.as_str()).collect();
    let cardinality = unique.len();
    prop.insert("x-finetype-cardinality".into(), json!(cardinality));

    let is_numeric = taxonomy
        .get(col.label)
        .and_then(|d| d.broad_type.as_ref())
        .map(|bt| {
            let bt_upper = bt.to_uppercase();
            bt_upper.contains("INT")
                || bt_upper.contains("DOUBLE")
                || bt_upper.contains("FLOAT")
                || bt_upper.contains("DECIMAL")
                || bt_upper.contains("NUMERIC")
        })
        .unwrap_or(false);

    if is_numeric {
        let mut nums: Vec<f64> = col
            .values
            .iter()
            .filter_map(|v| v.replace(',', "").parse::<f64>().ok())
            .collect();
        if !nums.is_empty() {
            nums.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            prop.insert("minimum".into(), json!(nums[0]));
            prop.insert("maximum".into(), json!(nums[nums.len() - 1]));
        }
    } else {
        let lengths: Vec<usize> = col.values.iter().map(|v| v.len()).collect();
        if let (Some(&min_len), Some(&max_len)) = (lengths.iter().min(), lengths.iter().max()) {
            prop.insert("minLength".into(), json!(min_len));
            prop.insert("maxLength".into(), json!(max_len));
        }
    }

    if enum_threshold > 0 && cardinality <= enum_threshold {
        let mut enum_vals: Vec<&str> = unique.into_iter().collect();
        enum_vals.sort();
        prop.insert("enum".into(), json!(enum_vals));
    }
}
