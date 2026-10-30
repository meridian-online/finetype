//! The `display` blocks on number definitions match the design's table, and
//! the table covers every definition the classifier calls a number. Every
//! temporal definition's `time` block writes its values the way the label's
//! source writes them.

use chrono::format::{parse, Parsed, StrftimeItems};
use chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime, Weekday};
use finetype_core::display::{display_kind_for, DisplayBlock, DisplayKind};
use finetype_core::Taxonomy;
use std::path::Path;

/// (key, number specifier, flags, fixed currency).
type Row = (
    &'static str,
    &'static str,
    &'static [&'static str],
    Option<&'static str>,
);

const MONEY: &[&str] = &["money"];
const FRACTION: &[&str] = &["fraction"];
const NONE: &[&str] = &[];

const NUMBER_TABLE: &[Row] = &[
    ("finance.currency.amount", ",.2f", MONEY, None),
    ("finance.currency.amount_comma", ",.2f", MONEY, None),
    ("finance.currency.amount_comma_suffix", ",.2f", MONEY, None),
    ("finance.currency.amount_space", ",.2f", MONEY, None),
    ("finance.currency.amount_code_prefix", ",.2f", MONEY, None),
    ("finance.currency.amount_multisym", ",.2f", MONEY, None),
    ("finance.currency.amount_neg_trailing", ",.2f", MONEY, None),
    ("finance.currency.amount_accounting", "(,.2f", MONEY, None),
    ("finance.currency.amount_lakh", ",.2f", MONEY, Some("INR")),
    (
        "finance.currency.amount_apostrophe",
        ",.2f",
        MONEY,
        Some("CHF"),
    ),
    ("finance.currency.amount_nodecimal", ",d", MONEY, None),
    ("finance.currency.amount_crypto", ",.8~f", MONEY, None),
    ("finance.rate.yield", "~%", FRACTION, None),
    ("finance.rate.basis_points", "~%", FRACTION, None),
    ("representation.numeric.percentage", "~%", FRACTION, None),
    ("representation.numeric.integer_number", ",d", NONE, None),
    ("representation.numeric.decimal_number", ",", NONE, None),
    (
        "representation.numeric.decimal_number_comma",
        ",",
        NONE,
        None,
    ),
    (
        "representation.numeric.scientific_notation",
        "~e",
        NONE,
        None,
    ),
    ("representation.numeric.si_number", "~s", NONE, None),
    ("representation.file.file_size", ",d", &["bytes"], None),
    ("representation.identifier.increment", "d", NONE, None),
    ("geography.coordinate.latitude", "~f", NONE, None),
    ("geography.coordinate.longitude", "~f", NONE, None),
    ("identity.person.height", ",~f", NONE, None),
    ("identity.person.weight", ",~f", NONE, None),
    ("datetime.component.year", "d", NONE, None),
];

fn taxonomy() -> Taxonomy {
    Taxonomy::from_directory(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../labels"))
        .expect("load taxonomy")
}

#[test]
fn number_blocks_match_the_design() {
    let t = taxonomy();
    assert_eq!(NUMBER_TABLE.len(), 27);
    for (key, number, flags, currency) in NUMBER_TABLE {
        let def = t
            .get(key)
            .unwrap_or_else(|| panic!("{key} not in taxonomy"));
        let expected = DisplayBlock {
            number: Some((*number).to_string()),
            money: flags.contains(&"money"),
            fraction: flags.contains(&"fraction"),
            bytes: flags.contains(&"bytes"),
            currency: currency.map(str::to_string),
            ..DisplayBlock::default()
        };
        assert_eq!(def.display.as_ref(), Some(&expected), "{key}");
    }
}

#[test]
fn every_number_definition_is_in_the_table() {
    let t = taxonomy();
    let missing: Vec<&String> = t
        .definitions()
        .filter(|(k, d)| display_kind_for(k, d) == Some(DisplayKind::Number))
        .map(|(k, _)| k)
        .filter(|k| !NUMBER_TABLE.iter().any(|(tk, ..)| tk == k))
        .collect();
    assert!(
        missing.is_empty(),
        "number definitions with no row: {missing:?}"
    );
}

/// Temporal definitions the design shows in another form than their source,
/// or that have no `format_string` to translate, with the exact specifier.
const TIME_EXCEPTIONS: &[(&str, &str)] = &[
    ("datetime.date.compact_ymd", "%Y-%m-%d"),
    ("datetime.date.compact_dmy", "%Y-%m-%d"),
    ("datetime.date.compact_mdy", "%Y-%m-%d"),
    ("datetime.date.compact_ym", "%Y-%m"),
    ("datetime.timestamp.iso_8601_compact", "%Y-%m-%dT%H:%M:%S"),
    ("datetime.epoch.unix_seconds", "%Y-%m-%d %H:%M:%S"),
    ("datetime.epoch.unix_milliseconds", "%Y-%m-%d %H:%M:%S"),
    ("datetime.epoch.unix_microseconds", "%Y-%m-%d %H:%M:%S"),
    ("datetime.timestamp.epoch_nanoseconds", "%Y-%m-%d %H:%M:%S"),
    (
        "datetime.timestamp.rfc_2822_ordinal",
        "%d %b %Y %H:%M:%S %Z",
    ),
    ("datetime.date.jp_era_short", "%Y-%m-%d"),
    ("datetime.date.jp_era_long", "%Y-%m-%d"),
    ("datetime.date.julian", "%y-%j"),
    ("datetime.date.chinese_ymd", "%Y年%-m月%-d日"),
    ("datetime.date.korean_ymd", "%Y년 %-m월 %-d일"),
    ("datetime.period.quarter", "Q%q %Y"),
    ("technology.identifier.snowflake_id", "%Y-%m-%d %H:%M:%S.%L"),
];

/// Mirrored definitions whose sample cannot round-trip, each with its reason.
const ROUND_TRIP_EXEMPT: &[(&str, &str)] = &[
    (
        "datetime.timestamp.iso_8601",
        "source writes a literal Z; the specifier writes the offset",
    ),
    (
        "datetime.timestamp.iso_8601_microseconds",
        "source writes a literal Z; the specifier writes the offset",
    ),
    (
        "datetime.timestamp.iso_8601_milliseconds",
        "source writes a literal Z; the specifier writes the offset",
    ),
    (
        "datetime.timestamp.iso_space_zulu",
        "source writes a literal Z; the specifier writes the offset",
    ),
];

/// Rewrite each `%` directive of `fmt` through `map`, which gets the
/// directive letter and returns its replacement (without the `%`). Literal
/// text and any `-`, `_`, `0` or `#` modifier are kept.
fn rewrite(fmt: &str, map: impl Fn(char) -> String) -> String {
    let mut out = String::new();
    let mut chars = fmt.chars();
    while let Some(c) = chars.next() {
        if c != '%' {
            out.push(c);
            continue;
        }
        out.push('%');
        let mut d = chars.next().expect("lone % in format");
        if matches!(d, '-' | '_' | '0' | '#') {
            out.push(d);
            d = chars.next().expect("modifier with no directive");
        }
        out.push_str(&map(d));
    }
    out
}

/// The design's mechanical translation of a DuckDB strptime format into
/// d3-time-format: `%g` becomes `%L`, `%z` and a literal trailing `Z` become
/// `%Z`, and every other directive is unchanged.
fn mirror(format_string: &str) -> String {
    let (body, zulu) = match format_string.strip_suffix('Z') {
        Some(b) if !b.ends_with('%') => (b, true),
        _ => (format_string, false),
    };
    let mut out = rewrite(body, |d| match d {
        'g' => "L".into(),
        'z' => "Z".into(),
        d => d.to_string(),
    });
    if zulu {
        out.push_str("%Z");
    }
    out
}

/// A DuckDB strptime format in chrono's dialect. DuckDB `%g` is exactly three
/// fraction digits and `%f` exactly six; chrono's bare `%f` reads up to nine
/// digits as nanoseconds, so they map to chrono's fixed-width `%3f` and `%6f`.
/// DuckDB's `%z` takes `+05`, `+0500` and `+05:00`, which is chrono's `%#z`.
fn duckdb_to_chrono(format_string: &str) -> String {
    rewrite(format_string, |d| match d {
        'g' => "3f".into(),
        'f' => "6f".into(),
        'z' => "#z".into(),
        d => d.to_string(),
    })
}

/// A d3-time-format specifier in chrono's dialect, for formatting: `%L`
/// milliseconds is `%3f`, `%f` microseconds is `%6f`, and `%Z` (an offset such
/// as `-0700`) is `%z`.
fn d3_to_chrono(spec: &str) -> String {
    rewrite(spec, |d| match d {
        'L' => "3f".into(),
        'f' => "6f".into(),
        'Z' => "z".into(),
        d => d.to_string(),
    })
}

/// Parse `s` with a chrono format into a moment and its offset. A format with
/// no day of the month names a month (or an ISO week), so the value is taken
/// at its first day; a format with no date or no time takes 1970-01-01 or
/// midnight, which the specifier, mirroring the same directives, never writes.
fn parse_moment(s: &str, chrono_fmt: &str) -> Result<(NaiveDateTime, i32), String> {
    let mut p = Parsed::new();
    parse(&mut p, s, StrftimeItems::new(chrono_fmt)).map_err(|e| format!("{s:?}: {e}"))?;
    let has_date = p.year().is_some()
        || p.year_mod_100().is_some()
        || p.isoyear().is_some()
        || p.month().is_some();
    let date = if has_date {
        if p.isoweek().is_some() && p.weekday().is_none() {
            p.set_weekday(Weekday::Mon).map_err(|e| e.to_string())?;
        }
        if p.month().is_some() && p.day().is_none() {
            p.set_day(1).map_err(|e| e.to_string())?;
        }
        p.to_naive_date().map_err(|e| format!("{s:?}: date: {e}"))?
    } else {
        NaiveDate::from_ymd_opt(1970, 1, 1).expect("epoch")
    };
    let time = if p.hour_mod_12().is_some() || p.minute().is_some() {
        p.to_naive_time().map_err(|e| format!("{s:?}: time: {e}"))?
    } else {
        NaiveTime::MIN
    };
    Ok((date.and_time(time), p.offset().unwrap_or(0)))
}

#[test]
fn every_temporal_definition_has_a_time_block() {
    let t = taxonomy();
    let temporal: Vec<&String> = t
        .definitions()
        .filter(|(k, d)| display_kind_for(k, d) == Some(DisplayKind::Time))
        .map(|(k, _)| k)
        .collect();
    assert_eq!(temporal.len(), 80, "temporal definitions: {temporal:?}");
    let missing: Vec<&&String> = temporal
        .iter()
        .filter(|k| {
            t.get(k)
                .and_then(|d| d.display.as_ref())
                .and_then(|d| d.time.as_ref())
                .is_none()
        })
        .collect();
    assert!(
        missing.is_empty(),
        "temporal definitions with no time block: {missing:?}"
    );
}

#[test]
fn time_blocks_follow_the_source_form() {
    let t = taxonomy();
    let mut wrong = Vec::new();
    for (key, def) in t.definitions() {
        if display_kind_for(key, def) != Some(DisplayKind::Time) {
            continue;
        }
        let expected = match TIME_EXCEPTIONS.iter().find(|(k, _)| k == key) {
            Some((_, spec)) => (*spec).to_string(),
            None => match &def.format_string {
                Some(f) => mirror(f),
                None => {
                    wrong.push(format!("{key}: no format_string and no exception"));
                    continue;
                }
            },
        };
        let actual = def.display.as_ref().and_then(|d| d.time.as_deref());
        if actual != Some(expected.as_str()) {
            wrong.push(format!("{key}: expected {expected:?}, found {actual:?}"));
        }
    }
    assert!(
        wrong.is_empty(),
        "time blocks off the source form:\n{}",
        wrong.join("\n")
    );
}

#[test]
fn time_specifiers_round_trip() {
    let t = taxonomy();
    let mut failures = Vec::new();
    let mut checked = 0;
    for (key, def) in t.definitions() {
        if display_kind_for(key, def) != Some(DisplayKind::Time)
            || TIME_EXCEPTIONS.iter().any(|(k, _)| k == key)
            || ROUND_TRIP_EXEMPT.iter().any(|(k, _)| k == key)
        {
            continue;
        }
        let Some(source) = &def.format_string else {
            failures.push(format!("{key}: no format_string"));
            continue;
        };
        let Some(spec) = def.display.as_ref().and_then(|d| d.time.as_deref()) else {
            failures.push(format!("{key}: no time block"));
            continue;
        };
        let Some(sample) = def.samples.first().and_then(|v| v.as_str()) else {
            failures.push(format!("{key}: no string sample"));
            continue;
        };
        let source = duckdb_to_chrono(source);
        let result = parse_moment(sample, &source).and_then(|(naive, offset)| {
            let tz = FixedOffset::east_opt(offset).ok_or("offset out of range")?;
            let moment = DateTime::<FixedOffset>::from_naive_utc_and_offset(naive - tz, tz);
            let shown = moment.format(&d3_to_chrono(spec)).to_string();
            let back = parse_moment(&shown, &source)?;
            if back == (naive, offset) {
                Ok(())
            } else {
                Err(format!(
                    "{sample:?} shown as {shown:?} reads back as {back:?}, not {:?}",
                    (naive, offset)
                ))
            }
        });
        match result {
            Ok(()) => checked += 1,
            Err(e) => failures.push(format!("{key}: {e}")),
        }
    }
    assert!(
        failures.is_empty(),
        "time specifiers that do not round-trip:\n{}",
        failures.join("\n")
    );
    assert_eq!(
        checked,
        80 - TIME_EXCEPTIONS.len() - ROUND_TRIP_EXEMPT.len()
    );
}
