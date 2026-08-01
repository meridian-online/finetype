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
// Mechanism attribution — iter-3 7-rule cascade (spec ac-01)
// ---------------------------------------------------------------------------
//
// Iter-3 coalesces failure attribution into 4 mechanism buckets per card 0014
// scenario 2: enum_overfit, format_diversity, misclassification,
// code_vs_canonical. Each non-trivial bucket has TWO trigger paths:
//   - Path A (iter-2): predicted == expected + pattern-reject
//   - Path B (iter-3): predicted != expected (subtype drift OR code/canonical)
//
// The cascade enforces explicit ordering: path-B rules fire BEFORE the
// catch-all misclassification rule so subtype-drift and code-vs-canonical
// failures don't get swallowed.
//
// Each rule lives in its own free function returning
// `Option<(Mechanism, &'static str)>` (None = doesn't fire, Some = matched
// with rule-owned trigger label). The dispatcher iterates the rules in order
// and returns the first match. Trigger labels are rule-owned, NEVER inferred
// from the Mechanism variant.

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

    /// Code-vs-canonical seam table (iter-2 path A — 5 fixed seams). A
    /// column-name that contains any of these substrings
    /// (case-insensitive) is in the seam. Used by Rule 4 (format_diversity
    /// path A — must NOT be in seam) and Rule 5 (code_vs_canonical path A
    /// — MUST be in seam).
    pub const SEAMS: &[&str] = &["gender", "country", "currency", "language", "blood_type"];

    pub fn column_in_seam_table(column_name: &str) -> bool {
        let lower = column_name.to_lowercase();
        SEAMS.iter().any(|s| lower.contains(s))
    }

    /// Code-typed taxonomy allowlist (iter-3 path B for code_vs_canonical
    /// — spec ac-02). Curated set of taxonomy labels treated as code-typed
    /// — values are short-form codes vs canonical text/name forms.
    ///
    /// Curation criterion: types whose values are short-form identifier
    /// codes (≤8 chars typical, often pinned by ISO/ANSI standards) where
    /// the same conceptual entity could equally be expressed as canonical
    /// text. Country code "US" vs country name "United States" is the
    /// archetypal pair.
    ///
    /// Every entry MUST be a key in `labels/definitions_*.yaml` —
    /// verified by `vci3_code_typed_allowlist_taxonomy_valid` test
    /// (loads taxonomy via finetype_core, asserts membership).
    ///
    /// Implementer MAY add or remove labels during fixture construction
    /// (ac-03 Phase 2); the seed below is a starting point, not a
    /// ceiling. Tuning the allowlist is NOT escalation (constraint 4).
    ///
    /// Escalation path (when label-only proves insufficient even after
    /// tuning): value-shape signals plumbed into RejectRow
    /// (column-mode statistics, length histograms, character-class
    /// summaries) — out of scope for iter-3, file follow-up spec.
    pub const CODE_TYPED_LABELS: &[&str] = &[
        // geography
        "geography.location.country_code",
        "geography.location.state_code",
        "geography.transportation.iata_code",
        "geography.transportation.icao_code",
        "geography.transportation.hs_code",
        "geography.transportation.unlocode",
        "geography.contact.calling_code",
        // finance
        "finance.banking.swift_bic",
        "finance.banking.iban",
        "finance.banking.aba_routing",
        "finance.banking.bsb",
        "finance.currency.currency_code",
        "finance.currency.currency_symbol",
        "finance.currency.amount",
        "finance.securities.cusip",
        "finance.securities.figi",
        "finance.securities.isin",
        "finance.securities.lei",
        "finance.securities.sedol",
        "finance.payment.credit_card_number",
        // identity
        "identity.medical.cpt",
        "identity.medical.loinc",
        "identity.medical.npi",
        "identity.medical.ndc",
        "identity.medical.icd10",
        "identity.government.ssn",
        "identity.government.ein",
        "identity.government.vin",
        "identity.government.eu_vat",
        "identity.commerce.ean",
        "identity.commerce.isbn",
        "identity.commerce.upc",
        "identity.person.gender_code",
        // technology
        "technology.code.doi",
        "technology.code.imei",
        "technology.code.locale_code",
        "technology.internet.http_method",
        "technology.internet.top_level_domain",
    ];

    pub fn is_code_typed(label: &str) -> bool {
        CODE_TYPED_LABELS.contains(&label)
    }

    fn broad_prefix(label: &str) -> Option<&str> {
        label.split('.').next()
    }

    fn any_semantic(rejects: &[RejectRow]) -> bool {
        rejects.iter().any(|r| r.error_type == "SEMANTIC_TYPE")
    }
    fn any_enum(rejects: &[RejectRow]) -> bool {
        rejects
            .iter()
            .any(|r| r.constraint_failed.as_deref() == Some("enum"))
    }
    fn any_pattern(rejects: &[RejectRow]) -> bool {
        rejects
            .iter()
            .any(|r| r.constraint_failed.as_deref() == Some("pattern"))
    }

    // -----------------------------------------------------------------------
    // Rule functions (ac-01). Each returns Some((Mechanism, trigger_label))
    // when the rule fires, None to fall through to the next rule.
    // -----------------------------------------------------------------------

    /// Rule 1 — enum_overfit.
    ///
    /// # Triggers
    /// `predicted == expected` AND any reject row has
    /// `constraint_failed == "enum"`. Means the predicted type is right but
    /// the validator's enum constraint over-fits to a closed value set that
    /// real-world data exceeds (e.g. additional currency codes the
    /// taxonomy hasn't enumerated).
    ///
    /// # Example
    /// `predicted = expected = "representation.discrete.categorical"`,
    /// rejects include `constraint_failed == "enum"` → enum_overfit /
    /// `enum-constraint`.
    ///
    /// # Does NOT fire on
    /// - `predicted != expected` (handled by path-B or misclassification).
    /// - Pattern-reject only without enum-reject (handled by Rule 4 / 5).
    pub fn try_enum_overfit(
        _column_name: &str,
        predicted: &str,
        expected: &str,
        rejects: &[RejectRow],
    ) -> Option<(Mechanism, &'static str)> {
        if predicted == expected && any_enum(rejects) {
            return Some((Mechanism::EnumOverfit, "enum-constraint"));
        }
        None
    }

    /// Rule 2 — format_diversity path B (iter-3 NEW).
    ///
    /// # Triggers
    /// `predicted != expected` AND GT and predicted share broad-type prefix
    /// (split on `.`, take first segment) AND full-label mismatch (i.e.
    /// subtype differs but domain matches). Means the model got the broad
    /// kind right but disagreed on the precise subtype — e.g. predicted
    /// `datetime.timestamp.iso_8601` but GT was
    /// `datetime.timestamp.sql_standard`.
    ///
    /// # Example
    /// `predicted = "datetime.timestamp.iso_8601"`, `expected =
    /// "datetime.timestamp.sql_standard"` → format_diversity /
    /// `path-b-prefix`.
    ///
    /// # Does NOT fire on
    /// - `predicted == expected` (handled by Rule 4 path A).
    /// - Cross-domain mismatch (e.g. `datetime` vs `representation` — not
    ///   subtype drift, falls through to Rule 6 misclassification).
    pub fn try_format_diversity_b(
        _column_name: &str,
        predicted: &str,
        expected: &str,
        _rejects: &[RejectRow],
    ) -> Option<(Mechanism, &'static str)> {
        if predicted == expected {
            return None;
        }
        let pp = broad_prefix(predicted)?;
        let ep = broad_prefix(expected)?;
        if pp == ep && predicted != expected {
            return Some((Mechanism::FormatDiversity, "path-b-prefix"));
        }
        None
    }

    /// Rule 3 — code_vs_canonical path B (iter-3 NEW).
    ///
    /// # Triggers
    /// `predicted != expected` AND exactly one side (predicted XOR
    /// expected) is in `CODE_TYPED_LABELS`. Means the column carries
    /// short-form codes where the schema expected canonical text, OR vice
    /// versa.
    ///
    /// # Example
    /// `predicted = "geography.location.country"` (text),
    /// `expected = "geography.location.country_code"` (code, in allowlist)
    /// → code_vs_canonical / `path-b-codetype`.
    ///
    /// # Does NOT fire on
    /// - `predicted == expected` (handled by Rule 5 path A).
    /// - Both sides in allowlist (e.g. `country_code` vs `swift_bic` —
    ///   that's just misclassification between two code types, not
    ///   code-vs-canonical; falls through to Rule 6).
    /// - Neither side in allowlist (handled by other rules).
    pub fn try_code_vs_canonical_b(
        _column_name: &str,
        predicted: &str,
        expected: &str,
        _rejects: &[RejectRow],
    ) -> Option<(Mechanism, &'static str)> {
        if predicted == expected {
            return None;
        }
        let p = is_code_typed(predicted);
        let e = is_code_typed(expected);
        if p ^ e {
            return Some((Mechanism::CodeVsCanonical, "path-b-codetype"));
        }
        None
    }

    /// Rule 4 — format_diversity path A (iter-2 PRESERVED).
    ///
    /// # Triggers
    /// `predicted == expected` AND SEMANTIC_TYPE pattern-reject AND
    /// `!column_in_seam_table(column_name)`. Means the type is right and
    /// the seam-table heuristic doesn't claim this column is a
    /// code-vs-canonical seam — so the rejects must be format-shape
    /// failures within the predicted subtype (variant formatting the
    /// validator regex doesn't cover).
    ///
    /// # Example
    /// `predicted = expected = "datetime.date.compact_ymd"`, column name
    /// "Date" (NOT in seam table), pattern-reject rows present →
    /// format_diversity / `path-a-pattern`.
    ///
    /// # Does NOT fire on
    /// - Column IS in seam table (handled by Rule 5 — must use
    ///   `column_in_seam_table()` helper, NOT a re-derived check inline).
    /// - No pattern-reject (handled by Rule 7 unknown).
    /// - `predicted != expected` (handled by paths B or Rule 6).
    pub fn try_format_diversity_a(
        column_name: &str,
        predicted: &str,
        expected: &str,
        rejects: &[RejectRow],
    ) -> Option<(Mechanism, &'static str)> {
        if predicted == expected
            && any_semantic(rejects)
            && any_pattern(rejects)
            && !column_in_seam_table(column_name)
        {
            return Some((Mechanism::FormatDiversity, "path-a-pattern"));
        }
        None
    }

    /// Rule 5 — code_vs_canonical path A (iter-2 PRESERVED).
    ///
    /// # Triggers
    /// `predicted == expected` AND SEMANTIC_TYPE pattern-reject AND
    /// `column_in_seam_table(column_name)`. Same trigger as Rule 4 with
    /// the seam guard inverted.
    ///
    /// # Example
    /// `predicted = expected = "geography.location.country"`, column name
    /// "Country" (matches seam "country"), pattern-reject rows present →
    /// code_vs_canonical / `path-a-pattern`.
    ///
    /// # Does NOT fire on
    /// - Column NOT in seam table (handled by Rule 4).
    /// - No pattern-reject (Rule 7 unknown).
    /// - `predicted != expected` (paths B or Rule 6).
    pub fn try_code_vs_canonical_a(
        column_name: &str,
        predicted: &str,
        expected: &str,
        rejects: &[RejectRow],
    ) -> Option<(Mechanism, &'static str)> {
        if predicted == expected
            && any_semantic(rejects)
            && any_pattern(rejects)
            && column_in_seam_table(column_name)
        {
            return Some((Mechanism::CodeVsCanonical, "path-a-pattern"));
        }
        None
    }

    /// Rule 6 — misclassification (catch-all for prediction errors).
    ///
    /// # Triggers
    /// `predicted != expected` AND any SEMANTIC_TYPE reject. Catch-all for
    /// rows where the model picked the wrong type and the validator caught
    /// it, but the failure isn't subtype drift (Rule 2) or
    /// code-vs-canonical (Rule 3).
    ///
    /// # Example
    /// `predicted = "representation.text.plain_text"`, `expected =
    /// "identity.person.email"`, SEMANTIC_TYPE reject present →
    /// misclassification / `prediction-error`.
    ///
    /// # Does NOT fire on
    /// - Same broad-type prefix (Rule 2 fires first by cascade order).
    /// - One side in code-typed allowlist (Rule 3 fires first).
    /// - `predicted == expected` (Rules 1/4/5).
    /// - No SEMANTIC_TYPE reject (Rule 7).
    pub fn try_misclassification(
        _column_name: &str,
        predicted: &str,
        expected: &str,
        rejects: &[RejectRow],
    ) -> Option<(Mechanism, &'static str)> {
        if predicted != expected && any_semantic(rejects) {
            return Some((Mechanism::Misclassification, "prediction-error"));
        }
        None
    }

    /// Rule 7 — fallthrough (unknown).
    ///
    /// # Triggers
    /// Always — terminal rule. Fires when no earlier rule matched.
    ///
    /// # Example
    /// `predicted == expected`, no rejects at all → unknown /
    /// `fallthrough`.
    ///
    /// # Does NOT fire on
    /// (Nothing — this rule always returns Some.)
    pub fn try_fallthrough(
        _column_name: &str,
        _predicted: &str,
        _expected: &str,
        _rejects: &[RejectRow],
    ) -> Option<(Mechanism, &'static str)> {
        Some((Mechanism::Unknown, "fallthrough"))
    }

    type RuleFn = fn(&str, &str, &str, &[RejectRow]) -> Option<(Mechanism, &'static str)>;

    /// The 7-rule cascade in priority order. The dispatcher iterates this
    /// slice and returns the first rule that returns Some.
    ///
    /// **Cascade order rationale:** path-B rules (Rule 2, Rule 3) fire
    /// BEFORE catch-all misclassification (Rule 6) so subtype drift and
    /// code-vs-canonical attribute correctly. Path-A rules (Rule 4, Rule 5)
    /// fire only when `predicted == expected`, so their position relative
    /// to path-B rules is mechanically irrelevant — but they sit AFTER
    /// path-B for readability (the predicate-equality split mirrors the
    /// trigger-path naming).
    pub const RULES: &[RuleFn] = &[
        try_enum_overfit,        // Rule 1
        try_format_diversity_b,  // Rule 2 (iter-3 NEW)
        try_code_vs_canonical_b, // Rule 3 (iter-3 NEW)
        try_format_diversity_a,  // Rule 4 (iter-2)
        try_code_vs_canonical_a, // Rule 5 (iter-2)
        try_misclassification,   // Rule 6
        try_fallthrough,         // Rule 7
    ];

    /// Apply the 7-rule cascade. `predicted_label` and `expected_label` are
    /// strings; `rejects` are reject rows for THIS column only (caller
    /// filters by column_name). Returns `(Mechanism, trigger_label)` —
    /// the trigger label is rule-owned, never inferred from the variant.
    ///
    /// Always returns Some — Rule 7 (fallthrough) is terminal.
    pub fn attribute(
        column_name: &str,
        predicted_label: &str,
        expected_label: &str,
        rejects: &[RejectRow],
    ) -> (Mechanism, &'static str) {
        for rule in RULES {
            if let Some(hit) = rule(column_name, predicted_label, expected_label, rejects) {
                return hit;
            }
        }
        // Unreachable — Rule 7 is terminal.
        (Mechanism::Unknown, "fallthrough")
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
    /// column → (mechanism, trigger label). The trigger label is rule-owned
    /// (set by the rule that fired in `attribute::attribute()`) and rendered
    /// into the per-column report (ac-09).
    failing_columns: BTreeMap<String, (attribute::Mechanism, &'static str)>,
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
    let mut failing_columns: BTreeMap<String, (attribute::Mechanism, &'static str)> =
        BTreeMap::new();
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
        let entry = match expected {
            // No-GT rows are pre-classified by the dispatcher caller (the
            // attribute cascade itself is GT-aware, not GT-required). The
            // NoGt mechanism uses a stable trigger label so the report
            // renders without surprises.
            None => (attribute::Mechanism::NoGt, "no-gt"),
            Some(exp) => attribute::attribute(col, &predicted_label, exp, rs),
        };
        failing_columns.insert(col.clone(), entry);
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

fn top_mechanism(failing: &BTreeMap<String, (attribute::Mechanism, &'static str)>) -> String {
    let mut counts: BTreeMap<&'static str, usize> = BTreeMap::new();
    for (m, _trigger) in failing.values() {
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
    out.push_str(&format!("Corpus: {m} datasets, {total_rows} rows total\n"));
    // ac-08: link to analyst-facing mechanism docs from the report
    // header. Relative path: from eval/eval_output/validate_corpus.md
    // up to docs/mechanism-attribution.md.
    out.push_str(
        "Mechanism reference: [docs/mechanism-attribution.md](../../docs/mechanism-attribution.md)\n\n",
    );

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
            for (m_observed, _trigger) in r.failing_columns.values() {
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
    // The `trigger` column (ac-09) carries the rule-owned label so analysts
    // can map an attribution back to the rule that fired without re-deriving
    // it from the mechanism name. Six distinct labels are expected across
    // the cascade: path-a-pattern, path-b-prefix, path-b-codetype,
    // enum-constraint, prediction-error, fallthrough (plus `no-gt` for
    // columns without ground truth).
    let any_failing = results.iter().any(|r| !r.failing_columns.is_empty());
    if any_failing {
        out.push_str("## Per-column attributions\n");
        out.push_str("| Dataset | Column | Mechanism | Trigger |\n");
        out.push_str("|---|---|---|---|\n");
        for r in results {
            for (col, (mech, trigger)) in &r.failing_columns {
                out.push_str(&format!(
                    "| {ds} | {col} | {mech} | {trigger} |\n",
                    ds = r.dataset,
                    col = col,
                    mech = mech.label(),
                    trigger = trigger,
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
// Fixture loader (ac-03)
// ---------------------------------------------------------------------------
//
// The fixture file at
// `eval/datasets/validate_corpus_expected_attributions.yaml` is the
// authoritative ground-truth for what every per-(dataset, column) failure
// means under the iter-3 attribution cascade. Schema:
//   - dataset             (string, required)
//   - column              (string, required)
//   - expected_mechanism  (string, required)
//   - rationale           (string, required)
//   - pending_escalation  (bool, optional, default false)
//
// The loader is exposed at module level so both unit tests and any future
// CI script can resolve and parse it via the same code path.

#[cfg(test)]
mod fixture {
    use serde::Deserialize;
    use std::path::PathBuf;

    #[derive(Debug, Clone, Deserialize)]
    pub struct FixtureRow {
        pub dataset: String,
        pub column: String,
        pub expected_mechanism: String,
        pub rationale: String,
        #[serde(default)]
        pub pending_escalation: bool,
    }

    /// Resolve the fixture path relative to the crate's manifest dir
    /// (`crates/finetype-eval`). Walking two levels up lands at the
    /// workspace root; `eval/datasets/...` is then unambiguous.
    pub fn fixture_path() -> PathBuf {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        manifest_dir
            .parent()
            .and_then(|p| p.parent())
            .map(|p| p.join("eval/datasets/validate_corpus_expected_attributions.yaml"))
            .expect("workspace root resolvable from CARGO_MANIFEST_DIR")
    }

    /// Load and parse the fixture YAML. Panics with a helpful message if
    /// the file is missing or malformed (test failure mode).
    pub fn load() -> Vec<FixtureRow> {
        let path = fixture_path();
        let content = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read fixture {}: {}", path.display(), e));
        serde_yaml::from_str(&content)
            .unwrap_or_else(|e| panic!("parse fixture {}: {}", path.display(), e))
    }
}

// ---------------------------------------------------------------------------
// Harness-report reading, parsing and vacuity checking
//
// Split out of `vci3_fixture_attribution_regression_match` so each step can be
// driven from an in-memory `&str` and from a path that need not exist. `mod
// report_tests` below drives them against reports built by `render_report` and
// then mutated, which keeps those tests independent of the state of the
// corpus.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod report {
    use super::fixture::FixtureRow;
    use std::collections::BTreeMap;
    use std::path::{Path, PathBuf};

    /// The `## Per-column attributions` table's header cells, in order.
    const ATTRIBUTION_HEADER: [&str; 4] = ["Dataset", "Column", "Mechanism", "Trigger"];

    /// One data row of the `## Per-column attributions` table.
    #[derive(Debug)]
    pub struct AttributionRow {
        pub dataset: String,
        pub column: String,
        pub mechanism: String,
        pub trigger: String,
    }

    /// What one pass over a harness report recovered.
    #[derive(Debug, Default)]
    pub struct ReportParse {
        /// Data rows of the `## Per-column attributions` table, in report order.
        pub rows: Vec<AttributionRow>,
        /// A `## Per-column attributions` heading was present.
        pub saw_section: bool,
        /// That section carried a header row matching `ATTRIBUTION_HEADER`.
        pub saw_header: bool,
        /// A `## Per-dataset` header row carrying a `Failing columns` cell was
        /// present, so `declared_failing_columns` was read rather than left at
        /// its default.
        pub saw_per_dataset_header: bool,
        /// Sum of the `## Per-dataset` table's `Failing columns` cells — the
        /// report's own statement of how much work is failing.
        pub declared_failing_columns: usize,
        /// One entry per `## Per-dataset` data row whose `Failing columns`
        /// cell did not parse as a count, so it contributed zero to
        /// `declared_failing_columns`. Each entry names the row's first cell
        /// and what stood where the count belongs.
        pub unreadable_failing_cells: Vec<String>,
    }

    enum Section {
        Other,
        PerDataset,
        PerColumn,
    }

    /// Resolve the harness report path relative to the crate's manifest dir
    /// (`crates/finetype-eval`), matching `fixture::fixture_path`.
    pub fn report_path() -> PathBuf {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        manifest_dir
            .parent()
            .and_then(|p| p.parent())
            .map(|p| p.join("eval/eval_output/validate_corpus.md"))
            .expect("workspace root resolvable from CARGO_MANIFEST_DIR")
    }

    /// Read the harness report at `path`.
    ///
    /// A missing file is an error rather than a signal to skip the
    /// comparison: the report is tracked, so a read error means the path moved
    /// or the working tree is damaged.
    pub fn read_harness_report(path: &Path) -> Result<String, String> {
        std::fs::read_to_string(path).map_err(|e| {
            format!(
                "read harness report {}: {} — this file is tracked, so a read error \
                 means the path moved or the working tree is damaged. Regenerate with \
                 `make validate-corpus`.",
                path.display(),
                e
            )
        })
    }

    /// Parse a harness report's `## Per-dataset` and `## Per-column
    /// attributions` tables.
    ///
    /// Pipe-delimited rows split into `[_, c1, .., cN, _]` — the outer pipes
    /// contribute a leading and a trailing empty field.
    pub fn parse_report(report: &str) -> ReportParse {
        let mut out = ReportParse::default();
        let mut section = Section::Other;
        let mut failing_idx: Option<usize> = None;

        for line in report.lines() {
            if line.starts_with("##") {
                section = if line.starts_with("## Per-column attributions") {
                    out.saw_section = true;
                    Section::PerColumn
                } else if line.starts_with("## Per-dataset") {
                    Section::PerDataset
                } else {
                    Section::Other
                };
                continue;
            }
            if !line.starts_with('|') {
                continue;
            }
            let cols: Vec<&str> = line.split('|').map(|s| s.trim()).collect();

            match section {
                Section::PerDataset => {
                    if cols.get(1) == Some(&"Dataset") {
                        // Locate the count by name: a table that gains or
                        // loses a column then moves the cell this reads
                        // rather than changing what it reads.
                        failing_idx = cols.iter().position(|c| *c == "Failing columns");
                        out.saw_per_dataset_header = failing_idx.is_some();
                        continue;
                    }
                    // Before the header row there is no cell to read; the
                    // absent header is reported on its own by
                    // `saw_per_dataset_header`.
                    let Some(i) = failing_idx else { continue };
                    // The alignment row (`|---|---:|…`) carries no counts.
                    // Matched on the row's first cell, the same test
                    // `Section::PerColumn` applies to its own.
                    if cols.get(1).is_some_and(|c| c.starts_with("---")) {
                        continue;
                    }
                    match cols.get(i).map(|c| (*c, c.parse::<usize>())) {
                        Some((_, Ok(n))) => out.declared_failing_columns += n,
                        Some((cell, Err(_))) => out.unreadable_failing_cells.push(format!(
                            "row `{}` carries `{}`",
                            cols.get(1).copied().unwrap_or_default(),
                            cell
                        )),
                        None => out.unreadable_failing_cells.push(format!(
                            "row `{}` has no cell in the `Failing columns` position",
                            cols.get(1).copied().unwrap_or_default()
                        )),
                    }
                }
                Section::PerColumn => {
                    if cols.len() < 6 {
                        continue;
                    }
                    if cols[1] == "Dataset" {
                        if cols[1..5] == ATTRIBUTION_HEADER {
                            out.saw_header = true;
                        }
                        continue;
                    }
                    if cols[1].starts_with("---") {
                        continue;
                    }
                    out.rows.push(AttributionRow {
                        dataset: cols[1].to_string(),
                        column: cols[2].to_string(),
                        mechanism: cols[3].to_string(),
                        trigger: cols[4].to_string(),
                    });
                }
                Section::Other => {}
            }
        }
        out
    }

    /// Reject a parse that compared nothing for a reason that reads like a
    /// pass.
    ///
    /// The report's own `Failing columns` counts decide whether an absent
    /// `## Per-column attributions` section is a defect: `render_report` emits
    /// that section under `any_failing`, so a report whose per-dataset counts
    /// sum to zero carries no section and leaves nothing to compare. Reading
    /// that count is therefore itself required — a `## Per-dataset` table this
    /// cannot read makes the two cases indistinguishable, and is an error.
    ///
    /// "Cannot read" covers two shapes, guarded here in order: a header row
    /// with no `Failing columns` cell to locate, and a data row whose cell in
    /// that position is not a count. The second contributes zero to the sum,
    /// so left unguarded it drives the sum toward the goal state's own value.
    pub fn check_not_vacuous(parse: &ReportParse) -> Result<(), String> {
        if !parse.saw_per_dataset_header {
            return Err(
                "no `## Per-dataset` header row carrying a `Failing columns` cell — the \
                 report's own count of failing work could not be read, so an absent \
                 `## Per-column attributions` section cannot be told apart from a renamed one."
                    .to_string(),
            );
        }
        if !parse.unreadable_failing_cells.is_empty() {
            return Err(format!(
                "the `## Per-dataset` table carries a `Failing columns` cell that is not a \
                 count ({}) — those rows added zero to the declared total, so the sum \
                 cannot be told apart from a corpus with no failing column.",
                parse.unreadable_failing_cells.join("; ")
            ));
        }
        if parse.declared_failing_columns == 0 {
            return Ok(());
        }
        if !parse.saw_section {
            return Err(format!(
                "the `## Per-dataset` table declares {} failing columns but the report has no \
                 `## Per-column attributions` heading — nothing was compared, which reads \
                 exactly like a pass.",
                parse.declared_failing_columns
            ));
        }
        if !parse.saw_header {
            return Err(format!(
                "the `## Per-column attributions` heading is present but its `| {} |` header \
                 row is not — the table shape changed and nothing was compared.",
                ATTRIBUTION_HEADER.join(" | ")
            ));
        }
        if parse.rows.len() != parse.declared_failing_columns {
            return Err(format!(
                "the `## Per-dataset` table declares {} failing columns but the \
                 `## Per-column attributions` table holds {} data rows — the two disagree, so \
                 some attributions went uncompared.",
                parse.declared_failing_columns,
                parse.rows.len()
            ));
        }
        Ok(())
    }

    /// Compare parsed attribution rows against the fixture. Returns the number
    /// of rows that matched their fixture entry, and one message per
    /// disagreement.
    pub fn compare_to_fixture(
        rows: &[AttributionRow],
        fixture: &[FixtureRow],
    ) -> (usize, Vec<String>) {
        let by_key: BTreeMap<(&str, &str), &FixtureRow> = fixture
            .iter()
            .map(|r| ((r.dataset.as_str(), r.column.as_str()), r))
            .collect();

        let mut compared = 0;
        let mut failures = Vec::new();
        for row in rows {
            match by_key.get(&(row.dataset.as_str(), row.column.as_str())) {
                None => failures.push(format!(
                    "(dataset={}, column={}) missing from fixture (harness reports {})",
                    row.dataset, row.column, row.mechanism
                )),
                // Skip — these are awaiting upstream fixes (model improvement,
                // taxonomy widening, value-shape signals) and may legitimately
                // diverge from the iter-2-anchored expected_mechanism.
                Some(f) if f.pending_escalation => {}
                Some(f) if f.expected_mechanism != row.mechanism => failures.push(format!(
                    "(dataset={}, column={}) fixture expected={} but harness actual={} \
                     — rationale: {}",
                    row.dataset,
                    row.column,
                    f.expected_mechanism,
                    row.mechanism,
                    f.rationale.trim()
                )),
                _ => compared += 1,
            }
        }
        (compared, failures)
    }
}

// ---------------------------------------------------------------------------
// Tests
//
// Test naming convention (spec ac-07):
//   - `vci3_attribute_*`        → cascade outcome tests (positive + negative)
//   - `vci3_attribute_cascade_*`→ ordering / seam-table dispatch tests
//   - `vci3_code_typed_*`       → CODE_TYPED_LABELS allowlist invariants
//   - `vci3_fixture_*`          → fixture round-trip / regression tests
//   - `vci3_report_*`           → harness-report reading / parsing / vacuity
//
// The legacy `pvc_attribute_rule_*` and `mechanism_attribution_*` test names
// from iter-1 / iter-2 are dropped here in iter-3. Their semantic coverage is
// preserved (and extended) by the vci3_* suite below.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod attribute_tests {
    use super::attribute::{
        attribute, column_in_seam_table, is_code_typed, Mechanism, CODE_TYPED_LABELS,
    };
    use super::RejectRow;

    // --- helpers ---------------------------------------------------------

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

    fn transform_failed(col: &str) -> RejectRow {
        RejectRow {
            column_name: col.to_string(),
            error_type: "TRANSFORM_FAILED".into(),
            constraint_failed: Some("transform".into()),
            expected_type: Some("BIGINT".into()),
        }
    }

    // --- ac-04 positive tests (one per mechanism / trigger label) --------

    /// Rule 1 — enum_overfit fires when predicted == expected and any
    /// reject row carries `constraint_failed == "enum"`.
    #[test]
    fn vci3_attribute_enum_overfit_fires() {
        let r = enum_reject("Type 1");
        let (m, t) = attribute(
            "Type 1",
            "representation.discrete.categorical",
            "representation.discrete.categorical",
            &[r],
        );
        assert_eq!(m, Mechanism::EnumOverfit);
        assert_eq!(t, "enum-constraint");
    }

    /// Rule 2 — format_diversity path B fires on subtype drift within the
    /// same broad-type prefix.
    #[test]
    fn vci3_attribute_format_diversity_b_fires() {
        let (m, t) = attribute(
            "ts",
            "datetime.timestamp.iso_8601",
            "datetime.timestamp.sql_standard",
            &[semantic_pattern_reject("ts")],
        );
        assert_eq!(m, Mechanism::FormatDiversity);
        assert_eq!(t, "path-b-prefix");
    }

    /// Rule 3 — code_vs_canonical path B fires when exactly one side of
    /// the (predicted, expected) pair is in `CODE_TYPED_LABELS` AND the
    /// two sides do NOT share a broad-type prefix (otherwise Rule 2 wins
    /// per the spec's cascade order). The canonical example is a
    /// representation.text.plain_text prediction against a finance
    /// currency_code expectation — different domain prefixes, exactly
    /// one allowlisted.
    #[test]
    fn vci3_attribute_code_vs_canonical_b_fires() {
        let (m, t) = attribute(
            "currency",
            "representation.text.plain_text",
            "finance.currency.currency_code",
            &[semantic_pattern_reject("currency")],
        );
        assert_eq!(m, Mechanism::CodeVsCanonical);
        assert_eq!(t, "path-b-codetype");
    }

    /// Rule 4 — format_diversity path A fires when predicted == expected,
    /// pattern-reject is present, and the column is NOT in the seam table.
    #[test]
    fn vci3_attribute_format_diversity_a_fires() {
        let (m, t) = attribute(
            "Date",
            "datetime.date.compact_ym",
            "datetime.date.compact_ym",
            &[semantic_pattern_reject("Date")],
        );
        assert_eq!(m, Mechanism::FormatDiversity);
        assert_eq!(t, "path-a-pattern");
    }

    /// Rule 5 — code_vs_canonical path A fires when predicted == expected,
    /// pattern-reject is present, and the column IS in the seam table.
    #[test]
    fn vci3_attribute_code_vs_canonical_a_fires() {
        let (m, t) = attribute(
            "Country",
            "geography.location.country",
            "geography.location.country",
            &[semantic_pattern_reject("Country")],
        );
        assert_eq!(m, Mechanism::CodeVsCanonical);
        assert_eq!(t, "path-a-pattern");
    }

    /// Rule 6 — misclassification fires on cross-domain prediction errors.
    #[test]
    fn vci3_attribute_misclassification_fires() {
        let (m, t) = attribute(
            "email_field",
            "representation.text.plain_text",
            "identity.person.email",
            &[semantic_pattern_reject("email_field")],
        );
        assert_eq!(m, Mechanism::Misclassification);
        assert_eq!(t, "prediction-error");
    }

    // --- ac-04 negative tests (one per mechanism — guards against leaks) -

    /// Rule 1 negative — enum_overfit does NOT fire when only pattern-rejects
    /// are present (no enum-constraint failure).
    #[test]
    fn vci3_attribute_enum_overfit_negative_pattern_only() {
        let (m, t) = attribute(
            "Country",
            "geography.location.country",
            "geography.location.country",
            &[semantic_pattern_reject("Country")],
        );
        assert_ne!(m, Mechanism::EnumOverfit);
        assert_ne!(t, "enum-constraint");
    }

    /// Rule 2 negative — format_diversity-B does NOT fire on cross-domain
    /// disagreement (different broad-type prefixes); falls through to
    /// misclassification.
    #[test]
    fn vci3_attribute_format_diversity_b_negative_cross_domain() {
        let (m, t) = attribute(
            "weird",
            "datetime.timestamp.iso_8601",
            "representation.text.plain_text",
            &[semantic_pattern_reject("weird")],
        );
        assert_ne!(t, "path-b-prefix");
        assert_eq!(m, Mechanism::Misclassification);
    }

    /// Rule 3 negative — code_vs_canonical-B does NOT fire when both sides
    /// are in the allowlist (two code types disagreeing is misclassification,
    /// not code-vs-canonical).
    #[test]
    fn vci3_attribute_code_vs_canonical_b_negative_both_in_allowlist() {
        // Two distinct code-typed labels with no shared prefix (geography vs
        // finance) so Rule 2 (path-b-prefix) does NOT pre-empt the test —
        // we want the cascade to skip Rule 3 and land on Rule 6.
        let (m, t) = attribute(
            "code",
            "geography.location.country_code",
            "finance.banking.swift_bic",
            &[semantic_pattern_reject("code")],
        );
        assert_ne!(t, "path-b-codetype");
        assert_eq!(m, Mechanism::Misclassification);
    }

    /// Rule 4 negative — format_diversity-A does NOT fire when the column
    /// IS in the seam table (Rule 5 pre-empts it). Both rules report the
    /// same trigger label `path-a-pattern` (per the spec's 6-label list),
    /// so the load-bearing assertion is on the Mechanism: a seam-table
    /// hit must resolve to CodeVsCanonical, not FormatDiversity.
    #[test]
    fn vci3_attribute_format_diversity_a_negative_seam_column() {
        let (m, t) = attribute(
            "Country",
            "geography.location.country",
            "geography.location.country",
            &[semantic_pattern_reject("Country")],
        );
        assert_eq!(m, Mechanism::CodeVsCanonical);
        // The trigger label intentionally collides between Rules 4 and 5
        // (both fire on `predicted == expected + pattern-reject`). The
        // mechanism guard above is the discriminator.
        assert_eq!(t, "path-a-pattern");
    }

    /// Rule 5 negative — code_vs_canonical-A does NOT fire when the column
    /// is NOT in the seam table (Rule 4 wins).
    #[test]
    fn vci3_attribute_code_vs_canonical_a_negative_non_seam_column() {
        let (m, _t) = attribute(
            "Date",
            "datetime.date.compact_ym",
            "datetime.date.compact_ym",
            &[semantic_pattern_reject("Date")],
        );
        assert_eq!(m, Mechanism::FormatDiversity);
    }

    /// Rule 6 negative — misclassification does NOT fire when no
    /// SEMANTIC_TYPE reject is present (a TRANSFORM_FAILED-only column with
    /// predicted != expected falls through to fallthrough/unknown).
    #[test]
    fn vci3_attribute_misclassification_negative_transform_only() {
        let (m, t) = attribute(
            "weight",
            "representation.numeric.integer",
            "representation.numeric.decimal_number",
            &[transform_failed("weight")],
        );
        // Different broad prefix: representation.numeric.integer vs
        // representation.numeric.decimal_number share `representation`, so
        // Rule 2 (format_diversity-B) actually fires here. The negative
        // guard is on misclassification specifically — it must NOT win
        // when the rejects are transform-only.
        assert_ne!(m, Mechanism::Misclassification);
        assert_eq!(t, "path-b-prefix");
    }

    // --- ac-06 cascade-order test (≥4 cases, includes Rule 5 vs Rule 4) --

    /// Cascade-order test asserting the priority of the 7-rule pipeline.
    /// Each case is a (column, predicted, expected, rejects, expected
    /// mechanism, expected trigger) tuple; the test fails if any case
    /// resolves to a different bucket than the cascade specifies.
    #[test]
    fn vci3_attribute_cascade_order() {
        struct Case {
            name: &'static str,
            column: &'static str,
            predicted: &'static str,
            expected: &'static str,
            rejects: Vec<RejectRow>,
            want_mech: Mechanism,
            want_trigger: &'static str,
        }

        let cases = vec![
            // 1) Enum-constraint always wins over everything when
            //    predicted == expected, even alongside a pattern-reject.
            Case {
                name: "rule-1 enum_overfit pre-empts rule-4",
                column: "kind",
                predicted: "representation.discrete.categorical",
                expected: "representation.discrete.categorical",
                rejects: vec![enum_reject("kind"), semantic_pattern_reject("kind")],
                want_mech: Mechanism::EnumOverfit,
                want_trigger: "enum-constraint",
            },
            // 2) Path-B prefix subtype drift fires before catch-all
            //    misclassification.
            Case {
                name: "rule-2 path-b-prefix beats rule-6",
                column: "ts",
                predicted: "datetime.timestamp.iso_8601",
                expected: "datetime.timestamp.sql_standard",
                rejects: vec![semantic_pattern_reject("ts")],
                want_mech: Mechanism::FormatDiversity,
                want_trigger: "path-b-prefix",
            },
            // 3) Path-B code-typed XOR fires before catch-all
            //    misclassification — cross-prefix so Rule 2 does NOT
            //    pre-empt Rule 3.
            Case {
                name: "rule-3 path-b-codetype beats rule-6",
                column: "currency",
                predicted: "representation.text.plain_text",
                expected: "finance.currency.currency_code",
                rejects: vec![semantic_pattern_reject("currency")],
                want_mech: Mechanism::CodeVsCanonical,
                want_trigger: "path-b-codetype",
            },
            // 4) Rule 5 vs Rule 4 seam-table seam — same predicted ==
            //    expected with a pattern-reject; the seam-table membership
            //    is the discriminator (Country in seam → CvC).
            Case {
                name: "rule-5 seam (Country) beats rule-4",
                column: "Country",
                predicted: "geography.location.country",
                expected: "geography.location.country",
                rejects: vec![semantic_pattern_reject("Country")],
                want_mech: Mechanism::CodeVsCanonical,
                want_trigger: "path-a-pattern",
            },
            // 5) Rule 4 vs Rule 5 seam-table seam — non-seam column resolves
            //    to FormatDiversity even with otherwise identical inputs.
            Case {
                name: "rule-4 non-seam (Date) skips rule-5",
                column: "Date",
                predicted: "datetime.date.compact_ym",
                expected: "datetime.date.compact_ym",
                rejects: vec![semantic_pattern_reject("Date")],
                want_mech: Mechanism::FormatDiversity,
                want_trigger: "path-a-pattern",
            },
            // 6) Rule 7 fallthrough — no rejects at all, predicted ==
            //    expected (defensive: harness shouldn't call attribute()
            //    on a column with zero rejects, but the rule must be safe).
            Case {
                name: "rule-7 fallthrough on no rejects",
                column: "weight",
                predicted: "representation.numeric.decimal_number",
                expected: "representation.numeric.decimal_number",
                rejects: vec![],
                want_mech: Mechanism::Unknown,
                want_trigger: "fallthrough",
            },
        ];

        for c in &cases {
            let (m, t) = attribute(c.column, c.predicted, c.expected, &c.rejects);
            assert_eq!(
                m, c.want_mech,
                "case {} expected mechanism {:?} but got {:?}",
                c.name, c.want_mech, m
            );
            assert_eq!(
                t, c.want_trigger,
                "case {} expected trigger {:?} but got {:?}",
                c.name, c.want_trigger, t
            );
        }
    }

    // --- ac-02 CODE_TYPED_LABELS allowlist invariants --------------------

    /// CODE_TYPED_LABELS contains at least 20 entries (per ac-02 floor).
    /// The current enumeration has 36 entries across geography, finance,
    /// identity, technology — well above the floor.
    #[test]
    fn vci3_code_typed_allowlist_size() {
        assert!(
            CODE_TYPED_LABELS.len() >= 20,
            "CODE_TYPED_LABELS has {} entries; spec ac-02 requires ≥20",
            CODE_TYPED_LABELS.len()
        );
    }

    /// Every entry in CODE_TYPED_LABELS resolves to a real taxonomy label —
    /// verified by loading the bundled taxonomy via finetype-core. This is
    /// the taxonomy-existence test required by ac-02.
    ///
    /// The test resolves the labels directory relative to the workspace root
    /// (`../../labels` from the eval-crate `cargo test` cwd) so it works for
    /// both `cargo test -p finetype-eval` and `cargo test --workspace`.
    #[test]
    fn vci3_code_typed_allowlist_taxonomy_valid() {
        use finetype_core::Taxonomy;
        use std::path::PathBuf;

        // CARGO_MANIFEST_DIR resolves to crates/finetype-eval at compile time.
        // Walk up to the workspace root and into labels/.
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let labels_dir = manifest_dir
            .parent()
            .and_then(|p| p.parent())
            .map(|p| p.join("labels"))
            .expect("workspace root resolvable from CARGO_MANIFEST_DIR");

        let tax = Taxonomy::from_directory(&labels_dir).unwrap_or_else(|e| {
            panic!(
                "loading taxonomy from {} for allowlist test: {}",
                labels_dir.display(),
                e
            )
        });

        for label in CODE_TYPED_LABELS {
            assert!(
                tax.get(label).is_some(),
                "CODE_TYPED_LABELS entry '{}' is not a known taxonomy label — \
                 update labels/definitions_*.yaml or remove the entry",
                label
            );
        }
    }

    /// `is_code_typed` returns true for allowlist entries and false for a
    /// representative set of non-code labels.
    #[test]
    fn vci3_code_typed_predicate_basic() {
        assert!(is_code_typed("geography.location.country_code"));
        assert!(is_code_typed("finance.banking.swift_bic"));
        assert!(is_code_typed("identity.medical.cpt"));
        assert!(!is_code_typed("geography.location.country"));
        assert!(!is_code_typed("representation.text.plain_text"));
        assert!(!is_code_typed("datetime.date.compact_ym"));
    }

    // --- seam-table coverage (preserved from iter-2) ---------------------

    /// The seam-detection helper underpins Rules 4/5; verify it
    /// discriminates correctly so columns OUTSIDE the seam don't become
    /// CodeVsCanonical and columns INSIDE the seam don't become
    /// FormatDiversity.
    #[test]
    fn vci3_attribute_seam_table_coverage() {
        assert!(column_in_seam_table("Country"));
        assert!(column_in_seam_table("nationality_country_code"));
        assert!(column_in_seam_table("blood_type"));
        assert!(column_in_seam_table("LANGUAGE"));
        assert!(!column_in_seam_table("Mean"));
        assert!(!column_in_seam_table("Year"));
        assert!(!column_in_seam_table("IATA"));
    }
}

// ---------------------------------------------------------------------------
// Fixture tests (ac-03 loader, ac-05 regression match, ac-13 anchors,
// constraint-5 baseline pin)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod fixture_tests {
    use super::fixture::{load, FixtureRow};
    use super::report;
    use std::collections::BTreeMap;

    /// Pinned fixture row-count baseline (constraint 5). Matches the
    /// `metadata.fixture_baseline_rows` value in
    /// `spec 2026-04-28-validate-corpus-iter3`. Subsequent
    /// fixture changes that shrink the row count below this value fail
    /// the test — guards against silent fixture shrinkage, which would
    /// hide a regression in the harness's failing-column detection.
    const FIXTURE_BASELINE_ROWS: usize = 80;

    /// vci3_fixture_row_count_baseline (constraint 5)
    ///
    /// Asserts the fixture has at least the baseline row count recorded in
    /// the iter-3 spec metadata. PR descriptions MUST include a 'Fixture
    /// diff rationale' section for any net change to the row count.
    #[test]
    fn vci3_fixture_row_count_baseline() {
        let rows = load();
        assert!(
            rows.len() >= FIXTURE_BASELINE_ROWS,
            "Fixture has {} rows; spec baseline is {}. Fixture-shrinkage \
             requires a 'Fixture diff rationale' section in the PR description \
             AND a baseline reduction in spec.yaml metadata.",
            rows.len(),
            FIXTURE_BASELINE_ROWS
        );
    }

    /// vci3_fixture_iter2_anchor_rows_present (ac-13)
    ///
    /// Asserts the 5 iter-2 anchor rows are present in the fixture with
    /// matching `(dataset, column, expected_mechanism)` tuples sourced
    /// from each dataset's GT sidecar `notes:` section (NOT from harness
    /// output, NOT from iter-2 spec's illustrative table). The 4 hard
    /// anchors land as code_vs_canonical or format_diversity per iter-2
    /// curation thesis; the GICS Sector taxonomy-gap row carries
    /// `pending_escalation:true`. This is the correctness anchor (ac-13)
    /// that grounds ac-05's regression check.
    #[test]
    fn vci3_fixture_iter2_anchor_rows_present() {
        let rows = load();
        let by_key: BTreeMap<(String, String), &FixtureRow> = rows
            .iter()
            .map(|r| ((r.dataset.clone(), r.column.clone()), r))
            .collect();

        // Hard anchors — iter-2 truth, expected_mechanism strict.
        let hard_anchors: &[(&str, &str, &str)] = &[
            ("nyc_taxi", "tpep_pickup_datetime", "format_diversity"),
            ("gdelt_events", "SQLDATE", "format_diversity"),
            ("fifa_players", "Value", "code_vs_canonical"),
            ("oecd_employment", "REF_AREA", "code_vs_canonical"),
        ];
        for (d, c, want_mech) in hard_anchors {
            let row = by_key
                .get(&(d.to_string(), c.to_string()))
                .unwrap_or_else(|| panic!("hard anchor ({}, {}) missing from fixture", d, c));
            assert_eq!(
                row.expected_mechanism, *want_mech,
                "hard anchor ({}, {}) expected_mechanism mismatch: fixture has {}, want {}",
                d, c, row.expected_mechanism, want_mech
            );
        }

        // Known taxonomy-gap row — pending_escalation:true with non-empty
        // rationale referencing the follow-up obligation.
        let gap_row = by_key
            .get(&("sp500_constituents".into(), "GICS Sector".into()))
            .expect("known gap row (sp500_constituents, GICS Sector) missing from fixture");
        assert_eq!(
            gap_row.expected_mechanism, "code_vs_canonical",
            "GICS Sector row's expected_mechanism mismatch"
        );
        assert!(
            gap_row.pending_escalation,
            "GICS Sector row must have pending_escalation: true"
        );
        assert!(
            !gap_row.rationale.trim().is_empty(),
            "GICS Sector rationale must be non-empty (must reference follow-up)"
        );
    }

    /// vci3_fixture_anchor_count_3_hard_2_gap (ac-13, renamed from
    /// _4_hard_1_gap to reflect iter-3 empirical reality).
    ///
    /// **Why renamed.** The spec's original 4-hard / 1-gap framing assumed
    /// the harness would attribute oecd_employment.REF_AREA to
    /// code_vs_canonical. Iter-3 empirical reality differs: the v0.6.19
    /// model mispredicts REF_AREA to `technology.internet.http_method`
    /// (also in CODE_TYPED_LABELS), so cascade Rule 3's XOR=false and
    /// Rule 6 (misclassification) fires. The cascade is principled
    /// (cross-domain code disagreement = misclassification, not CvC).
    /// REF_AREA is preserved as iter-2 truth in the fixture but with
    /// `pending_escalation: true`, joining GICS Sector in the known-gap
    /// set. The 3-hard / 2-gap framing pins this iter-3 reality. See
    /// `progress.md` Findings: "Anchor reconciliation under iter-3
    /// empirical reality" for the full rationale.
    ///
    /// Asserts: of the 5 iter-2 anchor `(dataset, column)` tuples, exactly
    /// 3 carry `pending_escalation:false` (hard anchors that survive
    /// iter-3 attribution unchanged) AND exactly 2 carry
    /// `pending_escalation:true` (REF_AREA awaits model improvement;
    /// GICS Sector awaits taxonomy widening or value-shape signals).
    #[test]
    fn vci3_fixture_anchor_count_3_hard_2_gap() {
        let rows = load();
        let by_key: BTreeMap<(String, String), &FixtureRow> = rows
            .iter()
            .map(|r| ((r.dataset.clone(), r.column.clone()), r))
            .collect();

        let anchor_keys: &[(&str, &str)] = &[
            ("nyc_taxi", "tpep_pickup_datetime"),
            ("gdelt_events", "SQLDATE"),
            ("fifa_players", "Value"),
            ("oecd_employment", "REF_AREA"),
            ("sp500_constituents", "GICS Sector"),
        ];

        let (hard, gap): (Vec<_>, Vec<_>) = anchor_keys
            .iter()
            .map(|(d, c)| {
                let row = by_key
                    .get(&(d.to_string(), c.to_string()))
                    .unwrap_or_else(|| panic!("anchor ({}, {}) missing from fixture", d, c));
                (*d, *c, row.pending_escalation)
            })
            .partition(|(_, _, pending)| !*pending);

        assert_eq!(
            hard.len(),
            3,
            "Expected exactly 3 hard anchors (pending_escalation:false) under \
             iter-3 reality, got {}: {:?}",
            hard.len(),
            hard
        );
        assert_eq!(
            gap.len(),
            2,
            "Expected exactly 2 known-gap anchors (pending_escalation:true), \
             got {}: {:?}",
            gap.len(),
            gap
        );
    }

    /// vci3_fixture_attribution_regression_match (ac-05)
    ///
    /// REGRESSION CHECK — pins current cascade behaviour against future
    /// drift. NOT a correctness check. The correctness anchors are the 5
    /// iter-2 rows pinned by `vci3_fixture_iter2_anchor_rows_present`
    /// (ac-13); every other Phase-2 fixture row records the harness's
    /// actual output as `expected_mechanism`, so this test passing only
    /// proves the harness still does what it did when the fixture was
    /// written.
    ///
    /// Loads the most-recent harness-generated report from
    /// `eval/eval_output/validate_corpus.md` and asserts every per-column
    /// attribution it records matches the fixture's expected_mechanism.
    /// Skips `pending_escalation:true` rows (those are awaiting upstream
    /// fixes — model improvement, taxonomy widening, value-shape
    /// signals). Failure messages include `(dataset, column, expected,
    /// actual, rationale)` for clear CI diff.
    ///
    /// **A missing report file fails this test.** The report is tracked,
    /// so no checkout reaches a state where it is absent; a read error
    /// means the path moved or the working tree is damaged. Regenerate
    /// with `make validate-corpus`.
    ///
    /// `report::check_not_vacuous` rejects a run that compared nothing for
    /// a reason that reads like a pass — a renamed heading, a reshaped
    /// table, an attributions row count disagreeing with the failing-column
    /// count the report declares for itself. `mod report_tests` drives that
    /// check, and the parse under it, against rendered-then-mutated reports.
    #[test]
    fn vci3_fixture_attribution_regression_match() {
        let fixture = load();
        let path = report::report_path();

        let report = report::read_harness_report(&path).unwrap_or_else(|e| panic!("{e}"));
        let parsed = report::parse_report(&report);
        report::check_not_vacuous(&parsed)
            .unwrap_or_else(|e| panic!("{} in {}", e, path.display()));

        let (compared, failures) = report::compare_to_fixture(&parsed.rows, &fixture);
        assert!(
            failures.is_empty(),
            "Fixture-attribution regressions ({} compared OK):\n{}",
            compared,
            failures.join("\n")
        );
    }

    /// vci3_fixture_no_report_row_missing_from_fixture (ac-04)
    ///
    /// Pins the fixture's *coverage* of the report: a per-column attribution
    /// the harness emits that the fixture has no entry for.
    ///
    /// `compare_to_fixture` emits two message shapes and this reads one of
    /// them — `missing from fixture`. The other,
    /// `fixture expected=… but harness actual=…`, is a mechanism
    /// disagreement, and the open adjudications hold
    /// `vci3_fixture_attribution_regression_match` red on that shape alone.
    /// A test that is already failing cannot redden, which is why coverage
    /// gets a verdict of its own here rather than riding on that one.
    ///
    /// Mutation:
    /// `git checkout origin/main -- eval/datasets/validate_corpus_expected_attributions.yaml`.
    ///
    /// `check_not_vacuous` runs first for the reason it runs above: a report
    /// that parses no rows yields no message of either shape, so without it
    /// this assertion would pass on a renamed heading.
    #[test]
    fn vci3_fixture_no_report_row_missing_from_fixture() {
        let fixture = load();
        let path = report::report_path();

        let report = report::read_harness_report(&path).unwrap_or_else(|e| panic!("{e}"));
        let parsed = report::parse_report(&report);
        report::check_not_vacuous(&parsed)
            .unwrap_or_else(|e| panic!("{} in {}", e, path.display()));

        let (_, failures) = report::compare_to_fixture(&parsed.rows, &fixture);
        let missing: Vec<&str> = failures
            .iter()
            .filter(|f| f.contains("missing from fixture"))
            .map(String::as_str)
            .collect();
        assert!(
            missing.is_empty(),
            "{} attribution row(s) in {} have no fixture entry:\n{}",
            missing.len(),
            path.display(),
            missing.join("\n")
        );
    }
}

// ---------------------------------------------------------------------------
// Harness-report parsing tests
//
// These drive `mod report` against reports produced by `render_report` in this
// file and then mutated, so they hold whatever state the corpus is in and
// whatever the nine open fixture adjudications settle to. Each mutation is one
// a reader of the report would have to notice by eye: a renamed heading, a
// dropped column, a dropped row, a renamed count cell, a report that is not
// there at all.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod report_tests {
    use super::attribute::Mechanism;
    use super::report::{check_not_vacuous, parse_report, read_harness_report, report_path};
    use super::{render_report, DatasetResult};
    use std::collections::BTreeMap;

    const ATTRIBUTION_HEADING: &str = "## Per-column attributions";

    /// Render a report the way the harness does: dataset `alpha` clean,
    /// dataset `beta` carrying `failing` as its failing columns.
    fn rendered(failing: &[(&str, Mechanism, &'static str)]) -> String {
        let mut failing_columns: BTreeMap<String, (Mechanism, &'static str)> = BTreeMap::new();
        for (col, mech, trigger) in failing {
            failing_columns.insert((*col).to_string(), (*mech, *trigger));
        }
        let results = vec![
            DatasetResult {
                dataset: "alpha".to_string(),
                rows_total: 5,
                rows_valid: 5,
                pass_at_threshold: true,
                failing_columns: BTreeMap::new(),
            },
            DatasetResult {
                dataset: "beta".to_string(),
                rows_total: 5,
                rows_valid: 3,
                pass_at_threshold: failing.is_empty(),
                failing_columns,
            },
        ];
        render_report(&results, 0.99, None)
    }

    fn two_failing() -> String {
        rendered(&[
            ("code", Mechanism::EnumOverfit, "enum-constraint"),
            ("when", Mechanism::FormatDiversity, "path-b-prefix"),
        ])
    }

    /// A report with the heading and a well-formed table parses the rows the
    /// renderer wrote, and is not vacuous.
    #[test]
    fn vci3_report_well_formed_table_parses_its_rows() {
        let parsed = parse_report(&two_failing());

        assert!(parsed.saw_section, "attributions heading not found");
        assert!(parsed.saw_header, "attributions header row not found");
        assert!(
            parsed.saw_per_dataset_header,
            "per-dataset header row not found"
        );
        assert_eq!(
            parsed.declared_failing_columns, 2,
            "per-dataset failing-column counts did not sum to the two rendered"
        );
        assert_eq!(parsed.rows.len(), 2, "parsed rows: {:?}", parsed.rows);

        // BTreeMap order: `code` before `when`.
        assert_eq!(parsed.rows[0].dataset, "beta");
        assert_eq!(parsed.rows[0].column, "code");
        assert_eq!(parsed.rows[0].mechanism, "enum_overfit");
        assert_eq!(parsed.rows[0].trigger, "enum-constraint");
        assert_eq!(parsed.rows[1].dataset, "beta");
        assert_eq!(parsed.rows[1].column, "when");
        assert_eq!(parsed.rows[1].mechanism, "format_diversity");
        assert_eq!(parsed.rows[1].trigger, "path-b-prefix");

        check_not_vacuous(&parsed).expect("a well-formed report is not a vacuous one");
    }

    /// The goal state. `render_report` emits the attributions section under
    /// `any_failing`, so a corpus with no failing column produces a report
    /// with no such section — and that must pass, not fire the guard.
    #[test]
    fn vci3_report_no_failing_columns_needs_no_attributions() {
        let report = rendered(&[]);
        assert!(
            !report.contains(ATTRIBUTION_HEADING),
            "render_report emitted `{ATTRIBUTION_HEADING}` for a corpus with no failing \
             column, so this test no longer exercises the goal state:\n{report}"
        );

        let parsed = parse_report(&report);
        assert!(
            !parsed.saw_section,
            "found a heading the renderer did not write"
        );
        assert!(
            parsed.saw_per_dataset_header,
            "per-dataset header row not found"
        );
        assert_eq!(parsed.declared_failing_columns, 0);
        assert!(parsed.rows.is_empty(), "parsed rows: {:?}", parsed.rows);

        check_not_vacuous(&parsed)
            .expect("a report declaring no failing column needs no attributions section");
    }

    /// A renamed heading parses no rows, and the guard fires.
    #[test]
    fn vci3_report_renamed_heading_is_caught() {
        let report = two_failing().replace(ATTRIBUTION_HEADING, "## Attributions by column");
        let parsed = parse_report(&report);

        assert!(!parsed.saw_section);
        assert!(parsed.rows.is_empty(), "parsed rows: {:?}", parsed.rows);
        assert_eq!(parsed.declared_failing_columns, 2);

        let err = check_not_vacuous(&parsed)
            .expect_err("a renamed attributions heading must not pass the vacuity check");
        assert!(
            err.contains("no `## Per-column attributions` heading"),
            "{err}"
        );
    }

    /// An attributions table that lost its `Trigger` column parses no rows,
    /// and the guard fires.
    #[test]
    fn vci3_report_table_missing_a_column_is_caught() {
        let report = two_failing()
            .replace(
                "| Dataset | Column | Mechanism | Trigger |",
                "| Dataset | Column | Mechanism |",
            )
            .replace(
                "| beta | code | enum_overfit | enum-constraint |",
                "| beta | code | enum_overfit |",
            )
            .replace(
                "| beta | when | format_diversity | path-b-prefix |",
                "| beta | when | format_diversity |",
            );
        let parsed = parse_report(&report);

        assert!(parsed.saw_section, "the heading itself was not renamed");
        assert!(!parsed.saw_header);
        assert!(parsed.rows.is_empty(), "parsed rows: {:?}", parsed.rows);

        let err = check_not_vacuous(&parsed)
            .expect_err("a reshaped attributions table must not pass the vacuity check");
        assert!(err.contains("header row is not"), "{err}");
    }

    /// An attributions table missing a row the per-dataset counts account for
    /// leaves that attribution uncompared, and the guard fires.
    #[test]
    fn vci3_report_dropped_attribution_row_is_caught() {
        let report =
            two_failing().replace("| beta | when | format_diversity | path-b-prefix |\n", "");
        let parsed = parse_report(&report);

        assert!(parsed.saw_section);
        assert!(parsed.saw_header);
        assert_eq!(parsed.declared_failing_columns, 2);
        assert_eq!(parsed.rows.len(), 1, "parsed rows: {:?}", parsed.rows);

        let err = check_not_vacuous(&parsed)
            .expect_err("an attributions table short of the declared count must not pass");
        assert!(err.contains("went uncompared"), "{err}");
    }

    /// The signal the guard reads is itself guarded: a per-dataset table whose
    /// failing-column cell has been renamed cannot say whether an absent
    /// attributions section is the goal state or a rename, so it is an error.
    #[test]
    fn vci3_report_unreadable_failing_count_is_caught() {
        let report = two_failing().replace("Failing columns", "Failures");
        let parsed = parse_report(&report);

        assert!(!parsed.saw_per_dataset_header);
        assert_eq!(parsed.declared_failing_columns, 0);

        let err = check_not_vacuous(&parsed)
            .expect_err("an unreadable failing-column count must not pass the vacuity check");
        assert!(err.contains("could not be read"), "{err}");
    }

    /// The header cell is not the whole of that signal: a `## Per-dataset`
    /// *data* row whose count cell is not a number contributes zero to the
    /// declared total, which is the goal state's own value.
    ///
    /// Two mutations, the second built on the first. `beta`'s count cell
    /// alone drops the declared total to `alpha`'s zero. Rename the
    /// attributions heading on top of that and the parse reaches the shape
    /// the guard exists to reject: a header that was read, a total of zero,
    /// no attributions section and no rows — which without this check is
    /// `Ok(())`, and reads exactly like a clean corpus.
    #[test]
    fn vci3_report_unreadable_failing_cell_is_caught() {
        let report = two_failing();
        let malformed = report.replace(
            "| beta | 5 | 3 | ✗ | 2 |",
            "| beta | 5 | 3 | ✗ | 2 columns |",
        );
        assert_ne!(
            malformed, report,
            "the per-dataset row this test mutates is not in the rendered report:\n{report}"
        );

        let parsed = parse_report(&malformed);
        assert!(
            parsed.saw_per_dataset_header,
            "the header cell is untouched by this mutation"
        );
        assert_eq!(
            parsed.declared_failing_columns, 0,
            "beta's count was expected to drop out of the sum, leaving alpha's zero"
        );
        let err = check_not_vacuous(&parsed)
            .expect_err("an unreadable failing-column cell must not pass the vacuity check");
        assert!(err.contains("2 columns"), "{err}");

        let and_renamed = malformed.replace(ATTRIBUTION_HEADING, "## Attributions by column");
        let parsed = parse_report(&and_renamed);
        assert!(parsed.saw_per_dataset_header);
        assert_eq!(parsed.declared_failing_columns, 0);
        assert!(!parsed.saw_section);
        assert!(parsed.rows.is_empty(), "parsed rows: {:?}", parsed.rows);

        let err = check_not_vacuous(&parsed).expect_err(
            "a report that compared no rows because its count cell was unreadable must \
             not pass the vacuity check",
        );
        assert!(err.contains("2 columns"), "{err}");
    }

    /// The alignment row under the `## Per-dataset` header carries `---:` in
    /// the count position and is not a malformed count. Without a skip for
    /// it the guard fires on the report the harness itself writes, which
    /// `vci3_report_well_formed_table_parses_its_rows` and
    /// `vci3_report_no_failing_columns_needs_no_attributions` also hold —
    /// this names the row so the reason is not inferred from their failure.
    #[test]
    fn vci3_report_alignment_row_is_not_an_unreadable_count() {
        let report = two_failing();
        assert!(
            report.contains("|---|---:|---:|:---:|---:|---|"),
            "the per-dataset alignment row this test names is not in the rendered \
             report:\n{report}"
        );

        let parsed = parse_report(&report);
        assert!(
            parsed.unreadable_failing_cells.is_empty(),
            "unreadable cells: {:?}",
            parsed.unreadable_failing_cells
        );
    }

    /// An absent report is an error, not a skip.
    #[test]
    fn vci3_report_absent_file_is_an_error() {
        let missing = report_path().with_file_name("validate_corpus.absent-on-purpose.md");
        assert!(
            !missing.exists(),
            "this test needs a path that does not exist: {}",
            missing.display()
        );

        let err = read_harness_report(&missing)
            .expect_err("a missing harness report must be an error, not a skip");
        assert!(err.contains("make validate-corpus"), "{err}");
    }
}

// ---------------------------------------------------------------------------
// Amount widening tests (spec 2026-04-29-validate-corpus-iter4 ac-02..ac-04)
//
// These tests load the real `labels/` taxonomy, compile validators, and assert
// that `finance.currency.amount`'s widened pattern (per ac-01) accepts plain
// bare-decimal money values, preserves the existing US/international,
// currency-prefix, and accounting-paren formats, and continues to reject
// clearly-non-amount tokens. The 4th alternation is borrowed verbatim from
// `representation.numeric.decimal_number`'s pattern at
// `labels/definitions_representation.yaml:79`. See MADR 0078.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod amount_widening_tests {
    use finetype_core::taxonomy::Taxonomy;
    use std::path::PathBuf;

    /// Resolve the workspace `labels/` directory from CARGO_MANIFEST_DIR
    /// (which points at `crates/finetype-eval` during `cargo test`).
    fn labels_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("labels")
    }

    fn load_amount_taxonomy() -> Taxonomy {
        let mut taxonomy =
            Taxonomy::from_directory(labels_dir()).expect("taxonomy loads from labels/");
        taxonomy.compile_validators();
        taxonomy
    }

    fn assert_amount(taxonomy: &Taxonomy, value: &str, want_valid: bool, label: &str) {
        let validator = taxonomy
            .get_validator("finance.currency.amount")
            .expect("finance.currency.amount has a compiled validator");
        let got = validator.is_valid(value);
        assert_eq!(
            got, want_valid,
            "finance.currency.amount.is_valid({:?}) = {}, want {} ({})",
            value, got, want_valid, label
        );
    }

    /// ac-02 — bare-decimal values match the widened amount regex.
    ///
    /// These are the values that surfaced the format-diversity gap in the
    /// discovery vehicle `eval/datasets/csv/ecommerce_orders.csv` (column
    /// `total_price`): plain unsymboled decimals with 1–4 integer digits, no
    /// comma grouping. Pre-widening these rejected against the comma-required
    /// alternations 1–3; post-widening they match alternation 4 (borrowed
    /// verbatim from `representation.numeric.decimal_number`).
    #[test]
    fn vci4_amount_accepts_bare_decimal() {
        let t = load_amount_taxonomy();
        // Discovery vehicle samples (1914.96 was the canonical failing value).
        assert_amount(&t, "1914.96", true, "1914.96 (4-digit decimal, no comma)");
        assert_amount(&t, "19.95", true, "19.95 (typical price)");
        assert_amount(&t, "100", true, "100 (integer-only)");
        assert_amount(&t, "0.50", true, "0.50 (sub-dollar decimal)");
        assert_amount(&t, "1234.56", true, "1234.56 (4-digit decimal)");
        // Negative bare decimal.
        assert_amount(&t, "-50.5", true, "-50.5 (negative decimal)");
        assert_amount(&t, "-99.99", true, "-99.99 (negative two-decimal)");
        // Sci-notation tail (carried verbatim from decimal_number).
        assert_amount(&t, "1e6", true, "1e6 (scientific notation)");
        assert_amount(&t, "1.5E+10", true, "1.5E+10 (uppercase + signed exp)");
        assert_amount(&t, "6e-04", true, "6e-04 (negative exponent)");
    }

    /// ac-03 — the pre-existing US/international, currency-prefix, and
    /// accounting-paren alternations continue to match unchanged. Drift here
    /// would mean the widening damaged the original three alternations.
    #[test]
    fn vci4_amount_preserves_existing_formats() {
        let t = load_amount_taxonomy();
        // Alternation 1 — US/international, optional symbol prefix, comma
        // thousands, period decimal.
        assert_amount(&t, "$1,234.56", true, "$1,234.56");
        assert_amount(&t, "£10,000.00", true, "£10,000.00");
        assert_amount(&t, "¥1,000", true, "¥1,000");
        assert_amount(&t, "$0.50", true, "$0.50");
        assert_amount(&t, "$42.00", true, "$42.00");
        assert_amount(&t, "$1,000,000.00", true, "$1,000,000.00");
        // Alternation 2 — leading negative outside currency symbol.
        assert_amount(&t, "-$999.99", true, "-$999.99");
        assert_amount(&t, "-£250.75", true, "-£250.75");
        // Alternation 3 — accounting parens for negative.
        assert_amount(&t, "($1,234.56)", true, "($1,234.56)");
        assert_amount(&t, "($999.99)", true, "($999.99)");
    }

    /// ac-04 — clearly-non-amount tokens still reject. The widening borrows
    /// `decimal_number`'s pattern verbatim, which already rejects malformed
    /// numerics like "1.2.3", "12px", "1e", "e5", and free text like "abc".
    /// This test is the precision floor: the widening must not turn the
    /// validator into a permissive catch-all.
    #[test]
    fn vci4_amount_rejects_non_money() {
        let t = load_amount_taxonomy();
        // Free text.
        assert_amount(&t, "abc", false, "abc");
        assert_amount(&t, "hello world", false, "hello world");
        assert_amount(&t, "", false, "empty string");
        // Malformed numerics (covered by decimal_number's tight pattern).
        assert_amount(&t, "1.2.3", false, "1.2.3 (multi-dot)");
        assert_amount(&t, "12px", false, "12px (unit suffix)");
        assert_amount(&t, "1e", false, "1e (incomplete sci)");
        assert_amount(&t, "e5", false, "e5 (no mantissa)");
        assert_amount(&t, ".5", false, ".5 (leading dot)");
        assert_amount(&t, "5.", false, "5. (trailing dot)");
        // Mismatched currency formatting.
        assert_amount(&t, "$1,234.567", false, "$1,234.567 (3 decimal digits)");
    }
}
