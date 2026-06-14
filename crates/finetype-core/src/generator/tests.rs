//! Tests for the generator module.

use super::*;

fn test_taxonomy() -> Taxonomy {
    Taxonomy::from_yaml(
        r#"
test.test.test:
  title: "Test"
  designation: universal
  locales: [UNIVERSAL]
  broad_type: VARCHAR
  release_priority: 1
"#,
    )
    .unwrap()
}

#[test]
fn test_datetime_iso_8601() {
    let mut gen = Generator::with_seed(test_taxonomy(), 42);
    let val = gen.generate_value("datetime.timestamp.iso_8601").unwrap();
    assert!(val.contains('T'));
    assert!(val.ends_with('Z'));
    assert_eq!(val.len(), 20);
}

#[test]
fn test_datetime_date_mdy_slash() {
    let mut gen = Generator::with_seed(test_taxonomy(), 42);
    let val = gen.generate_value("datetime.date.mdy_slash").unwrap();
    assert_eq!(val.len(), 10);
    assert!(val.contains('/'));
}

#[test]
fn test_technology_ipv4() {
    let mut gen = Generator::with_seed(test_taxonomy(), 42);
    let val = gen.generate_value("technology.internet.ip_v4").unwrap();
    assert_eq!(val.split('.').count(), 4);
}

#[test]
fn test_technology_uuid() {
    let mut gen = Generator::with_seed(test_taxonomy(), 42);
    let val = gen
        .generate_value("representation.identifier.uuid")
        .unwrap();
    assert_eq!(val.len(), 36);
    assert_eq!(val.split('-').count(), 5);
}

#[test]
fn test_identity_email() {
    let mut gen = Generator::with_seed(test_taxonomy(), 42);
    let val = gen.generate_value("identity.person.email").unwrap();
    assert!(val.contains('@'));
    assert!(val.contains('.'));
}

#[test]
fn test_identity_phone() {
    let mut gen = Generator::with_seed(test_taxonomy(), 42);
    let val = gen.generate_value("identity.person.phone_number").unwrap();
    assert!(!val.is_empty());
}

#[test]
fn test_geography_latitude() {
    let mut gen = Generator::with_seed(test_taxonomy(), 42);
    let val = gen.generate_value("geography.coordinate.latitude").unwrap();
    let lat: f64 = val.parse().unwrap();
    assert!((-90.0..=90.0).contains(&lat));
}

#[test]
fn test_representation_integer() {
    let mut gen = Generator::with_seed(test_taxonomy(), 42);
    let val = gen
        .generate_value("representation.numeric.integer_number")
        .unwrap();
    let _: i64 = val.parse().unwrap();
}

#[test]
fn test_representation_hex_color() {
    let mut gen = Generator::with_seed(test_taxonomy(), 42);
    let val = gen
        .generate_value("representation.format.color_hex")
        .unwrap();
    assert!(val.len() == 7 || val.len() == 6);
}

#[test]
fn test_container_json() {
    let mut gen = Generator::with_seed(test_taxonomy(), 42);
    let val = gen.generate_value("container.object.json").unwrap();
    assert!(val.starts_with('{'));
    assert!(val.ends_with('}'));
}

#[test]
fn test_container_query_string() {
    let mut gen = Generator::with_seed(test_taxonomy(), 42);
    let val = gen
        .generate_value("container.key_value.query_string")
        .unwrap();
    assert!(val.contains('='));
    assert!(val.contains('&'));
}

#[test]
fn test_all_domains_have_generators() {
    let mut gen = Generator::with_seed(test_taxonomy(), 42);
    // Test one type from each domain
    assert!(gen.generate_value("datetime.timestamp.iso_8601").is_ok());
    assert!(gen.generate_value("technology.internet.ip_v4").is_ok());
    assert!(gen.generate_value("identity.person.email").is_ok());
    assert!(gen.generate_value("geography.location.country").is_ok());
    assert!(gen
        .generate_value("representation.numeric.integer_number")
        .is_ok());
    assert!(gen.generate_value("container.object.json").is_ok());
}

#[test]
fn test_unknown_label_returns_error() {
    let mut gen = Generator::with_seed(test_taxonomy(), 42);
    assert!(gen.generate_value("nonexistent.type.foo").is_err());
}

#[test]
fn test_credit_card_luhn_valid() {
    let mut gen = Generator::with_seed(test_taxonomy(), 42);
    for _ in 0..100 {
        let val = gen
            .generate_value("finance.payment.credit_card_number")
            .unwrap();
        // Verify Luhn validity
        assert!(
            luhn::valid(&val),
            "Credit card number {} failed Luhn check",
            val
        );
        // Verify correct lengths
        assert!(
            val.len() == 15 || val.len() == 16,
            "Credit card length {} unexpected for {}",
            val.len(),
            val
        );
        // Verify correct prefixes
        let first = val.chars().next().unwrap();
        assert!(
            matches!(first, '3' | '4' | '5' | '6'),
            "Unexpected credit card prefix: {}",
            val
        );
    }
}

#[test]
fn test_imei_luhn_valid() {
    let mut gen = Generator::with_seed(test_taxonomy(), 42);
    for _ in 0..100 {
        let val = gen.generate_value("technology.code.imei").unwrap();
        assert_eq!(val.len(), 15, "IMEI should be 15 digits: {}", val);
        assert!(luhn::valid(&val), "IMEI {} failed Luhn check", val);
    }
}

#[test]
fn test_ean_check_digit_valid() {
    let mut gen = Generator::with_seed(test_taxonomy(), 42);
    for _ in 0..100 {
        let val = gen.generate_value("identity.commerce.ean").unwrap();
        assert!(
            val.len() == 8 || val.len() == 13,
            "EAN length {} unexpected for {}",
            val.len(),
            val
        );
        // Verify EAN check digit
        let (body, check_str) = val.split_at(val.len() - 1);
        let expected_check = {
            let sum: u32 = body
                .bytes()
                .enumerate()
                .map(|(i, b)| {
                    let d = (b - b'0') as u32;
                    if i % 2 == 0 {
                        d
                    } else {
                        d * 3
                    }
                })
                .sum();
            ((10 - (sum % 10)) % 10) as u8
        };
        let actual_check = check_str.bytes().next().unwrap() - b'0';
        assert_eq!(
            actual_check, expected_check,
            "EAN {} has invalid check digit (expected {}, got {})",
            val, expected_check, actual_check
        );
    }
}

#[test]
fn test_credit_card_network_prefixes() {
    let mut gen = Generator::with_seed(test_taxonomy(), 42);
    let mut saw_visa = false;
    let mut saw_mc = false;
    let mut saw_amex = false;
    let mut saw_discover = false;
    for _ in 0..200 {
        let val = gen
            .generate_value("finance.payment.credit_card_number")
            .unwrap();
        if val.starts_with('4') && val.len() == 16 {
            saw_visa = true;
        }
        if val.starts_with("51")
            || val.starts_with("52")
            || val.starts_with("53")
            || val.starts_with("54")
            || val.starts_with("55")
        {
            saw_mc = true;
        }
        if val.starts_with("34") || val.starts_with("37") {
            saw_amex = true;
        }
        if val.starts_with("6011") {
            saw_discover = true;
        }
    }
    assert!(saw_visa, "Should generate Visa cards");
    assert!(saw_mc, "Should generate Mastercard cards");
    assert!(saw_amex, "Should generate Amex cards");
    assert!(saw_discover, "Should generate Discover cards");
}

#[test]
fn test_phone_number_valid() {
    let mut gen = Generator::with_seed(test_taxonomy(), 42);
    let mut valid_count = 0;
    let mut national_count = 0;
    let mut intl_count = 0;
    let mut e164_count = 0;
    let total = 200;
    // Test across different locales for diversity
    let locales = ["EN_US", "EN_GB", "EN_AU", "DE", "FR", "ES", "JA"];
    for (i, _) in (0..total).enumerate() {
        gen.locale = Some(locales[i % locales.len()].to_string());
        let val = gen.generate_value("identity.person.phone_number").unwrap();
        // All phone numbers should contain digits
        assert!(
            val.chars().any(|c| c.is_ascii_digit()),
            "Phone number should contain digits: {}",
            val
        );
        // Classify format type
        if val.starts_with('+') {
            if val.contains(' ') || val.contains('-') || val.contains('(') {
                intl_count += 1;
            } else {
                e164_count += 1;
            }
        } else {
            national_count += 1;
        }
        // Parse with phonenumber crate (try with + prefix for national numbers)
        if val.starts_with('+') {
            if let Ok(number) = phonenumber::parse(None, &val) {
                if phonenumber::is_valid(&number) {
                    valid_count += 1;
                }
            }
        }
    }
    gen.locale = None;
    // At least 50% of +prefixed numbers should pass strict validation
    let intl_total = intl_count + e164_count;
    if intl_total > 0 {
        let valid_pct = valid_count as f64 / intl_total as f64 * 100.0;
        assert!(
            valid_pct >= 50.0,
            "Only {:.0}% of international phone numbers passed validation ({}/{})",
            valid_pct,
            valid_count,
            intl_total
        );
    }
    // Verify format diversity: all three formats should appear
    assert!(
        national_count > 0,
        "Should generate NATIONAL format numbers"
    );
    assert!(
        intl_count > 0,
        "Should generate INTERNATIONAL format numbers"
    );
    assert!(e164_count > 0, "Should generate E164 format numbers");
}

#[test]
fn test_phone_number_locale_routing() {
    let mut gen = Generator::with_seed(test_taxonomy(), 99);

    // Helper: generate many samples and check that at least one has the expected prefix
    fn locale_produces_prefix(
        gen: &mut Generator,
        locale: &str,
        prefix: &str,
        national_prefix: &str,
    ) {
        gen.locale = Some(locale.to_string());
        let mut saw_intl = false;
        let mut saw_national = false;
        for _ in 0..30 {
            let val = gen.generate_value("identity.person.phone_number").unwrap();
            if val.starts_with(prefix) {
                saw_intl = true;
            }
            if val.starts_with(national_prefix) || val.starts_with('(') {
                saw_national = true;
            }
        }
        assert!(
            saw_intl || saw_national,
            "{} should produce {} or {} numbers in 30 samples",
            locale,
            prefix,
            national_prefix
        );
    }

    // US locale → +1 or (AAA) format
    locale_produces_prefix(&mut gen, "EN_US", "+1", "(");

    // GB locale → +44 or 0 format
    locale_produces_prefix(&mut gen, "EN_GB", "+44", "0");

    // AU locale → +61 or 0 format
    locale_produces_prefix(&mut gen, "EN_AU", "+61", "0");

    // DE locale → +49 or 0 format
    locale_produces_prefix(&mut gen, "DE", "+49", "0");

    // FR locale → +33 or 0 format
    locale_produces_prefix(&mut gen, "FR", "+33", "0");

    // JA locale → +81 or 0 format
    locale_produces_prefix(&mut gen, "JA", "+81", "0");

    gen.locale = None;
}

#[test]
fn test_locale_aware_names() {
    let mut gen = Generator::with_seed(test_taxonomy(), 42);

    // German names
    gen.locale = Some("DE".to_string());
    let first = gen.generate_value("identity.person.first_name").unwrap();
    let de_names = locale_data::first_names("DE");
    assert!(
        de_names.contains(&first.as_str()),
        "DE first name '{}' not in German name list",
        first
    );

    // Japanese names
    gen.locale = Some("JA".to_string());
    let first = gen.generate_value("identity.person.first_name").unwrap();
    let ja_names = locale_data::first_names("JA");
    assert!(
        ja_names.contains(&first.as_str()),
        "JA first name '{}' not in Japanese name list",
        first
    );

    gen.locale = None;
}

#[test]
fn test_locale_aware_months() {
    let mut gen = Generator::with_seed(test_taxonomy(), 42);

    // French month names
    gen.locale = Some("FR".to_string());
    let month = gen.generate_value("datetime.component.month_name").unwrap();
    let fr_months = locale_data::month_names("FR");
    assert!(
        fr_months.contains(&month.as_str()),
        "FR month '{}' not in French month list",
        month
    );

    gen.locale = None;
}

#[test]
fn test_localized_generation() {
    let taxonomy = Taxonomy::from_directory("../../labels").unwrap();
    let mut gen = Generator::with_seed(taxonomy, 42);

    let samples = gen.generate_all_localized(5, 2);
    assert!(!samples.is_empty(), "Should generate localized samples");

    // All labels should be 4-level (domain.category.type.LOCALE)
    for sample in &samples {
        let parts: Vec<&str> = sample.label.split('.').collect();
        assert_eq!(parts.len(), 4, "Label should be 4-level: {}", sample.label);
    }

    // Should have UNIVERSAL labels
    let universal_count = samples
        .iter()
        .filter(|s| s.label.ends_with(".UNIVERSAL"))
        .count();
    assert!(universal_count > 0, "Should have UNIVERSAL-suffixed labels");

    // Should have locale labels (not UNIVERSAL)
    let locale_count = samples
        .iter()
        .filter(|s| !s.label.ends_with(".UNIVERSAL"))
        .count();
    assert!(locale_count > 0, "Should have locale-specific labels");
}

#[test]
fn test_isin_check_digit_valid() {
    let mut gen = Generator::with_seed(test_taxonomy(), 42);
    for _ in 0..100 {
        let val = gen.generate_value("finance.securities.isin").unwrap();
        assert_eq!(val.len(), 12, "ISIN should be 12 chars: {}", val);
        assert!(
            val[..2].chars().all(|c| c.is_ascii_uppercase()),
            "ISIN should start with 2 letters: {}",
            val
        );
        // Verify ISIN check digit by recomputing
        let body = &val[..11];
        let expected = gen.isin_check_digit(body);
        let actual = val.chars().last().unwrap().to_digit(10).unwrap() as u8;
        assert_eq!(
            actual, expected,
            "ISIN {} has invalid check digit (expected {}, got {})",
            val, expected, actual
        );
    }
}

#[test]
fn test_isin_known_values() {
    // Verify against known real ISINs
    let gen = Generator::with_seed(test_taxonomy(), 42);
    // Apple Inc: US0378331005
    assert_eq!(gen.isin_check_digit("US037833100"), 5);
    // SAP SE: DE0007164600
    assert_eq!(gen.isin_check_digit("DE000716460"), 0);
}

#[test]
fn test_cusip_check_digit_valid() {
    let mut gen = Generator::with_seed(test_taxonomy(), 42);
    for _ in 0..100 {
        let val = gen.generate_value("finance.securities.cusip").unwrap();
        assert_eq!(val.len(), 9, "CUSIP should be 9 chars: {}", val);
        let body = &val[..8];
        let expected = gen.cusip_check_digit(body);
        let actual = val.chars().last().unwrap().to_digit(10).unwrap() as u8;
        assert_eq!(
            actual, expected,
            "CUSIP {} has invalid check digit (expected {}, got {})",
            val, expected, actual
        );
    }
}

#[test]
fn test_sedol_check_digit_valid() {
    let mut gen = Generator::with_seed(test_taxonomy(), 42);
    for _ in 0..100 {
        let val = gen.generate_value("finance.securities.sedol").unwrap();
        assert_eq!(val.len(), 7, "SEDOL should be 7 chars: {}", val);
        // No vowels allowed in SEDOL
        assert!(
            !val[..6].contains(['A', 'E', 'I', 'O', 'U']),
            "SEDOL should not contain vowels: {}",
            val
        );
        let body = &val[..6];
        let expected = gen.sedol_check_digit(body);
        let actual = val.chars().last().unwrap().to_digit(10).unwrap() as u8;
        assert_eq!(
            actual, expected,
            "SEDOL {} has invalid check digit (expected {}, got {})",
            val, expected, actual
        );
    }
}

#[test]
fn test_swift_bic_format() {
    let mut gen = Generator::with_seed(test_taxonomy(), 42);
    let mut saw_8char = false;
    let mut saw_11char = false;
    for _ in 0..100 {
        let val = gen.generate_value("finance.banking.swift_bic").unwrap();
        assert!(
            val.len() == 8 || val.len() == 11,
            "SWIFT/BIC should be 8 or 11 chars: {} (len={})",
            val,
            val.len()
        );
        // First 4 chars must be letters (bank code)
        assert!(
            val[..4].chars().all(|c| c.is_ascii_uppercase()),
            "SWIFT bank code should be uppercase letters: {}",
            val
        );
        // Chars 5-6 must be letters (country code)
        assert!(
            val[4..6].chars().all(|c| c.is_ascii_uppercase()),
            "SWIFT country code should be uppercase letters: {}",
            val
        );
        if val.len() == 8 {
            saw_8char = true;
        }
        if val.len() == 11 {
            saw_11char = true;
        }
    }
    assert!(saw_8char, "Should generate 8-char SWIFT codes");
    assert!(saw_11char, "Should generate 11-char SWIFT codes");
}

#[test]
fn test_lei_check_digits_valid() {
    let mut gen = Generator::with_seed(test_taxonomy(), 42);
    for _ in 0..100 {
        let val = gen.generate_value("finance.securities.lei").unwrap();
        assert_eq!(val.len(), 20, "LEI should be 20 chars: {}", val);
        // Verify check digits by recomputing
        let body = &val[..18];
        let expected = gen.lei_check_digits(body);
        let actual = &val[18..];
        assert_eq!(
            actual, expected,
            "LEI {} has invalid check digits (expected {}, got {})",
            val, expected, actual
        );
    }
}

#[test]
fn test_currency_code_format() {
    let mut gen = Generator::with_seed(test_taxonomy(), 42);
    for _ in 0..50 {
        let val = gen
            .generate_value("finance.currency.currency_code")
            .unwrap();
        assert_eq!(val.len(), 3, "Currency code should be 3 chars: {}", val);
        assert!(
            val.chars().all(|c| c.is_ascii_uppercase()),
            "Currency code should be uppercase: {}",
            val
        );
    }
}

#[test]
fn test_currency_symbol_format() {
    let mut gen = Generator::with_seed(test_taxonomy(), 42);
    for _ in 0..50 {
        let val = gen
            .generate_value("finance.currency.currency_symbol")
            .unwrap();
        assert!(!val.is_empty(), "Currency symbol should not be empty");
        // Should be short (1-3 chars typically)
        assert!(
            val.len() <= 4,
            "Currency symbol should be short: {} (len={})",
            val,
            val.len()
        );
    }
}

// ── New format coverage tests ─────────────────────────

#[test]
fn test_chinese_ymd() {
    let mut gen = Generator::with_seed(test_taxonomy(), 42);
    for _ in 0..20 {
        let val = gen.generate_value("datetime.date.chinese_ymd").unwrap();
        assert!(val.contains('年'), "Should contain 年: {}", val);
        assert!(val.contains('月'), "Should contain 月: {}", val);
        assert!(val.contains('日'), "Should contain 日: {}", val);
    }
}

#[test]
fn test_korean_ymd() {
    let mut gen = Generator::with_seed(test_taxonomy(), 42);
    for _ in 0..20 {
        let val = gen.generate_value("datetime.date.korean_ymd").unwrap();
        assert!(val.contains('년'), "Should contain 년: {}", val);
        assert!(val.contains('월'), "Should contain 월: {}", val);
        assert!(val.contains('일'), "Should contain 일: {}", val);
    }
}

#[test]
fn test_jp_era_short() {
    let mut gen = Generator::with_seed(test_taxonomy(), 42);
    for _ in 0..20 {
        let val = gen.generate_value("datetime.date.jp_era_short").unwrap();
        // Should start with era letter (R, H, S, T, or M)
        let first = val.chars().next().unwrap();
        assert!(
            "RHSTM".contains(first),
            "Should start with era letter: {}",
            val
        );
        // Should contain slashes
        assert_eq!(
            val.matches('/').count(),
            2,
            "Should have 2 slashes: {}",
            val
        );
    }
}

#[test]
fn test_jp_era_long() {
    let mut gen = Generator::with_seed(test_taxonomy(), 42);
    for _ in 0..20 {
        let val = gen.generate_value("datetime.date.jp_era_long").unwrap();
        assert!(val.contains('年'), "Should contain 年: {}", val);
        assert!(val.contains('月'), "Should contain 月: {}", val);
        assert!(val.contains('日'), "Should contain 日: {}", val);
        // Should start with an era name
        let starts_with_era = val.starts_with("令和")
            || val.starts_with("平成")
            || val.starts_with("昭和")
            || val.starts_with("大正")
            || val.starts_with("明治");
        assert!(starts_with_era, "Should start with era name: {}", val);
    }
}

#[test]
fn test_jp_era_offset_correctness() {
    let gen = Generator::with_seed(test_taxonomy(), 42);
    // R6 → 2024 (2018 + 6 = 2024)
    assert_eq!(gen.gregorian_to_jp_era(2024), ("R", 6));
    // R1 → 2019 (2018 + 1 = 2019)
    assert_eq!(gen.gregorian_to_jp_era(2019), ("R", 1));
    // H31 → 2019 boundary: 1988 + 31 = 2019, but Reiwa starts 2019
    // Our implementation: year >= 2019 → R, so H31 doesn't exist (correct for post-April)
    assert_eq!(gen.gregorian_to_jp_era(2019), ("R", 1));
    // H1 → 1989
    assert_eq!(gen.gregorian_to_jp_era(1989), ("H", 1));
    // S64 → 1989, but since 1989 >= 1989 → H1
    assert_eq!(gen.gregorian_to_jp_era(1989), ("H", 1));
    // S1 → 1926
    assert_eq!(gen.gregorian_to_jp_era(1926), ("S", 1));
}

#[test]
fn test_amount_accounting() {
    let mut gen = Generator::with_seed(test_taxonomy(), 42);
    for _ in 0..20 {
        let val = gen
            .generate_value("finance.currency.amount_accounting")
            .unwrap();
        assert!(val.contains('$'), "Should contain $: {}", val);
        // Parentheses or regular format
        let is_negative = val.starts_with('(') && val.ends_with(')');
        let is_positive = val.starts_with('$');
        assert!(
            is_negative || is_positive,
            "Should be ($X) or $X format: {}",
            val
        );
    }
}

#[test]
fn test_amount_lakh() {
    let mut gen = Generator::with_seed(test_taxonomy(), 42);
    for _ in 0..20 {
        let val = gen.generate_value("finance.currency.amount_lakh").unwrap();
        assert!(
            val.contains('₹') || val.starts_with("Rs"),
            "Should contain ₹ or Rs: {}",
            val
        );
    }
}

#[test]
fn test_indian_grouping_helper() {
    assert_eq!(Generator::format_indian_grouping(1234567), "12,34,567");
    assert_eq!(Generator::format_indian_grouping(100000), "1,00,000");
    assert_eq!(Generator::format_indian_grouping(999), "999");
    assert_eq!(Generator::format_indian_grouping(1000), "1,000");
    assert_eq!(Generator::format_indian_grouping(10000), "10,000");
    assert_eq!(Generator::format_indian_grouping(100000000), "10,00,00,000");
}

#[test]
fn test_format_int_with_separator() {
    assert_eq!(
        Generator::format_int_with_separator(1234567, ','),
        "1,234,567"
    );
    assert_eq!(
        Generator::format_int_with_separator(1234567, '.'),
        "1.234.567"
    );
    assert_eq!(
        Generator::format_int_with_separator(1234567, '\''),
        "1'234'567"
    );
    assert_eq!(Generator::format_int_with_separator(999, ','), "999");
    assert_eq!(Generator::format_int_with_separator(0, ','), "0");
}

#[test]
fn test_amount_crypto() {
    let mut gen = Generator::with_seed(test_taxonomy(), 42);
    for _ in 0..20 {
        let val = gen
            .generate_value("finance.currency.amount_crypto")
            .unwrap();
        let tickers = ["BTC", "ETH", "SOL", "DOGE", "XRP", "ADA"];
        let has_ticker = tickers.iter().any(|t| val.contains(t));
        assert!(has_ticker, "Should contain a crypto ticker: {}", val);
        assert!(val.contains('.'), "Should have decimal: {}", val);
    }
}

#[test]
fn test_yield_generator() {
    let mut gen = Generator::with_seed(test_taxonomy(), 42);
    for _ in 0..20 {
        let val = gen.generate_value("finance.rate.yield").unwrap();
        assert!(val.ends_with('%'), "Should end with %: {}", val);
        let first = val.chars().next().unwrap();
        assert!(
            first == '+' || first == '-',
            "Should start with +/-: {}",
            val
        );
    }
}

#[test]
fn test_basis_points() {
    let mut gen = Generator::with_seed(test_taxonomy(), 42);
    for _ in 0..20 {
        let val = gen.generate_value("finance.rate.basis_points").unwrap();
        assert!(val.contains("bps"), "Should contain bps: {}", val);
    }
}

#[test]
fn test_clf_timestamp() {
    let mut gen = Generator::with_seed(test_taxonomy(), 42);
    for _ in 0..20 {
        let val = gen.generate_value("datetime.timestamp.clf").unwrap();
        // Format: 15/Jan/2024:14:30:00 +0000
        assert!(val.contains(':'), "Should contain colon: {}", val);
        assert!(val.contains('/'), "Should contain slash: {}", val);
    }
}

#[test]
fn test_syslog_bsd_timestamp() {
    let mut gen = Generator::with_seed(test_taxonomy(), 42);
    for _ in 0..20 {
        let val = gen.generate_value("datetime.timestamp.syslog_bsd").unwrap();
        // Format: Jan 15 14:30:00 (no year)
        assert!(val.contains(':'), "Should contain time: {}", val);
        // Should NOT contain a 4-digit year
        let has_year = val
            .split_whitespace()
            .any(|w| w.len() == 4 && w.chars().all(|c| c.is_ascii_digit()));
        assert!(!has_year, "Should not contain year: {}", val);
    }
}

#[test]
fn test_epoch_nanoseconds() {
    let mut gen = Generator::with_seed(test_taxonomy(), 42);
    for _ in 0..20 {
        let val = gen
            .generate_value("datetime.timestamp.epoch_nanoseconds")
            .unwrap();
        assert!(
            val.len() >= 18 && val.len() <= 19,
            "Should be 18-19 digits: {} (len={})",
            val,
            val.len()
        );
        assert!(
            val.chars().all(|c| c.is_ascii_digit()),
            "Should be all digits: {}",
            val
        );
    }
}

#[test]
fn test_quarter() {
    let mut gen = Generator::with_seed(test_taxonomy(), 42);
    for _ in 0..20 {
        let val = gen.generate_value("datetime.period.quarter").unwrap();
        assert!(val.contains('Q'), "Should contain Q: {}", val);
        // Either "Q1 2024" or "2024-Q1"
        let has_q_num =
            val.contains("Q1") || val.contains("Q2") || val.contains("Q3") || val.contains("Q4");
        assert!(has_q_num, "Should have Q1-Q4: {}", val);
    }
}

#[test]
fn test_fiscal_year() {
    let mut gen = Generator::with_seed(test_taxonomy(), 42);
    for _ in 0..20 {
        let val = gen.generate_value("datetime.period.fiscal_year").unwrap();
        assert!(val.starts_with("FY"), "Should start with FY: {}", val);
    }
}

#[test]
fn test_all_54_new_generators_produce_output() {
    let mut gen = Generator::with_seed(test_taxonomy(), 42);
    let new_types = [
        // 23 date types
        "datetime.date.ymd_slash",
        "datetime.date.ymd_dot",
        "datetime.date.dmy_dash",
        "datetime.date.mdy_dash",
        "datetime.date.mdy_short_slash",
        "datetime.date.dmy_short_slash",
        "datetime.date.dmy_short_dot",
        "datetime.date.dmy_space_abbrev",
        "datetime.date.dmy_space_full",
        "datetime.date.abbrev_month_no_comma",
        "datetime.date.full_month_no_comma",
        "datetime.date.dmy_dash_abbrev",
        "datetime.date.dmy_dash_abbrev_short",
        "datetime.date.year_month",
        "datetime.date.compact_ym",
        "datetime.date.month_year_full",
        "datetime.date.month_year_abbrev",
        "datetime.date.month_year_slash",
        "datetime.date.weekday_dmy_full",
        "datetime.date.chinese_ymd",
        "datetime.date.korean_ymd",
        "datetime.date.jp_era_short",
        "datetime.date.jp_era_long",
        // 2 period types
        "datetime.period.quarter",
        "datetime.period.fiscal_year",
        // 16 timestamp types
        "datetime.timestamp.sql_microseconds",
        "datetime.timestamp.sql_milliseconds",
        "datetime.timestamp.iso_8601_milliseconds",
        "datetime.timestamp.iso_8601_millis_offset",
        "datetime.timestamp.iso_8601_micros_offset",
        "datetime.timestamp.clf",
        "datetime.timestamp.syslog_bsd",
        "datetime.timestamp.sql_microseconds_offset",
        "datetime.timestamp.pg_short_offset",
        "datetime.timestamp.dot_dmy_24h",
        "datetime.timestamp.slash_ymd_24h",
        "datetime.timestamp.ctime",
        "datetime.timestamp.epoch_nanoseconds",
        "datetime.timestamp.iso_space_zulu",
        "datetime.timestamp.dot_ymd_24h",
        // 11 currency types
        "finance.currency.amount_accounting",
        "finance.currency.amount_comma_suffix",
        "finance.currency.amount_space",
        "finance.currency.amount_lakh",
        "finance.currency.amount_apostrophe",
        "finance.currency.amount_nodecimal",
        "finance.currency.amount_code_prefix",
        "finance.currency.amount_crypto",
        "finance.currency.amount_multisym",
        "finance.currency.amount_neg_trailing",
        // 2 rate types
        "finance.rate.basis_points",
        "finance.rate.yield",
    ];

    assert_eq!(new_types.len(), 52, "Should have exactly 52 new types");

    for type_key in &new_types {
        let result = gen.generate_value(type_key);
        assert!(
            result.is_ok(),
            "Generator should succeed for {}: {:?}",
            type_key,
            result.err()
        );
        let val = result.unwrap();
        assert!(
            !val.is_empty(),
            "Output should not be empty for {}",
            type_key
        );
    }
}

// ── v17 ac-02: generator uniqueness + structural bars ──────────────────
// Each test draws (target + 200) samples, asserts ≥target unique, and
// spot-checks structural invariants on every sample.

fn collect_samples(key: &str, n: usize) -> Vec<String> {
    let mut gen = Generator::with_seed(test_taxonomy(), 42);
    (0..n).map(|_| gen.generate_value(key).unwrap()).collect()
}

#[test]
#[ignore = "printer only; run with --ignored --nocapture to see samples"]
fn ac02_print_samples() {
    for key in [
        "finance.banking.swift_bic",
        "identity.medical.cpt",
        "representation.file.excel_format",
        "identity.government.ssn",
    ] {
        println!("=== {} ===", key);
        let mut gen = Generator::with_seed(test_taxonomy(), 42);
        for _ in 0..12 {
            println!("  {}", gen.generate_value(key).unwrap());
        }
    }
}

#[test]
fn ac02_swift_bic_unique_and_structured() {
    let samples = collect_samples("finance.banking.swift_bic", 1200);
    let unique: std::collections::HashSet<_> = samples.iter().cloned().collect();
    assert!(
        unique.len() >= 1000,
        "swift_bic: expected ≥1000 unique, got {}",
        unique.len()
    );
    for s in &samples {
        assert!(
            s.len() == 8 || s.len() == 11,
            "swift_bic length must be 8 or 11, got {:?}",
            s
        );
        let bytes = s.as_bytes();
        // Positions 1-4: bank code (A-Z)
        assert!(
            bytes[0..4].iter().all(|b| b.is_ascii_uppercase()),
            "bank code must be A-Z: {}",
            s
        );
        // Positions 5-6: country (A-Z)
        assert!(
            bytes[4..6].iter().all(|b| b.is_ascii_uppercase()),
            "country code must be A-Z: {}",
            s
        );
        // Positions 7-8 (and 9-11 if branch): alphanumeric
        assert!(
            bytes[6..].iter().all(|b| b.is_ascii_alphanumeric()),
            "location/branch must be alphanumeric: {}",
            s
        );
    }
}

#[test]
fn ac02_cpt_unique_and_structured() {
    let samples = collect_samples("identity.medical.cpt", 1200);
    let unique: std::collections::HashSet<_> = samples.iter().cloned().collect();
    assert!(
        unique.len() >= 1000,
        "cpt: expected ≥1000 unique, got {}",
        unique.len()
    );
    let mut saw_cat1 = false;
    let mut saw_cat2 = false;
    let mut saw_cat3 = false;
    for s in &samples {
        if s.len() == 5 && s.chars().all(|c| c.is_ascii_digit()) {
            saw_cat1 = true;
        } else if s.len() == 5 && s.ends_with('F') {
            saw_cat2 = true;
        } else if s.len() == 5 && s.ends_with('T') {
            saw_cat3 = true;
        } else if s.len() == 5 && s.ends_with('U') {
            // U suffix permitted by YAML pattern.
        } else {
            panic!("Unexpected CPT shape: {:?}", s);
        }
    }
    assert!(saw_cat1, "should see Category I (5-digit) codes");
    assert!(saw_cat2, "should see Category II (F-suffix) codes");
    assert!(saw_cat3, "should see Category III (T-suffix) codes");
}

#[test]
fn ac02_excel_format_unique_and_structured() {
    // Excel's legitimate format-string vocabulary is naturally smaller
    // than SWIFT/CPT/SSN (the grammar is denser), so we draw a larger
    // sample pool to reach ≥500 unique without biasing the generator
    // toward artificial noise.
    let samples = collect_samples("representation.file.excel_format", 2000);
    let unique: std::collections::HashSet<_> = samples.iter().cloned().collect();
    assert!(
        unique.len() >= 500,
        "excel_format: expected ≥500 unique, got {}",
        unique.len()
    );
    for s in &samples {
        assert!(!s.is_empty(), "format must be non-empty");
        assert!(s.len() <= 100, "format must be ≤100 chars: {:?}", s);
    }
}

#[test]
fn ac02_ssn_unique_and_structured() {
    let samples = collect_samples("identity.government.ssn", 1200);
    let unique: std::collections::HashSet<_> = samples.iter().cloned().collect();
    assert!(
        unique.len() >= 1000,
        "ssn: expected ≥1000 unique, got {}",
        unique.len()
    );
    let mut saw_dashed = false;
    let mut saw_plain = false;
    for s in &samples {
        let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
        assert_eq!(digits.len(), 9, "ssn must have 9 digits: {:?}", s);
        let area: u32 = digits[0..3].parse().unwrap();
        let group: u32 = digits[3..5].parse().unwrap();
        let serial: u32 = digits[5..9].parse().unwrap();
        assert!(
            area != 0 && area != 666 && area < 900,
            "invalid area: {}",
            s
        );
        assert!(group != 0, "invalid group: {}", s);
        assert!(serial != 0, "invalid serial: {}", s);
        if s.contains('-') {
            saw_dashed = true;
            assert_eq!(s.len(), 11, "dashed ssn must be 11 chars: {:?}", s);
        } else {
            saw_plain = true;
            assert_eq!(s.len(), 9, "no-dash ssn must be 9 chars: {:?}", s);
        }
    }
    assert!(saw_dashed, "should see dashed ssns");
    assert!(saw_plain, "should see no-dash ssns");
}
