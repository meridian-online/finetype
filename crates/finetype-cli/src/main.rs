//! FineType CLI
//!
//! Command-line interface for precision format detection.

use anyhow::Result;
use clap::{Parser, Subcommand};
use finetype_core::{format_report, Checker, Generator, Label, Taxonomy};
use finetype_model::Classifier;
use serde_json::json;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use tracing_subscriber::EnvFilter;

// ═══════════════════════════════════════════════════════════════════════════════
// EMBEDDED MODELS (compile-time)
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(feature = "embed-models")]
mod embedded {
    include!(concat!(env!("OUT_DIR"), "/embedded_models.rs"));
}

#[derive(Parser)]
#[command(name = "finetype")]
#[command(author = "Hugh Cameron")]
#[command(version)]
#[command(about = "Precision format detection for text data", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Classify text input
    Infer {
        /// Single text input
        #[arg(short, long)]
        input: Option<String>,

        /// File containing inputs (one per line)
        #[arg(short, long)]
        file: Option<PathBuf>,

        /// Model directory
        #[arg(short, long, default_value = "models/default")]
        model: PathBuf,

        /// Output format (plain, json, csv)
        #[arg(short, long, default_value = "plain")]
        output: OutputFormat,

        /// Include confidence score
        #[arg(long)]
        confidence: bool,

        /// Include input value in output
        #[arg(short, long)]
        value: bool,

        /// Model type: char-cnn (166 types, default) or tiered (legacy 34-model cascade).
        /// Sense→Sharpen pipeline masks char-cnn output, making tiered routing redundant.
        #[arg(long, default_value = "char-cnn")]
        model_type: ModelType,

        /// Inference mode: row (per-value) or column (distribution-based disambiguation)
        #[arg(long, default_value = "row")]
        mode: InferenceMode,

        /// Sample size for column mode (default 100)
        #[arg(long, default_value = "100")]
        sample_size: usize,

        /// Print throughput statistics to stderr after inference
        #[arg(long)]
        bench: bool,

        /// Column name for header hint (used with --mode column)
        #[arg(long)]
        header: Option<String>,

        /// Read JSONL from stdin: {"header":"col_name","values":["v1","v2",...]}
        /// Outputs one JSON line per input with classification results.
        /// Requires --mode column.
        #[arg(long)]
        batch: bool,

        /// Disable Sense classifier (use Sharpen-only pipeline with header hints)
        #[arg(long)]
        sharp_only: bool,
    },

    /// Generate synthetic training data
    Generate {
        /// Number of samples per label
        #[arg(short, long, default_value = "100")]
        samples: usize,

        /// Minimum release priority
        #[arg(short, long, default_value = "3")]
        priority: u8,

        /// Output file
        #[arg(short, long, default_value = "training.ndjson")]
        output: PathBuf,

        /// Taxonomy file or directory
        #[arg(short, long, default_value = "labels")]
        taxonomy: PathBuf,

        /// Random seed for reproducibility
        #[arg(long, default_value = "42")]
        seed: u64,

        /// Generate 4-level labels with locale suffixes (domain.category.type.LOCALE)
        #[arg(long)]
        localized: bool,
    },

    /// Train a model
    #[command(hide = true)]
    Train {
        /// Training data file (NDJSON)
        #[arg(short, long)]
        data: PathBuf,

        /// Taxonomy file or directory
        #[arg(short, long, default_value = "labels")]
        taxonomy: PathBuf,

        /// Output directory for model
        #[arg(short, long, default_value = "models/default")]
        output: PathBuf,

        /// Number of epochs
        #[arg(short, long, default_value = "5")]
        epochs: usize,

        /// Batch size
        #[arg(short, long, default_value = "32")]
        batch_size: usize,

        /// Device (cpu, cuda, metal)
        #[arg(long, default_value = "cpu")]
        device: String,

        /// Model type (transformer, char_cnn)
        #[arg(long, default_value = "char-cnn")]
        model_type: ModelType,

        /// Random seed for deterministic training reproducibility
        #[arg(long)]
        seed: Option<u64>,
    },

    /// Show taxonomy information
    Taxonomy {
        /// Taxonomy file or directory
        #[arg(short, long, default_value = "labels")]
        file: PathBuf,

        /// Filter by domain
        #[arg(short, long)]
        domain: Option<String>,

        /// Filter by category
        #[arg(short, long)]
        category: Option<String>,

        /// Minimum release priority
        #[arg(long)]
        priority: Option<u8>,

        /// Output format (plain, json, csv)
        #[arg(short, long, default_value = "plain")]
        output: OutputFormat,

        /// Export all fields (description, validation, samples, etc.)
        #[arg(long)]
        full: bool,
    },

    /// Export JSON Schema for a type
    Schema {
        /// Type key (e.g., "identity.person.email") or glob pattern ("identity.person.*")
        type_key: String,

        /// Taxonomy file or directory
        #[arg(short, long, default_value = "labels")]
        file: PathBuf,

        /// Pretty-print JSON output
        #[arg(long)]
        pretty: bool,
    },

    /// Validate generator ↔ taxonomy alignment
    Check {
        /// Taxonomy file or directory
        #[arg(short, long, default_value = "labels")]
        taxonomy: PathBuf,

        /// Number of samples to generate per definition
        #[arg(short, long, default_value = "50")]
        samples: usize,

        /// Random seed for reproducibility
        #[arg(long, default_value = "42")]
        seed: u64,

        /// Minimum release priority to check (0 = all)
        #[arg(short, long)]
        priority: Option<u8>,

        /// Show verbose failure details
        #[arg(short, long)]
        verbose: bool,

        /// Output format (plain, json)
        #[arg(short, long, default_value = "plain")]
        output: OutputFormat,
    },

    /// Validate data quality against taxonomy schemas
    Validate {
        /// Input file (NDJSON with value/label fields, or plain text with --label)
        #[arg(short, long)]
        file: PathBuf,

        /// Validate all values against this label (for plain text input)
        #[arg(short, long)]
        label: Option<String>,

        /// Taxonomy file or directory
        #[arg(short, long, default_value = "labels")]
        taxonomy: PathBuf,

        /// Strategy for handling invalid values
        #[arg(long, default_value = "quarantine")]
        strategy: ValidateStrategy,

        /// Output format for quality report (plain, json, csv)
        #[arg(short, long, default_value = "plain")]
        output: OutputFormat,

        /// Quarantine file path (quarantine strategy only)
        #[arg(long, default_value = "quarantine.ndjson")]
        quarantine_file: PathBuf,

        /// Cleaned output file path (null/ffill/bfill strategies)
        #[arg(long, default_value = "cleaned.ndjson")]
        cleaned_file: PathBuf,
    },

    /// Profile a CSV file — detect column types using column-mode inference
    Profile {
        /// Input CSV file
        #[arg(short, long)]
        file: PathBuf,

        /// Model directory
        #[arg(short, long, default_value = "models/default")]
        model: PathBuf,

        /// Output format (plain, json, csv)
        #[arg(short, long, default_value = "plain")]
        output: OutputFormat,

        /// Maximum values to sample per column (default 100)
        #[arg(long, default_value = "100")]
        sample_size: usize,

        /// CSV delimiter character (default: auto-detect)
        #[arg(long)]
        delimiter: Option<char>,

        /// Disable column name header hints
        #[arg(long)]
        no_header_hint: bool,

        /// Model type (char-cnn, tiered, transformer)
        #[arg(long, default_value = "char-cnn")]
        model_type: ModelType,

        /// Disable Sense classifier (use Sharpen-only pipeline with header hints)
        #[arg(long)]
        sharp_only: bool,
    },

    /// Evaluate column-mode inference on GitTables benchmark
    #[command(hide = true)]
    EvalGittables {
        /// Directory containing GitTables benchmark data
        #[arg(short, long, default_value = "eval/gittables")]
        dir: PathBuf,

        /// Model directory
        #[arg(short, long, default_value = "models/default")]
        model: PathBuf,

        /// Maximum values to sample per column (default 100)
        #[arg(long, default_value = "100")]
        sample_size: usize,

        /// Output format (plain, json)
        #[arg(short, long, default_value = "plain")]
        output: OutputFormat,
    },

    /// Evaluate model accuracy on a test set
    #[command(hide = true)]
    Eval {
        /// Test data file (NDJSON with "text" and "classification" fields)
        #[arg(short, long)]
        data: PathBuf,

        /// Model directory
        #[arg(short, long, default_value = "models/default")]
        model: PathBuf,

        /// Taxonomy file or directory
        #[arg(short, long, default_value = "labels")]
        taxonomy: PathBuf,

        /// Model type (transformer, char_cnn)
        #[arg(long, default_value = "char-cnn")]
        model_type: ModelType,

        /// Number of top confusions to show
        #[arg(long, default_value = "20")]
        top_confusions: usize,

        /// Output format (plain, json)
        #[arg(short, long, default_value = "plain")]
        output: OutputFormat,
    },
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
enum OutputFormat {
    Plain,
    Json,
    Csv,
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
enum ModelType {
    Transformer,
    CharCnn,
    Tiered,
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
enum InferenceMode {
    /// Classify each value independently (default)
    Row,
    /// Treat all inputs as one column, use distribution to disambiguate
    Column,
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
enum ValidateStrategy {
    /// Quarantine invalid values to a separate file (default)
    Quarantine,
    /// Replace invalid values with NULL
    Null,
    /// Forward-fill: replace invalid with last valid value
    Ffill,
    /// Backward-fill: replace invalid with next valid value
    Bfill,
}

impl ValidateStrategy {
    fn name(&self) -> &'static str {
        match self {
            Self::Quarantine => "quarantine",
            Self::Null => "null",
            Self::Ffill => "ffill",
            Self::Bfill => "bfill",
        }
    }
}

fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Infer {
            input,
            file,
            model,
            output,
            confidence,
            value,
            model_type,
            mode,
            sample_size,
            bench,
            header,
            batch,
            sharp_only,
        } => cmd_infer(
            input,
            file,
            model,
            output,
            confidence,
            value,
            model_type,
            mode,
            sample_size,
            bench,
            header,
            batch,
            sharp_only,
        ),

        Commands::Generate {
            samples,
            priority,
            output,
            taxonomy,
            seed,
            localized,
        } => cmd_generate(samples, priority, output, taxonomy, seed, localized),

        Commands::Train {
            data,
            taxonomy,
            output,
            epochs,
            batch_size,
            device,
            model_type,
            seed,
        } => cmd_train(
            data, taxonomy, output, epochs, batch_size, device, model_type, seed,
        ),

        Commands::Taxonomy {
            file,
            domain,
            category,
            priority,
            output,
            full,
        } => cmd_taxonomy(file, domain, category, priority, output, full),

        Commands::Schema {
            type_key,
            file,
            pretty,
        } => cmd_schema(type_key, file, pretty),

        Commands::Check {
            taxonomy,
            samples,
            seed,
            priority,
            verbose,
            output,
        } => cmd_check(taxonomy, samples, seed, priority, verbose, output),

        Commands::Validate {
            file,
            label,
            taxonomy,
            strategy,
            output,
            quarantine_file,
            cleaned_file,
        } => cmd_validate(
            file,
            label,
            taxonomy,
            strategy,
            output,
            quarantine_file,
            cleaned_file,
        ),

        Commands::Profile {
            file,
            model,
            output,
            sample_size,
            delimiter,
            no_header_hint,
            model_type,
            sharp_only,
        } => cmd_profile(
            file,
            model,
            output,
            sample_size,
            delimiter,
            no_header_hint,
            model_type,
            sharp_only,
        ),

        Commands::EvalGittables {
            dir,
            model,
            sample_size,
            output,
        } => cmd_eval_gittables(dir, model, sample_size, output),

        Commands::Eval {
            data,
            model,
            taxonomy,
            model_type,
            top_confusions,
            output,
        } => cmd_eval(data, model, taxonomy, model_type, top_confusions, output),
    }
}

#[allow(clippy::too_many_arguments)]
fn cmd_infer(
    input: Option<String>,
    file: Option<PathBuf>,
    model: PathBuf,
    output: OutputFormat,
    show_confidence: bool,
    show_value: bool,
    model_type: ModelType,
    mode: InferenceMode,
    sample_size: usize,
    bench: bool,
    header: Option<String>,
    batch: bool,
    sharp_only: bool,
) -> Result<()> {
    use finetype_model::{ClassificationResult, ColumnClassifier, ColumnConfig};
    use std::time::Instant;

    // Batch mode: read JSONL from stdin, classify each column group
    if batch {
        if !matches!(mode, InferenceMode::Column) {
            anyhow::bail!("--batch requires --mode column");
        }
        return cmd_infer_batch(model, model_type, sample_size, sharp_only);
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

    let total_values = inputs.len();
    let t_start = Instant::now();

    // Load taxonomy for value-mode enrichment (locale detection, broad_type)
    let taxonomy_path = std::path::PathBuf::from("labels");
    let taxonomy = load_taxonomy(&taxonomy_path).ok().map(|mut t| {
        t.compile_locale_validators();
        t
    });

    /// Detect locale for a single value by testing it against all locale validators.
    /// Unlike `detect_locale_from_validation` (column mode, pass-rate ranking),
    /// this returns the first locale whose validator passes for a single value.
    fn detect_single_value_locale(value: &str, label: &str, taxonomy: &Taxonomy) -> Option<String> {
        let locale_validators = taxonomy.get_locale_validators(label)?;
        for (locale, validator) in locale_validators {
            if validator.validate(value).is_valid {
                return Some(locale.clone());
            }
        }
        None
    }

    // Helper to output result
    fn output_result(
        text: &str,
        result: &ClassificationResult,
        output: OutputFormat,
        show_value: bool,
        show_confidence: bool,
        taxonomy: Option<&Taxonomy>,
    ) {
        // Detect locale for suffix and JSON enrichment
        let locale = taxonomy.and_then(|tax| detect_single_value_locale(text, &result.label, tax));

        // Build display label: append .LOCALE suffix when detected
        let display_label = if let Some(ref loc) = locale {
            format!("{}.{}", result.label, loc)
        } else {
            result.label.clone()
        };

        match output {
            OutputFormat::Plain => {
                if show_value && show_confidence {
                    println!("{}\t{}\t{:.4}", text, display_label, result.confidence);
                } else if show_value {
                    println!("{}\t{}", text, display_label);
                } else if show_confidence {
                    println!("{}\t{:.4}", display_label, result.confidence);
                } else {
                    println!("{}", display_label);
                }
            }
            OutputFormat::Json => {
                let mut obj = serde_json::Map::new();
                obj.insert("label".to_string(), json!(result.label));
                if show_value {
                    obj.insert("input".to_string(), json!(text));
                }
                if show_confidence {
                    obj.insert("confidence".to_string(), json!(result.confidence));
                }
                // Enrich with taxonomy fields when available
                if let Some(tax) = taxonomy {
                    if let Some(def) = tax.get(&result.label) {
                        if let Some(ref bt) = def.broad_type {
                            obj.insert("broad_type".to_string(), json!(bt));
                        }
                    }
                }
                if let Some(ref loc) = locale {
                    obj.insert("locale".to_string(), json!(loc));
                }
                println!("{}", serde_json::Value::Object(obj));
            }
            OutputFormat::Csv => {
                if show_value && show_confidence {
                    println!(
                        "\"{}\",\"{}\",{:.4}",
                        text, display_label, result.confidence
                    );
                } else if show_value {
                    println!("\"{}\",\"{}\"", text, display_label);
                } else if show_confidence {
                    println!("\"{}\",{:.4}", display_label, result.confidence);
                } else {
                    println!("\"{}\"", display_label);
                }
            }
        }
    }

    // Column mode: treat all inputs as one column, return single prediction
    if matches!(mode, InferenceMode::Column) {
        let classifier: Box<dyn finetype_model::ValueClassifier> = match model_type {
            ModelType::CharCnn => Box::new(load_char_classifier(&model)?),
            ModelType::Tiered => Box::new(load_tiered_classifier(&model)?),
            ModelType::Transformer => Box::new(finetype_model::Classifier::load(&model)?),
        };
        let config = ColumnConfig {
            sample_size,
            ..Default::default()
        };
        let semantic_hint = load_semantic_hint();
        let mut column_classifier = if let Some(semantic) = semantic_hint {
            // Load entity classifier (shares Model2Vec tokenizer/embeddings)
            let entity = load_entity_classifier(&semantic);
            let mut cc = ColumnClassifier::with_semantic_hint(classifier, config, semantic);
            if let Some(entity) = entity {
                cc.set_entity_classifier(entity);
            }
            cc
        } else {
            ColumnClassifier::new(classifier, config)
        };

        // Load taxonomy for validation-based attractor demotion (Rule 14)
        let taxonomy_path = std::path::PathBuf::from("labels");
        if let Ok(mut taxonomy) = load_taxonomy(&taxonomy_path) {
            taxonomy.compile_validators();
            taxonomy.compile_locale_validators();
            column_classifier.set_taxonomy(taxonomy);
        }

        // Wire up Sense classifier (Sense → Sharpen pipeline)
        if !sharp_only {
            wire_sense(&mut column_classifier);
        }

        let result = if let Some(ref hdr) = header {
            column_classifier.classify_column_with_header(&inputs, hdr)?
        } else {
            column_classifier.classify_column(&inputs)?
        };

        match output {
            OutputFormat::Plain => {
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

    // Row mode: classify each value independently
    match model_type {
        ModelType::Transformer => {
            let classifier = Classifier::load(&model)?;
            let batch_size = 32;
            for chunk in inputs.chunks(batch_size) {
                let batch_texts: Vec<String> = chunk.to_vec();
                let results = classifier.classify_batch(&batch_texts)?;
                for (text, result) in chunk.iter().zip(results.iter()) {
                    output_result(
                        text,
                        result,
                        output,
                        show_value,
                        show_confidence,
                        taxonomy.as_ref(),
                    );
                }
            }
        }
        ModelType::CharCnn => {
            let classifier = load_char_classifier(&model)?;
            let batch_size = 128;
            for chunk in inputs.chunks(batch_size) {
                let batch_texts: Vec<String> = chunk.to_vec();
                let results = classifier.classify_batch(&batch_texts)?;
                for (text, result) in chunk.iter().zip(results.iter()) {
                    output_result(
                        text,
                        result,
                        output,
                        show_value,
                        show_confidence,
                        taxonomy.as_ref(),
                    );
                }
            }
        }
        ModelType::Tiered => {
            let classifier = load_tiered_classifier(&model)?;
            let batch_size = 128;
            if bench {
                // Use timed variant for tier-level breakdown
                let mut total_timing = finetype_model::TierTiming {
                    encode_ms: 0.0,
                    tier0_ms: 0.0,
                    tier1_ms: 0.0,
                    tier1_models: 0,
                    tier2_ms: 0.0,
                    tier2_models: 0,
                    total_ms: 0.0,
                };
                for chunk in inputs.chunks(batch_size) {
                    let batch_texts: Vec<String> = chunk.to_vec();
                    let (results, timing) = classifier.classify_batch_timed(&batch_texts)?;
                    total_timing.encode_ms += timing.encode_ms;
                    total_timing.tier0_ms += timing.tier0_ms;
                    total_timing.tier1_ms += timing.tier1_ms;
                    total_timing.tier1_models = total_timing.tier1_models.max(timing.tier1_models);
                    total_timing.tier2_ms += timing.tier2_ms;
                    total_timing.tier2_models = total_timing.tier2_models.max(timing.tier2_models);
                    total_timing.total_ms += timing.total_ms;
                    for (text, result) in chunk.iter().zip(results.iter()) {
                        output_result(
                            text,
                            result,
                            output,
                            show_value,
                            show_confidence,
                            taxonomy.as_ref(),
                        );
                    }
                }
                let elapsed = t_start.elapsed();
                let secs = elapsed.as_secs_f64();
                let vps = total_values as f64 / secs;
                eprintln!(
                    "[bench] model=Tiered  values={}  elapsed={:.3}s  throughput={:.0} val/sec",
                    total_values, secs, vps
                );
                eprintln!(
                    "[bench] breakdown: encode={:.1}ms  T0={:.1}ms  T1={:.1}ms ({} models)  T2={:.1}ms ({} models)",
                    total_timing.encode_ms, total_timing.tier0_ms,
                    total_timing.tier1_ms, total_timing.tier1_models,
                    total_timing.tier2_ms, total_timing.tier2_models
                );
                let inference_ms =
                    total_timing.tier0_ms + total_timing.tier1_ms + total_timing.tier2_ms;
                if inference_ms > 0.0 {
                    eprintln!(
                        "[bench] tier share: T0={:.1}%  T1={:.1}%  T2={:.1}%",
                        total_timing.tier0_ms / inference_ms * 100.0,
                        total_timing.tier1_ms / inference_ms * 100.0,
                        total_timing.tier2_ms / inference_ms * 100.0
                    );
                }
                return Ok(());
            }
            for chunk in inputs.chunks(batch_size) {
                let batch_texts: Vec<String> = chunk.to_vec();
                let results = classifier.classify_batch(&batch_texts)?;
                for (text, result) in chunk.iter().zip(results.iter()) {
                    output_result(
                        text,
                        result,
                        output,
                        show_value,
                        show_confidence,
                        taxonomy.as_ref(),
                    );
                }
            }
        }
    }

    if bench {
        let elapsed = t_start.elapsed();
        let secs = elapsed.as_secs_f64();
        let vps = total_values as f64 / secs;
        eprintln!(
            "[bench] model={:?}  values={}  elapsed={:.3}s  throughput={:.0} val/sec",
            model_type, total_values, secs, vps
        );
    }

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// INFER BATCH — JSONL column-mode batch classification (NNFT-130)
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
fn cmd_infer_batch(
    model: PathBuf,
    model_type: ModelType,
    sample_size: usize,
    sharp_only: bool,
) -> Result<()> {
    use finetype_model::{ColumnClassifier, ColumnConfig, ValueClassifier};
    use std::time::Instant;

    let t_start = Instant::now();

    // Load value-level classifier
    let classifier: Box<dyn ValueClassifier> = match model_type {
        ModelType::CharCnn => Box::new(load_char_classifier(&model)?),
        ModelType::Tiered => Box::new(load_tiered_classifier(&model)?),
        ModelType::Transformer => Box::new(finetype_model::Classifier::load(&model)?),
    };

    let config = ColumnConfig {
        sample_size,
        ..Default::default()
    };

    // Wire up semantic hint (Model2Vec) — same as profile command
    let mut column_classifier = if let Some(semantic) = load_semantic_hint() {
        eprintln!("Loaded semantic hint classifier (Model2Vec)");
        // Load entity classifier (shares Model2Vec tokenizer/embeddings)
        let entity = load_entity_classifier(&semantic);
        let mut cc = ColumnClassifier::with_semantic_hint(classifier, config, semantic);
        if let Some(entity) = entity {
            eprintln!("Loaded entity classifier (full_name demotion gate)");
            cc.set_entity_classifier(entity);
        }
        cc
    } else {
        ColumnClassifier::new(classifier, config)
    };

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

    // Wire up Sense classifier (Sense → Sharpen pipeline)
    if !sharp_only {
        wire_sense(&mut column_classifier);
    }

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

        let result = if !header_str.is_empty() {
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

/// Load a CharClassifier: try the model directory first, then fall back to
/// the embedded model if the path doesn't exist (release binaries).
///
/// Automatically loads validation patterns from the taxonomy to enable
/// pattern-gated post-processing (NNFT-064).
fn load_char_classifier(model: &PathBuf) -> Result<finetype_model::CharClassifier> {
    let mut classifier = if model.exists() {
        finetype_model::CharClassifier::load(model)?
    } else {
        #[cfg(feature = "embed-models")]
        {
            finetype_model::CharClassifier::from_bytes(
                embedded::FLAT_WEIGHTS,
                embedded::FLAT_LABELS,
                embedded::FLAT_CONFIG,
            )?
        }
        #[cfg(not(feature = "embed-models"))]
        {
            anyhow::bail!(
                "Model directory {:?} not found. Build with `embed-models` feature for standalone use.",
                model
            )
        }
    };

    // Load validation patterns from taxonomy for pattern-gated post-processing.
    // This validates model predictions against taxonomy regex patterns and falls
    // back to next-best predictions on mismatch (e.g., "C85" ≠ iata_code pattern).
    let taxonomy_path = PathBuf::from("labels");
    if let Ok(taxonomy) = load_taxonomy(&taxonomy_path) {
        let patterns = finetype_model::extract_validation_patterns(&taxonomy);
        if !patterns.is_empty() {
            classifier.set_validation_patterns(patterns);
        }
    }

    Ok(classifier)
}

/// Load a TieredClassifier: try the model directory first, then fall back to
/// the embedded tiered model if the path doesn't exist (release binaries).
fn load_tiered_classifier(model: &PathBuf) -> Result<finetype_model::TieredClassifier> {
    if model.exists() && model.join("tier_graph.json").exists() {
        Ok(finetype_model::TieredClassifier::load(model)?)
    } else {
        #[cfg(feature = "embed-models")]
        {
            if embedded::EMBEDDED_MODEL_TYPE == "tiered" {
                Ok(finetype_model::TieredClassifier::from_embedded(
                    embedded::TIER_GRAPH,
                    embedded::get_tiered_model_data,
                )?)
            } else {
                anyhow::bail!(
                    "Tiered model not found at {:?} and embedded model is flat. \
                     Use --model-type char-cnn or provide a tiered model path.",
                    model
                )
            }
        }
        #[cfg(not(feature = "embed-models"))]
        {
            anyhow::bail!(
                "Model directory {:?} not found. Build with `embed-models` feature for standalone use.",
                model
            )
        }
    }
}

/// Load the semantic hint classifier for column name classification.
///
/// Resolution order:
///  1. models/model2vec directory on disk (development)
///  2. Embedded Model2Vec bytes (release binaries)
///  3. None — falls back to hardcoded header_hint()
fn load_semantic_hint() -> Option<finetype_model::SemanticHintClassifier> {
    // Try disk-based model first (development workflow)
    let model_dir = std::path::PathBuf::from("models/model2vec");
    if model_dir.join("model.safetensors").exists() {
        return finetype_model::SemanticHintClassifier::load(&model_dir)
            .map_err(|e| eprintln!("Warning: Failed to load Model2Vec from disk: {e}"))
            .ok();
    }

    // Try embedded model bytes (release binary)
    #[cfg(feature = "embed-models")]
    {
        if embedded::HAS_MODEL2VEC {
            return finetype_model::SemanticHintClassifier::from_bytes(
                embedded::M2V_TOKENIZER,
                embedded::M2V_MODEL,
                embedded::M2V_TYPE_EMBEDDINGS,
                embedded::M2V_LABEL_INDEX,
            )
            .map_err(|e| eprintln!("Warning: Failed to load embedded Model2Vec: {e}"))
            .ok();
        }
    }

    None
}

/// Load the entity classifier for full_name demotion (NNFT-152).
///
/// Requires a loaded SemanticHintClassifier to share the Model2Vec tokenizer
/// and embeddings. Resolution order:
///  1. models/entity-classifier directory on disk (development)
///  2. Embedded entity classifier bytes (release binaries)
///  3. None — entity demotion disabled
fn load_entity_classifier(
    semantic: &finetype_model::SemanticHintClassifier,
) -> Option<finetype_model::EntityClassifier> {
    // Try disk-based model first (development workflow)
    let model_dir = std::path::PathBuf::from("models/entity-classifier");
    if model_dir.join("model.safetensors").exists() {
        return finetype_model::EntityClassifier::load(
            &model_dir,
            semantic.tokenizer().clone(),
            semantic.embeddings().clone(),
        )
        .map_err(|e| eprintln!("Warning: Failed to load entity classifier from disk: {e}"))
        .ok();
    }

    // Try embedded model bytes (release binary)
    #[cfg(feature = "embed-models")]
    {
        if embedded::HAS_ENTITY_CLASSIFIER {
            return finetype_model::EntityClassifier::from_bytes(
                embedded::ENTITY_MODEL,
                embedded::ENTITY_CONFIG,
                semantic.tokenizer().clone(),
                semantic.embeddings().clone(),
            )
            .map_err(|e| eprintln!("Warning: Failed to load embedded entity classifier: {e}"))
            .ok();
        }
    }

    None
}

/// Load the Sense classifier for broad category prediction (NNFT-171).
///
/// Resolution order:
///  1. models/sense directory on disk (development)
///  2. Embedded Sense bytes (release binaries)
///  3. None — Sense pipeline disabled, uses legacy header hints
fn load_sense() -> Option<finetype_model::SenseClassifier> {
    // Try disk-based model first (development workflow)
    let model_dir = std::path::PathBuf::from("models/sense");
    if model_dir.join("model.safetensors").exists() {
        return finetype_model::SenseClassifier::load(&model_dir)
            .map_err(|e| eprintln!("Warning: Failed to load Sense classifier from disk: {e}"))
            .ok();
    }

    // Try embedded model bytes (release binary)
    #[cfg(feature = "embed-models")]
    {
        if embedded::HAS_SENSE_CLASSIFIER {
            return finetype_model::SenseClassifier::from_bytes(
                embedded::SENSE_MODEL,
                embedded::SENSE_CONFIG,
            )
            .map_err(|e| eprintln!("Warning: Failed to load embedded Sense classifier: {e}"))
            .ok();
        }
    }

    None
}

/// Load shared Model2Vec resources (tokenizer + embeddings).
///
/// Resolution order:
///  1. models/model2vec directory on disk (development)
///  2. Embedded Model2Vec bytes (release binaries)
///  3. None — no shared resources available
fn load_model2vec_resources() -> Option<finetype_model::Model2VecResources> {
    // Try disk-based model first (development workflow)
    let model_dir = std::path::PathBuf::from("models/model2vec");
    if model_dir.join("model.safetensors").exists() {
        return finetype_model::Model2VecResources::load(&model_dir)
            .map_err(|e| eprintln!("Warning: Failed to load Model2Vec resources from disk: {e}"))
            .ok();
    }

    // Try embedded model bytes (release binary)
    #[cfg(feature = "embed-models")]
    {
        if embedded::HAS_MODEL2VEC {
            return finetype_model::Model2VecResources::from_bytes(
                embedded::M2V_TOKENIZER,
                embedded::M2V_MODEL,
            )
            .map_err(|e| eprintln!("Warning: Failed to load embedded Model2Vec resources: {e}"))
            .ok();
        }
    }

    None
}

/// Wire up the Sense classifier into a ColumnClassifier.
///
/// Loads Model2VecResources + SenseClassifier + LabelCategoryMap and calls
/// `set_sense()`. Silently skips if any component is unavailable.
fn wire_sense(cc: &mut finetype_model::ColumnClassifier) {
    let sense = match load_sense() {
        Some(s) => s,
        None => return,
    };
    let m2v = match load_model2vec_resources() {
        Some(r) => r,
        None => {
            eprintln!("Warning: Sense classifier loaded but Model2Vec resources unavailable — Sense disabled");
            return;
        }
    };
    let label_map = finetype_model::LabelCategoryMap::new();
    eprintln!("Loaded Sense classifier (broad category prediction)");
    cc.set_sense(sense, m2v, label_map);
}

// ═══════════════════════════════════════════════════════════════════════════════
// GENERATE — Create synthetic training data
// ═══════════════════════════════════════════════════════════════════════════════

fn cmd_generate(
    samples: usize,
    priority: u8,
    output: PathBuf,
    taxonomy_path: PathBuf,
    seed: u64,
    localized: bool,
) -> Result<()> {
    eprintln!("Loading taxonomy from {:?}", taxonomy_path);

    let taxonomy = load_taxonomy(&taxonomy_path)?;

    eprintln!(
        "Loaded {} label definitions across {} domains",
        taxonomy.len(),
        taxonomy.domains().len()
    );

    let mode = if localized {
        "localized (4-level)"
    } else {
        "flat (3-level)"
    };
    eprintln!(
        "Generating {} samples per label (priority >= {}, mode: {})",
        samples, priority, mode
    );

    let mut generator = Generator::with_seed(taxonomy, seed);
    let all_samples = if localized {
        generator.generate_all_localized(priority, samples)
    } else {
        generator.generate_all(priority, samples)
    };

    eprintln!("Generated {} total samples", all_samples.len());

    // Write to file
    let mut file = std::fs::File::create(&output)?;
    for sample in all_samples {
        let record = json!({
            "text": sample.text,
            "classification": sample.label,
        });
        writeln!(file, "{}", record)?;
    }

    eprintln!("Saved to {:?}", output);
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// TRAIN — Train a classification model
// ═══════════════════════════════════════════════════════════════════════════════

#[allow(clippy::too_many_arguments)]
fn cmd_train(
    data: PathBuf,
    taxonomy_path: PathBuf,
    output: PathBuf,
    epochs: usize,
    batch_size: usize,
    _device: String,
    model_type: ModelType,
    seed: Option<u64>,
) -> Result<()> {
    use finetype_core::Sample;
    use std::io::BufRead;

    eprintln!("Loading taxonomy from {:?}", taxonomy_path);
    let taxonomy = load_taxonomy(&taxonomy_path)?;
    eprintln!("Loaded {} label definitions", taxonomy.len());

    eprintln!("Loading training data from {:?}", data);
    let file = std::fs::File::open(&data)?;
    let reader = std::io::BufReader::new(file);

    let mut samples = Vec::new();
    for line in reader.lines() {
        let line = line?;
        if line.is_empty() {
            continue;
        }
        let record: serde_json::Value = serde_json::from_str(&line)?;
        let text = record["text"].as_str().unwrap_or("").to_string();
        let label = record["classification"].as_str().unwrap_or("").to_string();
        samples.push(Sample { text, label });
    }
    eprintln!("Loaded {} training samples", samples.len());

    // Snapshot: if output directory already contains model files, back it up
    let snapshot_path = snapshot_model_dir(&output)?;

    match model_type {
        ModelType::Transformer => {
            use finetype_model::{Trainer, TrainingConfig};

            let config = TrainingConfig {
                batch_size,
                epochs,
                learning_rate: 1e-4,
                max_seq_length: 128,
                warmup_steps: 100,
                weight_decay: 0.01,
            };

            eprintln!("Training Transformer model");
            eprintln!("Training config: {:?}", config);

            let trainer = Trainer::new(config);
            trainer.train(&taxonomy, &samples, &output)?;
        }
        ModelType::CharCnn => {
            use finetype_model::{CharTrainer, CharTrainingConfig};

            let config = CharTrainingConfig {
                batch_size,
                epochs,
                learning_rate: 1e-3,
                max_seq_length: 128,
                embed_dim: 32,
                num_filters: 64,
                hidden_dim: 128,
                weight_decay: 1e-4,
                shuffle: true,
                seed,
            };

            eprintln!("Training CharCNN model");
            eprintln!("Training config: {:?}", config);

            let trainer = CharTrainer::new(config);
            trainer.train(&taxonomy, &samples, &output)?;
        }
        ModelType::Tiered => {
            use finetype_model::{TieredTrainer, TieredTrainingConfig};

            let config = TieredTrainingConfig {
                batch_size,
                epochs,
                learning_rate: 1e-3,
                max_seq_length: 128,
                embed_dim: 32,
                num_filters: 64,
                hidden_dim: 128,
                weight_decay: 1e-4,
                tier2_min_types: 1,
                seed,
            };

            eprintln!("Training Tiered models (Tier 0 -> Tier 1 -> Tier 2)");
            eprintln!("Training config: {:?}", config);

            let trainer = TieredTrainer::new(config);
            let report = trainer.train_all(&taxonomy, &samples, &output)?;
            eprintln!("{}", report);
        }
    }

    // Write training manifest
    TrainingManifest {
        output: &output,
        data_file: &data,
        epochs,
        batch_size,
        seed,
        model_type: &model_type,
        n_classes: taxonomy.len(),
        n_samples: samples.len(),
        snapshot_path: snapshot_path.as_deref(),
    }
    .write()?;

    eprintln!("Training complete! Model saved to {:?}", output);
    Ok(())
}

/// Snapshot an existing model directory before overwriting.
///
/// If the output directory exists and contains model files (model.safetensors
/// or tier_graph.json), copies it to `{output}.snapshot.{ISO-timestamp}`.
/// Returns the snapshot path if a snapshot was taken, or None.
fn snapshot_model_dir(output: &Path) -> Result<Option<PathBuf>> {
    if !output.exists() {
        return Ok(None);
    }

    // Check for model files that indicate a trained model lives here
    let has_model = output.join("model.safetensors").exists()
        || output.join("tier_graph.json").exists()
        || output.join("tier0").join("model.safetensors").exists();

    if !has_model {
        return Ok(None);
    }

    let timestamp = chrono::Utc::now().format("%Y%m%dT%H%M%SZ");
    let dir_name = output
        .file_name()
        .map(|n: &std::ffi::OsStr| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "model".to_string());
    let snapshot_name = format!("{}.snapshot.{}", dir_name, timestamp);
    let snapshot_path = output
        .parent()
        .unwrap_or(Path::new("."))
        .join(&snapshot_name);

    eprintln!("Snapshot: backing up {:?} -> {:?}", output, snapshot_path);
    copy_dir_recursive(output, &snapshot_path)?;
    eprintln!("Snapshot complete: {:?}", snapshot_path);

    Ok(Some(snapshot_path))
}

/// Recursively copy a directory.
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

/// Training provenance metadata written alongside model artifacts.
struct TrainingManifest<'a> {
    output: &'a Path,
    data_file: &'a Path,
    epochs: usize,
    batch_size: usize,
    seed: Option<u64>,
    model_type: &'a ModelType,
    n_classes: usize,
    n_samples: usize,
    snapshot_path: Option<&'a Path>,
}

impl TrainingManifest<'_> {
    /// Write manifest.json to the model output directory.
    fn write(&self) -> Result<()> {
        let manifest = serde_json::json!({
            "data_file": self.data_file.to_string_lossy(),
            "epochs": self.epochs,
            "batch_size": self.batch_size,
            "seed": self.seed,
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "model_type": format!("{:?}", self.model_type).to_lowercase(),
            "n_classes": self.n_classes,
            "n_samples": self.n_samples,
            "parent_snapshot": self.snapshot_path.map(|p: &Path| p.to_string_lossy().to_string()),
        });

        let manifest_str = serde_json::to_string_pretty(&manifest)?;
        std::fs::create_dir_all(self.output)?;
        std::fs::write(self.output.join("manifest.json"), manifest_str)?;
        eprintln!(
            "Training manifest written to {:?}",
            self.output.join("manifest.json")
        );

        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// TAXONOMY — Display taxonomy information
// ═══════════════════════════════════════════════════════════════════════════════

fn cmd_taxonomy(
    file: PathBuf,
    domain: Option<String>,
    category: Option<String>,
    priority: Option<u8>,
    output: OutputFormat,
    full: bool,
) -> Result<()> {
    let taxonomy = load_taxonomy(&file)?;

    // Collect matching definitions
    let mut defs: Vec<(&String, &finetype_core::Definition)> =
        if let (Some(dom), Some(cat)) = (&domain, &category) {
            taxonomy.by_category(dom, cat)
        } else if let Some(dom) = &domain {
            taxonomy.by_domain(dom)
        } else if let Some(prio) = priority {
            taxonomy.at_priority(prio)
        } else {
            taxonomy.definitions().collect()
        };

    // Apply priority filter even when domain/category is set
    if let Some(prio) = priority {
        defs.retain(|(_, d)| d.release_priority >= prio);
    }

    defs.sort_by_key(|(k, _)| (*k).clone());

    match output {
        OutputFormat::Plain => {
            println!("Domains: {:?}", taxonomy.domains());
            println!("Total labels: {}", taxonomy.len());
            if let Some(dom) = &domain {
                println!("Categories in {}: {:?}", dom, taxonomy.categories(dom));
            }
            println!();

            for (key, def) in &defs {
                let broad = def.broad_type.as_deref().unwrap_or("?");
                println!(
                    "{} \u{2192} {} (priority: {}, {:?})",
                    key, broad, def.release_priority, def.designation
                );
                if let Some(title) = &def.title {
                    println!("  {}", title);
                }
            }

            println!("\n{} definitions shown", defs.len());
        }
        OutputFormat::Json => {
            let labels: Vec<_> = defs
                .iter()
                .map(|(key, d)| {
                    if full {
                        definition_to_full_json(key, d)
                    } else {
                        json!({
                            "key": key,
                            "title": d.title,
                            "broad_type": d.broad_type,
                            "designation": format!("{:?}", d.designation),
                            "priority": d.release_priority,
                            "transform": d.transform,
                            "locales": d.locales,
                        })
                    }
                })
                .collect();
            println!("{}", serde_json::to_string_pretty(&labels)?);
        }
        OutputFormat::Csv => {
            println!("key,broad_type,priority,designation,title");
            for (key, def) in &defs {
                println!(
                    "\"{}\",\"{}\",{},\"{:?}\",\"{}\"",
                    key,
                    def.broad_type.as_deref().unwrap_or(""),
                    def.release_priority,
                    def.designation,
                    def.title.as_deref().unwrap_or("")
                );
            }
        }
    }

    Ok(())
}

/// Convert a Serialize value to serde_json::Value.
/// Used for serde_yaml::Value fields (samples, references, decompose) that need JSON output.
fn to_json_value<T: serde::Serialize>(value: &T) -> serde_json::Value {
    serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
}

/// Serialize a Definition with all fields for --full export.
fn definition_to_full_json(key: &str, d: &finetype_core::Definition) -> serde_json::Value {
    let label = Label::parse(key);

    let samples: serde_json::Value = to_json_value(&d.samples);

    let validation = d.validation.as_ref().map(|v| v.to_json_schema());

    let validation_by_locale: Option<serde_json::Map<String, serde_json::Value>> =
        d.validation_by_locale.as_ref().map(|locales| {
            locales
                .iter()
                .map(|(locale, v)| (locale.clone(), v.to_json_schema()))
                .collect()
        });

    let decompose = d.decompose.as_ref().map(to_json_value);

    let references = d.references.as_ref().map(to_json_value);

    // Serialize designation as snake_case string via serde
    let designation = serde_json::to_value(&d.designation).unwrap_or(json!("universal"));

    let mut obj = serde_json::Map::new();
    obj.insert("key".into(), json!(key));
    if let Some(ref l) = label {
        obj.insert("domain".into(), json!(l.domain));
        obj.insert("category".into(), json!(l.category));
        obj.insert("type".into(), json!(l.type_name));
    }
    obj.insert("title".into(), json!(d.title));
    obj.insert("description".into(), json!(d.description));
    obj.insert("designation".into(), designation);
    obj.insert("broad_type".into(), json!(d.broad_type));
    obj.insert("format_string".into(), json!(d.format_string));
    obj.insert("transform".into(), json!(d.transform));
    obj.insert("transform_ext".into(), json!(d.transform_ext));
    obj.insert("locales".into(), json!(d.locales));
    obj.insert("tier".into(), json!(d.tier));
    obj.insert("release_priority".into(), json!(d.release_priority));
    obj.insert("aliases".into(), json!(d.aliases));
    obj.insert("notes".into(), json!(d.notes));
    obj.insert("samples".into(), json!(samples));
    obj.insert(
        "validation".into(),
        validation.unwrap_or(serde_json::Value::Null),
    );
    if let Some(locales) = validation_by_locale {
        obj.insert(
            "validation_by_locale".into(),
            serde_json::Value::Object(locales),
        );
    }
    if let Some(dec) = decompose {
        obj.insert("decompose".into(), dec);
    }
    if let Some(refs) = references {
        obj.insert("references".into(), refs);
    }

    serde_json::Value::Object(obj)
}

// ═══════════════════════════════════════════════════════════════════════════════
// SCHEMA — Export JSON Schema for types
// ═══════════════════════════════════════════════════════════════════════════════

fn cmd_schema(type_key: String, file: PathBuf, pretty: bool) -> Result<()> {
    let taxonomy = load_taxonomy(&file)?;

    // Collect matching definitions: exact match or glob
    let matches: Vec<(&String, &finetype_core::Definition)> = if type_key.contains('*') {
        // Glob pattern — support "domain.*", "domain.category.*", "*.category.*"
        let prefix = type_key.trim_end_matches(".*").trim_end_matches('*');
        let mut matched: Vec<_> = taxonomy
            .definitions()
            .filter(|(k, _)| {
                if prefix.is_empty() {
                    true // "*" matches all
                } else {
                    k.starts_with(prefix)
                        && (k.len() == prefix.len()
                            || k.as_bytes().get(prefix.len()) == Some(&b'.'))
                }
            })
            .collect();
        matched.sort_by_key(|(k, _)| (*k).clone());
        matched
    } else {
        // Exact match
        match taxonomy.get(&type_key) {
            Some(_) => taxonomy
                .definitions()
                .filter(|(k, _)| k.as_str() == type_key)
                .collect(),
            None => {
                // Suggest similar types by edit distance
                let mut suggestions: Vec<(&String, usize)> = taxonomy
                    .definitions()
                    .map(|(k, _)| (k, levenshtein_distance(&type_key, k)))
                    .collect();
                suggestions.sort_by_key(|(_, d)| *d);
                suggestions.truncate(5);

                eprintln!("Error: unknown type '{}'", type_key);
                if !suggestions.is_empty() {
                    eprintln!("\nDid you mean:");
                    for (s, _) in &suggestions {
                        eprintln!("  {}", s);
                    }
                }
                std::process::exit(1);
            }
        }
    };

    if matches.is_empty() {
        eprintln!("No types matching '{}'", type_key);
        std::process::exit(1);
    }

    let schemas: Vec<serde_json::Value> = matches
        .iter()
        .map(|(key, def)| build_json_schema(key, def))
        .collect();

    let output = if schemas.len() == 1 {
        &schemas[0]
    } else {
        // Multiple results — wrap in array
        &serde_json::Value::Array(schemas.clone())
    };

    let json_str = if pretty {
        serde_json::to_string_pretty(output)?
    } else {
        serde_json::to_string(output)?
    };
    println!("{}", json_str);

    Ok(())
}

/// Build an enriched JSON Schema document for a type definition.
fn build_json_schema(key: &str, def: &finetype_core::Definition) -> serde_json::Value {
    let mut schema = serde_json::Map::new();

    // Standard JSON Schema metadata
    schema.insert(
        "$schema".into(),
        json!("https://json-schema.org/draft/2020-12/schema"),
    );
    schema.insert(
        "$id".into(),
        json!(format!("https://noon.sh/schemas/{}", key)),
    );

    if let Some(title) = &def.title {
        schema.insert("title".into(), json!(title));
    }
    if let Some(desc) = &def.description {
        schema.insert("description".into(), json!(desc.trim()));
    }

    // Merge validation keywords from the type's validation schema
    if let Some(validation) = &def.validation {
        let val_schema = validation.to_json_schema();
        if let serde_json::Value::Object(val_obj) = val_schema {
            for (k, v) in val_obj {
                schema.insert(k, v);
            }
        }
    } else {
        // No validation — default to string type
        schema.insert("type".into(), json!("string"));
    }

    // Add examples from samples
    if !def.samples.is_empty() {
        schema.insert("examples".into(), to_json_value(&def.samples));
    }

    serde_json::Value::Object(schema)
}

/// Simple Levenshtein distance for type name suggestions.
fn levenshtein_distance(a: &str, b: &str) -> usize {
    let b_len = b.len();
    let mut prev = (0..=b_len).collect::<Vec<_>>();
    let mut curr = vec![0; b_len + 1];
    for (i, ca) in a.chars().enumerate() {
        curr[0] = i + 1;
        for (j, cb) in b.chars().enumerate() {
            let cost = if ca == cb { 0 } else { 1 };
            curr[j + 1] = (prev[j + 1] + 1).min(curr[j] + 1).min(prev[j] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[b_len]
}

// ═══════════════════════════════════════════════════════════════════════════════
// CHECK — Validate generator ↔ taxonomy alignment
// ═══════════════════════════════════════════════════════════════════════════════

fn cmd_check(
    taxonomy_path: PathBuf,
    samples: usize,
    seed: u64,
    priority: Option<u8>,
    verbose: bool,
    output: OutputFormat,
) -> Result<()> {
    eprintln!("Loading taxonomy from {:?}", taxonomy_path);
    let taxonomy = load_taxonomy(&taxonomy_path)?;
    eprintln!("Loaded {} definitions", taxonomy.len());

    let checker = Checker::new(samples).with_seed(seed);
    eprintln!(
        "Checking {} samples per definition (seed={})...",
        samples, seed
    );

    let report = checker.run(&taxonomy);

    match output {
        OutputFormat::Plain => {
            print!("{}", format_report(&report, verbose));
        }
        OutputFormat::Json => {
            let results: Vec<serde_json::Value> = report
                .results
                .iter()
                .filter(|r| priority.map(|p| r.release_priority >= p).unwrap_or(true))
                .map(|r| {
                    let mut obj = serde_json::Map::new();
                    obj.insert("key".to_string(), json!(r.key));
                    obj.insert("domain".to_string(), json!(r.domain));
                    obj.insert("generator_exists".to_string(), json!(r.generator_exists));
                    obj.insert("samples_generated".to_string(), json!(r.samples_generated));
                    obj.insert("samples_passed".to_string(), json!(r.samples_passed));
                    obj.insert("samples_failed".to_string(), json!(r.samples_failed));
                    obj.insert("pass_rate".to_string(), json!(r.pass_rate()));
                    obj.insert("has_pattern".to_string(), json!(r.has_pattern));
                    obj.insert("release_priority".to_string(), json!(r.release_priority));
                    obj.insert("passed".to_string(), json!(r.passed()));
                    if !r.failures.is_empty() {
                        let failures: Vec<serde_json::Value> = r
                            .failures
                            .iter()
                            .map(|f| {
                                json!({
                                    "sample": f.sample,
                                    "reason": format!("{}", f.reason),
                                })
                            })
                            .collect();
                        obj.insert("failures".to_string(), json!(failures));
                    }
                    serde_json::Value::Object(obj)
                })
                .collect();

            let summary = json!({
                "total_definitions": report.total_definitions,
                "generators_found": report.generators_found,
                "generators_missing": report.generators_missing,
                "fully_passing": report.fully_passing,
                "has_failures": report.has_failures,
                "no_pattern": report.no_pattern,
                "total_samples": report.total_samples,
                "total_passed": report.total_passed,
                "total_failed": report.total_failed,
                "pass_rate": report.pass_rate(),
                "all_passed": report.all_passed(),
                "results": results,
            });

            println!("{}", serde_json::to_string_pretty(&summary)?);
        }
        OutputFormat::Csv => {
            println!("key,domain,generator_exists,samples_generated,samples_passed,samples_failed,pass_rate,has_pattern,priority,passed");
            for r in &report.results {
                if priority.map(|p| r.release_priority >= p).unwrap_or(true) {
                    println!(
                        "\"{}\",\"{}\",{},{},{},{},{:.4},{},{},{}",
                        r.key,
                        r.domain,
                        r.generator_exists,
                        r.samples_generated,
                        r.samples_passed,
                        r.samples_failed,
                        r.pass_rate(),
                        r.has_pattern,
                        r.release_priority,
                        r.passed(),
                    );
                }
            }
        }
    }

    // Exit non-zero if checks failed
    if !report.all_passed() {
        std::process::exit(1);
    }

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// VALIDATE — Validate data quality against taxonomy schemas
// ═══════════════════════════════════════════════════════════════════════════════

#[allow(clippy::too_many_arguments)]
fn cmd_validate(
    file: PathBuf,
    label: Option<String>,
    taxonomy_path: PathBuf,
    strategy: ValidateStrategy,
    output: OutputFormat,
    quarantine_file: PathBuf,
    cleaned_file: PathBuf,
) -> Result<()> {
    use finetype_core::{
        validate_column_for_label, ColumnValidationResult, InvalidStrategy, ValidationCheck,
    };
    use std::collections::HashMap;

    // Load taxonomy
    eprintln!("Loading taxonomy from {:?}", taxonomy_path);
    let taxonomy = load_taxonomy(&taxonomy_path)?;
    eprintln!("Loaded {} label definitions", taxonomy.len());

    // Track whether input was plain text (for output formatting)
    let is_plain_text = label.is_some();

    // Parse input into (row_index, value, label) tuples
    let content = std::fs::read_to_string(&file)?;

    struct InputRow {
        row_index: usize,
        value: Option<String>,
        label: String,
    }

    let mut rows: Vec<InputRow> = Vec::new();

    if let Some(ref lbl) = label {
        // Plain text mode: each line is a value, all validated against the same label
        for (i, line) in content.lines().enumerate() {
            let value = if line.is_empty() || line == "NULL" || line == "null" {
                None
            } else {
                Some(line.to_string())
            };
            rows.push(InputRow {
                row_index: i,
                value,
                label: lbl.clone(),
            });
        }
    } else {
        // NDJSON mode: each line has value + label fields
        for (i, line) in content.lines().enumerate() {
            if line.is_empty() {
                continue;
            }
            let record: serde_json::Value =
                serde_json::from_str(line).map_err(|e| anyhow::anyhow!("Line {}: {}", i + 1, e))?;

            // Support both (value/label) and (input/class) field names
            let value = record
                .get("value")
                .or_else(|| record.get("input"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let lbl = record
                .get("label")
                .or_else(|| record.get("class"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Line {}: missing 'label' or 'class' field", i + 1))?
                .to_string();

            rows.push(InputRow {
                row_index: i,
                value,
                label: lbl,
            });
        }
    }

    if rows.is_empty() {
        eprintln!("No input data");
        return Ok(());
    }

    eprintln!("Read {} rows", rows.len());

    // Group by label, preserving original row indices
    let mut groups: HashMap<String, Vec<(usize, Option<String>)>> = HashMap::new();
    for row in &rows {
        groups
            .entry(row.label.clone())
            .or_default()
            .push((row.row_index, row.value.clone()));
    }

    // Map CLI strategy to core strategy
    let core_strategy = match strategy {
        ValidateStrategy::Quarantine => InvalidStrategy::Quarantine,
        ValidateStrategy::Null => InvalidStrategy::SetNull,
        ValidateStrategy::Ffill => InvalidStrategy::ForwardFill,
        ValidateStrategy::Bfill => InvalidStrategy::BackwardFill,
    };

    // Validate each group
    struct GroupResult {
        label: String,
        result: ColumnValidationResult,
        original_row_indices: Vec<usize>,
    }

    let mut group_results: Vec<GroupResult> = Vec::new();
    let mut any_invalid = false;

    let mut sorted_labels: Vec<String> = groups.keys().cloned().collect();
    sorted_labels.sort();

    for lbl in &sorted_labels {
        let entries = groups.get(lbl).unwrap();
        let original_indices: Vec<usize> = entries.iter().map(|(idx, _)| *idx).collect();
        let values: Vec<Option<&str>> = entries.iter().map(|(_, v)| v.as_deref()).collect();

        match validate_column_for_label(&values, lbl, &taxonomy, core_strategy) {
            Ok(result) => {
                if result.stats.invalid_count > 0 {
                    any_invalid = true;
                }
                group_results.push(GroupResult {
                    label: lbl.clone(),
                    result,
                    original_row_indices: original_indices,
                });
            }
            Err(e) => {
                eprintln!("Warning: skipping label '{}': {}", lbl, e);
            }
        }
    }

    // Aggregate totals
    let total_values: usize = group_results
        .iter()
        .map(|g| g.result.stats.total_count)
        .sum();
    let total_valid: usize = group_results
        .iter()
        .map(|g| g.result.stats.valid_count)
        .sum();
    let total_invalid: usize = group_results
        .iter()
        .map(|g| g.result.stats.invalid_count)
        .sum();
    let total_null: usize = group_results
        .iter()
        .map(|g| g.result.stats.null_count)
        .sum();

    // ── Output quality report ──────────────────────────────────────────────
    match output {
        OutputFormat::Plain => {
            println!("Data Quality Report");
            println!("{}", "═".repeat(60));
            println!();

            for gr in &group_results {
                let s = &gr.result.stats;
                let valid_pct = if s.total_count > 0 {
                    s.valid_count as f64 / s.total_count as f64 * 100.0
                } else {
                    0.0
                };
                let invalid_pct = if s.total_count > 0 {
                    s.invalid_count as f64 / s.total_count as f64 * 100.0
                } else {
                    0.0
                };
                let null_pct = if s.total_count > 0 {
                    s.null_count as f64 / s.total_count as f64 * 100.0
                } else {
                    0.0
                };

                println!("Column: {} ({} values)", gr.label, s.total_count);
                println!("  Valid:    {:>6} ({:>5.1}%)", s.valid_count, valid_pct);
                println!("  Invalid:  {:>6} ({:>5.1}%)", s.invalid_count, invalid_pct);
                println!("  Null:     {:>6} ({:>5.1}%)", s.null_count, null_pct);
                println!(
                    "  Validity: {:>5.1}% (of non-null)",
                    s.validity_rate() * 100.0
                );

                if !s.error_patterns.is_empty() {
                    println!("  Top errors:");
                    let mut sorted: Vec<(&ValidationCheck, &usize)> =
                        s.error_patterns.iter().collect();
                    sorted.sort_by(|a, b| b.1.cmp(a.1));
                    for (check, count) in sorted {
                        let pct = if s.invalid_count > 0 {
                            *count as f64 / s.invalid_count as f64 * 100.0
                        } else {
                            0.0
                        };
                        println!("    {:<12} {:>4} ({:>5.1}%)", check, count, pct);
                    }
                }
                println!();
            }

            println!("{}", "═".repeat(60));
            println!(
                "OVERALL: {} values, {} valid, {} invalid, {} null",
                total_values, total_valid, total_invalid, total_null
            );
            println!("Strategy: {}", strategy.name());

            if matches!(strategy, ValidateStrategy::Quarantine) {
                let q_count: usize = group_results
                    .iter()
                    .map(|g| g.result.quarantined.len())
                    .sum();
                if q_count > 0 {
                    println!("Quarantine file: {:?} ({} rows)", quarantine_file, q_count);
                }
            } else {
                println!("Cleaned file: {:?}", cleaned_file);
            }
        }
        OutputFormat::Json => {
            let columns: Vec<serde_json::Value> = group_results
                .iter()
                .map(|gr| {
                    let s = &gr.result.stats;
                    let errors: serde_json::Map<String, serde_json::Value> = s
                        .error_patterns
                        .iter()
                        .map(|(k, v)| (k.to_string(), json!(*v)))
                        .collect();
                    json!({
                        "label": gr.label,
                        "total": s.total_count,
                        "valid": s.valid_count,
                        "invalid": s.invalid_count,
                        "null": s.null_count,
                        "validity_rate": s.validity_rate(),
                        "error_patterns": errors,
                    })
                })
                .collect();

            let report = json!({
                "columns": columns,
                "summary": {
                    "total": total_values,
                    "valid": total_valid,
                    "invalid": total_invalid,
                    "null": total_null,
                    "strategy": strategy.name(),
                },
            });
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        OutputFormat::Csv => {
            println!("label,total,valid,invalid,null,validity_rate");
            for gr in &group_results {
                let s = &gr.result.stats;
                println!(
                    "\"{}\",{},{},{},{},{:.4}",
                    gr.label,
                    s.total_count,
                    s.valid_count,
                    s.invalid_count,
                    s.null_count,
                    s.validity_rate()
                );
            }
        }
    }

    // ── Write quarantine file (quarantine strategy) ────────────────────────
    if matches!(strategy, ValidateStrategy::Quarantine) {
        let mut quarantine_rows: Vec<serde_json::Value> = Vec::new();
        for gr in &group_results {
            for q in &gr.result.quarantined {
                let orig_row = gr.original_row_indices[q.row_index];
                let error_msgs: Vec<String> = q.errors.iter().map(|e| e.message.clone()).collect();
                quarantine_rows.push(json!({
                    "row": orig_row,
                    "value": q.value,
                    "label": gr.label,
                    "errors": error_msgs,
                }));
            }
        }
        if !quarantine_rows.is_empty() {
            quarantine_rows.sort_by_key(|r| r["row"].as_u64().unwrap_or(0));
            let mut qfile = std::fs::File::create(&quarantine_file)?;
            for row in &quarantine_rows {
                writeln!(qfile, "{}", row)?;
            }
            eprintln!(
                "Wrote {} quarantined rows to {:?}",
                quarantine_rows.len(),
                quarantine_file
            );
        }
    }

    // ── Write cleaned file (null/ffill/bfill strategies) ───────────────────
    if !matches!(strategy, ValidateStrategy::Quarantine) {
        // Reassemble cleaned data in original row order
        let mut cleaned_rows: Vec<(usize, Option<String>, String)> = Vec::new();
        for gr in &group_results {
            for (i, cleaned_val) in gr.result.values.iter().enumerate() {
                let orig_row = gr.original_row_indices[i];
                cleaned_rows.push((orig_row, cleaned_val.clone(), gr.label.clone()));
            }
        }
        cleaned_rows.sort_by_key(|(row, _, _)| *row);

        let mut cfile = std::fs::File::create(&cleaned_file)?;
        if is_plain_text {
            // Plain text output: one value per line
            for (_, value, _) in &cleaned_rows {
                match value {
                    Some(v) => writeln!(cfile, "{}", v)?,
                    None => writeln!(cfile, "NULL")?,
                }
            }
        } else {
            // NDJSON output
            for (_, value, lbl) in &cleaned_rows {
                match value {
                    Some(v) => writeln!(cfile, "{}", json!({"value": v, "label": lbl}))?,
                    None => {
                        writeln!(
                            cfile,
                            "{}",
                            json!({"value": serde_json::Value::Null, "label": lbl})
                        )?;
                    }
                }
            }
        }
        eprintln!(
            "Wrote {} cleaned rows to {:?}",
            cleaned_rows.len(),
            cleaned_file
        );
    }

    // Exit code: 0 = all valid, 1 = some invalid
    if any_invalid {
        std::process::exit(1);
    }

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// PROFILE — Detect column types in a CSV file
// ═══════════════════════════════════════════════════════════════════════════════

#[allow(clippy::too_many_arguments)]
fn cmd_profile(
    file: PathBuf,
    model: PathBuf,
    output: OutputFormat,
    sample_size: usize,
    delimiter: Option<char>,
    no_header_hint: bool,
    model_type: ModelType,
    sharp_only: bool,
) -> Result<()> {
    use finetype_model::{ColumnClassifier, ColumnConfig, ValueClassifier};

    eprintln!("Loading model from {:?}", model);
    let classifier: Box<dyn ValueClassifier> = match model_type {
        ModelType::CharCnn => Box::new(load_char_classifier(&model)?),
        ModelType::Tiered => Box::new(load_tiered_classifier(&model)?),
        ModelType::Transformer => Box::new(finetype_model::Classifier::load(&model)?),
    };

    let config = ColumnConfig {
        sample_size,
        ..Default::default()
    };
    let mut column_classifier = if let Some(semantic) = load_semantic_hint() {
        eprintln!("Loaded semantic hint classifier (Model2Vec)");
        // Load entity classifier (shares Model2Vec tokenizer/embeddings)
        let entity = load_entity_classifier(&semantic);
        let mut cc = ColumnClassifier::with_semantic_hint(classifier, config, semantic);
        if let Some(entity) = entity {
            eprintln!("Loaded entity classifier (full_name demotion gate)");
            cc.set_entity_classifier(entity);
        }
        cc
    } else {
        ColumnClassifier::new(classifier, config)
    };

    // Load taxonomy for validation-based attractor demotion (Rule 14)
    // Pre-compile validators for the hot path (NNFT-116)
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

    // Wire up Sense classifier (Sense → Sharpen pipeline)
    if !sharp_only {
        wire_sense(&mut column_classifier);
    }

    eprintln!("Reading {:?}", file);

    // Build CSV reader with optional delimiter
    let mut reader_builder = csv::ReaderBuilder::new();
    reader_builder.flexible(true);
    if let Some(delim) = delimiter {
        reader_builder.delimiter(delim as u8);
    }
    let mut reader = reader_builder.from_path(&file)?;

    // Get headers
    let headers: Vec<String> = reader.headers()?.iter().map(|h| h.to_string()).collect();

    let n_cols = headers.len();
    eprintln!("Found {} columns: {:?}", n_cols, headers);

    // Collect column values
    let mut columns: Vec<Vec<String>> = vec![Vec::new(); n_cols];
    let mut row_count = 0;

    for result in reader.records() {
        let record = result?;
        row_count += 1;
        for (i, field) in record.iter().enumerate() {
            if i < n_cols {
                let trimmed = field.trim();
                // Skip empty, NULL, NA, N/A values
                if !trimmed.is_empty()
                    && trimmed != "NULL"
                    && trimmed != "null"
                    && trimmed != "NA"
                    && trimmed != "N/A"
                    && trimmed != "nan"
                    && trimmed != "NaN"
                    && trimmed != "None"
                {
                    columns[i].push(trimmed.to_string());
                }
            }
        }
    }

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
    }

    let mut profiles: Vec<ColProfile> = Vec::new();

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
                confidence: 0.0,
                samples_used: 0,
                non_null_count: 0,
                null_count,
                disambiguation_applied: false,
                disambiguation_rule: None,
                detected_locale: None,
            });
            continue;
        }

        let result = if no_header_hint {
            column_classifier.classify_column(col_values)?
        } else {
            column_classifier.classify_column_with_header(col_values, &name)?
        };

        profiles.push(ColProfile {
            name,
            label: result.label,
            confidence: result.confidence,
            samples_used: result.samples_used,
            non_null_count: col_values.len(),
            null_count,
            disambiguation_applied: result.disambiguation_applied,
            disambiguation_rule: result.disambiguation_rule,
            detected_locale: result.detected_locale,
        });
    }

    // Output results
    match output {
        OutputFormat::Plain => {
            println!(
                "FineType Column Profile — {:?} ({} rows, {} columns)",
                file, row_count, n_cols
            );
            println!("{}", "═".repeat(80));
            println!();
            println!("  {:<25} {:<45} {:>6}", "COLUMN", "TYPE", "CONF");
            println!("  {}", "─".repeat(78));

            for p in &profiles {
                let conf_str = if p.non_null_count > 0 {
                    format!("{:.1}%", p.confidence * 100.0)
                } else {
                    "—".to_string()
                };
                let disambig = if p.disambiguation_applied {
                    format!(" [{}]", p.disambiguation_rule.as_deref().unwrap_or("rule"))
                } else {
                    String::new()
                };
                println!(
                    "  {:<25} {:<45} {:>6}{}",
                    p.name, p.label, conf_str, disambig
                );
            }

            println!();
            let typed_cols = profiles.iter().filter(|p| p.label != "unknown").count();
            println!(
                "{}/{} columns typed, {} rows analyzed",
                typed_cols, n_cols, row_count
            );
        }
        OutputFormat::Json => {
            let cols: Vec<serde_json::Value> = profiles
                .iter()
                .map(|p| {
                    let mut obj = serde_json::Map::new();
                    obj.insert("column".to_string(), json!(p.name));
                    obj.insert("type".to_string(), json!(p.label));
                    obj.insert("confidence".to_string(), json!(p.confidence));
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
                    serde_json::Value::Object(obj)
                })
                .collect();

            let result = json!({
                "file": file.to_string_lossy(),
                "rows": row_count,
                "columns": cols,
            });
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        OutputFormat::Csv => {
            println!("column,type,confidence,samples_used,non_null,null,disambiguation,locale");
            for p in &profiles {
                println!(
                    "\"{}\",\"{}\",{:.4},{},{},{},\"{}\",\"{}\"",
                    p.name,
                    p.label,
                    p.confidence,
                    p.samples_used,
                    p.non_null_count,
                    p.null_count,
                    p.disambiguation_rule.as_deref().unwrap_or(""),
                    p.detected_locale.as_deref().unwrap_or("")
                );
            }
        }
    }

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// EVAL-GITTABLES — Column-mode evaluation on GitTables benchmark
// ═══════════════════════════════════════════════════════════════════════════════

fn cmd_eval_gittables(
    dir: PathBuf,
    model: PathBuf,
    sample_size: usize,
    output: OutputFormat,
) -> Result<()> {
    use finetype_model::{CharClassifier, ColumnClassifier, ColumnConfig};
    use std::collections::HashMap;
    use std::time::Instant;

    let start = Instant::now();

    // ── 1. Load model once ──────────────────────────────────────────────────
    eprintln!("Loading model from {:?}", model);
    let classifier = CharClassifier::load(&model)?;
    let config = ColumnConfig {
        sample_size,
        ..Default::default()
    };
    let column_classifier = ColumnClassifier::new(Box::new(classifier), config);

    // ── 2. Load ground truth ────────────────────────────────────────────────
    eprintln!("Loading ground truth from {:?}", dir);

    #[derive(Debug)]
    #[allow(dead_code)]
    struct Annotation {
        gt_label: String,
        ontology: String,
    }

    // Read ground truth CSVs: (table_file, col_idx) -> Annotation
    let mut ground_truth: HashMap<(String, usize), Annotation> = HashMap::new();

    // Helper to load a GT csv file
    fn load_gt(
        path: &std::path::Path,
        ontology: &str,
        suffix: &str,
        gt: &mut HashMap<(String, usize), Annotation>,
    ) -> Result<usize> {
        let mut count = 0;
        let mut reader = csv::ReaderBuilder::new().from_path(path)?;
        for result in reader.records() {
            let record = result?;
            // Columns: (row_number), table_id, target_column, annotation_id, annotation_label
            let table_id = record.get(1).unwrap_or("");
            let col_idx: usize = record.get(2).unwrap_or("0").parse().unwrap_or(0);
            let gt_label = record.get(4).unwrap_or("").to_string();

            // Strip suffix from table_id: GitTables_1501_schema -> GitTables_1501
            let table_file = table_id.replace(suffix, "");

            gt.entry((table_file, col_idx)).or_insert(Annotation {
                gt_label,
                ontology: ontology.to_string(),
            });
            count += 1;
        }
        Ok(count)
    }

    // Load schema.org first (preferred), then dbpedia (fills gaps)
    let schema_path = dir.join("schema_gt.csv");
    let dbpedia_path = dir.join("dbpedia_gt.csv");

    if schema_path.exists() {
        let n = load_gt(&schema_path, "schema.org", "_schema", &mut ground_truth)?;
        eprintln!("  Schema.org: {} annotations", n);
    }
    if dbpedia_path.exists() {
        let n = load_gt(&dbpedia_path, "dbpedia", "_dbpedia", &mut ground_truth)?;
        eprintln!("  DBpedia: {} annotations (after merge)", n);
    }
    eprintln!("  Total unique: {} annotated columns", ground_truth.len());

    // ── 3. Group annotations by table ───────────────────────────────────────
    let mut tables_to_cols: HashMap<String, Vec<(usize, String)>> = HashMap::new();
    for ((table_file, col_idx), ann) in &ground_truth {
        tables_to_cols
            .entry(table_file.clone())
            .or_default()
            .push((*col_idx, ann.gt_label.clone()));
    }
    eprintln!("  {} unique tables with annotations", tables_to_cols.len());

    // ── 4. Domain mapping (same as eval.sql) ────────────────────────────────
    let domain_map: HashMap<&str, &str> = [
        ("email", "identity"),
        ("url", "technology"),
        ("date", "datetime"),
        ("start date", "datetime"),
        ("end date", "datetime"),
        ("start time", "datetime"),
        ("end time", "datetime"),
        ("time", "datetime"),
        ("created", "datetime"),
        ("updated", "datetime"),
        ("year", "datetime"),
        ("postal code", "geography"),
        ("zip code", "geography"),
        ("country", "geography"),
        ("state", "geography"),
        ("city", "geography"),
        ("id", "identity"),
        ("name", "identity"),
        ("percentage", "numeric"),
        ("age", "numeric"),
        ("price", "numeric"),
        ("weight", "numeric"),
        ("height", "numeric"),
        ("depth", "numeric"),
        ("width", "numeric"),
        ("length", "numeric"),
        ("duration", "numeric"),
        ("gender", "identity"),
        ("author", "identity"),
        ("description", "representation"),
        ("title", "representation"),
        ("abstract", "representation"),
        ("comment", "representation"),
        ("status", "representation"),
        ("category", "representation"),
        ("type", "representation"),
    ]
    .iter()
    .copied()
    .collect();

    // ── 5. Process each table ───────────────────────────────────────────────
    eprintln!("\nProcessing tables...");

    #[allow(dead_code)]
    struct ColumnPrediction {
        table_file: String,
        col_idx: usize,
        gt_label: String,
        row_mode_label: String,
        column_mode_label: String,
        disambiguation_applied: bool,
        disambiguation_rule: Option<String>,
        n_values: usize,
    }

    let mut predictions: Vec<ColumnPrediction> = Vec::new();
    let mut tables_processed = 0;
    let mut tables_missing = 0;

    let tables_dir = dir.join("tables/tables");
    let mut table_names: Vec<String> = tables_to_cols.keys().cloned().collect();
    table_names.sort();

    for table_file in &table_names {
        let csv_path = tables_dir.join(format!("{}.csv", table_file));
        if !csv_path.exists() {
            tables_missing += 1;
            continue;
        }

        // Read the CSV
        let mut reader = csv::ReaderBuilder::new()
            .flexible(true)
            .from_path(&csv_path)?;

        let headers: Vec<String> = reader.headers()?.iter().map(|h| h.to_string()).collect();

        // Build header index: "col0" -> 0, "col1" -> 1, etc.
        // The first column is typically "column00" (row index) which we skip
        let mut header_to_pos: HashMap<String, usize> = HashMap::new();
        for (pos, name) in headers.iter().enumerate() {
            header_to_pos.insert(name.clone(), pos);
        }

        // Collect all column values
        let n_cols = headers.len();
        let mut columns: Vec<Vec<String>> = vec![Vec::new(); n_cols];
        for result in reader.records() {
            let record = result?;
            for (i, field) in record.iter().enumerate() {
                if i < n_cols {
                    let trimmed = field.trim();
                    if !trimmed.is_empty()
                        && trimmed != "NULL"
                        && trimmed != "null"
                        && trimmed != "NA"
                        && trimmed != "N/A"
                        && trimmed != "nan"
                        && trimmed != "NaN"
                        && trimmed != "None"
                    {
                        columns[i].push(trimmed.to_string());
                    }
                }
            }
        }

        // Process each annotated column
        let annotated_cols = tables_to_cols.get(table_file).unwrap();
        for (col_idx, gt_label) in annotated_cols {
            let col_name = format!("col{}", col_idx);
            let pos = match header_to_pos.get(&col_name) {
                Some(p) => *p,
                None => continue, // Column doesn't exist in this table
            };

            let col_values = &columns[pos];
            if col_values.is_empty() {
                continue;
            }

            // Row-mode: classify each value independently, take majority vote
            let batch_results = column_classifier.classifier().classify_batch(col_values)?;
            let mut vote_counts: HashMap<String, usize> = HashMap::new();
            for r in &batch_results {
                *vote_counts.entry(r.label.clone()).or_default() += 1;
            }
            let row_mode_label = vote_counts
                .iter()
                .max_by_key(|(_, count)| *count)
                .map(|(label, _)| label.clone())
                .unwrap_or_else(|| "unknown".to_string());

            // Column-mode: use ColumnClassifier with disambiguation rules
            let col_result = column_classifier.classify_column(col_values)?;

            predictions.push(ColumnPrediction {
                table_file: table_file.clone(),
                col_idx: *col_idx,
                gt_label: gt_label.clone(),
                row_mode_label,
                column_mode_label: col_result.label,
                disambiguation_applied: col_result.disambiguation_applied,
                disambiguation_rule: col_result.disambiguation_rule,
                n_values: col_values.len(),
            });
        }

        tables_processed += 1;
        if tables_processed % 100 == 0 {
            eprint!(
                "\r  Processed {}/{} tables...",
                tables_processed,
                table_names.len()
            );
        }
    }
    eprintln!(
        "\r  Processed {} tables ({} missing CSVs)",
        tables_processed, tables_missing
    );

    let elapsed = start.elapsed();
    eprintln!(
        "  {} columns evaluated in {:.1}s\n",
        predictions.len(),
        elapsed.as_secs_f64()
    );

    // ── 6. Compute accuracy metrics ─────────────────────────────────────────

    // Domain-level accuracy for mapped types
    struct DomainAccuracy {
        total: usize,
        row_correct: usize,
        col_correct: usize,
    }

    let mut domain_acc: HashMap<String, DomainAccuracy> = HashMap::new();
    let mut overall_row_correct = 0usize;
    let mut overall_col_correct = 0usize;
    let mut overall_mapped = 0usize;

    // Year-specific tracking
    let mut year_total = 0usize;
    let mut year_row_correct = 0usize;
    let mut year_col_correct = 0usize;
    let mut year_row_predictions: HashMap<String, usize> = HashMap::new();
    let mut year_col_predictions: HashMap<String, usize> = HashMap::new();

    // Disambiguation tracking
    let mut disambig_count = 0usize;
    let mut disambig_rules: HashMap<String, usize> = HashMap::new();

    for pred in &predictions {
        // Check if disambiguation was applied
        if pred.disambiguation_applied {
            disambig_count += 1;
            if let Some(rule) = &pred.disambiguation_rule {
                *disambig_rules.entry(rule.clone()).or_default() += 1;
            }
        }

        // Year-specific tracking
        if pred.gt_label == "year" {
            year_total += 1;
            let row_domain = pred.row_mode_label.split('.').next().unwrap_or("");
            let col_domain = pred.column_mode_label.split('.').next().unwrap_or("");
            if row_domain == "datetime" {
                year_row_correct += 1;
            }
            if col_domain == "datetime" {
                year_col_correct += 1;
            }
            *year_row_predictions
                .entry(pred.row_mode_label.clone())
                .or_default() += 1;
            *year_col_predictions
                .entry(pred.column_mode_label.clone())
                .or_default() += 1;
        }

        // Domain accuracy for mapped types
        if let Some(&expected_domain) = domain_map.get(pred.gt_label.as_str()) {
            let row_domain = pred.row_mode_label.split('.').next().unwrap_or("");
            let col_domain = pred.column_mode_label.split('.').next().unwrap_or("");

            let row_match = row_domain == expected_domain
                || (expected_domain == "numeric" && row_domain == "representation");
            let col_match = col_domain == expected_domain
                || (expected_domain == "numeric" && col_domain == "representation");

            let entry = domain_acc
                .entry(expected_domain.to_string())
                .or_insert(DomainAccuracy {
                    total: 0,
                    row_correct: 0,
                    col_correct: 0,
                });
            entry.total += 1;
            if row_match {
                entry.row_correct += 1;
                overall_row_correct += 1;
            }
            if col_match {
                entry.col_correct += 1;
                overall_col_correct += 1;
            }
            overall_mapped += 1;
        }
    }

    // Where column-mode disagrees with row-mode
    let mut improvements: Vec<&ColumnPrediction> = Vec::new();
    let mut regressions: Vec<&ColumnPrediction> = Vec::new();
    for pred in &predictions {
        if pred.row_mode_label != pred.column_mode_label {
            if let Some(&expected_domain) = domain_map.get(pred.gt_label.as_str()) {
                let row_domain = pred.row_mode_label.split('.').next().unwrap_or("");
                let col_domain = pred.column_mode_label.split('.').next().unwrap_or("");
                let row_match = row_domain == expected_domain
                    || (expected_domain == "numeric" && row_domain == "representation");
                let col_match = col_domain == expected_domain
                    || (expected_domain == "numeric" && col_domain == "representation");

                if col_match && !row_match {
                    improvements.push(pred);
                } else if !col_match && row_match {
                    regressions.push(pred);
                }
            }
        }
    }

    // ── 7. Output results ───────────────────────────────────────────────────

    match output {
        OutputFormat::Plain | OutputFormat::Csv => {
            println!("GitTables Column-Mode Evaluation");
            println!("{}", "═".repeat(70));
            println!();
            println!("SCALE");
            println!("  Tables processed:     {}", tables_processed);
            println!("  Columns evaluated:    {}", predictions.len());
            println!("  Columns with mapping: {}", overall_mapped);
            println!("  Evaluation time:      {:.1}s", elapsed.as_secs_f64());
            println!();

            // Domain-level accuracy comparison
            println!("DOMAIN-LEVEL ACCURACY (Row-Mode vs Column-Mode)");
            println!(
                "  {:<18} {:>6} {:>12} {:>12} {:>8}",
                "Domain", "Cols", "Row-Mode", "Col-Mode", "Delta"
            );
            println!("  {}", "─".repeat(60));

            let mut sorted_domains: Vec<(String, &DomainAccuracy)> =
                domain_acc.iter().map(|(k, v)| (k.clone(), v)).collect();
            sorted_domains.sort_by(|a, b| b.1.total.cmp(&a.1.total));

            for (domain, acc) in &sorted_domains {
                let row_pct = acc.row_correct as f64 / acc.total as f64 * 100.0;
                let col_pct = acc.col_correct as f64 / acc.total as f64 * 100.0;
                let delta = col_pct - row_pct;
                let delta_str = if delta > 0.0 {
                    format!("+{:.1}%", delta)
                } else if delta < 0.0 {
                    format!("{:.1}%", delta)
                } else {
                    "  —".to_string()
                };
                println!(
                    "  {:<18} {:>6} {:>10.1}% {:>10.1}% {:>8}",
                    domain, acc.total, row_pct, col_pct, delta_str
                );
            }

            // Overall
            let overall_row_pct = overall_row_correct as f64 / overall_mapped as f64 * 100.0;
            let overall_col_pct = overall_col_correct as f64 / overall_mapped as f64 * 100.0;
            let overall_delta = overall_col_pct - overall_row_pct;
            println!("  {}", "─".repeat(60));
            println!(
                "  {:<18} {:>6} {:>10.1}% {:>10.1}% {:>+7.1}%",
                "OVERALL", overall_mapped, overall_row_pct, overall_col_pct, overall_delta
            );

            // Year-specific report
            if year_total > 0 {
                println!();
                println!("YEAR COLUMN ANALYSIS (NNFT-026 Impact)");
                println!("  Year columns found: {}", year_total);
                println!(
                    "  Row-mode accuracy:    {:.1}% ({}/{})",
                    year_row_correct as f64 / year_total as f64 * 100.0,
                    year_row_correct,
                    year_total
                );
                println!(
                    "  Column-mode accuracy: {:.1}% ({}/{})",
                    year_col_correct as f64 / year_total as f64 * 100.0,
                    year_col_correct,
                    year_total
                );

                println!();
                println!("  Row-mode predictions for 'year' columns:");
                let mut row_sorted: Vec<_> = year_row_predictions.iter().collect();
                row_sorted.sort_by(|a, b| b.1.cmp(a.1));
                for (label, count) in &row_sorted {
                    let pct = **count as f64 / year_total as f64 * 100.0;
                    println!("    {:.1}%  {}", pct, label);
                }

                println!();
                println!("  Column-mode predictions for 'year' columns:");
                let mut col_sorted: Vec<_> = year_col_predictions.iter().collect();
                col_sorted.sort_by(|a, b| b.1.cmp(a.1));
                for (label, count) in &col_sorted {
                    let pct = **count as f64 / year_total as f64 * 100.0;
                    println!("    {:.1}%  {}", pct, label);
                }
            }

            // Disambiguation summary
            if disambig_count > 0 {
                println!();
                println!("DISAMBIGUATION RULES APPLIED");
                println!(
                    "  {} of {} columns had disambiguation applied",
                    disambig_count,
                    predictions.len()
                );
                let mut rule_sorted: Vec<_> = disambig_rules.iter().collect();
                rule_sorted.sort_by(|a, b| b.1.cmp(a.1));
                for (rule, count) in &rule_sorted {
                    println!("    {:>4}x  {}", count, rule);
                }
            }

            // Improvements and regressions
            if !improvements.is_empty() || !regressions.is_empty() {
                println!();
                println!("COLUMN-MODE IMPACT (domain-level changes)");
                println!(
                    "  Improvements: {} columns (row wrong → column correct)",
                    improvements.len()
                );
                println!(
                    "  Regressions:  {} columns (row correct → column wrong)",
                    regressions.len()
                );

                if !improvements.is_empty() {
                    println!();
                    println!("  Top improvements (showing up to 15):");
                    for pred in improvements.iter().take(15) {
                        println!(
                            "    {}/col{} [{}]: {} → {}",
                            pred.table_file,
                            pred.col_idx,
                            pred.gt_label,
                            pred.row_mode_label,
                            pred.column_mode_label
                        );
                    }
                }

                if !regressions.is_empty() {
                    println!();
                    println!("  Regressions (showing up to 15):");
                    for pred in regressions.iter().take(15) {
                        println!(
                            "    {}/col{} [{}]: {} → {}",
                            pred.table_file,
                            pred.col_idx,
                            pred.gt_label,
                            pred.row_mode_label,
                            pred.column_mode_label
                        );
                    }
                }
            }

            println!();
        }
        OutputFormat::Json => {
            let domain_results: Vec<serde_json::Value> = {
                let mut sorted: Vec<(String, &DomainAccuracy)> =
                    domain_acc.iter().map(|(k, v)| (k.clone(), v)).collect();
                sorted.sort_by(|a, b| b.1.total.cmp(&a.1.total));
                sorted
                    .iter()
                    .map(|(domain, acc)| {
                        json!({
                            "domain": domain,
                            "total": acc.total,
                            "row_correct": acc.row_correct,
                            "col_correct": acc.col_correct,
                            "row_accuracy": acc.row_correct as f64 / acc.total as f64,
                            "col_accuracy": acc.col_correct as f64 / acc.total as f64,
                        })
                    })
                    .collect()
            };

            let result = json!({
                "tables_processed": tables_processed,
                "columns_evaluated": predictions.len(),
                "columns_mapped": overall_mapped,
                "elapsed_seconds": elapsed.as_secs_f64(),
                "overall": {
                    "row_accuracy": overall_row_correct as f64 / overall_mapped as f64,
                    "col_accuracy": overall_col_correct as f64 / overall_mapped as f64,
                    "improvements": improvements.len(),
                    "regressions": regressions.len(),
                },
                "year": {
                    "total": year_total,
                    "row_correct": year_row_correct,
                    "col_correct": year_col_correct,
                },
                "disambiguation": {
                    "columns_affected": disambig_count,
                    "rules": disambig_rules,
                },
                "domains": domain_results,
            });
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
    }

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// EVAL — Evaluate model accuracy on a test set
// ═══════════════════════════════════════════════════════════════════════════════

fn cmd_eval(
    data: PathBuf,
    model: PathBuf,
    _taxonomy_path: PathBuf,
    model_type: ModelType,
    top_confusions: usize,
    output: OutputFormat,
) -> Result<()> {
    use finetype_model::{CharClassifier, ClassificationResult};
    use std::collections::HashMap;

    eprintln!("Loading test data from {:?}", data);
    let file = std::fs::File::open(&data)?;
    let reader = std::io::BufReader::new(file);

    let mut test_samples: Vec<(String, String)> = Vec::new();
    for line in reader.lines() {
        let line = line?;
        if line.is_empty() {
            continue;
        }
        let record: serde_json::Value = serde_json::from_str(&line)?;
        let text = record["text"].as_str().unwrap_or("").to_string();
        let label = record["classification"].as_str().unwrap_or("").to_string();
        test_samples.push((text, label));
    }
    eprintln!("Loaded {} test samples", test_samples.len());

    // Run inference
    eprintln!("Loading model from {:?}", model);
    let mut predictions: Vec<ClassificationResult> = Vec::new();

    match model_type {
        ModelType::CharCnn => {
            let classifier = CharClassifier::load(&model)?;
            eprintln!("Running inference...");

            // Batch inference for efficiency
            let batch_size = 128;
            let texts: Vec<String> = test_samples.iter().map(|(t, _)| t.clone()).collect();
            for chunk in texts.chunks(batch_size) {
                let batch_results = classifier.classify_batch(chunk)?;
                predictions.extend(batch_results);
            }
        }
        ModelType::Transformer => {
            let classifier = Classifier::load(&model)?;
            eprintln!("Running inference...");

            let batch_size = 32;
            let texts: Vec<String> = test_samples.iter().map(|(t, _)| t.clone()).collect();
            for chunk in texts.chunks(batch_size) {
                let batch_results = classifier.classify_batch(chunk)?;
                predictions.extend(batch_results);
            }
        }
        ModelType::Tiered => {
            let classifier = load_tiered_classifier(&model)?;
            eprintln!("Running tiered inference...");

            let batch_size = 128;
            let texts: Vec<String> = test_samples.iter().map(|(t, _)| t.clone()).collect();
            for chunk in texts.chunks(batch_size) {
                let batch_results = classifier.classify_batch(chunk)?;
                predictions.extend(batch_results);
            }
        }
    }

    eprintln!("Computing metrics...");

    // Compute metrics
    let mut correct = 0usize;
    let mut top3_correct = 0usize;
    let total = test_samples.len();

    // Per-class counts: true_positives, false_positives, false_negatives
    let mut tp: HashMap<String, usize> = HashMap::new();
    let mut fp: HashMap<String, usize> = HashMap::new();
    let mut fn_: HashMap<String, usize> = HashMap::new();

    // Confusion pairs: (actual, predicted) -> count
    let mut confusion: HashMap<(String, String), usize> = HashMap::new();

    // Confidence distribution
    let mut confidence_correct: Vec<f32> = Vec::new();
    let mut confidence_wrong: Vec<f32> = Vec::new();

    for (i, ((_text, actual), pred)) in test_samples.iter().zip(predictions.iter()).enumerate() {
        let predicted = &pred.label;

        if predicted == actual {
            correct += 1;
            confidence_correct.push(pred.confidence);
            *tp.entry(actual.clone()).or_default() += 1;
        } else {
            confidence_wrong.push(pred.confidence);
            *fp.entry(predicted.clone()).or_default() += 1;
            *fn_.entry(actual.clone()).or_default() += 1;
            *confusion
                .entry((actual.clone(), predicted.clone()))
                .or_default() += 1;
        }

        // Top-3 accuracy
        let mut scores = pred.all_scores.clone();
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        let top3_labels: Vec<&str> = scores.iter().take(3).map(|(l, _)| l.as_str()).collect();
        if top3_labels.contains(&actual.as_str()) {
            top3_correct += 1;
        }

        // Progress
        if (i + 1) % 1000 == 0 {
            eprint!("\r  Processed {}/{}...", i + 1, total);
        }
    }
    eprintln!();

    let accuracy = correct as f64 / total as f64;
    let top3_accuracy = top3_correct as f64 / total as f64;

    let avg_confidence_correct = if confidence_correct.is_empty() {
        0.0
    } else {
        confidence_correct.iter().sum::<f32>() / confidence_correct.len() as f32
    };
    let avg_confidence_wrong = if confidence_wrong.is_empty() {
        0.0
    } else {
        confidence_wrong.iter().sum::<f32>() / confidence_wrong.len() as f32
    };

    // Collect all classes
    let mut all_classes: Vec<String> = tp
        .keys()
        .chain(fp.keys())
        .chain(fn_.keys())
        .cloned()
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    all_classes.sort();

    // Sort confusions by count
    let mut confusion_vec: Vec<((String, String), usize)> = confusion.into_iter().collect();
    confusion_vec.sort_by(|a, b| b.1.cmp(&a.1));

    match output {
        OutputFormat::Plain | OutputFormat::Csv => {
            println!("FineType Model Evaluation");
            println!("{}", "=".repeat(60));
            println!();
            println!("OVERALL");
            println!("  Samples:        {}", total);
            println!(
                "  Accuracy:       {:.2}% ({}/{})",
                accuracy * 100.0,
                correct,
                total
            );
            println!(
                "  Top-3 Accuracy: {:.2}% ({}/{})",
                top3_accuracy * 100.0,
                top3_correct,
                total
            );
            println!(
                "  Avg confidence (correct):   {:.4}",
                avg_confidence_correct
            );
            println!("  Avg confidence (incorrect): {:.4}", avg_confidence_wrong);
            println!();

            // Per-class metrics
            println!("PER-CLASS METRICS");
            println!(
                "  {:50} {:>6} {:>6} {:>6} {:>8}",
                "class", "prec", "rec", "f1", "support"
            );
            println!("  {}", "-".repeat(80));

            let mut macro_precision = 0.0f64;
            let mut macro_recall = 0.0f64;
            let mut macro_f1 = 0.0f64;
            let mut n_classes = 0;

            for class in &all_classes {
                let t = *tp.get(class).unwrap_or(&0) as f64;
                let f_p = *fp.get(class).unwrap_or(&0) as f64;
                let f_n = *fn_.get(class).unwrap_or(&0) as f64;

                let precision = if t + f_p > 0.0 { t / (t + f_p) } else { 0.0 };
                let recall = if t + f_n > 0.0 { t / (t + f_n) } else { 0.0 };
                let f1 = if precision + recall > 0.0 {
                    2.0 * precision * recall / (precision + recall)
                } else {
                    0.0
                };
                let support = (t + f_n) as usize;

                if support > 0 {
                    println!(
                        "  {:50} {:>5.1}% {:>5.1}% {:>5.1}% {:>8}",
                        class,
                        precision * 100.0,
                        recall * 100.0,
                        f1 * 100.0,
                        support,
                    );
                    macro_precision += precision;
                    macro_recall += recall;
                    macro_f1 += f1;
                    n_classes += 1;
                }
            }

            if n_classes > 0 {
                println!("  {}", "-".repeat(80));
                println!(
                    "  {:50} {:>5.1}% {:>5.1}% {:>5.1}% {:>8}",
                    "macro avg",
                    (macro_precision / n_classes as f64) * 100.0,
                    (macro_recall / n_classes as f64) * 100.0,
                    (macro_f1 / n_classes as f64) * 100.0,
                    total,
                );
            }

            // Top confusions
            if !confusion_vec.is_empty() {
                println!();
                println!("TOP CONFUSIONS (actual -> predicted)");
                for ((actual, predicted), count) in confusion_vec.iter().take(top_confusions) {
                    println!("  {:>4}x  {} -> {}", count, actual, predicted);
                }
            }
        }
        OutputFormat::Json => {
            let per_class: Vec<serde_json::Value> = all_classes
                .iter()
                .filter_map(|class| {
                    let t = *tp.get(class).unwrap_or(&0) as f64;
                    let f_p = *fp.get(class).unwrap_or(&0) as f64;
                    let f_n = *fn_.get(class).unwrap_or(&0) as f64;
                    let support = (t + f_n) as usize;
                    if support == 0 {
                        return None;
                    }
                    let precision = if t + f_p > 0.0 { t / (t + f_p) } else { 0.0 };
                    let recall = if t + f_n > 0.0 { t / (t + f_n) } else { 0.0 };
                    let f1 = if precision + recall > 0.0 {
                        2.0 * precision * recall / (precision + recall)
                    } else {
                        0.0
                    };
                    Some(json!({
                        "class": class,
                        "precision": precision,
                        "recall": recall,
                        "f1": f1,
                        "support": support,
                    }))
                })
                .collect();

            let top_conf: Vec<serde_json::Value> = confusion_vec
                .iter()
                .take(top_confusions)
                .map(|((actual, predicted), count)| {
                    json!({
                        "actual": actual,
                        "predicted": predicted,
                        "count": count,
                    })
                })
                .collect();

            let result = json!({
                "total_samples": total,
                "accuracy": accuracy,
                "top3_accuracy": top3_accuracy,
                "correct": correct,
                "avg_confidence_correct": avg_confidence_correct,
                "avg_confidence_wrong": avg_confidence_wrong,
                "per_class": per_class,
                "top_confusions": top_conf,
            });
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
    }

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// HELPERS
// ═══════════════════════════════════════════════════════════════════════════════

/// Load taxonomy from a file or directory.
fn load_taxonomy(path: &PathBuf) -> Result<Taxonomy> {
    if path.exists() {
        if path.is_dir() {
            Ok(Taxonomy::from_directory(path)?)
        } else {
            Ok(Taxonomy::from_file(path)?)
        }
    } else {
        // Fall back to embedded taxonomy (release binaries)
        #[cfg(feature = "embed-models")]
        {
            Ok(Taxonomy::from_yamls(embedded::TAXONOMY_YAMLS)?)
        }
        #[cfg(not(feature = "embed-models"))]
        {
            anyhow::bail!(
                "Taxonomy path {:?} not found. Build with `embed-models` feature for standalone use.",
                path
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_snapshot_skips_empty_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let output = tmp.path().join("new_model");
        // Directory doesn't exist yet — no snapshot
        let result = snapshot_model_dir(&output).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_snapshot_skips_dir_without_model_files() {
        let tmp = tempfile::tempdir().unwrap();
        let output = tmp.path().join("empty_model");
        fs::create_dir_all(&output).unwrap();
        fs::write(output.join("readme.txt"), "not a model").unwrap();
        // No model.safetensors or tier_graph.json — no snapshot
        let result = snapshot_model_dir(&output).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_snapshot_flat_model() {
        let tmp = tempfile::tempdir().unwrap();
        let output = tmp.path().join("char-cnn");
        fs::create_dir_all(&output).unwrap();
        fs::write(output.join("model.safetensors"), "fake-weights").unwrap();
        fs::write(output.join("config.yaml"), "n_classes: 10").unwrap();
        fs::write(output.join("labels.json"), "[]").unwrap();

        let snapshot = snapshot_model_dir(&output).unwrap();
        assert!(snapshot.is_some());
        let snapshot_path = snapshot.unwrap();

        // Snapshot should contain the same files
        assert!(snapshot_path.join("model.safetensors").exists());
        assert!(snapshot_path.join("config.yaml").exists());
        assert!(snapshot_path.join("labels.json").exists());
        // Verify content is preserved
        assert_eq!(
            fs::read_to_string(snapshot_path.join("model.safetensors")).unwrap(),
            "fake-weights"
        );
        // Original should still exist
        assert!(output.join("model.safetensors").exists());
        // Snapshot name should contain "snapshot"
        let name = snapshot_path.file_name().unwrap().to_string_lossy();
        assert!(name.contains("snapshot"));
    }

    #[test]
    fn test_snapshot_tiered_model() {
        let tmp = tempfile::tempdir().unwrap();
        let output = tmp.path().join("tiered-v2");
        let tier0 = output.join("tier0");
        fs::create_dir_all(&tier0).unwrap();
        fs::write(tier0.join("model.safetensors"), "tier0-weights").unwrap();
        fs::write(output.join("tier_graph.json"), "{}").unwrap();

        let snapshot = snapshot_model_dir(&output).unwrap();
        assert!(snapshot.is_some());
        let snapshot_path = snapshot.unwrap();

        // Nested structure should be preserved
        assert!(snapshot_path
            .join("tier0")
            .join("model.safetensors")
            .exists());
        assert!(snapshot_path.join("tier_graph.json").exists());
        assert_eq!(
            fs::read_to_string(snapshot_path.join("tier0").join("model.safetensors")).unwrap(),
            "tier0-weights"
        );
    }

    #[test]
    fn test_copy_dir_recursive() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");

        // Create nested structure
        fs::create_dir_all(src.join("sub1").join("sub2")).unwrap();
        fs::write(src.join("a.txt"), "file-a").unwrap();
        fs::write(src.join("sub1").join("b.txt"), "file-b").unwrap();
        fs::write(src.join("sub1").join("sub2").join("c.txt"), "file-c").unwrap();

        copy_dir_recursive(&src, &dst).unwrap();

        assert_eq!(fs::read_to_string(dst.join("a.txt")).unwrap(), "file-a");
        assert_eq!(
            fs::read_to_string(dst.join("sub1").join("b.txt")).unwrap(),
            "file-b"
        );
        assert_eq!(
            fs::read_to_string(dst.join("sub1").join("sub2").join("c.txt")).unwrap(),
            "file-c"
        );
    }

    #[test]
    fn test_training_manifest_write() {
        let tmp = tempfile::tempdir().unwrap();
        let output = tmp.path().join("model");
        fs::create_dir_all(&output).unwrap();

        let manifest = TrainingManifest {
            output: &output,
            data_file: Path::new("training.ndjson"),
            epochs: 5,
            batch_size: 32,
            seed: Some(42),
            model_type: &ModelType::Tiered,
            n_classes: 171,
            n_samples: 17100,
            snapshot_path: Some(Path::new("models/tiered-v2.snapshot.20260228T120000Z")),
        };

        manifest.write().unwrap();

        let manifest_path = output.join("manifest.json");
        assert!(manifest_path.exists());

        let content: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&manifest_path).unwrap()).unwrap();
        assert_eq!(content["epochs"], 5);
        assert_eq!(content["batch_size"], 32);
        assert_eq!(content["seed"], 42);
        assert_eq!(content["model_type"], "tiered");
        assert_eq!(content["n_classes"], 171);
        assert_eq!(content["n_samples"], 17100);
        assert_eq!(content["data_file"], "training.ndjson");
        assert!(content["timestamp"].is_string());
        assert!(content["parent_snapshot"].is_string());
    }

    #[test]
    fn test_training_manifest_no_seed_no_snapshot() {
        let tmp = tempfile::tempdir().unwrap();
        let output = tmp.path().join("model");
        fs::create_dir_all(&output).unwrap();

        let manifest = TrainingManifest {
            output: &output,
            data_file: Path::new("data.ndjson"),
            epochs: 10,
            batch_size: 64,
            seed: None,
            model_type: &ModelType::CharCnn,
            n_classes: 169,
            n_samples: 16900,
            snapshot_path: None,
        };

        manifest.write().unwrap();

        let content: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(output.join("manifest.json")).unwrap())
                .unwrap();
        assert!(content["seed"].is_null());
        assert!(content["parent_snapshot"].is_null());
        assert_eq!(content["model_type"], "charcnn");
    }
}
