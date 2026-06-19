//! Deterministic datetime format detection (spec 2026-06-19-deterministic-datetime-parser).
//!
//! A delimited datetime string resolves to exactly ONE taxonomy leaf by its shape and
//! field ranges — there is nothing to learn. The Sense model, a flat softmax over 240
//! labels, instead *guesses* between near-identical datetime sub-formats (iso_8601 vs
//! iso_8601_milliseconds vs sql_standard vs rfc_3339) and is routinely wrong on the
//! sub-leaf even when it correctly sees "this is a timestamp". This module replaces that
//! guess with a deterministic value-based reading.
//!
//! ## Why exact-anchored regexes, not optional suffixes
//!
//! Every format below is anchored `^…$` with EXACT field widths and no optional trailing
//! groups. Consequence: a value matches AT MOST ONE format (a `…:00.123` millisecond
//! value matches only the millis regex, never the plain-second one). So there is no
//! specificity-ordering hazard — detection is simply "which single format does ≥95% of
//! the column match". A genuinely mixed column (some millis, some not) matches no single
//! format at threshold and we assert nothing, leaving the model's guess intact. That is
//! the safe default: we only override when the values speak with one voice.
//!
//! ## Over-emission safety
//!
//! Delimited formats (separators, `T`, offsets) are unmistakably datetime — a string of
//! the shape `2020-01-03 14:22:09` with valid field ranges cannot be anything else, so
//! they are reported with `delimited = true` and the caller asserts unconditionally.
//! Bare-integer formats (epoch seconds/millis/…, a 4-digit year) are reported with
//! `delimited = false`: a 10-digit integer is honestly epoch-or-id-or-phone, so the
//! caller asserts those ONLY when the model already predicted a `datetime.*` leaf
//! (corroboration). This module reports the reading; the corroboration gate lives at the
//! call site.

use regex::Regex;
use std::sync::OnceLock;

/// A deterministic datetime reading of a column.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectedDatetime {
    /// The taxonomy leaf the values parse as (e.g. `datetime.timestamp.sql_standard`).
    pub leaf: &'static str,
    /// `true` when the format carries unambiguous datetime separators (delimiters / `T` /
    /// offset / letters). A delimited reading is safe to assert regardless of the model's
    /// prediction. `false` for bare-integer formats (epoch, year), which the caller must
    /// gate on a corroborating `datetime.*` model prediction.
    pub delimited: bool,
}

/// Fraction of non-null sample values that must match a single format for it to win.
const MATCH_THRESHOLD: f64 = 0.95;
/// Minimum non-null values required to make a deterministic call at all.
const MIN_VALUES: usize = 1;

/// A single exact-anchored datetime format: a structural regex plus a field-range check.
struct Format {
    leaf: &'static str,
    delimited: bool,
    re: Regex,
    /// Range validation over the regex captures. The regex guarantees shape; this
    /// rejects e.g. month `13` or hour `25`. Group indices are format-specific.
    check: fn(&regex::Captures) -> bool,
}

fn n(c: &regex::Captures, i: usize) -> u32 {
    c.get(i).and_then(|m| m.as_str().parse().ok()).unwrap_or(0)
}
fn ok_month(m: u32) -> bool {
    (1..=12).contains(&m)
}
fn ok_day(d: u32) -> bool {
    (1..=31).contains(&d)
}
fn ok_hour(h: u32) -> bool {
    h <= 23
}
fn ok_minsec(x: u32) -> bool {
    x <= 60 // allow leap second 60
}
fn ok_year4(y: u32) -> bool {
    (1000..=9999).contains(&y)
}

// Field-range checks. Capture-group layout is documented per regex below.
fn chk_ymd(c: &regex::Captures) -> bool {
    ok_year4(n(c, 1)) && ok_month(n(c, 2)) && ok_day(n(c, 3))
}
fn chk_ymd_hms(c: &regex::Captures) -> bool {
    ok_year4(n(c, 1))
        && ok_month(n(c, 2))
        && ok_day(n(c, 3))
        && ok_hour(n(c, 4))
        && ok_minsec(n(c, 5))
        && ok_minsec(n(c, 6))
}
fn chk_ym(c: &regex::Captures) -> bool {
    ok_year4(n(c, 1)) && ok_month(n(c, 2))
}
fn chk_hms(c: &regex::Captures) -> bool {
    ok_hour(n(c, 1)) && ok_minsec(n(c, 2)) && ok_minsec(n(c, 3))
}
fn chk_hm(c: &regex::Captures) -> bool {
    ok_hour(n(c, 1)) && ok_minsec(n(c, 2))
}
fn chk_true(_c: &regex::Captures) -> bool {
    true
}

/// The ordered format table. Order is irrelevant for correctness (formats are mutually
/// exclusive per value) but groups related shapes for readability.
fn formats() -> &'static [Format] {
    static FORMATS: OnceLock<Vec<Format>> = OnceLock::new();
    FORMATS.get_or_init(|| {
        let f = |leaf, delimited, pat: &str, check: fn(&regex::Captures) -> bool| Format {
            leaf,
            delimited,
            re: Regex::new(pat).expect("static datetime regex compiles"),
            check,
        };
        let ymd = r"(\d{4})-(\d{2})-(\d{2})";
        let ymd_t = r"(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2}):(\d{2})";
        let ymd_sp = r"(\d{4})-(\d{2})-(\d{2}) (\d{2}):(\d{2}):(\d{2})";
        vec![
            // ── ISO-8601 with `T` separator (most specific suffixes first for reading;
            //    mutual exclusivity makes order non-load-bearing) ──
            f(
                "datetime.timestamp.iso_8601_micros_offset",
                true,
                &format!(r"^{ymd_t}\.\d{{6}}[+-]\d{{2}}:?\d{{2}}$"),
                chk_ymd_hms,
            ),
            f(
                "datetime.timestamp.iso_8601_millis_offset",
                true,
                &format!(r"^{ymd_t}\.\d{{3}}[+-]\d{{2}}:?\d{{2}}$"),
                chk_ymd_hms,
            ),
            f(
                "datetime.timestamp.iso_8601_offset",
                true,
                &format!(r"^{ymd_t}[+-]\d{{2}}:?\d{{2}}$"),
                chk_ymd_hms,
            ),
            f(
                "datetime.timestamp.iso_8601_microseconds",
                true,
                &format!(r"^{ymd_t}\.\d{{6}}Z?$"),
                chk_ymd_hms,
            ),
            f(
                "datetime.timestamp.iso_8601_milliseconds",
                true,
                &format!(r"^{ymd_t}\.\d{{3}}Z?$"),
                chk_ymd_hms,
            ),
            f(
                "datetime.timestamp.iso_8601",
                true,
                &format!(r"^{ymd_t}Z?$"),
                chk_ymd_hms,
            ),
            // ── SQL / space-separated ──
            f(
                "datetime.timestamp.iso_space_zulu",
                true,
                &format!(r"^{ymd_sp}Z$"),
                chk_ymd_hms,
            ),
            f(
                "datetime.timestamp.sql_microseconds_offset",
                true,
                &format!(r"^{ymd_sp}\.\d{{6}}[+-]\d{{2}}:?\d{{2}}$"),
                chk_ymd_hms,
            ),
            f(
                // rfc_3339 space form carries an offset; pg_short_offset is its
                // millisecond+short-offset sibling. Keep rfc_3339 for the plain
                // second-precision space-offset shape.
                "datetime.timestamp.rfc_3339",
                true,
                &format!(r"^{ymd_sp} ?[+-]\d{{2}}:?\d{{2}}$"),
                chk_ymd_hms,
            ),
            f(
                "datetime.timestamp.sql_microseconds",
                true,
                &format!(r"^{ymd_sp}\.\d{{6}}$"),
                chk_ymd_hms,
            ),
            f(
                "datetime.timestamp.sql_milliseconds",
                true,
                &format!(r"^{ymd_sp}\.\d{{3}}$"),
                chk_ymd_hms,
            ),
            f(
                "datetime.timestamp.sql_standard",
                true,
                &format!(r"^{ymd_sp}$"),
                chk_ymd_hms,
            ),
            // ── Slash / dot YMD (year unambiguously first) ──
            f(
                "datetime.timestamp.slash_ymd_24h",
                true,
                r"^(\d{4})/(\d{2})/(\d{2}) (\d{2}):(\d{2}):(\d{2})$",
                chk_ymd_hms,
            ),
            f(
                "datetime.timestamp.dot_ymd_24h",
                true,
                r"^(\d{4})\.(\d{2})\.(\d{2}) (\d{2}):(\d{2}):(\d{2})$",
                chk_ymd_hms,
            ),
            // ── Dates ──
            f("datetime.date.iso", true, &format!(r"^{ymd}$"), chk_ymd),
            f(
                "datetime.date.ymd_slash",
                true,
                r"^(\d{4})/(\d{2})/(\d{2})$",
                chk_ymd,
            ),
            f(
                "datetime.date.ymd_dot",
                true,
                r"^(\d{4})\.(\d{2})\.(\d{2})$",
                chk_ymd,
            ),
            f(
                "datetime.date.year_month",
                true,
                r"^(\d{4})-(\d{2})$",
                chk_ym,
            ),
            // ── Times ──
            f(
                "datetime.time.iso",
                true,
                r"^(\d{2}):(\d{2}):(\d{2})\.\d{1,9}$",
                chk_hms,
            ),
            f(
                "datetime.time.hms_24h",
                true,
                r"^(\d{1,2}):(\d{2}):(\d{2})$",
                chk_hms,
            ),
            f("datetime.time.hm_24h", true, r"^(\d{2}):(\d{2})$", chk_hm),
            // ── Bare integers (NOT delimited — caller gates on a datetime.* model call) ──
            f("datetime.epoch.unix_seconds", false, r"^\d{10}$", chk_true),
            f(
                "datetime.epoch.unix_milliseconds",
                false,
                r"^\d{13}$",
                chk_true,
            ),
            f(
                "datetime.epoch.unix_microseconds",
                false,
                r"^\d{16}$",
                chk_true,
            ),
            f(
                "datetime.timestamp.epoch_nanoseconds",
                false,
                r"^\d{19}$",
                chk_true,
            ),
            f("datetime.component.year", false, r"^(\d{4})$", |c| {
                // A plausible calendar year, not any 4-digit integer.
                (1700..=2200).contains(&n(c, 1))
            }),
        ]
    })
}

/// Does this single value match this exact format (shape + field ranges)?
fn matches(fmt: &Format, value: &str) -> bool {
    match fmt.re.captures(value) {
        Some(caps) => (fmt.check)(&caps),
        None => false,
    }
}

/// The locale-ambiguous `dd/mm/yyyy` ↔ `mm/dd/yyyy` (and dash) families. Structurally one
/// shape; the leaf is decided by which positional field exceeds 12 ACROSS the column.
/// Returns `None` when undecidable (every value's first two fields ≤ 12) or contradictory
/// (evidence for both) — we never guess locale.
struct AmbiguousFamily {
    re: Regex,
    /// leaf when field 1 is the day (day-first)
    day_first_leaf: &'static str,
    /// leaf when field 2 is the day (month-first)
    month_first_leaf: &'static str,
}

fn ambiguous_families() -> &'static [AmbiguousFamily] {
    static FAMS: OnceLock<Vec<AmbiguousFamily>> = OnceLock::new();
    FAMS.get_or_init(|| {
        vec![
            AmbiguousFamily {
                re: Regex::new(r"^(\d{2})/(\d{2})/(\d{4})$").unwrap(),
                day_first_leaf: "datetime.date.dmy_slash",
                month_first_leaf: "datetime.date.mdy_slash",
            },
            AmbiguousFamily {
                re: Regex::new(r"^(\d{2})-(\d{2})-(\d{4})$").unwrap(),
                day_first_leaf: "datetime.date.dmy_dash",
                month_first_leaf: "datetime.date.mdy_dash",
            },
        ]
    })
}

/// Resolve an ambiguous family over a whole column. Returns the leaf when ≥ threshold of
/// values fit the shape AND the column carries one-directional disambiguating evidence.
fn resolve_ambiguous(fam: &AmbiguousFamily, values: &[&str]) -> Option<&'static str> {
    let mut matched = 0usize;
    let mut day_first = false; // saw field1 in 13..=31 (and field2 a valid month)
    let mut month_first = false; // saw field2 in 13..=31 (and field1 a valid month)
    for v in values {
        let caps = match fam.re.captures(v) {
            Some(c) => c,
            None => continue,
        };
        let f1 = n(&caps, 1);
        let f2 = n(&caps, 2);
        let y = n(&caps, 3);
        // Both fields must be a plausible day or month, year plausible.
        if !ok_year4(y) || !(1..=31).contains(&f1) || !(1..=31).contains(&f2) {
            continue;
        }
        matched += 1;
        if f1 > 12 && f2 <= 12 {
            day_first = true;
        }
        if f2 > 12 && f1 <= 12 {
            month_first = true;
        }
    }
    if values.is_empty() || (matched as f64) < (values.len() as f64) * MATCH_THRESHOLD {
        return None;
    }
    match (day_first, month_first) {
        (true, false) => Some(fam.day_first_leaf),
        (false, true) => Some(fam.month_first_leaf),
        // Undecidable (no field > 12) or contradictory (both) — never guess locale.
        _ => None,
    }
}

/// Deterministically read a column's datetime format from its sample values.
///
/// Returns `Some` when ≥95% of the non-null values parse as exactly one format. Delimited
/// readings (`delimited = true`) are safe to assert unconditionally; bare-integer readings
/// (`delimited = false`) must be gated by the caller on a corroborating `datetime.*` model
/// prediction. Returns `None` for empty, mixed, or non-datetime columns.
pub fn detect_datetime_format<S: AsRef<str>>(samples: &[S]) -> Option<DetectedDatetime> {
    let values: Vec<&str> = samples
        .iter()
        .map(|s| s.as_ref().trim())
        .filter(|s| !s.is_empty())
        .collect();
    if values.len() < MIN_VALUES {
        return None;
    }
    let total = values.len() as f64;

    // Pass 1: exact, per-value-unambiguous formats. Each value matches at most one, so the
    // winner is simply the format the most values match, provided it clears threshold.
    let mut best: Option<(&Format, usize)> = None;
    for fmt in formats() {
        let hits = values.iter().filter(|v| matches(fmt, v)).count();
        if hits as f64 >= total * MATCH_THRESHOLD && best.map(|(_, b)| hits > b).unwrap_or(true) {
            best = Some((fmt, hits));
        }
    }
    if let Some((fmt, _)) = best {
        return Some(DetectedDatetime {
            leaf: fmt.leaf,
            delimited: fmt.delimited,
        });
    }

    // Pass 2: locale-ambiguous slash/dash families, resolved by column-wide field ranges.
    for fam in ambiguous_families() {
        if let Some(leaf) = resolve_ambiguous(fam, &values) {
            return Some(DetectedDatetime {
                leaf,
                delimited: true,
            });
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn det(vals: &[&str]) -> Option<DetectedDatetime> {
        detect_datetime_format(vals)
    }
    fn leaf(vals: &[&str]) -> Option<&'static str> {
        det(vals).map(|d| d.leaf)
    }

    #[test]
    fn sql_standard_timestamp() {
        let d = det(&["2020-01-03 14:22:09", "1999-12-31 00:00:00"]).unwrap();
        assert_eq!(d.leaf, "datetime.timestamp.sql_standard");
        assert!(d.delimited);
    }

    #[test]
    fn millis_beats_plain_iso_no_overlap() {
        // A millisecond value matches ONLY the millis format, never plain iso_8601.
        assert_eq!(
            leaf(&["2020-01-03T14:22:09.123", "2021-06-01T00:00:00.000"]),
            Some("datetime.timestamp.iso_8601_milliseconds")
        );
        assert_eq!(
            leaf(&["2020-01-03T14:22:09", "2021-06-01T00:00:00"]),
            Some("datetime.timestamp.iso_8601")
        );
    }

    #[test]
    fn iso_offset_and_micros() {
        assert_eq!(
            leaf(&["2020-01-03T14:22:09+02:00"]),
            Some("datetime.timestamp.iso_8601_offset")
        );
        assert_eq!(
            leaf(&["2020-01-03T14:22:09.123456"]),
            Some("datetime.timestamp.iso_8601_microseconds")
        );
        assert_eq!(
            leaf(&["2020-01-03T14:22:09.123456+0200"]),
            Some("datetime.timestamp.iso_8601_micros_offset")
        );
    }

    #[test]
    fn iso_date_and_year_month() {
        assert_eq!(
            leaf(&["2020-01-03", "1999-12-31"]),
            Some("datetime.date.iso")
        );
        assert_eq!(
            leaf(&["2020-01", "1999-12"]),
            Some("datetime.date.year_month")
        );
    }

    #[test]
    fn range_validation_rejects_impossible_fields() {
        // Month 13 / day 45 / hour 25 — shape matches but ranges fail, so NOT datetime.
        assert_eq!(det(&["2020-13-03"]), None);
        assert_eq!(det(&["2020-01-45"]), None);
        assert_eq!(det(&["2020-01-03 25:00:00"]), None);
    }

    #[test]
    fn mixed_column_asserts_nothing() {
        // Some millis, some plain — no single format clears threshold.
        assert_eq!(
            det(&["2020-01-03T14:22:09.123", "2020-01-03T14:22:09"]),
            None
        );
    }

    #[test]
    fn dmy_slash_resolved_by_day_over_12() {
        // 31 in field 1 → day-first → dmy.
        assert_eq!(
            leaf(&["31/01/2020", "05/06/2019"]),
            Some("datetime.date.dmy_slash")
        );
    }

    #[test]
    fn mdy_slash_resolved_by_day_in_field_2() {
        // 31 in field 2 → day-second → mdy.
        assert_eq!(
            leaf(&["01/31/2020", "06/05/2019"]),
            Some("datetime.date.mdy_slash")
        );
    }

    #[test]
    fn undecidable_slash_locale_asserts_nothing() {
        // Every field ≤ 12 — genuinely ambiguous, we must not guess.
        assert_eq!(det(&["01/02/2020", "03/04/2019"]), None);
    }

    #[test]
    fn contradictory_slash_locale_asserts_nothing() {
        // Evidence for BOTH day-first and month-first → refuse.
        assert_eq!(det(&["31/01/2020", "01/31/2020"]), None);
    }

    #[test]
    fn bare_epoch_is_not_delimited() {
        let d = det(&["1609459200", "1612137600"]).unwrap();
        assert_eq!(d.leaf, "datetime.epoch.unix_seconds");
        assert!(!d.delimited, "bare integer must be flagged non-delimited");
    }

    #[test]
    fn bare_year_is_not_delimited_and_range_checked() {
        let d = det(&["1999", "2020", "1850"]).unwrap();
        assert_eq!(d.leaf, "datetime.component.year");
        assert!(!d.delimited);
        // 9999 is a 4-digit integer but not a plausible year.
        assert_eq!(det(&["9999", "8888"]), None);
    }

    #[test]
    fn negative_control_alphanumeric_id_is_not_datetime() {
        assert_eq!(det(&["A1B2C3", "X9Y8Z7", "Q4R5S6"]), None);
    }

    #[test]
    fn negative_control_decimal_is_not_datetime() {
        assert_eq!(det(&["3.14159", "2.71828", "1.41421"]), None);
    }

    #[test]
    fn empty_and_blank_columns() {
        assert_eq!(det(&[]), None);
        assert_eq!(det(&["", "  ", "\t"]), None);
    }

    #[test]
    fn time_only_formats() {
        assert_eq!(
            leaf(&["14:22:09", "00:00:01"]),
            Some("datetime.time.hms_24h")
        );
        assert_eq!(leaf(&["14:22", "00:00"]), Some("datetime.time.hm_24h"));
    }

    /// Every leaf the detector can emit must be a real taxonomy leaf, and a positive
    /// example must also pass that leaf's taxonomy regex — keeps this module and the
    /// taxonomy a single source of truth, so a taxonomy rename can't silently desync.
    #[test]
    fn all_emitted_leaves_exist_in_taxonomy() {
        use crate::Taxonomy;
        use std::path::PathBuf;
        let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let labels = loop {
            let l = dir.join("labels");
            if l.is_dir() {
                break l;
            }
            assert!(
                dir.pop(),
                "could not locate labels/ from CARGO_MANIFEST_DIR"
            );
        };
        let tax = Taxonomy::from_directory(labels).expect("load taxonomy");
        let mut leaves: Vec<&'static str> = formats().iter().map(|f| f.leaf).collect();
        for fam in ambiguous_families() {
            leaves.push(fam.day_first_leaf);
            leaves.push(fam.month_first_leaf);
        }
        for leaf in leaves {
            assert!(
                tax.get(leaf).is_some(),
                "detector emits leaf `{leaf}` absent from the taxonomy"
            );
        }
    }
}
