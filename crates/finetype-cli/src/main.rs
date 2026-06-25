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
use serde_json::json;
use std::io::{self, BufRead, Read, Write};
use std::path::PathBuf;
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

        /// Inference mode: column (distribution-based disambiguation, default) or
        /// row. The shipped model is column-level, so row mode is unsupported.
        #[arg(long, default_value = "column")]
        mode: InferenceMode,

        /// Sample size for column mode (default 100)
        #[arg(long, default_value = "100")]
        sample_size: usize,

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

        /// Value-encoder model2vec directory (required when the model config has a
        /// `value_attention` block — choice 0106). Encodes the FTMB v6 value
        /// strings into per-value embeddings for the attention pool.
        #[arg(long)]
        value_encoder: Option<PathBuf>,
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
    /// Frictionless Data Package descriptor (choice 0105) — one Data Resource
    /// wrapping a Table Schema whose `type`/`format` come from the authoritative
    /// taxonomy map. `profile` only; the interoperable family-standard envelope.
    Datapackage,
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
            mode,
            sample_size,
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
            mode,
            sample_size,
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
                enum_threshold,
                stats,
                verbose,
                raw_model,
                no_validation_veto,
            )
        }

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
            value_encoder,
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
            value_encoder,
        ),

        Commands::ExtractFeatures {
            header,
            json,
            validation,
        } => cmd_extract_features(header, json, validation),
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

    // Build the multi-branch column classifier (the shipped model).
    let model_path = PathBuf::from("models/default");
    let mb = load_multi_branch_classifier(&model_path)?;
    eprintln!(
        "Loaded multi-branch classifier ({} classes)",
        mb.n_classes()
    );
    let mut column_classifier = ColumnClassifier::with_multi_branch(mb, config);
    wire_model2vec_and_siblings(&mut column_classifier);

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
    value_encoder: Option<PathBuf>,
) -> Result<()> {
    use finetype_model::model2vec_shared::Model2VecResources;
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

    // Cross-value attention (choice 0106): encode the FTMB v6 value strings into
    // per-value embeddings, once, with the value encoder. Done after model_config so
    // we know whether attention is enabled.
    let (train_data, val_data) = if let Some(va) = model_config.value_attention.clone() {
        let enc_dir = value_encoder.as_ref().ok_or_else(|| {
            anyhow::anyhow!(
                "model config has a `value_attention` block but --value-encoder was not given"
            )
        })?;
        let enc = Model2VecResources::load(enc_dir).map_err(|e| {
            anyhow::anyhow!("failed to load value encoder {}: {e}", enc_dir.display())
        })?;
        eprintln!(
            "Value attention: encoding up to {} values/col with {} ({}d) for {} train + {} val records",
            va.n_values,
            enc_dir.display(),
            va.value_embed_dim,
            train_records.len(),
            val_records.len(),
        );
        (
            train_data.with_value_attention(&train_records, &va, &enc)?,
            val_data.with_value_attention(&val_records, &va, &enc)?,
        )
    } else {
        (train_data, val_data)
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
                wire_model2vec_only(&mut column_classifier);
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
fn cmd_infer_batch(model: PathBuf, sample_size: usize) -> Result<()> {
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
        wire_model2vec_only(&mut column_classifier);
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
                // Dual-encoder: load the embedded value-branch encoder (potion-8M)
                // when present, so a released binary with no disk models drives the
                // value-aggregation branch correctly. Single-encoder models embed
                // HAS_MB_VALUE_M2V=false → None (value branch shares m2v).
                let value_m2v = if embedded::HAS_MB_VALUE_M2V {
                    Some(
                        finetype_model::Model2VecResources::from_bytes(
                            embedded::MB_VALUE_TOKENIZER,
                            embedded::MB_VALUE_MODEL,
                        )
                        .map_err(|e| {
                            anyhow::anyhow!("Failed to load embedded value encoder: {e}")
                        })?,
                    )
                } else {
                    None
                };
                return finetype_model::MultiBranchClassifier::from_bytes(
                    embedded::MB_CONFIG,
                    embedded::MB_LABELS,
                    embedded::MB_WEIGHTS,
                    m2v,
                    value_m2v,
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

/// Wire Model2Vec for a multi-branch classifier WITHOUT sibling context.
///
/// `infer` classifies one column at a time and never calls the cross-column
/// `classify_columns_with_context` path, so the sibling-context attention model
/// (396,800 params) would be loaded from disk only to never be invoked. Skipping
/// that load is the single-value/single-column infer fast path (card 0006).
/// `profile`, which has real siblings, still wires both via
/// `wire_model2vec_and_siblings`.
fn wire_model2vec_only(cc: &mut finetype_model::ColumnClassifier) {
    if let Some(m2v) = load_model2vec_resources() {
        cc.set_model2vec(m2v);
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
        OutputFormat::Plain
        | OutputFormat::Markdown
        | OutputFormat::Arrow
        | OutputFormat::Datapackage => {
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
        | OutputFormat::JsonSchema
        | OutputFormat::Datapackage => {
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

    // Frictionless mapping gate (choice 0105, spec 2026-06-24 ac-01): every
    // definition must carry a valid {type, format} block, else `check` fails.
    let mut fx_failures: Vec<String> = Vec::new();
    for (key, def) in taxonomy.definitions() {
        match &def.frictionless {
            None => fx_failures.push(format!("{key}: missing `frictionless` block")),
            Some(fr) => {
                if let Err(e) = fr.validate() {
                    fx_failures.push(format!("{key}: {e}"));
                }
            }
        }
    }
    if !fx_failures.is_empty() {
        eprintln!(
            "\nFrictionless mapping check FAILED ({} definition(s)):",
            fx_failures.len()
        );
        for f in &fx_failures {
            eprintln!("  - {f}");
        }
        std::process::exit(1);
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
