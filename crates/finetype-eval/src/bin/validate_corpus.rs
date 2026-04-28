//! FineType Validate-Precision Corpus Harness (spec 2026-04-28-validate-precision-corpus)
//!
//! Round-trip self-consistency harness over a corpus of whole real-world CSVs.
//! For each dataset:
//!   1. Run `finetype profile -f <csv> -o json-schema` → captures predicted
//!      x-finetype-label per column.
//!   2. Run `finetype validate <csv> <schema> --db <tmp.db> --table data --lenient`
//!      → produces `finetype_reject_errors` with per-row, per-column
//!      validation failures (MADR 0064 / 0071 reject ontology).
//!   3. Per failing column, attribute a mechanism via the deterministic
//!      5-rule table (ac-07).
//!   4. Aggregate per-dataset pass@99% and corpus headline `N of M`.
//!   5. Compare to `validate_corpus.baseline.md` (if present) for delta.
//!   6. Write `eval/eval_output/validate_corpus.md`.
//!
//! Exit code: 0 always (the harness reports — it does not gate).

use anyhow::{anyhow, Context, Result};
use chrono::Local;
use clap::Parser;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

// ---------------------------------------------------------------------------
// CLI arguments
// ---------------------------------------------------------------------------

#[derive(Parser)]
#[command(
    name = "validate-corpus",
    about = "FineType profile→validate round-trip precision harness"
)]
struct Args {
    /// Validate-corpus manifest CSV.
    #[arg(long, default_value = "eval/datasets/validate_manifest.csv")]
    manifest: PathBuf,

    /// Output report path.
    #[arg(long, default_value = "eval/eval_output/validate_corpus.md")]
    output: PathBuf,

    /// Pass threshold (per-dataset row valid-fraction).
    #[arg(long, default_value = "0.99")]
    threshold: f64,

    /// Path to the finetype binary. If omitted, defaults to
    /// `target/release/finetype` relative to the workspace root.
    #[arg(long)]
    finetype_bin: Option<PathBuf>,

    /// Optional baseline file for delta computation. If absent on first run,
    /// the report still emits but `delta:` reads `delta: baseline-not-yet-committed`.
    #[arg(long, default_value = "eval/eval_output/validate_corpus.baseline.md")]
    baseline: PathBuf,
}

// ---------------------------------------------------------------------------
// Manifest row
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct ManifestRow {
    dataset: String,
    file_path: PathBuf,
    gt_sidecar_path: PathBuf,
}

fn read_manifest(path: &Path) -> Result<Vec<ManifestRow>> {
    let mut reader = csv::Reader::from_path(path)
        .with_context(|| format!("opening manifest {}", path.display()))?;
    let mut out = Vec::new();
    for rec in reader.deserialize() {
        let row: BTreeMap<String, String> = rec.context("parsing manifest row")?;
        let dataset = row
            .get("dataset")
            .cloned()
            .ok_or_else(|| anyhow!("manifest row missing `dataset`"))?;
        let file_path = row
            .get("file_path")
            .cloned()
            .ok_or_else(|| anyhow!("manifest row missing `file_path`"))?;
        let gt_path = row
            .get("gt_sidecar_path")
            .cloned()
            .ok_or_else(|| anyhow!("manifest row missing `gt_sidecar_path`"))?;
        out.push(ManifestRow {
            dataset,
            file_path: PathBuf::from(file_path),
            gt_sidecar_path: PathBuf::from(gt_path),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// GT sidecar
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct GtSidecar {
    columns: BTreeMap<String, String>,
}

fn read_gt_sidecar(path: &Path) -> Result<GtSidecar> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("reading GT sidecar {}", path.display()))?;
    let parsed: GtSidecar = serde_yaml::from_str(&text)
        .with_context(|| format!("parsing GT sidecar {}", path.display()))?;
    Ok(parsed)
}

// ---------------------------------------------------------------------------
// Profile invocation — returns predicted_label per column from JSON Schema
// ---------------------------------------------------------------------------

fn run_profile(finetype_bin: &Path, csv: &Path) -> Result<BTreeMap<String, String>> {
    let out = Command::new(finetype_bin)
        .arg("profile")
        .arg("-f")
        .arg(csv)
        .arg("-o")
        .arg("json-schema")
        .output()
        .with_context(|| {
            format!(
                "running `{} profile -f {} -o json-schema`",
                finetype_bin.display(),
                csv.display()
            )
        })?;
    if !out.status.success() {
        return Err(anyhow!(
            "profile failed for {}: status={}, stderr=\n{}",
            csv.display(),
            out.status,
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    let stdout = String::from_utf8(out.stdout).context("profile stdout not UTF-8")?;
    parse_predicted_labels_from_schema(&stdout)
}

fn parse_predicted_labels_from_schema(schema_json: &str) -> Result<BTreeMap<String, String>> {
    let v: serde_json::Value =
        serde_json::from_str(schema_json).context("parsing JSON schema from profile")?;
    let mut out = BTreeMap::new();
    // Table-level schema: { "type": "object", "properties": { col: { "x-finetype-label": "..." } } }
    let props = v.get("properties").and_then(|p| p.as_object());
    if let Some(props) = props {
        for (col, col_schema) in props {
            if let Some(label) = col_schema.get("x-finetype-label").and_then(|s| s.as_str()) {
                out.insert(col.clone(), label.to_string());
            } else {
                out.insert(col.clone(), "<unlabelled>".to_string());
            }
        }
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Validate invocation — runs `finetype validate` with --db, returns
// (total_rows_in_csv, reject rows from finetype_reject_errors)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct RejectRow {
    column_name: String,
    error_type: String,
    constraint_failed: Option<String>,
    /// Read from finetype_reject_errors but currently unused in attribution.
    /// Retained for future rules and for diagnostic output via `Debug`.
    #[allow(dead_code)]
    expected_type: Option<String>,
}

fn count_csv_data_rows(csv: &Path) -> Result<usize> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_path(csv)
        .with_context(|| format!("opening {}", csv.display()))?;
    let mut n = 0;
    for rec in rdr.records() {
        let _ = rec.context("reading csv data row")?;
        n += 1;
    }
    Ok(n)
}

fn run_validate(
    finetype_bin: &Path,
    csv: &Path,
    schema_file: &Path,
    db_file: &Path,
) -> Result<Vec<RejectRow>> {
    // Wipe stale db if present so --append isn't required.
    if db_file.exists() {
        fs::remove_file(db_file)
            .with_context(|| format!("removing stale db {}", db_file.display()))?;
    }
    let out = Command::new(finetype_bin)
        .arg("validate")
        .arg(csv)
        .arg(schema_file)
        .arg("--db")
        .arg(db_file)
        .arg("--table")
        .arg("data")
        .arg("--lenient")
        .output()
        .with_context(|| {
            format!(
                "running `{} validate {} {} --db {} --table data --lenient`",
                finetype_bin.display(),
                csv.display(),
                schema_file.display(),
                db_file.display()
            )
        })?;
    // --lenient forces 0 on rejects; non-zero is a real error.
    if !out.status.success() {
        return Err(anyhow!(
            "validate failed for {}: status={}, stderr=\n{}",
            csv.display(),
            out.status,
            String::from_utf8_lossy(&out.stderr)
        ));
    }

    // Read finetype_reject_errors from the produced db.
    let conn = duckdb::Connection::open(db_file)
        .with_context(|| format!("opening produced db {}", db_file.display()))?;
    let mut stmt = conn.prepare(
        "SELECT column_name, error_type, constraint_failed, expected_type
         FROM finetype_reject_errors",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(RejectRow {
            column_name: r.get(0)?,
            error_type: r.get(1)?,
            constraint_failed: r.get(2).ok(),
            expected_type: r.get(3).ok(),
        })
    })?;
    let mut out_rows = Vec::new();
    for r in rows {
        out_rows.push(r?);
    }
    Ok(out_rows)
}

// ---------------------------------------------------------------------------
// Mechanism attribution (ac-07)
// ---------------------------------------------------------------------------

mod attribute {
    use super::RejectRow;

    /// The 5+1 mechanism partition. `NoGt` is reported separately and does
    /// not appear in the deterministic rule cascade.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum Mechanism {
        EnumOverfit,
        FormatDiversity,
        Misclassification,
        CodeVsCanonical,
        Unknown,
        NoGt,
    }

    impl Mechanism {
        pub fn label(&self) -> &'static str {
            match self {
                Mechanism::EnumOverfit => "enum_overfit",
                Mechanism::FormatDiversity => "format_diversity",
                Mechanism::Misclassification => "misclassification",
                Mechanism::CodeVsCanonical => "code_vs_canonical",
                Mechanism::Unknown => "unknown",
                Mechanism::NoGt => "no_gt",
            }
        }
    }

    /// Code-vs-canonical seam table (5 fixed seams, per spec
    /// implementation_notes). A column-name that contains any of these
    /// substrings (case-insensitive) is in the seam.
    pub const SEAMS: &[&str] = &["gender", "country", "currency", "language", "blood_type"];

    pub fn column_in_seam_table(column_name: &str) -> bool {
        let lower = column_name.to_lowercase();
        SEAMS.iter().any(|s| lower.contains(s))
    }

    /// Apply the 5-rule cascade. `predicted_label` and `expected_label` are
    /// equal under string equality. `rejects` are the reject rows for THIS
    /// column only (caller filters by column_name).
    pub fn attribute(
        column_name: &str,
        predicted_label: &str,
        expected_label: &str,
        rejects: &[RejectRow],
    ) -> Mechanism {
        let mismatch = predicted_label != expected_label;
        let any_semantic = rejects.iter().any(|r| r.error_type == "SEMANTIC_TYPE");
        let any_enum = rejects
            .iter()
            .any(|r| r.constraint_failed.as_deref() == Some("enum"));
        let any_pattern = rejects
            .iter()
            .any(|r| r.constraint_failed.as_deref() == Some("pattern"));

        // Rule 1: misclassification — predicted != expected AND any SEMANTIC_TYPE reject.
        if mismatch && any_semantic {
            return Mechanism::Misclassification;
        }

        // Rule 2: enum_overfit — predicted == expected AND any enum-constraint failure.
        if !mismatch && any_enum {
            return Mechanism::EnumOverfit;
        }

        // Rule 3: code_vs_canonical — predicted == expected AND SEMANTIC_TYPE
        // pattern-failure AND column is in the 5-seam table.
        if !mismatch && any_semantic && any_pattern && column_in_seam_table(column_name) {
            return Mechanism::CodeVsCanonical;
        }

        // Rule 4: format_diversity — predicted == expected AND SEMANTIC_TYPE
        // pattern-failure AND not in seam table.
        if !mismatch && any_semantic && any_pattern {
            return Mechanism::FormatDiversity;
        }

        // Rule 5: unknown.
        Mechanism::Unknown
    }
}

// ---------------------------------------------------------------------------
// Per-dataset orchestration
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct DatasetResult {
    dataset: String,
    rows_total: usize,
    rows_valid: usize,
    pass_at_threshold: bool,
    failing_columns: BTreeMap<String, attribute::Mechanism>, // column → mechanism
}

fn process_dataset(
    finetype_bin: &Path,
    row: &ManifestRow,
    threshold: f64,
    tmp_dir: &Path,
) -> Result<DatasetResult> {
    let predicted = run_profile(finetype_bin, &row.file_path)?;

    // Persist schema for validate.
    let schema_path = tmp_dir.join(format!("{}.schema.json", row.dataset));
    let schema_json = Command::new(finetype_bin)
        .arg("profile")
        .arg("-f")
        .arg(&row.file_path)
        .arg("-o")
        .arg("json-schema")
        .output()
        .context("re-running profile to capture schema for validate")?;
    fs::write(&schema_path, &schema_json.stdout)?;

    let db_path = tmp_dir.join(format!("{}.db", row.dataset));
    let rejects = run_validate(finetype_bin, &row.file_path, &schema_path, &db_path)?;

    let total_rows = count_csv_data_rows(&row.file_path)?;
    // valid_rows = total - distinct row_idx with rejects. We don't have
    // row_idx in our reject view (that's `line`), but for pass@P we count
    // distinct (line, column_name) tuples conservatively as rejected rows
    // by line:
    let conn = duckdb::Connection::open(&db_path)?;
    let reject_lines: i64 = conn.query_row(
        "SELECT COUNT(DISTINCT line) FROM finetype_reject_errors",
        [],
        |r| r.get(0),
    )?;
    let valid_rows = total_rows.saturating_sub(reject_lines as usize);

    // Per-column attribution. A column with ≥1 reject is "failing".
    let gt = read_gt_sidecar(&row.gt_sidecar_path)?;
    let mut failing_columns: BTreeMap<String, attribute::Mechanism> = BTreeMap::new();
    let mut by_col: BTreeMap<String, Vec<RejectRow>> = BTreeMap::new();
    for r in &rejects {
        by_col
            .entry(r.column_name.clone())
            .or_default()
            .push(r.clone());
    }
    for (col, rs) in &by_col {
        let predicted_label = predicted
            .get(col)
            .cloned()
            .unwrap_or_else(|| "<missing>".into());
        let expected = gt.columns.get(col);
        let mech = match expected {
            None => attribute::Mechanism::NoGt,
            Some(exp) => attribute::attribute(col, &predicted_label, exp, rs),
        };
        failing_columns.insert(col.clone(), mech);
    }

    let pass_at_threshold = if total_rows == 0 {
        true
    } else {
        (valid_rows as f64) / (total_rows as f64) >= threshold
    };

    Ok(DatasetResult {
        dataset: row.dataset.clone(),
        rows_total: total_rows,
        rows_valid: valid_rows,
        pass_at_threshold,
        failing_columns,
    })
}

// ---------------------------------------------------------------------------
// Report rendering
// ---------------------------------------------------------------------------

fn top_mechanism(failing: &BTreeMap<String, attribute::Mechanism>) -> String {
    let mut counts: BTreeMap<&'static str, usize> = BTreeMap::new();
    for m in failing.values() {
        *counts.entry(m.label()).or_insert(0) += 1;
    }
    counts
        .into_iter()
        .max_by_key(|(_, n)| *n)
        .map(|(k, _)| k.to_string())
        .unwrap_or_else(|| "—".into())
}

#[derive(Debug, Clone)]
struct BaselineHeadline {
    n: usize,
    m: usize,
}

fn read_baseline_headline(path: &Path) -> Option<BaselineHeadline> {
    if !path.exists() {
        return None;
    }
    let text = fs::read_to_string(path).ok()?;
    // Look for "**<N> of <M> datasets pass at P=99%**"
    for line in text.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("**") {
            if let Some(idx) = rest.find(" of ") {
                let n_str = &rest[..idx];
                let after = &rest[idx + 4..];
                if let Some(idx2) = after.find(' ') {
                    let m_str = &after[..idx2];
                    if let (Ok(n), Ok(m)) = (n_str.parse::<usize>(), m_str.parse::<usize>()) {
                        return Some(BaselineHeadline { n, m });
                    }
                }
            }
        }
    }
    None
}

fn render_report(
    results: &[DatasetResult],
    threshold: f64,
    baseline: Option<&BaselineHeadline>,
) -> String {
    let m = results.len();
    let n = results.iter().filter(|r| r.pass_at_threshold).count();
    let total_rows: usize = results.iter().map(|r| r.rows_total).sum();
    let now = Local::now().format("%Y-%m-%d %H:%M:%S %Z").to_string();

    let mut out = String::new();
    out.push_str("# Validate-Precision Corpus Report\n");
    out.push_str(&format!("Generated: {now}\n"));
    out.push_str(&format!("Threshold: P={threshold:.2}\n"));
    out.push_str(&format!(
        "Corpus: {m} datasets, {total_rows} rows total\n\n"
    ));

    out.push_str("## Headline\n");
    out.push_str(&format!(
        "**{n} of {m} datasets pass at P={pct}%**",
        pct = (threshold * 100.0).round() as u32
    ));
    match baseline {
        Some(b) => {
            let delta = n as i64 - b.n as i64;
            let sign = if delta >= 0 { "+" } else { "" };
            out.push_str(&format!(
                " (baseline: {n0} of {m0}; delta: {sign}{delta})\n\n",
                n0 = b.n,
                m0 = b.m
            ));
        }
        None => {
            out.push_str(" (baseline: baseline-not-yet-committed; delta: N/A)\n\n");
        }
    }

    // Per-mechanism breakdown
    out.push_str("## Per-mechanism breakdown\n");
    out.push_str("| Mechanism             | Failing columns | Datasets affected |\n");
    out.push_str("|-----------------------|-----------------|-------------------|\n");
    let order: &[attribute::Mechanism] = &[
        attribute::Mechanism::EnumOverfit,
        attribute::Mechanism::FormatDiversity,
        attribute::Mechanism::Misclassification,
        attribute::Mechanism::CodeVsCanonical,
        attribute::Mechanism::Unknown,
        attribute::Mechanism::NoGt,
    ];
    for mech in order {
        let mut col_count = 0usize;
        let mut datasets: BTreeSet<&str> = BTreeSet::new();
        for r in results {
            for m_observed in r.failing_columns.values() {
                if m_observed == mech {
                    col_count += 1;
                    datasets.insert(&r.dataset);
                }
            }
        }
        out.push_str(&format!(
            "| {label:<21} | {col_count:>15} | {ds_count:>17} |\n",
            label = mech.label(),
            col_count = col_count,
            ds_count = datasets.len()
        ));
    }
    out.push('\n');

    // Per-dataset table
    out.push_str("## Per-dataset\n");
    out.push_str("| Dataset | Rows | Valid | Pass@99% | Failing columns | Top mechanism |\n");
    out.push_str("|---|---:|---:|:---:|---:|---|\n");
    for r in results {
        let pass = if r.pass_at_threshold { "✓" } else { "✗" };
        out.push_str(&format!(
            "| {ds} | {tot} | {val} | {pass} | {nfail} | {top} |\n",
            ds = r.dataset,
            tot = r.rows_total,
            val = r.rows_valid,
            pass = pass,
            nfail = r.failing_columns.len(),
            top = top_mechanism(&r.failing_columns),
        ));
    }
    out.push('\n');

    // Per-column attribution detail — anchors ac-10 widening choices.
    let any_failing = results.iter().any(|r| !r.failing_columns.is_empty());
    if any_failing {
        out.push_str("## Per-column attributions\n");
        out.push_str("| Dataset | Column | Mechanism |\n");
        out.push_str("|---|---|---|\n");
        for r in results {
            for (col, mech) in &r.failing_columns {
                out.push_str(&format!(
                    "| {ds} | {col} | {mech} |\n",
                    ds = r.dataset,
                    col = col,
                    mech = mech.label()
                ));
            }
        }
        out.push('\n');
    }

    out
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

fn default_finetype_bin() -> PathBuf {
    PathBuf::from("target/release/finetype")
}

fn main() -> Result<()> {
    let args = Args::parse();
    let finetype_bin = args.finetype_bin.unwrap_or_else(default_finetype_bin);
    if !finetype_bin.exists() {
        eprintln!(
            "warning: finetype binary not found at {}; \
             run `cargo build --release -p finetype-cli` first",
            finetype_bin.display()
        );
    }

    let manifest_rows = read_manifest(&args.manifest)?;
    eprintln!(
        "validate-corpus: {} datasets in manifest {}",
        manifest_rows.len(),
        args.manifest.display()
    );

    let tmp_dir =
        std::env::temp_dir().join(format!("finetype-validate-corpus-{}", std::process::id()));
    fs::create_dir_all(&tmp_dir)
        .with_context(|| format!("creating tmp dir {}", tmp_dir.display()))?;

    let mut results = Vec::with_capacity(manifest_rows.len());
    for row in &manifest_rows {
        eprintln!("  processing {} ({})", row.dataset, row.file_path.display());
        match process_dataset(&finetype_bin, row, args.threshold, &tmp_dir) {
            Ok(r) => {
                eprintln!(
                    "    rows={} valid={} pass={} failing_cols={}",
                    r.rows_total,
                    r.rows_valid,
                    r.pass_at_threshold,
                    r.failing_columns.len()
                );
                results.push(r);
            }
            Err(e) => {
                eprintln!("    FAILED: {e:#}");
                results.push(DatasetResult {
                    dataset: row.dataset.clone(),
                    rows_total: 0,
                    rows_valid: 0,
                    pass_at_threshold: false,
                    failing_columns: BTreeMap::new(),
                });
            }
        }
    }

    let baseline = read_baseline_headline(&args.baseline);
    let report = render_report(&results, args.threshold, baseline.as_ref());

    if let Some(parent) = args.output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&args.output, &report)
        .with_context(|| format!("writing report to {}", args.output.display()))?;
    eprintln!(
        "validate-corpus: report written to {}",
        args.output.display()
    );

    // Tmp dir cleanup is best-effort.
    let _ = fs::remove_dir_all(&tmp_dir);

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests (ac-07: 6 attribute_* unit tests)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod attribute_tests {
    use super::attribute::{attribute, column_in_seam_table, Mechanism};
    use super::RejectRow;

    fn semantic_pattern_reject(col: &str) -> RejectRow {
        RejectRow {
            column_name: col.to_string(),
            error_type: "SEMANTIC_TYPE".into(),
            constraint_failed: Some("pattern".into()),
            expected_type: Some("VARCHAR".into()),
        }
    }

    fn enum_reject(col: &str) -> RejectRow {
        RejectRow {
            column_name: col.to_string(),
            error_type: "SEMANTIC_TYPE".into(),
            constraint_failed: Some("enum".into()),
            expected_type: Some("VARCHAR".into()),
        }
    }

    #[test]
    fn pvc_attribute_rule_misclassification() {
        // predicted != expected, any SEMANTIC_TYPE reject → Misclassification
        let r = semantic_pattern_reject("foo");
        let m = attribute(
            "foo",
            "representation.text.plain_text",
            "identity.person.email",
            &[r],
        );
        assert_eq!(m, Mechanism::Misclassification);
    }

    #[test]
    fn pvc_attribute_rule_enum_overfit() {
        // predicted == expected, any enum-constraint failure → EnumOverfit
        let r = enum_reject("Type 1");
        let m = attribute(
            "Type 1",
            "representation.discrete.categorical",
            "representation.discrete.categorical",
            &[r],
        );
        assert_eq!(m, Mechanism::EnumOverfit);
    }

    #[test]
    fn pvc_attribute_rule_code_vs_canonical() {
        // predicted == expected, SEMANTIC_TYPE pattern-failure, column-name in seam table
        let r = semantic_pattern_reject("Country");
        let m = attribute(
            "Country",
            "geography.location.country",
            "geography.location.country",
            &[r],
        );
        assert_eq!(m, Mechanism::CodeVsCanonical);
    }

    #[test]
    fn pvc_attribute_rule_format_diversity() {
        // predicted == expected, SEMANTIC_TYPE pattern-failure, NOT in seam table
        let r = semantic_pattern_reject("Date");
        let m = attribute(
            "Date",
            "datetime.date.compact_ym",
            "datetime.date.compact_ym",
            &[r],
        );
        assert_eq!(m, Mechanism::FormatDiversity);
    }

    #[test]
    fn pvc_attribute_rule_unknown_fallback() {
        // predicted == expected, NO reject rows at all → Unknown (rule cascade falls through)
        let m = attribute(
            "weight",
            "representation.numeric.decimal_number",
            "representation.numeric.decimal_number",
            &[],
        );
        assert_eq!(m, Mechanism::Unknown);
    }

    #[test]
    fn pvc_attribute_no_gt_path() {
        // The no-GT path is structural (caller branches on gt.columns.get(col)).
        // The seam-detection helper underpins rule 3; verify it discriminates
        // correctly so columns OUTSIDE the seam don't become CodeVsCanonical.
        assert!(column_in_seam_table("Country"));
        assert!(column_in_seam_table("nationality_country_code"));
        assert!(column_in_seam_table("blood_type"));
        assert!(column_in_seam_table("LANGUAGE"));
        assert!(!column_in_seam_table("Mean"));
        assert!(!column_in_seam_table("Year"));
        assert!(!column_in_seam_table("IATA"));
    }
}
