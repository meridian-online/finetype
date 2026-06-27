//! `profile` command: discover and classify column types.

use super::*;
use finetype_cli::enum_emission::collect_unique_values_if_categorical;
use finetype_core::enum_domain::{detect_enum_domain, EnumConfig, EnumDomain};
use finetype_mcp::datapackage;

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

fn quality_band_label(confidence: f32) -> &'static str {
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
fn low_band_runner_up(label: &str, confidence: f32, votes: &[(String, f32)]) -> Option<String> {
    if confidence >= QUALITY_LOW_THRESHOLD {
        return None;
    }
    votes
        .iter()
        .map(|(l, _)| l)
        .find(|l| l.as_str() != label)
        .cloned()
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
) -> Result<()> {
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

    // Wire Model2Vec + sibling context for the multi-branch header enrichment.
    wire_model2vec_and_siblings(&mut column_classifier);

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

        let n_cols = headers.len();
        eprintln!("Read {} rows", row_count);

        // Profile each column
        struct ColProfile {
            name: String,
            label: String,
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

        /// Per-column validation + quality data.
        struct ColProfileQuality {
            valid_count: usize,
            invalid_count: usize,
            null_count: usize,
            score: finetype_core::ColumnQualityScore,
            invalid_samples: Vec<String>,
        }

        let mut profiles: Vec<ColProfile> = Vec::new();

        // When sibling-context attention is available, classify all columns
        // together so each column benefits from cross-column context.
        if column_classifier.has_sibling_context() && !no_header_hint {
            // Build column descriptors for all non-empty columns
            let mut col_inputs: Vec<(usize, Vec<String>, String, String, usize)> = Vec::new(); // (index, values, header_hint, name, null_count)
            let mut empty_profiles: Vec<(usize, ColProfile)> = Vec::new();

            for (i, col_values) in columns.iter().enumerate() {
                let name = headers
                    .get(i)
                    .cloned()
                    .unwrap_or_else(|| format!("col_{}", i));
                let null_count = row_count - col_values.len();

                if col_values.is_empty() {
                    empty_profiles.push((
                        i,
                        ColProfile {
                            name,
                            label: "unknown".to_string(),
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
                        },
                    ));
                } else {
                    let header_hint = if is_json_input {
                        path_leaf(&name)
                    } else {
                        name.clone()
                    };
                    col_inputs.push((i, col_values.clone(), header_hint, name, null_count));
                }
            }

            // Classify all non-empty columns with sibling context
            let context_columns: Vec<(Vec<String>, String)> = col_inputs
                .iter()
                .map(|(_, values, header, _, _)| (values.clone(), header.clone()))
                .collect();
            let context_results =
                column_classifier.classify_columns_with_context(&context_columns)?;

            // Merge results back in original order
            let mut all_entries: Vec<(usize, ColProfile)> = Vec::new();
            all_entries.extend(empty_profiles);
            for ((idx, values, _, name, null_count), result) in
                col_inputs.into_iter().zip(context_results)
            {
                let (vp_rate, vetoed, advisory_low) = col_validation_veto(
                    &result.label,
                    &values,
                    enrichment_taxonomy.as_ref(),
                    &veto_safe,
                    veto_enabled,
                );
                let (final_label, vetoed_type, fallback_rule) =
                    resolve_veto_outcome(vetoed, &result.label, &values);
                let mut result = result;
                if let Some(rule) = fallback_rule {
                    result.disambiguation_applied = true;
                    result.disambiguation_rule = Some(rule.to_string());
                }
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
                    collect_unique_values_if_categorical(&final_label, &values, enum_threshold);
                let enum_domain = detect_enum_domain(&final_label, &values, &EnumConfig::default());
                let quality_band = quality_band_label(result.confidence);
                let runner_up =
                    low_band_runner_up(&final_label, result.confidence, &result.vote_distribution);
                all_entries.push((
                    idx,
                    ColProfile {
                        name,
                        label: final_label,
                        quality_band,
                        runner_up,
                        confidence: result.confidence,
                        samples_used: result.samples_used,
                        non_null_count: values.len(),
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
                    },
                ));
            }
            all_entries.sort_by_key(|(idx, _)| *idx);
            profiles = all_entries.into_iter().map(|(_, p)| p).collect();
        } else {
            // Per-column classification (standard path)
            for (i, col_values) in columns.iter().enumerate() {
                let name = headers
                    .get(i)
                    .cloned()
                    .unwrap_or_else(|| format!("col_{}", i));
                let null_count = row_count - col_values.len();

                if col_values.is_empty() {
                    profiles.push(ColProfile {
                        name,
                        label: "unknown".to_string(),
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
                    continue;
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

                let (vp_rate, vetoed, advisory_low) = col_validation_veto(
                    &result.label,
                    col_values,
                    enrichment_taxonomy.as_ref(),
                    &veto_safe,
                    veto_enabled,
                );
                let (final_label, vetoed_type, fallback_rule) =
                    resolve_veto_outcome(vetoed, &result.label, col_values);
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
                profiles.push(ColProfile {
                    name,
                    label: final_label,
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
                });
            }
        } // end else (per-column path)

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
                    let conf_str = if p.non_null_count > 0 {
                        format!("{:.1}%", p.confidence * 100.0)
                    } else {
                        "—".to_string()
                    };
                    let broad =
                        resolve_broad_type_display(p.broad_type.as_deref(), &p.unique_values);
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
                    let quality_str = if false {
                        // validate removed (AC-10)
                        match &p.quality {
                            Some(q) => format!(" {:>7.1}%", q.score.type_conforming_rate * 100.0),
                            None => "      —".to_string(),
                        }
                    } else {
                        String::new()
                    };
                    // Honest confidence band: call out medium/low only — `high` is
                    // the default expectation, so an unmarked row reads as trusted.
                    let band_str = match p.quality_band {
                        "low" => match &p.runner_up {
                            Some(ru) => format!(" ⚑ low (maybe {ru})"),
                            None => " ⚑ low".to_string(),
                        },
                        "medium" => " ~ medium".to_string(),
                        _ => String::new(),
                    };
                    println!(
                        "  {:<25} {:<38} {:>8} {:>6}{}{}{}{}{}",
                        p.name,
                        p.label,
                        broad,
                        conf_str,
                        quality_str,
                        disambig,
                        locale_str,
                        veto_str,
                        band_str
                    );
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
                    .map(|p| {
                        let mut obj = serde_json::Map::new();
                        obj.insert("column".to_string(), json!(p.name));
                        obj.insert("type".to_string(), json!(p.label));
                        obj.insert("confidence".to_string(), json!(p.confidence));
                        obj.insert("quality_band".to_string(), json!(p.quality_band));
                        if let Some(ru) = &p.runner_up {
                            obj.insert("runner_up".to_string(), json!(ru));
                        }
                        let resolved_broad =
                            resolve_broad_type_display(p.broad_type.as_deref(), &p.unique_values);
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
                                        obj.insert(
                                            "invalid_samples".to_string(),
                                            json!(q.invalid_samples),
                                        );
                                    }
                                }
                                None => {
                                    obj.insert("quality".to_string(), json!(null));
                                }
                            }
                        }
                        serde_json::Value::Object(obj)
                    })
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
