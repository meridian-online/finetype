//! Per-column SQL projection used by `finetype validate --db --table` to
//! materialise typed columns from the staging table into the user's
//! destination table.
//!
//! The single public entry point is [`build_transform_projection`]. The
//! 5-branch decision tree it implements is documented on the function. The
//! helpers `format_column_name` and `SchemaExtensions` are public so the
//! validate-materialise path in `main.rs` can share them with this module
//! without re-deriving identifier-quoting or `x-finetype-*` extraction
//! semantics.
//!
//! This file was lifted out of `main.rs` in v0.6.19 (MADR 0071, ac-05) so
//! the projection helper and its unit tests can live in a 2-file pair —
//! source here, tests in `tests/build_transform_projection.rs` — keeping
//! `main.rs` to dispatch + command bodies.

use finetype_core::Taxonomy;

/// Per-column extensions carried from schema authoring time (ac-11).
///
/// Populated from `x-finetype-label` and `x-finetype-confidence` on each
/// column-level schema. Missing entries are allowed — the corresponding
/// reject columns render as NULL (graceful degradation).
#[derive(Default)]
pub struct SchemaExtensions {
    pub by_column: std::collections::HashMap<String, (Option<String>, Option<f64>)>,
}

impl SchemaExtensions {
    pub fn extract(schema: &serde_json::Value) -> Self {
        let mut ext = SchemaExtensions::default();
        if let Some(props) = schema.get("properties").and_then(|p| p.as_object()) {
            for (col_name, col_schema) in props {
                let label = col_schema
                    .get("x-finetype-label")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let confidence = col_schema
                    .get("x-finetype-confidence")
                    .and_then(|v| v.as_f64());
                ext.by_column.insert(col_name.clone(), (label, confidence));
            }
        }
        ext
    }

    pub fn get(&self, col_name: &str) -> (Option<String>, Option<f64>) {
        self.by_column
            .get(col_name)
            .cloned()
            .unwrap_or((None, None))
    }
}

/// Format a column name for SQL.
///
/// Always quotes with double-quotes for safety — this prevents breakage from
/// DuckDB reserved words (name, type, source, etc.), spaces, special chars,
/// and digit-leading names. Standard SQL-compliant.
pub fn format_column_name(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

/// Build the per-column projection list for the validate-materialise CTAS.
///
/// Lifted from cmd_load's per-column expression builder (deleted in v0.6.19)
/// and called by the validate materialise CTAS. The 5-branch decision tree is:
///
/// 1. **Unlabelled** (`x-finetype-label` absent on the column schema) → emit a
///    bare quoted identifier. Preserves MADR 0064 ac-11 graceful-degradation —
///    columns without a label fall through as VARCHAR.
/// 2. **Labelled but unknown to taxonomy** (label present but
///    `Taxonomy::ddl_info()` returns `None`) → bare quoted identifier. Same
///    graceful-degradation contract as branch 1 — an unknown label cannot drive
///    a typed cast.
/// 3. **VARCHAR-typed** (`ddl_info.duckdb_type == "VARCHAR"`) → bare quoted
///    identifier. Mirrors `build_load_expr` branch 1 — no redundant CAST for
///    VARCHAR.
/// 4. **Has transform** → `<transform> AS "col"`. When `try_wrap` is true the
///    full transform expression is wrapped in `TRY(...)` so DuckDB returns NULL
///    on cast failures instead of aborting the CTAS. The transform string is
///    expected to contain `{col}`, which is substituted with the quoted
///    identifier.
/// 5. **No transform, non-VARCHAR** → fallback `CAST("col" AS T) AS "col"` (or
///    `TRY_CAST` when `try_wrap` is true). Mirrors `build_load_expr` branch 3.
///
/// `try_wrap=true` is the validate path's binding-choice contract (constraint
/// in spec): every typed transform is wrapped in `TRY(...)` so the CTAS sees a
/// NULL on transform failure rather than aborting. The pre-CTAS sweep then
/// detects `staging IS NOT NULL AND TRY(transform) IS NULL` and emits a
/// `TRANSFORM_FAILED` reject row, removing the `__row_idx` from the valid set
/// before the user-table CTAS runs (ac-03 + ac-04).
pub fn build_transform_projection(
    headers: &[String],
    extensions: &SchemaExtensions,
    taxonomy: &Taxonomy,
    try_wrap: bool,
) -> String {
    headers
        .iter()
        .map(|h| build_transform_projection_one(h, extensions, taxonomy, try_wrap))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Single-column branch for `build_transform_projection`. Pulled out so the
/// 5-branch logic can be unit-tested independently of `headers` slicing.
pub fn build_transform_projection_one(
    header: &str,
    extensions: &SchemaExtensions,
    taxonomy: &Taxonomy,
    try_wrap: bool,
) -> String {
    let col_ref = format_column_name(header);
    let (label_opt, _confidence) = extensions.get(header);

    // Branch 1 — unlabelled column (no x-finetype-label). Bare passthrough.
    let label = match label_opt {
        Some(s) => s,
        None => return col_ref,
    };

    // Branch 2 — labelled but unknown to the taxonomy. Bare passthrough.
    let info = match taxonomy.ddl_info(&label) {
        Some(i) => i,
        None => return col_ref,
    };

    // Branch 3 — VARCHAR-typed. Bare passthrough.
    if info.duckdb_type == "VARCHAR" {
        return col_ref;
    }

    // Branch 4 — has transform. Substitute {col}, optionally TRY-wrap.
    if let Some(tf) = info.transform.as_ref() {
        let cast_expr = tf.replace("{col}", &col_ref);
        if try_wrap {
            return format!("TRY({}) AS {}", cast_expr, col_ref);
        }
        return format!("{} AS {}", cast_expr, col_ref);
    }

    // Branch 5 — no transform, non-VARCHAR. CAST or TRY_CAST.
    if try_wrap {
        format!(
            "TRY_CAST({} AS {}) AS {}",
            col_ref, info.duckdb_type, col_ref
        )
    } else {
        format!("CAST({} AS {}) AS {}", col_ref, info.duckdb_type, col_ref)
    }
}
