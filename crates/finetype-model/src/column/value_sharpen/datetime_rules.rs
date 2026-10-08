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
// ═══════════════════════════════════════════════════════════════════════════════
// Value-shape datetime recoveries — from a single value up
// ═══════════════════════════════════════════════════════════════════════════════
//
// The model reads a one-value column (`finetype infer -i`, `ft_infer(v)`) with no
// sibling values to correct it, and every value rule above wants three or more
// values before it fires. So `PT30M` stayed `alphanumeric_id`, `America/New_York`
// stayed `continent`, a Common Log Format timestamp stayed `plain_text` and
// `04-03-24` stayed `hm_24h` — four datetime values read outside the domain.
//
// Each recovery below asserts a leaf only on a shape no other taxonomy leaf
// writes, and each shape sits strictly inside its leaf's own validator, so the
// downstream validation veto never rejects what one asserts.

/// Fraction of the non-empty values a value-shape recovery needs before it asserts
/// its leaf: the bar `label_validates_sample` holds a leaf's own validator to.
const SHAPE_RECOVERY_BAR: f64 = 0.9;

/// True when there is at least one non-empty value and at least 90% of the
/// non-empty values satisfy `is_shape`. No minimum column size, on purpose: the
/// shapes are self-precise, and a one-value column is the case these exist for.
fn shape_holds(values: &[String], is_shape: impl Fn(&str) -> bool) -> bool {
    let non_empty = non_empty_trimmed(values);
    if non_empty.is_empty() {
        return false;
    }
    let hits = non_empty.iter().filter(|v| is_shape(v)).count();
    hits as f64 >= non_empty.len() as f64 * SHAPE_RECOVERY_BAR
}

/// An ISO 8601 duration under the strict grammar
/// `[-]P[nY][nM][nW|nD][T[nH][nM][n[.n]S]]`: at least one component, and a `T`
/// only when a time component follows it. Strictly inside the
/// `datetime.duration.iso_8601` validator (which also admits a bare trailing `T`).
pub(crate) fn is_iso_8601_duration(v: &str) -> bool {
    use std::sync::OnceLock;
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    let re = RE.get_or_init(|| {
        regex::Regex::new(
            r"^-?P(?:(\d+)Y)?(?:(\d+)M)?(?:(\d+)[WD])?(T(?:(\d+)H)?(?:(\d+)M)?(?:(\d+(?:\.\d+)?)S)?)?$",
        )
        .expect("iso 8601 duration regex")
    });
    let Some(c) = re.captures(v) else {
        return false;
    };
    let has_date = (1..=3).any(|i| c.get(i).is_some());
    let has_time = (5..=7).any(|i| c.get(i).is_some());
    (has_date || has_time) && (c.get(4).is_none() || has_time)
}

/// An IANA tz database zone name: one of the database's top-level areas, then a
/// capitalised location, optionally a second (`America/Argentina/Buenos_Aires`).
/// The area list is closed — the database has used these names throughout — so
/// `Region/City` text in general (`Sales/North`) is not admitted. Strictly inside
/// the `datetime.offset.iana` validator.
pub(crate) fn is_iana_zone_name(v: &str) -> bool {
    use std::sync::OnceLock;
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    let re = RE.get_or_init(|| {
        regex::Regex::new(concat!(
            r"^(?:Africa|America|Antarctica|Arctic|Asia|Atlantic|Australia|Europe|",
            r"Indian|Pacific|Etc)/[A-Z][A-Za-z_]*(?:/[A-Z][A-Za-z_]*)?$",
        ))
        .expect("iana zone regex")
    });
    re.is_match(v)
}

/// A Common Log Format timestamp, `dd/Mon/yyyy:HH:MM:SS ±zz`, bracketed or not,
/// with the offset in any form DuckDB's `%z` parses: `+00` (the form its
/// `strftime` writes), `+0000` or `+00:00`. The colon joining date to time and the
/// month abbreviation are the shape nothing else writes. Strictly inside the
/// `datetime.timestamp.clf` validator.
pub(crate) fn is_clf_timestamp(v: &str) -> bool {
    use std::sync::OnceLock;
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    let re = RE.get_or_init(|| {
        regex::Regex::new(concat!(
            r"^(\[?)(\d{2})/(?:Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)/\d{4}:",
            r"(\d{2}):(\d{2}):(\d{2}) [+-](\d{2})(?::?(\d{2}))?(\]?)$",
        ))
        .expect("clf timestamp regex")
    });
    let Some(c) = re.captures(v) else {
        return false;
    };
    let n = |i: usize| -> u32 { c.get(i).and_then(|m| m.as_str().parse().ok()).unwrap_or(0) };
    let bracketed = !c[1].is_empty();
    bracketed == !c[8].is_empty()
        && (1..=31).contains(&n(2))
        && n(3) <= 23
        && n(4) <= 59
        && n(5) <= 60
        && n(6) <= 23
        && n(7) <= 59
}

/// The three readings of a two-digit-year `NN-NN-NN` date, in the order DuckDB's
/// CSV sniffer prefers them when more than one parses: `%y-%m-%d`, then
/// `%d-%m-%y`, then `%m-%d-%y`. Each entry is the leaf and the positions of its
/// month and day fields.
const SHORT_DATE_READINGS: [(&str, usize, usize); 3] = [
    ("datetime.date.short_ymd", 1, 2),
    ("datetime.date.short_dmy", 1, 0),
    ("datetime.date.short_mdy", 0, 1),
];

/// The short-date leaf a column of `NN-NN-NN` values reads as: the first reading
/// in [`SHORT_DATE_READINGS`] under which every such value is a valid date, so the
/// leaf is the one DuckDB itself reads the column as. `None` when fewer than 90%
/// of the non-empty values have the shape, or no reading parses them all.
pub(crate) fn short_date_reading(values: &[String]) -> Option<&'static str> {
    use std::sync::OnceLock;
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    let re = RE
        .get_or_init(|| regex::Regex::new(r"^(\d{2})-(\d{2})-(\d{2})$").expect("short date regex"));
    let non_empty = non_empty_trimmed(values);
    let fields: Vec<[u32; 3]> = non_empty
        .iter()
        .filter_map(|v| {
            let c = re.captures(v)?;
            Some([1, 2, 3].map(|i| c[i].parse().unwrap_or(0)))
        })
        .collect();
    if fields.is_empty() || (fields.len() as f64) < non_empty.len() as f64 * SHAPE_RECOVERY_BAR {
        return None;
    }
    SHORT_DATE_READINGS
        .iter()
        .find(|(_, month, day)| {
            fields
                .iter()
                .all(|f| (1..=12).contains(&f[*month]) && (1..=31).contains(&f[*day]))
        })
        .map(|(leaf, _, _)| *leaf)
}

/// `iso_duration_recovery` (default ON): ISO 8601 durations the model read as an
/// identifier (`PT30M` → `alphanumeric_id`). Fires on any label outside
/// `datetime.duration`. Rule 14 below still catches the looser SOTAB variants
/// (`PD1TH0M0`) on the `sedol` attractor.
pub(crate) fn iso_duration_recovery(values: &[String], label: &str) -> Option<(String, String)> {
    if label.starts_with("datetime.duration.") || crate::rhh::is_disabled("iso_duration_recovery") {
        return None;
    }
    shape_holds(values, is_iso_8601_duration).then(|| {
        (
            "datetime.duration.iso_8601".to_string(),
            "iso_duration_recovery".to_string(),
        )
    })
}

/// `iana_zone_recovery` (default ON): IANA zone names the model read as a place
/// (`America/New_York` → `continent`). Fires on any label but the leaf itself.
pub(crate) fn iana_zone_recovery(values: &[String], label: &str) -> Option<(String, String)> {
    if label == "datetime.offset.iana" || crate::rhh::is_disabled("iana_zone_recovery") {
        return None;
    }
    shape_holds(values, is_iana_zone_name).then(|| {
        (
            "datetime.offset.iana".to_string(),
            "iana_zone_recovery".to_string(),
        )
    })
}

/// `clf_timestamp_recovery` (default ON): Common Log Format timestamps the model
/// read as text, or as another timestamp leaf (a column of them reads
/// `iso_8601_offset`). Fires on any label but the leaf itself.
pub(crate) fn clf_timestamp_recovery(values: &[String], label: &str) -> Option<(String, String)> {
    if label == "datetime.timestamp.clf" || crate::rhh::is_disabled("clf_timestamp_recovery") {
        return None;
    }
    shape_holds(values, is_clf_timestamp).then(|| {
        (
            "datetime.timestamp.clf".to_string(),
            "clf_timestamp_recovery".to_string(),
        )
    })
}

/// `short_date_recovery` (default ON): two-digit-year dates the model read outside
/// the date category (`04-03-24` → `hm_24h`). Fires on any label outside
/// `datetime.date`; a short-date label already there is left to Rule 2, which
/// settles day-first against month-first from the column's own evidence.
pub(crate) fn short_date_recovery(values: &[String], label: &str) -> Option<(String, String)> {
    if label.starts_with("datetime.date.") || crate::rhh::is_disabled("short_date_recovery") {
        return None;
    }
    short_date_reading(values).map(|leaf| (leaf.to_string(), "short_date_recovery".to_string()))
}
