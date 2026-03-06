//! FineType Core
//!
//! Core library for precision format detection taxonomy and data generation.
//!
//! - `taxonomy` — domain.category.type label format with transformation contracts
//! - `generator` — synthetic data generation for all 151 types
//! - `checker` — taxonomy ↔ generator alignment validation
//! - `tokenizer` — text tokenization for model training

pub mod checker;
pub mod generator;
pub mod json_reader;
pub mod locale_data;
pub mod quality;
pub mod taxonomy;
pub mod tokenizer;
pub mod validator;

pub use checker::{format_report, CheckReport, Checker};
pub use generator::{Generator, Sample};
pub use json_reader::{collect_json, collect_ndjson, JsonPathMap};
pub use quality::{
    compute_column_quality, compute_file_grade, ColumnQualityScore, FileQualityGrade,
};
pub use taxonomy::{
    Definition, Designation, Label, Taxonomy, TierGraph, TierGraphSummary, Validation,
};
pub use tokenizer::Tokenizer;
pub use validator::{
    validate_column, validate_column_for_label, validate_value, validate_value_for_label,
    ColumnStats, ColumnValidationResult, CompiledValidator, InvalidStrategy, QuarantinedValue,
    ValidationCheck, ValidationError, ValidationResult, ValidatorError,
};
