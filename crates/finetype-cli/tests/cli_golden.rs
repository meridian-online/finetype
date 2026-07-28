//! Golden integration tests for FineType CLI commands.
//!
//! These tests call the compiled binary and assert structured output.
//! All tests are `#[ignore]` because they load the model (~3s startup).
//!
//! Run with: `cargo test -p finetype-cli --test cli_golden -- --ignored`

use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

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

/// Run `finetype taxonomy <key> -o json-schema` and return the first
/// schema object from the always-array output.
///
/// The schema verb was retired in v0.6.19 (card 0006) — type-mode export
/// now lives on `taxonomy KEY -o json-schema`. Output is always a JSON
/// array; tests project to `[0]` to keep the per-schema assertion bodies
/// unchanged.
fn run_taxonomy_json_schema(type_key: &str) -> Value {
    let output = Command::new("cargo")
        .args([
            "run",
            "-p",
            "finetype-cli",
            "--",
            "taxonomy",
            type_key,
            "-o",
            "json-schema",
        ])
        .current_dir(workspace_root())
        .output()
        .expect("failed to run finetype taxonomy");

    assert!(
        output.status.success(),
        "taxonomy json-schema failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("invalid utf8");
    let array: Value = serde_json::from_str(&stdout).unwrap_or_else(|e| {
        panic!("failed to parse taxonomy json-schema output: {e}\nOutput: {stdout}");
    });
    let arr = array
        .as_array()
        .expect("taxonomy -o json-schema should always emit an array");
    assert!(
        !arr.is_empty(),
        "taxonomy {type_key} -o json-schema returned an empty array"
    );
    arr[0].clone()
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
    assert_column_type(&cols, "status", "representation.text.word");
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

    // Core assertions — the showstoppers that the hint expansion fixed
    assert_column_type(&cols, "Name", "identity.person.full_name");
    assert_column_type(&cols, "Survived", "representation.boolean.binary");
    assert_column_type(&cols, "Sex", "identity.person.gender");
    assert_column_type(&cols, "Fare", "finance.currency.amount");
    assert_column_type(&cols, "Embarked", "representation.text.word");

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
    assert_column_type(&cols, "job_title", "representation.text.word");
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
// LOAD GOLDEN TESTS — removed in v0.6.19 alongside `finetype load`.
//
// `cmd_load`'s standalone DDL emission is gone. The typed-CTAS shape it
// previously produced now lives inside `cmd_validate_table`'s materialise
// path (driven by `build_transform_projection`). End-to-end coverage for
// the typed-CTAS round trip lives at
// `crates/finetype-cli/tests/validate_cli.rs::test_vrp_typed_ctas_round_trip`,
// and the projection builder itself is unit-tested in `main.rs` (5 cases:
// VARCHAR pass-through, transform with TRY-wrap, transform without TRY-wrap,
// fallback CAST, unknown-label pass-through).
// ═══════════════════════════════════════════════════════════════════════════════

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

    // Should have 250 types
    assert_eq!(entries.len(), 250, "taxonomy should have 250 types");

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
// TAXONOMY JSON-SCHEMA GOLDEN TESTS — v0.6.19 (card 0006)
// ═══════════════════════════════════════════════════════════════════════════════
//
// Type-mode JSON Schema export migrated from the retired `schema KEY` verb to
// `taxonomy KEY -o json-schema`. Output is always a JSON array; tests project
// to `[0]` via run_taxonomy_json_schema().

#[test]
#[ignore]
fn golden_taxonomy_json_schema_email() {
    let schema = run_taxonomy_json_schema("identity.person.email");

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

    // FineType extension fields — v0.6.19 type-mode now carries BOTH
    // `x-finetype-label` and `x-finetype-pii` (matching table-mode's
    // verbosity contract from PR #51). The pre-existing `schema KEY`
    // verb only emitted `x-finetype-pii`; the migration to
    // `taxonomy KEY -o json-schema` adds the label extension. Other
    // derivable fields (broad-type, transform, format-string,
    // transform-ext, domain, confidence) remain dropped per PR #51.
    assert_eq!(
        schema["x-finetype-label"].as_str(),
        Some("identity.person.email"),
        "x-finetype-label should equal the queried key (added in v0.6.19)"
    );
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
/// Spec: `.orbit/specs/2026-04-20-v16-n1-email-regression/`.
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
fn golden_taxonomy_json_schema_iso_date() {
    let schema = run_taxonomy_json_schema("datetime.date.iso");

    assert_eq!(schema["type"].as_str(), Some("string"));
    assert!(schema["pattern"].is_string());

    // v0.6.19 type-mode carries label + pii both. Derivable fields
    // (broad-type, transform, format-string, transform-ext, domain,
    // confidence) remain dropped.
    assert_eq!(
        schema["x-finetype-label"].as_str(),
        Some("datetime.date.iso"),
        "x-finetype-label should equal the queried key (added in v0.6.19)"
    );
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

// ═══════════════════════════════════════════════════════════════════════════════
// `infer --mode column --batch --explain` — diagnostic cascade (NDJSON in/out)
// Subsumed the historical `infer-type` subcommand in MADR 0088.
// ═══════════════════════════════════════════════════════════════════════════════

/// Closed mechanism vocabulary per MADR 0075 + 0081. The cascade MUST emit
/// one of these tokens; any other value is a regression.
const CLOSED_MECHANISMS: &[&str] = &[
    "format_diversity_path_a",
    "format_diversity_path_b",
    "code_vs_canonical_path_a",
    "code_vs_canonical_path_b",
    "enum_overfit",
    "misclassification",
    "prediction_confirmed",
    "validator_widening",
    "unknown_no_fit",
    "fallthrough",
];

/// Run `finetype infer --mode column --batch --explain` with NDJSON-on-stdin.
/// Returns the parsed NDJSON output lines as a Vec<Value>.
fn run_infer_explain_batch(input_lines: &[&str]) -> Vec<Value> {
    let mut child = Command::new("cargo")
        .args([
            "run",
            "-p",
            "finetype-cli",
            "--",
            "infer",
            "--mode",
            "column",
            "--batch",
            "--explain",
        ])
        .current_dir(workspace_root())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn finetype infer --explain");
    {
        use std::io::Write as _;
        let stdin = child.stdin.as_mut().expect("piped stdin");
        for line in input_lines {
            writeln!(stdin, "{line}").expect("write stdin");
        }
    }
    let out = child
        .wait_with_output()
        .expect("failed waiting for finetype infer --explain");
    assert!(
        out.status.success(),
        "infer --explain failed (rc={:?}): {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).expect("invalid utf8");
    stdout
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            serde_json::from_str(l).unwrap_or_else(|e| panic!("output line not JSON: {e} ({l})"))
        })
        .collect()
}

/// Single canonical email column round-trips through the cascade as
/// `prediction_confirmed` (Sense was right). The smoke gate that fires from
/// `scripts/cron_preamble.sh` depends on this exact mechanism for the same
/// fixture — so this test also doubles as a regression guard on the cron
/// preamble's H05 halt path.
#[test]
#[ignore]
fn infer_explain_single_line_email_confirms() {
    let input = r#"{"column_name":"email","predicted_type":"identity.person.email","samples":["alice@example.com","bob@example.com","carol@example.com","dave@example.org","eve@example.net","frank@example.com","grace@example.io","henry@example.com"]}"#;
    let out = run_infer_explain_batch(&[input]);
    assert_eq!(out.len(), 1, "expected 1 output line, got {}", out.len());
    let r = &out[0];
    assert_eq!(r["inferred_correct_type"], "identity.person.email");
    assert_eq!(r["mechanism"], "prediction_confirmed");
    assert!(
        r["confidence"].as_f64().unwrap_or(0.0) >= 0.5,
        "confidence {:?} below 0.5 for canonical email column",
        r["confidence"]
    );
}

/// A three-line NDJSON stream produces a three-line NDJSON response in
/// the same order. Verifies the batch wire shape: one input → one output,
/// model + taxonomy loaded once for the whole stream.
#[test]
#[ignore]
fn infer_explain_batch_preserves_input_order() {
    let inputs = &[
        r#"{"column_name":"email","predicted_type":"identity.person.email","samples":["a@x.com","b@x.com","c@x.com"]}"#,
        r#"{"column_name":"age","predicted_type":"representation.numeric.integer","samples":["25","30","45"]}"#,
        r#"{"column_name":"weird","predicted_type":"identity.person.email","samples":["foo","bar","baz"]}"#,
    ];
    let out = run_infer_explain_batch(inputs);
    assert_eq!(out.len(), 3, "expected 3 output lines, got {}", out.len());
    for (i, r) in out.iter().enumerate() {
        let mech = r["mechanism"].as_str().expect("mechanism present");
        assert!(
            CLOSED_MECHANISMS.contains(&mech),
            "row {i}: mechanism {mech:?} not in closed 10-token set"
        );
        assert!(
            r.get("inferred_correct_type").is_some(),
            "row {i}: missing inferred_correct_type"
        );
        assert!(r.get("signals").is_some(), "row {i}: missing signals");
    }
}

/// `--explain` without `--batch` must be rejected at startup. The flag
/// requires NDJSON-on-stdin batch semantics by construction; allowing it
/// without `--batch` would silently load the model and then hang or
/// misinterpret single-line input.
#[test]
fn infer_explain_without_batch_is_rejected() {
    let out = Command::new("cargo")
        .args([
            "run",
            "-p",
            "finetype-cli",
            "--",
            "infer",
            "--mode",
            "column",
            "--explain",
        ])
        .current_dir(workspace_root())
        .stdin(Stdio::null())
        .output()
        .expect("failed to run finetype infer");
    assert!(
        !out.status.success(),
        "--explain without --batch must fail; got rc={:?}",
        out.status.code()
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("--explain requires --mode column --batch"),
        "expected guard message in stderr; got: {stderr}"
    );
}

/// `--explain` with `--mode row` is rejected. Without column semantics, the
/// cascade can't dispatch correctly. (Column is the default mode, so the
/// rejection only fires when row mode is requested explicitly.)
#[test]
fn infer_explain_without_column_mode_is_rejected() {
    let out = Command::new("cargo")
        .args([
            "run",
            "-p",
            "finetype-cli",
            "--",
            "infer",
            "--batch",
            "--explain",
            "--mode",
            "row",
        ])
        .current_dir(workspace_root())
        .stdin(Stdio::null())
        .output()
        .expect("failed to run finetype infer");
    assert!(
        !out.status.success(),
        "--explain without --mode column must fail; got rc={:?}",
        out.status.code()
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// `profile --files` — batch mode (model loads once per invocation)
// ═══════════════════════════════════════════════════════════════════════════════

/// Build a temp directory containing N small CSVs and a `paths.txt` listing
/// them. Returns (tmp_root, paths_file, out_dir). The tmp_root is the
/// caller's to keep alive as long as the files are needed.
fn build_batch_fixture(n: usize) -> (tempfile::TempDir, PathBuf, PathBuf) {
    let tmp = tempfile::tempdir().expect("tempdir");
    let in_dir = tmp.path().join("in");
    let out_dir = tmp.path().join("schemas");
    std::fs::create_dir_all(&in_dir).expect("mkdir in");
    let mut paths = Vec::with_capacity(n);
    for i in 0..n {
        let p = in_dir.join(format!("t{i}.csv"));
        std::fs::write(
            &p,
            format!(
                "email,age\nalice@example.com,{}\nbob@example.com,{}\n",
                i,
                i + 1
            ),
        )
        .expect("write csv");
        paths.push(p.to_string_lossy().to_string());
    }
    let paths_file = tmp.path().join("paths.txt");
    std::fs::write(&paths_file, paths.join("\n") + "\n").expect("write paths");
    (tmp, paths_file, out_dir)
}

/// Batch mode produces one output file per input. Each output is valid
/// JSON Schema with FineType extensions.
#[test]
#[ignore]
fn profile_files_batch_produces_one_output_per_input() {
    let (_tmp, paths, out_dir) = build_batch_fixture(3);
    let out = Command::new("cargo")
        .args([
            "run",
            "-p",
            "finetype-cli",
            "--",
            "profile",
            "--files",
            paths.to_str().unwrap(),
            "--out-dir",
            out_dir.to_str().unwrap(),
            "-o",
            "json-schema",
        ])
        .current_dir(workspace_root())
        .output()
        .expect("failed to run profile --files");
    assert!(
        out.status.success(),
        "profile --files failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    for i in 0..3 {
        let schema_path = out_dir.join(format!("t{i}.json"));
        assert!(
            schema_path.exists(),
            "expected {} to exist",
            schema_path.display()
        );
        let body = std::fs::read_to_string(&schema_path).expect("read schema");
        let v: Value = serde_json::from_str(&body)
            .unwrap_or_else(|e| panic!("schema {i} not JSON: {e}\n{body}"));
        assert!(
            v.get("properties").and_then(|p| p.as_object()).is_some(),
            "schema {i} missing properties: {body}"
        );
    }
    // Model-load amortisation evidence: the classifier-loading lines fire
    // once for the whole batch, not once per file. Sanity-check that we
    // saw ≤1 "Loaded multi-branch classifier" line in stderr regardless
    // of the 3-file batch size.
    let stderr = String::from_utf8_lossy(&out.stderr);
    let load_lines = stderr.matches("Loaded multi-branch classifier").count();
    assert!(
        load_lines <= 1,
        "expected ≤1 'Loaded multi-branch classifier' line across batch \
         of 3 (amortisation), got {load_lines}:\n{stderr}"
    );
}

/// `--files` without `--out-dir` is rejected at clap. Without an output
/// directory the per-file outputs have nowhere to land.
#[test]
fn profile_files_requires_out_dir() {
    let (_tmp, paths, _out_dir) = build_batch_fixture(1);
    let out = Command::new("cargo")
        .args([
            "run",
            "-p",
            "finetype-cli",
            "--",
            "profile",
            "--files",
            paths.to_str().unwrap(),
            "-o",
            "json-schema",
        ])
        .current_dir(workspace_root())
        .output()
        .expect("failed to run profile --files");
    assert!(
        !out.status.success(),
        "--files without --out-dir must fail; got rc={:?}",
        out.status.code()
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("out-dir"),
        "expected clap error mentioning --out-dir; got: {stderr}"
    );
}

/// Batch mode with non-`json-schema` output is rejected. The other format
/// branches still write to stdout via `println!` and would interleave
/// across files; refuse early until they're plumbed through the per-file
/// writer.
#[test]
fn profile_files_rejects_non_json_schema_output() {
    let (_tmp, paths, out_dir) = build_batch_fixture(1);
    let out = Command::new("cargo")
        .args([
            "run",
            "-p",
            "finetype-cli",
            "--",
            "profile",
            "--files",
            paths.to_str().unwrap(),
            "--out-dir",
            out_dir.to_str().unwrap(),
            "-o",
            "plain",
        ])
        .current_dir(workspace_root())
        .output()
        .expect("failed to run profile --files");
    assert!(
        !out.status.success(),
        "profile --files -o plain must fail; got rc={:?}",
        out.status.code()
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("json-schema"),
        "expected clap error mentioning json-schema; got: {stderr}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// COMPACT-DATE PRECISION — the eight-digit-figure false positive
// ═══════════════════════════════════════════════════════════════════════════════

/// Pull one column's full emitted record out of a `profile -o json` document.
fn column_record<'a>(profile: &'a Value, column_name: &str) -> &'a Value {
    profile["columns"]
        .as_array()
        .expect("profile missing columns array")
        .iter()
        .find(|c| c["column"].as_str() == Some(column_name))
        .unwrap_or_else(|| panic!("column '{column_name}' not found in profile"))
}

/// `datetime.date.compact_ymd` must separate a real YYYYMMDD date column from
/// the eight-digit figures and surrogate keys that sit beside it.
///
/// The fixture is a financial-statement shape taken from real corpus columns:
/// a `date` column of genuine YYYYMMDD values, three balance-sheet figures
/// (`commonStock`, `researchDevelopment`, `longTermDebt`), a constant
/// `marketCap`, and an eight-digit `game_id`. Under a shape-only `^\d{8}$`
/// validator every one of these validates as a date at 100%, so the hard
/// validation veto has nothing to push back with and the figures ship as
/// confident dates with a `strptime` transform attached.
///
/// The assertions read the WHOLE emitted record — type, broad_type,
/// validation_pass_rate, quality_band, disambiguation_rule, format_string,
/// transform — because a label-only check cannot tell a fixed column from one
/// that kept its date transform.
#[test]
#[ignore]
fn golden_profile_compact_ymd_rejects_eight_digit_figures() {
    let profile = run_profile_json(&fixture_path("compact_ymd_vs_eight_digit_figures.csv"));

    // The genuine date column is untouched: still a date, still validating at
    // 100%, still carrying its format and transform, no rule fired.
    let date = column_record(&profile, "date");
    assert_eq!(date["type"], "datetime.date.compact_ymd");
    assert_eq!(date["broad_type"], "DATE");
    assert_eq!(date["validation_pass_rate"], 1.0);
    assert_eq!(date["quality_band"], "high");
    assert_eq!(date["format_string"], "%Y%m%d");
    assert_eq!(date["transform"], "strptime({col}, '%Y%m%d')::DATE");
    assert!(
        date["disambiguation_rule"].is_null(),
        "the real date column must not need a rule to survive; got {:?}",
        date["disambiguation_rule"]
    );

    // The balance-sheet figures are hard-vetoed off the date label. The
    // validation pass rate is the load-bearing field: it is what the veto
    // reads, and a shape-only validator reports 1.0 here.
    for name in ["commonStock", "researchDevelopment"] {
        let col = column_record(&profile, name);
        assert_eq!(
            col["type"], "representation.numeric.integer_number",
            "{name} should type as an integer, not a date"
        );
        assert_eq!(col["broad_type"], "BIGINT", "{name} broad_type");
        assert_eq!(
            col["validation_pass_rate"], 0.0,
            "{name} must FAIL the compact_ymd validator, not pass it at 1.0"
        );
        assert_eq!(
            col["disambiguation_rule"], "veto_fallback:vocab",
            "{name} should reach its type through the validation veto"
        );
        assert!(
            col["format_string"].is_null() && col["transform"] != "strptime({col}, '%Y%m%d')::DATE",
            "{name} must not keep a date transform: {:?} / {:?}",
            col["format_string"],
            col["transform"]
        );
    }

    // Nothing else in the table may claim to be a compact date.
    let stray: Vec<&str> = profile["columns"]
        .as_array()
        .expect("columns array")
        .iter()
        .filter(|c| {
            c["type"]
                .as_str()
                .is_some_and(|t| t.starts_with("datetime.date.compact_"))
        })
        .filter_map(|c| c["column"].as_str())
        .filter(|n| *n != "date")
        .collect();
    assert!(
        stray.is_empty(),
        "only the real date column may type as a compact date; also got {stray:?}"
    );
}

/// The other half of the contract: a genuine NINETEENTH-CENTURY YYYYMMDD column
/// must keep its date type, its format string and its `strptime` transform.
///
/// The first revision of this change narrowed the year to `(19|20)\d{2}`, and a
/// reviewer refuted it through this exact table: both columns dropped from
/// `datetime.date.compact_ymd` (pass rate 1.0, transform attached) to
/// `representation.numeric.integer_number` at pass rate 0.0 with rule
/// `veto_fallback:vocab`, format string null and the date transform stripped.
/// Genuine dates shipping as confident integers is the same defect the
/// tightening exists to stop, so the year field carries no window and this test
/// is what says so at the CLI boundary. Reads the whole emitted record, because
/// a label-only check cannot see a stripped transform.
#[test]
#[ignore]
fn golden_profile_compact_ymd_accepts_nineteenth_century_dates() {
    let profile = run_profile_json(&fixture_path("compact_ymd_historical_dates.csv"));

    for name in ["date", "FirstAddedDate"] {
        let col = column_record(&profile, name);

        // The WHOLE emitted record, field for field, minus `confidence` — that
        // one is a raw model score and is asserted separately as a finite
        // probability rather than pinned to a literal. `quality_band` is `low`
        // because an eight-row two-column table gives the model little to go
        // on; it is pinned because a stripped record changes it too, and this
        // test exists to notice any field moving.
        let mut expected = serde_json::Map::new();
        expected.insert("column".into(), json!(name));
        expected.insert("type".into(), json!("datetime.date.compact_ymd"));
        expected.insert("broad_type".into(), json!("DATE"));
        expected.insert("validation_pass_rate".into(), json!(1.0));
        expected.insert("quality_band".into(), json!("low"));
        expected.insert("format_string".into(), json!("%Y%m%d"));
        expected.insert("transform".into(), json!("strptime({col}, '%Y%m%d')::DATE"));
        expected.insert("is_generic".into(), json!(false));
        expected.insert("non_null".into(), json!(8));
        expected.insert("null".into(), json!(0));
        expected.insert("samples_used".into(), json!(8));

        let actual: serde_json::Map<String, Value> = col
            .as_object()
            .expect("column record is an object")
            .iter()
            .filter(|(k, _)| k.as_str() != "confidence")
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        assert_eq!(
            actual, expected,
            "{name}: an 1865-1872 YYYYMMDD column must emit the same record a 20th-century one \
             does. Under a `(19|20)\\d{{2}}` year window this becomes \
             representation.numeric.integer_number at pass rate 0.0, rule veto_fallback:vocab, \
             format_string null and the strptime transform stripped."
        );

        let confidence = col["confidence"].as_f64().unwrap_or_else(|| {
            panic!(
                "{name}: confidence must be a number, got {:?}",
                col["confidence"]
            )
        });
        assert!(
            confidence.is_finite() && confidence > 0.0 && confidence <= 1.0,
            "{name}: confidence must be a real probability, got {confidence}"
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// COMPACT-DATE PRECISION — the DAY-FIRST leaf, which shipped a confident wrong
// date for as long as it has existed
//
// These six columns are one per row of the residual table in
// `docs/compact-date-residual.tsv`, and they are the reason that table exists:
// the corpus-honest gate certified the year-first tightening and could not see
// this. Its sample is ~3% of GitTables, it is non-adversarial, and it scores
// label transitions in AGGREGATE — while the defect here is one column family
// moving from an integer to a high-confidence date with a `strptime` transform
// attached. A gate verdict is not coverage.
//
// Every one of these asserts the FULL emitted record. A label-only assertion
// sails straight past the defect, because the damage is the `format_string` and
// `transform` keys riding along with the label.
// ═══════════════════════════════════════════════════════════════════════════════

/// Assert a column's WHOLE emitted record, field for field, minus `confidence`.
///
/// `confidence` is a raw model score and is checked separately as a finite
/// probability rather than pinned to a literal. Everything else is pinned —
/// including `quality_band`, `is_generic` and the counts — because this family
/// of tests exists to notice ANY field moving, and a label-only check cannot
/// tell a fixed column from one that kept its date transform.
fn assert_full_record(profile: &Value, column_name: &str, expected: Value, why: &str) {
    let col = column_record(profile, column_name);
    let actual: serde_json::Map<String, Value> = col
        .as_object()
        .expect("column record is an object")
        .iter()
        .filter(|(k, _)| k.as_str() != "confidence")
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    assert_eq!(
        Value::Object(actual),
        expected,
        "{column_name}: {why}"
    );
    let confidence = col["confidence"].as_f64().unwrap_or_else(|| {
        panic!(
            "{column_name}: confidence must be a number, got {:?}",
            col["confidence"]
        )
    });
    assert!(
        confidence.is_finite() && confidence > 0.0 && confidence <= 1.0,
        "{column_name}: confidence must be a real probability, got {confidence}"
    );
}

/// ROW 1 — the exact values the year-first tightening was written for.
///
/// The twenty corpus values in `compact_ymd`'s REJECT set. Measured through the
/// CLI on four sides (`docs/compact-date-residual.tsv`), this column typed
/// `datetime.date.compact_dmy` at 0.9878, quality band `high`, carrying
/// `format_string` `%d%m%Y` and `transform` `strptime({col}, '%d%m%Y')::DATE`
/// on the RELEASED binary, and 0.9064 with the same label and the same
/// transform after the year-first tightening. Twenty eight-digit financial
/// figures, shipped as confident dates, on both.
///
/// Neither the label nor either of those two keys may reappear.
#[test]
#[ignore]
fn golden_profile_compact_dmy_rejects_the_year_first_reject_set() {
    let profile = run_profile_json(&fixture_path("compact_dmy_ymd_reject_set.csv"));
    assert_full_record(
        &profile,
        "value",
        json!({
            "column": "value",
            "type": "representation.numeric.integer_number",
            "broad_type": "BIGINT",
            "quality_band": "low",
            "runner_up": "datetime.component.year",
            "transform": "CAST({col} AS BIGINT)",
            "disambiguation_applied": true,
            "disambiguation_rule": "veto_fallback:vocab",
            "is_generic": true,
            "validation_pass_rate": 0.0,
            "validation_vetoed": true,
            "vetoed_type": "datetime.component.year",
            "samples_used": 20,
            "non_null": 20,
            "null": 0,
        }),
        "under a shape-only day-first validator this column is \
         datetime.date.compact_dmy at 0.9878 released / 0.9064 on main, quality \
         band high, carrying format_string %d%m%Y and transform \
         strptime({col}, '%d%m%Y')::DATE. Neither key may reappear.",
    );
}

/// ROW 2 — sequential eight-digit post ids.
#[test]
#[ignore]
fn golden_profile_compact_dmy_rejects_sequential_ids() {
    let profile = run_profile_json(&fixture_path("compact_dmy_sequential_ids.csv"));
    assert_full_record(
        &profile,
        "PostId",
        json!({
            "column": "PostId",
            "type": "representation.identifier.increment",
            "broad_type": "BIGINT",
            "quality_band": "low",
            "runner_up": "representation.numeric.integer_number",
            "transform": "CAST({col} AS BIGINT)",
            "disambiguation_applied": true,
            "disambiguation_rule": "numeric_sequential_detection",
            "is_generic": true,
            "validation_pass_rate": 1.0,
            "samples_used": 8,
            "non_null": 8,
            "null": 0,
        }),
        "a run of consecutive surrogate keys typed datetime.date.compact_dmy \
         at 0.8341 released / 0.8110 on main, with a %d%m%Y strptime transform, \
         under the shape-only validator. validation_pass_rate here is the \
         INCREMENT label's, not the date label's — the date label is gone.",
    );
}

/// ROW 3 — round-hundred share counts. THE ALLOWLIST'S OWN CASE.
///
/// This is the row the tightened pattern ALONE does not fix, and the reason the
/// allowlist line is half the change rather than tidying.
///
/// `datetime.date.compact_dmy` was absent from `labels/veto_safe.txt` while
/// both its siblings were on it, so its validator's verdict was ADVISORY: the
/// profile path computed the pass rate and then let the label stand, `strptime`
/// transform and all. Tighten the pattern and leave the allowlist alone and
/// this column still emits `datetime.date.compact_dmy` at high confidence with
/// `validation_pass_rate` 0.0 and `validation_advisory_low` true — the
/// validator rejects every value in the column and nothing acts on the
/// rejection. A validator whose verdict nothing acts on is not a validator.
///
/// That is not an argument, it is a row of
/// `docs/compact-dmy-mutation-matrix.md`: the `drop_allowlist_entry` mutation
/// leaves both windows intact, removes only the allowlist line, and kills this
/// test. `revert_whole_change` kills it too. Both halves are load bearing and
/// the matrix is what says so.
#[test]
#[ignore]
fn golden_profile_compact_dmy_vetoes_round_hundred_share_counts() {
    let profile = run_profile_json(&fixture_path(
        "compact_dmy_round_hundred_share_counts.csv",
    ));
    assert_full_record(
        &profile,
        "sharesOutstanding",
        json!({
            "column": "sharesOutstanding",
            "type": "representation.numeric.integer_number",
            "broad_type": "BIGINT",
            "quality_band": "high",
            "transform": "CAST({col} AS BIGINT)",
            "disambiguation_applied": true,
            "disambiguation_rule": "veto_fallback:vocab",
            "is_generic": false,
            "validation_pass_rate": 0.0,
            "validation_vetoed": true,
            "vetoed_type": "datetime.date.compact_dmy",
            "samples_used": 8,
            "non_null": 8,
            "null": 0,
        }),
        "`validation_vetoed` true with `vetoed_type` datetime.date.compact_dmy \
         is the whole point: the HARD veto fired. If this record instead shows \
         type datetime.date.compact_dmy with validation_advisory_low true, the \
         allowlist entry has been dropped and the validator's verdict is being \
         computed and discarded.",
    );
}

/// ROW 4 — unconstrained eight-digit numbers. Neither side may claim these.
#[test]
#[ignore]
fn golden_profile_compact_dmy_leaves_unconstrained_eight_digit_alone() {
    let profile = run_profile_json(&fixture_path(
        "compact_dmy_unconstrained_eight_digit.csv",
    ));
    assert_full_record(
        &profile,
        "value",
        json!({
            "column": "value",
            "type": "representation.numeric.integer_number",
            "broad_type": "BIGINT",
            "quality_band": "low",
            "transform": "CAST({col} AS BIGINT)",
            "disambiguation_applied": true,
            "disambiguation_rule": "increment_substance_veto",
            "is_generic": true,
            "validation_pass_rate": 1.0,
            "samples_used": 8,
            "non_null": 8,
            "null": 0,
        }),
        "this column is identical on both sides of the change and is here to \
         catch a tightening that starts moving columns it was never aimed at.",
    );
}

/// ROW 5 — genuine YYYYMMDD. The year-first leaf must be untouched.
///
/// Tightening a validator moves a MODEL INPUT: the per-label pass-rate vector
/// feeds the multi-branch model's validation branch, so a day-first edit can
/// reach a year-first column. This row is the guard against that.
#[test]
#[ignore]
fn golden_profile_compact_dmy_change_leaves_genuine_ymd_dates_alone() {
    let profile = run_profile_json(&fixture_path("compact_dmy_genuine_ymd_dates.csv"));
    assert_full_record(
        &profile,
        "date",
        json!({
            "column": "date",
            "type": "datetime.date.compact_ymd",
            "broad_type": "DATE",
            "quality_band": "high",
            "format_string": "%Y%m%d",
            "transform": "strptime({col}, '%Y%m%d')::DATE",
            "is_generic": false,
            "validation_pass_rate": 1.0,
            "samples_used": 8,
            "non_null": 8,
            "null": 0,
        }),
        "a real YYYYMMDD date column must keep its label, its format string \
         and its strptime transform. A day-first validator edit that reaches \
         this column has moved the model off data it was never aimed at.",
    );
}

/// ROW 6 — genuine DD-MM-YYYY dates, including three from before 1900.
///
/// The accept side. The tightening must not make this column worse, and it does
/// not: the record is byte-identical on both sides of the change.
///
/// It also records something the corpus says and this fixture demonstrates: the
/// engine does not type genuine day-first compact dates as
/// `datetime.date.compact_dmy` AT ALL — here the column is vetoed off
/// `datetime.timestamp.iso_8601` and emitted as `unknown`. In a 33,250-table
/// pass the 972 columns typed `compact_dmy` are headed `game_id` (218), `id`
/// (100), `minorityInterest` (58), `pos` (49), `trials` (33) — surrogate keys
/// and financial figures. The label's real-world population in this corpus is
/// zero, which is why tightening it costs nothing; the corpus is not date-poor
/// either, holding 1,987 day-first columns on the separator-bearing leaves.
/// If this record ever becomes `datetime.date.compact_dmy` at pass rate 1.0
/// that is an IMPROVEMENT — update the expectation and delete this paragraph.
#[test]
#[ignore]
fn golden_profile_compact_dmy_change_leaves_genuine_day_first_dates_alone() {
    let profile = run_profile_json(&fixture_path(
        "compact_dmy_genuine_day_first_dates.csv",
    ));
    assert_full_record(
        &profile,
        "date",
        json!({
            "column": "date",
            "type": "unknown",
            "broad_type": "—",
            "quality_band": "medium",
            "disambiguation_applied": true,
            "disambiguation_rule": "header_hint_cross_domain:date",
            "is_generic": false,
            "validation_pass_rate": 0.0,
            "validation_vetoed": true,
            "vetoed_type": "datetime.timestamp.iso_8601",
            "x-finetype-unknown-reason":
                "validation rejected 'iso_8601': only 0% of values matched its format",
            "samples_used": 8,
            "non_null": 8,
            "null": 0,
        }),
        "identical on both sides of the change. The day-first tightening must \
         not touch a column of genuine day-first dates — and a century year \
         window would, which is what a reviewer proved on the year-first leaf \
         with 1865-1872 values.",
    );
}
