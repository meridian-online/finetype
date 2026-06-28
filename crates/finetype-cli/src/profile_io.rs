//! Profile I/O helpers: JSON/CSV input reading and JSON output reconstruction.

use super::*;

// ═══════════════════════════════════════════════════════════════════════════════
// PROFILE HELPERS — JSON/CSV input reading and JSON output reconstruction
// ═══════════════════════════════════════════════════════════════════════════════

/// Read JSON or NDJSON input into (headers, columns, row_count).
pub(crate) fn read_json_input(
    file: &std::path::Path,
    ext: &str,
) -> Result<(Vec<String>, Vec<Vec<String>>, usize)> {
    use finetype_core::json_reader;

    if ext == "json" {
        let content = std::fs::read_to_string(file)?;
        let value: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| anyhow::anyhow!("Malformed JSON in {:?}: {}", file, e))?;

        match &value {
            serde_json::Value::Array(arr) => {
                // Top-level array → treat as multi-row
                let mut all_paths: indexmap::IndexMap<String, Vec<Option<String>>> =
                    indexmap::IndexMap::new();
                let row_count = arr.len();
                for item in arr {
                    let item_map = json_reader::collect_json(item);
                    for (path, values) in item_map.all_paths() {
                        let entry = all_paths.entry(path.clone()).or_default();
                        entry.extend(values.iter().cloned());
                    }
                    // Fill missing paths with None
                    for (path, values) in &mut all_paths {
                        if !item_map.all_paths().contains_key(path) {
                            values.push(None);
                        }
                    }
                }
                let headers: Vec<String> = all_paths.keys().cloned().collect();
                let columns: Vec<Vec<String>> = all_paths
                    .values()
                    .map(|vals| {
                        vals.iter()
                            .filter_map(|v| v.clone())
                            .filter(|v| !v.is_empty())
                            .collect()
                    })
                    .collect();
                eprintln!(
                    "Found {} paths across {} array elements",
                    headers.len(),
                    row_count
                );
                Ok((headers, columns, row_count))
            }
            serde_json::Value::Object(_) => {
                let path_map = json_reader::collect_json(&value);
                let headers: Vec<String> = path_map.paths().cloned().collect();
                let columns: Vec<Vec<String>> = headers
                    .iter()
                    .map(|h| {
                        path_map
                            .get(h)
                            .map(|vals| {
                                vals.iter()
                                    .filter_map(|v| v.clone())
                                    .filter(|v| !v.is_empty())
                                    .collect()
                            })
                            .unwrap_or_default()
                    })
                    .collect();
                let row_count = path_map.row_count();
                eprintln!("Found {} paths in single JSON document", headers.len());
                Ok((headers, columns, row_count))
            }
            _ => {
                anyhow::bail!(
                    "JSON input must be an object or array of objects, got scalar value in {:?}",
                    file
                );
            }
        }
    } else {
        // NDJSON/JSONL: read line by line
        let reader = std::fs::File::open(file)?;
        let path_map = json_reader::collect_ndjson(reader)
            .map_err(|e| anyhow::anyhow!("Error reading NDJSON from {:?}: {}", file, e))?;

        let headers: Vec<String> = path_map.paths().cloned().collect();
        let columns: Vec<Vec<String>> = headers
            .iter()
            .map(|h| {
                path_map
                    .get(h)
                    .map(|vals| {
                        vals.iter()
                            .filter_map(|v| v.clone())
                            .filter(|v| !v.is_empty())
                            .collect()
                    })
                    .unwrap_or_default()
            })
            .collect();
        let row_count = path_map.row_count();
        eprintln!(
            "Found {} paths across {} NDJSON documents",
            headers.len(),
            row_count
        );
        Ok((headers, columns, row_count))
    }
}

/// Tokens treated as null-ish and dropped from a column's value list,
/// matching the historical csv-crate reader's filter (empty + these seven).
const NULLISH_TOKENS: [&str; 7] = ["NULL", "null", "NA", "N/A", "nan", "NaN", "None"];

/// True when `trimmed` is the empty string or one of the null-ish tokens.
fn is_nullish(trimmed: &str) -> bool {
    trimmed.is_empty() || NULLISH_TOKENS.contains(&trimmed)
}

/// Read CSV (or Parquet) input into (headers, columns, row_count) by shelling
/// out to the external `duckdb` CLI (choice 0100). This replaces the bespoke
/// csv-crate reader: DuckDB's parallel CSV sniffer handles dialect detection,
/// quoting, and ragged rows, while Parquet is read by the same path with every
/// column cast to VARCHAR.
///
/// DuckDB reads the file (`read_csv(auto_detect, all_varchar, null_padding)` or
/// `read_parquet` with a VARCHAR cast) and re-emits it as a canonical CSV on
/// stdout via the `-csv` output mode (cross-platform; no Unix-only
/// `/dev/stdout`). We parse that canonical CSV with
/// the `csv` crate — quoting/escaping is well-defined, so no value is mangled —
/// and apply the SAME null-ish filtering the old reader did (drop empty strings
/// and the tokens NULL/null/NA/N/A/nan/NaN/None) into per-column `Vec<String>`.
///
/// HARD DEPENDENCY: `duckdb` must be on PATH (ac-02). When it is absent we fail
/// with the same actionable error shape validate already uses.
pub(crate) fn read_csv_input(
    file: &std::path::Path,
    delimiter: Option<char>,
) -> Result<(Vec<String>, Vec<Vec<String>>, usize)> {
    let is_parquet = file
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.eq_ignore_ascii_case("parquet"))
        .unwrap_or(false);

    let input_literal = crate::sql::sql_quote(&file.to_string_lossy());

    // Build the SELECT source. Parquet: cast every column to VARCHAR so the
    // engine sees the same VARCHAR cells the CSV path produces (matches
    // validate.rs's `COLUMNS(*)::VARCHAR` contract). CSV: auto-detect the
    // dialect (honouring an explicit delimiter when given) with `all_varchar`
    // + `null_padding` — short ragged rows are padded with NULLs, the duckdb
    // analogue of the csv crate's `flexible(true)`.
    // Build the SELECT source. Parquet has no fallback; CSV keeps its `opts`
    // string so we can retry single-threaded on a parallel-scanner failure.
    let (source, csv_opts) = if is_parquet {
        (
            format!("SELECT COLUMNS(*)::VARCHAR FROM read_parquet({input_literal})"),
            None,
        )
    } else {
        let mut opts = String::from("auto_detect=true, all_varchar=true, null_padding=true");
        if let Some(delim) = delimiter {
            // Render the delimiter as a SQL literal so quotes/backslashes are
            // escaped safely; an explicit delimiter pins the sniffer.
            opts.push_str(", sep=");
            opts.push_str(&crate::sql::sql_quote(&delim.to_string()));
        }
        (
            format!("SELECT * FROM read_csv({input_literal}, {opts})"),
            Some(opts),
        )
    };

    // Re-emit as canonical CSV on the child's stdout via duckdb's `-csv` output
    // mode, then read it back with the csv crate. `-csv` is cross-platform (no
    // `/dev/stdout`, which is Unix-only and would break the Windows path that is
    // the whole point of choice 0100); duckdb writes RFC-4180 CSV — header row,
    // comma-delimited, quoted where needed — straight to stdout.
    //
    // `.nullvalue ''` is pinned as a leading command so a genuine NULL (an empty
    // cell, or a short row padded by `null_padding`) always renders as the empty
    // string — which `is_nullish` drops. WITHOUT this the null rendering would
    // inherit the user's `~/.duckdbrc` `.nullvalue`: if it were set to a token
    // outside NULLISH_TOKENS (e.g. "\\N"), a real NULL would survive as a kept
    // value and corrupt the column. Pinning it makes ingestion environment-
    // independent.
    let run_duckdb = |source: &str| -> Result<std::process::Output> {
        let query = format!("{source};");
        std::process::Command::new("duckdb")
            .arg("-csv")
            .arg("-c")
            .arg(".nullvalue ''")
            .arg("-c")
            .arg(&query)
            .output()
            .map_err(|e| {
                anyhow::anyhow!(
                    "could not invoke duckdb CLI (is duckdb on PATH?): {e}. \
                     Install it from https://duckdb.org/docs/installation"
                )
            })
    };

    let mut out = run_duckdb(&source)?;

    // duckdb's PARALLEL CSV scanner rejects `null_padding` combined with a
    // quoted field containing a newline ("parallel scanner does not support
    // null_padding in conjunction with quoted new lines"). Its own remedy is
    // `parallel=false`, so on a first-pass read failure we retry single-threaded
    // before giving up: clean files keep the fast parallel path, while
    // ragged/quoted files now parse rather than aborting the profile (and, in
    // batch mode, taking the rest of the run down with them).
    if !out.status.success() {
        if let Some(opts) = csv_opts {
            let retry = format!("SELECT * FROM read_csv({input_literal}, {opts}, parallel=false)");
            out = run_duckdb(&retry)?;
        }
    }
    if !out.status.success() {
        anyhow::bail!(
            "duckdb failed to read {:?}: {}",
            file,
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }

    // Parse the canonical CSV duckdb emitted. It is well-formed (quoted where
    // needed, comma-delimited, single header row), so the csv crate reads it
    // back losslessly.
    let mut reader = csv::ReaderBuilder::new().from_reader(out.stdout.as_slice());

    let headers: Vec<String> = reader.headers()?.iter().map(|h| h.to_string()).collect();
    let n_cols = headers.len();
    eprintln!("Found {} columns: {:?}", n_cols, headers);

    let mut columns: Vec<Vec<String>> = vec![Vec::new(); n_cols];
    let mut row_count = 0;

    for result in reader.records() {
        let record = result?;
        row_count += 1;
        for (i, field) in record.iter().enumerate() {
            if i < n_cols {
                let trimmed = field.trim();
                if !is_nullish(trimmed) {
                    columns[i].push(trimmed.to_string());
                }
            }
        }
    }

    Ok((headers, columns, row_count))
}

/// Extract the leaf component from a JSON path for use as header hint.
/// "users[].address.city" → "city"
/// "users[]" → "users"
/// "email" → "email"
pub(crate) fn path_leaf(path: &str) -> String {
    // Remove trailing [] brackets
    let clean = path.trim_end_matches("[]");
    // Take the last component after dot
    if let Some(pos) = clean.rfind('.') {
        clean[pos + 1..].to_string()
    } else {
        clean.to_string()
    }
}

/// Reconstruct a nested JSON schema from flat path profiles.
/// Converts flat paths like "users[].address.city" into nested structure.
pub(crate) fn reconstruct_json_schema(
    profiles: &[(String, String, Option<String>, f32)],
) -> serde_json::Value {
    let mut root = serde_json::Map::new();

    for (name, label, broad_type, confidence) in profiles {
        if label == "unknown" {
            continue;
        }

        let type_info = {
            let mut obj = serde_json::Map::new();
            obj.insert("type".to_string(), json!(label));
            if let Some(bt) = broad_type {
                obj.insert("broad_type".to_string(), json!(bt));
            }
            obj.insert(
                "confidence".to_string(),
                json!(format!("{:.1}%", confidence * 100.0)),
            );
            serde_json::Value::Object(obj)
        };

        insert_path(&mut root, name, type_info);
    }

    serde_json::Value::Object(root)
}

/// Insert a type_info value at a nested path within a JSON map.
/// Handles both dot notation (a.b) and array notation (a[]).
pub(crate) fn insert_path(
    root: &mut serde_json::Map<String, serde_json::Value>,
    path: &str,
    value: serde_json::Value,
) {
    let parts: Vec<&str> = path.split('.').collect();

    if parts.len() == 1 {
        let key = parts[0];
        if let Some(name) = key.strip_suffix("[]") {
            let entry = root
                .entry(name.to_string())
                .or_insert_with(|| json!({"_array": true}));
            if let serde_json::Value::Object(obj) = entry {
                obj.insert("_items".to_string(), value);
            }
        } else {
            root.insert(key.to_string(), value);
        }
        return;
    }

    let key = parts[0];
    let rest = parts[1..].join(".");

    if let Some(name) = key.strip_suffix("[]") {
        let entry = root
            .entry(name.to_string())
            .or_insert_with(|| json!({"_array": true}));
        if let serde_json::Value::Object(obj) = entry {
            obj.insert("_array".to_string(), json!(true));
            let items = obj
                .entry("_items".to_string())
                .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
            if let serde_json::Value::Object(items_map) = items {
                insert_path(items_map, &rest, value);
            }
        }
    } else {
        let entry = root
            .entry(key.to_string())
            .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
        if let serde_json::Value::Object(obj) = entry {
            insert_path(obj, &rest, value);
        }
    }
}
