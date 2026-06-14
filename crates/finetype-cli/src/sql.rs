//! SQL string helpers and DuckDB query utilities for the reject sidecar.

use super::*;

/// Quote a string for safe embedding as a DuckDB single-quoted literal.
/// DuckDB doubles single-quotes inside literals.
pub(crate) fn sql_quote(s: &str) -> String {
    let escaped = s.replace('\'', "''");
    format!("'{}'", escaped)
}

/// Quote an identifier for DuckDB. Identifiers are wrapped in double-quotes;
/// internal double-quotes are doubled.
pub(crate) fn sql_ident(s: &str) -> String {
    let escaped = s.replace('"', "\"\"");
    format!("\"{}\"", escaped)
}

/// Render an `Option<String>` as a SQL literal ('value' or NULL).
pub(crate) fn sql_opt_str(v: &Option<String>) -> String {
    match v {
        Some(s) => sql_quote(s),
        None => "NULL".to_string(),
    }
}

/// Render an `Option<f64>` as a SQL literal (numeric or NULL).
pub(crate) fn sql_opt_f64(v: &Option<f64>) -> String {
    match v {
        Some(f) => format!("{}", f),
        None => "NULL".to_string(),
    }
}

/// Run a small query against `db_path` (must exist) using the duckdb CLI
/// and return the trimmed stdout. Returns None when the database does not
/// exist or the query fails.
pub(crate) fn duckdb_query_scalar(db_path: &PathBuf, sql: &str) -> Option<String> {
    if !db_path.exists() {
        return None;
    }
    let out = std::process::Command::new("duckdb")
        .arg("-noheader")
        .arg("-list")
        .arg(db_path)
        .arg("-c")
        .arg(sql)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

/// Determine the scan_id for this invocation per constraint 7 and ac-09.
///
/// Fresh database or no existing sidecar → 1.
/// Existing sidecar → MAX(scan_id) + 1.
pub(crate) fn next_scan_id(db_path: &PathBuf) -> i64 {
    // Query requires the sidecar to exist. Wrap in a try/coerce so an
    // absent table yields "0" via COALESCE.
    let sql = "SELECT COALESCE(MAX(scan_id), 0) FROM finetype_reject_errors;";
    match duckdb_query_scalar(db_path, sql) {
        Some(s) => s.parse::<i64>().unwrap_or(0) + 1,
        None => 1,
    }
}

/// Check whether `table_name` already exists in `db_path`. Used for the
/// ac-09 staging-collision gate (error exit 2 when not using --append).
pub(crate) fn user_table_exists(db_path: &PathBuf, table_name: &str) -> bool {
    let sql = format!(
        "SELECT 1 FROM duckdb_tables WHERE table_name = {} LIMIT 1;",
        sql_quote(table_name)
    );
    matches!(duckdb_query_scalar(db_path, &sql).as_deref(), Some("1"))
}

/// Render the reject sidecar's CREATE TABLE IF NOT EXISTS statement per
/// ontology (spec `RejectEntry`, 13 columns).
pub(crate) const REJECT_SIDECAR_DDL: &str = "\
CREATE TABLE IF NOT EXISTS finetype_reject_errors (
    scan_id BIGINT,
    file_id BIGINT,
    line BIGINT,
    column_idx INTEGER,
    column_name VARCHAR,
    error_type VARCHAR,
    csv_line VARCHAR,
    byte_position BIGINT,
    error_message VARCHAR,
    type_confidence DOUBLE,
    expected_type VARCHAR,
    constraint_failed VARCHAR,
    constraint_value VARCHAR
);";
