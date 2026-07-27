//! Column-mode inference for distribution-based type disambiguation.
//!
//! Column-mode takes a vector of string values (a column sample), runs
//! single-value inference on each, aggregates the predictions, and applies
//! disambiguation rules to determine the most likely type for the entire column.
//!
//! This is critical for resolving ambiguous types like:
//! - `mdy_slash` vs `dmy_slash` dates (MM/DD vs DD/MM)
//! - `short_dmy` vs `short_mdy` dates
//! - `latitude` vs `longitude` coordinates
//! - Numeric types (port, increment, postal_code, integer_number)

use crate::entity::EntityClassifier;
use crate::features::{extract_features, FEATURE_DIM};
use crate::inference::{ClassificationResult, InferenceError, ValueClassifier};
use crate::model2vec_shared::Model2VecResources;
use crate::multi_branch::MultiBranchClassifier;
use crate::rhh;
use crate::semantic::SemanticHintClassifier;
use crate::sibling_context::SiblingContextAttention;
use finetype_core::{Designation, Taxonomy};
use rayon::prelude::*;
use std::collections::HashMap;

/// No-op classifier used as placeholder when multi-branch is active.
/// Never actually called — the multi-branch code path bypasses ValueClassifier.
struct NoopClassifier;

impl ValueClassifier for NoopClassifier {
    fn classify(&self, _text: &str) -> Result<ClassificationResult, InferenceError> {
        Err(InferenceError::InvalidPath(
            "NoopClassifier should never be called — multi-branch bypasses ValueClassifier".into(),
        ))
    }

    fn classify_batch(
        &self,
        _texts: &[String],
    ) -> Result<Vec<ClassificationResult>, InferenceError> {
        Err(InferenceError::InvalidPath(
            "NoopClassifier should never be called — multi-branch bypasses ValueClassifier".into(),
        ))
    }
}

/// Aggregated column-level features: mean, variance, min, and max of per-value
/// feature vectors (expanded with column-level statistics).
///
/// Used by disambiguation rules for column-level decisions. Variance is the
/// critical new signal: zero length-variance distinguishes structured codes,
/// and dot-segment variance distinguishes structured codes from free-form text.
/// Aggregated column-level features: per-feature mean, variance, min, max
/// across all sampled values. Used by disambiguation rules and the spike
/// disambiguator model (AC-2).
#[derive(Debug, Clone)]
pub struct ColumnFeatures {
    /// Element-wise mean across all values.
    pub mean: [f32; FEATURE_DIM],
    /// Element-wise variance across all values.
    pub variance: [f32; FEATURE_DIM],
    /// Element-wise minimum across all values.
    pub min: [f32; FEATURE_DIM],
    /// Element-wise maximum across all values.
    pub max: [f32; FEATURE_DIM],
}

impl ColumnFeatures {
    /// Create a zero-initialized `ColumnFeatures`.
    fn empty() -> Self {
        Self {
            mean: [0.0f32; FEATURE_DIM],
            variance: [0.0f32; FEATURE_DIM],
            min: [0.0f32; FEATURE_DIM],
            max: [0.0f32; FEATURE_DIM],
        }
    }
}

/// Compute aggregated column-level features (mean, variance, min, max) from
/// per-value feature vectors using a two-pass algorithm.
///
/// Pass 1: accumulate sum, track min/max → compute mean.
/// Pass 2: accumulate squared deviations → compute variance.
pub fn aggregate_features(per_value: &[[f32; FEATURE_DIM]]) -> ColumnFeatures {
    if per_value.is_empty() {
        return ColumnFeatures::empty();
    }

    let n = per_value.len() as f32;

    // Initialize min/max from first element
    let mut mean = [0.0f32; FEATURE_DIM];
    let mut min_vals = per_value[0];
    let mut max_vals = per_value[0];

    // Pass 1: sum + min/max
    for features in per_value {
        for i in 0..FEATURE_DIM {
            mean[i] += features[i];
            if features[i] < min_vals[i] {
                min_vals[i] = features[i];
            }
            if features[i] > max_vals[i] {
                max_vals[i] = features[i];
            }
        }
    }
    for m in &mut mean {
        *m /= n;
    }

    // Pass 2: variance (sum of squared deviations / n)
    let mut variance = [0.0f32; FEATURE_DIM];
    for features in per_value {
        for i in 0..FEATURE_DIM {
            let diff = features[i] - mean[i];
            variance[i] += diff * diff;
        }
    }
    for v in &mut variance {
        *v /= n;
    }

    ColumnFeatures {
        mean,
        variance,
        min: min_vals,
        max: max_vals,
    }
}

/// Feature index constants for disambiguation rules (expanded with column-level statistics).
/// Must match the indices in `features::FEATURE_NAMES`.
mod feature_idx {
    pub const IS_FLOAT: usize = 2;
    pub const HAS_LEADING_ZERO: usize = 3;
    #[allow(dead_code)]
    pub const IS_HEX_STRING: usize = 7;
    pub const LENGTH: usize = 10;
    pub const DIGIT_RATIO: usize = 17;
    pub const SEGMENT_COUNT_DOT: usize = 24;
    pub const SEGMENT_COUNT_SLASH: usize = 26;
    #[allow(dead_code)]
    pub const HAS_COLON: usize = 32;
    #[allow(dead_code)]
    pub const HAS_DASH: usize = 33;
    pub const ALPHA_RATIO: usize = 18;
    pub const HAS_NEGATIVE_PREFIX: usize = 34;
    #[allow(dead_code)]
    pub const HAS_PERCENT: usize = 35;
}

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
    "representation.boolean.binary",   // 0/1
    "representation.boolean.initials", // T/F, Y/N
    "representation.boolean.terms",    // true/false, yes/no, on/off
    "technology.development.boolean",  // legacy
    "representation.logical.boolean",  // legacy interim label
    "technology.data.boolean",         // legacy
];

/// Geography location types used by both geography protection (header hints)
/// and entity demotion geography rescue. Extracted to module level so both
/// code paths share the same definition.
const LOCATION_TYPES: &[&str] = &[
    "geography.location.city",
    "geography.location.country",
    "geography.location.region",
    "geography.location.state",
    "geography.location.continent",
];

/// Person-name hint types that trigger geography protection.
/// Model2Vec can return any of these for "name" headers — not just full_name.
/// All should trigger the geography guard to prevent overriding correct
/// location predictions.
const PERSON_NAME_HINTS: &[&str] = &[
    "identity.person.full_name",
    "identity.person.last_name",
    "identity.person.first_name",
];

/// Max fraction of a column's values that may contain internal whitespace for it
/// to be treated as login-handle-shaped (username) rather than person names.
/// Corpus-grounded (spec 2026-06-17-full-name-username-veto ac-00): `author`
/// columns sit at 0.005, real-name columns (player_name/person/name/artist) at
/// 0.94-1.0 — a threshold at 0.15 separates them with a wide margin.
const USERNAME_WHITESPACE_MAX_FRACTION: f32 = 0.15;

/// Min fraction of DISTINCT values for a handle-shaped column to be treated as a
/// username. Real username columns are high-cardinality (a distinct handle per
/// row); low-cardinality repeating vocabularies (exchange codes, drug names) that
/// happen to be single-token are not. Calibrated against the spec ac-03 isolated
/// corpus pass (genuine `author` ~1.0; false-positive vocabularies well below 0.9).
const USERNAME_MIN_DISTINCT_FRACTION: f32 = 0.9;

/// Value-based username recovery (decision 0048; spec
/// `2026-06-17-full-name-username-veto`). `identity.person.full_name` is the
/// model's single largest over-emission (249,568 corpus columns), dominated by
/// login-handle columns the model reads as person names (~165k `author`). Real
/// person names are multi-token ("First Last"); login handles are single tokens
/// over a restricted charset. This detects handle-shaped columns by value alone
/// (NOT header — decision 0042): low internal-whitespace fraction AND most values
/// matching a handle charset (ASCII alphanumeric + `. _ -`, at least one letter).
///
/// Safe-by-construction for full_name precision: a *correct* full_name column is
/// always multi-token (has whitespace), so the whitespace guard can never demote
/// a true positive — it only reclassifies columns full_name could never have been
/// right about.
fn is_username_handle_shaped(values: &[String]) -> bool {
    let non_empty = non_empty_trimmed(values);
    // Need enough evidence to judge a column's shape.
    if non_empty.len() < 4 {
        return false;
    }
    let mut with_space = 0usize;
    let mut handle_charset = 0usize;
    for v in &non_empty {
        if v.chars().any(char::is_whitespace) {
            with_space += 1;
        } else if v.chars().any(|c| c.is_ascii_alphabetic())
            && v.chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
        {
            handle_charset += 1;
        }
    }
    let n = non_empty.len() as f32;
    let space_frac = with_space as f32 / n;
    let handle_frac = handle_charset as f32 / n;

    // High-cardinality guard. A real username column has a distinct handle per
    // row; a low-cardinality repeating vocabulary that merely LOOKS handle-shaped
    // (exchange codes NMS/NYQ, drug names, role/platform names) is NOT a username
    // — it is enum/word territory. The isolated corpus pass (spec ac-03) showed
    // 55% of un-guarded relocations were exactly this class, so the guard is
    // load-bearing. Threshold calibrated against that pass: genuine `author`
    // columns sit at distinct-fraction ~1.0; the false-positive vocabularies fall
    // well below 0.9.
    let distinct = non_empty
        .iter()
        .collect::<std::collections::HashSet<_>>()
        .len();
    let distinct_frac = distinct as f32 / n;

    space_frac <= USERNAME_WHITESPACE_MAX_FRACTION
        && handle_frac >= 0.80
        && distinct_frac >= USERNAME_MIN_DISTINCT_FRACTION
}

/// Hardcoded list of labels known to be generic catch-all predictions.
/// Used as a fallback when taxonomy is not available for designation lookup.
const HARDCODED_GENERIC_LABELS: &[&str] = &[
    "representation.text.word",
    "representation.text.plain_text",
    "representation.numeric.integer_number",
    "representation.numeric.decimal_number",
    "representation.identifier.increment",
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
///    reliably distinguish these types from character patterns.
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

    // Signal 1b: Numeric postal code heuristic is pattern-based, not model-driven.
    // It should yield to explicit header hints (e.g., a column with 3-digit
    // values detected as postal_code). Preserves postal code detection for headerless columns.
    if disambiguation_rule
        .as_ref()
        .is_some_and(|r| r == "numeric_postal_code_detection")
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

    // Signal 4: Designation-aware expansion.
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
/// each locale's validation pattern from `validation_by_locale`.
///
/// Full-column auto-increment test for `increment_substance_veto`
/// (spec 2026-06-16-column-statistics-lever). A genuine `increment` is a
/// CONTIGUOUS, near-unique run of non-negative integers — `distinct ≈ max−min+1`
/// (fills its range, almost no gaps) with almost no duplicates. This is a
/// FULL-COLUMN fact: the value-sharpen sequential check runs on the 100-value
/// stepped sample, where a true `1..N` run becomes `1, k, 2k, …` (uniform but not
/// contiguous) and any evenly-spaced numeric column looks sequential, so it
/// over-emits increment (gold precision 0.056). Returns `Some(true)` for a genuine
/// increment, `Some(false)` for a plain integer column wearing the label, and
/// `None` when there are too few integers to judge (caller leaves the label alone).
/// Does the sample pass `leaf`'s taxonomy validator at a high rate? Used by
/// `datetime_format_refinement` to assert only leaves the downstream validation veto
/// (threshold 0.5) will accept — the 0.9 bar here sits comfortably above it, so a
/// confirmed assertion is never re-vetoed, and an under-bar column keeps the model's label.
/// Conclusive, value-self-sufficient cede leaves the reshaped model no longer emits,
/// re-asserted from values by `ceded_leaf_recovery` (spec 2026-06-27-model-label-space-reshape).
/// = ac-0 CEDE_CLEAN − (leaves owned by datetime_format_refinement / structured_string_refinement
/// / isbn_header_recovery) − (permissive-validator leaves color_hex/rgb the ac-0 challenge demoted).
/// Every validator here is structurally exclusive; ordering is irrelevant (exactly-one-match gate).
const CEDED_RECOVERY_LEAVES: &[&str] = &[
    "container.key_value.query_string",
    "container.object.html",
    "container.object.json",
    "container.object.json_array",
    "container.object.xml",
    "datetime.component.day_of_week",
    "datetime.component.month_name",
    "datetime.component.periodicity",
    "finance.banking.iban",
    "finance.crypto.bitcoin_address",
    "finance.crypto.ethereum_address",
    "finance.currency.currency_code",
    "finance.currency.currency_symbol",
    "finance.rate.basis_points",
    "finance.securities.figi",
    "geography.address.street_suffix",
    "geography.coordinate.dms",
    "geography.coordinate.mgrs",
    "geography.coordinate.plus_code",
    "geography.format.wkt",
    "geography.location.continent",
    "geography.transportation.iso6346",
    "identity.commerce.isrc",
    "identity.government.pan_india",
    "identity.government.vin",
    "identity.medical.icd10",
    "identity.person.email",
    "identity.person.email_display",
    "identity.person.gender",
    "identity.person.phone_e164",
    "representation.boolean.terms",
    "representation.file.mime_type",
    "representation.format.color_hsl",
    "representation.identifier.uuid",
    "representation.numeric.si_number",
    "representation.scientific.inchi",
    "representation.text.emoji",
    "technology.cloud.aws_arn",
    "technology.cloud.s3_uri",
    "technology.code.doi",
    "technology.cryptographic.jwt",
    "technology.identifier.ulid",
    "technology.internet.cidr",
    "technology.internet.data_uri",
    "technology.internet.http_method",
    "technology.internet.ip_v4",
    "technology.internet.ip_v4_with_port",
    "technology.internet.ip_v6",
    "technology.internet.mac_address",
    "technology.internet.urn",
    "technology.internet.user_agent",
];

/// ≥90% of the sample's non-empty values must pass `leaf`'s taxonomy validator.
///
/// Two things this function deliberately does NOT do, both for speed and neither
/// changing its answer:
///
/// 1. It resolves the validator ONCE instead of calling `validate_value_for_label`
///    per value. That call built a full `ValidationResult` — formatting a
///    human-readable message for every failing value — and this function reads
///    only the boolean. Those discarded messages were ~15% of `profile` wall
///    clock. Resolution never looks at the value, so a resolution failure is a
///    property of the label, not of one value: the old loop would then have
///    taken the `Err` arm for every value, left `checked == 0`, and returned
///    false — which is what the early `return false` below does.
/// 2. It stops as soon as the 0.9 bar is arithmetically out of reach. With `r`
///    values left, the best attainable ratio is `(passed + r) / (checked + r)`
///    (every remaining value non-empty and passing). That expression is monotone
///    non-decreasing in `r` while `passed <= checked`, and f64 division is
///    correctly rounded, hence monotone — so when the ceiling is already below
///    the bar, no suffix of the sample can reach it and the final comparison
///    would have been false.
fn label_validates_sample(tax: &Taxonomy, leaf: &str, sample: &[String]) -> bool {
    let Some(validator) = finetype_core::validator::resolve_label_validator(leaf, tax) else {
        return false;
    };
    let mut checked = 0usize;
    let mut passed = 0usize;
    let mut remaining = sample.len();
    for v in sample {
        remaining -= 1;
        let t = v.trim();
        if t.is_empty() {
            continue;
        }
        checked += 1;
        if validator.is_valid(t) {
            passed += 1;
        }
        if ((passed + remaining) as f64) / ((checked + remaining) as f64) < 0.9 {
            return false;
        }
    }
    checked > 0 && (passed as f64) / (checked as f64) >= 0.9
}

/// True when the column sample OUTRIGHT CONTRADICTS `leaf` — the leaf has a
/// universal validator that fewer than half the non-empty values pass. The
/// reliable-NO direction of the validation asymmetry (memory
/// `validation-gate-asymmetry`): used only to DECLINE a header-hint override the
/// values disprove, never to assert a type. `country_code_corroboration`'s
/// `get_validator` gate; returns false when the leaf has no universal validator
/// (locale-specific or validator-less types — no evidence either way, so the
/// hint is left to stand).
///
/// The bar is NEAR-TOTAL contradiction (≤10% pass), not a simple majority. Some
/// universal validators are imperfect — they reject genuine members (the `url`
/// pattern rejects some valid URLs, the `country_code` enum some real codes, the
/// `utc` offset pattern a real `+05:00`). At a 0.5 bar those imprecise validators
/// wrongly block a CORRECT hint (gold A/B: href→url, Country Code→country_code,
/// a real utc_offset all regressed). The genuine over-emits this guard targets —
/// `year` on decimals, `offset.utc` on millisecond integers, `url` on msg-ids —
/// pass their hinted validator at ~0%, so the tight bar keeps every recovery
/// while dropping the imperfect-validator regressions.
fn sample_contradicts_label(tax: &Taxonomy, leaf: &str, sample: &[String]) -> bool {
    let Some(validator) = tax.get_validator(leaf) else {
        return false;
    };
    let non_empty = non_empty_trimmed(sample);
    if non_empty.len() < 3 {
        return false;
    }
    let pass = non_empty.iter().filter(|v| validator.is_valid(v)).count();
    (pass as f64) / (non_empty.len() as f64) <= 0.10
}

fn values_form_increment(values: &[String]) -> Option<bool> {
    let n_nonempty = values.iter().filter(|v| !v.trim().is_empty()).count();
    let ints: Vec<i64> = values
        .iter()
        .filter_map(|v| v.trim().parse::<i64>().ok())
        .collect();
    if ints.len() < 5 {
        return None;
    }
    let all_int = ints.len() as f32 >= 0.95 * n_nonempty.max(1) as f32;
    let distinct = ints
        .iter()
        .copied()
        .collect::<std::collections::HashSet<_>>()
        .len();
    let min = *ints.iter().min().unwrap();
    let max = *ints.iter().max().unwrap();
    let span = (max - min + 1) as f64; // cardinality of the integer range
    let contiguous = span > 0.0 && distinct as f64 / span >= 0.80; // fills its range
    let near_unique = distinct as f64 / ints.len() as f64 >= 0.90; // ~no duplicates
    Some(all_int && min >= 0 && distinct >= 5 && contiguous && near_unique)
}

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
    /// Whether the prediction is generic (low-confidence, attractor-demoted,
    /// boolean, or designation-based). Generic predictions yield to header hints
    /// and indicate the type may not be precise enough for downstream transforms.
    pub is_generic: bool,
    /// Aggregated column-level features (mean, variance, min, max of 36-dim
    /// per-value features). Available for disambiguation analysis and the
    /// learned disambiguator spike.
    pub column_features: Option<ColumnFeatures>,
}

/// Column-mode classifier that wraps a single-value classifier.
///
/// Accepts any `ValueClassifier` implementation (e.g. `CharClassifier`)
/// via `Box<dyn ValueClassifier>`.
pub struct ColumnClassifier {
    classifier: Box<dyn ValueClassifier>,
    config: ColumnConfig,
    /// Optional semantic column name classifier (Model2Vec embeddings).
    /// When present, used as the primary header hint source before falling
    /// back to the hardcoded `header_hint()` dictionary.
    /// Bypassed when Sense is active (Sense already sees the header).
    semantic_hint: Option<SemanticHintClassifier>,
    /// Optional taxonomy for validation-based attractor demotion.
    /// When present, enables Signal 1 (validation failure) in the
    /// attractor demotion disambiguation rule (Rule 14).
    taxonomy: Option<Taxonomy>,
    /// Optional entity classifier for full_name demotion.
    /// When present and majority vote is full_name, runs a binary demotion
    /// check: if the column is confidently non-person, demotes to entity_name.
    /// Bypassed when Sense is active (Sense entity subtype replaces this).
    entity_classifier: Option<EntityClassifier>,
    /// Shared Model2Vec resources for sibling-context header encoding (multi-branch).
    model2vec: Option<Model2VecResources>,
    /// Optional sibling-context attention module.
    /// When present and Sense is active, enriches column header embeddings with
    /// cross-column context before Sense classification. Requires `model2vec` to
    /// encode headers. Without a trained model artifact, this is None and the
    /// pipeline is unchanged.
    sibling_context: Option<SiblingContextAttention>,
    /// Optional multi-branch column classifier (Sherlock-style).
    /// When present, `classify_column_with_header` uses the multi-branch forward
    /// pass directly (column-level features → MLP → label), bypassing both
    /// ValueClassifier and Sense→Sharpen. The multi-branch model is fundamentally
    /// column-level, not value-level.
    multi_branch: Option<MultiBranchClassifier>,
    /// Diagnostic flag: skip all Sharpen post-processing (feature_sharpen,
    /// value_sharpen, apply_header_sharpen). Returns raw multi-branch model
    /// output. Used for ablation studies — not exposed in public API.
    skip_sharpen: bool,
}

impl ColumnClassifier {
    /// Create a new column classifier wrapping any ValueClassifier.
    pub fn new(classifier: Box<dyn ValueClassifier>, config: ColumnConfig) -> Self {
        Self {
            classifier,
            config,
            semantic_hint: None,
            taxonomy: None,
            skip_sharpen: false,
            entity_classifier: None,
            model2vec: None,
            sibling_context: None,
            multi_branch: None,
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
            entity_classifier: None,
            model2vec: None,
            sibling_context: None,
            multi_branch: None,
            skip_sharpen: false,
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

    /// Attach an entity classifier for full_name demotion.
    ///
    /// When present and the majority vote is `identity.person.full_name`,
    /// the entity classifier runs a binary demotion check. If the column
    /// is confidently non-person (place, organization, creative work),
    /// the prediction is demoted to `representation.text.entity_name`.
    pub fn set_entity_classifier(&mut self, entity: EntityClassifier) {
        self.entity_classifier = Some(entity);
    }

    /// Attach a sibling-context attention module.
    ///
    /// When present (multi-branch), `classify_columns_with_context` encodes all
    /// column headers with Model2Vec, runs sibling-context attention to enrich
    /// them, then feeds the enriched header into the multi-branch header branch.
    pub fn set_sibling_context(&mut self, sibling: SiblingContextAttention) {
        self.sibling_context = Some(sibling);
    }

    /// Check whether sibling-context attention is available.
    pub fn has_sibling_context(&self) -> bool {
        self.sibling_context.is_some()
    }

    /// Attach Model2Vec resources without Sense.
    ///
    /// Used when multi-branch is active: sibling-context attention needs Model2Vec
    /// to encode headers, but the Sense classifier is not required.
    pub fn set_model2vec(&mut self, model2vec: Model2VecResources) {
        self.model2vec = Some(model2vec);
    }

    /// Create a column classifier using a multi-branch model.
    ///
    /// Multi-branch is fundamentally column-level (Vec<String> → features → label),
    /// not value-level. It does NOT use the `ValueClassifier` trait. When
    /// `classify_column_with_header` is called and multi-branch is present,
    /// it takes a dedicated code path that extracts 3-branch features and runs
    /// the MLP forward pass directly.
    ///
    /// A dummy `ValueClassifier` is still required for the struct field, but it
    /// is never called when multi-branch is active.
    pub fn with_multi_branch(multi_branch: MultiBranchClassifier, config: ColumnConfig) -> Self {
        // Use a no-op ValueClassifier as placeholder — never called when
        // multi-branch is active.
        let dummy = Box::new(NoopClassifier);
        Self {
            classifier: dummy,
            config,
            semantic_hint: None,
            taxonomy: None,
            entity_classifier: None,
            model2vec: None,
            sibling_context: None,
            multi_branch: Some(multi_branch),
            skip_sharpen: false,
        }
    }

    /// Set the skip_sharpen diagnostic flag. When true, the multi-branch pipeline
    /// returns raw model output without any Sharpen post-processing.
    /// Used for ablation studies only — not part of the public API contract.
    pub fn set_skip_sharpen(&mut self, skip: bool) {
        self.skip_sharpen = skip;
    }

    /// Check whether the multi-branch classifier is active.
    pub fn has_multi_branch(&self) -> bool {
        self.multi_branch.is_some()
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
        // Multi-branch: delegate to column-level classifier (no header context)
        if let Some(ref mb) = self.multi_branch {
            return self.classify_multi_branch(mb, values, "");
        }

        if values.is_empty() {
            return Ok(ColumnResult {
                label: "unknown".to_string(),
                confidence: 0.0,
                vote_distribution: vec![],
                disambiguation_applied: false,
                disambiguation_rule: None,
                samples_used: 0,
                detected_locale: None,
                is_generic: false,
                column_features: None,
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

        // Step 3: Aggregate votes — collapse 4-level locale labels to 3-level.
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
        votes.sort_by_key(|b| std::cmp::Reverse(b.1));

        // Validation-based candidate elimination: reject candidates
        // whose JSON Schema validation contract is violated by >50% of sample
        // values. Uses pre-compiled validators from taxonomy cache.
        if let Some(taxonomy) = self.taxonomy.as_ref() {
            let non_empty_count = sample.iter().filter(|v| !v.trim().is_empty()).count();
            if non_empty_count >= 3 {
                let validated: Vec<(String, usize)> = votes
                    .iter()
                    .filter(|(label, _)| {
                        taxonomy
                            .get_validator(label)
                            .map(|validator| {
                                let pass_count = sample
                                    .iter()
                                    .filter(|v| validator.is_valid(v.trim()))
                                    .count();
                                pass_count as f32 / non_empty_count as f32 >= 0.5
                            })
                            .unwrap_or(true) // no validator → keep
                    })
                    .cloned()
                    .collect();
                // Safety: if ALL eliminated, keep original votes
                if !validated.is_empty() {
                    votes = validated;
                }
            }
        }

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
                is_generic: false,
                column_features: None,
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
                is_generic: false,
                column_features: None,
            }
        };

        // Step 4b: Username recovery veto (decision 0048; spec
        // 2026-06-17-full-name-username-veto). A handle is a username, not a person
        // name nor an entity name, so run before entity demotion. No header hints
        // apply on this (header-less) path, so placement here is final.
        self.apply_username_veto(&mut result, &sample);

        // Step 5: Entity demotion gate.
        // When the majority label is full_name and the entity classifier is loaded,
        // check whether the column actually contains person names. If confidently
        // non-person, demote to entity_name. Fires after disambiguation but before
        // header hints (which may later override this).
        if result.label == "identity.person.full_name" {
            if let Some(ref entity_model) = self.entity_classifier {
                match entity_model.should_demote(&sample) {
                    Ok(true) => {
                        result.label = "representation.text.entity_name".to_string();
                        result.disambiguation_applied = true;
                        result.disambiguation_rule = Some("entity_demotion:nonperson".to_string());
                        result.detected_locale = None;
                    }
                    Ok(false) => {} // Keep full_name — classifier thinks it's person
                    Err(e) => {
                        // Log but don't fail — entity classifier is optional
                        tracing::warn!("Entity classifier error, skipping demotion: {}", e);
                    }
                }
            }
        }

        // Step 5b: Feature-based disambiguation (extended to legacy path).
        // Compute aggregated column features and apply feature disambiguation rules
        // (F1–F6). Previously only ran in Sense→Sharpen path; now also in legacy
        // to fix decimal/numeric_code (F5) when Sense is absent.
        let per_value_features: Vec<[f32; FEATURE_DIM]> =
            sample.iter().map(|v| extract_features(v)).collect();
        let column_features = aggregate_features(&per_value_features);
        result.column_features = Some(column_features.clone());
        feature_disambiguate(&mut result, &column_features, &votes, n_samples);

        // Step 5c: Column schema-validation gate (Precision Principle).
        // A per-value classifier (CharCNN) has no column context, so it can
        // confidently emit a type whose schema the column's values violate —
        // e.g. decimal magnitude → latitude, where longitude/depth values exceed
        // the ±90 latitude range. Unlike attractor demotion (Step 4), this gate
        // applies to ANY predicted label: if most values fail the predicted
        // type's JSON Schema, the prediction is not supported by the data.
        if let Some(taxonomy) = self.taxonomy.as_ref() {
            schema_validation_gate(&mut result, &sample, &votes, taxonomy);
        }

        // Step 6: Post-hoc locale detection via validation patterns.
        // When taxonomy is available, run sample values against validation_by_locale
        // patterns to detect the most likely locale. This takes priority over any
        // model-derived locale from vote aggregation, because validation patterns
        // are precise structural rules (see Precision Principle, decision-002).
        if let Some(taxonomy) = self.taxonomy.as_ref() {
            if let Some(locale) = detect_locale_from_validation(&sample, &result.label, taxonomy) {
                result.detected_locale = Some(locale);
            }
        }

        self.finalize_is_generic(&mut result);
        Ok(result)
    }

    /// Value-based username recovery (decision 0048; spec
    /// `2026-06-17-full-name-username-veto`). Reclassifies a `full_name` result to
    /// `username` when the column's values are login-handle-shaped
    /// ([`is_username_handle_shaped`]). `full_name` is the model's single largest
    /// over-emission (249,568 corpus columns, ~165k of them `author` handle
    /// columns); the handle shape is decided by value alone, NOT by the header
    /// (decision 0042 deprecates header hints). Must run AFTER header hints so a
    /// deprecated `author -> full_name` cross-domain hint cannot resurrect a handle
    /// column. Safe-by-construction for full_name precision: a correct full_name is
    /// always multi-token (has whitespace), so the whitespace guard never demotes a
    /// true positive. RHH-disableable via `full_name_username_veto`.
    fn apply_username_veto(&self, result: &mut ColumnResult, sample: &[String]) {
        if result.label == "identity.person.full_name"
            && !rhh::is_disabled("full_name_username_veto")
            && is_username_handle_shaped(sample)
        {
            result.label = "identity.person.username".to_string();
            result.disambiguation_applied = true;
            result.disambiguation_rule = Some("full_name_username_veto".to_string());
            result.detected_locale = None;
        }
    }

    /// Set `is_generic` on a result based on its final label and disambiguation state.
    /// Called before returning from classification methods to ensure the field is always
    /// computed on the final (post-hint, post-demotion) label.
    fn finalize_is_generic(&self, result: &mut ColumnResult) {
        // `representation.discrete.categorical` was retired as an emitted label
        // (choice 0102): enum-ness is the orthogonal `x-finetype-enum` domain
        // property (detected value-side at profile time), not a competing leaf.
        // The Sharpen producers now emit the honest text residual `word` directly
        // (choice 0107 stage 3), and the multi-branch model does not predict
        // categorical — so there is no sentinel left to reframe here.
        result.is_generic = is_generic_prediction(
            &result.label,
            &result.disambiguation_rule,
            self.taxonomy.as_ref(),
        );
    }

    /// Get a reference to the underlying classifier.
    pub fn classifier(&self) -> &dyn ValueClassifier {
        &*self.classifier
    }

    /// Classify multiple columns with sibling context.
    ///
    /// When sibling-context attention is available:
    /// 1. Encode all column headers with Model2Vec → `[N_cols, 128]`
    /// 2. Run sibling-context attention → `[N_cols, 128]` (enriched)
    /// 3. For each column: run the active pipeline with enriched headers
    ///
    /// Supports both Sense→Sharpen and multi-branch pipelines:
    /// - Sense→Sharpen: enriched header feeds into Sense classification
    /// - Multi-branch: enriched header feeds into the 4th header branch MLP
    ///
    /// When sibling context is NOT available (no trained model), falls back to
    /// per-column `classify_column_with_header` — producing identical results.
    pub fn classify_columns_with_context(
        &self,
        columns: &[(Vec<String>, String)], // (values, header) per column
    ) -> Result<Vec<ColumnResult>, InferenceError> {
        // Fast path: no sibling context or no Model2Vec → per-column classification
        if !self.has_sibling_context() || self.model2vec.is_none() {
            return columns
                .par_iter()
                .map(|(values, header)| self.classify_column_with_header(values, header))
                .collect();
        }

        // Only the multi-branch path consumes sibling-context enrichment; any
        // other model classifies per-column.
        let Some(ref mb) = self.multi_branch else {
            return columns
                .par_iter()
                .map(|(values, header)| self.classify_column_with_header(values, header))
                .collect();
        };

        let sibling_ctx = self.sibling_context.as_ref().unwrap();
        let m2v = self.model2vec.as_ref().unwrap();

        // Step 1: Encode all column headers with Model2Vec
        let headers: Vec<&str> = columns.iter().map(|(_, h)| h.as_str()).collect();
        let header_embs = m2v.encode_batch(&headers)?; // [N_cols, D]

        // Step 2: Run sibling-context attention → enriched [N_cols, D]
        let enriched = sibling_ctx.forward(&header_embs)?;

        // Step 3: multi-branch with the enriched header (header branch MLP + Sharpen).
        //
        // Columns are independent here — steps 1 and 2 already did all the
        // cross-column work, and this step only reads `&self` and row `i` of the
        // enriched matrix — so the loop is data-parallel.
        //
        // `par_iter().enumerate()` is an INDEXED parallel iterator and
        // `collect::<Result<Vec<_>, _>>()` from one restores input order, so the
        // returned sequence matches `columns` position for position regardless of
        // how the work was scheduled. Column order is contract, not cosmetics: the
        // caller zips these results back against its own column list by position.
        // Nothing here may collect into an unordered container.
        let results: Vec<ColumnResult> = columns
            .par_iter()
            .enumerate()
            .map(|(i, (values, header))| {
                let enriched_header = enriched.get(i)?; // [D]
                self.classify_multi_branch_with_enriched(mb, values, header, &enriched_header)
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(results)
    }

    /// Classify a column of values with an optional header name hint.
    ///
    /// When multi-branch is active (the shipped default), uses the multi-branch
    /// column-level pipeline. Otherwise falls back to the legacy ValueClassifier
    /// pipeline: CharCNN → vote → disambiguation → entity demotion → header hints.
    pub fn classify_column_with_header(
        &self,
        values: &[String],
        header: &str,
    ) -> Result<ColumnResult, InferenceError> {
        // Multi-branch pipeline: when multi-branch is active, use it directly.
        // Multi-branch is column-level (features → MLP → label), bypassing
        // the ValueClassifier path entirely.
        if let Some(ref mb) = self.multi_branch {
            return self.classify_multi_branch(mb, values, header);
        }

        let mut result = self.classify_column(values)?;

        // Entity demotion guard: when the entity classifier has
        // made a deliberate data-driven demotion (full_name → entity_name),
        // skip header hint override. The entity classifier analyzed the actual
        // column values; header hints are a weaker signal that would undo the
        // demotion (entity_name is broad_words → generic → header overrides).
        if result
            .disambiguation_rule
            .as_ref()
            .is_some_and(|r| r.starts_with("entity_demotion"))
        {
            self.finalize_is_generic(&mut result);
            return Ok(result);
        }

        // Epoch seconds detection (legacy pipeline).
        // Runs before header hints to prevent "created_date" → iso_8601 mismatch.
        if !result.label.starts_with("datetime.") {
            if let Some(epoch_label) = detect_epoch_seconds(values) {
                result.label = epoch_label;
                result.confidence = 0.85;
                result.disambiguation_applied = true;
                result.disambiguation_rule = Some("epoch_seconds_range_detection".to_string());
                self.finalize_is_generic(&mut result);
                return Ok(result);
            }
        }

        // Apply header hint: hardcoded first (curated knowledge), then Model2Vec.
        // Hardcoded hints have been curated through multiple iterations
        // and cover known cases precisely. Model2Vec adds value for unknown headers
        // but can override correct hardcoded mappings.
        let hardcoded_hint_legacy = header_hint(header).map(|h| h.to_string());
        let hinted_type: Option<String> = hardcoded_hint_legacy.clone().or_else(|| {
            self.semantic_hint
                .as_ref()
                .and_then(|sh| sh.classify_header(header))
                .map(|r| r.label.clone())
        });
        let hint_is_hardcoded_legacy = hardcoded_hint_legacy.is_some();

        if let Some(hinted_type) = hinted_type.as_deref() {
            // If the model already predicts the hinted type, just boost confidence
            if result.label == hinted_type {
                result.confidence = (result.confidence + 0.1).min(1.0);
                self.finalize_is_generic(&mut result);
                return Ok(result);
            }

            // Measurement disambiguation: height and weight values are
            // numerically indistinguishable (all small integers in overlapping
            // ranges). When the header provides a specific measurement hint,
            // trust it over the model prediction.
            const MEASUREMENT_TYPES: &[&str] =
                &["identity.person.height", "identity.person.weight"];
            const COORDINATE_TYPES: &[&str] = &[
                "geography.coordinate.latitude",
                "geography.coordinate.longitude",
            ];
            if MEASUREMENT_TYPES.contains(&hinted_type)
                && MEASUREMENT_TYPES.contains(&result.label.as_str())
            {
                result.label = hinted_type.to_string();
                result.confidence = 0.9;
                result.disambiguation_applied = true;
                result.disambiguation_rule =
                    Some(format!("header_hint_measurement:{}", header.to_lowercase()));
                self.finalize_is_generic(&mut result);
                return Ok(result);
            }
            // Scientific measurement override: header contains
            // measurement keywords (pressure, temperature, etc.) and model
            // predicts latitude/longitude. Header is authoritative.
            if hinted_type == "representation.numeric.decimal_number"
                && COORDINATE_TYPES.contains(&result.label.as_str())
            {
                result.label = hinted_type.to_string();
                result.confidence = 0.8;
                result.disambiguation_applied = true;
                result.disambiguation_rule = Some(format!(
                    "header_hint_sci_measurement:{}",
                    header.to_lowercase()
                ));
                self.finalize_is_generic(&mut result);
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
            // Designation-aware gating: when taxonomy is available,
            // types with broad designations (broad_words, broad_characters,
            // broad_numbers, broad_object) are treated as generic because the
            // CharCNN cannot reliably distinguish them from character patterns
            // alone. Falls back to a hardcoded list when taxonomy is unavailable.
            let is_generic = is_generic_prediction(
                &result.label,
                &result.disambiguation_rule,
                self.taxonomy.as_ref(),
            );

            // Geography protection: when the hint is a person-name type
            // (full_name, last_name, first_name), check if the model sees
            // geography.location signal. Many geographic datasets use "name"
            // as a header for city, country, or region columns. Model2Vec may
            // return any person-name type for "name" headers.
            if PERSON_NAME_HINTS.contains(&hinted_type) {
                // Case 1: Model already predicts a location type — keep it
                // rather than overriding to a person-name type.
                if LOCATION_TYPES.contains(&result.label.as_str()) {
                    result.confidence = result.confidence.max(0.5);
                    result.disambiguation_applied = true;
                    result.disambiguation_rule = Some(format!(
                        "header_hint_location_keep:{}",
                        header.to_lowercase()
                    ));
                    self.finalize_is_generic(&mut result);
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
                            self.finalize_is_generic(&mut result);
                            return Ok(result);
                        }
                    }
                }
            }

            // Same-domain geographic override: when both the hint
            // and prediction are location types, trust the header name.
            if LOCATION_TYPES.contains(&hinted_type)
                && LOCATION_TYPES.contains(&result.label.as_str())
                && result.label != hinted_type
                && result.confidence <= 0.90
            {
                result.label = hinted_type.to_string();
                result.confidence = result.confidence.max(0.6);
                result.disambiguation_applied = true;
                result.disambiguation_rule = Some(format!(
                    "header_hint_geo_override:{}",
                    header.to_lowercase()
                ));
                self.finalize_is_generic(&mut result);
                return Ok(result);
            }

            // Same-category hardcoded hint override:
            // When the hardcoded header hint and prediction share the same
            // domain.category (e.g., both datetime.timestamp.*), the header
            // is authoritative for format disambiguation. Only applies when
            // confidence is moderate (≤0.80) — high confidence predictions
            // within the same category are more likely correct than the hint.
            if hint_is_hardcoded_legacy && result.label != hinted_type && result.confidence <= 0.80
            {
                let hint_category = hinted_type.rsplitn(2, '.').last().unwrap_or("");
                let pred_category = result.label.rsplitn(2, '.').last().unwrap_or("");
                if !hint_category.is_empty()
                    && hint_category == pred_category
                    && hint_category.contains('.')
                {
                    result.label = hinted_type.to_string();
                    result.confidence = result.confidence.max(0.7);
                    result.disambiguation_applied = true;
                    result.disambiguation_rule = Some(format!(
                        "header_hint_same_category:{}",
                        header.to_lowercase()
                    ));
                    self.finalize_is_generic(&mut result);
                    return Ok(result);
                }
            }

            // Cross-domain hardcoded hint override:
            // When a hardcoded hint and prediction differ in both domain AND
            // base type name, the header is authoritative. Catches structural
            // confusion (postal_code vs CPT, epoch vs NPI) where patterns are
            // identical and only the header disambiguates.
            // Does NOT fire when base type names match (uuid vs uuid).
            if hint_is_hardcoded_legacy && result.label != hinted_type {
                let hint_domain = hinted_type.split('.').next().unwrap_or("");
                let pred_domain = result.label.split('.').next().unwrap_or("");
                let hint_base = hinted_type.rsplit('.').next().unwrap_or("");
                let pred_base = result.label.rsplit('.').next().unwrap_or("");
                if !hint_domain.is_empty()
                    && !pred_domain.is_empty()
                    && hint_domain != pred_domain
                    && hint_base != pred_base
                {
                    result.label = hinted_type.to_string();
                    result.confidence = result.confidence.max(0.5);
                    result.disambiguation_applied = true;
                    result.disambiguation_rule = Some(format!(
                        "header_hint_cross_domain:{}",
                        header.to_lowercase()
                    ));
                    self.finalize_is_generic(&mut result);
                    return Ok(result);
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
            } else if hint_is_hardcoded_legacy && result.confidence < 0.5 && !hint_in_votes {
                // Hardcoded header hint with low confidence — apply even when the
                // hinted type was eliminated from votes by validation. Hardcoded hints
                // are curated human knowledge (e.g., header "npi" → identity.medical.npi).
                // When the model is uncertain (< 0.5) and the header is an exact match,
                // the header is more authoritative than validation pass rates on
                // potentially synthetic data.
                result.label = hinted_type.to_string();
                result.confidence = 0.6;
                result.disambiguation_applied = true;
                result.disambiguation_rule =
                    Some(format!("header_hint_hardcoded:{}", header.to_lowercase()));
            } else if is_generic && !hint_in_votes {
                // Financial model2vec guard: block model2vec financial hints when
                // the CharCNN saw no financial signal in values. See Sense pipeline
                // for detailed rationale.
                let is_financial_hint = hinted_type.starts_with("finance.");
                if is_financial_hint && !hint_is_hardcoded_legacy {
                    // Model2vec financial hint with no value evidence — skip.
                } else {
                    result.label = hinted_type.to_string();
                    result.confidence = 0.5;
                    result.disambiguation_applied = true;
                    result.disambiguation_rule =
                        Some(format!("header_hint_generic:{}", header.to_lowercase()));
                }
            } else if result.confidence < 0.3 && !hint_in_votes {
                // Very low confidence and hint type not even in votes —
                // still apply hint but with low confidence
                result.label = hinted_type.to_string();
                result.confidence = 0.4;
                result.disambiguation_applied = true;
                result.disambiguation_rule =
                    Some(format!("header_hint_fallback:{}", header.to_lowercase()));
            }

            // If header hint changed the label, re-detect locale for the new type
            //. The detected_locale from classify_column was
            // for the original label — re-run detection against the new label's
            // validation_by_locale patterns.
            if result.label != original_label {
                result.detected_locale = self
                    .taxonomy
                    .as_ref()
                    .and_then(|t| detect_locale_from_validation(values, &result.label, t));
            }
        }

        self.finalize_is_generic(&mut result);
        Ok(result)
    }

    /// Get a reference to the configuration.
    pub fn config(&self) -> &ColumnConfig {
        &self.config
    }

    /// The shared Sharpen firing tail (Steps 3-6), extracted so the three classify
    /// entry points run byte-identical Sharpen. The order is load-bearing:
    /// feature_sharpen → value_sharpen → datetime/structured refinement →
    /// sharpen_and_guard (header hints) → username veto → post-hoc locale → finalize.
    ///
    /// `classify_multi_branch`, `classify_multi_branch_with_enriched`, and
    /// `compose_from_sense` all call this after seeding `result` from the Sense label.
    /// `compose_from_sense` is what the corpus-honest gate scores (cmd_run.rs), so any
    /// drift between it and the native paths = the gate scoring a different pipeline
    /// than production ships. Keeping the tail here makes that drift impossible; the
    /// `native_and_compose_run_identical_sharpen` test pins the parity.
    fn run_sharpen(
        &self,
        result: &mut ColumnResult,
        header: &str,
        sample: &[String],
        values: &[String],
        column_features: &ColumnFeatures,
    ) {
        // Steps 3-5: Sharpen post-processing (skipped when skip_sharpen is set)
        if !self.skip_sharpen {
            // Step 3: Feature-based Sharpen rules (F1-F6)
            feature_sharpen(result, column_features);

            // Step 4: Value-based Sharpen rules (R1-R19)
            if let Some((resolved_label, rule_name)) = value_sharpen(
                sample,
                &result.label,
                result.confidence,
                self.taxonomy.as_ref(),
            ) {
                result.label = resolved_label;
                result.disambiguation_applied = true;
                result.disambiguation_rule = Some(rule_name);
            }

            // Step 5: Header hints (Model2Vec semantic matching)
            // Deterministic datetime sub-format read (value-based, over-emission-safe;
            // runs before header hints so a delimited timestamp is read from values).
            self.datetime_format_refinement(result, sample);
            self.structured_string_refinement(result, sample);
            self.sharpen_and_guard(result, header, sample, values);

            // Step 5b: Username recovery veto — value-based, runs AFTER header hints
            // so a deprecated author->full_name cross-domain hint can't resurrect a
            // handle column (decision 0048; spec 2026-06-17-full-name-username-veto).
            self.apply_username_veto(result, sample);
        }

        // Step 6: Post-hoc locale detection
        if let Some(taxonomy) = self.taxonomy.as_ref() {
            if let Some(locale) = detect_locale_from_validation(sample, &result.label, taxonomy) {
                result.detected_locale = Some(locale);
            }
        }

        self.finalize_is_generic(result);
    }

    fn classify_multi_branch(
        &self,
        mb: &MultiBranchClassifier,
        values: &[String],
        header: &str,
    ) -> Result<ColumnResult, InferenceError> {
        if values.is_empty() {
            return Ok(ColumnResult {
                label: "unknown".to_string(),
                confidence: 0.0,
                vote_distribution: vec![],
                disambiguation_applied: false,
                disambiguation_rule: None,
                samples_used: 0,
                detected_locale: None,
                is_generic: false,
                column_features: None,
            });
        }

        // Sample values (same strategy as classify_column)
        let sample = if values.len() <= self.config.sample_size {
            values.to_vec()
        } else {
            let step = values.len() as f64 / self.config.sample_size as f64;
            (0..self.config.sample_size)
                .map(|i| values[(i as f64 * step) as usize].clone())
                .collect()
        };

        let samples_used = sample.len();

        // Step 1: Classify via multi-branch (feature extraction + forward pass)
        let (label, confidence) = mb.classify_column(&sample, header, self.taxonomy.as_ref())?;

        // Step 2: Compute deterministic ColumnFeatures (36-dim, no neural inference)
        let per_value_features: Vec<[f32; FEATURE_DIM]> =
            sample.iter().map(|v| extract_features(v)).collect();
        let column_features = aggregate_features(&per_value_features);

        let mut result = ColumnResult {
            label: label.clone(),
            confidence,
            vote_distribution: vec![(label, confidence)],
            disambiguation_applied: false,
            disambiguation_rule: Some("multi-branch".to_string()),
            samples_used,
            detected_locale: None,
            is_generic: false,
            column_features: Some(column_features.clone()),
        };

        // Honest-gate composition (FINETYPE_INJECT_LABEL): override the Sense label
        // with an externally-supplied one (another model's prediction), then run the
        // REAL Sharpen stack on it — lets us compose any model's predictions without
        // that model being in the binary. Diagnostic only; empty/unset = no-op.
        if !self.skip_sharpen {
            if let Ok(inj) = std::env::var("FINETYPE_INJECT_LABEL") {
                if !inj.is_empty() {
                    result.label = inj;
                    result.confidence = 1.0;
                }
            }
        }
        // Steps 3-6: the shared Sharpen firing tail (see `run_sharpen`).
        self.run_sharpen(&mut result, header, &sample, values, &column_features);
        Ok(result)
    }

    /// Compose a cached Sense label through the REAL Sharpen stack, skipping the
    /// expensive value-encode + multi-branch forward.
    ///
    /// For corpus-honest gating of Sharpen-rule changes (spec
    /// 2026-06-27-composed-accuracy-roadmap): the value encode (potion-8M over ~100
    /// values/column) produces an identical Sense across baseline and candidate, so
    /// re-running it on the 33k-file stratified sample for every rule iteration is pure
    /// waste. Given the model's cached Sense label, this runs only the deterministic
    /// `column_features` (no neural inference, see `classify_multi_branch` Step 2) and
    /// the Sharpen stack (header hints re-encode only the cheap header) — minutes instead
    /// of ~an hour. NOT for production inference; a diagnostic re-sharpen path.
    ///
    /// SHARES the Sharpen firing tail with `classify_multi_branch` via `run_sharpen`
    /// (Steps 3-6), so native parity is structural, not maintained by hand. Validated at
    /// 99.4% native parity (100% on rule-affected labels); the `compose_from_sense_runs_sharpen`
    /// and `native_and_compose_run_identical_sharpen` tests pin the behaviour.
    pub fn compose_from_sense(
        &self,
        header: &str,
        values: &[String],
        sense_label: &str,
        sense_conf: f32,
    ) -> Result<ColumnResult, InferenceError> {
        if values.is_empty() {
            return Ok(ColumnResult {
                label: "unknown".to_string(),
                confidence: 0.0,
                vote_distribution: vec![],
                disambiguation_applied: false,
                disambiguation_rule: None,
                samples_used: 0,
                detected_locale: None,
                is_generic: false,
                column_features: None,
            });
        }
        // Sample (same strategy as classify_multi_branch).
        let sample = if values.len() <= self.config.sample_size {
            values.to_vec()
        } else {
            let step = values.len() as f64 / self.config.sample_size as f64;
            (0..self.config.sample_size)
                .map(|i| values[(i as f64 * step) as usize].clone())
                .collect()
        };
        let samples_used = sample.len();
        // Step 2: deterministic ColumnFeatures (no neural inference).
        let per_value_features: Vec<[f32; FEATURE_DIM]> =
            sample.iter().map(|v| extract_features(v)).collect();
        let column_features = aggregate_features(&per_value_features);
        let mut result = ColumnResult {
            label: sense_label.to_string(),
            confidence: sense_conf,
            vote_distribution: vec![(sense_label.to_string(), sense_conf)],
            disambiguation_applied: false,
            disambiguation_rule: Some("compose-from-sense".to_string()),
            samples_used,
            detected_locale: None,
            is_generic: false,
            column_features: Some(column_features.clone()),
        };
        // Steps 3-6: the shared Sharpen firing tail — identical to the native paths.
        // The corpus-honest gate scores THIS path, so it MUST match production; the
        // shared `run_sharpen` makes any drift impossible.
        self.run_sharpen(&mut result, header, &sample, values, &column_features);
        Ok(result)
    }

    /// Multi-branch classification with enriched header + Sharpen (AC-1).
    ///
    /// Like `classify_multi_branch()` but uses a sibling-context-enriched
    /// header tensor instead of the raw header string. Called from
    /// `classify_columns_with_context()` when both multi-branch and sibling
    /// context are active.
    ///
    /// Note: header hints require the raw header string, which is passed
    /// separately from the enriched tensor.
    fn classify_multi_branch_with_enriched(
        &self,
        mb: &MultiBranchClassifier,
        values: &[String],
        header: &str,
        enriched_header: &candle_core::Tensor,
    ) -> Result<ColumnResult, InferenceError> {
        if values.is_empty() {
            return Ok(ColumnResult {
                label: "unknown".to_string(),
                confidence: 0.0,
                vote_distribution: vec![],
                disambiguation_applied: false,
                disambiguation_rule: None,
                samples_used: 0,
                detected_locale: None,
                is_generic: false,
                column_features: None,
            });
        }

        // Sample values (same strategy as classify_column)
        let sample = if values.len() <= self.config.sample_size {
            values.to_vec()
        } else {
            let step = values.len() as f64 / self.config.sample_size as f64;
            (0..self.config.sample_size)
                .map(|i| values[(i as f64 * step) as usize].clone())
                .collect()
        };

        let samples_used = sample.len();

        // Step 1: Classify via multi-branch with enriched header
        let (label, confidence) = mb.classify_column_with_enriched_header(
            &sample,
            enriched_header,
            self.taxonomy.as_ref(),
        )?;

        // Step 2: Compute deterministic ColumnFeatures
        let per_value_features: Vec<[f32; FEATURE_DIM]> =
            sample.iter().map(|v| extract_features(v)).collect();
        let column_features = aggregate_features(&per_value_features);

        let mut result = ColumnResult {
            label: label.clone(),
            confidence,
            vote_distribution: vec![(label, confidence)],
            disambiguation_applied: false,
            disambiguation_rule: Some("multi-branch-sibling".to_string()),
            samples_used,
            detected_locale: None,
            is_generic: false,
            column_features: Some(column_features.clone()),
        };

        // Honest-gate composition (FINETYPE_INJECT_LABEL): override the Sense label
        // with an externally-supplied one (another model's prediction), then run the
        // REAL Sharpen stack on it — lets us compose any model's predictions without
        // that model being in the binary. Diagnostic only; empty/unset = no-op.
        if !self.skip_sharpen {
            if let Ok(inj) = std::env::var("FINETYPE_INJECT_LABEL") {
                if !inj.is_empty() {
                    result.label = inj;
                    result.confidence = 1.0;
                }
            }
        }
        // Steps 3-6: the shared Sharpen firing tail (see `run_sharpen`).
        self.run_sharpen(&mut result, header, &sample, values, &column_features);
        Ok(result)
    }

    /// Header hint Sharpen for the multi-branch pipeline (AC-5).
    ///
    /// Applies header-based semantic matching to refine multi-branch predictions.
    /// Simplified from the Sense→Sharpen header logic:
    /// - No unmasked_votes (no CharCNN votes exist)
    /// - No Sense entity demotion guard (no Sense)
    /// - Financial model2vec guard preserved (prevents false financial overrides)
    /// - Geography protection preserved (prevents person-name hints overriding locations)
    /// - Same-domain/cross-domain overrides preserved
    fn apply_header_sharpen(&self, result: &mut ColumnResult, header: &str, sample: &[String]) {
        let label_before = result.label.clone();

        // 0094 — header hint family `header_hint_coord_veto` (default ON, like the
        // other header hints; RHH-disableable via RHH_DISABLE_HINTS). Header-
        // corroboration demotion for the value-identical coordinate boundary: a
        // latitude/longitude prediction whose header does NOT corroborate a
        // coordinate, on a generic numeric column, is demoted to decimal_number.
        // Demotion-only — the header can veto a false coordinate, never promote
        // one, so a mislabelled header cannot create a coordinate. Fires in the
        // no-hint gap the sci_measurement override below misses.
        if !rhh::is_disabled("header_hint_coord_veto")
            && matches!(
                result.label.as_str(),
                "geography.coordinate.latitude" | "geography.coordinate.longitude"
            )
            && !header_corroborates_coordinate(header)
            && values_look_like_generic_decimals(sample)
        {
            result.label = "representation.numeric.decimal_number".to_string();
            result.confidence = result.confidence.min(0.6);
            result.disambiguation_applied = true;
            result.disambiguation_rule =
                Some(format!("coord_header_veto:{}", header.to_lowercase()));
            return;
        }

        // 0094 — header hint family `header_hint_postal_veto` (spec
        // 2026-06-10-postal-header-veto): the same header-corroboration demotion
        // for the value-identical postal boundary. A postal_code prediction on a
        // bare-integer column (volumes, counts, sequence numbers — 4–5 digit
        // integers are indistinguishable from Nordic/Australian postcodes by
        // value alone; gold-measured precision 0.133) is demoted to
        // integer_number unless the header carries a postal token. Leading-zero
        // values are postal evidence (01219-style zips) and block the veto.
        // Demotion-only — a header can never promote a postal code.
        if !rhh::is_disabled("header_hint_postal_veto")
            && result.label == "geography.address.postal_code"
            && !header_corroborates_postal(header)
            && values_look_like_generic_integers(sample)
        {
            result.label = "representation.numeric.integer_number".to_string();
            result.confidence = result.confidence.min(0.6);
            result.disambiguation_applied = true;
            result.disambiguation_rule =
                Some(format!("postal_header_veto:{}", header.to_lowercase()));
            return;
        }

        // 0094 — header hint family `header_hint_state_code_promote` (default ON).
        // PROMOTION for the value-identical state_code boundary. state_code scores
        // P=R=0.000 on gold: a column of 2-letter subdivision codes (TX, FL, NM)
        // with a `State` header is classified as the state NAME type, region, or
        // full_address — state_code is never emitted. Unlike the coordinate
        // promotion (which already exists header-only and so needed a guard), there
        // is NO existing path to state_code, so this adds one. SAFETY: the
        // promotion REQUIRES both a closed-vocabulary value match (>=80% of values
        // in STATE_CODES) AND a state/province header — the header is load-bearing
        // because 2-letter codes overlap ISO country codes (CA = California AND
        // Canada), so value-match alone would steal country_code columns. Both
        // conditions together cannot fire on a country column (country header) or a
        // state-NAME column (full words fail the code vocab).
        if !rhh::is_disabled("header_hint_state_code_promote")
            && result.label != "geography.location.state_code"
            && header_corroborates_state(header)
            && values_look_like_state_codes(sample)
        {
            result.label = "geography.location.state_code".to_string();
            result.confidence = result.confidence.max(0.8);
            result.disambiguation_applied = true;
            result.disambiguation_rule =
                Some(format!("state_code_promote:{}", header.to_lowercase()));
            return;
        }

        // RHH instrumentation hooks (compile out when feature `rhh-instrumentation`
        // is off — every `disable_*` becomes a constant `false`, the optimiser
        // eliminates the conjunctions, and there is zero runtime overhead).
        // See .orbit/specs/2026-04-24-remove-header-hints/spec.yaml ac-02.
        let disable_measurement = rhh::is_disabled("header_hint_measurement");
        let disable_sci_measurement = rhh::is_disabled("header_hint_sci_measurement");
        let disable_person_override = rhh::is_disabled("header_hint_person_override");
        let disable_geo_override = rhh::is_disabled("header_hint_geo_override");
        let disable_same_category = rhh::is_disabled("header_hint_same_category");
        let disable_cross_domain = rhh::is_disabled("header_hint_cross_domain");
        let disable_catchall = rhh::is_disabled("header_hint");
        let disable_generic = rhh::is_disabled("header_hint_generic");
        let disable_hardcoded = rhh::is_disabled("header_hint_hardcoded");
        let disable_fallback = rhh::is_disabled("header_hint_fallback");

        // Get header hint: hardcoded first, then Model2Vec semantic
        let hardcoded_hint = header_hint(header).map(|h| h.to_string());
        let hinted_type: Option<String> = hardcoded_hint.clone().or_else(|| {
            self.semantic_hint
                .as_ref()
                .and_then(|sh| sh.classify_header(header))
                .map(|r| r.label.clone())
        });
        let hint_is_hardcoded = hardcoded_hint.is_some();

        let hinted_type = match hinted_type.as_deref() {
            Some(h) => h,
            None => return,
        };

        // Already predicts the hinted type — boost confidence
        if result.label == hinted_type {
            result.confidence = (result.confidence + 0.1).min(1.0);
            return;
        }

        // Value-corroboration (spec 2026-06-25-sharpen-stage-audit ac-1). The
        // deprecated regex header_hint table (decision 0042) substring-matches
        // compound headers — "priceEpsCurrentYear"/"CitesPerYear"/"(year)"→year,
        // "epoch_number"→unix_seconds — and overrides a now-correct value-based
        // Sense label with a type the column's VALUES disprove; the override then
        // stands wrong or is hard-vetoed to `unknown`. Decline any override whose
        // hinted type the values outright contradict (its universal validator
        // passes ≤10%): the model's value-based label was right. Reliable-NO
        // direction only — demotion-safe, never asserts. The `result.label ==
        // hinted_type` agreement case already returned above, so this only ever
        // blocks a genuine override. RHH-disableable.
        //
        // SCOPED to labels whose universal validator reliably separates a real
        // member from an over-emit (gold A/B, attention model): year + epoch.
        // The broad guard was net +6 but NOT gold-clean — the `url`, `offset.utc`,
        // `country_code` and `iana` universal validators reject their own genuine
        // members at ~0% (href→url, a real utc_offset→utc, "Country Code"→
        // country_code all regressed), so pass-rate cannot tell a true
        // contradiction from a real member there. Those clusters need per-label
        // value tests / validator fixes (see output/sharpen-audit/ac1_measurement.md).
        // Same scoping discipline as R32 (`schema_fail_demotion`): a closed label
        // set cannot regress unrelated columns.
        const CORROBORATION_SCOPE: &[&str] = &[
            "datetime.component.year",
            "datetime.epoch.unix_seconds",
            "datetime.epoch.unix_milliseconds",
        ];
        if !rhh::is_disabled("header_hint_value_corroboration") {
            let contradicted = if hinted_type == "technology.internet.url" {
                // url uses a value-SHAPE test, not its validator: gold treats
                // root-relative paths (`/partner/x.asp?id=…`) as url, which the
                // "complete web address" validator rejects — so the validator would
                // wrongly block a real url column. Decline the `link`/`url` header
                // promotion only when the values are neither url-shaped (scheme://,
                // protocol-relative //, or root-relative /) NOR bare numbers. Bare
                // numbers are left to `url_bare_number_veto` downstream (which
                // correctly demotes them to integer/decimal), so the guard does not
                // strip the path that rescues a 0/1 `perm_unlink` column.
                let (is_bare, _) = values_look_like_bare_numbers(sample);
                !is_bare && values_are_clearly_non_url(sample)
            } else if CORROBORATION_SCOPE.contains(&hinted_type) {
                self.taxonomy
                    .as_ref()
                    .is_some_and(|tax| sample_contradicts_label(tax, hinted_type, sample))
            } else {
                false
            };
            if contradicted {
                return;
            }
        }

        // Value-grounded gender sibling is authoritative over the header hint.
        // value_sharpen's gender_detection routes a bare M/F column to
        // gender_code; the VALUES (not the "gender"/"sex" header) distinguish the
        // single-char code from the word type. A same-family header hint agrees
        // it is gender — so it must not override the sibling the values chose.
        // Without this, the same-category / hardcoded-authority overrides revert
        // gender_code → gender and the validation veto then rejects M/F (0% pass)
        // — the two-deterministic-steps-fighting trap the deterministic-layer
        // audit flagged. Per spec 2026-06-12-false-veto-trio-resolution.
        if result.disambiguation_rule.as_deref() == Some("gender_detection")
            && matches!(
                hinted_type,
                "identity.person.gender" | "identity.person.gender_code"
            )
        {
            return;
        }

        // Measurement disambiguation: height/weight
        const MEASUREMENT_TYPES: &[&str] = &["identity.person.height", "identity.person.weight"];
        const COORDINATE_TYPES: &[&str] = &[
            "geography.coordinate.latitude",
            "geography.coordinate.longitude",
        ];
        if !disable_measurement
            && MEASUREMENT_TYPES.contains(&hinted_type)
            && MEASUREMENT_TYPES.contains(&result.label.as_str())
        {
            result.label = hinted_type.to_string();
            result.confidence = 0.9;
            result.disambiguation_applied = true;
            result.disambiguation_rule =
                Some(format!("header_hint_measurement:{}", header.to_lowercase()));
            return;
        }

        // Scientific measurement override (coordinates → decimal when header says measurement)
        if !disable_sci_measurement
            && hinted_type == "representation.numeric.decimal_number"
            && COORDINATE_TYPES.contains(&result.label.as_str())
        {
            result.label = hinted_type.to_string();
            result.confidence = 0.8;
            result.disambiguation_applied = true;
            result.disambiguation_rule = Some(format!(
                "header_hint_sci_measurement:{}",
                header.to_lowercase()
            ));
            return;
        }

        // Check hint in vote distribution (single-entry for multi-branch)
        let hint_in_votes = result
            .vote_distribution
            .iter()
            .any(|(label, _)| label == hinted_type);

        let is_generic = is_generic_prediction(
            &result.label,
            &result.disambiguation_rule,
            self.taxonomy.as_ref(),
        );

        // Geography protection: person-name hints don't override location types
        if PERSON_NAME_HINTS.contains(&hinted_type)
            && LOCATION_TYPES.contains(&result.label.as_str())
        {
            if !disable_person_override && hint_is_hardcoded {
                // Hardcoded person-name hint overrides location
                result.label = hinted_type.to_string();
                result.confidence = result.confidence.max(0.6);
                result.disambiguation_applied = true;
                result.disambiguation_rule = Some(format!(
                    "header_hint_person_override:{}",
                    header.to_lowercase()
                ));
            }
            // Model2Vec person-name hints do NOT override locations
            // (no unmasked votes to check in multi-branch)
            return;
        }

        // Same-domain geographic override
        if !disable_geo_override
            && LOCATION_TYPES.contains(&hinted_type)
            && LOCATION_TYPES.contains(&result.label.as_str())
            && result.label != hinted_type
            && (hint_is_hardcoded || result.confidence <= 0.90)
        {
            result.label = hinted_type.to_string();
            result.confidence = result.confidence.max(0.6);
            result.disambiguation_applied = true;
            result.disambiguation_rule = Some(format!(
                "header_hint_geo_override:{}",
                header.to_lowercase()
            ));
            return;
        }

        // Guard: iso_8601 date/timestamp catch-all should not override specific
        // datetime predictions. The catch-all (h.contains("date"/"timestamp") →
        // iso_8601) fires for any header containing those words, but the model's
        // header branch processes these words and produces a more specific datetime
        // prediction (mdy_slash, clf, dmy_hm, etc.). Trust the model's specificity
        // over the generic catch-all. Aligns with decision 0042 direction.
        if hinted_type == "datetime.timestamp.iso_8601"
            && hint_is_hardcoded
            && result.label.starts_with("datetime.")
            && result.label != "datetime.timestamp.iso_8601"
        {
            return;
        }

        // Same-category hardcoded hint override
        // Hardcoded hints are definitive for same-category overrides — no confidence
        // threshold. E.g. "phone" → phone_number overrides ssn@1.00 because both
        // are identity.person.* and the header is unambiguous.
        if !disable_same_category && hint_is_hardcoded && result.label != hinted_type {
            let hint_category = hinted_type.rsplitn(2, '.').last().unwrap_or("");
            let pred_category = result.label.rsplitn(2, '.').last().unwrap_or("");
            if !hint_category.is_empty()
                && hint_category == pred_category
                && hint_category.contains('.')
            {
                result.label = hinted_type.to_string();
                result.confidence = result.confidence.max(0.7);
                result.disambiguation_applied = true;
                result.disambiguation_rule = Some(format!(
                    "header_hint_same_category:{}",
                    header.to_lowercase()
                ));
                // No return — fall through to post-hint guards (e.g., country_code guard).
            }
        }

        // Cross-domain hardcoded hint override
        if !disable_cross_domain && hint_is_hardcoded && result.label != hinted_type {
            let hint_domain = hinted_type.split('.').next().unwrap_or("");
            let pred_domain = result.label.split('.').next().unwrap_or("");
            let hint_base = hinted_type.rsplit('.').next().unwrap_or("");
            let pred_base = result.label.rsplit('.').next().unwrap_or("");
            if !hint_domain.is_empty()
                && !pred_domain.is_empty()
                && hint_domain != pred_domain
                && hint_base != pred_base
            {
                result.label = hinted_type.to_string();
                result.confidence = result.confidence.max(0.5);
                result.disambiguation_applied = true;
                result.disambiguation_rule = Some(format!(
                    "header_hint_cross_domain:{}",
                    header.to_lowercase()
                ));
                return;
            }
        }

        // General hint logic
        if !disable_catchall && (result.confidence < 0.5 || is_generic) && hint_in_votes {
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
        } else if !disable_generic && is_generic && !hint_in_votes {
            // Financial model2vec guard: block model2vec financial hints with no value evidence
            let is_financial_hint = hinted_type.starts_with("finance.");
            if is_financial_hint && !hint_is_hardcoded {
                tracing::debug!(
                    column = %header,
                    blocked_hint = %hinted_type,
                    current_label = %result.label,
                    "Financial model2vec hint blocked (no vote evidence)"
                );
            } else {
                result.label = hinted_type.to_string();
                result.confidence = 0.5;
                result.disambiguation_applied = true;
                result.disambiguation_rule =
                    Some(format!("header_hint_generic:{}", header.to_lowercase()));
            }
        } else if !disable_hardcoded && hint_is_hardcoded && !hint_in_votes {
            // Hardcoded hint authority with domain-dependent threshold
            // Same-domain: 0.95 (was 0.90, originally 0.50) — unblocks phone@0.915
            // overriding ssn, year@0.83 overriding compact_ym, url@0.60 overriding
            // docker_ref. Only predictions >=0.95 resist a hardcoded same-domain hint.
            // Cross-domain: 0.85 (unchanged).
            let h_domain = hinted_type.split('.').next().unwrap_or("");
            let p_domain = result.label.split('.').next().unwrap_or("");
            let threshold = if h_domain != p_domain { 0.85 } else { 0.95 };
            if result.confidence < threshold {
                result.label = hinted_type.to_string();
                result.confidence = 0.5;
                result.disambiguation_applied = true;
                result.disambiguation_rule =
                    Some(format!("header_hint_hardcoded:{}", header.to_lowercase()));
            }
        } else if !disable_fallback && result.confidence < 0.3 && !hint_in_votes {
            result.label = hinted_type.to_string();
            result.confidence = 0.4;
            result.disambiguation_applied = true;
            result.disambiguation_rule =
                Some(format!("header_hint_fallback:{}", header.to_lowercase()));
        }

        // Country/country_code post-hint guard (v14, ac-05).
        // After ALL header hint processing, if the label is "country" but >=95%
        // of values are strict ISO 3166-1 alpha-2 codes (^[A-Z]{2}$), override
        // to country_code. This guard fires LAST — after the same-category
        // hardcoded hint override which would otherwise overwrite
        // a value_sharpen correction. See spec review finding A1/F1.
        if result.label == "geography.location.country" {
            let non_empty = non_empty_trimmed(sample);
            if non_empty.len() >= 3 {
                let alpha2_count = non_empty
                    .iter()
                    .filter(|v| v.len() == 2 && v.chars().all(|c| c.is_ascii_uppercase()))
                    .count();
                let alpha2_rate = alpha2_count as f32 / non_empty.len() as f32;
                if alpha2_rate >= 0.95 {
                    result.label = "geography.location.country_code".to_string();
                    result.confidence = result.confidence.max(0.8);
                    result.disambiguation_applied = true;
                    result.disambiguation_rule = Some(format!(
                        "country_code_post_hint_guard:alpha2_rate={:.2}",
                        alpha2_rate
                    ));
                }
            }
        }

        // Re-detect locale if label changed
        if result.label != label_before {
            if let Some(taxonomy) = self.taxonomy.as_ref() {
                result.detected_locale =
                    detect_locale_from_validation(sample, &result.label, taxonomy);
            }
        }
    }

    /// Sharpen the label with header hints, then run the post-sharpen guards.
    ///
    /// Single entry point for the inference paths: every header-hint application
    /// is followed by the unconditional guard stage. `apply_header_sharpen`'s
    /// hint branches early-`return`, so a guard that must inspect a label the
    /// hint just CREATED is unreachable from inside it (the
    /// `amount_bare_number_veto` trap). Such guards live in
    /// `apply_post_sharpen_guards`, which runs here regardless of which hint
    /// branch fired. No-op on an empty header, matching the previous call-site
    /// `!header.is_empty()` guard.
    fn sharpen_and_guard(
        &self,
        result: &mut ColumnResult,
        header: &str,
        sample: &[String],
        values: &[String],
    ) {
        if header.is_empty() {
            return;
        }
        self.apply_header_sharpen(result, header, sample);
        self.apply_post_sharpen_guards(result, header, sample, values);
    }

    /// `datetime_format_refinement` (default ON). Deterministic datetime sub-format
    /// detection (spec 2026-06-19-deterministic-datetime-parser). A delimited datetime
    /// string (`2020-01-03 14:22:09`, ISO, offset, slash/dot YMD, …) resolves to exactly
    /// ONE taxonomy leaf by its shape and field ranges — there is nothing to learn, yet
    /// the flat-softmax Sense model routinely guesses the wrong sub-leaf (iso_8601 vs
    /// `…_milliseconds` vs `sql_standard`) or the validation veto demotes a real timestamp
    /// to `unknown`. Read the format deterministically (finetype_core::datetime_format)
    /// and assert it. Over-emission-safe by construction: a DELIMITED reading is
    /// unmistakably datetime, so it is asserted unconditionally (it recovers a timestamp
    /// the model mislabelled `alphanumeric_id`/`unknown` and fixes the sub-leaf); a
    /// BARE-INTEGER reading (epoch seconds/millis/…, a 4-digit year) is asserted ONLY when
    /// the model already predicted a `datetime.*` leaf, because a bare 10-digit integer is
    /// genuinely epoch-or-id-or-phone and grabbing it would relocate non-datetime mass.
    /// Value-based (decision 0048); RHH-disableable.
    fn datetime_format_refinement(&self, result: &mut ColumnResult, sample: &[String]) {
        if rhh::is_disabled("datetime_format_refinement") {
            return;
        }
        let detected = match finetype_core::datetime_format::detect_datetime_format(sample) {
            Some(d) => d,
            None => return,
        };
        // Corroboration gate: a bare integer is only datetime when the model agrees.
        if !detected.delimited && !result.label.starts_with("datetime.") {
            return;
        }
        // Already the exact leaf — leave the disambiguation metadata untouched.
        if result.label == detected.leaf {
            return;
        }
        // Veto-consistency gate: assert a leaf ONLY if the column's values actually pass
        // that leaf's OWN taxonomy validator. The detector uses format-family regexes that
        // are intentionally looser than some strict taxonomy patterns (e.g. the taxonomy
        // requires a trailing `Z` on iso_8601_milliseconds, so a zoneless `…:09.123` reads
        // as millis here but fails that leaf's schema). Without this gate we would assert a
        // leaf the downstream validation veto (profile.rs) then HARD-REJECTS into
        // `unknown`/`alphanumeric_id` — strictly worse than the model's datetime guess.
        // Gating on the same validator the veto uses keeps the two a single source of truth.
        match self.taxonomy.as_ref() {
            Some(tax) if label_validates_sample(tax, detected.leaf, sample) => {}
            _ => return,
        }
        result.label = detected.leaf.to_string();
        // A delimited datetime read is deterministic and certain; reflect that in the
        // confidence (feeds the quality band). A corroborated bare-integer read keeps the
        // model's confidence — the model, not the shape, decided it was temporal.
        if detected.delimited {
            result.confidence = result.confidence.max(0.99);
        }
        result.disambiguation_applied = true;
        result.disambiguation_rule = Some("datetime_format_refinement".to_string());
        result.detected_locale = None;
    }

    /// `structured_string_refinement` (default ON). Four value-determinable readers. Three
    /// were mined from the `plain_text` residual (spec 2026-06-19-plain-text-type-discovery):
    /// `technology.filesystem.windows_path`, `technology.internet.message_id`,
    /// `technology.code.qualified_name`; the fourth, `technology.internet.url`, RECOVERS a
    /// confident model `url` prediction that header/sibling context demoted to plain_text
    /// (spec 2026-06-27-composed-accuracy-roadmap). Each has a precise validator (qualified_name:
    /// zero prose false positives across the 447k-column corpus). The shipped 240-dim Sense
    /// model cannot predict them, so — exactly like `datetime_format_refinement` — they are
    /// recovered deterministically in the Sharpen layer, gated so they cannot relocate
    /// non-matching columns. CORROBORATION gate: fire only where the model gave up
    /// (`plain_text`/`word`/`unknown` — the discovery mined these types from exactly there),
    /// never overriding a confident foreign prediction (`hostname`/`url`/`email`).
    /// VETO-CONSISTENCY gate: assert a leaf only if ≥90% of the column's values pass that
    /// leaf's OWN taxonomy validator (`label_validates_sample`, the same validator the
    /// downstream veto uses — a single source of truth). Value-based (decision 0048);
    /// RHH-disableable.
    fn structured_string_refinement(&self, result: &mut ColumnResult, sample: &[String]) {
        if rhh::is_disabled("structured_string_refinement") {
            return;
        }
        let tax = match self.taxonomy.as_ref() {
            Some(t) => t,
            None => return,
        };
        // Per-type corroboration gate (the labels the model produces for columns of this
        // shape when it can't name them). `windows_path` and `message_id` have UNAMBIGUOUS
        // validators (drive-letter/UNC backslash; angle-bracket `<…@…>`) that nothing else
        // passes, so they may fire on the path/locator and email mispredictions too — the
        // veto-consistency gate (`label_validates_sample`, ≥90%) is the real guard. But
        // `qualified_name` (dotted reverse-DNS) STRUCTURALLY overlaps `hostname` / `url`, so
        // it fires on the residual labels ONLY, never overriding a confident locator. The
        // three patterns are mutually exclusive, so at most one validates a given column.
        const RESIDUAL: &[&str] = &[
            "representation.text.plain_text",
            "representation.text.word",
            "unknown",
        ];
        // url override tier: residual + the confident-wrong labels the model reaches for on a
        // real URL value (see the url reader comment below). The url validator gates every one.
        const URL_FIRE_ON: &[&str] = &[
            "representation.text.plain_text",
            "representation.text.word",
            "unknown",
            "technology.internet.ip_v6",
            "container.object.xml",
            "technology.cloud.aws_arn",
            "finance.crypto.bitcoin_address",
            "technology.cryptographic.jwt",
            "representation.text.entity_name",
            "geography.address.full_address",
            "representation.identifier.alphanumeric_id",
            "identity.government.eu_vat",
        ];
        // RHH toggle for the override tier (gate baseline = residual-only, the prior behaviour).
        let url_fire_on: &[&str] = if rhh::is_disabled("url_override_tier") {
            RESIDUAL
        } else {
            URL_FIRE_ON
        };
        let readers: [(&str, &[&str]); 4] = [
            (
                "technology.filesystem.windows_path",
                &[
                    "representation.text.plain_text",
                    "representation.text.word",
                    "unknown",
                    "technology.internet.url",
                    "technology.internet.urn",
                ],
            ),
            (
                "technology.internet.message_id",
                &[
                    "representation.text.plain_text",
                    "representation.text.word",
                    "unknown",
                    "identity.person.email",
                ],
            ),
            // `url` is a RECOVERY, not a residual-mined type: the model predicts url
            // confidently, but header/sibling context (e.g. a `parent_id` header) demotes a
            // column of bare URLs to plain_text. Re-assert url where >=90% of values pass the
            // url validator; the scheme://host mandate self-limits, so prose and bare
            // hostnames (no scheme) never validate. Mutually exclusive with qualified_name
            // (dotted reverse-DNS has no scheme/slash). Spec 2026-06-27-composed-accuracy-roadmap.
            //
            // OVERRIDE tier (reservoir-mining sweep 2026-07-14): the residual-only fire-on
            // could not reach real URL columns the sibling-aware model labels with a CONFIDENT
            // wrong type — ip_v6 / xml (namespace URIs) / aws_arn / bitcoin_address / jwt /
            // entity_name / full_address / alphanumeric_id / eu_vat. A genuine member of any of
            // those lacks a `scheme://dotted-host`, so the url validator (the real guard) rejects
            // it; adding them to fire-on only lets the validator adjudicate. Deliberately NOT
            // "any label": url-subtype leaves (doi/urn) where url would be a demotion are
            // excluded. Verifier: 0 FP across 1,772 candidate recoveries.
            ("technology.internet.url", url_fire_on),
            ("technology.code.qualified_name", RESIDUAL),
        ];
        for (leaf, fire_on) in readers {
            if fire_on.contains(&result.label.as_str()) && label_validates_sample(tax, leaf, sample)
            {
                result.label = leaf.to_string();
                result.confidence = result.confidence.max(0.99);
                result.disambiguation_applied = true;
                result.disambiguation_rule = Some("structured_string_refinement".to_string());
                result.detected_locale = None;
                return;
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// DISAMBIGUATION RULES
// ═══════════════════════════════════════════════════════════════════════════════

mod feature_sharpen;
mod guards;
mod header_sharpen;
mod helpers;
mod substance;
mod value_sharpen;

pub(crate) use feature_sharpen::*;
pub(crate) use header_sharpen::*;
pub(crate) use helpers::*;
pub(crate) use substance::*;
pub(crate) use value_sharpen::*;

#[cfg(test)]
mod tests;
