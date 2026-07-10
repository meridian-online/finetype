use super::super::*;

// ── Attractor demotion tests (Rule 14) ──────────────────────────────

// Was testing street_number demotion which no longer exists.

#[test]
fn test_attractor_confidence_demotion() {
    // Low confidence postal_code prediction (0.6) — should demote
    let values: Vec<String> = vec![
        "1500", "2300", "45000", "800", "99", "12", "5600", "340", "78", "4100",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let votes = vec![
        ("geography.address.postal_code".to_string(), 6),
        ("representation.numeric.integer_number".to_string(), 4),
    ];

    let result = disambiguate_attractor_demotion(&values, &votes, 10, None);
    assert!(
        result.is_some(),
        "Should demote postal_code at 0.6 confidence"
    );
    let (label, rule) = result.unwrap();
    assert_eq!(label, "representation.numeric.integer_number");
    assert!(rule.starts_with("attractor_demotion_confidence:"));
}

#[test]
fn test_attractor_cardinality_demotion() {
    // 4 unique short words classified as first_name at high confidence — categorical
    let values: Vec<String> = vec![
        "Soccer", "Baseball", "Tennis", "Hockey", "Soccer", "Baseball", "Tennis", "Hockey",
        "Soccer", "Baseball",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let votes = vec![
        ("identity.person.first_name".to_string(), 9),
        ("representation.text.word".to_string(), 1),
    ];

    // High confidence (0.9) — Signal 2 won't fire, but Signal 3 (cardinality) should
    let result = disambiguate_attractor_demotion(&values, &votes, 10, None);
    assert!(
        result.is_some(),
        "Should demote first_name with 4 unique values"
    );
    let (label, rule) = result.unwrap();
    assert_eq!(label, "representation.text.word");
    assert!(rule.starts_with("attractor_demotion_cardinality:"));
}

#[test]
fn test_attractor_cardinality_single_value() {
    // Single unique value (e.g., airports.type = "airport" repeated) — categorical
    let values: Vec<String> = vec![
        "airport", "airport", "airport", "airport", "airport", "airport", "airport", "airport",
        "airport", "airport",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let votes = vec![("identity.person.first_name".to_string(), 10)];

    // Cardinality 1 — strongest signal that this is NOT a person's name
    let result = disambiguate_attractor_demotion(&values, &votes, 10, None);
    assert!(
        result.is_some(),
        "Should demote first_name with 1 unique value"
    );
    let (label, rule) = result.unwrap();
    assert_eq!(label, "representation.text.word");
    assert!(rule.starts_with("attractor_demotion_cardinality:"));
}

#[test]
fn test_attractor_shape_only_pattern_does_not_confirm() {
    // company-reference audit W2 item 9: a shape-only pattern (`^[A-Z]{4}$`)
    // confirms every 4-letter token, so it must NOT disarm the attractor
    // demotion. Real ICAO codes at low confidence (0.6 < 0.85) with only the
    // shape pattern to vouch for them are now demoted via Signal 2 — in
    // production the `membership: icao_airports` guard is what keeps genuine
    // airport columns, not this shape pattern.
    let values: Vec<String> = vec!["EGLL", "KJFK", "LFPG", "EDDF", "RJTT", "VHHH"]
        .into_iter()
        .map(String::from)
        .collect();
    let votes = vec![
        ("geography.transportation.icao_code".to_string(), 6),
        ("representation.identifier.alphanumeric_id".to_string(), 4),
    ];

    let yaml = r#"
geography.transportation.icao_code:
  title: "ICAO Code"
  validation:
    type: string
    pattern: "^[A-Z]{4}$"
    minLength: 4
    maxLength: 4
  tier: [VARCHAR, geography, transportation]
  release_priority: 5
  samples: ["EGLL"]
"#;
    let taxonomy = Taxonomy::from_yaml(yaml).unwrap();

    let result = disambiguate_attractor_demotion(&values, &votes, 10, Some(&taxonomy));
    assert!(
        result.is_some(),
        "a shape-only pattern must not protect a low-confidence attractor"
    );
    assert!(
        result.unwrap().1.contains("icao_code"),
        "the demotion should record the attractor label it fired on"
    );
}

#[test]
fn test_attractor_precise_validator_confirms_skips_signal2() {
    // The confirmation short-circuit survives for GENUINELY precise validators:
    // a closed enum is positive evidence and still gates Signal 2. Only
    // shape-only patterns lost their power to confirm (W2 item 9).
    let values: Vec<String> = vec!["EGLL", "KJFK", "LFPG", "EDDF", "RJTT", "VHHH"]
        .into_iter()
        .map(String::from)
        .collect();
    let votes = vec![
        ("geography.transportation.icao_code".to_string(), 6),
        ("representation.identifier.alphanumeric_id".to_string(), 4),
    ];

    let yaml = r#"
geography.transportation.icao_code:
  title: "ICAO Code"
  validation:
    type: string
    enum: ["EGLL", "KJFK", "LFPG", "EDDF", "RJTT", "VHHH"]
  tier: [VARCHAR, geography, transportation]
  release_priority: 5
  samples: ["EGLL"]
"#;
    let taxonomy = Taxonomy::from_yaml(yaml).unwrap();

    // Confidence 0.6 < 0.85 → Signal 2 would fire, BUT the enum validator is
    // precise and passes → validation_confirmed = true → Signal 2 skipped.
    let result = disambiguate_attractor_demotion(&values, &votes, 10, Some(&taxonomy));
    assert!(
        result.is_none(),
        "a precise enum validator must still confirm and skip demotion"
    );
}

// ── Rule 32: closed-set over-emit demotion (earthquake-roundtrip ac-03) ──

#[test]
fn test_schema_fail_demotion_measurement_unit_to_categorical() {
    // net/locationSource/magSource: short network codes the model labels
    // measurement_unit (a closed SI-unit enum). None is an SI unit → 100%
    // fail the enum → demote to categorical (low cardinality).
    let values: Vec<String> = vec!["us", "ci", "nc", "us", "nn", "ci", "us", "nc"]
        .into_iter()
        .map(String::from)
        .collect();
    let yaml = r#"
representation.scientific.measurement_unit:
  title: "Measurement Unit"
  validation:
    type: string
    enum: [meter, kilogram, second, m, kg, s]
  tier: [VARCHAR, scientific]
  release_priority: 3
  samples: ["meter"]
"#;
    let taxonomy = Taxonomy::from_yaml(yaml).unwrap();
    let result = value_sharpen(
        &values,
        "representation.scientific.measurement_unit",
        0.9,
        Some(&taxonomy),
    );
    let (label, rule) = result.expect("network codes should demote off measurement_unit");
    assert_eq!(label, "representation.text.word");
    assert!(rule.starts_with("schema_fail_demotion:"));
}

#[test]
fn test_schema_fail_demotion_geohash_to_alphanumeric_id() {
    // id: unique event identifiers the model labels geohash. The geohash
    // base32 alphabet excludes a/i/l/o, so network-prefixed ids ("ci…")
    // fail the pattern → demote to alphanumeric_id (high cardinality).
    let values: Vec<String> = (0..30).map(|i| format!("ci40{:06}", i)).collect();
    let yaml = r#"
geography.coordinate.geohash:
  title: "Geohash"
  validation:
    type: string
    pattern: "^[0-9b-hjkmnp-z]{6,12}$"
  tier: [VARCHAR, geography, coordinate]
  release_priority: 5
  samples: ["u4pruyd"]
"#;
    let taxonomy = Taxonomy::from_yaml(yaml).unwrap();
    let result = value_sharpen(
        &values,
        "geography.coordinate.geohash",
        0.9,
        Some(&taxonomy),
    );
    let (label, rule) = result.expect("event ids should demote off geohash");
    assert_eq!(label, "representation.identifier.alphanumeric_id");
    assert!(rule.starts_with("schema_fail_demotion:"));
}

#[test]
fn test_schema_fail_demotion_partial_fail_kept() {
    // A column only ~17% of whose values fail the geohash pattern is NOT demoted.
    // Genuine geohash columns fail their own (v13 min-6) pattern ~23% on legitimate
    // short geohashes, and real unit columns fail the incomplete SI enum ~44%; the
    // >50% bar spares them. Only overwhelming failure (network codes that are 100%
    // non-units) demotes. Locks the bar against over-demoting genuine columns —
    // the earthquake `id` column (~28% fail) is intentionally left as geohash here
    // rather than risk regressing real short-geohash detection (the v13 trade-off).
    let mut values: Vec<String> = (0..25).map(|i| format!("bcd{:03}", i)).collect();
    values.extend((0..5).map(|i| format!("icd{:03}", i))); // 'i' ∉ base32 → fail
    let yaml = r#"
geography.coordinate.geohash:
  title: "Geohash"
  validation:
    type: string
    pattern: "^[0-9b-hjkmnp-z]{6,12}$"
  tier: [VARCHAR, geography, coordinate]
  release_priority: 5
  samples: ["u4pruyd"]
"#;
    let taxonomy = Taxonomy::from_yaml(yaml).unwrap();
    let result = value_sharpen(
        &values,
        "geography.coordinate.geohash",
        0.9,
        Some(&taxonomy),
    );
    assert!(
        result.is_none(),
        "partial-fail column must be kept, not demoted (>50% bar spares genuine columns)"
    );
}

#[test]
fn test_schema_fail_demotion_keeps_real_geohash() {
    // A genuine geohash column (all values valid base32) must NOT be demoted.
    let values: Vec<String> = (0..30).map(|i| format!("bcd{:03}", i)).collect();
    let yaml = r#"
geography.coordinate.geohash:
  title: "Geohash"
  validation:
    type: string
    pattern: "^[0-9b-hjkmnp-z]{6,12}$"
  tier: [VARCHAR, geography, coordinate]
  release_priority: 5
  samples: ["u4pruyd"]
"#;
    let taxonomy = Taxonomy::from_yaml(yaml).unwrap();
    let result = value_sharpen(
        &values,
        "geography.coordinate.geohash",
        0.9,
        Some(&taxonomy),
    );
    assert!(
        result.is_none(),
        "valid geohash values must not be demoted off geohash"
    );
}

#[test]
fn test_schema_fail_demotion_keeps_real_units() {
    // A genuine units column whose values ARE SI units must NOT be demoted —
    // the rule fires on schema-validation failure, not on the label alone.
    let values: Vec<String> = vec!["meter", "kg", "second", "m", "kg", "meter"]
        .into_iter()
        .map(String::from)
        .collect();
    let yaml = r#"
representation.scientific.measurement_unit:
  title: "Measurement Unit"
  validation:
    type: string
    enum: [meter, kilogram, second, m, kg, s]
  tier: [VARCHAR, scientific]
  release_priority: 3
  samples: ["meter"]
"#;
    let taxonomy = Taxonomy::from_yaml(yaml).unwrap();
    let result = value_sharpen(
        &values,
        "representation.scientific.measurement_unit",
        0.9,
        Some(&taxonomy),
    );
    assert!(
        result.is_none(),
        "real SI-unit values must not be demoted off measurement_unit"
    );
}

#[test]
fn test_schema_fail_demotion_utc_integers_to_categorical() {
    // Plain integers the Sense stage over-emits as datetime.offset.utc. None
    // matches the `UTC +HH:MM` pattern → 100% fail → demote (low cardinality →
    // categorical). This is the corpus utc over-emit the v24 retrain made worse.
    let values: Vec<String> = vec!["5", "100", "23", "5", "100", "7", "23", "5"]
        .into_iter()
        .map(String::from)
        .collect();
    let yaml = r#"
datetime.offset.utc:
  title: "UTC Offset"
  validation:
    type: string
    pattern: "^UTC [+-]\\d{2}:\\d{2}$"
  tier: [VARCHAR, datetime, offset]
  release_priority: 4
  samples: ["UTC +05:00"]
"#;
    let taxonomy = Taxonomy::from_yaml(yaml).unwrap();
    let result = value_sharpen(&values, "datetime.offset.utc", 0.9, Some(&taxonomy));
    let (label, rule) = result.expect("integers should demote off datetime.offset.utc");
    assert_eq!(label, "representation.text.word");
    assert!(rule.starts_with("schema_fail_demotion:"));
}

#[test]
fn test_schema_fail_demotion_keeps_real_utc() {
    // A genuine UTC-offset column (all values match the pattern) must NOT be
    // demoted — the regression guard for widening the allowlist to utc.
    let values: Vec<String> = vec![
        "UTC +05:00",
        "UTC -08:00",
        "UTC +00:00",
        "UTC +05:30",
        "UTC -03:00",
        "UTC +09:00",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let yaml = r#"
datetime.offset.utc:
  title: "UTC Offset"
  validation:
    type: string
    pattern: "^UTC [+-]\\d{2}:\\d{2}$"
  tier: [VARCHAR, datetime, offset]
  release_priority: 4
  samples: ["UTC +05:00"]
"#;
    let taxonomy = Taxonomy::from_yaml(yaml).unwrap();
    let result = value_sharpen(&values, "datetime.offset.utc", 0.9, Some(&taxonomy));
    assert!(
        result.is_none(),
        "real UTC-offset values must not be demoted off datetime.offset.utc"
    );
}

#[test]
fn test_schema_fail_demotion_url_codes_to_alphanumeric_id() {
    // Bare ids/codes the Sense stage over-emits as technology.internet.url. None
    // has a scheme:// → 100% fail → demote (high cardinality → alphanumeric_id).
    let values: Vec<String> = (0..30).map(|i| format!("ABC{:05}", i)).collect();
    let yaml = r#"
technology.internet.url:
  title: "URL"
  validation:
    type: string
    pattern: "^(?:https?|ftp|file)://[^\\s]+$"
  tier: [VARCHAR, technology, internet]
  release_priority: 4
  samples: ["https://example.com"]
"#;
    let taxonomy = Taxonomy::from_yaml(yaml).unwrap();
    let result = value_sharpen(&values, "technology.internet.url", 0.9, Some(&taxonomy));
    let (label, rule) = result.expect("bare codes should demote off technology.internet.url");
    assert_eq!(label, "representation.identifier.alphanumeric_id");
    assert!(rule.starts_with("schema_fail_demotion:"));
}

#[test]
fn test_schema_fail_demotion_keeps_real_url() {
    // A genuine URL column (all values have a valid scheme://) must NOT be
    // demoted — the regression guard for widening the allowlist to url.
    let values: Vec<String> = vec![
        "https://example.com",
        "http://foo.org/path",
        "https://a.b.c/x?y=1",
        "ftp://files.example.com",
        "https://news.site/article",
        "http://localhost:8080",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let yaml = r#"
technology.internet.url:
  title: "URL"
  validation:
    type: string
    pattern: "^(?:https?|ftp|file)://[^\\s]+$"
  tier: [VARCHAR, technology, internet]
  release_priority: 4
  samples: ["https://example.com"]
"#;
    let taxonomy = Taxonomy::from_yaml(yaml).unwrap();
    let result = value_sharpen(&values, "technology.internet.url", 0.9, Some(&taxonomy));
    assert!(
        result.is_none(),
        "real URL values must not be demoted off technology.internet.url"
    );
}

#[test]
fn test_schema_fail_demotion_prose_off_wkt() {
    // Free-text descriptions the Sense stage over-emits as geography.format.wkt
    // (company-reference audit W1a). wkt is absent from veto_safe.txt (rare-type
    // starvation) so before this widening a contradicted wkt assertion shipped
    // with only an advisory flag. Prose fails the leading-geometry-keyword
    // pattern ~100% → demote (high cardinality → alphanumeric_id, which the
    // validation veto then adjudicates honestly downstream).
    let values: Vec<String> = (0..30)
        .map(|i| format!("Provides consulting and advisory services to sector {}", i))
        .collect();
    let yaml = r#"
geography.format.wkt:
  title: "Well-Known Text Geometry"
  validation:
    type: string
    pattern: "^(POINT|LINESTRING|POLYGON|MULTI(POINT|LINESTRING|POLYGON)|GEOMETRYCOLLECTION)\\s*(Z|M|ZM)?\\s*(\\(|EMPTY)"
  tier: [VARCHAR, format]
  release_priority: 3
  samples: ["POINT (30 10)"]
"#;
    let taxonomy = Taxonomy::from_yaml(yaml).unwrap();
    let result = value_sharpen(&values, "geography.format.wkt", 0.9, Some(&taxonomy));
    let (label, rule) = result.expect("prose should demote off geography.format.wkt");
    assert_eq!(label, "representation.identifier.alphanumeric_id");
    assert!(rule.starts_with("schema_fail_demotion:"));
}

#[test]
fn test_schema_fail_demotion_keeps_real_wkt() {
    // A genuine WKT column (all values lead with a geometry keyword) must NOT
    // be demoted — the regression guard for widening the allowlist to wkt.
    let values: Vec<String> = vec![
        "POINT (30 10)",
        "LINESTRING (30 10, 10 30, 40 40)",
        "POLYGON ((30 10, 40 40, 20 40, 10 20, 30 10))",
        "MULTIPOINT ((10 40), (40 30))",
        "POINT Z (1 2 3)",
        "GEOMETRYCOLLECTION EMPTY",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let yaml = r#"
geography.format.wkt:
  title: "Well-Known Text Geometry"
  validation:
    type: string
    pattern: "^(POINT|LINESTRING|POLYGON|MULTI(POINT|LINESTRING|POLYGON)|GEOMETRYCOLLECTION)\\s*(Z|M|ZM)?\\s*(\\(|EMPTY)"
  tier: [VARCHAR, format]
  release_priority: 3
  samples: ["POINT (30 10)"]
"#;
    let taxonomy = Taxonomy::from_yaml(yaml).unwrap();
    let result = value_sharpen(&values, "geography.format.wkt", 0.9, Some(&taxonomy));
    assert!(
        result.is_none(),
        "real WKT values must not be demoted off geography.format.wkt"
    );
}

#[test]
fn test_schema_fail_demotion_junk_off_user_agent() {
    // Generic id/version columns the shipped default over-emits as
    // technology.internet.user_agent (the model's largest single over-emit,
    // 359/13,478 corpus columns — company-reference audit W1a). None carries a
    // known client prefix → 100% fail → demote (high cardinality →
    // alphanumeric_id).
    let values: Vec<String> = (0..30)
        .map(|i| format!("app-build-{:04}.release", i))
        .collect();
    let yaml = r#"
technology.internet.user_agent:
  title: "User Agent String"
  validation:
    type: string
    pattern: "^(Mozilla/|curl/|python-requests/|Wget/|Go-http-client/|axios/|PostmanRuntime/|kube-probe/|Java/|okhttp/|Apache-HttpClient/|libcurl/|node-fetch/|Dalvik/|CFNetwork/|Lynx/|Links |Scrapy/|Googlebot/|Bingbot/|Slackbot|Twitterbot/|facebookexternalhit/|LinkedInBot/|Prometheus/|Datadog/|Ruby/|Dart/|grpc-|HTTPie/|bot|spider|crawl)"
    minLength: 10
    maxLength: 500
  tier: [VARCHAR, internet]
  release_priority: 3
  samples: ["curl/7.64.1"]
"#;
    let taxonomy = Taxonomy::from_yaml(yaml).unwrap();
    let result = value_sharpen(
        &values,
        "technology.internet.user_agent",
        0.9,
        Some(&taxonomy),
    );
    let (label, rule) = result.expect("bare build ids should demote off user_agent");
    assert_eq!(label, "representation.identifier.alphanumeric_id");
    assert!(rule.starts_with("schema_fail_demotion:"));
}

#[test]
fn test_schema_fail_demotion_keeps_real_user_agent() {
    // A genuine user-agent column (known client prefixes) must NOT be demoted —
    // the regression guard for widening the allowlist to user_agent.
    let values: Vec<String> = vec![
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        "Mozilla/5.0 (iPhone; CPU iPhone OS 15_0 like Mac OS X)",
        "curl/7.64.1",
        "python-requests/2.28.1",
        "Googlebot/2.1 (+http://www.google.com/bot.html)",
        "okhttp/4.9.3 android-client",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let yaml = r#"
technology.internet.user_agent:
  title: "User Agent String"
  validation:
    type: string
    pattern: "^(Mozilla/|curl/|python-requests/|Wget/|Go-http-client/|axios/|PostmanRuntime/|kube-probe/|Java/|okhttp/|Apache-HttpClient/|libcurl/|node-fetch/|Dalvik/|CFNetwork/|Lynx/|Links |Scrapy/|Googlebot/|Bingbot/|Slackbot|Twitterbot/|facebookexternalhit/|LinkedInBot/|Prometheus/|Datadog/|Ruby/|Dart/|grpc-|HTTPie/|bot|spider|crawl)"
    minLength: 10
    maxLength: 500
  tier: [VARCHAR, internet]
  release_priority: 3
  samples: ["curl/7.64.1"]
"#;
    let taxonomy = Taxonomy::from_yaml(yaml).unwrap();
    let result = value_sharpen(
        &values,
        "technology.internet.user_agent",
        0.9,
        Some(&taxonomy),
    );
    assert!(
        result.is_none(),
        "real user-agent values must not be demoted off technology.internet.user_agent"
    );
}

// Was testing street_number non-demotion which no longer exists.

#[test]
fn test_attractor_no_demotion_high_confidence() {
    // Attractor at >0.85 with valid values — should NOT demote
    let values: Vec<String> = vec![
        "10001", "90210", "30301", "60601", "02101", "75001", "33101", "94102", "20001", "98101",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let votes = vec![
        ("geography.address.postal_code".to_string(), 9),
        ("representation.numeric.integer_number".to_string(), 1),
    ];

    let yaml = r#"
geography.address.postal_code:
  title: "Postal Code"
  validation:
    type: string
    pattern: "^[0-9]{3,10}$"
    minLength: 3
    maxLength: 10
  tier: [VARCHAR, geography, address]
  release_priority: 5
  samples: ["10001"]
"#;
    let taxonomy = Taxonomy::from_yaml(yaml).unwrap();

    // All pass validation AND confidence is 0.9 → no demotion
    let result = disambiguate_attractor_demotion(&values, &votes, 10, Some(&taxonomy));
    assert!(
        result.is_none(),
        "Should NOT demote real postal codes at 0.9 confidence"
    );
}

#[test]
fn test_select_fallback_numeric() {
    // All integer values, no representation.* in votes
    let values: Vec<String> = vec!["100", "200", "300"]
        .into_iter()
        .map(String::from)
        .collect();
    let votes = vec![
        ("geography.address.postal_code".to_string(), 8),
        ("representation.numeric.integer_number".to_string(), 2),
    ];

    let result = select_fallback(&votes, true, false, false, &values);
    assert_eq!(result, "representation.numeric.integer_number");
}

#[test]
fn test_select_fallback_text() {
    let values: Vec<String> = vec!["Soccer", "Tennis"]
        .into_iter()
        .map(String::from)
        .collect();
    let votes = vec![
        ("identity.person.first_name".to_string(), 8),
        ("identity.person.username".to_string(), 2),
    ];

    let result = select_fallback(&votes, false, true, false, &values);
    assert_eq!(result, "representation.text.word");
}

#[test]
fn test_select_fallback_from_votes() {
    // representation.* type exists in the vote distribution → use it
    let values: Vec<String> = vec!["100", "200"].into_iter().map(String::from).collect();
    let votes = vec![
        ("geography.address.postal_code".to_string(), 6),
        ("representation.numeric.decimal_number".to_string(), 3),
        ("representation.numeric.integer_number".to_string(), 1),
    ];

    let result = select_fallback(&votes, true, false, false, &values);
    assert_eq!(
        result, "representation.numeric.decimal_number",
        "Should use representation.* type from votes when available"
    );
}

// ── Locale-aware attractor demotion tests ───────────────────────────

#[test]
fn test_attractor_demotion_locale_validation_demotes_salary() {
    // Salary column predicted as postal_code: 6-digit values fail ALL locale
    // patterns. Using values clearly in salary range (>99999) that cannot
    // be valid postal codes in any locale.
    let values: Vec<String> = vec![
        "102000", "245000", "112000", "350000", "178000", "195000", "267000", "188000", "103000",
        "272000",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let votes = vec![
        ("geography.address.postal_code".to_string(), 9),
        ("representation.numeric.integer_number".to_string(), 1),
    ];

    let yaml = r#"
geography.address.postal_code:
  title: "Postal Code"
  validation:
    type: string
    minLength: 3
    maxLength: 10
    maximum: 99999
  validation_by_locale:
    EN_US:
      type: string
      pattern: "^(\\d{5})(?:[ \\-](\\d{4}))?$"
      minLength: 5
      maxLength: 10
    EN_GB:
      type: string
      pattern: "^[A-Z]{1,2}\\d[A-Z\\d]?\\s?\\d[A-Z]{2}$"
      minLength: 5
      maxLength: 8
    DE:
      type: string
      pattern: "^\\d{5}$"
      minLength: 5
      maxLength: 5
  tier: [VARCHAR, address]
  release_priority: 4
  samples: ["10001"]
"#;
    let mut taxonomy = Taxonomy::from_yaml(yaml).unwrap();
    taxonomy.compile_validators();
    taxonomy.compile_locale_validators();

    let result = disambiguate_attractor_demotion(&values, &votes, 10, Some(&taxonomy));
    assert!(
        result.is_some(),
        "Should demote salary values despite matching universal validation"
    );
    let (label, rule) = result.unwrap();
    assert_eq!(label, "representation.numeric.integer_number");
    assert!(
        rule.starts_with("attractor_demotion_validation:"),
        "Should demote via validation signal, got: {}",
        rule
    );
}

#[test]
fn test_attractor_accepts_real_us_postal_codes() {
    // Real US ZIP codes: match EN_US locale pattern → locale-confirmed, no demotion
    let values: Vec<String> = vec![
        "10001", "90210", "30301", "60601", "02101", "75001", "33101", "94102", "20001", "98101",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let votes = vec![
        ("geography.address.postal_code".to_string(), 9),
        ("representation.numeric.integer_number".to_string(), 1),
    ];

    let yaml = r#"
geography.address.postal_code:
  title: "Postal Code"
  validation:
    type: string
    minLength: 3
    maxLength: 10
    maximum: 99999
  validation_by_locale:
    EN_US:
      type: string
      pattern: "^(\\d{5})(?:[ \\-](\\d{4}))?$"
      minLength: 5
      maxLength: 10
    EN_GB:
      type: string
      pattern: "^[A-Z]{1,2}\\d[A-Z\\d]?\\s?\\d[A-Z]{2}$"
      minLength: 5
      maxLength: 8
    DE:
      type: string
      pattern: "^\\d{5}$"
      minLength: 5
      maxLength: 5
  tier: [VARCHAR, address]
  release_priority: 4
  samples: ["10001"]
"#;
    let mut taxonomy = Taxonomy::from_yaml(yaml).unwrap();
    taxonomy.compile_validators();
    taxonomy.compile_locale_validators();

    // US ZIPs match EN_US locale at >50% → locale-confirmed → no demotion
    let result = disambiguate_attractor_demotion(&values, &votes, 10, Some(&taxonomy));
    assert!(
        result.is_none(),
        "Should NOT demote real US ZIP codes (locale-confirmed by EN_US)"
    );
}

#[test]
fn test_attractor_accepts_real_uk_postcodes() {
    // Real UK postcodes: match EN_GB locale pattern → locale-confirmed, no demotion
    let values: Vec<String> = vec![
        "EC1A 1BB", "W1C 1AX", "M2 5BQ", "SW1A 1AA", "B1 1BB", "LS1 1BA",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let votes = vec![("geography.address.postal_code".to_string(), 6)];

    let yaml = r#"
geography.address.postal_code:
  title: "Postal Code"
  validation:
    type: string
    minLength: 3
    maxLength: 10
    maximum: 99999
  validation_by_locale:
    EN_US:
      type: string
      pattern: "^(\\d{5})(?:[ \\-](\\d{4}))?$"
      minLength: 5
      maxLength: 10
    EN_GB:
      type: string
      pattern: "^[A-Z]{1,2}\\d[A-Z\\d]?\\s?\\d[A-Z]{2}$"
      minLength: 5
      maxLength: 8
  tier: [VARCHAR, address]
  release_priority: 4
  samples: ["10001"]
"#;
    let mut taxonomy = Taxonomy::from_yaml(yaml).unwrap();
    taxonomy.compile_validators();
    taxonomy.compile_locale_validators();

    let result = disambiguate_attractor_demotion(&values, &votes, 6, Some(&taxonomy));
    assert!(
        result.is_none(),
        "Should NOT demote real UK postcodes (locale-confirmed by EN_GB)"
    );
}

#[test]
fn test_attractor_locale_low_confidence_accepted() {
    // US ZIP codes at low confidence (0.6) — normally Signal 2 would demote,
    // but locale validation confirms the type → no demotion
    let values: Vec<String> = vec![
        "10001", "90210", "30301", "60601", "02101", "75001", "33101", "94102", "20001", "98101",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let votes = vec![
        ("geography.address.postal_code".to_string(), 6),
        ("representation.numeric.integer_number".to_string(), 4),
    ];

    let yaml = r#"
geography.address.postal_code:
  title: "Postal Code"
  validation:
    type: string
    minLength: 3
    maxLength: 10
    maximum: 99999
  validation_by_locale:
    EN_US:
      type: string
      pattern: "^(\\d{5})(?:[ \\-](\\d{4}))?$"
      minLength: 5
      maxLength: 10
  tier: [VARCHAR, address]
  release_priority: 4
  samples: ["10001"]
"#;
    let mut taxonomy = Taxonomy::from_yaml(yaml).unwrap();
    taxonomy.compile_validators();
    taxonomy.compile_locale_validators();

    // Confidence 0.6 < 0.85 → Signal 2 would fire, BUT locale validation
    // confirms the type → locale_confirmed = true → Signal 2 skipped
    let result = disambiguate_attractor_demotion(&values, &votes, 10, Some(&taxonomy));
    assert!(
        result.is_none(),
        "Should NOT demote real US ZIPs at low confidence when locale confirms them"
    );
}

#[test]
fn test_attractor_locale_confirmed_skips_cardinality() {
    // Phone numbers with locale confirmation should NOT be demoted
    // by Signal 3 (cardinality), even with few unique values. Small tables
    // with legitimate phone numbers are common in web-scraped datasets.
    let values: Vec<String> = vec![
        "(805) 638-3078",
        "(650) 440-2450",
        "(805) 638-3078",
        "(805) 638-3078",
        "(650) 440-2450",
        "(650) 440-2450",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    // 2 unique values, 6 total — classic cardinality demotion target
    let votes = vec![("identity.person.phone_number".to_string(), 6)];

    let yaml = r#"
identity.person.phone_number:
  title: "Phone Number"
  validation:
    type: string
    minLength: 7
    maxLength: 20
    pattern: "^[+]?[0-9\\s()\\-\\.]+$"
  validation_by_locale:
    EN_US:
      type: string
      pattern: "^(\\+?1[\\s\\-\\.]*)?\\(?\\d{3}\\)?[\\s\\-\\.]*\\d{3}[\\s\\-\\.]*\\d{4}$"
      minLength: 10
      maxLength: 18
  tier: [VARCHAR, person]
  release_priority: 4
  samples: ["+1 (555) 123-4567"]
"#;
    let mut taxonomy = Taxonomy::from_yaml(yaml).unwrap();
    taxonomy.compile_validators();
    taxonomy.compile_locale_validators();

    let result = disambiguate_attractor_demotion(&values, &votes, 6, Some(&taxonomy));
    assert!(
        result.is_none(),
        "Should NOT demote phone numbers when locale-confirmed, even with 2 unique values"
    );
}

#[test]
fn test_attractor_universal_only_does_not_confirm_locale_type() {
    // Precision Principle: For locale-specific types, passing the
    // universal validation pattern does NOT count as confirmation. Only locale
    // patterns can confirm. These values pass universal phone validation
    // (digits + formatting chars) but don't match any locale pattern.
    let values: Vec<String> = vec![
        "123-456", "789-012", "345-678", "123-456", "789-012", "345-678",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    // Low confidence — Signal 2 would fire if not confirmed
    let votes = vec![
        ("identity.person.phone_number".to_string(), 4),
        ("representation.discrete.categorical".to_string(), 2),
    ];

    let yaml = r#"
identity.person.phone_number:
  title: "Phone Number"
  validation:
    type: string
    minLength: 7
    maxLength: 20
    pattern: "^[+]?[0-9\\s()\\-\\.]+$"
  validation_by_locale:
    EN_US:
      type: string
      pattern: "^(\\+?1[\\s\\-\\.]*)?\\(?\\d{3}\\)?[\\s\\-\\.]*\\d{3}[\\s\\-\\.]*\\d{4}$"
      minLength: 10
      maxLength: 18
  tier: [VARCHAR, person]
  release_priority: 4
  samples: ["+1 (555) 123-4567"]
"#;
    let mut taxonomy = Taxonomy::from_yaml(yaml).unwrap();
    taxonomy.compile_validators();
    taxonomy.compile_locale_validators();

    // Confidence 0.67 < 0.85 AND no locale confirmation → should demote
    // despite universal pattern matching (precision principle)
    let result = disambiguate_attractor_demotion(&values, &votes, 6, Some(&taxonomy));
    assert!(
        result.is_some(),
        "Should demote phone_number at low confidence when only universal validates (no locale)"
    );
    let (_, rule) = result.unwrap();
    assert!(
        rule.starts_with("attractor_demotion_confidence:"),
        "Should demote via confidence signal, got: {}",
        rule
    );
}

#[test]
fn test_attractor_first_name_cardinality_unchanged() {
    // first_name has no locale validators, so cardinality demotion
    // still works exactly as before — this is a regression guard.
    let values: Vec<String> = vec![
        "John", "Jane", "Bob", "John", "Jane", "Bob", "John", "Jane", "Bob", "John",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let votes = vec![
        ("identity.person.first_name".to_string(), 9),
        ("representation.text.word".to_string(), 1),
    ];

    // 3 unique values, text attractor, no locale validators → must still demote
    let result = disambiguate_attractor_demotion(&values, &votes, 10, None);
    assert!(
        result.is_some(),
        "first_name with 3 unique values should still be demoted (no locale validators)"
    );
    let (label, rule) = result.unwrap();
    assert_eq!(label, "representation.text.word");
    assert!(
        rule.starts_with("attractor_demotion_cardinality:"),
        "Should demote via cardinality signal, got: {}",
        rule
    );
}

// ==========================================================================
// Designation-aware is_generic_prediction tests
// ==========================================================================

#[test]
fn test_is_generic_attractor_demoted_always_generic() {
    // Attractor-demoted predictions are always generic, regardless of designation
    let rule = Some("attractor_demotion_validation:something".to_string());
    assert!(
        is_generic_prediction("identity.person.email", &rule, None),
        "Attractor-demoted predictions should always be generic"
    );
}

#[test]
fn test_is_generic_boolean_always_generic() {
    // Boolean types are always generic
    assert!(
        is_generic_prediction("representation.boolean.binary", &None, None),
        "Boolean types should always be generic"
    );
    assert!(
        is_generic_prediction("representation.boolean.terms", &None, None),
        "Boolean types should always be generic"
    );
}

#[test]
fn test_is_generic_broad_words_with_taxonomy() {
    // broad_words designation should make a prediction generic when taxonomy is available
    let yaml = r#"
identity.person.gender:
  title: Gender
  designation: broad_words
  tier: [VARCHAR, identity, person]
  release_priority: 1
  samples: ["Male"]
identity.person.email:
  title: Email
  designation: universal
  tier: [VARCHAR, identity, person]
  release_priority: 5
  samples: ["test@example.com"]
"#;
    let taxonomy = Taxonomy::from_yaml(yaml).unwrap();

    // broad_words → generic
    assert!(
        is_generic_prediction("identity.person.gender", &None, Some(&taxonomy)),
        "broad_words types should be generic when taxonomy is available"
    );

    // universal → not generic (not in hardcoded list either)
    assert!(
        !is_generic_prediction("identity.person.email", &None, Some(&taxonomy)),
        "universal types should NOT be generic (unless in hardcoded list)"
    );
}

#[test]
fn test_is_generic_broad_characters_with_taxonomy() {
    let yaml = r#"
identity.person.password:
  title: Password
  designation: broad_characters
  tier: [VARCHAR, identity, person]
  release_priority: 1
  samples: ["p@ssw0rd"]
"#;
    let taxonomy = Taxonomy::from_yaml(yaml).unwrap();

    assert!(
        is_generic_prediction("identity.person.password", &None, Some(&taxonomy)),
        "broad_characters types should be generic when taxonomy is available"
    );
}

#[test]
fn test_is_generic_broad_numbers_with_taxonomy() {
    let yaml = r#"
representation.identifier.increment:
  title: Increment
  designation: broad_numbers
  tier: [BIGINT, identifier]
  release_priority: 1
  samples: ["42"]
"#;
    let taxonomy = Taxonomy::from_yaml(yaml).unwrap();

    assert!(
        is_generic_prediction(
            "representation.identifier.increment",
            &None,
            Some(&taxonomy)
        ),
        "broad_numbers types should be generic when taxonomy is available"
    );
}

#[test]
fn test_is_generic_fallback_without_taxonomy() {
    // Without taxonomy, falls back to hardcoded list
    assert!(
        is_generic_prediction("representation.text.word", &None, None),
        "Hardcoded generic label should be generic without taxonomy"
    );
    assert!(
        is_generic_prediction("identity.person.phone_number", &None, None),
        "Hardcoded generic label should be generic without taxonomy"
    );
    assert!(
        !is_generic_prediction("identity.person.email", &None, None),
        "Non-hardcoded label should NOT be generic without taxonomy"
    );
}

#[test]
fn test_is_generic_locale_specific_not_generic() {
    // locale_specific designation should NOT be generic (when not in hardcoded list)
    let yaml = r#"
geography.address.postal_code:
  title: Postal Code
  designation: locale_specific
  tier: [VARCHAR, geography, address]
  release_priority: 5
  samples: ["90210"]
"#;
    let taxonomy = Taxonomy::from_yaml(yaml).unwrap();

    // postal_code is locale_specific and NOT in the hardcoded list → not generic
    assert!(
        !is_generic_prediction("geography.address.postal_code", &None, Some(&taxonomy)),
        "locale_specific types not in hardcoded list should NOT be generic"
    );
}

#[test]
fn test_is_generic_hardcoded_overrides_taxonomy() {
    // phone_number is in the hardcoded list AND has locale_specific designation.
    // Hardcoded list (Signal 3) takes precedence — the type stays generic so
    // header hints can still override when the model uses it as a catch-all.
    let yaml = r#"
identity.person.phone_number:
  title: Phone Number
  designation: locale_specific
  tier: [VARCHAR, identity, person]
  release_priority: 5
  samples: ["+1 (202) 555-0100"]
"#;
    let taxonomy = Taxonomy::from_yaml(yaml).unwrap();

    assert!(
        is_generic_prediction("identity.person.phone_number", &None, Some(&taxonomy)),
        "phone_number is in hardcoded list — stays generic regardless of designation"
    );
}

// ==========================================================================
// Rule fixes for 3 profile eval misses
// ==========================================================================

#[test]
fn test_is_generic_numeric_postal_code_detection() {
    // Fix 1: numeric_postal_code_detection should yield to header hints
    // (e.g., "cvv" column with 3-digit values detected as postal_code)
    let rule = Some("numeric_postal_code_detection".to_string());
    assert!(
        is_generic_prediction("geography.address.postal_code", &rule, None),
        "numeric_postal_code_detection should be treated as generic to yield to header hints"
    );

    // numeric_port_detection assertion removed (port type removed)
}

// AC-3: sharpen_attractor_demotion — confidence threshold
#[test]
fn test_sharpen_attractor_high_confidence_no_demotion() {
    // postal_code with high confidence (0.95) should NOT be demoted
    let values: Vec<String> = vec!["10001", "90210", "60601", "30301", "94102"]
        .into_iter()
        .map(String::from)
        .collect();

    let result = sharpen_attractor_demotion(
        &values,
        "geography.address.postal_code",
        0.95, // high confidence
        None, // no taxonomy for validation
    );

    assert!(
        result.is_none(),
        "High confidence (0.95) should NOT trigger attractor demotion"
    );
}

#[test]
fn test_sharpen_attractor_low_confidence_demotes() {
    // postal_code with low confidence (0.30) should be demoted
    let values: Vec<String> = vec!["42", "100", "7", "256", "1024"]
        .into_iter()
        .map(String::from)
        .collect();

    let result = sharpen_attractor_demotion(
        &values,
        "geography.address.postal_code",
        0.30, // low confidence — below 0.85 threshold
        None, // no taxonomy for validation
    );

    assert!(
        result.is_some(),
        "Low confidence (0.30) should trigger attractor demotion"
    );
    let (label, rule) = result.unwrap();
    assert_eq!(label, "representation.numeric.integer_number");
    assert!(
        rule.starts_with("attractor_demotion_confidence"),
        "Rule should be confidence-based demotion, got: {}",
        rule
    );
}

#[test]
fn test_sharpen_attractor_text_low_confidence_demotes_to_categorical() {
    // first_name with low confidence and low cardinality → categorical
    let values: Vec<String> = vec!["alpha", "beta", "gamma", "alpha", "beta"]
        .into_iter()
        .map(String::from)
        .collect();

    let result = sharpen_attractor_demotion(
        &values,
        "identity.person.first_name",
        0.40, // low confidence
        None,
    );

    assert!(
        result.is_some(),
        "Low confidence text attractor should demote"
    );
    let (label, _rule) = result.unwrap();
    assert_eq!(label, "representation.text.word");
}

#[test]
fn test_sharpen_attractor_non_attractor_type_ignored() {
    // A type that's not in any attractor list should never trigger demotion
    let values: Vec<String> = vec!["hello@example.com"]
        .into_iter()
        .map(String::from)
        .collect();

    let result = sharpen_attractor_demotion(
        &values,
        "identity.person.email",
        0.30, // even with low confidence
        None,
    );

    assert!(
        result.is_none(),
        "Non-attractor type should never trigger demotion"
    );
}
