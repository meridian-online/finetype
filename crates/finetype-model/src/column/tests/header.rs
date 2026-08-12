use super::super::*;
use super::make_result;

#[test]
fn test_header_hint_names() {
    // Bare "name" is now ambiguous — returns None so Sense+CharCNN decide
    assert_eq!(header_hint("Name"), None);
    assert_eq!(header_hint("full_name"), Some("identity.person.full_name"));
    assert_eq!(
        header_hint("first_name"),
        Some("identity.person.first_name")
    );
    assert_eq!(header_hint("last_name"), Some("identity.person.last_name"));
    assert_eq!(header_hint("surname"), Some("identity.person.last_name"));
    // spec 2026-06-25-sharpen-stage-audit: the broad `ends_with(" name")` →
    // full_name arm was retired (net damage — country_name/template_name/
    // agency_name mis-promoted). Unqualified "* name" headers now carry no hint;
    // only the first/last/full-qualified arms above remain.
    assert_eq!(header_hint("display name"), None);
    assert_eq!(header_hint("user name"), None);
}

#[test]
fn test_header_hint_geo() {
    assert_eq!(
        header_hint("latitude"),
        Some("geography.coordinate.latitude")
    );
    assert_eq!(header_hint("lat"), Some("geography.coordinate.latitude"));
    assert_eq!(
        header_hint("longitude"),
        Some("geography.coordinate.longitude")
    );
    assert_eq!(header_hint("lng"), Some("geography.coordinate.longitude"));
    assert_eq!(header_hint("country"), Some("geography.location.country"));
    assert_eq!(header_hint("city"), Some("geography.location.city"));
}

#[test]
fn test_header_hint_identity() {
    assert_eq!(header_hint("gender"), Some("identity.person.gender"));
    assert_eq!(header_hint("Sex"), Some("identity.person.gender"));
    // "age" is not a taxonomy type; hint redirects to integer_number
    assert_eq!(
        header_hint("age"),
        Some("representation.numeric.integer_number")
    );
    assert_eq!(
        header_hint("Age"),
        Some("representation.numeric.integer_number")
    );
}

#[test]
fn test_header_hint_tech() {
    assert_eq!(header_hint("url"), Some("technology.internet.url"));
    assert_eq!(header_hint("URL"), Some("technology.internet.url"));
    assert_eq!(header_hint("website"), Some("technology.internet.url"));
    assert_eq!(header_hint("ip_address"), Some("technology.internet.ip_v4"));
    assert_eq!(header_hint("uuid"), Some("technology.identifier.uuid"));
    // "port" header hint removed
}

#[test]
fn test_header_hint_date() {
    assert_eq!(header_hint("date"), Some("datetime.timestamp.iso_8601"));
    assert_eq!(
        header_hint("created_date"),
        Some("datetime.timestamp.iso_8601")
    );
    assert_eq!(header_hint("year"), Some("datetime.component.year"));
    assert_eq!(header_hint("birth_date"), Some("datetime.date.iso"));
    assert_eq!(header_hint("dob"), Some("datetime.date.iso"));
    // Specific timestamp formats take priority over generic catch-all
    assert_eq!(
        header_hint("rfc_2822_timestamp"),
        Some("datetime.timestamp.rfc_2822")
    );
    assert_eq!(header_hint("rfc2822"), Some("datetime.timestamp.rfc_2822"));
    assert_eq!(header_hint("rfc_3339"), Some("datetime.timestamp.rfc_3339"));
    assert_eq!(
        header_hint("sql_timestamp"),
        Some("datetime.timestamp.sql_standard")
    );
    // Epoch/Unix timestamps — exact match
    assert_eq!(
        header_hint("unix_epoch"),
        Some("datetime.epoch.unix_seconds")
    );
    assert_eq!(
        header_hint("epoch_time"),
        Some("datetime.epoch.unix_seconds")
    );
    assert_eq!(
        header_hint("unix_ms"),
        Some("datetime.epoch.unix_milliseconds")
    );
    // Epoch/Unix — substring match
    assert_eq!(
        header_hint("created_epoch"),
        Some("datetime.epoch.unix_seconds")
    );
    assert_eq!(
        header_hint("timestamp_unix"),
        Some("datetime.epoch.unix_seconds")
    );
}

#[test]
fn test_header_hint_categorical() {
    // text types confused with region/country
    assert_eq!(header_hint("language"), Some("representation.text.word"));
    assert_eq!(header_hint("sport"), Some("representation.text.word"));
    assert_eq!(header_hint("species"), Some("representation.text.word"));
    assert_eq!(header_hint("exchange"), Some("representation.text.word"));
}

#[test]
fn test_header_hint_measurements() {
    // numeric measurements confused with numeric_code
    assert_eq!(
        header_hint("altitude"),
        Some("representation.numeric.integer_number")
    );
    assert_eq!(
        header_hint("elevation"),
        Some("representation.numeric.integer_number")
    );
    assert_eq!(
        header_hint("pages"),
        Some("representation.numeric.integer_number")
    );
    assert_eq!(
        header_hint("heart_rate"),
        Some("representation.numeric.integer_number")
    );
    assert_eq!(
        header_hint("attendance"),
        Some("representation.numeric.integer_number")
    );
    // "response time" intentionally unhinted — model handles both
    // integer and decimal response times correctly without hints.
    assert_eq!(header_hint("response_time_ms"), None);
    assert_eq!(
        header_hint("payload_size_bytes"),
        Some("representation.numeric.integer_number")
    );
    assert_eq!(
        header_hint("duration_minutes"),
        Some("representation.numeric.integer_number")
    );
}

#[test]
fn test_header_hint_numeric() {
    // Financial hints → finance.currency.amount
    assert_eq!(header_hint("price"), Some("finance.currency.amount"));
    assert_eq!(header_hint("amount"), Some("finance.currency.amount"));
    assert_eq!(header_hint("salary"), Some("finance.currency.amount"));
    assert_eq!(header_hint("revenue"), Some("finance.currency.amount"));
    assert_eq!(header_hint("income"), Some("finance.currency.amount"));
    assert_eq!(header_hint("expense"), Some("finance.currency.amount"));
    assert_eq!(header_hint("fare"), Some("finance.currency.amount"));
    assert_eq!(header_hint("fee"), Some("finance.currency.amount"));
    assert_eq!(
        header_hint("count"),
        Some("representation.numeric.integer_number")
    );
    // "id" intentionally unhinted — genuinely ambiguous
    assert_eq!(header_hint("id"), None);
}

#[test]
fn test_header_hint_no_match() {
    assert_eq!(header_hint("foo"), None);
    assert_eq!(header_hint("xyz"), None);
    assert_eq!(header_hint("data"), None);
    assert_eq!(header_hint("column1"), None);
}

#[test]
fn test_header_hint_coverage() {
    // Verify at least 20 distinct column name patterns are covered
    let test_headers = vec![
        "email",
        "phone",
        "zip",
        "postal",
        "name",
        "full_name",
        "first_name",
        "last_name",
        "latitude",
        "longitude",
        "country",
        "city",
        "state",
        "gender",
        "url",
        "ip",
        "uuid",
        "port",
        "date",
        "year",
        "password",
        "price",
        "amount",
        "count",
        "address",
        "street",
    ];
    let matches: Vec<&str> = test_headers
        .iter()
        .filter(|h| header_hint(h).is_some())
        .copied()
        .collect();
    assert!(
        matches.len() >= 20,
        "Expected at least 20 matches, got {}: {:?}",
        matches.len(),
        matches
    );
}

// ── New header hint tests ─────────────────────────────────

#[test]
fn class_rank_grade_headers_no_longer_hint_ordinal() {
    // spec 2026-06-25-sharpen-stage-audit: the class/rank/grade/tier → ordinal
    // header arms were retired (value-blind, gold-negative on the attention model
    // — Grade, Region Rank, GlobalRank, TldRank, usageclass were mis-promoted to
    // ordinal over a correct numeric Sense). These headers now carry no hint; the
    // model's value-based prediction stands.
    assert_eq!(header_hint("Pclass"), None);
    assert_eq!(header_hint("class"), None);
    assert_eq!(header_hint("grade"), None);
    assert_eq!(header_hint("rank"), None);
    assert_eq!(header_hint("tier"), None);
}

#[test]
fn test_header_hint_count_columns() {
    assert_eq!(
        header_hint("SibSp"),
        Some("representation.numeric.integer_number")
    );
    assert_eq!(
        header_hint("Parch"),
        Some("representation.numeric.integer_number")
    );
    assert_eq!(
        header_hint("siblings"),
        Some("representation.numeric.integer_number")
    );
    assert_eq!(
        header_hint("children"),
        Some("representation.numeric.integer_number")
    );
    assert_eq!(
        header_hint("qty"),
        Some("representation.numeric.integer_number")
    );
}

#[test]
fn test_header_hint_survival_columns() {
    assert_eq!(
        header_hint("Survived"),
        Some("representation.boolean.binary")
    );
    assert_eq!(header_hint("alive"), Some("representation.boolean.binary"));
    assert_eq!(header_hint("active"), Some("representation.boolean.binary"));
}

#[test]
fn test_header_hint_ticket_cabin() {
    assert_eq!(
        header_hint("Ticket"),
        Some("representation.identifier.alphanumeric_id")
    );
    assert_eq!(
        header_hint("Cabin"),
        Some("representation.identifier.alphanumeric_id")
    );
    assert_eq!(
        header_hint("seat"),
        Some("representation.identifier.alphanumeric_id")
    );
}

#[test]
fn test_header_hint_embarked() {
    assert_eq!(header_hint("Embarked"), Some("representation.text.word"));
    assert_eq!(header_hint("terminal"), Some("representation.text.word"));
}

#[test]
fn test_header_hint_fare() {
    assert_eq!(header_hint("Fare"), Some("finance.currency.amount"));
    assert_eq!(header_hint("fee"), Some("finance.currency.amount"));
}

#[test]
fn compound_class_headers_no_longer_hint_ordinal() {
    // spec 2026-06-25-sharpen-stage-audit: the substring class/grade/rank/tier →
    // ordinal arm was retired alongside its exact-match twin. Compound headers
    // containing "class" no longer hint ordinal.
    assert_eq!(header_hint("passenger_class"), None);
    assert_eq!(header_hint("skill_grade"), None);
}

#[test]
fn header_hint_value_corroboration_is_default_on() {
    assert!(!rhh::is_disabled("header_hint_value_corroboration"));
}

#[test]
fn test_header_hint_publisher() {
    assert_eq!(
        header_hint("publisher"),
        Some("representation.text.entity_name"),
        "publisher should hint to entity_name"
    );
}

#[test]
fn test_header_hint_measurement_keywords() {
    assert_eq!(
        header_hint("pressure_atm"),
        Some("representation.numeric.decimal_number"),
        "pressure_atm should hint to decimal_number"
    );
    assert_eq!(
        header_hint("temperature"),
        Some("representation.numeric.decimal_number"),
        "temperature should hint to decimal_number"
    );
    assert_eq!(
        header_hint("voltage"),
        Some("representation.numeric.decimal_number"),
        "voltage should hint to decimal_number"
    );
}

#[test]
fn test_header_hint_priority_hardcoded_first() {
    // "job_title" has a hardcoded hint → categorical
    // Model2Vec might return entity_name for this header
    // With hardcoded-first, header_hint should win
    assert_eq!(
        header_hint("job_title"),
        Some("representation.text.word"),
        "job_title should hint to categorical (hardcoded)"
    );
    assert_eq!(
        header_hint("occupation"),
        Some("representation.text.word"),
        "occupation should hint to categorical (hardcoded)"
    );
}

#[test]
fn ac04_author_header_hint() {
    assert_eq!(
        header_hint("author"),
        Some("identity.person.full_name"),
        "author should hint to full_name"
    );
    assert_eq!(
        header_hint("authors"),
        Some("identity.person.full_name"),
        "authors should hint to full_name"
    );
    assert_eq!(
        header_hint("author_name"),
        Some("identity.person.full_name"),
        "author_name should hint to full_name"
    );
}

#[test]
fn ac03a_same_category_hardcoded_override_unconditional() {
    // Path A: hardcoded "phone" hint overrides ssn@1.00 because both are
    // identity.person.* — same category, no confidence threshold.
    let mock = crate::inference::MockClassifier::new("identity.person.ssn");
    let cc = ColumnClassifier::with_defaults(Box::new(mock));
    let mut result = make_result("identity.person.ssn", 1.00);
    let sample: Vec<String> = vec!["555-0100".into(); 5];

    cc.apply_header_sharpen(&mut result, "phone", &sample);

    assert_eq!(
        result.label, "identity.person.phone_number",
        "phone hint must override ssn@1.00 (same category, hardcoded)"
    );
    assert!(
        result.disambiguation_applied,
        "disambiguation_applied should be true"
    );
    assert_eq!(
        result.disambiguation_rule.as_deref(),
        Some("header_hint_same_category:phone"),
        "rule should be header_hint_same_category"
    );
}

#[test]
fn ac03b_same_domain_hardcoded_override_below_095() {
    // Path B: hardcoded "year" hint overrides compact_ym@0.83 because
    // both are datetime.* (same domain) and 0.83 < 0.95.
    let mock = crate::inference::MockClassifier::new("datetime.date.compact_ym");
    let cc = ColumnClassifier::with_defaults(Box::new(mock));
    let mut result = make_result("datetime.date.compact_ym", 0.83);
    let sample: Vec<String> = vec!["2024".into(); 5];

    cc.apply_header_sharpen(&mut result, "year", &sample);

    assert_eq!(
        result.label, "datetime.component.year",
        "year hint must override compact_ym@0.83 (same domain, < 0.90)"
    );
    assert!(result.disambiguation_applied);
}

#[test]
fn ac03b_same_domain_hardcoded_override_url_over_docker_ref() {
    // Path B: hardcoded "url" hint overrides docker_ref@0.60 because
    // both are technology.* (same domain) and 0.60 < 0.95.
    let mock = crate::inference::MockClassifier::new("technology.development.docker_ref");
    let cc = ColumnClassifier::with_defaults(Box::new(mock));
    let mut result = make_result("technology.development.docker_ref", 0.60);
    let sample: Vec<String> = vec!["https://example.com/track/123".into(); 5];

    cc.apply_header_sharpen(&mut result, "tracking_url", &sample);

    assert_eq!(
        result.label, "technology.internet.url",
        "url hint must override docker_ref@0.60 (same domain, < 0.90)"
    );
    assert!(result.disambiguation_applied);
}

#[test]
fn ac03b_same_domain_hardcoded_does_not_override_at_095() {
    // Regression: a hardcoded same-domain hint does NOT override when
    // confidence >= 0.95 — the model is very confident.
    // Uses phone→ssn (identity.person vs identity.government, same domain).
    let mock = crate::inference::MockClassifier::new("identity.government.ssn");
    let cc = ColumnClassifier::with_defaults(Box::new(mock));
    let mut result = make_result("identity.government.ssn", 0.96);
    let sample: Vec<String> = vec!["123-45-6789".into(); 5];

    cc.apply_header_sharpen(&mut result, "phone", &sample);

    assert_eq!(
        result.label, "identity.government.ssn",
        "hardcoded hint must NOT override at confidence >= 0.95"
    );
    assert!(
        !result.disambiguation_applied,
        "disambiguation should not be applied at >= 0.95"
    );
}

// ── AC-07(a): h.contains("uri") removal ─────────────────────────────

#[test]
fn ac07a_data_uri_header_no_longer_matches_url() {
    // After removing h.contains("uri"), "data_uri" should NOT map to url.
    // This was the root cause of audit item #1: the keyword match forced
    // "data_uri" → url, overriding the model's prediction.
    assert_eq!(
        header_hint("data_uri"),
        None,
        "data_uri should not match url after h.contains(\"uri\") removal"
    );
}

#[test]
fn ac07a_uri_exact_match_still_works() {
    // The exact match for "uri" at line 3859 must still map to url.
    assert_eq!(header_hint("uri"), Some("technology.internet.url"));
}

#[test]
fn ac07a_url_keyword_still_works() {
    // h.contains("url") still catches url-containing headers.
    assert_eq!(header_hint("download_url"), Some("technology.internet.url"));
    assert_eq!(header_hint("redirect_url"), Some("technology.internet.url"));
}

#[test]
fn ac07a_link_href_keywords_still_work() {
    // Other url keyword matches are unaffected.
    assert_eq!(
        header_hint("external_link"),
        Some("technology.internet.url")
    );
    assert_eq!(header_hint("href"), Some("technology.internet.url"));
}

#[test]
fn ac07a_request_uri_no_longer_matches_url() {
    // Pure "uri" headers without "url" lose the keyword hint.
    // The model handles these from value patterns.
    assert_eq!(
        header_hint("request_uri"),
        None,
        "request_uri should not match url after h.contains(\"uri\") removal"
    );
}

#[test]
fn ac05_alpha2_regex_rejects_lowercase() {
    // Lowercase should not match.
    let lower = ["au", "us", "gb"];
    for code in &lower {
        assert!(
            !(code.len() == 2 && code.chars().all(|c| c.is_ascii_uppercase())),
            "{} should not match alpha-2 pattern",
            code
        );
    }
}

// ── header_corroborates_title + entity_name_title_header_demotion
//    (dataset-descriptor audit: naics.title, nyc_payroll.title_description,
//    compound_codelist.title, gittables `Title` — all gold plain_text,
//    none entity_name) ──

#[test]
fn title_header_corroboration() {
    for h in [
        "title",
        "Title",
        "naics.title",
        "title_description",
        "primary_title",
    ] {
        assert!(header_corroborates_title(h), "should corroborate: {h}");
    }
    // false friends — `title` must match as a whole TOKEN, not a substring.
    for h in ["subtitle", "titled", "entitled", "name", "description"] {
        assert!(!header_corroborates_title(h), "should NOT corroborate: {h}");
    }
}

#[test]
fn entity_name_title_demotion_is_default_on() {
    assert!(!rhh::is_disabled("entity_name_title_header_demotion"));
}

#[test]
fn entity_name_title_demotion_fixes_naics_title_column() {
    // The card's motivating case: naics_codes.csv `title` — category labels
    // ("Crop Production", "Offices of Lawyers"), currently entity_name.
    let cc =
        ColumnClassifier::with_defaults(Box::new(crate::inference::MockClassifier::new("unknown")));
    let titles: Vec<String> = vec![
        "Soybean Farming",
        "Commercial and Institutional Building Construction",
        "Offices of Lawyers",
        "Custom Computer Programming Services",
        "General Automotive Repair",
        "Beauty Salons",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let r = cc
        .compose_from_sense("title", &titles, "representation.text.entity_name", 0.822)
        .unwrap();
    assert_eq!(
        r.label, "representation.text.plain_text",
        "rule was {:?}",
        r.disambiguation_rule
    );
    assert_eq!(
        r.disambiguation_rule.as_deref(),
        Some("entity_name_title_header_demotion")
    );
}

#[test]
fn entity_name_title_demotion_fires_on_namespaced_header() {
    // The dataset-descriptor catalog names the column `naics.title` (a
    // `<dataset>.<column>` namespace), which must corroborate identically to
    // the bare `title` header the raw CSV carries.
    let cc =
        ColumnClassifier::with_defaults(Box::new(crate::inference::MockClassifier::new("unknown")));
    let titles: Vec<String> = vec![
        "Soybean Farming",
        "Offices of Lawyers",
        "Beauty Salons",
        "Retail Bakeries",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let r = cc
        .compose_from_sense(
            "naics.title",
            &titles,
            "representation.text.entity_name",
            0.8,
        )
        .unwrap();
    assert_eq!(r.label, "representation.text.plain_text");
}

#[test]
fn entity_name_title_demotion_requires_the_title_token() {
    // Same values, a header that does NOT carry the `title` token — must
    // NOT demote (the header gate is load-bearing; no value-shape check
    // backs this rule, per the guard's own doc comment).
    let cc =
        ColumnClassifier::with_defaults(Box::new(crate::inference::MockClassifier::new("unknown")));
    let titles: Vec<String> = vec![
        "Soybean Farming",
        "Offices of Lawyers",
        "Beauty Salons",
        "Retail Bakeries",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let r = cc
        .compose_from_sense(
            "company_name",
            &titles,
            "representation.text.entity_name",
            0.8,
        )
        .unwrap();
    assert_ne!(
        r.label, "representation.text.plain_text",
        "a non-title header must not trigger the demotion"
    );
}

#[test]
fn entity_name_title_demotion_does_not_touch_other_labels() {
    // A `title`-headed column the model already typed correctly (e.g. a real
    // word/enum column) must be left alone — the guard only ever touches an
    // entity_name overcall.
    let values: Vec<String> = vec!["Mr", "Mrs", "Dr", "Ms", "Mr", "Dr"]
        .into_iter()
        .map(String::from)
        .collect();
    let cc =
        ColumnClassifier::with_defaults(Box::new(crate::inference::MockClassifier::new("unknown")));
    let r = cc
        .compose_from_sense("title", &values, "representation.text.word", 0.9)
        .unwrap();
    assert_eq!(r.label, "representation.text.word");
    assert_ne!(
        r.disambiguation_rule.as_deref(),
        Some("entity_name_title_header_demotion")
    );
}
