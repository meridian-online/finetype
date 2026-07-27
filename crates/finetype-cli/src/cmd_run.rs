//! `cmd_run` — extracted from main.rs (mechanical split, no behaviour change).

use super::*;

/// Run the diagnostic cascade over an NDJSON stream of (column_name,
/// predicted_type, samples) inputs, emitting one JSON line per input with
/// the inferred correct type, confidence, mechanism token, and signals.
///
/// The taxonomy + validators load once across the whole stream — this is
/// the batch-mode amortisation that makes corpus-scale attribution
/// tractable. Wire shapes are defined in
/// `finetype_core::infer::{InferInput, InferOutput}`.
///
/// Exposed via `finetype infer --mode column --batch --explain`; subsumes
/// the historical `infer-type` subcommand (removed in the same change).
pub(crate) fn cmd_infer_explain_batch(taxonomy_path: &std::path::Path) -> Result<()> {
    use finetype_core::infer::{infer, InferInput};
    use std::io::{BufRead, Write};

    // Load taxonomy + compile validators (same loader as cmd_validate).
    // Single load amortised across every line on stdin.
    let mut taxonomy = load_taxonomy(&taxonomy_path.to_path_buf())?;
    taxonomy.compile_validators();
    taxonomy.compile_locale_validators();

    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();

    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let input: InferInput = serde_json::from_str(&line)
            .map_err(|e| anyhow::anyhow!("failed to parse stdin JSON line ({}): {}", e, line))?;
        let result = infer(&taxonomy, &input);
        writeln!(out, "{}", serde_json::to_string(&result)?)?;
    }
    Ok(())
}

// The MCP server role (`FineTypeServer::new` + `serve_stdio`) is deprecated in
// favour of arcform's `arc mcp`. The `finetype mcp` subcommand is kept working for
// now, so this internal call site opts out of the deprecation lint at function scope.
#[allow(deprecated)]
pub(crate) fn cmd_mcp() -> Result<()> {
    use finetype_model::{ColumnClassifier, ColumnConfig};

    eprintln!("Starting FineType MCP server...");

    let config = ColumnConfig {
        sample_size: 100,
        ..Default::default()
    };

    // Build the multi-branch column classifier (the shipped model).
    let model_path = PathBuf::from("models/default");
    let mb = load_multi_branch_classifier(&model_path)?;
    eprintln!(
        "Loaded multi-branch classifier ({} classes)",
        mb.n_classes()
    );
    let mut column_classifier = ColumnClassifier::with_multi_branch(mb, config);
    wire_model2vec(&mut column_classifier);

    // Load taxonomy for validation-based disambiguation
    let taxonomy_path = PathBuf::from("labels");
    let mut taxonomy = load_taxonomy(&taxonomy_path)?;
    taxonomy.compile_validators();
    taxonomy.compile_locale_validators();
    eprintln!(
        "Loaded taxonomy ({} types, {} validators cached, {} with locale validators)",
        taxonomy.labels().len(),
        taxonomy.validator_count(),
        taxonomy.locale_validator_count()
    );
    column_classifier.set_taxonomy(taxonomy.clone());

    // Create MCP server with fully-configured classifier
    let server = finetype_mcp::FineTypeServer::new(column_classifier, taxonomy);

    eprintln!("FineType MCP server ready (stdio transport)");

    // Run the async server
    tokio::runtime::Runtime::new()?.block_on(server.serve_stdio())?;

    Ok(())
}

/// Re-sharpen cached Sense predictions through the real Sharpen stack without the
/// value-encode (corpus-honest gate fast path, spec 2026-06-27-composed-accuracy-roadmap).
/// Input TSV: `id<TAB>header<TAB>sense_label<TAB>sense_conf<TAB>values(0x1f-joined)`.
///
/// Output TSV — the WHOLE user-visible record, one column per line:
/// `id<TAB>label<TAB>confidence<TAB>quality_band<TAB>runner_up<TAB>disambiguation_rule`.
///
/// It used to emit `id<TAB>label` only, and that is how a rule change was once
/// certified "byte-identical across 837,625 columns" while it was in fact
/// collapsing confidence from 0.877 to 0.500, dropping the quality band from
/// `high` to `low`, surfacing a `runner_up`, and rewriting the
/// `disambiguation_rule` — every one of which `finetype profile -o json`
/// prints. A label-only diff cannot see any of that. `quality_band` and
/// `runner_up` are computed with the same two functions `profile` uses, so this
/// line is the profile record minus the fields Sharpen cannot touch.
///
/// `detected_locale` is deliberately NOT emitted: it is not run-to-run stable
/// (the same binary over `eval/datasets/csv/ecommerce_orders.csv` produced
/// `ZH_TW`, `AR_SA` and `HE` on three consecutive runs), so including it would
/// make every diff report spurious transitions. That nondeterminism is a real
/// defect and is tracked separately; until it is fixed, locale is out of scope
/// for any byte-identity claim.
pub(crate) fn cmd_resharpen(input: PathBuf, output: PathBuf, model: PathBuf) -> Result<()> {
    use finetype_model::{ColumnClassifier, ColumnConfig};
    use std::io::{BufRead, BufReader, BufWriter, Write};

    let config = ColumnConfig {
        sample_size: 100,
        ..Default::default()
    };
    let mb = load_multi_branch_classifier(&model)?;
    let mut cc = ColumnClassifier::with_multi_branch(mb, config);
    wire_model2vec(&mut cc);
    let mut taxonomy = load_taxonomy(&PathBuf::from("labels"))?;
    taxonomy.compile_validators();
    taxonomy.compile_locale_validators();
    cc.set_taxonomy(taxonomy);

    let reader = BufReader::new(std::fs::File::open(&input)?);
    let mut out = BufWriter::new(std::fs::File::create(&output)?);
    let mut n = 0usize;
    for line in reader.lines() {
        let line = line?;
        if line.is_empty() {
            continue;
        }
        let mut parts = line.splitn(5, '\t');
        let id = parts.next().unwrap_or("");
        let header = parts.next().unwrap_or("");
        let sense_label = parts.next().unwrap_or("");
        let sense_conf: f32 = parts.next().unwrap_or("1.0").parse().unwrap_or(1.0);
        let values: Vec<String> = parts
            .next()
            .unwrap_or("")
            .split('\u{1f}')
            .filter(|v| !v.is_empty())
            .map(|s| s.to_string())
            .collect();
        let composed = cc.compose_from_sense(header, &values, sense_label, sense_conf)?;
        let band = crate::profile::quality_band_label(composed.confidence);
        let runner_up = crate::profile::low_band_runner_up(
            &composed.label,
            composed.confidence,
            &composed.vote_distribution,
        );
        writeln!(
            out,
            "{}\t{}\t{:.6}\t{}\t{}\t{}",
            id,
            composed.label,
            composed.confidence,
            band,
            runner_up.as_deref().unwrap_or(""),
            composed.disambiguation_rule.as_deref().unwrap_or("")
        )?;
        n += 1;
    }
    out.flush()?;
    eprintln!("resharpen: composed {} columns -> {}", n, output.display());
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn cmd_infer(
    input: Option<String>,
    file: Option<PathBuf>,
    output: OutputFormat,
    show_confidence: bool,
    show_value: bool,
    mode: InferenceMode,
    sample_size: usize,
    header: Option<String>,
    batch: bool,
    explain: bool,
    taxonomy: PathBuf,
) -> Result<()> {
    use finetype_model::{ColumnClassifier, ColumnConfig};

    // --explain: diagnostic cascade over an NDJSON stream. Subsumes the
    // historical `infer-type` subcommand; lives on `infer` to keep the
    // CLI surface flat.
    if explain {
        if !batch || !matches!(mode, InferenceMode::Column) {
            anyhow::bail!("--explain requires --mode column --batch");
        }
        return cmd_infer_explain_batch(&taxonomy);
    }

    let model = resolve_model_path();

    // Batch mode: read JSONL from stdin, classify each column group
    if batch {
        if !matches!(mode, InferenceMode::Column) {
            anyhow::bail!("--batch requires --mode column");
        }
        return cmd_infer_batch(model, sample_size);
    }

    // Collect inputs
    let inputs: Vec<String> = if let Some(text) = input {
        vec![text]
    } else if let Some(path) = file {
        std::fs::read_to_string(path)?
            .lines()
            .map(String::from)
            .filter(|s| !s.is_empty())
            .collect()
    } else {
        // Read from stdin
        io::stdin()
            .lock()
            .lines()
            .map_while(|l| l.ok())
            .filter(|s| !s.is_empty())
            .collect()
    };

    if inputs.is_empty() {
        eprintln!("No input provided");
        return Ok(());
    }

    // Column mode: treat all inputs as one column, return single prediction
    if matches!(mode, InferenceMode::Column) {
        // Taxonomy is needed by BOTH the deterministic fast-path and the full
        // classifier (validation-based demotion) — load and compile it once.
        let taxonomy_path = std::path::PathBuf::from("labels");
        let mut col_taxonomy = load_taxonomy(&taxonomy_path).ok();
        if let Some(t) = col_taxonomy.as_mut() {
            t.compile_validators();
            t.compile_locale_validators();
        }

        // Deterministic fast-path (card 0006): a structurally-conclusive sample —
        // email, IPv4/v6, MAC, windows_path, message_id, delimited ISO datetime —
        // is value-determinable (decision 0048), so the neural model adds nothing.
        // Resolving it here skips the ~0.08s multi-branch load, the dominant warm
        // cost of single-shot `infer` (memory infer-latency-breakdown). The leaf
        // set is conservative by construction, so the answer matches the full
        // Sense→Sharpen pipeline (finetype_core::fast_path). Engaged only without
        // an explicit --header (a header can steer the full pipeline; the
        // value-only case is the one whose agreement we can guarantee) and
        // kill-switchable via RHH.
        let fast_leaf =
            if header.is_none() && !finetype_model::rhh::is_disabled("deterministic_fast_path") {
                col_taxonomy
                    .as_ref()
                    .and_then(|tax| finetype_core::deterministic_fast_path(tax, &inputs))
            } else {
                None
            };

        let result = if let Some(leaf) = fast_leaf {
            finetype_model::ColumnResult {
                label: leaf,
                confidence: 0.99,
                vote_distribution: Vec::new(),
                disambiguation_applied: true,
                disambiguation_rule: Some("deterministic_fast_path".to_string()),
                samples_used: inputs.len(),
                detected_locale: None,
                is_generic: false,
                column_features: None,
            }
        } else {
            let config = ColumnConfig {
                sample_size,
                ..Default::default()
            };
            let mb = load_multi_branch_classifier(&model)?;
            let mut column_classifier = ColumnClassifier::with_multi_branch(mb, config);

            // Validation-based attractor demotion (Rule 14) needs the taxonomy.
            if let Some(taxonomy) = col_taxonomy {
                column_classifier.set_taxonomy(taxonomy);
            }

            // Multi-branch path: wire Model2Vec for header enrichment, no siblings.
            if column_classifier.has_multi_branch() {
                wire_model2vec(&mut column_classifier);
            }

            if let Some(ref hdr) = header {
                column_classifier.classify_column_with_header(&inputs, hdr)?
            } else {
                column_classifier.classify_column(&inputs)?
            }
        };

        match output {
            // datapackage is a profile-only table format; for single-value
            // `infer` it degrades to plain output.
            OutputFormat::Plain
            | OutputFormat::Markdown
            | OutputFormat::Arrow
            | OutputFormat::JsonSchema
            | OutputFormat::Datapackage => {
                println!("{}", result.label);
                if show_confidence {
                    println!(
                        "  confidence: {:.4} ({} samples)",
                        result.confidence, result.samples_used
                    );
                }
                if let Some(locale) = &result.detected_locale {
                    println!("  locale: {}", locale);
                }
                if result.disambiguation_applied {
                    println!(
                        "  disambiguation: {}",
                        result.disambiguation_rule.as_deref().unwrap_or("unknown")
                    );
                }
                if show_value {
                    println!("  vote distribution:");
                    for (label, frac) in &result.vote_distribution {
                        if *frac >= 0.01 {
                            println!("    {:.1}%  {}", frac * 100.0, label);
                        }
                    }
                }
            }
            OutputFormat::Json => {
                let mut obj = serde_json::Map::new();
                obj.insert("label".to_string(), json!(result.label));
                obj.insert("confidence".to_string(), json!(result.confidence));
                obj.insert("samples_used".to_string(), json!(result.samples_used));
                obj.insert(
                    "disambiguation_applied".to_string(),
                    json!(result.disambiguation_applied),
                );
                if let Some(rule) = &result.disambiguation_rule {
                    obj.insert("disambiguation_rule".to_string(), json!(rule));
                }
                if let Some(locale) = &result.detected_locale {
                    obj.insert("locale".to_string(), json!(locale));
                }
                let votes: Vec<serde_json::Value> = result
                    .vote_distribution
                    .iter()
                    .filter(|(_, f)| *f >= 0.01)
                    .map(|(l, f)| json!({"label": l, "fraction": f}))
                    .collect();
                obj.insert("vote_distribution".to_string(), json!(votes));
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::Value::Object(obj))?
                );
            }
            OutputFormat::Csv => {
                println!(
                    "{},{:.4},{}",
                    result.label, result.confidence, result.samples_used
                );
            }
        }
        return Ok(());
    }

    // Row mode (per-value) required a value-level model. The only shipped
    // model is the column-level multi-branch model (choice 0107), so row mode
    // is no longer supported.
    anyhow::bail!(
        "Row mode is unsupported: the shipped model is column-level. Use --mode column (the default) or `finetype profile`."
    )
}

// ═══════════════════════════════════════════════════════════════════════════════
// INFER BATCH — JSONL column-mode batch classification
// ═══════════════════════════════════════════════════════════════════════════════

/// Batch column-mode inference: reads JSONL from stdin, classifies each column
/// group using the full pipeline (tiered model + Model2Vec + disambiguation +
/// attractor demotion), and writes one JSON line per input to stdout.
///
/// Input JSONL format:
///   {"header": "col_name", "values": ["v1", "v2", ...]}
///   {"values": ["v1", "v2", ...]}
///
/// Output JSONL format:
///   {"label": "identity.person.email", "confidence": 0.95, ...}
pub(crate) fn cmd_infer_batch(model: PathBuf, sample_size: usize) -> Result<()> {
    use finetype_model::{ColumnClassifier, ColumnConfig};
    use std::time::Instant;

    let t_start = Instant::now();

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
    let taxonomy_path = std::path::PathBuf::from("labels");
    if let Ok(mut taxonomy) = load_taxonomy(&taxonomy_path) {
        taxonomy.compile_validators();
        taxonomy.compile_locale_validators();
        eprintln!(
            "Loaded taxonomy ({} types, {} validators, {} locale validators)",
            taxonomy.labels().len(),
            taxonomy.validator_count(),
            taxonomy.locale_validator_count()
        );
        column_classifier.set_taxonomy(taxonomy);
    }

    // Multi-branch path: wire Model2Vec for header enrichment, no sibling context.
    if column_classifier.has_multi_branch() {
        wire_model2vec(&mut column_classifier);
    }

    // Separate taxonomy handle for the headerless deterministic fast-path, so
    // `--batch` resolves structurally-conclusive values the same way `infer -i`
    // does (the classifier owns the first handle). Compile validators only —
    // the fast-path is value-structural, no locale steering.
    let fast_path_tax = {
        let mut t = load_taxonomy(&taxonomy_path).ok();
        if let Some(t) = t.as_mut() {
            t.compile_validators();
        }
        t
    };

    let load_elapsed = t_start.elapsed();
    eprintln!("Model loaded in {:.2}s", load_elapsed.as_secs_f64());

    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());
    let stdin = io::stdin();

    let mut n_columns = 0u64;
    let mut n_values = 0u64;
    let mut n_errors = 0u64;

    for line in stdin.lock().lines() {
        let line = line?;
        if line.is_empty() {
            continue;
        }

        // Parse JSONL input
        let input: serde_json::Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => {
                let err_obj = json!({"error": format!("invalid JSON: {e}")});
                writeln!(out, "{}", err_obj)?;
                n_errors += 1;
                continue;
            }
        };

        let values: Vec<String> = match input.get("values").and_then(|v| v.as_array()) {
            Some(arr) => arr
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect(),
            None => {
                let err_obj = json!({"error": "missing or invalid 'values' array"});
                writeln!(out, "{}", err_obj)?;
                n_errors += 1;
                continue;
            }
        };

        if values.is_empty() {
            let err_obj = json!({"error": "empty values array"});
            writeln!(out, "{}", err_obj)?;
            n_errors += 1;
            continue;
        }

        n_values += values.len() as u64;

        let header_str = input.get("header").and_then(|h| h.as_str()).unwrap_or("");

        // Headerless deterministic fast-path (mirrors cmd_infer): a structurally
        // conclusive value is value-determinable, so resolve it without the model
        // and identically to `infer -i`. Only fires with no header (a header can
        // legitimately steer the full pipeline) and is RHH kill-switchable.
        let fast_leaf = if header_str.is_empty()
            && !finetype_model::rhh::is_disabled("deterministic_fast_path")
        {
            fast_path_tax
                .as_ref()
                .and_then(|tax| finetype_core::deterministic_fast_path(tax, &values))
        } else {
            None
        };

        let result = if let Some(leaf) = fast_leaf {
            finetype_model::ColumnResult {
                label: leaf,
                confidence: 0.99,
                vote_distribution: Vec::new(),
                disambiguation_applied: true,
                disambiguation_rule: Some("deterministic_fast_path".to_string()),
                samples_used: values.len(),
                detected_locale: None,
                is_generic: false,
                column_features: None,
            }
        } else if !header_str.is_empty() {
            column_classifier.classify_column_with_header(&values, header_str)?
        } else {
            column_classifier.classify_column(&values)?
        };

        let mut obj = serde_json::Map::new();
        obj.insert("label".to_string(), json!(result.label));
        obj.insert("confidence".to_string(), json!(result.confidence));
        obj.insert("samples_used".to_string(), json!(result.samples_used));
        if result.disambiguation_applied {
            obj.insert(
                "disambiguation_rule".to_string(),
                json!(result.disambiguation_rule),
            );
        }
        if let Some(locale) = &result.detected_locale {
            obj.insert("locale".to_string(), json!(locale));
        }

        writeln!(out, "{}", serde_json::Value::Object(obj))?;
        n_columns += 1;

        // Progress indicator every 1000 columns
        if n_columns.is_multiple_of(1000) {
            eprintln!(
                "  classified {} columns ({} values)...",
                n_columns, n_values
            );
        }
    }

    out.flush()?;

    let total_elapsed = t_start.elapsed();
    eprintln!(
        "Batch complete: {} columns, {} values, {} errors in {:.2}s ({:.0} cols/sec)",
        n_columns,
        n_values,
        n_errors,
        total_elapsed.as_secs_f64(),
        n_columns as f64 / total_elapsed.as_secs_f64()
    );

    Ok(())
}
