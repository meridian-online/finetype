use super::*;

mod categorical_rules;
mod datetime_rules;
mod format_rules;
mod numeric_rules;

pub(crate) use categorical_rules::*;
pub(crate) use datetime_rules::*;
pub(crate) use format_rules::*;
pub(crate) use numeric_rules::*;

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
        let non_empty = non_empty_trimmed(values);
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
        let non_empty = non_empty_trimmed(values);
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
        let non_empty = non_empty_trimmed(values);
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
        let non_empty = non_empty_trimmed(values);
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

    // Rule 16b: full_address whitespace guard (BACKLOG #8). A real address is
    // always multi-token; a single-token column the model collapsed into the
    // address attractor is an identifier (idx 43/44: CUDNN benchmark IDs).
    if result_label == "geography.address.full_address" {
        if let Some((label, rule)) = disambiguate_full_address_whitespace_guard(values) {
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
        let non_empty = non_empty_trimmed(values);
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
        let non_empty = non_empty_trimmed(values);
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
    // Veto-blind strict-validator additions (company-reference audit, W1a —
    // output/company-reference-audit/findings_and_action_plan.md): these labels
    // carry strict validators (anchored keyword/prefix/grid patterns a genuine
    // column passes ~100%) but are OMITTED from labels/veto_safe.txt by
    // rare-type starvation, so a schema-contradicted assertion previously
    // shipped with only an advisory flag — verified live via label injection
    // (a prose column asserted wkt kept the label at pass_rate 0.0). Admitting
    // them here gives the hard-NO the veto cannot: wkt (leading geometry
    // keyword), user_agent (known-client prefix alternation; the shipped
    // default's largest single over-emit, 359/13,478 corpus columns), and the
    // strict coordinate/container/chemistry tail (mgrs / plus_code / dms /
    // iso6346 / inchi). Same schema-contradicted discipline as utc/url above.
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
        || result_label == "geography.format.wkt"
        || result_label == "technology.internet.user_agent"
        || result_label == "geography.coordinate.mgrs"
        || result_label == "geography.coordinate.plus_code"
        || result_label == "geography.coordinate.dms"
        || result_label == "geography.transportation.iso6346"
        || result_label == "representation.scientific.inchi"
        // swift_bic joined 2026-07-04 (company-reference audit follow-up 2):
        // measured shipping on real GLEIF data at pass_rate 0.007 — a
        // normalized-name column wearing the BIC label with only an advisory
        // flag. Positional structure (^[A-Z]{4}[A-Z]{2}[A-Z0-9]{2}...) passes
        // genuine BIC columns ~100%; prose/name columns fail overwhelmingly.
        || result_label == "finance.banking.swift_bic"
        // query_string joined 2026-07-09 (v0.6.42 dataset-descriptor audit,
        // notes.md item 2): the flat softmax emits query_string @1.0 on
        // low-cardinality enums / short-code columns (NAICS `level` =
        // sector/subsector/…) that carry NEITHER `=` NOR `&`. Its validator
        // `^[^=&]+=([^&]*)(&[^=&]+=[^&]*)*$` REQUIRES a `k=v` pair, so a genuine
        // query-string column passes ~100% while these fail ~100% — but
        // query_string is absent from veto_safe.txt (rare-type starvation), so
        // the veto only flags it. Admitting it here gives the hard-NO the veto
        // cannot: the >50% bar demotes to the vocabulary residual (`word` for the
        // small enum) and never touches a real query-string column.
        || result_label == "container.key_value.query_string"
    {
        if let Some(taxonomy) = taxonomy {
            if let Some((label, rule)) = schema_fail_demotion(values, result_label, taxonomy) {
                return Some((label, rule));
            }
        }
    }

    // R34: excel_format prose demotion. The CharCNN reads dotted/suffixed
    // company names (`A.E.R.C.O. S.A.`, Spanish `SL`/`SLU`/`SA`/`SICAV`
    // registry forms) as Excel-format-shaped, and nothing recovers them:
    // excel_format is absent from R32's strict-validator list, its taxonomy
    // pattern ends in `\w` so real names PASS it (schema_fail_demotion's
    // >50%-fail bar never trips), and org_name_geography_demotion only fires
    // on geography leaves. Value-shape separation: an Excel number-format
    // string's alphabetic content lives either inside quoted "..." literals /
    // [...] sections or in the bare-token alphabet {a,d,e,g,h,m,p,s,y}
    // (y/m/d/h/s date-time codes, AM/PM, Exponent, General — case-insensitive).
    // Strip the quoted/bracketed sections; any remaining alphabetic char
    // outside that alphabet marks the value non-format. Demote to entity_name
    // when >50% of values are non-format AND the median whitespace-token count
    // is >=2 — the >=2-token gate is load-bearing (mirrors org_suffix_ratio's
    // multi-word discipline): it spares single-token id/code columns that a
    // stray letter would otherwise flag. Measured separation: 200/200 GLEIF
    // legal names flagged non-format, 0/10 legit format strings flagged.
    // Value-based per 0048; demote-only; RHH-disableable.
    if result_label == "representation.file.excel_format"
        && !crate::rhh::is_disabled("excel_format_prose_demotion")
    {
        let non_empty = non_empty_trimmed(values);
        if non_empty.len() >= 3 {
            // Strip double-quoted "..." literals and [...] sections — the two
            // format-syntax carriers of arbitrary text (0"kg", [$-409],
            // [Red]). Unterminated sections swallow the rest of the value,
            // matching how a format parser treats them.
            let strip_literals = |v: &str| -> String {
                let mut out = String::with_capacity(v.len());
                let mut in_quote = false;
                let mut in_bracket = false;
                for c in v.chars() {
                    if in_quote {
                        in_quote = c != '"';
                    } else if in_bracket {
                        in_bracket = c != ']';
                    } else {
                        match c {
                            '"' => in_quote = true,
                            '[' => in_bracket = true,
                            _ => out.push(c),
                        }
                    }
                }
                out
            };
            let is_non_format = |v: &str| {
                strip_literals(v).chars().any(|c| {
                    c.is_alphabetic()
                        && !matches!(
                            c.to_ascii_lowercase(),
                            'a' | 'd' | 'e' | 'g' | 'h' | 'm' | 'p' | 's' | 'y'
                        )
                })
            };
            let mut tok_counts: Vec<usize> = non_empty
                .iter()
                .map(|v| v.split_whitespace().count())
                .collect();
            tok_counts.sort_unstable();
            let median_tokens = tok_counts[tok_counts.len() / 2];
            let non_format_n = non_empty.iter().filter(|v| is_non_format(v)).count();
            if median_tokens >= 2 && non_format_n * 2 > non_empty.len() {
                return Some((
                    "representation.text.entity_name".to_string(),
                    format!(
                        "excel_format_prose_demotion:non_format={}/{}",
                        non_format_n,
                        non_empty.len()
                    ),
                ));
            }
        }
    }

    // R33: entity_name prose override (company-reference audit, gold-priced).
    // entity_name over-emits onto free-text prose (titles, descriptions,
    // sentence fragments) — 25 of the expanded gold's entity_name assertions
    // sit on plain_text/word truth, and entity_name carries no validator to
    // contradict them. Value-shape separation, measured on gold both-sides
    // (16 true entity columns incl. connector-word org names like "Bank of
    // America Corporation" and species binomials: ZERO false fires; synthetic
    // prose: fires): a value is prose-like when it carries >=2 lowercase
    // alphabetic words of length >=4 (connectors "of"/"and" are shorter; org
    // names are Title-Case/ALL-CAPS), and the column demotes when the median
    // token count is >=3 AND >=50% of values are prose-like. Single-token
    // vocabularies (status enums) are deliberately NOT covered — gold holds
    // genuine single-token entity_name columns and no value shape separates
    // them. Value-based per 0048; demote-only.
    if result_label == "representation.text.entity_name" {
        let non_empty = non_empty_trimmed(values);
        if non_empty.len() >= 4 {
            let is_prose = |v: &str| {
                v.split_whitespace()
                    .filter(|w| {
                        w.len() >= 4 && w.chars().all(|c| c.is_alphabetic() && c.is_lowercase())
                    })
                    .count()
                    >= 2
            };
            let mut tok_counts: Vec<usize> = non_empty
                .iter()
                .map(|v| v.split_whitespace().count())
                .collect();
            tok_counts.sort_unstable();
            let median_tokens = tok_counts[tok_counts.len() / 2];
            let prose_n = non_empty.iter().filter(|v| is_prose(v)).count();
            if median_tokens >= 3 && prose_n * 2 >= non_empty.len() {
                return Some((
                    "representation.text.plain_text".to_string(),
                    format!(
                        "entity_prose_override:prose={}/{}",
                        prose_n,
                        non_empty.len()
                    ),
                ));
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
        let non_empty = non_empty_trimmed(values);
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
    let non_empty = non_empty_trimmed(values);
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
        // A shape-only pattern (e.g. icao's `^[A-Z]{4}$`) confirms every
        // same-shape token, so it must NOT count as confirmation that disarms
        // the demotion below (company-reference audit W2 item 9). Only a
        // genuinely precise validator — enum, or a pattern requiring literal
        // structure — is positive evidence.
        let has_precise_validation = taxonomy
            .get(result_label)
            .and_then(|d| d.validation.as_ref())
            .map(|v| v.is_precise())
            .unwrap_or(false);

        let non_empty = non_empty_trimmed(values);

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
                if has_precise_validation && fail_rate <= 0.3 {
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
        let non_empty = non_empty_trimmed(values);
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
        "representation.identifier.alphanumeric_id".to_string()
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
        // Only a genuinely precise validator (enum, or a pattern requiring
        // literal structure) is positive evidence; a shape-only pattern confirms
        // any same-shape token and must not disarm the demotion (W2 item 9).
        let has_precise_validation = taxonomy
            .get(top_label)
            .and_then(|d| d.validation.as_ref())
            .map(|v| v.is_precise())
            .unwrap_or(false);

        let non_empty = non_empty_trimmed(values);

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
                if has_precise_validation && fail_rate <= 0.3 {
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
        let non_empty = non_empty_trimmed(values);
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

    let non_empty = non_empty_trimmed(values);
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
        "representation.identifier.alphanumeric_id".to_string()
    } else {
        "representation.text.word".to_string()
    }
}
