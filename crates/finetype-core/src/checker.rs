//! Taxonomy ↔ Generator alignment checker.
//!
//! Validates that:
//! 1. Every YAML definition key has a working generator
//! 2. Every generated sample passes the definition's validation constraints
//! 3. Samples respect minLength, maxLength bounds
//!
//! This is a data quality gate — run before training to catch generator bugs.

use crate::generator::Generator;
use crate::taxonomy::{Definition, Taxonomy};
use crate::validator::CompiledValidator;
use std::collections::BTreeMap;
use std::fmt;

/// Result of checking a single definition.
#[derive(Debug, Clone)]
pub struct DefinitionCheckResult {
    /// The definition key (e.g., "datetime.timestamp.iso_8601")
    pub key: String,
    /// Whether the generator produced any output (vs NotImplemented error)
    pub generator_exists: bool,
    /// Number of samples generated
    pub samples_generated: usize,
    /// Number of samples that passed all validation checks
    pub samples_passed: usize,
    /// Number of samples that failed validation
    pub samples_failed: usize,
    /// Whether the definition has a validation pattern
    pub has_pattern: bool,
    /// Individual failure details (capped to avoid flooding)
    pub failures: Vec<CheckFailure>,
    /// Release priority of this definition
    pub release_priority: u8,
    /// Domain for grouping
    pub domain: String,
}

impl DefinitionCheckResult {
    /// Whether this definition fully passed all checks.
    pub fn passed(&self) -> bool {
        self.generator_exists && self.samples_failed == 0
    }

    /// Pass rate as a fraction (0.0 to 1.0).
    pub fn pass_rate(&self) -> f64 {
        if self.samples_generated == 0 {
            return 0.0;
        }
        self.samples_passed as f64 / self.samples_generated as f64
    }
}

/// A single validation failure.
#[derive(Debug, Clone)]
pub struct CheckFailure {
    pub sample: String,
    pub reason: FailureReason,
}

/// Why a sample failed validation.
#[derive(Debug, Clone)]
pub enum FailureReason {
    /// Sample didn't match the validation regex pattern
    PatternMismatch { pattern: String },
    /// Sample was shorter than minLength
    TooShort { actual: usize, min: u32 },
    /// Sample was longer than maxLength
    TooLong { actual: usize, max: u32 },
    /// Generator returned an error
    GeneratorError { message: String },
}

impl fmt::Display for FailureReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FailureReason::PatternMismatch { pattern } => {
                write!(f, "pattern mismatch (expected: {})", pattern)
            }
            FailureReason::TooShort { actual, min } => {
                write!(f, "too short ({} < {})", actual, min)
            }
            FailureReason::TooLong { actual, max } => {
                write!(f, "too long ({} > {})", actual, max)
            }
            FailureReason::GeneratorError { message } => {
                write!(f, "generator error: {}", message)
            }
        }
    }
}

/// Aggregate results for a full check run.
#[derive(Debug)]
pub struct CheckReport {
    /// Per-definition results, sorted by key
    pub results: Vec<DefinitionCheckResult>,
    /// Total definitions in taxonomy
    pub total_definitions: usize,
    /// Definitions with working generators
    pub generators_found: usize,
    /// Definitions missing generators
    pub generators_missing: usize,
    /// Definitions where all samples passed validation
    pub fully_passing: usize,
    /// Definitions with at least one validation failure
    pub has_failures: usize,
    /// Definitions with no validation pattern (untestable)
    pub no_pattern: usize,
    /// Total samples generated
    pub total_samples: usize,
    /// Total samples passing
    pub total_passed: usize,
    /// Total samples failing
    pub total_failed: usize,
}

impl CheckReport {
    /// Build aggregate stats from individual results.
    pub fn from_results(results: Vec<DefinitionCheckResult>, total_definitions: usize) -> Self {
        let generators_found = results.iter().filter(|r| r.generator_exists).count();
        let generators_missing = results.iter().filter(|r| !r.generator_exists).count();
        let fully_passing = results.iter().filter(|r| r.passed()).count();
        let has_failures = results
            .iter()
            .filter(|r| r.generator_exists && r.samples_failed > 0)
            .count();
        let no_pattern = results.iter().filter(|r| !r.has_pattern).count();
        let total_samples: usize = results.iter().map(|r| r.samples_generated).sum();
        let total_passed: usize = results.iter().map(|r| r.samples_passed).sum();
        let total_failed: usize = results.iter().map(|r| r.samples_failed).sum();

        Self {
            results,
            total_definitions,
            generators_found,
            generators_missing,
            fully_passing,
            has_failures,
            no_pattern,
            total_samples,
            total_passed,
            total_failed,
        }
    }

    /// Overall pass rate.
    pub fn pass_rate(&self) -> f64 {
        if self.total_samples == 0 {
            return 0.0;
        }
        self.total_passed as f64 / self.total_samples as f64
    }

    /// Whether the entire check passed (all generators exist, all validations pass).
    pub fn all_passed(&self) -> bool {
        self.generators_missing == 0 && self.has_failures == 0
    }

    /// Get results grouped by domain.
    pub fn by_domain(&self) -> BTreeMap<String, Vec<&DefinitionCheckResult>> {
        let mut map: BTreeMap<String, Vec<&DefinitionCheckResult>> = BTreeMap::new();
        for result in &self.results {
            map.entry(result.domain.clone()).or_default().push(result);
        }
        map
    }

    /// Get only failing results.
    pub fn failures(&self) -> Vec<&DefinitionCheckResult> {
        self.results.iter().filter(|r| !r.passed()).collect()
    }

    /// Get results at or above a priority level.
    pub fn at_priority(&self, min_priority: u8) -> Vec<&DefinitionCheckResult> {
        self.results
            .iter()
            .filter(|r| r.release_priority >= min_priority)
            .collect()
    }
}

/// Checker that validates generator ↔ taxonomy alignment.
pub struct Checker {
    samples_per_key: usize,
    max_failures_per_key: usize,
    seed: u64,
}

impl Checker {
    /// Create a new checker.
    pub fn new(samples_per_key: usize) -> Self {
        Self {
            samples_per_key,
            max_failures_per_key: 5,
            seed: 42,
        }
    }

    /// Set the random seed for reproducibility.
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }

    /// Set the maximum number of failure details to keep per key.
    pub fn with_max_failures(mut self, max: usize) -> Self {
        self.max_failures_per_key = max;
        self
    }

    /// Run the full check against the taxonomy.
    pub fn run(&self, taxonomy: &Taxonomy) -> CheckReport {
        let mut generator = Generator::with_seed(taxonomy.clone(), self.seed);
        let mut results = Vec::new();

        let mut keys: Vec<String> = taxonomy.labels().to_vec();
        keys.sort();

        for key in &keys {
            let definition = taxonomy.get(key);
            let result = self.check_definition(key, definition, &mut generator);
            results.push(result);
        }

        CheckReport::from_results(results, taxonomy.len())
    }

    /// Check a single definition.
    fn check_definition(
        &self,
        key: &str,
        definition: Option<&Definition>,
        generator: &mut Generator,
    ) -> DefinitionCheckResult {
        let parts: Vec<&str> = key.split('.').collect();
        let domain = parts.first().copied().unwrap_or("unknown").to_string();

        let (has_pattern, release_priority) = match definition {
            Some(def) => {
                let has_pat = def
                    .validation
                    .as_ref()
                    .and_then(|v| v.pattern.as_ref())
                    .is_some();
                (has_pat, def.release_priority)
            }
            None => (false, 0),
        };

        // Pre-compile validator once for this definition (NNFT-116)
        let compiled = definition
            .and_then(|d| d.validation.as_ref())
            .and_then(|v| CompiledValidator::new_for_label(v, key).ok());

        let mut samples_generated = 0usize;
        let mut samples_passed = 0usize;
        let mut samples_failed = 0usize;
        let mut failures = Vec::new();
        let mut generator_exists = false;

        for _ in 0..self.samples_per_key {
            match generator.generate_value(key) {
                Ok(sample) => {
                    generator_exists = true;
                    samples_generated += 1;

                    match self.validate_sample(&sample, definition, compiled.as_ref()) {
                        Ok(()) => {
                            samples_passed += 1;
                        }
                        Err(reason) => {
                            samples_failed += 1;
                            if failures.len() < self.max_failures_per_key {
                                failures.push(CheckFailure {
                                    sample: sample.clone(),
                                    reason,
                                });
                            }
                        }
                    }
                }
                Err(e) => {
                    // First error tells us whether generator exists
                    let msg = e.to_string();
                    if msg.contains("not implemented") || msg.contains("Unknown label") {
                        // Generator doesn't exist for this key
                        // generator_exists stays false
                    } else {
                        // Generator exists but errored
                        generator_exists = true;
                        samples_generated += 1;
                        samples_failed += 1;
                        if failures.len() < self.max_failures_per_key {
                            failures.push(CheckFailure {
                                sample: String::new(),
                                reason: FailureReason::GeneratorError { message: msg },
                            });
                        }
                    }
                }
            }
        }

        DefinitionCheckResult {
            key: key.to_string(),
            generator_exists,
            samples_generated,
            samples_passed,
            samples_failed,
            has_pattern,
            failures,
            release_priority,
            domain,
        }
    }

    /// Validate a single sample against its definition's constraints.
    ///
    /// Uses `CompiledValidator` for JSON Schema-compliant validation, then
    /// maps errors back to `FailureReason` variants for reporting.
    fn validate_sample(
        &self,
        sample: &str,
        definition: Option<&Definition>,
        compiled: Option<&CompiledValidator>,
    ) -> Result<(), FailureReason> {
        let def = match definition {
            Some(d) => d,
            None => return Ok(()), // No definition to validate against
        };

        let validation = match &def.validation {
            Some(v) => v,
            None => return Ok(()), // No validation constraints
        };

        // Use pre-compiled validator if available
        if let Some(compiled) = compiled {
            let result = compiled.validate(sample);
            if !result.is_valid {
                return Err(map_first_error(&result, validation, sample));
            }
            return Ok(());
        }

        // Fallback: compile per-call (shouldn't happen with taxonomy pre-compiled)
        match CompiledValidator::new(validation) {
            Ok(compiled) => {
                let result = compiled.validate(sample);
                if !result.is_valid {
                    return Err(map_first_error(&result, validation, sample));
                }
                Ok(())
            }
            Err(_) => Err(FailureReason::PatternMismatch {
                pattern: format!(
                    "INVALID SCHEMA: {}",
                    validation.pattern.as_deref().unwrap_or("(no pattern)")
                ),
            }),
        }
    }
}

impl Default for Checker {
    fn default() -> Self {
        Self::new(50)
    }
}

/// Map the first error from a `ValidationResult` to a `FailureReason` for reporting.
fn map_first_error(
    result: &crate::validator::ValidationResult,
    validation: &crate::taxonomy::Validation,
    sample: &str,
) -> FailureReason {
    if let Some(error) = result.errors.first() {
        match error.check {
            crate::validator::ValidationCheck::MinLength => FailureReason::TooShort {
                actual: sample.len(),
                min: validation.min_length.unwrap_or(0),
            },
            crate::validator::ValidationCheck::MaxLength => FailureReason::TooLong {
                actual: sample.len(),
                max: validation.max_length.unwrap_or(0),
            },
            crate::validator::ValidationCheck::Pattern => FailureReason::PatternMismatch {
                pattern: validation.pattern.clone().unwrap_or_default(),
            },
            _ => {
                // Minimum, Maximum, Enum — report with error message
                FailureReason::PatternMismatch {
                    pattern: error.message.clone(),
                }
            }
        }
    } else {
        FailureReason::PatternMismatch {
            pattern: "(unknown error)".to_string(),
        }
    }
}

/// Format a check report for terminal display.
pub fn format_report(report: &CheckReport, verbose: bool) -> String {
    let mut out = String::new();

    // Header
    out.push_str(&format!(
        "FineType Taxonomy Check — {} definitions\n",
        report.total_definitions
    ));
    out.push_str(&format!("{}\n\n", "=".repeat(60)));

    // Summary
    out.push_str("SUMMARY\n");
    out.push_str(&format!(
        "  Generators found:  {}/{}\n",
        report.generators_found, report.total_definitions
    ));
    out.push_str(&format!(
        "  Generators missing: {}\n",
        report.generators_missing
    ));
    out.push_str(&format!("  Fully passing:     {}\n", report.fully_passing));
    out.push_str(&format!("  Has failures:      {}\n", report.has_failures));
    out.push_str(&format!(
        "  No pattern:        {} (untestable)\n",
        report.no_pattern
    ));
    out.push_str(&format!(
        "  Samples:           {}/{} passed ({:.1}%)\n",
        report.total_passed,
        report.total_samples,
        report.pass_rate() * 100.0
    ));
    out.push('\n');

    // Per-domain summary
    out.push_str("BY DOMAIN\n");
    for (domain, results) in report.by_domain() {
        let total = results.len();
        let passing = results.iter().filter(|r| r.passed()).count();
        let missing = results.iter().filter(|r| !r.generator_exists).count();
        let marker = if passing == total {
            "\u{2705}"
        } else {
            "\u{274c}"
        };
        out.push_str(&format!(
            "  {} {:20} {}/{} passing",
            marker, domain, passing, total,
        ));
        if missing > 0 {
            out.push_str(&format!(" ({} missing generators)", missing));
        }
        out.push('\n');
    }
    out.push('\n');

    // Missing generators
    let missing: Vec<&DefinitionCheckResult> = report
        .results
        .iter()
        .filter(|r| !r.generator_exists)
        .collect();
    if !missing.is_empty() {
        out.push_str(&format!("MISSING GENERATORS ({})\n", missing.len()));
        for r in &missing {
            out.push_str(&format!(
                "  - {} (priority: {})\n",
                r.key, r.release_priority
            ));
        }
        out.push('\n');
    }

    // Validation failures
    let failing: Vec<&DefinitionCheckResult> = report
        .results
        .iter()
        .filter(|r| r.generator_exists && r.samples_failed > 0)
        .collect();
    if !failing.is_empty() {
        out.push_str(&format!("VALIDATION FAILURES ({})\n", failing.len()));
        for r in &failing {
            out.push_str(&format!(
                "  {} — {}/{} failed ({:.0}% pass rate)\n",
                r.key,
                r.samples_failed,
                r.samples_generated,
                r.pass_rate() * 100.0,
            ));
            if verbose {
                for failure in &r.failures {
                    let sample_display = if failure.sample.len() > 60 {
                        format!("{}...", &failure.sample[..57])
                    } else {
                        failure.sample.clone()
                    };
                    out.push_str(&format!("    {:?} -> {}\n", sample_display, failure.reason));
                }
            }
        }
        out.push('\n');
    }

    // Untestable (no validation pattern)
    if verbose {
        let no_pattern: Vec<&DefinitionCheckResult> = report
            .results
            .iter()
            .filter(|r| r.generator_exists && !r.has_pattern)
            .collect();
        if !no_pattern.is_empty() {
            out.push_str(&format!("NO VALIDATION PATTERN ({})\n", no_pattern.len()));
            for r in &no_pattern {
                out.push_str(&format!(
                    "  - {} (priority: {})\n",
                    r.key, r.release_priority
                ));
            }
            out.push('\n');
        }
    }

    // Final verdict
    if report.all_passed() {
        out.push_str("\u{2705} ALL CHECKS PASSED\n");
    } else {
        out.push_str(&format!(
            "\u{274c} CHECKS FAILED — {} missing generators, {} definitions with failures\n",
            report.generators_missing, report.has_failures
        ));
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::taxonomy::Taxonomy;

    fn test_taxonomy() -> Taxonomy {
        Taxonomy::from_yaml(
            r#"
datetime.timestamp.iso_8601:
  title: "ISO 8601"
  description: "Standard datetime"
  designation: universal
  locales: [UNIVERSAL]
  broad_type: TIMESTAMP
  format_string: "%Y-%m-%dT%H:%M:%SZ"
  transform: "strptime({col}, '%Y-%m-%dT%H:%M:%SZ')"
  validation:
    type: string
    pattern: "^\\d{4}-\\d{2}-\\d{2}T\\d{2}:\\d{2}:\\d{2}Z$"
    minLength: 20
    maxLength: 20
  tier: [TIMESTAMP, timestamp]
  release_priority: 5
  samples:
    - "2024-01-15T10:30:00Z"

technology.internet.ip_v4:
  title: "IPv4 Address"
  description: "Standard IPv4"
  designation: universal
  locales: [UNIVERSAL]
  broad_type: INET
  transform: "{col}::INET"
  validation:
    type: string
    pattern: "^\\d{1,3}\\.\\d{1,3}\\.\\d{1,3}\\.\\d{1,3}$"
  tier: [VARCHAR, internet]
  release_priority: 5
  samples:
    - "192.168.1.1"
"#,
        )
        .unwrap()
    }

    #[test]
    fn test_checker_basic() {
        let taxonomy = test_taxonomy();
        let checker = Checker::new(10).with_seed(42);
        let report = checker.run(&taxonomy);

        assert_eq!(report.total_definitions, 2);
        assert_eq!(report.generators_found, 2);
        assert_eq!(report.generators_missing, 0);
    }

    #[test]
    fn test_iso_8601_passes_validation() {
        let taxonomy = test_taxonomy();
        let checker = Checker::new(100).with_seed(42);
        let report = checker.run(&taxonomy);

        let iso = report
            .results
            .iter()
            .find(|r| r.key == "datetime.timestamp.iso_8601")
            .unwrap();
        assert!(iso.passed(), "ISO 8601 should pass all validations");
        assert_eq!(iso.samples_passed, 100);
    }

    #[test]
    fn test_ipv4_passes_validation() {
        let taxonomy = test_taxonomy();
        let checker = Checker::new(100).with_seed(42);
        let report = checker.run(&taxonomy);

        let ipv4 = report
            .results
            .iter()
            .find(|r| r.key == "technology.internet.ip_v4")
            .unwrap();
        assert!(ipv4.passed(), "IPv4 should pass all validations");
    }

    #[test]
    fn test_report_format() {
        let taxonomy = test_taxonomy();
        let checker = Checker::new(10).with_seed(42);
        let report = checker.run(&taxonomy);
        let formatted = format_report(&report, false);
        assert!(formatted.contains("FineType Taxonomy Check"));
        assert!(formatted.contains("BY DOMAIN"));
    }

    #[test]
    fn test_missing_generator_detected() {
        let taxonomy = Taxonomy::from_yaml(
            r#"
nonexistent.domain.type:
  title: "Nonexistent"
  designation: universal
  locales: [UNIVERSAL]
  broad_type: VARCHAR
  release_priority: 1
  validation:
    type: string
    pattern: ".*"
  samples: []
"#,
        )
        .unwrap();

        let checker = Checker::new(5).with_seed(42);
        let report = checker.run(&taxonomy);
        assert_eq!(report.generators_missing, 1);
        assert!(!report.all_passed());
    }
}
