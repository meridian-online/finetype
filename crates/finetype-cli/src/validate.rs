//! `validate` command: table validation against a schema with reject sidecar.

use super::*;

pub(crate) fn cmd_validate_table(
    file: PathBuf,
    schema_path: PathBuf,
    db: Option<PathBuf>,
    table: Option<String>,
    append: bool,
    lenient: bool,
    output: OutputFormat,
) -> Result<()> {
    use finetype_core::table_validator::validate_table;

    // ── (1) Load + parse schema, fail-fast per ac-08 ─────────────────────────
    eprintln!("Loading schema from {}", schema_path.display());
    let schema = load_schema_or_exit(&schema_path);
    let extensions = SchemaExtensions::extract(&schema);

    // ── Input file existence check (exit 2 if missing) ───────────────────────
    if !file.exists() {
        eprintln!("error: input file not found: {}", file.display());
        exit_with(2);
    }

    // ── Mode selection: check-only when --db is absent, materialise when present ─
    //    `clap`'s `requires` cross-references guarantee that `db` and
    //    `table` are either both supplied or both omitted, and that
    //    `--append` is only ever set with `--db` present. The conditional
    //    `if let` blocks below are the runtime expression of that contract.

    // ── Pre-flight: staging-collision gate (ac-09) — only when materialising ─
    //    If the user's target table exists and --append was not supplied,
    //    refuse with exit 2. `--append` implies explicit acceptance of an
    //    existing .db.
    if let (Some(db_path), Some(table_name)) = (db.as_ref(), table.as_ref()) {
        if !append && user_table_exists(db_path, table_name) {
            eprintln!(
                "error: table '{}' already exists in {} — pass --append to reuse",
                table_name,
                db_path.display()
            );
            exit_with(2);
        }
    }

    // ── Read input into memory (CSV-or-Parquet) ──────────────────────────────
    //    Parquet inputs are streamed through DuckDB's read_parquet to a
    //    temp CSV, then read by the same csv::Reader the CSV path uses —
    //    so the validation engine sees the same normalised Option<String>
    //    cells regardless of source. Column names flow through verbatim
    //    (DuckDB COPY preserves parquet schema names), matching what the
    //    materialise path's read_parquet binding sees later.
    eprintln!("Reading {}", file.display());
    let is_parquet = file
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.eq_ignore_ascii_case("parquet"))
        .unwrap_or(false);
    // Keep the temp file alive for the duration of the read (path borrowed below).
    let _parquet_csv_tmp: Option<tempfile::NamedTempFile>;
    let read_path: PathBuf = if is_parquet {
        let tmp = match tempfile::Builder::new()
            .prefix("finetype-validate-parquet-")
            .suffix(".csv")
            .tempfile()
        {
            Ok(t) => t,
            Err(e) => {
                eprintln!("error: could not create temp file for parquet→csv: {}", e);
                exit_with(2);
            }
        };
        let copy_sql = format!(
            "COPY (SELECT * FROM read_parquet({})) TO {} (HEADER, DELIMITER ',', QUOTE '\"');",
            sql_quote(&file.to_string_lossy()),
            sql_quote(&tmp.path().to_string_lossy()),
        );
        let out = std::process::Command::new("duckdb")
            .arg("-c")
            .arg(&copy_sql)
            .output();
        match out {
            Ok(o) if o.status.success() => {}
            Ok(o) => {
                eprintln!(
                    "error: duckdb parquet→csv failed: {}",
                    String::from_utf8_lossy(&o.stderr).trim()
                );
                exit_with(2);
            }
            Err(e) => {
                eprintln!(
                    "error: could not invoke duckdb CLI (is duckdb on PATH?): {}",
                    e
                );
                exit_with(2);
            }
        }
        let path = tmp.path().to_path_buf();
        _parquet_csv_tmp = Some(tmp);
        path
    } else {
        _parquet_csv_tmp = None;
        file.clone()
    };
    let mut rdr = match csv::Reader::from_path(&read_path) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: could not read input file {}: {}", file.display(), e);
            exit_with(2);
        }
    };
    let headers: Vec<String> = match rdr.headers() {
        Ok(h) => h.iter().map(|s| s.to_string()).collect(),
        Err(e) => {
            eprintln!("error: could not read CSV headers: {}", e);
            exit_with(2);
        }
    };
    let n_cols = headers.len();

    let mut rows: Vec<Vec<Option<String>>> = Vec::new();
    for result in rdr.records() {
        let record = match result {
            Ok(r) => r,
            Err(e) => {
                eprintln!("error: CSV parse error: {}", e);
                exit_with(2);
            }
        };
        let row: Vec<Option<String>> = (0..n_cols)
            .map(|i| {
                let val = record.get(i).unwrap_or("").trim();
                if val.is_empty() || val == "NULL" || val == "null" {
                    None
                } else {
                    Some(val.to_string())
                }
            })
            .collect();
        rows.push(row);
    }
    eprintln!("Read {} rows, {} columns", rows.len(), n_cols);

    // ── (5) Validate via finetype_core::table_validator ──────────────────────
    let result = match validate_table(&headers, &rows, &schema) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: validation engine failed: {}", e);
            exit_with(2);
        }
    };

    // ── Materialise path (only when --db/--table supplied) ──────────────────
    //    Check-only mode skips this entire block — no DuckDB shell-out, no
    //    .db file written. The pass/fail decision is the validation engine
    //    output alone, governed by the exit-code grid below.
    //
    //    Returns `Some((scan_id, transform_failed_count))` on materialise,
    //    `None` on check-only. `transform_failed_count` is the number of
    //    TRANSFORM_FAILED reject rows the pre-CTAS sweep emitted for this
    //    scan_id — counted post-script via a tiny duckdb query against the
    //    output .db. Both reject classes (SEMANTIC_TYPE engine rejects and
    //    TRANSFORM_FAILED sweep rejects) feed the exit-code decision.
    let materialise: Option<(i64, usize)> = if let (Some(db_path), Some(table_name)) =
        (db.as_ref(), table.as_ref())
    {
        // Compute scan_id now (before generating SQL).
        let scan_id: i64 = if append { next_scan_id(db_path) } else { 1 };

        // Load taxonomy for typed-column projection (ac-02). Graceful failure:
        // if `labels/` is absent (release binary without embed), we fall back
        // to a bare-passthrough projection — every column emits as VARCHAR,
        // matching the prior behaviour exactly. The TRANSFORM_FAILED sweep
        // becomes a no-op for the same reason (no typed columns → no
        // candidates → no INSERTs into __finetype_transform_failures).
        let taxonomy_path = std::path::PathBuf::from("labels");
        let taxonomy = load_taxonomy(&taxonomy_path).ok();

        // ── Generate SQL script (steps 3–10 of ac-09) ────────────────────────
        //    TEMPORARY staging table is auto-dropped when the DuckDB session
        //    ends, providing RAII-equivalent cleanup on success AND failure
        //    paths (ac-09 step 10 + the constraint's staging-cleanup rule).
        let uuid = uuid::Uuid::new_v4().simple().to_string();
        let staging_ident = format!("__finetype_staging_{}", uuid);
        let failures_ident = format!("__finetype_transform_failures_{}", uuid);
        let user_table_ident = sql_ident(table_name);
        let input_literal = sql_quote(&file.to_string_lossy());

        // Build the staging projection: SELECT * from the input file, add a
        // row-index column so we can filter to valid_row_indices later.
        let read_fn = if file
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.eq_ignore_ascii_case("parquet"))
            .unwrap_or(false)
        {
            // Cast every parquet column to VARCHAR so the staging table
            // matches the CSV path's all_varchar=true contract — typed
            // transforms (REGEXP_REPLACE, LIKE, TRY_CAST) downstream assume
            // VARCHAR staging cells.
            format!(
                "(SELECT COLUMNS(*)::VARCHAR FROM read_parquet({}))",
                input_literal
            )
        } else {
            format!("read_csv({}, header=true, all_varchar=true)", input_literal)
        };

        // Valid-indices filter. Render as an IN list when non-empty, otherwise
        // use `0=1` to select nothing. Used both by the per-column TRANSFORM
        // failure sweep and the user-table CTAS — the engine-rejected rows
        // are excluded from both.
        let valid_filter_predicate = if result.valid_row_indices.is_empty() {
            "0=1".to_string()
        } else {
            let idx_list: Vec<String> = result
                .valid_row_indices
                .iter()
                .map(|i| i.to_string())
                .collect();
            format!("__row_idx IN ({})", idx_list.join(","))
        };

        // ── Typed-column projection (ac-02) ───────────────────────────────
        //    For each column, pick:
        //      • bare quoted ident          (unlabelled / unknown / VARCHAR)
        //      • TRY(transform) AS "col"    (typed with transform — try_wrap)
        //      • TRY_CAST("col" AS T) AS "col" (typed without transform)
        //    See `build_transform_projection` for the 5-branch decision tree.
        let projection = match taxonomy.as_ref() {
            Some(t) => build_transform_projection(&headers, &extensions, t, true),
            None => "* EXCLUDE(__row_idx)".to_string(),
        };

        // ── Pre-CTAS transform-failure sweep (ac-03 + ac-04) ──────────────
        //    For each typed column with a transform (or a non-VARCHAR
        //    ddl_info), an INSERT detects rows where:
        //      staging IS NOT NULL  AND  TRY(transform) IS NULL
        //    NULL staging cells pass through (NULL-in-NULL-out is NOT a
        //    transform failure — staging-NULL → typed-NULL is documented
        //    NULL-flow, see ac-04 + spec constraint). The detected
        //    `__row_idx`s are excluded from the user-table CTAS below; the
        //    detection table feeds the TRANSFORM_FAILED reject INSERT later.
        let mut failure_inserts: Vec<String> = Vec::new();
        if let Some(t) = taxonomy.as_ref() {
            for (col_idx, header) in headers.iter().enumerate() {
                let (label_opt, type_confidence) = extensions.get(header);
                let label = match label_opt {
                    Some(l) => l,
                    None => continue,
                };
                let info = match t.ddl_info(&label) {
                    Some(i) => i,
                    None => continue,
                };
                if info.duckdb_type == "VARCHAR" {
                    continue;
                }
                let col_ref = format_column_name(header);
                // Build the TRY(...) expression that matches the projection
                // branch. Branch 4 (transform present) → TRY(transform);
                // Branch 5 (no transform, non-VARCHAR) → TRY_CAST(col AS T).
                let try_expr = if let Some(tf) = info.transform.as_ref() {
                    format!("TRY({})", tf.replace("{col}", &col_ref))
                } else {
                    format!("TRY_CAST({} AS {})", col_ref, info.duckdb_type)
                };
                // error_message convention:
                //   • SEMANTIC_TYPE rows  — `error_message` carries the
                //     engine's pass/fail diagnostic ("validation failed"
                //     or a parse error from the JSON Schema engine).
                //   • TRANSFORM_FAILED rows — `error_message` carries
                //     `transform_failed: <transform-expression>` so the
                //     reject sidecar names exactly which DuckDB cast or
                //     strptime() refused the cell.
                let error_message = if let Some(tf) = info.transform.as_ref() {
                    format!("transform_failed: {}", tf)
                } else {
                    format!("transform_failed: CAST AS {}", info.duckdb_type)
                };
                let insert = format!(
                    "INSERT INTO {failures} SELECT __row_idx, {col_idx}, {col_name}, {err_msg}, {expected}, {type_conf}, CAST({col_ref} AS VARCHAR) FROM {staging} WHERE {valid_filter} AND {col_ref} IS NOT NULL AND {try_expr} IS NULL;",
                    failures = sql_ident(&failures_ident),
                    col_idx = col_idx,
                    col_name = sql_quote(header),
                    err_msg = sql_quote(&error_message),
                    expected = sql_quote(&label),
                    type_conf = sql_opt_f64(&type_confidence),
                    col_ref = col_ref,
                    staging = sql_ident(&staging_ident),
                    valid_filter = valid_filter_predicate,
                    try_expr = try_expr,
                );
                failure_inserts.push(insert);
            }
        }

        // The user-table filter excludes engine-invalid rows AND any row that
        // any TRANSFORM_FAILED sweep flagged. The `NOT IN (SELECT ...)` clause
        // is always emitted so the temp-table relationship stays explicit
        // (when `failure_inserts` is empty the subquery returns zero rows
        // and the clause has no effect).
        let user_table_where = format!(
            "WHERE ({}) AND __row_idx NOT IN (SELECT row_idx FROM {})",
            valid_filter_predicate,
            sql_ident(&failures_ident),
        );

        // If --append and the user's table already exists, INSERT INTO rather
        // than CREATE TABLE AS. Otherwise CREATE TABLE AS from the staging.
        let exists_before_run = user_table_exists(db_path, table_name);
        let user_table_stmt = if append && exists_before_run {
            format!(
                "INSERT INTO {} SELECT {} FROM {} {};",
                user_table_ident,
                projection,
                sql_ident(&staging_ident),
                user_table_where
            )
        } else {
            format!(
                "CREATE TABLE {} AS SELECT {} FROM {} {};",
                user_table_ident,
                projection,
                sql_ident(&staging_ident),
                user_table_where
            )
        };

        // Build reject INSERTs. One row per engine RejectRecord (error_type
        // SEMANTIC_TYPE). Authored-time (expected_type, type_confidence)
        // comes from SchemaExtensions keyed by column name — NULL when the
        // schema lacks x-finetype-* (ac-11 graceful degradation).
        let mut reject_values: Vec<String> = Vec::with_capacity(result.rejects.len());
        for r in &result.rejects {
            let (expected_type, type_confidence) = extensions.get(&r.column_name);
            let line = (r.row_index as i64) + 1;
            let tuple = format!(
                "({scan_id}, 0, {line}, {col_idx}, {col_name}, 'SEMANTIC_TYPE', NULL, NULL, {err_msg}, {type_conf}, {exp_type}, {c_failed}, {c_value})",
                scan_id = scan_id,
                line = line,
                col_idx = r.column_index,
                col_name = sql_quote(&r.column_name),
                err_msg = sql_quote(&r.error_message),
                type_conf = sql_opt_f64(&type_confidence),
                exp_type = sql_opt_str(&expected_type),
                c_failed = sql_quote(&r.constraint_failed),
                c_value = sql_opt_str(&r.constraint_value),
            );
            reject_values.push(tuple);
        }

        let mut script = String::with_capacity(1024 + 128 * reject_values.len());
        script.push_str("BEGIN TRANSACTION;\n");
        script.push_str(&format!(
            "CREATE TEMPORARY TABLE {} AS SELECT row_number() OVER () - 1 AS __row_idx, * FROM {};\n",
            sql_ident(&staging_ident),
            read_fn
        ));
        // Transform-failure detection table is always emitted, even when
        // there are no typed columns — keeps the user-table CTAS's
        // `NOT IN (SELECT row_idx FROM …)` clause uniform.
        script.push_str(&format!(
            "CREATE TEMPORARY TABLE {} (row_idx BIGINT, column_idx INTEGER, column_name VARCHAR, error_message VARCHAR, expected_type VARCHAR, type_confidence DOUBLE, constraint_value VARCHAR);\n",
            sql_ident(&failures_ident),
        ));
        for insert in &failure_inserts {
            script.push_str(insert);
            script.push('\n');
        }
        script.push_str(&user_table_stmt);
        script.push('\n');
        script.push_str(REJECT_SIDECAR_DDL);
        script.push('\n');
        if !reject_values.is_empty() {
            script.push_str("INSERT INTO finetype_reject_errors VALUES\n");
            script.push_str(&reject_values.join(",\n"));
            script.push_str(";\n");
        }
        // TRANSFORM_FAILED reject rows from the pre-CTAS sweep. Pulls from
        // the temp detection table (one row per (row, typed-column) failure).
        // `csv_line` and `byte_position` stay NULL — the FineType engine
        // doesn't surface those for transform-cast failures.
        if !failure_inserts.is_empty() {
            script.push_str(&format!(
                "INSERT INTO finetype_reject_errors SELECT {scan_id}, 0, row_idx + 1, column_idx, column_name, 'TRANSFORM_FAILED', NULL, NULL, error_message, type_confidence, expected_type, 'transform', constraint_value FROM {failures};\n",
                scan_id = scan_id,
                failures = sql_ident(&failures_ident),
            ));
        }
        script.push_str("COMMIT;\n");
        // TEMPORARY tables (staging + failures) are auto-dropped on session
        // end — no explicit DROP needed; this is the RAII-equivalent cleanup
        // on both success AND error paths (ac-09 step 10).

        // ── Execute the script against the output .db ────────────────────────
        // Pipe SQL via stdin instead of `-c <script>` so large schemas
        // (many enum literals → multi-megabyte INSERT scripts) don't trip
        // the OS argv limit (E2BIG / "Argument list too long" on macOS at
        // ~256KB). See .orbit/specs/2026-04-28-validate-precision-corpus/
        // (un_locode / rio2016_athletes baseline runs hit ARG_MAX).
        use std::io::Write as _;
        let duckdb_out = (|| -> std::io::Result<std::process::Output> {
            let mut child = std::process::Command::new("duckdb")
                .arg(db_path)
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()?;
            if let Some(mut stdin) = child.stdin.take() {
                stdin.write_all(script.as_bytes())?;
            }
            child.wait_with_output()
        })();
        let duckdb_out = match duckdb_out {
            Ok(o) => o,
            Err(e) => {
                eprintln!(
                    "error: could not invoke duckdb CLI (is duckdb on PATH?): {}",
                    e
                );
                exit_with(2);
            }
        };
        if !duckdb_out.status.success() {
            eprintln!(
                "error: duckdb execution failed: {}",
                String::from_utf8_lossy(&duckdb_out.stderr).trim()
            );
            exit_with(2);
        }

        // Count TRANSFORM_FAILED rows emitted by the pre-CTAS sweep for
        // this scan_id. Feeds the exit-code grid below — a transform
        // failure is a reject, so any non-zero count flips exit 0 → 1
        // (with --lenient still able to force 0).
        let transform_failed_count: usize = if failure_inserts.is_empty() {
            0
        } else {
            duckdb_query_scalar(
                db_path,
                &format!(
                    "SELECT COUNT(*) FROM finetype_reject_errors WHERE scan_id = {} AND error_type = 'TRANSFORM_FAILED';",
                    scan_id
                ),
            )
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(0)
        };

        Some((scan_id, transform_failed_count))
    } else {
        None
    };

    // ── Summary report ───────────────────────────────────────────────────────
    //    `engine_reject_count` covers the JSON Schema / FineType engine
    //    rejects (error_type=SEMANTIC_TYPE). `transform_failed_count` covers
    //    the pre-CTAS sweep's TRANSFORM_FAILED rows. The two are reported
    //    separately so analysts can see the split, and both feed the
    //    exit-code grid below — any reject (either kind) flips exit 0 → 1
    //    unless --lenient is set.
    let engine_reject_count = result.rejects.len();
    let transform_failed_count = materialise.map(|(_, c)| c).unwrap_or(0);
    let total_reject_count = engine_reject_count + transform_failed_count;
    let scan_id = materialise.map(|(s, _)| s);
    match output {
        OutputFormat::Plain
        | OutputFormat::Arrow
        | OutputFormat::Csv
        | OutputFormat::Markdown
        | OutputFormat::JsonSchema
        | OutputFormat::Datapackage => {
            println!("Validation Report");
            println!("{}", "═".repeat(60));
            println!("  Input:        {}", file.display());
            println!("  Schema:       {}", schema_path.display());
            if let (Some(db_path), Some(table_name), Some(sid)) =
                (db.as_ref(), table.as_ref(), scan_id)
            {
                println!("  Output DB:    {}", db_path.display());
                println!("  Target table: {}", table_name);
                println!("  Scan ID:      {}", sid);
            } else {
                println!("  Mode:         check-only (no .db written)");
            }
            println!();
            println!("  Total rows:        {:>6}", result.total_rows);
            println!("  Valid rows:        {:>6}", result.valid_rows);
            println!("  Invalid rows:      {:>6}", result.invalid_rows);
            println!("  Rejects:           {:>6}", total_reject_count);
            if scan_id.is_some() {
                println!("    SEMANTIC_TYPE:   {:>6}", engine_reject_count);
                println!("    TRANSFORM_FAILED:{:>6}", transform_failed_count);
            }
            println!("  Grade:             {}", result.grade);
            println!("{}", "═".repeat(60));
        }
        OutputFormat::Json => {
            let report = json!({
                "input": file.display().to_string(),
                "schema": schema_path.display().to_string(),
                "db": db.as_ref().map(|p| p.display().to_string()),
                "table": table,
                "scan_id": scan_id,
                "mode": if scan_id.is_some() { "materialise" } else { "check-only" },
                "total_rows": result.total_rows,
                "valid_rows": result.valid_rows,
                "invalid_rows": result.invalid_rows,
                "rejects": total_reject_count,
                "rejects_by_type": {
                    "SEMANTIC_TYPE": engine_reject_count,
                    "TRANSFORM_FAILED": transform_failed_count,
                },
                "grade": result.grade,
            });
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
    }

    // ── Exit-code grid (ac-10) ───────────────────────────────────────────────
    if total_reject_count > 0 {
        if lenient {
            exit_with(0);
        } else {
            exit_with(1);
        }
    }
    // Zero rejects — always exit 0.
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// PROFILE — Detect column types in a CSV file
// ═══════════════════════════════════════════════════════════════════════════════

/// Resolve the display broad_type for a profile column, accounting for ENUM threshold.
///
/// When the taxonomy says ENUM but the column's cardinality exceeds the threshold
/// (or threshold is 0), downgrade to VARCHAR for display.
pub(crate) fn resolve_broad_type_display<'a>(
    broad_type: Option<&'a str>,
    unique_values: &Option<Vec<String>>,
) -> &'a str {
    match broad_type {
        Some("ENUM") => {
            if unique_values.is_some() {
                "ENUM"
            } else {
                "VARCHAR"
            }
        }
        Some(bt) => bt,
        None => "—",
    }
}

/// ac-06: evaluate the validation-as-veto for one column. Returns
/// `(pass_rate, hard_vetoed, advisory_low)`. A no-op `(None, false, false)`
/// when the veto is disabled, the taxonomy is unavailable, or the label has
/// no applicable validation. `values` are the column's non-null sample values.
pub(crate) fn col_validation_veto(
    label: &str,
    values: &[String],
    taxonomy: Option<&finetype_core::Taxonomy>,
    safe: &std::collections::HashSet<String>,
    enabled: bool,
) -> (Option<f64>, bool, bool) {
    if !enabled {
        return (None, false, false);
    }
    let Some(tax) = taxonomy else {
        return (None, false, false);
    };
    let opt: Vec<Option<&str>> = values.iter().map(|s| Some(s.as_str())).collect();
    let v = finetype_core::evaluate_validation_veto(label, &opt, tax, safe);
    (v.pass_rate, v.vetoed, v.advisory_low)
}

/// Resolve a HARD veto into the column's final label (spec
/// 2026-06-12-veto-shape-fallback). Without a veto the prediction stands.
/// With one, the value shape decides between the two residual labels
/// (generic id / vocabulary) instead of an unconditional `unknown` — the
/// precedence decision the flat Sense head cannot express. The fallback
/// never re-asserts the vetoed label itself, and
/// `FINETYPE_NO_VETO_FALLBACK=1` restores the pre-spec demotion. Returns
/// `(final_label, vetoed_type, fallback_rule)`.
pub(crate) fn resolve_veto_outcome(
    vetoed: bool,
    predicted: &str,
    values: &[String],
) -> (String, Option<String>, Option<&'static str>) {
    if !vetoed {
        return (predicted.to_string(), None, None);
    }
    let disabled = std::env::var("FINETYPE_NO_VETO_FALLBACK").is_ok_and(|v| v == "1");
    if !disabled {
        let opt: Vec<Option<&str>> = values.iter().map(|s| Some(s.as_str())).collect();
        if let Some(fallback) = finetype_core::veto_shape_fallback(&opt) {
            // Re-asserting the SAME label the veto rejected is normally circular,
            // so a fallback equal to `predicted` demotes to `unknown`. The one
            // exception is the `alphanumeric_id` id residual (BACKLOG #13): its
            // validation pattern requires a LETTER, so it hard-vetoes columns the
            // shape detector independently confirms are ids (BenchmarkName's
            // `CONV_FWD…<…>` — out of the pattern's charset). The shape signal
            // (distinct + id charset) and the strict pattern legitimately
            // disagree, so the shape detector's id verdict is allowed to stand.
            let is_id_residual = fallback.ends_with("alphanumeric_id");
            if fallback != predicted || is_id_residual {
                let rule = if is_id_residual {
                    "veto_fallback:id"
                } else {
                    "veto_fallback:vocab"
                };
                return (
                    fallback.to_string(),
                    Some(predicted.to_string()),
                    Some(rule),
                );
            }
        }
    }
    ("unknown".to_string(), Some(predicted.to_string()), None)
}
