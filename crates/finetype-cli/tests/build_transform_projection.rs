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

#[test]
fn epoch_nanoseconds_transform_does_not_bind_error_on_varchar() {
    // Regression: epoch_nanoseconds columns arrive as VARCHAR. The old
    // transform `epoch_ns({col})` has no DuckDB binder overload for VARCHAR
    // (epoch_ns expects TIMESTAMP/TIME/DATE), so the whole validate run
    // aborted with a Binder Error — TRY(...) cannot catch a *bind*-time
    // failure. The fix converts the 19-digit text via a BIGINT cast first
    // (make_timestamp_ns({col}::BIGINT)), so the only argument fed to a
    // typed function is the cast, and the projection binds cleanly.
    let tax = test_taxonomy();
    let label = "datetime.timestamp.epoch_nanoseconds";
    let info = tax
        .ddl_info(label)
        .expect("epoch_nanoseconds is a known label");
    let transform = info
        .transform
        .as_ref()
        .expect("epoch_nanoseconds has a transform");

    // Must NOT call epoch_ns(...) directly on the (VARCHAR) column.
    assert!(
        !transform.contains("epoch_ns({col})"),
        "epoch_nanoseconds must not feed a bare VARCHAR column to epoch_ns \
         (no binder overload → aborts validate); got: {}",
        transform
    );
    // Must cast the textual column to BIGINT before any timestamp function,
    // matching the sibling unix_seconds/milliseconds/microseconds shape.
    assert!(
        transform.contains("{col}::BIGINT"),
        "expected a `{{col}}::BIGINT` cast so the VARCHAR storage binds, got: {}",
        transform
    );

    // The rendered projection is TRY-wrapped and aliased like every other
    // typed transform — no special-casing in the codegen.
    let ext = ext_with_label("epoch_nanoseconds", Some(label));
    let headers = vec!["epoch_nanoseconds".to_string()];
    let proj = build_transform_projection(&headers, &ext, &tax, true);
    assert!(
        proj.starts_with("TRY(") && proj.ends_with("AS \"epoch_nanoseconds\""),
        "expected TRY(...) AS alias, got: {}",
        proj
    );
}

#[test]
#[ignore = "needs the duckdb CLI; CI runs it with --include-ignored"]
fn percentage_transform_extracts_the_number_from_every_accepted_form() {
    // Runs the published transform through the external `duckdb` CLI.
    use std::process::Command;
    let tax = test_taxonomy();
    let label = "representation.numeric.percentage";
    let transform = tax
        .ddl_info(label)
        .expect("percentage is a known label")
        .transform
        .clone()
        .expect("percentage has a transform");
    let cases: &[(&str, f64)] = &[
        ("50%", 50.0),
        ("50 %", 50.0),
        ("50\u{00A0}%", 50.0),
        ("50\u{202F}%", 50.0),
        ("%50", 50.0),
        ("-12.5%", -12.5),
        ("12.5", 12.5),
        ("%-12.5", -12.5),
        ("-12.5 %", -12.5),
    ];
    for (value, expected) in cases {
        let sql = format!(
            "SELECT {}",
            transform.replace("{col}", &format!("'{}'", value))
        );
        let out = Command::new("duckdb")
            .args(["-no-init", "-noheader", "-list", "-c", &sql])
            .output()
            .expect("run the duckdb CLI");
        assert!(
            out.status.success(),
            "duckdb failed on {value:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let got: f64 = String::from_utf8_lossy(&out.stdout)
            .trim()
            .parse()
            .unwrap_or_else(|_| panic!("non-numeric result for {value:?}"));
        assert_eq!(got, *expected, "{value:?}");
    }

    // A value the rule rejects extracts nothing (TRY-wrapped, as validate does).
    let sql = format!(
        "SELECT TRY({}) IS NULL",
        transform.replace("{col}", "'abc 12'")
    );
    let out = Command::new("duckdb")
        .args(["-no-init", "-noheader", "-list", "-c", &sql])
        .output()
        .expect("run the duckdb CLI");
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "true");
}

#[test]
#[ignore = "needs the duckdb CLI; CI runs it with --include-ignored"]
fn file_size_transform_gives_bytes_for_every_unit_family() {
    // Runs the published transform through the external `duckdb` CLI.
    use std::process::Command;
    let tax = test_taxonomy();
    let transform = tax
        .ddl_info("representation.file.file_size")
        .expect("file_size is a known label")
        .transform
        .clone()
        .expect("file_size has a transform");
    let cases: &[(&str, f64)] = &[
        ("512", 512.0),
        ("100 B", 100.0),
        ("2 TB", 2e12),
        ("1.5 kB", 1500.0),
        ("1 MB", 1e6),
        ("3 GB", 3e9),
        ("1 PB", 1e15),
        ("4KiB", 4096.0),
        ("3 GiB", 3.0 * 1073741824.0),
        ("2 TiB", 2.0 * 1099511627776.0),
        ("10M", 10.0 * 1048576.0),
        ("2T", 2.0 * 1099511627776.0),
        ("1P", 1125899906842624.0),
    ];
    for (value, expected) in cases {
        let sql = format!(
            "SELECT {}",
            transform.replace("{col}", &format!("'{}'", value))
        );
        let out = Command::new("duckdb")
            .args(["-no-init", "-noheader", "-list", "-c", &sql])
            .output()
            .expect("run the duckdb CLI");
        assert!(
            out.status.success(),
            "duckdb failed on {value:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let got: f64 = String::from_utf8_lossy(&out.stdout)
            .trim()
            .parse()
            .unwrap_or_else(|_| panic!("non-numeric result for {value:?}"));
        assert_eq!(got, *expected, "{value:?}");
    }
}

#[test]
#[ignore = "needs the duckdb CLI; CI runs it with --include-ignored"]
fn locale_code_decompose_extracts_language_and_region() {
    // Runs the published decompose expressions through the external `duckdb` CLI.
    use std::process::Command;
    let tax = test_taxonomy();
    let decompose = tax
        .ddl_info("technology.code.locale_code")
        .expect("locale_code is a known label")
        .decompose
        .clone()
        .expect("locale_code has decompose");
    let expr = |part: &str| -> String {
        decompose[part]
            .as_str()
            .unwrap_or_else(|| panic!("decompose part {part}"))
            .to_string()
    };
    let run = |part: &str, value: &str| -> String {
        let sql = format!(
            "SELECT {}",
            expr(part).replace("{col}", &format!("'{}'", value))
        );
        let out = Command::new("duckdb")
            .args(["-no-init", "-noheader", "-list", "-c", &sql])
            .output()
            .expect("run the duckdb CLI");
        assert!(
            out.status.success(),
            "duckdb failed on {value:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    };
    let cases: &[(&str, &str, &str)] = &[
        ("en-US", "en", "us"),
        ("pt_BR", "pt", "br"),
        ("es-419", "es", "419"),
        ("zh-Hant-TW", "zh", "tw"),
        ("zh-cmn-Hans-CN", "zh", "cn"),
        ("zh-Hant", "zh", ""),
        ("en", "en", ""),
    ];
    for (value, language, country) in cases {
        assert_eq!(run("language", value), *language, "language of {value:?}");
        assert_eq!(run("country", value), *country, "country of {value:?}");
    }
}

#[test]
#[ignore = "needs the duckdb CLI; CI runs it with --include-ignored"]
fn abbreviated_month_transform_casts_every_accepted_form() {
    // Runs the published transform through the external `duckdb` CLI.
    use std::process::Command;
    let tax = test_taxonomy();
    let label = "datetime.date.abbreviated_month";
    let transform = tax
        .ddl_info(label)
        .expect("abbreviated_month is a known label")
        .transform
        .clone()
        .expect("abbreviated_month has a transform");
    let cases: &[(&str, &str)] = &[
        ("Feb 4, 2023", "2023-02-04"),
        ("Feb. 4, 2023", "2023-02-04"),
        ("4 Feb 2023", "2023-02-04"),
        ("4 Feb. 2023", "2023-02-04"),
        ("4 Feb 23", "2023-02-04"),
        ("4 Feb. 23", "2023-02-04"),
        ("Sept 4, 2023", "2023-09-04"),
        ("Sept. 4, 2023", "2023-09-04"),
        ("4 Sept 2023", "2023-09-04"),
        ("4 Sept 23", "2023-09-04"),
        ("Sep 4, 2023", "2023-09-04"),
    ];
    for (value, expected) in cases {
        let sql = format!(
            "SELECT {}",
            transform.replace("{col}", &format!("'{}'", value))
        );
        let out = Command::new("duckdb")
            .args(["-no-init", "-noheader", "-list", "-c", &sql])
            .output()
            .expect("run the duckdb CLI");
        assert!(
            out.status.success(),
            "duckdb failed on {value:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&out.stdout).trim(),
            *expected,
            "{value:?}"
        );
    }

    // A French value stays un-castable by the English-only strptime.
    let sql = format!(
        "SELECT TRY({}) IS NULL",
        transform.replace("{col}", "'4 janv. 2023'")
    );
    let out = Command::new("duckdb")
        .args(["-no-init", "-noheader", "-list", "-c", &sql])
        .output()
        .expect("run the duckdb CLI");
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "true");
}

#[test]
#[ignore = "needs the duckdb CLI; CI runs it with --include-ignored"]
fn offset_timestamp_transforms_parse_every_accepted_form() {
    // Runs the published transforms through the external `duckdb` CLI, in UTC.
    use std::process::Command;
    let tax = test_taxonomy();
    let rfc_3339: &[(&str, &str)] = &[
        ("2024-01-01 10:00:00Z", "2024-01-01 10:00:00+00"),
        ("2024-01-01 10:00:00z", "2024-01-01 10:00:00+00"),
        (
            "2024-01-01 10:00:00.123+05:30",
            "2024-01-01 04:30:00.123+00",
        ),
        ("2024-01-01 10:00:00+05:30", "2024-01-01 04:30:00+00"),
        ("2024-01-01 10:00:00+0530", "2024-01-01 04:30:00+00"),
        ("2024-01-01 10:00:00 +0000", "2024-01-01 10:00:00+00"),
        ("2024-01-01 10:00:00 -05:00", "2024-01-01 15:00:00+00"),
        ("2024-01-01 10:00:00 Z", "2024-01-01 10:00:00+00"),
        ("2024-01-01 10:00:00 z", "2024-01-01 10:00:00+00"),
    ];
    let iso_8601_offset: &[(&str, &str)] = &[
        ("2024-01-01T10:00:00+05:30", "2024-01-01 04:30:00+00"),
        ("2024-01-01T10:00:00+0530", "2024-01-01 04:30:00+00"),
        ("2024-01-01T10:00:00+05", "2024-01-01 05:00:00+00"),
        ("2024-01-01t10:00:00+05:30", "2024-01-01 04:30:00+00"),
        ("2024-01-01T10:00:00+5:30", "2024-01-01 04:30:00+00"),
        ("2024-01-01T10:00:00+0:53", "2024-01-01 09:07:00+00"),
        ("2024-01-01T10:00:00-08:00", "2024-01-01 18:00:00+00"),
    ];
    for (label, cases) in [
        ("datetime.timestamp.rfc_3339", rfc_3339),
        ("datetime.timestamp.iso_8601_offset", iso_8601_offset),
    ] {
        let transform = tax
            .ddl_info(label)
            .unwrap_or_else(|| panic!("{label} is a known label"))
            .transform
            .clone()
            .unwrap_or_else(|| panic!("{label} has a transform"));
        for (value, expected) in cases {
            let sql = format!(
                "SET TimeZone='UTC'; SELECT {}",
                transform.replace("{col}", &format!("'{}'", value))
            );
            let out = Command::new("duckdb")
                .args(["-no-init", "-noheader", "-list", "-c", &sql])
                .output()
                .expect("run the duckdb CLI");
            assert!(
                out.status.success(),
                "duckdb failed on {label} {value:?}: {}",
                String::from_utf8_lossy(&out.stderr)
            );
            assert_eq!(
                String::from_utf8_lossy(&out.stdout).trim(),
                *expected,
                "{label} {value:?}"
            );
        }
    }
}

#[test]
#[ignore = "needs the duckdb CLI; CI runs it with --include-ignored"]
fn blood_type_decompose_normalises_the_minus_sign() {
    // Runs the published decompose expressions through the external `duckdb` CLI.
    use std::process::Command;
    let tax = test_taxonomy();
    let decompose = tax
        .ddl_info("identity.person.blood_type")
        .expect("blood_type is a known label")
        .decompose
        .clone()
        .expect("blood_type has decompose");
    let run = |part: &str, value: &str| -> String {
        let expr = decompose[part].as_str().expect("decompose part");
        let sql = format!("SELECT {}", expr.replace("{col}", &format!("'{}'", value)));
        let out = Command::new("duckdb")
            .args(["-no-init", "-noheader", "-list", "-c", &sql])
            .output()
            .expect("run the duckdb CLI");
        assert!(
            out.status.success(),
            "duckdb failed on {value:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    };
    for (value, group, rh) in [
        ("AB\u{2212}", "AB", "-"),
        ("O-", "O", "-"),
        ("a+", "A", "+"),
        ("AB", "AB", ""),
    ] {
        assert_eq!(run("blood_group", value), group, "group of {value:?}");
        assert_eq!(run("rh_factor", value), rh, "rh of {value:?}");
    }
}

#[test]
#[ignore = "needs the duckdb CLI; CI runs it with --include-ignored"]
fn protein_sequence_decompose_on_ambiguity_and_gap_forms() {
    // Runs the published decompose expressions through the external `duckdb` CLI.
    use std::process::Command;
    let tax = test_taxonomy();
    let decompose = tax
        .ddl_info("representation.scientific.protein_sequence")
        .expect("protein_sequence is a known label")
        .decompose
        .clone()
        .expect("protein_sequence has decompose");
    let run = |part: &str, value: &str| -> String {
        let expr = decompose[part].as_str().expect("decompose part");
        let sql = format!("SELECT {}", expr.replace("{col}", &format!("'{}'", value)));
        let out = Command::new("duckdb")
            .args(["-no-init", "-noheader", "-list", "-c", &sql])
            .output()
            .expect("run the duckdb CLI");
        assert!(
            out.status.success(),
            "duckdb failed on {value:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    };
    // Gaps and ambiguity codes count as characters (the notes say so).
    for (value, length, weight) in [("MKVBZULLIO", "10", "1100"), ("MKV-LLIVGS", "10", "1100")] {
        assert_eq!(run("length", value), length, "length of {value:?}");
        assert_eq!(
            run("molecular_weight_estimate", value),
            weight,
            "weight of {value:?}"
        );
    }
}

#[test]
#[ignore = "needs the duckdb CLI; CI runs it with --include-ignored"]
fn coordinates_transform_and_parts_read_every_accepted_form() {
    // Runs the published transform and decompose parts through the external `duckdb` CLI.
    use std::process::Command;
    let tax = test_taxonomy();
    let label = "geography.coordinate.coordinates";
    let info = tax.ddl_info(label).expect("coordinates is a known label");
    let transform = info.transform.clone().expect("coordinates has a transform");
    let decompose = info
        .decompose
        .clone()
        .expect("coordinates has decompose parts");
    let cases: &[(&str, f64, f64)] = &[
        ("48.8761, 2.3681", 48.8761, 2.3681),
        (
            "48.8761373390584, 2.3680844979417",
            48.8761373390584,
            2.3680844979417,
        ),
        ("40.7128,-74.0060", 40.7128, -74.006),
        ("35.6762 139.6917", 35.6762, 139.6917),
        ("35.6762  139.6917", 35.6762, 139.6917),
        ("35.6762\t139.6917", 35.6762, 139.6917),
        ("-33.8688 , 151.2093", -33.8688, 151.2093),
        ("5,10", 5.0, 10.0),
    ];
    let run = |sql: &str, what: &str| -> String {
        let out = Command::new("duckdb")
            .args(["-no-init", "-noheader", "-list", "-c", sql])
            .output()
            .expect("run the duckdb CLI");
        assert!(
            out.status.success(),
            "duckdb failed on {what}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    };
    for (value, lat, lon) in cases {
        let quoted = format!("'{}'", value);
        // The transform returns a struct; read each field as a number.
        for (field, expected) in [("lat", lat), ("lon", lon)] {
            let sql = format!("SELECT ({}).{field}", transform.replace("{col}", &quoted));
            let got: f64 = run(&sql, &format!("transform {value:?}"))
                .parse()
                .unwrap_or_else(|_| panic!("non-numeric {field} for {value:?}"));
            assert_eq!(got, *expected, "transform {field} of {value:?}");
        }
        for (part, expected) in [("latitude", lat), ("longitude", lon)] {
            let expr = decompose[part].as_str().expect("decompose part");
            let sql = format!("SELECT {}", expr.replace("{col}", &quoted));
            let got: f64 = run(&sql, &format!("{part} of {value:?}"))
                .parse()
                .unwrap_or_else(|_| panic!("non-numeric {part} for {value:?}"));
            assert_eq!(got, *expected, "{part} of {value:?}");
        }
    }
}

#[test]
#[ignore = "needs the duckdb CLI and the spatial extension; CI runs it with --include-ignored"]
fn coordinates_transform_ext_puts_longitude_first() {
    // ST_Point takes x (longitude) first: "48.8761, 2.3681" is x = 2.3681, y = 48.8761.
    use std::process::Command;
    let tax = test_taxonomy();
    let ext = tax
        .get("geography.coordinate.coordinates")
        .expect("coordinates is a known label")
        .transform_ext
        .clone()
        .expect("coordinates has a transform_ext");
    let point = ext.replace("{col}", "'48.8761, 2.3681'");
    let sql = format!("INSTALL spatial; LOAD spatial; SELECT ST_X({point}), ST_Y({point})");
    let out = Command::new("duckdb")
        .args(["-no-init", "-noheader", "-list", "-csv", "-c", &sql])
        .output()
        .expect("run the duckdb CLI");
    assert!(
        out.status.success(),
        "duckdb failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(stdout.trim().lines().last(), Some("2.3681,48.8761"));
}

/// Runs a published decompose part through the external `duckdb` CLI.
fn run_color_part(tax: &Taxonomy, label: &str, part: &str, value: &str) -> String {
    use std::process::Command;
    let decompose = tax
        .ddl_info(label)
        .expect("known label")
        .decompose
        .clone()
        .expect("label has decompose");
    let expr = decompose[part].as_str().expect("decompose part");
    let sql = format!("SELECT {}", expr.replace("{col}", &format!("'{}'", value)));
    let out = Command::new("duckdb")
        .args(["-no-init", "-noheader", "-list", "-c", &sql])
        .output()
        .expect("run the duckdb CLI");
    assert!(
        out.status.success(),
        "duckdb failed on {value:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

#[test]
#[ignore = "needs the duckdb CLI; CI runs it with --include-ignored"]
fn color_rgb_parts_read_the_modern_forms() {
    let tax = test_taxonomy();
    let rgb = "representation.format.color_rgb";
    // Space-separated, slash alpha, percentages (scaled to 0-255), bare triples.
    for (value, expect) in [
        ("rgb(255 0 0)", ["255", "0", "0"]),
        ("rgb(255 128 0 / 0.5)", ["255", "128", "0"]),
        ("rgb(100% 0% 0% / 50%)", ["255", "0", "0"]),
        ("rgb(50% 25.5% 0%)", ["128", "65", "0"]),
        ("rgba(0,0,255,.5)", ["0", "0", "255"]),
        ("255,0,0", ["255", "0", "0"]),
        ("(128, 128, 128)", ["128", "128", "128"]),
    ] {
        for (part, want) in ["red", "green", "blue"].iter().zip(expect) {
            assert_eq!(
                run_color_part(&tax, rgb, part, value),
                want,
                "{part} of {value:?}"
            );
        }
    }
}

#[test]
#[ignore = "needs the duckdb CLI; CI runs it with --include-ignored"]
fn color_hsl_parts_read_the_modern_forms() {
    let tax = test_taxonomy();
    let hsl = "representation.format.color_hsl";
    // A `none` component, which has no number, reads as NULL (the CLI prints it
    // as the text NULL with -noheader -list).
    for (value, expect) in [
        ("hsl(120deg 75% 25% / 60%)", ["120", "75", "25"]),
        ("hsl(540 0 0)", ["540", "0", "0"]),
        ("hsl(120.5, 100%, 50%)", ["121", "100", "50"]),
        ("hsl(none 100% 50%)", ["NULL", "100", "50"]),
        ("hsl(120 none none / none)", ["120", "NULL", "NULL"]),
        ("hsla(240, 100%, 50%, 0.5)", ["240", "100", "50"]),
    ] {
        for (part, want) in ["hue", "saturation", "lightness"].iter().zip(expect) {
            assert_eq!(
                run_color_part(&tax, hsl, part, value),
                want,
                "{part} of {value:?}"
            );
        }
    }
}
