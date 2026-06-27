use super::*;

/// Attractor types — types the CharCNN over-confidently assigns to generic data.
/// Numeric attractors catch integers misclassified as postal codes, etc.
pub(crate) const NUMERIC_ATTRACTORS: &[&str] = &["geography.address.postal_code"];
/// Text attractors catch short words/phrases misclassified as identity types.
/// Note: full_name is NOT included — its false positives are rare (2 in eval)
/// and the header hint system handles them. Including full_name causes more
/// regressions (company, venue, publisher columns whose GT maps to "name"→full_name).
/// phone_number is included here (not NUMERIC) because phone strings contain
/// formatting characters (+, parens, hyphens, spaces). Locale validation via
/// validation_by_locale confirms real phone columns; non-phone data is demoted.
pub(crate) const TEXT_ATTRACTORS: &[&str] = &[
    "identity.person.first_name",
    "identity.person.phone_number",
    "identity.person.username",
    "geography.address.street_name",
];
/// Code attractors catch alphanumeric codes misclassified as specific identifiers.
pub(crate) const CODE_ATTRACTORS: &[&str] = &[
    "geography.transportation.icao_code",
    "identity.medical.ndc",
    "finance.securities.cusip",
    "technology.internet.top_level_domain",
];
/// Value-based Sharpen rules for the multi-branch pipeline (AC-3).
///
/// Adapted from `disambiguate()` with label-based triggers instead of
/// vote-based `contains_pair()`:
/// - Rules that checked `contains_pair(top_labels, a, b)` now check if
///   `result.label` matches either type
/// - Rules that checked `top_labels.first()` now check `result.label`
/// - R12 (numeric): triggers when result.label is any numeric-adjacent type
/// - R15 (attractor demotion): uses `result.confidence` directly instead of
///   `top_count / n_samples` (which gives 0.0 with multi-branch single-entry
///   votes where `confidence as usize` truncates to 0)
/// - `results: &[ClassificationResult]` parameter dropped (R12 never used it)
///
/// Returns Some((resolved_label, rule_name)) if a rule was applied.
pub(crate) fn value_sharpen(
    values: &[String],
    result_label: &str,
    result_confidence: f32,
    taxonomy: Option<&Taxonomy>,
) -> Option<(String, String)> {
    // Rule 1: Date slash disambiguation (mdy_slash vs dmy_slash)
    // ADAPTED: fire when result.label is either date type (not both in top-3)
    if result_label == DATE_SLASH_PAIR.0 || result_label == DATE_SLASH_PAIR.1 {
        if let Some(label) = disambiguate_slash_dates(values) {
            return Some((label, "date_slash_disambiguation".to_string()));
        }
    }

    // Rule 2: Short date disambiguation (short_mdy vs short_dmy)
    if result_label == SHORT_DATE_PAIR.0 || result_label == SHORT_DATE_PAIR.1 {
        if let Some(label) = disambiguate_short_dates(values) {
            return Some((label, "short_date_disambiguation".to_string()));
        }
    }

    // Rule 21: Coordinate plausibility gate — demote to decimal_number when values
    // exceed all coordinate ranges. The model over-predicts latitude/longitude on
    // plain numeric columns (earthquake depth, error measurements). Real coordinates
    // must be within [-180, 180]; if >20% of values exceed that, they're not coords.
    // Must fire BEFORE Rule 3 (coordinate disambiguation) to filter non-coordinates.
    if result_label == COORDINATE_PAIR.0 || result_label == COORDINATE_PAIR.1 {
        let non_empty: Vec<&str> = values
            .iter()
            .map(|v| v.trim())
            .filter(|v| !v.is_empty())
            .collect();
        if non_empty.len() >= 3 {
            let mut parseable = 0usize;
            let mut out_of_range = 0usize;
            for v in &non_empty {
                if let Ok(val) = v.parse::<f64>() {
                    parseable += 1;
                    if val.abs() > 180.0 {
                        out_of_range += 1;
                    }
                }
            }
            if parseable >= 3 && (out_of_range as f32 / parseable as f32) > 0.1 {
                return Some((
                    "representation.numeric.decimal_number".to_string(),
                    format!(
                        "coordinate_plausibility_gate:out_of_range={}/{}",
                        out_of_range, parseable
                    ),
                ));
            }
        }
    }

    // Rule 3: Coordinate disambiguation (latitude vs longitude)
    // ADAPTED: fire when result.label is either coordinate type
    if result_label == COORDINATE_PAIR.0 || result_label == COORDINATE_PAIR.1 {
        if let Some(label) = disambiguate_coordinates(values) {
            return Some((label, "coordinate_disambiguation".to_string()));
        }
    }

    // Rule 4: IPv4 address detection (dotted-quad pattern)
    if let Some(label) = disambiguate_ipv4(values) {
        return Some((label, "ipv4_detection".to_string()));
    }

    // Rule 5: Day-of-week name detection
    if let Some(label) = disambiguate_day_of_week(values) {
        return Some((label, "day_of_week_name_detection".to_string()));
    }

    // Rule 6: Month name detection
    if let Some(label) = disambiguate_month_name(values) {
        return Some((label, "month_name_detection".to_string()));
    }

    // Rule 7: Boolean sub-type normalization
    // ADAPTED: check result.label for boolean types
    let top_labels_single = [result_label];
    if let Some((label, rule)) = disambiguate_boolean_subtype(values, &top_labels_single) {
        return Some((label, rule));
    }

    // Rule 8: Gender detection
    if let Some(label) = disambiguate_gender(values) {
        return Some((label, "gender_detection".to_string()));
    }

    // Rule 9: Boolean override
    if let Some((label, rule)) = disambiguate_boolean_override(values, &top_labels_single) {
        return Some((label, rule));
    }

    // Rule 12: Numeric type disambiguation
    // ADAPTED: trigger when result.label is any numeric-adjacent type
    let numeric_types = [
        "representation.identifier.increment",
        "representation.numeric.integer_number",
        "representation.numeric.decimal_number",
        "geography.address.postal_code",
        "datetime.component.year",
        "representation.identifier.numeric_code",
    ];
    if numeric_types.contains(&result_label) {
        // Pass empty results — R12 never uses them (let _ = results)
        if let Some((label, rule)) = disambiguate_numeric(values, &[], &top_labels_single) {
            return Some((label, rule));
        }
    }

    // Rule 13: SI number override
    if result_label == "representation.numeric.si_number" {
        if let Some((label, rule)) = disambiguate_si_number(values) {
            return Some((label, rule));
        }
    }

    // Rule 19: Percentage without '%' sign → decimal_number
    if result_label == "representation.numeric.percentage" {
        let has_pct_sign = values.iter().any(|v| v.contains('%'));
        if !has_pct_sign {
            return Some((
                "representation.numeric.decimal_number".to_string(),
                "percentage_no_sign".to_string(),
            ));
        }
    }

    // Rule 20: HS code validation gate — demote to decimal_number when values
    // don't match HS code format. The model over-predicts hs_code on plain decimal
    // columns (pe_ratio, sepal_length, humidity_pct, earthquake measurements).
    // HS codes are structured digit groups: 4 digits + optional dot-separated 2-digit
    // groups (e.g., "8471.30", "8471.30.00"). Plain decimals like "3.14" or "0.887" fail.
    if result_label == "geography.transportation.hs_code" {
        let non_empty: Vec<&str> = values
            .iter()
            .map(|v| v.trim())
            .filter(|v| !v.is_empty())
            .collect();
        if non_empty.len() >= 3 {
            let match_count = non_empty.iter().filter(|v| is_hs_code_format(v)).count();
            let match_rate = match_count as f32 / non_empty.len() as f32;
            if match_rate < 0.5 {
                return Some((
                    "representation.numeric.decimal_number".to_string(),
                    format!("hs_code_validation_gate:match_rate={:.2}", match_rate),
                ));
            }
        }
    }

    // Rule 22: UPC digit-count gate — correct UPC predictions when value lengths
    // don't match 12-digit UPC format. The model confuses similar digit-string
    // identifiers: EAN (13 digits), NPI (10 digits) get predicted as UPC.
    if result_label == "identity.commerce.upc" {
        let digit_only: Vec<&str> = values
            .iter()
            .map(|v| v.trim())
            .filter(|v| !v.is_empty() && v.chars().all(|c| c.is_ascii_digit()))
            .collect();
        if digit_only.len() >= 3 {
            // Count values by digit length
            let twelve = digit_only.iter().filter(|v| v.len() == 12).count();
            let thirteen = digit_only.iter().filter(|v| v.len() == 13).count();
            let eight = digit_only.iter().filter(|v| v.len() == 8).count();
            let total = digit_only.len();
            let twelve_rate = twelve as f32 / total as f32;

            if twelve_rate < 0.5 {
                // Not UPC — check if EAN (13 or 8 digits)
                let ean_rate = (thirteen + eight) as f32 / total as f32;
                if ean_rate > 0.5 {
                    return Some((
                        "identity.commerce.ean".to_string(),
                        format!("upc_digit_count_gate:ean_rate={:.2}", ean_rate),
                    ));
                }
                // Otherwise demote to numeric_code (generic identifier)
                return Some((
                    "representation.identifier.numeric_code".to_string(),
                    format!("upc_digit_count_gate:twelve_rate={:.2}", twelve_rate),
                ));
            }
        }
    }

    // Rule 23: ISIN format gate — correct ISRC predictions when values match
    // ISIN format (2-letter country code + 9 alphanumeric + 1 check digit).
    // ISRC format is different: CC-XXX-YY-NNNNN with dashes.
    if result_label == "identity.commerce.isrc" {
        let non_empty: Vec<&str> = values
            .iter()
            .map(|v| v.trim())
            .filter(|v| !v.is_empty())
            .collect();
        if non_empty.len() >= 3 {
            let isin_count = non_empty.iter().filter(|v| is_isin_format(v)).count();
            let isin_rate = isin_count as f32 / non_empty.len() as f32;
            if isin_rate > 0.5 {
                return Some((
                    "finance.securities.isin".to_string(),
                    format!("isin_format_gate:match_rate={:.2}", isin_rate),
                ));
            }
        }
    }

    // Rule 24: ISSN/EIN dash-position gate — correct EIN predictions when values
    // match ISSN format (DDDD-DDDD, dash at position 4) rather than EIN (DD-DDDDDDD,
    // dash at position 2).
    if result_label == "identity.government.ein" {
        let non_empty: Vec<&str> = values
            .iter()
            .map(|v| v.trim())
            .filter(|v| !v.is_empty())
            .collect();
        if non_empty.len() >= 3 {
            let issn_count = non_empty.iter().filter(|v| is_issn_format(v)).count();
            let issn_rate = issn_count as f32 / non_empty.len() as f32;
            if issn_rate > 0.5 {
                return Some((
                    "identity.commerce.issn".to_string(),
                    format!("issn_format_gate:match_rate={:.2}", issn_rate),
                ));
            }
        }
    }

    // Rule 14: Duration override — ISO 8601 durations misclassified as SEDOL
    if result_label == "finance.securities.sedol" {
        if let Some((label, rule)) = disambiguate_duration_override(values) {
            return Some((label, rule));
        }
    }

    // Rule 17: UTC offset override
    if let Some((label, rule)) = disambiguate_utc_offset_override(values) {
        return Some((label, rule));
    }

    // Rule 15: Attractor type demotion
    // ADAPTED: uses result.confidence directly instead of top_count / n_samples
    // (which produces 0.0 with multi-branch because confidence as usize truncates to 0)
    if let Some((label, rule)) =
        sharpen_attractor_demotion(values, result_label, result_confidence, taxonomy)
    {
        return Some((label, rule));
    }

    // Rule 16: Text length demotion (BACKLOG #7 extends from full_address to the
    // entity_name / word long-prose overcalls).
    if matches!(
        result_label,
        "geography.address.full_address"
            | "representation.text.entity_name"
            | "representation.text.word"
    ) {
        if let Some((label, rule)) = disambiguate_text_length_demotion(
            values,
            &[(result_label.to_string(), 1)], // Minimal vote entry for the function signature
        ) {
            return Some((label, rule));
        }
    }

    // Rule 31: Version vs dmy_short_dot gate.
    // If model predicts dmy_short_dot but ≥30% of X.Y.Z values have segments
    // that are impossible as DD.MM.YY dates (day=0, month>12, month=0, year>99),
    // override to version. Version strings like "0.2.53" and "6.27.84" are
    // structurally similar to "DD.MM.YY" but have out-of-range segments.
    // The 30% threshold catches obvious version columns while allowing actual
    // dmy_short_dot columns with occasional malformed entries through.
    if result_label == "datetime.date.dmy_short_dot" {
        let non_empty: Vec<&str> = values
            .iter()
            .map(|v| v.trim())
            .filter(|v| !v.is_empty())
            .collect();
        if non_empty.len() >= 3 {
            let invalid_date_count = non_empty
                .iter()
                .filter(|v| {
                    let parts: Vec<&str> = v.split('.').collect();
                    if parts.len() != 3 {
                        return false;
                    }
                    let day = parts[0].parse::<u32>().unwrap_or(1);
                    let month = parts[1].parse::<u32>().unwrap_or(1);
                    let year = parts[2].parse::<u32>().unwrap_or(0);
                    day == 0 || day > 31 || month == 0 || month > 12 || year > 99
                })
                .count();
            let invalid_rate = invalid_date_count as f32 / non_empty.len() as f32;
            if invalid_rate >= 0.30 {
                return Some((
                    "technology.development.version".to_string(),
                    format!(
                        "version_dmy_short_dot_gate:invalid_date={:.2}",
                        invalid_rate
                    ),
                ));
            }
        }
    }

    // Rule 27: Year vs compact_ym gate (v15, Option C)
    // If model predicts compact_ym but ≥90% of values are exactly 4 digits
    // AND ≥80% of those 4-digit values fall in 1900–2100 (plausible years),
    // override to year. compact_ym is strictly 6 digits (YYYYMM); 4-digit
    // values are years, not compact year-month. The year range check prevents
    // false positives on non-year 4-digit values (spec review finding L1).
    if result_label == "datetime.date.compact_ym" {
        let non_empty: Vec<&str> = values
            .iter()
            .map(|v| v.trim())
            .filter(|v| !v.is_empty())
            .collect();
        if non_empty.len() >= 3 {
            let four_digit_count = non_empty
                .iter()
                .filter(|v| v.len() == 4 && v.chars().all(|c| c.is_ascii_digit()))
                .count();
            let four_digit_rate = four_digit_count as f32 / non_empty.len() as f32;
            if four_digit_rate >= 0.90 {
                // Check year plausibility: ≥80% in 1900–2100
                let year_count = non_empty
                    .iter()
                    .filter(|v| {
                        v.len() == 4
                            && v.parse::<u16>()
                                .map(|n| (1900..=2100).contains(&n))
                                .unwrap_or(false)
                    })
                    .count();
                let year_rate = year_count as f32 / non_empty.len() as f32;
                if year_rate >= 0.80 {
                    return Some((
                        "datetime.component.year".to_string(),
                        format!(
                            "year_compact_ym_gate:four_digit={:.2},year_range={:.2}",
                            four_digit_rate, year_rate
                        ),
                    ));
                }
            }
        }
    }

    // Rule 32: Closed-set over-emit demotion (earthquake-roundtrip-precision ac-03;
    // utc/url widening per v24 relitigation memo).
    // The Sense stage over-confidently emits narrow, pattern-bound types onto
    // free-form code/id/numeric columns. Four such labels are in scope:
    //   - measurement_unit (closed SI-unit enum) onto short network codes
    //   - geohash (base32 pattern) onto event ids
    //   - datetime.offset.utc (`^UTC [+-]\d{2}:\d{2}$`) onto plain integers
    //   - technology.internet.url (scheme://… pattern) onto bare ids/codes
    //   - geography.index.h3 (`^[0-9a-f]{15}$`) onto generic alphanumeric ids —
    //     h3's sibling geohash was already covered; h3 was the gap (spec
    //     2026-06-25-sharpen-stage-audit). hipc_gene_… / coord_id-style ids fail
    //     the 15-char-hex validator ~100%, so the >50% bar fires cleanly while real
    //     h3 cells (which validate) are untouched.
    // Demote — value-based, gated on the column's own schema-validation fail-rate
    // (>50%, see schema_fail_demotion) — to a representation.* fallback. utc and url
    // are added because their over-emits are schema-contradicted: the offending
    // columns fail the label's own validator ~100% (an integer is never `UTC +HH:MM`;
    // a bare code is never a URL), so the >50% bar fires cleanly while genuine
    // utc/url columns (which validate ~100%) are untouched. This is the additive
    // hard-negative retrain's job done with a value-based rule instead — see the v24
    // memo: retrain was 0-for-2 and moved the over-emit rather than removing it.
    // Scoped to a closed set of strict-validator labels, so unlike the attractor lists
    // (which also arm cardinality/confidence signals) it cannot regress unrelated
    // columns. Last-resort value-based Sharpen (decisions 0038/0048).
    //
    // Identifier/code over-emit additions (spec 2026-06-27-composed-accuracy-roadmap,
    // gold audit): aws_arn / ethereum_address / orcid / cpt / http_method / boolean.terms
    // are closed-validator types the flat softmax over-emits onto generic alphanumeric
    // ids (hipc_* / BenchmarkName), status vocabularies (*_status) and boolean-adjacent
    // columns. A genuine ARN / ORCID / CPT / HTTP-method / boolean-term column validates
    // ~100%; the offending columns fail the label's own validator ~100%, so the >50% bar
    // fires cleanly and the cardinality fallback lands the gold residual (alphanumeric_id
    // for high-card ids, word for small status vocabularies). Same schema-contradicted
    // demote discipline as utc/url above; the additive hard-negative retrain's job done
    // with a value-based rule (the over-emit retrains were 0-for-6).
    if result_label == "representation.scientific.measurement_unit"
        || result_label == "geography.coordinate.geohash"
        || result_label == "datetime.offset.utc"
        || result_label == "technology.internet.url"
        || result_label == "geography.index.h3"
        || result_label == "technology.cloud.aws_arn"
        || result_label == "finance.crypto.ethereum_address"
        || result_label == "identity.academic.orcid"
        || result_label == "identity.medical.cpt"
        || result_label == "technology.internet.http_method"
        || result_label == "representation.boolean.terms"
    {
        if let Some(taxonomy) = taxonomy {
            if let Some((label, rule)) = schema_fail_demotion(values, result_label, taxonomy) {
                return Some((label, rule));
            }
        }
    }

    // R32: word low-cardinality vocabulary override (spec
    // 2026-06-12-text-vocab-override). `word` carries no validator, so
    // neither the validation veto nor schema_fail_demotion can ever correct
    // it — a status/type vocabulary asserted as free words stays wrong. A
    // genuine word column is mostly distinct; a vocabulary repeats a
    // bounded set of generic words. Scoped to `word` ONLY: the corpus-honest
    // gate round 1 measured entity_name (3,752) and plain_text (2,115)
    // oracle-refuted moves — a column repeating eight manufacturer names IS
    // entity_name and repeated boilerplate IS plain_text; low cardinality
    // does not make named entities or prose an enum. Geography labels
    // (city/region) misfire likewise (4/17 true-city gold columns), and a
    // true http-method column is itself a small vocabulary. Value-based per
    // decision 0048.
    if result_label == "representation.text.word" {
        let non_empty: Vec<&str> = values
            .iter()
            .map(|v| v.trim())
            .filter(|v| !v.is_empty())
            .collect();
        let n = non_empty.len();
        if n >= 4 {
            let distinct: std::collections::HashSet<&str> = non_empty.iter().copied().collect();
            let d = distinct.len();
            if (2..=12).contains(&d) && (d as f32 / n as f32) <= 0.6 {
                return Some((
                    "representation.text.word".to_string(),
                    format!("text_vocab_override:distinct={}/{}", d, n),
                ));
            }
        }
    }

    None
}
/// Demote a closed-set label whose own schema the column's values fail.
///
/// Fires only when the predicted type has a compiled validator (enum or pattern)
/// and >50% of non-empty values fail it. Picks a `representation.*` fallback by
/// cardinality: 1–20 distinct → `discrete.categorical`, else
/// `identifier.alphanumeric_id`. Both are non-trivial taxonomy types, so the
/// recall guard (non_trivial_pct) is unaffected — no collapse to plain text.
pub(crate) fn schema_fail_demotion(
    values: &[String],
    result_label: &str,
    taxonomy: &Taxonomy,
) -> Option<(String, String)> {
    let non_empty: Vec<&str> = values
        .iter()
        .map(|v| v.trim())
        .filter(|v| !v.is_empty())
        .collect();
    if non_empty.len() < 3 {
        return None;
    }
    // Prefer the pre-compiled validator; fall back to compile-per-call when the
    // taxonomy cache isn't populated (mirrors disambiguate_attractor_demotion).
    let fail_count = if let Some(compiled) = taxonomy.get_validator(result_label) {
        non_empty.iter().filter(|v| !compiled.is_valid(v)).count()
    } else {
        let validation = taxonomy.get(result_label)?.validation.as_ref()?;
        non_empty
            .iter()
            .filter(|v| {
                finetype_core::validate_value(v, validation)
                    .map(|r| !r.is_valid)
                    .unwrap_or(false)
            })
            .count()
    };
    let fail_rate = fail_count as f32 / non_empty.len() as f32;
    // Demote only on majority failure (>50%). The instinct for a closed type is a
    // tight bar — a real geohash or SI-unit column "should" validate ~100%. But the
    // shipped definitions don't: the geohash pattern's v13 min-length-6 deliberately
    // rejects valid 4-5 char geohashes (~23% of a genuine column — see the geohash
    // `notes`), and the measurement_unit enum lists only 30 SI units, so real unit
    // columns fail ~44%. A tighter bar would demote those genuine columns. >50% fires
    // only when the column is overwhelmingly not the type (network codes that are
    // 100% non-units), which is the over-emit this rule targets. Precision Principle.
    if fail_rate <= 0.5 {
        return None;
    }
    let mut unique: Vec<&str> = non_empty.clone();
    unique.sort_unstable();
    unique.dedup();
    let fallback = if (1..=20).contains(&unique.len()) {
        "representation.text.word"
    } else {
        "representation.identifier.alphanumeric_id"
    };
    Some((
        fallback.to_string(),
        format!(
            "schema_fail_demotion:{}:fail_rate={:.2}",
            result_label, fail_rate
        ),
    ))
}
/// Attractor demotion adapted for multi-branch pipeline (AC-3, R15).
///
/// Uses `result_confidence` directly as the majority fraction instead of
/// computing `top_count as f32 / n_samples as f32` — which gives 0.0 with
/// multi-branch single-entry votes where `confidence as usize` truncates to 0.
///
/// When no second-place vote exists (multi-branch single-entry), uses
/// taxonomy-based category demotion for the fallback.
pub(crate) fn sharpen_attractor_demotion(
    values: &[String],
    result_label: &str,
    result_confidence: f32,
    taxonomy: Option<&Taxonomy>,
) -> Option<(String, String)> {
    let is_numeric = NUMERIC_ATTRACTORS.contains(&result_label);
    let is_text = TEXT_ATTRACTORS.contains(&result_label);
    let is_code = CODE_ATTRACTORS.contains(&result_label);

    if !is_numeric && !is_text && !is_code {
        return None;
    }

    // Signal 1: Validation failure (strongest signal)
    let mut locale_confirmed = false;
    let mut validation_confirmed = false;
    let mut has_locale_validators = false;
    if let Some(taxonomy) = taxonomy {
        let has_pattern = taxonomy
            .get(result_label)
            .and_then(|d| d.validation.as_ref())
            .and_then(|v| v.pattern.as_ref())
            .is_some();

        let non_empty: Vec<&str> = values
            .iter()
            .map(|v| v.trim())
            .filter(|v| !v.is_empty())
            .collect();

        if non_empty.len() >= 3 {
            if let Some(locale_validators) = taxonomy.get_locale_validators(result_label) {
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
                    locale_confirmed = true;
                }
            }

            if !locale_confirmed {
                let fail_count = if let Some(compiled) = taxonomy.get_validator(result_label) {
                    non_empty.iter().filter(|v| !compiled.is_valid(v)).count()
                } else if let Some(def) = taxonomy.get(result_label) {
                    if let Some(validation) = &def.validation {
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
                    let fallback = sharpen_select_fallback(is_numeric, is_text, is_code, values);
                    return Some((
                        fallback,
                        format!("attractor_demotion_validation:{}", result_label),
                    ));
                }
                if has_pattern && fail_rate <= 0.3 {
                    validation_confirmed = true;
                }
            }
        }
    }

    // Signal 2: Confidence threshold
    // ADAPTED: use result_confidence directly instead of top_count / n_samples
    let confirmed = locale_confirmed || (!has_locale_validators && validation_confirmed);
    if !confirmed && result_confidence < 0.85 {
        let fallback = sharpen_select_fallback(is_numeric, is_text, is_code, values);
        return Some((
            fallback,
            format!("attractor_demotion_confidence:{}", result_label),
        ));
    }

    // Signal 3: Cardinality mismatch (text attractors only)
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
                "representation.text.word".to_string(),
                format!("attractor_demotion_cardinality:{}", result_label),
            ));
        }
    }

    None
}
/// Select fallback for Sharpen attractor demotion (no votes available).
///
/// Without a vote distribution, uses taxonomy-based category demotion.
pub(crate) fn sharpen_select_fallback(
    is_numeric: bool,
    is_text: bool,
    is_code: bool,
    values: &[String],
) -> String {
    if is_numeric {
        let has_decimal = values.iter().any(|v| v.contains('.'));
        if has_decimal {
            "representation.numeric.decimal_number".to_string()
        } else {
            "representation.numeric.integer_number".to_string()
        }
    } else if is_text {
        "representation.text.word".to_string()
    } else if is_code {
        "representation.alphanumeric.alphanumeric_id".to_string()
    } else {
        "representation.text.word".to_string()
    }
}
/// Apply disambiguation rules when the vote distribution contains known ambiguous pairs.
///
/// Returns Some((resolved_label, rule_name)) if a rule was applied, None otherwise.
pub(crate) fn disambiguate(
    values: &[String],
    results: &[ClassificationResult],
    votes: &[(String, usize)],
    n_samples: usize,
    taxonomy: Option<&Taxonomy>,
) -> Option<(String, String)> {
    // Get the top labels in the vote
    let top_labels: Vec<&str> = votes.iter().take(3).map(|(l, _)| l.as_str()).collect();

    // Rule 1: Date slash disambiguation (mdy_slash vs dmy_slash)
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
    // Only fire when coordinate labels are competitive — if a non-coordinate label
    // dominates (>3x combined coordinate votes), skip disambiguation to avoid
    // false-positive coordinate detection on generic decimal columns.
    if contains_pair(&top_labels, COORDINATE_PAIR.0, COORDINATE_PAIR.1) {
        let coord_votes: usize = votes
            .iter()
            .filter(|(l, _)| l == COORDINATE_PAIR.0 || l == COORDINATE_PAIR.1)
            .map(|(_, c)| c)
            .sum();
        let top_votes = votes.first().map(|(_, c)| *c).unwrap_or(0);
        let top_is_coord = votes
            .first()
            .map(|(l, _)| l == COORDINATE_PAIR.0 || l == COORDINATE_PAIR.1)
            .unwrap_or(false);
        if top_is_coord || coord_votes * 3 >= top_votes {
            if let Some(label) = disambiguate_coordinates(values) {
                return Some((label, "coordinate_disambiguation".to_string()));
            }
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

    // Rule 19: Percentage without '%' sign → decimal_number.
    // When percentage wins the vote but no values contain a '%' character, the
    // prediction is based purely on numeric range overlap (small decimals look
    // like percentages). Real percentage columns have explicit "35.36%" values.
    if top_labels
        .first()
        .is_some_and(|l| *l == "representation.numeric.percentage")
    {
        let has_pct_sign = values.iter().any(|v| v.contains('%'));
        if !has_pct_sign {
            return Some((
                "representation.numeric.decimal_number".to_string(),
                "percentage_no_sign".to_string(),
            ));
        }
    }

    // Rule 14: Duration override — ISO 8601 durations (PT20M, P1DT12H)
    // misclassified as SEDOL stock codes because both are 5-8 char alphanumeric
    // strings starting with uppercase letters. Check for duration pattern before
    // attractor demotion would demote SEDOL to alphanumeric_id (losing duration).
    if top_labels
        .first()
        .is_some_and(|l| *l == "finance.securities.sedol")
    {
        if let Some((label, rule)) = disambiguate_duration_override(values) {
            return Some((label, rule));
        }
    }

    // Rule 17: UTC offset override — standalone UTC offset values like "+05:30"
    // misclassified as time formats or other types because the HH:MM pattern
    // overlaps. The mandatory leading +/- sign at ≥80% of values is sufficient
    // to distinguish offsets — no top-label guard needed.
    if let Some((label, rule)) = disambiguate_utc_offset_override(values) {
        return Some((label, rule));
    }

    // Rule 15: Attractor type demotion — demote over-eager specific type
    // predictions (postal_code, first_name, etc.) when evidence doesn't
    // support the specific prediction. Three signals: validation failure,
    // confidence threshold, and cardinality mismatch.
    if let Some((label, rule)) = disambiguate_attractor_demotion(values, votes, n_samples, taxonomy)
    {
        return Some((label, rule));
    }

    // Rule 16: Text length demotion — full_address predictions where
    // the median value length exceeds 100 characters are almost certainly
    // free-form text (descriptions, paragraphs, recipe steps) rather than
    // street addresses. Demote to representation.text.plain_text.
    // Threshold 100 gives 0% false demotion rate on SOTAB evaluation data.
    if let Some((label, rule)) = disambiguate_text_length_demotion(values, votes) {
        return Some((label, rule));
    }

    None
}
/// Detect day-of-week columns where values are day names (Monday, Tuesday, etc.).
///
/// Rule: If ≥80% of non-empty values are recognized day names → datetime.component.day_of_week
pub(crate) fn disambiguate_day_of_week(values: &[String]) -> Option<String> {
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
pub(crate) fn disambiguate_month_name(values: &[String]) -> Option<String> {
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
pub(crate) fn disambiguate_boolean_subtype(
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
        // Require exactly 2 unique values (both 0 AND 1 present). All-zero or all-one
        // columns are count/sentinel data, not boolean. Also reject when one value
        // dominates >95% of samples — not meaningful boolean variation.
        if unique_values.len() == 2 {
            let dominant_frac = non_empty
                .iter()
                .filter(|&&v| v == "0")
                .count()
                .max(non_empty.iter().filter(|&&v| v == "1").count())
                as f64
                / n;
            if dominant_frac <= 0.95 {
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
    } else {
        None
    }
}
/// Detect gender columns by checking if all values match a known gender value set.
///
/// Rule: If ALL non-empty values are in the gender set → identity.person.gender
pub(crate) fn disambiguate_gender(values: &[String]) -> Option<String> {
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

    // Single-character alpha sex codes (ICAO Doc 9303: M/F/X) are gender_code,
    // not gender. The gender enum is the word set [male,female,other,unknown],
    // so a column of bare M/F validates against gender_code's enum
    // [M,F,X,0,1,2,9] and is hard-vetoed against gender. Alpha-only: the
    // ISO-5218 numeric codes (0/1/2/9) are deliberately NOT a value-only
    // trigger — they would over-emit gender_code onto every boolean/ordinal
    // numeric column (boolean_subtype owns 0/1 and runs first). Per the
    // deterministic-layer audit false-veto resolution (spec
    // 2026-06-12-false-veto-trio-resolution).
    let all_code = non_empty
        .iter()
        .all(|v| matches!(v.to_ascii_uppercase().as_str(), "M" | "F" | "X"));
    if all_code {
        return Some("identity.person.gender_code".to_string());
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
pub(crate) fn disambiguate_boolean_override(
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
                    "representation.text.word".to_string(),
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

        // All-zero or single-value columns are not boolean — they're counts,
        // sentinels, or constant columns. Require ≥2 distinct values for
        // boolean classification to stand. (Distillation v2: 175 cases)
        if n_unique < 2 {
            return Some((
                "representation.numeric.integer_number".to_string(),
                "boolean_override_single_value".to_string(),
            ));
        }

        // If one value dominates >95% of samples, it's not meaningful boolean data.
        // For n_unique==2, one of {count(==first), count(!=first)} is the majority;
        // .max() picks whichever is larger regardless of which value came first in
        // the parsed vector.
        let dominant_count = parsed
            .iter()
            .filter(|&&v| v == parsed[0])
            .count()
            .max(parsed.iter().filter(|&&v| v != parsed[0]).count());
        let dominant_frac = dominant_count as f64 / parsed.len() as f64;
        if n_unique == 2 && dominant_frac > 0.95 {
            return Some((
                "representation.numeric.integer_number".to_string(),
                "boolean_override_skewed".to_string(),
            ));
        }

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

// disambiguate_small_integer_ordinal removed in a Sharpen rule audit.
// Ablation: net -2 (0 fixes, 2 regressions). v19-relu model handles these correctly.

// disambiguate_categorical removed in the same audit.
// Both branches ablated: categorical_single_char (net 0), categorical_low_cardinality (net -1).
// The demotion guard is no longer needed — there are no demotion rules to guard against.
// v19-relu model handles categorical detection without heuristic overrides.

// ═══════════════════════════════════════════════════════════════════════════════
// HEADER NAME HINTS
// ═══════════════════════════════════════════════════════════════════════════════
/// Detect Unix epoch seconds from value ranges.
///
/// 10-digit integers in the range 946684800–2524608000 (2000-01-01 to 2050-01-01)
/// are Unix epoch seconds. CharCNN consistently misclassifies these as NPI or other
/// identity types because the digit pattern overlaps.
///
/// Also detects epoch milliseconds (13-digit integers in range
/// 946684800000–2524608000000).
///
/// Requires ≥80% of non-empty values to be parseable as epoch timestamps,
/// allowing some nulls or header rows.
pub(crate) fn detect_epoch_seconds(values: &[String]) -> Option<String> {
    const EPOCH_MIN: i64 = 946_684_800; // 2000-01-01T00:00:00Z
    const EPOCH_MAX: i64 = 2_524_608_000; // 2050-01-01T00:00:00Z
    const EPOCH_MS_MIN: i64 = EPOCH_MIN * 1000;
    const EPOCH_MS_MAX: i64 = EPOCH_MAX * 1000;

    let non_empty: Vec<&str> = values
        .iter()
        .map(|v| v.trim())
        .filter(|v| !v.is_empty())
        .collect();

    if non_empty.len() < 3 {
        return None;
    }

    let mut epoch_sec_count = 0usize;
    let mut epoch_ms_count = 0usize;
    let mut parseable_count = 0usize;

    for val in &non_empty {
        // Try parsing as integer first, then as float with .0 fractional part
        let num: Option<i64> = val.parse::<i64>().ok().or_else(|| {
            val.parse::<f64>().ok().and_then(|f| {
                if f.fract() == 0.0 {
                    Some(f as i64)
                } else {
                    None
                }
            })
        });

        if let Some(n) = num {
            parseable_count += 1;
            if (EPOCH_MIN..=EPOCH_MAX).contains(&n) {
                epoch_sec_count += 1;
            } else if (EPOCH_MS_MIN..=EPOCH_MS_MAX).contains(&n) {
                epoch_ms_count += 1;
            }
        }
    }

    // Require ≥80% parseable as numbers and ≥80% of those in epoch range
    let n = non_empty.len();
    if parseable_count < n * 80 / 100 {
        return None;
    }

    if epoch_sec_count >= parseable_count * 80 / 100 {
        Some("datetime.epoch.unix_seconds".to_string())
    } else if epoch_ms_count >= parseable_count * 80 / 100 {
        Some("datetime.epoch.unix_milliseconds".to_string())
    } else {
        None
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// DISAMBIGUATION RULES
// ═══════════════════════════════════════════════════════════════════════════════
/// Disambiguate mdy_slash vs dmy_slash dates.
///
/// Pattern: `DD/MM/YYYY` or `MM/DD/YYYY`
/// Rule: If ANY value has first component > 12, it must be DD/MM (dmy_slash).
///       If ANY value has second component > 12, it must be MM/DD (mdy_slash).
pub(crate) fn disambiguate_slash_dates(values: &[String]) -> Option<String> {
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
        // First component > 12 means it's the day → DD/MM/YYYY → dmy_slash
        Some("datetime.date.dmy_slash".to_string())
    } else if second_over_12 && !first_over_12 {
        // Second component > 12 means it's the day → MM/DD/YYYY → mdy_slash
        Some("datetime.date.mdy_slash".to_string())
    } else {
        // Both ambiguous or contradictory — let model decide
        None
    }
}
/// Disambiguate short_dmy vs short_mdy dates.
///
/// Pattern: `DD-MM-YY` or `MM-DD-YY`
/// Rule: Same as slash dates but with dash separator.
pub(crate) fn disambiguate_short_dates(values: &[String]) -> Option<String> {
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
pub(crate) fn disambiguate_coordinates(values: &[String]) -> Option<String> {
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
/// Rule: If ≥80% of non-empty values match `\d{1,3}.\d{1,3}.\d{1,3}.\d{1,3}`
/// with each octet in 0..255, classify as ip_v4.
///
/// This prevents the common confusion between IP addresses and version numbers
/// (e.g., "10.0.32.113" looks like a semver to the model).
pub(crate) fn disambiguate_ipv4(values: &[String]) -> Option<String> {
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
/// Covers: port, increment, postal_code, integer_number, year
pub(crate) fn disambiguate_numeric(
    values: &[String],
    results: &[ClassificationResult],
    top_labels: &[&str],
) -> Option<(String, String)> {
    // Only trigger for numeric-looking columns
    let numeric_types = [
        "representation.identifier.increment",
        "representation.numeric.integer_number",
        "representation.numeric.decimal_number",
        "geography.address.postal_code",
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
            "representation.identifier.increment".to_string(),
            "numeric_sequential_detection".to_string(),
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
        // R25 guard: HTTP status code gate (v15, Option C).
        // If all values are 3-digit and ≥90% are in 100-599 (HTTP status range),
        // these are status codes, not postal codes. Keep the model's prediction.
        if digit_lengths.iter().all(|&l| l == 3) {
            let status_count = parsed.iter().filter(|&&v| (100..=599).contains(&v)).count();
            let status_rate = status_count as f64 / parsed.len() as f64;
            if status_rate >= 0.90 {
                // Don't convert to postal_code — return None to keep model prediction
                return None;
            }
        }
        // Consistent digit length, typical postal range → postal code
        return Some((
            "geography.address.postal_code".to_string(),
            "numeric_postal_code_detection".to_string(),
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
pub(crate) fn disambiguate_si_number(values: &[String]) -> Option<(String, String)> {
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
pub(crate) fn disambiguate_duration_override(values: &[String]) -> Option<(String, String)> {
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
/// UTC offset override: standalone offsets misclassified as time values.
///
/// UTC offsets like "+05:30", "-08:00", "+00:00" follow the pattern [+-]HH:MM.
/// The CharCNN sees the HH:MM structure and predicts time types (hm_24h,
/// hms_24h) since those share the same colon-separated digit format. The
/// mandatory leading sign (+/-) is the syntactic distinguisher.
///
/// Rule: If the top vote is a datetime.time.* type and ≥80% of non-empty
/// values match ^[+-]HH:MM$, override to datetime.offset.utc.
pub(crate) fn disambiguate_utc_offset_override(values: &[String]) -> Option<(String, String)> {
    let non_empty: Vec<&str> = values
        .iter()
        .map(|v| v.trim())
        .filter(|v| !v.is_empty())
        .collect();

    if non_empty.len() < 3 {
        return None;
    }

    // UTC offset pattern: mandatory +/- sign, then exactly HH:MM
    let offset_count = non_empty
        .iter()
        .filter(|v| {
            let bytes = v.as_bytes();
            // Must be exactly 6 chars: [+-]HH:MM
            if bytes.len() != 6 {
                return false;
            }
            // First char must be + or -
            if bytes[0] != b'+' && bytes[0] != b'-' {
                return false;
            }
            // Then two digits, colon, two digits
            bytes[1].is_ascii_digit()
                && bytes[2].is_ascii_digit()
                && bytes[3] == b':'
                && bytes[4].is_ascii_digit()
                && bytes[5].is_ascii_digit()
        })
        .count();

    let fraction = offset_count as f64 / non_empty.len() as f64;

    if fraction >= 0.8 {
        Some((
            "datetime.offset.utc".to_string(),
            "utc_offset_override_time".to_string(),
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
/// length exceeds 100 characters, demote to representation.text.plain_text.
/// Threshold 100 gives 0% false demotion rate on evaluation data.
pub(crate) fn disambiguate_text_length_demotion(
    values: &[String],
    votes: &[(String, usize)],
) -> Option<(String, String)> {
    let top_label = votes.first().map(|(l, _)| l.as_str())?;

    // LENGTH direction ONLY — orthogonal to the litigated short-vocab
    // text_vocab_override NO-GO (R32), which fires on LOW cardinality
    // (distinct 2..=12, distinct/n <= 0.6). Long prose is HIGH cardinality with
    // median length > 100, so the two signals can never co-fire. A real
    // entity_name / word / address is never 100+ chars, so the demotion is
    // safe-by-construction (precedent: 0% false-demotion on the full_address arm).
    const LONG_PROSE_SOURCES: &[&str] = &[
        "geography.address.full_address",
        "representation.text.entity_name",
        "representation.text.word",
    ];
    if !LONG_PROSE_SOURCES.contains(&top_label) {
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
        // Keep the existing rule string on the full_address arm (preserves
        // test_text_length_demotion_long_text_as_address); tag the new arms.
        let rule = if top_label == "geography.address.full_address" {
            "text_length_demotion_full_address".to_string()
        } else {
            format!("text_length_demotion_long_prose:{top_label}")
        };
        Some(("representation.text.plain_text".to_string(), rule))
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
/// **Validation Precision:** For locale-specific types (those with
/// `validation_by_locale`), only locale-level confirmation gates Signals 2 and 3.
/// Universal validation can reject (Signal 1) but cannot confirm — passing a
/// permissive universal pattern like `^[+]?[0-9\s()\-\.]+$` is not evidence.
/// For types without locale validation, universal confirmation still gates Signal 2.
///
/// This rule runs AFTER all other disambiguation rules and BEFORE header hint
/// override, so header hints can still rescue legitimate predictions that were
/// demoted (e.g., model says postal_code at 0.7, header is "zip_code").
pub(crate) fn disambiguate_attractor_demotion(
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
    // Tracks two independent confirmation signals (Precision Principle):
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
        // Use pre-compiled validator from taxonomy cache.
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
    // Confirmation gating (Precision Principle):
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
    // SKIP if locale-confirmed: locale-specific patterns are
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
                "representation.text.word".to_string(),
                format!("attractor_demotion_cardinality:{}", top_label),
            ));
        }
    }

    None
}
/// Column schema-validation gate (Precision Principle).
///
/// Computes the column's pass-rate against the predicted type's JSON Schema
/// validator. When >50% of non-empty values fail, the prediction is not
/// supported by the data — demote to the strongest alternative that *does*
/// validate, else to a representation.* fallback chosen by value shape.
///
/// This generalises Signal 1 of [`disambiguate_attractor_demotion`] beyond the
/// attractor lists: a per-value classifier can over-emit any type whose schema
/// the column violates (notably `geography.coordinate.latitude` on decimal
/// columns where longitude/depth values exceed ±90). Locale-specific types are
/// skipped — their real validation lives in `validation_by_locale`, and a
/// permissive universal pattern is not evidence either way.
///
/// Runs AFTER feature disambiguation and BEFORE locale detection, so a demoted
/// label can still pick up a locale on the fallback type if applicable.
pub(crate) fn schema_validation_gate(
    result: &mut ColumnResult,
    values: &[String],
    votes: &[(String, usize)],
    taxonomy: &Taxonomy,
) {
    // Already a generic representation.* type — nothing stricter to fail against.
    if result.label.starts_with("representation.") {
        return;
    }
    // Locale-specific types validate through validation_by_locale, not the
    // universal pattern — skip to avoid demoting on a permissive universal.
    if taxonomy.get_locale_validators(&result.label).is_some() {
        return;
    }
    let Some(validator) = taxonomy.get_validator(&result.label) else {
        return;
    };

    let non_empty: Vec<&str> = values
        .iter()
        .map(|v| v.trim())
        .filter(|v| !v.is_empty())
        .collect();
    if non_empty.len() < 3 {
        return;
    }

    let fail_count = non_empty.iter().filter(|v| !validator.is_valid(v)).count();
    let fail_rate = fail_count as f32 / non_empty.len() as f32;
    if fail_rate <= 0.5 {
        return;
    }

    // Prefer the strongest alternative vote that validates against its own schema.
    let demoted = votes
        .iter()
        .find(|(label, _)| {
            label != &result.label
                && taxonomy
                    .get_validator(label)
                    .map(|alt| {
                        let pass = non_empty.iter().filter(|v| alt.is_valid(v)).count();
                        pass as f32 / non_empty.len() as f32 > 0.5
                    })
                    .unwrap_or(false)
        })
        .map(|(label, _)| label.clone())
        .unwrap_or_else(|| {
            let is_numeric = non_empty.iter().all(|v| v.parse::<f64>().is_ok());
            select_fallback(votes, is_numeric, !is_numeric, false, values)
        });

    if demoted != result.label {
        result.disambiguation_applied = true;
        result.disambiguation_rule = Some(format!("schema_validation_gate:{}", result.label));
        result.label = demoted;
        result.confidence = result.confidence.min(0.6);
        result.detected_locale = None;
    }
}
pub(crate) fn select_fallback(
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
        "representation.text.word".to_string()
    } else if is_code {
        "representation.alphanumeric.alphanumeric_id".to_string()
    } else {
        "representation.text.word".to_string()
    }
}
