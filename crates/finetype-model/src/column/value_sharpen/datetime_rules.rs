use super::*;

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

    let non_empty = non_empty_lower(values);

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

    let non_empty = non_empty_lower(values);

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

    let non_empty = non_empty_trimmed(values);

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
    let non_empty = non_empty_trimmed(values);

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
    let non_empty = non_empty_trimmed(values);

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
