//! Value normalization for finetype_cast().
//!
//! Given a detected finetype label, normalizes the raw string value into a canonical
//! form suitable for DuckDB TRY_CAST. Returns None if validation fails.

/// Normalize a value based on its detected finetype label.
///
/// Returns the normalized string suitable for TRY_CAST to the target DuckDB type,
/// or None if the value doesn't validate for the detected type.
pub fn normalize(value: &str, label: &str) -> Option<String> {
    let domain = label.split('.').next().unwrap_or("");
    let category = label.split('.').nth(1).unwrap_or("");

    match (domain, category) {
        ("datetime", "date") => normalize_date(value, label),
        ("datetime", "time") => normalize_time(value, label),
        ("datetime", "timestamp") => normalize_timestamp(value, label),
        ("datetime", "epoch") => normalize_epoch(value),
        ("datetime", "duration") => Some(value.to_string()), // ISO 8601 durations pass through
        ("datetime", "component") => Some(value.to_string()),
        ("datetime", "offset") => Some(value.to_string()),
        ("technology", "development") if label == "technology.development.boolean" => {
            normalize_boolean(value)
        }
        ("representation", "boolean") => normalize_boolean(value),
        ("representation", "identifier") if label == "representation.identifier.uuid" => {
            normalize_uuid(value)
        }
        ("representation", "identifier") if label == "representation.identifier.increment" => {
            normalize_numeric(value, label)
        }
        ("technology", "internet") => normalize_internet(value, label),
        ("representation", "numeric") => normalize_numeric(value, label),
        ("container", "object") if label == "container.object.json" => normalize_json(value),
        _ => Some(value.to_string()), // Pass through for types that don't need normalization
    }
}

// ─── Date Normalization ──────────────────────────────────────────────────────

/// Month name lookup (case-insensitive).
fn month_number(name: &str) -> Option<u32> {
    match name.to_lowercase().as_str() {
        "january" | "jan" => Some(1),
        "february" | "feb" => Some(2),
        "march" | "mar" => Some(3),
        "april" | "apr" => Some(4),
        "may" => Some(5),
        "june" | "jun" => Some(6),
        "july" | "jul" => Some(7),
        "august" | "aug" => Some(8),
        "september" | "sep" | "sept" => Some(9),
        "october" | "oct" => Some(10),
        "november" | "nov" => Some(11),
        "december" | "dec" => Some(12),
        _ => None,
    }
}

/// Normalize a date value to ISO 8601 (YYYY-MM-DD).
fn normalize_date(value: &str, label: &str) -> Option<String> {
    let v = value.trim();
    match label {
        "datetime.date.iso" | "datetime.date.short_ymd" => {
            // Already YYYY-MM-DD or YYYY/MM/DD
            let v = v.replace('/', "-");
            validate_date_parts(&v)
        }
        "datetime.date.mdy_slash" | "datetime.date.short_mdy" => {
            // MM/DD/YYYY or MM-DD-YYYY
            let parts: Vec<&str> = v.splitn(3, ['/', '-']).collect();
            if parts.len() == 3 {
                let m = parts[0].trim();
                let d = parts[1].trim();
                let y = normalize_year(parts[2].trim());
                validate_date_parts(&format!("{}-{:0>2}-{:0>2}", y, m, d))
            } else {
                None
            }
        }
        "datetime.date.dmy_slash" | "datetime.date.dmy_dot" | "datetime.date.short_dmy" => {
            // DD/MM/YYYY or DD.MM.YYYY or DD-MM-YYYY
            let parts: Vec<&str> = v.splitn(3, ['/', '.', '-']).collect();
            if parts.len() == 3 {
                let d = parts[0].trim();
                let m = parts[1].trim();
                let y = normalize_year(parts[2].trim());
                validate_date_parts(&format!("{}-{:0>2}-{:0>2}", y, m, d))
            } else {
                None
            }
        }
        "datetime.date.long_full_month"
        | "datetime.date.abbreviated_month"
        | "datetime.date.weekday_full_month"
        | "datetime.date.weekday_abbreviated_month" => {
            // "January 15, 2024" / "Jan 15, 2024" / "Monday, January 15, 2024" etc.
            normalize_named_month_date(v)
        }
        "datetime.date.compact_ymd" => {
            // YYYYMMDD
            if v.len() == 8 && v.chars().all(|c| c.is_ascii_digit()) {
                let y = &v[0..4];
                let m = &v[4..6];
                let d = &v[6..8];
                validate_date_parts(&format!("{}-{}-{}", y, m, d))
            } else {
                None
            }
        }
        "datetime.date.compact_mdy" => {
            // MMDDYYYY
            if v.len() == 8 && v.chars().all(|c| c.is_ascii_digit()) {
                let m = &v[0..2];
                let d = &v[2..4];
                let y = &v[4..8];
                validate_date_parts(&format!("{}-{}-{}", y, m, d))
            } else {
                None
            }
        }
        "datetime.date.compact_dmy" => {
            // DDMMYYYY
            if v.len() == 8 && v.chars().all(|c| c.is_ascii_digit()) {
                let d = &v[0..2];
                let m = &v[2..4];
                let y = &v[4..8];
                validate_date_parts(&format!("{}-{}-{}", y, m, d))
            } else {
                None
            }
        }
        _ => {
            // Other date types: pass through
            Some(v.to_string())
        }
    }
}

/// Parse dates with named months: "January 15, 2024", "15 Jan 2024", etc.
fn normalize_named_month_date(value: &str) -> Option<String> {
    // Strip weekday prefix if present (e.g., "Monday, ")
    let v = if let Some(idx) = value.find(", ") {
        let before = &value[..idx];
        // Check if it starts with a weekday
        let weekdays = [
            "monday",
            "tuesday",
            "wednesday",
            "thursday",
            "friday",
            "saturday",
            "sunday",
        ];
        if weekdays
            .iter()
            .any(|w| before.to_lowercase().starts_with(w))
        {
            value[idx + 2..].trim()
        } else {
            value
        }
    } else {
        value
    };

    // Extract tokens: words and numbers
    let tokens: Vec<&str> = v.split([' ', ',', '-']).filter(|s| !s.is_empty()).collect();

    let mut month = None;
    let mut day = None;
    let mut year = None;

    for token in &tokens {
        if let Some(m) = month_number(token) {
            month = Some(m);
        } else if let Ok(num) = token.parse::<u32>() {
            if num > 31 {
                year = Some(num);
            } else if day.is_none() {
                // Strip ordinal suffix (1st, 2nd, 3rd, 4th)
                let clean = token.trim_end_matches(|c: char| c.is_alphabetic());
                if let Ok(d) = clean.parse::<u32>() {
                    day = Some(d);
                }
            }
        } else {
            // Try stripping ordinal suffix
            let clean = token.trim_end_matches(|c: char| c.is_alphabetic());
            if let Ok(d) = clean.parse::<u32>() {
                if d <= 31 && day.is_none() {
                    day = Some(d);
                }
            }
        }
    }

    if let (Some(y), Some(m), Some(d)) = (year, month, day) {
        validate_date_parts(&format!("{:04}-{:02}-{:02}", y, m, d))
    } else {
        None
    }
}

/// Normalize a 2-digit year to 4-digit. 00-49 → 2000-2049, 50-99 → 1950-1999.
fn normalize_year(y: &str) -> String {
    if y.len() == 2 {
        if let Ok(n) = y.parse::<u32>() {
            if n <= 49 {
                return format!("20{:02}", n);
            } else {
                return format!("19{:02}", n);
            }
        }
    }
    y.to_string()
}

/// Validate that a YYYY-MM-DD string has valid ranges.
fn validate_date_parts(iso: &str) -> Option<String> {
    let parts: Vec<&str> = iso.split('-').collect();
    if parts.len() != 3 {
        return None;
    }
    let y: u32 = parts[0].parse().ok()?;
    let m: u32 = parts[1].parse().ok()?;
    let d: u32 = parts[2].parse().ok()?;

    if !(1..=9999).contains(&y) || !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }

    // Basic days-in-month validation
    let max_days = match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if y.is_multiple_of(4) && (!y.is_multiple_of(100) || y.is_multiple_of(400)) {
                29
            } else {
                28
            }
        }
        _ => return None,
    };

    if d > max_days {
        return None;
    }

    Some(format!("{:04}-{:02}-{:02}", y, m, d))
}

// ─── Time Normalization ──────────────────────────────────────────────────────

/// Normalize a time value to HH:MM:SS (24-hour).
fn normalize_time(value: &str, label: &str) -> Option<String> {
    let v = value.trim();
    match label {
        "datetime.time.hms_24h" | "datetime.time.iso" => {
            // Already HH:MM:SS or close to it
            Some(v.to_string())
        }
        "datetime.time.hm_24h" => {
            // HH:MM → HH:MM:00
            Some(format!("{}:00", v))
        }
        "datetime.time.hm_12h" | "datetime.time.hms_12h" => normalize_12h_time(v),
        _ => Some(v.to_string()),
    }
}

/// Convert 12-hour time to 24-hour format.
fn normalize_12h_time(value: &str) -> Option<String> {
    let v = value.trim().to_uppercase();
    let is_pm = v.contains("PM");
    let is_am = v.contains("AM");

    if !is_pm && !is_am {
        return Some(value.to_string());
    }

    let time_part = v
        .replace("AM", "")
        .replace("PM", "")
        .replace("A.M.", "")
        .replace("P.M.", "")
        .trim()
        .to_string();

    let parts: Vec<&str> = time_part.split(':').collect();
    if parts.is_empty() {
        return None;
    }

    let mut hour: u32 = parts[0].trim().parse().ok()?;
    let min: u32 = if parts.len() > 1 {
        parts[1].trim().parse().ok()?
    } else {
        0
    };
    let sec: u32 = if parts.len() > 2 {
        parts[2].trim().parse().ok()?
    } else {
        0
    };

    if is_pm && hour != 12 {
        hour += 12;
    }
    if is_am && hour == 12 {
        hour = 0;
    }

    if hour > 23 || min > 59 || sec > 59 {
        return None;
    }

    Some(format!("{:02}:{:02}:{:02}", hour, min, sec))
}

// ─── Timestamp Normalization ─────────────────────────────────────────────────

/// Normalize a timestamp value. Most timestamp formats are already close to ISO 8601.
fn normalize_timestamp(value: &str, _label: &str) -> Option<String> {
    // Timestamps are complex; pass through as-is since DuckDB's TRY_CAST
    // handles many timestamp formats natively.
    let v = value.trim();
    if v.is_empty() {
        return None;
    }
    Some(v.to_string())
}

// ─── Epoch Normalization ─────────────────────────────────────────────────────

/// Validate epoch values are numeric.
fn normalize_epoch(value: &str) -> Option<String> {
    let v = value.trim();
    if v.parse::<i64>().is_ok() {
        Some(v.to_string())
    } else {
        None
    }
}

// ─── Boolean Normalization ───────────────────────────────────────────────────

/// Normalize boolean values to 'true' or 'false'.
fn normalize_boolean(value: &str) -> Option<String> {
    match value.trim().to_lowercase().as_str() {
        "true" | "yes" | "y" | "1" | "on" | "t" => Some("true".to_string()),
        "false" | "no" | "n" | "0" | "off" | "f" => Some("false".to_string()),
        _ => None,
    }
}

// ─── UUID Normalization ──────────────────────────────────────────────────────

/// Normalize UUID to lowercase hyphenated form.
fn normalize_uuid(value: &str) -> Option<String> {
    let v = value.trim().to_lowercase();
    // Remove braces/hyphens to get raw hex
    let hex: String = v.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    if hex.len() != 32 {
        return None;
    }
    Some(format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    ))
}

// ─── Internet Type Normalization ─────────────────────────────────────────────

/// Normalize internet types (IPs, URLs, etc.).
fn normalize_internet(value: &str, label: &str) -> Option<String> {
    let v = value.trim();
    match label {
        "technology.internet.ip_v4" => {
            // Validate IPv4: 4 octets, each 0-255
            let parts: Vec<&str> = v.split('.').collect();
            if parts.len() != 4 {
                return None;
            }
            for part in &parts {
                let n: u16 = part.parse().ok()?;
                if n > 255 {
                    return None;
                }
            }
            Some(v.to_string())
        }
        "technology.internet.http_status_code" => {
            let n: u16 = v.parse().ok()?;
            if (100..=599).contains(&n) {
                Some(v.to_string())
            } else {
                None
            }
        }
        "technology.internet.port" => {
            let n: u32 = v.parse().ok()?;
            if n <= 65535 {
                Some(v.to_string())
            } else {
                None
            }
        }
        _ => Some(v.to_string()),
    }
}

// ─── Numeric Normalization ───────────────────────────────────────────────────

/// Normalize numeric values: strip formatting (commas, currency symbols, whitespace).
fn normalize_numeric(value: &str, label: &str) -> Option<String> {
    let v = value.trim();
    match label {
        "representation.numeric.integer_number" | "representation.identifier.increment" => {
            // Strip commas and whitespace
            let clean: String = v
                .chars()
                .filter(|c| c.is_ascii_digit() || *c == '-' || *c == '+')
                .collect();
            clean.parse::<i64>().ok()?;
            Some(clean)
        }
        "representation.numeric.decimal_number" => {
            // Strip commas, keep decimal point
            let clean: String = v
                .chars()
                .filter(|c| c.is_ascii_digit() || *c == '.' || *c == '-' || *c == '+')
                .collect();
            clean.parse::<f64>().ok()?;
            Some(clean)
        }
        "representation.numeric.percentage" => {
            // Strip % sign and normalize
            let clean = v.trim_end_matches('%').trim();
            let clean: String = clean
                .chars()
                .filter(|c| c.is_ascii_digit() || *c == '.' || *c == '-' || *c == '+')
                .collect();
            clean.parse::<f64>().ok()?;
            Some(clean)
        }
        "representation.numeric.scientific_notation" => {
            // Validate as scientific notation
            v.parse::<f64>().ok()?;
            Some(v.to_string())
        }
        _ => Some(v.to_string()),
    }
}

// ─── JSON Normalization ──────────────────────────────────────────────────────

/// Validate JSON is well-formed.
fn normalize_json(value: &str) -> Option<String> {
    let v = value.trim();
    // Just validate it parses as JSON
    serde_json::from_str::<serde_json::Value>(v).ok()?;
    Some(v.to_string())
}

// ═══════════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ── Date Tests ───────────────────────────────────────────────────────

    #[test]
    fn test_iso_date_passthrough() {
        assert_eq!(
            normalize("2024-01-15", "datetime.date.iso"),
            Some("2024-01-15".to_string())
        );
    }

    #[test]
    fn test_mdy_date_normalized() {
        assert_eq!(
            normalize("01/15/2024", "datetime.date.mdy_slash"),
            Some("2024-01-15".to_string())
        );
    }

    #[test]
    fn test_dmy_date_normalized() {
        assert_eq!(
            normalize("15/01/2024", "datetime.date.dmy_slash"),
            Some("2024-01-15".to_string())
        );
    }

    #[test]
    fn test_dmy_dot_date_normalized() {
        assert_eq!(
            normalize("15.01.2024", "datetime.date.dmy_dot"),
            Some("2024-01-15".to_string())
        );
    }

    #[test]
    fn test_long_date_normalized() {
        assert_eq!(
            normalize("January 15, 2024", "datetime.date.long_full_month"),
            Some("2024-01-15".to_string())
        );
    }

    #[test]
    fn test_abbreviated_date_normalized() {
        assert_eq!(
            normalize("Jan 15, 2024", "datetime.date.abbreviated_month"),
            Some("2024-01-15".to_string())
        );
    }

    #[test]
    fn test_weekday_date_normalized() {
        assert_eq!(
            normalize(
                "Monday, January 15, 2024",
                "datetime.date.weekday_full_month"
            ),
            Some("2024-01-15".to_string())
        );
    }

    #[test]
    fn test_compact_ymd() {
        assert_eq!(
            normalize("20240115", "datetime.date.compact_ymd"),
            Some("2024-01-15".to_string())
        );
    }

    #[test]
    fn test_compact_mdy() {
        assert_eq!(
            normalize("01152024", "datetime.date.compact_mdy"),
            Some("2024-01-15".to_string())
        );
    }

    #[test]
    fn test_invalid_date_returns_none() {
        assert_eq!(normalize("13/32/2024", "datetime.date.mdy_slash"), None);
    }

    #[test]
    fn test_feb_leap_year() {
        assert_eq!(
            normalize("02/29/2024", "datetime.date.mdy_slash"),
            Some("2024-02-29".to_string())
        );
        assert_eq!(normalize("02/29/2023", "datetime.date.mdy_slash"), None);
    }

    // ── Time Tests ───────────────────────────────────────────────────────

    #[test]
    fn test_24h_time_passthrough() {
        assert_eq!(
            normalize("14:30:45", "datetime.time.hms_24h"),
            Some("14:30:45".to_string())
        );
    }

    #[test]
    fn test_12h_to_24h() {
        assert_eq!(
            normalize("2:30 PM", "datetime.time.hm_12h"),
            Some("14:30:00".to_string())
        );
        assert_eq!(
            normalize("12:00 AM", "datetime.time.hm_12h"),
            Some("00:00:00".to_string())
        );
        assert_eq!(
            normalize("12:00 PM", "datetime.time.hm_12h"),
            Some("12:00:00".to_string())
        );
    }

    #[test]
    fn test_hm_appends_seconds() {
        assert_eq!(
            normalize("14:30", "datetime.time.hm_24h"),
            Some("14:30:00".to_string())
        );
    }

    // ── Boolean Tests ────────────────────────────────────────────────────

    #[test]
    fn test_boolean_normalization() {
        assert_eq!(
            normalize("Yes", "technology.development.boolean"),
            Some("true".to_string())
        );
        assert_eq!(
            normalize("no", "technology.development.boolean"),
            Some("false".to_string())
        );
        assert_eq!(
            normalize("1", "technology.development.boolean"),
            Some("true".to_string())
        );
        assert_eq!(
            normalize("0", "technology.development.boolean"),
            Some("false".to_string())
        );
        assert_eq!(normalize("maybe", "technology.development.boolean"), None);
    }

    #[test]
    fn test_boolean_binary_normalization() {
        assert_eq!(
            normalize("1", "representation.boolean.binary"),
            Some("true".to_string())
        );
        assert_eq!(
            normalize("0", "representation.boolean.binary"),
            Some("false".to_string())
        );
    }

    #[test]
    fn test_boolean_initials_normalization() {
        assert_eq!(
            normalize("T", "representation.boolean.initials"),
            Some("true".to_string())
        );
        assert_eq!(
            normalize("F", "representation.boolean.initials"),
            Some("false".to_string())
        );
        assert_eq!(
            normalize("Y", "representation.boolean.initials"),
            Some("true".to_string())
        );
        assert_eq!(
            normalize("N", "representation.boolean.initials"),
            Some("false".to_string())
        );
    }

    #[test]
    fn test_boolean_terms_normalization() {
        assert_eq!(
            normalize("True", "representation.boolean.terms"),
            Some("true".to_string())
        );
        assert_eq!(
            normalize("FALSE", "representation.boolean.terms"),
            Some("false".to_string())
        );
        assert_eq!(
            normalize("yes", "representation.boolean.terms"),
            Some("true".to_string())
        );
        assert_eq!(
            normalize("off", "representation.boolean.terms"),
            Some("false".to_string())
        );
    }

    // ── UUID Tests ───────────────────────────────────────────────────────

    #[test]
    fn test_uuid_normalization() {
        assert_eq!(
            normalize(
                "550E8400-E29B-41D4-A716-446655440000",
                "representation.identifier.uuid"
            ),
            Some("550e8400-e29b-41d4-a716-446655440000".to_string())
        );
    }

    #[test]
    fn test_uuid_invalid() {
        assert_eq!(
            normalize("not-a-uuid", "representation.identifier.uuid"),
            None
        );
    }

    // ── Numeric Tests ────────────────────────────────────────────────────

    #[test]
    fn test_integer_strip_commas() {
        assert_eq!(
            normalize("1,234,567", "representation.numeric.integer_number"),
            Some("1234567".to_string())
        );
    }

    #[test]
    fn test_percentage_strip_sign() {
        assert_eq!(
            normalize("95.5%", "representation.numeric.percentage"),
            Some("95.5".to_string())
        );
    }

    // ── Internet Tests ───────────────────────────────────────────────────

    #[test]
    fn test_ipv4_valid() {
        assert_eq!(
            normalize("192.168.1.1", "technology.internet.ip_v4"),
            Some("192.168.1.1".to_string())
        );
    }

    #[test]
    fn test_ipv4_invalid() {
        assert_eq!(
            normalize("999.999.999.999", "technology.internet.ip_v4"),
            None
        );
    }

    // ── JSON Tests ───────────────────────────────────────────────────────

    #[test]
    fn test_json_valid() {
        assert!(normalize("{\"key\": \"value\"}", "container.object.json").is_some());
    }

    #[test]
    fn test_json_invalid() {
        assert_eq!(normalize("{bad json}", "container.object.json"), None);
    }

    // ── Passthrough Tests ────────────────────────────────────────────────

    #[test]
    fn test_unknown_type_passthrough() {
        assert_eq!(
            normalize("anything", "identity.person.first_name"),
            Some("anything".to_string())
        );
    }
}
