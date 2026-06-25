//! FineType Core
//!
//! Core library for precision format detection taxonomy and data generation.
//!
//! - `taxonomy` — domain.category.type label format with transformation contracts
//! - `generator` — synthetic data generation for all 151 types
//! - `checker` — taxonomy ↔ generator alignment validation
//! - `tokenizer` — text tokenization for model training

pub mod checker;
pub mod checksum;
pub mod datetime_format;
pub mod enum_domain;
pub mod fast_path;
pub mod generator;
pub mod infer;
pub mod json_reader;
pub mod locale_data;
pub mod quality;
pub mod table_validator;
pub mod taxonomy;
pub mod tokenizer;
pub mod validation_veto;
pub mod validator;

pub use checker::{format_report, CheckReport, Checker};
pub use fast_path::deterministic_fast_path;
pub use generator::{Generator, Sample};
pub use json_reader::{collect_json, collect_ndjson, JsonPathMap};
pub use quality::{
    compute_column_quality, compute_file_grade, ColumnQualityScore, FileQualityGrade,
};
pub use table_validator::{
    split_rows, validate_table, CellError, ColumnValidationStats, RowErrors, TableValidationResult,
    TableValidatorError,
};
#[cfg(feature = "embedded-taxonomy")]
pub use taxonomy::frictionless_for;
pub use taxonomy::{
    DdlInfo, Definition, Designation, Frictionless, Label, Taxonomy, TierGraph, TierGraphSummary,
    Validation, FRICTIONLESS_TYPES,
};
pub use tokenizer::Tokenizer;
pub use validation_veto::{
    audited_safe_labels, evaluate_validation_veto, veto_shape_fallback, ValidationVeto,
    VETO_THRESHOLD,
};
pub use validator::{
    validate_column, validate_column_for_label, validate_value, validate_value_for_label,
    ColumnStats, ColumnValidationResult, CompiledValidator, InvalidStrategy, QuarantinedValue,
    ValidationCheck, ValidationError, ValidationResult, ValidatorError,
};
