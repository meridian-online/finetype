use super::*;

/// Disambiguation rule pairs: types that are ambiguous in single-value mode.
pub(crate) const DATE_SLASH_PAIR: (&str, &str) =
    ("datetime.date.mdy_slash", "datetime.date.dmy_slash");
pub(crate) const SHORT_DATE_PAIR: (&str, &str) =
    ("datetime.date.short_mdy", "datetime.date.short_dmy");
pub(crate) const COORDINATE_PAIR: (&str, &str) = (
    "geography.coordinate.latitude",
    "geography.coordinate.longitude",
);
/// Feature-based Sharpen rules for the multi-branch pipeline (AC-2).
///
/// Adapted from `feature_disambiguate()` with vote-dependent guards removed:
/// - F2: fires on slash_segments threshold alone (no docker_ref in votes check)
/// - F3: fires on feature thresholds alone (no hs_code in votes / vote fraction)
/// - F6: uses fixed categorical fallback (no next-vote lookup)
///
/// All other rules (F1, F4, F5) are unchanged — they never depended on votes.
pub(crate) fn feature_sharpen(result: &mut ColumnResult, column_features: &ColumnFeatures) {
    let label_before = result.label.clone();

    // Rule F1: Leading-zero pre-filter — numeric_code vs postal_code.
    // Unchanged from feature_disambiguate — no vote dependency.
    let leading_zero_ratio = column_features.mean[feature_idx::HAS_LEADING_ZERO];
    if leading_zero_ratio >= 0.3 {
        let code_confusion_types = ["geography.address.postal_code", "identity.medical.cpt"];
        if code_confusion_types.contains(&result.label.as_str()) {
            let numeric_code_label = "representation.identifier.numeric_code";
            result.label = numeric_code_label.to_string();
            result.confidence = result.confidence.max(0.7);
            result.disambiguation_applied = true;
            result.disambiguation_rule = Some(format!(
                "feature_leading_zero:{:.0}%",
                leading_zero_ratio * 100.0
            ));
        }
    }

    // Rule F2: docker_ref vs hostname — slash segments signal container refs.
    // ADAPTED: removed docker_in_votes guard. Multi-branch produces single-entry
    // votes, so docker_ref would never appear as runner-up. Fire on feature
    // threshold alone — high slash segments is a strong enough signal.
    let slash_segments = column_features.mean[feature_idx::SEGMENT_COUNT_SLASH];
    if result.label == "technology.internet.hostname" && slash_segments >= 1.5 {
        result.label = "technology.development.docker_ref".to_string();
        result.confidence = result.confidence.max(0.7);
        result.disambiguation_applied = true;
        result.disambiguation_rule = Some(format!("feature_slash_segments:{:.1}", slash_segments));
    }

    // Rule F3 (hs_code vs decimal_number) was removed.
    // F3 created false hs_code predictions from statistical features that R20
    // (value_sharpen HS code validation gate) then cleaned up — net zero with
    // extra complexity. R20 remains as the authoritative HS code check.

    // Rule F4: git_sha collapsed into technology.cryptographic.hash — rule removed.

    // Rule F5: numeric_code without leading zeros → integer_number or decimal_number.
    // Unchanged — no vote dependency.
    let is_float_ratio = column_features.mean[feature_idx::IS_FLOAT];
    if result.label == "representation.identifier.numeric_code" && leading_zero_ratio < 0.01 {
        if is_float_ratio > 0.5 {
            result.label = "representation.numeric.decimal_number".to_string();
            result.confidence = result.confidence.max(0.7);
            result.disambiguation_applied = true;
            result.disambiguation_rule = Some(format!(
                "feature_decimal_over_numeric_code:float={:.2},leading_zero={:.2}",
                is_float_ratio, leading_zero_ratio
            ));
        } else {
            result.label = "representation.numeric.integer_number".to_string();
            result.confidence = result.confidence.max(0.7);
            result.disambiguation_applied = true;
            result.disambiguation_rule =
                Some(format!("feature_no_leading_zero:{:.2}", leading_zero_ratio));
        }
    }

    // Rule F5b: decimal_number → integer_number when no value is fractional
    // (BACKLOG #10). The Sense head emits decimal_number on whole-number columns
    // (e.g. "1","2","3"). IS_FLOAT is the fraction of values carrying a decimal
    // point; a hard-zero IS_FLOAT means NOT ONE value is fractional, so the column
    // is integer_number, not decimal — the exact mirror of F5's float branch. The
    // 0.01 floor (rather than 0.0) absorbs float imprecision in the mean
    // aggregation, so it fires only on a genuinely all-integer column. Values
    // rendered WITH a point ("1.0","2.0") carry IS_FLOAT≈1.0 and stay decimal.
    if result.label == "representation.numeric.decimal_number" && is_float_ratio < 0.01 {
        result.label = "representation.numeric.integer_number".to_string();
        result.confidence = result.confidence.max(0.7);
        result.disambiguation_applied = true;
        result.disambiguation_rule = Some(format!(
            "feature_decimal_to_integer_is_float:{:.2}",
            is_float_ratio
        ));
    }

    // Rule F6: Short alphabetic codes misclassified as file.extension.
    // ADAPTED: uses fixed categorical fallback instead of next-vote lookup.
    // Multi-branch single-entry votes have no second-place candidate.
    let feat_mean_length = column_features.mean[feature_idx::LENGTH];
    let feat_dot_segments = column_features.mean[feature_idx::SEGMENT_COUNT_DOT];
    let feat_alpha_ratio = column_features.mean[feature_idx::ALPHA_RATIO];
    if result.label == "representation.file.extension"
        && feat_mean_length <= 4.0
        && feat_dot_segments < 1.1
        && feat_alpha_ratio >= 0.8
    {
        result.label = "representation.text.word".to_string();
        result.confidence = result.confidence.max(0.6);
        result.disambiguation_applied = true;
        result.disambiguation_rule = Some(format!(
            "feature_short_code_not_extension:len={:.1},dots={:.2},alpha={:.2}",
            feat_mean_length, feat_dot_segments, feat_alpha_ratio
        ));
    }

    // Trace point: Feature rule outcome
    if result.label != label_before {
        tracing::debug!(
            rule = ?result.disambiguation_rule,
            old_label = %label_before,
            new_label = %result.label,
            "Feature sharpen rule applied"
        );
    }
}
/// Feature-based disambiguation: use aggregated column features to resolve
/// known confusion pairs that the CharCNN model cannot distinguish.
///
/// Runs after standard disambiguation rules. Modifies `result` in place when
/// a feature signal is strong enough to override the current prediction.
///
/// expanded to use variance/min/max statistics and float-parseability
/// for hs_code/decimal_number disambiguation.
pub(crate) fn feature_disambiguate(
    result: &mut ColumnResult,
    column_features: &ColumnFeatures,
    votes: &[(String, usize)],
    n_samples: usize,
) {
    let label_before = result.label.clone();

    // Rule F1: Leading-zero pre-filter — numeric_code vs postal_code.
    //
    // When a significant fraction of values have leading zeros (e.g., "00123",
    // "04500") and the winner is postal_code or cpt, override to numeric_code.
    // Postal codes in formats like US ZIP (5 digits) overlap with numeric codes
    // (NAICS, FIPS, ISO country numeric), but leading zeros are a strong signal
    // for code-like data that should be preserved as VARCHAR.
    //
    // Deliberately excludes integer_number — integers can legitimately have
    // occasional leading zeros (e.g., zero-padded sequences) without being codes.
    // Only postal_code and cpt predictions warrant this override since their
    // validation patterns overlap specifically with numeric codes.
    let leading_zero_ratio = column_features.mean[feature_idx::HAS_LEADING_ZERO];
    if leading_zero_ratio >= 0.3 {
        let code_confusion_types = ["geography.address.postal_code", "identity.medical.cpt"];
        if code_confusion_types.contains(&result.label.as_str()) {
            let numeric_code_label = "representation.identifier.numeric_code";
            result.label = numeric_code_label.to_string();
            result.confidence = result.confidence.max(0.7);
            result.disambiguation_applied = true;
            result.disambiguation_rule = Some(format!(
                "feature_leading_zero:{:.0}%",
                leading_zero_ratio * 100.0
            ));
        }
    }

    // Rule F2: docker_ref vs hostname — slash segments signal container refs.
    //
    // Docker refs (e.g., "docker.io/library/nginx:latest") have slash-separated
    // segments (registry/namespace/image). Hostnames (e.g., "api.example.com")
    // use dots but rarely slashes. A high segment_count_slash is a strong signal.
    let slash_segments = column_features.mean[feature_idx::SEGMENT_COUNT_SLASH];
    if result.label == "technology.internet.hostname" && slash_segments >= 1.5 {
        // Multiple slash segments → likely docker refs
        let docker_in_votes = votes
            .iter()
            .any(|(l, _)| l == "technology.development.docker_ref");
        if docker_in_votes {
            result.label = "technology.development.docker_ref".to_string();
            result.confidence = result.confidence.max(0.7);
            result.disambiguation_applied = true;
            result.disambiguation_rule =
                Some(format!("feature_slash_segments:{:.1}", slash_segments));
        }
    }

    // Rule F3: hs_code vs decimal_number — HS codes are pure digits with dots.
    //
    // HS codes (e.g., "8471.30", "6204.62.40") have high digit_ratio and
    // dot-separated segments. decimal_number also has dots but typically lower
    // digit_ratio (mixed with other chars in real columns) and fewer segments.
    //
    // Enhanced with float-parseability signal. HS codes with 3+
    // segments (e.g., "6204.62.40") don't parse as float, while decimal_number
    // values always do. Two trigger paths:
    //   Path A (original): digit_ratio >= 0.75 AND dot_segments >= 2.0
    //   Path B (new):      digit_ratio >= 0.75 AND is_float_fraction < 1.0
    //
    // Enhanced with two guards to reduce false positives:
    //   Guard 1 — Negative prefix: HS codes never have negative values. If any
    //     values start with '-' followed by a digit, this is a financial/numeric
    //     column, not HS codes. Uses has_negative_prefix mean > 0 as the signal.
    //   Guard 2 — Dot-segment variance: HS code columns have consistent dot
    //     structure (all "XX.XX" or all "XXXX.XX.XX"). High dot-segment variance
    //     indicates mixed formats typical of decimal_number columns.
    let digit_ratio = column_features.mean[feature_idx::DIGIT_RATIO];
    let dot_segments = column_features.mean[feature_idx::SEGMENT_COUNT_DOT];
    let is_float_fraction = column_features.mean[feature_idx::IS_FLOAT];
    let has_neg_prefix = column_features.mean[feature_idx::HAS_NEGATIVE_PREFIX];
    let dot_segment_variance = column_features.variance[feature_idx::SEGMENT_COUNT_DOT];
    let f3_path_a = digit_ratio >= 0.75 && dot_segments >= 2.0;
    let f3_path_b = digit_ratio >= 0.75 && is_float_fraction < 1.0 && dot_segments >= 1.5;
    let f3_neg_guard = has_neg_prefix > 0.0; // Any negative values → not HS codes
    let f3_dot_var_guard = dot_segment_variance > 0.5; // Inconsistent dot structure → not HS codes
    if result.label == "representation.numeric.decimal_number"
        && (f3_path_a || f3_path_b)
        && !f3_neg_guard
        && !f3_dot_var_guard
    {
        let hs_in_votes = votes
            .iter()
            .any(|(l, _)| l == "geography.transportation.hs_code");
        if hs_in_votes {
            let hs_votes = votes
                .iter()
                .find(|(l, _)| l == "geography.transportation.hs_code")
                .map(|(_, c)| *c)
                .unwrap_or(0);
            let hs_frac = hs_votes as f32 / n_samples as f32;
            if hs_frac >= 0.10 {
                result.label = "geography.transportation.hs_code".to_string();
                result.confidence = hs_frac.max(0.6);
                result.disambiguation_applied = true;
                result.disambiguation_rule = Some(format!(
                    "feature_hs_code:digit_ratio={:.2},dots={:.1},float={:.2},neg={:.2},dot_var={:.2}",
                    digit_ratio, dot_segments, is_float_fraction, has_neg_prefix, dot_segment_variance
                ));
            }
        }
    }

    // Rule F4: git_sha collapsed into technology.cryptographic.hash — rule removed.

    // Rule F5: numeric_code without leading zeros → integer_number or decimal_number
    // (extended for decimal disambiguation).
    //
    // numeric_code exists to preserve leading zeros (ZIP codes, NAICS, FIPS).
    // Without leading zeros, the values are plain numbers and should be typed
    // as integer_number (BIGINT) or decimal_number (DOUBLE) instead.
    //
    // When values contain decimal points (IS_FLOAT > 0), prefer decimal_number:
    //   - earthquakes gap: "10.0, 100.0, 101.0" → decimal_number (not numeric_code)
    //   - Decimal points signal measurement data, not identifier codes
    //
    // When no decimal points, demote to integer_number as before.
    //
    // Threshold 0.01 (rather than 0.0) accounts for float imprecision in the
    // mean aggregation. Effectively requires zero leading-zero values.
    let is_float_ratio = column_features.mean[feature_idx::IS_FLOAT];
    if result.label == "representation.identifier.numeric_code" && leading_zero_ratio < 0.01 {
        if is_float_ratio > 0.5 {
            // Majority of values have decimal points → decimal_number
            result.label = "representation.numeric.decimal_number".to_string();
            result.confidence = result.confidence.max(0.7);
            result.disambiguation_applied = true;
            result.disambiguation_rule = Some(format!(
                "feature_decimal_over_numeric_code:float={:.2},leading_zero={:.2}",
                is_float_ratio, leading_zero_ratio
            ));
        } else {
            // No decimal points, no leading zeros → integer_number
            result.label = "representation.numeric.integer_number".to_string();
            result.confidence = result.confidence.max(0.7);
            result.disambiguation_applied = true;
            result.disambiguation_rule =
                Some(format!("feature_no_leading_zero:{:.2}", leading_zero_ratio));
        }
    }

    // Rule F6: Short alphabetic codes misclassified as file.extension.
    //
    // Short 2-3 letter alphabetic codes (e.g., earthquake magnitude types "mb",
    // "ms", network codes "us", "ci", "nc") get classified as file.extension by
    // the CharCNN because they resemble file extensions without the dot. Real file
    // extensions in tabular data would contain dots (e.g., ".csv", ".json") or
    // appear in longer paths.
    //
    // Trigger conditions (all must hold):
    //   - Winner is representation.file.extension
    //   - Mean value length <= 4.0 (short codes)
    //   - Mean dot-segment count close to 1.0 (no dots — segment_count_dot counts
    //     parts split by '.', so 1.0 means zero dots)
    //   - High alpha ratio (>= 0.8, predominantly alphabetic)
    //
    // Action: demote to the next vote (typically categorical or ordinal). If no
    // viable second vote exists, fall back to representation.categorical.categorical.
    let feat_mean_length = column_features.mean[feature_idx::LENGTH];
    let feat_dot_segments = column_features.mean[feature_idx::SEGMENT_COUNT_DOT];
    let feat_alpha_ratio = column_features.mean[feature_idx::ALPHA_RATIO];
    if result.label == "representation.file.extension"
        && feat_mean_length <= 4.0
        && feat_dot_segments < 1.1
        && feat_alpha_ratio >= 0.8
    {
        let next_label = votes
            .iter()
            .find(|(l, _)| l != "representation.file.extension")
            .map(|(l, _)| l.clone());
        let fallback = "representation.categorical.categorical".to_string();
        let chosen = next_label.unwrap_or(fallback);
        result.label = chosen;
        result.confidence = result.confidence.max(0.6);
        result.disambiguation_applied = true;
        result.disambiguation_rule = Some(format!(
            "feature_short_code_not_extension:len={:.1},dots={:.2},alpha={:.2}",
            feat_mean_length, feat_dot_segments, feat_alpha_ratio
        ));
    }

    // Trace point 5: Feature rule outcome
    if result.label != label_before {
        tracing::debug!(
            rule = ?result.disambiguation_rule,
            old_label = %label_before,
            new_label = %result.label,
            "Feature rule applied"
        );
    }
}
