//! Golden integration tests for FineType CLI commands.
//!
//! These tests call the compiled binary and assert structured output.
//! All tests are `#[ignore]` because they load the model (~3s startup).
//!
//! Run with: `cargo test -p finetype-cli --test cli_golden -- --ignored`

use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;

// ═══════════════════════════════════════════════════════════════════════════════
// HELPERS
// ═══════════════════════════════════════════════════════════════════════════════

/// Get the workspace root directory.
fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent() // crates/
        .unwrap()
        .parent() // finetype/
        .unwrap()
        .to_path_buf()
}

/// Run `finetype profile -f <path> -o json` and return parsed JSON.
fn run_profile_json(csv_path: &Path) -> Value {
    let output = Command::new("cargo")
        .args([
            "run",
            "-p",
            "finetype-cli",
            "--",
            "profile",
            "-f",
            csv_path.to_str().unwrap(),
            "-o",
            "json",
        ])
        .current_dir(workspace_root())
        .output()
        .expect("failed to run finetype profile");

    assert!(
        output.status.success(),
        "profile failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("invalid utf8");
    serde_json::from_str(&stdout).unwrap_or_else(|e| {
        panic!("failed to parse profile JSON: {e}\nOutput: {stdout}");
    })
}

/// Run `finetype load -f <path>` and return the DDL string.
fn run_load(csv_path: &Path) -> String {
    let output = Command::new("cargo")
        .args([
            "run",
            "-p",
            "finetype-cli",
            "--",
            "load",
            "-f",
            csv_path.to_str().unwrap(),
        ])
        .current_dir(workspace_root())
        .output()
        .expect("failed to run finetype load");

    assert!(
        output.status.success(),
        "load failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8(output.stdout).expect("invalid utf8")
}

/// Run `finetype taxonomy --output json` and return parsed JSON array.
fn run_taxonomy_json() -> Value {
    let output = Command::new("cargo")
        .args([
            "run",
            "-p",
            "finetype-cli",
            "--",
            "taxonomy",
            "--output",
            "json",
        ])
        .current_dir(workspace_root())
        .output()
        .expect("failed to run finetype taxonomy");

    assert!(
        output.status.success(),
        "taxonomy failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("invalid utf8");
    serde_json::from_str(&stdout).unwrap_or_else(|e| {
        panic!("failed to parse taxonomy JSON: {e}");
    })
}

/// Run `finetype infer -i <input> --mode <mode> --output json` and return
/// parsed JSON.
fn run_infer_json(input: &str, mode: &str) -> Value {
    let output = Command::new("cargo")
        .args([
            "run",
            "-p",
            "finetype-cli",
            "--",
            "infer",
            "-i",
            input,
            "--mode",
            mode,
            "--output",
            "json",
        ])
        .current_dir(workspace_root())
        .output()
        .expect("failed to run finetype infer");

    assert!(
        output.status.success(),
        "infer failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("invalid utf8");
    serde_json::from_str(&stdout).unwrap_or_else(|e| {
        panic!("failed to parse infer JSON: {e}\nOutput: {stdout}");
    })
}

/// Run `finetype schema <key> --pretty` and return parsed JSON.
fn run_schema_json(type_key: &str) -> Value {
    let output = Command::new("cargo")
        .args([
            "run",
            "-p",
            "finetype-cli",
            "--",
            "schema",
            type_key,
            "--pretty",
        ])
        .current_dir(workspace_root())
        .output()
        .expect("failed to run finetype schema");

    assert!(
        output.status.success(),
        "schema failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("invalid utf8");
    serde_json::from_str(&stdout).unwrap_or_else(|e| {
        panic!("failed to parse schema JSON: {e}\nOutput: {stdout}");
    })
}

/// Extract column profiles as a vec of (column_name, type_label, broad_type).
fn extract_columns(profile: &Value) -> Vec<(String, String, String)> {
    profile["columns"]
        .as_array()
        .expect("profile missing columns array")
        .iter()
        .map(|col| {
            (
                col["column"].as_str().unwrap_or("").to_string(),
                col["type"].as_str().unwrap_or("").to_string(),
                col["broad_type"].as_str().unwrap_or("").to_string(),
            )
        })
        .collect()
}

/// Assert a column has the expected type label.
fn assert_column_type(
    columns: &[(String, String, String)],
    column_name: &str,
    expected_type: &str,
) {
    let col = columns
        .iter()
        .find(|(name, _, _)| name == column_name)
        .unwrap_or_else(|| panic!("column '{column_name}' not found in profile"));
    assert_eq!(
        col.1, expected_type,
        "column '{}': expected type '{}', got '{}'",
        column_name, expected_type, col.1
    );
}

/// Assert a column has the expected broad_type.
fn assert_column_broad_type(
    columns: &[(String, String, String)],
    column_name: &str,
    expected_broad_type: &str,
) {
    let col = columns
        .iter()
        .find(|(name, _, _)| name == column_name)
        .unwrap_or_else(|| panic!("column '{column_name}' not found in profile"));
    assert_eq!(
        col.2, expected_broad_type,
        "column '{}': expected broad_type '{}', got '{}'",
        column_name, expected_broad_type, col.2
    );
}

/// Assert a column's type starts with the expected domain prefix.
fn assert_column_domain(
    columns: &[(String, String, String)],
    column_name: &str,
    expected_domain: &str,
) {
    let col = columns
        .iter()
        .find(|(name, _, _)| name == column_name)
        .unwrap_or_else(|| panic!("column '{column_name}' not found in profile"));
    assert!(
        col.1.starts_with(expected_domain),
        "column '{}': expected domain '{}', got type '{}'",
        column_name,
        expected_domain,
        col.1
    );
}

/// Path to a dataset CSV file (eval/datasets/csv/).
fn dataset_path(name: &str) -> PathBuf {
    workspace_root()
        .join("eval")
        .join("datasets")
        .join("csv")
        .join(name)
}

/// Path to a fixture CSV file (tests/fixtures/).
fn fixture_path(name: &str) -> PathBuf {
    workspace_root().join("tests").join("fixtures").join(name)
}

// ═══════════════════════════════════════════════════════════════════════════════
// PROFILE GOLDEN TESTS — REAL-WORLD DATASETS
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn golden_profile_datetime_formats() {
    let profile = run_profile_json(&dataset_path("datetime_formats.csv"));
    let cols = extract_columns(&profile);

    assert_eq!(cols.len(), 14, "datetime_formats should have 14 columns");

    // Every column should be in the datetime domain
    assert_column_type(&cols, "iso_date", "datetime.date.iso");
    assert_column_type(&cols, "us_date", "datetime.date.mdy_slash");
    assert_column_type(&cols, "eu_date", "datetime.date.dmy_slash");
    assert_column_type(&cols, "iso_timestamp", "datetime.timestamp.iso_8601");
    assert_column_type(&cols, "sql_timestamp", "datetime.timestamp.sql_standard");
    assert_column_type(&cols, "unix_epoch", "datetime.epoch.unix_seconds");
    assert_column_type(&cols, "unix_ms", "datetime.epoch.unix_milliseconds");
    assert_column_type(&cols, "year", "datetime.component.year");
    assert_column_type(&cols, "month_name", "datetime.component.month_name");
    assert_column_type(&cols, "day_of_week", "datetime.component.day_of_week");
    assert_column_type(&cols, "time_24h", "datetime.time.hms_24h");
    assert_column_type(&cols, "duration_iso", "datetime.duration.iso_8601");
    assert_column_type(&cols, "utc_offset", "datetime.offset.utc");
    assert_column_type(&cols, "timezone", "datetime.offset.iana");

    // Verify broad_types for key columns
    assert_column_broad_type(&cols, "iso_date", "DATE");
    assert_column_broad_type(&cols, "iso_timestamp", "TIMESTAMP");
    assert_column_broad_type(&cols, "unix_epoch", "TIMESTAMP");
    assert_column_broad_type(&cols, "time_24h", "TIME");
    assert_column_broad_type(&cols, "duration_iso", "INTERVAL");
    assert_column_broad_type(&cols, "year", "SMALLINT");
}

#[test]
#[ignore]
fn golden_profile_ecommerce_orders() {
    let profile = run_profile_json(&dataset_path("ecommerce_orders.csv"));
    let cols = extract_columns(&profile);

    assert_eq!(cols.len(), 12, "ecommerce_orders should have 12 columns");

    // Key type assertions
    assert_column_type(
        &cols,
        "order_id",
        "representation.identifier.alphanumeric_id",
    );
    assert_column_type(&cols, "customer_email", "identity.person.email");
    assert_column_type(&cols, "order_date", "datetime.date.iso");
    assert_column_type(&cols, "total_price", "finance.currency.amount");
    assert_column_type(&cols, "currency", "finance.currency.currency_code");
    assert_column_type(&cols, "credit_card_last4", "geography.address.postal_code");
    assert_column_type(&cols, "shipping_country", "geography.location.country");
    assert_column_type(
        &cols,
        "shipping_postal_code",
        "geography.address.postal_code",
    );
    assert_column_type(&cols, "status", "representation.discrete.categorical");
    assert_column_type(&cols, "is_gift", "representation.boolean.terms");
    assert_column_type(&cols, "tracking_url", "technology.internet.url");
    // v16: phone correctly classified as phone_number (v14 misclassified as ssn)
    assert_column_type(&cols, "phone", "identity.person.phone_number");

    // Broad types for key columns
    assert_column_broad_type(&cols, "order_date", "DATE");
    assert_column_broad_type(&cols, "total_price", "DECIMAL");
    assert_column_broad_type(&cols, "is_gift", "BOOLEAN");
}

#[test]
#[ignore]
fn golden_profile_titanic() {
    let profile = run_profile_json(&dataset_path("titanic.csv"));
    let cols = extract_columns(&profile);

    assert_eq!(cols.len(), 12, "titanic should have 12 columns");

    // Core assertions — the showstoppers that NNFT-254 fixed
    assert_column_type(&cols, "Name", "identity.person.full_name");
    assert_column_type(&cols, "Survived", "representation.boolean.binary");
    assert_column_type(&cols, "Sex", "identity.person.gender");
    assert_column_type(&cols, "Fare", "finance.currency.amount");
    assert_column_type(&cols, "Embarked", "representation.discrete.categorical");

    // Cabin should NOT be ICD10 — this was the showstopper bug
    assert_column_domain(&cols, "Cabin", "representation.");

    // Age: currently integer_number
    assert_column_type(&cols, "Age", "representation.numeric.integer_number");

    // SibSp → ordinal (low cardinality integers), Parch → integer
    assert_column_type(&cols, "SibSp", "representation.discrete.ordinal");
    assert_column_type(&cols, "Parch", "representation.numeric.integer_number");

    // Broad types
    assert_column_broad_type(&cols, "Survived", "BOOLEAN");
    assert_column_broad_type(&cols, "Fare", "DECIMAL");
    assert_column_broad_type(&cols, "Parch", "BIGINT");
}

#[test]
#[ignore]
fn golden_profile_people_directory() {
    let profile = run_profile_json(&dataset_path("people_directory.csv"));
    let cols = extract_columns(&profile);

    assert_eq!(cols.len(), 14, "people_directory should have 14 columns");

    // Identity types
    assert_column_type(&cols, "full_name", "identity.person.full_name");
    assert_column_type(&cols, "first_name", "identity.person.first_name");
    assert_column_type(&cols, "last_name", "identity.person.last_name");
    assert_column_type(&cols, "email", "identity.person.email");
    // phone→ssn is a known model misclassification
    assert_column_type(&cols, "phone", "identity.government.ssn");
    assert_column_type(&cols, "gender", "identity.person.gender");
    assert_column_type(&cols, "ssn", "identity.government.ssn");
    assert_column_type(&cols, "height_cm", "identity.person.height");
    assert_column_type(&cols, "weight_kg", "identity.person.weight");

    // Datetime
    assert_column_domain(&cols, "date_of_birth", "datetime.");

    // Representation
    assert_column_type(&cols, "company", "representation.text.entity_name");
    assert_column_type(&cols, "job_title", "representation.discrete.categorical");
    assert_column_type(&cols, "salary", "finance.currency.amount");

    // Broad types
    assert_column_broad_type(&cols, "salary", "DECIMAL");
    assert_column_broad_type(&cols, "height_cm", "DOUBLE");
}

// ═══════════════════════════════════════════════════════════════════════════════
// PROFILE GOLDEN TESTS — FOCUSED FIXTURES
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn golden_profile_ambiguous_headers() {
    let profile = run_profile_json(&fixture_path("ambiguous_headers.csv"));
    let cols = extract_columns(&profile);

    assert_eq!(cols.len(), 6, "ambiguous_headers should have 6 columns");

    // "id" column with integer values should be numeric
    assert_column_domain(&cols, "id", "representation.");

    // "code" with alphanumeric values
    assert_column_domain(&cols, "code", "representation.");

    // "value" with decimal values
    assert_column_domain(&cols, "value", "representation.");

    // "status" with text categories
    assert_column_domain(&cols, "status", "representation.");

    // "date" with ISO dates
    assert_column_domain(&cols, "date", "datetime.");

    // "name" with person names
    assert_column_domain(&cols, "name", "identity.");
}

#[test]
#[ignore]
fn golden_profile_numeric_edge_cases() {
    let profile = run_profile_json(&fixture_path("numeric_edge_cases.csv"));
    let cols = extract_columns(&profile);

    assert_eq!(cols.len(), 6, "numeric_edge_cases should have 6 columns");

    // "count" with only 5 small integers → numeric_code (low cardinality)
    assert_column_domain(&cols, "count", "representation.");

    // Decimals → finance.currency.amount (model inference)
    assert_column_type(&cols, "price", "finance.currency.amount");

    // Zip-like codes with leading zeros → postal_code (VARCHAR)
    assert_column_type(&cols, "zip_code", "geography.address.postal_code");
    assert_column_broad_type(&cols, "zip_code", "VARCHAR");

    // Percentages → decimal_number (model inference)
    assert_column_type(&cols, "percentage", "representation.numeric.decimal_number");

    // Large integers → integer_number (model inference)
    assert_column_type(&cols, "population", "representation.numeric.integer_number");

    // Negative decimals
    assert_column_type(
        &cols,
        "temperature",
        "representation.numeric.decimal_number",
    );
}

#[test]
#[ignore]
fn golden_profile_categoricals() {
    let profile = run_profile_json(&fixture_path("categoricals.csv"));
    let cols = extract_columns(&profile);

    assert_eq!(cols.len(), 5, "categoricals should have 5 columns");

    // Boolean yes/no → binary (0/1 mapped from yes/no)
    assert_column_type(&cols, "active", "representation.boolean.binary");

    // Single-char codes (M/F) → gender (model inference)
    assert_column_type(&cols, "gender_code", "identity.person.gender");

    // Low-cardinality text → ordinal
    assert_column_type(&cols, "priority", "representation.discrete.ordinal");

    // True/false boolean
    assert_column_type(&cols, "is_verified", "representation.boolean.terms");

    // Color names → color_hex (model sees color-like text)
    assert_column_domain(&cols, "color", "representation.");
}

// ═══════════════════════════════════════════════════════════════════════════════
// LOAD GOLDEN TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn golden_load_datetime_formats() {
    let ddl = run_load(&dataset_path("datetime_formats.csv"));

    // Should contain CREATE TABLE
    assert!(
        ddl.contains("CREATE TABLE"),
        "load output should contain CREATE TABLE"
    );

    // Key columns should have correct DuckDB types (not VARCHAR).
    // All column names are double-quoted (decision 0047).
    assert!(
        ddl.contains("::DATE AS \"iso_date\""),
        "iso_date should be DATE, got:\n{ddl}"
    );
    assert!(
        ddl.contains("strptime(\"iso_timestamp\""),
        "iso_timestamp should use strptime"
    );
    assert!(
        ddl.contains("to_timestamp(\"unix_epoch\""),
        "unix_epoch should use to_timestamp"
    );
    assert!(
        ddl.contains("to_timestamp(\"unix_ms\""),
        "unix_ms should use to_timestamp"
    );
    assert!(
        ddl.contains("::TIME AS \"time_24h\""),
        "time_24h should be TIME"
    );
    // duration_iso is passed through (VARCHAR) — no ::INTERVAL cast in load
    assert!(
        ddl.contains("\"duration_iso\""),
        "duration_iso column should be present"
    );

    // Source comment
    assert!(
        ddl.contains("datetime_formats.csv"),
        "DDL should reference source file"
    );
}

#[test]
#[ignore]
fn golden_load_ecommerce_orders() {
    let ddl = run_load(&dataset_path("ecommerce_orders.csv"));

    assert!(ddl.contains("CREATE TABLE"), "should contain CREATE TABLE");

    // Typed columns should not be plain VARCHAR.
    // All column names are double-quoted (decision 0047).
    assert!(
        ddl.contains("::DATE AS \"order_date\""),
        "order_date should be DATE"
    );
    assert!(
        ddl.contains("AS DECIMAL") && ddl.contains("\"total_price\""),
        "total_price should be DECIMAL"
    );
    assert!(
        ddl.contains("AS BOOLEAN) AS \"is_gift\""),
        "is_gift should be BOOLEAN"
    );

    // all_varchar=true should be in the read_csv call
    assert!(
        ddl.contains("all_varchar=true"),
        "should use all_varchar=true"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// TAXONOMY GOLDEN TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn golden_taxonomy_structure() {
    let taxonomy = run_taxonomy_json();

    let entries = taxonomy
        .as_array()
        .expect("taxonomy should be a JSON array");

    // Should have 240 types
    assert_eq!(entries.len(), 240, "taxonomy should have 240 types");

    // Each entry should have key, broad_type, title
    for entry in entries {
        assert!(entry["key"].is_string(), "entry missing 'key': {:?}", entry);
        assert!(
            entry["broad_type"].is_string(),
            "entry missing 'broad_type': {:?}",
            entry
        );
        assert!(
            entry["title"].is_string(),
            "entry missing 'title': {:?}",
            entry
        );
    }

    // Spot-check a few known types
    let keys: Vec<&str> = entries.iter().map(|e| e["key"].as_str().unwrap()).collect();
    assert!(
        keys.contains(&"identity.person.email"),
        "should contain email"
    );
    assert!(
        keys.contains(&"datetime.date.iso"),
        "should contain iso date"
    );
    assert!(
        keys.contains(&"geography.address.postal_code"),
        "should contain postal_code"
    );
    assert!(
        keys.contains(&"finance.currency.currency_code"),
        "should contain currency_code"
    );
}

#[test]
#[ignore]
fn golden_taxonomy_domains() {
    let taxonomy = run_taxonomy_json();
    let entries = taxonomy.as_array().unwrap();

    // Count types per domain
    let mut domain_counts = std::collections::HashMap::new();
    for entry in entries {
        let key = entry["key"].as_str().unwrap();
        let domain = key.split('.').next().unwrap();
        *domain_counts.entry(domain.to_string()).or_insert(0) += 1;
    }

    // Verify expected domain counts
    assert_eq!(domain_counts.get("container"), Some(&11));
    assert_eq!(domain_counts.get("datetime"), Some(&84));
    assert_eq!(domain_counts.get("finance"), Some(&28));
    assert_eq!(domain_counts.get("geography"), Some(&25));
    assert_eq!(domain_counts.get("identity"), Some(&33));
    assert_eq!(domain_counts.get("representation"), Some(&33));
    assert_eq!(domain_counts.get("technology"), Some(&26));
}

// ═══════════════════════════════════════════════════════════════════════════════
// SCHEMA GOLDEN TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn golden_schema_email() {
    let schema = run_schema_json("identity.person.email");

    // JSON Schema required fields
    assert!(
        schema["$schema"].is_string(),
        "schema should have $schema field"
    );
    assert_eq!(
        schema["type"].as_str(),
        Some("string"),
        "email schema type should be 'string'"
    );
    assert!(
        schema["pattern"].is_string(),
        "email schema should have pattern"
    );

    // FineType extension fields — v0.6.19 reduced schema surface keeps
    // only x-finetype-label + x-finetype-pii. Derivable fields
    // (broad-type, transform, format-string, transform-ext) were dropped
    // per the schema verbosity reduction.
    assert_eq!(
        schema["x-finetype-pii"].as_bool(),
        Some(true),
        "email should be marked as PII"
    );
    assert!(
        schema["x-finetype-broad-type"].is_null(),
        "x-finetype-broad-type was dropped from schema export in v0.6.19"
    );
    assert!(
        schema["x-finetype-transform"].is_null(),
        "x-finetype-transform was dropped from schema export in v0.6.19"
    );
    assert!(
        schema["x-finetype-format-string"].is_null(),
        "x-finetype-format-string was dropped from schema export in v0.6.19"
    );

    // Should have examples
    assert!(
        schema["examples"].is_array(),
        "email schema should have examples"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// INFER REGRESSION GUARDS
// ═══════════════════════════════════════════════════════════════════════════════

/// Regression guard for the v16 N=1 email column case.
///
/// History: v14 classified `john@example.com` correctly under
/// `--mode column` at N=1. v16 regressed — same input returned
/// `representation.text.plain_text`. Severity was low (workaround:
/// use N≥5 columns) and a fix direction was deferred to either a
/// value-based sharpen rule (decision 0048) or a retraining change.
///
/// Verified fixed on `models/default → sherlock-v19-relu-s42`
/// (2026-04-28) — the 5-branch ReLU model recovers the email
/// signal at N=1. URL and IPv4 controls were never broken; we
/// assert them too as belt-and-braces against future N=1
/// regressions across the technology.internet domain.
///
/// Spec: `orbit/specs/2026-04-20-v16-n1-email-regression/`.
#[test]
#[ignore]
fn golden_infer_n1_email_column() {
    let result = run_infer_json("john@example.com", "column");
    assert_eq!(
        result["label"].as_str(),
        Some("identity.person.email"),
        "N=1 email column should classify as identity.person.email \
         (regression guard from v16-era behaviour)"
    );
}

#[test]
#[ignore]
fn golden_infer_n1_url_column() {
    let result = run_infer_json("https://example.com/path", "column");
    assert_eq!(
        result["label"].as_str(),
        Some("technology.internet.url"),
        "N=1 URL column should classify as technology.internet.url"
    );
}

#[test]
#[ignore]
fn golden_infer_n1_ipv4_column() {
    let result = run_infer_json("192.168.1.1", "column");
    assert_eq!(
        result["label"].as_str(),
        Some("technology.internet.ip_v4"),
        "N=1 IPv4 column should classify as technology.internet.ip_v4"
    );
}

#[test]
#[ignore]
fn golden_schema_iso_date() {
    let schema = run_schema_json("datetime.date.iso");

    assert_eq!(schema["type"].as_str(), Some("string"));
    assert!(schema["pattern"].is_string());

    // v0.6.19 reduced schema surface — broad-type dropped. PII flag
    // (false here) is the only retained extension besides label.
    assert_eq!(
        schema["x-finetype-pii"].as_bool(),
        Some(false),
        "iso_date should not be marked as PII"
    );
    assert!(
        schema["x-finetype-broad-type"].is_null(),
        "x-finetype-broad-type was dropped from schema export in v0.6.19"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// PROFILE JSON SCHEMA OUTPUT — v0.6.19 (card 0003)
// ═══════════════════════════════════════════════════════════════════════════════
//
// `finetype profile -f <file> -o json-schema [--stats] [--enum-threshold N]`
// emits a table-level JSON Schema document to stdout. The output replaces
// the table-mode of the legacy `finetype schema <file.csv>` invocation
// (deletion ships with card 0006). Helper module:
// `crates/finetype-mcp/src/json_schema.rs`.

/// Run `finetype profile -f <path> -o json-schema [extra args]` and return parsed JSON.
fn run_profile_json_schema(csv_path: &Path, extra_args: &[&str]) -> Value {
    let mut args: Vec<&str> = vec![
        "run",
        "-p",
        "finetype-cli",
        "--",
        "profile",
        "-f",
        csv_path.to_str().unwrap(),
        "-o",
        "json-schema",
    ];
    args.extend_from_slice(extra_args);

    let output = Command::new("cargo")
        .args(&args)
        .current_dir(workspace_root())
        .output()
        .expect("failed to run finetype profile -o json-schema");

    assert!(
        output.status.success(),
        "profile -o json-schema failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("invalid utf8");
    serde_json::from_str(&stdout).unwrap_or_else(|e| {
        panic!("failed to parse profile json-schema output: {e}\nOutput: {stdout}");
    })
}

/// Round-trip parity stand-in (ac-10 degraded path): the helper output
/// must parse as JSON and satisfy the structural-shape contract — top-level
/// `$schema`, `type: object`, populated `properties`, plus the trimmed
/// `x-finetype-label` / `x-finetype-pii` extensions on at least one column.
///
/// When card 0005's `validate --schema -` (stdin) lands, this test can be
/// promoted to actual round-trip via piping. For v0.6.19 (cards 0003 →
/// 0006 → 0005), the structural assertion is the gate.
#[test]
#[ignore]
fn golden_profile_json_schema_people_directory() {
    let path = workspace_root().join("eval/datasets/csv/people_directory.csv");
    let schema = run_profile_json_schema(&path, &[]);

    assert!(
        schema["$schema"].is_string(),
        "json-schema output must declare $schema URI"
    );
    assert_eq!(
        schema["type"].as_str(),
        Some("object"),
        "json-schema output must be a JSON Schema object type"
    );
    assert!(
        schema["$id"].is_string(),
        "json-schema output must declare $id"
    );

    let properties = schema["properties"]
        .as_object()
        .expect("properties must be a JSON object");
    assert!(
        !properties.is_empty(),
        "people_directory should produce non-empty properties"
    );

    // At least one column property must carry both extensions from the
    // PR #51 verbosity contract.
    let has_label = properties
        .values()
        .any(|p| p.get("x-finetype-label").is_some());
    assert!(
        has_label,
        "at least one property must carry x-finetype-label"
    );
    let has_pii = properties
        .values()
        .any(|p| p.get("x-finetype-pii").is_some());
    assert!(has_pii, "at least one property must carry x-finetype-pii");

    // Negative assertion: the dropped extensions must NOT re-appear.
    for (col, prop) in properties.iter() {
        for dropped in [
            "x-finetype-broad-type",
            "x-finetype-transform",
            "x-finetype-transform-ext",
            "x-finetype-format-string",
            "x-finetype-domain",
            "x-finetype-confidence",
        ] {
            assert!(
                prop.get(dropped).is_none(),
                "{dropped} was dropped in v0.6.19 (column {col})"
            );
        }
    }

    // Without --stats, the diagnostic extensions must NOT appear. The
    // observed-data constraints `minLength`/`maxLength`/`minimum`/`maximum`/
    // `enum` are NOT in this list because validation contracts on the type
    // definition (e.g. `representation.discrete.categorical` carries
    // `minLength: 1, maxLength: 50`; `identity.person.gender` carries an
    // `enum`) inject those keywords from the type, not from observed data.
    for (col, prop) in properties.iter() {
        for stats_diagnostic in ["x-finetype-null-rate", "x-finetype-cardinality"] {
            assert!(
                prop.get(stats_diagnostic).is_none(),
                "{stats_diagnostic} should only appear with --stats (column {col})"
            );
        }
    }
}

/// `--stats` attaches observed-data constraints + diagnostic extensions.
#[test]
#[ignore]
fn golden_profile_json_schema_stats_ecommerce_orders() {
    let path = workspace_root().join("eval/datasets/csv/ecommerce_orders.csv");
    let schema = run_profile_json_schema(&path, &["--stats", "--enum-threshold", "50"]);

    let properties = schema["properties"]
        .as_object()
        .expect("properties must be a JSON object");
    assert!(!properties.is_empty(), "expected non-empty properties");

    // Every column property under --stats should carry the diagnostic
    // extensions, even when no string-length / numeric range applies.
    for (col, prop) in properties.iter() {
        assert!(
            prop.get("x-finetype-null-rate").is_some(),
            "{col} should carry x-finetype-null-rate under --stats"
        );
        assert!(
            prop.get("x-finetype-cardinality").is_some(),
            "{col} should carry x-finetype-cardinality under --stats"
        );
    }

    // At least one column should produce a string length range — the
    // ecommerce fixture has VARCHAR-shaped columns guaranteed to land
    // in the string branch.
    let any_min_length = properties
        .values()
        .any(|p| p.get("minLength").is_some() && p.get("maxLength").is_some());
    assert!(
        any_min_length,
        "at least one string column should produce minLength/maxLength under --stats"
    );
}

/// `--enum-threshold` controls whether the `enum` keyword is added by
/// `attach_stats` for low-cardinality columns. Validation contracts on
/// the type definition (e.g. `identity.person.gender`) may inject `enum`
/// independently — so the gate test verifies that increasing the
/// threshold can only ADD enum-bearing columns, never remove them, and
/// that threshold=50 produces at least one enum.
#[test]
#[ignore]
fn golden_profile_json_schema_enum_threshold_titanic() {
    let path = workspace_root().join("eval/datasets/csv/titanic.csv");

    // Threshold 0: only enums injected by type validation appear.
    let schema_off = run_profile_json_schema(&path, &["--stats", "--enum-threshold", "0"]);
    let props_off = schema_off["properties"]
        .as_object()
        .expect("properties object");
    let enums_off: std::collections::BTreeSet<String> = props_off
        .iter()
        .filter(|(_, p)| p.get("enum").is_some())
        .map(|(k, _)| k.clone())
        .collect();

    // Threshold 50: validation enums + stats-derived enums for any
    // low-cardinality column.
    let schema_on = run_profile_json_schema(&path, &["--stats", "--enum-threshold", "50"]);
    let props_on = schema_on["properties"]
        .as_object()
        .expect("properties object");
    let enums_on: std::collections::BTreeSet<String> = props_on
        .iter()
        .filter(|(_, p)| p.get("enum").is_some())
        .map(|(k, _)| k.clone())
        .collect();

    // Monotonicity: threshold=50 must contain every enum from threshold=0.
    assert!(
        enums_off.is_subset(&enums_on),
        "enum-bearing columns at threshold=0 must be a subset of those at threshold=50 \
         (off={enums_off:?}, on={enums_on:?})"
    );
    // Existence: at least one enum must appear at threshold=50.
    assert!(
        !enums_on.is_empty(),
        "at least one low-cardinality column should carry enum under --enum-threshold=50"
    );
}
