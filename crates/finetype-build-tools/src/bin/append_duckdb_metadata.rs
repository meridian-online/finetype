//! Append DuckDB extension metadata to a compiled shared library.
//!
//! Replaces the Python `append_extension_metadata.py` script from duckdb-finetype.

use clap::Parser;
use std::path::PathBuf;
use std::process;

#[derive(Parser)]
#[command(
    name = "append-duckdb-metadata",
    about = "Append DuckDB extension metadata to a shared library"
)]
struct Args {
    /// Path to the raw shared library (.so / .dylib / .dll)
    #[arg(short = 'l', long = "library-file")]
    library_file: PathBuf,

    /// Extension name
    #[arg(short = 'n', long = "extension-name")]
    extension_name: String,

    /// Output file path (defaults to <extension-name>.duckdb_extension)
    #[arg(short = 'o', long = "out-file")]
    out_file: Option<PathBuf>,

    /// DuckDB platform string (e.g., linux_amd64, osx_arm64)
    #[arg(short = 'p', long = "duckdb-platform")]
    duckdb_platform: String,

    /// DuckDB version (e.g., v1.2.0)
    #[arg(long = "duckdb-version", alias = "dv")]
    duckdb_version: String,

    /// Extension version (e.g., 0.5.1)
    #[arg(long = "extension-version", alias = "ev")]
    extension_version: String,

    /// ABI type (default: C_STRUCT)
    #[arg(long = "abi-type", default_value = "C_STRUCT")]
    abi_type: String,
}

fn main() {
    let args = Args::parse();

    let output = args
        .out_file
        .unwrap_or_else(|| PathBuf::from(format!("{}.duckdb_extension", args.extension_name)));

    eprintln!("Creating extension binary:");
    eprintln!(" - Input file: {}", args.library_file.display());
    eprintln!(" - Output file: {}", output.display());
    eprintln!(" - Metadata:");
    eprintln!("   - platform          = {}", args.duckdb_platform);
    eprintln!("   - duckdb_version    = {}", args.duckdb_version);
    eprintln!("   - extension_version = {}", args.extension_version);
    eprintln!("   - abi_type          = {}", args.abi_type);

    if let Err(e) = finetype_build_tools::append_metadata(
        &args.library_file,
        &output,
        &args.duckdb_platform,
        &args.duckdb_version,
        &args.extension_version,
        &args.abi_type,
    ) {
        eprintln!("Error: {e}");
        process::exit(1);
    }

    eprintln!("Done.");
}
