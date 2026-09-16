//! `profile` command: discover and classify column types.

use super::*;
use finetype_cli::enum_emission::collect_unique_values_if_categorical;
use finetype_core::enum_domain::{detect_enum_domain, EnumConfig, EnumDomain};
use finetype_mcp::datapackage;
use rayon::prelude::*;

// Honest confidence signal (spec 2026-06-18-calibrated-confidence-abstention).
// The shipped confidence ranks correctness but is not calibrated (over-confident
// in the high bins), so we surface a quality BAND over the ranking rather than the
// raw number. Thresholds are the data-driven knees from the gold+representative
// reliability curve (output/calibrated-confidence/ac00_reliability_finding.md):
// below 0.70 accuracy drops to ~coin-flip on representative data; >=0.85 is the
// trust band. Below the LOW threshold the runner-up type is surfaced so a shaky
// column reads "probably X, maybe Y" instead of a bare guess.
const QUALITY_HIGH_THRESHOLD: f32 = 0.85;
const QUALITY_LOW_THRESHOLD: f32 = 0.70;

pub(crate) fn quality_band_label(confidence: f32) -> &'static str {
    if confidence >= QUALITY_HIGH_THRESHOLD {
        "high"
    } else if confidence >= QUALITY_LOW_THRESHOLD {
        "medium"
    } else {
        "low"
    }
}

/// The second-best vote, surfaced only on the `low` band (and only when it
/// differs from the emitted label). `vote_distribution` is empty on rule/veto
/// paths, which sit above the low threshold anyway, so this is `None` there.
pub(crate) fn low_band_runner_up(
    label: &str,
    confidence: f32,
    votes: &[(String, f32)],
) -> Option<String> {
    if confidence >= QUALITY_LOW_THRESHOLD {
        return None;
    }
    votes
        .iter()
        .map(|(l, _)| l)
        .find(|l| l.as_str() != label)
        .cloned()
}

/// One column's object in the `-o json` output.
///
/// A **nominated** column publishes `"nominated": true` and none of
/// `confidence`, `quality_band`, `runner_up`: each of those describes the
/// classifier's answer, and for a nominated column that answer was discarded.
/// It keeps every per-column statistic — `samples_used`, `non_null`, `null`,
/// `locale` — because those are facts about the data, computed on the same pass
/// and true whoever chose the label. `broad_type`, `format_string` and
/// `transform` are read off the nominated label, not off the discarded answer,
/// because the whole loop looks them up from the final label.
///
/// The validation keys are absent for a different reason: they were never
/// computed. See [`decide_label`].
fn json_column_object(p: &ColProfile, verbose: bool) -> serde_json::Value {
    let mut obj = serde_json::Map::new();
    obj.insert("column".to_string(), json!(p.name));
    obj.insert("type".to_string(), json!(p.label));
    if p.nominated {
        obj.insert("nominated".to_string(), json!(true));
    } else {
        obj.insert("confidence".to_string(), json!(p.confidence));
        obj.insert("quality_band".to_string(), json!(p.quality_band));
        if let Some(ru) = &p.runner_up {
            obj.insert("runner_up".to_string(), json!(ru));
        }
    }
    let resolved_broad = resolve_broad_type_display(p.broad_type.as_deref(), &p.unique_values);
    obj.insert("broad_type".to_string(), json!(resolved_broad));
    if let Some(fs) = &p.format_string {
        obj.insert("format_string".to_string(), json!(fs));
    }
    if let Some(tr) = &p.transform {
        obj.insert("transform".to_string(), json!(tr));
    }
    obj.insert("is_generic".to_string(), json!(p.is_generic));
    obj.insert("samples_used".to_string(), json!(p.samples_used));
    obj.insert("non_null".to_string(), json!(p.non_null_count));
    obj.insert("null".to_string(), json!(p.null_count));
    if p.disambiguation_applied {
        obj.insert("disambiguation_applied".to_string(), json!(true));
        if let Some(rule) = &p.disambiguation_rule {
            obj.insert("disambiguation_rule".to_string(), json!(rule));
        }
    }
    if let Some(locale) = &p.detected_locale {
        obj.insert("locale".to_string(), json!(locale));
    }
    // ac-06: validation-as-veto signals. pass_rate is
    // emitted whenever the predicted type had an applicable
    // validation; the veto/advisory flags and the original
    // label surface only when they fired.
    if let Some(rate) = p.validation_pass_rate {
        let r = (rate * 10000.0).round() / 10000.0;
        obj.insert("validation_pass_rate".to_string(), json!(r));
    }
    if p.validation_vetoed {
        obj.insert("validation_vetoed".to_string(), json!(true));
        if let Some(vt) = &p.vetoed_type {
            obj.insert("vetoed_type".to_string(), json!(vt));
        }
    }
    if p.validation_advisory_low {
        obj.insert("validation_advisory_low".to_string(), json!(true));
    }
    // x-finetype-unknown-reason: why an `unknown` column stayed
    // untyped — parity with the json-schema surface (ab75b83) so
    // the explanation reads identically across CLI json,
    // json-schema and MCP. None for any typed column.
    if let Some(reason) = unknown_reason_for(p) {
        obj.insert("x-finetype-unknown-reason".to_string(), json!(reason));
    }
    // Include unique values for categorical columns in verbose mode
    if verbose {
        if let Some(ref uv) = p.unique_values {
            obj.insert("unique_values".to_string(), json!(uv));
        }
    }
    // x-finetype-enum: the observed OPEN bounded domain, for
    // any non-denylisted column (choice 0102). Descriptive
    // metadata — distinct from the validation `enum` keyword.
    if let Some(ref ed) = p.enum_domain {
        obj.insert(
            "x-finetype-enum".to_string(),
            json!({
                "open": ed.open,
                "distinct": ed.distinct,
                "rows": ed.rows,
                "cohesion": (ed.cohesion * 1000.0).round() / 1000.0,
                "domain": ed.domain,
            }),
        );
    }
    if false {
        // validate removed (AC-10)
        match &p.quality {
            Some(q) => {
                let r = |v: f64| (v * 10000.0).round() / 10000.0;
                obj.insert(
                    "quality".to_string(),
                    json!({
                        "valid": q.valid_count,
                        "invalid": q.invalid_count,
                        "null": q.null_count,
                        "type_conforming_rate": r(q.score.type_conforming_rate),
                        "null_rate": r(q.score.null_rate),
                        "completeness": r(q.score.completeness),
                        "quality_score": r(q.score.quality_score),
                    }),
                );
                if !q.invalid_samples.is_empty() {
                    obj.insert("invalid_samples".to_string(), json!(q.invalid_samples));
                }
            }
            None => {
                obj.insert("quality".to_string(), json!(null));
            }
        }
    }
    serde_json::Value::Object(obj)
}

/// One row of the `-o plain` table.
///
/// A **nominated** column reads `decl` where an inferred one reads a
/// percentage, and carries no confidence band: `high`/`medium`/`low` rank how
/// far to trust a guess, and there is no guess here to rank. The veto
/// annotations are absent for a different reason — not suppressed at print
/// time, but never computed, because `decide_label` does not run the veto for a
/// nominated column. That distinction is deliberate: a print-time suppression
/// would hide a veto that had in fact fired.
fn plain_row(p: &ColProfile) -> String {
    let conf_str = if p.nominated {
        "decl".to_string()
    } else if p.non_null_count > 0 {
        format!("{:.1}%", p.confidence * 100.0)
    } else {
        "—".to_string()
    };
    let broad = resolve_broad_type_display(p.broad_type.as_deref(), &p.unique_values);
    let disambig = if p.disambiguation_applied {
        format!(" [{}]", p.disambiguation_rule.as_deref().unwrap_or("rule"))
    } else {
        String::new()
    };
    let locale_str = if let Some(locale) = &p.detected_locale {
        format!(" locale:{}", locale)
    } else {
        String::new()
    };
    // ac-06: annotate a hard veto (predicted type NULLed) or an
    // advisory low-pass (sub-threshold but not audited-safe).
    let veto_str = if p.validation_vetoed {
        let rate = p.validation_pass_rate.unwrap_or(0.0) * 100.0;
        format!(
            " ⊘ vetoed:{} ({:.0}% pass)",
            p.vetoed_type.as_deref().unwrap_or("?"),
            rate
        )
    } else if p.validation_advisory_low {
        let rate = p.validation_pass_rate.unwrap_or(0.0) * 100.0;
        format!(" ⚠ low-pass {:.0}% (advisory)", rate)
    } else {
        String::new()
    };
    // Honest confidence band: call out medium/low only — `high` is
    // the default expectation, so an unmarked row reads as trusted.
    let band_str = if p.nominated {
        String::new()
    } else {
        match p.quality_band {
            "low" => match &p.runner_up {
                Some(ru) => format!(" ⚑ low (maybe {ru})"),
                None => " ⚑ low".to_string(),
            },
            "medium" => " ~ medium".to_string(),
            _ => String::new(),
        }
    };
    format!(
        "  {:<25} {:<38} {:>8} {:>6}{}{}{}{}",
        p.name, p.label, broad, conf_str, disambig, locale_str, veto_str, band_str
    )
}

/// A column's final label, and every signal that depends on how it was reached.
struct LabelOutcome {
    label: String,
    nominated: bool,
    validation_pass_rate: Option<f64>,
    validation_vetoed: bool,
    validation_advisory_low: bool,
    vetoed_type: Option<String>,
    fallback_rule: Option<&'static str>,
}

/// Decide a column's final label.
///
/// **A nomination is taken as given.** With one, the label is the nominated one
/// and the validation-as-veto does not run: the veto's job is to stop the model
/// asserting a type the data contradicts, and there is no model assertion here
/// to stop. A person who declares a column's type has said what it IS; data
/// that has drifted from it is a `ft_validate` finding against the schema, not
/// grounds for FineType to publish a different type than the one it was told.
///
/// Without one, this is exactly the path it always was: evaluate the veto
/// against the predicted label, and resolve a hard veto into a residual.
///
/// `predicted` is the classifier's answer. It is computed for a nominated
/// column too — the model is loaded for the file regardless, so skipping one
/// column's forward pass saves nothing measurable, and the per-column
/// statistics the `json` output publishes for EVERY column come off the same
/// pass.
fn decide_label(
    nomination: Option<&str>,
    predicted: &str,
    values: &[String],
    taxonomy: Option<&finetype_core::Taxonomy>,
    veto_safe: &std::collections::HashSet<String>,
    veto_enabled: bool,
) -> LabelOutcome {
    if let Some(label) = nomination {
        return LabelOutcome {
            label: label.to_string(),
            nominated: true,
            validation_pass_rate: None,
            validation_vetoed: false,
            validation_advisory_low: false,
            vetoed_type: None,
            fallback_rule: None,
        };
    }

    let (validation_pass_rate, validation_vetoed, validation_advisory_low) =
        col_validation_veto(predicted, values, taxonomy, veto_safe, veto_enabled);
    let (label, vetoed_type, fallback_rule) =
        resolve_veto_outcome(validation_vetoed, predicted, values);
    LabelOutcome {
        label,
        nominated: false,
        validation_pass_rate,
        validation_vetoed,
        validation_advisory_low,
        vetoed_type,
        fallback_rule,
    }
}

struct ColProfile {
    name: String,
    label: String,
    /// Whether `label` was DECLARED by the caller (`--nominations`) rather than
    /// inferred. A nominated column publishes no confidence, no quality band,
    /// no runner-up and no veto signal: the classifier still ran, and every one
    /// of those is a fact about the answer it gave, which was discarded.
    nominated: bool,
    confidence: f32,
    samples_used: usize,
    non_null_count: usize,
    null_count: usize,
    disambiguation_applied: bool,
    disambiguation_rule: Option<String>,
    detected_locale: Option<String>,
    // Taxonomy contract fields
    broad_type: Option<String>,
    format_string: Option<String>,
    transform: Option<String>,
    is_generic: bool,
    // Validation quality fields
    quality: Option<ColProfileQuality>,
    // Unique values for ENUM/categorical columns
    unique_values: Option<Vec<String>>,
    // ac-06 validation-as-veto: fraction of sample values passing
    // the predicted type's validation (None = no applicable
    // validation), whether that triggered a HARD veto (label NULLed
    // to "unknown"), an ADVISORY low-pass flag (sub-threshold but the
    // type is not audited-safe — surfaced, not NULLed), and the
    // original predicted label when hard-vetoed.
    validation_pass_rate: Option<f64>,
    validation_vetoed: bool,
    validation_advisory_low: bool,
    vetoed_type: Option<String>,
    // x-finetype-enum: the column's OBSERVED open bounded domain
    // (spec 2026-06-17-enum-domain-emission, choice 0102). Descriptive —
    // emitted as an extension, NOT the validation-enforced `enum` keyword,
    // which stays conservative via `unique_values`.
    enum_domain: Option<EnumDomain>,
    // Honest confidence signal (spec 2026-06-18-calibrated-confidence-abstention):
    // a quality band (high/medium/low) over the existing confidence, plus the
    // runner-up type on the `low` band. Purely additive — the predicted label
    // and raw confidence are unchanged.
    quality_band: &'static str,
    runner_up: Option<String>,
}

/// Synthesise a human-readable reason an `unknown` column could not be
/// typed, so an analyst sees WHY a column is untyped, not just THAT it
/// is (card 0020, honest typing — Pillar 1). `None` for typed columns.
///
/// Three causes, in order of specificity: a tighter type was predicted
/// but its format validation rejected the values (the hard veto); too
/// few values to judge; or the model found no confident type.
fn unknown_reason_for(p: &ColProfile) -> Option<String> {
    if p.label != "unknown" {
        return None;
    }
    if p.validation_vetoed {
        let rejected = p.vetoed_type.as_deref().unwrap_or("a tighter type");
        let leaf = rejected.rsplit('.').next().unwrap_or(rejected);
        return Some(match p.validation_pass_rate {
            Some(r) => format!(
                "validation rejected '{}': only {}% of values matched its format",
                leaf,
                (r * 100.0).round() as i64
            ),
            None => format!("validation rejected '{}'", leaf),
        });
    }
    if p.non_null_count < 3 {
        return Some("too few non-null values to classify".to_string());
    }
    Some("no type matched with sufficient confidence".to_string())
}

/// Per-column validation + quality data.
struct ColProfileQuality {
    valid_count: usize,
    invalid_count: usize,
    null_count: usize,
    score: finetype_core::ColumnQualityScore,
    invalid_samples: Vec<String>,
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn cmd_profile(
    file: Option<PathBuf>,
    files: Option<PathBuf>,
    out_dir: Option<PathBuf>,
    output: OutputFormat,
    sample_size: usize,
    delimiter: Option<char>,
    no_header_hint: bool,
    enum_threshold: usize,
    stats: bool,
    verbose: bool,
    raw_model: bool,
    no_validation_veto: bool,
    nominations_path: Option<PathBuf>,
) -> Result<()> {
    use finetype_cli::nominations::Nominations;
    use finetype_model::{ColumnClassifier, ColumnConfig};
    use std::io::Write as _;

    // Batch mode (--files) currently writes one output per input to
    // <out_dir>/<stem>.<ext>. Stems are taken from the input file stem;
    // ext is chosen per output format (json for json/json-schema, csv
    // for csv, txt for plain, md for markdown, arrow for arrow).
    let batch_mode = files.is_some();
    let paths: Vec<PathBuf> = if let Some(ref single) = file {
        vec![single.clone()]
    } else {
        let files_list = files.as_ref().expect("either file or files is required");
        std::fs::read_to_string(files_list)
            .map_err(|e| anyhow::anyhow!("could not read --files list {:?}: {}", files_list, e))?
            .lines()
            .map(str::trim)
            .filter(|s| !s.is_empty() && !s.starts_with('#'))
            .map(PathBuf::from)
            .collect()
    };
    if paths.is_empty() {
        return Err(anyhow::anyhow!("no input paths to profile"));
    }

    // Nominations are read and checked BEFORE the model is loaded. Everything
    // wrong with the file — a bad key, a missing `label`, a stem no input
    // matches — is knowable from the file and the input list alone, and a
    // person who mistyped a label should hear about it now rather than after a
    // multi-second model load and a full profiling pass.
    let nominations_origin = nominations_path
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    let nominations: Option<Nominations> = match nominations_path.as_ref() {
        None => None,
        Some(path) => {
            let n = Nominations::load(path)?;
            let stems: Vec<String> = paths
                .iter()
                .map(|p| {
                    p.file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or_default()
                        .to_string()
                })
                .collect();
            n.check_stems(&stems, &nominations_origin)?;
            Some(n)
        }
    };
    let nominations = nominations.as_ref();
    let batch_ext = match output {
        OutputFormat::Json | OutputFormat::JsonSchema | OutputFormat::Datapackage => "json",
        OutputFormat::Csv => "csv",
        OutputFormat::Plain => "txt",
        OutputFormat::Markdown => "md",
        OutputFormat::Arrow => "arrow",
    };
    if batch_mode {
        // Batch mode routes the json-schema and datapackage outputs through
        // the per-file writer. The other format branches still use
        // `println!` to stdout, which would interleave outputs and ignore
        // --out-dir. Refuse early until they're converted.
        if !matches!(output, OutputFormat::JsonSchema | OutputFormat::Datapackage) {
            let mut cmd = <Cli as clap::CommandFactory>::command();
            let err = cmd.error(
                clap::error::ErrorKind::ArgumentConflict,
                "--files currently requires -o json-schema or -o datapackage (other formats not yet wired through the per-file writer)",
            );
            err.exit();
        }
        if let Some(ref od) = out_dir {
            std::fs::create_dir_all(od)
                .map_err(|e| anyhow::anyhow!("could not create --out-dir {:?}: {}", od, e))?;
        }
    }

    let model = resolve_model_path();

    eprintln!("Loading model from {:?}", model);
    let config = ColumnConfig {
        sample_size,
        ..Default::default()
    };
    let mb = load_multi_branch_classifier(&model)?;
    eprintln!(
        "Loaded multi-branch classifier ({} classes)",
        mb.n_classes()
    );
    let mut column_classifier = ColumnClassifier::with_multi_branch(mb, config);

    // Load taxonomy for validation-based attractor demotion (Rule 14)
    // Pre-compile validators for the hot path
    let taxonomy_path = std::path::PathBuf::from("labels");
    if let Ok(mut taxonomy) = load_taxonomy(&taxonomy_path) {
        taxonomy.compile_validators();
        taxonomy.compile_locale_validators();
        eprintln!(
            "Loaded taxonomy for attractor demotion ({} types, {} validators cached, {} with locale validators)",
            taxonomy.labels().len(),
            taxonomy.validator_count(),
            taxonomy.locale_validator_count()
        );
        column_classifier.set_taxonomy(taxonomy);
    }

    // Wire the shared Model2Vec encoder the multi-branch header branch needs.
    wire_model2vec(&mut column_classifier);

    // Diagnostic: skip Sharpen post-processing for ablation studies
    if raw_model {
        column_classifier.set_skip_sharpen(true);
        eprintln!("WARNING: --raw-model active — Sharpen post-processing disabled");
    }

    // Load taxonomy for enrichment ONCE for the whole batch (reused across
    // every file in the per-file loop below). `taxonomy_path` is already bound
    // above for the classifier's validation taxonomy. Hoisting this out of the
    // loop is the batch-mode amortisation point — load + validator-compile of
    // 245 types is per-batch work, not per-file (accuracy-identical; the loop
    // body only ever reads `enrichment_taxonomy` immutably after this).
    let mut enrichment_taxonomy = load_taxonomy(&taxonomy_path).ok();

    // A nominated label is checked against the taxonomy the run is actually
    // using, before any column is classified. Without a taxonomy there is
    // nothing to check it against, and a nomination whose bounds could not be
    // looked up would publish a bare label — which is what nominating was for.
    if let Some(n) = nominations {
        match enrichment_taxonomy.as_ref() {
            Some(taxonomy) => n.check_against_taxonomy(taxonomy, &nominations_origin)?,
            None => anyhow::bail!(
                "--nominations {} needs the bundled taxonomy at `labels/` to check the labels it \
                 declares; run from the FineType source tree or ship with embedded taxonomy",
                nominations_origin
            ),
        }
    }

    // ac-06: validation-as-veto. Compile the enrichment taxonomy's validators
    // once for the batch (the per-column veto checks sample values against the
    // predicted type's schema) and load the audited-safe allowlist that scopes
    // the HARD veto. Skipped entirely under --no-validation-veto.
    let veto_enabled = !no_validation_veto;
    let veto_safe = if veto_enabled {
        finetype_core::audited_safe_labels()
    } else {
        std::collections::HashSet::new()
    };
    if veto_enabled {
        if let Some(ref mut tax) = enrichment_taxonomy {
            tax.compile_validators();
        }
    }

    // Count successfully-profiled files so a batch where every file fails
    // (e.g. a systemic error) exits non-zero rather than silently producing
    // nothing (see the post-loop backstop).
    let mut batch_success = 0usize;

    // Per-file loop. Model + taxonomy + classifier are loaded above and
    // reused across iterations — that's the batch-mode amortisation
    // point. Single-file mode (--file) runs this loop once with stdout
    // as the writer; batch mode (--files + --out-dir) loops over the
    // listed paths and routes each iteration's output to a file.
    for path in &paths {
        let file: &std::path::Path = path.as_path();

        eprintln!("Reading {:?}", file);

        // Detect file format by extension
        let ext = file
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_default();
        let is_json_input = matches!(ext.as_str(), "json" | "ndjson" | "jsonl");

        // Read BEFORE opening the output writer so a file that fails to ingest
        // leaves no empty output behind. In batch mode an unreadable file is
        // skipped-and-logged — one bad file must not abort the whole `--files`
        // run; in single-file mode the error propagates as before.
        let read_result = if is_json_input {
            read_json_input(file, &ext)
        } else {
            read_csv_input(file, delimiter)
        };
        let (headers, columns, row_count) = match read_result {
            Ok(r) => r,
            Err(e) => {
                if batch_mode {
                    eprintln!("WARNING: skipping {:?}: {}", file, e);
                    continue;
                }
                return Err(e);
            }
        };

        let mut writer: Box<dyn std::io::Write> = if batch_mode {
            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("output");
            let out_path = out_dir
                .as_ref()
                .expect("--out-dir is required with --files (clap-validated)")
                .join(format!("{}.{}", stem, batch_ext));
            Box::new(std::io::BufWriter::new(
                std::fs::File::create(&out_path).map_err(|e| {
                    anyhow::anyhow!("could not create output {:?}: {}", out_path, e)
                })?,
            ))
        } else {
            Box::new(std::io::BufWriter::new(std::io::stdout()))
        };

        // The nominations file keys on the input's file stem.
        let stem = file
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default();
        if let Some(n) = nominations {
            n.check_columns(stem, &headers, file, &nominations_origin)?;
        }

        let n_cols = headers.len();
        eprintln!("Read {} rows", row_count);

        // Per-column classification.
        //
        // One column's profile depends on nothing but that column, so the loop
        // is data-parallel. `par_iter().enumerate()` is an INDEXED parallel
        // iterator and `collect::<Result<Vec<_>, _>>()` from one yields results
        // in input order — the emitted column sequence still matches the input
        // file's column sequence, which eval fixtures and the DuckDB extension
        // both depend on. Nothing here may collect into an unordered container.
        //
        // There used to be a second, cross-column path here, taken whenever a
        // sibling-context model happened to be on disk. No released binary ever
        // embedded that model, so no user ever took it.
        let profiles: Vec<ColProfile> = columns
            .par_iter()
            .enumerate()
            .map(|(i, col_values)| -> anyhow::Result<ColProfile> {
                let name = headers
                    .get(i)
                    .cloned()
                    .unwrap_or_else(|| format!("col_{}", i));
                let null_count = row_count - col_values.len();

                if col_values.is_empty() {
                    // A column with no values still carries its declared type:
                    // nothing was inferred here, so there is nothing for the
                    // absence of data to overturn.
                    let nomination = nominations.and_then(|n| n.get(stem, &name));
                    return Ok(ColProfile {
                        name,
                        label: nomination
                            .map(|d| d.label.clone())
                            .unwrap_or_else(|| "unknown".to_string()),
                        nominated: nomination.is_some(),
                        quality_band: "low",
                        runner_up: None,
                        confidence: 0.0,
                        samples_used: 0,
                        non_null_count: 0,
                        null_count,
                        disambiguation_applied: false,
                        disambiguation_rule: None,
                        detected_locale: None,
                        broad_type: None,
                        format_string: None,
                        transform: None,
                        is_generic: false,
                        quality: None,
                        unique_values: None,
                        validation_pass_rate: None,
                        validation_vetoed: false,
                        validation_advisory_low: false,
                        vetoed_type: None,
                        enum_domain: None,
                    });
                }

                // For JSON paths, extract the leaf as header hint (e.g., "users[].email" → "email")
                let header_hint = if is_json_input {
                    path_leaf(&name)
                } else {
                    name.clone()
                };

                let result = if no_header_hint {
                    column_classifier.classify_column(col_values)?
                } else {
                    column_classifier.classify_column_with_header(col_values, &header_hint)?
                };

                let outcome = decide_label(
                    nominations
                        .and_then(|n| n.get(stem, &name))
                        .map(|d| d.label.as_str()),
                    &result.label,
                    col_values,
                    enrichment_taxonomy.as_ref(),
                    &veto_safe,
                    veto_enabled,
                );
                let LabelOutcome {
                    label: final_label,
                    nominated,
                    validation_pass_rate: vp_rate,
                    validation_vetoed: vetoed,
                    validation_advisory_low: advisory_low,
                    vetoed_type,
                    fallback_rule,
                } = outcome;
                let mut result = result;
                if let Some(rule) = fallback_rule {
                    result.disambiguation_applied = true;
                    result.disambiguation_rule = Some(rule.to_string());
                }

                // Look up taxonomy contract fields for the (possibly vetoed) label
                let (broad_type, format_string, transform) =
                    if let Some(ref taxonomy) = enrichment_taxonomy {
                        if let Some(def) = taxonomy.get(&final_label) {
                            (
                                def.broad_type.clone(),
                                def.format_string.clone(),
                                def.transform.clone(),
                            )
                        } else {
                            (None, None, None)
                        }
                    } else {
                        (None, None, None)
                    };

                let unique_values =
                    collect_unique_values_if_categorical(&final_label, col_values, enum_threshold);
                let enum_domain =
                    detect_enum_domain(&final_label, col_values, &EnumConfig::default());
                let quality_band = quality_band_label(result.confidence);
                let runner_up =
                    low_band_runner_up(&final_label, result.confidence, &result.vote_distribution);
                Ok(ColProfile {
                    name,
                    label: final_label,
                    nominated,
                    quality_band,
                    runner_up,
                    confidence: result.confidence,
                    samples_used: result.samples_used,
                    non_null_count: col_values.len(),
                    null_count,
                    disambiguation_applied: result.disambiguation_applied,
                    disambiguation_rule: result.disambiguation_rule,
                    detected_locale: result.detected_locale,
                    broad_type,
                    format_string,
                    transform,
                    is_generic: result.is_generic,
                    quality: None,
                    unique_values,
                    validation_pass_rate: vp_rate,
                    validation_vetoed: vetoed,
                    validation_advisory_low: advisory_low,
                    vetoed_type,
                    enum_domain,
                })
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        // Output results
        match output {
            OutputFormat::Plain => {
                println!(
                    "FineType Column Profile — {:?} ({} rows, {} columns)",
                    file, row_count, n_cols
                );
                println!("{}", "═".repeat(80));
                println!();
                if false {
                    // validate removed (AC-10)
                    println!(
                        "  {:<25} {:<38} {:>8} {:>6} {:>8}",
                        "COLUMN", "TYPE", "BROAD", "CONF", "VALID"
                    );
                } else {
                    println!(
                        "  {:<25} {:<38} {:>8} {:>6}",
                        "COLUMN", "TYPE", "BROAD", "CONF"
                    );
                }
                println!("  {}", "─".repeat(78));

                for p in &profiles {
                    println!("{}", plain_row(p));
                    // Show top 3 invalid samples inline (plain output, validate mode)
                    if false {
                        // validate removed (AC-10)
                        if let Some(ref q) = p.quality {
                            for sample in q.invalid_samples.iter().take(3) {
                                println!("  {:>25} ⚠ \"{}\"", "", sample);
                            }
                        }
                    }
                }

                println!();
                let typed_cols = profiles.iter().filter(|p| p.label != "unknown").count();
                if false {
                    // validate removed (AC-10)
                    let scores: Vec<_> = profiles
                        .iter()
                        .filter_map(|p| p.quality.as_ref().map(|q| q.score.clone()))
                        .collect();
                    let grade = finetype_core::compute_file_grade(&scores);
                    println!(
                        "{}/{} columns typed, {} rows analyzed — Quality: {}",
                        typed_cols, n_cols, row_count, grade
                    );
                } else {
                    println!(
                        "{}/{} columns typed, {} rows analyzed",
                        typed_cols, n_cols, row_count
                    );
                }
            }
            OutputFormat::Json => {
                let cols: Vec<serde_json::Value> = profiles
                    .iter()
                    .map(|p| json_column_object(p, verbose))
                    .collect();

                // Compute file-level grade when validation is active
                let file_grade = if false {
                    // validate removed (AC-10)
                    let scores: Vec<_> = profiles
                        .iter()
                        .filter_map(|p| p.quality.as_ref().map(|q| q.score.clone()))
                        .collect();
                    Some(finetype_core::compute_file_grade(&scores))
                } else {
                    None
                };

                if is_json_input {
                    // Structured JSON output: reconstruct nested hierarchy
                    let schema_input: Vec<(String, String, Option<String>, f32)> = profiles
                        .iter()
                        .map(|p| {
                            (
                                p.name.clone(),
                                p.label.clone(),
                                p.broad_type.clone(),
                                p.confidence,
                            )
                        })
                        .collect();
                    let schema = reconstruct_json_schema(&schema_input);
                    let mut result = json!({
                        "file": file.to_string_lossy(),
                        "rows": row_count,
                        "schema": schema,
                        "columns": cols,
                    });
                    if let Some(grade) = &file_grade {
                        result["grade"] = json!(grade.to_string());
                    }
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    let mut result = json!({
                        "file": file.to_string_lossy(),
                        "rows": row_count,
                        "columns": cols,
                    });
                    if let Some(grade) = &file_grade {
                        result["grade"] = json!(grade.to_string());
                    }
                    println!("{}", serde_json::to_string_pretty(&result)?);
                }
            }
            OutputFormat::Csv => {
                println!("column,type,confidence,quality_band,runner_up,broad_type,format_string,transform,is_generic,samples_used,non_null,null,disambiguation,locale");
                for p in &profiles {
                    println!(
                        "\"{}\",\"{}\",{:.4},\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",{},{},{},{},\"{}\",\"{}\"",
                        p.name,
                        p.label,
                        p.confidence,
                        p.quality_band,
                        p.runner_up.as_deref().unwrap_or(""),
                        p.broad_type.as_deref().unwrap_or(""),
                        p.format_string.as_deref().unwrap_or(""),
                        p.transform.as_deref().unwrap_or(""),
                        p.is_generic,
                        p.samples_used,
                        p.non_null_count,
                        p.null_count,
                        p.disambiguation_rule.as_deref().unwrap_or(""),
                        p.detected_locale.as_deref().unwrap_or("")
                    );
                }
            }
            OutputFormat::Markdown => {
                println!(
                    "## FineType Column Profile — `{}`\n",
                    file.to_string_lossy()
                );
                println!("{} rows, {} columns\n", row_count, n_cols);
                if false {
                    // validate removed (AC-10)
                    println!("| Column | Type | Broad Type | Confidence | Valid Rate | Quality |");
                    println!("|--------|------|-----------|----------:|-----------:|--------:|");
                } else {
                    println!("| Column | Type | Broad Type | Confidence | Quality |");
                    println!("|--------|------|-----------|----------:|---------|");
                }
                for p in &profiles {
                    let conf_str = if p.non_null_count > 0 {
                        format!("{:.1}%", p.confidence * 100.0)
                    } else {
                        "—".to_string()
                    };
                    let broad =
                        resolve_broad_type_display(p.broad_type.as_deref(), &p.unique_values);
                    if false {
                        // validate removed (AC-10)
                        let (valid_str, score_str) = match &p.quality {
                            Some(q) => (
                                format!("{:.1}%", q.score.type_conforming_rate * 100.0),
                                format!("{:.1}%", q.score.quality_score * 100.0),
                            ),
                            None => ("—".to_string(), "—".to_string()),
                        };
                        println!(
                            "| {} | `{}` | {} | {} | {} | {} |",
                            p.name, p.label, broad, conf_str, valid_str, score_str
                        );
                    } else {
                        let band_cell = match (p.quality_band, &p.runner_up) {
                            ("low", Some(ru)) => format!("low (maybe `{ru}`)"),
                            (band, _) => band.to_string(),
                        };
                        println!(
                            "| {} | `{}` | {} | {} | {} |",
                            p.name, p.label, broad, conf_str, band_cell
                        );
                    }
                }
                let typed_cols = profiles.iter().filter(|p| p.label != "unknown").count();
                if false {
                    // validate removed (AC-10)
                    let scores: Vec<_> = profiles
                        .iter()
                        .filter_map(|p| p.quality.as_ref().map(|q| q.score.clone()))
                        .collect();
                    let grade = finetype_core::compute_file_grade(&scores);
                    println!(
                        "\n{}/{} columns typed — **Quality: {}**",
                        typed_cols, n_cols, grade
                    );
                    // Data Issues section for columns with invalid samples
                    let issues: Vec<_> = profiles
                        .iter()
                        .filter_map(|p| {
                            p.quality.as_ref().and_then(|q| {
                                if q.invalid_samples.is_empty() {
                                    None
                                } else {
                                    Some((&p.name, &q.invalid_samples))
                                }
                            })
                        })
                        .collect();
                    if !issues.is_empty() {
                        println!("\n### Data Issues\n");
                        for (name, samples) in &issues {
                            println!("**{}** — invalid samples:", name);
                            for s in *samples {
                                println!("- `{}`", s);
                            }
                            println!();
                        }
                    }
                } else {
                    println!("\n{}/{} columns typed", typed_cols, n_cols);
                }
            }
            OutputFormat::Arrow => {
                // Arrow IPC JSON schema format
                let fields: Vec<serde_json::Value> = profiles
                    .iter()
                    .map(|p| {
                        let duckdb_type = p.broad_type.as_deref().unwrap_or("VARCHAR");
                        let arrow_type = duckdb_to_arrow_type(duckdb_type);
                        json!({
                            "name": p.name,
                            "type": arrow_type,
                            "nullable": true,
                            "children": [],
                        })
                    })
                    .collect();

                let schema = json!({
                    "fields": fields,
                    "metadata": {
                        "finetype_version": env!("CARGO_PKG_VERSION"),
                        "source": file.file_name().and_then(|f| f.to_str()).unwrap_or("unknown"),
                        "row_count": row_count.to_string(),
                    }
                });

                println!("{}", serde_json::to_string_pretty(&schema)?);
            }
            OutputFormat::JsonSchema => {
                // ac-03 / ac-05: emit table-level JSON Schema via the shared
                // helper. Taxonomy enrichment is required for label → property
                // shape; without it, we cannot produce a meaningful schema.
                let taxonomy = enrichment_taxonomy.as_ref().ok_or_else(|| {
                    anyhow::anyhow!(
                        "JSON Schema output requires the bundled taxonomy at `labels/`; \
                     run from the FineType source tree or ship with embedded taxonomy."
                    )
                })?;

                let file_stem = file.file_stem().and_then(|s| s.to_str()).unwrap_or("table");
                let file_id = file
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("data.csv");

                // Project profile rows + raw column values into the helper's
                // borrowed-input shape. `columns` parallels `profiles` by index.
                // Synthesise an unknown-reason per column (card 0020: honest
                // typing — an analyst sees WHY a column is untyped). Held in a
                // parallel Vec so the schema columns can borrow it.
                let unknown_reasons: Vec<Option<String>> =
                    profiles.iter().map(unknown_reason_for).collect();
                let cols: Vec<json_schema::TableSchemaColumn<'_>> = profiles
                    .iter()
                    .enumerate()
                    .map(|(i, p)| {
                        let values: &[String] = columns.get(i).map(|v| v.as_slice()).unwrap_or(&[]);
                        json_schema::TableSchemaColumn {
                            name: &p.name,
                            label: &p.label,
                            values,
                            null_count: p.null_count,
                            unknown_reason: unknown_reasons[i].as_deref(),
                            nominated: p.nominated,
                        }
                    })
                    .collect();

                let schema = json_schema::emit_table_schema(
                    &cols,
                    file_stem,
                    file_id,
                    taxonomy,
                    stats,
                    enum_threshold,
                );

                writeln!(writer, "{}", serde_json::to_string_pretty(&schema)?)?;
            }
            OutputFormat::Datapackage => {
                // ac-02: Frictionless Data Package descriptor (choice 0105).
                // Same taxonomy enrichment + borrowed-column shape as the
                // json-schema branch; type/format come from the authoritative
                // `frictionless` map keyed on each column's label.
                let taxonomy = enrichment_taxonomy.as_ref().ok_or_else(|| {
                    anyhow::anyhow!(
                        "datapackage output requires the bundled taxonomy at `labels/`; \
                     run from the FineType source tree or ship with embedded taxonomy."
                    )
                })?;

                let cols: Vec<datapackage::DatapackageColumn<'_>> = profiles
                    .iter()
                    .enumerate()
                    .map(|(i, p)| {
                        let values: &[String] = columns.get(i).map(|v| v.as_slice()).unwrap_or(&[]);
                        datapackage::DatapackageColumn {
                            name: &p.name,
                            label: &p.label,
                            values,
                            confidence: Some(p.confidence),
                            locale: p.detected_locale.as_deref(),
                            nominated: p.nominated,
                        }
                    })
                    .collect();

                let content = std::fs::read(file)
                    .map_err(|e| anyhow::anyhow!("could not read {:?} for hashing: {}", file, e))?;
                let resource = datapackage::ResourceMeta::for_path(file, &content);
                let descriptor =
                    datapackage::emit_datapackage(&cols, &resource, taxonomy, enum_threshold);
                writeln!(writer, "{}", serde_json::to_string_pretty(&descriptor)?)?;
            }
        }

        writer.flush()?;
        batch_success += 1;
    } // end per-file loop

    // Batch resilience backstop: a per-file read failure is skipped-and-logged
    // above, but if EVERY listed file failed (e.g. duckdb is not on PATH, or
    // none parsed) the run must not exit 0 having produced nothing — that would
    // hide a systemic failure from a caller gating on exit status. Single-file
    // mode already surfaces the error directly via `?` and never reaches here.
    if batch_mode && batch_success == 0 && !paths.is_empty() {
        anyhow::bail!(
            "no input files could be profiled — all {} failed (see the warnings \
             above). If duckdb is not installed, install it from \
             https://duckdb.org/docs/installation.",
            paths.len()
        );
    }

    Ok(())
}

#[cfg(test)]
mod nomination_tests {
    use super::*;
    use std::collections::HashSet;

    fn taxonomy() -> finetype_core::Taxonomy {
        let mut t = finetype_core::Taxonomy::from_yaml(
            r#"
representation.text.plain_text:
  broad_type: VARCHAR
  frictionless:
    type: string
  validation:
    type: string
    minLength: 1
    maxLength: 65536
identity.person.email:
  broad_type: VARCHAR
  frictionless:
    type: string
    format: email
  validation:
    type: string
    pattern: '^[^@ ]+@[^@ ]+\.[^@ ]+$'
    minLength: 5
    maxLength: 254
"#,
        )
        .expect("test taxonomy parses");
        t.compile_validators();
        t
    }

    fn profile(label: &str, nominated: bool) -> ColProfile {
        ColProfile {
            name: "corpus".to_string(),
            label: label.to_string(),
            nominated,
            confidence: 0.4213,
            samples_used: 7,
            non_null_count: 7,
            null_count: 1,
            disambiguation_applied: false,
            disambiguation_rule: None,
            detected_locale: None,
            broad_type: Some("VARCHAR".to_string()),
            format_string: Some("%s".to_string()),
            transform: Some("CAST({col} AS VARCHAR)".to_string()),
            is_generic: false,
            quality: None,
            unique_values: None,
            validation_pass_rate: None,
            validation_vetoed: false,
            validation_advisory_low: false,
            vetoed_type: None,
            enum_domain: None,
            // `low` is the band a 0.42 confidence earns. A nominated column
            // must not print it, and the band is left here deliberately so a
            // suppression that only works for `high` would fail this test.
            quality_band: "low",
            runner_up: Some("identity.person.email".to_string()),
        }
    }

    #[test]
    fn a_nominated_column_reads_decl_and_carries_no_band_or_veto_annotation() {
        let row = plain_row(&profile("representation.text.plain_text", true));
        assert!(row.contains("representation.text.plain_text"), "{row}");
        assert!(row.contains("decl"), "{row}");
        assert!(
            !row.contains('%'),
            "a declared type printed a percentage: {row}"
        );
        for absent in [" ⚑ low", " ~ medium", " ⊘ vetoed:", " ⚠ low-pass"] {
            assert!(!row.contains(absent), "row carried `{absent}`: {row}");
        }

        // The same column inferred still prints both.
        let row = plain_row(&profile("representation.text.plain_text", false));
        assert!(row.contains("42.1%"), "{row}");
        assert!(row.contains(" ⚑ low"), "{row}");
        assert!(!row.contains("decl"), "{row}");
    }

    #[test]
    fn a_nominated_columns_json_object_drops_the_classifiers_answer_and_keeps_the_data() {
        let obj = json_column_object(&profile("representation.text.plain_text", true), false);
        let obj = obj.as_object().expect("column object");

        assert_eq!(obj["type"], "representation.text.plain_text");
        assert_eq!(obj["nominated"], true);
        for absent in [
            "confidence",
            "quality_band",
            "runner_up",
            "validation_pass_rate",
            "validation_vetoed",
            "vetoed_type",
            "validation_advisory_low",
        ] {
            assert!(
                !obj.contains_key(absent),
                "a declared type published `{absent}`: {obj:?}"
            );
        }
        // Facts about the data, true whoever chose the label.
        assert_eq!(obj["broad_type"], "VARCHAR");
        assert_eq!(obj["format_string"], "%s");
        assert_eq!(obj["transform"], "CAST({col} AS VARCHAR)");
        assert_eq!(obj["samples_used"], 7);
        assert_eq!(obj["non_null"], 7);
        assert_eq!(obj["null"], 1);

        // Inferred, the same column publishes the classifier's answer and no
        // `nominated` key.
        let obj = json_column_object(&profile("representation.text.plain_text", false), false);
        let obj = obj.as_object().expect("column object");
        assert!(!obj.contains_key("nominated"), "{obj:?}");
        assert_eq!(obj["quality_band"], "low");
        assert!(obj.contains_key("confidence"), "{obj:?}");
        assert!(obj.contains_key("runner_up"), "{obj:?}");
    }

    #[test]
    fn an_undeclared_column_is_untouched_by_another_columns_nomination() {
        // AC5's last clause, at the grain the flag actually acts on: the
        // per-column decision. Two columns, one nominated, and the other's
        // object is byte-identical to what it is with no nomination anywhere.
        let inferred = json_column_object(&profile("identity.person.email", false), false);
        let _ = json_column_object(&profile("representation.text.plain_text", true), false);
        let again = json_column_object(&profile("identity.person.email", false), false);
        assert_eq!(inferred, again);
    }

    #[test]
    fn the_data_cannot_overturn_a_nomination() {
        // `identity.person.email` is on the audited-safe allowlist, so a column
        // of non-emails hard-vetoes it: `resolve_veto_outcome` would replace
        // the label with a residual picked from the value shape. A nomination
        // is taken as given, so the veto never runs.
        let taxonomy = taxonomy();
        let safe: HashSet<String> = ["identity.person.email".to_string()].into_iter().collect();
        let values: Vec<String> = (0..10)
            .map(|i| format!("not an address at all {i}"))
            .collect();

        // The premise: inferred, this column really is hard-vetoed.
        let inferred = decide_label(
            None,
            "identity.person.email",
            &values,
            Some(&taxonomy),
            &safe,
            true,
        );
        assert!(
            inferred.validation_vetoed,
            "the premise failed — this column is not vetoed when inferred, so \
             the nominated case below proves nothing"
        );
        assert_ne!(inferred.label, "identity.person.email");

        // Nominated, the same column and the same values keep the label.
        let nominated = decide_label(
            Some("identity.person.email"),
            "identity.person.email",
            &values,
            Some(&taxonomy),
            &safe,
            true,
        );
        assert_eq!(nominated.label, "identity.person.email");
        assert!(nominated.nominated);
        assert!(!nominated.validation_vetoed);
        assert!(!nominated.validation_advisory_low);
        assert_eq!(nominated.validation_pass_rate, None);
        assert_eq!(nominated.vetoed_type, None);
        assert_eq!(nominated.fallback_rule, None);

        // …and none of that reaches the output.
        let mut p = profile("identity.person.email", true);
        p.validation_pass_rate = nominated.validation_pass_rate;
        p.validation_vetoed = nominated.validation_vetoed;
        p.validation_advisory_low = nominated.validation_advisory_low;
        p.vetoed_type = nominated.vetoed_type.clone();
        assert!(!plain_row(&p).contains(" ⊘ vetoed:"));
        let obj = json_column_object(&p, false);
        let obj = obj.as_object().unwrap();
        for absent in ["validation_vetoed", "vetoed_type", "validation_pass_rate"] {
            assert!(!obj.contains_key(absent), "{obj:?}");
        }
    }

    #[test]
    fn a_nomination_beats_the_classifiers_answer() {
        // The nominated label and the predicted label are DIFFERENT strings on
        // purpose. A test where they agree cannot tell `decide_label` taking
        // the nomination from `decide_label` taking the prediction, and would
        // pass just as well against code that ignores the nomination entirely.
        let taxonomy = taxonomy();
        let safe: HashSet<String> = HashSet::new();
        let values: Vec<String> = vec!["ada@example.com".into(), "grace@example.org".into()];
        let out = decide_label(
            Some("representation.text.plain_text"),
            "identity.person.email",
            &values,
            Some(&taxonomy),
            &safe,
            true,
        );
        assert_eq!(out.label, "representation.text.plain_text");
        assert!(out.nominated);
        // …and the classifier's answer really was the other one, so the
        // assertion above is about the choice and not about the fixture.
        let inferred = decide_label(
            None,
            "identity.person.email",
            &values,
            Some(&taxonomy),
            &safe,
            true,
        );
        assert_eq!(inferred.label, "identity.person.email");
    }

    #[test]
    fn inference_still_decides_a_column_nobody_nominated() {
        let taxonomy = taxonomy();
        let safe: HashSet<String> = HashSet::new();
        let values: Vec<String> = vec!["ada@example.com".into(), "grace@example.org".into()];
        let out = decide_label(
            None,
            "identity.person.email",
            &values,
            Some(&taxonomy),
            &safe,
            true,
        );
        assert_eq!(out.label, "identity.person.email");
        assert!(!out.nominated);
    }
}
