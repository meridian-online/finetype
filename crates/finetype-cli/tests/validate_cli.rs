//! Integration tests for `finetype validate` — the DuckDB-native reject pipeline.
//!
//! Covers spec `.orbit/specs/2026-04-22-duckdb-extension-ergonomics/` acceptance
//! criteria ac-06, ac-08, ac-09, ac-10, ac-11, ac-12, and ac-13 (test grid).
//!
//! These tests compile and invoke the `finetype` binary. They also shell out
//! to the `duckdb` CLI to assert database state. Both binaries must be on
//! PATH; the `duckdb` dependency is already required by the production
//! `cmd_validate_table` path. Tests are `#[ignore]` and run via
//!
//!     cargo test -p finetype-cli --test validate_cli -- --ignored

use std::path::{Path, PathBuf};
use std::process::Command;

// ═══════════════════════════════════════════════════════════════════════════════
// HELPERS
// ═══════════════════════════════════════════════════════════════════════════════

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

/// Invoke `finetype validate` with the given args and return (status_code, stdout, stderr).
fn run_validate(args: &[&str]) -> (i32, String, String) {
    let mut all_args: Vec<&str> = vec!["run", "-q", "-p", "finetype-cli", "--", "validate"];
    all_args.extend_from_slice(args);
    let output = Command::new("cargo")
        .args(&all_args)
        .current_dir(workspace_root())
        .output()
        .expect("failed to spawn cargo run");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (code, stdout, stderr)
}

/// Invoke `finetype` directly (no subcommand prefix) with raw args.
/// Used to assert behaviour of the `load` subcommand removal (ac-06).
fn run_finetype_raw(args: &[&str]) -> (i32, String, String) {
    let mut all_args: Vec<&str> = vec!["run", "-q", "-p", "finetype-cli", "--"];
    all_args.extend_from_slice(args);
    let output = Command::new("cargo")
        .args(&all_args)
        .current_dir(workspace_root())
        .output()
        .expect("failed to spawn cargo run");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (code, stdout, stderr)
}

/// Query a .db file via the duckdb CLI and return trimmed stdout.
/// Panics on CLI failure — tests assert success by presence of expected values.
fn duckdb_query(db: &Path, sql: &str) -> String {
    let output = Command::new("duckdb")
        .arg("-noheader")
        .arg("-list")
        .arg(db)
        .arg("-c")
        .arg(sql)
        .output()
        .expect("failed to spawn duckdb");
    assert!(
        output.status.success(),
        "duckdb query failed: {}\nsql: {}",
        String::from_utf8_lossy(&output.stderr),
        sql
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

/// Write a minimal schema JSON file and return its path.
fn write_schema(dir: &Path, body: &str) -> PathBuf {
    let p = dir.join("schema.json");
    std::fs::write(&p, body).unwrap();
    p
}

/// Write a CSV file and return its path.
fn write_csv(dir: &Path, name: &str, body: &str) -> PathBuf {
    let p = dir.join(name);
    std::fs::write(&p, body).unwrap();
    p
}

/// Canonical 3-column schema with x-finetype extensions on order_id.
const SCHEMA_WITH_EXT: &str = r#"{
  "type": "object",
  "properties": {
    "order_id": {
      "type": "string",
      "pattern": "^ORD-[0-9]{5}$",
      "x-finetype-label": "identity.code.id",
      "x-finetype-confidence": 0.99
    },
    "status": {"type": "string", "enum": ["pending", "shipped"]}
  }
}"#;

/// Same shape, no x-finetype extensions (ac-11 graceful-NULL check).
const SCHEMA_NO_EXT: &str = r#"{
  "type": "object",
  "properties": {
    "order_id": {"type": "string", "pattern": "^ORD-[0-9]{5}$"},
    "status": {"type": "string", "enum": ["pending", "shipped"]}
  }
}"#;

const CSV_TWO_REJECTS: &str =
    "order_id,status\nORD-11111,pending\nXXX-22222,UNKNOWN\nORD-33333,shipped\n";

// ═══════════════════════════════════════════════════════════════════════════════
// ac-13 test grid — 5 CLI-side tests (vrp_ac13_*)
// ═══════════════════════════════════════════════════════════════════════════════

/// ac-06 + ac-13: The CLI creates the reject sidecar with the 13-column
/// RejectEntry schema and emits SEMANTIC_TYPE in error_type.
#[test]
#[ignore]
fn test_vrp_ac13_cli_writes_db_with_sidecar() {
    let tmp = tempfile::tempdir().unwrap();
    let schema = write_schema(tmp.path(), SCHEMA_WITH_EXT);
    let csv = write_csv(tmp.path(), "in.csv", CSV_TWO_REJECTS);
    let db = tmp.path().join("out.db");

    let (code, _stdout, stderr) = run_validate(&[
        csv.to_str().unwrap(),
        schema.to_str().unwrap(),
        "--db",
        db.to_str().unwrap(),
        "--table",
        "orders",
    ]);
    assert_eq!(
        code, 1,
        "expected exit 1 (rejects present): stderr={stderr}"
    );

    // 13-column ontology check (ac-06)
    let cols = duckdb_query(
        &db,
        "SELECT string_agg(column_name, ',' ORDER BY column_index) \
         FROM duckdb_columns WHERE table_name = 'finetype_reject_errors';",
    );
    let expected = "scan_id,file_id,line,column_idx,column_name,error_type,\
                    csv_line,byte_position,error_message,type_confidence,\
                    expected_type,constraint_failed,constraint_value";
    assert_eq!(cols, expected);

    // SEMANTIC_TYPE marker on every FineType-emitted reject.
    let distinct = duckdb_query(
        &db,
        "SELECT DISTINCT error_type FROM finetype_reject_errors;",
    );
    assert_eq!(distinct, "SEMANTIC_TYPE");

    // User table is present with valid rows only (2 valid out of 3).
    let user_rows = duckdb_query(&db, "SELECT COUNT(*) FROM orders;");
    assert_eq!(user_rows, "2");
}

/// ac-09 + ac-13: After a successful run the TEMPORARY staging table is
/// gone (DuckDB auto-drops TEMPORARY on session exit — this is the
/// RAII-equivalent cleanup the spec requires).
#[test]
#[ignore]
fn test_vrp_ac13_cli_staging_cleanup_on_success() {
    let tmp = tempfile::tempdir().unwrap();
    let schema = write_schema(tmp.path(), SCHEMA_WITH_EXT);
    let csv = write_csv(tmp.path(), "in.csv", CSV_TWO_REJECTS);
    let db = tmp.path().join("out.db");

    let (_code, _stdout, _stderr) = run_validate(&[
        csv.to_str().unwrap(),
        schema.to_str().unwrap(),
        "--db",
        db.to_str().unwrap(),
        "--table",
        "orders",
    ]);

    let staging_count = duckdb_query(
        &db,
        "SELECT COUNT(*) FROM duckdb_tables \
         WHERE table_name LIKE '__finetype_staging_%';",
    );
    assert_eq!(staging_count, "0", "staging table leaked after success");
}

/// ac-09 + ac-13: When the flow bails before opening a DuckDB session, no
/// `.db` file is created at all — the TEMPORARY/session-lifetime guarantee
/// is vacuously true, and no staging residue exists on disk.
#[test]
#[ignore]
fn test_vrp_ac13_cli_staging_cleanup_on_failure() {
    let tmp = tempfile::tempdir().unwrap();
    let schema = write_schema(tmp.path(), SCHEMA_WITH_EXT);
    let db = tmp.path().join("out.db");

    let (code, _stdout, _stderr) = run_validate(&[
        "/nonexistent/input.csv",
        schema.to_str().unwrap(),
        "--db",
        db.to_str().unwrap(),
        "--table",
        "orders",
    ]);
    assert_eq!(code, 2);
    assert!(
        !db.exists(),
        "error path wrote a partial .db at {}",
        db.display()
    );
}

/// ac-10 + ac-13: exit-code grid.
/// | rejects | --lenient | expected exit |
/// | 0       | no        | 0             |
/// | >0      | no        | 1             |
/// | >0      | yes       | 0             |
/// | error   | —         | 2             |
#[test]
#[ignore]
fn test_vrp_ac13_cli_exit_code_grid() {
    let tmp = tempfile::tempdir().unwrap();
    let schema = write_schema(tmp.path(), SCHEMA_WITH_EXT);
    let clean_csv = write_csv(
        tmp.path(),
        "clean.csv",
        "order_id,status\nORD-11111,pending\n",
    );
    let rejecting_csv = write_csv(tmp.path(), "dirty.csv", CSV_TWO_REJECTS);

    // (a) zero rejects, no --lenient → exit 0
    let db_a = tmp.path().join("a.db");
    let (code_a, _, _) = run_validate(&[
        clean_csv.to_str().unwrap(),
        schema.to_str().unwrap(),
        "--db",
        db_a.to_str().unwrap(),
        "--table",
        "orders",
    ]);
    assert_eq!(code_a, 0, "zero-reject should exit 0");

    // (b) rejects, no --lenient → exit 1
    let db_b = tmp.path().join("b.db");
    let (code_b, _, _) = run_validate(&[
        rejecting_csv.to_str().unwrap(),
        schema.to_str().unwrap(),
        "--db",
        db_b.to_str().unwrap(),
        "--table",
        "orders",
    ]);
    assert_eq!(code_b, 1, "reject-without-lenient should exit 1");

    // (c) rejects, --lenient → exit 0
    let db_c = tmp.path().join("c.db");
    let (code_c, _, _) = run_validate(&[
        rejecting_csv.to_str().unwrap(),
        schema.to_str().unwrap(),
        "--db",
        db_c.to_str().unwrap(),
        "--table",
        "orders",
        "--lenient",
    ]);
    assert_eq!(code_c, 0, "reject-with-lenient should exit 0");

    // (d) error → exit 2 regardless of --lenient
    let db_d = tmp.path().join("d.db");
    let (code_d, _, _) = run_validate(&[
        "/nonexistent/input.csv",
        schema.to_str().unwrap(),
        "--db",
        db_d.to_str().unwrap(),
        "--table",
        "orders",
        "--lenient",
    ]);
    assert_eq!(code_d, 2, "error path should exit 2 even with --lenient");
}

/// ac-08 + ac-13: malformed-schema error grid — 4 failure modes each exit
/// 2 and leave no `.db` on disk.
#[test]
#[ignore]
fn test_vrp_ac13_cli_malformed_schema_error_grid() {
    let tmp = tempfile::tempdir().unwrap();
    let csv = write_csv(tmp.path(), "in.csv", CSV_TWO_REJECTS);

    // (1) missing-file
    let db1 = tmp.path().join("db1.db");
    let (code1, _, stderr1) = run_validate(&[
        csv.to_str().unwrap(),
        "/nonexistent/schema.json",
        "--db",
        db1.to_str().unwrap(),
        "--table",
        "orders",
    ]);
    assert_eq!(code1, 2);
    assert!(stderr1.contains("schema file not found"), "got: {stderr1}");
    assert!(!db1.exists());

    // (2) invalid JSON
    let bad_json = tmp.path().join("bad.json");
    std::fs::write(&bad_json, "{ this is { not valid json").unwrap();
    let db2 = tmp.path().join("db2.db");
    let (code2, _, stderr2) = run_validate(&[
        csv.to_str().unwrap(),
        bad_json.to_str().unwrap(),
        "--db",
        db2.to_str().unwrap(),
        "--table",
        "orders",
    ]);
    assert_eq!(code2, 2);
    assert!(stderr2.contains("invalid JSON"), "got: {stderr2}");
    assert!(!db2.exists());

    // (3) missing properties block
    let no_props = tmp.path().join("noprops.json");
    std::fs::write(&no_props, r#"{"type": "object"}"#).unwrap();
    let db3 = tmp.path().join("db3.db");
    let (code3, _, stderr3) = run_validate(&[
        csv.to_str().unwrap(),
        no_props.to_str().unwrap(),
        "--db",
        db3.to_str().unwrap(),
        "--table",
        "orders",
    ]);
    assert_eq!(code3, 2);
    assert!(
        stderr3.contains("missing required `properties`"),
        "got: {stderr3}"
    );
    assert!(!db3.exists());

    // (4) permission-denied (best-effort: chmod 000; skip if chmod fails,
    //     e.g. non-POSIX filesystems).
    let unreadable = tmp.path().join("nope.json");
    std::fs::write(&unreadable, SCHEMA_WITH_EXT).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&unreadable).unwrap().permissions();
        perms.set_mode(0o000);
        // chmod 000 doesn't block the current process when running as root.
        // Probe by trying to read after chmod; if we still can, skip the
        // sub-case rather than produce a false-positive assertion.
        let chmod_ok = std::fs::set_permissions(&unreadable, perms).is_ok();
        let effective = chmod_ok && std::fs::read_to_string(&unreadable).is_err();
        if effective {
            let db4 = tmp.path().join("db4.db");
            let (code4, _, stderr4) = run_validate(&[
                csv.to_str().unwrap(),
                unreadable.to_str().unwrap(),
                "--db",
                db4.to_str().unwrap(),
                "--table",
                "orders",
            ]);
            assert_eq!(code4, 2);
            assert!(
                stderr4.contains("permission denied") || stderr4.contains("not found"),
                "got: {stderr4}"
            );
            assert!(!db4.exists());
        }
        // Restore perms so tempdir can clean up regardless.
        let mut restore = std::fs::metadata(&unreadable).unwrap().permissions();
        restore.set_mode(0o644);
        let _ = std::fs::set_permissions(&unreadable, restore);
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// ac-11: x-finetype-label / x-finetype-confidence surface as
// expected_type / type_confidence; graceful NULL on absence.
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn test_vrp_ac11_xft_extensions_surface() {
    let tmp = tempfile::tempdir().unwrap();
    let schema = write_schema(tmp.path(), SCHEMA_WITH_EXT);
    let csv = write_csv(tmp.path(), "in.csv", CSV_TWO_REJECTS);
    let db = tmp.path().join("out.db");

    let (_code, _, _) = run_validate(&[
        csv.to_str().unwrap(),
        schema.to_str().unwrap(),
        "--db",
        db.to_str().unwrap(),
        "--table",
        "orders",
    ]);

    // order_id column has x-finetype-* → expected_type + type_confidence populated.
    let order_id_row = duckdb_query(
        &db,
        "SELECT expected_type || '|' || type_confidence \
         FROM finetype_reject_errors WHERE column_name='order_id';",
    );
    assert_eq!(order_id_row, "identity.code.id|0.99");

    // status column lacks x-finetype-* → NULLs.
    let status_nulls = duckdb_query(
        &db,
        "SELECT COUNT(*) FROM finetype_reject_errors \
         WHERE column_name='status' AND expected_type IS NULL AND type_confidence IS NULL;",
    );
    assert_eq!(status_nulls, "1");
}

#[test]
#[ignore]
fn test_vrp_ac11_null_on_absence() {
    let tmp = tempfile::tempdir().unwrap();
    let schema = write_schema(tmp.path(), SCHEMA_NO_EXT);
    let csv = write_csv(tmp.path(), "in.csv", CSV_TWO_REJECTS);
    let db = tmp.path().join("out.db");

    let (_code, _, _) = run_validate(&[
        csv.to_str().unwrap(),
        schema.to_str().unwrap(),
        "--db",
        db.to_str().unwrap(),
        "--table",
        "orders",
    ]);

    // With no x-finetype-* on any property, ALL rejects have NULL extension cols.
    let all_null = duckdb_query(
        &db,
        "SELECT COUNT(*) FROM finetype_reject_errors \
         WHERE expected_type IS NULL AND type_confidence IS NULL;",
    );
    let total = duckdb_query(&db, "SELECT COUNT(*) FROM finetype_reject_errors;");
    assert_eq!(all_null, total);
    assert_ne!(total, "0", "test fixture must produce rejects");
}

// ═══════════════════════════════════════════════════════════════════════════════
// ac-12: end-to-end on the ecommerce_orders fixture —
// authored-time type_confidence distinguishes "classifier wrong"
// from "data bad".
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn test_vrp_ac12_ecommerce_end_to_end() {
    let tmp = tempfile::tempdir().unwrap();
    // Use a narrow 2-column slice (order_id + status) from the ecommerce
    // fixture so the test doesn't hinge on the full fixture's evolving
    // column set.
    let fixture = workspace_root().join("eval/datasets/csv/ecommerce_orders.csv");
    if !fixture.exists() {
        eprintln!("skipping ac-12: fixture missing at {}", fixture.display());
        return;
    }

    // Project the fixture down to (order_id, status) to keep the schema small.
    let slice_path = tmp.path().join("slice.csv");
    let raw = std::fs::read_to_string(&fixture).unwrap();
    let mut lines = raw.lines();
    let header = lines.next().unwrap();
    let header_cols: Vec<&str> = header.split(',').collect();
    let oi = header_cols.iter().position(|h| *h == "order_id").unwrap();
    let si = header_cols.iter().position(|h| *h == "status").unwrap();
    let mut slice = String::from("order_id,status\n");
    // 50 rows is plenty for the e2e signal; keep bounded so the test is fast.
    for (i, row) in lines.take(50).enumerate() {
        let cols: Vec<&str> = row.split(',').collect();
        // Mutate the 3rd row to guarantee both reject types are exercised
        // in case the fixture is clean.
        if i == 2 {
            slice.push_str(&format!("BAD-{oi:05},INVALID_STATUS\n"));
        } else {
            slice.push_str(&format!("{},{}\n", cols[oi], cols[si]));
        }
    }
    std::fs::write(&slice_path, &slice).unwrap();

    let schema = write_schema(tmp.path(), SCHEMA_WITH_EXT);
    let db = tmp.path().join("ecom.db");

    let (_code, _, _) = run_validate(&[
        slice_path.to_str().unwrap(),
        schema.to_str().unwrap(),
        "--db",
        db.to_str().unwrap(),
        "--table",
        "orders",
    ]);

    // At least one reject with high authored confidence (classifier-sure
    // row deemed bad by the validator).
    let high_conf_rejects = duckdb_query(
        &db,
        "SELECT COUNT(*) FROM finetype_reject_errors \
         WHERE column_name='order_id' AND type_confidence >= 0.99;",
    );
    assert_ne!(
        high_conf_rejects, "0",
        "ac-12: expected at least one classifier-confident reject on order_id"
    );

    // scan_id is 1 on fresh db.
    let scan = duckdb_query(&db, "SELECT MAX(scan_id) FROM finetype_reject_errors;");
    assert_eq!(scan, "1");
}

// ═══════════════════════════════════════════════════════════════════════════════
// ac-02 / ac-03 / ac-04 — typed CTAS + TRANSFORM_FAILED + NULL-in-NULL-out
// (validate absorbs load — spec 2026-04-28-validate-absorbs-load)
// ═══════════════════════════════════════════════════════════════════════════════

/// Schema with a labelled DATE column. `datetime.date.iso` carries a
/// strptime transform and a non-VARCHAR DuckDB type, so the materialise
/// path emits `TRY(strptime(...)::DATE) AS "delivery_date"` (per
/// `build_transform_projection` branch 4 + `try_wrap=true`).
const SCHEMA_TYPED_DATE: &str = r#"{
  "type": "object",
  "properties": {
    "order_id": {
      "type": "string",
      "x-finetype-label": "identity.code.id",
      "x-finetype-confidence": 0.99
    },
    "delivery_date": {
      "type": "string",
      "x-finetype-label": "datetime.date.iso",
      "x-finetype-confidence": 0.97
    }
  }
}"#;

/// ac-03: One row's `delivery_date` contains `2024-02-30` — well-formed
/// against the schema's pattern (10 chars, `\d{4}-\d{2}-\d{2}` if any),
/// but DuckDB's strptime refuses it (Feb has no 30th). The pre-CTAS
/// sweep must record a TRANSFORM_FAILED reject and exclude the row from
/// the user table.
#[test]
#[ignore]
fn test_vrp_transform_failure_emits_reject_row() {
    let tmp = tempfile::tempdir().unwrap();
    let schema = write_schema(tmp.path(), SCHEMA_TYPED_DATE);
    // 3 rows: one valid, one with the impossible date, one valid.
    let csv = write_csv(
        tmp.path(),
        "in.csv",
        "order_id,delivery_date\nORD-1,2024-01-15\nORD-2,2024-02-30\nORD-3,2024-03-20\n",
    );
    let db = tmp.path().join("out.db");

    let (code, _, stderr) = run_validate(&[
        csv.to_str().unwrap(),
        schema.to_str().unwrap(),
        "--db",
        db.to_str().unwrap(),
        "--table",
        "orders",
    ]);
    assert_eq!(
        code, 1,
        "transform-failure should produce a reject row → exit 1; stderr={stderr}"
    );

    // One TRANSFORM_FAILED reject, on delivery_date, line 3 (1-indexed,
    // 0-based row idx=1 + 1 = 2, but engine convention is row_idx+1=3
    // because line 1 is the CSV header — see existing reject INSERT).
    let tf_count = duckdb_query(
        &db,
        "SELECT COUNT(*) FROM finetype_reject_errors \
         WHERE error_type = 'TRANSFORM_FAILED' AND column_name = 'delivery_date';",
    );
    assert_eq!(tf_count, "1", "expected exactly one TRANSFORM_FAILED row");

    // constraint_failed = 'transform' (extends the reject ontology).
    let cf = duckdb_query(
        &db,
        "SELECT DISTINCT constraint_failed FROM finetype_reject_errors \
         WHERE error_type = 'TRANSFORM_FAILED';",
    );
    assert_eq!(cf, "transform");

    // constraint_value carries the original staging cell text so analysts
    // can see what DuckDB choked on without re-reading the CSV.
    let cv = duckdb_query(
        &db,
        "SELECT constraint_value FROM finetype_reject_errors \
         WHERE error_type = 'TRANSFORM_FAILED';",
    );
    assert_eq!(cv, "2024-02-30");

    // expected_type is the x-finetype-label, type_confidence the authored 0.97.
    let exp = duckdb_query(
        &db,
        "SELECT expected_type || '|' || type_confidence FROM finetype_reject_errors \
         WHERE error_type = 'TRANSFORM_FAILED';",
    );
    assert_eq!(exp, "datetime.date.iso|0.97");

    // The user table excludes the failed row — 2 rows survive.
    let user_count = duckdb_query(&db, "SELECT COUNT(*) FROM orders;");
    assert_eq!(user_count, "2");

    // …and the surviving rows are typed DATE, not VARCHAR.
    let col_type = duckdb_query(
        &db,
        "SELECT data_type FROM duckdb_columns \
         WHERE table_name = 'orders' AND column_name = 'delivery_date';",
    );
    assert_eq!(col_type, "DATE");
}

/// ac-04: NULL staging cells must NOT trigger TRANSFORM_FAILED. The
/// failure predicate is `staging IS NOT NULL AND TRY(transform) IS NULL`.
/// An empty cell becomes a NULL → typed-NULL passthrough, no reject.
#[test]
#[ignore]
fn test_vrp_null_staging_passes_to_typed_null() {
    let tmp = tempfile::tempdir().unwrap();
    let schema = write_schema(tmp.path(), SCHEMA_TYPED_DATE);
    // Middle row has an empty delivery_date cell.
    let csv = write_csv(
        tmp.path(),
        "in.csv",
        "order_id,delivery_date\nORD-1,2024-01-15\nORD-2,\nORD-3,2024-03-20\n",
    );
    let db = tmp.path().join("out.db");

    let (code, _, stderr) = run_validate(&[
        csv.to_str().unwrap(),
        schema.to_str().unwrap(),
        "--db",
        db.to_str().unwrap(),
        "--table",
        "orders",
    ]);
    assert_eq!(
        code, 0,
        "NULL staging cell must NOT register a reject; stderr={stderr}"
    );

    // No TRANSFORM_FAILED rows.
    let tf_count = duckdb_query(
        &db,
        "SELECT COUNT(*) FROM finetype_reject_errors \
         WHERE error_type = 'TRANSFORM_FAILED';",
    );
    assert_eq!(tf_count, "0");

    // All 3 rows survive in the user table.
    let user_count = duckdb_query(&db, "SELECT COUNT(*) FROM orders;");
    assert_eq!(user_count, "3");

    // The empty cell is a typed NULL.
    let null_dates = duckdb_query(
        &db,
        "SELECT COUNT(*) FROM orders WHERE delivery_date IS NULL;",
    );
    assert_eq!(null_dates, "1");
}

/// ac-02: A clean run (no transform failures) round-trips through the
/// typed projection — the user table's `delivery_date` is DATE, the
/// `order_id` column passes through as VARCHAR (its label
/// `identity.code.id` isn't in the taxonomy → bare passthrough).
#[test]
#[ignore]
fn test_vrp_typed_ctas_round_trip() {
    let tmp = tempfile::tempdir().unwrap();
    let schema = write_schema(tmp.path(), SCHEMA_TYPED_DATE);
    let csv = write_csv(
        tmp.path(),
        "in.csv",
        "order_id,delivery_date\nORD-1,2024-01-15\nORD-2,2024-03-20\n",
    );
    let db = tmp.path().join("out.db");

    let (code, _, stderr) = run_validate(&[
        csv.to_str().unwrap(),
        schema.to_str().unwrap(),
        "--db",
        db.to_str().unwrap(),
        "--table",
        "orders",
    ]);
    assert_eq!(code, 0, "clean fixture should exit 0; stderr={stderr}");

    let date_type = duckdb_query(
        &db,
        "SELECT data_type FROM duckdb_columns \
         WHERE table_name = 'orders' AND column_name = 'delivery_date';",
    );
    assert_eq!(date_type, "DATE");

    let id_type = duckdb_query(
        &db,
        "SELECT data_type FROM duckdb_columns \
         WHERE table_name = 'orders' AND column_name = 'order_id';",
    );
    assert_eq!(id_type, "VARCHAR");

    // Date arithmetic confirms typed semantics.
    let span = duckdb_query(
        &db,
        "SELECT date_diff('day', MIN(delivery_date), MAX(delivery_date)) FROM orders;",
    );
    assert_eq!(span, "65");
}

/// ac-02 + graceful-degradation: A column whose `x-finetype-label` is
/// not in the taxonomy must pass through as a bare quoted identifier,
/// the same as an unlabelled column. No CAST, no transform, no reject.
#[test]
#[ignore]
fn test_vrp_unknown_label_passes_as_varchar() {
    let tmp = tempfile::tempdir().unwrap();
    // `identity.code.id` is the canonical fixture label that is NOT in
    // the taxonomy — the existing 8 vrp_* tests rely on this exact
    // label for their graceful-degradation contract (see
    // SCHEMA_WITH_EXT).
    let schema = write_schema(tmp.path(), SCHEMA_WITH_EXT);
    let csv = write_csv(
        tmp.path(),
        "in.csv",
        "order_id,status\nORD-11111,pending\nORD-22222,shipped\n",
    );
    let db = tmp.path().join("out.db");

    let (code, _, stderr) = run_validate(&[
        csv.to_str().unwrap(),
        schema.to_str().unwrap(),
        "--db",
        db.to_str().unwrap(),
        "--table",
        "orders",
    ]);
    assert_eq!(code, 0, "clean fixture should exit 0; stderr={stderr}");

    // Both columns are VARCHAR (no typed CAST emitted).
    let id_type = duckdb_query(
        &db,
        "SELECT data_type FROM duckdb_columns \
         WHERE table_name = 'orders' AND column_name = 'order_id';",
    );
    assert_eq!(id_type, "VARCHAR");

    // No transform failures recorded for an all-VARCHAR projection.
    let tf_count = duckdb_query(
        &db,
        "SELECT COUNT(*) FROM finetype_reject_errors \
         WHERE error_type = 'TRANSFORM_FAILED';",
    );
    assert_eq!(tf_count, "0");
}

/// ac-06: `finetype load …` no longer exists. Clap's unknown-subcommand
/// handler must reject the verb with exit code 2 (clap convention) and
/// emit a help-style error pointing at the available subcommands.
/// No shim, no warning, no carve-out — hard removal posture (PR #51 lineage).
#[test]
#[ignore]
fn test_vrp_load_subcommand_removed() {
    let (code, _stdout, stderr) = run_finetype_raw(&["load", "--file", "any.csv"]);
    assert_eq!(
        code, 2,
        "`finetype load` must exit 2 via clap unknown-subcommand handler; \
         got code={code}, stderr={stderr}"
    );
    // Clap's stock message for an unrecognised subcommand mentions the
    // offending verb. Either "unrecognized" or "unknown" wording is
    // acceptable across clap versions.
    let stderr_lower = stderr.to_lowercase();
    assert!(
        stderr_lower.contains("load")
            && (stderr_lower.contains("unrecognized")
                || stderr_lower.contains("unknown")
                || stderr_lower.contains("invalid")),
        "stderr should signal unknown subcommand mentioning 'load'; got: {stderr}"
    );
}
