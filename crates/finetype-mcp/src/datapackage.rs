//! Frictionless Data Package emitter (choice 0105, spec
//! 2026-06-24-frictionless-datapackage-profile-output ac-02).
//!
//! Emits a conformant Frictionless **Data Package** descriptor for a profiled
//! file: one Data Resource wrapping a Table Schema whose `type`/`format` come
//! from the authoritative `frictionless` map on each taxonomy definition (the
//! 244→16 fold FineType owns for the Meridian family). This is the standard
//! interoperable envelope alongside the existing `json-schema` output —
//! additive, never a replacement.
//!
//! Boundary (decision shared with arcform 0017): a Data Package *describes*, it
//! does not *execute* — the DuckDB `transform`/`broad_type`/`format_string` are
//! NOT emitted; they are recoverable from the label via the bundled taxonomy.
//!
//! ac-02 scope is the standard core `{name, type, format?, constraints?}`. The
//! `x-finetype-*` custom properties are ac-03 and slot into `field_object`.

use crate::json_schema::TableSchemaColumn;
use finetype_core::enum_domain::label_is_enum_keyword_eligible;
use finetype_core::Taxonomy;
use serde_json::{json, Map, Value};
use std::collections::BTreeSet;

/// The Frictionless v2.0 Data Package profile this emitter targets.
pub const DATAPACKAGE_PROFILE: &str = "https://datapackage.org/profiles/2.0/datapackage.json";

/// Resource-level metadata for the single Data Resource. Computed by the caller
/// (which holds the file path) so this crate stays free of filesystem/hashing
/// dependencies — mirrors the borrowed-input shape of [`TableSchemaColumn`].
pub struct ResourceMeta {
    /// URL-usable resource/package identifier (slug of the file stem).
    pub name: String,
    /// POSIX path to the data, relative to the descriptor (the file basename).
    pub path: String,
    /// Format token, e.g. `csv`, `parquet`, `json`, `ndjson`.
    pub format: String,
    /// IANA media type, e.g. `text/csv`.
    pub mediatype: String,
    /// Character encoding (`utf-8`) for text formats; `None` for binary
    /// formats like Parquet, where Frictionless `encoding` does not apply.
    pub encoding: Option<String>,
    /// File size in bytes.
    pub bytes: u64,
    /// `sha256:<hex>` content hash.
    pub hash: String,
    /// ISO-8601 descriptor-creation timestamp (package level).
    pub created: String,
}

/// Emit a Frictionless Data Package descriptor for one profiled file.
///
/// `enum_threshold` gates the closed-categorical `enum` constraint (0 to
/// suppress). The caller chooses pretty-printing and output destination.
pub fn emit_datapackage(
    cols: &[TableSchemaColumn<'_>],
    resource: &ResourceMeta,
    taxonomy: &Taxonomy,
    enum_threshold: usize,
) -> Value {
    let fields: Vec<Value> = cols
        .iter()
        .map(|col| field_object(col, taxonomy, enum_threshold))
        .collect();

    let mut resource_obj = Map::new();
    resource_obj.insert("name".into(), json!(resource.name));
    resource_obj.insert("path".into(), json!(resource.path));
    resource_obj.insert("format".into(), json!(resource.format));
    resource_obj.insert("mediatype".into(), json!(resource.mediatype));
    if let Some(encoding) = &resource.encoding {
        resource_obj.insert("encoding".into(), json!(encoding));
    }
    resource_obj.insert("bytes".into(), json!(resource.bytes));
    resource_obj.insert("hash".into(), json!(resource.hash));
    resource_obj.insert("schema".into(), json!({ "fields": Value::Array(fields) }));

    json!({
        "$schema": DATAPACKAGE_PROFILE,
        "name": resource.name,
        "created": resource.created,
        "resources": [Value::Object(resource_obj)],
    })
}

/// Build one Table Schema field descriptor: `{name, type, format?, constraints?}`.
fn field_object(col: &TableSchemaColumn<'_>, taxonomy: &Taxonomy, enum_threshold: usize) -> Value {
    let def = taxonomy.get(col.label);
    let fx = def.and_then(|d| d.frictionless.as_ref());

    let mut field = Map::new();
    field.insert("name".into(), json!(col.name));

    // Type + format from the authoritative map; unknown/absent → string.
    match fx {
        Some(f) => {
            field.insert("type".into(), json!(f.ftype));
            if let Some(fmt) = &f.format {
                field.insert("format".into(), json!(fmt));
            }
        }
        None => {
            field.insert("type".into(), json!("string"));
        }
    }

    if let Some(constraints) = constraints_for(col, def, enum_threshold) {
        field.insert("constraints".into(), constraints);
    }

    Value::Object(field)
}

/// Frictionless `constraints` for a field: `pattern`/`minLength`/`maxLength`/
/// `minimum`/`maximum` from the type's static validation, plus `enum` ONLY when
/// the column is a closed categorical (observed, enum-keyword-eligible). Returns
/// `None` when no constraint applies.
fn constraints_for(
    col: &TableSchemaColumn<'_>,
    def: Option<&finetype_core::Definition>,
    enum_threshold: usize,
) -> Option<Value> {
    let mut c = Map::new();

    if let Some(v) = def.and_then(|d| d.validation.as_ref()) {
        if let Some(p) = &v.pattern {
            c.insert("pattern".into(), json!(p));
        }
        if let Some(n) = v.min_length {
            c.insert("minLength".into(), json!(n));
        }
        if let Some(n) = v.max_length {
            c.insert("maxLength".into(), json!(n));
        }
        if let Some(x) = v.minimum {
            c.insert("minimum".into(), json!(x));
        }
        if let Some(x) = v.maximum {
            c.insert("maximum".into(), json!(x));
        }
    }

    // Closed-categorical enum: same gate as the json-schema emitter so both
    // surfaces agree (spec 2026-06-17-enum-domain-emission).
    if enum_threshold > 0 && !col.values.is_empty() && label_is_enum_keyword_eligible(col.label) {
        let unique: BTreeSet<&str> = col.values.iter().map(|s| s.as_str()).collect();
        if unique.len() <= enum_threshold {
            let mut vals: Vec<&str> = unique.into_iter().collect();
            vals.sort();
            c.insert("enum".into(), json!(vals));
        }
    }

    if c.is_empty() {
        None
    } else {
        Some(Value::Object(c))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use finetype_core::Taxonomy;

    fn test_taxonomy() -> Taxonomy {
        // Two leaves: a string/email with validation, and a bare date/pattern.
        Taxonomy::from_yaml(
            r#"
identity.person.email:
  broad_type: VARCHAR
  frictionless:
    type: string
    format: email
  validation:
    type: string
    pattern: "^.+@.+$"
    minLength: 5
    maxLength: 254
datetime.date.dmy_slash:
  broad_type: DATE
  frictionless:
    type: date
    format: "%d/%m/%Y"
"#,
        )
        .expect("test taxonomy parses")
    }

    fn meta() -> ResourceMeta {
        ResourceMeta {
            name: "sample".into(),
            path: "sample.csv".into(),
            format: "csv".into(),
            mediatype: "text/csv".into(),
            encoding: Some("utf-8".into()),
            bytes: 42,
            hash: "sha256:deadbeef".into(),
            created: "2026-06-24T00:00:00Z".into(),
        }
    }

    #[test]
    fn emits_conformant_shape_with_map_type_format_and_constraints() {
        let tax = test_taxonomy();
        let cols = vec![
            TableSchemaColumn {
                name: "email",
                label: "identity.person.email",
                values: &[],
                null_count: 0,
            },
            TableSchemaColumn {
                name: "d",
                label: "datetime.date.dmy_slash",
                values: &[],
                null_count: 0,
            },
            // Unknown label → string, no format, no constraints.
            TableSchemaColumn {
                name: "mystery",
                label: "unknown",
                values: &[],
                null_count: 0,
            },
        ];
        let dp = emit_datapackage(&cols, &meta(), &tax, 32);

        assert_eq!(dp["$schema"], json!(DATAPACKAGE_PROFILE));
        assert_eq!(dp["name"], json!("sample"));
        let res = &dp["resources"][0];
        assert_eq!(res["path"], json!("sample.csv"));
        assert_eq!(res["format"], json!("csv"));
        assert_eq!(res["hash"], json!("sha256:deadbeef"));
        assert_eq!(res["encoding"], json!("utf-8"));

        let fields = res["schema"]["fields"].as_array().unwrap();
        assert_eq!(fields.len(), 3);

        // email: string/email + constraints from validation
        assert_eq!(fields[0]["type"], json!("string"));
        assert_eq!(fields[0]["format"], json!("email"));
        assert_eq!(fields[0]["constraints"]["minLength"], json!(5));
        assert_eq!(fields[0]["constraints"]["maxLength"], json!(254));
        assert!(fields[0]["constraints"]["pattern"].is_string());

        // date: date/<pattern>, no validation → no constraints
        assert_eq!(fields[1]["type"], json!("date"));
        assert_eq!(fields[1]["format"], json!("%d/%m/%Y"));
        assert!(fields[1].get("constraints").is_none());

        // unknown: string, no format, no constraints
        assert_eq!(fields[2]["type"], json!("string"));
        assert!(fields[2].get("format").is_none());
        assert!(fields[2].get("constraints").is_none());
    }

    #[test]
    fn omits_encoding_when_none() {
        let tax = test_taxonomy();
        let mut m = meta();
        m.encoding = None;
        let cols = vec![TableSchemaColumn {
            name: "email",
            label: "identity.person.email",
            values: &[],
            null_count: 0,
        }];
        let dp = emit_datapackage(&cols, &m, &tax, 32);
        assert!(dp["resources"][0].get("encoding").is_none());
    }
}
