//! Unit tests for `build_transform_projection` — the per-column SQL projection
//! used by `finetype validate --db --table` to materialise typed columns from
//! the staging table into the user's destination table.
//!
//! Lifted out of `crates/finetype-cli/src/main.rs`'s `tests` module in v0.6.19
//! (MADR 0071, ac-05) so the unit tests live next to the helper they exercise
//! without bloating `main.rs`. The function under test lives in
//! `crates/finetype-cli/src/transform_projection.rs`.
//!
//! Each branch of the 5-branch decision tree is pinned by its own test; the
//! `*_no_try_wrap_matches_legacy_load_shape` test additionally pins the
//! equivalence with the retired `cmd_load` per-column expression shape.

use finetype_cli::transform_projection::{
    build_transform_projection, format_column_name, SchemaExtensions,
};
use finetype_core::Taxonomy;

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Helper — builds a SchemaExtensions with a single label for `col`.
fn ext_with_label(col: &str, label: Option<&str>) -> SchemaExtensions {
    let mut ext = SchemaExtensions::default();
    ext.by_column
        .insert(col.to_string(), (label.map(|s| s.to_string()), None));
    ext
}

/// Helper — load taxonomy from `labels/` (or fall back to embedded).
fn test_taxonomy() -> Taxonomy {
    let path = std::path::PathBuf::from("../../labels");
    if path.is_dir() {
        return Taxonomy::from_directory(&path).expect("taxonomy load");
    }
    let path = std::path::PathBuf::from("labels");
    Taxonomy::from_directory(&path).expect("taxonomy load")
}

// ── format_column_name ───────────────────────────────────────────────────────

#[test]
fn test_format_column_name_with_embedded_quotes() {
    // Double-quotes in column names are escaped per SQL standard.
    let name = format_column_name("col\"name");
    assert_eq!(name, "\"col\"\"name\"");
}

// ── build_transform_projection (ac-01) ───────────────────────────────────────
//
// Lifted from cmd_load's per-column expression builder (deleted in v0.6.19)
// and called by the validate materialise CTAS. The 5-branch logic is
// documented on the function. Each test pins one branch.

#[test]
fn build_transform_projection_emits_try_wrap_when_requested() {
    // Branch 4 with try_wrap=true: typed column with a transform gets
    // wrapped in TRY(...). datetime.date.iso has a strptime transform.
    let tax = test_taxonomy();
    let ext = ext_with_label("order_date", Some("datetime.date.iso"));
    let headers = vec!["order_date".to_string()];
    let proj = build_transform_projection(&headers, &ext, &tax, true);
    assert!(
        proj.starts_with("TRY("),
        "expected TRY(...) wrap, got: {}",
        proj
    );
    assert!(
        proj.ends_with("AS \"order_date\""),
        "expected AS alias, got: {}",
        proj
    );
}

#[test]
fn build_transform_projection_passes_through_unlabelled_as_varchar() {
    // Branch 1: column with no x-finetype-label → bare quoted identifier.
    let tax = test_taxonomy();
    let ext = SchemaExtensions::default();
    let headers = vec!["raw_col".to_string()];
    let proj = build_transform_projection(&headers, &ext, &tax, true);
    assert_eq!(proj, "\"raw_col\"");
}

#[test]
fn build_transform_projection_passes_through_unknown_label_as_varchar() {
    // Branch 2: labelled but the label isn't in the taxonomy → bare
    // passthrough. Preserves graceful-degradation (MADR 0064 ac-11) for
    // CLI fixtures that author labels not yet (or no longer) in the
    // taxonomy — e.g. today's `identity.code.id` test fixtures.
    let tax = test_taxonomy();
    let ext = ext_with_label("user_id", Some("identity.code.id"));
    let headers = vec!["user_id".to_string()];
    let proj = build_transform_projection(&headers, &ext, &tax, true);
    assert_eq!(proj, "\"user_id\"");
}

#[test]
fn build_transform_projection_emits_bare_passthrough_for_varchar() {
    // Branch 3: labelled with a VARCHAR-typed taxonomy entry → bare
    // quoted identifier (no redundant CAST). identity.person.email is
    // VARCHAR.
    let tax = test_taxonomy();
    let ext = ext_with_label("contact_email", Some("identity.person.email"));
    let headers = vec!["contact_email".to_string()];
    let proj = build_transform_projection(&headers, &ext, &tax, true);
    assert_eq!(proj, "\"contact_email\"");
}

#[test]
fn build_transform_projection_no_try_wrap_matches_legacy_load_shape() {
    // Equivalence pin: with try_wrap=false the projection's per-column
    // shape exactly matches the legacy cmd_load expression shape that
    // was retired in v0.6.19 — bare quoted ident for VARCHAR, transform
    // substituted with `AS "col"` for typed-with-transform, bare ident
    // for unlabelled (graceful degradation per MADR 0064 ac-11).
    // Each branch is tested as a single-column projection so transform
    // expressions containing commas don't trip a naive split.
    let tax = test_taxonomy();

    // VARCHAR (identity.person.email) — bare quoted ident, no CAST.
    let ext_email = ext_with_label("email", Some("identity.person.email"));
    let proj_email = build_transform_projection(&["email".to_string()], &ext_email, &tax, false);
    assert_eq!(proj_email, "\"email\"");

    // DATE with strptime transform — `<transform> AS "col"`. The exact
    // transform string comes from the taxonomy so this stays in sync if
    // the strptime format ever shifts.
    let info = tax.ddl_info("datetime.date.iso").expect("known label");
    let ext_date = ext_with_label("order_date", Some("datetime.date.iso"));
    let proj_date = build_transform_projection(&["order_date".to_string()], &ext_date, &tax, false);
    let expected_transform = info
        .transform
        .as_ref()
        .expect("datetime.date.iso has a transform")
        .replace("{col}", "\"order_date\"");
    assert_eq!(
        proj_date,
        format!("{} AS \"order_date\"", expected_transform),
    );

    // Unlabelled — bare quoted ident.
    let ext_raw = SchemaExtensions::default();
    let proj_raw = build_transform_projection(&["raw_col".to_string()], &ext_raw, &tax, false);
    assert_eq!(proj_raw, "\"raw_col\"");
}
