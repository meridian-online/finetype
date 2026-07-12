//! Shared helpers for emitting JSON Schema documents.
//!
//! Two emitters live here, one per shape:
//!
//! - `emit_table_schema()` — table-level, one schema per CSV/Parquet file.
//!   Call sites: CLI `profile -o json-schema` (card 0003), MCP `profile`
//!   tool's `format: "json-schema"` branch (card 0003).
//! - `emit_type_schema()` — per-type, one schema per taxonomy definition.
//!   Call sites: CLI `taxonomy KEY -o json-schema` (card 0006), MCP
//!   `taxonomy` tool's `format: "json-schema"` branch (choice 0101 —
//!   absorbed from the retired `schema` tool's type-key branch).
//!
//! Both emitters share the verbosity contract below — exactly two
//! `x-finetype-*` extensions on emitted schemas (`x-finetype-label` and
//! `x-finetype-pii`).
//!
//! The helpers live in `finetype-mcp` because both `finetype-cli` (which
//! depends on this crate) and the MCP tools need them, and the rally
//! deliberately keeps them out of `finetype-core` for v0.6.19.
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
//!
//! An `unknown` column (which carries no type, so the two-extension
//! contract does not apply to it) additionally gets `x-finetype-unknown-reason`
//! when a reason is available — a human-readable "why this is untyped" string
//! (card 0020 honest typing). Like the `--stats` diagnostics, it is an
//! analyst-facing annotation outside the typed-column verbosity contract.

use finetype_core::enum_domain::{detect_enum_domain, label_is_enum_keyword_eligible, EnumConfig};
use finetype_core::{Definition, Taxonomy};
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
    /// For an `unknown` column, a human-readable reason it could not be typed
    /// (card 0020 honest typing — an analyst sees WHY, not just THAT). `None`
    /// for typed columns or when no reason is available.
    pub unknown_reason: Option<&'a str>,
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
            if let Some(reason) = col.unknown_reason {
                prop.insert("x-finetype-unknown-reason".into(), json!(reason));
            }
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

        // Verbosity contract: label + pii.
        prop.insert("x-finetype-label".into(), json!(col.label));
        prop.insert("x-finetype-pii".into(), json!(pii));

        // x-finetype-enum surfaces by DEFAULT (not gated on --stats): a bounded value
        // domain is a first-class analyst signal — a `level` / `status` column shows its
        // members without a flag. `detect_enum_domain` returns None for open /
        // high-cardinality / denylisted columns, so it emits only where a real domain
        // exists. Descriptive only (validators ignore `x-finetype-*`), so the round-trip
        // is unaffected. The heavier observed-data constraints stay under --stats.
        if !col.values.is_empty() {
            attach_enum_domain(&mut prop, col);
        }

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

    // Conservative standard `enum` keyword (a CLOSED validation constraint):
    // categorical/boolean only, to avoid enum_overfit (card 0014). Shared gate so
    // the CLI json-schema and MCP emit the SAME keyword (was cardinality-only — an
    // enum_overfit hazard; spec 2026-06-17-enum-domain-emission).
    if enum_threshold > 0
        && cardinality <= enum_threshold
        && label_is_enum_keyword_eligible(col.label)
    {
        let mut enum_vals: Vec<&str> = unique.iter().copied().collect();
        enum_vals.sort();
        prop.insert("enum".into(), json!(enum_vals));
    }

    // x-finetype-enum moved OUT of the --stats block to the default path
    // (`attach_enum_domain`, called by `emit_table_schema`) so a bounded domain
    // surfaces without a flag.
}

/// `x-finetype-enum`: the OPEN observed value domain for any non-denylisted bounded
/// column (choice 0102). Emitted by DEFAULT (not gated on --stats) — descriptive
/// metadata analysts rely on to see a column's membership ("this is one of these N
/// values"); validators ignore `x-finetype-*`, so the round-trip is unaffected.
/// `detect_enum_domain` returns None for open / high-cardinality / denylisted
/// (numeric/coordinate/datetime/identifier/url) columns, so nothing is emitted where a
/// bounded domain does not genuinely exist.
fn attach_enum_domain(prop: &mut serde_json::Map<String, Value>, col: &TableSchemaColumn<'_>) {
    if let Some(ed) = detect_enum_domain(col.label, col.values, &EnumConfig::default()) {
        prop.insert(
            "x-finetype-enum".into(),
            json!({
                "open": ed.open,
                "distinct": ed.distinct,
                "rows": ed.rows,
                "cohesion": (ed.cohesion * 1000.0).round() / 1000.0,
                "domain": ed.domain,
            }),
        );
    }
}

/// Emit a per-type JSON Schema document for a single taxonomy definition.
///
/// Used by `taxonomy KEY -o json-schema` (CLI, card 0006) and the MCP
/// `taxonomy` tool's `format: "json-schema"` branch (choice 0101). The emitter merges validation
/// keywords from the taxonomy definition (pattern, type, minLength, etc.)
/// alongside the `$schema` / `$id` / `title` / `description` envelope and
/// surfaces sample values as JSON Schema `examples`.
///
/// Verbosity contract (PR #51, extended to type-mode in card 0006): the
/// returned schema carries exactly two `x-finetype-*` extensions —
/// `x-finetype-label` (the canonical type key) and `x-finetype-pii`
/// (boolean from the taxonomy). The pre-existing CLI emitter only
/// surfaced `x-finetype-pii`; card 0006 adds `x-finetype-label` so both
/// emitter surfaces match. Other extensions (broad-type, transform,
/// transform-ext, format-string, domain, confidence) are derivable from
/// the label plus the bundled taxonomy and are deliberately omitted.
pub fn emit_type_schema(label: &str, def: &Definition) -> Value {
    let mut schema = serde_json::Map::new();

    schema.insert(
        "$schema".into(),
        json!("https://json-schema.org/draft/2020-12/schema"),
    );
    schema.insert(
        "$id".into(),
        json!(format!("https://meridian.online/schemas/{}", label)),
    );

    if let Some(title) = &def.title {
        schema.insert("title".into(), json!(title));
    }
    if let Some(desc) = &def.description {
        schema.insert("description".into(), json!(desc.trim()));
    }

    // Merge validation keywords from the type's validation schema.
    if let Some(validation) = &def.validation {
        let val_schema = validation.to_json_schema();
        if let Value::Object(val_obj) = val_schema {
            for (k, v) in val_obj {
                schema.insert(k, v);
            }
        }
    } else {
        // No validation — default to plain string.
        schema.insert("type".into(), json!("string"));
    }

    // Examples from the definition's sample list (when present).
    if !def.samples.is_empty() {
        if let Ok(samples) = serde_json::to_value(&def.samples) {
            schema.insert("examples".into(), samples);
        }
    }

    // Verbosity contract: label + pii only.
    schema.insert("x-finetype-label".into(), json!(label));
    schema.insert("x-finetype-pii".into(), json!(def.pii.unwrap_or(false)));

    Value::Object(schema)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn labels_path() -> PathBuf {
        // Walk up from CARGO_MANIFEST_DIR (crates/finetype-mcp) to
        // workspace root, then into `labels`.
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        manifest.parent().unwrap().parent().unwrap().join("labels")
    }

    // ac-03 parity / behaviour guard (spec 2026-06-17-enum-domain-emission).
    // emit_table_schema is the SHARED json-schema emitter for both the CLI
    // (`profile -o json-schema`) and the MCP profile tool, so this locks the
    // unified enum policy for both surfaces at once.
    #[test]
    fn table_schema_enum_policy_open_domain_denylist_conservative_keyword() {
        let taxonomy = Taxonomy::from_directory(labels_path()).expect("load taxonomy");
        let v = |xs: &[&str]| xs.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        let country = v(&["US", "GB", "FR", "US", "GB", "FR", "US", "GB", "FR", "US"]);
        let word = v(&[
            "red", "green", "blue", "red", "green", "blue", "red", "green",
        ]);
        let ints = v(&["1", "2", "3", "1", "2", "3", "1", "2"]);
        let cols = vec![
            TableSchemaColumn {
                name: "country",
                label: "geography.location.country_code",
                values: &country,
                null_count: 0,
                unknown_reason: None,
            },
            TableSchemaColumn {
                name: "colour",
                label: "representation.text.word",
                values: &word,
                null_count: 0,
                unknown_reason: None,
            },
            TableSchemaColumn {
                name: "n",
                label: "representation.numeric.integer_number",
                values: &ints,
                null_count: 0,
                unknown_reason: None,
            },
        ];
        let schema = emit_table_schema(&cols, "t", "id", &taxonomy, true, 32);
        let props = schema
            .get("properties")
            .and_then(|p| p.as_object())
            .expect("properties");

        // Decoupled open-domain emission: country_code (NOT categorical) gets its domain.
        let c = props.get("country").unwrap();
        assert_eq!(
            c.get("x-finetype-enum").and_then(|e| e.get("domain")),
            Some(&json!(["FR", "GB", "US"])),
            "country_code should emit its open enum domain regardless of label",
        );

        // A `word` column gets the open domain but NOT the conservative `enum`
        // keyword (word is not keyword-eligible — the over-emission fix).
        let w = props.get("colour").unwrap();
        assert!(
            w.get("x-finetype-enum").is_some(),
            "word should get an open domain"
        );
        assert!(
            w.get("enum").is_none(),
            "word must NOT get the closed `enum` keyword (conservative gate)",
        );

        // Denylist: a numeric column gets no enum domain even at low cardinality.
        let n = props.get("n").unwrap();
        assert!(
            n.get("x-finetype-enum").is_none(),
            "integer_number is denylisted — no enum domain",
        );
    }

    // The enum domain is a DEFAULT signal (not gated on --stats): a bounded column
    // shows its members without a flag, while the heavier observed-data constraints
    // (cardinality, null-rate, min/max) stay under --stats.
    #[test]
    fn table_schema_enum_domain_surfaces_without_stats() {
        let taxonomy = Taxonomy::from_directory(labels_path()).expect("load taxonomy");
        let v = |xs: &[&str]| xs.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        // A NAICS-style `level` column: a small bounded numeric-looking vocabulary that
        // the model calls `word` — exactly the enum the author wants visible by default.
        let level = v(&["2", "3", "4", "5", "6", "2", "3", "4", "5", "6", "2", "3"]);
        let cols = vec![TableSchemaColumn {
            name: "level",
            label: "representation.text.word",
            values: &level,
            null_count: 0,
            unknown_reason: None,
        }];
        let schema = emit_table_schema(&cols, "t", "id", &taxonomy, false, 0);
        let props = schema
            .get("properties")
            .and_then(|p| p.as_object())
            .expect("properties");
        let lvl = props.get("level").unwrap();

        // The domain surfaces even with stats=false / enum_threshold=0.
        assert_eq!(
            lvl.get("x-finetype-enum").and_then(|e| e.get("domain")),
            Some(&json!(["2", "3", "4", "5", "6"])),
            "a bounded `word` column must surface its enum domain without --stats",
        );
        // But the --stats-only observed-data fields must NOT appear. (minLength/
        // maxLength are excluded here deliberately — for a `word` label those come
        // from the taxonomy validation merge, not from --stats, so they are present
        // either way; cardinality and null-rate are set ONLY in attach_stats.)
        assert!(
            lvl.get("x-finetype-cardinality").is_none(),
            "cardinality is a --stats field, absent without the flag",
        );
        assert!(
            lvl.get("x-finetype-null-rate").is_none(),
            "null-rate is a --stats field, absent without the flag",
        );
    }

    #[test]
    fn unknown_column_carries_reason_when_present() {
        let taxonomy = Taxonomy::from_directory(labels_path()).expect("load taxonomy");
        let vals: Vec<String> = vec![];
        let cols = vec![
            TableSchemaColumn {
                name: "mystery",
                label: "unknown",
                values: &vals,
                null_count: 3,
                unknown_reason: Some(
                    "validation rejected 'npi': only 12% of values matched its format",
                ),
            },
            TableSchemaColumn {
                name: "bare",
                label: "unknown",
                values: &vals,
                null_count: 3,
                unknown_reason: None,
            },
        ];
        let schema = emit_table_schema(&cols, "t", "id", &taxonomy, false, 0);
        let props = schema
            .get("properties")
            .and_then(|p| p.as_object())
            .unwrap();
        // Reason surfaces alongside the unknown label.
        let m = props.get("mystery").unwrap();
        assert_eq!(m.get("x-finetype-label"), Some(&json!("unknown")));
        assert_eq!(
            m.get("x-finetype-unknown-reason").and_then(|r| r.as_str()),
            Some("validation rejected 'npi': only 12% of values matched its format"),
        );
        // No reason → the key is simply absent (no empty noise).
        let b = props.get("bare").unwrap();
        assert_eq!(b.get("x-finetype-label"), Some(&json!("unknown")));
        assert!(b.get("x-finetype-unknown-reason").is_none());
    }

    #[test]
    fn emit_type_schema_email_carries_label_and_pii_only() {
        let taxonomy = Taxonomy::from_directory(labels_path()).expect("load taxonomy");
        let def = taxonomy
            .get("identity.person.email")
            .expect("email definition present");

        let schema = emit_type_schema("identity.person.email", def);

        // Envelope present.
        assert!(
            schema.get("$schema").and_then(|v| v.as_str()).is_some(),
            "$schema should be set"
        );
        assert!(
            schema.get("$id").and_then(|v| v.as_str()).is_some(),
            "$id should be set"
        );

        // Verbosity-contract extensions present.
        assert_eq!(
            schema.get("x-finetype-label").and_then(|v| v.as_str()),
            Some("identity.person.email"),
            "x-finetype-label should equal the queried key"
        );
        assert_eq!(
            schema.get("x-finetype-pii").and_then(|v| v.as_bool()),
            Some(true),
            "email is PII"
        );

        // Dropped extensions absent (verbosity contract from PR #51).
        for dropped in [
            "x-finetype-broad-type",
            "x-finetype-transform",
            "x-finetype-transform-ext",
            "x-finetype-format-string",
            "x-finetype-domain",
            "x-finetype-confidence",
        ] {
            assert!(
                schema.get(dropped).is_none(),
                "{} should not appear on emitted type schema",
                dropped
            );
        }
    }
}
