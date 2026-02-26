//! Column-mode inference for distribution-based type disambiguation.
//!
//! Column-mode takes a vector of string values (a column sample), runs
//! single-value inference on each, aggregates the predictions, and applies
//! disambiguation rules to determine the most likely type for the entire column.
//!
//! This is critical for resolving ambiguous types like:
//! - `us_slash` vs `eu_slash` dates (MM/DD vs DD/MM)
//! - `short_dmy` vs `short_mdy` dates
//! - `latitude` vs `longitude` coordinates
//! - Numeric types (port, increment, postal_code, integer_number)

use crate::inference::{ClassificationResult, InferenceError, ValueClassifier};
use crate::semantic::SemanticHintClassifier;
use finetype_core::{Designation, Taxonomy};
use std::collections::HashMap;

/// Strip a locale suffix from a 4-level label to get the 3-level taxonomy key.
///
/// Examples:
///   "geography.address.postal_code.EN_US" → ("geography.address.postal_code", Some("EN_US"))
///   "geography.address.postal_code.UNIVERSAL" → ("geography.address.postal_code", Some("UNIVERSAL"))
///   "geography.address.postal_code" → ("geography.address.postal_code", None)
///   "representation.boolean.binary" → ("representation.boolean.binary", None)
///
/// Detection heuristic: if the label has 4+ dot-separated parts and the last part
/// is ALL_UPPERCASE (locale code or UNIVERSAL), treat it as a locale suffix.
fn strip_locale_suffix(label: &str) -> (&str, Option<&str>) {
    if let Some((prefix, suffix)) = label.rsplit_once('.') {
        // Check if suffix looks like a locale code: all uppercase, 2-5 chars
        // (e.g., EN, EN_US, UNIVERSAL, FR_FR, DE, AR)
        let is_locale = !suffix.is_empty()
            && suffix.len() <= 10
            && suffix.chars().all(|c| c.is_ascii_uppercase() || c == '_')
            && prefix.contains('.'); // Must have at least domain.category.type before suffix
        if is_locale {
            (prefix, Some(suffix))
        } else {
            (label, None)
        }
    } else {
        (label, None)
    }
}

/// All known boolean type labels (current and legacy).
/// Centralised to avoid label mismatches across disambiguation rules.
const BOOLEAN_LABELS: &[&str] = &[
    "representation.boolean.binary",   // NNFT-075: 0/1
    "representation.boolean.initials", // NNFT-075: T/F, Y/N
    "representation.boolean.terms",    // NNFT-075: true/false, yes/no, on/off
    "technology.development.boolean",  // legacy (pre-NNFT-075 model)
    "representation.logical.boolean",  // legacy interim label
    "technology.data.boolean",         // legacy
];

/// Hardcoded list of labels known to be generic catch-all predictions.
/// Used as a fallback when taxonomy is not available for designation lookup.
const HARDCODED_GENERIC_LABELS: &[&str] = &[
    "representation.text.word",
    "representation.text.plain_text",
    "representation.numeric.integer_number",
    "representation.numeric.decimal_number",
    "representation.numeric.increment",
    "representation.discrete.categorical",
    "datetime.component.day_of_month",
    // Username/phone are common catch-alls for unrecognized text
    "identity.person.username",
    "identity.person.first_name",
    "identity.person.phone_number",
    // IATA is the model's default for uppercase 3-letter codes
    "geography.transportation.iata_code",
];

/// Determine whether a prediction should be treated as "generic" — i.e.,
/// a type the CharCNN cannot reliably distinguish from character patterns,
/// so it should defer to header hints when available.
///
/// Uses four signals — any match returns `true`:
/// 1. Attractor-demoted predictions are always generic (already uncertain).
/// 2. Boolean types are always generic.
/// 3. Hardcoded list of known catch-all labels (always applies).
/// 4. When taxonomy is available, broad designations (BroadWords, BroadCharacters,
///    BroadNumbers, BroadObject) are additionally generic — the CharCNN cannot
///    reliably distinguish these types from character patterns (NNFT-139).
///
/// Signal 4 is **additive**: it expands the generic set beyond the hardcoded
/// list (e.g., `gender`, `occupation` become generic via their `broad_words`
/// designation) but never removes types that are already in the hardcoded list.
fn is_generic_prediction(
    label: &str,
    disambiguation_rule: &Option<String>,
    taxonomy: Option<&Taxonomy>,
) -> bool {
    // Signal 1: Attractor-demoted predictions are inherently uncertain —
    // they should yield to header hints the same way generic types do.
    if disambiguation_rule
        .as_ref()
        .is_some_and(|r| r.starts_with("attractor_demotion"))
    {
        return true;
    }

    // Signal 2: Boolean types are always generic.
    if BOOLEAN_LABELS.contains(&label) {
        return true;
    }

    // Signal 3: Hardcoded list — always applies regardless of taxonomy.
    if HARDCODED_GENERIC_LABELS.contains(&label) {
        return true;
    }

    // Signal 4: Designation-aware expansion (NNFT-139).
    // When the taxonomy is available, broad designations mark types that the
    // CharCNN cannot reliably distinguish from character patterns alone.
    // This is ADDITIVE — it catches types like gender, occupation, nationality
    // that aren't in the hardcoded list but are still too ambiguous.
    if let Some(taxonomy) = taxonomy {
        if let Some(def) = taxonomy.get(label) {
            return matches!(
                def.designation,
                Designation::BroadWords
                    | Designation::BroadCharacters
                    | Designation::BroadNumbers
                    | Designation::BroadObject
            );
        }
    }

    false
}

/// Detect the most likely locale for a column by running sample values against
/// each locale's validation pattern from `validation_by_locale` (NNFT-140).
///
/// Returns the locale code with the highest pass rate above 50%, or None if
/// no locale patterns exist or none reach the threshold.
///
/// This implements post-hoc locale detection (decision-002, Option B):
/// the type classifier determines WHAT the data is (phone_number, postal_code),
/// then validation patterns determine WHERE it's from (EN_US, EN_GB, DE).
fn detect_locale_from_validation(
    values: &[String],
    label: &str,
    taxonomy: &Taxonomy,
) -> Option<String> {
    let locale_validators = taxonomy.get_locale_validators(label)?;

    // Count non-empty values for calculating pass rates
    let non_empty: Vec<&str> = values
        .iter()
        .map(|s| s.as_str())
        .filter(|s| !s.is_empty())
        .collect();
    if non_empty.is_empty() {
        return None;
    }

    let mut best_locale: Option<String> = None;
    let mut best_pass_rate: f32 = 0.0;

    for (locale, validator) in locale_validators {
        let pass_count = non_empty
            .iter()
            .filter(|v| validator.validate(v).is_valid)
            .count();
        let pass_rate = pass_count as f32 / non_empty.len() as f32;

        if pass_rate > best_pass_rate {
            best_pass_rate = pass_rate;
            best_locale = Some(locale.clone());
        }
    }

    // Only report locale if >50% of values match the pattern
    if best_pass_rate > 0.5 {
        best_locale
    } else {
        None
    }
}

/// Configuration for column-mode inference.
#[derive(Debug, Clone)]
pub struct ColumnConfig {
    /// Maximum number of values to sample from the column (default: 100).
    pub sample_size: usize,
    /// Minimum fraction of votes a type needs to be the winner (default: 0.3).
    /// If no type reaches this threshold, the result confidence is lowered.
    pub min_agreement: f32,
}

impl Default for ColumnConfig {
    fn default() -> Self {
        Self {
            sample_size: 100,
            min_agreement: 0.3,
        }
    }
}

/// Result of column-mode inference.
#[derive(Debug, Clone)]
pub struct ColumnResult {
    /// The predicted type label for the column (3-level: domain.category.type).
    pub label: String,
    /// Confidence score (0.0 to 1.0).
    pub confidence: f32,
    /// Vote distribution: label → fraction of samples classified as this type.
    /// Labels are 3-level (locale suffixes collapsed).
    pub vote_distribution: Vec<(String, f32)>,
    /// Whether a disambiguation rule was applied to override the majority vote.
    pub disambiguation_applied: bool,
    /// Name of the disambiguation rule applied, if any.
    pub disambiguation_rule: Option<String>,
    /// Number of values actually classified.
    pub samples_used: usize,
    /// Detected locale for the column, if the winning type is locale-specific.
    /// e.g., "EN_US", "FR_FR", "UNIVERSAL". None if the model was trained
    /// without locale labels or the type has no locale variants.
    pub detected_locale: Option<String>,
}

/// Column-mode classifier that wraps a single-value classifier.
///
/// Accepts any `ValueClassifier` implementation (CharClassifier, TieredClassifier, etc.)
/// via `Box<dyn ValueClassifier>`.
pub struct ColumnClassifier {
    classifier: Box<dyn ValueClassifier>,
    config: ColumnConfig,
    /// Optional semantic column name classifier (Model2Vec embeddings).
    /// When present, used as the primary header hint source before falling
    /// back to the hardcoded `header_hint()` dictionary.
    semantic_hint: Option<SemanticHintClassifier>,
    /// Optional taxonomy for validation-based attractor demotion.
    /// When present, enables Signal 1 (validation failure) in the
    /// attractor demotion disambiguation rule (Rule 14).
    taxonomy: Option<Taxonomy>,
}

impl ColumnClassifier {
    /// Create a new column classifier wrapping any ValueClassifier.
    pub fn new(classifier: Box<dyn ValueClassifier>, config: ColumnConfig) -> Self {
        Self {
            classifier,
            config,
            semantic_hint: None,
            taxonomy: None,
        }
    }

    /// Create with default configuration.
    pub fn with_defaults(classifier: Box<dyn ValueClassifier>) -> Self {
        Self::new(classifier, ColumnConfig::default())
    }

    /// Create a column classifier with a semantic hint classifier.
    ///
    /// The semantic classifier uses Model2Vec embeddings to map column names
    /// to type labels, replacing the hardcoded header_hint() dictionary.
    /// Falls back to header_hint() when the semantic classifier doesn't match.
    pub fn with_semantic_hint(
        classifier: Box<dyn ValueClassifier>,
        config: ColumnConfig,
        semantic: SemanticHintClassifier,
    ) -> Self {
        Self {
            classifier,
            config,
            semantic_hint: Some(semantic),
            taxonomy: None,
        }
    }

    /// Attach a semantic hint classifier to an existing ColumnClassifier.
    pub fn set_semantic_hint(&mut self, semantic: SemanticHintClassifier) {
        self.semantic_hint = Some(semantic);
    }

    /// Attach a taxonomy for validation-based attractor demotion.
    ///
    /// When the taxonomy is present, the attractor demotion rule (Rule 14)
    /// can validate predicted type values against their validation schemas,
    /// enabling Signal 1 (validation failure) to catch over-eager predictions.
    pub fn set_taxonomy(&mut self, taxonomy: Taxonomy) {
        self.taxonomy = Some(taxonomy);
    }

    /// Classify a column of values, returning a single type prediction.
    ///
    /// The algorithm:
    /// 1. Sample up to `config.sample_size` values
    /// 2. Run single-value inference on each
    /// 3. Aggregate votes by predicted label
    /// 4. Apply disambiguation rules for known ambiguous pairs
    /// 5. Return the final label with confidence
    pub fn classify_column(&self, values: &[String]) -> Result<ColumnResult, InferenceError> {
        if values.is_empty() {
            return Ok(ColumnResult {
                label: "unknown".to_string(),
                confidence: 0.0,
                vote_distribution: vec![],
                disambiguation_applied: false,
                disambiguation_rule: None,
                samples_used: 0,
                detected_locale: None,
            });
        }

        // Step 1: Sample values
        let sample = if values.len() <= self.config.sample_size {
            values.to_vec()
        } else {
            // Deterministic sampling: evenly spaced
            let step = values.len() as f64 / self.config.sample_size as f64;
            (0..self.config.sample_size)
                .map(|i| values[(i as f64 * step) as usize].clone())
                .collect()
        };

        let n_samples = sample.len();

        // Step 2: Run batch inference
        let results = self.classifier.classify_batch(&sample)?;

        // Step 3: Aggregate votes — collapse 4-level locale labels to 3-level
        // Track both 3-level type votes and locale distribution within each type.
        let mut vote_counts_3level: HashMap<String, usize> = HashMap::new();
        let mut locale_votes: HashMap<String, HashMap<String, usize>> = HashMap::new(); // 3-level → locale → count
        for result in &results {
            let (base_label, locale) = strip_locale_suffix(&result.label);
            *vote_counts_3level
                .entry(base_label.to_string())
                .or_default() += 1;
            if let Some(loc) = locale {
                *locale_votes
                    .entry(base_label.to_string())
                    .or_default()
                    .entry(loc.to_string())
                    .or_default() += 1;
            }
        }

        // Sort by count descending (3-level labels)
        let mut votes: Vec<(String, usize)> = vote_counts_3level.into_iter().collect();
        votes.sort_by(|a, b| b.1.cmp(&a.1));

        let vote_distribution: Vec<(String, f32)> = votes
            .iter()
            .map(|(label, count)| (label.clone(), *count as f32 / n_samples as f32))
            .collect();

        // Majority winner (3-level)
        let (majority_label, majority_count) = votes.first().cloned().unwrap_or_default();
        let majority_fraction = majority_count as f32 / n_samples as f32;

        // Determine dominant locale for the winning type
        let detected_locale = locale_votes.get(&majority_label).and_then(|locales| {
            locales
                .iter()
                .max_by_key(|(_, count)| *count)
                .map(|(locale, _)| locale.clone())
                .filter(|l| l != "UNIVERSAL") // Don't report UNIVERSAL as a locale
        });

        // Step 4: Apply disambiguation rules (operates on 3-level labels)
        let disambiguation =
            disambiguate(&sample, &results, &votes, n_samples, self.taxonomy.as_ref());

        let mut result = if let Some((label, rule_name)) = disambiguation {
            // Disambiguation may change the winning label — re-derive locale if needed
            let disambig_locale = locale_votes.get(&label).and_then(|locales| {
                locales
                    .iter()
                    .max_by_key(|(_, count)| *count)
                    .map(|(locale, _)| locale.clone())
                    .filter(|l| l != "UNIVERSAL")
            });
            // Attractor demotion rules get moderate confidence; all others get high confidence
            let confidence = if rule_name.starts_with("attractor_demotion") {
                majority_fraction.max(0.5)
            } else {
                majority_fraction.max(0.8) // Disambiguation rules are high-confidence
            };
            ColumnResult {
                label,
                confidence,
                vote_distribution,
                disambiguation_applied: true,
                disambiguation_rule: Some(rule_name),
                samples_used: n_samples,
                detected_locale: disambig_locale,
            }
        } else {
            // No disambiguation needed — use majority vote
            let confidence = if majority_fraction >= self.config.min_agreement {
                majority_fraction
            } else {
                majority_fraction * 0.5 // Low agreement → low confidence
            };

            ColumnResult {
                label: majority_label,
                confidence,
                vote_distribution,
                disambiguation_applied: false,
                disambiguation_rule: None,
                samples_used: n_samples,
                detected_locale,
            }
        };

        // Step 5: Post-hoc locale detection via validation patterns (NNFT-140).
        // When taxonomy is available, run sample values against validation_by_locale
        // patterns to detect the most likely locale. This takes priority over any
        // model-derived locale from vote aggregation, because validation patterns
        // are precise structural rules (see Precision Principle, decision-002).
        if let Some(taxonomy) = self.taxonomy.as_ref() {
            if let Some(locale) = detect_locale_from_validation(&sample, &result.label, taxonomy) {
                result.detected_locale = Some(locale);
            }
        }

        Ok(result)
    }

    /// Get a reference to the underlying classifier.
    pub fn classifier(&self) -> &dyn ValueClassifier {
        &*self.classifier
    }

    /// Classify a column of values with an optional header name hint.
    ///
    /// The header name (e.g., "Age", "Email", "zip_code") provides a soft signal
    /// that can adjust the final prediction. The hint never overrides a high-confidence
    /// model prediction — it only boosts a candidate type when the model is uncertain.
    pub fn classify_column_with_header(
        &self,
        values: &[String],
        header: &str,
    ) -> Result<ColumnResult, InferenceError> {
        let mut result = self.classify_column(values)?;

        // Apply header hint: try semantic classifier first, fall back to hardcoded
        let hinted_type: Option<String> = self
            .semantic_hint
            .as_ref()
            .and_then(|sh| sh.classify_header(header))
            .map(|r| r.label)
            .or_else(|| header_hint(header).map(String::from));

        if let Some(hinted_type) = hinted_type.as_deref() {
            // If the model already predicts the hinted type, just boost confidence
            if result.label == hinted_type {
                result.confidence = (result.confidence + 0.1).min(1.0);
                return Ok(result);
            }

            // Measurement disambiguation: age, height, and weight values are
            // numerically indistinguishable (all small integers in overlapping
            // ranges). When the header provides a specific measurement hint,
            // trust it over the model prediction.
            const MEASUREMENT_TYPES: &[&str] = &[
                "identity.person.age",
                "identity.person.height",
                "identity.person.weight",
            ];
            if MEASUREMENT_TYPES.contains(&hinted_type)
                && MEASUREMENT_TYPES.contains(&result.label.as_str())
            {
                result.label = hinted_type.to_string();
                result.confidence = 0.9;
                result.disambiguation_applied = true;
                result.disambiguation_rule =
                    Some(format!("header_hint_measurement:{}", header.to_lowercase()));
                return Ok(result);
            }

            // Check if the hinted type is in the vote distribution
            let hint_in_votes = result
                .vote_distribution
                .iter()
                .any(|(label, _)| label == hinted_type);

            // Only override if model confidence is low (< 0.5)
            // or the result is a generic type AND the hint matches a candidate.
            //
            // Designation-aware gating (NNFT-139): when taxonomy is available,
            // types with broad designations (broad_words, broad_characters,
            // broad_numbers, broad_object) are treated as generic because the
            // CharCNN cannot reliably distinguish them from character patterns
            // alone. Falls back to a hardcoded list when taxonomy is unavailable.
            let is_generic = is_generic_prediction(
                &result.label,
                &result.disambiguation_rule,
                self.taxonomy.as_ref(),
            );

            // Geography protection: when the hint is full_name, check if the
            // model sees geography.location signal. Many geographic datasets
            // use "name" as a header for city, country, or region columns.
            // The model often correctly identifies the geography type but the
            // full_name hint would override it.
            if hinted_type == "identity.person.full_name" {
                const LOCATION_TYPES: &[&str] = &[
                    "geography.location.city",
                    "geography.location.country",
                    "geography.location.region",
                    "geography.location.state",
                    "geography.location.continent",
                ];

                // Case 1: Model already predicts a location type — keep it
                // rather than overriding to full_name.
                if LOCATION_TYPES.contains(&result.label.as_str()) {
                    result.confidence = result.confidence.max(0.5);
                    result.disambiguation_applied = true;
                    result.disambiguation_rule = Some(format!(
                        "header_hint_location_keep:{}",
                        header.to_lowercase()
                    ));
                    return Ok(result);
                }

                // Case 2: Prediction was demoted to generic but geography
                // votes exist — pick the top geography type.
                if is_generic {
                    let top_location = result
                        .vote_distribution
                        .iter()
                        .filter(|(label, _)| LOCATION_TYPES.contains(&label.as_str()))
                        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
                    if let Some((loc_label, loc_frac)) = top_location {
                        if *loc_frac >= 0.10 {
                            result.label = loc_label.clone();
                            result.confidence = loc_frac.max(0.5);
                            result.disambiguation_applied = true;
                            result.disambiguation_rule =
                                Some(format!("header_hint_location:{}", header.to_lowercase()));
                            return Ok(result);
                        }
                    }
                }
            }

            let original_label = result.label.clone();

            if (result.confidence < 0.5 || is_generic) && hint_in_votes {
                let hint_fraction = result
                    .vote_distribution
                    .iter()
                    .find(|(label, _)| label == hinted_type)
                    .map(|(_, frac)| *frac)
                    .unwrap_or(0.0);

                result.label = hinted_type.to_string();
                result.confidence = hint_fraction.max(0.6);
                result.disambiguation_applied = true;
                result.disambiguation_rule = Some(format!("header_hint:{}", header.to_lowercase()));
            } else if is_generic && !hint_in_votes {
                // Generic prediction (integer, username, etc.) + header hint:
                // trust the header even when the model doesn't vote for the
                // hinted type — the header name is a strong enough signal
                result.label = hinted_type.to_string();
                result.confidence = 0.5;
                result.disambiguation_applied = true;
                result.disambiguation_rule =
                    Some(format!("header_hint_generic:{}", header.to_lowercase()));
            } else if result.confidence < 0.3 && !hint_in_votes {
                // Very low confidence and hint type not even in votes —
                // still apply hint but with low confidence
                result.label = hinted_type.to_string();
                result.confidence = 0.4;
                result.disambiguation_applied = true;
                result.disambiguation_rule =
                    Some(format!("header_hint_fallback:{}", header.to_lowercase()));
            }

            // If header hint changed the label, clear stale locale detection
            // (NNFT-140). The detected_locale from classify_column was for the
            // original label — it's invalid for the new label. We can't re-detect
            // here because we don't have the raw sample values; locale will be
            // None for header-hint-overridden types. This is conservative and
            // correct: better no locale than a wrong locale.
            if result.label != original_label {
                result.detected_locale = None;
            }
        }

        Ok(result)
    }

    /// Get a reference to the configuration.
    pub fn config(&self) -> &ColumnConfig {
        &self.config
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// DISAMBIGUATION RULES
// ═══════════════════════════════════════════════════════════════════════════════

/// Disambiguation rule pairs: types that are ambiguous in single-value mode.
const DATE_SLASH_PAIR: (&str, &str) = ("datetime.date.us_slash", "datetime.date.eu_slash");

const SHORT_DATE_PAIR: (&str, &str) = ("datetime.date.short_mdy", "datetime.date.short_dmy");

const COORDINATE_PAIR: (&str, &str) = (
    "geography.coordinate.latitude",
    "geography.coordinate.longitude",
);

/// Attractor types — types the CharCNN over-confidently assigns to generic data.
/// Numeric attractors catch integers misclassified as postal codes, CVVs, etc.
const NUMERIC_ATTRACTORS: &[&str] = &[
    "geography.address.postal_code",
    "geography.address.street_number",
    "identity.payment.cvv",
];

/// Text attractors catch short words/phrases misclassified as identity types.
/// Note: full_name is NOT included — its false positives are rare (2 in eval)
/// and the header hint system handles them. Including full_name causes more
/// regressions (company, venue, publisher columns whose GT maps to "name"→full_name).
/// phone_number is included here (not NUMERIC) because phone strings contain
/// formatting characters (+, parens, hyphens, spaces). Locale validation via
/// validation_by_locale confirms real phone columns; non-phone data is demoted.
const TEXT_ATTRACTORS: &[&str] = &[
    "identity.person.first_name",
    "identity.person.phone_number",
    "identity.person.username",
    "geography.address.street_name",
];

/// Code attractors catch alphanumeric codes misclassified as specific identifiers.
const CODE_ATTRACTORS: &[&str] = &[
    "geography.transportation.icao_code",
    "identity.medical.ndc",
    "identity.payment.cusip",
    "technology.internet.top_level_domain",
];

/// Apply disambiguation rules when the vote distribution contains known ambiguous pairs.
///
/// Returns Some((resolved_label, rule_name)) if a rule was applied, None otherwise.
fn disambiguate(
    values: &[String],
    results: &[ClassificationResult],
    votes: &[(String, usize)],
    n_samples: usize,
    taxonomy: Option<&Taxonomy>,
) -> Option<(String, String)> {
    // Get the top labels in the vote
    let top_labels: Vec<&str> = votes.iter().take(3).map(|(l, _)| l.as_str()).collect();

    // Rule 1: Date slash disambiguation (us_slash vs eu_slash)
    if contains_pair(&top_labels, DATE_SLASH_PAIR.0, DATE_SLASH_PAIR.1) {
        if let Some(label) = disambiguate_slash_dates(values) {
            return Some((label, "date_slash_disambiguation".to_string()));
        }
    }

    // Rule 2: Short date disambiguation (short_mdy vs short_dmy)
    if contains_pair(&top_labels, SHORT_DATE_PAIR.0, SHORT_DATE_PAIR.1) {
        if let Some(label) = disambiguate_short_dates(values) {
            return Some((label, "short_date_disambiguation".to_string()));
        }
    }

    // Rule 3: Coordinate disambiguation (latitude vs longitude)
    if contains_pair(&top_labels, COORDINATE_PAIR.0, COORDINATE_PAIR.1) {
        if let Some(label) = disambiguate_coordinates(values) {
            return Some((label, "coordinate_disambiguation".to_string()));
        }
    }

    // Rule 4: IPv4 address detection (dotted-quad pattern)
    if let Some(label) = disambiguate_ipv4(values) {
        return Some((label, "ipv4_detection".to_string()));
    }

    // Rule 5: Day-of-week name detection (Monday, Tuesday, etc.)
    if let Some(label) = disambiguate_day_of_week(values) {
        return Some((label, "day_of_week_name_detection".to_string()));
    }

    // Rule 6: Month name detection (January, February, etc.)
    if let Some(label) = disambiguate_month_name(values) {
        return Some((label, "month_name_detection".to_string()));
    }

    // Rule 7: Boolean sub-type normalization (binary/terms/initials)
    if let Some((label, rule)) = disambiguate_boolean_subtype(values, &top_labels) {
        return Some((label, rule));
    }

    // Rule 8: Gender detection (must be before generic categorical)
    if let Some(label) = disambiguate_gender(values) {
        return Some((label, "gender_detection".to_string()));
    }

    // Rule 9: Boolean override — prevent boolean classification for small integer spreads
    if let Some((label, rule)) = disambiguate_boolean_override(values, &top_labels) {
        return Some((label, rule));
    }

    // Rule 10: Small-integer ordinal detection — override day_of_month for
    // columns where all values are small positive integers (e.g. Pclass: 1,2,3)
    if let Some((label, rule)) = disambiguate_small_integer_ordinal(values, &top_labels) {
        return Some((label, rule));
    }

    // Rule 11: Categorical detection — low cardinality string columns
    if let Some((label, rule)) = disambiguate_categorical(values, &top_labels) {
        return Some((label, rule));
    }

    // Rule 12: Numeric type disambiguation
    if let Some((label, rule)) = disambiguate_numeric(values, results, &top_labels) {
        return Some((label, rule));
    }

    // Rule 13: SI number override — if the top vote is si_number but no sampled
    // values contain an SI suffix (K, M, B, T, G, etc.), the model confused
    // plain decimals for SI numbers. Override to decimal_number.
    if top_labels
        .first()
        .is_some_and(|l| *l == "representation.numeric.si_number")
    {
        if let Some((label, rule)) = disambiguate_si_number(values) {
            return Some((label, rule));
        }
    }

    // Rule 14: Duration override — ISO 8601 durations (PT20M, P1DT12H)
    // misclassified as SEDOL stock codes because both are 5-8 char alphanumeric
    // strings starting with uppercase letters. Check for duration pattern before
    // attractor demotion would demote SEDOL to alphanumeric_id (losing duration).
    if top_labels
        .first()
        .is_some_and(|l| *l == "identity.payment.sedol")
    {
        if let Some((label, rule)) = disambiguate_duration_override(values) {
            return Some((label, rule));
        }
    }

    // Rule 15: Attractor type demotion — demote over-eager specific type
    // predictions (postal_code, cvv, first_name, etc.) when evidence doesn't
    // support the specific prediction. Three signals: validation failure,
    // confidence threshold, and cardinality mismatch.
    if let Some((label, rule)) = disambiguate_attractor_demotion(values, votes, n_samples, taxonomy)
    {
        return Some((label, rule));
    }

    // Rule 16: Text length demotion — full_address predictions where
    // the median value length exceeds 100 characters are almost certainly
    // free-form text (descriptions, paragraphs, recipe steps) rather than
    // street addresses. Demote to representation.text.sentence.
    // Threshold 100 gives 0% false demotion rate on SOTAB evaluation data.
    if let Some((label, rule)) = disambiguate_text_length_demotion(values, votes) {
        return Some((label, rule));
    }

    None
}

/// Check if two labels are both present in the top candidates.
fn contains_pair(labels: &[&str], a: &str, b: &str) -> bool {
    labels.contains(&a) && labels.contains(&b)
}

/// Detect day-of-week columns where values are day names (Monday, Tuesday, etc.).
///
/// Rule: If ≥80% of non-empty values are recognized day names → datetime.component.day_of_week
fn disambiguate_day_of_week(values: &[String]) -> Option<String> {
    const DAY_NAMES: &[&str] = &[
        "monday",
        "tuesday",
        "wednesday",
        "thursday",
        "friday",
        "saturday",
        "sunday",
        "mon",
        "tue",
        "wed",
        "thu",
        "fri",
        "sat",
        "sun",
        "mo",
        "tu",
        "we",
        "th",
        "fr",
        "sa",
        "su",
    ];

    let non_empty: Vec<String> = values
        .iter()
        .map(|v| v.trim().to_lowercase())
        .filter(|v| !v.is_empty())
        .collect();

    if non_empty.len() < 3 {
        return None;
    }

    let matching = non_empty
        .iter()
        .filter(|v| DAY_NAMES.contains(&v.as_str()))
        .count();
    let fraction = matching as f64 / non_empty.len() as f64;

    if fraction >= 0.8 {
        Some("datetime.component.day_of_week".to_string())
    } else {
        None
    }
}

/// Detect month-name columns where values are month names (January, February, etc.).
///
/// Rule: If ≥80% of non-empty values are recognized month names → datetime.component.month_name
fn disambiguate_month_name(values: &[String]) -> Option<String> {
    const MONTH_NAMES: &[&str] = &[
        "january",
        "february",
        "march",
        "april",
        "may",
        "june",
        "july",
        "august",
        "september",
        "october",
        "november",
        "december",
        "jan",
        "feb",
        "mar",
        "apr",
        "jun",
        "jul",
        "aug",
        "sep",
        "oct",
        "nov",
        "dec",
    ];

    let non_empty: Vec<String> = values
        .iter()
        .map(|v| v.trim().to_lowercase())
        .filter(|v| !v.is_empty())
        .collect();

    if non_empty.len() < 3 {
        return None;
    }

    let matching = non_empty
        .iter()
        .filter(|v| MONTH_NAMES.contains(&v.as_str()))
        .count();
    let fraction = matching as f64 / non_empty.len() as f64;

    if fraction >= 0.8 {
        Some("datetime.component.month_name".to_string())
    } else {
        None
    }
}

/// Normalize boolean sub-types based on actual value content.
///
/// When the top prediction or any boolean label appears in the top 3 votes,
/// examine the actual values to determine the correct boolean sub-type:
/// - 0/1 integers → `representation.boolean.binary`
/// - true/false/yes/no/on/off text → `representation.boolean.terms`
/// - T/F/Y/N single characters → `representation.boolean.initials`
///
/// Also detects boolean-valued columns that were misclassified as non-boolean
/// types (e.g., categorical), overriding when ≥80% of values match.
fn disambiguate_boolean_subtype(
    values: &[String],
    top_labels: &[&str],
) -> Option<(String, String)> {
    let non_empty: Vec<&str> = values
        .iter()
        .map(|v| v.trim())
        .filter(|v| !v.is_empty())
        .collect();

    if non_empty.len() < 3 {
        return None;
    }

    // Check if values are boolean-like
    let binary_values: &[&str] = &["0", "1"];
    let terms_values: &[&str] = &[
        "true", "false", "True", "False", "TRUE", "FALSE", "yes", "no", "Yes", "No", "YES", "NO",
        "on", "off", "On", "Off", "ON", "OFF",
    ];
    let initials_values: &[&str] = &["T", "F", "t", "f", "Y", "N", "y", "n"];

    let binary_count = non_empty
        .iter()
        .filter(|v| binary_values.contains(v))
        .count();
    let terms_count = non_empty
        .iter()
        .filter(|v| terms_values.contains(v))
        .count();
    let initials_count = non_empty
        .iter()
        .filter(|v| initials_values.contains(v))
        .count();

    let n = non_empty.len() as f64;
    let binary_frac = binary_count as f64 / n;
    let terms_frac = terms_count as f64 / n;
    let initials_frac = initials_count as f64 / n;

    // Only fire if a boolean type is in the top predictions, OR if the values
    // themselves are overwhelmingly boolean (catches cases where model predicted
    // categorical/other for True/False columns)
    let has_boolean_vote = top_labels.iter().any(|l| BOOLEAN_LABELS.contains(l));
    let max_frac = binary_frac.max(terms_frac).max(initials_frac);

    if !has_boolean_vote && max_frac < 0.8 {
        return None;
    }

    // Pick the best matching sub-type (must have ≥80% of values matching)
    //
    // For binary (0/1), also require ≤2 unique values to avoid false positives
    // on skewed integer columns (e.g. SibSp: mostly 0s and 1s, but range 0-8).
    if terms_frac >= 0.8 {
        Some((
            "representation.boolean.terms".to_string(),
            "boolean_subtype_terms".to_string(),
        ))
    } else if initials_frac >= 0.8 {
        Some((
            "representation.boolean.initials".to_string(),
            "boolean_subtype_initials".to_string(),
        ))
    } else if binary_frac >= 0.8 {
        let unique_values: std::collections::HashSet<&str> = non_empty.iter().copied().collect();
        if unique_values.len() <= 2 {
            Some((
                "representation.boolean.binary".to_string(),
                "boolean_subtype_binary".to_string(),
            ))
        } else {
            None
        }
    } else {
        None
    }
}

/// Detect gender columns by checking if all values match a known gender value set.
///
/// Rule: If ALL non-empty values are in the gender set → identity.person.gender
fn disambiguate_gender(values: &[String]) -> Option<String> {
    const GENDER_VALUES: &[&str] = &[
        "male",
        "female",
        "m",
        "f",
        "Male",
        "Female",
        "M",
        "F",
        "MALE",
        "FEMALE",
        "man",
        "woman",
        "Man",
        "Woman",
        "MAN",
        "WOMAN",
        "boy",
        "girl",
        "Boy",
        "Girl",
        // Inclusive gender values
        "non-binary",
        "Non-binary",
        "Non-Binary",
        "NON-BINARY",
        "nonbinary",
        "Nonbinary",
        "other",
        "Other",
        "OTHER",
        "prefer not to say",
        "Prefer not to say",
        "unknown",
        "Unknown",
        "UNKNOWN",
        "x",
        "X",
        "genderqueer",
        "Genderqueer",
        "agender",
        "Agender",
        "transgender",
        "Transgender",
    ];

    let non_empty: Vec<&str> = values
        .iter()
        .map(|v| v.trim())
        .filter(|v| !v.is_empty())
        .collect();

    if non_empty.len() < 3 {
        return None;
    }

    let all_gender = non_empty.iter().all(|v| GENDER_VALUES.contains(v));
    if all_gender {
        Some("identity.person.gender".to_string())
    } else {
        None
    }
}

/// Override boolean classification when the column has small integer values
/// with more than 2 unique values and a spread > 1.
///
/// Rule: If majority vote is boolean but values are integers with >2 unique values
///       spanning 0-N where N > 1, override to integer_number.
fn disambiguate_boolean_override(
    values: &[String],
    top_labels: &[&str],
) -> Option<(String, String)> {
    // Only trigger when boolean is in the top predictions
    let has_boolean = top_labels.iter().any(|l| BOOLEAN_LABELS.contains(l));
    if !has_boolean {
        return None;
    }

    let non_empty: Vec<&str> = values
        .iter()
        .map(|v| v.trim())
        .filter(|v| !v.is_empty())
        .collect();
    if non_empty.len() < 3 {
        return None;
    }

    // Check single-character non-numeric values first (e.g., Embarked: S, C, Q)
    let all_single_char = non_empty.iter().all(|v| v.chars().count() == 1);
    let all_digits = non_empty
        .iter()
        .all(|v| v.chars().all(|c| c.is_ascii_digit()));
    if all_single_char && !all_digits {
        let mut unique_chars: Vec<&str> = non_empty.clone();
        unique_chars.sort();
        unique_chars.dedup();
        if unique_chars.len() >= 2 {
            // Single chars that aren't just 0/1 or T/F → categorical
            let is_boolean_set = unique_chars.len() == 2 && {
                let set: std::collections::HashSet<&str> = unique_chars.iter().copied().collect();
                set.contains("0") && set.contains("1")
                    || set.contains("T") && set.contains("F")
                    || set.contains("t") && set.contains("f")
                    || set.contains("Y") && set.contains("N")
                    || set.contains("y") && set.contains("n")
            };
            if !is_boolean_set {
                return Some((
                    "representation.discrete.categorical".to_string(),
                    "boolean_override_single_char_categorical".to_string(),
                ));
            }
        }
    }

    // Parse values as integers — check for small integer spread
    let parsed: Vec<i64> = values
        .iter()
        .filter_map(|v| v.trim().parse::<i64>().ok())
        .collect();

    if parsed.len() >= 3 {
        let mut unique: Vec<i64> = parsed.clone();
        unique.sort();
        unique.dedup();
        let n_unique = unique.len();
        let min = *unique.first().unwrap();
        let max = *unique.last().unwrap();

        // If >2 unique integer values and spread > 1, it's not boolean
        if n_unique > 2 && (max - min) > 1 {
            return Some((
                "representation.numeric.integer_number".to_string(),
                "boolean_override_integer_spread".to_string(),
            ));
        }
    }

    None
}

/// Override day_of_month or similar classifications for small-integer columns
/// that look like ordinal/class labels (e.g. Pclass: 1, 2, 3).
///
/// Rule: If values are all small positive integers with ≤10 unique values,
///       the range is small (max ≤ 20), and the top prediction is day_of_month
///       or another misfit type, override to ordinal.
fn disambiguate_small_integer_ordinal(
    values: &[String],
    top_labels: &[&str],
) -> Option<(String, String)> {
    // Only trigger when day_of_month or generic integer types lead the vote
    let misfit_types = [
        "datetime.component.day_of_month",
        "representation.numeric.integer_number",
        "representation.numeric.increment",
    ];
    let top_is_misfit = top_labels
        .first()
        .map(|l| misfit_types.contains(l))
        .unwrap_or(false);
    if !top_is_misfit {
        return None;
    }

    let parsed: Vec<i64> = values
        .iter()
        .filter_map(|v| v.trim().parse::<i64>().ok())
        .collect();

    if parsed.len() < 3 {
        return None;
    }

    let mut unique: Vec<i64> = parsed.clone();
    unique.sort();
    unique.dedup();
    let n_unique = unique.len();
    let min = *unique.first().unwrap();
    let max = *unique.last().unwrap();

    // Ordinal pattern: small set of small positive integers
    // e.g. {1,2,3} for Pclass, {1,2,3,4,5} for ratings
    if (2..=10).contains(&n_unique) && min >= 0 && max <= 20 {
        // Exclude pure boolean (only {0,1})
        if n_unique == 2 && min == 0 && max == 1 {
            return None;
        }
        // Exclude sequential ranges that look like increments (1..N where N matches sample count)
        // Only classify as ordinal if there's repetition (i.e. fewer unique than total samples)
        if n_unique < parsed.len() {
            return Some((
                "representation.discrete.ordinal".to_string(),
                "small_integer_ordinal".to_string(),
            ));
        }
    }

    None
}

/// Detect categorical columns based on cardinality and value characteristics.
///
/// Rules:
/// - All values are single characters with > 2 unique → categorical
/// - 3-20 unique string values → categorical
fn disambiguate_categorical(values: &[String], top_labels: &[&str]) -> Option<(String, String)> {
    let non_empty: Vec<&str> = values
        .iter()
        .map(|v| v.trim())
        .filter(|v| !v.is_empty())
        .collect();

    if non_empty.len() < 3 {
        return None;
    }

    let mut unique_values: Vec<&str> = non_empty.clone();
    unique_values.sort();
    unique_values.dedup();
    let n_unique = unique_values.len();

    // All single-character values with > 2 unique → categorical
    // But not if all values are numeric digits (handled by numeric rules)
    if non_empty.iter().all(|v| v.chars().count() == 1) && n_unique > 2 {
        let all_digits = non_empty
            .iter()
            .all(|v| v.chars().all(|c| c.is_ascii_digit()));
        if !all_digits {
            return Some((
                "representation.discrete.categorical".to_string(),
                "categorical_single_char".to_string(),
            ));
        }
    }

    // Low cardinality string column: 3-20 unique values, not already categorical
    // Only override if the current top prediction is a generic type
    let mut generic_types = vec![
        "representation.text.word",
        "representation.text.plain_text",
        "representation.text.abbreviation",
        "representation.numeric.integer_number",
        "datetime.component.day_of_month",
    ];
    generic_types.extend_from_slice(BOOLEAN_LABELS);
    let top_is_generic = top_labels
        .first()
        .map(|l| generic_types.contains(l))
        .unwrap_or(false);

    if (3..=20).contains(&n_unique) && top_is_generic {
        // Check that values are short strings (not sentences)
        let all_short = non_empty.iter().all(|v| v.len() <= 50);
        // Check that values are not purely numeric (handled by numeric rules)
        let all_numeric = non_empty.iter().all(|v| v.parse::<f64>().is_ok());

        if all_short && !all_numeric {
            return Some((
                "representation.discrete.categorical".to_string(),
                "categorical_low_cardinality".to_string(),
            ));
        }
    }

    let _ = top_labels; // suppress warning
    None
}

// ═══════════════════════════════════════════════════════════════════════════════
// HEADER NAME HINTS
// ═══════════════════════════════════════════════════════════════════════════════

/// Map a column header name to a hinted type label.
///
/// Uses case-insensitive substring/keyword matching. Returns the most likely
/// type label for the column based on its name, or None if no match.
fn header_hint(header: &str) -> Option<&'static str> {
    let h = header.to_lowercase();
    // Remove common prefixes/suffixes and separators for matching
    let h = h.replace(['_', '-'], " ");
    let h = h.trim();

    // Exact or near-exact matches first (most specific)
    match h {
        "email" | "e mail" | "email address" | "emailaddress" => {
            return Some("identity.person.email");
        }
        "url" | "uri" | "link" | "href" | "website" | "homepage" => {
            return Some("technology.internet.url");
        }
        "ip" | "ip address" | "ipaddress" | "ip addr" | "source ip" | "destination ip"
        | "src ip" | "dst ip" | "server ip" | "client ip" | "remote ip" | "local ip" => {
            return Some("technology.internet.ip_v4");
        }
        "uuid" | "guid" => {
            return Some("technology.identifier.uuid");
        }
        "gender" | "sex" => {
            return Some("identity.person.gender");
        }
        "age" => {
            return Some("identity.person.age");
        }
        "latitude" | "lat" => {
            return Some("geography.coordinate.latitude");
        }
        "longitude" | "lng" | "lon" | "long" => {
            return Some("geography.coordinate.longitude");
        }
        "country" | "country name" => {
            return Some("geography.location.country");
        }
        "country code" | "alpha 2" | "alpha 3" | "iso country" | "iso alpha 2" | "iso alpha 3"
        | "country iso" => {
            return Some("geography.location.country_code");
        }
        "city" | "city name" => {
            return Some("geography.location.city");
        }
        "state" | "province" | "region" => {
            return Some("geography.location.state");
        }
        "currency" | "currency code" => {
            return Some("identity.financial.currency_code");
        }
        "port" => {
            return Some("technology.internet.port");
        }
        "id" | "identifier" => {
            return Some("representation.numeric.increment");
        }
        // Count / frequency columns — small integers representing quantities
        "sibsp" | "parch" | "siblings" | "parents" | "children" | "dependents" | "qty"
        | "quantity" => {
            return Some("representation.numeric.integer_number");
        }
        // Class / rank / tier columns — ordinal categories
        "class" | "pclass" | "grade" | "rank" | "level" | "tier" | "rating" | "priority"
        | "score" => {
            return Some("representation.discrete.ordinal");
        }
        // Survival / binary outcome columns
        "survived" | "alive" | "deceased" | "dead" | "active" | "enabled" | "disabled"
        | "deleted" | "verified" | "approved" | "flagged" => {
            return Some("representation.boolean.binary");
        }
        // UTC / timezone offset columns
        "utc offset" | "gmt offset" | "timezone offset" | "tz offset" | "utcoffset"
        | "gmtoffset" => {
            return Some("datetime.offset.utc");
        }
        // Financial code columns
        "cvv" | "cvc" | "security code" | "card security" => {
            return Some("identity.payment.cvv");
        }
        "swift" | "swift code" | "bic" | "bic code" | "swiftcode" | "biccode" => {
            return Some("identity.payment.swift_bic");
        }
        "issn" => {
            return Some("technology.code.issn");
        }
        // Medical identifiers
        "npi" | "npi number" => {
            return Some("identity.medical.npi");
        }
        "ean" | "barcode" | "gtin" | "upc" => {
            return Some("technology.code.ean");
        }
        // Operating system
        "os" | "operating system" | "platform" => {
            return Some("technology.development.os");
        }
        // Occupation / job title
        "occupation" | "job title" | "jobtitle" | "job" | "profession" | "role" | "position" => {
            return Some("identity.person.occupation");
        }
        // Subcountry / subregion → state/province level
        "subcountry" | "subregion" | "sub region" | "sub country" => {
            return Some("geography.location.state");
        }
        // Embarked / boarding columns — categorical
        "embarked" | "boarded" | "departed" | "terminal" | "gate" => {
            return Some("representation.discrete.categorical");
        }
        // Ticket / cabin — alphanumeric identifiers
        "ticket" | "ticket number" | "ticketno" => {
            return Some("representation.alphanumeric.alphanumeric_id");
        }
        "cabin" | "room" | "compartment" | "berth" | "seat" => {
            return Some("representation.alphanumeric.alphanumeric_id");
        }
        // Fare / fee columns
        "fare" | "fee" | "toll" | "charge" => {
            return Some("representation.numeric.decimal_number");
        }
        _ => {}
    }

    // Keyword/substring matching (less specific)
    if h.contains("email") || h.contains("e mail") {
        return Some("identity.person.email");
    }
    if h.contains("phone") || h.contains("tel") || h.contains("mobile") || h.contains("fax") {
        return Some("identity.person.phone_number");
    }
    // IP address — match " ip" suffix, "ip " prefix, or " ip " infix
    // (underscores already replaced with spaces, exact "ip" handled above)
    if h.ends_with(" ip") || h.starts_with("ip ") || h.contains(" ip ") {
        return Some("technology.internet.ip_v4");
    }
    if h.contains("zip") || h.contains("postal") || h.contains("postcode") {
        return Some("geography.address.postal_code");
    }
    if h.contains("name") && (h.contains("first") || h.contains("given")) {
        return Some("identity.person.first_name");
    }
    if h.contains("name") && (h.contains("last") || h.contains("family") || h.contains("sur")) {
        return Some("identity.person.last_name");
    }
    if h.contains("name") && (h.contains("full") || h.contains("complete") || h == "name") {
        return Some("identity.person.full_name");
    }
    if h == "name" || h.ends_with(" name") {
        return Some("identity.person.full_name");
    }
    if h.contains("address") && !h.contains("email") && !h.contains("ip") {
        return Some("geography.address.street_address");
    }
    if h.contains("street") {
        return Some("geography.address.street_address");
    }
    if h.contains("born") || h.contains("birth") || h.contains("dob") {
        return Some("datetime.date.iso_date");
    }
    if h.contains("date") || h.contains("timestamp") || h.contains("datetime") {
        return Some("datetime.timestamp.iso_8601");
    }
    if h.contains("year") {
        return Some("datetime.component.year");
    }
    if h.contains("weight") {
        return Some("identity.person.weight");
    }
    if h.contains("height") {
        return Some("identity.person.height");
    }
    if h.contains("password") || h.contains("passwd") {
        return Some("identity.credential.password");
    }
    if h.contains("url") || h.contains("uri") || h.contains("link") || h.contains("href") {
        return Some("technology.internet.url");
    }
    if h.contains("price") || h.contains("cost") || h.contains("amount") || h.contains("salary") {
        return Some("representation.numeric.decimal_number");
    }
    if h.contains("count") || h.contains("quantity") || h.contains("num") {
        return Some("representation.numeric.integer_number");
    }
    if h.contains("class") || h.contains("grade") || h.contains("rank") || h.contains("tier") {
        return Some("representation.discrete.ordinal");
    }
    if h.contains("ticket") || h.contains("cabin") || h.contains("seat") || h.contains("room") {
        return Some("representation.alphanumeric.alphanumeric_id");
    }
    if h.contains("fare") || h.contains("fee") || h.contains("charge") || h.contains("toll") {
        return Some("representation.numeric.decimal_number");
    }

    None
}

// ═══════════════════════════════════════════════════════════════════════════════
// DISAMBIGUATION RULES
// ═══════════════════════════════════════════════════════════════════════════════

/// Disambiguate us_slash vs eu_slash dates.
///
/// Pattern: `DD/MM/YYYY` or `MM/DD/YYYY`
/// Rule: If ANY value has first component > 12, it must be DD/MM (eu_slash).
///       If ANY value has second component > 12, it must be MM/DD (us_slash).
fn disambiguate_slash_dates(values: &[String]) -> Option<String> {
    let mut first_over_12 = false;
    let mut second_over_12 = false;

    for val in values {
        let parts: Vec<&str> = val.split('/').collect();
        if parts.len() >= 2 {
            if let Ok(first) = parts[0].parse::<u32>() {
                if first > 12 {
                    first_over_12 = true;
                }
            }
            if let Ok(second) = parts[1].parse::<u32>() {
                if second > 12 {
                    second_over_12 = true;
                }
            }
        }
    }

    if first_over_12 && !second_over_12 {
        // First component > 12 means it's the day → DD/MM/YYYY → eu_slash
        Some("datetime.date.eu_slash".to_string())
    } else if second_over_12 && !first_over_12 {
        // Second component > 12 means it's the day → MM/DD/YYYY → us_slash
        Some("datetime.date.us_slash".to_string())
    } else {
        // Both ambiguous or contradictory — let model decide
        None
    }
}

/// Disambiguate short_dmy vs short_mdy dates.
///
/// Pattern: `DD-MM-YY` or `MM-DD-YY`
/// Rule: Same as slash dates but with dash separator.
fn disambiguate_short_dates(values: &[String]) -> Option<String> {
    let mut first_over_12 = false;
    let mut second_over_12 = false;

    for val in values {
        let parts: Vec<&str> = val.split('-').collect();
        if parts.len() >= 2 {
            if let Ok(first) = parts[0].parse::<u32>() {
                if first > 12 {
                    first_over_12 = true;
                }
            }
            if let Ok(second) = parts[1].parse::<u32>() {
                if second > 12 {
                    second_over_12 = true;
                }
            }
        }
    }

    if first_over_12 && !second_over_12 {
        Some("datetime.date.short_dmy".to_string())
    } else if second_over_12 && !first_over_12 {
        Some("datetime.date.short_mdy".to_string())
    } else {
        None
    }
}

/// Disambiguate latitude vs longitude coordinates.
///
/// Rule: If ANY |value| > 90, it must be longitude (latitude max is 90).
///       If ALL |values| ≤ 90, it's likely latitude.
fn disambiguate_coordinates(values: &[String]) -> Option<String> {
    let mut any_over_90 = false;
    let mut all_parseable = true;
    let mut parsed_count = 0;

    for val in values {
        if let Ok(v) = val.trim().parse::<f64>() {
            parsed_count += 1;
            if v.abs() > 90.0 {
                any_over_90 = true;
            }
        } else {
            all_parseable = false;
        }
    }

    // Need at least some parseable values
    if parsed_count < 3 {
        return None;
    }

    if any_over_90 {
        Some("geography.coordinate.longitude".to_string())
    } else if all_parseable {
        // All values within [-90, 90] — likely latitude
        Some("geography.coordinate.latitude".to_string())
    } else {
        None
    }
}

/// Detect IPv4 addresses via dotted-quad pattern.
///
/// Rule: If ≥80% of non-empty values match `\d{1,3}.\d{1,3}.\d{1,3}.\d{1,3}`
/// with each octet in 0..255, classify as ip_v4.
///
/// This prevents the common confusion between IP addresses and version numbers
/// (e.g., "10.0.32.113" looks like a semver to the model).
fn disambiguate_ipv4(values: &[String]) -> Option<String> {
    let non_empty: Vec<&str> = values
        .iter()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    if non_empty.len() < 3 {
        return None;
    }

    let mut ipv4_count = 0;
    for val in &non_empty {
        let parts: Vec<&str> = val.split('.').collect();
        if parts.len() == 4 {
            let all_valid = parts.iter().all(|p| {
                p.parse::<u16>()
                    .map(|n| n <= 255 && !p.is_empty())
                    .unwrap_or(false)
            });
            if all_valid {
                ipv4_count += 1;
            }
        }
    }

    let fraction = ipv4_count as f64 / non_empty.len() as f64;
    if fraction >= 0.8 {
        Some("technology.internet.ip_v4".to_string())
    } else {
        None
    }
}

/// Disambiguate numeric types based on value range and distribution.
///
/// Covers: port, increment, postal_code, integer_number, street_number, year
fn disambiguate_numeric(
    values: &[String],
    results: &[ClassificationResult],
    top_labels: &[&str],
) -> Option<(String, String)> {
    // Only trigger for numeric-looking columns
    let numeric_types = [
        "technology.internet.port",
        "representation.numeric.increment",
        "representation.numeric.integer_number",
        "representation.numeric.decimal_number",
        "geography.address.postal_code",
        "geography.address.street_number",
        "datetime.component.year",
    ];

    let has_numeric_confusion = top_labels.iter().any(|l| numeric_types.contains(l));
    if !has_numeric_confusion {
        return None;
    }

    // Parse all values as integers
    let parsed: Vec<i64> = values
        .iter()
        .filter_map(|v| v.trim().parse::<i64>().ok())
        .collect();

    if parsed.len() < 3 {
        return None;
    }

    let min = *parsed.iter().min().unwrap();
    let max = *parsed.iter().max().unwrap();
    let range = max - min;

    // Check for sequential/increment pattern
    let mut sorted = parsed.clone();
    sorted.sort();
    sorted.dedup();
    let is_sequential = if sorted.len() >= 3 {
        let diffs: Vec<i64> = sorted.windows(2).map(|w| w[1] - w[0]).collect();
        let avg_diff = diffs.iter().sum::<i64>() as f64 / diffs.len() as f64;
        let variance = diffs
            .iter()
            .map(|d| (*d as f64 - avg_diff).powi(2))
            .sum::<f64>()
            / diffs.len() as f64;
        // Low variance in diffs → sequential
        variance < (avg_diff * 0.5).powi(2) && avg_diff > 0.0
    } else {
        false
    };

    // Port detection: 0-65535, common ports cluster
    // Require ≥30% of values to match common ports (not just "any").
    // This prevents false positives on age/count columns where a few values
    // (e.g., 22, 25, 53) coincidentally match common port numbers.
    let all_in_port_range = min >= 0 && max <= 65535;
    let common_ports = [80, 443, 8080, 3306, 5432, 22, 21, 25, 53, 3000, 8000, 8443];
    let common_port_count = parsed.iter().filter(|v| common_ports.contains(v)).count();
    let common_port_fraction = common_port_count as f64 / parsed.len() as f64;
    let has_common_ports = common_port_fraction >= 0.3;

    // Postal code detection: typically 3-10 digits, non-sequential, bounded range
    let all_positive = min > 0;
    let typical_postal_range = all_positive && max <= 99999 && min >= 100;
    let digit_lengths: Vec<usize> = values
        .iter()
        .filter_map(|v| {
            let trimmed = v.trim();
            if trimmed.chars().all(|c| c.is_ascii_digit()) {
                Some(trimmed.len())
            } else {
                None
            }
        })
        .collect();
    let consistent_digits = if !digit_lengths.is_empty() {
        let first_len = digit_lengths[0];
        digit_lengths.iter().all(|&l| l == first_len)
    } else {
        false
    };

    // Year detection: 4-digit integers in 1900-2100 range
    // Relaxed: ≥80% of values must be in year range (allows occasional outliers)
    let year_candidates: Vec<i64> = parsed
        .iter()
        .filter(|&&v| (1900..=2100).contains(&v))
        .copied()
        .collect();
    let count_trimmed_4digit = values
        .iter()
        .filter(|v| {
            let t = v.trim();
            t.len() == 4 && t.chars().all(|c| c.is_ascii_digit())
        })
        .count();
    let fraction_4digit = if values.is_empty() {
        0.0
    } else {
        count_trimmed_4digit as f64 / values.len() as f64
    };
    let mostly_4digit = fraction_4digit >= 0.8;
    let year_fraction = if parsed.is_empty() {
        0.0
    } else {
        year_candidates.len() as f64 / parsed.len() as f64
    };
    let is_year_column = year_fraction >= 0.8 && parsed.len() >= 3 && mostly_4digit;

    // Decision logic — year check BEFORE sequential, because a column of
    // years (e.g., 2018, 2019, 2020) is more likely to be years than IDs.
    if is_year_column {
        // All values are 4-digit integers in 1900-2100 range → year
        return Some((
            "datetime.component.year".to_string(),
            "numeric_year_detection".to_string(),
        ));
    }

    if is_sequential && min >= 0 && range > 0 {
        // Sequential integers → increment
        return Some((
            "representation.numeric.increment".to_string(),
            "numeric_sequential_detection".to_string(),
        ));
    }

    if has_common_ports && all_in_port_range && !is_sequential {
        // Has common ports and all in range → port
        return Some((
            "technology.internet.port".to_string(),
            "numeric_port_detection".to_string(),
        ));
    }

    if consistent_digits && typical_postal_range && !is_sequential {
        // Exclude year-like columns: if ≥80% of 4-digit values are in 1900-2100,
        // prefer year over postal code (e.g., years with occasional outlier)
        if mostly_4digit && year_fraction >= 0.8 {
            return Some((
                "datetime.component.year".to_string(),
                "numeric_year_detection".to_string(),
            ));
        }
        // Consistent digit length, typical postal range → postal code
        return Some((
            "geography.address.postal_code".to_string(),
            "numeric_postal_code_detection".to_string(),
        ));
    }

    // Street number: small positive integers, typically 1-9999
    let street_range = all_positive && max < 100000 && min >= 1;
    let is_street_candidate = top_labels.contains(&"geography.address.street_number");
    if is_street_candidate
        && street_range
        && !is_sequential
        && !has_common_ports
        && !consistent_digits
    {
        return Some((
            "geography.address.street_number".to_string(),
            "numeric_street_number_detection".to_string(),
        ));
    }

    // Fallback: if we couldn't determine more specifically, use the model majority
    // (return None to let the majority vote stand)
    let _ = results; // suppress unused warning
    None
}

/// SI number override: plain decimals misclassified as si_number.
///
/// The T2_DOUBLE_numeric model sometimes predicts `si_number` for columns of
/// plain decimals (e.g. "5.1", "3.5") because the numeric prefix of SI values
/// (before the suffix like K, M, G) looks identical. If no sampled values
/// contain an SI suffix, override to `decimal_number`.
fn disambiguate_si_number(values: &[String]) -> Option<(String, String)> {
    // SI suffixes: K/k (kilo), M/m (mega), B/b (billion), T/t (tera/trillion),
    // G/g (giga). Also check for % which would be percentage.
    const SI_SUFFIXES: &[char] = &['K', 'k', 'M', 'm', 'B', 'b', 'T', 't', 'G', 'g'];

    let has_si_suffix = values.iter().any(|v| {
        let trimmed = v.trim();
        if trimmed.is_empty() {
            return false;
        }
        // Check if the last character (ignoring trailing whitespace) is an SI suffix
        trimmed
            .chars()
            .last()
            .is_some_and(|c| SI_SUFFIXES.contains(&c))
    });

    if !has_si_suffix {
        Some((
            "representation.numeric.decimal_number".to_string(),
            "si_number_override_no_suffix".to_string(),
        ))
    } else {
        None
    }
}

/// Duration override: ISO 8601 durations misclassified as SEDOL codes.
///
/// ISO 8601 durations (PT20M, P1DT12H, PD1TH0M0) start with 'P' followed
/// by time component letters (Y, M, D, T, H, S) and digits. SEDOL codes are
/// exactly 7 alphanumeric chars but exclude certain letters. The CharCNN sees
/// 5-8 char alphanumeric strings starting with P and predicts SEDOL.
///
/// Rule: If the top vote is SEDOL and ≥50% of non-empty values start with 'P'
/// followed by at least one duration component letter, override to iso_8601 duration.
fn disambiguate_duration_override(values: &[String]) -> Option<(String, String)> {
    let non_empty: Vec<&str> = values
        .iter()
        .map(|v| v.trim())
        .filter(|v| !v.is_empty())
        .collect();

    if non_empty.len() < 3 {
        return None;
    }

    // ISO 8601 duration pattern: starts with P, then contains digits and
    // time component designators (Y=years, M=months, W=weeks, D=days,
    // T=time separator, H=hours, S=seconds). Also handles non-standard
    // variants like PD1TH0M0 found in SOTAB data.
    let duration_count = non_empty
        .iter()
        .filter(|v| {
            let s = v.as_bytes();
            if s.is_empty() || s[0] != b'P' {
                return false;
            }
            // After the P, must contain at least one duration component letter
            let after_p = &s[1..];
            after_p
                .iter()
                .any(|&b| matches!(b, b'Y' | b'M' | b'W' | b'D' | b'T' | b'H' | b'S'))
        })
        .count();

    let fraction = duration_count as f64 / non_empty.len() as f64;

    if fraction >= 0.5 {
        Some((
            "datetime.duration.iso_8601".to_string(),
            "duration_override_sedol".to_string(),
        ))
    } else {
        None
    }
}

/// Text length demotion: full_address with long median value length.
///
/// The CharCNN often classifies free-form text (descriptions, recipe steps,
/// paragraphs) as `geography.address.full_address` because addresses and
/// text share features like commas, numbers, and mixed casing. However,
/// real addresses have a median value length around 23 chars while text
/// overcall has a median of 53+ chars.
///
/// Rule: If the top vote is full_address and the median non-empty value
/// length exceeds 100 characters, demote to representation.text.sentence.
/// Threshold 100 gives 0% false demotion rate on evaluation data.
fn disambiguate_text_length_demotion(
    values: &[String],
    votes: &[(String, usize)],
) -> Option<(String, String)> {
    let top_label = votes.first().map(|(l, _)| l.as_str())?;

    if top_label != "geography.address.full_address" {
        return None;
    }

    let mut lengths: Vec<usize> = values
        .iter()
        .map(|v| v.trim())
        .filter(|v| !v.is_empty())
        .map(|v| v.len())
        .collect();

    if lengths.len() < 3 {
        return None;
    }

    lengths.sort_unstable();
    let median = lengths[lengths.len() / 2];

    if median > 100 {
        Some((
            "representation.text.sentence".to_string(),
            "text_length_demotion_full_address".to_string(),
        ))
    } else {
        None
    }
}

/// Demote "attractor" types back to generic representation.* types when
/// the evidence doesn't support the specific prediction.
///
/// Three independent signals, checked in order of strength:
/// 1. Validation failure: >50% of sample values fail the type's validation schema
/// 2. Confidence threshold: top vote fraction < 0.85 (skipped when confirmed)
/// 3. Cardinality mismatch: text attractor + 1-20 unique values → categorical
///    (skipped when locale-confirmed)
///
/// **Validation Precision (NNFT-132):** For locale-specific types (those with
/// `validation_by_locale`), only locale-level confirmation gates Signals 2 and 3.
/// Universal validation can reject (Signal 1) but cannot confirm — passing a
/// permissive universal pattern like `^[+]?[0-9\s()\-\.]+$` is not evidence.
/// For types without locale validation, universal confirmation still gates Signal 2.
///
/// This rule runs AFTER all other disambiguation rules and BEFORE header hint
/// override, so header hints can still rescue legitimate predictions that were
/// demoted (e.g., model says postal_code at 0.7, header is "zip_code").
fn disambiguate_attractor_demotion(
    values: &[String],
    votes: &[(String, usize)],
    n_samples: usize,
    taxonomy: Option<&Taxonomy>,
) -> Option<(String, String)> {
    let (top_label, top_count) = votes.first()?;
    let majority_fraction = *top_count as f32 / n_samples as f32;

    let is_numeric = NUMERIC_ATTRACTORS.contains(&top_label.as_str());
    let is_text = TEXT_ATTRACTORS.contains(&top_label.as_str());
    let is_code = CODE_ATTRACTORS.contains(&top_label.as_str());

    if !is_numeric && !is_text && !is_code {
        return None;
    }

    // Signal 1: Validation failure (strongest signal)
    // If taxonomy available and predicted type has a validation schema with a
    // regex pattern, check sample values against it. Demote if >50% fail.
    //
    // Tracks two independent confirmation signals (NNFT-132 Precision Principle):
    // - locale_confirmed: locale-specific pattern matched >50% (strong evidence)
    // - validation_confirmed: universal validation pattern passed (weaker evidence)
    //
    // For locale-specific types (those with validation_by_locale), only
    // locale_confirmed gates Signals 2 and 3. Universal validation can reject
    // (Signal 1) but cannot confirm — a permissive universal pattern is not
    // evidence of type identity. For types without locale validation, universal
    // validation_confirmed still gates Signal 2.
    let mut locale_confirmed = false;
    let mut validation_confirmed = false;
    let mut has_locale_validators = false;
    if let Some(taxonomy) = taxonomy {
        // Use pre-compiled validator from taxonomy cache (NNFT-116).
        // Falls back to compile-per-call if cache not populated.
        let has_pattern = taxonomy
            .get(top_label)
            .and_then(|d| d.validation.as_ref())
            .and_then(|v| v.pattern.as_ref())
            .is_some();

        let non_empty: Vec<&str> = values
            .iter()
            .map(|v| v.trim())
            .filter(|v| !v.is_empty())
            .collect();

        if non_empty.len() >= 3 {
            // Check locale-specific validators first (if available).
            // If any locale passes >50%, the type is locale-confirmed.
            if let Some(locale_validators) = taxonomy.get_locale_validators(top_label) {
                has_locale_validators = true;
                let mut best_pass_rate: f32 = 0.0;
                for compiled in locale_validators.values() {
                    let pass_count = non_empty.iter().filter(|v| compiled.is_valid(v)).count();
                    let pass_rate = pass_count as f32 / non_empty.len() as f32;
                    if pass_rate > best_pass_rate {
                        best_pass_rate = pass_rate;
                    }
                }
                if best_pass_rate > 0.5 {
                    // A locale pattern matched well — strong confirmation.
                    locale_confirmed = true;
                }
            }

            // If locale validators didn't confirm, fall through to universal validation.
            if !locale_confirmed {
                let fail_count = if let Some(compiled) = taxonomy.get_validator(top_label) {
                    // Fast path: pre-compiled validator (no per-value regex compilation)
                    non_empty.iter().filter(|v| !compiled.is_valid(v)).count()
                } else if let Some(def) = taxonomy.get(top_label) {
                    if let Some(validation) = &def.validation {
                        // Fallback: compile per-call (shouldn't happen with cache populated)
                        non_empty
                            .iter()
                            .filter(|v| {
                                finetype_core::validate_value(v, validation)
                                    .map(|r| !r.is_valid)
                                    .unwrap_or(false)
                            })
                            .count()
                    } else {
                        0
                    }
                } else {
                    0
                };

                let fail_rate = fail_count as f32 / non_empty.len() as f32;
                if fail_rate > 0.5 {
                    let fallback = select_fallback(votes, is_numeric, is_text, is_code, values);
                    return Some((
                        fallback,
                        format!("attractor_demotion_validation:{}", top_label),
                    ));
                }
                // If validation has a regex pattern and values mostly pass
                // (≤30% fail), that's positive evidence FOR the type.
                // NOTE: For locale-specific types this is "format-compatible but
                // unconfirmed" — it does NOT gate Signals 2/3 (see below).
                if has_pattern && fail_rate <= 0.3 {
                    validation_confirmed = true;
                }
            }
        }
    }

    // Signal 2: Confidence threshold
    // True positives for attractor types cluster at >0.9 confidence.
    // False positives cluster at 0.3–0.8.
    //
    // Confirmation gating (NNFT-132 Precision Principle):
    // - Locale-specific types: only locale_confirmed skips this signal.
    //   Universal validation passing is "format-compatible" not "confirmed".
    // - Other types: validation_confirmed (universal pattern match) suffices.
    let confirmed = locale_confirmed || (!has_locale_validators && validation_confirmed);
    if !confirmed && majority_fraction < 0.85 {
        let fallback = select_fallback(votes, is_numeric, is_text, is_code, values);
        return Some((
            fallback,
            format!("attractor_demotion_confidence:{}", top_label),
        ));
    }

    // Signal 3: Cardinality mismatch (text attractors only)
    // Low cardinality columns (1-20 unique values) predicted as identity
    // types → demote to categorical. A column with 1–2 unique values is the
    // strongest possible signal (e.g., "airport" repeated 7k times is NOT a
    // person's first_name).
    //
    // SKIP if locale-confirmed (NNFT-132): locale-specific patterns are
    // strong structural evidence that overcomes low cardinality. Small tables
    // (common in web-scraped datasets like SOTAB) legitimately have few unique
    // phone numbers or postal codes — cardinality alone shouldn't override
    // locale-level format confirmation.
    if is_text && !locale_confirmed {
        let non_empty: Vec<&str> = values
            .iter()
            .map(|v| v.trim())
            .filter(|v| !v.is_empty())
            .collect();
        let mut unique: Vec<&str> = non_empty.clone();
        unique.sort();
        unique.dedup();
        if (1..=20).contains(&unique.len()) {
            return Some((
                "representation.discrete.categorical".to_string(),
                format!("attractor_demotion_cardinality:{}", top_label),
            ));
        }
    }

    None
}

/// Select the best fallback type when demoting an attractor prediction.
///
/// Priority:
/// 1. Use an existing representation.* type from the vote distribution
/// 2. Default: numeric → integer/decimal, text → categorical, code → alphanumeric_id
fn select_fallback(
    votes: &[(String, usize)],
    is_numeric: bool,
    is_text: bool,
    is_code: bool,
    values: &[String],
) -> String {
    // Check if a representation.* type exists in votes (skip the attractor at [0])
    for (label, _) in votes.iter().skip(1) {
        if label.starts_with("representation.") {
            return label.clone();
        }
    }

    // Default fallback by attractor category
    if is_numeric {
        let has_decimal = values.iter().any(|v| v.contains('.'));
        if has_decimal {
            "representation.numeric.decimal_number".to_string()
        } else {
            "representation.numeric.integer_number".to_string()
        }
    } else if is_text {
        "representation.discrete.categorical".to_string()
    } else if is_code {
        "representation.alphanumeric.alphanumeric_id".to_string()
    } else {
        "representation.text.word".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Disambiguation rule unit tests ──────────────────────────────────

    #[test]
    fn test_slash_date_eu_detected() {
        let values: Vec<String> = vec![
            "15/01/2024",
            "28/06/2023",
            "03/11/2022",
            "31/12/2019",
            "12/05/2020",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        let result = disambiguate_slash_dates(&values);
        assert_eq!(result, Some("datetime.date.eu_slash".to_string()));
    }

    #[test]
    fn test_slash_date_us_detected() {
        let values: Vec<String> = vec![
            "01/15/2024",
            "06/28/2023",
            "11/03/2022",
            "12/31/2019",
            "05/12/2020",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        let result = disambiguate_slash_dates(&values);
        assert_eq!(result, Some("datetime.date.us_slash".to_string()));
    }

    #[test]
    fn test_slash_date_ambiguous() {
        // All values have both components ≤ 12 — ambiguous
        let values: Vec<String> = vec![
            "01/02/2024",
            "03/04/2023",
            "05/06/2022",
            "07/08/2021",
            "09/10/2020",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        let result = disambiguate_slash_dates(&values);
        assert_eq!(result, None);
    }

    #[test]
    fn test_short_date_dmy_detected() {
        let values: Vec<String> = vec!["15-01-24", "28-06-23", "31-12-19"]
            .into_iter()
            .map(String::from)
            .collect();

        let result = disambiguate_short_dates(&values);
        assert_eq!(result, Some("datetime.date.short_dmy".to_string()));
    }

    #[test]
    fn test_short_date_mdy_detected() {
        let values: Vec<String> = vec!["01-15-24", "06-28-23", "12-31-19"]
            .into_iter()
            .map(String::from)
            .collect();

        let result = disambiguate_short_dates(&values);
        assert_eq!(result, Some("datetime.date.short_mdy".to_string()));
    }

    #[test]
    fn test_coordinates_longitude_detected() {
        let values: Vec<String> = vec!["-74.0060", "151.2093", "-0.1278", "139.6917", "2.3522"]
            .into_iter()
            .map(String::from)
            .collect();

        let result = disambiguate_coordinates(&values);
        assert_eq!(result, Some("geography.coordinate.longitude".to_string()));
    }

    #[test]
    fn test_coordinates_latitude_detected() {
        let values: Vec<String> = vec!["40.7128", "-33.8688", "51.5074", "35.6762", "-22.9068"]
            .into_iter()
            .map(String::from)
            .collect();

        let result = disambiguate_coordinates(&values);
        assert_eq!(result, Some("geography.coordinate.latitude".to_string()));
    }

    #[test]
    fn test_numeric_sequential_detection() {
        let values: Vec<String> = vec!["1", "2", "3", "4", "5", "6", "7", "8", "9", "10"]
            .into_iter()
            .map(String::from)
            .collect();

        // Create mock results with increment label
        let results: Vec<ClassificationResult> = values
            .iter()
            .map(|_| ClassificationResult {
                label: "representation.numeric.increment".to_string(),
                confidence: 0.8,
                all_scores: vec![],
            })
            .collect();

        let votes = vec![
            ("representation.numeric.increment".to_string(), 8),
            ("representation.numeric.integer_number".to_string(), 2),
        ];
        let top_labels: Vec<&str> = votes.iter().map(|(l, _)| l.as_str()).collect();

        let result = disambiguate_numeric(&values, &results, &top_labels);
        assert!(result.is_some());
        let (label, _rule) = result.unwrap();
        assert_eq!(label, "representation.numeric.increment");
    }

    #[test]
    fn test_numeric_port_detection() {
        let values: Vec<String> = vec![
            "80", "443", "8080", "3306", "22", "5432", "3000", "8443", "25", "53",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        let results: Vec<ClassificationResult> = values
            .iter()
            .map(|_| ClassificationResult {
                label: "technology.internet.port".to_string(),
                confidence: 0.7,
                all_scores: vec![],
            })
            .collect();

        let votes = vec![
            ("technology.internet.port".to_string(), 7),
            ("representation.numeric.integer_number".to_string(), 3),
        ];
        let top_labels: Vec<&str> = votes.iter().map(|(l, _)| l.as_str()).collect();

        let result = disambiguate_numeric(&values, &results, &top_labels);
        assert!(result.is_some());
        let (label, _rule) = result.unwrap();
        assert_eq!(label, "technology.internet.port");
    }

    #[test]
    fn test_numeric_postal_code_detection() {
        let values: Vec<String> = vec![
            "10001", "90210", "30301", "60601", "02101", "75001", "33101", "94102", "20001",
            "98101",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        let results: Vec<ClassificationResult> = values
            .iter()
            .map(|_| ClassificationResult {
                label: "geography.address.postal_code".to_string(),
                confidence: 0.6,
                all_scores: vec![],
            })
            .collect();

        let votes = vec![
            ("geography.address.postal_code".to_string(), 6),
            ("representation.numeric.integer_number".to_string(), 4),
        ];
        let top_labels: Vec<&str> = votes.iter().map(|(l, _)| l.as_str()).collect();

        let result = disambiguate_numeric(&values, &results, &top_labels);
        assert!(result.is_some());
        let (label, _rule) = result.unwrap();
        assert_eq!(label, "geography.address.postal_code");
    }

    #[test]
    fn test_year_detection() {
        let values: Vec<String> = vec![
            "2020", "2019", "2021", "2018", "2023", "2015", "2022", "2017", "2024", "2016",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        let results: Vec<ClassificationResult> = values
            .iter()
            .map(|_| ClassificationResult {
                label: "representation.numeric.integer_number".to_string(),
                confidence: 0.6,
                all_scores: vec![],
            })
            .collect();

        let votes = vec![
            ("representation.numeric.integer_number".to_string(), 5),
            ("geography.address.street_number".to_string(), 3),
            ("datetime.component.year".to_string(), 2),
        ];
        let top_labels: Vec<&str> = votes.iter().map(|(l, _)| l.as_str()).collect();

        let result = disambiguate_numeric(&values, &results, &top_labels);
        assert!(result.is_some());
        let (label, rule) = result.unwrap();
        assert_eq!(label, "datetime.component.year");
        assert_eq!(rule, "numeric_year_detection");
    }

    #[test]
    fn test_year_detection_historical() {
        // Historical years in typical range
        let values: Vec<String> = vec!["1945", "1918", "1969", "1989", "2001"]
            .into_iter()
            .map(String::from)
            .collect();

        let results: Vec<ClassificationResult> = values
            .iter()
            .map(|_| ClassificationResult {
                label: "representation.numeric.decimal_number".to_string(),
                confidence: 0.5,
                all_scores: vec![],
            })
            .collect();

        let votes = vec![
            ("representation.numeric.decimal_number".to_string(), 3),
            ("representation.numeric.integer_number".to_string(), 2),
        ];
        let top_labels: Vec<&str> = votes.iter().map(|(l, _)| l.as_str()).collect();

        let result = disambiguate_numeric(&values, &results, &top_labels);
        assert!(result.is_some());
        let (label, _) = result.unwrap();
        assert_eq!(label, "datetime.component.year");
    }

    #[test]
    fn test_year_not_triggered_for_5digit_postal() {
        // 5-digit postal codes should NOT trigger year rule
        let values: Vec<String> = vec![
            "10001", "90210", "30301", "60601", "02101", "75001", "33101", "94102", "20001",
            "98101",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        let results: Vec<ClassificationResult> = values
            .iter()
            .map(|_| ClassificationResult {
                label: "geography.address.postal_code".to_string(),
                confidence: 0.6,
                all_scores: vec![],
            })
            .collect();

        let votes = vec![
            ("geography.address.postal_code".to_string(), 6),
            ("representation.numeric.integer_number".to_string(), 4),
        ];
        let top_labels: Vec<&str> = votes.iter().map(|(l, _)| l.as_str()).collect();

        let result = disambiguate_numeric(&values, &results, &top_labels);
        assert!(result.is_some());
        let (label, _) = result.unwrap();
        // Should be postal_code, NOT year (5-digit values)
        assert_eq!(label, "geography.address.postal_code");
    }

    #[test]
    fn test_sequential_years_still_detected_as_year() {
        // Sequential 4-digit numbers in year range → still year (more likely
        // a column of consecutive years than auto-increment IDs starting at 2001)
        let values: Vec<String> = vec![
            "2001", "2002", "2003", "2004", "2005", "2006", "2007", "2008", "2009", "2010",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        let results: Vec<ClassificationResult> = values
            .iter()
            .map(|_| ClassificationResult {
                label: "representation.numeric.increment".to_string(),
                confidence: 0.7,
                all_scores: vec![],
            })
            .collect();

        let votes = vec![
            ("representation.numeric.increment".to_string(), 7),
            ("representation.numeric.integer_number".to_string(), 3),
        ];
        let top_labels: Vec<&str> = votes.iter().map(|(l, _)| l.as_str()).collect();

        let result = disambiguate_numeric(&values, &results, &top_labels);
        assert!(result.is_some());
        let (label, _) = result.unwrap();
        // Year wins over increment when values are in 1900-2100 range
        assert_eq!(label, "datetime.component.year");
    }

    #[test]
    fn test_sequential_non_year_still_increment() {
        // Sequential numbers outside year range → increment
        let values: Vec<String> = vec!["1", "2", "3", "4", "5", "6", "7", "8", "9", "10"]
            .into_iter()
            .map(String::from)
            .collect();

        let results: Vec<ClassificationResult> = values
            .iter()
            .map(|_| ClassificationResult {
                label: "representation.numeric.increment".to_string(),
                confidence: 0.8,
                all_scores: vec![],
            })
            .collect();

        let votes = vec![
            ("representation.numeric.increment".to_string(), 8),
            ("representation.numeric.integer_number".to_string(), 2),
        ];
        let top_labels: Vec<&str> = votes.iter().map(|(l, _)| l.as_str()).collect();

        let result = disambiguate_numeric(&values, &results, &top_labels);
        assert!(result.is_some());
        let (label, _) = result.unwrap();
        assert_eq!(label, "representation.numeric.increment");
    }

    #[test]
    fn test_year_not_triggered_for_ports() {
        // Port numbers (some happen to be in year range but have common ports)
        let values: Vec<String> = vec!["80", "443", "8080", "3306", "22"]
            .into_iter()
            .map(String::from)
            .collect();

        let results: Vec<ClassificationResult> = values
            .iter()
            .map(|_| ClassificationResult {
                label: "technology.internet.port".to_string(),
                confidence: 0.8,
                all_scores: vec![],
            })
            .collect();

        let votes = vec![
            ("technology.internet.port".to_string(), 4),
            ("representation.numeric.integer_number".to_string(), 1),
        ];
        let top_labels: Vec<&str> = votes.iter().map(|(l, _)| l.as_str()).collect();

        let result = disambiguate_numeric(&values, &results, &top_labels);
        // Should NOT be year (values are 2-4 digits, not all 4-digit)
        if let Some((label, _)) = result {
            assert_ne!(label, "datetime.component.year");
        }
    }

    #[test]
    fn test_year_with_outlier_not_postal_code() {
        // Year column with one outlier outside 1900-2100 — should still be year (≥80% rule)
        let values: Vec<String> = vec![
            "2020", "2019", "2021", "2018", "2023", "2015", "2022", "2017", "2024", "1776",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        let results: Vec<ClassificationResult> = values
            .iter()
            .map(|_| ClassificationResult {
                label: "geography.address.postal_code".to_string(),
                confidence: 0.6,
                all_scores: vec![],
            })
            .collect();

        let votes = vec![
            ("geography.address.postal_code".to_string(), 5),
            ("representation.numeric.decimal_number".to_string(), 3),
            ("datetime.component.year".to_string(), 2),
        ];
        let top_labels: Vec<&str> = votes.iter().map(|(l, _)| l.as_str()).collect();

        let result = disambiguate_numeric(&values, &results, &top_labels);
        assert!(result.is_some());
        let (label, _rule) = result.unwrap();
        // Should be year, NOT postal_code: 9 of 10 values are in 1900-2100 (90% ≥ 80%)
        assert_eq!(label, "datetime.component.year");
    }

    #[test]
    fn test_year_with_many_outliers_not_year() {
        // Only 60% of values in year range — below 80% threshold, should NOT be year
        let values: Vec<String> = vec![
            "2020", "2019", "2021", "1500", "1600", "1700", "1800", "2022", "2023", "2024",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        let results: Vec<ClassificationResult> = values
            .iter()
            .map(|_| ClassificationResult {
                label: "representation.numeric.integer_number".to_string(),
                confidence: 0.6,
                all_scores: vec![],
            })
            .collect();

        let votes = vec![
            ("representation.numeric.integer_number".to_string(), 5),
            ("geography.address.postal_code".to_string(), 3),
            ("datetime.component.year".to_string(), 2),
        ];
        let top_labels: Vec<&str> = votes.iter().map(|(l, _)| l.as_str()).collect();

        let result = disambiguate_numeric(&values, &results, &top_labels);
        // 6/10 in year range = 60% < 80% threshold → should NOT be year
        if let Some((label, _)) = result {
            assert_ne!(label, "datetime.component.year");
        }
    }

    #[test]
    fn test_year_with_non4digit_outlier() {
        // Year column where 1 of 10 values is not a 4-digit integer (e.g., "NA" or empty)
        // With the relaxed check, 9/10 = 90% ≥ 80% should still detect as year
        let values: Vec<String> = vec![
            "2020", "2019", "2021", "2018", "2023", "2015", "2022", "2017", "2024", "NA",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        let results: Vec<ClassificationResult> = values
            .iter()
            .map(|_| ClassificationResult {
                label: "representation.numeric.decimal_number".to_string(),
                confidence: 0.6,
                all_scores: vec![],
            })
            .collect();

        let votes = vec![
            ("representation.numeric.decimal_number".to_string(), 8),
            ("datetime.component.year".to_string(), 2),
        ];
        let top_labels: Vec<&str> = votes.iter().map(|(l, _)| l.as_str()).collect();

        let result = disambiguate_numeric(&values, &results, &top_labels);
        assert!(result.is_some());
        let (label, rule) = result.unwrap();
        // 9 of 10 values are 4-digit (90% ≥ 80%) and all parseable ones are in year range
        assert_eq!(label, "datetime.component.year");
        assert_eq!(rule, "numeric_year_detection");
    }

    #[test]
    fn test_year_with_decimal_format() {
        // Year column where values have decimal formatting like "2020.0"
        // These are not 4-digit integers, so the fraction check matters
        let values: Vec<String> = vec![
            "2020", "2019", "2021.0", "2018", "2023", "2015", "2022", "2017.0", "2024", "2016",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        let results: Vec<ClassificationResult> = values
            .iter()
            .map(|_| ClassificationResult {
                label: "representation.numeric.decimal_number".to_string(),
                confidence: 0.7,
                all_scores: vec![],
            })
            .collect();

        let votes = vec![("representation.numeric.decimal_number".to_string(), 10)];
        let top_labels: Vec<&str> = votes.iter().map(|(l, _)| l.as_str()).collect();

        let result = disambiguate_numeric(&values, &results, &top_labels);
        assert!(result.is_some());
        let (label, _) = result.unwrap();
        // 8 of 10 values are 4-digit (80% ≥ 80%), and all integers parse into year range
        assert_eq!(label, "datetime.component.year");
    }

    #[test]
    fn test_not_year_when_too_few_4digit() {
        // Column where less than 80% of values are 4-digit — should NOT be year
        let values: Vec<String> = vec![
            "2020", "2019", "NA", "N/A", "", "2015", "2022", "null", "2024", "missing",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        let results: Vec<ClassificationResult> = values
            .iter()
            .map(|_| ClassificationResult {
                label: "representation.numeric.decimal_number".to_string(),
                confidence: 0.5,
                all_scores: vec![],
            })
            .collect();

        let votes = vec![("representation.numeric.decimal_number".to_string(), 10)];
        let top_labels: Vec<&str> = votes.iter().map(|(l, _)| l.as_str()).collect();

        let result = disambiguate_numeric(&values, &results, &top_labels);
        // 5 of 10 values are 4-digit (50% < 80%) → should NOT be year
        if let Some((label, _)) = result {
            assert_ne!(label, "datetime.component.year");
        }
    }

    #[test]
    fn test_age_column_not_detected_as_port() {
        // Age values like 22, 25, 53 coincidentally match common port numbers,
        // but the fraction is too low (3/10 = 30%) and the column is clearly ages.
        // With the ≥30% threshold and the sequential/year checks running first,
        // this should NOT be classified as port.
        let values: Vec<String> = vec!["22", "25", "30", "35", "40", "45", "50", "53", "60", "70"]
            .into_iter()
            .map(String::from)
            .collect();

        let results: Vec<ClassificationResult> = values
            .iter()
            .map(|_| ClassificationResult {
                label: "technology.internet.port".to_string(),
                confidence: 0.7,
                all_scores: vec![],
            })
            .collect();

        let votes = vec![
            ("technology.internet.port".to_string(), 6),
            ("representation.numeric.integer_number".to_string(), 4),
        ];
        let top_labels: Vec<&str> = votes.iter().map(|(l, _)| l.as_str()).collect();

        let result = disambiguate_numeric(&values, &results, &top_labels);
        // Should NOT be port — only 3 of 10 values (22, 25, 53) match common ports
        // and these are typical age values
        if let Some((label, _)) = result {
            assert_ne!(label, "technology.internet.port");
        }
    }

    #[test]
    fn test_age_column_with_mixed_values_not_port() {
        // Realistic Titanic-like age column: mix of young and old ages.
        // Values 21, 22, 25, 53 match common ports but that's only 4/15 = 27% < 30%.
        let values: Vec<String> = vec![
            "2", "5", "14", "17", "21", "22", "25", "28", "33", "38", "42", "47", "53", "61", "75",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        let results: Vec<ClassificationResult> = values
            .iter()
            .map(|_| ClassificationResult {
                label: "technology.internet.port".to_string(),
                confidence: 0.6,
                all_scores: vec![],
            })
            .collect();

        let votes = vec![
            ("technology.internet.port".to_string(), 8),
            ("representation.numeric.integer_number".to_string(), 7),
        ];
        let top_labels: Vec<&str> = votes.iter().map(|(l, _)| l.as_str()).collect();

        let result = disambiguate_numeric(&values, &results, &top_labels);
        if let Some((label, _)) = result {
            assert_ne!(label, "technology.internet.port");
        }
    }

    #[test]
    fn test_empty_column() {
        // Just test the ColumnResult for empty case
        let result = ColumnResult {
            label: "unknown".to_string(),
            confidence: 0.0,
            vote_distribution: vec![],
            disambiguation_applied: false,
            disambiguation_rule: None,
            samples_used: 0,
            detected_locale: None,
        };
        assert_eq!(result.label, "unknown");
        assert_eq!(result.samples_used, 0);
        assert_eq!(result.detected_locale, None);
    }

    // ── Locale suffix stripping tests ────────────────────────────────────

    #[test]
    fn test_strip_locale_suffix_4level_country() {
        let (base, locale) = strip_locale_suffix("geography.address.postal_code.EN_US");
        assert_eq!(base, "geography.address.postal_code");
        assert_eq!(locale, Some("EN_US"));
    }

    #[test]
    fn test_strip_locale_suffix_4level_universal() {
        let (base, locale) = strip_locale_suffix("representation.boolean.binary.UNIVERSAL");
        assert_eq!(base, "representation.boolean.binary");
        assert_eq!(locale, Some("UNIVERSAL"));
    }

    #[test]
    fn test_strip_locale_suffix_3level_unchanged() {
        let (base, locale) = strip_locale_suffix("geography.address.postal_code");
        assert_eq!(base, "geography.address.postal_code");
        assert_eq!(locale, None);
    }

    #[test]
    fn test_strip_locale_suffix_short_locale() {
        let (base, locale) = strip_locale_suffix("geography.location.city.EN");
        assert_eq!(base, "geography.location.city");
        assert_eq!(locale, Some("EN"));
    }

    #[test]
    fn test_strip_locale_suffix_no_false_positive_on_type() {
        // "iso" is lowercase — should NOT be treated as a locale suffix
        let (base, locale) = strip_locale_suffix("datetime.date.iso");
        assert_eq!(base, "datetime.date.iso");
        assert_eq!(locale, None);
    }

    #[test]
    fn test_strip_locale_suffix_no_false_positive_on_short_label() {
        // Only two parts — the last part should not be treated as locale
        let (base, locale) = strip_locale_suffix("representation.EN");
        assert_eq!(base, "representation.EN");
        assert_eq!(locale, None);
    }

    // ── Cardinality & categorical rule tests ────────────────────────────

    #[test]
    fn test_gender_detection_mixed_case() {
        let values: Vec<String> = vec![
            "male", "female", "Male", "Female", "male", "female", "male", "Female", "male", "Male",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        let result = disambiguate_gender(&values);
        assert_eq!(result, Some("identity.person.gender".to_string()));
    }

    #[test]
    fn test_gender_detection_single_char() {
        let values: Vec<String> = vec!["M", "F", "M", "F", "M", "F", "M", "F", "M", "F"]
            .into_iter()
            .map(String::from)
            .collect();

        let result = disambiguate_gender(&values);
        assert_eq!(result, Some("identity.person.gender".to_string()));
    }

    #[test]
    fn test_gender_detection_with_nonbinary() {
        // People directory: Male, Female, Non-binary
        let values: Vec<String> = vec![
            "Male",
            "Female",
            "Male",
            "Non-binary",
            "Female",
            "Male",
            "Female",
            "Male",
            "Non-binary",
            "Female",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        let result = disambiguate_gender(&values);
        assert_eq!(
            result,
            Some("identity.person.gender".to_string()),
            "Non-binary should be recognized as a valid gender value"
        );
    }

    #[test]
    fn test_gender_detection_with_other_inclusive() {
        let values: Vec<String> = vec![
            "Male",
            "Female",
            "Other",
            "Male",
            "Female",
            "Prefer not to say",
            "Male",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        let result = disambiguate_gender(&values);
        assert_eq!(result, Some("identity.person.gender".to_string()));
    }

    #[test]
    fn test_gender_detection_fails_for_non_gender() {
        let values: Vec<String> = vec!["red", "blue", "green", "red", "blue"]
            .into_iter()
            .map(String::from)
            .collect();

        let result = disambiguate_gender(&values);
        assert_eq!(result, None);
    }

    #[test]
    fn test_ipv4_detection_standard_ips() {
        let values: Vec<String> = vec![
            "192.168.1.1",
            "10.0.0.1",
            "172.16.0.1",
            "8.8.8.8",
            "1.2.3.4",
            "10.0.0.255",
            "192.168.0.100",
            "255.255.255.0",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        let result = disambiguate_ipv4(&values);
        assert_eq!(
            result,
            Some("technology.internet.ip_v4".to_string()),
            "Standard IPv4 addresses should be detected"
        );
    }

    #[test]
    fn test_ipv4_detection_rejects_version_numbers() {
        // Semantic version numbers have different structure (fewer octets, >255 values)
        let values: Vec<String> = vec![
            "1.0.0", "2.1.3", "3.14.159", "0.2.53", "6.27.84", "4.24.59", "7.23.74",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        let result = disambiguate_ipv4(&values);
        assert_eq!(
            result, None,
            "Version numbers should NOT match IPv4 pattern"
        );
    }

    #[test]
    fn test_ipv4_detection_rejects_decimals() {
        let values: Vec<String> = vec!["151.3", "165.0", "161.2", "169.1", "181.7"]
            .into_iter()
            .map(String::from)
            .collect();

        let result = disambiguate_ipv4(&values);
        assert_eq!(
            result, None,
            "Decimal numbers should NOT match IPv4 pattern"
        );
    }

    #[test]
    fn test_ipv4_detection_mixed_with_some_invalid() {
        // 80% threshold: 8 valid out of 10 = 80%
        let values: Vec<String> = vec![
            "10.0.0.1", "10.0.0.2", "10.0.0.3", "10.0.0.4", "10.0.0.5", "10.0.0.6", "10.0.0.7",
            "10.0.0.8", "N/A", "unknown",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        let result = disambiguate_ipv4(&values);
        assert_eq!(
            result,
            Some("technology.internet.ip_v4".to_string()),
            "80% valid IPs should trigger detection"
        );
    }

    #[test]
    fn test_boolean_override_integer_spread() {
        // SibSp-like column: integers 0-8 with >2 unique values
        let values: Vec<String> = vec!["0", "1", "2", "3", "0", "1", "4", "0", "5", "8"]
            .into_iter()
            .map(String::from)
            .collect();
        let top_labels = vec![
            "representation.logical.boolean",
            "representation.numeric.integer_number",
        ];

        let result = disambiguate_boolean_override(&values, &top_labels);
        assert!(result.is_some());
        let (label, rule) = result.unwrap();
        assert_eq!(label, "representation.numeric.integer_number");
        assert_eq!(rule, "boolean_override_integer_spread");
    }

    #[test]
    fn test_boolean_override_preserves_real_boolean() {
        // Actual boolean column: only 0 and 1
        let values: Vec<String> = vec!["0", "1", "0", "1", "1", "0", "0", "1", "0", "1"]
            .into_iter()
            .map(String::from)
            .collect();
        let top_labels = vec!["representation.logical.boolean"];

        let result = disambiguate_boolean_override(&values, &top_labels);
        // Should return None — this IS a boolean column (only 2 unique, spread=1)
        assert!(result.is_none());
    }

    #[test]
    fn test_boolean_override_single_char_categorical() {
        // Embarked-like column: single chars S, C, Q
        let values: Vec<String> = vec!["S", "C", "Q", "S", "S", "C", "Q", "S", "S", "C"]
            .into_iter()
            .map(String::from)
            .collect();
        let top_labels = vec!["representation.logical.boolean", "representation.text.word"];

        let result = disambiguate_boolean_override(&values, &top_labels);
        assert!(result.is_some());
        let (label, rule) = result.unwrap();
        assert_eq!(label, "representation.discrete.categorical");
        assert_eq!(rule, "boolean_override_single_char_categorical");
    }

    #[test]
    fn test_boolean_override_preserves_true_false_chars() {
        // T/F single-char boolean values should stay boolean
        let values: Vec<String> = vec!["T", "F", "T", "F", "T", "F", "T", "F"]
            .into_iter()
            .map(String::from)
            .collect();
        let top_labels = vec!["representation.logical.boolean"];

        let result = disambiguate_boolean_override(&values, &top_labels);
        // Should return None — T/F is a valid boolean encoding
        assert!(result.is_none());
    }

    #[test]
    fn test_categorical_single_char_detection() {
        // Column of single characters with >2 unique values
        let values: Vec<String> = vec!["A", "B", "C", "D", "A", "B", "C", "A", "B", "D"]
            .into_iter()
            .map(String::from)
            .collect();
        let top_labels = vec!["representation.text.word"];

        let result = disambiguate_categorical(&values, &top_labels);
        assert!(result.is_some());
        let (label, rule) = result.unwrap();
        assert_eq!(label, "representation.discrete.categorical");
        assert_eq!(rule, "categorical_single_char");
    }

    #[test]
    fn test_categorical_low_cardinality() {
        // Column with 3-20 unique short string values
        let values: Vec<String> = vec![
            "red", "blue", "green", "red", "blue", "green", "red", "blue", "green", "red",
        ]
        .into_iter()
        .map(String::from)
        .collect();
        let top_labels = vec!["representation.text.word"];

        let result = disambiguate_categorical(&values, &top_labels);
        assert!(result.is_some());
        let (label, rule) = result.unwrap();
        assert_eq!(label, "representation.discrete.categorical");
        assert_eq!(rule, "categorical_low_cardinality");
    }

    #[test]
    fn test_categorical_not_triggered_for_high_cardinality() {
        // Column with >20 unique values → not categorical
        let values: Vec<String> = (1..=25).map(|i| format!("value_{}", i)).collect();
        let top_labels = vec!["representation.text.word"];

        let result = disambiguate_categorical(&values, &top_labels);
        assert!(result.is_none());
    }

    #[test]
    fn test_categorical_not_triggered_for_numeric_values() {
        // Purely numeric column should not be overridden to categorical
        let values: Vec<String> = vec!["1", "2", "3", "1", "2", "3", "1", "2", "3", "1"]
            .into_iter()
            .map(String::from)
            .collect();
        let top_labels = vec!["representation.numeric.integer_number"];

        let result = disambiguate_categorical(&values, &top_labels);
        // Should be None because values are all numeric
        assert!(result.is_none());
    }

    #[test]
    fn test_categorical_not_triggered_for_specific_types() {
        // If top prediction is already a specific type (e.g., iata_code), don't override
        let values: Vec<String> = vec!["SYD", "LAX", "JFK", "LHR", "SYD", "LAX"]
            .into_iter()
            .map(String::from)
            .collect();
        let top_labels = vec!["geography.transport.iata_code"];

        let result = disambiguate_categorical(&values, &top_labels);
        assert!(result.is_none());
    }

    // ── Header hint tests ───────────────────────────────────────────────

    #[test]
    fn test_header_hint_email() {
        assert_eq!(header_hint("Email"), Some("identity.person.email"));
        assert_eq!(header_hint("email_address"), Some("identity.person.email"));
        assert_eq!(header_hint("E-Mail"), Some("identity.person.email"));
        assert_eq!(header_hint("user_email"), Some("identity.person.email"));
    }

    #[test]
    fn test_header_hint_phone() {
        assert_eq!(header_hint("phone"), Some("identity.person.phone_number"));
        assert_eq!(
            header_hint("Phone Number"),
            Some("identity.person.phone_number")
        );
        assert_eq!(
            header_hint("telephone"),
            Some("identity.person.phone_number")
        );
        assert_eq!(header_hint("mobile"), Some("identity.person.phone_number"));
    }

    #[test]
    fn test_header_hint_postal() {
        assert_eq!(header_hint("zip"), Some("geography.address.postal_code"));
        assert_eq!(
            header_hint("zip_code"),
            Some("geography.address.postal_code")
        );
        assert_eq!(
            header_hint("Postal Code"),
            Some("geography.address.postal_code")
        );
        assert_eq!(
            header_hint("postcode"),
            Some("geography.address.postal_code")
        );
    }

    #[test]
    fn test_header_hint_names() {
        assert_eq!(header_hint("Name"), Some("identity.person.full_name"));
        assert_eq!(header_hint("full_name"), Some("identity.person.full_name"));
        assert_eq!(
            header_hint("first_name"),
            Some("identity.person.first_name")
        );
        assert_eq!(header_hint("last_name"), Some("identity.person.last_name"));
        assert_eq!(header_hint("surname"), Some("identity.person.last_name"));
    }

    #[test]
    fn test_header_hint_geo() {
        assert_eq!(
            header_hint("latitude"),
            Some("geography.coordinate.latitude")
        );
        assert_eq!(header_hint("lat"), Some("geography.coordinate.latitude"));
        assert_eq!(
            header_hint("longitude"),
            Some("geography.coordinate.longitude")
        );
        assert_eq!(header_hint("lng"), Some("geography.coordinate.longitude"));
        assert_eq!(header_hint("country"), Some("geography.location.country"));
        assert_eq!(header_hint("city"), Some("geography.location.city"));
    }

    #[test]
    fn test_header_hint_identity() {
        assert_eq!(header_hint("gender"), Some("identity.person.gender"));
        assert_eq!(header_hint("Sex"), Some("identity.person.gender"));
        assert_eq!(header_hint("age"), Some("identity.person.age"));
        assert_eq!(header_hint("Age"), Some("identity.person.age"));
    }

    #[test]
    fn test_header_hint_tech() {
        assert_eq!(header_hint("url"), Some("technology.internet.url"));
        assert_eq!(header_hint("URL"), Some("technology.internet.url"));
        assert_eq!(header_hint("website"), Some("technology.internet.url"));
        assert_eq!(header_hint("ip_address"), Some("technology.internet.ip_v4"));
        assert_eq!(header_hint("uuid"), Some("technology.identifier.uuid"));
        assert_eq!(header_hint("port"), Some("technology.internet.port"));
    }

    #[test]
    fn test_header_hint_date() {
        assert_eq!(header_hint("date"), Some("datetime.timestamp.iso_8601"));
        assert_eq!(
            header_hint("created_date"),
            Some("datetime.timestamp.iso_8601")
        );
        assert_eq!(header_hint("year"), Some("datetime.component.year"));
        assert_eq!(header_hint("birth_date"), Some("datetime.date.iso_date"));
        assert_eq!(header_hint("dob"), Some("datetime.date.iso_date"));
    }

    #[test]
    fn test_header_hint_numeric() {
        assert_eq!(
            header_hint("price"),
            Some("representation.numeric.decimal_number")
        );
        assert_eq!(
            header_hint("amount"),
            Some("representation.numeric.decimal_number")
        );
        assert_eq!(
            header_hint("count"),
            Some("representation.numeric.integer_number")
        );
        assert_eq!(header_hint("id"), Some("representation.numeric.increment"));
    }

    #[test]
    fn test_header_hint_no_match() {
        assert_eq!(header_hint("foo"), None);
        assert_eq!(header_hint("xyz"), None);
        assert_eq!(header_hint("data"), None);
        assert_eq!(header_hint("column1"), None);
    }

    #[test]
    fn test_header_hint_coverage() {
        // Verify at least 20 distinct column name patterns are covered
        let test_headers = vec![
            "email",
            "phone",
            "zip",
            "postal",
            "name",
            "full_name",
            "first_name",
            "last_name",
            "latitude",
            "longitude",
            "country",
            "city",
            "state",
            "gender",
            "age",
            "url",
            "ip",
            "uuid",
            "port",
            "date",
            "year",
            "password",
            "price",
            "amount",
            "count",
            "address",
            "street",
        ];
        let matches: Vec<&str> = test_headers
            .iter()
            .filter(|h| header_hint(h).is_some())
            .copied()
            .collect();
        assert!(
            matches.len() >= 20,
            "Expected at least 20 matches, got {}: {:?}",
            matches.len(),
            matches
        );
    }

    // ── NNFT-076: Small-integer disambiguation tests ────────────────────

    #[test]
    fn test_boolean_override_with_current_model_label() {
        // The actual model outputs "technology.development.boolean", not the
        // previously-checked labels. Verify the override fires for this label.
        let values: Vec<String> = vec!["0", "1", "2", "3", "0", "1", "4", "0", "5", "8"]
            .into_iter()
            .map(String::from)
            .collect();
        let top_labels = vec![
            "technology.development.boolean",
            "representation.numeric.integer_number",
        ];

        let result = disambiguate_boolean_override(&values, &top_labels);
        assert!(
            result.is_some(),
            "Boolean override must trigger for technology.development.boolean"
        );
        let (label, rule) = result.unwrap();
        assert_eq!(label, "representation.numeric.integer_number");
        assert_eq!(rule, "boolean_override_integer_spread");
    }

    #[test]
    fn test_boolean_override_preserves_real_boolean_current_label() {
        // Actual {0,1} boolean column with current model label should NOT override
        let values: Vec<String> = vec!["0", "1", "0", "1", "1", "0", "0", "1", "0", "1"]
            .into_iter()
            .map(String::from)
            .collect();
        let top_labels = vec!["technology.development.boolean"];

        let result = disambiguate_boolean_override(&values, &top_labels);
        assert!(
            result.is_none(),
            "Real boolean {{0,1}} must not be overridden"
        );
    }

    #[test]
    fn test_small_integer_ordinal_pclass() {
        // Pclass: values {1, 2, 3} with repetitions, model says day_of_month
        let values: Vec<String> = vec!["3", "1", "3", "1", "3", "2", "3", "1", "3", "3", "1", "2"]
            .into_iter()
            .map(String::from)
            .collect();
        let top_labels = vec![
            "datetime.component.day_of_month",
            "representation.numeric.integer_number",
        ];

        let result = disambiguate_small_integer_ordinal(&values, &top_labels);
        assert!(
            result.is_some(),
            "Small integer ordinal should fire for Pclass"
        );
        let (label, rule) = result.unwrap();
        assert_eq!(label, "representation.discrete.ordinal");
        assert_eq!(rule, "small_integer_ordinal");
    }

    #[test]
    fn test_small_integer_ordinal_skips_boolean() {
        // Pure {0, 1} column should not become ordinal
        let values: Vec<String> = vec!["0", "1", "0", "1", "1", "0", "0", "1"]
            .into_iter()
            .map(String::from)
            .collect();
        let top_labels = vec!["datetime.component.day_of_month"];

        let result = disambiguate_small_integer_ordinal(&values, &top_labels);
        assert!(result.is_none(), "Pure {{0,1}} should not be ordinal");
    }

    #[test]
    fn test_small_integer_ordinal_skips_large_range() {
        // Integers with max > 20 should not trigger ordinal
        let values: Vec<String> = vec!["1", "5", "10", "25", "50", "1", "5", "25"]
            .into_iter()
            .map(String::from)
            .collect();
        let top_labels = vec!["datetime.component.day_of_month"];

        let result = disambiguate_small_integer_ordinal(&values, &top_labels);
        assert!(
            result.is_none(),
            "Large-range integers should not be ordinal"
        );
    }

    #[test]
    fn test_small_integer_ordinal_ratings() {
        // Star ratings: {1, 2, 3, 4, 5} with repetitions
        let values: Vec<String> = vec!["5", "4", "3", "5", "2", "4", "5", "1", "3", "4", "5", "5"]
            .into_iter()
            .map(String::from)
            .collect();
        let top_labels = vec![
            "datetime.component.day_of_month",
            "representation.numeric.integer_number",
        ];

        let result = disambiguate_small_integer_ordinal(&values, &top_labels);
        assert!(result.is_some(), "Star ratings should be ordinal");
        let (label, _) = result.unwrap();
        assert_eq!(label, "representation.discrete.ordinal");
    }

    // ── NNFT-076: New header hint tests ─────────────────────────────────

    #[test]
    fn test_header_hint_class_columns() {
        assert_eq!(
            header_hint("Pclass"),
            Some("representation.discrete.ordinal")
        );
        assert_eq!(
            header_hint("class"),
            Some("representation.discrete.ordinal")
        );
        assert_eq!(
            header_hint("grade"),
            Some("representation.discrete.ordinal")
        );
        assert_eq!(
            header_hint("rating"),
            Some("representation.discrete.ordinal")
        );
    }

    #[test]
    fn test_header_hint_count_columns() {
        assert_eq!(
            header_hint("SibSp"),
            Some("representation.numeric.integer_number")
        );
        assert_eq!(
            header_hint("Parch"),
            Some("representation.numeric.integer_number")
        );
        assert_eq!(
            header_hint("siblings"),
            Some("representation.numeric.integer_number")
        );
        assert_eq!(
            header_hint("children"),
            Some("representation.numeric.integer_number")
        );
        assert_eq!(
            header_hint("qty"),
            Some("representation.numeric.integer_number")
        );
    }

    #[test]
    fn test_header_hint_survival_columns() {
        assert_eq!(
            header_hint("Survived"),
            Some("representation.boolean.binary")
        );
        assert_eq!(header_hint("alive"), Some("representation.boolean.binary"));
        assert_eq!(header_hint("active"), Some("representation.boolean.binary"));
    }

    #[test]
    fn test_header_hint_ticket_cabin() {
        assert_eq!(
            header_hint("Ticket"),
            Some("representation.alphanumeric.alphanumeric_id")
        );
        assert_eq!(
            header_hint("Cabin"),
            Some("representation.alphanumeric.alphanumeric_id")
        );
        assert_eq!(
            header_hint("seat"),
            Some("representation.alphanumeric.alphanumeric_id")
        );
    }

    #[test]
    fn test_header_hint_embarked() {
        assert_eq!(
            header_hint("Embarked"),
            Some("representation.discrete.categorical")
        );
        assert_eq!(
            header_hint("terminal"),
            Some("representation.discrete.categorical")
        );
    }

    #[test]
    fn test_header_hint_fare() {
        assert_eq!(
            header_hint("Fare"),
            Some("representation.numeric.decimal_number")
        );
        assert_eq!(
            header_hint("fee"),
            Some("representation.numeric.decimal_number")
        );
    }

    #[test]
    fn test_header_hint_class_keyword_matching() {
        // Keyword matching for compound names containing "class"
        assert_eq!(
            header_hint("passenger_class"),
            Some("representation.discrete.ordinal")
        );
        assert_eq!(
            header_hint("ticket_class"),
            Some("representation.discrete.ordinal")
        );
    }

    // ── NNFT-084: SI number override tests ─────────────────────────────

    #[test]
    fn test_si_number_override_plain_decimals() {
        // Plain decimal values with no SI suffixes → should override to decimal_number
        let values: Vec<String> = vec!["5.1", "3.5", "1.4", "7.9", "0.2", "4.6"]
            .into_iter()
            .map(String::from)
            .collect();
        let result = disambiguate_si_number(&values);
        assert_eq!(
            result,
            Some((
                "representation.numeric.decimal_number".to_string(),
                "si_number_override_no_suffix".to_string()
            ))
        );
    }

    #[test]
    fn test_si_number_override_real_si_values() {
        // Values with SI suffixes → should NOT override
        let values: Vec<String> = vec!["5.1K", "3.5M", "1.4B"]
            .into_iter()
            .map(String::from)
            .collect();
        let result = disambiguate_si_number(&values);
        assert_eq!(result, None);
    }

    #[test]
    fn test_si_number_override_mixed_values() {
        // Even one SI suffix means the column is genuinely SI → no override
        let values: Vec<String> = vec!["5.1", "3.5K", "1.4"]
            .into_iter()
            .map(String::from)
            .collect();
        let result = disambiguate_si_number(&values);
        assert_eq!(result, None);
    }

    #[test]
    fn test_si_number_override_negative_decimals() {
        // Negative decimals with no suffixes → should override
        let values: Vec<String> = vec!["-450.12", "732.57", "-1.003", "98.6"]
            .into_iter()
            .map(String::from)
            .collect();
        let result = disambiguate_si_number(&values);
        assert_eq!(
            result,
            Some((
                "representation.numeric.decimal_number".to_string(),
                "si_number_override_no_suffix".to_string()
            ))
        );
    }

    #[test]
    fn test_si_number_override_empty_values() {
        // Empty values → should override (no SI suffixes found)
        let values: Vec<String> = vec!["", "  ", ""].into_iter().map(String::from).collect();
        let result = disambiguate_si_number(&values);
        assert_eq!(
            result,
            Some((
                "representation.numeric.decimal_number".to_string(),
                "si_number_override_no_suffix".to_string()
            ))
        );
    }

    // ── NNFT-090: Day-of-week / month name / boolean sub-type tests ─────

    #[test]
    fn test_day_of_week_full_names() {
        let values: Vec<String> = vec![
            "Monday",
            "Tuesday",
            "Wednesday",
            "Thursday",
            "Friday",
            "Saturday",
            "Sunday",
            "Monday",
            "Friday",
            "Wednesday",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        let result = disambiguate_day_of_week(&values);
        assert_eq!(result, Some("datetime.component.day_of_week".to_string()));
    }

    #[test]
    fn test_day_of_week_abbreviated() {
        let values: Vec<String> = vec![
            "Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun", "Mon", "Fri", "Wed",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        let result = disambiguate_day_of_week(&values);
        assert_eq!(result, Some("datetime.component.day_of_week".to_string()));
    }

    #[test]
    fn test_day_of_week_two_letter() {
        let values: Vec<String> = vec!["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"]
            .into_iter()
            .map(String::from)
            .collect();

        let result = disambiguate_day_of_week(&values);
        assert_eq!(result, Some("datetime.component.day_of_week".to_string()));
    }

    #[test]
    fn test_day_of_week_not_triggered_for_names() {
        let values: Vec<String> = vec!["Alice", "Bob", "Charlie", "David", "Eve", "Frank", "Grace"]
            .into_iter()
            .map(String::from)
            .collect();

        let result = disambiguate_day_of_week(&values);
        assert_eq!(result, None);
    }

    #[test]
    fn test_day_of_week_too_few_values() {
        let values: Vec<String> = vec!["Monday", "Tuesday"]
            .into_iter()
            .map(String::from)
            .collect();

        let result = disambiguate_day_of_week(&values);
        assert_eq!(result, None);
    }

    #[test]
    fn test_day_of_week_below_threshold() {
        // Only 2 of 10 are day names (20% < 80%)
        let values: Vec<String> = vec![
            "Monday", "Apple", "Banana", "Cherry", "Date", "Fig", "Grape", "Tuesday", "Kiwi",
            "Lemon",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        let result = disambiguate_day_of_week(&values);
        assert_eq!(result, None);
    }

    #[test]
    fn test_month_name_full_names() {
        let values: Vec<String> = vec![
            "January",
            "February",
            "March",
            "April",
            "May",
            "June",
            "July",
            "August",
            "September",
            "October",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        let result = disambiguate_month_name(&values);
        assert_eq!(result, Some("datetime.component.month_name".to_string()));
    }

    #[test]
    fn test_month_name_abbreviated() {
        let values: Vec<String> = vec![
            "Jan", "Feb", "Mar", "Apr", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        let result = disambiguate_month_name(&values);
        assert_eq!(result, Some("datetime.component.month_name".to_string()));
    }

    #[test]
    fn test_month_name_mixed_case() {
        let values: Vec<String> = vec![
            "january", "FEBRUARY", "March", "april", "MAY", "June", "july", "AUGUST",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        let result = disambiguate_month_name(&values);
        assert_eq!(result, Some("datetime.component.month_name".to_string()));
    }

    #[test]
    fn test_month_name_not_triggered_for_names() {
        // "May" overlaps with month name, but others don't
        let values: Vec<String> = vec![
            "Alice", "Bob", "Charlie", "David", "Eve", "Frank", "Grace", "Helen", "Ivan", "Jack",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        let result = disambiguate_month_name(&values);
        assert_eq!(result, None);
    }

    #[test]
    fn test_month_name_too_few_values() {
        let values: Vec<String> = vec!["January", "February"]
            .into_iter()
            .map(String::from)
            .collect();

        let result = disambiguate_month_name(&values);
        assert_eq!(result, None);
    }

    #[test]
    fn test_boolean_subtype_terms() {
        let values: Vec<String> = vec![
            "True", "False", "True", "True", "False", "True", "False", "False", "True", "False",
        ]
        .into_iter()
        .map(String::from)
        .collect();
        let top_labels = vec!["representation.boolean.terms"];

        let result = disambiguate_boolean_subtype(&values, &top_labels);
        assert!(result.is_some());
        let (label, rule) = result.unwrap();
        assert_eq!(label, "representation.boolean.terms");
        assert_eq!(rule, "boolean_subtype_terms");
    }

    #[test]
    fn test_boolean_subtype_binary() {
        let values: Vec<String> = vec!["0", "1", "0", "1", "1", "0", "0", "1", "0", "1"]
            .into_iter()
            .map(String::from)
            .collect();
        let top_labels = vec!["representation.boolean.binary"];

        let result = disambiguate_boolean_subtype(&values, &top_labels);
        assert!(result.is_some());
        let (label, rule) = result.unwrap();
        assert_eq!(label, "representation.boolean.binary");
        assert_eq!(rule, "boolean_subtype_binary");
    }

    #[test]
    fn test_boolean_subtype_initials() {
        let values: Vec<String> = vec!["T", "F", "T", "F", "T", "T", "F", "F", "T", "F"]
            .into_iter()
            .map(String::from)
            .collect();
        let top_labels = vec!["representation.boolean.initials"];

        let result = disambiguate_boolean_subtype(&values, &top_labels);
        assert!(result.is_some());
        let (label, rule) = result.unwrap();
        assert_eq!(label, "representation.boolean.initials");
        assert_eq!(rule, "boolean_subtype_initials");
    }

    #[test]
    fn test_boolean_subtype_yes_no() {
        let values: Vec<String> = vec!["yes", "no", "yes", "yes", "no", "no", "yes", "no"]
            .into_iter()
            .map(String::from)
            .collect();
        let top_labels = vec!["representation.boolean.terms"];

        let result = disambiguate_boolean_subtype(&values, &top_labels);
        assert!(result.is_some());
        let (label, _) = result.unwrap();
        assert_eq!(label, "representation.boolean.terms");
    }

    #[test]
    fn test_boolean_subtype_override_categorical() {
        // True/False column misclassified as categorical — boolean detection fires
        // because ≥80% of values are boolean-like terms
        let values: Vec<String> = vec![
            "True", "False", "True", "True", "False", "True", "False", "False",
        ]
        .into_iter()
        .map(String::from)
        .collect();
        let top_labels = vec!["representation.discrete.categorical"];

        let result = disambiguate_boolean_subtype(&values, &top_labels);
        assert!(
            result.is_some(),
            "Should override categorical for True/False column"
        );
        let (label, _) = result.unwrap();
        assert_eq!(label, "representation.boolean.terms");
    }

    #[test]
    fn test_boolean_subtype_not_triggered_for_mixed() {
        // Mixed values that aren't clearly boolean
        let values: Vec<String> = vec!["yes", "no", "maybe", "yes", "unknown", "no"]
            .into_iter()
            .map(String::from)
            .collect();
        let top_labels = vec!["representation.text.word"];

        let result = disambiguate_boolean_subtype(&values, &top_labels);
        assert!(result.is_none());
    }

    #[test]
    fn test_boolean_subtype_too_few_values() {
        let values: Vec<String> = vec!["True", "False"]
            .into_iter()
            .map(String::from)
            .collect();
        let top_labels = vec!["representation.boolean.terms"];

        let result = disambiguate_boolean_subtype(&values, &top_labels);
        assert!(result.is_none());
    }

    #[test]
    fn test_boolean_subtype_skewed_integers_not_binary() {
        // SibSp-like column: mostly 0s and 1s but with values up to 8
        // Should NOT be classified as binary despite >80% being 0/1
        let values: Vec<String> = vec![
            "0", "1", "0", "1", "0", "0", "1", "0", "0", "1", "2", "0", "3", "0", "1", "0", "0",
            "1", "4", "0",
        ]
        .into_iter()
        .map(String::from)
        .collect();
        let top_labels = vec![
            "representation.numeric.integer_number",
            "representation.boolean.binary",
        ];

        let result = disambiguate_boolean_subtype(&values, &top_labels);
        // >2 unique values (0,1,2,3,4) → should NOT fire for binary
        assert!(
            result.is_none(),
            "Skewed integer column (SibSp-like) should not be classified as boolean"
        );
    }

    #[test]
    fn test_boolean_subtype_pure_binary_still_works() {
        // Pure 0/1 column with exactly 2 unique values → should still be binary
        let values: Vec<String> = vec!["0", "1", "1", "0", "0", "1", "0", "1", "1", "0"]
            .into_iter()
            .map(String::from)
            .collect();
        let top_labels = vec!["representation.boolean.binary"];

        let result = disambiguate_boolean_subtype(&values, &top_labels);
        assert!(result.is_some());
        let (label, _) = result.unwrap();
        assert_eq!(label, "representation.boolean.binary");
    }

    /// Integration test: verify that semantic hint classifier influences column classification.
    /// Skips if Model2Vec model files are not present.
    #[test]
    fn test_classify_column_with_semantic_hint() {
        use crate::semantic::SemanticHintClassifier;

        let model_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("models")
            .join("model2vec");

        if !model_dir.join("model.safetensors").exists() {
            eprintln!("Skipping semantic column integration test: models/model2vec not found");
            return;
        }

        let semantic = SemanticHintClassifier::load(&model_dir).unwrap();

        // Create a mock classifier that delegates value-level inference
        // We use a simple stub here — the semantic hint should override generic
        // value predictions when the header name is semantically clear.
        let base_classifier =
            crate::inference::MockClassifier::new("representation.numeric.decimal_number");
        let column_classifier = ColumnClassifier::with_semantic_hint(
            Box::new(base_classifier),
            ColumnConfig::default(),
            semantic,
        );

        // The base classifier always returns decimal_number, but the semantic hint
        // for "weight_kg" should override to identity.person.weight
        let values: Vec<String> = vec!["72.5", "85.0", "63.2", "90.1"]
            .into_iter()
            .map(String::from)
            .collect();
        let result = column_classifier
            .classify_column_with_header(&values, "weight_kg")
            .unwrap();
        assert_eq!(
            result.label, "identity.person.weight",
            "Semantic hint for 'weight_kg' should override generic decimal_number"
        );

        // Generic column names should NOT override (semantic hint returns None)
        let result2 = column_classifier
            .classify_column_with_header(&values, "col1")
            .unwrap();
        assert_eq!(
            result2.label, "representation.numeric.decimal_number",
            "Generic 'col1' should not trigger semantic override"
        );
    }

    // ── Attractor demotion tests (Rule 14) ──────────────────────────────

    #[test]
    fn test_attractor_validation_demotion() {
        // Values that fail CVV validation (^[0-9]{3,4}$): negative numbers and
        // 5+ digit integers — should demote to integer_number
        let values: Vec<String> = vec![
            "-200", "15000", "3500", "-50", "12000", "800", "25000", "-100", "45000", "600",
        ]
        .into_iter()
        .map(String::from)
        .collect();
        let votes = vec![
            ("identity.payment.cvv".to_string(), 9),
            ("representation.numeric.integer_number".to_string(), 1),
        ];

        let yaml = r#"
identity.payment.cvv:
  title: "CVV"
  validation:
    type: string
    pattern: "^[0-9]{3,4}$"
    minLength: 3
    maxLength: 4
  tier: [VARCHAR, identity, payment]
  release_priority: 5
  samples: ["123"]
"#;
        let taxonomy = Taxonomy::from_yaml(yaml).unwrap();

        let result = disambiguate_attractor_demotion(&values, &votes, 10, Some(&taxonomy));
        assert!(
            result.is_some(),
            "Should demote CVV when values fail validation"
        );
        let (label, rule) = result.unwrap();
        assert_eq!(label, "representation.numeric.integer_number");
        assert!(rule.starts_with("attractor_demotion_validation:"));
    }

    #[test]
    fn test_attractor_confidence_demotion() {
        // Low confidence postal_code prediction (0.6) — should demote
        let values: Vec<String> = vec![
            "1500", "2300", "45000", "800", "99", "12", "5600", "340", "78", "4100",
        ]
        .into_iter()
        .map(String::from)
        .collect();
        let votes = vec![
            ("geography.address.postal_code".to_string(), 6),
            ("representation.numeric.integer_number".to_string(), 4),
        ];

        let result = disambiguate_attractor_demotion(&values, &votes, 10, None);
        assert!(
            result.is_some(),
            "Should demote postal_code at 0.6 confidence"
        );
        let (label, rule) = result.unwrap();
        assert_eq!(label, "representation.numeric.integer_number");
        assert!(rule.starts_with("attractor_demotion_confidence:"));
    }

    #[test]
    fn test_attractor_cardinality_demotion() {
        // 4 unique short words classified as first_name at high confidence — categorical
        let values: Vec<String> = vec![
            "Soccer", "Baseball", "Tennis", "Hockey", "Soccer", "Baseball", "Tennis", "Hockey",
            "Soccer", "Baseball",
        ]
        .into_iter()
        .map(String::from)
        .collect();
        let votes = vec![
            ("identity.person.first_name".to_string(), 9),
            ("representation.text.word".to_string(), 1),
        ];

        // High confidence (0.9) — Signal 2 won't fire, but Signal 3 (cardinality) should
        let result = disambiguate_attractor_demotion(&values, &votes, 10, None);
        assert!(
            result.is_some(),
            "Should demote first_name with 4 unique values"
        );
        let (label, rule) = result.unwrap();
        assert_eq!(label, "representation.discrete.categorical");
        assert!(rule.starts_with("attractor_demotion_cardinality:"));
    }

    #[test]
    fn test_attractor_cardinality_single_value() {
        // Single unique value (e.g., airports.type = "airport" repeated) — categorical
        let values: Vec<String> = vec![
            "airport", "airport", "airport", "airport", "airport", "airport", "airport", "airport",
            "airport", "airport",
        ]
        .into_iter()
        .map(String::from)
        .collect();
        let votes = vec![("identity.person.first_name".to_string(), 10)];

        // Cardinality 1 — strongest signal that this is NOT a person's name
        let result = disambiguate_attractor_demotion(&values, &votes, 10, None);
        assert!(
            result.is_some(),
            "Should demote first_name with 1 unique value"
        );
        let (label, rule) = result.unwrap();
        assert_eq!(label, "representation.discrete.categorical");
        assert!(rule.starts_with("attractor_demotion_cardinality:"));
    }

    #[test]
    fn test_attractor_validation_confirmed_skips_signal2() {
        // ICAO code at low confidence (0.6) but values pass validation → no demotion
        // This tests that validation confirmation gates Signal 2.
        let values: Vec<String> = vec!["EGLL", "KJFK", "LFPG", "EDDF", "RJTT", "VHHH"]
            .into_iter()
            .map(String::from)
            .collect();
        let votes = vec![
            ("geography.transportation.icao_code".to_string(), 6),
            ("representation.alphanumeric.alphanumeric_id".to_string(), 4),
        ];

        let yaml = r#"
geography.transportation.icao_code:
  title: "ICAO Code"
  validation:
    type: string
    pattern: "^[A-Z]{4}$"
    minLength: 4
    maxLength: 4
  tier: [VARCHAR, geography, transportation]
  release_priority: 5
  samples: ["EGLL"]
"#;
        let taxonomy = Taxonomy::from_yaml(yaml).unwrap();

        // Confidence 0.6 < 0.85 → Signal 2 would fire, BUT validation
        // pattern passes → validation_confirmed = true → Signal 2 skipped
        let result = disambiguate_attractor_demotion(&values, &votes, 10, Some(&taxonomy));
        assert!(
            result.is_none(),
            "Should NOT demote ICAO codes when validation confirms them"
        );
    }

    #[test]
    fn test_attractor_no_demotion_true_positive() {
        // Actual CVV values at high confidence (>0.85) — should NOT demote
        let values: Vec<String> = vec![
            "123", "456", "789", "012", "345", "678", "901", "234", "567", "890",
        ]
        .into_iter()
        .map(String::from)
        .collect();
        let votes = vec![("identity.payment.cvv".to_string(), 10)];

        let yaml = r#"
identity.payment.cvv:
  title: "CVV"
  validation:
    type: string
    pattern: "^[0-9]{3,4}$"
    minLength: 3
    maxLength: 4
  tier: [VARCHAR, identity, payment]
  release_priority: 5
  samples: ["123"]
"#;
        let taxonomy = Taxonomy::from_yaml(yaml).unwrap();

        // All pass validation AND confidence is 1.0 → no demotion
        let result = disambiguate_attractor_demotion(&values, &votes, 10, Some(&taxonomy));
        assert!(
            result.is_none(),
            "Should NOT demote actual CVVs at high confidence"
        );
    }

    #[test]
    fn test_attractor_no_demotion_high_confidence() {
        // Attractor at >0.85 with valid values — should NOT demote
        let values: Vec<String> = vec![
            "10001", "90210", "30301", "60601", "02101", "75001", "33101", "94102", "20001",
            "98101",
        ]
        .into_iter()
        .map(String::from)
        .collect();
        let votes = vec![
            ("geography.address.postal_code".to_string(), 9),
            ("representation.numeric.integer_number".to_string(), 1),
        ];

        let yaml = r#"
geography.address.postal_code:
  title: "Postal Code"
  validation:
    type: string
    pattern: "^[0-9]{3,10}$"
    minLength: 3
    maxLength: 10
  tier: [VARCHAR, geography, address]
  release_priority: 5
  samples: ["10001"]
"#;
        let taxonomy = Taxonomy::from_yaml(yaml).unwrap();

        // All pass validation AND confidence is 0.9 → no demotion
        let result = disambiguate_attractor_demotion(&values, &votes, 10, Some(&taxonomy));
        assert!(
            result.is_none(),
            "Should NOT demote real postal codes at 0.9 confidence"
        );
    }

    #[test]
    fn test_select_fallback_numeric() {
        // All integer values, no representation.* in votes
        let values: Vec<String> = vec!["100", "200", "300"]
            .into_iter()
            .map(String::from)
            .collect();
        let votes = vec![
            ("geography.address.postal_code".to_string(), 8),
            ("geography.address.street_number".to_string(), 2),
        ];

        let result = select_fallback(&votes, true, false, false, &values);
        assert_eq!(result, "representation.numeric.integer_number");
    }

    #[test]
    fn test_select_fallback_text() {
        let values: Vec<String> = vec!["Soccer", "Tennis"]
            .into_iter()
            .map(String::from)
            .collect();
        let votes = vec![
            ("identity.person.first_name".to_string(), 8),
            ("identity.person.username".to_string(), 2),
        ];

        let result = select_fallback(&votes, false, true, false, &values);
        assert_eq!(result, "representation.discrete.categorical");
    }

    #[test]
    fn test_select_fallback_from_votes() {
        // representation.* type exists in the vote distribution → use it
        let values: Vec<String> = vec!["100", "200"].into_iter().map(String::from).collect();
        let votes = vec![
            ("geography.address.postal_code".to_string(), 6),
            ("representation.numeric.decimal_number".to_string(), 3),
            ("geography.address.street_number".to_string(), 1),
        ];

        let result = select_fallback(&votes, true, false, false, &values);
        assert_eq!(
            result, "representation.numeric.decimal_number",
            "Should use representation.* type from votes when available"
        );
    }

    // ── Locale-aware attractor demotion tests ───────────────────────────

    #[test]
    fn test_attractor_demotion_locale_validation_demotes_salary() {
        // Salary column predicted as postal_code: 6-digit values fail ALL locale
        // patterns. Using values clearly in salary range (>99999) that cannot
        // be valid postal codes in any locale.
        let values: Vec<String> = vec![
            "102000", "245000", "112000", "350000", "178000", "195000", "267000", "188000",
            "103000", "272000",
        ]
        .into_iter()
        .map(String::from)
        .collect();
        let votes = vec![
            ("geography.address.postal_code".to_string(), 9),
            ("representation.numeric.integer_number".to_string(), 1),
        ];

        let yaml = r#"
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
        let mut taxonomy = Taxonomy::from_yaml(yaml).unwrap();
        taxonomy.compile_validators();
        taxonomy.compile_locale_validators();

        let result = disambiguate_attractor_demotion(&values, &votes, 10, Some(&taxonomy));
        assert!(
            result.is_some(),
            "Should demote salary values despite matching universal validation"
        );
        let (label, rule) = result.unwrap();
        assert_eq!(label, "representation.numeric.integer_number");
        assert!(
            rule.starts_with("attractor_demotion_validation:"),
            "Should demote via validation signal, got: {}",
            rule
        );
    }

    #[test]
    fn test_attractor_accepts_real_us_postal_codes() {
        // Real US ZIP codes: match EN_US locale pattern → locale-confirmed, no demotion
        let values: Vec<String> = vec![
            "10001", "90210", "30301", "60601", "02101", "75001", "33101", "94102", "20001",
            "98101",
        ]
        .into_iter()
        .map(String::from)
        .collect();
        let votes = vec![
            ("geography.address.postal_code".to_string(), 9),
            ("representation.numeric.integer_number".to_string(), 1),
        ];

        let yaml = r#"
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
        let mut taxonomy = Taxonomy::from_yaml(yaml).unwrap();
        taxonomy.compile_validators();
        taxonomy.compile_locale_validators();

        // US ZIPs match EN_US locale at >50% → locale-confirmed → no demotion
        let result = disambiguate_attractor_demotion(&values, &votes, 10, Some(&taxonomy));
        assert!(
            result.is_none(),
            "Should NOT demote real US ZIP codes (locale-confirmed by EN_US)"
        );
    }

    #[test]
    fn test_attractor_accepts_real_uk_postcodes() {
        // Real UK postcodes: match EN_GB locale pattern → locale-confirmed, no demotion
        let values: Vec<String> = vec![
            "EC1A 1BB", "W1C 1AX", "M2 5BQ", "SW1A 1AA", "B1 1BB", "LS1 1BA",
        ]
        .into_iter()
        .map(String::from)
        .collect();
        let votes = vec![("geography.address.postal_code".to_string(), 6)];

        let yaml = r#"
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
  tier: [VARCHAR, address]
  release_priority: 4
  samples: ["10001"]
"#;
        let mut taxonomy = Taxonomy::from_yaml(yaml).unwrap();
        taxonomy.compile_validators();
        taxonomy.compile_locale_validators();

        let result = disambiguate_attractor_demotion(&values, &votes, 6, Some(&taxonomy));
        assert!(
            result.is_none(),
            "Should NOT demote real UK postcodes (locale-confirmed by EN_GB)"
        );
    }

    #[test]
    fn test_attractor_locale_low_confidence_accepted() {
        // US ZIP codes at low confidence (0.6) — normally Signal 2 would demote,
        // but locale validation confirms the type → no demotion
        let values: Vec<String> = vec![
            "10001", "90210", "30301", "60601", "02101", "75001", "33101", "94102", "20001",
            "98101",
        ]
        .into_iter()
        .map(String::from)
        .collect();
        let votes = vec![
            ("geography.address.postal_code".to_string(), 6),
            ("representation.numeric.integer_number".to_string(), 4),
        ];

        let yaml = r#"
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
  tier: [VARCHAR, address]
  release_priority: 4
  samples: ["10001"]
"#;
        let mut taxonomy = Taxonomy::from_yaml(yaml).unwrap();
        taxonomy.compile_validators();
        taxonomy.compile_locale_validators();

        // Confidence 0.6 < 0.85 → Signal 2 would fire, BUT locale validation
        // confirms the type → locale_confirmed = true → Signal 2 skipped
        let result = disambiguate_attractor_demotion(&values, &votes, 10, Some(&taxonomy));
        assert!(
            result.is_none(),
            "Should NOT demote real US ZIPs at low confidence when locale confirms them"
        );
    }

    #[test]
    fn test_attractor_locale_confirmed_skips_cardinality() {
        // NNFT-132: Phone numbers with locale confirmation should NOT be demoted
        // by Signal 3 (cardinality), even with few unique values. Small tables
        // with legitimate phone numbers are common in web-scraped datasets.
        let values: Vec<String> = vec![
            "(805) 638-3078",
            "(650) 440-2450",
            "(805) 638-3078",
            "(805) 638-3078",
            "(650) 440-2450",
            "(650) 440-2450",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        // 2 unique values, 6 total — classic cardinality demotion target
        let votes = vec![("identity.person.phone_number".to_string(), 6)];

        let yaml = r#"
identity.person.phone_number:
  title: "Phone Number"
  validation:
    type: string
    minLength: 7
    maxLength: 20
    pattern: "^[+]?[0-9\\s()\\-\\.]+$"
  validation_by_locale:
    EN_US:
      type: string
      pattern: "^(\\+?1[\\s\\-\\.]*)?\\(?\\d{3}\\)?[\\s\\-\\.]*\\d{3}[\\s\\-\\.]*\\d{4}$"
      minLength: 10
      maxLength: 18
  tier: [VARCHAR, person]
  release_priority: 4
  samples: ["+1 (555) 123-4567"]
"#;
        let mut taxonomy = Taxonomy::from_yaml(yaml).unwrap();
        taxonomy.compile_validators();
        taxonomy.compile_locale_validators();

        let result = disambiguate_attractor_demotion(&values, &votes, 6, Some(&taxonomy));
        assert!(
            result.is_none(),
            "Should NOT demote phone numbers when locale-confirmed, even with 2 unique values"
        );
    }

    #[test]
    fn test_attractor_universal_only_does_not_confirm_locale_type() {
        // NNFT-132 Precision Principle: For locale-specific types, passing the
        // universal validation pattern does NOT count as confirmation. Only locale
        // patterns can confirm. These values pass universal phone validation
        // (digits + formatting chars) but don't match any locale pattern.
        let values: Vec<String> = vec![
            "123-456", "789-012", "345-678", "123-456", "789-012", "345-678",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        // Low confidence — Signal 2 would fire if not confirmed
        let votes = vec![
            ("identity.person.phone_number".to_string(), 4),
            ("representation.discrete.categorical".to_string(), 2),
        ];

        let yaml = r#"
identity.person.phone_number:
  title: "Phone Number"
  validation:
    type: string
    minLength: 7
    maxLength: 20
    pattern: "^[+]?[0-9\\s()\\-\\.]+$"
  validation_by_locale:
    EN_US:
      type: string
      pattern: "^(\\+?1[\\s\\-\\.]*)?\\(?\\d{3}\\)?[\\s\\-\\.]*\\d{3}[\\s\\-\\.]*\\d{4}$"
      minLength: 10
      maxLength: 18
  tier: [VARCHAR, person]
  release_priority: 4
  samples: ["+1 (555) 123-4567"]
"#;
        let mut taxonomy = Taxonomy::from_yaml(yaml).unwrap();
        taxonomy.compile_validators();
        taxonomy.compile_locale_validators();

        // Confidence 0.67 < 0.85 AND no locale confirmation → should demote
        // despite universal pattern matching (precision principle)
        let result = disambiguate_attractor_demotion(&values, &votes, 6, Some(&taxonomy));
        assert!(
            result.is_some(),
            "Should demote phone_number at low confidence when only universal validates (no locale)"
        );
        let (_, rule) = result.unwrap();
        assert!(
            rule.starts_with("attractor_demotion_confidence:"),
            "Should demote via confidence signal, got: {}",
            rule
        );
    }

    #[test]
    fn test_attractor_first_name_cardinality_unchanged() {
        // NNFT-132: first_name has no locale validators, so cardinality demotion
        // still works exactly as before — this is a regression guard.
        let values: Vec<String> = vec![
            "John", "Jane", "Bob", "John", "Jane", "Bob", "John", "Jane", "Bob", "John",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        let votes = vec![
            ("identity.person.first_name".to_string(), 9),
            ("representation.text.word".to_string(), 1),
        ];

        // 3 unique values, text attractor, no locale validators → must still demote
        let result = disambiguate_attractor_demotion(&values, &votes, 10, None);
        assert!(
            result.is_some(),
            "first_name with 3 unique values should still be demoted (no locale validators)"
        );
        let (label, rule) = result.unwrap();
        assert_eq!(label, "representation.discrete.categorical");
        assert!(
            rule.starts_with("attractor_demotion_cardinality:"),
            "Should demote via cardinality signal, got: {}",
            rule
        );
    }

    // ── Duration override tests ─────────────────────────────────────────

    #[test]
    fn test_duration_override_standard_durations() {
        // Standard ISO 8601 durations like PT20M (20 minutes), PT1H (1 hour)
        let values: Vec<String> = vec![
            "PT20M", "PT30M", "PT10M", "PT15M", "PT1H", "PT45M", "PT5M", "PT60M",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        let result = disambiguate_duration_override(&values);
        assert!(result.is_some(), "Should detect ISO 8601 durations");
        let (label, rule) = result.unwrap();
        assert_eq!(label, "datetime.duration.iso_8601");
        assert_eq!(rule, "duration_override_sedol");
    }

    #[test]
    fn test_duration_override_complex_durations() {
        // Complex durations with multiple components: P1DT12H, P2Y3M, PT1H30M
        let values: Vec<String> = vec!["P1DT12H", "P2Y3M", "PT1H30M", "P30D", "P1Y", "PT2H15M30S"]
            .into_iter()
            .map(String::from)
            .collect();

        let result = disambiguate_duration_override(&values);
        assert!(result.is_some(), "Should detect complex ISO 8601 durations");
        let (label, _) = result.unwrap();
        assert_eq!(label, "datetime.duration.iso_8601");
    }

    #[test]
    fn test_duration_override_malformed_sotab_durations() {
        // Non-standard durations found in SOTAB: PD1TH0M0, PD3TH0M0
        let values: Vec<String> = vec![
            "PD1TH0M0", "PD3TH0M0", "PT30M", "PT20M", "PD1TH0M0", "PT10M",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        let result = disambiguate_duration_override(&values);
        assert!(
            result.is_some(),
            "Should detect non-standard duration variants"
        );
        let (label, _) = result.unwrap();
        assert_eq!(label, "datetime.duration.iso_8601");
    }

    #[test]
    fn test_duration_override_not_triggered_for_sedol() {
        // Real SEDOL codes: 7 alphanumeric chars, restricted charset
        let values: Vec<String> = vec![
            "B0YBKJ7", "B1YW440", "B39J2S1", "B0JNMQ2", "BWFGQN3", "B082RF1",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        let result = disambiguate_duration_override(&values);
        assert!(
            result.is_none(),
            "Should NOT trigger for real SEDOL codes (no duration pattern)"
        );
    }

    #[test]
    fn test_duration_override_not_triggered_below_threshold() {
        // Mixed column: mostly non-duration with a few durations
        let values: Vec<String> = vec![
            "ABC1234", "DEF5678", "GHI9012", "PT20M", "JKL3456", "MNO7890", "PQR1234", "STU5678",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        let result = disambiguate_duration_override(&values);
        assert!(
            result.is_none(),
            "Should NOT trigger when <50% of values are durations"
        );
    }

    #[test]
    fn test_duration_override_week_durations() {
        // Week-based durations: P1W, P2W
        let values: Vec<String> = vec!["P1W", "P2W", "P3W", "P4W", "P1W", "P2W"]
            .into_iter()
            .map(String::from)
            .collect();

        let result = disambiguate_duration_override(&values);
        assert!(result.is_some(), "Should detect week-based durations");
        let (label, _) = result.unwrap();
        assert_eq!(label, "datetime.duration.iso_8601");
    }

    // ── Text length demotion tests ──

    #[test]
    fn test_text_length_demotion_long_text_as_address() {
        // Long descriptions/paragraphs misclassified as full_address
        let values: Vec<String> = vec![
            "Contact information of the hotel Record of Zelenograd: phone, location map, address on the map. Full amenities and services list available.",
            "The layout of the room includes two bedrooms and a spacious lounge, but each room has its own plasma TV for entertainment and relaxation purposes.",
            "Services provided by the hotel Record (Zelenograd): Credit cards (Visa, MasterCard, World), free Wi-Fi, gym, spa, pool, conference rooms available.",
            "STANDARD WITH THE KITCHEN number one category. The Record Hotel has 1 Standard Room with Kitchen area for extended stays and business travelers.",
            "Preheat oven to 350 degrees. Grease a small baking dish or small cast iron skillet and fill with peaches. Sprinkle cinnamon over peaches and set aside.",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        let votes = vec![("geography.address.full_address".to_string(), 5)];
        let result = disambiguate_text_length_demotion(&values, &votes);
        assert!(result.is_some(), "Should demote long text overcall");
        let (label, rule) = result.unwrap();
        assert_eq!(label, "representation.text.sentence");
        assert_eq!(rule, "text_length_demotion_full_address");
    }

    #[test]
    fn test_text_length_demotion_real_address_not_demoted() {
        // Real addresses should NOT be demoted (typical length 20-40 chars)
        let values: Vec<String> = vec![
            "123 Main St, Springfield, IL",
            "456 Oak Ave, Portland, OR",
            "789 Pine Rd, Austin, TX",
            "101 Elm Blvd, Denver, CO",
            "202 Maple Dr, Seattle, WA",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        let votes = vec![("geography.address.full_address".to_string(), 5)];
        let result = disambiguate_text_length_demotion(&values, &votes);
        assert!(
            result.is_none(),
            "Should NOT demote real addresses (median ~28 chars)"
        );
    }

    #[test]
    fn test_text_length_demotion_ignores_non_address() {
        // Rule should not fire for non-full_address predictions
        let values: Vec<String> = vec![
            "This is a very long text that exceeds one hundred characters and should demonstrate that length alone does not trigger demotion for other types.",
            "Another very long text value that is clearly longer than the threshold of one hundred characters but is not predicted as full_address by the model.",
            "Yet another long text to ensure the median is above the threshold value for the test to be meaningful in demonstrating the rule only applies to addresses.",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        let votes = vec![("identity.person.full_name".to_string(), 3)];
        let result = disambiguate_text_length_demotion(&values, &votes);
        assert!(
            result.is_none(),
            "Should NOT fire for non-full_address predictions"
        );
    }

    #[test]
    fn test_text_length_demotion_borderline_not_demoted() {
        // Values right at the boundary (median ~95 chars) should NOT be demoted
        let values: Vec<String> = vec![
            "123 Main Street, Apartment 4B, Springfield, Illinois 62704, United States of America — Near the park",
            "456 Oak Avenue, Suite 200, Portland, Oregon 97201, United States of America — Downtown district",
            "789 Pine Road, Building C, Unit 12, Austin, Texas 78701, United States of America — East campus",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        let votes = vec![("geography.address.full_address".to_string(), 3)];
        let result = disambiguate_text_length_demotion(&values, &votes);
        assert!(
            result.is_none(),
            "Should NOT demote borderline addresses (median ~95 chars)"
        );
    }

    // ==========================================================================
    // NNFT-139: Designation-aware is_generic_prediction tests
    // ==========================================================================

    #[test]
    fn test_is_generic_attractor_demoted_always_generic() {
        // Attractor-demoted predictions are always generic, regardless of designation
        let rule = Some("attractor_demotion_validation:something".to_string());
        assert!(
            is_generic_prediction("identity.person.email", &rule, None),
            "Attractor-demoted predictions should always be generic"
        );
    }

    #[test]
    fn test_is_generic_boolean_always_generic() {
        // Boolean types are always generic
        assert!(
            is_generic_prediction("representation.boolean.binary", &None, None),
            "Boolean types should always be generic"
        );
        assert!(
            is_generic_prediction("representation.boolean.terms", &None, None),
            "Boolean types should always be generic"
        );
    }

    #[test]
    fn test_is_generic_broad_words_with_taxonomy() {
        // broad_words designation should make a prediction generic when taxonomy is available
        let yaml = r#"
identity.person.gender:
  title: Gender
  designation: broad_words
  tier: [VARCHAR, identity, person]
  release_priority: 1
  samples: ["Male"]
identity.person.email:
  title: Email
  designation: universal
  tier: [VARCHAR, identity, person]
  release_priority: 5
  samples: ["test@example.com"]
"#;
        let taxonomy = Taxonomy::from_yaml(yaml).unwrap();

        // broad_words → generic
        assert!(
            is_generic_prediction("identity.person.gender", &None, Some(&taxonomy)),
            "broad_words types should be generic when taxonomy is available"
        );

        // universal → not generic (not in hardcoded list either)
        assert!(
            !is_generic_prediction("identity.person.email", &None, Some(&taxonomy)),
            "universal types should NOT be generic (unless in hardcoded list)"
        );
    }

    #[test]
    fn test_is_generic_broad_characters_with_taxonomy() {
        let yaml = r#"
identity.person.password:
  title: Password
  designation: broad_characters
  tier: [VARCHAR, identity, person]
  release_priority: 1
  samples: ["p@ssw0rd"]
"#;
        let taxonomy = Taxonomy::from_yaml(yaml).unwrap();

        assert!(
            is_generic_prediction("identity.person.password", &None, Some(&taxonomy)),
            "broad_characters types should be generic when taxonomy is available"
        );
    }

    #[test]
    fn test_is_generic_broad_numbers_with_taxonomy() {
        let yaml = r#"
representation.numeric.increment:
  title: Increment
  designation: broad_numbers
  tier: [INTEGER, representation, numeric]
  release_priority: 1
  samples: ["42"]
"#;
        let taxonomy = Taxonomy::from_yaml(yaml).unwrap();

        assert!(
            is_generic_prediction("representation.numeric.increment", &None, Some(&taxonomy)),
            "broad_numbers types should be generic when taxonomy is available"
        );
    }

    #[test]
    fn test_is_generic_fallback_without_taxonomy() {
        // Without taxonomy, falls back to hardcoded list
        assert!(
            is_generic_prediction("representation.text.word", &None, None),
            "Hardcoded generic label should be generic without taxonomy"
        );
        assert!(
            is_generic_prediction("identity.person.phone_number", &None, None),
            "Hardcoded generic label should be generic without taxonomy"
        );
        assert!(
            !is_generic_prediction("identity.person.email", &None, None),
            "Non-hardcoded label should NOT be generic without taxonomy"
        );
    }

    #[test]
    fn test_is_generic_locale_specific_not_generic() {
        // locale_specific designation should NOT be generic (when not in hardcoded list)
        let yaml = r#"
geography.address.postal_code:
  title: Postal Code
  designation: locale_specific
  tier: [VARCHAR, geography, address]
  release_priority: 5
  samples: ["90210"]
"#;
        let taxonomy = Taxonomy::from_yaml(yaml).unwrap();

        // postal_code is locale_specific and NOT in the hardcoded list → not generic
        assert!(
            !is_generic_prediction("geography.address.postal_code", &None, Some(&taxonomy)),
            "locale_specific types not in hardcoded list should NOT be generic"
        );
    }

    #[test]
    fn test_is_generic_hardcoded_overrides_taxonomy() {
        // phone_number is in the hardcoded list AND has locale_specific designation.
        // Hardcoded list (Signal 3) takes precedence — the type stays generic so
        // header hints can still override when the model uses it as a catch-all.
        let yaml = r#"
identity.person.phone_number:
  title: Phone Number
  designation: locale_specific
  tier: [VARCHAR, identity, person]
  release_priority: 5
  samples: ["+1 (202) 555-0100"]
"#;
        let taxonomy = Taxonomy::from_yaml(yaml).unwrap();

        assert!(
            is_generic_prediction("identity.person.phone_number", &None, Some(&taxonomy)),
            "phone_number is in hardcoded list — stays generic regardless of designation"
        );
    }

    // ==========================================================================
    // NNFT-140: Post-hoc locale detection tests
    // ==========================================================================

    #[test]
    fn test_detect_locale_us_phone_numbers() {
        // US phone numbers should detect EN_US locale
        let yaml = r#"
identity.person.phone_number:
  title: Phone Number
  designation: locale_specific
  tier: [VARCHAR, identity, person]
  release_priority: 5
  samples: ["+1 (202) 555-0100"]
  validation:
    type: string
    pattern: "^[+]?[0-9\\s()\\-\\.]+$"
  validation_by_locale:
    EN_US:
      type: string
      pattern: "^(\\+?1[\\s\\-./]*)?\\(?\\d{3}\\)?[\\s\\-./]*\\d{3}[\\s\\-./]*\\d{4}$"
      minLength: 10
      maxLength: 30
    EN_GB:
      type: string
      pattern: "^(\\+?44[\\s\\-./]*(\\(0\\))?)?0?\\d{2,5}([\\s\\-./]*\\d{1,8}){1,3}$"
      minLength: 10
      maxLength: 30
"#;
        let mut taxonomy = Taxonomy::from_yaml(yaml).unwrap();
        taxonomy.compile_locale_validators();

        let values: Vec<String> = vec![
            "+1 (202) 555-0100",
            "+1 (415) 555-0199",
            "(312) 555-0142",
            "1-800-555-0123",
            "(617) 555-0187",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        let locale =
            detect_locale_from_validation(&values, "identity.person.phone_number", &taxonomy);
        assert_eq!(
            locale,
            Some("EN_US".to_string()),
            "US phone numbers should detect EN_US"
        );
    }

    #[test]
    fn test_detect_locale_uk_phone_numbers() {
        let yaml = r#"
identity.person.phone_number:
  title: Phone Number
  designation: locale_specific
  tier: [VARCHAR, identity, person]
  release_priority: 5
  samples: ["+44 20 7946 0958"]
  validation:
    type: string
    pattern: "^[+]?[0-9\\s()\\-\\.]+$"
  validation_by_locale:
    EN_US:
      type: string
      pattern: "^(\\+?1[\\s\\-./]*)?\\(?\\d{3}\\)?[\\s\\-./]*\\d{3}[\\s\\-./]*\\d{4}$"
      minLength: 10
      maxLength: 30
    EN_GB:
      type: string
      pattern: "^(\\+?44[\\s\\-./]*(\\(0\\))?)?0?\\d{2,5}([\\s\\-./]*\\d{1,8}){1,3}$"
      minLength: 10
      maxLength: 30
"#;
        let mut taxonomy = Taxonomy::from_yaml(yaml).unwrap();
        taxonomy.compile_locale_validators();

        let values: Vec<String> = vec![
            "+44 20 7946 0958",
            "020 7946 0123",
            "+44 121 496 0987",
            "0161 496 0654",
            "+44 131 496 0321",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        let locale =
            detect_locale_from_validation(&values, "identity.person.phone_number", &taxonomy);
        assert_eq!(
            locale,
            Some("EN_GB".to_string()),
            "UK phone numbers should detect EN_GB"
        );
    }

    #[test]
    fn test_detect_locale_no_validators() {
        // Types without validation_by_locale should return None
        let yaml = r#"
identity.person.email:
  title: Email
  designation: universal
  tier: [VARCHAR, identity, person]
  release_priority: 5
  samples: ["test@example.com"]
"#;
        let taxonomy = Taxonomy::from_yaml(yaml).unwrap();

        let values: Vec<String> = vec!["test@example.com", "user@domain.org"]
            .into_iter()
            .map(String::from)
            .collect();

        let locale = detect_locale_from_validation(&values, "identity.person.email", &taxonomy);
        assert_eq!(
            locale, None,
            "Types without locale validators should return None"
        );
    }

    #[test]
    fn test_detect_locale_no_match_above_threshold() {
        // Values that don't match any locale pattern well enough should return None
        let yaml = r#"
identity.person.phone_number:
  title: Phone Number
  designation: locale_specific
  tier: [VARCHAR, identity, person]
  release_priority: 5
  samples: ["+1 (202) 555-0100"]
  validation_by_locale:
    EN_US:
      type: string
      pattern: "^(\\+?1[\\s\\-./]*)?\\(?\\d{3}\\)?[\\s\\-./]*\\d{3}[\\s\\-./]*\\d{4}$"
      minLength: 10
      maxLength: 30
"#;
        let mut taxonomy = Taxonomy::from_yaml(yaml).unwrap();
        taxonomy.compile_locale_validators();

        // Random strings that don't match any phone pattern
        let values: Vec<String> = vec!["abc", "hello world", "12345", "not-a-phone"]
            .into_iter()
            .map(String::from)
            .collect();

        let locale =
            detect_locale_from_validation(&values, "identity.person.phone_number", &taxonomy);
        assert_eq!(
            locale, None,
            "Non-matching values should not detect any locale"
        );
    }

    #[test]
    fn test_detect_locale_empty_values() {
        let yaml = r#"
identity.person.phone_number:
  title: Phone Number
  designation: locale_specific
  tier: [VARCHAR, identity, person]
  release_priority: 5
  samples: ["+1 (202) 555-0100"]
  validation_by_locale:
    EN_US:
      type: string
      pattern: "^(\\+?1[\\s\\-./]*)?\\(?\\d{3}\\)?[\\s\\-./]*\\d{3}[\\s\\-./]*\\d{4}$"
      minLength: 10
      maxLength: 30
"#;
        let mut taxonomy = Taxonomy::from_yaml(yaml).unwrap();
        taxonomy.compile_locale_validators();

        let values: Vec<String> = vec!["", "", ""].into_iter().map(String::from).collect();

        let locale =
            detect_locale_from_validation(&values, "identity.person.phone_number", &taxonomy);
        assert_eq!(
            locale, None,
            "All-empty values should not detect any locale"
        );
    }
}
