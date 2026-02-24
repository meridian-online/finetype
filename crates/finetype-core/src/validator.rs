//! Validation engine for finetype labels.
//!
//! Validates string values against the JSON Schema fragment stored in each
//! type definition's `validation` field. Supports column-level validation
//! with configurable strategies for handling invalid data.
//!
//! ## Validation modes
//!
//! **`CompiledValidator`** — Pre-compiles the JSON Schema once and validates
//! many values without re-compilation. Use for hot paths (attractor demotion,
//! column validation). Backed by the `jsonschema` crate.
//!
//! **`validate_value()`** — Standalone function that compiles per-call.
//! Preserved for backwards compatibility; use `CompiledValidator` in hot paths.

use crate::taxonomy::{Taxonomy, Validation};
use std::collections::HashMap;
use thiserror::Error;

// ═══════════════════════════════════════════════════════════════════════════════
// ERRORS
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Error, Debug)]
pub enum ValidatorError {
    #[error("Unknown label: {0}")]
    UnknownLabel(String),
    #[error("No validation schema for label: {0}")]
    NoSchema(String),
    #[error("Invalid regex pattern for label {label}: {detail}")]
    InvalidPattern { label: String, detail: String },
    #[error("Failed to compile JSON Schema for label {label}: {detail}")]
    SchemaCompilationError { label: String, detail: String },
}

// ═══════════════════════════════════════════════════════════════════════════════
// COMPILED VALIDATOR (pre-compiled JSON Schema)
// ═══════════════════════════════════════════════════════════════════════════════

/// A pre-compiled validator for a single type definition.
///
/// Compiles the JSON Schema once; validates many values without re-compilation.
/// String-applicable keywords (`pattern`, `minLength`, `maxLength`, `enum`) are
/// delegated to the jsonschema crate. Numeric bounds (`minimum`, `maximum`) are
/// handled manually because JSON Schema applies them to JSON numbers, but we
/// validate string representations of numbers (parsing with `f64::from_str`).
pub struct CompiledValidator {
    /// Pre-compiled JSON Schema validator (pattern, length, enum checks).
    schema_validator: jsonschema::Validator,
    /// Manual numeric minimum bound (string→f64 parsing).
    minimum: Option<f64>,
    /// Manual numeric maximum bound (string→f64 parsing).
    maximum: Option<f64>,
}

impl CompiledValidator {
    /// Compile a new validator from a `Validation` schema fragment.
    ///
    /// Converts the validation to JSON Schema, pre-compiles with jsonschema-rs,
    /// and stores numeric bounds for manual checking.
    pub fn new(validation: &Validation) -> Result<Self, ValidatorError> {
        let json_schema = validation.to_json_schema();
        let schema_validator = jsonschema::validator_for(&json_schema).map_err(|e| {
            ValidatorError::SchemaCompilationError {
                label: String::new(),
                detail: e.to_string(),
            }
        })?;
        Ok(Self {
            schema_validator,
            minimum: validation.minimum,
            maximum: validation.maximum,
        })
    }

    /// Compile a new validator with a label for error context.
    pub fn new_for_label(validation: &Validation, label: &str) -> Result<Self, ValidatorError> {
        let json_schema = validation.to_json_schema();
        let schema_validator = jsonschema::validator_for(&json_schema).map_err(|e| {
            ValidatorError::SchemaCompilationError {
                label: label.to_string(),
                detail: e.to_string(),
            }
        })?;
        Ok(Self {
            schema_validator,
            minimum: validation.minimum,
            maximum: validation.maximum,
        })
    }

    /// Fast boolean-only validation for hot loops.
    ///
    /// Returns `true` if the value passes all checks (JSON Schema + numeric bounds).
    pub fn is_valid(&self, value: &str) -> bool {
        let json_value = serde_json::Value::String(value.to_string());
        if !self.schema_validator.is_valid(&json_value) {
            return false;
        }
        // Manual numeric bounds (preserves string→f64 parsing semantics)
        if let Some(minimum) = self.minimum {
            if let Ok(num) = value.parse::<f64>() {
                if num < minimum {
                    return false;
                }
            }
            // Non-numeric string silently passes (matches bespoke behaviour)
        }
        if let Some(maximum) = self.maximum {
            if let Ok(num) = value.parse::<f64>() {
                if num > maximum {
                    return false;
                }
            }
        }
        true
    }

    /// Full validation with detailed error reporting.
    ///
    /// Returns a `ValidationResult` with all errors collected, matching the
    /// same structure as the standalone `validate_value()` function.
    pub fn validate(&self, value: &str) -> ValidationResult {
        let mut errors = Vec::new();
        let json_value = serde_json::Value::String(value.to_string());

        // Collect jsonschema errors and map to our ValidationCheck types
        for error in self.schema_validator.iter_errors(&json_value) {
            let check = match error.kind() {
                jsonschema::error::ValidationErrorKind::Pattern { .. } => ValidationCheck::Pattern,
                jsonschema::error::ValidationErrorKind::MinLength { .. } => {
                    ValidationCheck::MinLength
                }
                jsonschema::error::ValidationErrorKind::MaxLength { .. } => {
                    ValidationCheck::MaxLength
                }
                jsonschema::error::ValidationErrorKind::Enum { .. } => ValidationCheck::Enum,
                _ => {
                    // Map any other jsonschema error to the most relevant check.
                    ValidationCheck::Pattern
                }
            };
            errors.push(ValidationError {
                check,
                message: error.to_string(),
            });
        }

        // Manual numeric bounds (same semantics as bespoke validator)
        if let Some(minimum) = self.minimum {
            if let Ok(num) = value.parse::<f64>() {
                if num < minimum {
                    errors.push(ValidationError {
                        check: ValidationCheck::Minimum,
                        message: format!("Value {} is less than minimum {}", num, minimum),
                    });
                }
            }
        }
        if let Some(maximum) = self.maximum {
            if let Ok(num) = value.parse::<f64>() {
                if num > maximum {
                    errors.push(ValidationError {
                        check: ValidationCheck::Maximum,
                        message: format!("Value {} exceeds maximum {}", num, maximum),
                    });
                }
            }
        }

        ValidationResult {
            is_valid: errors.is_empty(),
            errors,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// VALIDATION RESULT (single value)
// ═══════════════════════════════════════════════════════════════════════════════

/// Result of validating a single value.
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether the value is valid.
    pub is_valid: bool,
    /// List of validation errors (empty if valid).
    pub errors: Vec<ValidationError>,
}

/// A single validation error with detail.
#[derive(Debug, Clone)]
pub struct ValidationError {
    /// Which check failed.
    pub check: ValidationCheck,
    /// Human-readable message.
    pub message: String,
}

/// The type of validation check that was performed.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ValidationCheck {
    Pattern,
    MinLength,
    MaxLength,
    Minimum,
    Maximum,
    Enum,
}

impl std::fmt::Display for ValidationCheck {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pattern => write!(f, "pattern"),
            Self::MinLength => write!(f, "minLength"),
            Self::MaxLength => write!(f, "maxLength"),
            Self::Minimum => write!(f, "minimum"),
            Self::Maximum => write!(f, "maximum"),
            Self::Enum => write!(f, "enum"),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// SINGLE-VALUE VALIDATION
// ═══════════════════════════════════════════════════════════════════════════════

/// Validate a single value against a Validation schema fragment.
///
/// Compiles a `CompiledValidator` per call. For hot paths (column validation,
/// attractor demotion), use `CompiledValidator::is_valid()` or the taxonomy's
/// cached validators instead.
///
/// Checks all applicable fields: pattern, minLength, maxLength, minimum, maximum, enum.
/// Returns a ValidationResult with all errors collected.
pub fn validate_value(
    value: &str,
    schema: &Validation,
) -> Result<ValidationResult, ValidatorError> {
    let compiled = CompiledValidator::new(schema)?;
    Ok(compiled.validate(value))
}

/// Validate a value against a label's schema from the taxonomy.
///
/// Uses the taxonomy's cached `CompiledValidator` if available (populated via
/// `taxonomy.compile_validators()`). Falls back to compile-per-call otherwise.
pub fn validate_value_for_label(
    value: &str,
    label: &str,
    taxonomy: &Taxonomy,
) -> Result<ValidationResult, ValidatorError> {
    // Fast path: use cached compiled validator
    if let Some(compiled) = taxonomy.get_validator(label) {
        return Ok(compiled.validate(value));
    }

    // Slow path: look up definition and compile per-call
    let definition = taxonomy
        .get(label)
        .ok_or_else(|| ValidatorError::UnknownLabel(label.to_string()))?;

    let schema = definition
        .validation
        .as_ref()
        .ok_or_else(|| ValidatorError::NoSchema(label.to_string()))?;

    validate_value(value, schema)
}

// ═══════════════════════════════════════════════════════════════════════════════
// COLUMN VALIDATION WITH STRATEGIES
// ═══════════════════════════════════════════════════════════════════════════════

/// Strategy for handling invalid values during column validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InvalidStrategy {
    /// Collect invalid values separately for review (default).
    #[default]
    Quarantine,
    /// Replace invalid values with NULL.
    SetNull,
    /// Replace invalid values with the last valid value.
    ForwardFill,
    /// Replace invalid values with the next valid value.
    BackwardFill,
}

/// A quarantined invalid value with context.
#[derive(Debug, Clone)]
pub struct QuarantinedValue {
    /// Row index (0-based).
    pub row_index: usize,
    /// The original value.
    pub value: String,
    /// Validation errors for this value.
    pub errors: Vec<ValidationError>,
}

/// Statistics from column validation.
#[derive(Debug, Clone)]
pub struct ColumnStats {
    /// Number of valid values.
    pub valid_count: usize,
    /// Number of invalid values.
    pub invalid_count: usize,
    /// Number of NULL values.
    pub null_count: usize,
    /// Total number of values.
    pub total_count: usize,
    /// Error pattern summary: check type → count of failures.
    pub error_patterns: HashMap<ValidationCheck, usize>,
}

impl ColumnStats {
    /// Percentage of valid (non-null) values.
    pub fn validity_rate(&self) -> f64 {
        let non_null = self.total_count - self.null_count;
        if non_null == 0 {
            return 0.0;
        }
        self.valid_count as f64 / non_null as f64
    }
}

/// Result of validating a column of values.
#[derive(Debug, Clone)]
pub struct ColumnValidationResult {
    /// The output values after applying the strategy.
    /// None represents NULL.
    pub values: Vec<Option<String>>,
    /// Validation statistics.
    pub stats: ColumnStats,
    /// Quarantined values (only populated in Quarantine mode).
    pub quarantined: Vec<QuarantinedValue>,
}

/// Validate a column of values against a schema with a specified strategy.
///
/// Compiles the JSON Schema once at the top of the function, then validates
/// each value against the compiled validator. This is significantly faster
/// than the previous approach which recompiled per value.
///
/// Each value is `Option<&str>` where None represents NULL.
pub fn validate_column(
    values: &[Option<&str>],
    schema: &Validation,
    strategy: InvalidStrategy,
) -> Result<ColumnValidationResult, ValidatorError> {
    // Compile once, validate many
    let compiled = CompiledValidator::new(schema)?;

    let total_count = values.len();
    let mut valid_count = 0;
    let mut invalid_count = 0;
    let mut null_count = 0;
    let mut error_patterns: HashMap<ValidationCheck, usize> = HashMap::new();
    let mut quarantined: Vec<QuarantinedValue> = Vec::new();

    // First pass: validate all values using pre-compiled validator
    let mut validation_results: Vec<Option<ValidationResult>> = Vec::with_capacity(total_count);
    for value in values {
        match value {
            None => {
                null_count += 1;
                validation_results.push(None);
            }
            Some(v) => {
                let result = compiled.validate(v);
                if result.is_valid {
                    valid_count += 1;
                } else {
                    invalid_count += 1;
                    for error in &result.errors {
                        *error_patterns.entry(error.check.clone()).or_insert(0) += 1;
                    }
                }
                validation_results.push(Some(result));
            }
        }
    }

    // Second pass: apply strategy to produce output values
    let output_values = apply_strategy(values, &validation_results, strategy, &mut quarantined);

    Ok(ColumnValidationResult {
        values: output_values,
        stats: ColumnStats {
            valid_count,
            invalid_count,
            null_count,
            total_count,
            error_patterns,
        },
        quarantined,
    })
}

/// Apply an invalid-value strategy to produce output values (shared helper).
fn apply_strategy(
    values: &[Option<&str>],
    validation_results: &[Option<ValidationResult>],
    strategy: InvalidStrategy,
    quarantined: &mut Vec<QuarantinedValue>,
) -> Vec<Option<String>> {
    let total_count = values.len();
    match strategy {
        InvalidStrategy::Quarantine => {
            let mut output = Vec::with_capacity(total_count);
            for (i, (value, result)) in values.iter().zip(validation_results.iter()).enumerate() {
                match (value, result) {
                    (None, _) => output.push(None),
                    (Some(v), Some(r)) if !r.is_valid => {
                        quarantined.push(QuarantinedValue {
                            row_index: i,
                            value: v.to_string(),
                            errors: r.errors.clone(),
                        });
                        output.push(None);
                    }
                    (Some(v), _) => output.push(Some(v.to_string())),
                }
            }
            output
        }
        InvalidStrategy::SetNull => {
            let mut output = Vec::with_capacity(total_count);
            for (value, result) in values.iter().zip(validation_results.iter()) {
                match (value, result) {
                    (None, _) => output.push(None),
                    (Some(_), Some(r)) if !r.is_valid => output.push(None),
                    (Some(v), _) => output.push(Some(v.to_string())),
                }
            }
            output
        }
        InvalidStrategy::ForwardFill => {
            let mut output = Vec::with_capacity(total_count);
            let mut last_valid: Option<String> = None;
            for (value, result) in values.iter().zip(validation_results.iter()) {
                match (value, result) {
                    (None, _) => output.push(None),
                    (Some(v), Some(r)) if r.is_valid => {
                        last_valid = Some(v.to_string());
                        output.push(Some(v.to_string()));
                    }
                    (Some(_), Some(_)) => {
                        output.push(last_valid.clone());
                    }
                    (Some(v), None) => {
                        output.push(Some(v.to_string()));
                    }
                }
            }
            output
        }
        InvalidStrategy::BackwardFill => {
            let mut output: Vec<Option<String>> = vec![None; total_count];
            let mut next_valid: Option<String> = None;
            for i in (0..total_count).rev() {
                match (&values[i], &validation_results[i]) {
                    (None, _) => {
                        output[i] = None;
                    }
                    (Some(v), Some(r)) if r.is_valid => {
                        next_valid = Some(v.to_string());
                        output[i] = Some(v.to_string());
                    }
                    (Some(_), Some(_)) => {
                        output[i] = next_valid.clone();
                    }
                    (Some(v), None) => {
                        output[i] = Some(v.to_string());
                    }
                }
            }
            output
        }
    }
}

/// Validate a column of values against a label's schema from the taxonomy.
///
/// Uses the taxonomy's cached `CompiledValidator` if available. Falls back
/// to compiling from the definition's schema.
pub fn validate_column_for_label(
    values: &[Option<&str>],
    label: &str,
    taxonomy: &Taxonomy,
    strategy: InvalidStrategy,
) -> Result<ColumnValidationResult, ValidatorError> {
    // Fast path: use cached compiled validator
    if let Some(compiled) = taxonomy.get_validator(label) {
        return validate_column_with_compiled(values, compiled, strategy);
    }

    // Slow path: look up definition and compile
    let definition = taxonomy
        .get(label)
        .ok_or_else(|| ValidatorError::UnknownLabel(label.to_string()))?;

    let schema = definition
        .validation
        .as_ref()
        .ok_or_else(|| ValidatorError::NoSchema(label.to_string()))?;

    validate_column(values, schema, strategy)
}

/// Validate a column using a pre-compiled validator (internal helper).
fn validate_column_with_compiled(
    values: &[Option<&str>],
    compiled: &CompiledValidator,
    strategy: InvalidStrategy,
) -> Result<ColumnValidationResult, ValidatorError> {
    let total_count = values.len();
    let mut valid_count = 0;
    let mut invalid_count = 0;
    let mut null_count = 0;
    let mut error_patterns: HashMap<ValidationCheck, usize> = HashMap::new();
    let mut quarantined: Vec<QuarantinedValue> = Vec::new();

    let mut validation_results: Vec<Option<ValidationResult>> = Vec::with_capacity(total_count);
    for value in values {
        match value {
            None => {
                null_count += 1;
                validation_results.push(None);
            }
            Some(v) => {
                let result = compiled.validate(v);
                if result.is_valid {
                    valid_count += 1;
                } else {
                    invalid_count += 1;
                    for error in &result.errors {
                        *error_patterns.entry(error.check.clone()).or_insert(0) += 1;
                    }
                }
                validation_results.push(Some(result));
            }
        }
    }

    // Apply strategy (same logic as validate_column)
    let output_values = apply_strategy(values, &validation_results, strategy, &mut quarantined);

    Ok(ColumnValidationResult {
        values: output_values,
        stats: ColumnStats {
            valid_count,
            invalid_count,
            null_count,
            total_count,
            error_patterns,
        },
        quarantined,
    })
}

// ═══════════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    fn ip_schema() -> Validation {
        Validation {
            schema_type: Some("string".to_string()),
            pattern: Some(
                r"^(?:(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)$"
                    .to_string(),
            ),
            min_length: Some(7),
            max_length: Some(15),
            minimum: None,
            maximum: None,
            enum_values: None,
        }
    }

    fn boolean_schema() -> Validation {
        Validation {
            schema_type: Some("string".to_string()),
            pattern: None,
            min_length: None,
            max_length: None,
            minimum: None,
            maximum: None,
            enum_values: Some(vec![
                "true".to_string(),
                "false".to_string(),
                "True".to_string(),
                "False".to_string(),
                "TRUE".to_string(),
                "FALSE".to_string(),
                "yes".to_string(),
                "no".to_string(),
                "0".to_string(),
                "1".to_string(),
            ]),
        }
    }

    fn port_schema() -> Validation {
        Validation {
            schema_type: Some("string".to_string()),
            pattern: Some(r"^\d+$".to_string()),
            min_length: None,
            max_length: None,
            minimum: Some(0.0),
            maximum: Some(65535.0),
            enum_values: None,
        }
    }

    // ── Single-value validation tests ────────────────────────────────────

    #[test]
    fn test_valid_ipv4() {
        let result = validate_value("192.168.1.1", &ip_schema()).unwrap();
        assert!(result.is_valid);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_invalid_ipv4_pattern() {
        let result = validate_value("999.999.999.999", &ip_schema()).unwrap();
        assert!(!result.is_valid);
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0].check, ValidationCheck::Pattern);
    }

    #[test]
    fn test_invalid_ipv4_too_short() {
        let result = validate_value("1.1.1", &ip_schema()).unwrap();
        assert!(!result.is_valid);
        // Should fail both pattern and minLength
        let checks: Vec<&ValidationCheck> = result.errors.iter().map(|e| &e.check).collect();
        assert!(checks.contains(&&ValidationCheck::Pattern));
        assert!(checks.contains(&&ValidationCheck::MinLength));
    }

    #[test]
    fn test_valid_boolean_enum() {
        let result = validate_value("true", &boolean_schema()).unwrap();
        assert!(result.is_valid);
    }

    #[test]
    fn test_invalid_boolean_enum() {
        let result = validate_value("maybe", &boolean_schema()).unwrap();
        assert!(!result.is_valid);
        assert_eq!(result.errors[0].check, ValidationCheck::Enum);
    }

    #[test]
    fn test_valid_port() {
        let result = validate_value("8080", &port_schema()).unwrap();
        assert!(result.is_valid);
    }

    #[test]
    fn test_invalid_port_too_high() {
        let result = validate_value("70000", &port_schema()).unwrap();
        assert!(!result.is_valid);
        assert_eq!(result.errors[0].check, ValidationCheck::Maximum);
    }

    #[test]
    fn test_no_constraints_always_valid() {
        let empty_schema = Validation {
            schema_type: Some("string".to_string()),
            pattern: None,
            min_length: None,
            max_length: None,
            minimum: None,
            maximum: None,
            enum_values: None,
        };
        let result = validate_value("anything", &empty_schema).unwrap();
        assert!(result.is_valid);
    }

    // ── Column validation tests ──────────────────────────────────────────

    #[test]
    fn test_column_quarantine() {
        let values = vec![
            Some("192.168.1.1"),
            Some("10.0.0.1"),
            Some("not-an-ip"),
            None,
            Some("172.16.0.1"),
        ];
        let result = validate_column(&values, &ip_schema(), InvalidStrategy::Quarantine).unwrap();

        assert_eq!(result.stats.valid_count, 3);
        assert_eq!(result.stats.invalid_count, 1);
        assert_eq!(result.stats.null_count, 1);
        assert_eq!(result.stats.total_count, 5);
        assert_eq!(result.quarantined.len(), 1);
        assert_eq!(result.quarantined[0].row_index, 2);
        assert_eq!(result.quarantined[0].value, "not-an-ip");

        // Invalid row becomes None in output
        assert_eq!(result.values[2], None);
        // Valid rows preserved
        assert_eq!(result.values[0], Some("192.168.1.1".to_string()));
    }

    #[test]
    fn test_column_set_null() {
        let values = vec![Some("192.168.1.1"), Some("not-an-ip"), Some("10.0.0.1")];
        let result = validate_column(&values, &ip_schema(), InvalidStrategy::SetNull).unwrap();

        assert_eq!(result.stats.valid_count, 2);
        assert_eq!(result.stats.invalid_count, 1);
        assert_eq!(result.values[0], Some("192.168.1.1".to_string()));
        assert_eq!(result.values[1], None);
        assert_eq!(result.values[2], Some("10.0.0.1".to_string()));
    }

    #[test]
    fn test_column_forward_fill() {
        let values = vec![
            Some("192.168.1.1"),
            Some("not-an-ip"),
            Some("10.0.0.1"),
            Some("bad"),
        ];
        let result = validate_column(&values, &ip_schema(), InvalidStrategy::ForwardFill).unwrap();

        assert_eq!(result.values[0], Some("192.168.1.1".to_string()));
        assert_eq!(result.values[1], Some("192.168.1.1".to_string())); // ffill from [0]
        assert_eq!(result.values[2], Some("10.0.0.1".to_string()));
        assert_eq!(result.values[3], Some("10.0.0.1".to_string())); // ffill from [2]
    }

    #[test]
    fn test_column_backward_fill() {
        let values = vec![
            Some("not-an-ip"),
            Some("192.168.1.1"),
            Some("bad"),
            Some("10.0.0.1"),
        ];
        let result = validate_column(&values, &ip_schema(), InvalidStrategy::BackwardFill).unwrap();

        assert_eq!(result.values[0], Some("192.168.1.1".to_string())); // bfill from [1]
        assert_eq!(result.values[1], Some("192.168.1.1".to_string()));
        assert_eq!(result.values[2], Some("10.0.0.1".to_string())); // bfill from [3]
        assert_eq!(result.values[3], Some("10.0.0.1".to_string()));
    }

    #[test]
    fn test_column_ffill_no_prior_valid() {
        let values = vec![Some("bad"), Some("192.168.1.1")];
        let result = validate_column(&values, &ip_schema(), InvalidStrategy::ForwardFill).unwrap();

        // No prior valid value → None
        assert_eq!(result.values[0], None);
        assert_eq!(result.values[1], Some("192.168.1.1".to_string()));
    }

    #[test]
    fn test_column_bfill_no_next_valid() {
        let values = vec![Some("192.168.1.1"), Some("bad")];
        let result = validate_column(&values, &ip_schema(), InvalidStrategy::BackwardFill).unwrap();

        assert_eq!(result.values[0], Some("192.168.1.1".to_string()));
        // No next valid value → None
        assert_eq!(result.values[1], None);
    }

    #[test]
    fn test_column_stats_error_patterns() {
        let values = vec![
            Some("192.168.1.1"),
            Some("x"),      // fails pattern + minLength
            Some("not-ip"), // fails pattern + minLength
        ];
        let result = validate_column(&values, &ip_schema(), InvalidStrategy::Quarantine).unwrap();

        assert_eq!(result.stats.valid_count, 1);
        assert_eq!(result.stats.invalid_count, 2);
        assert_eq!(
            result.stats.error_patterns.get(&ValidationCheck::Pattern),
            Some(&2)
        );
        // Both "x" (len=1) and "not-ip" (len=6) are < minLength 7
        assert_eq!(
            result.stats.error_patterns.get(&ValidationCheck::MinLength),
            Some(&2)
        );
    }

    #[test]
    fn test_column_all_nulls() {
        let values: Vec<Option<&str>> = vec![None, None, None];
        let result = validate_column(&values, &ip_schema(), InvalidStrategy::Quarantine).unwrap();

        assert_eq!(result.stats.null_count, 3);
        assert_eq!(result.stats.valid_count, 0);
        assert_eq!(result.stats.invalid_count, 0);
    }

    #[test]
    fn test_validity_rate() {
        let stats = ColumnStats {
            valid_count: 8,
            invalid_count: 2,
            null_count: 5,
            total_count: 15,
            error_patterns: HashMap::new(),
        };
        // 8 valid out of 10 non-null = 80%
        assert!((stats.validity_rate() - 0.8).abs() < f64::EPSILON);
    }

    #[test]
    fn test_validity_rate_all_null() {
        let stats = ColumnStats {
            valid_count: 0,
            invalid_count: 0,
            null_count: 5,
            total_count: 5,
            error_patterns: HashMap::new(),
        };
        assert!((stats.validity_rate() - 0.0).abs() < f64::EPSILON);
    }

    // ── Taxonomy integration test ────────────────────────────────────────

    #[test]
    fn test_validate_with_taxonomy() {
        let yaml = r#"
datetime.timestamp.iso_8601:
  title: "ISO 8601"
  broad_type: TIMESTAMP
  validation:
    type: string
    pattern: "^\\d{4}-\\d{2}-\\d{2}T\\d{2}:\\d{2}:\\d{2}Z$"
    minLength: 20
    maxLength: 20
  tier: [TIMESTAMP, timestamp]
  release_priority: 5
  samples: ["2024-01-15T10:30:00Z"]
"#;
        let taxonomy = Taxonomy::from_yaml(yaml).unwrap();

        let result = validate_value_for_label(
            "2024-01-15T10:30:00Z",
            "datetime.timestamp.iso_8601",
            &taxonomy,
        )
        .unwrap();
        assert!(result.is_valid);

        let result =
            validate_value_for_label("not-a-timestamp", "datetime.timestamp.iso_8601", &taxonomy)
                .unwrap();
        assert!(!result.is_valid);
    }

    #[test]
    fn test_unknown_label_error() {
        let yaml = r#"
datetime.timestamp.iso_8601:
  title: "ISO 8601"
  validation:
    type: string
  tier: [TIMESTAMP, timestamp]
  samples: ["2024-01-15T10:30:00Z"]
"#;
        let taxonomy = Taxonomy::from_yaml(yaml).unwrap();
        let result = validate_value_for_label("test", "nonexistent.label", &taxonomy);
        assert!(matches!(result, Err(ValidatorError::UnknownLabel(_))));
    }

    // ── CompiledValidator tests (NNFT-116) ──────────────────────────────

    #[test]
    fn test_compiled_validator_pattern() {
        let compiled = CompiledValidator::new(&ip_schema()).unwrap();
        assert!(compiled.is_valid("192.168.1.1"));
        assert!(!compiled.is_valid("999.999.999.999"));
        assert!(!compiled.is_valid("not-an-ip"));
    }

    #[test]
    fn test_compiled_validator_length() {
        let schema = Validation {
            schema_type: Some("string".to_string()),
            pattern: None,
            min_length: Some(3),
            max_length: Some(10),
            minimum: None,
            maximum: None,
            enum_values: None,
        };
        let compiled = CompiledValidator::new(&schema).unwrap();
        assert!(compiled.is_valid("abc"));
        assert!(compiled.is_valid("abcdefghij")); // exactly 10
        assert!(!compiled.is_valid("ab")); // too short
        assert!(!compiled.is_valid("abcdefghijk")); // too long (11)
    }

    #[test]
    fn test_compiled_validator_enum() {
        let compiled = CompiledValidator::new(&boolean_schema()).unwrap();
        assert!(compiled.is_valid("true"));
        assert!(compiled.is_valid("false"));
        assert!(compiled.is_valid("0"));
        assert!(compiled.is_valid("1"));
        assert!(!compiled.is_valid("maybe"));
        assert!(!compiled.is_valid("TRUE "));
    }

    #[test]
    fn test_compiled_validator_numeric_bounds() {
        let compiled = CompiledValidator::new(&port_schema()).unwrap();
        assert!(compiled.is_valid("0"));
        assert!(compiled.is_valid("8080"));
        assert!(compiled.is_valid("65535"));
        assert!(!compiled.is_valid("70000")); // exceeds maximum
        assert!(!compiled.is_valid("-1")); // below minimum (pattern also fails)
    }

    #[test]
    fn test_compiled_validator_non_numeric_bounds_pass() {
        // Non-numeric strings silently pass minimum/maximum checks
        // (preserves bespoke behaviour: only pattern check should catch format)
        let schema = Validation {
            schema_type: Some("string".to_string()),
            pattern: None,
            min_length: None,
            max_length: None,
            minimum: Some(0.0),
            maximum: Some(100.0),
            enum_values: None,
        };
        let compiled = CompiledValidator::new(&schema).unwrap();
        assert!(compiled.is_valid("hello")); // non-numeric → passes min/max
        assert!(compiled.is_valid("50"));
        assert!(!compiled.is_valid("200")); // numeric → fails maximum
    }

    #[test]
    fn test_compiled_validator_is_valid_fast_path() {
        let compiled = CompiledValidator::new(&ip_schema()).unwrap();
        // is_valid returns bool without error detail — suitable for hot loops
        assert!(compiled.is_valid("10.0.0.1"));
        assert!(!compiled.is_valid("invalid"));
    }

    #[test]
    fn test_compiled_validator_detailed_errors() {
        let compiled = CompiledValidator::new(&ip_schema()).unwrap();
        let result = compiled.validate("x");
        assert!(!result.is_valid);
        let checks: Vec<&ValidationCheck> = result.errors.iter().map(|e| &e.check).collect();
        assert!(checks.contains(&&ValidationCheck::Pattern));
        assert!(checks.contains(&&ValidationCheck::MinLength));
    }

    #[test]
    fn test_taxonomy_validator_cache() {
        let yaml = r#"
datetime.timestamp.iso_8601:
  title: "ISO 8601"
  validation:
    type: string
    pattern: "^\\d{4}-\\d{2}-\\d{2}T\\d{2}:\\d{2}:\\d{2}Z$"
    minLength: 20
    maxLength: 20
  tier: [TIMESTAMP, timestamp]
  release_priority: 5
  samples: ["2024-01-15T10:30:00Z"]

technology.internet.ip_v4:
  title: "IPv4"
  validation:
    type: string
    pattern: "^\\d{1,3}\\.\\d{1,3}\\.\\d{1,3}\\.\\d{1,3}$"
  tier: [VARCHAR, internet]
  release_priority: 5
  samples: ["192.168.1.1"]
"#;
        let mut taxonomy = Taxonomy::from_yaml(yaml).unwrap();
        // Before compilation: no validators cached
        assert!(taxonomy
            .get_validator("datetime.timestamp.iso_8601")
            .is_none());
        assert_eq!(taxonomy.validator_count(), 0);

        // Compile
        taxonomy.compile_validators();
        assert_eq!(taxonomy.validator_count(), 2);

        // Cached validators work
        let iso = taxonomy
            .get_validator("datetime.timestamp.iso_8601")
            .unwrap();
        assert!(iso.is_valid("2024-01-15T10:30:00Z"));
        assert!(!iso.is_valid("not-a-timestamp"));

        let ipv4 = taxonomy.get_validator("technology.internet.ip_v4").unwrap();
        assert!(ipv4.is_valid("192.168.1.1"));
        // Note: YAML pattern is ^\d{1,3}\.\d{1,3}... which matches any 1-3 digit octets
        assert!(ipv4.is_valid("999.999.999.999")); // format match (not range-checked)
        assert!(!ipv4.is_valid("not-an-ip"));

        // Non-existent label returns None
        assert!(taxonomy.get_validator("nonexistent").is_none());
    }

    #[test]
    fn test_validate_value_unchanged_api() {
        // Existing validate_value() API produces identical results to before
        let result = validate_value("192.168.1.1", &ip_schema()).unwrap();
        assert!(result.is_valid);

        let result = validate_value("999.999.999.999", &ip_schema()).unwrap();
        assert!(!result.is_valid);
        assert_eq!(result.errors[0].check, ValidationCheck::Pattern);

        let result = validate_value("8080", &port_schema()).unwrap();
        assert!(result.is_valid);

        let result = validate_value("70000", &port_schema()).unwrap();
        assert!(!result.is_valid);
        assert_eq!(result.errors[0].check, ValidationCheck::Maximum);

        let result = validate_value("true", &boolean_schema()).unwrap();
        assert!(result.is_valid);

        let result = validate_value("maybe", &boolean_schema()).unwrap();
        assert!(!result.is_valid);
    }

    #[test]
    fn test_validate_value_for_label_with_cache() {
        let yaml = r#"
datetime.timestamp.iso_8601:
  title: "ISO 8601"
  validation:
    type: string
    pattern: "^\\d{4}-\\d{2}-\\d{2}T\\d{2}:\\d{2}:\\d{2}Z$"
    minLength: 20
    maxLength: 20
  tier: [TIMESTAMP, timestamp]
  release_priority: 5
  samples: ["2024-01-15T10:30:00Z"]
"#;
        let mut taxonomy = Taxonomy::from_yaml(yaml).unwrap();
        taxonomy.compile_validators();

        // Should use cached path
        let result = validate_value_for_label(
            "2024-01-15T10:30:00Z",
            "datetime.timestamp.iso_8601",
            &taxonomy,
        )
        .unwrap();
        assert!(result.is_valid);

        let result =
            validate_value_for_label("not-a-timestamp", "datetime.timestamp.iso_8601", &taxonomy)
                .unwrap();
        assert!(!result.is_valid);
    }

    #[test]
    fn test_to_json_schema_structure() {
        let schema = Validation {
            schema_type: Some("string".to_string()),
            pattern: Some(r"^\d+$".to_string()),
            min_length: Some(1),
            max_length: Some(5),
            minimum: Some(0.0),
            maximum: Some(99999.0),
            enum_values: None,
        };
        let json = schema.to_json_schema();
        let obj = json.as_object().unwrap();
        assert_eq!(obj.get("type").unwrap(), "string");
        assert_eq!(obj.get("pattern").unwrap(), r"^\d+$");
        assert_eq!(obj.get("minLength").unwrap(), 1);
        assert_eq!(obj.get("maxLength").unwrap(), 5);
        // minimum/maximum deliberately excluded from JSON Schema
        assert!(obj.get("minimum").is_none());
        assert!(obj.get("maximum").is_none());
    }
}
