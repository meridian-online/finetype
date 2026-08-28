//! Frictionless Data Package emitter (choice 0105, spec
//! 2026-06-24-frictionless-datapackage-profile-output ac-02 + ac-03).
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
//! - ac-02: the standard core `{name, type, format?, constraints?}`.
//! - ac-03: FineType richness as `x-finetype-*` custom properties (never fork
//!   the core spec). A descriptor stripped of all `x-` keys is a valid plain
//!   Data Package — the vendored v2.0 profile sets `additionalProperties: false`
//!   nowhere, so the extensions validate cleanly.

use finetype_core::enum_domain::{detect_enum_domain, label_is_enum_keyword_eligible, EnumConfig};
use finetype_core::frictionless_vocabulary::constraint_vocabulary;
use finetype_core::{PatternFit, Taxonomy};
use serde_json::{json, Map, Value};
use std::collections::BTreeSet;

/// The Frictionless v2.0 Data Package profile this emitter targets.
pub const DATAPACKAGE_PROFILE: &str = "https://datapackage.org/profiles/2.0/datapackage.json";

/// Per-column input to the Data Package emitter. Carries the profile-derived
/// signals the shared [`crate::json_schema::TableSchemaColumn`] does not
/// (`confidence`, `locale`), so the json-schema surface stays untouched.
pub struct DatapackageColumn<'a> {
    /// Raw column header.
    pub name: &'a str,
    /// FineType label (e.g. `identity.person.email`) or `unknown`.
    pub label: &'a str,
    /// Observed column values (used for the closed-categorical `enum`
    /// constraint and the open `x-finetype-enum-domain`).
    pub values: &'a [String],
    /// Inference confidence (0..1), surfaced as `x-finetype-confidence`.
    pub confidence: Option<f32>,
    /// Detected locale for locale-specific types, surfaced as
    /// `x-finetype-locale`. `None` / `UNIVERSAL` is omitted.
    pub locale: Option<&'a str>,
}

/// Resource-level metadata for the single Data Resource. Computed by the caller
/// (which holds the file path) so this crate stays free of filesystem/hashing
/// dependencies.
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

impl ResourceMeta {
    /// Build resource metadata for a file on disk (`content` is its raw bytes,
    /// already read by the caller): format/mediatype from the extension, sha256
    /// hash + size from the bytes, `encoding` for text formats, `created` = now.
    pub fn for_path(path: &std::path::Path, content: &[u8]) -> Self {
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("table");
        let name = slugify(stem);
        let rel = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("data")
            .to_string();
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let (format, mediatype, text) = format_for_ext(&ext);
        Self::assemble(name, rel, format, mediatype, text, content)
    }

    /// Build resource metadata for inline data with no file path (MCP `data`
    /// argument). Assumes CSV; `name`/`path` use a synthetic `data` stem.
    pub fn for_inline(content: &[u8]) -> Self {
        Self::assemble(
            "data".into(),
            "data.csv".into(),
            "csv".into(),
            "text/csv",
            true,
            content,
        )
    }

    fn assemble(
        name: String,
        path: String,
        format: String,
        mediatype: &str,
        text: bool,
        content: &[u8],
    ) -> Self {
        use sha2::{Digest, Sha256};
        ResourceMeta {
            name,
            path,
            format,
            mediatype: mediatype.to_string(),
            encoding: text.then(|| "utf-8".to_string()),
            bytes: content.len() as u64,
            hash: format!("sha256:{:x}", Sha256::digest(content)),
            created: chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        }
    }
}

/// (format token, mediatype, is-text). Text formats carry `encoding`.
fn format_for_ext(ext: &str) -> (String, &'static str, bool) {
    match ext {
        "csv" | "" => ("csv".into(), "text/csv", true),
        "tsv" => ("tsv".into(), "text/tab-separated-values", true),
        "parquet" => ("parquet".into(), "application/vnd.apache.parquet", false),
        "ndjson" | "jsonl" => ("ndjson".into(), "application/x-ndjson", true),
        "json" => ("json".into(), "application/json", true),
        other => (other.to_string(), "application/octet-stream", false),
    }
}

/// Slug a file stem into a Frictionless-legal name: lowercase, keeping only
/// `[a-z0-9._-]`, other chars → `-`.
fn slugify(s: &str) -> String {
    let slug: String = s
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_') {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let trimmed = slug.trim_matches('-');
    if trimmed.is_empty() {
        "resource".to_string()
    } else {
        trimmed.to_string()
    }
}

/// Emit a Frictionless Data Package descriptor for one profiled file.
///
/// `enum_threshold` gates the closed-categorical `enum` constraint (0 to
/// suppress). The caller chooses pretty-printing and output destination.
pub fn emit_datapackage(
    cols: &[DatapackageColumn<'_>],
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

/// Build one Table Schema field: `{name, type, format?, constraints?}` plus the
/// `x-finetype-*` custom properties (ac-03).
fn field_object(col: &DatapackageColumn<'_>, taxonomy: &Taxonomy, enum_threshold: usize) -> Value {
    let def = taxonomy.get(col.label);
    let fx = def.and_then(|d| d.frictionless.as_ref());

    let mut field = Map::new();
    field.insert("name".into(), json!(col.name));

    // ── ac-02 core: type + format from the authoritative map ──
    let ftype = fx.map(|f| f.ftype.as_str()).unwrap_or("string");
    field.insert("type".into(), json!(ftype));
    if let Some(fmt) = fx.and_then(|f| f.format.as_ref()) {
        field.insert("format".into(), json!(fmt));
    }

    // The published `pattern` is fitted to the values observed in THIS column,
    // not taken from the type. A correct label whose column carries legitimate
    // variants the type's canonical pattern rejects would otherwise ship a
    // constraint the data violates.
    let fit = def
        .and_then(|d| d.validation.as_ref())
        .and_then(|v| v.fit_pattern(col.values));

    let FieldConstraints { kept, off_type } = constraints_for(
        col.label,
        col.values,
        def,
        ftype,
        enum_threshold,
        fit.as_ref(),
    );
    if !kept.is_empty() {
        field.insert("constraints".into(), Value::Object(kept));
    }

    // ── ac-03 custom properties (lossless carriers the fold drops) ──
    field.insert("x-finetype-label".into(), json!(col.label));
    // Constraints the type's own validation states but v2 has no place for on
    // THIS declared type. Kept beside the field rather than dropped: a
    // `datetime.date.iso` column really does match that pattern and really is
    // ten characters long, and a reader may already be using it.
    if !off_type.is_empty() {
        field.insert(
            "x-finetype-unsupported-constraints".into(),
            Value::Object(off_type),
        );
    }
    // A widened or dropped pattern is stated rather than done silently: the
    // consumer can see which rule the type would have imposed and that the
    // column's own values are why it did not survive verbatim.
    if let Some(fit) = &fit {
        if let Some(outcome) = fit.outcome() {
            field.insert(
                "x-finetype-pattern-fit".into(),
                json!({ "outcome": outcome, "canonical": fit.canonical() }),
            );
        }
    }
    if let Some(conf) = col.confidence {
        field.insert(
            "x-finetype-confidence".into(),
            json!((conf as f64 * 10000.0).round() / 10000.0),
        );
    }
    if def.and_then(|d| d.pii).unwrap_or(false) {
        field.insert("x-finetype-pii".into(), json!(true));
    }
    if let Some(loc) = col.locale {
        if !loc.is_empty() && loc != "UNIVERSAL" {
            field.insert("x-finetype-locale".into(), json!(loc));
        }
    }
    // The OPEN observed domain — distinct from the constraining `enum`.
    if let Some(ed) = detect_enum_domain(col.label, col.values, &EnumConfig::default()) {
        field.insert(
            "x-finetype-enum-domain".into(),
            json!({
                "open": ed.open,
                "distinct": ed.distinct,
                "rows": ed.rows,
                "cohesion": (ed.cohesion * 1000.0).round() / 1000.0,
                "domain": ed.domain,
            }),
        );
    }

    Value::Object(field)
}

/// A field's `constraints`, split by whether Data Package v2 has a place for
/// each keyword on the field's **declared type**.
struct FieldConstraints {
    /// Goes into `constraints`.
    kept: Map<String, Value>,
    /// Goes into `x-finetype-unsupported-constraints`.
    off_type: Map<String, Value>,
}

/// Frictionless `constraints` for a field: `pattern`/`minLength`/`maxLength`/
/// `minimum`/`maximum` from the type's static validation, plus `enum` ONLY when
/// the column is a closed categorical (observed, enum-keyword-eligible).
///
/// Every keyword is then routed by the declared type's v2 vocabulary
/// ([`finetype_core::frictionless_vocabulary`]). The field object in v2 is a
/// fifteen-branch `oneOf`, one branch per type, each with its own `constraints`
/// properties — so `pattern` beside `"type": "integer"` is not surplus
/// information a reader skips, it is a descriptor `frictionless==5.19.0`
/// refuses outright. Keywords with no place on the declared type move to
/// `x-finetype-unsupported-constraints`; they are true of the column and the
/// standard has nowhere to put them.
///
/// A declared type outside the profile's fifteen (the taxonomy's `list`, which
/// v2 specifies and neither the vendored profile nor the reference
/// implementation implements) has no vocabulary to route against, so its
/// constraints are left where they are — filtering against a vocabulary we do
/// not have would be a guess, and the field is refused for its type either way.
///
/// `fit` is the reconciliation of the type's canonical `pattern` with this
/// column's observed values; it decides whether a `pattern` is published, and
/// which one. `None` means the type carries no pattern.
fn constraints_for(
    label: &str,
    values: &[String],
    def: Option<&finetype_core::Definition>,
    ftype: &str,
    enum_threshold: usize,
    fit: Option<&PatternFit>,
) -> FieldConstraints {
    let mut c = Map::new();

    if let Some(v) = def.and_then(|d| d.validation.as_ref()) {
        if let Some(pattern) = fit.and_then(|f| f.published()) {
            c.insert("pattern".into(), json!(pattern));
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

    // Closed-categorical enum: same eligibility gate as the json-schema emitter,
    // but only on `string` fields — an observed-value enum (raw strings) is only
    // type-consistent there; on a boolean field the profile's field `oneOf`
    // rightly rejects a string enum.
    if ftype == "string"
        && enum_threshold > 0
        && !values.is_empty()
        && label_is_enum_keyword_eligible(label)
    {
        let unique: BTreeSet<&str> = values.iter().map(|s| s.as_str()).collect();
        if unique.len() <= enum_threshold {
            let mut vals: Vec<&str> = unique.into_iter().collect();
            vals.sort();
            c.insert("enum".into(), json!(vals));
        }
    }

    let Some(vocabulary) = constraint_vocabulary(ftype) else {
        return FieldConstraints {
            kept: c,
            off_type: Map::new(),
        };
    };

    let mut kept = Map::new();
    let mut off_type = Map::new();
    for (keyword, value) in c {
        if vocabulary.contains(&keyword.as_str()) {
            kept.insert(keyword, value);
        } else {
            off_type.insert(keyword, value);
        }
    }
    FieldConstraints { kept, off_type }
}

#[cfg(test)]
mod tests {
    use super::*;
    use finetype_core::Taxonomy;

    fn test_taxonomy() -> Taxonomy {
        // A string/email with validation + pii, and a bare date/pattern.
        Taxonomy::from_yaml(
            r#"
identity.person.email:
  broad_type: VARCHAR
  pii: true
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
geography.location.country_code:
  broad_type: VARCHAR
  frictionless:
    type: string
  validation:
    type: string
    pattern: "^[A-Z]{2}$"
datetime.date.iso:
  broad_type: DATE
  frictionless:
    type: date
    format: "%Y-%m-%d"
  validation:
    type: string
    pattern: '^\d{4}-\d{2}-\d{2}$'
    minLength: 10
    maxLength: 10
representation.numeric.integer_number:
  broad_type: BIGINT
  frictionless:
    type: integer
  validation:
    type: string
    pattern: '^-?\d+$'
    minimum: 0
"#,
        )
        .expect("test taxonomy parses")
    }

    /// One column, emitted, with the values it was profiled from.
    fn field_for(label: &str, values: &[&str]) -> Value {
        let owned: Vec<String> = values.iter().map(|s| s.to_string()).collect();
        let cols = vec![DatapackageColumn {
            name: "col",
            label,
            values: &owned,
            confidence: Some(0.9),
            locale: None,
        }];
        let dp = emit_datapackage(&cols, &meta(), &test_taxonomy(), 32);
        dp["resources"][0]["schema"]["fields"][0].clone()
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
    fn emits_core_shape_and_custom_properties() {
        let tax = test_taxonomy();
        let cols = vec![
            DatapackageColumn {
                name: "email",
                label: "identity.person.email",
                values: &[],
                confidence: Some(0.875), // exactly representable → stable rounding
                locale: None,
            },
            DatapackageColumn {
                name: "d",
                label: "datetime.date.dmy_slash",
                values: &[],
                confidence: Some(0.5),
                locale: Some("UNIVERSAL"), // sentinel → omitted
            },
            DatapackageColumn {
                name: "mystery",
                label: "unknown",
                values: &[],
                confidence: None,
                locale: None,
            },
        ];
        let dp = emit_datapackage(&cols, &meta(), &tax, 32);

        assert_eq!(dp["$schema"], json!(DATAPACKAGE_PROFILE));
        let res = &dp["resources"][0];
        assert_eq!(res["format"], json!("csv"));
        assert_eq!(res["hash"], json!("sha256:deadbeef"));
        let fields = res["schema"]["fields"].as_array().unwrap();
        assert_eq!(fields.len(), 3);

        // email: ac-02 core + ac-03 props (label, confidence rounded, pii).
        assert_eq!(fields[0]["type"], json!("string"));
        assert_eq!(fields[0]["format"], json!("email"));
        assert_eq!(fields[0]["constraints"]["minLength"], json!(5));
        assert_eq!(
            fields[0]["x-finetype-label"],
            json!("identity.person.email")
        );
        assert_eq!(fields[0]["x-finetype-confidence"], json!(0.875));
        assert_eq!(fields[0]["x-finetype-pii"], json!(true));

        // date: no validation → no constraints; non-pii → no x-finetype-pii;
        // UNIVERSAL locale sentinel omitted.
        assert_eq!(fields[1]["type"], json!("date"));
        assert_eq!(fields[1]["format"], json!("%d/%m/%Y"));
        assert!(fields[1].get("constraints").is_none());
        assert!(fields[1].get("x-finetype-pii").is_none());
        assert!(fields[1].get("x-finetype-locale").is_none());

        // unknown: string, no format/constraints, but label still carried.
        assert_eq!(fields[2]["type"], json!("string"));
        assert!(fields[2].get("format").is_none());
        assert_eq!(fields[2]["x-finetype-label"], json!("unknown"));
        assert!(fields[2].get("x-finetype-confidence").is_none());
    }

    #[test]
    fn for_path_emits_relative_basename() {
        // v2.0 rejects an absolute resource.path (also ../, ~, scheme). The
        // emitter must reduce an absolute source path to the descriptor-relative
        // basename. (Found downstream: dovetail emitted the absolute path and
        // every fixture failed 2.0 conformance.)
        let m = ResourceMeta::for_path(std::path::Path::new("/abs/dir/data.csv"), b"x");
        assert_eq!(m.path, "data.csv");
        assert!(!m.path.starts_with('/'));
        assert!(!m.path.contains(".."));
        assert!(!m.path.contains(':'));
        assert_eq!(m.name, "data");
        assert_eq!(m.format, "csv");
    }

    #[test]
    fn omits_encoding_when_none() {
        let tax = test_taxonomy();
        let mut m = meta();
        m.encoding = None;
        let cols = vec![DatapackageColumn {
            name: "email",
            label: "identity.person.email",
            values: &[],
            confidence: None,
            locale: None,
        }];
        let dp = emit_datapackage(&cols, &m, &tax, 32);
        assert!(dp["resources"][0].get("encoding").is_none());
    }

    #[test]
    fn a_column_matching_its_type_publishes_the_canonical_pattern() {
        let field = field_for("geography.location.country_code", &["US", "FR", "JP"]);
        assert_eq!(field["constraints"]["pattern"], json!("^[A-Z]{2}$"));
        assert!(field.get("x-finetype-pattern-fit").is_none());
    }

    #[test]
    fn a_column_with_a_few_extra_shapes_publishes_a_widened_pattern() {
        // The label is right — this is a country-code column — and a minority of
        // its values are subdivision codes. The emitted rule admits both.
        let field = field_for(
            "geography.location.country_code",
            &["US", "FR", "US-DE", "US-CA", "CA-ON"],
        );
        assert_eq!(
            field["constraints"]["pattern"],
            json!("^[A-Z]{2}$|^[A-Z]{2}-[A-Z]{2}$")
        );
        assert_eq!(
            field["x-finetype-pattern-fit"],
            json!({ "outcome": "widened", "canonical": "^[A-Z]{2}$" })
        );
        // The label is untouched: widening a constraint is not re-typing.
        assert_eq!(
            field["x-finetype-label"],
            json!("geography.location.country_code")
        );
    }

    #[test]
    fn a_column_too_varied_to_enumerate_publishes_no_pattern() {
        let field = field_for(
            "geography.location.country_code",
            &[
                "United States",
                "France",
                "Japan",
                "The Netherlands",
                "Cote d'Ivoire",
            ],
        );
        assert!(
            field
                .get("constraints")
                .and_then(|c| c.get("pattern"))
                .is_none(),
            "expected no pattern, got {}",
            field["constraints"]
        );
        assert_eq!(
            field["x-finetype-pattern-fit"],
            json!({ "outcome": "omitted", "canonical": "^[A-Z]{2}$" })
        );
    }

    #[test]
    fn stripping_x_keys_leaves_valid_core() {
        // ac-03 invariant: a descriptor with all x- keys removed is still the
        // standard core (type present on every field).
        let tax = test_taxonomy();
        let cols = vec![DatapackageColumn {
            name: "email",
            label: "identity.person.email",
            values: &[],
            confidence: Some(0.9),
            locale: None,
        }];
        let dp = emit_datapackage(&cols, &meta(), &tax, 32);
        let field = &dp["resources"][0]["schema"]["fields"][0];
        let obj = field.as_object().unwrap();
        let core: Vec<_> = obj.keys().filter(|k| !k.starts_with("x-")).collect();
        assert!(core.contains(&&"name".to_string()));
        assert!(core.contains(&&"type".to_string()));
    }

    #[test]
    fn a_date_field_carries_its_off_type_constraints_beside_it() {
        // `datetime.date.iso` states a pattern and a fixed length, and all three
        // are true of the column. v2's `date` branch has a place for none of
        // them, and `frictionless==5.19.0` refuses the package over each one
        // ("constraint \"maxLength\" is not supported by type \"date\"").
        let field = field_for("datetime.date.iso", &["2026-08-28", "1999-01-01"]);
        assert_eq!(field["type"], json!("date"));
        assert!(
            field.get("constraints").is_none(),
            "nothing in this label's validation is legal on a v2 `date`, so the \
             field should carry no `constraints` at all: {field}"
        );

        // Filtered, not dropped: every keyword is readable beside the field.
        let off = &field["x-finetype-unsupported-constraints"];
        assert_eq!(off["pattern"], json!(r"^\d{4}-\d{2}-\d{2}$"));
        assert_eq!(off["minLength"], json!(10));
        assert_eq!(off["maxLength"], json!(10));
        assert_eq!(
            off.as_object().map(|o| o.len()),
            Some(3),
            "exactly the three keywords the type has no place for"
        );
    }

    #[test]
    fn an_integer_field_keeps_minimum_and_sheds_pattern() {
        // The routing is per keyword, not per field: `minimum` IS in v2's
        // `integer` vocabulary and stays; `pattern` is not and moves.
        let field = field_for("representation.numeric.integer_number", &["42", "7"]);
        assert_eq!(field["type"], json!("integer"));
        assert_eq!(field["constraints"]["minimum"], json!(0.0));
        assert!(
            field["constraints"].get("pattern").is_none(),
            "a v2 `integer` field may not carry `pattern`: {field}"
        );
        assert_eq!(
            field["x-finetype-unsupported-constraints"]["pattern"],
            json!(r"^-?\d+$")
        );
    }

    #[test]
    fn a_string_field_is_untouched_by_the_routing() {
        // The vocabulary is per type, so the type that owns `pattern`,
        // `minLength` and `maxLength` keeps all three in `constraints` and gains
        // no custom key.
        let field = field_for("identity.person.email", &["a@b.com"]);
        assert_eq!(field["type"], json!("string"));
        assert_eq!(field["constraints"]["pattern"], json!("^.+@.+$"));
        assert_eq!(field["constraints"]["minLength"], json!(5));
        assert_eq!(field["constraints"]["maxLength"], json!(254));
        assert!(field.get("x-finetype-unsupported-constraints").is_none());
    }

    #[test]
    fn the_routing_loses_no_keyword() {
        // The two halves partition: whatever the emitter computed is in exactly
        // one of `constraints` and `x-finetype-unsupported-constraints`.
        for (label, values) in [
            ("datetime.date.iso", &["2026-08-28"][..]),
            ("representation.numeric.integer_number", &["42"][..]),
            ("identity.person.email", &["a@b.com"][..]),
        ] {
            let field = field_for(label, values);
            let kept: BTreeSet<String> = field
                .get("constraints")
                .and_then(|c| c.as_object())
                .map(|o| o.keys().cloned().collect())
                .unwrap_or_default();
            let off: BTreeSet<String> = field
                .get("x-finetype-unsupported-constraints")
                .and_then(|c| c.as_object())
                .map(|o| o.keys().cloned().collect())
                .unwrap_or_default();
            assert!(
                kept.is_disjoint(&off),
                "{label}: a keyword appears on both sides"
            );

            let def = test_taxonomy()
                .get(label)
                .cloned()
                .expect("label in fixture");
            let v = def
                .validation
                .as_ref()
                .expect("fixture labels all validate");
            let mut expected: BTreeSet<String> = BTreeSet::new();
            if v.pattern.is_some() {
                expected.insert("pattern".into());
            }
            if v.min_length.is_some() {
                expected.insert("minLength".into());
            }
            if v.max_length.is_some() {
                expected.insert("maxLength".into());
            }
            if v.minimum.is_some() {
                expected.insert("minimum".into());
            }
            if v.maximum.is_some() {
                expected.insert("maximum".into());
            }
            let seen: BTreeSet<String> = kept.union(&off).cloned().collect();
            assert_eq!(
                seen, expected,
                "{label}: the routing must move every keyword, not lose one"
            );
        }
    }
}
