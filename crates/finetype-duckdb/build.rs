//! Build script for finetype_duckdb.
//!
//! Models are downloaded at runtime from HuggingFace Hub (or loaded from a local
//! path via `FINETYPE_MODEL_DIR`). No model files are needed at build time.

fn main() {
    // Nothing to do — model loading is handled at runtime.
    // This file exists because Cargo expects it when build.rs was previously present.
}
