use super::super::*;
use super::{make_result, vals};

// ── Cardinality & categorical rule tests ────────────────────────────

#[test]
fn test_gender_detection_mixed_case() {
    let values: Vec<String> = vec![
        "male", "female", "Male", "Female", "male", "female", "male", "Female", "male", "Male",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let result = disambiguate_gender(&values);
    assert_eq!(result, Some("identity.person.gender".to_string()));
}

#[test]
fn test_gender_detection_single_char_is_gender_code() {
    // Bare single-char sex codes (ICAO M/F/X) are gender_code, not gender:
    // they validate against [M,F,X,0,1,2,9], not the word enum
    // [male,female,other,unknown] which would hard-veto them.
    let values: Vec<String> = vec!["M", "F", "M", "F", "M", "F", "M", "F", "M", "F"]
        .into_iter()
        .map(String::from)
        .collect();

    let result = disambiguate_gender(&values);
    assert_eq!(result, Some("identity.person.gender_code".to_string()));
}

#[test]
fn test_gender_detection_single_char_mixed_case_with_x() {
    let values: Vec<String> = vec!["m", "f", "X", "M", "f", "x", "F"]
        .into_iter()
        .map(String::from)
        .collect();

    let result = disambiguate_gender(&values);
    assert_eq!(result, Some("identity.person.gender_code".to_string()));
}

#[test]
fn test_gender_detection_word_and_code_mix_stays_gender() {
    // Mixed word + single-char (not all single-char) -> word type gender.
    let values: Vec<String> = vec!["male", "M", "female", "F", "male"]
        .into_iter()
        .map(String::from)
        .collect();

    let result = disambiguate_gender(&values);
    assert_eq!(result, Some("identity.person.gender".to_string()));
}

#[test]
fn test_gender_detection_numeric_codes_do_not_fire() {
    // ISO-5218 numerics are NOT a value-only trigger (boolean/ordinal
    // over-emit guard) — a bare 0/1/2 column must not become gender_code.
    let values: Vec<String> = vec!["0", "1", "2", "1", "0", "2"]
        .into_iter()
        .map(String::from)
        .collect();

    let result = disambiguate_gender(&values);
    assert_eq!(result, None);
}

#[test]
fn test_gender_detection_with_nonbinary() {
    // People directory: Male, Female, Non-binary
    let values: Vec<String> = vec![
        "Male",
        "Female",
        "Male",
        "Non-binary",
        "Female",
        "Male",
        "Female",
        "Male",
        "Non-binary",
        "Female",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let result = disambiguate_gender(&values);
    assert_eq!(
        result,
        Some("identity.person.gender".to_string()),
        "Non-binary should be recognized as a valid gender value"
    );
}

#[test]
fn test_gender_detection_with_other_inclusive() {
    let values: Vec<String> = vec![
        "Male",
        "Female",
        "Other",
        "Male",
        "Female",
        "Prefer not to say",
        "Male",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let result = disambiguate_gender(&values);
    assert_eq!(result, Some("identity.person.gender".to_string()));
}

#[test]
fn test_gender_detection_fails_for_non_gender() {
    let values: Vec<String> = vec!["red", "blue", "green", "red", "blue"]
        .into_iter()
        .map(String::from)
        .collect();

    let result = disambiguate_gender(&values);
    assert_eq!(result, None);
}

#[test]
fn test_ipv4_detection_standard_ips() {
    let values: Vec<String> = vec![
        "192.168.1.1",
        "10.0.0.1",
        "172.16.0.1",
        "8.8.8.8",
        "1.2.3.4",
        "10.0.0.255",
        "192.168.0.100",
        "255.255.255.0",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let result = disambiguate_ipv4(&values);
    assert_eq!(
        result,
        Some("technology.internet.ip_v4".to_string()),
        "Standard IPv4 addresses should be detected"
    );
}

#[test]
fn test_ipv4_detection_rejects_version_numbers() {
    // Semantic version numbers have different structure (fewer octets, >255 values)
    let values: Vec<String> = vec![
        "1.0.0", "2.1.3", "3.14.159", "0.2.53", "6.27.84", "4.24.59", "7.23.74",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let result = disambiguate_ipv4(&values);
    assert_eq!(
        result, None,
        "Version numbers should NOT match IPv4 pattern"
    );
}

#[test]
fn test_ipv4_detection_rejects_decimals() {
    let values: Vec<String> = vec!["151.3", "165.0", "161.2", "169.1", "181.7"]
        .into_iter()
        .map(String::from)
        .collect();

    let result = disambiguate_ipv4(&values);
    assert_eq!(
        result, None,
        "Decimal numbers should NOT match IPv4 pattern"
    );
}

#[test]
fn test_ipv4_detection_mixed_with_some_invalid() {
    // 80% threshold: 8 valid out of 10 = 80%
    let values: Vec<String> = vec![
        "10.0.0.1", "10.0.0.2", "10.0.0.3", "10.0.0.4", "10.0.0.5", "10.0.0.6", "10.0.0.7",
        "10.0.0.8", "N/A", "unknown",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let result = disambiguate_ipv4(&values);
    assert_eq!(
        result,
        Some("technology.internet.ip_v4".to_string()),
        "80% valid IPs should trigger detection"
    );
}

// ── Header hint tests ───────────────────────────────────────────────

#[test]
fn test_header_hint_email() {
    assert_eq!(header_hint("Email"), Some("identity.person.email"));
    assert_eq!(header_hint("email_address"), Some("identity.person.email"));
    assert_eq!(header_hint("E-Mail"), Some("identity.person.email"));
    assert_eq!(header_hint("user_email"), Some("identity.person.email"));
}

#[test]
fn test_header_hint_phone() {
    assert_eq!(header_hint("phone"), Some("identity.person.phone_number"));
    assert_eq!(
        header_hint("Phone Number"),
        Some("identity.person.phone_number")
    );
    assert_eq!(
        header_hint("telephone"),
        Some("identity.person.phone_number")
    );
    assert_eq!(header_hint("mobile"), Some("identity.person.phone_number"));
}

#[test]
fn v15_email_display_guard() {
    // "email_display" should NOT match the email hint
    assert_eq!(header_hint("email_display"), None);
    // But "email" still matches
    assert_eq!(header_hint("email"), Some("identity.person.email"));
    // And "customer_email" still matches
    assert_eq!(header_hint("customer_email"), Some("identity.person.email"));
}

#[test]
fn v15_phone_e164_guard() {
    // "phone_e164" should NOT match the phone hint
    assert_eq!(header_hint("phone_e164"), None);
    // But "phone" still matches
    assert_eq!(header_hint("phone"), Some("identity.person.phone_number"));
    // And "phone_number" still matches
    assert_eq!(
        header_hint("phone_number"),
        Some("identity.person.phone_number")
    );
}

#[test]
fn v15_ip_port_guard() {
    // "ip_v4_with_port" should NOT match the ip_v4 hint
    assert_eq!(header_hint("ip_v4_with_port"), None);
    // But "ip_address" still matches
    assert_eq!(header_hint("ip_address"), Some("technology.internet.ip_v4"));
    // And "server_ip" still matches
    assert_eq!(header_hint("server_ip"), Some("technology.internet.ip_v4"));
}

// ── Text length demotion tests ──

#[test]
fn test_text_length_demotion_long_text_as_address() {
    // Long descriptions/paragraphs misclassified as full_address
    let values: Vec<String> = vec![
            "Contact information of the hotel Record of Zelenograd: phone, location map, address on the map. Full amenities and services list available.",
            "The layout of the room includes two bedrooms and a spacious lounge, but each room has its own plasma TV for entertainment and relaxation purposes.",
            "Services provided by the hotel Record (Zelenograd): Credit cards (Visa, MasterCard, World), free Wi-Fi, gym, spa, pool, conference rooms available.",
            "STANDARD WITH THE KITCHEN number one category. The Record Hotel has 1 Standard Room with Kitchen area for extended stays and business travelers.",
            "Preheat oven to 350 degrees. Grease a small baking dish or small cast iron skillet and fill with peaches. Sprinkle cinnamon over peaches and set aside.",
        ]
        .into_iter()
        .map(String::from)
        .collect();

    let votes = vec![("geography.address.full_address".to_string(), 5)];
    let result = disambiguate_text_length_demotion(&values, &votes);
    assert!(result.is_some(), "Should demote long text overcall");
    let (label, rule) = result.unwrap();
    assert_eq!(label, "representation.text.plain_text");
    assert_eq!(rule, "text_length_demotion_full_address");
}

#[test]
fn test_text_length_demotion_real_address_not_demoted() {
    // Real addresses should NOT be demoted (typical length 20-40 chars)
    let values: Vec<String> = vec![
        "123 Main St, Springfield, IL",
        "456 Oak Ave, Portland, OR",
        "789 Pine Rd, Austin, TX",
        "101 Elm Blvd, Denver, CO",
        "202 Maple Dr, Seattle, WA",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let votes = vec![("geography.address.full_address".to_string(), 5)];
    let result = disambiguate_text_length_demotion(&values, &votes);
    assert!(
        result.is_none(),
        "Should NOT demote real addresses (median ~28 chars)"
    );
}

#[test]
fn test_text_length_demotion_ignores_non_address() {
    // Rule should not fire for non-full_address predictions
    let values: Vec<String> = vec![
            "This is a very long text that exceeds one hundred characters and should demonstrate that length alone does not trigger demotion for other types.",
            "Another very long text value that is clearly longer than the threshold of one hundred characters but is not predicted as full_address by the model.",
            "Yet another long text to ensure the median is above the threshold value for the test to be meaningful in demonstrating the rule only applies to addresses.",
        ]
        .into_iter()
        .map(String::from)
        .collect();

    let votes = vec![("identity.person.full_name".to_string(), 3)];
    let result = disambiguate_text_length_demotion(&values, &votes);
    assert!(
        result.is_none(),
        "Should NOT fire for non-full_address predictions"
    );
}

#[test]
fn test_text_length_demotion_long_prose_entity_name() {
    // BACKLOG #7: long news-prose misread as entity_name → plain_text.
    let values: Vec<String> = vec![
        "(Reuters) - Citigroup Inc said on Friday it recorded an additional charge related to its previously announced restructuring and severance programme.",
        "(Bloomberg) - The central bank held rates steady on Thursday, signalling caution amid mixed signals on inflation and a cooling labour market this quarter.",
        "LONDON - Shares in the mining group fell sharply after it warned that full-year production would land below guidance owing to disruption at two key sites.",
        "NEW YORK - The technology company unveiled a sweeping reorganisation on Tuesday, consolidating its hardware and services divisions under a single leader.",
        "PARIS - The luxury conglomerate reported quarterly sales that beat analyst expectations, lifted by resilient demand across its leather goods business.",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let votes = vec![("representation.text.entity_name".to_string(), 5)];
    let result = disambiguate_text_length_demotion(&values, &votes);
    assert!(result.is_some(), "Should demote long entity_name prose");
    let (label, rule) = result.unwrap();
    assert_eq!(label, "representation.text.plain_text");
    assert!(rule.starts_with("text_length_demotion_long_prose:"));
}

#[test]
fn test_text_length_demotion_short_entity_not_demoted() {
    // Real entity names are short — must NOT be demoted.
    let values: Vec<String> = vec!["Apple Inc", "Microsoft", "Sony", "Nestle", "Adobe"]
        .into_iter()
        .map(String::from)
        .collect();
    let votes = vec![("representation.text.entity_name".to_string(), 5)];
    assert!(disambiguate_text_length_demotion(&values, &votes).is_none());
}

#[test]
fn test_full_address_whitespace_guard_demotes_nospace_id() {
    // BACKLOG #8: single-token letter+digit strings collapsed into full_address
    // (idx 43: CUDNN benchmark IDs) → alphanumeric_id.
    let values: Vec<String> = vec![
        "ACTIVATION_FWD8731790411514196767<CUDNN_ACTIVATION_CLIPPED_RELU>",
        "CONV_FWD_1234<CUDNN_CONVOLUTION>",
        "ADD_TENSOR_99<CUDNN_OP>",
        "POOL_MAX_42<CUDNN_POOLING>",
        "BATCHNORM_FWD_7<CUDNN_BN>",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let (label, rule) = disambiguate_full_address_whitespace_guard(&values).unwrap();
    assert_eq!(label, "representation.identifier.alphanumeric_id");
    assert!(rule.starts_with("full_address_whitespace_guard"));
}

#[test]
fn test_full_address_whitespace_guard_keeps_real_addresses() {
    // Real multi-token addresses (whitespace present) must NOT be demoted, incl.
    // comma-less foreign addresses.
    let values: Vec<String> = vec![
        "123 Main St Springfield IL",
        "Hauptstrasse 12 8001 Zurich",
        "45 Rue de la Paix Paris",
        "789 Pine Rd Austin TX",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    assert!(disambiguate_full_address_whitespace_guard(&values).is_none());
}

#[test]
fn test_text_length_demotion_borderline_not_demoted() {
    // Values right at the boundary (median ~95 chars) should NOT be demoted
    let values: Vec<String> = vec![
            "123 Main Street, Apartment 4B, Springfield, Illinois 62704, United States of America — Near the park",
            "456 Oak Avenue, Suite 200, Portland, Oregon 97201, United States of America — Downtown district",
            "789 Pine Road, Building C, Unit 12, Austin, Texas 78701, United States of America — East campus",
        ]
        .into_iter()
        .map(String::from)
        .collect();

    let votes = vec![("geography.address.full_address".to_string(), 3)];
    let result = disambiguate_text_length_demotion(&values, &votes);
    assert!(
        result.is_none(),
        "Should NOT demote borderline addresses (median ~95 chars)"
    );
}

// ==========================================================================
// Post-hoc locale detection tests
// ==========================================================================

#[test]
fn test_detect_locale_us_phone_numbers() {
    // US phone numbers should detect EN_US locale
    let yaml = r#"
identity.person.phone_number:
  title: Phone Number
  designation: locale_specific
  tier: [VARCHAR, identity, person]
  release_priority: 5
  samples: ["+1 (202) 555-0100"]
  validation:
    type: string
    pattern: "^[+]?[0-9\\s()\\-\\.]+$"
  validation_by_locale:
    EN_US:
      type: string
      pattern: "^(\\+?1[\\s\\-./]*)?\\(?\\d{3}\\)?[\\s\\-./]*\\d{3}[\\s\\-./]*\\d{4}$"
      minLength: 10
      maxLength: 30
    EN_GB:
      type: string
      pattern: "^(\\+?44[\\s\\-./]*(\\(0\\))?)?0?\\d{2,5}([\\s\\-./]*\\d{1,8}){1,3}$"
      minLength: 10
      maxLength: 30
"#;
    let mut taxonomy = Taxonomy::from_yaml(yaml).unwrap();
    taxonomy.compile_locale_validators();

    let values: Vec<String> = vec![
        "+1 (202) 555-0100",
        "+1 (415) 555-0199",
        "(312) 555-0142",
        "1-800-555-0123",
        "(617) 555-0187",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let locale = detect_locale_from_validation(&values, "identity.person.phone_number", &taxonomy);
    assert_eq!(
        locale,
        Some("EN_US".to_string()),
        "US phone numbers should detect EN_US"
    );
}

#[test]
fn test_detect_locale_uk_phone_numbers() {
    let yaml = r#"
identity.person.phone_number:
  title: Phone Number
  designation: locale_specific
  tier: [VARCHAR, identity, person]
  release_priority: 5
  samples: ["+44 20 7946 0958"]
  validation:
    type: string
    pattern: "^[+]?[0-9\\s()\\-\\.]+$"
  validation_by_locale:
    EN_US:
      type: string
      pattern: "^(\\+?1[\\s\\-./]*)?\\(?\\d{3}\\)?[\\s\\-./]*\\d{3}[\\s\\-./]*\\d{4}$"
      minLength: 10
      maxLength: 30
    EN_GB:
      type: string
      pattern: "^(\\+?44[\\s\\-./]*(\\(0\\))?)?0?\\d{2,5}([\\s\\-./]*\\d{1,8}){1,3}$"
      minLength: 10
      maxLength: 30
"#;
    let mut taxonomy = Taxonomy::from_yaml(yaml).unwrap();
    taxonomy.compile_locale_validators();

    let values: Vec<String> = vec![
        "+44 20 7946 0958",
        "020 7946 0123",
        "+44 121 496 0987",
        "0161 496 0654",
        "+44 131 496 0321",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let locale = detect_locale_from_validation(&values, "identity.person.phone_number", &taxonomy);
    assert_eq!(
        locale,
        Some("EN_GB".to_string()),
        "UK phone numbers should detect EN_GB"
    );
}

#[test]
fn test_detect_locale_no_validators() {
    // Types without validation_by_locale should return None
    let yaml = r#"
identity.person.email:
  title: Email
  designation: universal
  tier: [VARCHAR, identity, person]
  release_priority: 5
  samples: ["test@example.com"]
"#;
    let taxonomy = Taxonomy::from_yaml(yaml).unwrap();

    let values: Vec<String> = vec!["test@example.com", "user@domain.org"]
        .into_iter()
        .map(String::from)
        .collect();

    let locale = detect_locale_from_validation(&values, "identity.person.email", &taxonomy);
    assert_eq!(
        locale, None,
        "Types without locale validators should return None"
    );
}

#[test]
fn test_detect_locale_no_match_above_threshold() {
    // Values that don't match any locale pattern well enough should return None
    let yaml = r#"
identity.person.phone_number:
  title: Phone Number
  designation: locale_specific
  tier: [VARCHAR, identity, person]
  release_priority: 5
  samples: ["+1 (202) 555-0100"]
  validation_by_locale:
    EN_US:
      type: string
      pattern: "^(\\+?1[\\s\\-./]*)?\\(?\\d{3}\\)?[\\s\\-./]*\\d{3}[\\s\\-./]*\\d{4}$"
      minLength: 10
      maxLength: 30
"#;
    let mut taxonomy = Taxonomy::from_yaml(yaml).unwrap();
    taxonomy.compile_locale_validators();

    // Random strings that don't match any phone pattern
    let values: Vec<String> = vec!["abc", "hello world", "12345", "not-a-phone"]
        .into_iter()
        .map(String::from)
        .collect();

    let locale = detect_locale_from_validation(&values, "identity.person.phone_number", &taxonomy);
    assert_eq!(
        locale, None,
        "Non-matching values should not detect any locale"
    );
}

#[test]
fn test_detect_locale_empty_values() {
    let yaml = r#"
identity.person.phone_number:
  title: Phone Number
  designation: locale_specific
  tier: [VARCHAR, identity, person]
  release_priority: 5
  samples: ["+1 (202) 555-0100"]
  validation_by_locale:
    EN_US:
      type: string
      pattern: "^(\\+?1[\\s\\-./]*)?\\(?\\d{3}\\)?[\\s\\-./]*\\d{3}[\\s\\-./]*\\d{4}$"
      minLength: 10
      maxLength: 30
"#;
    let mut taxonomy = Taxonomy::from_yaml(yaml).unwrap();
    taxonomy.compile_locale_validators();

    let values: Vec<String> = vec!["", "", ""].into_iter().map(String::from).collect();

    let locale = detect_locale_from_validation(&values, "identity.person.phone_number", &taxonomy);
    assert_eq!(
        locale, None,
        "All-empty values should not detect any locale"
    );
}

#[test]
fn ac02_ipv6_header_returns_ip_v6() {
    // "ip_v6" normalizes to "ip v6" — the v6 check must fire before the
    // generic ip_v4 catch-all.
    assert_eq!(
        header_hint("ip_v6"),
        Some("technology.internet.ip_v6"),
        "ip_v6 should return ip_v6"
    );
    assert_eq!(
        header_hint("server_ipv6"),
        Some("technology.internet.ip_v6"),
        "server_ipv6 should return ip_v6"
    );
    assert_eq!(
        header_hint("ipv6_address"),
        Some("technology.internet.ip_v6"),
        "ipv6_address should return ip_v6"
    );
    // Regression: source_ip still returns ip_v4 (exact match arm)
    assert_eq!(
        header_hint("source_ip"),
        Some("technology.internet.ip_v4"),
        "source_ip should still return ip_v4"
    );
    // Regression: ip_address still returns ip_v4
    assert_eq!(
        header_hint("ip_address"),
        Some("technology.internet.ip_v4"),
        "ip_address should still return ip_v4"
    );
}

#[test]
fn debug_ipv6_sharpen_override() {
    // Full apply_header_sharpen test: ip_v6 hint should override ip_v4
    let mock = crate::inference::MockClassifier::new("technology.internet.ip_v4");
    let cc = ColumnClassifier::with_defaults(Box::new(mock));
    let mut result = make_result("technology.internet.ip_v4", 0.70);
    let sample: Vec<String> = vec!["2001:db8::1".into(); 5];

    let hint = header_hint("ip_v6");
    assert_eq!(hint, Some("technology.internet.ip_v6"));

    cc.apply_header_sharpen(&mut result, "ip_v6", &sample);

    eprintln!("result.label = {}", result.label);
    eprintln!("result.rule = {:?}", result.disambiguation_rule);
    assert_eq!(
        result.label, "technology.internet.ip_v6",
        "ip_v6 hint must override ip_v4@0.70 via same-category path"
    );
}

// AC-3: value_sharpen — R8 gender with single-entry votes
#[test]
fn test_sharpen_r8_gender_single_vote() {
    let values: Vec<String> = vec!["Male", "Female", "Male", "Female", "Male"]
        .into_iter()
        .map(String::from)
        .collect();

    let result = value_sharpen(&values, "representation.discrete.categorical", 0.80, None);

    assert!(result.is_some(), "R8 should fire for gender values");
    let (label, rule) = result.unwrap();
    assert_eq!(label, "identity.person.gender");
    assert_eq!(rule, "gender_detection");
}

#[test]
fn username_veto_fires_on_login_handles() {
    // Corpus `author` shape: single-token handles, no internal whitespace.
    let handles = vals(&[
        "tptacek",
        "patio11",
        "rms",
        "jacquesm",
        "petercooper",
        "dfens",
        "schof",
    ]);
    assert!(is_username_handle_shaped(&handles));
}

#[test]
fn username_veto_fires_on_handles_with_digits_underscores() {
    let handles = vals(&["peter123", "dc2k08", "adam_null", "steve19", "mr-justin"]);
    assert!(is_username_handle_shaped(&handles));
}

#[test]
fn username_veto_skips_real_full_names() {
    // Multi-token "First Last" → whitespace fraction 1.0, stays full_name.
    let names = vals(&[
        "John Smith",
        "Mary Jane Watson",
        "Alan Turing",
        "Ada Lovelace",
        "Grace Hopper",
    ]);
    assert!(!is_username_handle_shaped(&names));
}

#[test]
fn username_veto_skips_entity_and_org_names() {
    // player_name / org-name shape: multi-word, has whitespace.
    let orgs = vals(&[
        "Portland Trail Blazers",
        "Golden State Warriors",
        "New York Knicks",
        "Boston Celtics",
    ]);
    assert!(!is_username_handle_shaped(&orgs));
}

#[test]
fn username_veto_skips_mixed_author_lists() {
    // `authors` (plural) corpus shape: ~0.58 whitespace fraction — too mixed.
    let mixed = vals(&[
        "Jane Doe",
        "jsmith",
        "Robert C. Martin",
        "kent_beck",
        "Martin Fowler",
    ]);
    assert!(!is_username_handle_shaped(&mixed));
}

#[test]
fn username_veto_needs_minimum_evidence() {
    // Too few non-empty values to judge.
    let scant = vals(&["rms", "", "  "]);
    assert!(!is_username_handle_shaped(&scant));
}

#[test]
fn username_veto_skips_low_cardinality_vocab() {
    // ac-03 false-positive class: single-token handle-charset values that are a
    // small REPEATING vocabulary (exchange codes, drug names) — not usernames.
    let drugs = vals(&[
        "ethanol", "ethanol", "ethanol", "morphine", "morphine", "morphine", "ethanol", "morphine",
    ]);
    assert!(!is_username_handle_shaped(&drugs));

    let exchanges = vals(&["NMS", "NYQ", "NMS", "NYQ", "NMS", "NYQ", "PCX", "NMS"]);
    assert!(!is_username_handle_shaped(&exchanges));
}

#[test]
fn username_veto_still_fires_on_high_cardinality_handles() {
    // Distinct handles per row survive the cardinality guard.
    let handles = vals(&[
        "ww520",
        "KevBurnsJr",
        "thinkcomp",
        "cjensen",
        "pmiller2",
        "vic_nyc",
        "andrewcooke",
        "lanstein",
    ]);
    assert!(is_username_handle_shaped(&handles));
}
