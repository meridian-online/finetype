//! Taxonomy definitions for FineType labels.
//!
//! The taxonomy is organized hierarchically:
//! - Domain (e.g., `datetime`, `technology`, `identity`)
//! - Category (e.g., `timestamp`, `internet`, `person`)
//! - Type (e.g., `iso_8601`, `ip_v4`, `email`)
//! - Full label: `domain.category.type.LOCALE`
//!
//! Each definition is a transformation contract — not just a label.
//! If the model says `datetime.date.us_slash`, that is a contract that
//! `strptime(value, '%m/%d/%Y')::DATE` will succeed.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use thiserror::Error;

/// Errors that can occur when working with the taxonomy.
#[derive(Error, Debug)]
pub enum TaxonomyError {
    #[error("Failed to read taxonomy file: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Failed to parse taxonomy YAML: {0}")]
    ParseError(#[from] serde_yaml::Error),
    #[error("Invalid label key (expected domain.category.type): {0}")]
    InvalidKey(String),
    #[error("No definition files found in: {0}")]
    NoFiles(String),
    #[error("Glob pattern error: {0}")]
    GlobError(String),
}

/// Designation indicates the scope and stability of a label.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Designation {
    /// Universal format, works across all locales
    #[default]
    Universal,
    /// Locale-specific format
    LocaleSpecific,
    /// Broad category - numbers
    BroadNumbers,
    /// Broad category - characters/strings
    BroadCharacters,
    /// Broad category - words/text
    BroadWords,
    /// Broad category - objects/structured data
    BroadObject,
}

/// JSON Schema validation fragment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Validation {
    #[serde(rename = "type")]
    pub schema_type: Option<String>,
    pub pattern: Option<String>,
    #[serde(rename = "minLength")]
    pub min_length: Option<u32>,
    #[serde(rename = "maxLength")]
    pub max_length: Option<u32>,
    pub minimum: Option<f64>,
    pub maximum: Option<f64>,
    #[serde(rename = "enum")]
    pub enum_values: Option<Vec<String>>,
}

impl Validation {
    /// Convert to a JSON Schema object for use with jsonschema validators.
    ///
    /// Only includes string-applicable keywords (`pattern`, `minLength`,
    /// `maxLength`, `enum`). The `minimum`/`maximum` keywords are deliberately
    /// excluded because JSON Schema applies them to numeric JSON values, but
    /// FineType validates string representations of numbers — that semantic is
    /// handled manually in `CompiledValidator`.
    pub fn to_json_schema(&self) -> serde_json::Value {
        let mut schema = serde_json::Map::new();
        schema.insert("type".into(), serde_json::Value::String("string".into()));
        if let Some(p) = &self.pattern {
            schema.insert("pattern".into(), serde_json::Value::String(p.clone()));
        }
        if let Some(n) = self.min_length {
            schema.insert(
                "minLength".into(),
                serde_json::Value::Number(serde_json::Number::from(n)),
            );
        }
        if let Some(n) = self.max_length {
            schema.insert(
                "maxLength".into(),
                serde_json::Value::Number(serde_json::Number::from(n)),
            );
        }
        if let Some(vals) = &self.enum_values {
            let arr: Vec<serde_json::Value> = vals
                .iter()
                .map(|v| serde_json::Value::String(v.clone()))
                .collect();
            schema.insert("enum".into(), serde_json::Value::Array(arr));
        }
        serde_json::Value::Object(schema)
    }
}

/// A single label definition in the taxonomy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Definition {
    /// Human-readable title
    pub title: Option<String>,
    /// Description of the label
    pub description: Option<String>,
    /// Designation/scope of the label
    #[serde(default)]
    pub designation: Designation,
    /// Supported locales
    #[serde(default)]
    pub locales: Vec<String>,
    /// Target DuckDB type
    pub broad_type: Option<String>,
    /// DuckDB strptime format string (null if not strptime-based)
    pub format_string: Option<String>,
    /// Alternative format string for type variants (e.g., ISO 8601 with fractional seconds)
    pub format_string_alt: Option<String>,
    /// DuckDB SQL expression ({col} = column placeholder)
    pub transform: Option<String>,
    /// Enhanced transform requiring a DuckDB extension
    pub transform_ext: Option<String>,
    /// Struct expansion for multi-field output
    #[serde(default)]
    pub decompose: Option<serde_yaml::Value>,
    /// JSON Schema fragment for data quality checks
    pub validation: Option<Validation>,
    /// Per-locale validation schemas for locale-specific types.
    /// When present, attractor demotion can validate values against
    /// locale-specific patterns (e.g., US ZIP code vs UK postcode)
    /// instead of only the universal fallback schema.
    #[serde(default)]
    pub validation_by_locale: Option<HashMap<String, Validation>>,
    /// Path from root to parent in the inference graph
    #[serde(default)]
    pub tier: Vec<String>,
    /// Release priority (higher = more important)
    #[serde(default)]
    pub release_priority: u8,
    /// Aliases for this label
    pub aliases: Option<Vec<String>>,
    /// Example samples
    #[serde(default)]
    pub samples: Vec<serde_yaml::Value>,
    /// External references
    pub references: Option<serde_yaml::Value>,
    /// Notes about the label
    pub notes: Option<String>,
}

/// Parsed label with domain, category, and type components.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Label {
    pub domain: String,
    pub category: String,
    pub type_name: String,
}

impl Label {
    /// Parse a label key like "datetime.timestamp.iso_8601"
    pub fn parse(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() == 3 {
            Some(Label {
                domain: parts[0].to_string(),
                category: parts[1].to_string(),
                type_name: parts[2].to_string(),
            })
        } else {
            None
        }
    }

    /// Get the full key (domain.category.type)
    pub fn key(&self) -> String {
        format!("{}.{}.{}", self.domain, self.category, self.type_name)
    }

    /// Get the full label with locale
    pub fn with_locale(&self, locale: &str) -> String {
        format!(
            "{}.{}.{}.{}",
            self.domain, self.category, self.type_name, locale
        )
    }
}

impl std::fmt::Display for Label {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.key())
    }
}

/// DDL-oriented metadata extracted from a Definition.
///
/// Provides DuckDB-specific contract fields for schema generation and transformation.
#[derive(Debug, Clone)]
pub struct DdlInfo {
    /// DuckDB SQL type (e.g., "VARCHAR", "TIMESTAMP", "DOUBLE", "DATE", "BOOLEAN", "BIGINT")
    pub duckdb_type: String,
    /// SQL transformation expression, e.g., "strptime({col}, '%Y-%m-%dT%H:%M:%SZ')"
    pub transform: Option<String>,
    /// Primary format string for strptime (e.g., "%Y-%m-%dT%H:%M:%SZ")
    pub format_string: Option<String>,
    /// Alternative format string for type variants (e.g., "%Y-%m-%dT%H:%M:%S.%gZ")
    pub format_string_alt: Option<String>,
    /// Whether column should be nullable in DDL (defaults to true)
    pub nullable: bool,
    /// Struct expansion for multi-field output
    pub decompose: Option<serde_yaml::Value>,
}

impl DdlInfo {
    /// Map a DuckDB broad_type string to the SQL type name.
    ///
    /// Returns the canonical SQL type for use in CREATE TABLE statements.
    pub fn duckdb_type_from_broad_type(broad_type: &str) -> String {
        match broad_type {
            // String types
            "VARCHAR" => "VARCHAR".to_string(),
            // Numeric types
            "DOUBLE" => "DOUBLE".to_string(),
            "BIGINT" => "BIGINT".to_string(),
            "DECIMAL" => "DECIMAL".to_string(),
            // Date/Time types
            "DATE" => "DATE".to_string(),
            "TIMESTAMP" => "TIMESTAMP".to_string(),
            "TIME" => "TIME".to_string(),
            // Boolean
            "BOOLEAN" => "BOOLEAN".to_string(),
            // JSON/Composite
            "JSON" => "JSON".to_string(),
            "STRUCT" => "STRUCT".to_string(),
            "LIST" => "LIST".to_string(),
            // If not recognized, default to VARCHAR
            _ => "VARCHAR".to_string(),
        }
    }
}

/// The complete taxonomy of label definitions.
///
/// Optionally holds a cache of pre-compiled validators (populated via
/// `compile_validators()`). When the cache is populated, validation
/// functions can use `get_validator()` for zero-compilation lookups.
pub struct Taxonomy {
    definitions: HashMap<String, Definition>,
    labels: Vec<String>,
    /// Pre-compiled validators keyed by label. Populated lazily via
    /// `compile_validators()`. `None` until first call.
    compiled_validators: Option<HashMap<String, crate::validator::CompiledValidator>>,
    /// Pre-compiled locale-specific validators keyed by label → locale.
    /// Populated via `compile_locale_validators()`. `None` until first call.
    compiled_locale_validators:
        Option<HashMap<String, HashMap<String, crate::validator::CompiledValidator>>>,
}

// Manual Debug: jsonschema::Validator doesn't implement Debug
impl std::fmt::Debug for Taxonomy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Taxonomy")
            .field("definitions", &self.definitions)
            .field("labels", &self.labels)
            .field(
                "compiled_validators",
                &self
                    .compiled_validators
                    .as_ref()
                    .map(|m| format!("{} cached", m.len())),
            )
            .field(
                "compiled_locale_validators",
                &self
                    .compiled_locale_validators
                    .as_ref()
                    .map(|m| format!("{} labels with locale validators", m.len())),
            )
            .finish()
    }
}

// Manual Clone: jsonschema::Validator doesn't implement Clone.
// The cache is dropped on clone — callers should call compile_validators()
// again on the cloned instance if needed.
impl Clone for Taxonomy {
    fn clone(&self) -> Self {
        Self {
            definitions: self.definitions.clone(),
            labels: self.labels.clone(),
            compiled_validators: None,
            compiled_locale_validators: None,
        }
    }
}

impl Taxonomy {
    /// Load taxonomy from a single YAML file.
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, TaxonomyError> {
        let content = std::fs::read_to_string(path)?;
        Self::from_yaml(&content)
    }

    /// Load taxonomy from all definitions_*.yaml files in a directory.
    pub fn from_directory<P: AsRef<Path>>(dir: P) -> Result<Self, TaxonomyError> {
        let pattern = dir.as_ref().join("definitions_*.yaml");
        let pattern_str = pattern.to_string_lossy().to_string();

        let paths: Vec<_> = glob::glob(&pattern_str)
            .map_err(|e| TaxonomyError::GlobError(e.to_string()))?
            .filter_map(|entry| entry.ok())
            .collect();

        if paths.is_empty() {
            return Err(TaxonomyError::NoFiles(pattern_str));
        }

        let mut all_definitions = HashMap::new();

        for path in paths {
            let content = std::fs::read_to_string(&path)?;
            let defs: HashMap<String, Definition> = serde_yaml::from_str(&content)?;
            all_definitions.extend(defs);
        }

        let mut labels: Vec<String> = all_definitions.keys().cloned().collect();
        labels.sort();

        Ok(Taxonomy {
            definitions: all_definitions,
            labels,
            compiled_validators: None,
            compiled_locale_validators: None,
        })
    }

    /// Parse taxonomy from YAML string.
    pub fn from_yaml(yaml: &str) -> Result<Self, TaxonomyError> {
        let raw: HashMap<String, Definition> = serde_yaml::from_str(yaml)?;

        let mut labels: Vec<String> = raw.keys().cloned().collect();
        labels.sort();

        Ok(Taxonomy {
            definitions: raw,
            labels,
            compiled_validators: None,
            compiled_locale_validators: None,
        })
    }

    /// Parse taxonomy from multiple YAML strings (e.g. embedded at compile time).
    pub fn from_yamls(yamls: &[&str]) -> Result<Self, TaxonomyError> {
        let mut all_definitions = HashMap::new();

        for yaml in yamls {
            let defs: HashMap<String, Definition> = serde_yaml::from_str(yaml)?;
            all_definitions.extend(defs);
        }

        let mut labels: Vec<String> = all_definitions.keys().cloned().collect();
        labels.sort();

        Ok(Taxonomy {
            definitions: all_definitions,
            labels,
            compiled_validators: None,
            compiled_locale_validators: None,
        })
    }

    /// Get a definition by its full key (e.g., "datetime.timestamp.iso_8601")
    pub fn get(&self, key: &str) -> Option<&Definition> {
        self.definitions.get(key)
    }

    /// Get all label keys (sorted)
    pub fn labels(&self) -> &[String] {
        &self.labels
    }

    /// Get all definitions
    pub fn definitions(&self) -> impl Iterator<Item = (&String, &Definition)> {
        self.definitions.iter()
    }

    /// Get definitions at or above a priority level
    pub fn at_priority(&self, min_priority: u8) -> Vec<(&String, &Definition)> {
        self.definitions
            .iter()
            .filter(|(_, d)| d.release_priority >= min_priority)
            .collect()
    }

    /// Get definitions by domain
    pub fn by_domain(&self, domain: &str) -> Vec<(&String, &Definition)> {
        self.definitions
            .iter()
            .filter(|(k, _)| k.starts_with(&format!("{}.", domain)))
            .collect()
    }

    /// Get definitions by domain and category
    pub fn by_category(&self, domain: &str, category: &str) -> Vec<(&String, &Definition)> {
        let prefix = format!("{}.{}.", domain, category);
        self.definitions
            .iter()
            .filter(|(k, _)| k.starts_with(&prefix))
            .collect()
    }

    /// Get all unique domains
    pub fn domains(&self) -> Vec<String> {
        let mut domains: Vec<String> = self
            .definitions
            .keys()
            .filter_map(|k| k.split('.').next().map(String::from))
            .collect();
        domains.sort();
        domains.dedup();
        domains
    }

    /// Get all unique categories within a domain
    pub fn categories(&self, domain: &str) -> Vec<String> {
        let prefix = format!("{}.", domain);
        let mut cats: Vec<String> = self
            .definitions
            .keys()
            .filter(|k| k.starts_with(&prefix))
            .filter_map(|k| k.split('.').nth(1).map(String::from))
            .collect();
        cats.sort();
        cats.dedup();
        cats
    }

    /// Number of definitions
    pub fn len(&self) -> usize {
        self.definitions.len()
    }

    /// Check if taxonomy is empty
    pub fn is_empty(&self) -> bool {
        self.definitions.is_empty()
    }

    /// Create label to index mapping for model training
    pub fn label_to_index(&self) -> HashMap<String, usize> {
        self.labels
            .iter()
            .enumerate()
            .map(|(i, l)| (l.clone(), i))
            .collect()
    }

    /// Create index to label mapping for model inference
    pub fn index_to_label(&self) -> HashMap<usize, String> {
        self.labels
            .iter()
            .enumerate()
            .map(|(i, l)| (i, l.clone()))
            .collect()
    }

    /// Pre-compile JSON Schema validators for all definitions that have a
    /// `validation` block.
    ///
    /// Call once after loading the taxonomy. Subsequent calls to
    /// `get_validator()` return references to the cached validators,
    /// eliminating per-value compilation overhead.
    ///
    /// Definitions whose schemas fail to compile are silently skipped (the
    /// fallback path in `validate_value()` handles them).
    pub fn compile_validators(&mut self) {
        let mut cache = HashMap::new();
        for (label, def) in &self.definitions {
            if let Some(validation) = &def.validation {
                if let Ok(compiled) =
                    crate::validator::CompiledValidator::new_for_label(validation, label)
                {
                    cache.insert(label.clone(), compiled);
                }
            }
        }
        self.compiled_validators = Some(cache);
    }

    /// Get a pre-compiled validator for a label.
    ///
    /// Returns `None` if validators haven't been compiled (call
    /// `compile_validators()` first) or if the label has no validation schema.
    pub fn get_validator(&self, label: &str) -> Option<&crate::validator::CompiledValidator> {
        self.compiled_validators
            .as_ref()
            .and_then(|cache| cache.get(label))
    }

    /// Number of cached compiled validators.
    pub fn validator_count(&self) -> usize {
        self.compiled_validators
            .as_ref()
            .map(|c| c.len())
            .unwrap_or(0)
    }

    /// Pre-compile locale-specific JSON Schema validators for all definitions
    /// that have a `validation_by_locale` block.
    ///
    /// Call after `compile_validators()`. Produces a nested cache:
    /// label → locale → CompiledValidator. Definitions or locales whose
    /// schemas fail to compile are silently skipped.
    pub fn compile_locale_validators(&mut self) {
        let mut cache: HashMap<String, HashMap<String, crate::validator::CompiledValidator>> =
            HashMap::new();
        for (label, def) in &self.definitions {
            if let Some(locale_map) = &def.validation_by_locale {
                let mut locale_cache = HashMap::new();
                for (locale, validation) in locale_map {
                    if let Ok(compiled) =
                        crate::validator::CompiledValidator::new_for_label(validation, label)
                    {
                        locale_cache.insert(locale.clone(), compiled);
                    }
                }
                if !locale_cache.is_empty() {
                    cache.insert(label.clone(), locale_cache);
                }
            }
        }
        self.compiled_locale_validators = Some(cache);
    }

    /// Get pre-compiled locale validators for a label.
    ///
    /// Returns `None` if locale validators haven't been compiled or if the
    /// label has no `validation_by_locale` block. Returns a map of
    /// locale → CompiledValidator.
    pub fn get_locale_validators(
        &self,
        label: &str,
    ) -> Option<&HashMap<String, crate::validator::CompiledValidator>> {
        self.compiled_locale_validators
            .as_ref()
            .and_then(|cache| cache.get(label))
    }

    /// Number of labels with cached locale validators.
    pub fn locale_validator_count(&self) -> usize {
        self.compiled_locale_validators
            .as_ref()
            .map(|c| c.len())
            .unwrap_or(0)
    }

    /// Extract DDL-oriented metadata from a definition.
    ///
    /// Returns `None` if the label doesn't exist. Otherwise returns a `DdlInfo`
    /// with broad_type, transform, format_string, format_string_alt, and decompose.
    ///
    /// The `nullable` field defaults to `true`; callers should override based on
    /// null counts from profiling if needed.
    pub fn ddl_info(&self, label: &str) -> Option<DdlInfo> {
        self.get(label).map(|def| {
            let duckdb_type = def
                .broad_type
                .as_ref()
                .map(|bt| DdlInfo::duckdb_type_from_broad_type(bt))
                .unwrap_or_else(|| "VARCHAR".to_string());

            DdlInfo {
                duckdb_type,
                transform: def.transform.clone(),
                format_string: def.format_string.clone(),
                format_string_alt: def.format_string_alt.clone(),
                nullable: true,
                decompose: def.decompose.clone(),
            }
        })
    }

    /// Build a tier graph from the taxonomy's tier fields.
    pub fn tier_graph(&self) -> TierGraph {
        TierGraph::from_taxonomy(self)
    }
}

/// Tier graph for hierarchical inference.
///
/// Extracts the tree structure from the `tier` field in each definition:
/// - **Tier 0**: Broad DuckDB type (e.g., VARCHAR, DATE, TIMESTAMP)
/// - **Tier 1**: Category within a broad type (e.g., internet, person, date)
/// - **Tier 2**: Specific type within a category (the full `domain.category.type` label)
///
/// The graph is built from the `tier: [BROAD_TYPE, category]` field in each definition.
#[derive(Debug, Clone)]
pub struct TierGraph {
    /// Sorted unique broad types (Tier 0 classes)
    broad_types: Vec<String>,
    /// broad_type → sorted category names (Tier 1 classes per broad type)
    categories: HashMap<String, Vec<String>>,
    /// (broad_type, category) → sorted full labels (Tier 2 classes)
    types: HashMap<(String, String), Vec<String>>,
    /// full label → (broad_type, category)
    label_path: HashMap<String, (String, String)>,
}

impl TierGraph {
    /// Build a tier graph from a taxonomy.
    pub fn from_taxonomy(taxonomy: &Taxonomy) -> Self {
        let mut categories: HashMap<String, Vec<String>> = HashMap::new();
        let mut types: HashMap<(String, String), Vec<String>> = HashMap::new();
        let mut label_path: HashMap<String, (String, String)> = HashMap::new();

        for (key, def) in taxonomy.definitions() {
            if def.tier.len() >= 2 {
                let broad_type = def.tier[0].clone();
                let category = def.tier[1].clone();

                categories
                    .entry(broad_type.clone())
                    .or_default()
                    .push(category.clone());

                types
                    .entry((broad_type.clone(), category.clone()))
                    .or_default()
                    .push(key.clone());

                label_path.insert(key.clone(), (broad_type, category));
            }
        }

        // Deduplicate and sort
        for cats in categories.values_mut() {
            cats.sort();
            cats.dedup();
        }
        for labels in types.values_mut() {
            labels.sort();
        }

        let mut broad_types: Vec<String> = categories.keys().cloned().collect();
        broad_types.sort();

        TierGraph {
            broad_types,
            categories,
            types,
            label_path,
        }
    }

    /// Get Tier 0 classes (sorted broad types).
    pub fn broad_types(&self) -> &[String] {
        &self.broad_types
    }

    /// Get Tier 1 classes for a broad type (sorted categories).
    pub fn categories_for(&self, broad_type: &str) -> &[String] {
        self.categories
            .get(broad_type)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Get Tier 2 classes for a (broad_type, category) pair (sorted full labels).
    pub fn types_for(&self, broad_type: &str, category: &str) -> &[String] {
        self.types
            .get(&(broad_type.to_string(), category.to_string()))
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Get the tier path (broad_type, category) for a full label.
    pub fn tier_path(&self, label: &str) -> Option<&(String, String)> {
        self.label_path.get(label)
    }

    /// Get the broad type (Tier 0 label) for a full label.
    pub fn broad_type_for(&self, label: &str) -> Option<&str> {
        self.label_path.get(label).map(|(bt, _)| bt.as_str())
    }

    /// Get the category (Tier 1 label) for a full label.
    pub fn category_for(&self, label: &str) -> Option<&str> {
        self.label_path.get(label).map(|(_, cat)| cat.as_str())
    }

    /// Whether a Tier 2 model is needed for this (broad_type, category) — true if >5 types.
    pub fn needs_tier2(&self, broad_type: &str, category: &str, min_types: usize) -> bool {
        self.types_for(broad_type, category).len() > min_types
    }

    /// Number of Tier 0 classes.
    pub fn num_broad_types(&self) -> usize {
        self.broad_types.len()
    }

    /// Number of Tier 1 classes for a broad type.
    pub fn num_categories(&self, broad_type: &str) -> usize {
        self.categories_for(broad_type).len()
    }

    /// Number of Tier 2 classes for a (broad_type, category).
    pub fn num_types(&self, broad_type: &str, category: &str) -> usize {
        self.types_for(broad_type, category).len()
    }

    /// Get all (broad_type, category) pairs that need a Tier 2 model.
    pub fn tier2_groups(&self, min_types: usize) -> Vec<(String, String)> {
        let mut groups: Vec<(String, String)> = self
            .types
            .iter()
            .filter(|(_, labels)| labels.len() > min_types)
            .map(|((bt, cat), _)| (bt.clone(), cat.clone()))
            .collect();
        groups.sort();
        groups
    }

    /// Get all (broad_type, category) pairs where Tier 1 directly resolves to a single type
    /// (no Tier 2 needed because there's only one type in this group).
    pub fn direct_resolve_groups(&self) -> Vec<((String, String), String)> {
        let mut groups: Vec<((String, String), String)> = self
            .types
            .iter()
            .filter(|(_, labels)| labels.len() == 1)
            .map(|((bt, cat), labels)| ((bt.clone(), cat.clone()), labels[0].clone()))
            .collect();
        groups.sort();
        groups
    }

    /// Summary of the tier graph structure.
    pub fn summary(&self) -> TierGraphSummary {
        let tier1_models = self.broad_types.len();
        let tier2_models_5 = self.tier2_groups(5).len();
        let tier2_models_1 = self.tier2_groups(1).len();
        let direct_resolve = self.direct_resolve_groups().len();
        let total_labels = self.label_path.len();

        TierGraphSummary {
            tier0_classes: self.broad_types.len(),
            tier1_models,
            tier2_models_gt5: tier2_models_5,
            tier2_models_gt1: tier2_models_1,
            direct_resolve_groups: direct_resolve,
            total_labels,
        }
    }
}

/// Summary statistics for a tier graph.
#[derive(Debug, Clone)]
pub struct TierGraphSummary {
    pub tier0_classes: usize,
    pub tier1_models: usize,
    pub tier2_models_gt5: usize,
    pub tier2_models_gt1: usize,
    pub direct_resolve_groups: usize,
    pub total_labels: usize,
}

impl std::fmt::Display for TierGraphSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Tier Graph Summary:")?;
        writeln!(f, "  Tier 0: {} broad types", self.tier0_classes)?;
        writeln!(
            f,
            "  Tier 1: {} models (one per broad type)",
            self.tier1_models
        )?;
        writeln!(
            f,
            "  Tier 2: {} models (categories with >5 types)",
            self.tier2_models_gt5
        )?;
        writeln!(
            f,
            "  Direct resolve: {} groups (single type, no Tier 2 needed)",
            self.direct_resolve_groups
        )?;
        writeln!(f, "  Total labels: {}", self.total_labels)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_YAML: &str = r#"
datetime.timestamp.iso_8601:
  title: "ISO 8601"
  description: "Standard international datetime format"
  designation: universal
  locales: [UNIVERSAL]
  broad_type: TIMESTAMP
  format_string: "%Y-%m-%dT%H:%M:%SZ"
  transform: "strptime({col}, '%Y-%m-%dT%H:%M:%SZ')"
  transform_ext: null
  decompose: null
  validation:
    type: string
    pattern: "^\\d{4}-\\d{2}-\\d{2}T\\d{2}:\\d{2}:\\d{2}Z$"
    minLength: 20
    maxLength: 20
  tier: [TIMESTAMP, timestamp]
  release_priority: 5
  aliases: [big_endian]
  samples:
    - "2024-01-15T10:30:00Z"
  references: null
  notes: null
"#;

    #[test]
    fn test_parse_yaml() {
        let taxonomy = Taxonomy::from_yaml(SAMPLE_YAML).unwrap();
        assert_eq!(taxonomy.len(), 1);
        assert_eq!(taxonomy.labels(), &["datetime.timestamp.iso_8601"]);
    }

    #[test]
    fn test_label_parse() {
        let label = Label::parse("datetime.timestamp.iso_8601").unwrap();
        assert_eq!(label.domain, "datetime");
        assert_eq!(label.category, "timestamp");
        assert_eq!(label.type_name, "iso_8601");
        assert_eq!(label.key(), "datetime.timestamp.iso_8601");
    }

    #[test]
    fn test_label_with_locale() {
        let label = Label::parse("datetime.date.abbreviated_month").unwrap();
        assert_eq!(
            label.with_locale("FR"),
            "datetime.date.abbreviated_month.FR"
        );
    }

    #[test]
    fn test_get_definition() {
        let taxonomy = Taxonomy::from_yaml(SAMPLE_YAML).unwrap();
        let def = taxonomy.get("datetime.timestamp.iso_8601").unwrap();
        assert_eq!(def.title.as_deref(), Some("ISO 8601"));
        assert_eq!(def.broad_type.as_deref(), Some("TIMESTAMP"));
        assert_eq!(def.release_priority, 5);
    }

    #[test]
    fn test_domains() {
        let taxonomy = Taxonomy::from_yaml(SAMPLE_YAML).unwrap();
        assert_eq!(taxonomy.domains(), vec!["datetime"]);
    }

    #[test]
    fn test_categories() {
        let taxonomy = Taxonomy::from_yaml(SAMPLE_YAML).unwrap();
        assert_eq!(taxonomy.categories("datetime"), vec!["timestamp"]);
    }

    #[test]
    fn test_at_priority() {
        let taxonomy = Taxonomy::from_yaml(SAMPLE_YAML).unwrap();
        assert_eq!(taxonomy.at_priority(5).len(), 1);
        assert_eq!(taxonomy.at_priority(6).len(), 0);
    }

    const TIERED_YAML: &str = r#"
datetime.timestamp.iso_8601:
  title: "ISO 8601"
  description: "Standard"
  designation: universal
  locales: [UNIVERSAL]
  broad_type: TIMESTAMP
  format_string: null
  transform: null
  validation:
    type: string
  tier: [TIMESTAMP, timestamp]
  release_priority: 5
  samples: ["2024-01-15T10:30:00Z"]

datetime.timestamp.rfc_2822:
  title: "RFC 2822"
  description: "Email"
  designation: universal
  locales: [UNIVERSAL]
  broad_type: TIMESTAMP
  format_string: null
  transform: null
  validation:
    type: string
  tier: [TIMESTAMP, timestamp]
  release_priority: 5
  samples: ["Mon, 15 Jan 2024 10:30:00 +0000"]

datetime.date.us_slash:
  title: "US Slash Date"
  description: "US format"
  designation: universal
  locales: [UNIVERSAL]
  broad_type: DATE
  format_string: null
  transform: null
  validation:
    type: string
  tier: [DATE, date]
  release_priority: 5
  samples: ["01/15/2024"]

technology.internet.ip_v4:
  title: "IPv4"
  description: "IP"
  designation: universal
  locales: [UNIVERSAL]
  broad_type: INET
  format_string: null
  transform: null
  validation:
    type: string
  tier: [INET, internet]
  release_priority: 5
  samples: ["192.168.1.1"]

technology.internet.ip_v6:
  title: "IPv6"
  description: "IP"
  designation: universal
  locales: [UNIVERSAL]
  broad_type: INET
  format_string: null
  transform: null
  validation:
    type: string
  tier: [INET, internet]
  release_priority: 5
  samples: ["::1"]
"#;

    #[test]
    fn test_tier_graph_broad_types() {
        let taxonomy = Taxonomy::from_yaml(TIERED_YAML).unwrap();
        let graph = taxonomy.tier_graph();
        assert_eq!(graph.broad_types(), &["DATE", "INET", "TIMESTAMP"]);
        assert_eq!(graph.num_broad_types(), 3);
    }

    #[test]
    fn test_tier_graph_categories() {
        let taxonomy = Taxonomy::from_yaml(TIERED_YAML).unwrap();
        let graph = taxonomy.tier_graph();
        assert_eq!(graph.categories_for("TIMESTAMP"), &["timestamp"]);
        assert_eq!(graph.categories_for("INET"), &["internet"]);
        assert_eq!(graph.categories_for("DATE"), &["date"]);
        assert_eq!(graph.categories_for("UNKNOWN").len(), 0);
    }

    #[test]
    fn test_tier_graph_types() {
        let taxonomy = Taxonomy::from_yaml(TIERED_YAML).unwrap();
        let graph = taxonomy.tier_graph();
        let ts_types = graph.types_for("TIMESTAMP", "timestamp");
        assert_eq!(ts_types.len(), 2);
        assert!(ts_types.contains(&"datetime.timestamp.iso_8601".to_string()));
        assert!(ts_types.contains(&"datetime.timestamp.rfc_2822".to_string()));

        let inet_types = graph.types_for("INET", "internet");
        assert_eq!(inet_types.len(), 2);
    }

    #[test]
    fn test_tier_graph_tier_path() {
        let taxonomy = Taxonomy::from_yaml(TIERED_YAML).unwrap();
        let graph = taxonomy.tier_graph();
        assert_eq!(
            graph.tier_path("datetime.timestamp.iso_8601"),
            Some(&("TIMESTAMP".to_string(), "timestamp".to_string()))
        );
        assert_eq!(
            graph.broad_type_for("technology.internet.ip_v4"),
            Some("INET")
        );
        assert_eq!(
            graph.category_for("technology.internet.ip_v4"),
            Some("internet")
        );
    }

    #[test]
    fn test_tier_graph_summary() {
        let taxonomy = Taxonomy::from_yaml(TIERED_YAML).unwrap();
        let graph = taxonomy.tier_graph();
        let summary = graph.summary();
        assert_eq!(summary.tier0_classes, 3);
        assert_eq!(summary.total_labels, 5);
    }

    // ── Validator cache tests (NNFT-116) ────────────────────────────────

    #[test]
    fn test_compile_validators_basic() {
        let mut taxonomy = Taxonomy::from_yaml(SAMPLE_YAML).unwrap();
        assert_eq!(taxonomy.validator_count(), 0);
        taxonomy.compile_validators();
        // SAMPLE_YAML has 1 definition with a validation block
        assert_eq!(taxonomy.validator_count(), 1);

        let validator = taxonomy.get_validator("datetime.timestamp.iso_8601");
        assert!(validator.is_some());
        assert!(validator.unwrap().is_valid("2024-01-15T10:30:00Z"));
        assert!(!validator.unwrap().is_valid("not-a-date"));
    }

    #[test]
    fn test_compile_validators_no_validation_block() {
        // Definitions without validation blocks should not produce validators
        let yaml = r#"
representation.discrete.categorical:
  title: "Categorical"
  broad_type: VARCHAR
  tier: [VARCHAR, discrete]
  release_priority: 3
  samples: ["red"]
"#;
        let mut taxonomy = Taxonomy::from_yaml(yaml).unwrap();
        taxonomy.compile_validators();
        assert_eq!(taxonomy.validator_count(), 0);
        assert!(taxonomy
            .get_validator("representation.discrete.categorical")
            .is_none());
    }

    #[test]
    fn test_clone_drops_cache() {
        let mut taxonomy = Taxonomy::from_yaml(SAMPLE_YAML).unwrap();
        taxonomy.compile_validators();
        assert_eq!(taxonomy.validator_count(), 1);

        let cloned = taxonomy.clone();
        // Cache is dropped on clone (jsonschema::Validator doesn't impl Clone)
        assert_eq!(cloned.validator_count(), 0);
    }

    #[test]
    fn test_to_json_schema_full() {
        let taxonomy = Taxonomy::from_yaml(SAMPLE_YAML).unwrap();
        let def = taxonomy.get("datetime.timestamp.iso_8601").unwrap();
        let validation = def.validation.as_ref().unwrap();
        let json = validation.to_json_schema();
        let obj = json.as_object().unwrap();
        assert_eq!(obj.get("type").unwrap(), "string");
        assert!(obj.get("pattern").is_some());
        assert_eq!(obj.get("minLength").unwrap(), 20);
        assert_eq!(obj.get("maxLength").unwrap(), 20);
    }

    /// Verify all taxonomy schemas from the labels/ directory compile
    /// successfully. This is the critical acceptance test for NNFT-116.
    #[test]
    fn test_all_taxonomy_schemas_compile() {
        // Try loading from labels/ directory (works in dev, may not in CI)
        let taxonomy_result = Taxonomy::from_directory("labels");
        if let Ok(mut taxonomy) = taxonomy_result {
            taxonomy.compile_validators();
            // Most of 169 types have validation blocks
            assert!(
                taxonomy.validator_count() >= 100,
                "Expected ≥100 compiled validators, got {}",
                taxonomy.validator_count()
            );
            eprintln!(
                "All {} validators compiled successfully from {} definitions",
                taxonomy.validator_count(),
                taxonomy.len()
            );
        } else {
            // In CI/release builds, labels/ may not exist — skip gracefully
            eprintln!("Skipping full taxonomy test (labels/ not found)");
        }
    }

    // ── Locale validator cache tests ────────────────────────────────────

    const LOCALE_YAML: &str = r#"
geography.address.postal_code:
  title: "Postal Code"
  validation:
    type: string
    minLength: 3
    maxLength: 10
    maximum: 99999
  validation_by_locale:
    EN_US:
      type: string
      pattern: "^(\\d{5})(?:[ \\-](\\d{4}))?$"
      minLength: 5
      maxLength: 10
    EN_GB:
      type: string
      pattern: "^[A-Z]{1,2}\\d[A-Z\\d]?\\s?\\d[A-Z]{2}$"
      minLength: 5
      maxLength: 8
    DE:
      type: string
      pattern: "^\\d{5}$"
      minLength: 5
      maxLength: 5
  tier: [VARCHAR, address]
  release_priority: 4
  samples: ["10001"]
"#;

    #[test]
    fn test_compile_locale_validators() {
        let mut taxonomy = Taxonomy::from_yaml(LOCALE_YAML).unwrap();
        assert_eq!(taxonomy.locale_validator_count(), 0);

        taxonomy.compile_locale_validators();
        assert_eq!(
            taxonomy.locale_validator_count(),
            1,
            "Should have 1 label with locale validators"
        );

        let locale_validators = taxonomy
            .get_locale_validators("geography.address.postal_code")
            .unwrap();
        assert_eq!(locale_validators.len(), 3, "Should have 3 locales compiled");
        assert!(locale_validators.contains_key("EN_US"));
        assert!(locale_validators.contains_key("EN_GB"));
        assert!(locale_validators.contains_key("DE"));
    }

    #[test]
    fn test_locale_postal_code_us() {
        let mut taxonomy = Taxonomy::from_yaml(LOCALE_YAML).unwrap();
        taxonomy.compile_locale_validators();
        let locales = taxonomy
            .get_locale_validators("geography.address.postal_code")
            .unwrap();

        let us = locales.get("EN_US").unwrap();
        assert!(us.is_valid("10001"));
        assert!(us.is_valid("90210"));
        assert!(us.is_valid("85000")); // 5-digit US ZIP is valid
        assert!(us.is_valid("95014-1234")); // ZIP+4
        assert!(!us.is_valid("EC1A 1BB")); // UK postcode fails US
        assert!(!us.is_valid("112000")); // 6-digit salary fails US
        assert!(!us.is_valid("1234")); // too short
    }

    #[test]
    fn test_locale_postal_code_gb() {
        let mut taxonomy = Taxonomy::from_yaml(LOCALE_YAML).unwrap();
        taxonomy.compile_locale_validators();
        let locales = taxonomy
            .get_locale_validators("geography.address.postal_code")
            .unwrap();

        let gb = locales.get("EN_GB").unwrap();
        assert!(gb.is_valid("EC1A 1BB"));
        assert!(gb.is_valid("W1C 1AX"));
        assert!(gb.is_valid("M2 5BQ"));
        assert!(gb.is_valid("SW1A 1AA"));
        assert!(!gb.is_valid("10001")); // US ZIP fails UK
        assert!(!gb.is_valid("85000")); // numeric fails UK
    }

    #[test]
    fn test_locale_postal_code_cross_rejection() {
        // 6+ digit salary values fail ALL locale patterns
        // (5-digit values like "85000" ARE valid US/DE postal codes,
        // so we only test clearly-salary-range values here)
        let mut taxonomy = Taxonomy::from_yaml(LOCALE_YAML).unwrap();
        taxonomy.compile_locale_validators();
        let locales = taxonomy
            .get_locale_validators("geography.address.postal_code")
            .unwrap();

        let salary_values = ["112000", "245000", "350000", "102500"];
        for value in &salary_values {
            for (locale, compiled) in locales {
                assert!(
                    !compiled.is_valid(value),
                    "Salary {} should fail locale {} validation",
                    value,
                    locale
                );
            }
        }
    }

    #[test]
    fn test_locale_validators_no_block() {
        // Definitions without validation_by_locale should return None
        let mut taxonomy = Taxonomy::from_yaml(SAMPLE_YAML).unwrap();
        taxonomy.compile_locale_validators();
        assert_eq!(taxonomy.locale_validator_count(), 0);
        assert!(taxonomy
            .get_locale_validators("datetime.timestamp.iso_8601")
            .is_none());
    }

    #[test]
    fn test_clone_drops_locale_cache() {
        let mut taxonomy = Taxonomy::from_yaml(LOCALE_YAML).unwrap();
        taxonomy.compile_locale_validators();
        assert_eq!(taxonomy.locale_validator_count(), 1);

        let cloned = taxonomy.clone();
        assert_eq!(cloned.locale_validator_count(), 0);
    }

    #[test]
    fn test_all_taxonomy_locale_schemas_compile() {
        // Try loading from labels/ directory — verifies real YAML locale patterns compile
        let taxonomy_result = Taxonomy::from_directory("labels");
        if let Ok(mut taxonomy) = taxonomy_result {
            taxonomy.compile_locale_validators();
            assert!(
                taxonomy.locale_validator_count() >= 1,
                "Expected ≥1 label with locale validators, got {}",
                taxonomy.locale_validator_count()
            );
            eprintln!(
                "{} labels with locale validators compiled successfully",
                taxonomy.locale_validator_count()
            );
        } else {
            eprintln!("Skipping full taxonomy locale test (labels/ not found)");
        }
    }

    #[test]
    fn test_ddl_info_from_definition() {
        let taxonomy = Taxonomy::from_yaml(SAMPLE_YAML).unwrap();
        let ddl_info = taxonomy
            .ddl_info("datetime.timestamp.iso_8601")
            .expect("Expected DdlInfo");

        assert_eq!(ddl_info.duckdb_type, "TIMESTAMP");
        assert_eq!(
            ddl_info.transform.as_deref(),
            Some("strptime({col}, '%Y-%m-%dT%H:%M:%SZ')")
        );
        assert_eq!(
            ddl_info.format_string.as_deref(),
            Some("%Y-%m-%dT%H:%M:%SZ")
        );
        assert!(ddl_info.nullable);
        assert!(ddl_info.format_string_alt.is_none());
        assert!(ddl_info.decompose.is_none());
    }

    #[test]
    fn test_ddl_info_missing_label() {
        let taxonomy = Taxonomy::from_yaml(SAMPLE_YAML).unwrap();
        let result = taxonomy.ddl_info("nonexistent.label.type");
        assert!(result.is_none(), "Expected None for missing label");
    }

    #[test]
    fn test_duckdb_type_mapping() {
        assert_eq!(DdlInfo::duckdb_type_from_broad_type("VARCHAR"), "VARCHAR");
        assert_eq!(
            DdlInfo::duckdb_type_from_broad_type("TIMESTAMP"),
            "TIMESTAMP"
        );
        assert_eq!(DdlInfo::duckdb_type_from_broad_type("DOUBLE"), "DOUBLE");
        assert_eq!(DdlInfo::duckdb_type_from_broad_type("DATE"), "DATE");
        assert_eq!(DdlInfo::duckdb_type_from_broad_type("BOOLEAN"), "BOOLEAN");
        assert_eq!(DdlInfo::duckdb_type_from_broad_type("BIGINT"), "BIGINT");
        assert_eq!(DdlInfo::duckdb_type_from_broad_type("JSON"), "JSON");
        assert_eq!(DdlInfo::duckdb_type_from_broad_type("STRUCT"), "STRUCT");
        assert_eq!(DdlInfo::duckdb_type_from_broad_type("LIST"), "LIST");
        // Unknown types should default to VARCHAR
        assert_eq!(
            DdlInfo::duckdb_type_from_broad_type("UNKNOWN_TYPE"),
            "VARCHAR"
        );
    }

    #[test]
    fn test_ddl_info_across_domains() {
        // Test DdlInfo extraction across multiple domains using real taxonomy
        let taxonomy_result = Taxonomy::from_directory("labels");
        if let Ok(taxonomy) = taxonomy_result {
            // Sample a few types from different domains
            let test_cases = vec![
                ("datetime.date.iso_8601", "DATE"),
                ("identity.person.email", "VARCHAR"),
                ("geography.address.postal_code", "VARCHAR"),
                ("representation.numeric.decimal_number", "DOUBLE"),
                ("finance.currency.currency_code", "VARCHAR"),
            ];

            for (label, expected_type) in test_cases {
                if let Some(ddl_info) = taxonomy.ddl_info(label) {
                    assert_eq!(
                        ddl_info.duckdb_type, expected_type,
                        "Wrong DDL type for {}",
                        label
                    );
                } else {
                    // It's OK if the label doesn't exist in the test environment
                    eprintln!("Label {} not found in test environment", label);
                }
            }
        } else {
            eprintln!("Skipping multi-domain test (labels/ not found)");
        }
    }
}
