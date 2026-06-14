//! Generators for the `datetime` domain.

use super::*;

impl Generator {
    // ═══════════════════════════════════════════════════════════════════════════
    // DOMAIN: datetime (46 types)
    // ═══════════════════════════════════════════════════════════════════════════

    pub(crate) fn gen_datetime(
        &mut self,
        category: &str,
        type_name: &str,
    ) -> Result<String, GeneratorError> {
        match (category, type_name) {
            // ── timestamp (12 types) ─────────────────────────────────────
            ("timestamp", "iso_8601") => Ok(self
                .random_datetime()
                .format("%Y-%m-%dT%H:%M:%SZ")
                .to_string()),
            ("timestamp", "iso_8601_compact") => {
                // v19: Compact ISO 8601 without separators: 20240327T183105
                // Must match validation pattern ^\d{8}T\d{6}$ exactly.
                // The 'T' separator between date and time digits is the
                // key structural signal that distinguishes this from
                // isbn/alphanumeric_id. Wider year range for variety.
                let dt = self.random_datetime();
                Ok(dt.format("%Y%m%dT%H%M%S").to_string())
            }
            ("timestamp", "iso_8601_microseconds") => {
                let dt = self.random_datetime();
                let micros = self.rng.gen_range(0..1000000);
                Ok(format!("{}.{:06}Z", dt.format("%Y-%m-%dT%H:%M:%S"), micros))
            }
            ("timestamp", "iso_8601_offset") => {
                let dt = self.random_datetime();
                let offset_h = self.rng.gen_range(-12i32..=12);
                Ok(format!(
                    "{}{:+03}:00",
                    dt.format("%Y-%m-%dT%H:%M:%S"),
                    offset_h
                ))
            }
            ("timestamp", "rfc_2822") => Ok(self
                .random_datetime()
                .format("%a, %d %b %Y %H:%M:%S +0000")
                .to_string()),
            ("timestamp", "rfc_2822_ordinal") => {
                let dt = self.random_datetime();
                let day = dt.day();
                let ord = match day % 10 {
                    1 if day != 11 => "st",
                    2 if day != 12 => "nd",
                    3 if day != 13 => "rd",
                    _ => "th",
                };
                Ok(format!(
                    "{}{} {} +0000",
                    day,
                    ord,
                    dt.format("%b %Y %H:%M:%S")
                ))
            }
            ("timestamp", "rfc_3339") => {
                // RFC 3339 uses SPACE separator (vs ISO 8601 which uses T)
                let dt = self.random_datetime();
                let offset_h = self.rng.gen_range(-12i32..=12);
                Ok(format!(
                    "{}{:+03}:00",
                    dt.format("%Y-%m-%d %H:%M:%S"),
                    offset_h
                ))
            }
            ("timestamp", "mdy_12h") => Ok(self
                .random_datetime()
                .format("%m/%d/%Y %I:%M %p")
                .to_string()),
            ("timestamp", "mdy_24h") => Ok(self
                .random_datetime()
                .format("%m/%d/%Y %H:%M:%S")
                .to_string()),
            ("timestamp", "dmy_hm") => {
                Ok(self.random_datetime().format("%d/%m/%Y %H:%M").to_string())
            }
            ("timestamp", "iso_microseconds") => {
                // v19: exactly 6 fractional digits, no timezone suffix.
                // Distinguished from iso_8601_milliseconds (3 digits + Z)
                // and numeric_code (no datetime separators at all).
                let dt = self.random_datetime();
                let micros = self.rng.gen_range(1..1000000u32);
                Ok(format!("{}.{:06}", dt.format("%Y-%m-%dT%H:%M:%S"), micros))
            }
            ("timestamp", "sql_standard") => Ok(self
                .random_datetime()
                .format("%Y-%m-%d %H:%M:%S")
                .to_string()),

            // ── new timestamp types (16 types) ──────────────────────────
            ("timestamp", "sql_microseconds") => {
                let dt = self.random_datetime();
                let micros = self.rng.gen_range(0..1_000_000u32);
                Ok(format!("{}.{:06}", dt.format("%Y-%m-%d %H:%M:%S"), micros))
            }
            ("timestamp", "sql_milliseconds") => {
                let dt = self.random_datetime();
                let millis = self.rng.gen_range(0..1000u32);
                Ok(format!("{}.{:03}", dt.format("%Y-%m-%d %H:%M:%S"), millis))
            }
            ("timestamp", "iso_8601_milliseconds") => {
                // v19: always has exactly 3 fractional digits + Z suffix.
                // Distinguished from iso_microseconds (6 digits) and
                // numeric_code (no datetime separators).
                let dt = self.random_datetime();
                let millis = self.rng.gen_range(0..1000u32);
                // Ensure non-zero millis for format distinctiveness
                let millis = if millis == 0 {
                    self.rng.gen_range(1..1000u32)
                } else {
                    millis
                };
                Ok(format!("{}.{:03}Z", dt.format("%Y-%m-%dT%H:%M:%S"), millis))
            }
            ("timestamp", "iso_8601_millis_offset") => {
                let dt = self.random_datetime();
                let millis = self.rng.gen_range(0..1000u32);
                let offset_h = self.rng.gen_range(-12i32..=12);
                let offset_m = if offset_h == 0 && self.rng.gen_bool(0.3) {
                    30
                } else {
                    0
                };
                Ok(format!(
                    "{}.{:03}{:+03}:{:02}",
                    dt.format("%Y-%m-%dT%H:%M:%S"),
                    millis,
                    offset_h,
                    offset_m
                ))
            }
            ("timestamp", "iso_8601_micros_offset") => {
                let dt = self.random_datetime();
                let micros = self.rng.gen_range(0..1_000_000u32);
                let offset_h = self.rng.gen_range(-12i32..=12);
                Ok(format!(
                    "{}.{:06}{:+03}:00",
                    dt.format("%Y-%m-%dT%H:%M:%S"),
                    micros,
                    offset_h
                ))
            }
            ("timestamp", "clf") => {
                // Apache/Nginx Common Log Format: 15/Jan/2024:14:30:00 +0000
                let dt = self.random_datetime();
                let offset_h = self.rng.gen_range(-12i32..=12);
                Ok(format!(
                    "{}:{} {:+03}00",
                    dt.format("%d/%b/%Y"),
                    dt.format("%H:%M:%S"),
                    offset_h
                ))
            }
            ("timestamp", "syslog_bsd") => {
                // RFC 3164 BSD syslog: Jan 15 14:30:00 (no year)
                let dt = self.random_datetime();
                let day = dt.day();
                // BSD syslog pads single-digit days with a space
                Ok(format!(
                    "{} {:>2} {}",
                    dt.format("%b"),
                    day,
                    dt.format("%H:%M:%S")
                ))
            }
            ("timestamp", "sql_microseconds_offset") => {
                let dt = self.random_datetime();
                let micros = self.rng.gen_range(0..1_000_000u32);
                let offset_h = self.rng.gen_range(-12i32..=12);
                Ok(format!(
                    "{}.{:06}{:+03}:00",
                    dt.format("%Y-%m-%d %H:%M:%S"),
                    micros,
                    offset_h
                ))
            }
            ("timestamp", "pg_short_offset") => {
                // v19: PostgreSQL 2-digit offset: 2024-01-15 14:30:00.123456-05
                // Distinguished from rfc_3339 (T separator + :00 colon offset)
                // by using space separator + short offset without colon.
                let dt = self.random_datetime();
                let micros = self.rng.gen_range(1..1_000_000u32);
                // Non-zero offsets help distinguish from sql_microseconds (no offset)
                let offset_h = loop {
                    let h = self.rng.gen_range(-12i32..=12);
                    if h != 0 || self.rng.gen_bool(0.2) {
                        break h;
                    }
                };
                Ok(format!(
                    "{}.{:06}{:+03}",
                    dt.format("%Y-%m-%d %H:%M:%S"),
                    micros,
                    offset_h
                ))
            }
            ("timestamp", "dot_dmy_24h") => Ok(self
                .random_datetime()
                .format("%d.%m.%Y %H:%M:%S")
                .to_string()),
            ("timestamp", "slash_ymd_24h") => Ok(self
                .random_datetime()
                .format("%Y/%m/%d %H:%M:%S")
                .to_string()),
            ("timestamp", "ctime") => {
                // C ctime() format: Mon Jan 15 14:30:00 2024
                let dt = self.random_datetime();
                let day = dt.day();
                Ok(format!(
                    "{} {:>2} {} {}",
                    dt.format("%a %b"),
                    day,
                    dt.format("%H:%M:%S"),
                    dt.format("%Y")
                ))
            }
            ("timestamp", "epoch_nanoseconds") => {
                // 19-digit nanosecond epoch (2015-2030 range)
                let secs = self.rng.gen_range(1_420_000_000i64..1_900_000_000);
                let nanos = self.rng.gen_range(0i64..1_000_000_000);
                Ok(format!("{}", secs * 1_000_000_000 + nanos))
            }
            ("timestamp", "iso_space_zulu") => {
                // RFC 3339 space variant: 2024-01-15 14:30:00Z
                Ok(format!(
                    "{}Z",
                    self.random_datetime().format("%Y-%m-%d %H:%M:%S")
                ))
            }
            ("timestamp", "dot_ymd_24h") => Ok(self
                .random_datetime()
                .format("%Y.%m.%d %H:%M:%S")
                .to_string()),

            // ── date (17 types) ──────────────────────────────────────────
            ("date", "iso") => Ok(self.random_datetime().format("%Y-%m-%d").to_string()),
            ("date", "mdy_slash") => Ok(self.random_datetime().format("%m/%d/%Y").to_string()),
            ("date", "dmy_slash") => Ok(self.random_datetime().format("%d/%m/%Y").to_string()),
            ("date", "dmy_dot") => Ok(self.random_datetime().format("%d.%m.%Y").to_string()),
            ("date", "compact_ymd") => {
                let dt = self.random_datetime();
                Ok(format!("{}{:02}{:02}", dt.year(), dt.month(), dt.day()))
            }
            ("date", "compact_mdy") => {
                let dt = self.random_datetime();
                Ok(format!("{:02}{:02}{}", dt.month(), dt.day(), dt.year()))
            }
            ("date", "compact_dmy") => {
                let dt = self.random_datetime();
                Ok(format!("{:02}{:02}{}", dt.day(), dt.month(), dt.year()))
            }
            ("date", "short_ymd") => Ok(self.random_datetime().format("%y-%m-%d").to_string()),
            ("date", "short_mdy") => Ok(self.random_datetime().format("%m-%d-%y").to_string()),
            ("date", "short_dmy") => Ok(self.random_datetime().format("%d-%m-%y").to_string()),
            ("date", "abbreviated_month") => {
                // CLDR-sourced locale patterns with abbreviated month names.
                let dt = self.random_datetime();
                let abbrevs = locale_data::month_abbreviations(self.current_locale());
                let month_abbr = abbrevs[(dt.month0() as usize) % abbrevs.len()];
                let pat = locale_data::date_format_pattern(self.current_locale(), false);
                Ok(Self::format_date_parts(
                    &pat,
                    dt.day(),
                    month_abbr,
                    dt.year(),
                ))
            }
            ("date", "long_full_month") => {
                // CLDR-sourced locale patterns with full month names.
                let dt = self.random_datetime();
                let months = locale_data::month_names(self.current_locale());
                let month_name = months[(dt.month0() as usize) % months.len()];
                let pat = locale_data::date_format_pattern(self.current_locale(), true);
                Ok(Self::format_date_parts(
                    &pat,
                    dt.day(),
                    month_name,
                    dt.year(),
                ))
            }
            ("date", "weekday_abbreviated_month") => {
                // CLDR-sourced locale patterns: weekday + abbreviated month date.
                let dt = self.random_datetime();
                let weekdays = locale_data::weekday_names(self.current_locale());
                let abbrevs = locale_data::month_abbreviations(self.current_locale());
                let weekday =
                    weekdays[(dt.weekday().num_days_from_monday() as usize) % weekdays.len()];
                let month_abbr = abbrevs[(dt.month0() as usize) % abbrevs.len()];
                let pat = locale_data::date_format_pattern(self.current_locale(), false);
                let date_part = Self::format_date_parts(&pat, dt.day(), month_abbr, dt.year());
                let (wk_before, wk_sep) = locale_data::weekday_format(self.current_locale());
                Ok(if wk_before {
                    format!("{}{}{}", weekday, wk_sep, date_part)
                } else {
                    format!("{}{}{}", date_part, wk_sep, weekday)
                })
            }
            ("date", "weekday_full_month") => {
                // CLDR-sourced locale patterns: weekday + full month date.
                let dt = self.random_datetime();
                let weekdays = locale_data::weekday_names(self.current_locale());
                let months = locale_data::month_names(self.current_locale());
                let weekday =
                    weekdays[(dt.weekday().num_days_from_monday() as usize) % weekdays.len()];
                let month_name = months[(dt.month0() as usize) % months.len()];
                let pat = locale_data::date_format_pattern(self.current_locale(), true);
                let date_part = Self::format_date_parts(&pat, dt.day(), month_name, dt.year());
                let (wk_before, wk_sep) = locale_data::weekday_format(self.current_locale());
                Ok(if wk_before {
                    format!("{}{}{}", weekday, wk_sep, date_part)
                } else {
                    format!("{}{}{}", date_part, wk_sep, weekday)
                })
            }
            ("date", "ordinal") => {
                // v19: ISO 8601 ordinal date YYYY-DDD (e.g., 2024-075).
                // Distinguished from abbreviated_month (has month names)
                // and month_year_full (has full month + year text).
                // The YYYY-DDD format with 3-digit zero-padded day-of-year
                // is the key structural signal.
                let year = self.rng.gen_range(2000..2030);
                let day = self.rng.gen_range(1..=365);
                Ok(format!("{}-{:03}", year, day))
            }
            ("date", "julian") => Ok(format!(
                "{:02}-{:03}",
                self.rng.gen_range(20..30),
                self.rng.gen_range(1..366)
            )),
            ("date", "iso_week") => Ok(format!(
                "{}-W{:02}",
                self.rng.gen_range(2020..2030),
                self.rng.gen_range(1..53)
            )),

            // ── new date types (23 types) ────────────────────────────────

            // Separator variants (7)
            ("date", "ymd_slash") => Ok(self.random_datetime().format("%Y/%m/%d").to_string()),
            ("date", "ymd_dot") => Ok(self.random_datetime().format("%Y.%m.%d").to_string()),
            ("date", "dmy_dash") => Ok(self.random_datetime().format("%d-%m-%Y").to_string()),
            ("date", "mdy_dash") => Ok(self.random_datetime().format("%m-%d-%Y").to_string()),
            ("date", "mdy_short_slash") => {
                Ok(self.random_datetime().format("%m/%d/%y").to_string())
            }
            ("date", "dmy_short_slash") => {
                Ok(self.random_datetime().format("%d/%m/%y").to_string())
            }
            ("date", "dmy_short_dot") => Ok(self.random_datetime().format("%d.%m.%y").to_string()),

            // Named month variants (6)
            ("date", "dmy_space_abbrev") => {
                // 15 Jan 2024
                let dt = self.random_datetime();
                Ok(format!("{} {} {}", dt.day(), dt.format("%b"), dt.year()))
            }
            ("date", "dmy_space_full") => {
                // 15 January 2024 — avoid "May" (3 chars) to meet minLength: 12
                let dt = self.random_datetime_avoiding_may();
                Ok(format!("{:02} {} {}", dt.day(), dt.format("%B"), dt.year()))
            }
            ("date", "abbrev_month_no_comma") => {
                // Jan 15 2024 (no comma, unlike abbreviated_month)
                let dt = self.random_datetime();
                Ok(format!("{} {} {}", dt.format("%b"), dt.day(), dt.year()))
            }
            ("date", "full_month_no_comma") => {
                // January 15 2024 — avoid "May" (3 chars) to meet minLength: 12
                let dt = self.random_datetime_avoiding_may();
                Ok(format!("{} {:02} {}", dt.format("%B"), dt.day(), dt.year()))
            }
            ("date", "dmy_dash_abbrev") => {
                // 15-Jan-2024 (Oracle NLS_DATE_FORMAT default)
                Ok(self.random_datetime().format("%d-%b-%Y").to_string())
            }
            ("date", "dmy_dash_abbrev_short") => {
                // 15-Jan-24 (Oracle DD-MON-RR)
                Ok(self.random_datetime().format("%d-%b-%y").to_string())
            }

            // Partial dates (5)
            ("date", "year_month") => Ok(self.random_datetime().format("%Y-%m").to_string()),
            ("date", "compact_ym") => {
                let dt = self.random_datetime();
                Ok(format!("{}{:02}", dt.year(), dt.month()))
            }
            ("date", "month_year_full") => {
                // January 2024
                let dt = self.random_datetime();
                Ok(format!("{} {}", dt.format("%B"), dt.year()))
            }
            ("date", "month_year_abbrev") => {
                // Jan 2024
                let dt = self.random_datetime();
                Ok(format!("{} {}", dt.format("%b"), dt.year()))
            }
            ("date", "month_year_slash") => {
                // 01/2024 or 01/24 (MM/YYYY or MM/YY)
                let dt = self.random_datetime();
                if self.rng.gen_bool(0.3) {
                    // MM/YY format (credit card expiration style)
                    Ok(format!("{:02}/{:02}", dt.month(), dt.year() % 100))
                } else {
                    // MM/YYYY format
                    Ok(format!("{:02}/{}", dt.month(), dt.year()))
                }
            }

            // Weekday variant (1)
            ("date", "weekday_dmy_full") => {
                // Monday, 15 January 2024 — use long months (≥6 chars) for minLength: 22
                let dt = self.random_datetime_long_month();
                Ok(format!(
                    "{}, {:02} {} {}",
                    dt.format("%A"),
                    dt.day(),
                    dt.format("%B"),
                    dt.year()
                ))
            }

            // CJK formats (4)
            ("date", "chinese_ymd") => {
                // 2024年1月15日
                let dt = self.random_datetime();
                Ok(format!("{}年{}月{}日", dt.year(), dt.month(), dt.day()))
            }
            ("date", "korean_ymd") => {
                // 2024년 1월 15일
                let dt = self.random_datetime();
                Ok(format!("{}년 {}월 {}일", dt.year(), dt.month(), dt.day()))
            }
            ("date", "jp_era_short") => {
                // v19: Japanese era short format: R6/01/15, H31/04/30
                // Must match validation pattern ^[RHSTM]\d{1,2}/\d{1,2}/\d{1,2}$
                // Distinguished from alphanumeric_id by the era letter prefix
                // (H, S, T, M, R) followed by slash-separated date components.
                // Vary padding: sometimes zero-padded month/day, sometimes not.
                let dt = self.random_datetime();
                let (era_letter, era_year) = self.gregorian_to_jp_era(dt.year());
                if self.rng.gen_bool(0.5) {
                    // Zero-padded month and day
                    Ok(format!(
                        "{}{}/{:02}/{:02}",
                        era_letter,
                        era_year,
                        dt.month(),
                        dt.day()
                    ))
                } else {
                    // Unpadded month and day
                    Ok(format!(
                        "{}{}/{}/{}",
                        era_letter,
                        era_year,
                        dt.month(),
                        dt.day()
                    ))
                }
            }
            ("date", "jp_era_long") => {
                // 令和6年1月15日 — Japanese era long format
                let dt = self.random_datetime();
                let (_, era_year) = self.gregorian_to_jp_era(dt.year());
                let era_name = self.jp_era_name(dt.year());
                Ok(format!(
                    "{}{}年{}月{}日",
                    era_name,
                    era_year,
                    dt.month(),
                    dt.day()
                ))
            }

            // ── period (2 types) ─────────────────────────────────────────
            ("period", "quarter") => {
                // Q1 2024 or 2024-Q1
                let year = self.rng.gen_range(2015..2030);
                let q = self.rng.gen_range(1..=4);
                if self.rng.gen_bool(0.5) {
                    Ok(format!("Q{} {}", q, year))
                } else {
                    Ok(format!("{}-Q{}", year, q))
                }
            }
            ("period", "fiscal_year") => {
                // FY2024 or FY24
                let year = self.rng.gen_range(2015i32..2030);
                if self.rng.gen_bool(0.6) {
                    Ok(format!("FY{}", year))
                } else {
                    Ok(format!("FY{}", year % 100))
                }
            }

            // ── time (5 types) ───────────────────────────────────────────
            ("time", "iso") => {
                let dt = self.random_datetime();
                let micros = self.rng.gen_range(0..1000000);
                Ok(format!("{}.{:06}", dt.format("%H:%M:%S"), micros))
            }
            ("time", "hms_24h") => Ok(self.random_datetime().format("%H:%M:%S").to_string()),
            ("time", "hm_24h") => Ok(self.random_datetime().format("%H:%M").to_string()),
            ("time", "hms_12h") => Ok(self.random_datetime().format("%I:%M:%S %p").to_string()),
            ("time", "hm_12h") => Ok(self.random_datetime().format("%I:%M %p").to_string()),

            // ── epoch (3 types) ──────────────────────────────────────────
            ("epoch", "unix_seconds") => Ok(self
                .rng
                .gen_range(1_000_000_000i64..2_000_000_000)
                .to_string()),
            ("epoch", "unix_milliseconds") => Ok(self
                .rng
                .gen_range(1_000_000_000_000i64..2_000_000_000_000)
                .to_string()),
            ("epoch", "unix_microseconds") => Ok(self
                .rng
                .gen_range(1_000_000_000_000_000i64..2_000_000_000_000_000)
                .to_string()),

            // ── offset (2 types) ─────────────────────────────────────────
            ("offset", "utc") => {
                let h = self.rng.gen_range(-12i32..=14);
                Ok(format!("UTC {:+03}:00", h))
            }
            ("offset", "iana") => {
                let tzs = [
                    "America/New_York",
                    "America/Los_Angeles",
                    "America/Chicago",
                    "Europe/London",
                    "Europe/Paris",
                    "Europe/Berlin",
                    "Asia/Tokyo",
                    "Asia/Shanghai",
                    "Asia/Singapore",
                    "Australia/Sydney",
                    "Pacific/Auckland",
                    "Africa/Cairo",
                ];
                Ok(tzs[self.rng.gen_range(0..tzs.len())].to_string())
            }

            // ── duration (1 type) ────────────────────────────────────────
            ("duration", "iso_8601") => {
                // Generate diverse ISO 8601 durations:
                // time-only (PT...), date+time (P...DT...), weeks (P...W),
                // verbose (P1Y2M3DT...), and negative durations (-P...)
                let variant = self.rng.gen_range(0..6);
                let neg = if self.rng.gen_bool(0.1) { "-" } else { "" };
                match variant {
                    0 => {
                        // Time-only: PT{H}H{M}M{S}S
                        let h = self.rng.gen_range(0..24);
                        let m = self.rng.gen_range(0..60);
                        let s = self.rng.gen_range(0..60);
                        if h > 0 {
                            Ok(format!("{}PT{}H{}M{}S", neg, h, m, s))
                        } else if m > 0 {
                            Ok(format!("{}PT{}M{}S", neg, m, s))
                        } else {
                            Ok(format!("{}PT{}S", neg, s))
                        }
                    }
                    1 => {
                        // Days + time: P{D}DT{H}H{M}M
                        let d = self.rng.gen_range(1..30);
                        let h = self.rng.gen_range(0..24);
                        Ok(format!("{}P{}DT{}H", neg, d, h))
                    }
                    2 => {
                        // Weeks: P{W}W
                        let w = self.rng.gen_range(1..52);
                        Ok(format!("{}P{}W", neg, w))
                    }
                    3 => {
                        // Verbose: P{Y}Y{M}M{D}DT{H}H{M}M{S}S
                        let y = self.rng.gen_range(0..5);
                        let mo = self.rng.gen_range(0..12);
                        let d = self.rng.gen_range(0..30);
                        let h = self.rng.gen_range(0..24);
                        let m = self.rng.gen_range(0..60);
                        let s = self.rng.gen_range(0..60);
                        Ok(format!("{}P{}Y{}M{}DT{}H{}M{}S", neg, y, mo, d, h, m, s))
                    }
                    4 => {
                        // Simple minutes: PT{M}M
                        let m = self.rng.gen_range(1..120);
                        Ok(format!("{}PT{}M", neg, m))
                    }
                    _ => {
                        // Year+month: P{Y}Y{M}M
                        let y = self.rng.gen_range(1..10);
                        let mo = self.rng.gen_range(0..12);
                        Ok(format!("{}P{}Y{}M", neg, y, mo))
                    }
                }
            }

            // ── component (6 types) ──────────────────────────────────────
            ("component", "year") => {
                // Weighted distribution: modern years most common, then historical, then future
                let year = if self.rng.gen_bool(0.60) {
                    // Modern era (60%): 1900-2025
                    self.rng.gen_range(1900..2026)
                } else if self.rng.gen_bool(0.625) {
                    // Historical (25% of total): 1000-1900
                    self.rng.gen_range(1000..1900)
                } else {
                    // Future (15% of total): 2026-2100
                    self.rng.gen_range(2026..2101)
                };
                Ok(year.to_string())
            }
            ("component", "month_name") => {
                let months = locale_data::month_names(self.current_locale());
                Ok(months[self.rng.gen_range(0..months.len())].to_string())
            }
            ("component", "day_of_week") => {
                let days = locale_data::weekday_names(self.current_locale());
                Ok(days[self.rng.gen_range(0..days.len())].to_string())
            }
            // century removed in taxonomy revision v0.5.1
            ("component", "periodicity") => {
                let periods = [
                    "Once",
                    "Daily",
                    "Weekly",
                    "Biweekly",
                    "Monthly",
                    "Quarterly",
                    "Yearly",
                    "Never",
                ];
                Ok(periods[self.rng.gen_range(0..periods.len())].to_string())
            }

            _ => Err(GeneratorError::NotImplemented(format!(
                "datetime.{}.{}",
                category, type_name
            ))),
        }
    }
}
