//! FineType CLI
//!
//! Command-line interface for precision format detection.

use anyhow::Result;
use clap::{Parser, Subcommand};
use finetype_cli::transform_projection::{
    build_transform_projection, format_column_name, SchemaExtensions,
};
use finetype_core::{format_report, Checker, Generator, Label, Taxonomy};
use finetype_mcp::json_schema;
use finetype_model::Classifier;
use serde_json::json;
use std::io::{self, BufRead, Read, Write};
use std::path::{Path, PathBuf};
use tracing_subscriber::EnvFilter;

// ═══════════════════════════════════════════════════════════════════════════════
// EMBEDDED MODELS (compile-time)
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(feature = "embed-models")]
mod embedded {
    include!(concat!(env!("OUT_DIR"), "/embedded_models.rs"));
}

/// Resolve the model directory from the `FINETYPE_MODEL` env var.
///
/// The CLI no longer exposes a `--model` flag — every subcommand that
/// loads a model reads this env var. The default is `models/default`,
/// which mirrors the runtime default used by the DuckDB extension and
/// MCP server.
fn resolve_model_path() -> PathBuf {
    std::env::var_os("FINETYPE_MODEL")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("models/default"))
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

        /// Output format (plain, json, csv)
        #[arg(short, long, default_value = "plain")]
        output: OutputFormat,

        /// Include confidence score
        #[arg(long)]
        confidence: bool,

        /// Include input value in output
        #[arg(short, long)]
        value: bool,

        /// Model type: multi-branch (default), char-cnn, or tiered (legacy).
        /// Multi-branch uses a single column-level forward pass + Sharpen post-processing.
        #[arg(long, default_value = "multi-branch")]
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
        /// Requires --mode column. Combine with `--explain` to instead
        /// run the diagnostic cascade (input: {"column_name","predicted_type","samples"},
        /// output: {"inferred_correct_type","confidence","mechanism","signals"}).
        #[arg(long)]
        batch: bool,

        /// Diagnostic cascade — given a column's predicted type and samples,
        /// return the inferred correct type plus a mechanism token explaining
        /// the predicted/actual relationship (one of ten closed tokens).
        /// Requires `--mode column --batch`; stdin is NDJSON of
        /// {"column_name","predicted_type","samples":[...]} and stdout is
        /// NDJSON of {"inferred_correct_type","confidence","mechanism",
        /// "signals":{...}}. Loads taxonomy + validators once across the
        /// whole stream.
        #[arg(long)]
        explain: bool,

        /// Taxonomy file or directory (used with `--explain`).
        #[arg(long, default_value = "labels")]
        taxonomy: PathBuf,
    },

    /// Generate synthetic training data
    #[command(hide = true)]
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
        #[arg(long, default_value = "multi-branch")]
        model_type: ModelType,

        /// Random seed for deterministic training reproducibility
        #[arg(long)]
        seed: Option<u64>,

        /// Enable feature-augmented training. Extracts deterministic
        /// features per sample and passes them alongside character encodings.
        #[arg(long)]
        use_features: bool,

        /// Enable hierarchical classification head. Uses tree softmax
        /// (7 domains → 43 categories → 250 leaf types) instead of flat 250-class softmax.
        #[arg(long)]
        hierarchical: bool,
    },

    /// Show taxonomy information (optionally filtered to a single type or glob)
    Taxonomy {
        /// Type key (e.g., "identity.person.email") or glob pattern
        /// ("identity.person.*"). When supplied, --domain / --category /
        /// --priority filters are ignored.
        type_key: Option<String>,

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

        /// Output format (plain, json, csv, json-schema)
        #[arg(short, long, default_value = "plain")]
        output: OutputFormat,

        /// Export all fields (description, validation, samples, etc.)
        #[arg(long)]
        full: bool,
    },

    /// Validate generator ↔ taxonomy alignment
    #[command(hide = true)]
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

    /// Validate a single value against a taxonomy label's CompiledValidator.
    /// Prints `PASS` or `FAIL`. Used by the runtime/eval parity test
    /// (scripts/validation_parity.py) to cross-check the live Rust validator
    /// against the Python eval gate on a fixed fixture.
    #[command(hide = true)]
    ValidateValue {
        /// Taxonomy label (e.g. `datetime.time.iso`)
        #[arg(short, long)]
        label: String,

        /// The value to validate
        value: String,

        /// Taxonomy file or directory
        #[arg(short, long, default_value = "labels")]
        taxonomy: PathBuf,
    },

    /// Validate CSV or Parquet data against a JSON Schema — check-only by default,
    /// or pass --db/--table to materialise valid rows + reject sidecar.
    Validate {
        /// Input CSV or Parquet file
        file: PathBuf,

        /// JSON Schema file to validate against
        schema: PathBuf,

        /// Output DuckDB database file (created if absent). Optional —
        /// when omitted, validation runs in check-only mode (no .db
        /// written). When supplied, --table is also required.
        #[arg(long, requires = "table")]
        db: Option<PathBuf>,

        /// Table name to create in the output database for valid rows.
        /// Optional — required only when --db is supplied.
        #[arg(long, requires = "db")]
        table: Option<String>,

        /// Append to an existing database. Required when --db already
        /// contains the named table or a prior finetype_reject_errors
        /// sidecar. Requires --db.
        #[arg(long, requires = "db")]
        append: bool,

        /// Force exit code 0 regardless of reject count (does not
        /// affect error exit code 2).
        #[arg(long)]
        lenient: bool,

        /// Output format for summary report (plain, json)
        #[arg(short, long, default_value = "plain")]
        output: OutputFormat,
    },

    /// Profile a CSV file — detect column types using column-mode inference
    Profile {
        /// Input CSV file (single-file mode). Mutually exclusive with --files.
        #[arg(short, long, conflicts_with = "files")]
        file: Option<PathBuf>,

        /// File containing input paths (one per line) for batch mode. The
        /// model + taxonomy load once, then each listed file is profiled in
        /// turn. Requires `--out-dir`.
        #[arg(long, conflicts_with = "file", requires = "out_dir")]
        files: Option<PathBuf>,

        /// Output directory for batch mode. One output per input is written
        /// as `<out_dir>/<stem>.<ext>` where ext is .json for json /
        /// json-schema, .csv for csv, etc. Only meaningful with `--files`.
        #[arg(long, conflicts_with = "file")]
        out_dir: Option<PathBuf>,

        /// Output format (plain, json, csv, markdown, arrow, json-schema)
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
        #[arg(long, default_value = "multi-branch")]
        model_type: ModelType,

        /// Cardinality threshold for ENUM columns (0 = disable ENUM, show VARCHAR).
        /// A column with at most this many distinct values is typed as an ENUM;
        /// above it, VARCHAR. Default 32 — tuned to reduce over-eager ENUM
        /// attribution in the profile→validate round-trip.
        #[arg(long, default_value = "32")]
        enum_threshold: usize,

        /// Attach observed-data constraints to JSON Schema output
        /// (minLength/maxLength, minimum/maximum, enum, x-finetype-null-rate,
        /// x-finetype-cardinality). Requires `-o json-schema`.
        #[arg(long)]
        stats: bool,

        /// Show additional detail and enable pipeline tracing (Sense, mask, hint, feature rule decisions)
        #[arg(short, long)]
        verbose: bool,

        /// Skip all Sharpen post-processing — return raw multi-branch model output.
        /// Diagnostic flag for ablation studies. Not part of the stable CLI contract.
        #[arg(long, hide = true)]
        raw_model: bool,

        /// Disable validation-as-veto. By default profile checks each
        /// column's sample values against the predicted type's validation
        /// and NULLs the prediction (→ "unknown") when fewer than half pass,
        /// scoped to audited-safe types (labels/veto_safe.txt). Types the
        /// false-veto sweep could not measure get an advisory flag, never a
        /// hard veto. This flag turns the whole mechanism off.
        #[arg(long)]
        no_validation_veto: bool,
    },

    /// Start MCP server for AI agent integration (stdio transport)
    Mcp,

    /// Train a multi-branch Sherlock-style model from FTMB feature data
    #[cfg(feature = "train")]
    #[command(name = "train-multi-branch", hide = true)]
    TrainMultiBranch {
        /// FTMB binary training data file
        #[arg(short, long)]
        data: PathBuf,

        /// Output directory for model artifacts
        #[arg(short, long, default_value = "models/multi-branch-v1")]
        output: PathBuf,

        /// Number of training epochs
        #[arg(short, long, default_value = "10")]
        epochs: usize,

        /// Batch size
        #[arg(long, default_value = "32")]
        batch_size: usize,

        /// Learning rate (AdamW)
        #[arg(long, default_value = "0.0001")]
        lr: f64,

        /// L2 regularization weight (AdamW weight_decay)
        #[arg(long, default_value = "0.0001")]
        weight_decay: f64,

        /// Dropout probability
        #[arg(long, default_value = "0.35")]
        dropout: f32,

        /// Random seed
        #[arg(long, default_value = "42")]
        seed: u64,

        /// Classification head type: flat or hierarchical
        #[arg(long, default_value = "flat")]
        head: String,

        /// Early stopping patience (epochs without improvement)
        #[arg(long, default_value = "10")]
        patience: usize,

        /// Logit-adjustment temperature τ for the train-time loss (choice 0097).
        /// 0 = off (default). When > 0, rare classes are up-weighted via a
        /// train-time logit prior (logit-adjusted loss, Menon et al. ICLR 2021)
        /// instead of by adding manufactured volume. Inference uses raw logits —
        /// zero inference cost. Flat head only. Typical values 0.5–1.0.
        #[arg(long, default_value = "0.0")]
        logit_adjust_tau: f64,

        /// Taxonomy directory (needed for label list)
        #[arg(long, default_value = "labels")]
        taxonomy: PathBuf,

        /// Validation split fraction (0.0-1.0)
        #[arg(long, default_value = "0.15")]
        val_split: f32,

        /// Disable TUI dashboard (log to stderr instead)
        #[arg(long)]
        no_tui: bool,

        /// Path to model config JSON (optional; uses built-in defaults if omitted)
        #[arg(long)]
        model_config: Option<PathBuf>,
    },

    /// Autonomous type-inference triangulator (bead finetype-7zi).
    ///
    /// Extract multi-branch feature vectors from a column of values (stdin)
    #[command(name = "extract-features", hide = true)]
    ExtractFeatures {
        /// Column header name (used for embedding context)
        #[arg(long)]
        header: Option<String>,

        /// Read input as a JSON array instead of one value per line
        #[arg(long)]
        json: bool,

        /// Include validation pass-rate features (239-dim, one per taxonomy type).
        /// Requires taxonomy to be available (labels/ directory or embedded).
        #[arg(long)]
        validation: bool,
    },

    /// Dump late-fusion training features over a labelled distilled corpus.
    ///
    /// Per labelled column, emits View1 (value-CharCNN 240-softmax aggregated
    /// over sampled values: mean ⊕ max ⊕ argmax-vote ⊕ 8 confidence scalars =
    /// 728) ⊕ View2 (multi-branch raw logits = 240) = 968 features. Output is a
    /// raw little-endian f32 blob `<out>.f32` (row-major [N,968]), a label tsv
    /// `<out>.labels.tsv` (`row_idx\tlabel_index\tlabel_name`), and a meta json
    /// `<out>.meta.json` (canonical 240-label order + dims + counts). This is
    /// the one expensive Rust pass; the tiny fusion head trains in Rust/Candle
    /// (train-fusion-head) over this blob — the repo is zero-Python-dep.
    #[command(name = "dump-fusion-features", hide = true)]
    DumpFusionFeatures {
        /// Labelled distilled CSV(.gz): columns `final_label,sample_values[,column_name]`.
        #[arg(short, long)]
        input: PathBuf,

        /// Value-level CharCNN model dir (feature_dim=0). Supplies View1.
        #[arg(long)]
        value_model: PathBuf,

        /// Multi-branch model dir (e.g. v19). Supplies View2 logits + canonical labels.
        #[arg(long)]
        mb_model: PathBuf,

        /// Output path stem; writes <stem>.f32 / <stem>.labels.tsv / <stem>.meta.json.
        #[arg(short, long)]
        output: PathBuf,

        /// Max values sampled per column for View1 aggregation.
        #[arg(long, default_value = "32")]
        sample_n: usize,

        /// Optional row cap (debug). 0 = no cap.
        #[arg(long, default_value = "0")]
        limit: usize,

        /// Optional CSV column carrying a per-row key (e.g. file_path\x01column_name).
        /// When set, writes <stem>.keys.tsv (`row_idx\tkey`) aligned to emitted rows and
        /// keeps rows whose label is unknown (prediction/keyed mode, not training).
        #[arg(long)]
        key_col: Option<String>,
    },

    /// Evaluate model accuracy on a test set
    #[command(hide = true)]
    Eval {
        /// Test data file (NDJSON with "text" and "classification" fields)
        #[arg(short, long)]
        data: PathBuf,

        /// Taxonomy file or directory
        #[arg(short, long, default_value = "labels")]
        taxonomy: PathBuf,

        /// Model type (transformer, char_cnn)
        #[arg(long, default_value = "multi-branch")]
        model_type: ModelType,

        /// Number of top confusions to show
        #[arg(long, default_value = "20")]
        top_confusions: usize,

        /// Output format (plain, json)
        #[arg(short, long, default_value = "plain")]
        output: OutputFormat,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
enum OutputFormat {
    Plain,
    Json,
    Csv,
    Markdown,
    Arrow,
    /// Table-level JSON Schema. Replaces the table-mode of the legacy
    /// `finetype schema <file.csv>` invocation. With `--stats`, attaches
    /// observed-data constraints (minLength/maxLength, minimum/maximum,
    /// enum) and the `x-finetype-null-rate` / `x-finetype-cardinality`
    /// extensions.
    JsonSchema,
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
enum ModelType {
    Transformer,
    CharCnn,
    Tiered,
    MultiBranch,
    /// B3 late-fusion Sense classifier (value-CharCNN + multi-branch → residual
    /// head). Loaded from a `fusion_manifest.json` directory. Profile-only.
    LateFusion,
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
enum InferenceMode {
    /// Classify each value independently (default)
    Row,
    /// Treat all inputs as one column, use distribution to disambiguate
    Column,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize tracing: RUST_LOG takes precedence, then --verbose enables
    // debug-level tracing for the inference pipeline, otherwise use defaults.
    let verbose_tracing = match &cli.command {
        Commands::Profile { verbose, .. } => *verbose,
        _ => false,
    };
    if std::env::var("RUST_LOG").is_ok() {
        tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from_default_env())
            .init();
    } else if verbose_tracing {
        tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::new("finetype_model=debug"))
            .with_target(false)
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from_default_env())
            .init();
    }

    match cli.command {
        Commands::Infer {
            input,
            file,
            output,
            confidence,
            value,
            model_type,
            mode,
            sample_size,
            bench,
            header,
            batch,
            explain,
            taxonomy,
        } => cmd_infer(
            input,
            file,
            output,
            confidence,
            value,
            model_type,
            mode,
            sample_size,
            bench,
            header,
            batch,
            explain,
            taxonomy,
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
            use_features,
            hierarchical,
        } => cmd_train(
            data,
            taxonomy,
            output,
            epochs,
            batch_size,
            device,
            model_type,
            seed,
            use_features,
            hierarchical,
        ),

        Commands::Taxonomy {
            type_key,
            file,
            domain,
            category,
            priority,
            output,
            full,
        } => cmd_taxonomy(type_key, file, domain, category, priority, output, full),

        Commands::Check {
            taxonomy,
            samples,
            seed,
            priority,
            verbose,
            output,
        } => cmd_check(taxonomy, samples, seed, priority, verbose, output),

        Commands::ValidateValue {
            label,
            value,
            taxonomy,
        } => cmd_validate_value(label, value, taxonomy),

        Commands::Validate {
            file,
            schema,
            db,
            table,
            append,
            lenient,
            output,
        } => cmd_validate_table(file, schema, db, table, append, lenient, output),

        Commands::Profile {
            file,
            files,
            out_dir,
            output,
            sample_size,
            delimiter,
            no_header_hint,
            model_type,
            enum_threshold,
            stats,
            verbose,
            raw_model,
            no_validation_veto,
        } => {
            // ac-04: --stats is gated to -o json-schema. Refuse early with a
            // clap-style error rather than silently dropping the flag.
            if stats && !matches!(output, OutputFormat::JsonSchema) {
                let mut cmd = <Cli as clap::CommandFactory>::command();
                let err = cmd.error(
                    clap::error::ErrorKind::ArgumentConflict,
                    "--stats requires -o json-schema",
                );
                err.exit();
            }
            // One of --file or --files must be supplied. clap enforces
            // mutual exclusion; this catches "neither was given".
            if file.is_none() && files.is_none() {
                let mut cmd = <Cli as clap::CommandFactory>::command();
                let err = cmd.error(
                    clap::error::ErrorKind::MissingRequiredArgument,
                    "one of --file or --files is required",
                );
                err.exit();
            }
            cmd_profile(
                file,
                files,
                out_dir,
                output,
                sample_size,
                delimiter,
                no_header_hint,
                model_type,
                enum_threshold,
                stats,
                verbose,
                raw_model,
                no_validation_veto,
            )
        }

        Commands::Eval {
            data,
            taxonomy,
            model_type,
            top_confusions,
            output,
        } => cmd_eval(data, taxonomy, model_type, top_confusions, output),

        Commands::Mcp => cmd_mcp(),

        #[cfg(feature = "train")]
        Commands::TrainMultiBranch {
            data,
            output,
            epochs,
            batch_size,
            lr,
            weight_decay,
            dropout,
            seed,
            head,
            patience,
            logit_adjust_tau,
            taxonomy,
            val_split,
            no_tui,
            model_config,
        } => cmd_train_multi_branch(
            data,
            output,
            epochs,
            batch_size,
            lr,
            weight_decay,
            dropout,
            seed,
            head,
            patience,
            logit_adjust_tau,
            taxonomy,
            val_split,
            no_tui,
            model_config,
        ),

        Commands::ExtractFeatures {
            header,
            json,
            validation,
        } => cmd_extract_features(header, json, validation),
        Commands::DumpFusionFeatures {
            input,
            value_model,
            mb_model,
            output,
            sample_n,
            limit,
            key_col,
        } => cmd_dump_fusion_features(
            &input,
            &value_model,
            &mb_model,
            &output,
            sample_n,
            limit,
            key_col.as_deref(),
        ),
    }
}

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
fn cmd_infer_explain_batch(taxonomy_path: &std::path::Path) -> Result<()> {
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

fn cmd_mcp() -> Result<()> {
    use finetype_model::{ColumnClassifier, ColumnConfig};

    eprintln!("Starting FineType MCP server...");

    let config = ColumnConfig {
        sample_size: 100,
        ..Default::default()
    };

    // Build column classifier — prefer multi-branch, fall back to CharCNN
    let model_path = PathBuf::from("models/default");
    let mut column_classifier = if let Ok(mb) = load_multi_branch_classifier(&model_path) {
        eprintln!(
            "Loaded multi-branch classifier ({} classes)",
            mb.n_classes()
        );
        let mut cc = ColumnClassifier::with_multi_branch(mb, config);
        wire_model2vec_and_siblings(&mut cc);
        cc
    } else {
        eprintln!("No multi-branch model found, falling back to CharCNN");
        let char_classifier = load_char_classifier(&model_path)?;
        if let Some(semantic) = load_semantic_hint() {
            eprintln!("Loaded semantic hint classifier (Model2Vec)");
            let entity = load_entity_classifier(&semantic);
            let mut cc = ColumnClassifier::with_semantic_hint(
                Box::new(char_classifier) as Box<dyn finetype_model::ValueClassifier>,
                config,
                semantic,
            );
            if let Some(entity) = entity {
                eprintln!("Loaded entity classifier (full_name demotion gate)");
                cc.set_entity_classifier(entity);
            }
            wire_sense(&mut cc);
            wire_sibling_context(&mut cc);
            cc
        } else {
            let mut cc = ColumnClassifier::new(
                Box::new(char_classifier) as Box<dyn finetype_model::ValueClassifier>,
                config,
            );
            wire_sense(&mut cc);
            wire_sibling_context(&mut cc);
            cc
        }
    };

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

/// Train a multi-branch Sherlock-style model from FTMB feature-vector data.
#[cfg(feature = "train")]
#[allow(clippy::too_many_arguments)]
fn cmd_train_multi_branch(
    data: PathBuf,
    output: PathBuf,
    epochs: usize,
    batch_size: usize,
    lr: f64,
    weight_decay: f64,
    dropout: f32,
    seed: u64,
    head: String,
    patience: usize,
    logit_adjust_tau: f64,
    taxonomy: PathBuf,
    val_split: f32,
    no_tui: bool,
    model_config: Option<PathBuf>,
) -> Result<()> {
    use finetype_train::multi_branch::{
        read_training_data, train_multi_branch, HeadType, MultiBranchConfig, MultiBranchDataset,
        MultiBranchTrainConfig,
    };
    use finetype_train::tui::{LogRenderer, TrainingRenderer};
    use rand::rngs::StdRng;
    use rand::seq::SliceRandom;
    use rand::SeedableRng;

    let head_type = match head.as_str() {
        "flat" => HeadType::Flat,
        "hierarchical" => HeadType::Hierarchical,
        _ => anyhow::bail!(
            "Unknown head type '{}'. Use 'flat' or 'hierarchical'.",
            head
        ),
    };

    // Load taxonomy to get sorted labels
    let taxonomy = Taxonomy::from_directory(&taxonomy)?;
    let labels_list: Vec<String> = taxonomy.labels().to_vec();
    let label_to_idx: std::collections::HashMap<String, u32> = taxonomy
        .label_to_index()
        .into_iter()
        .map(|(k, v)| (k, v as u32))
        .collect();
    let n_classes = taxonomy.len();

    eprintln!("Loading training data from {}...", data.display());
    let (header, records, table_groups) = read_training_data(&data)?;
    eprintln!(
        "Loaded {} records ({} char, {} embed, {} stats dims, {} table groups)",
        records.len(),
        header.char_dim,
        header.embed_dim,
        header.stats_dim,
        table_groups.len(),
    );

    // Filter records to only include labels present in taxonomy.
    // Build old→new index mapping for remapping table group indices.
    let mut valid_records = Vec::new();
    let mut old_to_new: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
    for (old_idx, record) in records.into_iter().enumerate() {
        if label_to_idx.contains_key(&record.label) {
            let new_idx = valid_records.len();
            old_to_new.insert(old_idx, new_idx);
            valid_records.push(record);
        }
    }

    // Remap table group indices, dropping records that were filtered out
    let remapped_groups: Vec<_> = table_groups
        .into_iter()
        .filter_map(|g| {
            let new_indices: Vec<usize> = g
                .record_indices
                .iter()
                .filter_map(|old| old_to_new.get(old).copied())
                .collect();
            if new_indices.is_empty() {
                None
            } else {
                Some(finetype_train::multi_branch::TableGroup {
                    record_indices: new_indices,
                    sibling_headers: g.sibling_headers,
                })
            }
        })
        .collect();

    eprintln!(
        "{} records match taxonomy ({} classes, {} groups retained)",
        valid_records.len(),
        n_classes,
        remapped_groups.len(),
    );

    // Split into train/val
    let mut indices: Vec<usize> = (0..valid_records.len()).collect();
    let mut rng = StdRng::seed_from_u64(seed);
    indices.shuffle(&mut rng);
    let val_size = (valid_records.len() as f32 * val_split) as usize;
    let (val_indices, train_indices) = indices.split_at(val_size);

    let train_records: Vec<_> = train_indices
        .iter()
        .map(|&i| valid_records[i].clone())
        .collect();
    let val_records: Vec<_> = val_indices
        .iter()
        .map(|&i| valid_records[i].clone())
        .collect();

    // Remap table groups for train/val splits — each group's record_indices
    // need to be re-indexed into the split-local arrays
    let train_idx_map: std::collections::HashMap<usize, usize> = train_indices
        .iter()
        .enumerate()
        .map(|(new, &old)| (old, new))
        .collect();
    let val_idx_map: std::collections::HashMap<usize, usize> = val_indices
        .iter()
        .enumerate()
        .map(|(new, &old)| (old, new))
        .collect();

    let mut train_groups = Vec::new();
    let mut val_groups = Vec::new();
    for group in &remapped_groups {
        // Count how many records from this group land in train vs val
        let train_remap: Vec<usize> = group
            .record_indices
            .iter()
            .filter_map(|idx| train_idx_map.get(idx).copied())
            .collect();
        let val_remap: Vec<usize> = group
            .record_indices
            .iter()
            .filter_map(|idx| val_idx_map.get(idx).copied())
            .collect();
        if !train_remap.is_empty() {
            train_groups.push(finetype_train::multi_branch::TableGroup {
                record_indices: train_remap,
                sibling_headers: group.sibling_headers.clone(),
            });
        }
        if !val_remap.is_empty() {
            val_groups.push(finetype_train::multi_branch::TableGroup {
                record_indices: val_remap,
                sibling_headers: group.sibling_headers.clone(),
            });
        }
    }

    eprintln!(
        "Train: {} ({} groups) | Val: {} ({} groups)",
        train_records.len(),
        train_groups.len(),
        val_records.len(),
        val_groups.len(),
    );

    let char_dim = header.char_dim as usize;
    let embed_dim = header.embed_dim as usize;
    let stats_dim = header.stats_dim as usize;
    let header_dim = header.header_dim as usize;
    let valid_dim = header.valid_dim as usize;

    let train_data = MultiBranchDataset::from_records_with_groups(
        &train_records,
        &label_to_idx,
        char_dim,
        embed_dim,
        stats_dim,
        header_dim,
        valid_dim,
        Some(train_groups),
    )?;
    let val_data = MultiBranchDataset::from_records_with_groups(
        &val_records,
        &label_to_idx,
        char_dim,
        embed_dim,
        stats_dim,
        header_dim,
        valid_dim,
        Some(val_groups),
    )?;

    let model_config =
        if let Some(config_path) = &model_config {
            // Load architecture from JSON config file
            let config_str = std::fs::read_to_string(config_path).map_err(|e| {
                anyhow::anyhow!(
                    "Failed to read model config {}: {}",
                    config_path.display(),
                    e
                )
            })?;
            let mut cfg: MultiBranchConfig = serde_json::from_str(&config_str).map_err(|e| {
                anyhow::anyhow!(
                    "Failed to parse model config {}: {}",
                    config_path.display(),
                    e
                )
            })?;
            // Override n_classes and dropout from CLI args (these are training params, not architecture)
            cfg.n_classes = n_classes;
            cfg.dropout = dropout;
            cfg.head_type = head_type.clone();
            eprintln!(
            "Loaded model config from {}: char_hidden={:?}, embed_hidden={:?}, merge_hidden={:?}",
            config_path.display(), cfg.char_hidden, cfg.embed_hidden, cfg.merge_hidden,
        );
            cfg
        } else {
            MultiBranchConfig {
                char_dim,
                embed_dim,
                stats_dim,
                header_dim,
                header_hidden: if header_dim > 0 { [128, 64] } else { [0, 0] },
                n_classes,
                dropout,
                head_type: head_type.clone(),
                ..Default::default()
            }
        };

    let train_config = MultiBranchTrainConfig {
        output_dir: output.clone(),
        epochs,
        batch_size,
        lr,
        weight_decay,
        patience,
        seed,
        logit_adjust_tau,
        ..Default::default()
    };

    let labels_opt = if head_type == HeadType::Hierarchical {
        Some(labels_list.as_slice())
    } else {
        None
    };

    // Create renderer: TUI dashboard by default, log-only with --no-tui
    let renderer: Option<Box<dyn TrainingRenderer>> = if no_tui {
        Some(Box::new(LogRenderer::new()))
    } else {
        let head_label = match &model_config.head_type {
            HeadType::Flat => "Flat",
            HeadType::Hierarchical => "Hierarchical",
        };
        let title = format!(
            "Multi-Branch {} ({} classes, {} epochs)",
            head_label, model_config.n_classes, train_config.epochs
        );
        match finetype_train::tui::TuiRenderer::new(title) {
            Ok(tui) => Some(Box::new(tui)),
            Err(e) => {
                eprintln!("TUI init failed ({e}), falling back to log output");
                Some(Box::new(LogRenderer::new()))
            }
        }
    };

    // Pass sibling-context model path if available — loaded inside
    // train_multi_branch on the same device as the training model to
    // avoid Metal device handle mismatch.
    let sibling_ctx_dir = std::path::PathBuf::from("models/sibling-context");
    let sibling_ctx_path = if sibling_ctx_dir.join("model.safetensors").exists() {
        eprintln!(
            "Sibling-context model found at {}",
            sibling_ctx_dir.display()
        );
        Some(sibling_ctx_dir)
    } else {
        None
    };

    let summary = train_multi_branch(
        &train_config,
        &model_config,
        &train_data,
        &val_data,
        labels_opt,
        sibling_ctx_path.as_deref(),
        renderer,
    )?;

    // Save label_map.json (index → label mapping, required for inference)
    let label_map_path = output.join("label_map.json");
    let label_map_json = serde_json::to_string_pretty(&labels_list)?;
    std::fs::write(&label_map_path, label_map_json)?;
    eprintln!(
        "Saved label map ({} labels) to {}",
        labels_list.len(),
        label_map_path.display()
    );

    eprintln!();
    eprintln!("Training complete:");
    eprintln!("  Best epoch: {}", summary.best_epoch + 1);
    eprintln!(
        "  Best val accuracy: {:.2}%",
        summary.best_val_accuracy * 100.0
    );
    eprintln!("  Total epochs: {}", summary.total_epochs);
    eprintln!("  Total time: {:.1}s", summary.total_time_secs);
    eprintln!("  Model saved to: {}", output.display());

    Ok(())
}

/// Extract multi-branch feature vectors from a column of values read from stdin.
///
/// Reads values (one per line, or JSON array with --json), then extracts:
/// - char: 960-dim character distribution features
/// - embed: 512-dim Model2Vec embedding aggregation features
/// - stats: 27-dim column-level statistics
///
/// Outputs JSON to stdout.
/// Dump late-fusion training features (View1 ⊕ View2) over a labelled corpus.
///
/// View1 (728) = value-CharCNN 240-softmax aggregated over the column's sampled
/// values: mean(240) ⊕ max(240) ⊕ argmax-vote-histogram(240) ⊕ 8 confidence
/// scalars. View2 (240) = multi-branch raw logits. Both aligned to the canonical
/// label order = the multi-branch `labels()` ordering. Writes a raw f32 blob plus
/// a label tsv and a meta json (see the subcommand doc). The fusion head trains
/// in Python on these.
fn cmd_dump_fusion_features(
    input: &PathBuf,
    value_model: &PathBuf,
    mb_model: &PathBuf,
    output: &Path,
    sample_n: usize,
    limit: usize,
    key_col: Option<&str>,
) -> Result<()> {
    use std::collections::HashMap;
    use std::io::Write;

    const V1_DIM: usize = 728; // 240*3 + 8
    const V2_DIM: usize = 240;
    const N_SCALARS: usize = 8;

    // Load models + taxonomy once (the expensive part).
    let value_clf = load_char_classifier(value_model)?;
    let mb = load_multi_branch_classifier(mb_model)?;
    let mut taxonomy = load_taxonomy(&PathBuf::from("labels"))?;
    taxonomy.compile_validators();
    taxonomy.compile_locale_validators();

    // Canonical label order = multi-branch labels.
    let canonical: Vec<String> = mb.labels().to_vec();
    let n_classes = canonical.len();
    if n_classes != V2_DIM {
        anyhow::bail!("multi-branch model has {n_classes} classes, fusion dump expects {V2_DIM}");
    }
    let name_to_idx: HashMap<&str, usize> = canonical
        .iter()
        .enumerate()
        .map(|(i, l)| (l.as_str(), i))
        .collect();

    let mut rdr =
        csv::Reader::from_path(input).map_err(|e| anyhow::anyhow!("open {input:?}: {e}"))?;
    let headers = rdr.headers()?.clone();
    let col = |name: &str| headers.iter().position(|h| h == name);
    let label_col = col("final_label")
        .or_else(|| col("classification"))
        .ok_or_else(|| anyhow::anyhow!("input missing final_label/classification column"))?;
    let values_col = col("sample_values")
        .ok_or_else(|| anyhow::anyhow!("input missing sample_values column"))?;
    let header_col = col("column_name");
    let key_idx = match key_col {
        Some(name) => {
            Some(col(name).ok_or_else(|| anyhow::anyhow!("input missing key column {name:?}"))?)
        }
        None => None,
    };

    let feat_path = output.with_extension("f32");
    let labels_path = output.with_extension("labels.tsv");
    let meta_path = output.with_extension("meta.json");
    let mut feat_out = std::io::BufWriter::new(std::fs::File::create(&feat_path)?);
    let mut labels_out = std::io::BufWriter::new(std::fs::File::create(&labels_path)?);
    writeln!(labels_out, "row_idx\tlabel_index\tlabel_name")?;
    let keys_path = output.with_extension("keys.tsv");
    let mut keys_out = match key_idx {
        Some(_) => {
            let mut w = std::io::BufWriter::new(std::fs::File::create(&keys_path)?);
            writeln!(w, "row_idx\tkey")?;
            Some(w)
        }
        None => None,
    };

    let mut n_rows = 0usize;
    let mut skipped_unknown_label = 0usize;
    let mut skipped_empty = 0usize;
    let row_dim = V1_DIM + V2_DIM;

    for rec in rdr.records() {
        let rec = rec?;
        let label = rec.get(label_col).unwrap_or("").trim().to_string();
        let raw_vals = rec.get(values_col).unwrap_or("[]");
        let header = header_col
            .and_then(|c| rec.get(c))
            .unwrap_or("")
            .trim()
            .to_string();

        // In keyed/predict mode we don't need a gold label (target_idx == -1 sentinel);
        // in training mode an unknown label is skipped.
        let target_idx: i64 = match name_to_idx.get(label.as_str()) {
            Some(&i) => i as i64,
            None => {
                if key_idx.is_some() {
                    -1
                } else {
                    skipped_unknown_label += 1;
                    continue;
                }
            }
        };

        let parsed: Vec<String> = serde_json::from_str(raw_vals).unwrap_or_default();
        let sampled: Vec<String> = parsed.into_iter().take(sample_n.max(1)).collect();
        if sampled.is_empty() {
            skipped_empty += 1;
            continue;
        }
        // ---- View1 + View2 row (shared with inference — see fusion::compute_fusion_row) ----
        let rowbuf = finetype_model::compute_fusion_row(
            &value_clf,
            &mb,
            &name_to_idx,
            n_classes,
            &sampled,
            &header,
            sample_n,
            Some(&taxonomy),
        )?;
        debug_assert_eq!(rowbuf.len(), row_dim);
        let mut bytes: Vec<u8> = Vec::with_capacity(row_dim * 4);
        for f in &rowbuf {
            bytes.extend_from_slice(&f.to_le_bytes());
        }
        feat_out.write_all(&bytes)?;
        writeln!(labels_out, "{}\t{}\t{}", n_rows, target_idx, label)?;
        if let (Some(w), Some(ki)) = (keys_out.as_mut(), key_idx) {
            writeln!(w, "{}\t{}", n_rows, rec.get(ki).unwrap_or(""))?;
        }

        n_rows += 1;
        if n_rows.is_multiple_of(5000) {
            eprintln!("  dumped {n_rows} rows...");
        }
        if limit > 0 && n_rows >= limit {
            break;
        }
    }
    feat_out.flush()?;
    labels_out.flush()?;
    if let Some(w) = keys_out.as_mut() {
        w.flush()?;
    }

    let meta = serde_json::json!({
        "n_rows": n_rows,
        "row_dim": row_dim,
        "view1_dim": V1_DIM,
        "view2_dim": V2_DIM,
        "n_scalars": N_SCALARS,
        "sample_n": sample_n,
        "value_model": value_model.to_string_lossy(),
        "mb_model": mb_model.to_string_lossy(),
        "input": input.to_string_lossy(),
        "skipped_unknown_label": skipped_unknown_label,
        "skipped_empty": skipped_empty,
        "label_order": canonical,
        "layout": "mean(240) | max(240) | vote(240) | scalars(8) | mb_logits(240)",
        "scalar_names": [
            "mean_top1","max_top1","min_top1","std_top1",
            "mean_entropy","vote_agreement","frac_unique","n_used_norm"
        ],
        "dtype": "float32_le",
    });
    std::fs::write(&meta_path, serde_json::to_string_pretty(&meta)? + "\n")?;

    eprintln!(
        "dump-fusion-features: wrote {n_rows} rows x {row_dim} f32 -> {feat_path:?}\n  \
         labels -> {labels_path:?}  meta -> {meta_path:?}\n  \
         skipped: unknown_label={skipped_unknown_label} empty={skipped_empty}"
    );
    Ok(())
}

fn cmd_extract_features(
    header: Option<String>,
    json_input: bool,
    include_validation: bool,
) -> Result<()> {
    use finetype_model::{
        extract_char_distribution, extract_column_stats, extract_embedding_aggregation,
        ValidationFeatureExtractor, CHAR_DIST_DIM, COLUMN_STATS_DIM, EMBED_AGG_DIM,
    };

    // Read values from stdin
    let stdin = io::stdin();
    let values: Vec<String> = if json_input {
        let mut buf = String::new();
        stdin.lock().read_to_string(&mut buf)?;
        let parsed: Vec<String> = serde_json::from_str(&buf)
            .map_err(|e| anyhow::anyhow!("Failed to parse JSON array from stdin: {}", e))?;
        parsed
    } else {
        stdin.lock().lines().collect::<Result<Vec<_>, _>>()?
    };

    if values.is_empty() {
        anyhow::bail!("No values provided on stdin");
    }

    let value_refs: Vec<&str> = values.iter().map(|s| s.as_str()).collect();

    // Load Model2Vec resources (shared across embed + header features)
    let m2v = load_model2vec_resources();

    // 1. Character distribution (960-dim, deterministic, no model needed)
    let char_features = extract_char_distribution(&value_refs).unwrap_or([0.0f32; CHAR_DIST_DIM]);

    // 2. Embedding aggregation (512-dim, requires Model2Vec)
    let embed_features = match &m2v {
        Some(m2v) => {
            extract_embedding_aggregation(&value_refs, m2v).unwrap_or([0.0f32; EMBED_AGG_DIM])
        }
        None => {
            eprintln!("Warning: Model2Vec not available, embedding features will be zeros");
            [0.0f32; EMBED_AGG_DIM]
        }
    };

    // 3. Column statistics (27-dim, deterministic)
    let stats_features = extract_column_stats(&value_refs).unwrap_or([0.0f32; COLUMN_STATS_DIM]);

    // 4. Header embedding (128-dim, requires Model2Vec + header string)
    let header_features: Vec<f32> = match (&m2v, &header) {
        (Some(m2v), Some(h)) if !h.is_empty() => {
            let embed_dim = m2v.embed_dim().unwrap_or(128);
            match m2v.encode_one(h) {
                Some(tensor) => tensor.to_vec1::<f32>().unwrap_or(vec![0.0f32; embed_dim]),
                None => vec![0.0f32; embed_dim],
            }
        }
        (Some(m2v), _) => {
            // No header provided — zero vector
            let embed_dim = m2v.embed_dim().unwrap_or(128);
            vec![0.0f32; embed_dim]
        }
        (None, _) => {
            eprintln!("Warning: Model2Vec not available, header features will be zeros");
            vec![0.0f32; 128]
        }
    };

    // 5. Validation pass-rate features (239-dim, requires taxonomy with compiled validators)
    let (validation_features, type_index_keys) = if include_validation {
        let taxonomy_path = PathBuf::from("labels");
        let mut taxonomy = load_taxonomy(&taxonomy_path)?;
        taxonomy.compile_validators();
        let extractor = ValidationFeatureExtractor::new(&taxonomy);
        let feats = extractor.extract(&value_refs, &taxonomy);
        let keys: Vec<String> = extractor.type_keys().to_vec();
        (feats, keys)
    } else {
        (Vec::new(), Vec::new())
    };

    // Output as JSON
    let mut output = json!({
        "char": char_features.to_vec(),
        "embed": embed_features.to_vec(),
        "stats": stats_features.to_vec(),
        "header_features": header_features,
        "header": header,
        "n_values": values.len(),
    });

    if include_validation {
        output["validation"] = json!(validation_features);
        output["type_index_keys"] = json!(type_index_keys);
    }

    let stdout = io::stdout();
    serde_json::to_writer(stdout.lock(), &output)?;
    println!();

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn cmd_infer(
    input: Option<String>,
    file: Option<PathBuf>,
    output: OutputFormat,
    show_confidence: bool,
    show_value: bool,
    model_type: ModelType,
    mode: InferenceMode,
    sample_size: usize,
    bench: bool,
    header: Option<String>,
    batch: bool,
    explain: bool,
    taxonomy: PathBuf,
) -> Result<()> {
    use finetype_model::{ClassificationResult, ColumnClassifier, ColumnConfig};
    use std::time::Instant;

    if matches!(model_type, ModelType::LateFusion) {
        anyhow::bail!("late-fusion is only supported by `finetype profile`");
    }

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
        return cmd_infer_batch(model, model_type, sample_size);
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
            OutputFormat::Plain
            | OutputFormat::Markdown
            | OutputFormat::Arrow
            | OutputFormat::JsonSchema => {
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
        let config = ColumnConfig {
            sample_size,
            ..Default::default()
        };
        let mut column_classifier = if matches!(model_type, ModelType::MultiBranch) {
            let mb = load_multi_branch_classifier(&model)?;
            ColumnClassifier::with_multi_branch(mb, config)
        } else {
            let classifier: Box<dyn finetype_model::ValueClassifier> = match model_type {
                ModelType::CharCnn => Box::new(load_char_classifier(&model)?),
                ModelType::Tiered => Box::new(load_tiered_classifier(&model)?),
                ModelType::Transformer => Box::new(finetype_model::Classifier::load(&model)?),
                ModelType::MultiBranch | ModelType::LateFusion => unreachable!(),
            };
            let semantic_hint = load_semantic_hint();
            if let Some(semantic) = semantic_hint {
                // Load entity classifier (shares Model2Vec tokenizer/embeddings)
                let entity = load_entity_classifier(&semantic);
                let mut cc = ColumnClassifier::with_semantic_hint(classifier, config, semantic);
                if let Some(entity) = entity {
                    cc.set_entity_classifier(entity);
                }
                cc
            } else {
                ColumnClassifier::new(classifier, config)
            }
        };

        // Load taxonomy for validation-based attractor demotion (Rule 14)
        let taxonomy_path = std::path::PathBuf::from("labels");
        if let Ok(mut taxonomy) = load_taxonomy(&taxonomy_path) {
            taxonomy.compile_validators();
            taxonomy.compile_locale_validators();
            column_classifier.set_taxonomy(taxonomy);
        }

        // Wire up Sense classifier (Sense → Sharpen pipeline) for legacy non-multi-branch models
        if !column_classifier.has_multi_branch() {
            wire_sense(&mut column_classifier);
            wire_sibling_context(&mut column_classifier);
        }
        // Multi-branch path: wire Model2Vec + sibling context for header enrichment
        if column_classifier.has_multi_branch() {
            wire_model2vec_and_siblings(&mut column_classifier);
        }

        let result = if let Some(ref hdr) = header {
            column_classifier.classify_column_with_header(&inputs, hdr)?
        } else {
            column_classifier.classify_column(&inputs)?
        };

        match output {
            OutputFormat::Plain
            | OutputFormat::Markdown
            | OutputFormat::Arrow
            | OutputFormat::JsonSchema => {
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
    if matches!(model_type, ModelType::MultiBranch) {
        anyhow::bail!(
            "Multi-branch models are column-level only. Use --mode column or `finetype profile` instead."
        );
    }
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
        ModelType::MultiBranch | ModelType::LateFusion => unreachable!("guarded above"),
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
fn cmd_infer_batch(model: PathBuf, model_type: ModelType, sample_size: usize) -> Result<()> {
    use finetype_model::{ColumnClassifier, ColumnConfig, ValueClassifier};
    use std::time::Instant;

    let t_start = Instant::now();

    let config = ColumnConfig {
        sample_size,
        ..Default::default()
    };

    let mut column_classifier = if matches!(model_type, ModelType::MultiBranch) {
        let mb = load_multi_branch_classifier(&model)?;
        eprintln!(
            "Loaded multi-branch classifier ({} classes)",
            mb.n_classes()
        );
        ColumnClassifier::with_multi_branch(mb, config)
    } else {
        // Load value-level classifier
        let classifier: Box<dyn ValueClassifier> = match model_type {
            ModelType::CharCnn => Box::new(load_char_classifier(&model)?),
            ModelType::Tiered => Box::new(load_tiered_classifier(&model)?),
            ModelType::Transformer => Box::new(finetype_model::Classifier::load(&model)?),
            ModelType::MultiBranch | ModelType::LateFusion => unreachable!(),
        };

        // Wire up semantic hint (Model2Vec) — same as profile command
        if let Some(semantic) = load_semantic_hint() {
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
        }
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

    // Wire up Sense classifier (Sense → Sharpen pipeline) for legacy non-multi-branch models
    if !column_classifier.has_multi_branch() {
        wire_sense(&mut column_classifier);
        wire_sibling_context(&mut column_classifier);
    }
    // Multi-branch path: wire Model2Vec + sibling context for header enrichment
    if column_classifier.has_multi_branch() {
        wire_model2vec_and_siblings(&mut column_classifier);
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

/// Load a MultiBranchClassifier: try the model directory first, then fall back to
/// the embedded model if the path doesn't exist (release binaries).
fn load_multi_branch_classifier(model: &PathBuf) -> Result<finetype_model::MultiBranchClassifier> {
    if model.exists() && model.join("config.json").exists() {
        finetype_model::MultiBranchClassifier::load(model).map_err(Into::into)
    } else {
        #[cfg(feature = "embed-models")]
        {
            if embedded::EMBEDDED_MODEL_TYPE == "multi-branch" && !embedded::MB_WEIGHTS.is_empty() {
                // Load Model2Vec resources (disk or embedded)
                let m2v = load_model2vec_resources().ok_or_else(|| {
                    anyhow::anyhow!(
                        "Multi-branch model requires Model2Vec resources but none found"
                    )
                })?;
                // Release-embedded single-encoder path: no separate value encoder
                // (dual-encoder potion-8M ships via the disk/HF path; ac-04 extends
                // this to embed the value encoder too).
                return finetype_model::MultiBranchClassifier::from_bytes(
                    embedded::MB_CONFIG,
                    embedded::MB_LABELS,
                    embedded::MB_WEIGHTS,
                    m2v,
                    None,
                )
                .map_err(Into::into);
            }
        }
        anyhow::bail!(
            "Model directory {:?} not found and no embedded multi-branch model available. \
             Set FINETYPE_MODEL_DIR or build with `embed-models` feature.",
            model
        )
    }
}

/// Load a CharClassifier: try the model directory first, then fall back to
/// the embedded model if the path doesn't exist (release binaries).
///
/// Automatically loads validation patterns from the taxonomy to enable
/// pattern-gated post-processing.
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

/// Load a B3 late-fusion classifier from a `fusion_manifest.json` directory.
///
/// The manifest points at the three frozen sub-models (paths relative to the
/// manifest dir, or absolute):
///   { "value_model": "...", "mb_model": "...", "head": "...", "sample_n": 32 }
///
/// Asserts `feature_dim == 0` on the value sub-model (ac-06): a feature-trained
/// CharCNN would silently zero-fill its feature vector at inference.
fn load_fusion_classifier(model: &Path) -> Result<finetype_model::FusionClassifier> {
    let manifest_path = model.join("fusion_manifest.json");
    let manifest: serde_json::Value = serde_json::from_slice(
        &std::fs::read(&manifest_path)
            .map_err(|e| anyhow::anyhow!("read {manifest_path:?}: {e}"))?,
    )
    .map_err(|e| anyhow::anyhow!("parse {manifest_path:?}: {e}"))?;

    let resolve = |key: &str| -> Result<PathBuf> {
        let p = manifest[key]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("fusion_manifest missing string field {key:?}"))?;
        let pb = PathBuf::from(p);
        Ok(if pb.is_absolute() { pb } else { model.join(pb) })
    };
    let value_dir = resolve("value_model")?;
    let mb_dir = resolve("mb_model")?;
    let head_dir = resolve("head")?;
    let sample_n = manifest["sample_n"].as_u64().unwrap_or(32) as usize;

    // Assert feature_dim == 0 on the value sub-model (read its config directly).
    let vcfg_path = value_dir.join("config.yaml");
    if let Ok(cfg) = std::fs::read_to_string(&vcfg_path) {
        for line in cfg.lines() {
            if let Some(rest) = line.trim().strip_prefix("feature_dim:") {
                let fd: usize = rest.trim().parse().unwrap_or(0);
                if fd != 0 {
                    anyhow::bail!(
                        "fusion value sub-model {value_dir:?} has feature_dim={fd}, expected 0 \
                         (a feature-trained CharCNN zero-fills features at inference)"
                    );
                }
            }
        }
    }

    let value_clf = load_char_classifier(&value_dir)?;
    let mb = load_multi_branch_classifier(&mb_dir)?;
    let (head, head_labels) = finetype_model::FusionHead::load(&head_dir)?;
    let fusion = finetype_model::FusionClassifier::new(value_clf, mb, head, head_labels, sample_n)?;
    Ok(fusion)
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

/// Load the entity classifier for full_name demotion.
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

/// Load the Sense classifier for broad category prediction.
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

/// Wire Model2Vec + sibling context for multi-branch classifiers.
///
/// When multi-branch is active, Sense is not used — but sibling-context attention
/// still needs Model2Vec to encode headers. This wires both independently of Sense.
fn wire_model2vec_and_siblings(cc: &mut finetype_model::ColumnClassifier) {
    if let Some(m2v) = load_model2vec_resources() {
        eprintln!("Loaded Model2Vec for multi-branch sibling context");
        cc.set_model2vec(m2v);
        wire_sibling_context(cc);
    }
}

/// Load and wire the sibling-context attention module.
///
/// Looks for `models/sibling-context/model.safetensors`. When found,
/// attaches to the column classifier. When absent, the pipeline is unchanged.
fn wire_sibling_context(cc: &mut finetype_model::ColumnClassifier) {
    let model_dir = std::path::PathBuf::from("models/sibling-context");
    if !model_dir.join("model.safetensors").exists() {
        return; // Silent — model is optional
    }
    match finetype_model::SiblingContextAttention::load(&model_dir) {
        Ok(sibling) => {
            eprintln!(
                "Loaded sibling-context attention ({} params)",
                sibling.param_count()
            );
            cc.set_sibling_context(sibling);
        }
        Err(e) => {
            eprintln!("Warning: Failed to load sibling-context model: {e}");
        }
    }
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
    use_features: bool,
    hierarchical: bool,
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
                use_features,
                use_hierarchical: hierarchical,
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
        ModelType::MultiBranch => {
            anyhow::bail!(
                "Multi-branch training uses `finetype train-multi-branch`, not `finetype train`."
            );
        }
        ModelType::LateFusion => {
            anyhow::bail!(
                "Late-fusion head training uses `train-fusion-head`, not `finetype train`."
            );
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
    type_key: Option<String>,
    file: PathBuf,
    domain: Option<String>,
    category: Option<String>,
    priority: Option<u8>,
    output: OutputFormat,
    full: bool,
) -> Result<()> {
    let taxonomy = load_taxonomy(&file)?;

    // Collect matching definitions. A positional KEY takes precedence
    // over --domain / --category / --priority filters and uses the same
    // exact-match-or-glob predicate previously implemented in
    // `cmd_schema` (card 0006 absorbs that path).
    let mut defs: Vec<(&String, &finetype_core::Definition)> = if let Some(key) = &type_key {
        if key.contains('*') {
            // Glob: support "domain.*", "domain.category.*", "*", etc.
            let prefix = key.trim_end_matches(".*").trim_end_matches('*');
            taxonomy
                .definitions()
                .filter(|(k, _)| {
                    if prefix.is_empty() {
                        true
                    } else {
                        k.starts_with(prefix)
                            && (k.len() == prefix.len()
                                || k.as_bytes().get(prefix.len()) == Some(&b'.'))
                    }
                })
                .collect()
        } else {
            // Exact match — exit 1 with edit-distance suggestions on miss.
            match taxonomy.get(key) {
                Some(_) => taxonomy
                    .definitions()
                    .filter(|(k, _)| k.as_str() == key.as_str())
                    .collect(),
                None => {
                    let mut suggestions: Vec<(&String, usize)> = taxonomy
                        .definitions()
                        .map(|(k, _)| (k, levenshtein_distance(key, k)))
                        .collect();
                    suggestions.sort_by_key(|(_, d)| *d);
                    suggestions.truncate(5);

                    eprintln!("Error: unknown type '{}'", key);
                    if !suggestions.is_empty() {
                        eprintln!("\nDid you mean:");
                        for (s, _) in &suggestions {
                            eprintln!("  {}", s);
                        }
                    }
                    std::process::exit(1);
                }
            }
        }
    } else if let (Some(dom), Some(cat)) = (&domain, &category) {
        taxonomy.by_category(dom, cat)
    } else if let Some(dom) = &domain {
        taxonomy.by_domain(dom)
    } else if let Some(prio) = priority {
        taxonomy.at_priority(prio)
    } else {
        taxonomy.definitions().collect()
    };

    // Apply priority filter on top of domain/category. Skipped when a
    // positional KEY is supplied (the KEY is authoritative — it pins to
    // a single type or a glob and ignores priority).
    if type_key.is_none() {
        if let Some(prio) = priority {
            defs.retain(|(_, d)| d.release_priority >= prio);
        }
    }

    defs.sort_by_key(|(k, _)| (*k).clone());

    // Glob-with-zero-matches under positional KEY gets the same exit-1
    // contract as exact-key-with-zero-matches (already handled above).
    if type_key.is_some() && defs.is_empty() {
        eprintln!(
            "Error: no types matching '{}'",
            type_key.as_deref().unwrap_or("")
        );
        std::process::exit(1);
    }

    match output {
        OutputFormat::Plain | OutputFormat::Markdown | OutputFormat::Arrow => {
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
        OutputFormat::JsonSchema => {
            // ac-03: per-type JSON Schema export, always-array shape
            // (even single matches) — matches `taxonomy`'s other output
            // formats. Pretty-print is unconditional, as with `Json`.
            let schemas: Vec<serde_json::Value> = defs
                .iter()
                .map(|(key, def)| json_schema::emit_type_schema(key, def))
                .collect();
            println!("{}", serde_json::to_string_pretty(&schemas)?);
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
    obj.insert("format_string_alt".into(), json!(d.format_string_alt));
    obj.insert("transform".into(), json!(d.transform));
    obj.insert("transform_ext".into(), json!(d.transform_ext));
    obj.insert("locales".into(), json!(d.locales));
    obj.insert("tier".into(), json!(d.tier));
    obj.insert("release_priority".into(), json!(d.release_priority));
    obj.insert("aliases".into(), json!(d.aliases));
    obj.insert("pii".into(), json!(d.pii));
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

/// Map DuckDB SQL type to Arrow DataType JSON representation.
///
/// Uses the Arrow IPC JSON schema format compatible with arrow-rs and pyarrow.
fn duckdb_to_arrow_type(duckdb_type: &str) -> serde_json::Value {
    match duckdb_type {
        "VARCHAR" => json!({"name": "utf8"}),
        "DOUBLE" => json!({"name": "floatingpoint", "precision": "DOUBLE"}),
        "BIGINT" => json!({"name": "int", "bitWidth": 64, "isSigned": true}),
        "DECIMAL" => json!({"name": "decimal", "precision": 38, "scale": 10, "bitWidth": 128}),
        "DATE" => json!({"name": "date", "unit": "DAY"}),
        "TIMESTAMP" => json!({"name": "timestamp", "unit": "MICROSECOND", "timezone": null}),
        "TIME" => json!({"name": "time", "unit": "MICROSECOND", "bitWidth": 64}),
        "BOOLEAN" => json!({"name": "bool"}),
        "JSON" => json!({"name": "utf8"}),
        "STRUCT" => json!({"name": "struct"}),
        "LIST" => json!({"name": "list"}),
        _ => json!({"name": "utf8"}),
    }
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
        OutputFormat::Plain
        | OutputFormat::Markdown
        | OutputFormat::Arrow
        | OutputFormat::JsonSchema => {
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
// VALIDATE — Schema-driven CSV quality gate
// ═══════════════════════════════════════════════════════════════════════════════

// ═══════════════════════════════════════════════════════════════════════════════
// VALIDATE — DuckDB-native reject pipeline (spec v1.2 ac-06, ac-08, ac-09, ac-10, ac-11)
// ═══════════════════════════════════════════════════════════════════════════════

/// Exit codes.
///
/// - 0: no rejects, no error
/// - 1: one or more rejects (default CI-gate)
/// - 2: error (bad schema, file unreadable, DuckDB error, staging collision
///   without `--append`). Not suppressed by `--lenient`.
///
/// `--lenient` forces 0 whenever the exit would otherwise be 1.
fn exit_with(code: i32) -> ! {
    std::process::exit(code);
}

/// Load + parse a JSON Schema file with structured error messages (ac-08).
///
/// Emits one-line `error:` messages to stderr, then exits 2. Fail-fast ordering:
/// (1) missing-file, (2) permission-denied, (3) invalid-JSON, (4) missing
/// `properties` object.
fn load_schema_or_exit(schema_path: &PathBuf) -> serde_json::Value {
    // (1) + (2): open and read. std::fs::read_to_string produces distinct
    //     io::ErrorKind values we can discriminate on.
    let schema_content = match std::fs::read_to_string(schema_path) {
        Ok(s) => s,
        Err(e) => {
            match e.kind() {
                std::io::ErrorKind::NotFound => {
                    eprintln!("error: schema file not found: {}", schema_path.display());
                }
                std::io::ErrorKind::PermissionDenied => {
                    eprintln!(
                        "error: permission denied reading schema file: {}",
                        schema_path.display()
                    );
                }
                _ => {
                    eprintln!(
                        "error: could not read schema file {}: {}",
                        schema_path.display(),
                        e
                    );
                }
            }
            exit_with(2);
        }
    };

    // (3): parse JSON. serde_json errors include a byte/line position.
    let schema: serde_json::Value = match serde_json::from_str(&schema_content) {
        Ok(v) => v,
        Err(e) => {
            eprintln!(
                "error: invalid JSON in schema file {}: {} (at line {} col {})",
                schema_path.display(),
                e,
                e.line(),
                e.column()
            );
            exit_with(2);
        }
    };

    // (4): structural check — must have an object `properties`.
    if !schema.is_object()
        || schema
            .get("properties")
            .and_then(|p| p.as_object())
            .is_none()
    {
        eprintln!(
            "error: schema file {} is missing required `properties` object",
            schema_path.display()
        );
        exit_with(2);
    }

    schema
}

// ═══════════════════════════════════════════════════════════════════════════════
// EVAL — Evaluate model accuracy on a test set
// ═══════════════════════════════════════════════════════════════════════════════

fn cmd_eval(
    data: PathBuf,
    _taxonomy_path: PathBuf,
    model_type: ModelType,
    top_confusions: usize,
    output: OutputFormat,
) -> Result<()> {
    use finetype_model::{CharClassifier, ClassificationResult};
    use std::collections::HashMap;

    let model = resolve_model_path();

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
        ModelType::MultiBranch => {
            anyhow::bail!(
                "Multi-branch models are column-level only and cannot be evaluated with value-level test data."
            );
        }
        ModelType::LateFusion => {
            anyhow::bail!(
                "Late-fusion models are column-level only and cannot be evaluated with value-level test data; use `finetype profile`."
            );
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
    confusion_vec.sort_by_key(|b| std::cmp::Reverse(b.1));

    match output {
        OutputFormat::Plain
        | OutputFormat::Csv
        | OutputFormat::Markdown
        | OutputFormat::Arrow
        | OutputFormat::JsonSchema => {
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

/// Validate a single value against a label's live `CompiledValidator`.
///
/// Hidden subcommand backing the runtime/eval parity test (ac-04). It
/// exercises the SAME validator the profile veto (ac-06) uses —
/// `validate_value_for_label`, which carries the ac-02 scoped enum
/// case-fold and the ac-01 widened patterns/bounds. Prints `PASS`/`FAIL`
/// so a shell-out test can cross-check it against the Python eval gate.
fn cmd_validate_value(label: String, value: String, taxonomy_path: PathBuf) -> Result<()> {
    let mut taxonomy = load_taxonomy(&taxonomy_path)?;
    taxonomy.compile_validators();
    let result = finetype_core::validate_value_for_label(&value, &label, &taxonomy)
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    println!("{}", if result.is_valid { "PASS" } else { "FAIL" });
    Ok(())
}

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

mod profile;
mod profile_io;
mod sql;
mod validate;

use profile::*;
use profile_io::*;
use sql::*;
use validate::*;

#[cfg(test)]
mod tests;
