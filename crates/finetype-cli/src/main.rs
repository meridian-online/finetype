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
    /// Re-sharpen cached Sense predictions (diagnostic: corpus-honest gate fast path).
    ///
    /// Reads a TSV of `id<TAB>header<TAB>sense_label<TAB>sense_conf<TAB>values(0x1f-joined)`
    /// and writes `id<TAB>composed_label`, running the real Sharpen stack WITHOUT the
    /// value-encode (compose_from_sense). Lets a Sharpen-rule change be corpus-honest-gated
    /// in minutes instead of re-encoding the 33k sample. Spec 2026-06-27-composed-accuracy-roadmap.
    Resharpen {
        /// Input TSV (id, header, sense_label, sense_conf, 0x1f-joined values)
        #[arg(short, long)]
        input: PathBuf,
        /// Output TSV (id, composed_label)
        #[arg(short, long)]
        output: PathBuf,
        /// Model directory
        #[arg(short, long, default_value = "models/default")]
        model: PathBuf,
    },
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
        /// (minLength/maxLength, minimum/maximum, the closed `enum` keyword,
        /// x-finetype-null-rate, x-finetype-cardinality). Requires `-o json-schema`.
        /// NB the descriptive `x-finetype-enum` domain surfaces by DEFAULT (no flag)
        /// for any bounded column.
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

        /// Cede-list: path to a file of taxonomy leaves (one per line, `#` comments
        /// ignored) to DENY from the output label space (spec
        /// 2026-06-27-model-label-space-reshape). These leaves are removed from the
        /// model's softmax head — it can no longer emit them — and are recovered
        /// deterministically in the Sharpen layer. n_classes shrinks by the number of
        /// ceded leaves present in the taxonomy; the validation branch (valid_dim,
        /// one feature per taxonomy type) is unaffected.
        #[arg(long)]
        cede_labels: Option<PathBuf>,
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
    /// `finetype schema <file.csv>` invocation. The descriptive `x-finetype-enum`
    /// value domain surfaces by DEFAULT for any bounded column. With `--stats`,
    /// additionally attaches observed-data constraints (minLength/maxLength,
    /// minimum/maximum, the closed `enum` keyword) and the `x-finetype-null-rate` /
    /// `x-finetype-cardinality` extensions.
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

        Commands::Resharpen {
            input,
            output,
            model,
        } => cmd_resharpen(input, output, model),

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
            cede_labels,
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
            cede_labels,
        ),

        Commands::ExtractFeatures {
            header,
            json,
            validation,
        } => cmd_extract_features(header, json, validation),
    }
}

mod cmd_run;
mod cmd_taxonomy;
mod cmd_train;
mod model_loading;
mod profile;
mod profile_io;
mod sql;
mod validate;

use cmd_run::*;
use cmd_taxonomy::*;
use cmd_train::*;
use model_loading::*;
use profile::*;
use profile_io::*;
use sql::*;
use validate::*;

#[cfg(test)]
mod tests;
